use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

/// Synthesize a model from a template, pattern, or rule
#[derive(Parser, Debug)]
pub struct Opts {
    /// Template name: escrow, handshake, mutual_cooperation, etc.
    #[arg(short, long)]
    pub template: Option<String>,

    /// Natural language description of the contract
    #[arg(short, long)]
    pub describe: Option<String>,

    /// Synthesize from a rule file containing formulas
    #[arg(short, long)]
    pub rule: Option<PathBuf>,

    /// Inline formulas (semicolon-separated)
    #[arg(long)]
    pub formulas: Option<String>,

    /// Generate LLM prompt for NL → Formulas (Step 1)
    #[arg(long)]
    pub generate_prompt: bool,

    /// LLM response containing generated formulas
    #[arg(long)]
    pub llm_response: Option<String>,

    /// File containing an LLM response with generated formulas
    #[arg(long)]
    pub llm_response_file: Option<PathBuf>,

    /// Output file path
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Verify parser-backed synthesized models against their input formulas
    #[arg(long)]
    pub verify: bool,

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
    let llm_response =
        load_llm_response(opts.llm_response.as_ref(), opts.llm_response_file.as_ref())?;

    // Step 1a: Generate LLM prompt for NL → Formulas
    if opts.generate_prompt {
        if let Some(description) = &opts.describe {
            println!("📝 LLM Prompt for Rule Generation (Step 1)\n");
            println!("{}", "=".repeat(60));
            println!(
                "{}",
                modality_lang::llm_synthesis::generate_prompt(description)
            );
            println!("{}", "=".repeat(60));
            println!(
                "\n💡 Send this prompt to Claude/GPT, then use --llm-response or --llm-response-file with the output"
            );
            return Ok(());
        } else {
            return Err(anyhow::anyhow!("--generate-prompt requires --describe"));
        }
    }

    // Step 1b + 2: Parse LLM response and synthesize
    if let Some(llm_response) = &llm_response {
        println!("🔧 Two-Step Pipeline: LLM Response → Model\n");

        // Parse formulas from LLM response
        let formulas = modality_lang::llm_synthesis::parse_llm_response(llm_response);

        if formulas.is_empty() {
            return Err(anyhow::anyhow!("No formulas found in LLM response"));
        }

        println!("📋 Extracted formulas:");
        for (i, f) in formulas.iter().enumerate() {
            println!("  F{}: {}", i + 1, f);
        }
        println!();

        let parsed_input = parse_formula_inputs(&formulas);
        if opts.verify {
            parsed_input.ensure_all_parsed()?;
        }
        let model = if parsed_input.formulas.is_empty() {
            if opts.verify {
                return Err(anyhow::anyhow!(
                    "--verify requires formulas that can be parsed by the Modality parser"
                ));
            }

            println!("⚠️  Could not parse formulas; using legacy string heuristics\n");
            let constraints = synthesize_constraints_from_strings(&formulas);

            println!("📊 Extracted constraints:");
            println!("  Actions: {:?}", constraints.actions);
            println!("  Ordering: {:?}", constraints.ordering);
            println!("  Authorization: {:?}", constraints.authorization);
            println!();

            modality_lang::formula_synthesis::synthesize_from_constraints("Contract", &constraints)
        } else {
            if !opts.verify {
                parsed_input.warn_unparsed();
            }
            println!(
                "📊 Parsed {} formula(s) with the Modality parser\n",
                parsed_input.formulas.len()
            );
            modality_lang::formula_synthesis::synthesize_from_formulas(
                "Contract",
                &parsed_input.formulas,
            )
        };

        if opts.verify {
            verify_synthesized_model(&model, &parsed_input.formulas)?;
            println!();
        }

        println!("✅ Synthesized model:\n");
        let output = format_synthesized_model(&model, &opts.format)?;
        println!("{}", output);

        if let Some(output_path) = &opts.output {
            if let Some(parent) = output_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(output_path, &output)?;
            println!("\n📁 Written to {}", output_path.display());
        }

        return Ok(());
    }

