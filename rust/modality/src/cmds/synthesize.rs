use anyhow::{Context, Result};
use clap::Parser;
use std::collections::{BTreeSet, HashSet};
use std::path::PathBuf;

/// Synthesize a model from a template, pattern, or rule
#[derive(Parser, Debug)]
pub struct Opts {
    /// Template name: escrow, handshake, mutual_cooperation, etc.
    #[arg(
        short,
        long,
        value_parser = [
            "escrow",
            "handshake",
            "mutual_cooperation",
            "atomic_swap",
            "multisig",
            "turn_taking",
            "alternating",
            "service_agreement",
            "delegation",
            "auction",
            "subscription",
            "milestone"
        ]
    )]
    pub template: Option<String>,

    /// Natural language description of the contract
    #[arg(short, long)]
    pub describe: Option<String>,

    /// Synthesize from a rule file containing formulas
    #[arg(short, long)]
    pub rule: Option<PathBuf>,

    /// Existing model file to test before synthesizing a replacement candidate
    #[arg(long)]
    pub existing_model: Option<PathBuf>,

    /// Proposed formula text to check against an existing model
    #[arg(long)]
    pub proposed_formula: Option<String>,

    /// File containing proposed formula(s) to check against an existing model
    #[arg(long)]
    pub proposed_rule: Option<PathBuf>,

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

    /// Original prompt or source text that led to the LLM response
    #[arg(long)]
    pub source_text: Option<String>,

    /// File containing original prompt or source text that led to the LLM response
    #[arg(long)]
    pub source_file: Option<PathBuf>,

    /// Output file path
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Write a review bundle for verified parser-backed synthesis
    #[arg(long)]
    pub review_bundle: Option<PathBuf>,

    /// Verify parser-backed synthesized models against their input formulas
    #[arg(long)]
    pub verify: bool,

    /// Maximum witness-node count for bounded μ-calculus search
    #[arg(long, default_value_t = 4)]
    pub max_states: usize,

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
    #[arg(short, long, default_value = "modality", value_parser = ["modality", "json"])]
    pub format: String,

    /// List available templates
    #[arg(short, long)]
    pub list: bool,
}

pub async fn run(opts: &Opts) -> Result<()> {
    ensure_output_format_is_supported(&opts.format)?;
    ensure_review_bundle_mode(opts)?;

    if has_existing_model_inputs(opts) {
        return run_existing_model_synthesis(opts);
    }

    if opts.list {
        ensure_list_mode_is_exclusive(opts)?;
        print_synthesis_list();
        return Ok(());
    }

    if opts.generate_prompt {
        ensure_prompt_generation_mode_is_exclusive(opts)?;
        let Some(description) = &opts.describe else {
            return Err(anyhow::anyhow!("--generate-prompt requires --describe"));
        };
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
    }

    if let Some(description) = &opts.describe {
        ensure_describe_mode_is_exclusive(opts)?;
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

    if opts.template.is_some() {
        ensure_template_mode_is_exclusive(opts)?;
    }

    if opts.formulas.is_some() {
        ensure_formulas_mode_is_exclusive(opts)?;
    }

    if opts.rule.is_some() {
        ensure_rule_mode_is_exclusive(opts)?;
    }

    if opts.llm_response.is_some() || opts.llm_response_file.is_some() {
        ensure_llm_response_mode_is_exclusive(opts)?;
    }

    let llm_response =
        load_llm_response(opts.llm_response.as_ref(), opts.llm_response_file.as_ref())?;
    let review_source = load_review_source(opts.source_text.as_ref(), opts.source_file.as_ref())?;

    if opts.verify && !has_verifiable_synthesis_inputs(opts) {
        return Err(anyhow::anyhow!(
            "--verify requires --formulas, --rule, --llm-response, or --llm-response-file"
        ));
    }

    // Step 1b + 2: Parse LLM response and synthesize
    if let Some(llm_response) = &llm_response {
        println!("🔧 Two-Step Pipeline: LLM Response → Model\n");

        // Parse formulas from LLM response
        let formulas = modality_lang::llm_synthesis::parse_llm_response(llm_response);

        if formulas.is_empty() {
            return Err(anyhow::anyhow!(
                "No formulas found in LLM response; expected Modality formula declarations or F1:/F2: formula lines"
            ));
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
        if parsed_input.formulas.is_empty() {
            return Err(anyhow::anyhow!(
                "No formulas in the LLM response could be parsed by the Modality parser"
            ));
        }
        if !opts.verify {
            parsed_input.warn_unparsed();
        }
        println!(
            "📊 Parsed {} formula(s) with the Modality parser\n",
            parsed_input.formulas.len()
        );
        let model = take_synthesized_model(synthesize_bounded(
            &parsed_input.formulas,
            opts,
            "Contract",
        ))?;

        if opts.verify {
            verify_synthesized_model_with_labels(
                &model,
                &parsed_input.formulas,
                &parsed_input.labels,
            )?;
            println!();
        }

        println!("✅ Synthesized model:\n");
        let output = format_synthesized_model(&model, &opts.format)?;
        println!("{}", output);

        write_output_file_if_requested(&output, opts.output.as_ref())?;
        write_llm_review_bundle_if_requested(
            opts.review_bundle.as_ref(),
            &llm_response_source_label(opts),
            llm_response,
            review_source.as_ref(),
            &formulas,
            &parsed_input,
            &output,
            &opts.format,
        )?;

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
            return Err(parsed_input.no_valid_formulas_error());
        }

        if !opts.verify {
            parsed_input.warn_unparsed();
        }

        let model = take_synthesized_model(synthesize_bounded(
            &parsed_input.formulas,
            opts,
            "Contract",
        ))?;

        if opts.verify {
            verify_synthesized_model_with_labels(
                &model,
                &parsed_input.formulas,
                &parsed_input.labels,
            )?;
            println!();
        }

        println!("✅ Synthesized model:\n");
        let output = format_synthesized_model(&model, &opts.format)?;
        println!("{}", output);

        write_output_file_if_requested(&output, opts.output.as_ref())?;

        return Ok(());
    }

    // Handle rule file-based synthesis
    if let Some(rule_path) = &opts.rule {
        let content = std::fs::read_to_string(rule_path)
            .with_context(|| format!("Failed to read rule file {}", rule_path.display()))?;

        println!("🔧 Synthesizing from rule file: {}\n", rule_path.display());

        let parsed_input = parse_formula_inputs(std::slice::from_ref(&content));
        if opts.verify {
            parsed_input.ensure_all_parsed()?;
        }
        if parsed_input.formulas.is_empty() {
            return Err(parsed_input.no_valid_formulas_error());
        }

        let bounded = synthesize_bounded(&parsed_input.formulas, opts, "Contract");
        match bounded.model {
            Some(model) => {
                let output = format_synthesized_model(&model, &opts.format)?;

                if opts.verify {
                    if let Err(err) = verify_synthesized_model_with_labels(
                        &model,
                        &parsed_input.formulas,
                        &parsed_input.labels,
                    ) {
                        write_rule_failed_review_bundle_if_requested(
                            opts.review_bundle.as_ref(),
                            rule_path,
                            &content,
                            review_source.as_ref(),
                            &parsed_input,
                            &output,
                            &opts.format,
                            &err,
                        )?;
                        return Err(anyhow::anyhow!(
                            "No satisfying witness found by bounded μ-calculus search for parser-backed rule formulas; verifier rejected the synthesized candidate: {}",
                            err
                        ));
                    }
                    println!();
                }

                write_or_print_model(&output, opts.output.as_ref())?;
                write_rule_review_bundle_if_requested(
                    opts.review_bundle.as_ref(),
                    rule_path,
                    &content,
                    review_source.as_ref(),
                    &parsed_input,
                    &output,
                    &opts.format,
                )?;
            }
            None => {
                let err = no_witness_error(bounded.unsat_reason.as_deref().unwrap_or("unsat"));
                let output = match &bounded.last_candidate {
                    Some(candidate) => format_synthesized_model(candidate, &opts.format)?,
                    None => format!("// no candidate within {} states\n", opts.max_states),
                };
                write_rule_failed_review_bundle_if_requested(
                    opts.review_bundle.as_ref(),
                    rule_path,
                    &content,
                    review_source.as_ref(),
                    &parsed_input,
                    &output,
                    &opts.format,
                    &err,
                )?;
                return Err(err);
            }
        }

        return Ok(());
    }

    let template = opts.template.as_ref().ok_or_else(|| {
        anyhow::anyhow!(
            "Please specify --template, --describe, --rule, --formulas, --llm-response, --llm-response-file, or use --list/--generate-prompt to see options"
        )
    })?;
    ensure_template_name_is_known(template)?;
    ensure_milestones_match_template(template, opts)?;
    ensure_template_party_names_are_valid(template, opts)?;

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
            let milestones = template_milestones(opts)?;
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

struct FormulaExampleGroup {
    title: &'static str,
    description: &'static str,
    formulas: &'static [&'static str],
}

const FORMULA_EXAMPLE_GROUPS: &[FormulaExampleGroup] = &[
    FormulaExampleGroup {
        title: r#"Core formula shapes"#,
        description: r#"Single-action requirements and always-safe commitments."#,
        formulas: &[
            r#"always([<+APPROVE>] true)"#,
            r#"always(<+A> true)"#,
            r#"[] always([-signed_by(/parties/alice.id) -signed_by(/parties/bob.id)] false)"#,
        ],
    },
];

fn print_synthesis_list() {
    print!("{}", synthesis_list_text());
}

fn synthesis_list_text() -> String {
    let mut output = String::new();

    output.push_str("Available templates:\n\n");
    output.push_str("  escrow              Two-party escrow with deposit/deliver/release\n");
    output.push_str("  handshake           Mutual agreement requiring both signatures\n");
    output.push_str(
        "  mutual_cooperation  Cooperation game - both must cooperate, defection blocked\n",
    );
    output.push_str("  atomic_swap         Both parties commit before either can claim\n");
    output.push_str("  multisig            N-of-M signature approval pattern\n");
    output.push_str("  turn_taking         Alternating two-party turn cycle\n");
    output.push_str("  service_agreement   Offer -> Accept -> Deliver -> Confirm -> Pay\n");
    output.push_str("  delegation          Principal grants agent authority to act\n");
    output.push_str("  auction             Seller lists, bidders bid, highest wins\n");
    output.push_str("  subscription        Recurring payment for service access\n");
    output.push_str("  milestone           Multi-phase project with payments\n");
    output.push_str("\nUsage:\n");
    output.push_str(
        "  modality model synthesize --template escrow --party-a Buyer --party-b Seller\n",
    );
    output.push_str("\nOr describe in natural language:\n");
    output
        .push_str("  modality model synthesize --describe \"escrow where buyer deposits funds\"\n");
    output
        .push_str("  modality model synthesize --describe \"Alice and Bob take turns signing\"\n");
    output.push_str("\nOr evolve an existing model with a proposed rule:\n");
    output.push_str(
        "  modality model synthesize --existing-model contract.modality --proposed-rule amendment.modality --output candidate.modality\n",
    );
    output.push_str(
        "  modality model synthesize --existing-model contract.modality --proposed-formula \"always([<+APPROVE>] true)\"\n",
    );
    output.push_str("\nOr synthesize and verify from formulas:\n");
    for group in FORMULA_EXAMPLE_GROUPS {
        output.push_str(&format!("\n  {}:\n", group.title));
        output.push_str(&format!("    {}\n", group.description));
        for formula in group.formulas {
            output.push_str(&format!(
                "    modality model synthesize --formulas \"{}\" --verify",
                escape_formula_for_command(formula)
            ));
            output.push('\n');
        }
    }
    output.push_str("\nOr generate a prompt and synthesize an LLM response file:\n");
    output.push_str(
        "  modality model synthesize --describe \"escrow where buyer deposits funds\" --generate-prompt\n",
    );
    output.push_str("  modality model synthesize --llm-response-file response.md --verify\n");
    output.push_str(
        "  modality model synthesize --llm-response-file response.md --verify --review-bundle review.md\n",
    );
    output.push_str(
        "  modality model synthesize --source-file prompt.md --llm-response-file response.md --verify --review-bundle review.md\n",
    );

    output
}

fn escape_formula_for_command(formula: &str) -> String {
    formula.replace('"', "\\\"")
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
        (None, Some(path)) => Ok(Some(std::fs::read_to_string(path).with_context(|| {
            format!("Failed to read LLM response file {}", path.display())
        })?)),
        (None, None) => Ok(None),
    }
}

