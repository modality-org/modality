//! `modal release`: the release contract. Packages are trusted only when
//! the release contract accepts the commit that names them
//! (`modality_common::release`). `init` makes the contract, `publish`
//! appends a release (run in CI with the release key), `verify` checks a
//! downloaded file against a published log, and `export`/`import` move a
//! log between a file and a contract directory.

use anyhow::{anyhow, bail, Context, Result};
use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};

use modality_common::contract_store::ContractStore;
use modality_common::release::{self, Entry, Log, LogCommit, Pin};

#[derive(Debug, Subcommand)]
pub enum Commands {
    #[command(about = "Make a release contract: the CI key may post releases; maintainers change the rest")]
    Init(InitOpts),
    #[command(about = "Append a release, signed by the release key, to a release log")]
    Publish(PublishOpts),
    #[command(about = "Check files against a release the release contract accepts")]
    Verify(VerifyOpts),
    #[command(about = "Write a release contract directory's log as JSON")]
    Export(ExportOpts),
    #[command(about = "Make a release contract directory from a log")]
    Import(ImportOpts),
}

#[derive(Debug, Parser)]
pub struct InitOpts {
    /// Directory for the new contract
    #[clap(long)]
    dir: PathBuf,
    /// A maintainer's passfile; signs the setup commit. Repeat for several.
    #[clap(long = "maintainer", required = true)]
    maintainers: Vec<String>,
    /// The key allowed to post releases (Modality ID or passfile)
    #[clap(long)]
    ci: String,
}

#[derive(Debug, Parser)]
pub struct PublishOpts {
    /// The release log to extend (path)
    #[clap(long)]
    log: PathBuf,
    /// Where to write the extended log
    #[clap(long)]
    out: PathBuf,
    #[clap(long)]
    channel: String,
    #[clap(long)]
    version: String,
    /// The source commit the files were built from
    #[clap(long)]
    git_commit: String,
    /// `<package path>=<local file>`, e.g. binaries/linux-x86_64/modal=./modal. Repeat.
    #[clap(long = "file", required = true)]
    files: Vec<String>,
    /// The release key's passfile
    #[clap(long)]
    sign: String,
}

#[derive(Debug, Parser)]
pub struct VerifyOpts {
    /// The release log: a path or an http(s) URL
    #[clap(long)]
    log: String,
    #[clap(long)]
    channel: String,
    /// The release to check; default: the channel's latest
    #[clap(long)]
    version: Option<String>,
    /// `<package path>=<local file>` to check. Repeat.
    #[clap(long = "file")]
    files: Vec<String>,
    /// Trust this contract instead of the one this build pins:
    /// `<contract id>:<genesis commit id>`
    #[clap(long)]
    pin: Option<String>,
    /// Output format (json or text)
    #[clap(long, default_value = "text")]
    output: String,
}

#[derive(Debug, Parser)]
pub struct ExportOpts {
    #[clap(long)]
    dir: PathBuf,
    #[clap(long)]
    out: PathBuf,
}

#[derive(Debug, Parser)]
pub struct ImportOpts {
    #[clap(long)]
    log: PathBuf,
    #[clap(long)]
    dir: PathBuf,
}

pub async fn run(command: &Commands) -> Result<()> {
    match command {
        Commands::Init(opts) => init(opts).await,
        Commands::Publish(opts) => publish(opts).await,
        Commands::Verify(opts) => verify(opts).await,
        Commands::Export(opts) => {
            let log = export(&ContractStore::open(&opts.dir)?)?;
            std::fs::write(&opts.out, serde_json::to_string_pretty(&log)?)?;
            println!("✅ Wrote {} commit(s) to {}", log.commits.len(), opts.out.display());
            Ok(())
        }
        Commands::Import(opts) => {
            let log: Log = serde_json::from_str(&std::fs::read_to_string(&opts.log)?)?;
            import(&log, &opts.dir)?;
            println!("✅ Imported {} commit(s) into {}", log.commits.len(), opts.dir.display());
            Ok(())
        }
    }
}