    if opts.list {
        println!("Available templates:\n");
        println!("  escrow              Two-party escrow with deposit/deliver/release");
        println!("  handshake           Mutual agreement requiring both signatures");
        println!("  mutual_cooperation  Cooperation game - both must cooperate, defection blocked");
        println!("  atomic_swap         Both parties commit before either can claim");
        println!("  multisig            N-of-M signature approval pattern");
        println!("  turn_taking         Alternating two-party turn cycle");
        println!("  service_agreement   Offer → Accept → Deliver → Confirm → Pay");
        println!("  delegation          Principal grants agent authority to act");
        println!("  auction             Seller lists, bidders bid, highest wins");
        println!("  subscription        Recurring payment for service access");
        println!("  milestone           Multi-phase project with payments");
        println!("\nUsage:");
        println!("  modality model synthesize --template escrow --party-a Buyer --party-b Seller");
        println!("\nOr describe in natural language:");
        println!("  modality model synthesize --describe \"escrow where buyer deposits funds\"");
        println!("  modality model synthesize --describe \"Alice and Bob take turns signing\"");
        println!("\nOr synthesize and verify from formulas:");
        println!("  modality model synthesize --formulas \"always([<+APPROVE>] true)\" --verify");
        println!(
            "  modality model synthesize --formulas \"always([<+APPROVE>] true & [<+REJECT>] true)\" --verify"
        );
        println!("  modality model synthesize --formulas \"[<+APPROVE>] true\" --verify");
        println!("  modality model synthesize --formulas \"<+APPROVE> true\" --verify");
        println!(
            "  modality model synthesize --formulas \"<+APPROVE> true | <+REJECT> true\" --verify"
        );
        println!(
            "  modality model synthesize --formulas \"<+CANCEL> true & ([+RELEASE] true -> eventually(<+DELIVER> true))\" --verify"
        );
        println!(
            "  modality model synthesize --formulas \"[<+RELEASE>] true -> eventually(<+DELIVER> true)\" --verify"
        );
        println!(
            "  modality model synthesize --formulas \"[<+RELEASE>] true -> eventually([<+DELIVER>] true)\" --verify"
        );
        println!(
            "  modality model synthesize --formulas \"[+RELEASE] true -> eventually([<+DELIVER>] true)\" --verify"
        );
        println!(
            "  modality model synthesize --formulas \"[+RELEASE] true -> (eventually(<+DEPOSIT> true) & eventually(<+DELIVER> true))\" --verify"
        );
        println!(
            "  modality model synthesize --formulas \"[<+RELEASE>] true -> (eventually(<+DEPOSIT> true) & eventually(<+DELIVER> true))\" --verify"
        );
        println!(
            "  modality model synthesize --formulas \"[+RELEASE] true -> (eventually([<+DEPOSIT>] true) & eventually([<+DELIVER>] true))\" --verify"
        );
        println!(
            "  modality model synthesize --formulas \"[<+RELEASE>] true -> (eventually([<+DEPOSIT>] true) & eventually([<+DELIVER>] true))\" --verify"
        );
        println!(
            "  modality model synthesize --formulas \"<+CANCEL> true & ([+DISPUTE] true -> always([-RELEASE] true))\" --verify"
        );
        println!(
            "  modality model synthesize --formulas \"[+DISPUTE] true -> (always([-RELEASE] true) & always([-REFUND] true))\" --verify"
        );
        println!(
            "  modality model synthesize --formulas \"[<+DISPUTE>] true -> always([-RELEASE] true)\" --verify"
        );
        println!(
            "  modality model synthesize --formulas \"[<+DISPUTE>] true -> (always([-RELEASE] true) & always([-REFUND] true))\" --verify"
        );
        println!(
            "  modality model synthesize --formulas \"[+DISPUTE] true -> (<+signed_by(/users/arbiter.id)> true & always([-RELEASE] true))\" --verify"
        );
        println!(
            "  modality model synthesize --formulas \"[+DISPUTE] true -> (<+signed_by(/users/alice.id) +signed_by(/users/bob.id)> true & always([-RELEASE] true))\" --verify"
        );
        println!(
            "  modality model synthesize --formulas \"[+DISPUTE] true -> (<+signed_by(/users/arbiter.id)> true & (always([-RELEASE] true) & always([-REFUND] true)))\" --verify"
        );
        println!(
            "  modality model synthesize --formulas \"[+DISPUTE] true -> (<+signed_by(/users/alice.id) +signed_by(/users/bob.id)> true & (always([-RELEASE] true) & always([-REFUND] true)))\" --verify"
        );
        println!(
            "  modality model synthesize --formulas \"[+DISPUTE] true -> ([<+signed_by(/users/arbiter.id)>] true & always([-RELEASE] true))\" --verify"
        );
        println!(
            "  modality model synthesize --formulas \"[+DISPUTE] true -> ([<+signed_by(/users/alice.id) +signed_by(/users/bob.id)>] true & always([-RELEASE] true))\" --verify"
        );
        println!(
            "  modality model synthesize --formulas \"[+DISPUTE] true -> ([<+signed_by(/users/arbiter.id)>] true & (always([-RELEASE] true) & always([-REFUND] true)))\" --verify"
        );
        println!(
            "  modality model synthesize --formulas \"[+DISPUTE] true -> ([<+signed_by(/users/alice.id) +signed_by(/users/bob.id)>] true & (always([-RELEASE] true) & always([-REFUND] true)))\" --verify"
        );
        println!(
            "  modality model synthesize --formulas \"[<+DISPUTE>] true -> (<+signed_by(/users/arbiter.id)> true & always([-RELEASE] true))\" --verify"
        );
        println!(
            "  modality model synthesize --formulas \"[<+DISPUTE>] true -> (<+signed_by(/users/alice.id) +signed_by(/users/bob.id)> true & always([-RELEASE] true))\" --verify"
        );
        println!(
            "  modality model synthesize --formulas \"[<+DISPUTE>] true -> (<+signed_by(/users/arbiter.id)> true & (always([-RELEASE] true) & always([-REFUND] true)))\" --verify"
        );
        println!(
            "  modality model synthesize --formulas \"[<+DISPUTE>] true -> (<+signed_by(/users/alice.id) +signed_by(/users/bob.id)> true & (always([-RELEASE] true) & always([-REFUND] true)))\" --verify"
        );
        println!(
            "  modality model synthesize --formulas \"[<+DISPUTE>] true -> ([<+signed_by(/users/arbiter.id)>] true & always([-RELEASE] true))\" --verify"
        );
        println!(
            "  modality model synthesize --formulas \"[<+DISPUTE>] true -> ([<+signed_by(/users/alice.id) +signed_by(/users/bob.id)>] true & always([-RELEASE] true))\" --verify"
        );
        println!(
            "  modality model synthesize --formulas \"[<+DISPUTE>] true -> ([<+signed_by(/users/arbiter.id)>] true & (always([-RELEASE] true) & always([-REFUND] true)))\" --verify"
        );
        println!(
            "  modality model synthesize --formulas \"[<+DISPUTE>] true -> ([<+signed_by(/users/alice.id) +signed_by(/users/bob.id)>] true & (always([-RELEASE] true) & always([-REFUND] true)))\" --verify"
        );
        println!(
            "  modality model synthesize --formulas \"[<+RELEASE>] true -> <+signed_by(/users/buyer.id)> true\" --verify"
        );
        println!(
            "  modality model synthesize --formulas \"[<+RELEASE>] true -> [<+signed_by(/users/buyer.id)>] true\" --verify"
        );
        println!(
            "  modality model synthesize --formulas \"[<+RELEASE>] true -> (<+signed_by(/users/buyer.id)> true & eventually([<+DELIVER>] true))\" --verify"
        );
        println!(
            "  modality model synthesize --formulas \"[<+RELEASE>] true -> (<+signed_by(/users/buyer.id)> true & eventually(<+DELIVER> true))\" --verify"
        );
        println!(
            "  modality model synthesize --formulas \"[<+RELEASE>] true -> ([<+signed_by(/users/buyer.id)>] true & eventually([<+DELIVER>] true))\" --verify"
        );
        println!(
            "  modality model synthesize --formulas \"[<+RELEASE>] true -> ([<+signed_by(/users/buyer.id)>] true & eventually(<+DELIVER> true))\" --verify"
        );
        println!(
            "  modality model synthesize --formulas \"[<+APPROVE>] true -> <+signed_by(/users/alice.id) +signed_by(/users/bob.id)> true\" --verify"
        );
        println!(
            "  modality model synthesize --formulas \"[<+APPROVE>] true -> [<+signed_by(/users/alice.id) +signed_by(/users/bob.id)>] true\" --verify"
        );
        println!(
            "  modality model synthesize --formulas \"[<+APPROVE>] true -> (<+signed_by(/users/alice.id) +signed_by(/users/bob.id)> true & eventually([<+DELIVER>] true))\" --verify"
        );
        println!(
            "  modality model synthesize --formulas \"[<+APPROVE>] true -> (<+signed_by(/users/alice.id) +signed_by(/users/bob.id)> true & eventually(<+DELIVER> true))\" --verify"
        );
        println!(
            "  modality model synthesize --formulas \"[<+APPROVE>] true -> ([<+signed_by(/users/alice.id) +signed_by(/users/bob.id)>] true & eventually([<+DELIVER>] true))\" --verify"
        );
        println!(
            "  modality model synthesize --formulas \"[<+APPROVE>] true -> ([<+signed_by(/users/alice.id) +signed_by(/users/bob.id)>] true & eventually(<+DELIVER> true))\" --verify"
        );
        println!(
            "  modality model synthesize --formulas \"[+APPROVE] true -> [<+signed_by(/users/reviewer.id)>] true\" --verify"
        );
        println!(
            "  modality model synthesize --formulas \"[+RELEASE] true -> (<+signed_by(/users/buyer.id)> true & eventually([<+DELIVER>] true))\" --verify"
        );
        println!(
            "  modality model synthesize --formulas \"[+RELEASE] true -> (<+signed_by(/users/buyer.id)> true & eventually(<+DELIVER> true))\" --verify"
        );
        println!(
            "  modality model synthesize --formulas \"[+RELEASE] true -> ([<+signed_by(/users/buyer.id)>] true & eventually([<+DELIVER>] true))\" --verify"
        );
        println!(
            "  modality model synthesize --formulas \"[+RELEASE] true -> ([<+signed_by(/users/buyer.id)>] true & eventually(<+DELIVER> true))\" --verify"
        );
        println!(
            "  modality model synthesize --formulas \"[+APPROVE] true -> <+signed_by(/users/alice.id) +signed_by(/users/bob.id)> true\" --verify"
        );
        println!(
            "  modality model synthesize --formulas \"[+APPROVE] true -> (<+signed_by(/users/alice.id) +signed_by(/users/bob.id)> true & eventually([<+DELIVER>] true))\" --verify"
        );
        println!(
            "  modality model synthesize --formulas \"[+APPROVE] true -> (<+signed_by(/users/alice.id) +signed_by(/users/bob.id)> true & eventually(<+DELIVER> true))\" --verify"
        );
        println!(
            "  modality model synthesize --formulas \"[+APPROVE] true -> ([<+signed_by(/users/alice.id) +signed_by(/users/bob.id)>] true & eventually([<+DELIVER>] true))\" --verify"
        );
        println!(
            "  modality model synthesize --formulas \"[+APPROVE] true -> ([<+signed_by(/users/alice.id) +signed_by(/users/bob.id)>] true & eventually(<+DELIVER> true))\" --verify"
        );
        println!(
            "  modality model synthesize --formulas \"[+APPROVE] true -> [<+signed_by(/users/alice.id) +signed_by(/users/bob.id)>] true\" --verify"
        );
        println!("\nOr generate a prompt and synthesize an LLM response file:");
        println!(
            "  modality model synthesize --describe \"escrow where buyer deposits funds\" --generate-prompt"
        );
        println!("  modality model synthesize --llm-response-file response.md --verify");
        return Ok(());
    }

