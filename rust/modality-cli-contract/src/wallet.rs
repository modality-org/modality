//! A wallet: a contract whose id is its owner's key, and which only that key
//! can extend. Its genesis names the key as the one signer (`create --key K
//! --signer K`), so the network refuses any commit the key did not sign.
//! The signer is fixed at creation.
//!
//! Anyone can send to the wallet's id before the wallet is on the network.
//! The owner takes what was sent with one `RECV` per `SEND`, stating what it
//! receives, as the network requires. A node's `/contract/account` lists
//! what the wallet holds and what waits for it.

use anyhow::{anyhow, bail, Context, Result};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashSet;
use std::path::PathBuf;

use modality_common::contract_store::ContractStore;
use modality_common::keypair::Keypair;

const REMOTE: &str = "origin";
const WALLET_FILE: &str = "wallet.json";

#[derive(Debug, Subcommand)]
pub enum Commands {
    #[command(about = "Create a wallet: a contract only your key can extend, at your key's id")]
    Create(CreateOpts),

    #[command(about = "Print the wallet's address (its contract id)")]
    Address(WalletOpts),

    #[command(about = "Show what the wallet holds, and how many sends wait for it")]
    Balance(WalletOpts),

    #[command(about = "List the sends to the wallet that it has not received")]
    Incoming(WalletOpts),

    #[command(about = "Receive every waiting send, and push the receives")]
    Recv(WalletOpts),

    #[command(about = "Send an asset the wallet holds to another contract")]
    Send(SendOpts),
}

#[derive(Debug, Parser)]
pub struct CreateOpts {
    /// The owner's key (passfile path or identity name). Default: a new key,
    /// written to `<dir>/owner.mod_passfile`.
    #[clap(long)]
    key: Option<String>,

    /// Wallet directory (default: ~/.modality/wallet)
    #[clap(long)]
    dir: Option<PathBuf>,

    /// The node the wallet talks to (default: a testnet bootstrapper)
    #[clap(long)]
    remote: Option<String>,

    /// Output format (json or text)
    #[clap(long, default_value = "text")]
    output: String,
}

#[derive(Debug, Parser)]
pub struct WalletOpts {
    /// Wallet directory (default: ~/.modality/wallet)
    #[clap(long)]
    dir: Option<PathBuf>,

    /// Use this node instead of the wallet's remote
    #[clap(long)]
    remote: Option<String>,

    /// Output format (json or text)
    #[clap(long, default_value = "text")]
    output: String,
}

#[derive(Debug, Parser)]
pub struct SendOpts {
    /// The contract to pay
    #[clap(long)]
    to: String,

    /// How much, in whole units of the asset (e.g. 1.5 MOD)
    #[clap(long)]
    amount: String,

    /// The asset: `MOD`, or its id together with --asset-contract
    #[clap(long, default_value = "MOD")]
    asset: String,

    /// The contract that created the asset (not needed for MOD)
    #[clap(long)]
    asset_contract: Option<String>,

    /// A note for the receiver (JSON or text)
    #[clap(long)]
    memo: Option<String>,

    #[clap(flatten)]
    wallet: WalletOpts,
}

/// `.contract/wallet.json`: which key signs for the wallet.
#[derive(Debug, Serialize, Deserialize)]
struct WalletFile {
    key: PathBuf,
}

pub async fn run(command: &Commands) -> Result<()> {
    match command {
        Commands::Create(opts) => create(opts).await,
        Commands::Address(opts) => address(opts),
        Commands::Balance(opts) => balance(opts).await,
        Commands::Incoming(opts) => incoming(opts).await,
        Commands::Recv(opts) => recv(opts).await,
        Commands::Send(opts) => send(opts).await,
    }
}

fn wallet_dir(dir: &Option<PathBuf>) -> Result<PathBuf> {
    match dir {
        Some(dir) => Ok(dir.clone()),
        None => modality_common::passfile::default_wallet_dir(),
    }
}

fn default_remote() -> Result<String> {
    modality_networks::networks::testnet()
        .bootstrappers
        .first()
        .cloned()
        .ok_or_else(|| anyhow!("the testnet names no bootstrapper; pass --remote"))
}

struct Wallet {
    dir: PathBuf,
    store: ContractStore,
    id: String,
    key: PathBuf,
    remote: String,
}