struct ReviewSource {
    label: String,
    content: String,
}

fn load_review_source(
    source_text: Option<&String>,
    source_file: Option<&PathBuf>,
) -> Result<Option<ReviewSource>> {
    match (source_text, source_file) {
        (Some(_), Some(_)) => Err(anyhow::anyhow!(
            "Use either --source-text or --source-file, not both"
        )),
        (Some(content), None) => Ok(Some(ReviewSource {
            label: "--source-text inline text".to_string(),
            content: content.clone(),
        })),
        (None, Some(path)) => Ok(Some(ReviewSource {
            label: format!("--source-file {}", path.display()),
            content: std::fs::read_to_string(path)
                .with_context(|| format!("Failed to read source file {}", path.display()))?,
        })),
        (None, None) => Ok(None),
    }
}

fn llm_response_source_label(opts: &Opts) -> String {
    if opts.llm_response.is_some() {
        "--llm-response inline text".to_string()
    } else if let Some(path) = &opts.llm_response_file {
        format!("--llm-response-file {}", path.display())
    } else {
        "--llm-response source unknown".to_string()
    }
}

fn has_existing_model_inputs(opts: &Opts) -> bool {
    opts.existing_model.is_some() || opts.proposed_formula.is_some() || opts.proposed_rule.is_some()
}

fn has_verifiable_synthesis_inputs(opts: &Opts) -> bool {
    opts.formulas.is_some()
        || opts.rule.is_some()
        || opts.llm_response.is_some()
        || opts.llm_response_file.is_some()
}

fn ensure_review_bundle_mode(opts: &Opts) -> Result<()> {
    if opts.source_text.is_some() && opts.source_file.is_some() {
        return Err(anyhow::anyhow!(
            "Use either --source-text or --source-file, not both"
        ));
    }

    if opts.review_bundle.is_none() && (opts.source_text.is_some() || opts.source_file.is_some()) {
        return Err(anyhow::anyhow!(
            "--source-text and --source-file require --review-bundle so the original source is captured"
        ));
    }

    if opts.review_bundle.is_none() {
        return Ok(());
    }

    if opts.llm_response.is_none() && opts.llm_response_file.is_none() && opts.rule.is_none() {
        return Err(anyhow::anyhow!(
            "--review-bundle requires --rule, --llm-response, or --llm-response-file"
        ));
    }

    if !opts.verify {
        return Err(anyhow::anyhow!(
            "--review-bundle requires --verify so the bundle includes a parser-backed verifier result"
        ));
    }

    Ok(())
}

fn ensure_output_format_is_supported(format: &str) -> Result<()> {
    match format {
        "modality" | "json" => Ok(()),
        other => Err(anyhow::anyhow!(
            "Unknown format: '{}'. Use 'modality' or 'json'.",
            other
        )),
    }
}

fn ensure_list_mode_is_exclusive(opts: &Opts) -> Result<()> {
    let conflicts = list_mode_conflicts(opts);
    if conflicts.is_empty() {
        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "--list cannot be combined with other synthesis modes: {}",
            conflicts.join(", ")
        ))
    }
}

fn list_mode_conflicts(opts: &Opts) -> Vec<&'static str> {
    let mut conflicts = Vec::new();

    if opts.template.is_some() {
        conflicts.push("--template");
    }
    if opts.describe.is_some() {
        conflicts.push("--describe");
    }
    if opts.rule.is_some() {
        conflicts.push("--rule");
    }
    if opts.formulas.is_some() {
        conflicts.push("--formulas");
    }
    if opts.generate_prompt {
        conflicts.push("--generate-prompt");
    }
    if opts.llm_response.is_some() {
        conflicts.push("--llm-response");
    }
    if opts.llm_response_file.is_some() {
        conflicts.push("--llm-response-file");
    }
    if opts.source_text.is_some() {
        conflicts.push("--source-text");
    }
    if opts.source_file.is_some() {
        conflicts.push("--source-file");
    }
    if opts.output.is_some() {
        conflicts.push("--output");
    }
    if opts.review_bundle.is_some() {
        conflicts.push("--review-bundle");
    }
    if opts.verify {
        conflicts.push("--verify");
    }
    if opts.milestones.is_some() {
        conflicts.push("--milestones");
    }

    conflicts
}

fn ensure_prompt_generation_mode_is_exclusive(opts: &Opts) -> Result<()> {
    let conflicts = prompt_generation_mode_conflicts(opts);
    if conflicts.is_empty() {
        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "--generate-prompt cannot be combined with other synthesis modes: {}",
            conflicts.join(", ")
        ))
    }
}

fn prompt_generation_mode_conflicts(opts: &Opts) -> Vec<&'static str> {
    let mut conflicts = Vec::new();

    if opts.template.is_some() {
        conflicts.push("--template");
    }
    if opts.rule.is_some() {
        conflicts.push("--rule");
    }
    if opts.formulas.is_some() {
        conflicts.push("--formulas");
    }
    if opts.llm_response.is_some() {
        conflicts.push("--llm-response");
    }
    if opts.llm_response_file.is_some() {
        conflicts.push("--llm-response-file");
    }
    if opts.source_text.is_some() {
        conflicts.push("--source-text");
    }
    if opts.source_file.is_some() {
        conflicts.push("--source-file");
    }
    if opts.output.is_some() {
        conflicts.push("--output");
    }
    if opts.review_bundle.is_some() {
        conflicts.push("--review-bundle");
    }
    if opts.verify {
        conflicts.push("--verify");
    }
    if opts.milestones.is_some() {
        conflicts.push("--milestones");
    }

    conflicts
}

fn ensure_describe_mode_is_exclusive(opts: &Opts) -> Result<()> {
    let conflicts = describe_mode_conflicts(opts);
    if conflicts.is_empty() {
        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "--describe cannot be combined with other synthesis modes: {}",
            conflicts.join(", ")
        ))
    }
}

fn describe_mode_conflicts(opts: &Opts) -> Vec<&'static str> {
    let mut conflicts = Vec::new();

    if opts.template.is_some() {
        conflicts.push("--template");
    }
    if opts.rule.is_some() {
        conflicts.push("--rule");
    }
    if opts.formulas.is_some() {
        conflicts.push("--formulas");
    }
    if opts.llm_response.is_some() {
        conflicts.push("--llm-response");
    }
    if opts.llm_response_file.is_some() {
        conflicts.push("--llm-response-file");
    }
    if opts.source_text.is_some() {
        conflicts.push("--source-text");
    }
    if opts.source_file.is_some() {
        conflicts.push("--source-file");
    }
    if opts.verify {
        conflicts.push("--verify");
    }
    if opts.milestones.is_some() {
        conflicts.push("--milestones");
    }

    conflicts
}