    // Handle formula-based synthesis (two-step pipeline)
    if let Some(formulas_str) = &opts.formulas {
        println!("🔧 Step 2: Model Synthesis (Formulas → Model)\n");

        // Parse formulas from semicolon-separated string
        let formula_strs: Vec<String> = formulas_str
            .split(';')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        println!("📋 Input formulas:");
        for (i, f) in formula_strs.iter().enumerate() {
            println!("  F{}: {}", i + 1, f);
        }
        println!();

        let parsed_input = parse_formula_inputs(&formula_strs);
        if opts.verify {
            parsed_input.ensure_all_parsed()?;
        }

        if parsed_input.formulas.is_empty() {
            return Err(anyhow::anyhow!("No valid formulas found"));
        }

        if !opts.verify {
            parsed_input.warn_unparsed();
        }

        // Extract constraints and synthesize
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "Contract",
            &parsed_input.formulas,
        );

        if opts.verify {
            verify_synthesized_model(&model, &parsed_input.formulas)?;
            println!();
        }

        println!("✅ Synthesized model:\n");
        let output = format_synthesized_model(&model, &opts.format)?;
        println!("{}", output);

        if let Some(output_path) = &opts.output {
            if let Some(parent) = output_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(output_path, &output)?;
            println!("\n📁 Written to {}", output_path.display());
        }