impl Wallet {
    fn open(opts: &WalletOpts) -> Result<Self> {
        let dir = wallet_dir(&opts.dir)?;
        let store = ContractStore::open(&dir).with_context(|| {
            format!("no wallet at {}; run `modal wallet create`", dir.display())
        })?;
        let config = store.load_config()?;
        let file: WalletFile = serde_json::from_str(
            &std::fs::read_to_string(store.contract_dir().join(WALLET_FILE))
                .with_context(|| format!("{} is a contract, not a wallet", dir.display()))?,
        )?;
        let remote = match &opts.remote {
            Some(remote) => remote.clone(),
            None => config
                .get_remote(REMOTE)
                .map(|r| r.url.clone())
                .ok_or_else(|| anyhow!("the wallet has no remote; pass --remote"))?,
        };
        Ok(Self {
            dir,
            id: config.contract_id,
            store,
            key: file.key,
            remote,
        })
    }

    async fn account(&self) -> Result<Account> {
        let response = crate::push::p2p_request(
            None,
            &self.remote,
            "/contract/account",
            &json!({ "contract_id": self.id }),
        )
        .await?;
        if !response.ok {
            bail!("{} cannot show the account: {:?}", self.remote, response.errors);
        }
        Ok(serde_json::from_value(
            response.data.ok_or_else(|| anyhow!("{} sent no account", self.remote))?,
        )?)
    }

    /// Make a commit with `args` (after `commit`), signed by the wallet's key.
    async fn commit(&self, args: &[&str]) -> Result<String> {
        let dir = self.dir.to_string_lossy().to_string();
        let key = self.key.to_string_lossy().to_string();
        let mut argv = vec!["commit", "--dir", &dir, "--sign", &key, "--payer", &self.id];
        argv.extend_from_slice(args);
        let made = crate::commit::make(&crate::commit::Opts::parse_from(argv))
            .await?
            .ok_or_else(|| anyhow!("nothing to commit"))?;
        Ok(made.commit_id)
    }

    /// Push every commit the remote has not taken. Returns their ids.
    async fn push(&self) -> Result<Vec<String>> {
        let unpushed = self.store.get_unpushed_commits(REMOTE)?;
        if unpushed.is_empty() {
            return Ok(unpushed);
        }
        let mut commits = Vec::new();
        for commit_id in &unpushed {
            let commit = self.store.load_commit(commit_id)?;
            commits.push(json!({"commit_id": commit_id, "body": commit.body, "head": commit.head}));
        }
        let response = crate::push::p2p_request(
            None,
            &self.remote,
            "/contract/push",
            &json!({"contract_id": self.id, "commits": commits}),
        )
        .await?;
        if !response.ok {
            bail!("{} refused the push: {:?}", self.remote, response.errors);
        }
        if let Some(last) = unpushed.last() {
            self.store.set_remote_head(REMOTE, last)?;
        }
        Ok(unpushed)
    }

    /// The sends this copy already receives, pushed or not.
    fn received_here(&self) -> Result<HashSet<(String, u64)>> {
        let mut received = HashSet::new();
        let mut current = self.store.get_head()?;
        while let Some(id) = current {
            let commit = self.store.load_commit(&id)?;
            for action in commit.body.iter().filter(|a| a.method == "recv") {
                if let Some(send) = action.value.get("send_commit_id").and_then(Value::as_str) {
                    let index = action.value.get("send_index").and_then(Value::as_u64).unwrap_or(0);
                    received.insert((send.to_string(), index));
                }
            }
            current = commit.head.parent.filter(|p| !p.is_empty());
        }
        Ok(received)
    }
}