fn ensure_template_mode_is_exclusive(opts: &Opts) -> Result<()> {
    let conflicts = template_mode_conflicts(opts);
    if conflicts.is_empty() {
        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "--template cannot be combined with other synthesis modes: {}",
            conflicts.join(", ")
        ))
    }
}

fn template_mode_conflicts(opts: &Opts) -> Vec<&'static str> {
    let mut conflicts = Vec::new();

    if opts.describe.is_some() {
        conflicts.push("--describe");
    }
    if opts.rule.is_some() {
        conflicts.push("--rule");
    }
    if opts.formulas.is_some() {
        conflicts.push("--formulas");
    }
    if opts.generate_prompt {
        conflicts.push("--generate-prompt");
    }
    if opts.llm_response.is_some() {
        conflicts.push("--llm-response");
    }
    if opts.llm_response_file.is_some() {
        conflicts.push("--llm-response-file");
    }
    if opts.source_text.is_some() {
        conflicts.push("--source-text");
    }
    if opts.source_file.is_some() {
        conflicts.push("--source-file");
    }
    if opts.verify {
        conflicts.push("--verify");
    }
    if opts.list {
        conflicts.push("--list");
    }

    conflicts
}

fn ensure_formulas_mode_is_exclusive(opts: &Opts) -> Result<()> {
    let conflicts = formulas_mode_conflicts(opts);
    if conflicts.is_empty() {
        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "--formulas cannot be combined with other synthesis modes: {}",
            conflicts.join(", ")
        ))
    }
}

fn formulas_mode_conflicts(opts: &Opts) -> Vec<&'static str> {
    let mut conflicts = Vec::new();

    if opts.template.is_some() {
        conflicts.push("--template");
    }
    if opts.describe.is_some() {
        conflicts.push("--describe");
    }
    if opts.rule.is_some() {
        conflicts.push("--rule");
    }
    if opts.generate_prompt {
        conflicts.push("--generate-prompt");
    }
    if opts.llm_response.is_some() {
        conflicts.push("--llm-response");
    }
    if opts.llm_response_file.is_some() {
        conflicts.push("--llm-response-file");
    }
    if opts.source_text.is_some() {
        conflicts.push("--source-text");
    }
    if opts.source_file.is_some() {
        conflicts.push("--source-file");
    }
    if opts.list {
        conflicts.push("--list");
    }
    if opts.review_bundle.is_some() {
        conflicts.push("--review-bundle");
    }
    if opts.milestones.is_some() {
        conflicts.push("--milestones");
    }

    conflicts
}

fn ensure_rule_mode_is_exclusive(opts: &Opts) -> Result<()> {
    let conflicts = rule_mode_conflicts(opts);
    if conflicts.is_empty() {
        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "--rule cannot be combined with other synthesis modes: {}",
            conflicts.join(", ")
        ))
    }
}

fn rule_mode_conflicts(opts: &Opts) -> Vec<&'static str> {
    let mut conflicts = Vec::new();

    if opts.template.is_some() {
        conflicts.push("--template");
    }
    if opts.describe.is_some() {
        conflicts.push("--describe");
    }
    if opts.formulas.is_some() {
        conflicts.push("--formulas");
    }
    if opts.generate_prompt {
        conflicts.push("--generate-prompt");
    }
    if opts.llm_response.is_some() {
        conflicts.push("--llm-response");
    }
    if opts.llm_response_file.is_some() {
        conflicts.push("--llm-response-file");
    }
    if opts.list {
        conflicts.push("--list");
    }
    if opts.milestones.is_some() {
        conflicts.push("--milestones");
    }

    conflicts
}

fn ensure_llm_response_mode_is_exclusive(opts: &Opts) -> Result<()> {
    let conflicts = llm_response_mode_conflicts(opts);
    if conflicts.is_empty() {
        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "{} cannot be combined with other synthesis modes: {}",
            llm_response_mode_flag(opts),
            conflicts.join(", ")
        ))
    }
}

fn llm_response_mode_flag(opts: &Opts) -> &'static str {
    if opts.llm_response.is_some() {
        "--llm-response"
    } else {
        "--llm-response-file"
    }
}

fn llm_response_mode_conflicts(opts: &Opts) -> Vec<&'static str> {
    let mut conflicts = Vec::new();

    if opts.template.is_some() {
        conflicts.push("--template");
    }
    if opts.describe.is_some() {
        conflicts.push("--describe");
    }
    if opts.rule.is_some() {
        conflicts.push("--rule");
    }
    if opts.formulas.is_some() {
        conflicts.push("--formulas");
    }
    if opts.generate_prompt {
        conflicts.push("--generate-prompt");
    }
    if opts.llm_response.is_some() && opts.llm_response_file.is_some() {
        conflicts.push("--llm-response-file");
    }
    if opts.list {
        conflicts.push("--list");
    }
    if opts.milestones.is_some() {
        conflicts.push("--milestones");
    }

    conflicts
}

fn run_existing_model_synthesis(opts: &Opts) -> Result<()> {
    ensure_existing_model_mode_is_exclusive(opts)?;

    let existing_model_path = opts.existing_model.as_ref().ok_or_else(|| {
        anyhow::anyhow!("--existing-model is required with --proposed-formula or --proposed-rule")
    })?;

    let proposed_source_count =
        (opts.proposed_formula.is_some() as usize) + (opts.proposed_rule.is_some() as usize);
    if proposed_source_count != 1 {
        return Err(anyhow::anyhow!(
            "Use exactly one of --proposed-formula or --proposed-rule with --existing-model"
        ));
    }

    let existing_input = load_existing_model_input(existing_model_path)?;
    let (parsed_input, proposed_declarations) = load_proposed_formula_inputs(opts)?;
    parsed_input.ensure_all_parsed()?;

    if parsed_input.formulas.is_empty() {
        return Err(anyhow::anyhow!("No proposed formulas found"));
    }

    let mut candidate_formulas = existing_input.formulas.clone();
    candidate_formulas.extend(parsed_input.formulas.clone());
    let mut candidate_labels = existing_input.labels.clone();
    candidate_labels.extend(parsed_input.labels.clone());

    println!(
        "🔎 Checking existing model '{}' against {} existing and {} proposed formula(s)\n",
        existing_input.model.name,
        existing_input.formulas.len(),
        parsed_input.formulas.len()
    );

    let failed = existing_model_unsatisfied_formula_labels(
        &existing_input.model,
        &candidate_formulas,
        &candidate_labels,
    );

    let output_model = if failed.is_empty() {
        println!("✅ Existing model satisfies every existing and proposed formula\n");
        existing_input.model.clone()
    } else {
        println!(
            "⚠️  Existing model does not satisfy {} formula(s): {}",
            failed.len(),
            failed.join(", ")
        );
        println!(
            "🔧 Synthesizing a local replacement candidate from existing plus proposed formulas\n"
        );

        let candidate_name = replacement_candidate_name(&existing_input.model);
        let candidate = take_synthesized_model(synthesize_bounded(
            &candidate_formulas,
            opts,
            &candidate_name,
        ))?;
        verify_synthesized_model_with_labels(&candidate, &candidate_formulas, &candidate_labels)?;
        println!();
        candidate
    };

    let mut output_declarations = existing_input.formula_declarations;
    output_declarations.extend(proposed_declarations);
    let output =
        format_synthesized_model_with_formulas(&output_model, &opts.format, &output_declarations)?;
    write_or_print_model(&output, opts.output.as_ref())?;

    Ok(())
}

fn ensure_existing_model_mode_is_exclusive(opts: &Opts) -> Result<()> {
    let conflicts = existing_model_mode_conflicts(opts);
    if conflicts.is_empty() {
        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "--existing-model cannot be combined with other synthesis modes: {}",
            conflicts.join(", ")
        ))
    }
}

fn existing_model_mode_conflicts(opts: &Opts) -> Vec<&'static str> {
    let mut conflicts = Vec::new();

    if opts.template.is_some() {
        conflicts.push("--template");
    }
    if opts.describe.is_some() {
        conflicts.push("--describe");
    }
    if opts.rule.is_some() {
        conflicts.push("--rule");
    }
    if opts.formulas.is_some() {
        conflicts.push("--formulas");
    }
    if opts.generate_prompt {
        conflicts.push("--generate-prompt");
    }
    if opts.llm_response.is_some() {
        conflicts.push("--llm-response");
    }
    if opts.llm_response_file.is_some() {
        conflicts.push("--llm-response-file");
    }
    if opts.source_text.is_some() {
        conflicts.push("--source-text");
    }
    if opts.source_file.is_some() {
        conflicts.push("--source-file");
    }
    if opts.milestones.is_some() {
        conflicts.push("--milestones");
    }
    if opts.list {
        conflicts.push("--list");
    }

    conflicts
}

fn ensure_milestones_match_template(template: &str, opts: &Opts) -> Result<()> {
    if opts.milestones.is_some() && template != "milestone" {
        Err(anyhow::anyhow!(
            "--milestones can only be used with --template milestone"
        ))
    } else {
        Ok(())
    }
}

fn ensure_template_name_is_known(template: &str) -> Result<()> {
    if is_known_template_name(template) {
        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "Unknown template: '{}'. Use --list to see available templates.",
            template
        ))
    }
}