        return Ok(());
    }

    // Handle rule file-based synthesis
    if let Some(rule_path) = &opts.rule {
        let content = std::fs::read_to_string(rule_path)?;

        println!("🔧 Synthesizing from rule file: {}\n", rule_path.display());

        let parsed_input = parse_formula_inputs(std::slice::from_ref(&content));
        if opts.verify {
            parsed_input.ensure_all_parsed()?;
        }
        if !parsed_input.formulas.is_empty() {
            let model = modality_lang::formula_synthesis::synthesize_from_formulas(
                "Contract",
                &parsed_input.formulas,
            );

            if opts.verify {
                verify_synthesized_model(&model, &parsed_input.formulas)?;
                println!();
            }

            let output = format_synthesized_model(&model, &opts.format)?;

            if let Some(output_path) = &opts.output {
                if let Some(parent) = output_path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(output_path, &output)?;
                println!("✅ Synthesized model written to {}", output_path.display());
            } else {
                println!("{}", output);
            }
        } else {
            if opts.verify {
                return Err(anyhow::anyhow!(
                    "--verify requires formulas that can be parsed by the Modality parser"
                ));
            }

            // Fallback to old heuristic approach
            let model = synthesize_from_rule(&content, &opts.party_a, &opts.party_b)?;
            let output = format_model(&model, &opts.format)?;

            if let Some(output_path) = &opts.output {
                if let Some(parent) = output_path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(output_path, &output)?;
                println!("✅ Synthesized model written to {}", output_path.display());
            } else {
                println!("{}", output);
            }
        }

        return Ok(());
    }

    // Handle natural language description
    if let Some(description) = &opts.describe {
        if opts.verify {
            return Err(anyhow::anyhow!(
                "--verify requires --formulas, --rule, --llm-response, or --llm-response-file"
            ));
        }

        let result = modality_lang::nl_mapper::map_nl_to_pattern(description);

        println!(
            "Detected pattern: {} (confidence: {:.0}%)",
            result.pattern.name(),
            result.confidence * 100.0
        );
        println!("Parties: {:?}\n", result.parties);

        if !result.suggestions.is_empty() {
            for suggestion in &result.suggestions {
                println!("💡 {}", suggestion);
            }
            println!();
        }

        if let Some(model) = result.model {
            let output = format_synthesized_model(&model, &opts.format)?;
            write_or_print_model(&output, opts.output.as_ref())?;
        } else {
            println!(
                "Could not generate model. Try using --template with one of the listed templates."
            );
        }

        return Ok(());
    }

    let template = opts.template.as_ref().ok_or_else(|| {
        anyhow::anyhow!(
            "Please specify --template, --describe, --rule, or use --list to see options"
        )
    })?;

    if opts.verify {
        return Err(anyhow::anyhow!(
            "--verify requires --formulas, --rule, --llm-response, or --llm-response-file"
        ));
    }

    let model = match template.as_str() {
        "escrow" => modality_lang::synthesis::templates::escrow(&opts.party_a, &opts.party_b),
        "handshake" => modality_lang::synthesis::templates::handshake(&opts.party_a, &opts.party_b),
        "mutual_cooperation" => {
            modality_lang::synthesis::templates::mutual_cooperation(&opts.party_a, &opts.party_b)
        }
        "atomic_swap" => {
            modality_lang::synthesis::templates::atomic_swap(&opts.party_a, &opts.party_b)
        }
        "multisig" => {
            modality_lang::synthesis::templates::multisig(&[&opts.party_a, &opts.party_b], 2)
        }
        "turn_taking" | "alternating" => {
            let pattern = modality_lang::synthesis::RulePattern::Alternating {
                parties: vec![opts.party_a.clone(), opts.party_b.clone()],
            };
            match modality_lang::synthesis::synthesize_from_pattern("TurnTaking", &pattern) {
                modality_lang::synthesis::SynthesisResult::Success(model) => model,
                modality_lang::synthesis::SynthesisResult::Failure(reason) => {
                    return Err(anyhow::anyhow!(reason))
                }
                modality_lang::synthesis::SynthesisResult::NeedsAssistance { question, .. } => {
                    return Err(anyhow::anyhow!(question))
                }
            }
        }
        "service_agreement" => {
            modality_lang::synthesis::templates::service_agreement(&opts.party_a, &opts.party_b)
        }
        "delegation" => {
            modality_lang::synthesis::templates::delegation(&opts.party_a, &opts.party_b)
        }
        "auction" => modality_lang::synthesis::templates::auction(&opts.party_a),
        "subscription" => {
            modality_lang::synthesis::templates::subscription(&opts.party_a, &opts.party_b)
        }
        "milestone" => {
            let milestones: Vec<&str> = opts
                .milestones
                .as_ref()
                .map(|m| m.split(',').map(|s| s.trim()).collect())
                .unwrap_or_else(|| vec!["Phase1", "Phase2", "Phase3"]);
            modality_lang::synthesis::templates::milestone(
                &opts.party_a,
                &opts.party_b,
                &milestones,
            )
        }
        other => {
            return Err(anyhow::anyhow!(
                "Unknown template: '{}'. Use --list to see available templates.",
                other
            ))
        }
    };

    let output = format_synthesized_model(&model, &opts.format)?;
    write_or_print_model(&output, opts.output.as_ref())?;

    Ok(())
}

fn load_llm_response(
    response: Option<&String>,
    response_file: Option<&PathBuf>,
) -> Result<Option<String>> {
    match (response, response_file) {
        (Some(_), Some(_)) => Err(anyhow::anyhow!(
            "Use either --llm-response or --llm-response-file, not both"
        )),
        (Some(response), None) => Ok(Some(response.clone())),
        (None, Some(path)) => Ok(Some(std::fs::read_to_string(path)?)),
        (None, None) => Ok(None),
    }
}

struct ParsedFormulaInputs {
    formulas: Vec<modality_lang::FormulaExpr>,
    unparsed: Vec<String>,
}

impl ParsedFormulaInputs {
    fn ensure_all_parsed(&self) -> Result<()> {
        if self.unparsed.is_empty() {
            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "--verify requires every input formula to parse with the Modality parser; unparsed: {}",
                self.unparsed.join(", ")
            ))
        }
    }

    fn warn_unparsed(&self) {
        if !self.unparsed.is_empty() {
            println!(
                "⚠️  Skipping {} unparsed formula input(s): {}",
                self.unparsed.len(),
                self.unparsed.join(", ")
            );
            println!("   Use --verify to fail instead of continuing with a partial parse.\n");
        }
    }
}

fn parse_formula_inputs(formulas: &[String]) -> ParsedFormulaInputs {
    let mut parsed_expressions = Vec::new();
    let mut unparsed = Vec::new();

    for (index, formula) in formulas.iter().enumerate() {
        match parse_formula_string(index, formula) {
            Some(parsed) => {
                parsed_expressions.extend(parsed.into_iter().map(|formula| formula.expression));
            }
            None => {
                let label = format!("F{}", index + 1);
                let preview = formula_preview(formula);
                if preview.is_empty() {
                    unparsed.push(label);
                } else {
                    unparsed.push(format!("{} `{}`", label, preview));
                }
            }
        }
    }

    ParsedFormulaInputs {
        formulas: parsed_expressions,
        unparsed,
    }
}