fn dir_arg(dir: &Path) -> String {
    dir.to_string_lossy().to_string()
}

async fn init(opts: &InitOpts) -> Result<()> {
    let dir = dir_arg(&opts.dir);
    let created = crate::create::make(&crate::create::Opts::parse_from(["create", "--dir", &dir])).await?;
    let store = ContractStore::open(&opts.dir)?;
    store.write_state("/keys/ci.id", &crate::signer_set::signer_id(&opts.ci)?.into())?;
    for (i, maintainer) in opts.maintainers.iter().enumerate() {
        store.write_state(
            &format!("/maintainers/{}.id", i + 1),
            &crate::signer_set::signer_id(maintainer)?.into(),
        )?;
    }
    std::fs::write(opts.dir.join("model").join("default.modality"), release::MODEL)?;
    let rules_dir = opts.dir.join("rules");
    std::fs::create_dir_all(&rules_dir)?;
    for (name, formula) in release::RULES {
        std::fs::write(
            rules_dir.join(format!("{name}.modality")),
            crate::add_rule::format_rule(formula, "$PARENT"),
        )?;
    }
    let mut argv = vec![
        "commit".to_string(),
        "--dir".into(),
        dir.clone(),
        "--all".into(),
        "-m".into(),
        "Release contract".into(),
    ];
    for maintainer in &opts.maintainers {
        argv.extend(["--sign".to_string(), maintainer.clone()]);
    }
    crate::commit::make(&crate::commit::Opts::parse_from(argv))
        .await?
        .ok_or_else(|| anyhow!("the setup commit is empty"))?;
    let log = export(&store)?;
    let pin = pin_of(&log);
    release::verify_log(&log, pin).context("the new release contract does not accept itself")?;
    println!("✅ Release contract created");
    println!("   Contract ID: {}", created.contract_id);
    println!("   Genesis commit: {}", created.genesis_commit_id);
    println!("   Pin it in modality_common::release for its channel.");
    Ok(())
}

async fn publish(opts: &PublishOpts) -> Result<()> {
    let log: Log = serde_json::from_str(&std::fs::read_to_string(&opts.log)?)
        .with_context(|| format!("reading {}", opts.log.display()))?;
    let work = tempfile_dir()?;
    import(&log, &work)?;
    let mut files = std::collections::BTreeMap::new();
    for spec in &opts.files {
        let (path, local) = spec
            .split_once('=')
            .ok_or_else(|| anyhow!("--file {spec}: expected <package path>=<local file>"))?;
        let bytes = std::fs::read(local).with_context(|| format!("reading {local}"))?;
        files.insert(path.to_string(), release::sha256_hex(&bytes));
    }
    let entry = Entry {
        version: opts.version.clone(),
        channel: opts.channel.clone(),
        git_commit: opts.git_commit.clone(),
        files,
    };
    let store = ContractStore::open(&work)?;
    store.write_state(
        &release::entry_path(&opts.channel, &opts.version),
        &serde_json::to_value(&entry)?,
    )?;
    store.write_state(&release::latest_path(&opts.channel), &opts.version.clone().into())?;
    let dir = dir_arg(&work);
    crate::commit::make(&crate::commit::Opts::parse_from([
        "commit",
        "--dir",
        &dir,
        "--all",
        "--sign",
        &opts.sign,
        "-m",
        &format!("Release {} {}", opts.channel, opts.version),
    ]))
    .await?
    .ok_or_else(|| anyhow!("the release commit is empty: is {} already published?", opts.version))?;
    let out = export(&store)?;
    release::verify_log(&out, pin_of(&out)).context("the release contract refuses this release")?;
    std::fs::write(&opts.out, serde_json::to_string_pretty(&out)?)?;
    let _ = std::fs::remove_dir_all(&work);
    println!("✅ Release {} {} accepted by contract {}", opts.channel, opts.version, out.contract_id);
    Ok(())
}

