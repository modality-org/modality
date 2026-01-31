use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Agent-friendly contract operations
#[derive(Parser, Debug)]
pub struct Opts {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Create a new empty contract
    Create {
        /// Optional template: escrow, handshake, service, swap, delegation, etc.
        #[arg(short = 't', long)]
        template: Option<String>,
        
        /// Output file (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
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
    
    /// Show contract history
    History {
        /// Contract JSON file
        #[arg(short, long)]
        contract: PathBuf,
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
        Command::Create { template, output } => {
            create_contract(template.as_deref(), output.as_ref())
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
        Command::Verify { model, formula } => {
            verify_contract(model, formula.as_deref())
        }
        Command::Parse { file } => {
            parse_contract_file(file)
        }
    }
}

fn create_contract(template: Option<&str>, output: Option<&PathBuf>) -> Result<()> {
    use modality_lang::agent::Contract;
    
    let contract = match template {
        None => Contract::empty(),
        Some("escrow") => Contract::escrow("PartyA", "PartyB"),
        Some("handshake") => Contract::handshake("PartyA", "PartyB"),
        Some("service") | Some("service_agreement") => Contract::service_agreement("Provider", "Consumer"),
        Some("swap") | Some("atomic_swap") => Contract::atomic_swap("PartyA", "PartyB"),
        Some("cooperation") | Some("mutual_cooperation") => Contract::mutual_cooperation("PartyA", "PartyB"),
        Some("delegation") => Contract::delegation("Principal", "Agent"),
        Some("auction") => Contract::auction("Seller"),
        Some("subscription") => Contract::subscription("Provider", "Subscriber"),
        Some(other) => return Err(anyhow::anyhow!(
            "Unknown template: '{}'. Options: escrow, handshake, service, swap, cooperation, delegation, auction, subscription\n\
            Or omit --template to create an empty contract.", other
        )),
    };
    
    let json = contract.to_json()?;
    
    if let Some(path) = output {
        std::fs::write(path, &json)?;
        println!("Contract created: {}", path.display());
    } else {
        println!("{}", json);
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