#[cfg(test)]
fn parse_formula_strings(formulas: &[String]) -> Vec<modality_lang::FormulaExpr> {
    parse_formula_inputs(formulas).formulas
}

fn parse_formula_string(index: usize, formula: &str) -> Option<Vec<modality_lang::Formula>> {
    modality_lang::parse_all_formulas_content_lalrpop(formula)
        .ok()
        .filter(|parsed| !parsed.is_empty())
        .or_else(|| {
            let wrapped = format!("formula generated_{} {{\n{}\n}}", index + 1, formula);
            modality_lang::parse_all_formulas_content_lalrpop(&wrapped)
                .ok()
                .filter(|parsed| !parsed.is_empty())
        })
}

#[cfg(test)]
fn ensure_all_formula_strings_parsed(formulas: &[String]) -> Result<()> {
    parse_formula_inputs(formulas).ensure_all_parsed()
}

#[cfg(test)]
fn unparsed_formula_string_labels(formulas: &[String]) -> Vec<String> {
    parse_formula_inputs(formulas).unparsed
}

fn formula_preview(formula: &str) -> String {
    const MAX_PREVIEW_LEN: usize = 80;

    let preview = formula.lines().next().unwrap_or("").trim();
    let mut chars = preview.chars();
    let truncated: String = chars.by_ref().take(MAX_PREVIEW_LEN).collect();
    if chars.next().is_some() {
        format!("{}...", truncated)
    } else {
        truncated
    }
}

fn synthesize_constraints_from_strings(
    formulas: &[String],
) -> modality_lang::formula_synthesis::SynthesisConstraints {
    let mut constraints = modality_lang::formula_synthesis::SynthesisConstraints::default();

    for f in formulas {
        // Look for ordering: [+X] implies eventually(<+Y> true)
        if f.contains("implies") && f.contains("eventually") {
            if let Some(box_start) = f.find("[+") {
                let rest = &f[box_start + 2..];
                if let Some(box_end) = rest.find(']') {
                    let action = rest[..box_end].trim();
                    if let Some(ev_start) = f.find("<+") {
                        let ev_rest = &f[ev_start + 2..];
                        if let Some(ev_end) = ev_rest.find('>') {
                            let prereq = ev_rest[..ev_end].trim();
                            if prereq != "signed_by" && !prereq.starts_with("signed_by") {
                                constraints
                                    .ordering
                                    .push((action.to_string(), prereq.to_string()));
                                constraints.actions.insert(action.to_string());
                                constraints.actions.insert(prereq.to_string());
                            }
                        }
                    }
                }
            }
        }

        // Look for authorization: [+X] implies <+signed_by(path)> true
        if f.contains("signed_by") {
            if let Some(start) = f.find("signed_by(") {
                let rest = &f[start + 10..];
                if let Some(end) = rest.find(')') {
                    let signer = rest[..end].trim().to_string();
                    // Find which action this is for
                    if let Some(box_start) = f.find("[+") {
                        let box_rest = &f[box_start + 2..];
                        if let Some(box_end) = box_rest.find(']') {
                            let action = box_rest[..box_end].trim().to_string();
                            constraints.actions.insert(action.clone());
                            constraints
                                .authorization
                                .entry(action)
                                .or_insert_with(Vec::new)
                                .push(signer);
                        }
                    }
                }
            }
        }
    }

    constraints
}

/// Synthesize a model from a rule file content
fn synthesize_from_rule(
    content: &str,
    party_a: &str,
    party_b: &str,
) -> Result<modality_lang::Model> {
    // Extract formula from rule content
    // Look for patterns like: signed_by(/users/X.id)
    let mut signers = Vec::new();

    // Simple regex-like extraction for signed_by predicates
    for line in content.lines() {
        if line.contains("signed_by") {
            // Extract path from signed_by(/path)
            if let Some(start) = line.find("signed_by(") {
                let rest = &line[start + 10..];
                if let Some(end) = rest.find(')') {
                    let path = &rest[..end];
                    signers.push(path.to_string());
                }
            }
        }
    }

    // If we found signers, create a model with transitions for each
    if !signers.is_empty() {
        let mut model = modality_lang::Model::new("default".to_string());
        model.set_initial("idle".to_string());

        for signer in &signers {
            // idle -> idle with signed_by
            let mut t = modality_lang::Transition::new("idle".to_string(), "idle".to_string());
            t.add_property(modality_lang::Property::new_predicate_from_call(
                "signed_by".to_string(),
                signer.clone(),
            ));
            model.add_transition(t);
        }

        return Ok(model);
    }

    // Fallback: use party names if no signers found
    let mut model = modality_lang::Model::new("default".to_string());
    model.set_initial("idle".to_string());

    let mut t1 = modality_lang::Transition::new("idle".to_string(), "idle".to_string());
    t1.add_property(modality_lang::Property::new_predicate_from_call(
        "signed_by".to_string(),
        format!("/users/{}.id", party_a.to_lowercase()),
    ));
    model.add_transition(t1);

    let mut t2 = modality_lang::Transition::new("idle".to_string(), "idle".to_string());
    t2.add_property(modality_lang::Property::new_predicate_from_call(
        "signed_by".to_string(),
        format!("/users/{}.id", party_b.to_lowercase()),
    ));
    model.add_transition(t2);

    Ok(model)
}

fn format_synthesized_model(model: &modality_lang::Model, format: &str) -> Result<String> {
    match format {
        "modality" => Ok(modality_lang::print_model(model)),
        "json" => Ok(serde_json::to_string_pretty(model)?),
        other => Err(anyhow::anyhow!(
            "Unknown format: '{}'. Use 'modality' or 'json'.",
            other
        )),
    }
}

