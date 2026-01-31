use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

use modal_common::keypair::Keypair;
use modal_common::contract_store::{ContractStore, CommitFile};

/// Agent-friendly contract operations
#[derive(Parser, Debug)]
pub struct Opts {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Create a new contract in a directory (dotcontract format)
    Create {
        /// Directory path where the contract will be created (defaults to current directory)
        #[arg(long)]
        dir: Option<PathBuf>,
        
        /// Output format (json or text)
        #[arg(long, default_value = "text")]
        output: String,
    },
    
    /// Propose a contract to another party
    Propose {
        /// Contract type: escrow, handshake, service, swap
        #[arg(short = 't', long)]
        r#type: String,
        
        /// Your agent ID
        #[arg(long)]
        from: String,
        
        /// Other party's agent ID
        #[arg(long)]
        to: String,
        
        /// Optional terms/description
        #[arg(long)]
        terms: Option<String>,
        
        /// Output file (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    
    /// Accept a proposal and create a contract
    Accept {
        /// Proposal JSON file
        #[arg(short, long)]
        proposal: PathBuf,
        
        /// Output file (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    
    /// Show contract status
    Status {
        /// Contract JSON file
        #[arg(short, long)]
        contract: PathBuf,
    },
    
    /// List available actions for an agent
    Actions {
        /// Contract JSON file
        #[arg(short, long)]
        contract: PathBuf,
        
        /// Your agent ID
        #[arg(long)]
        agent: String,
    },
    
    /// Commit an action to the contract
    Act {
        /// Contract JSON file
        #[arg(short, long)]
        contract: PathBuf,
        
        /// Your agent ID
        #[arg(long)]
        agent: String,
        
        /// Action name (e.g., deposit, deliver, release)
        #[arg(long)]
        action: String,
        
        /// Output file (default: overwrites input)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    
    /// Show contract history (from JSON file)
    History {
        /// Contract JSON file
        #[arg(short, long)]
        contract: PathBuf,
    },
    
    /// Show commit log (dotcontract format)
    Log {
        /// Contract directory (defaults to current directory)
        #[arg(long)]
        dir: Option<PathBuf>,
        
        /// Number of commits to show (default: all)
        #[arg(short = 'n', long)]
        limit: Option<usize>,
        
        /// Output format (json or text)
        #[arg(long, default_value = "text")]
        output: String,
    },
    
    /// Verify contract properties
    Verify {
        /// Contract model file (.modality or JSON)
        #[arg(short, long)]
        model: PathBuf,
        
        /// Formula to check
        #[arg(short, long)]
        formula: Option<String>,
    },
    
    /// Parse and validate a contract from .modality file
    Parse {
        /// Contract file (.modality)
        file: PathBuf,
    },
}

pub async fn run(opts: &Opts) -> Result<()> {
    match &opts.command {
        Command::Create { dir, output } => {
            create_contract(dir.as_ref(), output)
        }
        Command::Propose { r#type, from, to, terms, output } => {
            propose_contract(r#type, from, to, terms.as_deref(), output.as_ref())
        }
        Command::Accept { proposal, output } => {
            accept_proposal(proposal, output.as_ref())
        }
        Command::Status { contract } => {
            show_status(contract)
        }
        Command::Actions { contract, agent } => {
            show_actions(contract, agent)
        }
        Command::Act { contract, agent, action, output } => {
            commit_action(contract, agent, action, output.as_ref())
        }
        Command::History { contract } => {
            show_history(contract)
        }
        Command::Log { dir, limit, output } => {
            show_log(dir.as_ref(), *limit, &output)
        }
        Command::Verify { model, formula } => {
            verify_contract(model, formula.as_deref())
        }
        Command::Parse { file } => {
            parse_contract_file(file)
        }
    }
}

fn create_contract(dir: Option<&PathBuf>, output: &str) -> Result<()> {
    // Determine the contract directory
    let dir = if let Some(path) = dir {
        path.clone()
    } else {
        std::env::current_dir()?
    };

    // Generate a keypair for the contract
    let keypair = Keypair::generate()?;
    let contract_id = keypair.as_public_address();

    // Initialize the contract store
    let store = ContractStore::init(&dir, contract_id.clone())?;

    // Create genesis commit
    let genesis = serde_json::json!({
        "genesis": {
            "contract_id": contract_id.clone(),
            "created_at": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs(),
            "public_key": keypair.public_key_as_base58_identity()
        }
    });

    // Save genesis
    store.save_genesis(&genesis)?;

    // Create initial genesis commit as HEAD
    let mut genesis_commit = CommitFile::new();
    genesis_commit.add_action(
        "genesis".to_string(),
        None,
        genesis.clone()
    );
    
    let genesis_commit_id = genesis_commit.compute_id()?;
    store.save_commit(&genesis_commit_id, &genesis_commit)?;
    store.set_head(&genesis_commit_id)?;

    // Output
    if output == "json" {
        println!("{}", serde_json::to_string_pretty(&serde_json::json!({
            "contract_id": contract_id,
            "directory": dir.display().to_string(),
            "genesis_commit_id": genesis_commit_id,
        }))?);
    } else {
        println!("✅ Contract created successfully!");
        println!("   Contract ID: {}", contract_id);
        println!("   Directory: {}", dir.display());
        println!("   Genesis commit: {}", genesis_commit_id);
        println!();
        println!("Next steps:");
        println!("  1. cd {}", dir.display());
        println!("  2. modality contract commit --path /data --value 'hello'");
        println!("  3. modality contract push --remote <node-multiaddr>");
    }

    Ok(())
}

fn propose_contract(contract_type: &str, from: &str, to: &str, terms: Option<&str>, output: Option<&PathBuf>) -> Result<()> {
    use modality_lang::agent::ContractProposal;
    
    let proposal = match contract_type {
        "escrow" => ContractProposal::escrow(from, to),
        "service" | "service_agreement" => {
            ContractProposal::service(from, to, terms.unwrap_or("Standard service agreement"))
        }
        other => return Err(anyhow::anyhow!("Unknown contract type for proposal: '{}'. Options: escrow, service", other)),
    };
    
    let json = proposal.to_json()?;
    
    if let Some(path) = output {
        std::fs::write(path, &json)?;
        println!("Proposal created: {}", path.display());
        println!("Send this file to {} for review", to);
    } else {
        println!("{}", json);
    }
    
    Ok(())
}

fn accept_proposal(proposal_path: &PathBuf, output: Option<&PathBuf>) -> Result<()> {
    use modality_lang::agent::ContractProposal;
    
    let proposal_json = std::fs::read_to_string(proposal_path)?;
    let proposal = ContractProposal::from_json(&proposal_json)?;
    
    println!("Proposal from: {}", proposal.proposed_by);
    println!("Type: {}", proposal.proposal_type);
    if let Some(terms) = &proposal.terms {
        println!("Terms: {}", terms);
    }
    println!("Parties: {}", proposal.parties.join(", "));
    
    let contract = proposal.accept();
    let json = contract.to_json()?;
    
    if let Some(path) = output {
        std::fs::write(path, &json)?;
        println!("\nContract created: {}", path.display());
    } else {
        println!("\n{}", json);
    }
    
    Ok(())
}

fn show_status(contract_path: &PathBuf) -> Result<()> {
    use modality_lang::agent::Contract;
    
    let json = std::fs::read_to_string(contract_path)?;
    let contract = Contract::from_json(&json)?;
    
    println!("{}", contract.summary());
    println!("");
    
    let status = contract.status();
    println!("Contract ID: {}", contract.id());
    println!("Type: {}", status.contract_type);
    println!("Parties: {}", status.parties.join(", "));
    println!("Active: {}", status.is_active);
    println!("Complete: {}", status.is_complete);
    println!("Actions: {}", status.action_count);
    
    println!("\nCurrent State:");
    for (part, state) in &status.current_state {
        println!("  {}: {}", part, state);
    }
    
    Ok(())
}

fn show_actions(contract_path: &PathBuf, agent: &str) -> Result<()> {
    use modality_lang::agent::Contract;
    
    let json = std::fs::read_to_string(contract_path)?;
    let contract = Contract::from_json(&json)?;
    
    let actions = contract.what_can_i_do(agent);
    
    if actions.is_empty() {
        println!("No actions available for '{}' right now.", agent);
        println!("\nThis could mean:");
        println!("  - It's another party's turn");
        println!("  - The contract is complete");
        println!("  - The contract has terminated");
    } else {
        println!("Available actions for '{}':\n", agent);
        for action in actions {
            println!("  {} - {}", action.name, action.description);
            if action.requires_signature {
                println!("    (requires your signature)");
            }
        }
        println!("\nUse: modality contract act --contract {} --agent {} --action <name>", 
            contract_path.display(), agent);
    }
    
    Ok(())
}

fn commit_action(contract_path: &PathBuf, agent: &str, action: &str, output: Option<&PathBuf>) -> Result<()> {
    use modality_lang::agent::Contract;
    
    let json = std::fs::read_to_string(contract_path)?;
    let mut contract = Contract::from_json(&json)?;
    
    match contract.act(agent, action) {
        Ok(result) => {
            println!("✓ {}", result.message);
            println!("  Sequence: {}", result.sequence);
            println!("  New state: {}", result.new_state);
            
            let updated_json = contract.to_json()?;
            let out_path = output.unwrap_or(contract_path);
            std::fs::write(out_path, &updated_json)?;
            println!("\nContract updated: {}", out_path.display());
        }
        Err(e) => {
            eprintln!("✗ Failed: {}", e);
            eprintln!("\nAvailable actions:");
            let actions = contract.what_can_i_do(agent);
            for a in actions {
                eprintln!("  - {}", a.name);
            }
            return Err(anyhow::anyhow!("Action failed"));
        }
    }
    
    Ok(())
}

fn show_history(contract_path: &PathBuf) -> Result<()> {
    use modality_lang::agent::Contract;
    
    let json = std::fs::read_to_string(contract_path)?;
    let contract = Contract::from_json(&json)?;
    
    let history = contract.history();
    
    if history.is_empty() {
        println!("No actions taken yet.");
    } else {
        println!("Contract History:\n");
        for entry in history {
            println!("  #{} | {} | by {} | {}",
                entry.sequence,
                entry.action,
                entry.by,
                format_timestamp(entry.timestamp)
            );
        }
    }
    
    Ok(())
}

fn show_log(dir: Option<&PathBuf>, limit: Option<usize>, output: &str) -> Result<()> {
    let dir = dir.cloned().unwrap_or_else(|| std::env::current_dir().unwrap());
    let store = ContractStore::open(&dir)?;
    
    // Get HEAD and walk backwards through commits
    let head = store.get_head()?;
    
    if head.is_none() {
        if output == "json" {
            println!("{{\"commits\": []}}");
        } else {
            println!("No commits yet.");
        }
        return Ok(());
    }
    
    let mut commits = Vec::new();
    let mut current = head;
    let mut count = 0;
    
    while let Some(commit_id) = current {
        if let Some(lim) = limit {
            if count >= lim {
                break;
            }
        }
        
        let commit = store.load_commit(&commit_id)?;
        commits.push((commit_id.clone(), commit.clone()));
        current = commit.head.parent.clone();
        count += 1;
    }
    
    if output == "json" {
        let json_commits: Vec<serde_json::Value> = commits.iter().map(|(id, commit)| {
            serde_json::json!({
                "id": id,
                "parent": commit.head.parent,
                "actions": commit.body.len(),
            })
        }).collect();
        
        println!("{}", serde_json::to_string_pretty(&serde_json::json!({
            "commits": json_commits
        }))?);
    } else {
        let config = store.load_config()?;
        println!("Contract: {}", config.contract_id);
        println!("Commits: {}\n", commits.len());
        
        for (id, commit) in &commits {
            let short_id = if id.len() > 12 { &id[..12] } else { id };
            
            println!("commit {} ({}...)", short_id, &id[..8.min(id.len())]);
            if let Some(parent) = &commit.head.parent {
                let short_parent = if parent.len() > 12 { &parent[..12] } else { parent };
                println!("Parent: {}...", short_parent);
            }
            
            // Show actions summary
            if !commit.body.is_empty() {
                println!("Actions:");
                for action in &commit.body {
                    let path = action.path.as_deref().unwrap_or("/");
                    println!("  {} {}", action.method, path);
                }
            }
            println!();
        }
    }
    
    Ok(())
}

fn format_timestamp(ts: u64) -> String {
    // Simple timestamp formatting
    use std::time::{UNIX_EPOCH, Duration};
    let d = UNIX_EPOCH + Duration::from_millis(ts);
    format!("{:?}", d)
}

fn parse_contract_file(file: &PathBuf) -> Result<()> {
    use modality_lang::lalrpop_parser::parse_contract_file as parse_contract;
    use modality_lang::ast::CommitStatement;
    
    println!("Parsing: {}\n", file.display());
    
    let contract = parse_contract(file)
        .map_err(|e| anyhow::anyhow!("Parse error: {}", e))?;
    
    println!("✓ Contract: {}", contract.name);
    println!("  Commits: {}\n", contract.commits.len());
    
    for (i, commit) in contract.commits.iter().enumerate() {
        println!("  Commit {}:", i);
        println!("    signed_by: {} \"{}\"", commit.signed_by, commit.signature);
        if commit.model.is_some() {
            println!("    model: (provided)");
        }
        for stmt in &commit.statements {
            match stmt {
                CommitStatement::AddRule(_) => {
                    println!("    add_rule: {{ <formula> }}");
                }
                CommitStatement::Do(properties) => {
                    let props_str: Vec<String> = properties.iter()
                        .map(|p| format!("{}{}", if p.sign == modality_lang::ast::PropertySign::Plus { "+" } else { "-" }, p.name))
                        .collect();
                    println!("    do: {}", props_str.join(" "));
                }
                _ => {}
            }
        }
        println!();
    }
    
    println!("✓ Contract is valid.");
    
    Ok(())
}

fn verify_contract(model_path: &PathBuf, formula: Option<&str>) -> Result<()> {
    use modality_lang::{parse_file_lalrpop, ModelChecker};
    
    // Parse the model
    let model = parse_file_lalrpop(model_path.to_str().unwrap())
        .map_err(|e| anyhow::anyhow!("Parse error: {}", e))?;
    println!("Model: {}", model.name);
    println!("Parts: {}", model.parts.len());
    
    for part in &model.parts {
        println!("  {} ({} transitions)", part.name, part.transitions.len());
    }
    
    if let Some(_formula_str) = formula {
        // TODO: Parse and check formula
        println!("\nFormula verification not yet implemented in CLI");
        println!("Use the Rust API for formula checking");
    } else {
        println!("\nModel structure verified.");
        println!("Use --formula to check a specific property.");
    }
    
    Ok(())
}
