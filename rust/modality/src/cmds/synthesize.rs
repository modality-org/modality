use anyhow::Result;
use clap::Parser;

/// Synthesize a model from a template or pattern
#[derive(Parser, Debug)]
pub struct Opts {
    /// Template name: escrow, handshake, mutual_cooperation, etc.
    #[arg(short, long)]
    pub template: Option<String>,
    
    /// Natural language description of the contract
    #[arg(short, long)]
    pub describe: Option<String>,
    
    /// First party/signer name
    #[arg(long, default_value = "Alice")]
    pub party_a: String,
    
    /// Second party/signer name
    #[arg(long, default_value = "Bob")]
    pub party_b: String,
    
    /// Milestones for milestone template (comma-separated)
    #[arg(long)]
    pub milestones: Option<String>,
    
    /// Output format: modality (default) or json
    #[arg(short, long, default_value = "modality")]
    pub format: String,
    
    /// List available templates
    #[arg(short, long)]
    pub list: bool,
}

pub async fn run(opts: &Opts) -> Result<()> {
    if opts.list {
        println!("Available templates:\n");
        println!("  escrow              Two-party escrow with deposit/deliver/release");
        println!("  handshake           Mutual agreement requiring both signatures");
        println!("  mutual_cooperation  Cooperation game - both must cooperate, defection blocked");
        println!("  atomic_swap         Both parties commit before either can claim");
        println!("  multisig            N-of-M signature approval pattern");
        println!("  service_agreement   Offer → Accept → Deliver → Confirm → Pay");
        println!("  delegation          Principal grants agent authority to act");
        println!("  auction             Seller lists, bidders bid, highest wins");
        println!("  subscription        Recurring payment for service access");
        println!("  milestone           Multi-phase project with payments");
        println!("\nUsage:");
        println!("  modality model synthesize --template escrow --party-a Buyer --party-b Seller");
        println!("\nOr describe in natural language:");
        println!("  modality model synthesize --describe \"escrow where buyer deposits funds\"");
        return Ok(());
    }

    // Handle natural language description
    if let Some(description) = &opts.describe {
        let result = modality_lang::nl_mapper::map_nl_to_pattern(description);
        
        println!("Detected pattern: {} (confidence: {:.0}%)", 
            result.pattern.name(), 
            result.confidence * 100.0);
        println!("Parties: {:?}\n", result.parties);
        
        if !result.suggestions.is_empty() {
            for suggestion in &result.suggestions {
                println!("💡 {}", suggestion);
            }
            println!();
        }
        
        if let Some(model) = result.model {
            match opts.format.as_str() {
                "modality" => {
                    let output = modality_lang::print_model(&model);
                    println!("{}", output);
                }
                "json" => {
                    let json = serde_json::to_string_pretty(&model)?;
                    println!("{}", json);
                }
                _ => {}
            }
        } else {
            println!("Could not generate model. Try using --template with one of the listed templates.");
        }
        
        return Ok(());
    }

    let template = opts.template.as_ref()
        .ok_or_else(|| anyhow::anyhow!("Please specify --template, --describe, or use --list to see options"))?;

    let model = match template.as_str() {
        "escrow" => modality_lang::synthesis::templates::escrow(&opts.party_a, &opts.party_b),
        "handshake" => modality_lang::synthesis::templates::handshake(&opts.party_a, &opts.party_b),
        "mutual_cooperation" => modality_lang::synthesis::templates::mutual_cooperation(&opts.party_a, &opts.party_b),
        "atomic_swap" => modality_lang::synthesis::templates::atomic_swap(&opts.party_a, &opts.party_b),
        "multisig" => modality_lang::synthesis::templates::multisig(&[&opts.party_a, &opts.party_b], 2),
        "service_agreement" => modality_lang::synthesis::templates::service_agreement(&opts.party_a, &opts.party_b),
        "delegation" => modality_lang::synthesis::templates::delegation(&opts.party_a, &opts.party_b),
        "auction" => modality_lang::synthesis::templates::auction(&opts.party_a),
        "subscription" => modality_lang::synthesis::templates::subscription(&opts.party_a, &opts.party_b),
        "milestone" => {
            let milestones: Vec<&str> = opts.milestones
                .as_ref()
                .map(|m| m.split(',').map(|s| s.trim()).collect())
                .unwrap_or_else(|| vec!["Phase1", "Phase2", "Phase3"]);
            modality_lang::synthesis::templates::milestone(&opts.party_a, &opts.party_b, &milestones)
        }
        other => return Err(anyhow::anyhow!("Unknown template: '{}'. Use --list to see available templates.", other)),
    };

    match opts.format.as_str() {
        "modality" => {
            let output = modality_lang::print_model(&model);
            println!("{}", output);
        }
        "json" => {
            let json = serde_json::to_string_pretty(&model)?;
            println!("{}", json);
        }
        other => return Err(anyhow::anyhow!("Unknown format: '{}'. Use 'modality' or 'json'.", other)),
    }

    Ok(())
}