#[derive(Debug, Deserialize)]
struct Account {
    mod_contract_id: Option<String>,
    holdings: Vec<Holding>,
    incoming: Vec<Incoming>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Holding {
    asset_contract: String,
    asset_id: String,
    balance: u64,
    divisibility: Option<u64>,
    decimals: Option<u32>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Incoming {
    send_commit_id: String,
    send_index: u64,
    from_contract: String,
    asset_contract: String,
    asset_id: String,
    amount: u64,
    memo: Option<Value>,
    decimals: Option<u32>,
}

impl Account {
    fn name(&self, asset_contract: &str, asset_id: &str) -> String {
        if asset_id == "MOD" && self.mod_contract_id.as_deref() == Some(asset_contract) {
            "MOD".to_string()
        } else {
            format!("{asset_id} ({asset_contract})")
        }
    }


}

pub use modality_common::amount::{format_amount, parse_amount};

async fn create(opts: &CreateOpts) -> Result<()> {
    let dir = wallet_dir(&opts.dir)?;
    if dir.join(".contract").exists() {
        bail!("a wallet already exists at {}", dir.display());
    }
    std::fs::create_dir_all(&dir)?;
    let key = match &opts.key {
        Some(reference) => std::fs::canonicalize(
            modality_common::passfile::resolve_passfile_path(reference)?,
        )?,
        None => {
            let path = dir.join("owner.mod_passfile");
            Keypair::generate()?.as_json_file(&path.to_string_lossy())?;
            std::fs::canonicalize(path)?
        }
    };
    let id = Keypair::from_json_file(&key.to_string_lossy())?.as_public_address();
    let remote = match &opts.remote {
        Some(remote) => remote.clone(),
        None => default_remote()?,
    };

    // A wallet for this key already on the network is the wallet: copy it.
    // A second genesis at the same id would be refused as a fork.
    let existing = crate::push::p2p_request(
        None,
        &remote,
        "/contract/pull",
        &json!({"contract_id": id, "since_commit_id": null}),
    )
    .await
    .ok()
    .filter(|r| r.ok)
    .and_then(|r| r.data)
    .and_then(|d| d.get("commits").and_then(Value::as_array).cloned())
    .filter(|commits| !commits.is_empty());

    let how = if existing.is_some() {
        let dir_arg = dir.to_string_lossy().to_string();
        crate::pull::run(&crate::pull::Opts::parse_from([
            "pull",
            "--contract-id",
            &id,
            "--remote",
            &remote,
            "--dir",
            &dir_arg,
            "--output",
            "json",
        ]))
        .await?;
        "copied from the network"
    } else {
        let dir_arg = dir.to_string_lossy().to_string();
        let key_arg = key.to_string_lossy().to_string();
        let created = crate::create::make(&crate::create::Opts::parse_from([
            "create", "--dir", &dir_arg, "--key", &key_arg, "--signer", &key_arg, "--payer",
            &key_arg,
        ]))
        .await?;
        debug_assert_eq!(created.contract_id, id);
        let store = ContractStore::open(&dir)?;
        let mut config = store.load_config()?;
        config.add_remote(REMOTE.to_string(), remote.clone());
        store.save_config(&config)?;
        "created"
    };
    let store = ContractStore::open(&dir)?;
    std::fs::write(
        store.contract_dir().join(WALLET_FILE),
        serde_json::to_string_pretty(&WalletFile { key: key.clone() })?,
    )?;

    if opts.output == "json" {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "address": id,
                "dir": dir,
                "key": key,
                "remote": remote,
                "status": how,
            }))?
        );
    } else {
        println!("✅ Wallet {how}");
        println!("   Address: {id}");
        println!("   Directory: {}", dir.display());
        println!("   Key: {}", key.display());
        println!("   Remote: {remote}");
        println!();
        println!("Share the address to be paid. Then: modal wallet recv");
    }
    Ok(())
}

fn address(opts: &WalletOpts) -> Result<()> {
    let wallet = Wallet::open(opts)?;
    if opts.output == "json" {
        println!("{}", json!({ "address": wallet.id }));
    } else {
        println!("{}", wallet.id);
    }
    Ok(())
}

async fn balance(opts: &WalletOpts) -> Result<()> {
    let wallet = Wallet::open(opts)?;
    let account = wallet.account().await?;
    if opts.output == "json" {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "address": wallet.id,
                "holdings": account.holdings.iter().map(|h| json!({
                    "asset": account.name(&h.asset_contract, &h.asset_id),
                    "asset_contract": h.asset_contract,
                    "asset_id": h.asset_id,
                    "balance": h.balance,
                    "amount": format_amount(h.balance, h.decimals),
                })).collect::<Vec<_>>(),
                "incoming": account.incoming.len(),
            }))?
        );
        return Ok(());
    }
    println!("Wallet {}", wallet.id);
    if account.holdings.is_empty() {
        println!("   Holds nothing yet");
    }
    for h in &account.holdings {
        println!(
            "   {} {}",
            format_amount(h.balance, h.decimals),
            account.name(&h.asset_contract, &h.asset_id)
        );
    }
    if !account.incoming.is_empty() {
        println!();
        println!(
            "{} send(s) wait to be received: modal wallet recv",
            account.incoming.len()
        );
    }
    Ok(())
}

async fn incoming(opts: &WalletOpts) -> Result<()> {
    let wallet = Wallet::open(opts)?;
    let account = wallet.account().await?;
    if opts.output == "json" {
        println!("{}", serde_json::to_string_pretty(&account.incoming)?);
        return Ok(());
    }
    if account.incoming.is_empty() {
        println!("Nothing waits to be received.");
    }
    for send in &account.incoming {
        println!(
            "{} {} from {} (SEND {} #{})",
            format_amount(send.amount, send.decimals),
            account.name(&send.asset_contract, &send.asset_id),
            send.from_contract,
            send.send_commit_id,
            send.send_index
        );
    }
    Ok(())
}