fn is_known_template_name(template: &str) -> bool {
    matches!(
        template,
        "escrow"
            | "handshake"
            | "mutual_cooperation"
            | "atomic_swap"
            | "multisig"
            | "turn_taking"
            | "alternating"
            | "service_agreement"
            | "delegation"
            | "auction"
            | "subscription"
            | "milestone"
    )
}

fn ensure_template_party_names_are_valid(template: &str, opts: &Opts) -> Result<()> {
    if !is_valid_template_identifier_component(&opts.party_a) {
        return Err(anyhow::anyhow!(
            "--party-a must contain only letters, numbers, and underscores, and must start with a letter or underscore"
        ));
    }

    if template != "auction" && !is_valid_template_identifier_component(&opts.party_b) {
        return Err(anyhow::anyhow!(
            "--party-b must contain only letters, numbers, and underscores, and must start with a letter or underscore"
        ));
    }

    Ok(())
}

fn template_milestones(opts: &Opts) -> Result<Vec<&str>> {
    let Some(milestones) = opts.milestones.as_ref() else {
        return Ok(vec!["Phase1", "Phase2", "Phase3"]);
    };

    let names: Vec<&str> = milestones.split(',').map(|name| name.trim()).collect();
    if names.iter().any(|name| name.is_empty()) {
        return Err(anyhow::anyhow!(
            "--milestones requires non-empty comma-separated names"
        ));
    }
    if names
        .iter()
        .any(|name| !is_valid_milestone_template_name(name))
    {
        return Err(anyhow::anyhow!(
            "--milestones names may contain only letters, numbers, underscores, and spaces, and must start with a letter or underscore"
        ));
    }
    let mut normalized_names = HashSet::new();
    if names
        .iter()
        .any(|name| !normalized_names.insert(normalized_milestone_template_name(name)))
    {
        return Err(anyhow::anyhow!(
            "--milestones names must be unique after spaces are normalized to underscores"
        ));
    }

    Ok(names)
}

fn is_valid_milestone_template_name(name: &str) -> bool {
    is_valid_template_identifier_component(&normalized_milestone_template_name(name))
}

fn normalized_milestone_template_name(name: &str) -> String {
    name.replace(' ', "_")
}

fn is_valid_template_identifier_component(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };

    (first.is_ascii_alphabetic() || first == '_')
        && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

struct ExistingModelInput {
    model: modality_lang::Model,
    formulas: Vec<modality_lang::FormulaExpr>,
    labels: Vec<String>,
    formula_declarations: Vec<String>,
}

fn load_existing_model_input(path: &PathBuf) -> Result<ExistingModelInput> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read existing model file {}", path.display()))?;
    let models = modality_lang::parse_all_models_content_lalrpop(&content)
        .map_err(|err| anyhow::anyhow!("Failed to parse existing model: {}", err))?;
    let formulas = modality_lang::parse_all_formulas_content_lalrpop(&content).map_err(|err| {
        anyhow::anyhow!(
            "Failed to parse formula declarations in existing model file: {}",
            err
        )
    })?;

    let model_count = models.len();
    if model_count > 1 {
        return Err(anyhow::anyhow!(
            "Expected exactly one model in {}, found {}",
            path.display(),
            model_count
        ));
    }
    let model = models
        .into_iter()
        .next()
        .ok_or_else(|| anyhow::anyhow!("No models found in {}", path.display()))?;

    let mut expressions = Vec::new();
    let mut labels = Vec::new();
    for (index, formula) in formulas.into_iter().enumerate() {
        let label = if formula.name.is_empty() {
            format!("existing F{}", index + 1)
        } else {
            format!("existing `{}`", formula.name)
        };
        expressions.push(formula.expression);
        labels.push(label);
    }

    Ok(ExistingModelInput {
        model,
        formulas: expressions,
        labels,
        formula_declarations: formula_declaration_blocks(&content),
    })
}

fn load_proposed_formula_inputs(opts: &Opts) -> Result<(ParsedFormulaInputs, Vec<String>)> {
    if let Some(formula) = &opts.proposed_formula {
        Ok((
            parse_formula_inputs(std::slice::from_ref(formula)),
            formula_declarations_for_input("proposed_formula", formula),
        ))
    } else if let Some(path) = &opts.proposed_rule {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read proposed rule file {}", path.display()))?;
        Ok((
            parse_formula_inputs(std::slice::from_ref(&content)),
            formula_declarations_for_input("proposed_rule", &content),
        ))
    } else {
        Err(anyhow::anyhow!(
            "Use --proposed-formula or --proposed-rule with --existing-model"
        ))
    }
}

fn formula_declarations_for_input(default_name: &str, content: &str) -> Vec<String> {
    let declarations = formula_declaration_blocks(content);
    if declarations.is_empty() && !content.trim().is_empty() {
        vec![format!(
            "formula {} {{\n{}\n}}",
            default_name,
            content.trim()
        )]
    } else {
        declarations
    }
}

fn formula_declaration_blocks(content: &str) -> Vec<String> {
    let lines: Vec<&str> = content
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty() && !line.starts_with("//"))
        .collect();

    let mut declarations = Vec::new();
    let mut index = 0;

    while index < lines.len() {
        let line = lines[index];
        if line.starts_with("formula ") {
            let mut formula_lines = vec![line];
            index += 1;

            while index < lines.len() {
                let line = lines[index];
                if line.starts_with("formula ") || line.starts_with("model ") {
                    break;
                }
                formula_lines.push(line);
                index += 1;
            }

            declarations.push(formula_lines.join("\n"));
        } else {
            index += 1;
        }
    }

    declarations
}

fn existing_model_unsatisfied_formula_labels(
    model: &modality_lang::Model,
    formulas: &[modality_lang::FormulaExpr],
    labels: &[String],
) -> Vec<String> {
    let checker = modality_lang::ModelChecker::new(model.clone());

    formulas
        .iter()
        .enumerate()
        .filter_map(|(index, expression)| {
            let checker_name = format!("F{}", index + 1);
            let formula = modality_lang::Formula::new(checker_name, expression.clone());
            let result = checker.check_formula(&formula);

            if result.is_satisfied {
                None
            } else {
                Some(
                    labels
                        .get(index)
                        .cloned()
                        .unwrap_or_else(|| format!("F{}", index + 1)),
                )
            }
        })
        .collect()
}

fn replacement_candidate_name(existing_model: &modality_lang::Model) -> String {
    if existing_model.name.is_empty() {
        "ContractCandidate".to_string()
    } else {
        format!("{}Candidate", existing_model.name)
    }
}

fn bounded_synthesis_options(opts: &Opts, name: &str) -> modality_synthesizer::SynthesisOptions {
    modality_synthesizer::SynthesisOptions {
        name: name.to_string(),
        max_states: opts.max_states,
        max_transitions: modality_synthesizer::SynthesisOptions::default().max_transitions,
    }
}

struct BoundedSynthesis {
    model: Option<modality_lang::Model>,
    last_candidate: Option<modality_lang::Model>,
    unsat_reason: Option<String>,
}

fn synthesize_bounded(
    formulas: &[modality_lang::FormulaExpr],
    opts: &Opts,
    name: &str,
) -> BoundedSynthesis {
    match modality_synthesizer::synthesize(formulas, bounded_synthesis_options(opts, name)) {
        modality_synthesizer::SynthesisResult::Witness(model) => BoundedSynthesis {
            model: Some(model.clone()),
            last_candidate: Some(model),
            unsat_reason: None,
        },
        modality_synthesizer::SynthesisResult::Unsat {
            reason,
            last_candidate,
            ..
        } => BoundedSynthesis {
            model: None,
            last_candidate,
            unsat_reason: Some(reason),
        },
    }
}

fn take_synthesized_model(bounded: BoundedSynthesis) -> Result<modality_lang::Model> {
    match bounded.model {
        Some(model) => Ok(model),
        None => Err(no_witness_error(
            bounded.unsat_reason.as_deref().unwrap_or("unsat"),
        )),
    }
}

fn no_witness_error(reason: &str) -> anyhow::Error {
    anyhow::anyhow!(
        "No satisfying witness found by bounded μ-calculus search for parser-backed formulas: {}",
        reason
    )
}

struct ParsedFormulaInputs {
    formulas: Vec<modality_lang::FormulaExpr>,
    labels: Vec<String>,
    unparsed: Vec<String>,
}

