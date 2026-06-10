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

    /// Output file path
    #[arg(short, long)]
    pub output: Option<PathBuf>,

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
                "\n💡 Send this prompt to Claude/GPT, then use --llm-response with the output"
            );
            return Ok(());
        } else {
            return Err(anyhow::anyhow!("--generate-prompt requires --describe"));
        }
    }

    // Step 1b + 2: Parse LLM response and synthesize
    if let Some(llm_response) = &opts.llm_response {
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

        let parsed_formulas = parse_formula_strings(&formulas);
        let model = if parsed_formulas.is_empty() {
            println!("⚠️  Could not parse formulas; using legacy string heuristics\n");
            let constraints = synthesize_constraints_from_strings(&formulas);

            println!("📊 Extracted constraints:");
            println!("  Actions: {:?}", constraints.actions);
            println!("  Ordering: {:?}", constraints.ordering);
            println!("  Authorization: {:?}", constraints.authorization);
            println!();

            modality_lang::formula_synthesis::synthesize_from_constraints("Contract", &constraints)
        } else {
            println!(
                "📊 Parsed {} formula(s) with the Modality parser\n",
                parsed_formulas.len()
            );
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &parsed_formulas)
        };

        println!("✅ Synthesized model:\n");
        let output = modality_lang::print_model(&model);
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

        let formulas = parse_formula_strings(&formula_strs);

        if formulas.is_empty() {
            return Err(anyhow::anyhow!("No valid formulas found"));
        }

        // Extract constraints and synthesize
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);

        println!("✅ Synthesized model:\n");
        let output = modality_lang::print_model(&model);
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

        // Try to parse formulas from rule file
        match modality_lang::parse_all_formulas_content_lalrpop(&content) {
            Ok(formulas) if !formulas.is_empty() => {
                let formula_exprs: Vec<_> = formulas.iter().map(|f| f.expression.clone()).collect();
                let model = modality_lang::formula_synthesis::synthesize_from_formulas(
                    "Contract",
                    &formula_exprs,
                );

                let output = modality_lang::print_model(&model);

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
            _ => {
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
        }

        return Ok(());
    }

    // Handle natural language description
    if let Some(description) = &opts.describe {
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

    match opts.format.as_str() {
        "modality" => {
            let output = modality_lang::print_model(&model);
            println!("{}", output);
        }
        "json" => {
            let json = serde_json::to_string_pretty(&model)?;
            println!("{}", json);
        }
        other => {
            return Err(anyhow::anyhow!(
                "Unknown format: '{}'. Use 'modality' or 'json'.",
                other
            ))
        }
    }

    Ok(())
}

fn parse_formula_strings(formulas: &[String]) -> Vec<modality_lang::FormulaExpr> {
    let mut parsed_formulas = Vec::new();

    for (index, formula) in formulas.iter().enumerate() {
        let parsed = modality_lang::parse_all_formulas_content_lalrpop(formula)
            .ok()
            .filter(|parsed| !parsed.is_empty())
            .or_else(|| {
                let wrapped = format!("formula generated_{} {{\n{}\n}}", index + 1, formula);
                modality_lang::parse_all_formulas_content_lalrpop(&wrapped).ok()
            });

        if let Some(parsed) = parsed {
            parsed_formulas.extend(parsed.into_iter().map(|formula| formula.expression));
        }
    }

    parsed_formulas
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