async fn recv(opts: &WalletOpts) -> Result<()> {
    let wallet = Wallet::open(opts)?;
    let account = wallet.account().await?;
    let already = wallet.received_here()?;
    let mut received = Vec::new();
    for send in &account.incoming {
        if already.contains(&(send.send_commit_id.clone(), send.send_index)) {
            continue;
        }
        let index = send.send_index.to_string();
        let amount = send.amount.to_string();
        let commit_id = wallet
            .commit(&[
                "--method",
                "recv",
                "--send-commit-id",
                &send.send_commit_id,
                "--send-index",
                &index,
                "--asset-contract",
                &send.asset_contract,
                "--asset-id",
                &send.asset_id,
                "--amount",
                &amount,
            ])
            .await
            .with_context(|| {
                format!(
                    "receiving SEND {} #{}",
                    send.send_commit_id, send.send_index
                )
            })?;
        received.push(json!({
            "commit_id": commit_id,
            "send_commit_id": send.send_commit_id,
            "send_index": send.send_index,
            "asset": account.name(&send.asset_contract, &send.asset_id),
            "amount": send.amount,
            "shown": format_amount(send.amount, send.decimals),
        }));
    }
    let pushed = wallet.push().await?;
    if opts.output == "json" {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "received": received,
                "pushed": pushed,
            }))?
        );
        return Ok(());
    }
    if received.is_empty() {
        println!("Nothing new to receive.");
    }
    for r in &received {
        println!(
            "✅ Received {} {}",
            r["shown"].as_str().unwrap_or_default(),
            r["asset"].as_str().unwrap_or_default()
        );
    }
    if !pushed.is_empty() {
        println!(
            "   Pushed {} commit(s) to {}; they count once sequenced.",
            pushed.len(),
            wallet.remote
        );
    }
    Ok(())
}

async fn send(opts: &SendOpts) -> Result<()> {
    let wallet = Wallet::open(&opts.wallet)?;
    let account = wallet.account().await?;
    let (asset_contract, asset_id) = match &opts.asset_contract {
        Some(creator) => (creator.clone(), opts.asset.clone()),
        None if opts.asset == "MOD" => (
            account.mod_contract_id.clone().ok_or_else(|| {
                anyhow!("this network has no MOD contract; name the asset with --asset-contract")
            })?,
            "MOD".to_string(),
        ),
        None => bail!("--asset {} needs --asset-contract, its creator", opts.asset),
    };
    let held = account
        .holdings
        .iter()
        .find(|h| h.asset_contract == asset_contract && h.asset_id == asset_id)
        .ok_or_else(|| {
            anyhow!(
                "the wallet holds no {}; received sends count once sequenced",
                account.name(&asset_contract, &asset_id)
            )
        })?;
    let decimals = held.decimals;
    let amount = parse_amount(&opts.amount, decimals)?;
    let step = held.divisibility.unwrap_or(1).max(1);
    if amount % step != 0 {
        bail!(
            "{} moves in steps of {}; {} is not one",
            account.name(&asset_contract, &asset_id),
            format_amount(step, decimals),
            opts.amount
        );
    }
    if amount > held.balance {
        bail!(
            "the wallet holds {} {}, less than {}",
            format_amount(held.balance, decimals),
            account.name(&asset_contract, &asset_id),
            opts.amount
        );
    }
    let amount_arg = amount.to_string();
    let mut args = vec![
        "--method",
        "send",
        "--asset-contract",
        &asset_contract,
        "--asset-id",
        &asset_id,
        "--to-contract",
        &opts.to,
        "--amount",
        &amount_arg,
    ];
    if let Some(memo) = &opts.memo {
        args.extend_from_slice(&["--memo", memo]);
    }
    let commit_id = wallet.commit(&args).await?;
    let pushed = wallet.push().await?;
    if opts.wallet.output == "json" {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "commit_id": commit_id,
                "to": opts.to,
                "asset": account.name(&asset_contract, &asset_id),
                "amount": amount,
                "pushed": pushed,
            }))?
        );
    } else {
        println!(
            "✅ Sent {} {} to {}",
            format_amount(amount, decimals),
            account.name(&asset_contract, &asset_id),
            opts.to
        );
        println!("   SEND commit {commit_id}; the receiver takes it with a RECV.");
    }
    Ok(())
}