fn verify_synthesized_model(
    model: &modality_lang::Model,
    formulas: &[modality_lang::FormulaExpr],
) -> Result<()> {
    println!(
        "🔎 Verifying synthesized model against {} formula(s)",
        formulas.len()
    );

    let checker = modality_lang::ModelChecker::new(model.clone());
    let mut failed = Vec::new();

    for (index, expression) in formulas.iter().enumerate() {
        let formula_name = format!("F{}", index + 1);
        let formula = modality_lang::Formula::new(formula_name.clone(), expression.clone());
        let result = checker.check_formula(&formula);

        if result.is_satisfied {
            println!("  ✅ {} satisfied", formula_name);
        } else {
            println!("  ❌ {} not satisfied", formula_name);
            failed.push(formula_name);
        }
    }

    if failed.is_empty() {
        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "Synthesized model failed verification for {} formula(s): {}",
            failed.len(),
            failed.join(", ")
        ))
    }
}

fn write_or_print_model(output: &str, output_path: Option<&PathBuf>) -> Result<()> {
    if let Some(output_path) = output_path {
        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(output_path, output)?;
        println!("✅ Synthesized model written to {}", output_path.display());
    } else {
        println!("{}", output);
    }

    Ok(())
}

/// Format a model for output
fn format_model(model: &modality_lang::Model, format: &str) -> Result<String> {
    match format {
        "modality" => {
            // Generate export default model syntax
            let mut output = String::new();
            output.push_str("export default model {\n");

            if let Some(initial) = &model.initial {
                output.push_str(&format!("  initial {}\n", initial));
            }
            output.push('\n');

            for transition in &model.transitions {
                let props: Vec<String> = transition
                    .properties
                    .iter()
                    .map(|p| {
                        let sign = if p.sign == modality_lang::PropertySign::Plus {
                            "+"
                        } else {
                            "-"
                        };
                        if let Some(source) = &p.source {
                            if let modality_lang::PropertySource::Predicate { args, .. } = source {
                                if let Some(arg) = args.get("arg") {
                                    return format!(
                                        "{}{}({})",
                                        sign,
                                        p.name,
                                        arg.as_str().unwrap_or("")
                                    );
                                }
                            }
                        }
                        format!("{}{}", sign, p.name)
                    })
                    .collect();

                let props_str = if props.is_empty() {
                    String::new()
                } else {
                    format!(" [{}]", props.join(" "))
                };

                output.push_str(&format!(
                    "  {} --> {}{}\n",
                    transition.from, transition.to, props_str
                ));
            }

            output.push_str("}\n");
            Ok(output)
        }
        "json" => {
            let json = serde_json::to_string_pretty(model)?;
            Ok(json)
        }
        other => Err(anyhow::anyhow!("Unknown format: '{}'", other)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_formula_strings_uses_modality_parser() {
        let formulas = vec![
            "always([<+APPROVE>] true)".to_string(),
            "eventually(<+DELIVER> true)".to_string(),
        ];

        let parsed = parse_formula_strings(&formulas);

        assert_eq!(parsed.len(), 2);

        let model = modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &parsed);
        let transitions = &model.parts[0].transitions;
        assert!(transitions
            .iter()
            .any(|transition| transition
                .properties
                .contains(&modality_lang::Property::new(
                    modality_lang::PropertySign::Plus,
                    "APPROVE".to_string(),
                ))));
        assert!(transitions
            .iter()
            .any(|transition| transition
                .properties
                .contains(&modality_lang::Property::new(
                    modality_lang::PropertySign::Plus,
                    "DELIVER".to_string(),
                ))));
    }

    #[test]
    fn parse_formula_strings_accepts_declared_formulas() {
        let formulas = vec!["formula existing_rule {\nalways([<+APPROVE>] true)\n}".to_string()];

        let parsed = parse_formula_strings(&formulas);

        assert_eq!(parsed.len(), 1);
    }

    #[test]
    fn llm_multiline_formula_declarations_round_trip_to_verification() {
        let response = r#"
```modality
F1: formula generated_1 {
always([<+APPROVE>] true)
}
```
"#;

        let formula_strings = modality_lang::llm_synthesis::parse_llm_response(response);
        assert_eq!(formula_strings.len(), 1);

        let formulas = parse_formula_strings(&formula_strings);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn format_synthesized_model_supports_json() {
        let model = modality_lang::Model::new("Contract".to_string());

        let json = format_synthesized_model(&model, "json").unwrap();

        assert!(json.contains("\"name\": \"Contract\""));
    }

    #[test]
    fn llm_response_loader_rejects_inline_and_file_together() {
        let response = "formula generated { true }".to_string();
        let path = PathBuf::from("response.md");

        let err = load_llm_response(Some(&response), Some(&path)).unwrap_err();

        assert!(err.to_string().contains("--llm-response-file"));
    }

    #[test]
    fn verify_synthesized_model_accepts_generated_candidate() {
        let formulas = parse_formula_strings(&["always([<+APPROVE>] true)".to_string()]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_compound_self_loop_example() {
        let formulas =
            parse_formula_strings(&["always([<+APPROVE>] true & [<+REJECT>] true)".to_string()]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_mixed_forbidden_example() {
        let formulas = parse_formula_strings(&[
            "<+CANCEL> true & ([+DISPUTE] true -> always([-RELEASE] true))".to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_compound_forbidden_example() {
        let formulas = parse_formula_strings(&[
            "[+DISPUTE] true -> (always([-RELEASE] true) & always([-REFUND] true))".to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_signer_and_forbidden_example() {
        let formulas = parse_formula_strings(&[
            "[+DISPUTE] true -> (<+signed_by(/users/arbiter.id)> true & always([-RELEASE] true))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_multi_signer_and_forbidden_example() {
        let formulas = parse_formula_strings(&[
            "[+DISPUTE] true -> (<+signed_by(/users/alice.id) +signed_by(/users/bob.id)> true & always([-RELEASE] true))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_signer_and_compound_forbidden_example() {
        let formulas = parse_formula_strings(&[
            "[+DISPUTE] true -> (<+signed_by(/users/arbiter.id)> true & (always([-RELEASE] true) & always([-REFUND] true)))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_multi_signer_and_compound_forbidden() {
        let formulas = parse_formula_strings(&[
            "[+DISPUTE] true -> (<+signed_by(/users/alice.id) +signed_by(/users/bob.id)> true & (always([-RELEASE] true) & always([-REFUND] true)))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_committed_signer_and_forbidden_example() {
        let formulas = parse_formula_strings(&[
            "[+DISPUTE] true -> ([<+signed_by(/users/arbiter.id)>] true & always([-RELEASE] true))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_committed_multi_signer_and_forbidden_example() {
        let formulas = parse_formula_strings(&[
            "[+DISPUTE] true -> ([<+signed_by(/users/alice.id) +signed_by(/users/bob.id)>] true & always([-RELEASE] true))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_committed_signer_and_compound_forbidden_example() {
        let formulas = parse_formula_strings(&[
            "[+DISPUTE] true -> ([<+signed_by(/users/arbiter.id)>] true & (always([-RELEASE] true) & always([-REFUND] true)))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_committed_multi_signer_compound_forbidden() {
        let formulas = parse_formula_strings(&[
            "[+DISPUTE] true -> ([<+signed_by(/users/alice.id) +signed_by(/users/bob.id)>] true & (always([-RELEASE] true) & always([-REFUND] true)))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_committed_action_signer_and_forbidden_example() {
        let formulas = parse_formula_strings(&[
            "[<+DISPUTE>] true -> (<+signed_by(/users/arbiter.id)> true & always([-RELEASE] true))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_committed_action_multi_signer_and_forbidden_example() {
        let formulas = parse_formula_strings(&[
            "[<+DISPUTE>] true -> (<+signed_by(/users/alice.id) +signed_by(/users/bob.id)> true & always([-RELEASE] true))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_committed_action_signer_compound_forbidden() {
        let formulas = parse_formula_strings(&[
            "[<+DISPUTE>] true -> (<+signed_by(/users/arbiter.id)> true & (always([-RELEASE] true) & always([-REFUND] true)))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_committed_action_multisig_compound_forbidden() {
        let formulas = parse_formula_strings(&[
            "[<+DISPUTE>] true -> (<+signed_by(/users/alice.id) +signed_by(/users/bob.id)> true & (always([-RELEASE] true) & always([-REFUND] true)))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_committed_action_committed_signer_and_forbidden_example() {
        let formulas = parse_formula_strings(&[
            "[<+DISPUTE>] true -> ([<+signed_by(/users/arbiter.id)>] true & always([-RELEASE] true))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_committed_action_committed_multi_signer_forbidden() {
        let formulas = parse_formula_strings(&[
            "[<+DISPUTE>] true -> ([<+signed_by(/users/alice.id) +signed_by(/users/bob.id)>] true & always([-RELEASE] true))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_committed_action_committed_signer_compound_forbidden() {
        let formulas = parse_formula_strings(&[
            "[<+DISPUTE>] true -> ([<+signed_by(/users/arbiter.id)>] true & (always([-RELEASE] true) & always([-REFUND] true)))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_committed_action_committed_multisig_compound_forbidden() {
        let formulas = parse_formula_strings(&[
            "[<+DISPUTE>] true -> ([<+signed_by(/users/alice.id) +signed_by(/users/bob.id)>] true & (always([-RELEASE] true) & always([-REFUND] true)))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_committed_action_forbidden_example() {
        let formulas =
            parse_formula_strings(&["[<+DISPUTE>] true -> always([-RELEASE] true)".to_string()]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_committed_eventual_example() {
        let formulas =
            parse_formula_strings(
                &["[+RELEASE] true -> eventually([<+DELIVER>] true)".to_string()],
            );
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_committed_action_eventual_example() {
        let formulas =
            parse_formula_strings(
                &["[<+RELEASE>] true -> eventually(<+DELIVER> true)".to_string()],
            );
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_committed_action_committed_eventual_example() {
        let formulas = parse_formula_strings(&[
            "[<+RELEASE>] true -> eventually([<+DELIVER>] true)".to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_committed_compound_forbidden_example() {
        let formulas = parse_formula_strings(&[
            "[<+DISPUTE>] true -> (always([-RELEASE] true) & always([-REFUND] true))".to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_compound_required_actions_example() {
        let formulas = parse_formula_strings(&[
            "[+RELEASE] true -> (eventually(<+DEPOSIT> true) & eventually(<+DELIVER> true))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_committed_action_compound_required_actions_example() {
        let formulas = parse_formula_strings(&[
            "[<+RELEASE>] true -> (eventually(<+DEPOSIT> true) & eventually(<+DELIVER> true))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_compound_committed_required_actions_example() {
        let formulas = parse_formula_strings(&[
            "[+RELEASE] true -> (eventually([<+DEPOSIT>] true) & eventually([<+DELIVER>] true))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_committed_action_compound_committed_example() {
        let formulas = parse_formula_strings(&[
            "[<+RELEASE>] true -> (eventually([<+DEPOSIT>] true) & eventually([<+DELIVER>] true))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_committed_signer_example() {
        let formulas = parse_formula_strings(&[
            "[+APPROVE] true -> [<+signed_by(/users/reviewer.id)>] true".to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_signer_and_committed_followup_example() {
        let formulas = parse_formula_strings(&[
            "[+RELEASE] true -> (<+signed_by(/users/buyer.id)> true & eventually([<+DELIVER>] true))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_signer_and_followup_example() {
        let formulas = parse_formula_strings(&[
            "[+RELEASE] true -> (<+signed_by(/users/buyer.id)> true & eventually(<+DELIVER> true))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_committed_signer_and_committed_followup_example() {
        let formulas = parse_formula_strings(&[
            "[+RELEASE] true -> ([<+signed_by(/users/buyer.id)>] true & eventually([<+DELIVER>] true))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_committed_signer_and_followup_example() {
        let formulas = parse_formula_strings(&[
            "[+RELEASE] true -> ([<+signed_by(/users/buyer.id)>] true & eventually(<+DELIVER> true))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_committed_action_signer_example() {
        let formulas = parse_formula_strings(&[
            "[<+RELEASE>] true -> <+signed_by(/users/buyer.id)> true".to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_committed_action_committed_signer_example() {
        let formulas = parse_formula_strings(&[
            "[<+RELEASE>] true -> [<+signed_by(/users/buyer.id)>] true".to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_committed_action_signer_and_followup_example() {
        let formulas = parse_formula_strings(&[
            "[<+RELEASE>] true -> (<+signed_by(/users/buyer.id)> true & eventually([<+DELIVER>] true))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_committed_action_signer_and_eventual_example() {
        let formulas = parse_formula_strings(&[
            "[<+RELEASE>] true -> (<+signed_by(/users/buyer.id)> true & eventually(<+DELIVER> true))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_committed_action_committed_signer_and_followup_example() {
        let formulas = parse_formula_strings(&[
            "[<+RELEASE>] true -> ([<+signed_by(/users/buyer.id)>] true & eventually([<+DELIVER>] true))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_committed_action_committed_signer_and_eventual_example() {
        let formulas = parse_formula_strings(&[
            "[<+RELEASE>] true -> ([<+signed_by(/users/buyer.id)>] true & eventually(<+DELIVER> true))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_committed_action_multi_signer_example() {
        let formulas = parse_formula_strings(&[
            "[<+APPROVE>] true -> <+signed_by(/users/alice.id) +signed_by(/users/bob.id)> true"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_committed_action_committed_multi_signer_example() {
        let formulas = parse_formula_strings(&[
            "[<+APPROVE>] true -> [<+signed_by(/users/alice.id) +signed_by(/users/bob.id)>] true"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_committed_action_multi_signer_and_followup_example() {
        let formulas = parse_formula_strings(&[
            "[<+APPROVE>] true -> (<+signed_by(/users/alice.id) +signed_by(/users/bob.id)> true & eventually([<+DELIVER>] true))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_committed_action_multi_signer_and_eventual_example() {
        let formulas = parse_formula_strings(&[
            "[<+APPROVE>] true -> (<+signed_by(/users/alice.id) +signed_by(/users/bob.id)> true & eventually(<+DELIVER> true))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_committed_action_multi_signer_followup_example() {
        let formulas = parse_formula_strings(&[
            "[<+APPROVE>] true -> ([<+signed_by(/users/alice.id) +signed_by(/users/bob.id)>] true & eventually([<+DELIVER>] true))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_committed_action_multi_signer_eventual_example() {
        let formulas = parse_formula_strings(&[
            "[<+APPROVE>] true -> ([<+signed_by(/users/alice.id) +signed_by(/users/bob.id)>] true & eventually(<+DELIVER> true))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_multi_signer_example() {
        let formulas = parse_formula_strings(&[
            "[+APPROVE] true -> <+signed_by(/users/alice.id) +signed_by(/users/bob.id)> true"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_multi_signer_and_committed_followup_example() {
        let formulas = parse_formula_strings(&[
            "[+APPROVE] true -> (<+signed_by(/users/alice.id) +signed_by(/users/bob.id)> true & eventually([<+DELIVER>] true))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_multi_signer_and_followup_example() {
        let formulas = parse_formula_strings(&[
            "[+APPROVE] true -> (<+signed_by(/users/alice.id) +signed_by(/users/bob.id)> true & eventually(<+DELIVER> true))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_committed_multi_signer_and_committed_followup_example() {
        let formulas = parse_formula_strings(&[
            "[+APPROVE] true -> ([<+signed_by(/users/alice.id) +signed_by(/users/bob.id)>] true & eventually([<+DELIVER>] true))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_committed_multi_signer_and_followup_example() {
        let formulas = parse_formula_strings(&[
            "[+APPROVE] true -> ([<+signed_by(/users/alice.id) +signed_by(/users/bob.id)>] true & eventually(<+DELIVER> true))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_committed_multi_signer_example() {
        let formulas = parse_formula_strings(&[
            "[+APPROVE] true -> [<+signed_by(/users/alice.id) +signed_by(/users/bob.id)>] true"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_rejects_unsatisfied_formula() {
        let mut model = modality_lang::Model::new("Contract".to_string());
        let mut part = modality_lang::Part::new("flow".to_string());
        part.add_transition(modality_lang::Transition::new(
            "init".to_string(),
            "done".to_string(),
        ));
        model.add_part(part);

        let formulas = vec![modality_lang::FormulaExpr::False];

        assert!(verify_synthesized_model(&model, &formulas).is_err());
    }

    #[test]
    fn verify_requires_every_input_formula_to_parse() {
        let formulas = vec![
            "always([<+APPROVE>] true)".to_string(),
            "always(".to_string(),
        ];

        let err = ensure_all_formula_strings_parsed(&formulas).unwrap_err();

        assert!(err.to_string().contains("F2"));
    }

    #[test]
    fn unparsed_formula_labels_include_formula_preview() {
        let formulas = vec![
            "always([<+APPROVE>] true)".to_string(),
            "always(".to_string(),
        ];

        let unparsed = unparsed_formula_string_labels(&formulas);

        assert_eq!(unparsed, vec!["F2 `always(`".to_string()]);
    }

    #[test]
    fn unparsed_formula_labels_truncate_long_formula_preview() {
        let formulas = vec![format!("always({}", "x".repeat(120))];

        let unparsed = unparsed_formula_string_labels(&formulas);

        assert_eq!(unparsed[0].len(), "F1 ``".len() + 83);
        assert!(unparsed[0].ends_with("...`"));
    }

    #[test]
    fn legacy_string_constraints_still_cover_unparseable_llm_output() {
        let formulas = vec![
            "[+RELEASE] implies eventually(<+DELIVER> true)".to_string(),
            "[+RELEASE] implies <+signed_by(/users/alice.id)> true".to_string(),
        ];

        let constraints = synthesize_constraints_from_strings(&formulas);

        assert!(constraints
            .ordering
            .contains(&("RELEASE".to_string(), "DELIVER".to_string())));
        assert_eq!(
            constraints.authorization.get("RELEASE"),
            Some(&vec!["/users/alice.id".to_string()])
        );
    }
}
