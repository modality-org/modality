use anyhow::Result;
use clap::Args;
use colored::Colorize;
use serde_json::Value;

#[derive(Args, Debug)]
pub struct Opts {
    /// Predicate name
    #[arg(value_name = "NAME")]
    name: String,

    /// Arguments as JSON string
    #[arg(long)]
    args: String,

    /// Contract ID
    #[arg(long, default_value = "modal.money")]
    contract_id: String,

    /// Block height for context
    #[arg(long, default_value = "1")]
    block_height: u64,

    /// Timestamp for context
    #[arg(long)]
    timestamp: Option<u64>,
}

pub async fn run(opts: &Opts) -> Result<()> {
    println!("\n🧪 Testing Predicate: {}\n", opts.name.cyan().bold());
    println!("{}", "━".repeat(80));

    // Parse arguments
    let args: Value = serde_json::from_str(&opts.args)?;
    let timestamp = opts.timestamp.unwrap_or_else(|| {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    });

    println!("\n{}:", "Input".bold());
    println!("  Contract:     {}", opts.contract_id);
    println!("  Predicate:    {}", opts.name);
    println!("  Arguments:    {}", serde_json::to_string_pretty(&args)?);
    println!("  Block Height: {}", opts.block_height);
    println!("  Timestamp:    {}", timestamp);

    println!("\n⚠️  Note: Actual predicate execution requires a running node");
    println!("This command simulates the result based on the predicate logic.\n");

    // Simulate result
    let (valid, gas_used) = simulate_predicate(&opts.name, &args);

    println!("{}", "━".repeat(80));
    println!("\n{}:", "Simulated Result".bold());
    if valid {
        println!("  Valid:        {}", "✅ true".green());
    } else {
        println!("  Valid:        {}", "❌ false".red());
    }
    println!("  Gas Used:     {}", gas_used);

    let proposition_str = if valid {
        format!("+{}", opts.name)
    } else {
        format!("-{}", opts.name)
    };
    let colored_proposition = if valid {
        proposition_str.green()
    } else {
        proposition_str.red()
    };
    println!("\n  Proposition:  {}", colored_proposition.bold());

    println!("\n{}", "━".repeat(80));
    println!("\n💡 To use in a modal formula:");
    println!("   <{}> true\n", proposition_str);

    Ok(())
}

fn simulate_predicate(name: &str, args: &Value) -> (bool, u64) {
    match name {
        "amount_in_range" => {
            if let (Some(amount), Some(min), Some(max)) = (
                args.get("amount").and_then(|v| v.as_f64()),
                args.get("min").and_then(|v| v.as_f64()),
                args.get("max").and_then(|v| v.as_f64()),
            ) {
                (amount >= min && amount <= max, 25)
            } else {
                (false, 25)
            }
        }
        "has_property" => {
            let has_path = args.get("path").is_some();
            (has_path, 35)
        }
        "timestamp_valid" => {
            let has_timestamp = args.get("timestamp").is_some();
            (has_timestamp, 30)
        }
        "signed_by" => {
            let has_all = args.get("message").is_some()
                && args.get("signature").is_some()
                && args.get("public_key").is_some();
            (has_all, 150)
        }
        "post_to_path" => {
            let has_path = args.get("path").is_some();
            (has_path, 70)
        }
        _ => (false, 50),
    }
}