impl ParsedFormulaInputs {
    fn ensure_all_parsed(&self) -> Result<()> {
        if self.unparsed.is_empty() {
            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "--verify requires every input formula to parse with the Modality parser; {} unparsed: {}",
                self.unparsed.len(),
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

    fn no_valid_formulas_error(&self) -> anyhow::Error {
        if self.unparsed.is_empty() {
            anyhow::anyhow!("No valid formulas found")
        } else {
            anyhow::anyhow!(
                "No valid formulas found; parser details: {}",
                self.unparsed.join(", ")
            )
        }
    }
}

fn parse_formula_inputs(formulas: &[String]) -> ParsedFormulaInputs {
    let mut parsed_expressions = Vec::new();
    let mut labels = Vec::new();
    let mut unparsed = Vec::new();

    for (index, formula) in formulas.iter().enumerate() {
        match parse_formula_string(index, formula) {
            Ok(parsed) => {
                let preview = formula_preview(formula);
                let formula_count = parsed.len();
                for (parsed_index, formula) in parsed.into_iter().enumerate() {
                    let input_label = parsed_formula_label(
                        index,
                        parsed_index,
                        formula_count,
                        &formula.name,
                        &preview,
                    );
                    parsed_expressions.push(formula.expression);
                    labels.push(input_label);
                }
            }
            Err(parse_error) => {
                let label = format!("F{}", index + 1);
                let preview = formula_preview(formula);
                if preview.is_empty() {
                    unparsed.push(format!(
                        "{} `<empty>` ({})",
                        label,
                        compact_parse_error(&parse_error)
                    ));
                } else {
                    unparsed.push(format!(
                        "{} `{}` ({})",
                        label,
                        preview,
                        compact_parse_error(&parse_error)
                    ));
                }
            }
        }
    }

    ParsedFormulaInputs {
        formulas: parsed_expressions,
        labels,
        unparsed,
    }
}

#[cfg(test)]
fn parse_formula_strings(formulas: &[String]) -> Vec<modality_lang::FormulaExpr> {
    parse_formula_inputs(formulas).formulas
}

fn parse_formula_string(
    index: usize,
    formula: &str,
) -> Result<Vec<modality_lang::Formula>, String> {
    match modality_lang::parse_all_formulas_content_lalrpop(formula) {
        Ok(parsed) if !parsed.is_empty() => return Ok(parsed),
        Ok(_) => {}
        Err(err) => {
            if let Ok(parsed) = parse_rule_formula_blocks_for_synthesis(formula) {
                if !parsed.is_empty() {
                    return Ok(parsed);
                }
            }

            let wrapped = format!("formula generated_{} {{\n{}\n}}", index + 1, formula);
            return match modality_lang::parse_all_formulas_content_lalrpop(&wrapped) {
                Ok(parsed) if !parsed.is_empty() => Ok(parsed),
                Ok(_) => Err("wrapped expression parse produced no formulas".to_string()),
                Err(wrapped_err) => Err(format!(
                    "declared formula parse failed: {}; wrapped expression parse failed: {}",
                    err, wrapped_err
                )),
            };
        }
    }

    let wrapped = format!("formula generated_{} {{\n{}\n}}", index + 1, formula);
    match modality_lang::parse_all_formulas_content_lalrpop(&wrapped) {
        Ok(parsed) if !parsed.is_empty() => Ok(parsed),
        Ok(_) => Err("wrapped expression parse produced no formulas".to_string()),
        Err(err) => Err(format!("wrapped expression parse failed: {}", err)),
    }
}

fn parse_rule_formula_blocks_for_synthesis(
    content: &str,
) -> Result<Vec<modality_lang::Formula>, String> {
    let content = strip_line_comments(content);
    let mut formulas = Vec::new();
    let mut cursor = 0usize;

    while let Some(rule_start) = find_word_from(&content, "rule", cursor) {
        let after_rule = rule_start + "rule".len();
        if content[rule_start..].starts_with("rule_for_this_commit") {
            cursor = after_rule;
            continue;
        }

        let Some(open_brace) = content[after_rule..].find('{').map(|i| after_rule + i) else {
            break;
        };
        let Some(close_brace) = find_matching_brace(&content, open_brace) else {
            return Err("failed to parse rule formula: unmatched rule `{`".to_string());
        };

        let rule_name = rule_name_between(&content[after_rule..open_brace]);
        let rule_body = &content[open_brace + 1..close_brace];
        let mut body_cursor = 0usize;
        let mut formula_index = 1usize;

        while let Some(formula_start) = find_word_from(rule_body, "formula", body_cursor) {
            let after_formula = formula_start + "formula".len();
            let Some(formula_open) = rule_body[after_formula..]
                .find('{')
                .map(|i| after_formula + i)
            else {
                break;
            };
            let Some(formula_close) = find_matching_brace(rule_body, formula_open) else {
                return Err("failed to parse rule formula: unmatched formula `{`".to_string());
            };
            let expr = &rule_body[formula_open + 1..formula_close];
            let name = if formula_index == 1 {
                rule_name.clone()
            } else {
                format!("{}_formula_{}", rule_name, formula_index)
            };
            let formula_src = format!("formula {name} {{\n{expr}\n}}");
            let formula = modality_lang::FormulaParser::new()
                .parse(&formula_src)
                .map_err(|e| format!("failed to parse rule formula `{name}`: {:?}", e))?;
            formulas.push(formula);
            body_cursor = formula_close + 1;
            formula_index += 1;
        }

        cursor = close_brace + 1;
    }

    Ok(formulas)
}

fn strip_line_comments(content: &str) -> String {
    content
        .lines()
        .map(|line| line.split_once("//").map_or(line, |(before, _)| before))
        .collect::<Vec<_>>()
        .join("\n")
}

fn find_word_from(haystack: &str, needle: &str, start: usize) -> Option<usize> {
    let mut search_from = start;
    while let Some(offset) = haystack[search_from..].find(needle) {
        let pos = search_from + offset;
        let before = haystack[..pos].chars().next_back();
        let after = haystack[pos + needle.len()..].chars().next();
        let before_ok = before.map_or(true, |c| !is_ident_char(c));
        let after_ok = after.map_or(true, |c| !is_ident_char(c));
        if before_ok && after_ok {
            return Some(pos);
        }
        search_from = pos + needle.len();
    }
    None
}

fn find_matching_brace(content: &str, open_brace: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (idx, ch) in content[open_brace..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(open_brace + idx);
                }
            }
            _ => {}
        }
    }
    None
}

fn rule_name_between(header: &str) -> String {
    header
        .split_whitespace()
        .find(|token| token.chars().all(is_ident_char))
        .map(sanitize_formula_name)
        .unwrap_or_else(|| "default_rule".to_string())
}

fn sanitize_formula_name(raw: &str) -> String {
    let mut name = raw
        .chars()
        .map(|c| if is_ident_char(c) { c } else { '_' })
        .collect::<String>();
    if name.is_empty() {
        name.push_str("rule");
    }
    if name.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        name.insert(0, '_');
    }
    name
}

fn is_ident_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

fn formula_preview(formula: &str) -> String {
    const MAX_PREVIEW_LEN: usize = 80;

    let preview = formula.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut chars = preview.chars();
    let truncated: String = chars.by_ref().take(MAX_PREVIEW_LEN).collect();
    if chars.next().is_some() {
        format!("{}...", truncated)
    } else {
        truncated
    }
}

#[derive(Default)]
struct FormulaFactSummary {
    actions: BTreeSet<String>,
    predicates: BTreeSet<String>,
    external_predicates: BTreeSet<String>,
    state_atoms: BTreeSet<String>,
}

impl FormulaFactSummary {
    fn from_formulas(formulas: &[modality_lang::FormulaExpr]) -> Self {
        let mut summary = Self::default();
        for formula in formulas {
            summary.visit_expr(formula);
        }
        summary
    }

    fn write_markdown(&self, output: &mut String) {
        output.push_str("- Action labels:\n");
        write_fact_list(output, &self.actions);
        output.push_str("- Predicate calls:\n");
        write_fact_list(output, &self.predicates);
        output.push_str("- External evidence predicates:\n");
        write_fact_list(output, &self.external_predicates);
        output.push_str("- Opaque witness atoms:\n");
        write_fact_list(output, &self.state_atoms);
        output.push('\n');
    }

    fn visit_expr(&mut self, expr: &modality_lang::FormulaExpr) {
        use modality_lang::FormulaExpr;

        match expr {
            FormulaExpr::True | FormulaExpr::False | FormulaExpr::Var(_) => {}
            FormulaExpr::Prop(atom) => {
                self.state_atoms.insert(atom.clone());
            }
            FormulaExpr::And(left, right)
            | FormulaExpr::Or(left, right)
            | FormulaExpr::Implies(left, right)
            | FormulaExpr::Until(left, right) => {
                self.visit_expr(left);
                self.visit_expr(right);
            }
            FormulaExpr::Not(inner)
            | FormulaExpr::Paren(inner)
            | FormulaExpr::Lfp(_, inner)
            | FormulaExpr::Gfp(_, inner)
            | FormulaExpr::Eventually(inner)
            | FormulaExpr::Always(inner)
            | FormulaExpr::Next(inner) => {
                self.visit_expr(inner);
            }
            FormulaExpr::Diamond(properties, inner)
            | FormulaExpr::Box(properties, inner)
            | FormulaExpr::DiamondBox(properties, inner) => {
                self.visit_properties(properties);
                self.visit_expr(inner);
            }
        }
    }

    fn visit_properties(&mut self, properties: &[modality_lang::Property]) {
        for property in properties {
            let rendered = render_property_fact(property);
            if property.is_predicate() {
                self.predicates.insert(rendered.clone());
                if is_external_evidence_predicate(&property.name) {
                    self.external_predicates.insert(rendered);
                }
            } else {
                self.actions.insert(rendered);
            }
        }
    }
}

fn write_fact_list(output: &mut String, facts: &BTreeSet<String>) {
    if facts.is_empty() {
        output.push_str("  - none\n");
        return;
    }

    for fact in facts {
        output.push_str(&format!("  - `{}`\n", fact));
    }
}

fn render_property_fact(property: &modality_lang::Property) -> String {
    let sign = match property.sign {
        modality_lang::PropertySign::Plus => "+",
        modality_lang::PropertySign::Minus => "-",
    };

    if let Some((_, args)) = property.get_predicate() {
        if let Some(arg) = args.get("arg").and_then(|value| value.as_str()) {
            return format!(
                "{}{}({})",
                sign,
                modality_lang::ast::predicate_display_name(&property.name),
                render_predicate_arg(arg)
            );
        }

        if let Some(args) = args.get("args").and_then(|value| value.as_array()) {
            let rendered_args = args
                .iter()
                .filter_map(|value| value.as_str())
                .map(render_predicate_arg)
                .collect::<Vec<_>>()
                .join(", ");
            return format!(
                "{}{}({})",
                sign,
                modality_lang::ast::predicate_display_name(&property.name),
                rendered_args
            );
        }
    }

    format!("{}{}", sign, property.name)
}

