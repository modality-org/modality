use anyhow::Result;
use clap::Args;
use colored::Colorize;

#[derive(Args, Debug)]
pub struct Opts {
    /// Program name
    #[arg(value_name = "NAME")]
    name: String,

    /// Contract ID (optional)
    #[arg(long)]
    contract_id: Option<String>,
}

pub async fn run(opts: &Opts) -> Result<()> {
    println!("\n📖 Program Information: {}\n", opts.name.cyan().bold());
    println!("{}", "━".repeat(80));

    if let Some(contract_id) = &opts.contract_id {
        println!("\n{}: {}", "Contract".bold(), contract_id);
        println!("{}: /__programs__/{}.wasm", "Path".bold(), opts.name);
        println!("\n⚠️  Fetching program details from datastore requires node access.");
        println!("    This feature will be implemented when integrated with a running node.\n");
    } else {
        println!("\n{}: General Program Information", "Topic".bold());
        println!("\n{}", "What are WASM Programs?".bold());
        println!("Programs are executable WASM modules that:");
        println!("  • Accept input arguments");
        println!("  • Perform computation");
        println!("  • Produce commit actions (post, create, send, etc.)");
        println!("  • Are executed by validators during consensus\n");

        println!("{}", "Key Differences from Predicates:".bold());
        println!("  Predicates → Evaluate to true/false (used in formulas)");
        println!("  Programs   → Produce commit actions (create state changes)\n");

        println!("{}", "Program Interface:".bold());
        println!("  Input:  {{{{ args: {{...}}, context: {{...}} }}}}");
        println!("  Output: {{{{ actions: [...], gas_used: N, errors: [] }}}}\n");

        println!("{}", "Execution Flow:".bold());
        println!("  1. User signs 'invoke' action with program path and args");
        println!("  2. Validators receive and validate signature");
        println!("  3. Validators execute program deterministically");
        println!("  4. Program returns actions");
        println!("  5. Actions are merged into commit");
        println!("  6. Validators process actions (creates state changes)\n");

        println!("{}", "Security Model:".bold());
        println!("  • User signature on invoke = indirect signature on results");
        println!("  • Execution is deterministic (all validators agree)");
        println!("  • Program code is in contract (verifiable)");
        println!("  • Gas metering prevents infinite loops\n");
    }

    println!("{}", "━".repeat(80));
    println!("\n💡 Create a program:");
    println!("   modal program create --dir ./my-program --name {}\n", opts.name);
    
    Ok(())
}

