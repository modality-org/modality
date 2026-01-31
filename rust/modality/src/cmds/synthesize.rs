use anyhow::Result;
use clap::Parser;

/// Synthesize a model from a template or pattern
#[derive(Parser, Debug)]
pub struct Opts {
    /// Template name: escrow, handshake, mutual_cooperation, trade
    #[arg(short, long)]
    pub template: Option<String>,
    
    /// First party/signer name
    #[arg(long, default_value = "Alice")]
    pub party_a: String,
    
    /// Second party/signer name
    #[arg(long, default_value = "Bob")]
    pub party_b: String,
    
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
        println!("\nUsage:");
        println!("  modality synthesize --template escrow --party-a Buyer --party-b Seller");
        return Ok(());
    }

    let template = opts.template.as_ref()
        .ok_or_else(|| anyhow::anyhow!("Please specify --template or use --list to see options"))?;

    let model = match template.as_str() {
        "escrow" => modality_lang::synthesis::templates::escrow(&opts.party_a, &opts.party_b),
        "handshake" => modality_lang::synthesis::templates::handshake(&opts.party_a, &opts.party_b),
        "mutual_cooperation" => modality_lang::synthesis::templates::mutual_cooperation(&opts.party_a, &opts.party_b),
        "atomic_swap" => modality_lang::synthesis::templates::atomic_swap(&opts.party_a, &opts.party_b),
        "multisig" => modality_lang::synthesis::templates::multisig(&[&opts.party_a, &opts.party_b], 2),
        "service_agreement" => modality_lang::synthesis::templates::service_agreement(&opts.party_a, &opts.party_b),
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