fn render_predicate_arg(arg: &str) -> String {
    if is_identifier_arg(arg) || is_path_arg(arg) {
        arg.to_string()
    } else {
        format!("\"{}\"", arg.replace('\\', "\\\\").replace('"', "\\\""))
    }
}

fn is_identifier_arg(value: &str) -> bool {
    let mut chars = value.chars();
    match chars.next() {
        Some(first) if first == '_' || first.is_ascii_alphabetic() => {}
        _ => return false,
    }
    chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

fn is_path_arg(value: &str) -> bool {
    value.starts_with('/')
        && value
            .chars()
            .skip(1)
            .all(|ch| ch == '_' || ch == '.' || ch == '/' || ch.is_ascii_alphanumeric())
}

fn is_external_evidence_predicate(name: &str) -> bool {
    !matches!(
        name,
        "signed_by"
            | "any_signed"
            | "all_signed"
            | "threshold"
            | "modifies"
            | "adds_rule"
            | "post_to"
    )
}

fn parsed_formula_label(
    input_index: usize,
    parsed_index: usize,
    formula_count: usize,
    formula_name: &str,
    preview: &str,
) -> String {
    let input_label = if formula_count == 1 {
        format!("F{}", input_index + 1)
    } else {
        format!("F{}.{}", input_index + 1, parsed_index + 1)
    };

    let detail = if !formula_name.starts_with("generated_") && !formula_name.is_empty() {
        formula_name.to_string()
    } else if preview.is_empty() {
        "<empty>".to_string()
    } else {
        preview.to_string()
    };

    format!("{} `{}`", input_label, detail)
}

fn compact_parse_error(error: &str) -> String {
    const MAX_ERROR_LEN: usize = 160;

    let compact = error.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut chars = compact.chars();
    let truncated: String = chars.by_ref().take(MAX_ERROR_LEN).collect();
    if chars.next().is_some() {
        format!("parser: {}...", truncated)
    } else {
        format!("parser: {}", truncated)
    }
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

fn format_synthesized_model_with_formulas(
    model: &modality_lang::Model,
    format: &str,
    formula_declarations: &[String],
) -> Result<String> {
    if format == "json" && !formula_declarations.is_empty() {
        return Ok(serde_json::to_string_pretty(&serde_json::json!({
            "model": model,
            "formula_declarations": formula_declarations,
        }))?);
    }

    let mut output = format_synthesized_model(model, format)?;

    if format == "modality" && !formula_declarations.is_empty() {
        output = output.trim_end().to_string();
        for declaration in formula_declarations {
            output.push_str("\n\n");
            output.push_str(declaration.trim());
        }
        output.push('\n');
    }

    Ok(output)
}

fn verify_synthesized_model_with_labels(
    model: &modality_lang::Model,
    formulas: &[modality_lang::FormulaExpr],
    labels: &[String],
) -> Result<()> {
    println!(
        "🔎 Verifying synthesized model against {} formula(s)",
        formulas.len()
    );

    let checker = modality_lang::ModelChecker::new(model.clone());
    let mut failed = Vec::new();

    for (index, expression) in formulas.iter().enumerate() {
        let formula_name = labels
            .get(index)
            .cloned()
            .unwrap_or_else(|| format!("F{}", index + 1));
        let checker_name = format!("F{}", index + 1);
        let formula = modality_lang::Formula::new(checker_name, expression.clone());
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
        write_output_file(output, output_path)?;
        println!("✅ Synthesized model written to {}", output_path.display());
    } else {
        println!("{}", output);
    }

    Ok(())
}

fn write_llm_review_bundle_if_requested(
    review_bundle_path: Option<&PathBuf>,
    source_label: &str,
    source_response: &str,
    review_source: Option<&ReviewSource>,
    extracted_formulas: &[String],
    parsed_input: &ParsedFormulaInputs,
    model_output: &str,
    model_format: &str,
) -> Result<()> {
    let Some(review_bundle_path) = review_bundle_path else {
        return Ok(());
    };

    let bundle = format_llm_review_bundle(
        source_label,
        source_response,
        review_source,
        extracted_formulas,
        parsed_input,
        model_output,
        model_format,
    );
    write_output_file(&bundle, review_bundle_path)?;
    println!(
        "✅ Synthesis review bundle written to {}",
        review_bundle_path.display()
    );

    Ok(())
}

fn write_rule_failed_review_bundle_if_requested(
    review_bundle_path: Option<&PathBuf>,
    rule_path: &PathBuf,
    rule_content: &str,
    review_source: Option<&ReviewSource>,
    parsed_input: &ParsedFormulaInputs,
    model_output: &str,
    model_format: &str,
    verifier_error: &anyhow::Error,
) -> Result<()> {
    let Some(review_bundle_path) = review_bundle_path else {
        return Ok(());
    };

    let bundle = format_failed_rule_review_bundle(
        rule_path,
        rule_content,
        review_source,
        parsed_input,
        model_output,
        model_format,
        verifier_error,
    );
    write_output_file(&bundle, review_bundle_path)?;
    println!(
        "⚠️  Synthesis failure review bundle written to {}",
        review_bundle_path.display()
    );

    Ok(())
}

fn format_failed_rule_review_bundle(
    rule_path: &PathBuf,
    rule_content: &str,
    review_source: Option<&ReviewSource>,
    parsed_input: &ParsedFormulaInputs,
    model_output: &str,
    model_format: &str,
    verifier_error: &anyhow::Error,
) -> String {
    let mut output = String::new();
    output.push_str("# Modality Synthesis Review Bundle\n\n");

    output.push_str("## Original Source\n\n");
    if let Some(review_source) = review_source {
        output.push_str(&format!("- Input: `{}`\n\n", review_source.label));
        output.push_str("```text\n");
        output.push_str(review_source.content.trim());
        output.push_str("\n```\n\n");
    } else {
        output.push_str(
            "- Not supplied. Use `--source-text` or `--source-file` with `--review-bundle` to capture reviewer context.\n\n",
        );
    }

    output.push_str("## Rule File\n\n");
    output.push_str(&format!("- Input: `--rule {}`\n\n", rule_path.display()));
    output.push_str("```text\n");
    output.push_str(rule_content.trim());
    output.push_str("\n```\n\n");

    output.push_str("## Extracted Facts\n\n");
    output.push_str(
        "- Extraction source: parser-backed formula AST, not inferred natural language.\n\n",
    );
    let facts = FormulaFactSummary::from_formulas(&parsed_input.formulas);
    facts.write_markdown(&mut output);

    output.push_str("## Review Checklist\n\n");
    write_review_checklist(
        &mut output,
        review_source,
        parsed_input.formulas.len(),
        parsed_input.formulas.len(),
        false,
    );

    output.push_str("## Parser Result\n\n");
    output.push_str(&format!(
        "- Parsed formulas: {}\n",
        parsed_input.formulas.len()
    ));
    output.push_str(&format!(
        "- Unparsed formulas: {}\n",
        parsed_input.unparsed.len()
    ));
    if !parsed_input.labels.is_empty() {
        output.push_str("- Parsed labels:\n");
        for label in &parsed_input.labels {
            output.push_str(&format!("  - {}\n", label));
        }
    }
    output.push('\n');

    output.push_str("## Verifier Result\n\n");
    output.push_str("- Status: failed (`--verify`)\n");
    output.push_str(
        "- Outcome: no satisfying witness was found by bounded μ-calculus search.\n",
    );
    output.push_str(&format!("- Verifier error: `{}`\n\n", verifier_error));

    output.push_str("## Candidate Witness Model\n\n");
    output.push_str(&format!("```{}\n", model_format));
    output.push_str(model_output.trim());
    output.push_str("\n```\n\n");

    output.push_str("## Assumptions\n\n");
    output.push_str("- Predicate meanings come from the runtime predicate library, not from the rule text alone.\n");
    output.push_str("- Signature, path, oracle, and external-world facts must still be supplied by contract evidence at verification time.\n\n");

    output.push_str("## Known Gaps\n\n");
    output.push_str("- This is bounded explicit-state μ-calculus search, not a complete model finder.\n");
    output.push_str(
        "- Review the candidate, formula shape, and predicate assumptions before raising `--max-states`.\n",
    );

    output
}

fn write_rule_review_bundle_if_requested(
    review_bundle_path: Option<&PathBuf>,
    rule_path: &PathBuf,
    rule_content: &str,
    review_source: Option<&ReviewSource>,
    parsed_input: &ParsedFormulaInputs,
    model_output: &str,
    model_format: &str,
) -> Result<()> {
    let Some(review_bundle_path) = review_bundle_path else {
        return Ok(());
    };

    let formulas: Vec<String> = if parsed_input.labels.is_empty() {
        Vec::new()
    } else {
        parsed_input.labels.clone()
    };
    let bundle = format_synthesis_review_bundle(
        "Rule File",
        &format!("--rule {}", rule_path.display()),
        "rule file supplied by the reviewer",
        rule_content,
        review_source,
        &formulas,
        parsed_input,
        model_output,
        model_format,
    );
    write_output_file(&bundle, review_bundle_path)?;
    println!(
        "✅ Synthesis review bundle written to {}",
        review_bundle_path.display()
    );

    Ok(())
}

fn format_llm_review_bundle(
    source_label: &str,
    source_response: &str,
    review_source: Option<&ReviewSource>,
    extracted_formulas: &[String],
    parsed_input: &ParsedFormulaInputs,
    model_output: &str,
    model_format: &str,
) -> String {
    format_synthesis_review_bundle(
        "LLM Response",
        source_label,
        "LLM response text supplied by the reviewer",
        source_response,
        review_source,
        extracted_formulas,
        parsed_input,
        model_output,
        model_format,
    )
}

fn format_synthesis_review_bundle(
    input_heading: &str,
    input_label: &str,
    input_type: &str,
    input_content: &str,
    review_source: Option<&ReviewSource>,
    extracted_formulas: &[String],
    parsed_input: &ParsedFormulaInputs,
    model_output: &str,
    model_format: &str,
) -> String {
    let mut output = String::new();
    output.push_str("# Modality Synthesis Review Bundle\n\n");

    output.push_str("## Original Source\n\n");
    if let Some(review_source) = review_source {
        output.push_str(&format!("- Input: `{}`\n", review_source.label));
        output.push_str(
            "- Source type: original prompt, source clause, or reviewer-supplied context before formula extraction\n\n",
        );
        output.push_str("```text\n");
        output.push_str(review_source.content.trim());
        output.push_str("\n```\n\n");
    } else {
        output.push_str(
            "- Not supplied. Use `--source-text` or `--source-file` with `--review-bundle` to capture the prompt or source clause that produced the LLM response.\n\n",
        );
    }

    output.push_str(&format!("## {}\n\n", input_heading));
    output.push_str(&format!("- Input: `{}`\n", input_label));
    output.push_str(&format!("- Source type: {}\n\n", input_type));
    output.push_str("```text\n");
    output.push_str(input_content.trim());
    output.push_str("\n```\n\n");

    output.push_str("## Extracted Facts\n\n");
    output.push_str(
        "- Extraction source: parser-backed formula AST, not inferred natural language.\n",
    );
    output.push_str(
        "- These facts summarize the modal actions, predicates, and opaque atoms that drive the witness search.\n\n",
    );
    let facts = FormulaFactSummary::from_formulas(&parsed_input.formulas);
    facts.write_markdown(&mut output);

    output.push_str("## Source Clause Trace\n\n");
    write_source_clause_trace(&mut output, review_source, extracted_formulas.len());

    output.push_str("## Review Checklist\n\n");
    write_review_checklist(
        &mut output,
        review_source,
        extracted_formulas.len(),
        parsed_input.formulas.len(),
        true,
    );

    output.push_str("## Extracted Formulas\n\n");
    for (index, formula) in extracted_formulas.iter().enumerate() {
        output.push_str(&format!("{}. `{}`\n", index + 1, formula_preview(formula)));
    }
    output.push('\n');

    output.push_str("## Parser Result\n\n");
    output.push_str(&format!(
        "- Parsed formulas: {}\n",
        parsed_input.formulas.len()
    ));
    output.push_str(&format!(
        "- Unparsed formulas: {}\n",
        parsed_input.unparsed.len()
    ));
    if !parsed_input.labels.is_empty() {
        output.push_str("- Parsed labels:\n");
        for label in &parsed_input.labels {
            output.push_str(&format!("  - {}\n", label));
        }
    }
    if !parsed_input.unparsed.is_empty() {
        output.push_str("- Unparsed details:\n");
        for detail in &parsed_input.unparsed {
            output.push_str(&format!("  - {}\n", detail));
        }
    }
    output.push('\n');

    output.push_str("## Verifier Result\n\n");
    output.push_str("- Status: passed (`--verify`)\n");
    output.push_str("- Scope: synthesized witness model checked against every parser-backed extracted formula\n\n");

    output.push_str("## Witness Model\n\n");
    output.push_str(&format!("```{}\n", model_format));
    output.push_str(model_output.trim());
    output.push_str("\n```\n\n");

    output.push_str("## Assumptions\n\n");
    output.push_str("- Predicate meanings come from the runtime predicate library, not from the LLM response.\n");
    output.push_str("- Signature, path, oracle, and external-world facts must be supplied by contract evidence at verification time.\n\n");

    output.push_str("## Known Gaps\n\n");
    output.push_str(
        "- Structured `F1:` source-clause trace lines are preserved for reviewer traceability, but automatic natural-language-to-facts extraction is not available in this path yet; the fact summary starts after formula extraction.\n",
    );
    output.push_str("- Passing synthesis proves the witness model satisfies the extracted formulas; it does not prove the extracted formulas capture the original intent.\n");

    output
}

fn write_source_clause_trace(
    output: &mut String,
    review_source: Option<&ReviewSource>,
    formula_count: usize,
) {
    let Some(review_source) = review_source else {
        output.push_str(
            "- No original source was supplied, so no source-clause trace is available.\n\n",
        );
        return;
    };

    let trace = extract_source_clause_trace(&review_source.content, formula_count);
    if trace.iter().all(Option::is_none) {
        output.push_str(
            "- No structured source-clause lines found. Prefix source lines with `F1:`, `F2:`, etc. to preserve reviewer-authored clause-to-formula traceability.\n\n",
        );
        return;
    }

    output.push_str(
        "- Trace source: reviewer-authored formula labels in the original source text.\n",
    );
    output.push_str("- These clauses are preserved for review; they are not inferred by the natural-language parser.\n\n");

    for (index, clause) in trace.iter().enumerate() {
        let formula_label = format!("F{}", index + 1);
        match clause {
            Some(clause) => {
                output.push_str(&format!(
                    "- {} source clause: {}\n",
                    formula_label,
                    clause.trim()
                ));
            }
            None => {
                output.push_str(&format!(
                    "- {} source clause: not supplied in structured source text\n",
                    formula_label
                ));
            }
        }
    }
    output.push('\n');
}

fn write_review_checklist(
    output: &mut String,
    review_source: Option<&ReviewSource>,
    formula_count: usize,
    parsed_formula_count: usize,
    verifier_passed: bool,
) {
    output.push_str("- Original source captured: ");
    output.push_str(if review_source.is_some() {
        "yes\n"
    } else {
        "no\n"
    });

    output.push_str("- Source-clause trace present: ");
    let trace_present = review_source
        .map(|source| {
            extract_source_clause_trace(&source.content, formula_count)
                .iter()
                .any(Option::is_some)
        })
        .unwrap_or(false);
    output.push_str(if trace_present { "yes\n" } else { "no\n" });
    output.push_str(
        "- Prompt-to-facts trace: not automatic; review preserved clauses against parser-backed formulas\n",
    );

    output.push_str(&format!(
        "- Parser-backed formulas: {}\n",
        parsed_formula_count
    ));
    output.push_str(if verifier_passed {
        "- Verifier result: passed\n"
    } else {
        "- Verifier result: failed\n"
    });
    output.push_str("- Assumptions section present: yes\n");
    output.push_str("- Known gaps section present: yes\n\n");
}

fn extract_source_clause_trace(source: &str, formula_count: usize) -> Vec<Option<String>> {
    let mut trace = vec![None; formula_count];
    for line in source.lines() {
        let trimmed = line.trim();
        let Some(after_f) = trimmed.strip_prefix('F') else {
            continue;
        };

        let digit_count = after_f.chars().take_while(|ch| ch.is_ascii_digit()).count();
        if digit_count == 0 {
            continue;
        }

        let (digits, rest) = after_f.split_at(digit_count);
        let Ok(formula_number) = digits.parse::<usize>() else {
            continue;
        };
        if formula_number == 0 || formula_number > formula_count {
            continue;
        }

        let rest = rest.trim_start();
        let Some(clause) = rest
            .strip_prefix(':')
            .or_else(|| rest.strip_prefix('-'))
            .or_else(|| rest.strip_prefix('.'))
        else {
            continue;
        };

        let clause = clause.trim();
        if !clause.is_empty() {
            trace[formula_number - 1] = Some(clause.to_string());
        }
    }

    trace
}

fn write_output_file_if_requested(output: &str, output_path: Option<&PathBuf>) -> Result<()> {
    if let Some(output_path) = output_path {
        write_output_file(output, output_path)?;
        println!("✅ Synthesized model written to {}", output_path.display());
    }

    Ok(())
}

fn write_output_file(output: &str, output_path: &PathBuf) -> Result<()> {
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create output directory {}", parent.display()))?;
    }
    std::fs::write(output_path, output).with_context(|| {
        format!(
            "Failed to write synthesized model to {}",
            output_path.display()
        )
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_test_opts() -> Opts {
        Opts {
            template: None,
            describe: None,
            rule: None,
            existing_model: None,
            proposed_formula: None,
            proposed_rule: None,
            formulas: None,
            generate_prompt: false,
            llm_response: None,
            llm_response_file: None,
            source_text: None,
            source_file: None,
            output: None,
            review_bundle: None,
            verify: false,
            max_states: 4,
            party_a: "Alice".to_string(),
            party_b: "Bob".to_string(),
            milestones: None,
            format: "modality".to_string(),
            list: false,
        }
    }

    #[test]
    fn synthesize_opts_restricts_output_format_values() {
        let json_opts =
            Opts::try_parse_from(["synthesize", "--format", "json"]).expect("json format parses");
        assert_eq!(json_opts.format, "json");

        let err = Opts::try_parse_from(["synthesize", "--format", "yaml"]).unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::InvalidValue);
    }

    #[test]
    fn synthesize_opts_restricts_template_values() {
        let milestone_opts = Opts::try_parse_from(["synthesize", "--template", "milestone"])
            .expect("milestone template parses");
        assert_eq!(milestone_opts.template.as_deref(), Some("milestone"));

        let err =
            Opts::try_parse_from(["synthesize", "--template", "made_up_template"]).unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::InvalidValue);
    }

    #[tokio::test]
    async fn no_input_error_lists_current_synthesis_modes() {
        let opts = default_test_opts();
        let err = run(&opts).await.unwrap_err();
        let message = err.to_string();
        assert!(message.contains("--template"));
        assert!(message.contains("--describe"));
        assert!(message.contains("--rule"));
        assert!(message.contains("--formulas"));
        assert!(message.contains("--llm-response"));
        assert!(message.contains("--llm-response-file"));
        assert!(message.contains("--list/--generate-prompt"));
    }

    #[test]
    fn parse_formula_strings_uses_modality_parser() {
        let formulas = vec![
            "always([<+APPROVE>] true)".to_string(),
            "eventually(<+DELIVER> true)".to_string(),
        ];
        let parsed = parse_formula_strings(&formulas);
        assert_eq!(parsed.len(), 2);
    }

    #[test]
    fn parse_formula_strings_accepts_declared_formulas() {
        let formulas = vec!["formula existing_rule {\nalways([<+APPROVE>] true)\n}".to_string()];
        let parsed = parse_formula_strings(&formulas);
        assert_eq!(parsed.len(), 1);
    }

    #[tokio::test]
    async fn formulas_mode_synthesizes_with_bounded_search() {
        let output_path = std::env::temp_dir().join(format!(
            "modality-synthesize-formulas-{}.modality",
            std::process::id()
        ));
        let mut opts = default_test_opts();
        opts.formulas = Some("always(<+A> true)".to_string());
        opts.verify = true;
        opts.output = Some(output_path.clone());

        run(&opts).await.unwrap();
        let output = std::fs::read_to_string(&output_path).unwrap();
        std::fs::remove_file(&output_path).ok();
        assert!(output.contains("model Contract"));
        assert!(output.contains("q0 --> q0"));
    }

    #[tokio::test]
    async fn rule_file_verify_writes_checked_model() {
        let rule_path = std::env::temp_dir().join(format!(
            "modality-synthesize-rule-{}.modality",
            std::process::id()
        ));
        let output_path = std::env::temp_dir().join(format!(
            "modality-synthesize-rule-output-{}.modality",
            std::process::id()
        ));
        std::fs::write(
            &rule_path,
            r#"
rule authorized {
  formula {
    [] always([-signed_by(/parties/alice.id) -signed_by(/parties/bob.id)] false)
  }
}
"#,
        )
        .unwrap();

        let mut opts = default_test_opts();
        opts.rule = Some(rule_path.clone());
        opts.verify = true;
        opts.output = Some(output_path.clone());

        run(&opts).await.unwrap();
        let output = std::fs::read_to_string(&output_path).unwrap();
        std::fs::remove_file(&rule_path).ok();
        std::fs::remove_file(&output_path).ok();
        assert!(output.contains("model Contract"));
        assert!(output.contains("+signed_by(/parties/alice.id)") || output.contains("+signed_by(/parties/bob.id)"));
    }

    #[tokio::test]
    async fn rule_file_verify_writes_review_bundle() {
        let rule_path = std::env::temp_dir().join(format!(
            "modality-synthesize-review-rule-{}.modality",
            std::process::id()
        ));
        let source_path = std::env::temp_dir().join(format!(
            "modality-synthesize-review-source-{}.txt",
            std::process::id()
        ));
        let output_path = std::env::temp_dir().join(format!(
            "modality-synthesize-review-output-{}.modality",
            std::process::id()
        ));
        let bundle_path = std::env::temp_dir().join(format!(
            "modality-synthesize-review-bundle-{}.md",
            std::process::id()
        ));
        std::fs::write(
            &rule_path,
            r#"
rule post_requires_reviewer {
  formula {
    always([+POST] true -> <+signed_by(/users/reviewer.id)> true)
  }
}
"#,
        )
        .unwrap();
        std::fs::write(
            &source_path,
            "F1: Every accepted post move must have reviewer signature evidence attached.\n",
        )
        .unwrap();

        let mut opts = default_test_opts();
        opts.rule = Some(rule_path.clone());
        opts.source_file = Some(source_path.clone());
        opts.verify = true;
        opts.output = Some(output_path.clone());
        opts.review_bundle = Some(bundle_path.clone());

        run(&opts).await.unwrap();
        let bundle = std::fs::read_to_string(&bundle_path).unwrap();
        std::fs::remove_file(&rule_path).ok();
        std::fs::remove_file(&source_path).ok();
        std::fs::remove_file(&output_path).ok();
        std::fs::remove_file(&bundle_path).ok();

        assert!(bundle.contains("# Modality Synthesis Review Bundle"));
        assert!(bundle.contains("## Original Source"));
        assert!(bundle.contains("Every accepted post move must have reviewer signature evidence attached."));
        assert!(bundle.contains("post_requires_reviewer"));
        assert!(bundle.contains("`+POST`"));
        assert!(bundle.contains("`+signed_by(/users/reviewer.id)`"));
        assert!(bundle.contains("- Verifier result: passed"));
        assert!(bundle.contains("## Witness Model"));
        assert!(bundle.contains("model Contract"));
    }

    #[tokio::test]
    async fn rule_file_verify_writes_failed_bundle_for_false() {
        let rule_path = std::env::temp_dir().join(format!(
            "modality-synthesize-unsat-rule-{}.modality",
            std::process::id()
        ));
        let bundle_path = std::env::temp_dir().join(format!(
            "modality-synthesize-unsat-bundle-{}.md",
            std::process::id()
        ));
        std::fs::write(
            &rule_path,
            r#"
rule impossible_contract {
  formula {
    false
  }
}
"#,
        )
        .unwrap();

        let mut opts = default_test_opts();
        opts.rule = Some(rule_path.clone());
        opts.verify = true;
        opts.review_bundle = Some(bundle_path.clone());

        let err = run(&opts).await.unwrap_err();
        let bundle = std::fs::read_to_string(&bundle_path).unwrap();
        std::fs::remove_file(&rule_path).ok();
        std::fs::remove_file(&bundle_path).ok();

        assert!(err.to_string().contains("No satisfying witness found by bounded"));
        assert!(bundle.contains("Verifier result: failed"), "{bundle}");
        assert!(
            bundle.contains("bounded μ-calculus search"),
            "bundle was:\n{bundle}"
        );
        assert!(
            bundle.contains("bounded explicit-state μ-calculus search"),
            "bundle was:\n{bundle}"
        );
    }

    #[tokio::test]
    async fn list_mode_rejects_other_modes() {
        let mut opts = default_test_opts();
        opts.list = true;
        opts.rule = Some(PathBuf::from("rules.modality"));
        let err = run(&opts).await.unwrap_err();
        assert!(err
            .to_string()
            .contains("--list cannot be combined with other synthesis modes: --rule"));
    }

    #[test]
    fn synthesis_list_includes_templates_and_core_examples() {
        let list = synthesis_list_text();
        assert!(list.contains("escrow"));
        assert!(list.contains("always([<+APPROVE>] true)"));
        assert!(list.contains("--existing-model"));
        assert!(list.contains("--review-bundle"));
    }

    #[test]
    fn source_clause_trace_preserves_structured_formula_labels() {
        let trace = extract_source_clause_trace(
            r#"
Intro text without a formula id.
F1: Approval must be available.
F2 - Approval requires a reviewer signature.
F4: Ignored because only three formulas were extracted.
F3. Approval requires external review evidence.
"#,
            3,
        );
        assert_eq!(
            trace,
            vec![
                Some("Approval must be available.".to_string()),
                Some("Approval requires a reviewer signature.".to_string()),
                Some("Approval requires external review evidence.".to_string()),
            ]
        );
    }

    #[tokio::test]
    async fn review_bundle_requires_verified_rule_or_llm_mode() {
        let mut opts = default_test_opts();
        opts.formulas = Some("always([<+APPROVE>] true)".to_string());
        opts.review_bundle = Some(PathBuf::from("review.md"));
        opts.verify = true;
        let err = run(&opts).await.unwrap_err();
        assert!(err
            .to_string()
            .contains("--review-bundle requires --rule, --llm-response, or --llm-response-file"));
    }

    #[test]
    fn existing_model_mode_rejects_other_synthesis_modes() {
        let mut opts = default_test_opts();
        opts.existing_model = Some(PathBuf::from("model.modality"));
        opts.proposed_formula = Some("always(<+A> true)".to_string());
        opts.template = Some("escrow".to_string());
        let err = existing_model_mode_conflicts(&opts);
        assert!(err.contains(&"--template"));
    }

    #[test]
    fn verify_requires_every_input_formula_to_parse() {
        let parsed = parse_formula_inputs(&["not a formula {{{".to_string()]);
        let err = parsed.ensure_all_parsed().unwrap_err();
        assert!(err.to_string().contains("--verify requires every input formula to parse"));
    }

    #[test]
    fn format_synthesized_model_supports_json() {
        let model = modality_lang::Model::new("Contract".to_string());
        let json = format_synthesized_model(&model, "json").unwrap();
        assert!(json.contains("\"name\""));
    }
}