async fn verify(opts: &VerifyOpts) -> Result<()> {
    let pin = match &opts.pin {
        Some(spec) => {
            let (contract, genesis) = spec
                .split_once(':')
                .ok_or_else(|| anyhow!("--pin: expected <contract id>:<genesis commit id>"))?;
            Pin {
                contract_id: Box::leak(contract.to_string().into_boxed_str()),
                genesis_commit_id: Box::leak(genesis.to_string().into_boxed_str()),
            }
        }
        None => release::pin_for(&opts.channel)
            .ok_or_else(|| anyhow!("this build pins no release contract for {}", opts.channel))?,
    };
    let log_json = if opts.log.starts_with("http://") || opts.log.starts_with("https://") {
        let response = reqwest::get(&opts.log).await?;
        if !response.status().is_success() {
            bail!("{}: HTTP {}", opts.log, response.status());
        }
        response.text().await?
    } else {
        std::fs::read_to_string(&opts.log)?
    };
    let log: Log = serde_json::from_str(&log_json).context("the release log is not JSON")?;
    let state = release::verify_log(&log, pin)?;
    let entry = release::entry(&state, &opts.channel, opts.version.as_deref())?;
    for spec in &opts.files {
        let (path, local) = spec
            .split_once('=')
            .ok_or_else(|| anyhow!("--file {spec}: expected <package path>=<local file>"))?;
        release::check_file(&entry, path, &std::fs::read(local)?)?;
    }
    if opts.output == "json" {
        println!("{}", serde_json::to_string_pretty(&serde_json::json!({
            "verified": true,
            "contract_id": pin.contract_id,
            "release": entry,
            "files_checked": opts.files.len(),
        }))?);
    } else {
        println!(
            "✅ {} release {} ({}) is accepted by release contract {}; {} file(s) match",
            entry.channel,
            entry.version,
            entry.git_commit,
            pin.contract_id,
            opts.files.len()
        );
    }
    Ok(())
}

/// The pin a log names for itself: its contract and first commit. Only for
/// checking a log we just wrote; a reader pins from its build.
fn pin_of(log: &Log) -> Pin {
    Pin {
        contract_id: Box::leak(log.contract_id.clone().into_boxed_str()),
        genesis_commit_id: Box::leak(
            log.commits
                .first()
                .map(|c| c.commit_id.clone())
                .unwrap_or_default()
                .into_boxed_str(),
        ),
    }
}

/// A contract directory's log, genesis first.
pub fn export(store: &ContractStore) -> Result<Log> {
    let contract_id = store.load_config()?.contract_id;
    let mut commits = Vec::new();
    let mut current = store.get_head()?;
    while let Some(id) = current {
        let commit = store.load_commit(&id)?;
        current = commit.head.parent.clone().filter(|p| !p.is_empty());
        commits.push(LogCommit {
            commit_id: id,
            body: serde_json::to_value(&commit.body)?,
            head: serde_json::to_value(&commit.head)?,
        });
    }
    commits.reverse();
    Ok(Log { contract_id, commits })
}

/// A contract directory holding `log`.
pub fn import(log: &Log, dir: &Path) -> Result<()> {
    let store = ContractStore::init(dir, log.contract_id.clone())?;
    let mut last = None;
    for commit in &log.commits {
        let file = modality_common::contract_store::CommitFile::verified(
            &commit.commit_id,
            Some(&commit.body),
            Some(&commit.head),
        )?;
        store.save_commit(&commit.commit_id, &file)?;
        last = Some(commit.commit_id.clone());
    }
    if let Some(head) = last {
        store.set_head(&head)?;
    }
    crate::checkout::checkout(&store)?;
    Ok(())
}

fn tempfile_dir() -> Result<PathBuf> {
    let dir = std::env::temp_dir().join(format!(
        "modal-release-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos()
    ));
    Ok(dir)
}
