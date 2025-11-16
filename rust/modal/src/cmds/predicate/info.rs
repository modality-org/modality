use anyhow::{Result, bail};
use clap::Args;
use colored::Colorize;

#[derive(Args, Debug)]
pub struct Opts {
    /// Predicate name (e.g., amount_in_range)
    #[arg(value_name = "NAME")]
    name: String,

    /// Contract ID (defaults to modal.money for network predicates)
    #[arg(long, default_value = "modal.money")]
    contract_id: String,
}

pub async fn run(opts: &Opts) -> Result<()> {
    println!("\n📖 Predicate Information: {}\n", opts.name.cyan().bold());
    println!("{}", "━".repeat(80));

    match opts.name.as_str() {
        "signed_by" => print_signed_by(),
        "amount_in_range" => print_amount_in_range(),
        "has_property" => print_has_property(),
        "timestamp_valid" => print_timestamp_valid(),
        "post_to_path" => print_post_to_path(),
        _ => {
            println!("\n❌ Unknown predicate: {}\n", opts.name.red());
            println!("Available predicates:");
            println!("  - signed_by");
            println!("  - amount_in_range");
            println!("  - has_property");
            println!("  - timestamp_valid");
            println!("  - post_to_path\n");
            bail!("Unknown predicate");
        }
    }

    println!("\n{}", "━".repeat(80));
    println!("\n💡 Test this predicate:");
    println!("   modal predicate test {}\n", opts.name);

    Ok(())
}

fn print_signed_by() {
    println!("\n{}: signed_by", "Name".bold());
    println!("{}: /_code/modal/signed_by.wasm", "Path".bold());
    println!("\n{}:", "Description".bold());
    println!("  Verify cryptographic signatures using public key cryptography");
    println!("\n{}:", "Arguments".bold());
    println!("  message: string - The message that was signed");
    println!("  signature: string - The signature to verify");
    println!("  public_key: string - The public key to verify against");
    println!("\n{}: 100-200", "Gas Usage".bold());
    println!("\n{}:", "Example".bold());
    println!("  +signed_by({{\"message\": \"hello\", \"signature\": \"sig123\", \"public_key\": \"pk456\"}})");
}

fn print_amount_in_range() {
    println!("\n{}: amount_in_range", "Name".bold());
    println!("{}: /_code/modal/amount_in_range.wasm", "Path".bold());
    println!("\n{}:", "Description".bold());
    println!("  Check if a numeric value is within specified bounds");
    println!("\n{}:", "Arguments".bold());
    println!("  amount: number - The value to check");
    println!("  min: number - Minimum allowed value (inclusive)");
    println!("  max: number - Maximum allowed value (inclusive)");
    println!("\n{}: 20-30", "Gas Usage".bold());
    println!("\n{}:", "Example".bold());
    println!("  +amount_in_range({{\"amount\": 100, \"min\": 0, \"max\": 1000}})");
}

fn print_has_property() {
    println!("\n{}: has_property", "Name".bold());
    println!("{}: /_code/modal/has_property.wasm", "Path".bold());
    println!("\n{}:", "Description".bold());
    println!("  Check if a JSON object has a specific property");
    println!("\n{}:", "Arguments".bold());
    println!("  path: string - JSON path (dot notation)");
    println!("  required: boolean - Whether the property must exist");
    println!("\n{}: 30-50", "Gas Usage".bold());
    println!("\n{}:", "Example".bold());
    println!("  +has_property({{\"path\": \"user.email\", \"required\": true}})");
}

fn print_timestamp_valid() {
    println!("\n{}: timestamp_valid", "Name".bold());
    println!("{}: /_code/modal/timestamp_valid.wasm", "Path".bold());
    println!("\n{}:", "Description".bold());
    println!("  Validate timestamps against age constraints");
    println!("\n{}:", "Arguments".bold());
    println!("  timestamp: number - Unix timestamp to validate");
    println!("  max_age_seconds: number (optional) - Maximum age in seconds");
    println!("\n{}: 25-35", "Gas Usage".bold());
    println!("\n{}:", "Example".bold());
    println!("  +timestamp_valid({{\"timestamp\": 1234567890, \"max_age_seconds\": 3600}})");
}

fn print_post_to_path() {
    println!("\n{}: post_to_path", "Name".bold());
    println!("{}: /_code/modal/post_to_path.wasm", "Path".bold());
    println!("\n{}:", "Description".bold());
    println!("  Verify that a commit includes a POST action to a specific path");
    println!("\n{}:", "Arguments".bold());
    println!("  path: string - The path to check for");
    println!("\n{}: 40-100", "Gas Usage".bold());
    println!("\n{}:", "Example".bold());
    println!("  +post_to_path({{\"path\": \"/_code/validator.wasm\"}})");
}

