use anyhow::{Context, Result};
use clap::Parser;
use std::collections::HashSet;
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
    #[arg(short, long, default_value = "modality", value_parser = ["modality", "json"])]
    pub format: String,

    /// List available templates
    #[arg(short, long)]
    pub list: bool,
}

pub async fn run(opts: &Opts) -> Result<()> {
    ensure_output_format_is_supported(&opts.format)?;

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

        // Extract constraints and synthesize
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "Contract",
            &parsed_input.formulas,
        );

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
        if !parsed_input.formulas.is_empty() {
            let model = modality_lang::formula_synthesis::synthesize_from_formulas(
                "Contract",
                &parsed_input.formulas,
            );

            if opts.verify {
                verify_synthesized_model_with_labels(
                    &model,
                    &parsed_input.formulas,
                    &parsed_input.labels,
                )?;
                println!();
            }

            let output = format_synthesized_model(&model, &opts.format)?;
            write_or_print_model(&output, opts.output.as_ref())?;
        } else {
            if opts.verify {
                return Err(anyhow::anyhow!(
                    "--verify requires formulas that can be parsed by the Modality parser"
                ));
            }

            // Fallback to old heuristic approach
            let model = synthesize_from_rule(&content, &opts.party_a, &opts.party_b)?;
            let output = format_model(&model, &opts.format)?;
            write_or_print_model(&output, opts.output.as_ref())?;
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
            r#"always([<+APPROVE>] true & [<+REJECT>] true)"#,
            r#"[<+APPROVE>] true"#,
            r#"<+APPROVE> true"#,
        ],
    },
    FormulaExampleGroup {
        title: r#"Alternatives and temporal candidates"#,
        description: r#"Choices, mixed permissive/committed actions, and next-step candidates."#,
        formulas: &[
            r#"<+APPROVE> true | <+REJECT> true"#,
            r#"<+APPROVE> true | [<+REJECT>] true"#,
            r#"(<+APPROVE> true | [<+REJECT>] true) & ([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"#,
            r#"next(<+APPROVE> true)"#,
            r#"next((<+APPROVE> true | [<+REJECT>] true))"#,
            r#"<+WAIT> true until <+APPROVE> true"#,
        ],
    },
    FormulaExampleGroup {
        title: r#"Ordering and eventual goals"#,
        description: r#"Requests, releases, and future actions the model should make reachable."#,
        formulas: &[
            r#"[+REQUEST] true -> eventually((<+APPROVE> true | [<+REJECT>] true))"#,
            r#"<+CANCEL> true & ([+RELEASE] true -> eventually(<+DELIVER> true))"#,
            r#"[<+RELEASE>] true -> eventually(<+DELIVER> true)"#,
            r#"[<+RELEASE>] true -> eventually([<+DELIVER>] true)"#,
            r#"[+RELEASE] true -> eventually([<+DELIVER>] true)"#,
            r#"always([+DELIVER] true -> eventually(<+DEPOSIT> true))"#,
            r#"always([+RELEASE] true -> eventually(<+DELIVER> true))"#,
            r#"always([+SUBMIT] true -> eventually(<+REVIEW> true))"#,
            r#"always([+APPROVE] true -> eventually(<+PUBLISH> true))"#,
            r#"always([+MERGE] true -> eventually(<+DEPLOY> true))"#,
            r#"always([+OPEN_ISSUE] true -> eventually(<+TRIAGE> true))"#,
            r#"always([+TRIAGE] true -> eventually(<+ASSIGN> true))"#,
            r#"always([+FIX] true -> eventually(<+VERIFY> true))"#,
            r#"always([+ALERT] true -> eventually(<+ACKNOWLEDGE> true))"#,
            r#"always([+ACKNOWLEDGE] true -> eventually(<+MITIGATE> true))"#,
            r#"always([+MITIGATE] true -> eventually(<+RESOLVE> true))"#,
            r#"always([+CREATE_ORDER] true -> eventually(<+APPROVE_ORDER> true))"#,
            r#"always([+APPROVE_ORDER] true -> eventually(<+FULFILL_ORDER> true))"#,
            r#"always([+FULFILL_ORDER] true -> eventually(<+PAY_INVOICE> true))"#,
            r#"always([+INGEST_DATA] true -> eventually(<+VALIDATE_DATA> true))"#,
            r#"always([+VALIDATE_DATA] true -> eventually(<+TRANSFORM_DATA> true))"#,
            r#"always([+TRANSFORM_DATA] true -> eventually(<+PUBLISH_DATASET> true))"#,
            r#"always([+INVITE_MEMBER] true -> eventually(<+ACCEPT_INVITE> true))"#,
            r#"always([+ACCEPT_INVITE] true -> eventually(<+PROVISION_ACCESS> true))"#,
            r#"always([+PROVISION_ACCESS] true -> eventually(<+COMPLETE_ONBOARDING> true))"#,
            r#"always([+PLAN_RELEASE] true -> eventually(<+APPROVE_QA> true))"#,
            r#"always([+APPROVE_QA] true -> eventually(<+ROLLOUT_RELEASE> true))"#,
            r#"always([+ROLLOUT_RELEASE] true -> eventually(<+MONITOR_RELEASE> true))"#,
            r#"always([+OPEN_TICKET] true -> eventually(<+ASSIGN_AGENT> true))"#,
            r#"always([+ASSIGN_AGENT] true -> eventually(<+RESPOND_TICKET> true))"#,
            r#"always([+RESPOND_TICKET] true -> eventually(<+RESOLVE_TICKET> true))"#,
            r#"always([+START_AUDIT] true -> eventually(<+COLLECT_EVIDENCE> true))"#,
            r#"always([+COLLECT_EVIDENCE] true -> eventually(<+REVIEW_EVIDENCE> true))"#,
            r#"always([+REVIEW_EVIDENCE] true -> eventually(<+CLOSE_AUDIT> true))"#,
            r#"always([+SUBMIT_EXPENSE] true -> eventually(<+APPROVE_EXPENSE> true))"#,
            r#"always([+APPROVE_EXPENSE] true -> eventually(<+REIMBURSE_EXPENSE> true))"#,
            r#"always([+REIMBURSE_EXPENSE] true -> eventually(<+CLOSE_EXPENSE> true))"#,
            r#"always([+ENROLL_TRAINING] true -> eventually(<+COMPLETE_TRAINING> true))"#,
            r#"always([+COMPLETE_TRAINING] true -> eventually(<+PASS_ASSESSMENT> true))"#,
            r#"always([+PASS_ASSESSMENT] true -> eventually(<+ISSUE_CERTIFICATE> true))"#,
            r#"always([+SCHEDULE_MAINTENANCE] true -> eventually(<+PERFORM_MAINTENANCE> true))"#,
            r#"always([+PERFORM_MAINTENANCE] true -> eventually(<+VERIFY_MAINTENANCE> true))"#,
            r#"always([+VERIFY_MAINTENANCE] true -> eventually(<+CLOSE_MAINTENANCE> true))"#,
            r#"always([+SCHEDULE_BACKUP] true -> eventually(<+RUN_BACKUP> true))"#,
            r#"always([+RUN_BACKUP] true -> eventually(<+VERIFY_BACKUP> true))"#,
            r#"always([+VERIFY_BACKUP] true -> eventually(<+ARCHIVE_BACKUP> true))"#,
            r#"always([+REQUEST_OFFBOARDING] true -> eventually(<+REVOKE_ACCESS> true))"#,
            r#"always([+REVOKE_ACCESS] true -> eventually(<+TRANSFER_OWNERSHIP> true))"#,
            r#"always([+TRANSFER_OWNERSHIP] true -> eventually(<+CONFIRM_DEPROVISIONING> true))"#,
            r#"always([+NOTICE_RENEWAL] true -> eventually(<+REVIEW_TERMS> true))"#,
            r#"always([+REVIEW_TERMS] true -> eventually(<+APPROVE_RENEWAL> true))"#,
            r#"always([+APPROVE_RENEWAL] true -> eventually(<+EXECUTE_RENEWAL> true))"#,
            r#"always([+REQUEST_CREDENTIAL] true -> eventually(<+VERIFY_IDENTITY> true))"#,
            r#"always([+VERIFY_IDENTITY] true -> eventually(<+ISSUE_CREDENTIAL> true))"#,
            r#"always([+ISSUE_CREDENTIAL] true -> eventually(<+ACCEPT_CREDENTIAL> true))"#,
            r#"always([+START_ACCESS_REVIEW] true -> eventually(<+COLLECT_ACCESS_EVIDENCE> true))"#,
            r#"always([+COLLECT_ACCESS_EVIDENCE] true -> eventually(<+APPROVE_ACCESS_REVIEW> true))"#,
            r#"always([+APPROVE_ACCESS_REVIEW] true -> eventually(<+REMEDIATE_ACCESS> true))"#,
            r#"always([+SUBMIT_CLAIM] true -> eventually(<+REVIEW_CLAIM> true))"#,
            r#"always([+REVIEW_CLAIM] true -> eventually(<+APPROVE_CLAIM> true))"#,
            r#"always([+APPROVE_CLAIM] true -> eventually(<+PAY_CLAIM> true))"#,
            r#"always([+SUBMIT_PRIVACY_REQUEST] true -> eventually(<+VERIFY_SUBJECT> true))"#,
            r#"always([+VERIFY_SUBJECT] true -> eventually(<+FULFILL_PRIVACY_REQUEST> true))"#,
            r#"always([+FULFILL_PRIVACY_REQUEST] true -> eventually(<+CLOSE_PRIVACY_REQUEST> true))"#,
            r#"always([+REQUEST_DELETION] true -> eventually(<+CHECK_RETENTION_POLICY> true))"#,
            r#"always([+CHECK_RETENTION_POLICY] true -> eventually(<+DELETE_RECORDS> true))"#,
            r#"always([+DELETE_RECORDS] true -> eventually(<+CONFIRM_DELETION> true))"#,
            r#"always([+REQUEST_ACCOUNT_RECOVERY] true -> eventually(<+VERIFY_RECOVERY_FACTOR> true))"#,
            r#"always([+VERIFY_RECOVERY_FACTOR] true -> eventually(<+ROTATE_CREDENTIAL> true))"#,
            r#"always([+ROTATE_CREDENTIAL] true -> eventually(<+CONFIRM_ACCOUNT_RECOVERY> true))"#,
            r#"always([+REQUEST_CONSENT_CHANGE] true -> eventually(<+REVIEW_CONSENT_SCOPE> true))"#,
            r#"always([+REVIEW_CONSENT_SCOPE] true -> eventually(<+APPLY_CONSENT_CHANGE> true))"#,
            r#"always([+APPLY_CONSENT_CHANGE] true -> eventually(<+CONFIRM_CONSENT_CHANGE> true))"#,
            r#"always([+OPEN_SECURITY_EXCEPTION] true -> eventually(<+ASSESS_EXCEPTION_RISK> true))"#,
            r#"always([+ASSESS_EXCEPTION_RISK] true -> eventually(<+APPROVE_EXCEPTION_MITIGATION> true))"#,
            r#"always([+APPROVE_EXCEPTION_MITIGATION] true -> eventually(<+CLOSE_SECURITY_EXCEPTION> true))"#,
            r#"always([+REPORT_VULNERABILITY] true -> eventually(<+TRIAGE_VULNERABILITY> true))"#,
            r#"always([+TRIAGE_VULNERABILITY] true -> eventually(<+APPLY_PATCH> true))"#,
            r#"always([+APPLY_PATCH] true -> eventually(<+VERIFY_PATCH> true))"#,
            r#"always([+DETECT_BREACH] true -> eventually(<+ASSESS_BREACH_SCOPE> true))"#,
            r#"always([+ASSESS_BREACH_SCOPE] true -> eventually(<+NOTIFY_AFFECTED_PARTIES> true))"#,
            r#"always([+NOTIFY_AFFECTED_PARTIES] true -> eventually(<+COMPLETE_BREACH_REVIEW> true))"#,
            r#"always([+START_VENDOR_REVIEW] true -> eventually(<+COLLECT_VENDOR_QUESTIONNAIRE> true))"#,
            r#"always([+COLLECT_VENDOR_QUESTIONNAIRE] true -> eventually(<+ASSESS_VENDOR_RISK> true))"#,
            r#"always([+ASSESS_VENDOR_RISK] true -> eventually(<+APPROVE_VENDOR> true))"#,
            r#"always([+REQUEST_DATA_ACCESS] true -> eventually(<+VERIFY_ACCESS_PURPOSE> true))"#,
            r#"always([+VERIFY_ACCESS_PURPOSE] true -> eventually(<+APPROVE_DATA_ACCESS> true))"#,
            r#"always([+APPROVE_DATA_ACCESS] true -> eventually(<+LOG_ACCESS_GRANT> true))"#,
            r#"always([+REQUEST_DATA_EXPORT] true -> eventually(<+CLASSIFY_EXPORT_DATA> true))"#,
            r#"always([+CLASSIFY_EXPORT_DATA] true -> eventually(<+APPROVE_DATA_EXPORT> true))"#,
            r#"always([+APPROVE_DATA_EXPORT] true -> eventually(<+TRANSMIT_EXPORT_PACKAGE> true))"#,
            r#"always([+REQUEST_DATA_SHARE] true -> eventually(<+VERIFY_RECIPIENT_AUTHORITY> true))"#,
            r#"always([+VERIFY_RECIPIENT_AUTHORITY] true -> eventually(<+APPROVE_DATA_SHARE> true))"#,
            r#"always([+APPROVE_DATA_SHARE] true -> eventually(<+RECORD_DATA_SHARE> true))"#,
            r#"always([+REQUEST_DATA_USE] true -> eventually(<+REVIEW_USE_LIMITS> true))"#,
            r#"always([+REVIEW_USE_LIMITS] true -> eventually(<+APPROVE_DATA_USE> true))"#,
            r#"always([+APPROVE_DATA_USE] true -> eventually(<+LOG_DATA_USE> true))"#,
            r#"always([+START_RETENTION_REVIEW] true -> eventually(<+CLASSIFY_RETENTION_RECORDS> true))"#,
            r#"always([+CLASSIFY_RETENTION_RECORDS] true -> eventually(<+APPROVE_RETENTION_PLAN> true))"#,
            r#"always([+APPROVE_RETENTION_PLAN] true -> eventually(<+ENFORCE_RETENTION_PLAN> true))"#,
            r#"always([+COLLECT_DATA] true -> eventually(<+MINIMIZE_DATASET> true))"#,
            r#"always([+MINIMIZE_DATASET] true -> eventually(<+APPROVE_MINIMIZED_DATA> true))"#,
            r#"always([+APPROVE_MINIMIZED_DATA] true -> eventually(<+RECORD_MINIMIZATION> true))"#,
            r#"always([+PREPARE_ANALYTICS_DATA] true -> eventually(<+ANONYMIZE_DATASET> true))"#,
            r#"always([+ANONYMIZE_DATASET] true -> eventually(<+VERIFY_ANONYMIZATION> true))"#,
            r#"always([+VERIFY_ANONYMIZATION] true -> eventually(<+RELEASE_ANONYMIZED_DATA> true))"#,
            r#"always([+REQUEST_PURPOSE_CHANGE] true -> eventually(<+REVIEW_PURPOSE_COMPATIBILITY> true))"#,
            r#"always([+REVIEW_PURPOSE_COMPATIBILITY] true -> eventually(<+APPROVE_PURPOSE_CHANGE> true))"#,
            r#"always([+APPROVE_PURPOSE_CHANGE] true -> eventually(<+RECORD_PURPOSE_CHANGE> true))"#,
            r#"always([+REQUEST_LAWFUL_BASIS_REVIEW] true -> eventually(<+ASSESS_LAWFUL_BASIS> true))"#,
            r#"always([+ASSESS_LAWFUL_BASIS] true -> eventually(<+APPROVE_PROCESSING_BASIS> true))"#,
            r#"always([+APPROVE_PROCESSING_BASIS] true -> eventually(<+RECORD_PROCESSING_BASIS> true))"#,
            r#"always([+REGISTER_DATASET] true -> eventually(<+CAPTURE_PROVENANCE> true))"#,
            r#"always([+CAPTURE_PROVENANCE] true -> eventually(<+VERIFY_PROVENANCE> true))"#,
            r#"always([+VERIFY_PROVENANCE] true -> eventually(<+APPROVE_PROVENANCE_RECORD> true))"#,
            r#"always([+PROFILE_DATASET] true -> eventually(<+VALIDATE_DATA_QUALITY> true))"#,
            r#"always([+VALIDATE_DATA_QUALITY] true -> eventually(<+APPROVE_QUALITY_REPORT> true))"#,
            r#"always([+APPROVE_QUALITY_REPORT] true -> eventually(<+PUBLISH_QUALITY_REPORT> true))"#,
            r#"always([+SUBMIT_DATASET] true -> eventually(<+CLASSIFY_DATASET> true))"#,
            r#"always([+CLASSIFY_DATASET] true -> eventually(<+APPROVE_DATA_CLASSIFICATION> true))"#,
            r#"always([+APPROVE_DATA_CLASSIFICATION] true -> eventually(<+RECORD_DATA_CLASSIFICATION> true))"#,
            r#"always([+START_DPIA] true -> eventually(<+ASSESS_PRIVACY_RISK> true))"#,
            r#"always([+ASSESS_PRIVACY_RISK] true -> eventually(<+APPROVE_DPIA> true))"#,
            r#"always([+APPROVE_DPIA] true -> eventually(<+RECORD_DPIA> true))"#,
            r#"always([+REQUEST_CROSS_BORDER_TRANSFER] true -> eventually(<+ASSESS_TRANSFER_MECHANISM> true))"#,
            r#"always([+ASSESS_TRANSFER_MECHANISM] true -> eventually(<+APPROVE_CROSS_BORDER_TRANSFER> true))"#,
            r#"always([+APPROVE_CROSS_BORDER_TRANSFER] true -> eventually(<+RECORD_TRANSFER_ASSESSMENT> true))"#,
            r#"always([+REGISTER_SUBPROCESSOR] true -> eventually(<+ASSESS_SUBPROCESSOR_RISK> true))"#,
            r#"always([+ASSESS_SUBPROCESSOR_RISK] true -> eventually(<+APPROVE_SUBPROCESSOR> true))"#,
            r#"always([+APPROVE_SUBPROCESSOR] true -> eventually(<+RECORD_SUBPROCESSOR> true))"#,
            r#"always([+REQUEST_DATA_LOCALIZATION] true -> eventually(<+ASSESS_RESIDENCY_REQUIREMENT> true))"#,
            r#"always([+ASSESS_RESIDENCY_REQUIREMENT] true -> eventually(<+APPROVE_LOCALIZATION_PLAN> true))"#,
            r#"always([+APPROVE_LOCALIZATION_PLAN] true -> eventually(<+RECORD_LOCALIZATION_CONTROL> true))"#,
            r#"always([+SUBMIT_MODEL_CARD] true -> eventually(<+EVALUATE_MODEL_RISK> true))"#,
            r#"always([+EVALUATE_MODEL_RISK] true -> eventually(<+APPROVE_MODEL_DEPLOYMENT> true))"#,
            r#"always([+APPROVE_MODEL_DEPLOYMENT] true -> eventually(<+PUBLISH_MODEL_CARD> true))"#,
            r#"always([+REGISTER_EVALUATION_DATASET] true -> eventually(<+RUN_BIAS_EVALUATION> true))"#,
            r#"always([+RUN_BIAS_EVALUATION] true -> eventually(<+APPROVE_EVALUATION_REPORT> true))"#,
            r#"always([+APPROVE_EVALUATION_REPORT] true -> eventually(<+ARCHIVE_EVALUATION_EVIDENCE> true))"#,
            r#"always([+SCHEDULE_MODEL_CALIBRATION] true -> eventually(<+RUN_CALIBRATION_CHECK> true))"#,
            r#"always([+RUN_CALIBRATION_CHECK] true -> eventually(<+APPROVE_CALIBRATION_REPORT> true))"#,
            r#"always([+APPROVE_CALIBRATION_REPORT] true -> eventually(<+RECORD_CALIBRATION_RESULT> true))"#,
            r#"always([+START_MODEL_MONITORING] true -> eventually(<+DETECT_MODEL_DRIFT> true))"#,
            r#"always([+DETECT_MODEL_DRIFT] true -> eventually(<+APPROVE_MODEL_UPDATE> true))"#,
            r#"always([+APPROVE_MODEL_UPDATE] true -> eventually(<+RECORD_MONITORING_REVIEW> true))"#,
            r#"always([+DETECT_MODEL_INCIDENT] true -> eventually(<+ASSESS_MODEL_IMPACT> true))"#,
            r#"always([+ASSESS_MODEL_IMPACT] true -> eventually(<+APPROVE_MODEL_ROLLBACK> true))"#,
            r#"always([+APPROVE_MODEL_ROLLBACK] true -> eventually(<+RECORD_MODEL_INCIDENT> true))"#,
            r#"always([+REQUEST_MODEL_RETIREMENT] true -> eventually(<+ASSESS_RETIREMENT_IMPACT> true))"#,
            r#"always([+ASSESS_RETIREMENT_IMPACT] true -> eventually(<+APPROVE_MODEL_RETIREMENT> true))"#,
            r#"always([+APPROVE_MODEL_RETIREMENT] true -> eventually(<+ARCHIVE_MODEL_ARTIFACTS> true))"#,
            r#"always([+COLLECT_RETRAINING_DATA] true -> eventually(<+APPROVE_RETRAINING_PLAN> true))"#,
            r#"always([+APPROVE_RETRAINING_PLAN] true -> eventually(<+TRAIN_CANDIDATE_MODEL> true))"#,
            r#"always([+TRAIN_CANDIDATE_MODEL] true -> eventually(<+VALIDATE_CANDIDATE_MODEL> true))"#,
            r#"always([+LOG_MODEL_DECISION] true -> eventually(<+REVIEW_DECISION_TRACE> true))"#,
            r#"always([+REVIEW_DECISION_TRACE] true -> eventually(<+APPROVE_MODEL_AUDIT> true))"#,
            r#"always([+APPROVE_MODEL_AUDIT] true -> eventually(<+RECORD_AUDIT_EVIDENCE> true))"#,
            r#"always([+START_MODEL_RED_TEAM] true -> eventually(<+REVIEW_RED_TEAM_FINDINGS> true))"#,
            r#"always([+REVIEW_RED_TEAM_FINDINGS] true -> eventually(<+APPROVE_SAFETY_MITIGATION> true))"#,
            r#"always([+APPROVE_SAFETY_MITIGATION] true -> eventually(<+RECORD_SAFETY_CASE> true))"#,
            r#"always([+REGISTER_MODEL_VERSION] true -> eventually(<+RUN_MODEL_VALIDATION> true))"#,
            r#"always([+RUN_MODEL_VALIDATION] true -> eventually(<+APPROVE_MODEL_VERSION> true))"#,
            r#"always([+APPROVE_MODEL_VERSION] true -> eventually(<+PROMOTE_MODEL_VERSION> true))"#,
            r#"always([+CAPTURE_MODEL_LINEAGE] true -> eventually(<+REVIEW_LINEAGE_REPORT> true))"#,
            r#"always([+REVIEW_LINEAGE_REPORT] true -> eventually(<+APPROVE_LINEAGE_RECORD> true))"#,
            r#"always([+APPROVE_LINEAGE_RECORD] true -> eventually(<+ARCHIVE_LINEAGE_RECORD> true))"#,
            r#"always([+SUBMIT_MODEL_ARTIFACT] true -> eventually(<+SCAN_MODEL_ARTIFACT> true))"#,
            r#"always([+SCAN_MODEL_ARTIFACT] true -> eventually(<+APPROVE_MODEL_ARTIFACT> true))"#,
            r#"always([+APPROVE_MODEL_ARTIFACT] true -> eventually(<+PUBLISH_MODEL_ARTIFACT> true))"#,
            r#"always([+REGISTER_MODEL_ENDPOINT] true -> eventually(<+RUN_ENDPOINT_SMOKE_TEST> true))"#,
            r#"always([+RUN_ENDPOINT_SMOKE_TEST] true -> eventually(<+APPROVE_ENDPOINT_ACTIVATION> true))"#,
            r#"always([+APPROVE_ENDPOINT_ACTIVATION] true -> eventually(<+ACTIVATE_MODEL_ENDPOINT> true))"#,
            r#"always([+PLAN_MODEL_CANARY] true -> eventually(<+MONITOR_CANARY_METRICS> true))"#,
            r#"always([+MONITOR_CANARY_METRICS] true -> eventually(<+APPROVE_FULL_ROLLOUT> true))"#,
            r#"always([+APPROVE_FULL_ROLLOUT] true -> eventually(<+EXPAND_MODEL_TRAFFIC> true))"#,
            r#"always([+DETECT_MODEL_DRIFT] true -> eventually(<+ASSESS_DRIFT_IMPACT> true))"#,
            r#"always([+ASSESS_DRIFT_IMPACT] true -> eventually(<+APPROVE_DRIFT_RESPONSE> true))"#,
            r#"always([+APPROVE_DRIFT_RESPONSE] true -> eventually(<+RECORD_DRIFT_RESPONSE> true))"#,
            r#"always([+START_SHADOW_EVALUATION] true -> eventually(<+COMPARE_SHADOW_OUTPUT> true))"#,
            r#"always([+COMPARE_SHADOW_OUTPUT] true -> eventually(<+APPROVE_SHADOW_PROMOTION> true))"#,
            r#"always([+APPROVE_SHADOW_PROMOTION] true -> eventually(<+PROMOTE_SHADOW_MODEL> true))"#,
            r#"always([+DETECT_MODEL_REGRESSION] true -> eventually(<+ASSESS_ROLLBACK_RISK> true))"#,
            r#"always([+ASSESS_ROLLBACK_RISK] true -> eventually(<+APPROVE_MODEL_ROLLBACK_PLAN> true))"#,
            r#"always([+APPROVE_MODEL_ROLLBACK_PLAN] true -> eventually(<+EXECUTE_MODEL_ROLLBACK> true))"#,
            r#"always([+REQUEST_MODEL_EXCEPTION] true -> eventually(<+ASSESS_MODEL_EXCEPTION> true))"#,
            r#"always([+ASSESS_MODEL_EXCEPTION] true -> eventually(<+APPROVE_MODEL_EXCEPTION> true))"#,
            r#"always([+APPROVE_MODEL_EXCEPTION] true -> eventually(<+RECORD_MODEL_EXCEPTION> true))"#,
            r#"always([+REQUEST_MODEL_DEPRECATION] true -> eventually(<+ASSESS_DEPRECATION_IMPACT> true))"#,
            r#"always([+ASSESS_DEPRECATION_IMPACT] true -> eventually(<+APPROVE_MODEL_DEPRECATION> true))"#,
            r#"always([+APPROVE_MODEL_DEPRECATION] true -> eventually(<+RECORD_MODEL_DEPRECATION> true))"#,
            r#"always([+REQUEST_MODEL_ATTESTATION] true -> eventually(<+COLLECT_ATTESTATION_EVIDENCE> true))"#,
            r#"always([+COLLECT_ATTESTATION_EVIDENCE] true -> eventually(<+APPROVE_MODEL_ATTESTATION> true))"#,
            r#"always([+APPROVE_MODEL_ATTESTATION] true -> eventually(<+PUBLISH_MODEL_ATTESTATION> true))"#,
            r#"always([+REQUEST_MODEL_DISCLOSURE] true -> eventually(<+REVIEW_DISCLOSURE_SCOPE> true))"#,
            r#"always([+REVIEW_DISCLOSURE_SCOPE] true -> eventually(<+APPROVE_MODEL_DISCLOSURE> true))"#,
            r#"always([+APPROVE_MODEL_DISCLOSURE] true -> eventually(<+PUBLISH_MODEL_DISCLOSURE> true))"#,
            r#"always([+REQUEST_MODEL_APPEAL] true -> eventually(<+REVIEW_MODEL_APPEAL> true))"#,
            r#"always([+REVIEW_MODEL_APPEAL] true -> eventually(<+APPROVE_MODEL_APPEAL> true))"#,
            r#"always([+APPROVE_MODEL_APPEAL] true -> eventually(<+RECORD_MODEL_APPEAL> true))"#,
            r#"always([+REQUEST_MODEL_OVERRIDE] true -> eventually(<+REVIEW_OVERRIDE_RISK> true))"#,
            r#"always([+REVIEW_OVERRIDE_RISK] true -> eventually(<+APPROVE_MODEL_OVERRIDE> true))"#,
            r#"always([+APPROVE_MODEL_OVERRIDE] true -> eventually(<+RECORD_OVERRIDE_AUDIT> true))"#,
            r#"always([+REQUEST_AGENT_ACTION] true -> eventually(<+SIMULATE_AGENT_ACTION> true))"#,
            r#"always([+SIMULATE_AGENT_ACTION] true -> eventually(<+APPROVE_AGENT_ACTION> true))"#,
            r#"always([+APPROVE_AGENT_ACTION] true -> eventually(<+EXECUTE_AGENT_ACTION> true))"#,
            r#"always([+REQUEST_TOOL_PERMISSION] true -> eventually(<+ASSESS_TOOL_RISK> true))"#,
            r#"always([+ASSESS_TOOL_RISK] true -> eventually(<+APPROVE_TOOL_PERMISSION> true))"#,
            r#"always([+APPROVE_TOOL_PERMISSION] true -> eventually(<+GRANT_TOOL_PERMISSION> true))"#,
            r#"always([+DELEGATE_AGENT_TASK] true -> eventually(<+REVIEW_AGENT_OUTPUT> true))"#,
            r#"always([+REVIEW_AGENT_OUTPUT] true -> eventually(<+APPROVE_AGENT_OUTPUT> true))"#,
            r#"always([+APPROVE_AGENT_OUTPUT] true -> eventually(<+ARCHIVE_AGENT_TRACE> true))"#,
            r#"always([+REQUEST_AGENT_POLICY_EXCEPTION] true -> eventually(<+ASSESS_AGENT_POLICY_RISK> true))"#,
            r#"always([+ASSESS_AGENT_POLICY_RISK] true -> eventually(<+APPROVE_AGENT_POLICY_EXCEPTION> true))"#,
            r#"always([+APPROVE_AGENT_POLICY_EXCEPTION] true -> eventually(<+RECORD_AGENT_POLICY_EXCEPTION> true))"#,
            r#"always([+REQUEST_SANDBOX_SESSION] true -> eventually(<+APPROVE_SANDBOX_BOUNDARY> true))"#,
            r#"always([+APPROVE_SANDBOX_BOUNDARY] true -> eventually(<+GRANT_SANDBOX_SESSION> true))"#,
            r#"always([+GRANT_SANDBOX_SESSION] true -> eventually(<+RECORD_SANDBOX_AUDIT> true))"#,
            r#"always([+REQUEST_AGENT_CAPABILITY] true -> eventually(<+EVALUATE_CAPABILITY_SCOPE> true))"#,
            r#"always([+EVALUATE_CAPABILITY_SCOPE] true -> eventually(<+APPROVE_AGENT_CAPABILITY> true))"#,
            r#"always([+APPROVE_AGENT_CAPABILITY] true -> eventually(<+ENABLE_AGENT_CAPABILITY> true))"#,
            r#"always([+PROPOSE_AGENT_MEMORY] true -> eventually(<+REVIEW_MEMORY_SCOPE> true))"#,
            r#"always([+REVIEW_MEMORY_SCOPE] true -> eventually(<+APPROVE_MEMORY_WRITE> true))"#,
            r#"always([+APPROVE_MEMORY_WRITE] true -> eventually(<+COMMIT_AGENT_MEMORY> true))"#,
            r#"always([+REQUEST_AGENT_HANDOFF] true -> eventually(<+PACKAGE_AGENT_CONTEXT> true))"#,
            r#"always([+PACKAGE_AGENT_CONTEXT] true -> eventually(<+APPROVE_AGENT_HANDOFF> true))"#,
            r#"always([+APPROVE_AGENT_HANDOFF] true -> eventually(<+ACCEPT_AGENT_HANDOFF> true))"#,
            r#"always([+REQUEST_EXTERNAL_TOOL_CALL] true -> eventually(<+ASSESS_TOOL_CALL_RISK> true))"#,
            r#"always([+ASSESS_TOOL_CALL_RISK] true -> eventually(<+APPROVE_EXTERNAL_TOOL_CALL> true))"#,
            r#"always([+APPROVE_EXTERNAL_TOOL_CALL] true -> eventually(<+EXECUTE_EXTERNAL_TOOL_CALL> true))"#,
            r#"always([+REQUEST_AGENT_CREDENTIAL_ROTATION] true -> eventually(<+VERIFY_AGENT_IDENTITY> true))"#,
            r#"always([+VERIFY_AGENT_IDENTITY] true -> eventually(<+APPROVE_AGENT_CREDENTIAL_ROTATION> true))"#,
            r#"always([+APPROVE_AGENT_CREDENTIAL_ROTATION] true -> eventually(<+ROTATE_AGENT_CREDENTIAL> true))"#,
            r#"always([+REPORT_AGENT_INCIDENT] true -> eventually(<+CONTAIN_AGENT_SESSION> true))"#,
            r#"always([+CONTAIN_AGENT_SESSION] true -> eventually(<+APPROVE_AGENT_REMEDIATION> true))"#,
            r#"always([+APPROVE_AGENT_REMEDIATION] true -> eventually(<+RECORD_AGENT_INCIDENT> true))"#,
            r#"always([+REQUEST_AGENT_PERMISSION_REVOKE] true -> eventually(<+ASSESS_PERMISSION_DEPENDENCIES> true))"#,
            r#"always([+ASSESS_PERMISSION_DEPENDENCIES] true -> eventually(<+APPROVE_AGENT_PERMISSION_REVOKE> true))"#,
            r#"always([+APPROVE_AGENT_PERMISSION_REVOKE] true -> eventually(<+REVOKE_AGENT_PERMISSION> true))"#,
            r#"always([+REQUEST_AGENT_DATA_EGRESS] true -> eventually(<+CLASSIFY_AGENT_OUTPUT> true))"#,
            r#"always([+CLASSIFY_AGENT_OUTPUT] true -> eventually(<+APPROVE_AGENT_DATA_EGRESS> true))"#,
            r#"always([+APPROVE_AGENT_DATA_EGRESS] true -> eventually(<+RELEASE_AGENT_OUTPUT> true))"#,
            r#"always([+REQUEST_AGENT_AUTONOMY] true -> eventually(<+ASSESS_AUTONOMY_RISK> true))"#,
            r#"always([+ASSESS_AUTONOMY_RISK] true -> eventually(<+APPROVE_AGENT_AUTONOMY> true))"#,
            r#"always([+APPROVE_AGENT_AUTONOMY] true -> eventually(<+ENABLE_AGENT_AUTONOMY> true))"#,
            r#"always([+PROPOSE_AGENT_PUBLICATION] true -> eventually(<+REVIEW_AGENT_CLAIMS> true))"#,
            r#"always([+REVIEW_AGENT_CLAIMS] true -> eventually(<+APPROVE_AGENT_PUBLICATION> true))"#,
            r#"always([+APPROVE_AGENT_PUBLICATION] true -> eventually(<+PUBLISH_AGENT_OUTPUT> true))"#,
            r#"always([+REQUEST_AGENT_SECRET_ACCESS] true -> eventually(<+REVIEW_SECRET_SCOPE> true))"#,
            r#"always([+REVIEW_SECRET_SCOPE] true -> eventually(<+APPROVE_AGENT_SECRET_ACCESS> true))"#,
            r#"always([+APPROVE_AGENT_SECRET_ACCESS] true -> eventually(<+GRANT_AGENT_SECRET_ACCESS> true))"#,
            r#"always([+REQUEST_AGENT_MODEL_ACCESS] true -> eventually(<+REVIEW_MODEL_ACCESS_SCOPE> true))"#,
            r#"always([+REVIEW_MODEL_ACCESS_SCOPE] true -> eventually(<+APPROVE_AGENT_MODEL_ACCESS> true))"#,
            r#"always([+APPROVE_AGENT_MODEL_ACCESS] true -> eventually(<+GRANT_AGENT_MODEL_ACCESS> true))"#,
            r#"always([+REQUEST_AGENT_SPEND] true -> eventually(<+ESTIMATE_AGENT_SPEND_RISK> true))"#,
            r#"always([+ESTIMATE_AGENT_SPEND_RISK] true -> eventually(<+APPROVE_AGENT_SPEND> true))"#,
            r#"always([+APPROVE_AGENT_SPEND] true -> eventually(<+EXECUTE_AGENT_SPEND> true))"#,
            r#"always([+DETECT_PROMPT_INJECTION] true -> eventually(<+QUARANTINE_AGENT_CONTEXT> true))"#,
            r#"always([+QUARANTINE_AGENT_CONTEXT] true -> eventually(<+APPROVE_CONTEXT_RESTORATION> true))"#,
            r#"always([+APPROVE_CONTEXT_RESTORATION] true -> eventually(<+RESTORE_AGENT_CONTEXT> true))"#,
            r#"always([+REQUEST_AGENT_NETWORK_ACCESS] true -> eventually(<+ASSESS_NETWORK_SCOPE> true))"#,
            r#"always([+ASSESS_NETWORK_SCOPE] true -> eventually(<+APPROVE_AGENT_NETWORK_ACCESS> true))"#,
            r#"always([+APPROVE_AGENT_NETWORK_ACCESS] true -> eventually(<+ENABLE_AGENT_NETWORK_ACCESS> true))"#,
            r#"always([+REQUEST_AGENT_STATE_EXPORT] true -> eventually(<+REDACT_AGENT_STATE> true))"#,
            r#"always([+REDACT_AGENT_STATE] true -> eventually(<+APPROVE_AGENT_STATE_EXPORT> true))"#,
            r#"always([+APPROVE_AGENT_STATE_EXPORT] true -> eventually(<+EXPORT_AGENT_STATE> true))"#,
            r#"always([+PROPOSE_AGENT_DEPENDENCY_UPDATE] true -> eventually(<+SCAN_AGENT_DEPENDENCY> true))"#,
            r#"always([+SCAN_AGENT_DEPENDENCY] true -> eventually(<+APPROVE_AGENT_DEPENDENCY_UPDATE> true))"#,
            r#"always([+APPROVE_AGENT_DEPENDENCY_UPDATE] true -> eventually(<+APPLY_AGENT_DEPENDENCY_UPDATE> true))"#,
            r#"always([+REQUEST_AGENT_IDENTITY_BINDING] true -> eventually(<+VERIFY_AGENT_ATTESTATION> true))"#,
            r#"always([+VERIFY_AGENT_ATTESTATION] true -> eventually(<+APPROVE_AGENT_IDENTITY_BINDING> true))"#,
            r#"always([+APPROVE_AGENT_IDENTITY_BINDING] true -> eventually(<+BIND_AGENT_IDENTITY> true))"#,
            r#"always([+REQUEST_AGENT_RUNTIME_MIGRATION] true -> eventually(<+SNAPSHOT_AGENT_RUNTIME> true))"#,
            r#"always([+SNAPSHOT_AGENT_RUNTIME] true -> eventually(<+APPROVE_AGENT_RUNTIME_MIGRATION> true))"#,
            r#"always([+APPROVE_AGENT_RUNTIME_MIGRATION] true -> eventually(<+MIGRATE_AGENT_RUNTIME> true))"#,
            r#"always([+REQUEST_AGENT_ROLLBACK] true -> eventually(<+VERIFY_ROLLBACK_POINT> true))"#,
            r#"always([+VERIFY_ROLLBACK_POINT] true -> eventually(<+APPROVE_AGENT_ROLLBACK> true))"#,
            r#"always([+APPROVE_AGENT_ROLLBACK] true -> eventually(<+ROLLBACK_AGENT_STATE> true))"#,
            r#"always([+REQUEST_AGENT_TELEMETRY_ACCESS] true -> eventually(<+REDACT_AGENT_TELEMETRY> true))"#,
            r#"always([+REDACT_AGENT_TELEMETRY] true -> eventually(<+APPROVE_AGENT_TELEMETRY_ACCESS> true))"#,
            r#"always([+APPROVE_AGENT_TELEMETRY_ACCESS] true -> eventually(<+EXPORT_AGENT_TELEMETRY> true))"#,
            r#"always([+REQUEST_AGENT_SESSION_RESUME] true -> eventually(<+VALIDATE_SESSION_CHECKPOINT> true))"#,
            r#"always([+VALIDATE_SESSION_CHECKPOINT] true -> eventually(<+APPROVE_AGENT_SESSION_RESUME> true))"#,
            r#"always([+APPROVE_AGENT_SESSION_RESUME] true -> eventually(<+RESUME_AGENT_SESSION> true))"#,
            r#"always([+REQUEST_AGENT_BACKUP] true -> eventually(<+VERIFY_BACKUP_SCOPE> true))"#,
            r#"always([+VERIFY_BACKUP_SCOPE] true -> eventually(<+APPROVE_AGENT_BACKUP> true))"#,
            r#"always([+APPROVE_AGENT_BACKUP] true -> eventually(<+CREATE_AGENT_BACKUP> true))"#,
            r#"always([+REQUEST_AGENT_LOG_RETENTION] true -> eventually(<+CLASSIFY_AGENT_LOGS> true))"#,
            r#"always([+CLASSIFY_AGENT_LOGS] true -> eventually(<+APPROVE_AGENT_LOG_RETENTION> true))"#,
            r#"always([+APPROVE_AGENT_LOG_RETENTION] true -> eventually(<+ENFORCE_AGENT_LOG_RETENTION> true))"#,
            r#"always([+REQUEST_AGENT_STATE_PURGE] true -> eventually(<+REVIEW_PURGE_SCOPE> true))"#,
            r#"always([+REVIEW_PURGE_SCOPE] true -> eventually(<+APPROVE_AGENT_STATE_PURGE> true))"#,
            r#"always([+APPROVE_AGENT_STATE_PURGE] true -> eventually(<+PURGE_AGENT_STATE> true))"#,
            r#"always([+REQUEST_AGENT_AUDIT_DISCLOSURE] true -> eventually(<+REDACT_AGENT_AUDIT_LOG> true))"#,
            r#"always([+REDACT_AGENT_AUDIT_LOG] true -> eventually(<+APPROVE_AGENT_AUDIT_DISCLOSURE> true))"#,
            r#"always([+APPROVE_AGENT_AUDIT_DISCLOSURE] true -> eventually(<+DISCLOSE_AGENT_AUDIT_LOG> true))"#,
            r#"always([+REQUEST_AGENT_ENVIRONMENT_TEARDOWN] true -> eventually(<+SNAPSHOT_AGENT_ENVIRONMENT> true))"#,
            r#"always([+SNAPSHOT_AGENT_ENVIRONMENT] true -> eventually(<+APPROVE_AGENT_ENVIRONMENT_TEARDOWN> true))"#,
            r#"always([+APPROVE_AGENT_ENVIRONMENT_TEARDOWN] true -> eventually(<+TEARDOWN_AGENT_ENVIRONMENT> true))"#,
            r#"always([+REQUEST_AGENT_CACHE_INVALIDATION] true -> eventually(<+ASSESS_CACHE_DEPENDENCIES> true))"#,
            r#"always([+ASSESS_CACHE_DEPENDENCIES] true -> eventually(<+APPROVE_AGENT_CACHE_INVALIDATION> true))"#,
            r#"always([+APPROVE_AGENT_CACHE_INVALIDATION] true -> eventually(<+INVALIDATE_AGENT_CACHE> true))"#,
            r#"always([+REQUEST_AGENT_CONTEXT_COMPACTION] true -> eventually(<+SUMMARIZE_AGENT_CONTEXT> true))"#,
            r#"always([+SUMMARIZE_AGENT_CONTEXT] true -> eventually(<+APPROVE_AGENT_CONTEXT_COMPACTION> true))"#,
            r#"always([+APPROVE_AGENT_CONTEXT_COMPACTION] true -> eventually(<+COMPACT_AGENT_CONTEXT> true))"#,
            r#"always([+REQUEST_AGENT_WORKSPACE_HANDOVER] true -> eventually(<+INVENTORY_WORKSPACE_STATE> true))"#,
            r#"always([+INVENTORY_WORKSPACE_STATE] true -> eventually(<+APPROVE_AGENT_WORKSPACE_HANDOVER> true))"#,
            r#"always([+APPROVE_AGENT_WORKSPACE_HANDOVER] true -> eventually(<+HANDOVER_AGENT_WORKSPACE> true))"#,
            r#"always([+REQUEST_AGENT_KNOWLEDGE_REFRESH] true -> eventually(<+REVIEW_KNOWLEDGE_SOURCES> true))"#,
            r#"always([+REVIEW_KNOWLEDGE_SOURCES] true -> eventually(<+APPROVE_AGENT_KNOWLEDGE_REFRESH> true))"#,
            r#"always([+APPROVE_AGENT_KNOWLEDGE_REFRESH] true -> eventually(<+REFRESH_AGENT_KNOWLEDGE> true))"#,
            r#"always([+REQUEST_AGENT_DELEGATION_RENEWAL] true -> eventually(<+REVIEW_DELEGATION_SCOPE> true))"#,
            r#"always([+REVIEW_DELEGATION_SCOPE] true -> eventually(<+APPROVE_AGENT_DELEGATION_RENEWAL> true))"#,
            r#"always([+APPROVE_AGENT_DELEGATION_RENEWAL] true -> eventually(<+RENEW_AGENT_DELEGATION> true))"#,
            r#"always([+DETECT_AGENT_POLICY_DRIFT] true -> eventually(<+ASSESS_POLICY_DRIFT> true))"#,
            r#"always([+ASSESS_POLICY_DRIFT] true -> eventually(<+APPROVE_POLICY_DRIFT_REMEDIATION> true))"#,
            r#"always([+APPROVE_POLICY_DRIFT_REMEDIATION] true -> eventually(<+REMEDIATE_AGENT_POLICY> true))"#,
            r#"always([+REQUEST_AGENT_PERFORMANCE_REVIEW] true -> eventually(<+COLLECT_AGENT_METRICS> true))"#,
            r#"always([+COLLECT_AGENT_METRICS] true -> eventually(<+APPROVE_AGENT_PERFORMANCE_REVIEW> true))"#,
            r#"always([+APPROVE_AGENT_PERFORMANCE_REVIEW] true -> eventually(<+RECORD_AGENT_PERFORMANCE_REVIEW> true))"#,
            r#"always([+REQUEST_AGENT_BUDGET_INCREASE] true -> eventually(<+ASSESS_AGENT_BUDGET_IMPACT> true))"#,
            r#"always([+ASSESS_AGENT_BUDGET_IMPACT] true -> eventually(<+APPROVE_AGENT_BUDGET_INCREASE> true))"#,
            r#"always([+APPROVE_AGENT_BUDGET_INCREASE] true -> eventually(<+APPLY_AGENT_BUDGET> true))"#,
            r#"always([+REQUEST_AGENT_RATE_LIMIT_CHANGE] true -> eventually(<+ASSESS_AGENT_RATE_LIMIT_RISK> true))"#,
            r#"always([+ASSESS_AGENT_RATE_LIMIT_RISK] true -> eventually(<+APPROVE_AGENT_RATE_LIMIT_CHANGE> true))"#,
            r#"always([+APPROVE_AGENT_RATE_LIMIT_CHANGE] true -> eventually(<+APPLY_AGENT_RATE_LIMIT> true))"#,
            r#"always([+REQUEST_AGENT_PROMPT_TEMPLATE_CHANGE] true -> eventually(<+REVIEW_PROMPT_TEMPLATE_DIFF> true))"#,
            r#"always([+REVIEW_PROMPT_TEMPLATE_DIFF] true -> eventually(<+APPROVE_AGENT_PROMPT_TEMPLATE_CHANGE> true))"#,
            r#"always([+APPROVE_AGENT_PROMPT_TEMPLATE_CHANGE] true -> eventually(<+APPLY_AGENT_PROMPT_TEMPLATE> true))"#,
            r#"always([+REQUEST_AGENT_GUARDRAIL_CHANGE] true -> eventually(<+TEST_AGENT_GUARDRAIL> true))"#,
            r#"always([+TEST_AGENT_GUARDRAIL] true -> eventually(<+APPROVE_AGENT_GUARDRAIL_CHANGE> true))"#,
            r#"always([+APPROVE_AGENT_GUARDRAIL_CHANGE] true -> eventually(<+APPLY_AGENT_GUARDRAIL> true))"#,
            r#"always([+REQUEST_AGENT_EVALUATOR_CHANGE] true -> eventually(<+VALIDATE_AGENT_EVALUATOR> true))"#,
            r#"always([+VALIDATE_AGENT_EVALUATOR] true -> eventually(<+APPROVE_AGENT_EVALUATOR_CHANGE> true))"#,
            r#"always([+APPROVE_AGENT_EVALUATOR_CHANGE] true -> eventually(<+APPLY_AGENT_EVALUATOR> true))"#,
            r#"always([+REQUEST_HUMAN_REVIEW] true -> eventually(<+TRIAGE_REVIEW_REQUEST> true))"#,
            r#"always([+TRIAGE_REVIEW_REQUEST] true -> eventually(<+APPROVE_HUMAN_REVIEW> true))"#,
            r#"always([+APPROVE_HUMAN_REVIEW] true -> eventually(<+RECORD_REVIEW_OUTCOME> true))"#,
            r#"always([+REQUEST_DECISION_EXPLANATION] true -> eventually(<+COLLECT_DECISION_FACTORS> true))"#,
            r#"always([+COLLECT_DECISION_FACTORS] true -> eventually(<+APPROVE_DECISION_EXPLANATION> true))"#,
            r#"always([+APPROVE_DECISION_EXPLANATION] true -> eventually(<+DELIVER_DECISION_EXPLANATION> true))"#,
            r#"always([+REQUEST_DECISION_CORRECTION] true -> eventually(<+REVIEW_DECISION_ERROR> true))"#,
            r#"always([+REVIEW_DECISION_ERROR] true -> eventually(<+APPROVE_DECISION_CORRECTION> true))"#,
            r#"always([+APPROVE_DECISION_CORRECTION] true -> eventually(<+RECORD_DECISION_CORRECTION> true))"#,
            r#"always([+REQUEST_DECISION_RECOURSE] true -> eventually(<+REVIEW_RECOURSE_OPTIONS> true))"#,
            r#"always([+REVIEW_RECOURSE_OPTIONS] true -> eventually(<+APPROVE_RECOURSE_PLAN> true))"#,
            r#"always([+APPROVE_RECOURSE_PLAN] true -> eventually(<+RECORD_RECOURSE_OUTCOME> true))"#,
            r#"always([+REQUEST_ADVERSE_ACTION_NOTICE] true -> eventually(<+COMPILE_NOTICE_EVIDENCE> true))"#,
            r#"always([+COMPILE_NOTICE_EVIDENCE] true -> eventually(<+APPROVE_ADVERSE_ACTION_NOTICE> true))"#,
            r#"always([+APPROVE_ADVERSE_ACTION_NOTICE] true -> eventually(<+DELIVER_ADVERSE_ACTION_NOTICE> true))"#,
            r#"always([+CONTEST_AUTOMATED_DECISION] true -> eventually(<+REVIEW_CONTEST_EVIDENCE> true))"#,
            r#"always([+REVIEW_CONTEST_EVIDENCE] true -> eventually(<+APPROVE_CONTEST_RESOLUTION> true))"#,
            r#"always([+APPROVE_CONTEST_RESOLUTION] true -> eventually(<+RECORD_CONTEST_RESOLUTION> true))"#,
            r#"[+RELEASE] true -> eventually((<+DEPOSIT> true & <+DELIVER> true))"#,
            r#"[+RELEASE] true -> eventually(([<+DEPOSIT>] true & [<+DELIVER>] true))"#,
            r#"[+RELEASE] true -> (eventually(<+DEPOSIT> true) & eventually(<+DELIVER> true))"#,
            r#"[<+RELEASE>] true -> (eventually(<+DEPOSIT> true) & eventually(<+DELIVER> true))"#,
            r#"[+RELEASE] true -> (eventually([<+DEPOSIT>] true) & eventually([<+DELIVER>] true))"#,
            r#"[<+RELEASE>] true -> (eventually([<+DEPOSIT>] true) & eventually([<+DELIVER>] true))"#,
            r#"always([+AGENT_A_TURN] true -> eventually(<+AGENT_B_TURN> true))"#,
            r#"always([+AGENT_B_TURN] true -> eventually(<+AGENT_A_TURN> true))"#,
            r#"lfp(X, ((<+REVIEW> true) & ((<+WAIT> true) & <>X)) | (<+APPROVE> true))"#,
            r#"lfp(X, ((<+REVIEW> true) & ((<+WAIT> true) & <>(X))) | (<+APPROVE> true))"#,
            r#"lfp(X, ((<+REVIEW> true) & ((<+WAIT> true) & <>((X)))) | (<+APPROVE> true))"#,
            r#"lfp(X, ((<+REVIEW> true) & ((<+WAIT> true) & <>X)) | ([<+APPROVE>] true))"#,
            r#"lfp(X, ((<+REVIEW> true) & ((<+WAIT> true) & <>(X))) | ([<+APPROVE>] true))"#,
            r#"lfp(X, ((<+REVIEW> true) & ((<+WAIT> true) & <>((X)))) | ([<+APPROVE>] true))"#,
            r#"gfp(X, []((X)) & ([<+ARCHIVE>] true))"#,
            r#"lfp(X, ([<+APPROVE>] true) | [<>]X)"#,
            r#"lfp(X, ([<+APPROVE>] true) | <>X)"#,
            r#"lfp(X, (<+APPROVE> true) | <>X)"#,
            r#"lfp(X, (<+APPROVE> true) | <>(X))"#,
            r#"lfp(X, (<+APPROVE> true) | <>((X)))"#,
            r#"lfp(X, ([<+APPROVE>] true) | <>((X)))"#,
            r#"lfp(X, [<>]X | ([<+APPROVE>] true))"#,
            r#"lfp(X, [<>](X) | ([<+APPROVE>] true))"#,
            r#"lfp(X, [<>]((X)) | ([<+APPROVE>] true))"#,
            r#"gfp(X, []X & (<+APPROVE> true))"#,
            r#"gfp(X, [](X) & (<+APPROVE> true))"#,
            r#"gfp(X, []((X)) & (<+APPROVE> true))"#,
            r#"gfp(X, ([<+APPROVE>] true) & []X)"#,
            r#"gfp(X, []X & ([<+APPROVE>] true))"#,
            r#"gfp(X, [](X) & ([<+APPROVE>] true))"#,
            r#"gfp(X, []((X)) & ([<+APPROVE>] true))"#,
            r#"gfp(X, ([<+APPROVE>] true) & [<>]X)"#,
            r#"gfp(X, [<>]X & ([<+APPROVE>] true))"#,
            r#"gfp(X, [<>](X) & ([<+APPROVE>] true))"#,
            r#"gfp(X, [<>]((X)) & ([<+APPROVE>] true))"#,
        ],
    },
    FormulaExampleGroup {
        title: r#"Authorization and predicates"#,
        description: r#"Signer, multisig, and oracle predicates attached to actions."#,
        formulas: &[
            r#"[<+RELEASE>] true -> <+signed_by(/users/buyer.id)> true"#,
            r#"[<+RELEASE>] true -> [<+signed_by(/users/buyer.id)>] true"#,
            r#"[<+APPROVE>] true -> <+signed_by(/users/alice.id) +signed_by(/users/bob.id)> true"#,
            r#"[<+APPROVE>] true -> [<+signed_by(/users/alice.id) +signed_by(/users/bob.id)>] true"#,
            r#"[+APPROVE] true -> [<+signed_by(/users/reviewer.id)>] true"#,
            r#"always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"#,
            r#"always([+REJECT] true -> <+signed_by(/users/reviewer.id)> true)"#,
            r#"always([+CANCEL] true -> <+signed_by(/users/requester.id)> true)"#,
            r#"always([+REFUND] true -> <+signed_by(/users/seller.id)> true)"#,
            r#"always([+TIMEOUT] true -> <+oracle_attests(/oracles/clock.id, "deadline_passed", "true")> true)"#,
            r#"always([+ESCALATE] true -> <+signed_by(/users/manager.id)> true)"#,
            r#"always([+WITHDRAW] true -> <+signed_by(/users/depositor.id)> true)"#,
            r#"always([+APPEAL] true -> <+signed_by(/users/appellant.id)> true)"#,
            r#"always([+REVOKE] true -> <+signed_by(/users/issuer.id)> true)"#,
            r#"always([+SUSPEND] true -> <+signed_by(/users/administrator.id)> true)"#,
            r#"always([+REINSTATE] true -> <+signed_by(/users/administrator.id)> true)"#,
            r#"always([+RENEW] true -> <+signed_by(/users/holder.id)> true)"#,
            r#"always([+TERMINATE] true -> <+signed_by(/users/counterparty.id)> true)"#,
            r#"always([+EXTEND] true -> <+signed_by(/users/owner.id)> true)"#,
            r#"always([+ASSIGN] true -> <+signed_by(/users/assigner.id)> true)"#,
            r#"always([+CERTIFY] true -> <+signed_by(/users/auditor.id)> true)"#,
            r#"always([+PUBLISH] true -> <+signed_by(/users/editor.id)> true)"#,
            r#"always([+REGISTER] true -> <+signed_by(/users/registrar.id)> true)"#,
            r#"always([+ACCEPT] true -> <+signed_by(/users/recipient.id)> true)"#,
            r#"always([+ACKNOWLEDGE] true -> <+signed_by(/users/recipient.id)> true)"#,
            r#"always([+CONFIRM_DELIVERY] true -> <+signed_by(/users/recipient.id)> true)"#,
            r#"always([+APPROVE_INVOICE] true -> <+signed_by(/users/payer.id)> true)"#,
            r#"always([+ACCEPT_MILESTONE] true -> <+signed_by(/users/verifier.id)> true)"#,
            r#"always([+APPROVE_INSPECTION] true -> <+signed_by(/users/inspector.id)> true)"#,
            r#"always([+ATTEST_COMPLIANCE] true -> <+signed_by(/users/compliance_officer.id)> true)"#,
            r#"always([+APPROVE_SAFETY] true -> <+signed_by(/users/safety_reviewer.id)> true)"#,
            r#"always([+ACCEPT_RISK] true -> <+signed_by(/users/risk_owner.id)> true)"#,
            r#"always([+CLOSE_INCIDENT] true -> <+signed_by(/users/incident_commander.id)> true)"#,
            r#"always([+FREEZE_CHANGE] true -> <+signed_by(/users/release_manager.id)> true)"#,
            r#"[+RELEASE] true -> <+oracle_attests(/oracles/delivery.id, "delivered", "true")> true"#,
            r#"(<+APPROVE> true | [<+REJECT>] true) & ([+APPROVE] true -> <+oracle_attests(/oracles/review.id, "approved", "true")> true)"#,
            r#"[+APPROVE] true -> <+signed_by(/users/alice.id) +signed_by(/users/bob.id)> true"#,
            r#"[+APPROVE] true -> [<+signed_by(/users/alice.id) +signed_by(/users/bob.id)>] true"#,
            r#"always([+ASSIGN_TASK] true -> <+signed_by(/users/task_requester.id) +signed_by(/users/worker_agent.id)> true)"#,
            r#"always([+RESOLVE_DISPUTE] true -> <+signed_by(/users/arbiter.id)> true)"#,
            r#"always([+UPDATE] true -> <+any_signed(/members)> true)"#,
            r#"always([+CHANGE_MEMBERS] true -> <+modifies(/members) +all_signed(/members)> true)"#,
            r#"always([+UPDATE_PROFILE] true -> <+any_signed(/members) -modifies(/members)> true)"#,
            r#"always([+CHANGE_CONFIG] true -> <+modifies(/config) +signed_by(/users/admin.id)> true)"#,
            r#"always([+CHANGE_PRIVATE] true -> <+modifies(/private) +all_signed(/members)> true)"#,
            r#"always([+SETTLE_ESCROW] true -> <+modifies(/escrow) +oracle_attests(/oracles/delivery.id, "delivered", "true")> true)"#,
            r#"[<+SETTLE_ESCROW>] true -> [<+modifies(/escrow) +oracle_attests(/oracles/delivery.id, "delivered", "true")>] true"#,
            r#"[<+ROTATE_KEY>] true -> [<+modifies(/keys) +signed_by(/users/security_admin.id)>] true"#,
            r#"[<+UPDATE_PROFILE>] true -> [<+any_signed(/members) -modifies(/members)>] true"#,
            r#"[<+CHANGE_MEMBERS>] true -> [<+modifies(/members) +all_signed(/members)>] true"#,
            r#"[<+CHANGE_CONFIG>] true -> [<+modifies(/config) +signed_by(/users/admin.id)>] true"#,
            r#"[<+CHANGE_PRIVATE>] true -> [<+modifies(/private) +all_signed(/members)>] true"#,
            r#"[<+EXECUTE_TREASURY>] true -> [<+modifies(/treasury) +threshold("2", /treasury/signers)>] true"#,
            r#"[<+PUBLISH_AUDIT>] true -> [<+modifies(/audit) +signed_by(/users/auditor.id) +oracle_attests(/oracles/audit.id, "passed", "true")>] true"#,
            r#"[<+CLOSE_INCIDENT>] true -> [<+modifies(/incidents) +signed_by(/users/incident_commander.id)>] true"#,
            r#"[<+FREEZE_CHANGE>] true -> [<+modifies(/releases) +signed_by(/users/release_manager.id)>] true"#,
            r#"[<+ACCEPT_RISK>] true -> [<+modifies(/risk) +signed_by(/users/risk_owner.id)>] true"#,
            r#"[<+APPROVE_SAFETY>] true -> [<+modifies(/safety) +signed_by(/users/safety_reviewer.id)>] true"#,
            r#"[<+ATTEST_COMPLIANCE>] true -> [<+modifies(/compliance) +signed_by(/users/compliance_officer.id)>] true"#,
            r#"[<+CONFIRM_DELIVERY>] true -> [<+modifies(/delivery) +signed_by(/users/recipient.id)>] true"#,
            r#"[<+APPROVE_INVOICE>] true -> [<+modifies(/invoices) +signed_by(/users/payer.id)>] true"#,
            r#"[<+APPROVE_BUDGET>] true -> [<+modifies(/budgets) +signed_by(/users/budget_owner.id)>] true"#,
            r#"[<+APPROVE_PURCHASE_ORDER>] true -> [<+modifies(/purchase_orders) +signed_by(/users/procurement_manager.id)>] true"#,
            r#"[<+APPROVE_CONTRACT>] true -> [<+modifies(/contracts) +signed_by(/users/legal_reviewer.id)>] true"#,
            r#"[<+ONBOARD_VENDOR>] true -> [<+modifies(/vendors) +signed_by(/users/vendor_manager.id)>] true"#,
            r#"[<+APPROVE_TIME_OFF>] true -> [<+modifies(/time_off) +signed_by(/users/manager.id)>] true"#,
            r#"[<+APPROVE_EXPENSE>] true -> [<+modifies(/expenses) +signed_by(/users/finance_manager.id)>] true"#,
            r#"[<+APPROVE_TRAVEL>] true -> [<+modifies(/travel) +signed_by(/users/travel_manager.id)>] true"#,
            r#"[<+APPROVE_REIMBURSEMENT>] true -> [<+modifies(/reimbursements) +signed_by(/users/payroll_manager.id)>] true"#,
            r#"[<+APPROVE_REFUND>] true -> [<+modifies(/refunds) +signed_by(/users/refund_manager.id)>] true"#,
            r#"[<+APPROVE_CREDIT>] true -> [<+modifies(/credits) +signed_by(/users/credit_manager.id)>] true"#,
            r#"[<+APPROVE_ADJUSTMENT>] true -> [<+modifies(/adjustments) +signed_by(/users/controller.id)>] true"#,
            r#"[<+APPROVE_PAYMENT>] true -> [<+modifies(/payments) +signed_by(/users/payment_approver.id)>] true"#,
            r#"[<+APPROVE_DISCOUNT>] true -> [<+modifies(/discounts) +signed_by(/users/sales_manager.id)>] true"#,
            r#"[<+APPROVE_COMMISSION>] true -> [<+modifies(/commissions) +signed_by(/users/revenue_lead.id)>] true"#,
            r#"[<+APPROVE_GRANT>] true -> [<+modifies(/grants) +signed_by(/users/grants_manager.id)>] true"#,
            r#"[<+APPROVE_LOAN>] true -> [<+modifies(/loans) +signed_by(/users/loan_officer.id)>] true"#,
            r#"[<+APPROVE_CLAIM>] true -> [<+modifies(/claims) +signed_by(/users/claims_adjuster.id)>] true"#,
            r#"[<+APPROVE_WITHDRAWAL>] true -> [<+modifies(/withdrawals) +signed_by(/users/treasury_officer.id)>] true"#,
            r#"[<+APPROVE_SETTLEMENT>] true -> [<+modifies(/settlements) +signed_by(/users/settlement_manager.id)>] true"#,
            r#"[<+APPROVE_DIVIDEND>] true -> [<+modifies(/dividends) +signed_by(/users/board_secretary.id)>] true"#,
            r#"[<+APPROVE_ROYALTY>] true -> [<+modifies(/royalties) +signed_by(/users/rights_manager.id)>] true"#,
            r#"[<+APPROVE_LICENSE>] true -> [<+modifies(/licenses) +signed_by(/users/licensing_manager.id)>] true"#,
            r#"[<+APPROVE_RENEWAL>] true -> [<+modifies(/renewals) +signed_by(/users/account_manager.id)>] true"#,
            r#"[<+APPROVE_SUBSCRIPTION>] true -> [<+modifies(/subscriptions) +signed_by(/users/customer_success_manager.id)>] true"#,
            r#"[<+APPROVE_ENTITLEMENT>] true -> [<+modifies(/entitlements) +signed_by(/users/access_manager.id)>] true"#,
            r#"[<+APPROVE_DEPRECATION>] true -> [<+modifies(/deprecations) +signed_by(/users/product_manager.id)>] true"#,
            r#"[<+APPROVE_ARCHIVE>] true -> [<+modifies(/archives) +signed_by(/users/records_manager.id)>] true"#,
            r#"[<+APPROVE_RETENTION>] true -> [<+modifies(/retention) +signed_by(/users/records_counsel.id)>] true"#,
            r#"[<+APPROVE_POLICY>] true -> [<+modifies(/policies) +signed_by(/users/policy_owner.id)>] true"#,
            r#"[<+APPROVE_CERTIFICATION>] true -> [<+modifies(/certifications) +signed_by(/users/certification_manager.id)>] true"#,
            r#"[<+APPROVE_ACCREDITATION>] true -> [<+modifies(/accreditations) +signed_by(/users/accreditation_manager.id)>] true"#,
            r#"[<+APPROVE_WAIVER>] true -> [<+modifies(/waivers) +signed_by(/users/waiver_authority.id)>] true"#,
            r#"[<+APPROVE_EXCEPTION>] true -> [<+modifies(/exceptions) +signed_by(/users/exception_owner.id)>] true"#,
            r#"[<+APPROVE_VARIANCE>] true -> [<+modifies(/variances) +signed_by(/users/variance_owner.id)>] true"#,
            r#"[<+APPROVE_EXTENSION>] true -> [<+modifies(/extensions) +signed_by(/users/extension_owner.id)>] true"#,
            r#"[<+APPROVE_AMENDMENT>] true -> [<+modifies(/amendments) +signed_by(/users/amendment_owner.id)>] true"#,
            r#"[<+APPROVE_ADDENDUM>] true -> [<+modifies(/addenda) +signed_by(/users/addendum_owner.id)>] true"#,
            r#"[<+APPROVE_SUPPLEMENT>] true -> [<+modifies(/supplements) +signed_by(/users/supplement_owner.id)>] true"#,
            r#"[<+APPROVE_APPENDIX>] true -> [<+modifies(/appendices) +signed_by(/users/appendix_owner.id)>] true"#,
            r#"[<+APPROVE_RIDER>] true -> [<+modifies(/riders) +signed_by(/users/rider_owner.id)>] true"#,
            r#"[<+APPROVE_ENDORSEMENT>] true -> [<+modifies(/endorsements) +signed_by(/users/endorsement_owner.id)>] true"#,
            r#"[<+APPROVE_EXHIBIT>] true -> [<+modifies(/exhibits) +signed_by(/users/exhibit_owner.id)>] true"#,
            r#"[<+APPROVE_SCHEDULE>] true -> [<+modifies(/schedules) +signed_by(/users/schedule_owner.id)>] true"#,
            r#"[<+APPROVE_ATTACHMENT>] true -> [<+modifies(/attachments) +signed_by(/users/attachment_owner.id)>] true"#,
            r#"[<+APPROVE_ANNEX>] true -> [<+modifies(/annexes) +signed_by(/users/annex_owner.id)>] true"#,
            r#"[<+APPROVE_ENCLOSURE>] true -> [<+modifies(/enclosures) +signed_by(/users/enclosure_owner.id)>] true"#,
            r#"[<+APPROVE_PACKAGE>] true -> [<+modifies(/packages) +signed_by(/users/package_owner.id)>] true"#,
            r#"[<+APPROVE_BUNDLE>] true -> [<+modifies(/bundles) +signed_by(/users/bundle_owner.id)>] true"#,
            r#"[<+APPROVE_DOSSIER>] true -> [<+modifies(/dossiers) +signed_by(/users/dossier_owner.id)>] true"#,
            r#"[<+APPROVE_FILE>] true -> [<+modifies(/files) +signed_by(/users/file_owner.id)>] true"#,
            r#"[<+APPROVE_RECORD>] true -> [<+modifies(/records) +signed_by(/users/record_owner.id)>] true"#,
            r#"[<+APPROVE_CASE>] true -> [<+modifies(/cases) +signed_by(/users/case_owner.id)>] true"#,
            r#"[<+APPROVE_TICKET>] true -> [<+modifies(/tickets) +signed_by(/users/ticket_owner.id)>] true"#,
            r#"[<+APPROVE_PROPOSAL>] true -> [<+modifies(/proposals) +signed_by(/users/proposal_owner.id)>] true"#,
            r#"[<+APPROVE_REQUEST>] true -> [<+modifies(/requests) +signed_by(/users/request_owner.id)>] true"#,
            r#"[<+APPROVE_APPLICATION>] true -> [<+modifies(/applications) +signed_by(/users/application_owner.id)>] true"#,
            r#"[<+APPROVE_SUBMISSION>] true -> [<+modifies(/submissions) +signed_by(/users/submission_owner.id)>] true"#,
            r#"[<+APPROVE_DOCUMENT>] true -> [<+modifies(/documents) +signed_by(/users/document_owner.id)>] true"#,
            r#"[<+APPROVE_REPORT>] true -> [<+modifies(/reports) +signed_by(/users/report_owner.id)>] true"#,
            r#"[<+APPROVE_MEMO>] true -> [<+modifies(/memos) +signed_by(/users/memo_owner.id)>] true"#,
            r#"[<+APPROVE_NOTE>] true -> [<+modifies(/notes) +signed_by(/users/note_owner.id)>] true"#,
            r#"[<+APPROVE_COMMENT>] true -> [<+modifies(/comments) +signed_by(/users/comment_owner.id)>] true"#,
            r#"[<+APPROVE_REPLY>] true -> [<+modifies(/replies) +signed_by(/users/reply_owner.id)>] true"#,
            r#"[<+APPROVE_FEEDBACK>] true -> [<+modifies(/feedback) +signed_by(/users/feedback_owner.id)>] true"#,
            r#"[<+APPROVE_RATING>] true -> [<+modifies(/ratings) +signed_by(/users/rating_owner.id)>] true"#,
            r#"[<+APPROVE_REVIEW>] true -> [<+modifies(/reviews) +signed_by(/users/review_owner.id)>] true"#,
            r#"[<+APPROVE_SURVEY>] true -> [<+modifies(/surveys) +signed_by(/users/survey_owner.id)>] true"#,
            r#"[<+APPROVE_RESPONSE>] true -> [<+modifies(/responses) +signed_by(/users/response_owner.id)>] true"#,
            r#"[<+APPROVE_RESULT>] true -> [<+modifies(/results) +signed_by(/users/result_owner.id)>] true"#,
            r#"[<+APPROVE_OUTCOME>] true -> [<+modifies(/outcomes) +signed_by(/users/outcome_owner.id)>] true"#,
            r#"[<+APPROVE_DECISION>] true -> [<+modifies(/decisions) +signed_by(/users/decision_owner.id)>] true"#,
            r#"[<+APPROVE_PLAN>] true -> [<+modifies(/plans) +signed_by(/users/plan_owner.id)>] true"#,
            r#"[<+APPROVE_STRATEGY>] true -> [<+modifies(/strategies) +signed_by(/users/strategy_owner.id)>] true"#,
            r#"[<+APPROVE_OBJECTIVE>] true -> [<+modifies(/objectives) +signed_by(/users/objective_owner.id)>] true"#,
            r#"[<+APPROVE_TARGET>] true -> [<+modifies(/targets) +signed_by(/users/target_owner.id)>] true"#,
            r#"[<+APPROVE_GOAL>] true -> [<+modifies(/goals) +signed_by(/users/goal_owner.id)>] true"#,
            r#"[<+APPROVE_KPI>] true -> [<+modifies(/kpis) +signed_by(/users/kpi_owner.id)>] true"#,
            r#"[<+APPROVE_METRIC>] true -> [<+modifies(/metrics) +signed_by(/users/metric_owner.id)>] true"#,
            r#"[<+APPROVE_OKR>] true -> [<+modifies(/okrs) +signed_by(/users/okr_owner.id)>] true"#,
            r#"[<+APPROVE_INITIATIVE>] true -> [<+modifies(/initiatives) +signed_by(/users/initiative_owner.id)>] true"#,
            r#"[<+APPROVE_EPIC>] true -> [<+modifies(/epics) +signed_by(/users/epic_owner.id)>] true"#,
            r#"[<+APPROVE_STORY>] true -> [<+modifies(/stories) +signed_by(/users/story_owner.id)>] true"#,
            r#"[<+APPROVE_TASK>] true -> [<+modifies(/tasks) +signed_by(/users/task_owner.id)>] true"#,
            r#"[<+APPROVE_BUG>] true -> [<+modifies(/bugs) +signed_by(/users/bug_owner.id)>] true"#,
            r#"[<+APPROVE_ISSUE>] true -> [<+modifies(/issues) +signed_by(/users/issue_owner.id)>] true"#,
            r#"[<+APPROVE_DEFECT>] true -> [<+modifies(/defects) +signed_by(/users/defect_owner.id)>] true"#,
            r#"[<+APPROVE_PATCH>] true -> [<+modifies(/patches) +signed_by(/users/patch_owner.id)>] true"#,
            r#"[<+APPROVE_HOTFIX>] true -> [<+modifies(/hotfixes) +signed_by(/users/hotfix_owner.id)>] true"#,
            r#"[<+APPROVE_RELEASE_CANDIDATE>] true -> [<+modifies(/release_candidates) +signed_by(/users/release_manager.id)>] true"#,
            r#"[<+APPROVE_DEPLOYMENT>] true -> [<+modifies(/deployments) +signed_by(/users/deployment_owner.id)>] true"#,
            r#"[<+APPROVE_ROLLOUT>] true -> [<+modifies(/rollouts) +signed_by(/users/rollout_owner.id)>] true"#,
            r#"[<+APPROVE_LAUNCH>] true -> [<+modifies(/launches) +signed_by(/users/launch_owner.id)>] true"#,
            r#"[<+APPROVE_GENERAL_AVAILABILITY>] true -> [<+modifies(/general_availability) +signed_by(/users/ga_owner.id)>] true"#,
            r#"[<+APPROVE_PRODUCTION>] true -> [<+modifies(/production) +signed_by(/users/production_owner.id)>] true"#,
            r#"[<+APPROVE_OPERATIONS>] true -> [<+modifies(/operations) +signed_by(/users/operations_owner.id)>] true"#,
            r#"[<+APPROVE_MAINTENANCE>] true -> [<+modifies(/maintenance) +signed_by(/users/maintenance_owner.id)>] true"#,
            r#"[<+APPROVE_SUPPORT>] true -> [<+modifies(/support) +signed_by(/users/support_owner.id)>] true"#,
            r#"[<+APPROVE_TRAINING>] true -> [<+modifies(/training) +signed_by(/users/training_owner.id)>] true"#,
            r#"[<+APPROVE_COMPLIANCE>] true -> [<+modifies(/compliance) +signed_by(/users/compliance_owner.id)>] true"#,
            r#"[<+APPROVE_ONBOARDING>] true -> [<+modifies(/onboarding) +signed_by(/users/onboarding_owner.id)>] true"#,
            r#"[<+APPROVE_OFFBOARDING>] true -> [<+modifies(/offboarding) +signed_by(/users/offboarding_owner.id)>] true"#,
            r#"[<+APPROVE_DEPROVISIONING>] true -> [<+modifies(/deprovisioning) +signed_by(/users/access_owner.id)>] true"#,
            r#"[<+APPROVE_ACCESS_REVIEW>] true -> [<+modifies(/access_reviews) +signed_by(/users/access_reviewer.id)>] true"#,
            r#"[<+APPROVE_IDENTITY_VERIFICATION>] true -> [<+modifies(/identity_verifications) +signed_by(/users/identity_reviewer.id)>] true"#,
            r#"[<+ISSUE_CREDENTIAL>] true -> [<+modifies(/credentials) +signed_by(/users/credential_issuer.id)>] true"#,
            r#"[<+REVOKE_CREDENTIAL>] true -> [<+modifies(/credential_revocations) +signed_by(/users/credential_issuer.id)>] true"#,
            r#"[<+RENEW_CREDENTIAL>] true -> [<+modifies(/credential_renewals) +signed_by(/users/credential_issuer.id)>] true"#,
            r#"[<+EXPIRE_CREDENTIAL>] true -> [<+modifies(/credential_expirations) +signed_by(/users/credential_issuer.id)>] true"#,
            r#"[<+SUSPEND_CREDENTIAL>] true -> [<+modifies(/credential_suspensions) +signed_by(/users/credential_issuer.id)>] true"#,
            r#"[<+REINSTATE_CREDENTIAL>] true -> [<+modifies(/credential_reinstatements) +signed_by(/users/credential_issuer.id)>] true"#,
            r#"[<+VERIFY_CREDENTIAL>] true -> [<+modifies(/credential_verifications) +signed_by(/users/credential_verifier.id)>] true"#,
            r#"[<+PRESENT_CREDENTIAL>] true -> [<+modifies(/credential_presentations) +signed_by(/users/credential_holder.id)>] true"#,
            r#"[<+SHARE_CREDENTIAL>] true -> [<+modifies(/credential_shares) +signed_by(/users/credential_holder.id)>] true"#,
            r#"[<+EXPORT_CREDENTIAL>] true -> [<+modifies(/credential_exports) +signed_by(/users/credential_holder.id)>] true"#,
            r#"[<+REQUEST_CREDENTIAL>] true -> [<+modifies(/credential_requests) +signed_by(/users/credential_holder.id)>] true"#,
            r#"[<+ACCEPT_CREDENTIAL>] true -> [<+modifies(/credential_acceptances) +signed_by(/users/credential_holder.id)>] true"#,
            r#"[<+REJECT_CREDENTIAL>] true -> [<+modifies(/credential_rejections) +signed_by(/users/credential_holder.id)>] true"#,
        ],
    },
    FormulaExampleGroup {
        title: r#"Authorization with eventual goals"#,
        description: r#"Authorized actions that also create follow-up obligations."#,
        formulas: &[
            r#"[+RELEASE] true -> (<+signed_by(/users/buyer.id)> true & (eventually(<+DEPOSIT> true) & eventually(<+DELIVER> true)))"#,
            r#"[+APPROVE] true -> (<+signed_by(/users/alice.id) +signed_by(/users/bob.id)> true & (eventually(<+DEPOSIT> true) & eventually(<+DELIVER> true)))"#,
            r#"[<+RELEASE>] true -> (<+signed_by(/users/buyer.id)> true & (eventually(<+DEPOSIT> true) & eventually(<+DELIVER> true)))"#,
            r#"[<+APPROVE>] true -> (<+signed_by(/users/alice.id) +signed_by(/users/bob.id)> true & (eventually(<+DEPOSIT> true) & eventually(<+DELIVER> true)))"#,
            r#"[+RELEASE] true -> ([<+signed_by(/users/buyer.id)>] true & (eventually(<+DEPOSIT> true) & eventually(<+DELIVER> true)))"#,
            r#"[<+RELEASE>] true -> ([<+signed_by(/users/buyer.id)>] true & (eventually(<+DEPOSIT> true) & eventually(<+DELIVER> true)))"#,
            r#"[+RELEASE] true -> ([<+signed_by(/users/buyer.id)>] true & (eventually([<+DEPOSIT>] true) & eventually([<+DELIVER>] true)))"#,
            r#"[+RELEASE] true -> (<+signed_by(/users/buyer.id)> true & (eventually([<+DEPOSIT>] true) & eventually([<+DELIVER>] true)))"#,
            r#"[<+RELEASE>] true -> (<+signed_by(/users/buyer.id)> true & (eventually([<+DEPOSIT>] true) & eventually([<+DELIVER>] true)))"#,
            r#"[<+RELEASE>] true -> ([<+signed_by(/users/buyer.id)>] true & (eventually([<+DEPOSIT>] true) & eventually([<+DELIVER>] true)))"#,
            r#"[+APPROVE] true -> (<+signed_by(/users/alice.id) +signed_by(/users/bob.id)> true & (eventually([<+DEPOSIT>] true) & eventually([<+DELIVER>] true)))"#,
            r#"[<+APPROVE>] true -> (<+signed_by(/users/alice.id) +signed_by(/users/bob.id)> true & (eventually([<+DEPOSIT>] true) & eventually([<+DELIVER>] true)))"#,
            r#"[+APPROVE] true -> ([<+signed_by(/users/alice.id) +signed_by(/users/bob.id)>] true & (eventually(<+DEPOSIT> true) & eventually(<+DELIVER> true)))"#,
            r#"[<+APPROVE>] true -> ([<+signed_by(/users/alice.id) +signed_by(/users/bob.id)>] true & (eventually(<+DEPOSIT> true) & eventually(<+DELIVER> true)))"#,
            r#"[+APPROVE] true -> ([<+signed_by(/users/alice.id) +signed_by(/users/bob.id)>] true & (eventually([<+DEPOSIT>] true) & eventually([<+DELIVER>] true)))"#,
            r#"[<+APPROVE>] true -> ([<+signed_by(/users/alice.id) +signed_by(/users/bob.id)>] true & (eventually([<+DEPOSIT>] true) & eventually([<+DELIVER>] true)))"#,
            r#"[+RELEASE] true -> (<+signed_by(/users/buyer.id)> true & eventually(<+DELIVER> true))"#,
            r#"[+RELEASE] true -> (<+signed_by(/users/buyer.id)> true & eventually([<+DELIVER>] true))"#,
            r#"[<+RELEASE>] true -> (<+signed_by(/users/buyer.id)> true & eventually(<+DELIVER> true))"#,
            r#"[<+RELEASE>] true -> (<+signed_by(/users/buyer.id)> true & eventually([<+DELIVER>] true))"#,
            r#"always([+USE_TOOL] true -> (<+signed_by(/users/tool_provider.id)> true & eventually([<+APPROVE_CAPABILITY>] true)))"#,
            r#"[+APPROVE] true -> (<+signed_by(/users/alice.id) +signed_by(/users/bob.id)> true & eventually(<+DELIVER> true))"#,
            r#"[+APPROVE] true -> (<+signed_by(/users/alice.id) +signed_by(/users/bob.id)> true & eventually([<+DELIVER>] true))"#,
            r#"[+APPROVE] true -> ([<+signed_by(/users/alice.id) +signed_by(/users/bob.id)>] true & eventually(<+DELIVER> true))"#,
            r#"[+APPROVE] true -> ([<+signed_by(/users/alice.id) +signed_by(/users/bob.id)>] true & eventually([<+DELIVER>] true))"#,
            r#"[<+APPROVE>] true -> (<+signed_by(/users/alice.id) +signed_by(/users/bob.id)> true & eventually(<+DELIVER> true))"#,
            r#"[<+APPROVE>] true -> (<+signed_by(/users/alice.id) +signed_by(/users/bob.id)> true & eventually([<+DELIVER>] true))"#,
            r#"[<+APPROVE>] true -> ([<+signed_by(/users/alice.id) +signed_by(/users/bob.id)>] true & eventually(<+DELIVER> true))"#,
            r#"[<+APPROVE>] true -> ([<+signed_by(/users/alice.id) +signed_by(/users/bob.id)>] true & eventually([<+DELIVER>] true))"#,
            r#"[+RELEASE] true -> (<+oracle_attests(/oracles/delivery.id, "delivered", "true")> true & (eventually(<+DEPOSIT> true) & eventually(<+DELIVER> true)))"#,
            r#"[+RELEASE] true -> (<+oracle_attests(/oracles/delivery.id, "delivered", "true")> true & eventually(<+DELIVER> true))"#,
            r#"[+RELEASE] true -> (<+oracle_attests(/oracles/delivery.id, "delivered", "true")> true & eventually([<+DELIVER>] true))"#,
            r#"[+RELEASE] true -> (<+oracle_attests(/oracles/delivery.id, "delivered", "true")> true & (eventually([<+DEPOSIT>] true) & eventually([<+DELIVER>] true)))"#,
            r#"[+RELEASE] true -> ([<+signed_by(/users/buyer.id)>] true & eventually(<+DELIVER> true))"#,
            r#"[<+RELEASE>] true -> ([<+signed_by(/users/buyer.id)>] true & eventually(<+DELIVER> true))"#,
            r#"[+RELEASE] true -> ([<+signed_by(/users/buyer.id)>] true & eventually([<+DELIVER>] true))"#,
            r#"[<+RELEASE>] true -> ([<+signed_by(/users/buyer.id)>] true & eventually([<+DELIVER>] true))"#,
            r#"[<+RELEASE>] true -> (<+oracle_attests(/oracles/delivery.id, "delivered", "true")> true & eventually(<+DELIVER> true))"#,
            r#"[<+RELEASE>] true -> (<+oracle_attests(/oracles/delivery.id, "delivered", "true")> true & eventually([<+DELIVER>] true))"#,
            r#"[<+RELEASE>] true -> (<+oracle_attests(/oracles/delivery.id, "delivered", "true")> true & (eventually(<+DEPOSIT> true) & eventually(<+DELIVER> true)))"#,
            r#"[<+RELEASE>] true -> (<+oracle_attests(/oracles/delivery.id, "delivered", "true")> true & (eventually([<+DEPOSIT>] true) & eventually([<+DELIVER>] true)))"#,
        ],
    },
    FormulaExampleGroup {
        title: r#"Forbidden-after guards"#,
        description: r#"Actions that block later transitions, optionally behind authorization."#,
        formulas: &[
            r#"<+CANCEL> true & ([+DISPUTE] true -> always([-RELEASE] true))"#,
            r#"always([+APPROVE] true -> always([-REJECT] true))"#,
            r#"always([+REJECT] true -> always([-APPROVE] true))"#,
            r#"always([+CANCEL] true -> always([-DELIVER] true))"#,
            r#"always([+REFUND] true -> always([-RELEASE] true))"#,
            r#"always([+TIMEOUT] true -> always([-COMPLETE] true))"#,
            r#"always([+ESCALATE] true -> always([-CLOSE] true))"#,
            r#"always([+WITHDRAW] true -> always([-CLAIM] true))"#,
            r#"always([+APPEAL] true -> always([-ENFORCE] true))"#,
            r#"always([+REVOKE] true -> always([-USE] true))"#,
            r#"always([+SUSPEND] true -> always([-ACCESS] true))"#,
            r#"always([<+CANCEL>] true -> always([-DELIVER] true))"#,
            r#"always([<+REFUND>] true -> always([-RELEASE] true))"#,
            r#"always([<+TIMEOUT>] true -> always([-COMPLETE] true))"#,
            r#"always([<+ESCALATE>] true -> always([-CLOSE] true))"#,
            r#"always([<+WITHDRAW>] true -> always([-CLAIM] true))"#,
            r#"always([<+APPEAL>] true -> always([-ENFORCE] true))"#,
            r#"always([<+REVOKE>] true -> always([-USE] true))"#,
            r#"always([<+SUSPEND>] true -> always([-ACCESS] true))"#,
            r#"always([+REINSTATE] true -> always([-SUSPEND] true))"#,
            r#"always([+RENEW] true -> always([-EXPIRE] true))"#,
            r#"always([+TERMINATE] true -> always([-RENEW] true))"#,
            r#"always([+EXTEND] true -> always([-TERMINATE] true))"#,
            r#"always([+ASSIGN] true -> always([-REASSIGN] true))"#,
            r#"always([+CERTIFY] true -> always([-DEPLOY] true))"#,
            r#"always([+PUBLISH] true -> always([-EMBARGO] true))"#,
            r#"always([+REGISTER] true -> always([-DELETE] true))"#,
            r#"always([+ACCEPT] true -> always([-REJECT] true))"#,
            r#"always([+ACKNOWLEDGE] true -> always([-DISPUTE] true))"#,
            r#"always([+CONFIRM_DELIVERY] true -> always([-REFUND] true))"#,
            r#"always([<+REINSTATE>] true -> always([-SUSPEND] true))"#,
            r#"always([<+RENEW>] true -> always([-EXPIRE] true))"#,
            r#"always([<+TERMINATE>] true -> always([-RENEW] true))"#,
            r#"always([<+EXTEND>] true -> always([-TERMINATE] true))"#,
            r#"always([<+ASSIGN>] true -> always([-REASSIGN] true))"#,
            r#"always([<+CERTIFY>] true -> always([-DEPLOY] true))"#,
            r#"always([<+PUBLISH>] true -> always([-EMBARGO] true))"#,
            r#"always([<+REGISTER>] true -> always([-DELETE] true))"#,
            r#"always([<+ACCEPT>] true -> always([-REJECT] true))"#,
            r#"always([<+ACKNOWLEDGE>] true -> always([-DISPUTE] true))"#,
            r#"always([<+CONFIRM_DELIVERY>] true -> always([-REFUND] true))"#,
            r#"always([+APPROVE_INVOICE] true -> always([-CHARGEBACK] true))"#,
            r#"always([+ACCEPT_MILESTONE] true -> always([-REWORK] true))"#,
            r#"always([+APPROVE_INSPECTION] true -> always([-DEFECT_CLAIM] true))"#,
            r#"always([+ATTEST_COMPLIANCE] true -> always([-NONCOMPLIANCE_FINDING] true))"#,
            r#"always([+APPROVE_SAFETY] true -> always([-UNSAFE_DEPLOYMENT] true))"#,
            r#"always([+ACCEPT_RISK] true -> always([-UNMITIGATED_EXPOSURE] true))"#,
            r#"always([+CLOSE_INCIDENT] true -> always([-REOPEN_INCIDENT] true))"#,
            r#"always([+FREEZE_CHANGE] true -> always([-DEPLOY] true))"#,
            r#"always([+DISPUTE] true -> (always([-RELEASE] true) & always([-REFUND] true)))"#,
            r#"[+DISPUTE] true -> (always([-RELEASE] true) & always([-REFUND] true))"#,
            r#"[<+DISPUTE>] true -> always([-RELEASE] true)"#,
            r#"[<+DISPUTE>] true -> (always([-RELEASE] true) & always([-REFUND] true))"#,
            r#"[+DISPUTE] true -> (<+signed_by(/users/arbiter.id)> true & always([-RELEASE] true))"#,
            r#"[+DISPUTE] true -> (<+signed_by(/users/arbiter.id)> true & (always([-RELEASE] true) & always([-REFUND] true)))"#,
            r#"[+DISPUTE] true -> (<+signed_by(/users/alice.id) +signed_by(/users/bob.id)> true & always([-RELEASE] true))"#,
            r#"[+DISPUTE] true -> (<+signed_by(/users/alice.id) +signed_by(/users/bob.id)> true & (always([-RELEASE] true) & always([-REFUND] true)))"#,
            r#"[+DISPUTE] true -> ([<+signed_by(/users/arbiter.id)>] true & always([-RELEASE] true))"#,
            r#"[+DISPUTE] true -> ([<+signed_by(/users/alice.id) +signed_by(/users/bob.id)>] true & always([-RELEASE] true))"#,
            r#"[+DISPUTE] true -> ([<+signed_by(/users/alice.id) +signed_by(/users/bob.id)>] true & (always([-RELEASE] true) & always([-REFUND] true)))"#,
            r#"[+DISPUTE] true -> (<+oracle_attests(/oracles/dispute.id, "opened", "true")> true & always([-RELEASE] true))"#,
            r#"[+DISPUTE] true -> (<+oracle_attests(/oracles/dispute.id, "opened", "true")> true & (always([-RELEASE] true) & always([-REFUND] true)))"#,
            r#"[+DISPUTE] true -> ([<+signed_by(/users/arbiter.id)>] true & (always([-RELEASE] true) & always([-REFUND] true)))"#,
            r#"[<+DISPUTE>] true -> (<+signed_by(/users/arbiter.id)> true & always([-RELEASE] true))"#,
            r#"[<+DISPUTE>] true -> (<+signed_by(/users/alice.id) +signed_by(/users/bob.id)> true & always([-RELEASE] true))"#,
            r#"[<+DISPUTE>] true -> (<+signed_by(/users/arbiter.id)> true & (always([-RELEASE] true) & always([-REFUND] true)))"#,
            r#"[<+DISPUTE>] true -> (<+signed_by(/users/alice.id) +signed_by(/users/bob.id)> true & (always([-RELEASE] true) & always([-REFUND] true)))"#,
            r#"[<+DISPUTE>] true -> ([<+signed_by(/users/arbiter.id)>] true & always([-RELEASE] true))"#,
            r#"[<+DISPUTE>] true -> ([<+signed_by(/users/alice.id) +signed_by(/users/bob.id)>] true & always([-RELEASE] true))"#,
            r#"[<+DISPUTE>] true -> ([<+signed_by(/users/arbiter.id)>] true & (always([-RELEASE] true) & always([-REFUND] true)))"#,
            r#"[<+DISPUTE>] true -> ([<+signed_by(/users/alice.id) +signed_by(/users/bob.id)>] true & (always([-RELEASE] true) & always([-REFUND] true)))"#,
            r#"[<+DISPUTE>] true -> (<+oracle_attests(/oracles/dispute.id, "opened", "true")> true & always([-RELEASE] true))"#,
            r#"[<+DISPUTE>] true -> (<+oracle_attests(/oracles/dispute.id, "opened", "true")> true & (always([-RELEASE] true) & always([-REFUND] true)))"#,
            r#"[<+DISPUTE>] true -> ([<+oracle_attests(/oracles/dispute.id, "opened", "true")>] true & always([-RELEASE] true))"#,
            r#"[<+DISPUTE>] true -> ([<+oracle_attests(/oracles/dispute.id, "opened", "true")>] true & (always([-RELEASE] true) & always([-REFUND] true)))"#,
            r#"[+APPROVE_INVOICE] true -> (<+signed_by(/users/finance_approver.id)> true & always([-CHARGEBACK] true))"#,
            r#"[+ACCEPT_MILESTONE] true -> (<+signed_by(/users/client_reviewer.id)> true & always([-REWORK] true))"#,
            r#"[+APPROVE_INSPECTION] true -> (<+signed_by(/users/inspector.id)> true & always([-DEFECT_CLAIM] true))"#,
            r#"[+ATTEST_COMPLIANCE] true -> (<+oracle_attests(/oracles/compliance.id, "status", "clear")> true & always([-NONCOMPLIANCE_FINDING] true))"#,
            r#"[<+APPROVE_INVOICE>] true -> (<+signed_by(/users/finance_approver.id)> true & always([-CHARGEBACK] true))"#,
            r#"[<+ACCEPT_MILESTONE>] true -> (<+signed_by(/users/client_reviewer.id)> true & always([-REWORK] true))"#,
            r#"[<+APPROVE_INSPECTION>] true -> (<+signed_by(/users/inspector.id)> true & always([-DEFECT_CLAIM] true))"#,
            r#"[<+ATTEST_COMPLIANCE>] true -> (<+oracle_attests(/oracles/compliance.id, "status", "clear")> true & always([-NONCOMPLIANCE_FINDING] true))"#,
            r#"[+APPROVE_INVOICE] true -> ([<+signed_by(/users/finance_approver.id)>] true & always([-CHARGEBACK] true))"#,
            r#"[+ACCEPT_MILESTONE] true -> ([<+signed_by(/users/client_reviewer.id)>] true & always([-REWORK] true))"#,
            r#"[+APPROVE_INSPECTION] true -> ([<+signed_by(/users/inspector.id)>] true & always([-DEFECT_CLAIM] true))"#,
            r#"[+ATTEST_COMPLIANCE] true -> ([<+oracle_attests(/oracles/compliance.id, "status", "clear")>] true & always([-NONCOMPLIANCE_FINDING] true))"#,
            r#"[<+APPROVE_INVOICE>] true -> ([<+signed_by(/users/finance_approver.id)>] true & always([-CHARGEBACK] true))"#,
            r#"[<+ACCEPT_MILESTONE>] true -> ([<+signed_by(/users/client_reviewer.id)>] true & always([-REWORK] true))"#,
            r#"[<+APPROVE_INSPECTION>] true -> ([<+signed_by(/users/inspector.id)>] true & always([-DEFECT_CLAIM] true))"#,
            r#"[<+ATTEST_COMPLIANCE>] true -> ([<+oracle_attests(/oracles/compliance.id, "status", "clear")>] true & always([-NONCOMPLIANCE_FINDING] true))"#,
            r#"[+APPROVE_SAFETY] true -> (<+signed_by(/users/safety_officer.id)> true & always([-UNSAFE_DEPLOYMENT] true))"#,
            r#"[+ACCEPT_RISK] true -> (<+signed_by(/users/risk_owner.id)> true & always([-UNMITIGATED_EXPOSURE] true))"#,
            r#"[+CLOSE_INCIDENT] true -> (<+signed_by(/users/incident_commander.id)> true & always([-REOPEN_INCIDENT] true))"#,
            r#"[+FREEZE_CHANGE] true -> (<+signed_by(/users/change_manager.id)> true & always([-DEPLOY] true))"#,
            r#"[<+APPROVE_SAFETY>] true -> (<+signed_by(/users/safety_officer.id)> true & always([-UNSAFE_DEPLOYMENT] true))"#,
            r#"[<+ACCEPT_RISK>] true -> (<+signed_by(/users/risk_owner.id)> true & always([-UNMITIGATED_EXPOSURE] true))"#,
            r#"[<+CLOSE_INCIDENT>] true -> (<+signed_by(/users/incident_commander.id)> true & always([-REOPEN_INCIDENT] true))"#,
            r#"[<+FREEZE_CHANGE>] true -> (<+signed_by(/users/change_manager.id)> true & always([-DEPLOY] true))"#,
            r#"[+APPROVE_SAFETY] true -> ([<+signed_by(/users/safety_officer.id)>] true & always([-UNSAFE_DEPLOYMENT] true))"#,
            r#"[+ACCEPT_RISK] true -> ([<+signed_by(/users/risk_owner.id)>] true & always([-UNMITIGATED_EXPOSURE] true))"#,
            r#"[+CLOSE_INCIDENT] true -> ([<+signed_by(/users/incident_commander.id)>] true & always([-REOPEN_INCIDENT] true))"#,
            r#"[+FREEZE_CHANGE] true -> ([<+signed_by(/users/change_manager.id)>] true & always([-DEPLOY] true))"#,
            r#"[<+APPROVE_SAFETY>] true -> ([<+signed_by(/users/safety_officer.id)>] true & always([-UNSAFE_DEPLOYMENT] true))"#,
            r#"[<+ACCEPT_RISK>] true -> ([<+signed_by(/users/risk_owner.id)>] true & always([-UNMITIGATED_EXPOSURE] true))"#,
            r#"[<+CLOSE_INCIDENT>] true -> ([<+signed_by(/users/incident_commander.id)>] true & always([-REOPEN_INCIDENT] true))"#,
            r#"[<+FREEZE_CHANGE>] true -> ([<+signed_by(/users/change_manager.id)>] true & always([-DEPLOY] true))"#,
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
    output.push_str("  mutual_cooperation  Cooperation game - both must cooperate, defection blocked\n");
    output.push_str("  atomic_swap         Both parties commit before either can claim\n");
    output.push_str("  multisig            N-of-M signature approval pattern\n");
    output.push_str("  turn_taking         Alternating two-party turn cycle\n");
    output.push_str("  service_agreement   Offer -> Accept -> Deliver -> Confirm -> Pay\n");
    output.push_str("  delegation          Principal grants agent authority to act\n");
    output.push_str("  auction             Seller lists, bidders bid, highest wins\n");
    output.push_str("  subscription        Recurring payment for service access\n");
    output.push_str("  milestone           Multi-phase project with payments\n");
    output.push_str("\nUsage:\n");
    output.push_str("  modality model synthesize --template escrow --party-a Buyer --party-b Seller\n");
    output.push_str("\nOr describe in natural language:\n");
    output.push_str(
        "  modality model synthesize --describe \"escrow where buyer deposits funds\"\n",
    );
    output.push_str("  modality model synthesize --describe \"Alice and Bob take turns signing\"\n");
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

fn has_existing_model_inputs(opts: &Opts) -> bool {
    opts.existing_model.is_some() || opts.proposed_formula.is_some() || opts.proposed_rule.is_some()
}

fn has_verifiable_synthesis_inputs(opts: &Opts) -> bool {
    opts.formulas.is_some()
        || opts.rule.is_some()
        || opts.llm_response.is_some()
        || opts.llm_response_file.is_some()
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
    if opts.output.is_some() {
        conflicts.push("--output");
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
    if opts.output.is_some() {
        conflicts.push("--output");
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
    if opts.list {
        conflicts.push("--list");
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
        println!("🔧 Synthesizing a local replacement candidate from existing plus proposed formulas\n");

        let candidate_name = replacement_candidate_name(&existing_input.model);
        let candidate = modality_lang::formula_synthesis::synthesize_from_formulas(
            &candidate_name,
            &candidate_formulas,
        );
        verify_synthesized_model_with_labels(
            &candidate,
            &candidate_formulas,
            &candidate_labels,
        )?;
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

fn parse_formula_string(index: usize, formula: &str) -> Result<Vec<modality_lang::Formula>, String> {
    match modality_lang::parse_all_formulas_content_lalrpop(formula) {
        Ok(parsed) if !parsed.is_empty() => return Ok(parsed),
        Ok(_) => {}
        Err(err) => {
            let wrapped = format!("formula generated_{} {{\n{}\n}}", index + 1, formula);
            return match modality_lang::parse_all_formulas_content_lalrpop(&wrapped) {
                Ok(parsed) if !parsed.is_empty() => Ok(parsed),
                Ok(_) => Err("wrapped expression parse produced no formulas".to_string()),
                Err(wrapped_err) => Err(
                    format!(
                        "declared formula parse failed: {}; wrapped expression parse failed: {}",
                        err, wrapped_err
                    ),
                ),
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

#[cfg(test)]
fn ensure_all_formula_strings_parsed(formulas: &[String]) -> Result<()> {
    parse_formula_inputs(formulas).ensure_all_parsed()
}

#[cfg(test)]
fn unparsed_formula_string_labels(formulas: &[String]) -> Vec<String> {
    parse_formula_inputs(formulas).unparsed
}

#[cfg(test)]
fn parsed_formula_string_labels(formulas: &[String]) -> Vec<String> {
    parse_formula_inputs(formulas).labels
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

fn synthesize_constraints_from_strings(
    formulas: &[String],
) -> modality_lang::formula_synthesis::SynthesisConstraints {
    let mut constraints = modality_lang::formula_synthesis::SynthesisConstraints::default();

    for f in formulas {
        // Look for ordering: [+X] true -> eventually(<+Y> true)
        if (f.contains("->") || f.contains("implies")) && f.contains("eventually") {
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
    if !is_valid_template_identifier_component(party_a) {
        return Err(anyhow::anyhow!(
            "--party-a must contain only letters, numbers, and underscores, and must start with a letter or underscore"
        ));
    }
    if !is_valid_template_identifier_component(party_b) {
        return Err(anyhow::anyhow!(
            "--party-b must contain only letters, numbers, and underscores, and must start with a letter or underscore"
        ));
    }

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

#[cfg(test)]
fn verify_synthesized_model(
    model: &modality_lang::Model,
    formulas: &[modality_lang::FormulaExpr],
) -> Result<()> {
    let labels = (1..=formulas.len())
        .map(|index| format!("F{}", index))
        .collect::<Vec<_>>();
    verify_synthesized_model_with_labels(model, formulas, &labels)
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
                                        format_fallback_predicate_arg(arg.as_str().unwrap_or(""))
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
                    format!(": {}", props.join(" "))
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

fn format_fallback_predicate_arg(arg: &str) -> String {
    if is_valid_template_identifier_component(arg) || is_path_literal(arg) {
        arg.to_string()
    } else {
        format!("\"{}\"", arg.replace('\\', "\\\\").replace('"', "\\\""))
    }
}

fn is_path_literal(value: &str) -> bool {
    value.starts_with('/')
        && value
            .chars()
            .skip(1)
            .all(|ch| ch == '_' || ch == '.' || ch == '/' || ch.is_ascii_alphanumeric())
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
            output: None,
            verify: false,
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

        let alias_opts = Opts::try_parse_from(["synthesize", "--template", "alternating"])
            .expect("alternating alias parses");
        assert_eq!(alias_opts.template.as_deref(), Some("alternating"));

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

        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &parsed);
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
    fn parse_formula_strings_accepts_multiple_declarations_from_one_input() {
        let formulas = vec![
            r#"
formula approval_required {
always([<+APPROVE>] true)
}

formula approval_signed {
[+APPROVE] true -> <+signed_by(/users/reviewer.id)> true
}
"#
            .to_string(),
        ];

        let parsed = parse_formula_strings(&formulas);
        assert_eq!(parsed.len(), 2);

        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &parsed);

        verify_synthesized_model(&model, &parsed).unwrap();
    }

    #[test]
    fn rule_file_content_with_multiple_formula_declarations_verifies() {
        let path = std::env::temp_dir().join(format!(
            "modality-synthesize-rules-{}.modality",
            std::process::id()
        ));
        std::fs::write(
            &path,
            r#"
formula approval_required {
always([<+APPROVE>] true)
}

formula approval_signed {
[+APPROVE] true -> <+signed_by(/users/reviewer.id)> true
}
"#,
        )
        .unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        std::fs::remove_file(&path).unwrap();

        let parsed_input = parse_formula_inputs(std::slice::from_ref(&content));
        parsed_input.ensure_all_parsed().unwrap();
        assert_eq!(parsed_input.formulas.len(), 2);

        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "Contract",
            &parsed_input.formulas,
        );

        verify_synthesized_model(&model, &parsed_input.formulas).unwrap();
    }

    #[tokio::test]
    async fn rule_file_mode_reports_missing_rule_path() {
        let rule_path = std::env::temp_dir().join(format!(
            "modality-synthesize-missing-rule-{}.modality",
            std::process::id()
        ));

        let mut opts = default_test_opts();
        opts.rule = Some(rule_path.clone());
        opts.verify = true;

        let err = run(&opts).await.unwrap_err();

        let message = err.to_string();
        assert!(message.contains("Failed to read rule file"));
        assert!(message.contains(&rule_path.display().to_string()));
    }

    #[tokio::test]
    async fn invalid_format_is_rejected_before_reading_rule_path() {
        let rule_path = std::env::temp_dir().join(format!(
            "modality-synthesize-invalid-format-rule-{}.modality",
            std::process::id()
        ));

        let mut opts = default_test_opts();
        opts.rule = Some(rule_path.clone());
        opts.format = "yaml".to_string();

        let err = run(&opts).await.unwrap_err();

        let message = err.to_string();
        assert!(message.contains("Unknown format: 'yaml'. Use 'modality' or 'json'."));
        assert!(!message.contains(&rule_path.display().to_string()));
    }

    #[tokio::test]
    async fn rule_file_mode_rejects_milestones_mode() {
        let mut opts = default_test_opts();
        opts.rule = Some(PathBuf::from("rules.modality"));
        opts.milestones = Some("Phase1,Phase2".to_string());

        let err = run(&opts).await.unwrap_err();

        assert!(
            err.to_string()
                .contains("--rule cannot be combined with other synthesis modes: --milestones")
        );
    }

    #[tokio::test]
    async fn rule_file_mode_rejects_llm_response_file_before_reading_paths() {
        let missing_rule_path = std::env::temp_dir().join(format!(
            "modality-synthesize-rule-conflict-{}.modality",
            std::process::id()
        ));
        let missing_response_path = std::env::temp_dir().join(format!(
            "modality-synthesize-rule-response-conflict-{}.md",
            std::process::id()
        ));

        let mut opts = default_test_opts();
        opts.rule = Some(missing_rule_path.clone());
        opts.llm_response_file = Some(missing_response_path.clone());

        let err = run(&opts).await.unwrap_err();

        let message = err.to_string();
        assert!(message.contains(
            "--rule cannot be combined with other synthesis modes: --llm-response-file"
        ));
        assert!(!message.contains(&missing_rule_path.display().to_string()));
        assert!(!message.contains(&missing_response_path.display().to_string()));
    }

    #[tokio::test]
    async fn rule_file_fallback_rejects_invalid_party_a() {
        let rule_path = std::env::temp_dir().join(format!(
            "modality-synthesize-rule-fallback-party-{}.txt",
            std::process::id()
        ));
        std::fs::write(&rule_path, "fallback rule text without parser formulas").unwrap();

        let mut opts = default_test_opts();
        opts.rule = Some(rule_path.clone());
        opts.party_a = "Alice Smith".to_string();

        let err = run(&opts).await.unwrap_err();
        std::fs::remove_file(rule_path).unwrap();

        assert!(err.to_string().contains("--party-a must contain only"));
    }

    #[tokio::test]
    async fn rule_file_with_explicit_signer_ignores_unused_invalid_parties() {
        let rule_path = std::env::temp_dir().join(format!(
            "modality-synthesize-rule-explicit-signer-{}.txt",
            std::process::id()
        ));
        let output_path = std::env::temp_dir().join(format!(
            "modality-synthesize-rule-explicit-signer-output-{}.modality",
            std::process::id()
        ));
        std::fs::write(&rule_path, "signed_by(/users/reviewer.id)").unwrap();

        let mut opts = default_test_opts();
        opts.rule = Some(rule_path.clone());
        opts.output = Some(output_path.clone());
        opts.party_a = "Alice Smith".to_string();
        opts.party_b = "2Bob".to_string();

        run(&opts).await.unwrap();

        let output = std::fs::read_to_string(&output_path).unwrap();
        std::fs::remove_file(rule_path).unwrap();
        std::fs::remove_file(output_path).unwrap();

        assert!(output.contains("+signed_by(/users/reviewer.id)"));
    }

    #[tokio::test]
    async fn rule_file_fallback_quotes_explicit_signer_args_when_needed() {
        let rule_path = std::env::temp_dir().join(format!(
            "modality-synthesize-rule-quoted-signer-{}.txt",
            std::process::id()
        ));
        let output_path = std::env::temp_dir().join(format!(
            "modality-synthesize-rule-quoted-signer-output-{}.modality",
            std::process::id()
        ));
        std::fs::write(&rule_path, "signed_by(reviewer key)").unwrap();

        let mut opts = default_test_opts();
        opts.rule = Some(rule_path.clone());
        opts.output = Some(output_path.clone());

        run(&opts).await.unwrap();

        let output = std::fs::read_to_string(&output_path).unwrap();
        std::fs::remove_file(rule_path).unwrap();
        std::fs::remove_file(output_path).unwrap();

        assert!(output.contains("+signed_by(\"reviewer key\")"));
        modality_lang::parse_all_models_content_lalrpop(&output).unwrap();
    }

    #[tokio::test]
    async fn rule_file_verify_writes_checked_model() {
        let rule_path = std::env::temp_dir().join(format!(
            "modality-synthesize-rules-run-{}.modality",
            std::process::id()
        ));
        let output_path = std::env::temp_dir().join(format!(
            "modality-synthesize-rules-output-{}.modality",
            std::process::id()
        ));
        std::fs::write(
            &rule_path,
            r#"
formula generated_1 {
lfp(X, ((<+REVIEW> true) & ((<+WAIT> true) & <>((X)))) | (<+APPROVE> true))
}

formula generated_2 {
gfp(X, []((X)) & ([<+ARCHIVE>] true))
}
"#,
        )
        .unwrap();

        let mut opts = default_test_opts();
        opts.rule = Some(rule_path.clone());
        opts.output = Some(output_path.clone());
        opts.verify = true;

        run(&opts).await.unwrap();

        let output = std::fs::read_to_string(&output_path).unwrap();
        std::fs::remove_file(rule_path).unwrap();
        std::fs::remove_file(output_path).unwrap();

        let models = modality_lang::parse_all_models_content_lalrpop(&output).unwrap();
        assert_eq!(models.len(), 1);
        assert!(output.contains("model Contract"));
        assert!(output.contains("+APPROVE"));
        assert!(output.contains("+REVIEW"));
        assert!(output.contains("+WAIT"));
        assert!(output.contains("+ARCHIVE"));
    }

    #[tokio::test]
    async fn rule_file_verify_writes_json_model() {
        let rule_path = std::env::temp_dir().join(format!(
            "modality-synthesize-rules-json-{}.modality",
            std::process::id()
        ));
        let output_path = std::env::temp_dir().join(format!(
            "modality-synthesize-rules-output-{}.json",
            std::process::id()
        ));
        std::fs::write(
            &rule_path,
            r#"
formula generated_1 {
lfp(X, ((<+REVIEW> true) & ((<+WAIT> true) & <>((X)))) | (<+APPROVE> true))
}

formula generated_2 {
gfp(X, []((X)) & ([<+ARCHIVE>] true))
}
"#,
        )
        .unwrap();

        let mut opts = default_test_opts();
        opts.rule = Some(rule_path.clone());
        opts.output = Some(output_path.clone());
        opts.verify = true;
        opts.format = "json".to_string();

        run(&opts).await.unwrap();

        let output = std::fs::read_to_string(&output_path).unwrap();
        std::fs::remove_file(rule_path).unwrap();
        std::fs::remove_file(output_path).unwrap();

        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert_eq!(parsed["name"], "Contract");
        let action_names = parsed["parts"][0]["transitions"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|transition| transition["properties"].as_array().unwrap())
            .map(|property| property["name"].as_str().unwrap())
            .collect::<Vec<_>>();

        assert!(action_names.contains(&"APPROVE"));
        assert!(action_names.contains(&"REVIEW"));
        assert!(action_names.contains(&"WAIT"));
        assert!(action_names.contains(&"ARCHIVE"));
    }

    #[tokio::test]
    async fn formula_mode_verify_writes_checked_fixed_point_model() {
        let output_path = std::env::temp_dir().join(format!(
            "modality-synthesize-fixed-point-output-{}.modality",
            std::process::id()
        ));

        let mut opts = default_test_opts();
        opts.formulas = Some(
            [
                "lfp(X, ((<+REVIEW> true) & ((<+WAIT> true) & <>((X)))) | (<+APPROVE> true))",
                "gfp(X, []((X)) & ([<+ARCHIVE>] true))",
            ]
            .join("; "),
        );
        opts.output = Some(output_path.clone());
        opts.verify = true;

        run(&opts).await.unwrap();

        let output = std::fs::read_to_string(&output_path).unwrap();
        std::fs::remove_file(output_path).unwrap();

        let models = modality_lang::parse_all_models_content_lalrpop(&output).unwrap();
        assert_eq!(models.len(), 1);
        assert!(output.contains("model Contract"));
        assert!(output.contains("+APPROVE"));
        assert!(output.contains("+REVIEW"));
        assert!(output.contains("+WAIT"));
        assert!(output.contains("+ARCHIVE"));
    }

    #[tokio::test]
    async fn formula_mode_verify_writes_checked_unlabeled_committed_fixed_point_model() {
        let output_path = std::env::temp_dir().join(format!(
            "modality-synthesize-unlabeled-committed-fixed-point-output-{}.modality",
            std::process::id()
        ));

        let mut opts = default_test_opts();
        opts.formulas = Some(
            [
                "lfp(X, [<>]X | ([<+APPROVE>] true))",
                "gfp(X, [<>]X & ([<+ARCHIVE>] true))",
            ]
            .join("; "),
        );
        opts.output = Some(output_path.clone());
        opts.verify = true;

        run(&opts).await.unwrap();

        let output = std::fs::read_to_string(&output_path).unwrap();
        std::fs::remove_file(output_path).unwrap();

        let models = modality_lang::parse_all_models_content_lalrpop(&output).unwrap();
        assert_eq!(models.len(), 1);
        assert!(output.contains("model Contract"));
        assert!(output.contains("+APPROVE"));
        assert!(output.contains("+ARCHIVE"));
    }

    #[tokio::test]
    async fn formula_mode_verify_writes_parenthesized_unlabeled_committed_fixed_point_model() {
        let output_path = std::env::temp_dir().join(format!(
            "modality-synthesize-parenthesized-unlabeled-committed-fixed-point-output-{}.modality",
            std::process::id()
        ));

        let mut opts = default_test_opts();
        opts.formulas = Some(
            [
                "lfp(X, [<>](X) | ([<+APPROVE>] true))",
                "gfp(X, [<>]((X)) & ([<+ARCHIVE>] true))",
            ]
            .join("; "),
        );
        opts.output = Some(output_path.clone());
        opts.verify = true;

        run(&opts).await.unwrap();

        let output = std::fs::read_to_string(&output_path).unwrap();
        std::fs::remove_file(output_path).unwrap();

        let models = modality_lang::parse_all_models_content_lalrpop(&output).unwrap();
        assert_eq!(models.len(), 1);
        assert!(output.contains("model Contract"));
        assert!(output.contains("+APPROVE"));
        assert!(output.contains("+ARCHIVE"));
    }

    #[tokio::test]
    async fn formula_mode_verify_writes_json_model() {
        let output_path = std::env::temp_dir().join(format!(
            "modality-synthesize-fixed-point-json-output-{}.json",
            std::process::id()
        ));

        let mut opts = default_test_opts();
        opts.formulas = Some(
            [
                "lfp(X, ((<+REVIEW> true) & ((<+WAIT> true) & <>((X)))) | (<+APPROVE> true))",
                "gfp(X, []((X)) & ([<+ARCHIVE>] true))",
            ]
            .join("; "),
        );
        opts.output = Some(output_path.clone());
        opts.verify = true;
        opts.format = "json".to_string();

        run(&opts).await.unwrap();

        let output = std::fs::read_to_string(&output_path).unwrap();
        std::fs::remove_file(output_path).unwrap();

        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert_eq!(parsed["name"], "Contract");
        let action_names = parsed["parts"][0]["transitions"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|transition| transition["properties"].as_array().unwrap())
            .map(|property| property["name"].as_str().unwrap())
            .collect::<Vec<_>>();

        assert!(action_names.contains(&"APPROVE"));
        assert!(action_names.contains(&"REVIEW"));
        assert!(action_names.contains(&"WAIT"));
        assert!(action_names.contains(&"ARCHIVE"));
    }

    #[tokio::test]
    async fn verify_rejects_prompt_generation_mode() {
        let mut opts = default_test_opts();
        opts.describe = Some("Generate approval rules".to_string());
        opts.generate_prompt = true;
        opts.verify = true;

        let err = run(&opts).await.unwrap_err();

        assert!(err.to_string().contains(
            "--generate-prompt cannot be combined with other synthesis modes: --verify"
        ));
    }

    #[tokio::test]
    async fn prompt_generation_mode_rejects_template_before_missing_description() {
        let mut opts = default_test_opts();
        opts.generate_prompt = true;
        opts.template = Some("escrow".to_string());

        let err = run(&opts).await.unwrap_err();

        let message = err.to_string();
        assert!(message.contains(
            "--generate-prompt cannot be combined with other synthesis modes: --template"
        ));
        assert!(!message.contains("--generate-prompt requires --describe"));
    }

    #[tokio::test]
    async fn prompt_generation_mode_rejects_llm_response_file_before_reading_path() {
        let missing_response_path = std::env::temp_dir().join(format!(
            "modality-synthesize-prompt-llm-response-{}.md",
            std::process::id()
        ));

        let mut opts = default_test_opts();
        opts.describe = Some("Generate approval rules".to_string());
        opts.generate_prompt = true;
        opts.llm_response_file = Some(missing_response_path.clone());

        let err = run(&opts).await.unwrap_err();

        let message = err.to_string();
        assert!(message.contains(
            "--generate-prompt cannot be combined with other synthesis modes: --llm-response-file"
        ));
        assert!(!message.contains(&missing_response_path.display().to_string()));
    }

    #[tokio::test]
    async fn prompt_generation_mode_rejects_llm_response_file_before_missing_description() {
        let missing_response_path = std::env::temp_dir().join(format!(
            "modality-synthesize-prompt-missing-description-{}.md",
            std::process::id()
        ));

        let mut opts = default_test_opts();
        opts.generate_prompt = true;
        opts.llm_response_file = Some(missing_response_path.clone());

        let err = run(&opts).await.unwrap_err();

        let message = err.to_string();
        assert!(message.contains(
            "--generate-prompt cannot be combined with other synthesis modes: --llm-response-file"
        ));
        assert!(!message.contains("--generate-prompt requires --describe"));
        assert!(!message.contains(&missing_response_path.display().to_string()));
    }

    #[tokio::test]
    async fn verify_rejects_list_mode() {
        let mut opts = default_test_opts();
        opts.list = true;
        opts.verify = true;

        let err = run(&opts).await.unwrap_err();

        assert!(err.to_string().contains(
            "--list cannot be combined with other synthesis modes: --verify"
        ));
    }

    #[tokio::test]
    async fn list_mode_rejects_llm_response_file_before_reading_path() {
        let missing_response_path = std::env::temp_dir().join(format!(
            "modality-synthesize-list-llm-response-{}.md",
            std::process::id()
        ));

        let mut opts = default_test_opts();
        opts.list = true;
        opts.llm_response_file = Some(missing_response_path.clone());

        let err = run(&opts).await.unwrap_err();

        let message = err.to_string();
        assert!(message.contains(
            "--list cannot be combined with other synthesis modes: --llm-response-file"
        ));
        assert!(!message.contains(&missing_response_path.display().to_string()));
    }

    #[tokio::test]
    async fn invalid_format_is_rejected_before_reading_llm_response_file() {
        let missing_response_path = std::env::temp_dir().join(format!(
            "modality-synthesize-invalid-format-response-{}.md",
            std::process::id()
        ));

        let mut opts = default_test_opts();
        opts.llm_response_file = Some(missing_response_path.clone());
        opts.format = "yaml".to_string();

        let err = run(&opts).await.unwrap_err();

        let message = err.to_string();
        assert!(message.contains("Unknown format: 'yaml'. Use 'modality' or 'json'."));
        assert!(!message.contains(&missing_response_path.display().to_string()));
    }

    #[tokio::test]
    async fn verify_rejects_describe_mode() {
        let mut opts = default_test_opts();
        opts.describe = Some("escrow where buyer deposits funds".to_string());
        opts.verify = true;

        let err = run(&opts).await.unwrap_err();

        assert!(err
            .to_string()
            .contains("--describe cannot be combined with other synthesis modes: --verify"));
    }

    #[tokio::test]
    async fn describe_mode_rejects_llm_response_file_before_reading_path() {
        let missing_response_path = std::env::temp_dir().join(format!(
            "modality-synthesize-describe-llm-response-{}.md",
            std::process::id()
        ));

        let mut opts = default_test_opts();
        opts.describe = Some("escrow where buyer deposits funds".to_string());
        opts.llm_response_file = Some(missing_response_path.clone());

        let err = run(&opts).await.unwrap_err();

        let message = err.to_string();
        assert!(message.contains(
            "--describe cannot be combined with other synthesis modes: --llm-response-file"
        ));
        assert!(!message.contains(&missing_response_path.display().to_string()));
    }

    #[tokio::test]
    async fn verify_rejects_template_mode() {
        let mut opts = default_test_opts();
        opts.template = Some("escrow".to_string());
        opts.verify = true;

        let err = run(&opts).await.unwrap_err();

        assert!(
            err.to_string()
                .contains("--template cannot be combined with other synthesis modes: --verify")
        );
    }

    #[tokio::test]
    async fn template_mode_rejects_irrelevant_milestones() {
        let mut opts = default_test_opts();
        opts.template = Some("escrow".to_string());
        opts.milestones = Some("Phase1,Phase2".to_string());

        let err = run(&opts).await.unwrap_err();

        assert!(
            err.to_string()
                .contains("--milestones can only be used with --template milestone")
        );
    }

    #[tokio::test]
    async fn template_mode_rejects_unknown_template_before_milestones() {
        let mut opts = default_test_opts();
        opts.template = Some("made_up_template".to_string());
        opts.milestones = Some("Phase1,Phase2".to_string());

        let err = run(&opts).await.unwrap_err();

        assert!(err
            .to_string()
            .contains("Unknown template: 'made_up_template'"));
    }

    #[tokio::test]
    async fn template_mode_rejects_unknown_template_before_party_names() {
        let mut opts = default_test_opts();
        opts.template = Some("made_up_template".to_string());
        opts.party_a = "Alice Smith".to_string();

        let err = run(&opts).await.unwrap_err();

        assert!(err
            .to_string()
            .contains("Unknown template: 'made_up_template'"));
    }

    #[tokio::test]
    async fn template_mode_rejects_invalid_party_a() {
        let mut opts = default_test_opts();
        opts.template = Some("escrow".to_string());
        opts.party_a = "Alice Smith".to_string();

        let err = run(&opts).await.unwrap_err();

        assert!(err.to_string().contains("--party-a must contain only"));
    }

    #[tokio::test]
    async fn template_mode_rejects_digit_started_party_b() {
        let mut opts = default_test_opts();
        opts.template = Some("service_agreement".to_string());
        opts.party_b = "2Consumer".to_string();

        let err = run(&opts).await.unwrap_err();

        assert!(
            err.to_string()
                .contains("--party-b must contain only letters")
        );
    }

    #[tokio::test]
    async fn auction_template_does_not_validate_unused_party_b() {
        let mut opts = default_test_opts();
        opts.template = Some("auction".to_string());
        opts.party_b = "Unused Party".to_string();

        run(&opts).await.unwrap();
    }

    #[tokio::test]
    async fn milestone_template_accepts_milestones() {
        let mut opts = default_test_opts();
        opts.template = Some("milestone".to_string());
        opts.milestones = Some("Design,Build".to_string());
        opts.output = Some(std::env::temp_dir().join(format!(
            "modality-synthesize-milestone-output-{}.modality",
            std::process::id()
        )));

        run(&opts).await.unwrap();

        let output_path = opts.output.as_ref().unwrap();
        let output = std::fs::read_to_string(output_path).unwrap();
        std::fs::remove_file(output_path).unwrap();

        assert!(output.contains("+COMPLETE_DESIGN"));
        assert!(output.contains("+PAY_DESIGN"));
        assert!(output.contains("+COMPLETE_BUILD"));
        assert!(output.contains("+PAY_BUILD"));
    }

    #[tokio::test]
    async fn milestone_template_rejects_empty_milestones() {
        let mut opts = default_test_opts();
        opts.template = Some("milestone".to_string());
        opts.milestones = Some("Design,,Build".to_string());

        let err = run(&opts).await.unwrap_err();

        assert!(
            err.to_string()
                .contains("--milestones requires non-empty comma-separated names")
        );
    }

    #[tokio::test]
    async fn milestone_template_rejects_invalid_milestone_names() {
        let mut opts = default_test_opts();
        opts.template = Some("milestone".to_string());
        opts.milestones = Some("Design Review,Build-Phase".to_string());

        let err = run(&opts).await.unwrap_err();

        assert!(err.to_string().contains(
            "--milestones names may contain only letters, numbers, underscores, and spaces"
        ));
    }

    #[tokio::test]
    async fn milestone_template_rejects_digit_started_milestone_names() {
        let mut opts = default_test_opts();
        opts.template = Some("milestone".to_string());
        opts.milestones = Some("1Design,Build".to_string());

        let err = run(&opts).await.unwrap_err();

        assert!(
            err.to_string()
                .contains("must start with a letter or underscore")
        );
    }

    #[tokio::test]
    async fn milestone_template_rejects_duplicate_milestones() {
        let mut opts = default_test_opts();
        opts.template = Some("milestone".to_string());
        opts.milestones = Some("Design,Design".to_string());

        let err = run(&opts).await.unwrap_err();

        assert!(err.to_string().contains("--milestones names must be unique"));
    }

    #[tokio::test]
    async fn milestone_template_rejects_normalized_duplicate_milestones() {
        let mut opts = default_test_opts();
        opts.template = Some("milestone".to_string());
        opts.milestones = Some("Design Review,Design_Review".to_string());

        let err = run(&opts).await.unwrap_err();

        assert!(err.to_string().contains(
            "--milestones names must be unique after spaces are normalized to underscores"
        ));
    }

    #[tokio::test]
    async fn milestone_template_trims_milestones() {
        let mut opts = default_test_opts();
        opts.template = Some("milestone".to_string());
        opts.milestones = Some(" Design , Build ".to_string());
        opts.output = Some(std::env::temp_dir().join(format!(
            "modality-synthesize-trimmed-milestone-output-{}.modality",
            std::process::id()
        )));

        run(&opts).await.unwrap();

        let output_path = opts.output.as_ref().unwrap();
        let output = std::fs::read_to_string(output_path).unwrap();
        std::fs::remove_file(output_path).unwrap();

        assert!(output.contains("+COMPLETE_DESIGN"));
        assert!(output.contains("+COMPLETE_BUILD"));
        assert!(!output.contains("+COMPLETE_ DESIGN "));
    }

    #[tokio::test]
    async fn template_mode_rejects_llm_response_file_before_reading_path() {
        let missing_response_path = std::env::temp_dir().join(format!(
            "modality-synthesize-template-llm-response-{}.md",
            std::process::id()
        ));

        let mut opts = default_test_opts();
        opts.template = Some("escrow".to_string());
        opts.llm_response_file = Some(missing_response_path.clone());

        let err = run(&opts).await.unwrap_err();

        let message = err.to_string();
        assert!(message.contains(
            "--template cannot be combined with other synthesis modes: --llm-response-file"
        ));
        assert!(!message.contains(&missing_response_path.display().to_string()));
    }

    #[tokio::test]
    async fn formulas_mode_rejects_rule_mode() {
        let mut opts = default_test_opts();
        opts.formulas = Some("always([<+APPROVE>] true)".to_string());
        opts.rule = Some(PathBuf::from("rules.modality"));

        let err = run(&opts).await.unwrap_err();

        assert!(
            err.to_string()
                .contains("--formulas cannot be combined with other synthesis modes: --rule")
        );
    }

    #[tokio::test]
    async fn formulas_mode_rejects_llm_response_file_before_reading_path() {
        let missing_response_path = std::env::temp_dir().join(format!(
            "modality-synthesize-formulas-llm-response-{}.md",
            std::process::id()
        ));

        let mut opts = default_test_opts();
        opts.formulas = Some("always([<+APPROVE>] true)".to_string());
        opts.llm_response_file = Some(missing_response_path.clone());

        let err = run(&opts).await.unwrap_err();

        let message = err.to_string();
        assert!(message.contains(
            "--formulas cannot be combined with other synthesis modes: --llm-response-file"
        ));
        assert!(!message.contains(&missing_response_path.display().to_string()));
    }

    #[tokio::test]
    async fn formulas_mode_reports_parser_details_when_no_formulas_parse() {
        let mut opts = default_test_opts();
        opts.formulas = Some("not a formula".to_string());

        let err = run(&opts).await.unwrap_err();

        let message = err.to_string();
        assert!(message.contains("No valid formulas found; parser details:"));
        assert!(message.contains("F1 `not a formula`"));
    }

    #[tokio::test]
    async fn llm_response_mode_rejects_rule_mode() {
        let mut opts = default_test_opts();
        opts.llm_response = Some("formula generated { always([<+APPROVE>] true) }".to_string());
        opts.rule = Some(PathBuf::from("rules.modality"));

        let err = run(&opts).await.unwrap_err();

        assert!(
            err.to_string()
                .contains("--rule cannot be combined with other synthesis modes: --llm-response")
        );
    }

    #[tokio::test]
    async fn llm_response_file_mode_rejects_rule_before_reading_path() {
        let missing_response_path = std::env::temp_dir().join(format!(
            "modality-synthesize-llm-rule-response-{}.md",
            std::process::id()
        ));

        let mut opts = default_test_opts();
        opts.llm_response_file = Some(missing_response_path.clone());
        opts.rule = Some(PathBuf::from("rules.modality"));

        let err = run(&opts).await.unwrap_err();

        let message = err.to_string();
        assert!(message.contains(
            "--rule cannot be combined with other synthesis modes: --llm-response-file"
        ));
        assert!(!message.contains(&missing_response_path.display().to_string()));
    }

    #[tokio::test]
    async fn llm_response_mode_rejects_response_file_before_reading_path() {
        let missing_response_path = std::env::temp_dir().join(format!(
            "modality-synthesize-conflicting-llm-response-{}.md",
            std::process::id()
        ));

        let mut opts = default_test_opts();
        opts.llm_response = Some("formula generated { always([<+APPROVE>] true) }".to_string());
        opts.llm_response_file = Some(missing_response_path.clone());

        let err = run(&opts).await.unwrap_err();

        let message = err.to_string();
        assert!(message.contains(
            "--llm-response cannot be combined with other synthesis modes: --llm-response-file"
        ));
        assert!(!message.contains(&missing_response_path.display().to_string()));
    }

    #[tokio::test]
    async fn llm_response_file_mode_names_active_flag_in_conflicts() {
        let missing_response_path = std::env::temp_dir().join(format!(
            "modality-synthesize-file-mode-conflict-{}.md",
            std::process::id()
        ));

        let mut opts = default_test_opts();
        opts.llm_response_file = Some(missing_response_path.clone());
        opts.milestones = Some("Design,Build".to_string());

        let err = run(&opts).await.unwrap_err();

        let message = err.to_string();
        assert!(message.contains(
            "--llm-response-file cannot be combined with other synthesis modes: --milestones"
        ));
        assert!(!message.contains(&missing_response_path.display().to_string()));
    }

    #[test]
    fn verify_synthesized_model_accepts_listed_formula_examples() {
        for group in FORMULA_EXAMPLE_GROUPS {
            for formula in group.formulas {
                let parsed = parse_formula_strings(&[formula.to_string()]);
                assert_eq!(
                    parsed.len(),
                    1,
                    "{} example failed to parse: {}",
                    group.title,
                    formula
                );

                let model =
                    modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &parsed);
                verify_synthesized_model(&model, &parsed).unwrap_or_else(|err| {
                    panic!(
                        "{} example failed verification: {}\n{}",
                        group.title, formula, err
                    )
                });
            }
        }
    }

    #[test]
    fn llm_multiline_formula_declarations_round_trip_to_verification() {
        let response = r#"
```modality
F1: formula generated_1 {
always([<+APPROVE>] true)
}
F2: formula generated_2 {
[+APPROVE] true -> <+signed_by(/users/reviewer.id)> true
}
```
"#;

        let formula_strings = modality_lang::llm_synthesis::parse_llm_response(response);
        assert_eq!(formula_strings.len(), 2);

        let formulas = parse_formula_strings(&formula_strings);
        assert_eq!(formulas.len(), 2);
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
    fn format_synthesized_model_with_formulas_preserves_json_declarations() {
        let model = modality_lang::Model::new("Contract".to_string());
        let declarations = vec![
            "formula proposed_rule {\nalways([<+APPROVE>] true)\n}".to_string(),
        ];

        let json = format_synthesized_model_with_formulas(&model, "json", &declarations).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["model"]["name"], "Contract");
        assert_eq!(
            parsed["formula_declarations"][0],
            "formula proposed_rule {\nalways([<+APPROVE>] true)\n}"
        );
    }

    #[test]
    fn synthesis_list_includes_existing_model_evolution_examples() {
        let output = synthesis_list_text();

        assert!(output.contains("--existing-model contract.modality --proposed-rule"));
        assert!(output.contains("--existing-model contract.modality --proposed-formula"));
    }

    #[test]
    fn synthesis_list_includes_review_publication_ordering_examples() {
        let output = synthesis_list_text();

        assert!(
            output.contains("always([+SUBMIT] true -> eventually(<+REVIEW> true))")
        );
        assert!(
            output.contains("always([+APPROVE] true -> eventually(<+PUBLISH> true))")
        );
        assert!(output.contains("always([+MERGE] true -> eventually(<+DEPLOY> true))"));
    }

    #[test]
    fn synthesis_list_includes_issue_remediation_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains("always([+OPEN_ISSUE] true -> eventually(<+TRIAGE> true))"));
        assert!(output.contains("always([+TRIAGE] true -> eventually(<+ASSIGN> true))"));
        assert!(output.contains("always([+FIX] true -> eventually(<+VERIFY> true))"));
    }

    #[test]
    fn synthesis_list_includes_incident_response_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains("always([+ALERT] true -> eventually(<+ACKNOWLEDGE> true))"));
        assert!(output.contains("always([+ACKNOWLEDGE] true -> eventually(<+MITIGATE> true))"));
        assert!(output.contains("always([+MITIGATE] true -> eventually(<+RESOLVE> true))"));
    }

    #[test]
    fn synthesis_list_includes_procurement_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains("always([+CREATE_ORDER] true -> eventually(<+APPROVE_ORDER> true))"));
        assert!(output.contains("always([+APPROVE_ORDER] true -> eventually(<+FULFILL_ORDER> true))"));
        assert!(output.contains("always([+FULFILL_ORDER] true -> eventually(<+PAY_INVOICE> true))"));
    }

    #[test]
    fn synthesis_list_includes_data_pipeline_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains("always([+INGEST_DATA] true -> eventually(<+VALIDATE_DATA> true))"));
        assert!(output.contains("always([+VALIDATE_DATA] true -> eventually(<+TRANSFORM_DATA> true))"));
        assert!(output.contains("always([+TRANSFORM_DATA] true -> eventually(<+PUBLISH_DATASET> true))"));
    }

    #[test]
    fn synthesis_list_includes_member_onboarding_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains("always([+INVITE_MEMBER] true -> eventually(<+ACCEPT_INVITE> true))"));
        assert!(output.contains("always([+ACCEPT_INVITE] true -> eventually(<+PROVISION_ACCESS> true))"));
        assert!(output.contains("always([+PROVISION_ACCESS] true -> eventually(<+COMPLETE_ONBOARDING> true))"));
    }

    #[test]
    fn synthesis_list_includes_release_rollout_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains("always([+PLAN_RELEASE] true -> eventually(<+APPROVE_QA> true))"));
        assert!(output.contains("always([+APPROVE_QA] true -> eventually(<+ROLLOUT_RELEASE> true))"));
        assert!(output.contains("always([+ROLLOUT_RELEASE] true -> eventually(<+MONITOR_RELEASE> true))"));
    }

    #[test]
    fn synthesis_list_includes_support_ticket_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains("always([+OPEN_TICKET] true -> eventually(<+ASSIGN_AGENT> true))"));
        assert!(output.contains("always([+ASSIGN_AGENT] true -> eventually(<+RESPOND_TICKET> true))"));
        assert!(output.contains("always([+RESPOND_TICKET] true -> eventually(<+RESOLVE_TICKET> true))"));
    }

    #[test]
    fn synthesis_list_includes_audit_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains("always([+START_AUDIT] true -> eventually(<+COLLECT_EVIDENCE> true))"));
        assert!(output.contains("always([+COLLECT_EVIDENCE] true -> eventually(<+REVIEW_EVIDENCE> true))"));
        assert!(output.contains("always([+REVIEW_EVIDENCE] true -> eventually(<+CLOSE_AUDIT> true))"));
    }

    #[test]
    fn synthesis_list_includes_expense_reimbursement_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains("always([+SUBMIT_EXPENSE] true -> eventually(<+APPROVE_EXPENSE> true))"));
        assert!(output.contains("always([+APPROVE_EXPENSE] true -> eventually(<+REIMBURSE_EXPENSE> true))"));
        assert!(output.contains("always([+REIMBURSE_EXPENSE] true -> eventually(<+CLOSE_EXPENSE> true))"));
    }

    #[test]
    fn synthesis_list_includes_training_certification_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains("always([+ENROLL_TRAINING] true -> eventually(<+COMPLETE_TRAINING> true))"));
        assert!(output.contains("always([+COMPLETE_TRAINING] true -> eventually(<+PASS_ASSESSMENT> true))"));
        assert!(output.contains("always([+PASS_ASSESSMENT] true -> eventually(<+ISSUE_CERTIFICATE> true))"));
    }

    #[test]
    fn synthesis_list_includes_asset_maintenance_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains("always([+SCHEDULE_MAINTENANCE] true -> eventually(<+PERFORM_MAINTENANCE> true))"));
        assert!(output.contains("always([+PERFORM_MAINTENANCE] true -> eventually(<+VERIFY_MAINTENANCE> true))"));
        assert!(output.contains("always([+VERIFY_MAINTENANCE] true -> eventually(<+CLOSE_MAINTENANCE> true))"));
    }

    #[test]
    fn synthesis_list_includes_backup_retention_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains("always([+SCHEDULE_BACKUP] true -> eventually(<+RUN_BACKUP> true))"));
        assert!(output.contains("always([+RUN_BACKUP] true -> eventually(<+VERIFY_BACKUP> true))"));
        assert!(output.contains("always([+VERIFY_BACKUP] true -> eventually(<+ARCHIVE_BACKUP> true))"));
    }

    #[test]
    fn synthesis_list_includes_offboarding_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output
            .contains("always([+REQUEST_OFFBOARDING] true -> eventually(<+REVOKE_ACCESS> true))"));
        assert!(output
            .contains("always([+REVOKE_ACCESS] true -> eventually(<+TRANSFER_OWNERSHIP> true))"));
        assert!(output.contains(
            "always([+TRANSFER_OWNERSHIP] true -> eventually(<+CONFIRM_DEPROVISIONING> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_contract_renewal_ordering_examples() {
        let output = synthesis_list_text();

        assert!(
            output.contains("always([+NOTICE_RENEWAL] true -> eventually(<+REVIEW_TERMS> true))")
        );
        assert!(
            output.contains("always([+REVIEW_TERMS] true -> eventually(<+APPROVE_RENEWAL> true))")
        );
        assert!(output.contains(
            "always([+APPROVE_RENEWAL] true -> eventually(<+EXECUTE_RENEWAL> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_credential_issuance_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "always([+REQUEST_CREDENTIAL] true -> eventually(<+VERIFY_IDENTITY> true))"
        ));
        assert!(output.contains(
            "always([+VERIFY_IDENTITY] true -> eventually(<+ISSUE_CREDENTIAL> true))"
        ));
        assert!(output.contains(
            "always([+ISSUE_CREDENTIAL] true -> eventually(<+ACCEPT_CREDENTIAL> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_access_review_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "always([+START_ACCESS_REVIEW] true -> eventually(<+COLLECT_ACCESS_EVIDENCE> true))"
        ));
        assert!(output.contains(
            "always([+COLLECT_ACCESS_EVIDENCE] true -> eventually(<+APPROVE_ACCESS_REVIEW> true))"
        ));
        assert!(output.contains(
            "always([+APPROVE_ACCESS_REVIEW] true -> eventually(<+REMEDIATE_ACCESS> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_claim_adjudication_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output
            .contains("always([+SUBMIT_CLAIM] true -> eventually(<+REVIEW_CLAIM> true))"));
        assert!(output
            .contains("always([+REVIEW_CLAIM] true -> eventually(<+APPROVE_CLAIM> true))"));
        assert!(
            output.contains("always([+APPROVE_CLAIM] true -> eventually(<+PAY_CLAIM> true))")
        );
    }

    #[test]
    fn synthesis_list_includes_privacy_request_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "always([+SUBMIT_PRIVACY_REQUEST] true -> eventually(<+VERIFY_SUBJECT> true))"
        ));
        assert!(output.contains(
            "always([+VERIFY_SUBJECT] true -> eventually(<+FULFILL_PRIVACY_REQUEST> true))"
        ));
        assert!(output.contains(
            "always([+FULFILL_PRIVACY_REQUEST] true -> eventually(<+CLOSE_PRIVACY_REQUEST> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_data_deletion_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "always([+REQUEST_DELETION] true -> eventually(<+CHECK_RETENTION_POLICY> true))"
        ));
        assert!(output.contains(
            "always([+CHECK_RETENTION_POLICY] true -> eventually(<+DELETE_RECORDS> true))"
        ));
        assert!(output.contains(
            "always([+DELETE_RECORDS] true -> eventually(<+CONFIRM_DELETION> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_account_recovery_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "always([+REQUEST_ACCOUNT_RECOVERY] true -> eventually(<+VERIFY_RECOVERY_FACTOR> true))"
        ));
        assert!(output.contains(
            "always([+VERIFY_RECOVERY_FACTOR] true -> eventually(<+ROTATE_CREDENTIAL> true))"
        ));
        assert!(output.contains(
            "always([+ROTATE_CREDENTIAL] true -> eventually(<+CONFIRM_ACCOUNT_RECOVERY> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_consent_change_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "always([+REQUEST_CONSENT_CHANGE] true -> eventually(<+REVIEW_CONSENT_SCOPE> true))"
        ));
        assert!(output.contains(
            "always([+REVIEW_CONSENT_SCOPE] true -> eventually(<+APPLY_CONSENT_CHANGE> true))"
        ));
        assert!(output.contains(
            "always([+APPLY_CONSENT_CHANGE] true -> eventually(<+CONFIRM_CONSENT_CHANGE> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_security_exception_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "always([+OPEN_SECURITY_EXCEPTION] true -> eventually(<+ASSESS_EXCEPTION_RISK> true))"
        ));
        assert!(output.contains(
            "always([+ASSESS_EXCEPTION_RISK] true -> eventually(<+APPROVE_EXCEPTION_MITIGATION> true))"
        ));
        assert!(output.contains(
            "always([+APPROVE_EXCEPTION_MITIGATION] true -> eventually(<+CLOSE_SECURITY_EXCEPTION> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_vulnerability_remediation_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "always([+REPORT_VULNERABILITY] true -> eventually(<+TRIAGE_VULNERABILITY> true))"
        ));
        assert!(output.contains(
            "always([+TRIAGE_VULNERABILITY] true -> eventually(<+APPLY_PATCH> true))"
        ));
        assert!(output.contains(
            "always([+APPLY_PATCH] true -> eventually(<+VERIFY_PATCH> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_breach_notification_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output
            .contains("always([+DETECT_BREACH] true -> eventually(<+ASSESS_BREACH_SCOPE> true))"));
        assert!(output.contains(
            "always([+ASSESS_BREACH_SCOPE] true -> eventually(<+NOTIFY_AFFECTED_PARTIES> true))"
        ));
        assert!(output.contains(
            "always([+NOTIFY_AFFECTED_PARTIES] true -> eventually(<+COMPLETE_BREACH_REVIEW> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_vendor_risk_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "always([+START_VENDOR_REVIEW] true -> eventually(<+COLLECT_VENDOR_QUESTIONNAIRE> true))"
        ));
        assert!(output.contains(
            "always([+COLLECT_VENDOR_QUESTIONNAIRE] true -> eventually(<+ASSESS_VENDOR_RISK> true))"
        ));
        assert!(output.contains(
            "always([+ASSESS_VENDOR_RISK] true -> eventually(<+APPROVE_VENDOR> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_data_access_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "always([+REQUEST_DATA_ACCESS] true -> eventually(<+VERIFY_ACCESS_PURPOSE> true))"
        ));
        assert!(output.contains(
            "always([+VERIFY_ACCESS_PURPOSE] true -> eventually(<+APPROVE_DATA_ACCESS> true))"
        ));
        assert!(output.contains(
            "always([+APPROVE_DATA_ACCESS] true -> eventually(<+LOG_ACCESS_GRANT> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_data_export_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "always([+REQUEST_DATA_EXPORT] true -> eventually(<+CLASSIFY_EXPORT_DATA> true))"
        ));
        assert!(output.contains(
            "always([+CLASSIFY_EXPORT_DATA] true -> eventually(<+APPROVE_DATA_EXPORT> true))"
        ));
        assert!(output.contains(
            "always([+APPROVE_DATA_EXPORT] true -> eventually(<+TRANSMIT_EXPORT_PACKAGE> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_data_sharing_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "always([+REQUEST_DATA_SHARE] true -> eventually(<+VERIFY_RECIPIENT_AUTHORITY> true))"
        ));
        assert!(output.contains(
            "always([+VERIFY_RECIPIENT_AUTHORITY] true -> eventually(<+APPROVE_DATA_SHARE> true))"
        ));
        assert!(output.contains(
            "always([+APPROVE_DATA_SHARE] true -> eventually(<+RECORD_DATA_SHARE> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_data_use_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output
            .contains("always([+REQUEST_DATA_USE] true -> eventually(<+REVIEW_USE_LIMITS> true))"));
        assert!(output
            .contains("always([+REVIEW_USE_LIMITS] true -> eventually(<+APPROVE_DATA_USE> true))"));
        assert!(output
            .contains("always([+APPROVE_DATA_USE] true -> eventually(<+LOG_DATA_USE> true))"));
    }

    #[test]
    fn synthesis_list_includes_retention_review_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "always([+START_RETENTION_REVIEW] true -> eventually(<+CLASSIFY_RETENTION_RECORDS> true))"
        ));
        assert!(output.contains(
            "always([+CLASSIFY_RETENTION_RECORDS] true -> eventually(<+APPROVE_RETENTION_PLAN> true))"
        ));
        assert!(output.contains(
            "always([+APPROVE_RETENTION_PLAN] true -> eventually(<+ENFORCE_RETENTION_PLAN> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_data_minimization_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output
            .contains("always([+COLLECT_DATA] true -> eventually(<+MINIMIZE_DATASET> true))"));
        assert!(output.contains(
            "always([+MINIMIZE_DATASET] true -> eventually(<+APPROVE_MINIMIZED_DATA> true))"
        ));
        assert!(output.contains(
            "always([+APPROVE_MINIMIZED_DATA] true -> eventually(<+RECORD_MINIMIZATION> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_data_anonymization_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "always([+PREPARE_ANALYTICS_DATA] true -> eventually(<+ANONYMIZE_DATASET> true))"
        ));
        assert!(output.contains(
            "always([+ANONYMIZE_DATASET] true -> eventually(<+VERIFY_ANONYMIZATION> true))"
        ));
        assert!(output.contains(
            "always([+VERIFY_ANONYMIZATION] true -> eventually(<+RELEASE_ANONYMIZED_DATA> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_purpose_change_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "always([+REQUEST_PURPOSE_CHANGE] true -> eventually(<+REVIEW_PURPOSE_COMPATIBILITY> true))"
        ));
        assert!(output.contains(
            "always([+REVIEW_PURPOSE_COMPATIBILITY] true -> eventually(<+APPROVE_PURPOSE_CHANGE> true))"
        ));
        assert!(output.contains(
            "always([+APPROVE_PURPOSE_CHANGE] true -> eventually(<+RECORD_PURPOSE_CHANGE> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_lawful_basis_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "always([+REQUEST_LAWFUL_BASIS_REVIEW] true -> eventually(<+ASSESS_LAWFUL_BASIS> true))"
        ));
        assert!(output.contains(
            "always([+ASSESS_LAWFUL_BASIS] true -> eventually(<+APPROVE_PROCESSING_BASIS> true))"
        ));
        assert!(output.contains(
            "always([+APPROVE_PROCESSING_BASIS] true -> eventually(<+RECORD_PROCESSING_BASIS> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_data_provenance_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "always([+REGISTER_DATASET] true -> eventually(<+CAPTURE_PROVENANCE> true))"
        ));
        assert!(output.contains(
            "always([+CAPTURE_PROVENANCE] true -> eventually(<+VERIFY_PROVENANCE> true))"
        ));
        assert!(output.contains(
            "always([+VERIFY_PROVENANCE] true -> eventually(<+APPROVE_PROVENANCE_RECORD> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_data_quality_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "always([+PROFILE_DATASET] true -> eventually(<+VALIDATE_DATA_QUALITY> true))"
        ));
        assert!(output.contains(
            "always([+VALIDATE_DATA_QUALITY] true -> eventually(<+APPROVE_QUALITY_REPORT> true))"
        ));
        assert!(output.contains(
            "always([+APPROVE_QUALITY_REPORT] true -> eventually(<+PUBLISH_QUALITY_REPORT> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_data_classification_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output
            .contains("always([+SUBMIT_DATASET] true -> eventually(<+CLASSIFY_DATASET> true))"));
        assert!(output.contains(
            "always([+CLASSIFY_DATASET] true -> eventually(<+APPROVE_DATA_CLASSIFICATION> true))"
        ));
        assert!(output.contains(
            "always([+APPROVE_DATA_CLASSIFICATION] true -> eventually(<+RECORD_DATA_CLASSIFICATION> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_dpia_ordering_examples() {
        let output = synthesis_list_text();

        assert!(
            output.contains("always([+START_DPIA] true -> eventually(<+ASSESS_PRIVACY_RISK> true))")
        );
        assert!(output.contains(
            "always([+ASSESS_PRIVACY_RISK] true -> eventually(<+APPROVE_DPIA> true))"
        ));
        assert!(output
            .contains("always([+APPROVE_DPIA] true -> eventually(<+RECORD_DPIA> true))"));
    }

    #[test]
    fn synthesis_list_includes_cross_border_transfer_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "always([+REQUEST_CROSS_BORDER_TRANSFER] true -> eventually(<+ASSESS_TRANSFER_MECHANISM> true))"
        ));
        assert!(output.contains(
            "always([+ASSESS_TRANSFER_MECHANISM] true -> eventually(<+APPROVE_CROSS_BORDER_TRANSFER> true))"
        ));
        assert!(output.contains(
            "always([+APPROVE_CROSS_BORDER_TRANSFER] true -> eventually(<+RECORD_TRANSFER_ASSESSMENT> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_subprocessor_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "always([+REGISTER_SUBPROCESSOR] true -> eventually(<+ASSESS_SUBPROCESSOR_RISK> true))"
        ));
        assert!(output.contains(
            "always([+ASSESS_SUBPROCESSOR_RISK] true -> eventually(<+APPROVE_SUBPROCESSOR> true))"
        ));
        assert!(output.contains(
            "always([+APPROVE_SUBPROCESSOR] true -> eventually(<+RECORD_SUBPROCESSOR> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_data_localization_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "always([+REQUEST_DATA_LOCALIZATION] true -> eventually(<+ASSESS_RESIDENCY_REQUIREMENT> true))"
        ));
        assert!(output.contains(
            "always([+ASSESS_RESIDENCY_REQUIREMENT] true -> eventually(<+APPROVE_LOCALIZATION_PLAN> true))"
        ));
        assert!(output.contains(
            "always([+APPROVE_LOCALIZATION_PLAN] true -> eventually(<+RECORD_LOCALIZATION_CONTROL> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_model_card_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "always([+SUBMIT_MODEL_CARD] true -> eventually(<+EVALUATE_MODEL_RISK> true))"
        ));
        assert!(output.contains(
            "always([+EVALUATE_MODEL_RISK] true -> eventually(<+APPROVE_MODEL_DEPLOYMENT> true))"
        ));
        assert!(output.contains(
            "always([+APPROVE_MODEL_DEPLOYMENT] true -> eventually(<+PUBLISH_MODEL_CARD> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_model_evaluation_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "always([+REGISTER_EVALUATION_DATASET] true -> eventually(<+RUN_BIAS_EVALUATION> true))"
        ));
        assert!(output.contains(
            "always([+RUN_BIAS_EVALUATION] true -> eventually(<+APPROVE_EVALUATION_REPORT> true))"
        ));
        assert!(output.contains(
            "always([+APPROVE_EVALUATION_REPORT] true -> eventually(<+ARCHIVE_EVALUATION_EVIDENCE> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_model_calibration_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "always([+SCHEDULE_MODEL_CALIBRATION] true -> eventually(<+RUN_CALIBRATION_CHECK> true))"
        ));
        assert!(output.contains(
            "always([+RUN_CALIBRATION_CHECK] true -> eventually(<+APPROVE_CALIBRATION_REPORT> true))"
        ));
        assert!(output.contains(
            "always([+APPROVE_CALIBRATION_REPORT] true -> eventually(<+RECORD_CALIBRATION_RESULT> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_model_monitoring_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "always([+START_MODEL_MONITORING] true -> eventually(<+DETECT_MODEL_DRIFT> true))"
        ));
        assert!(output.contains(
            "always([+DETECT_MODEL_DRIFT] true -> eventually(<+APPROVE_MODEL_UPDATE> true))"
        ));
        assert!(output.contains(
            "always([+APPROVE_MODEL_UPDATE] true -> eventually(<+RECORD_MONITORING_REVIEW> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_model_incident_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "always([+DETECT_MODEL_INCIDENT] true -> eventually(<+ASSESS_MODEL_IMPACT> true))"
        ));
        assert!(output.contains(
            "always([+ASSESS_MODEL_IMPACT] true -> eventually(<+APPROVE_MODEL_ROLLBACK> true))"
        ));
        assert!(output.contains(
            "always([+APPROVE_MODEL_ROLLBACK] true -> eventually(<+RECORD_MODEL_INCIDENT> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_model_retirement_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "always([+REQUEST_MODEL_RETIREMENT] true -> eventually(<+ASSESS_RETIREMENT_IMPACT> true))"
        ));
        assert!(output.contains(
            "always([+ASSESS_RETIREMENT_IMPACT] true -> eventually(<+APPROVE_MODEL_RETIREMENT> true))"
        ));
        assert!(output.contains(
            "always([+APPROVE_MODEL_RETIREMENT] true -> eventually(<+ARCHIVE_MODEL_ARTIFACTS> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_model_retraining_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "always([+COLLECT_RETRAINING_DATA] true -> eventually(<+APPROVE_RETRAINING_PLAN> true))"
        ));
        assert!(output.contains(
            "always([+APPROVE_RETRAINING_PLAN] true -> eventually(<+TRAIN_CANDIDATE_MODEL> true))"
        ));
        assert!(output.contains(
            "always([+TRAIN_CANDIDATE_MODEL] true -> eventually(<+VALIDATE_CANDIDATE_MODEL> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_model_audit_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "always([+LOG_MODEL_DECISION] true -> eventually(<+REVIEW_DECISION_TRACE> true))"
        ));
        assert!(output.contains(
            "always([+REVIEW_DECISION_TRACE] true -> eventually(<+APPROVE_MODEL_AUDIT> true))"
        ));
        assert!(output.contains(
            "always([+APPROVE_MODEL_AUDIT] true -> eventually(<+RECORD_AUDIT_EVIDENCE> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_model_safety_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "always([+START_MODEL_RED_TEAM] true -> eventually(<+REVIEW_RED_TEAM_FINDINGS> true))"
        ));
        assert!(output.contains(
            "always([+REVIEW_RED_TEAM_FINDINGS] true -> eventually(<+APPROVE_SAFETY_MITIGATION> true))"
        ));
        assert!(output.contains(
            "always([+APPROVE_SAFETY_MITIGATION] true -> eventually(<+RECORD_SAFETY_CASE> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_model_version_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "always([+REGISTER_MODEL_VERSION] true -> eventually(<+RUN_MODEL_VALIDATION> true))"
        ));
        assert!(output.contains(
            "always([+RUN_MODEL_VALIDATION] true -> eventually(<+APPROVE_MODEL_VERSION> true))"
        ));
        assert!(output.contains(
            "always([+APPROVE_MODEL_VERSION] true -> eventually(<+PROMOTE_MODEL_VERSION> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_model_lineage_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "always([+CAPTURE_MODEL_LINEAGE] true -> eventually(<+REVIEW_LINEAGE_REPORT> true))"
        ));
        assert!(output.contains(
            "always([+REVIEW_LINEAGE_REPORT] true -> eventually(<+APPROVE_LINEAGE_RECORD> true))"
        ));
        assert!(output.contains(
            "always([+APPROVE_LINEAGE_RECORD] true -> eventually(<+ARCHIVE_LINEAGE_RECORD> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_model_artifact_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "always([+SUBMIT_MODEL_ARTIFACT] true -> eventually(<+SCAN_MODEL_ARTIFACT> true))"
        ));
        assert!(output.contains(
            "always([+SCAN_MODEL_ARTIFACT] true -> eventually(<+APPROVE_MODEL_ARTIFACT> true))"
        ));
        assert!(output.contains(
            "always([+APPROVE_MODEL_ARTIFACT] true -> eventually(<+PUBLISH_MODEL_ARTIFACT> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_model_endpoint_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "always([+REGISTER_MODEL_ENDPOINT] true -> eventually(<+RUN_ENDPOINT_SMOKE_TEST> true))"
        ));
        assert!(output.contains(
            "always([+RUN_ENDPOINT_SMOKE_TEST] true -> eventually(<+APPROVE_ENDPOINT_ACTIVATION> true))"
        ));
        assert!(output.contains(
            "always([+APPROVE_ENDPOINT_ACTIVATION] true -> eventually(<+ACTIVATE_MODEL_ENDPOINT> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_model_canary_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "always([+PLAN_MODEL_CANARY] true -> eventually(<+MONITOR_CANARY_METRICS> true))"
        ));
        assert!(output.contains(
            "always([+MONITOR_CANARY_METRICS] true -> eventually(<+APPROVE_FULL_ROLLOUT> true))"
        ));
        assert!(output.contains(
            "always([+APPROVE_FULL_ROLLOUT] true -> eventually(<+EXPAND_MODEL_TRAFFIC> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_model_drift_response_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "always([+DETECT_MODEL_DRIFT] true -> eventually(<+ASSESS_DRIFT_IMPACT> true))"
        ));
        assert!(output.contains(
            "always([+ASSESS_DRIFT_IMPACT] true -> eventually(<+APPROVE_DRIFT_RESPONSE> true))"
        ));
        assert!(output.contains(
            "always([+APPROVE_DRIFT_RESPONSE] true -> eventually(<+RECORD_DRIFT_RESPONSE> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_model_shadow_promotion_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "always([+START_SHADOW_EVALUATION] true -> eventually(<+COMPARE_SHADOW_OUTPUT> true))"
        ));
        assert!(output.contains(
            "always([+COMPARE_SHADOW_OUTPUT] true -> eventually(<+APPROVE_SHADOW_PROMOTION> true))"
        ));
        assert!(output.contains(
            "always([+APPROVE_SHADOW_PROMOTION] true -> eventually(<+PROMOTE_SHADOW_MODEL> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_model_rollback_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "always([+DETECT_MODEL_REGRESSION] true -> eventually(<+ASSESS_ROLLBACK_RISK> true))"
        ));
        assert!(output.contains(
            "always([+ASSESS_ROLLBACK_RISK] true -> eventually(<+APPROVE_MODEL_ROLLBACK_PLAN> true))"
        ));
        assert!(output.contains(
            "always([+APPROVE_MODEL_ROLLBACK_PLAN] true -> eventually(<+EXECUTE_MODEL_ROLLBACK> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_model_exception_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "always([+REQUEST_MODEL_EXCEPTION] true -> eventually(<+ASSESS_MODEL_EXCEPTION> true))"
        ));
        assert!(output.contains(
            "always([+ASSESS_MODEL_EXCEPTION] true -> eventually(<+APPROVE_MODEL_EXCEPTION> true))"
        ));
        assert!(output.contains(
            "always([+APPROVE_MODEL_EXCEPTION] true -> eventually(<+RECORD_MODEL_EXCEPTION> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_model_deprecation_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "always([+REQUEST_MODEL_DEPRECATION] true -> eventually(<+ASSESS_DEPRECATION_IMPACT> true))"
        ));
        assert!(output.contains(
            "always([+ASSESS_DEPRECATION_IMPACT] true -> eventually(<+APPROVE_MODEL_DEPRECATION> true))"
        ));
        assert!(output.contains(
            "always([+APPROVE_MODEL_DEPRECATION] true -> eventually(<+RECORD_MODEL_DEPRECATION> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_model_attestation_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "always([+REQUEST_MODEL_ATTESTATION] true -> eventually(<+COLLECT_ATTESTATION_EVIDENCE> true))"
        ));
        assert!(output.contains(
            "always([+COLLECT_ATTESTATION_EVIDENCE] true -> eventually(<+APPROVE_MODEL_ATTESTATION> true))"
        ));
        assert!(output.contains(
            "always([+APPROVE_MODEL_ATTESTATION] true -> eventually(<+PUBLISH_MODEL_ATTESTATION> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_model_disclosure_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "always([+REQUEST_MODEL_DISCLOSURE] true -> eventually(<+REVIEW_DISCLOSURE_SCOPE> true))"
        ));
        assert!(output.contains(
            "always([+REVIEW_DISCLOSURE_SCOPE] true -> eventually(<+APPROVE_MODEL_DISCLOSURE> true))"
        ));
        assert!(output.contains(
            "always([+APPROVE_MODEL_DISCLOSURE] true -> eventually(<+PUBLISH_MODEL_DISCLOSURE> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_model_appeal_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "always([+REQUEST_MODEL_APPEAL] true -> eventually(<+REVIEW_MODEL_APPEAL> true))"
        ));
        assert!(output.contains(
            "always([+REVIEW_MODEL_APPEAL] true -> eventually(<+APPROVE_MODEL_APPEAL> true))"
        ));
        assert!(output.contains(
            "always([+APPROVE_MODEL_APPEAL] true -> eventually(<+RECORD_MODEL_APPEAL> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_model_override_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "always([+REQUEST_MODEL_OVERRIDE] true -> eventually(<+REVIEW_OVERRIDE_RISK> true))"
        ));
        assert!(output.contains(
            "always([+REVIEW_OVERRIDE_RISK] true -> eventually(<+APPROVE_MODEL_OVERRIDE> true))"
        ));
        assert!(output.contains(
            "always([+APPROVE_MODEL_OVERRIDE] true -> eventually(<+RECORD_OVERRIDE_AUDIT> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_agent_memory_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "always([+PROPOSE_AGENT_MEMORY] true -> eventually(<+REVIEW_MEMORY_SCOPE> true))"
        ));
        assert!(output.contains(
            "always([+REVIEW_MEMORY_SCOPE] true -> eventually(<+APPROVE_MEMORY_WRITE> true))"
        ));
        assert!(output.contains(
            "always([+APPROVE_MEMORY_WRITE] true -> eventually(<+COMMIT_AGENT_MEMORY> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_agent_handoff_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "always([+REQUEST_AGENT_HANDOFF] true -> eventually(<+PACKAGE_AGENT_CONTEXT> true))"
        ));
        assert!(output.contains(
            "always([+PACKAGE_AGENT_CONTEXT] true -> eventually(<+APPROVE_AGENT_HANDOFF> true))"
        ));
        assert!(output.contains(
            "always([+APPROVE_AGENT_HANDOFF] true -> eventually(<+ACCEPT_AGENT_HANDOFF> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_external_tool_call_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "always([+REQUEST_EXTERNAL_TOOL_CALL] true -> eventually(<+ASSESS_TOOL_CALL_RISK> true))"
        ));
        assert!(output.contains(
            "always([+ASSESS_TOOL_CALL_RISK] true -> eventually(<+APPROVE_EXTERNAL_TOOL_CALL> true))"
        ));
        assert!(output.contains(
            "always([+APPROVE_EXTERNAL_TOOL_CALL] true -> eventually(<+EXECUTE_EXTERNAL_TOOL_CALL> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_agent_credential_rotation_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "always([+REQUEST_AGENT_CREDENTIAL_ROTATION] true -> eventually(<+VERIFY_AGENT_IDENTITY> true))"
        ));
        assert!(output.contains(
            "always([+VERIFY_AGENT_IDENTITY] true -> eventually(<+APPROVE_AGENT_CREDENTIAL_ROTATION> true))"
        ));
        assert!(output.contains(
            "always([+APPROVE_AGENT_CREDENTIAL_ROTATION] true -> eventually(<+ROTATE_AGENT_CREDENTIAL> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_agent_incident_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "always([+REPORT_AGENT_INCIDENT] true -> eventually(<+CONTAIN_AGENT_SESSION> true))"
        ));
        assert!(output.contains(
            "always([+CONTAIN_AGENT_SESSION] true -> eventually(<+APPROVE_AGENT_REMEDIATION> true))"
        ));
        assert!(output.contains(
            "always([+APPROVE_AGENT_REMEDIATION] true -> eventually(<+RECORD_AGENT_INCIDENT> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_agent_permission_revoke_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "always([+REQUEST_AGENT_PERMISSION_REVOKE] true -> eventually(<+ASSESS_PERMISSION_DEPENDENCIES> true))"
        ));
        assert!(output.contains(
            "always([+ASSESS_PERMISSION_DEPENDENCIES] true -> eventually(<+APPROVE_AGENT_PERMISSION_REVOKE> true))"
        ));
        assert!(output.contains(
            "always([+APPROVE_AGENT_PERMISSION_REVOKE] true -> eventually(<+REVOKE_AGENT_PERMISSION> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_agent_data_egress_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "always([+REQUEST_AGENT_DATA_EGRESS] true -> eventually(<+CLASSIFY_AGENT_OUTPUT> true))"
        ));
        assert!(output.contains(
            "always([+CLASSIFY_AGENT_OUTPUT] true -> eventually(<+APPROVE_AGENT_DATA_EGRESS> true))"
        ));
        assert!(output.contains(
            "always([+APPROVE_AGENT_DATA_EGRESS] true -> eventually(<+RELEASE_AGENT_OUTPUT> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_agent_autonomy_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "always([+REQUEST_AGENT_AUTONOMY] true -> eventually(<+ASSESS_AUTONOMY_RISK> true))"
        ));
        assert!(output.contains(
            "always([+ASSESS_AUTONOMY_RISK] true -> eventually(<+APPROVE_AGENT_AUTONOMY> true))"
        ));
        assert!(output.contains(
            "always([+APPROVE_AGENT_AUTONOMY] true -> eventually(<+ENABLE_AGENT_AUTONOMY> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_agent_publication_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "always([+PROPOSE_AGENT_PUBLICATION] true -> eventually(<+REVIEW_AGENT_CLAIMS> true))"
        ));
        assert!(output.contains(
            "always([+REVIEW_AGENT_CLAIMS] true -> eventually(<+APPROVE_AGENT_PUBLICATION> true))"
        ));
        assert!(output.contains(
            "always([+APPROVE_AGENT_PUBLICATION] true -> eventually(<+PUBLISH_AGENT_OUTPUT> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_agent_secret_access_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "always([+REQUEST_AGENT_SECRET_ACCESS] true -> eventually(<+REVIEW_SECRET_SCOPE> true))"
        ));
        assert!(output.contains(
            "always([+REVIEW_SECRET_SCOPE] true -> eventually(<+APPROVE_AGENT_SECRET_ACCESS> true))"
        ));
        assert!(output.contains(
            "always([+APPROVE_AGENT_SECRET_ACCESS] true -> eventually(<+GRANT_AGENT_SECRET_ACCESS> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_agent_model_access_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "always([+REQUEST_AGENT_MODEL_ACCESS] true -> eventually(<+REVIEW_MODEL_ACCESS_SCOPE> true))"
        ));
        assert!(output.contains(
            "always([+REVIEW_MODEL_ACCESS_SCOPE] true -> eventually(<+APPROVE_AGENT_MODEL_ACCESS> true))"
        ));
        assert!(output.contains(
            "always([+APPROVE_AGENT_MODEL_ACCESS] true -> eventually(<+GRANT_AGENT_MODEL_ACCESS> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_agent_spend_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "always([+REQUEST_AGENT_SPEND] true -> eventually(<+ESTIMATE_AGENT_SPEND_RISK> true))"
        ));
        assert!(output.contains(
            "always([+ESTIMATE_AGENT_SPEND_RISK] true -> eventually(<+APPROVE_AGENT_SPEND> true))"
        ));
        assert!(output.contains(
            "always([+APPROVE_AGENT_SPEND] true -> eventually(<+EXECUTE_AGENT_SPEND> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_agent_prompt_injection_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "always([+DETECT_PROMPT_INJECTION] true -> eventually(<+QUARANTINE_AGENT_CONTEXT> true))"
        ));
        assert!(output.contains(
            "always([+QUARANTINE_AGENT_CONTEXT] true -> eventually(<+APPROVE_CONTEXT_RESTORATION> true))"
        ));
        assert!(output.contains(
            "always([+APPROVE_CONTEXT_RESTORATION] true -> eventually(<+RESTORE_AGENT_CONTEXT> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_agent_network_access_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "always([+REQUEST_AGENT_NETWORK_ACCESS] true -> eventually(<+ASSESS_NETWORK_SCOPE> true))"
        ));
        assert!(output.contains(
            "always([+ASSESS_NETWORK_SCOPE] true -> eventually(<+APPROVE_AGENT_NETWORK_ACCESS> true))"
        ));
        assert!(output.contains(
            "always([+APPROVE_AGENT_NETWORK_ACCESS] true -> eventually(<+ENABLE_AGENT_NETWORK_ACCESS> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_agent_state_export_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "always([+REQUEST_AGENT_STATE_EXPORT] true -> eventually(<+REDACT_AGENT_STATE> true))"
        ));
        assert!(output.contains(
            "always([+REDACT_AGENT_STATE] true -> eventually(<+APPROVE_AGENT_STATE_EXPORT> true))"
        ));
        assert!(output.contains(
            "always([+APPROVE_AGENT_STATE_EXPORT] true -> eventually(<+EXPORT_AGENT_STATE> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_agent_dependency_update_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "always([+PROPOSE_AGENT_DEPENDENCY_UPDATE] true -> eventually(<+SCAN_AGENT_DEPENDENCY> true))"
        ));
        assert!(output.contains(
            "always([+SCAN_AGENT_DEPENDENCY] true -> eventually(<+APPROVE_AGENT_DEPENDENCY_UPDATE> true))"
        ));
        assert!(output.contains(
            "always([+APPROVE_AGENT_DEPENDENCY_UPDATE] true -> eventually(<+APPLY_AGENT_DEPENDENCY_UPDATE> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_agent_identity_binding_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "always([+REQUEST_AGENT_IDENTITY_BINDING] true -> eventually(<+VERIFY_AGENT_ATTESTATION> true))"
        ));
        assert!(output.contains(
            "always([+VERIFY_AGENT_ATTESTATION] true -> eventually(<+APPROVE_AGENT_IDENTITY_BINDING> true))"
        ));
        assert!(output.contains(
            "always([+APPROVE_AGENT_IDENTITY_BINDING] true -> eventually(<+BIND_AGENT_IDENTITY> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_agent_runtime_migration_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "always([+REQUEST_AGENT_RUNTIME_MIGRATION] true -> eventually(<+SNAPSHOT_AGENT_RUNTIME> true))"
        ));
        assert!(output.contains(
            "always([+SNAPSHOT_AGENT_RUNTIME] true -> eventually(<+APPROVE_AGENT_RUNTIME_MIGRATION> true))"
        ));
        assert!(output.contains(
            "always([+APPROVE_AGENT_RUNTIME_MIGRATION] true -> eventually(<+MIGRATE_AGENT_RUNTIME> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_agent_rollback_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "always([+REQUEST_AGENT_ROLLBACK] true -> eventually(<+VERIFY_ROLLBACK_POINT> true))"
        ));
        assert!(output.contains(
            "always([+VERIFY_ROLLBACK_POINT] true -> eventually(<+APPROVE_AGENT_ROLLBACK> true))"
        ));
        assert!(output.contains(
            "always([+APPROVE_AGENT_ROLLBACK] true -> eventually(<+ROLLBACK_AGENT_STATE> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_agent_telemetry_access_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "always([+REQUEST_AGENT_TELEMETRY_ACCESS] true -> eventually(<+REDACT_AGENT_TELEMETRY> true))"
        ));
        assert!(output.contains(
            "always([+REDACT_AGENT_TELEMETRY] true -> eventually(<+APPROVE_AGENT_TELEMETRY_ACCESS> true))"
        ));
        assert!(output.contains(
            "always([+APPROVE_AGENT_TELEMETRY_ACCESS] true -> eventually(<+EXPORT_AGENT_TELEMETRY> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_agent_session_resume_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "always([+REQUEST_AGENT_SESSION_RESUME] true -> eventually(<+VALIDATE_SESSION_CHECKPOINT> true))"
        ));
        assert!(output.contains(
            "always([+VALIDATE_SESSION_CHECKPOINT] true -> eventually(<+APPROVE_AGENT_SESSION_RESUME> true))"
        ));
        assert!(output.contains(
            "always([+APPROVE_AGENT_SESSION_RESUME] true -> eventually(<+RESUME_AGENT_SESSION> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_agent_backup_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "always([+REQUEST_AGENT_BACKUP] true -> eventually(<+VERIFY_BACKUP_SCOPE> true))"
        ));
        assert!(output.contains(
            "always([+VERIFY_BACKUP_SCOPE] true -> eventually(<+APPROVE_AGENT_BACKUP> true))"
        ));
        assert!(output.contains(
            "always([+APPROVE_AGENT_BACKUP] true -> eventually(<+CREATE_AGENT_BACKUP> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_agent_log_retention_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "always([+REQUEST_AGENT_LOG_RETENTION] true -> eventually(<+CLASSIFY_AGENT_LOGS> true))"
        ));
        assert!(output.contains(
            "always([+CLASSIFY_AGENT_LOGS] true -> eventually(<+APPROVE_AGENT_LOG_RETENTION> true))"
        ));
        assert!(output.contains(
            "always([+APPROVE_AGENT_LOG_RETENTION] true -> eventually(<+ENFORCE_AGENT_LOG_RETENTION> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_agent_state_purge_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "always([+REQUEST_AGENT_STATE_PURGE] true -> eventually(<+REVIEW_PURGE_SCOPE> true))"
        ));
        assert!(output.contains(
            "always([+REVIEW_PURGE_SCOPE] true -> eventually(<+APPROVE_AGENT_STATE_PURGE> true))"
        ));
        assert!(output.contains(
            "always([+APPROVE_AGENT_STATE_PURGE] true -> eventually(<+PURGE_AGENT_STATE> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_agent_audit_disclosure_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "always([+REQUEST_AGENT_AUDIT_DISCLOSURE] true -> eventually(<+REDACT_AGENT_AUDIT_LOG> true))"
        ));
        assert!(output.contains(
            "always([+REDACT_AGENT_AUDIT_LOG] true -> eventually(<+APPROVE_AGENT_AUDIT_DISCLOSURE> true))"
        ));
        assert!(output.contains(
            "always([+APPROVE_AGENT_AUDIT_DISCLOSURE] true -> eventually(<+DISCLOSE_AGENT_AUDIT_LOG> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_agent_environment_teardown_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "always([+REQUEST_AGENT_ENVIRONMENT_TEARDOWN] true -> eventually(<+SNAPSHOT_AGENT_ENVIRONMENT> true))"
        ));
        assert!(output.contains(
            "always([+SNAPSHOT_AGENT_ENVIRONMENT] true -> eventually(<+APPROVE_AGENT_ENVIRONMENT_TEARDOWN> true))"
        ));
        assert!(output.contains(
            "always([+APPROVE_AGENT_ENVIRONMENT_TEARDOWN] true -> eventually(<+TEARDOWN_AGENT_ENVIRONMENT> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_agent_cache_invalidation_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "always([+REQUEST_AGENT_CACHE_INVALIDATION] true -> eventually(<+ASSESS_CACHE_DEPENDENCIES> true))"
        ));
        assert!(output.contains(
            "always([+ASSESS_CACHE_DEPENDENCIES] true -> eventually(<+APPROVE_AGENT_CACHE_INVALIDATION> true))"
        ));
        assert!(output.contains(
            "always([+APPROVE_AGENT_CACHE_INVALIDATION] true -> eventually(<+INVALIDATE_AGENT_CACHE> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_agent_context_compaction_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "always([+REQUEST_AGENT_CONTEXT_COMPACTION] true -> eventually(<+SUMMARIZE_AGENT_CONTEXT> true))"
        ));
        assert!(output.contains(
            "always([+SUMMARIZE_AGENT_CONTEXT] true -> eventually(<+APPROVE_AGENT_CONTEXT_COMPACTION> true))"
        ));
        assert!(output.contains(
            "always([+APPROVE_AGENT_CONTEXT_COMPACTION] true -> eventually(<+COMPACT_AGENT_CONTEXT> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_agent_workspace_handover_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "always([+REQUEST_AGENT_WORKSPACE_HANDOVER] true -> eventually(<+INVENTORY_WORKSPACE_STATE> true))"
        ));
        assert!(output.contains(
            "always([+INVENTORY_WORKSPACE_STATE] true -> eventually(<+APPROVE_AGENT_WORKSPACE_HANDOVER> true))"
        ));
        assert!(output.contains(
            "always([+APPROVE_AGENT_WORKSPACE_HANDOVER] true -> eventually(<+HANDOVER_AGENT_WORKSPACE> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_agent_knowledge_refresh_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "always([+REQUEST_AGENT_KNOWLEDGE_REFRESH] true -> eventually(<+REVIEW_KNOWLEDGE_SOURCES> true))"
        ));
        assert!(output.contains(
            "always([+REVIEW_KNOWLEDGE_SOURCES] true -> eventually(<+APPROVE_AGENT_KNOWLEDGE_REFRESH> true))"
        ));
        assert!(output.contains(
            "always([+APPROVE_AGENT_KNOWLEDGE_REFRESH] true -> eventually(<+REFRESH_AGENT_KNOWLEDGE> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_agent_delegation_renewal_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "always([+REQUEST_AGENT_DELEGATION_RENEWAL] true -> eventually(<+REVIEW_DELEGATION_SCOPE> true))"
        ));
        assert!(output.contains(
            "always([+REVIEW_DELEGATION_SCOPE] true -> eventually(<+APPROVE_AGENT_DELEGATION_RENEWAL> true))"
        ));
        assert!(output.contains(
            "always([+APPROVE_AGENT_DELEGATION_RENEWAL] true -> eventually(<+RENEW_AGENT_DELEGATION> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_agent_policy_drift_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "always([+DETECT_AGENT_POLICY_DRIFT] true -> eventually(<+ASSESS_POLICY_DRIFT> true))"
        ));
        assert!(output.contains(
            "always([+ASSESS_POLICY_DRIFT] true -> eventually(<+APPROVE_POLICY_DRIFT_REMEDIATION> true))"
        ));
        assert!(output.contains(
            "always([+APPROVE_POLICY_DRIFT_REMEDIATION] true -> eventually(<+REMEDIATE_AGENT_POLICY> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_agent_performance_review_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "always([+REQUEST_AGENT_PERFORMANCE_REVIEW] true -> eventually(<+COLLECT_AGENT_METRICS> true))"
        ));
        assert!(output.contains(
            "always([+COLLECT_AGENT_METRICS] true -> eventually(<+APPROVE_AGENT_PERFORMANCE_REVIEW> true))"
        ));
        assert!(output.contains(
            "always([+APPROVE_AGENT_PERFORMANCE_REVIEW] true -> eventually(<+RECORD_AGENT_PERFORMANCE_REVIEW> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_agent_budget_increase_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "always([+REQUEST_AGENT_BUDGET_INCREASE] true -> eventually(<+ASSESS_AGENT_BUDGET_IMPACT> true))"
        ));
        assert!(output.contains(
            "always([+ASSESS_AGENT_BUDGET_IMPACT] true -> eventually(<+APPROVE_AGENT_BUDGET_INCREASE> true))"
        ));
        assert!(output.contains(
            "always([+APPROVE_AGENT_BUDGET_INCREASE] true -> eventually(<+APPLY_AGENT_BUDGET> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_agent_rate_limit_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "always([+REQUEST_AGENT_RATE_LIMIT_CHANGE] true -> eventually(<+ASSESS_AGENT_RATE_LIMIT_RISK> true))"
        ));
        assert!(output.contains(
            "always([+ASSESS_AGENT_RATE_LIMIT_RISK] true -> eventually(<+APPROVE_AGENT_RATE_LIMIT_CHANGE> true))"
        ));
        assert!(output.contains(
            "always([+APPROVE_AGENT_RATE_LIMIT_CHANGE] true -> eventually(<+APPLY_AGENT_RATE_LIMIT> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_agent_prompt_template_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "always([+REQUEST_AGENT_PROMPT_TEMPLATE_CHANGE] true -> eventually(<+REVIEW_PROMPT_TEMPLATE_DIFF> true))"
        ));
        assert!(output.contains(
            "always([+REVIEW_PROMPT_TEMPLATE_DIFF] true -> eventually(<+APPROVE_AGENT_PROMPT_TEMPLATE_CHANGE> true))"
        ));
        assert!(output.contains(
            "always([+APPROVE_AGENT_PROMPT_TEMPLATE_CHANGE] true -> eventually(<+APPLY_AGENT_PROMPT_TEMPLATE> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_agent_guardrail_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "always([+REQUEST_AGENT_GUARDRAIL_CHANGE] true -> eventually(<+TEST_AGENT_GUARDRAIL> true))"
        ));
        assert!(output.contains(
            "always([+TEST_AGENT_GUARDRAIL] true -> eventually(<+APPROVE_AGENT_GUARDRAIL_CHANGE> true))"
        ));
        assert!(output.contains(
            "always([+APPROVE_AGENT_GUARDRAIL_CHANGE] true -> eventually(<+APPLY_AGENT_GUARDRAIL> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_agent_evaluator_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "always([+REQUEST_AGENT_EVALUATOR_CHANGE] true -> eventually(<+VALIDATE_AGENT_EVALUATOR> true))"
        ));
        assert!(output.contains(
            "always([+VALIDATE_AGENT_EVALUATOR] true -> eventually(<+APPROVE_AGENT_EVALUATOR_CHANGE> true))"
        ));
        assert!(output.contains(
            "always([+APPROVE_AGENT_EVALUATOR_CHANGE] true -> eventually(<+APPLY_AGENT_EVALUATOR> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_human_review_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "always([+REQUEST_HUMAN_REVIEW] true -> eventually(<+TRIAGE_REVIEW_REQUEST> true))"
        ));
        assert!(output.contains(
            "always([+TRIAGE_REVIEW_REQUEST] true -> eventually(<+APPROVE_HUMAN_REVIEW> true))"
        ));
        assert!(output.contains(
            "always([+APPROVE_HUMAN_REVIEW] true -> eventually(<+RECORD_REVIEW_OUTCOME> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_decision_explanation_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "always([+REQUEST_DECISION_EXPLANATION] true -> eventually(<+COLLECT_DECISION_FACTORS> true))"
        ));
        assert!(output.contains(
            "always([+COLLECT_DECISION_FACTORS] true -> eventually(<+APPROVE_DECISION_EXPLANATION> true))"
        ));
        assert!(output.contains(
            "always([+APPROVE_DECISION_EXPLANATION] true -> eventually(<+DELIVER_DECISION_EXPLANATION> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_decision_correction_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "always([+REQUEST_DECISION_CORRECTION] true -> eventually(<+REVIEW_DECISION_ERROR> true))"
        ));
        assert!(output.contains(
            "always([+REVIEW_DECISION_ERROR] true -> eventually(<+APPROVE_DECISION_CORRECTION> true))"
        ));
        assert!(output.contains(
            "always([+APPROVE_DECISION_CORRECTION] true -> eventually(<+RECORD_DECISION_CORRECTION> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_decision_recourse_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "always([+REQUEST_DECISION_RECOURSE] true -> eventually(<+REVIEW_RECOURSE_OPTIONS> true))"
        ));
        assert!(output.contains(
            "always([+REVIEW_RECOURSE_OPTIONS] true -> eventually(<+APPROVE_RECOURSE_PLAN> true))"
        ));
        assert!(output.contains(
            "always([+APPROVE_RECOURSE_PLAN] true -> eventually(<+RECORD_RECOURSE_OUTCOME> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_adverse_action_notice_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "always([+REQUEST_ADVERSE_ACTION_NOTICE] true -> eventually(<+COMPILE_NOTICE_EVIDENCE> true))"
        ));
        assert!(output.contains(
            "always([+COMPILE_NOTICE_EVIDENCE] true -> eventually(<+APPROVE_ADVERSE_ACTION_NOTICE> true))"
        ));
        assert!(output.contains(
            "always([+APPROVE_ADVERSE_ACTION_NOTICE] true -> eventually(<+DELIVER_ADVERSE_ACTION_NOTICE> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_automated_decision_contest_ordering_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "always([+CONTEST_AUTOMATED_DECISION] true -> eventually(<+REVIEW_CONTEST_EVIDENCE> true))"
        ));
        assert!(output.contains(
            "always([+REVIEW_CONTEST_EVIDENCE] true -> eventually(<+APPROVE_CONTEST_RESOLUTION> true))"
        ));
        assert!(output.contains(
            "always([+APPROVE_CONTEST_RESOLUTION] true -> eventually(<+RECORD_CONTEST_RESOLUTION> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_committed_gfp_branch_order_example() {
        let output = synthesis_list_text();

        assert!(output.contains("gfp(X, []X & ([<+APPROVE>] true))"));
    }

    #[test]
    fn synthesis_list_includes_parenthesized_committed_gfp_order_example() {
        let output = synthesis_list_text();

        assert!(output.contains("gfp(X, [](X) & ([<+APPROVE>] true))"));
    }

    #[test]
    fn synthesis_list_includes_parenthesized_committed_gfp_branch_order_example() {
        let output = synthesis_list_text();

        assert!(output.contains("gfp(X, []((X)) & ([<+APPROVE>] true))"));
    }

    #[test]
    fn synthesis_list_includes_parenthesized_unlabeled_committed_gfp_branch_order_example() {
        let output = synthesis_list_text();

        assert!(output.contains("gfp(X, [<>](X) & ([<+APPROVE>] true))"));
    }

    #[test]
    fn synthesis_list_includes_nested_parenthesized_unlabeled_committed_gfp_branch_order_example() {
        let output = synthesis_list_text();

        assert!(output.contains("gfp(X, [<>]((X)) & ([<+APPROVE>] true))"));
    }

    #[test]
    fn synthesis_list_includes_nested_parenthesized_unlabeled_committed_lfp_branch_order_example() {
        let output = synthesis_list_text();

        assert!(output.contains("lfp(X, [<>]((X)) | ([<+APPROVE>] true))"));
    }

    #[test]
    fn synthesis_list_includes_nested_parenthesized_committed_lfp_goal_example() {
        let output = synthesis_list_text();

        assert!(output.contains("lfp(X, ([<+APPROVE>] true) | <>((X)))"));
    }

    #[test]
    fn synthesis_list_includes_raw_guarded_branch_before_committed_goal_example() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "lfp(X, ((<+REVIEW> true) & ((<+WAIT> true) & <>X)) | ([<+APPROVE>] true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_parenthesized_guarded_branch_before_committed_goal_example() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "lfp(X, ((<+REVIEW> true) & ((<+WAIT> true) & <>(X))) | ([<+APPROVE>] true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_raw_guarded_branch_before_goal_example() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "lfp(X, ((<+REVIEW> true) & ((<+WAIT> true) & <>X)) | (<+APPROVE> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_parenthesized_guarded_branch_before_goal_example() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "lfp(X, ((<+REVIEW> true) & ((<+WAIT> true) & <>(X))) | (<+APPROVE> true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_nested_parenthesized_lfp_goal_example() {
        let output = synthesis_list_text();

        assert!(output.contains("lfp(X, (<+APPROVE> true) | <>((X)))"));
    }

    #[test]
    fn synthesis_list_includes_parenthesized_lfp_goal_example() {
        let output = synthesis_list_text();

        assert!(output.contains("lfp(X, (<+APPROVE> true) | <>(X))"));
    }

    #[test]
    fn synthesis_list_includes_raw_lfp_goal_example() {
        let output = synthesis_list_text();

        assert!(output.contains("lfp(X, (<+APPROVE> true) | <>X)"));
    }

    #[test]
    fn synthesis_list_includes_raw_committed_lfp_goal_example() {
        let output = synthesis_list_text();

        assert!(output.contains("lfp(X, ([<+APPROVE>] true) | <>X)"));
    }

    #[test]
    fn synthesis_list_includes_permissive_gfp_branch_order_example() {
        let output = synthesis_list_text();

        assert!(output.contains("gfp(X, []X & (<+APPROVE> true))"));
    }

    #[test]
    fn synthesis_list_includes_parenthesized_permissive_gfp_order_example() {
        let output = synthesis_list_text();

        assert!(output.contains("gfp(X, [](X) & (<+APPROVE> true))"));
    }

    #[test]
    fn synthesis_list_includes_parenthesized_permissive_gfp_branch_order_example() {
        let output = synthesis_list_text();

        assert!(output.contains("gfp(X, []((X)) & (<+APPROVE> true))"));
    }

    #[test]
    fn synthesis_list_includes_authorization_predicate_examples() {
        let output = synthesis_list_text();

        assert!(output.contains("Authorization and predicates"));
        assert!(output.contains("always([+UPDATE] true -> <+any_signed(/members)> true)"));
        assert!(output.contains(
            "always([+CHANGE_MEMBERS] true -> <+modifies(/members) +all_signed(/members)> true)"
        ));
        assert!(output.contains(
            r#"[<+SETTLE_ESCROW>] true -> [<+modifies(/escrow) +oracle_attests(/oracles/delivery.id, \"delivered\", \"true\")>] true"#
        ));
        assert!(output.contains(
            r#"[<+EXECUTE_TREASURY>] true -> [<+modifies(/treasury) +threshold(\"2\", /treasury/signers)>] true"#
        ));
        assert!(output.contains(
            r#"[<+PUBLISH_AUDIT>] true -> [<+modifies(/audit) +signed_by(/users/auditor.id) +oracle_attests(/oracles/audit.id, \"passed\", \"true\")>] true"#
        ));
    }

    #[test]
    fn synthesis_list_includes_credential_access_predicate_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            r#"[<+APPROVE_ACCESS_REVIEW>] true -> [<+modifies(/access_reviews) +signed_by(/users/access_reviewer.id)>] true"#
        ));
        assert!(output.contains(
            r#"[<+ISSUE_CREDENTIAL>] true -> [<+modifies(/credentials) +signed_by(/users/credential_issuer.id)>] true"#
        ));
        assert!(output.contains(
            r#"[<+VERIFY_CREDENTIAL>] true -> [<+modifies(/credential_verifications) +signed_by(/users/credential_verifier.id)>] true"#
        ));
        assert!(output.contains(
            r#"[<+PRESENT_CREDENTIAL>] true -> [<+modifies(/credential_presentations) +signed_by(/users/credential_holder.id)>] true"#
        ));
        assert!(output.contains(
            r#"[<+SHARE_CREDENTIAL>] true -> [<+modifies(/credential_shares) +signed_by(/users/credential_holder.id)>] true"#
        ));
    }

    #[test]
    fn synthesis_list_includes_credential_lifecycle_predicate_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            r#"[<+REVOKE_CREDENTIAL>] true -> [<+modifies(/credential_revocations) +signed_by(/users/credential_issuer.id)>] true"#
        ));
        assert!(output.contains(
            r#"[<+RENEW_CREDENTIAL>] true -> [<+modifies(/credential_renewals) +signed_by(/users/credential_issuer.id)>] true"#
        ));
        assert!(output.contains(
            r#"[<+EXPIRE_CREDENTIAL>] true -> [<+modifies(/credential_expirations) +signed_by(/users/credential_issuer.id)>] true"#
        ));
        assert!(output.contains(
            r#"[<+SUSPEND_CREDENTIAL>] true -> [<+modifies(/credential_suspensions) +signed_by(/users/credential_issuer.id)>] true"#
        ));
        assert!(output.contains(
            r#"[<+REINSTATE_CREDENTIAL>] true -> [<+modifies(/credential_reinstatements) +signed_by(/users/credential_issuer.id)>] true"#
        ));
    }

    #[test]
    fn synthesis_list_includes_credential_exchange_predicate_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            r#"[<+EXPORT_CREDENTIAL>] true -> [<+modifies(/credential_exports) +signed_by(/users/credential_holder.id)>] true"#
        ));
        assert!(output.contains(
            r#"[<+REQUEST_CREDENTIAL>] true -> [<+modifies(/credential_requests) +signed_by(/users/credential_holder.id)>] true"#
        ));
        assert!(output.contains(
            r#"[<+ACCEPT_CREDENTIAL>] true -> [<+modifies(/credential_acceptances) +signed_by(/users/credential_holder.id)>] true"#
        ));
        assert!(output.contains(
            r#"[<+REJECT_CREDENTIAL>] true -> [<+modifies(/credential_rejections) +signed_by(/users/credential_holder.id)>] true"#
        ));
    }

    #[test]
    fn synthesis_list_includes_access_governance_predicate_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            r#"[<+APPROVE_COMPLIANCE>] true -> [<+modifies(/compliance) +signed_by(/users/compliance_owner.id)>] true"#
        ));
        assert!(output.contains(
            r#"[<+APPROVE_ONBOARDING>] true -> [<+modifies(/onboarding) +signed_by(/users/onboarding_owner.id)>] true"#
        ));
        assert!(output.contains(
            r#"[<+APPROVE_OFFBOARDING>] true -> [<+modifies(/offboarding) +signed_by(/users/offboarding_owner.id)>] true"#
        ));
        assert!(output.contains(
            r#"[<+APPROVE_DEPROVISIONING>] true -> [<+modifies(/deprovisioning) +signed_by(/users/access_owner.id)>] true"#
        ));
        assert!(output.contains(
            r#"[<+APPROVE_ACCESS_REVIEW>] true -> [<+modifies(/access_reviews) +signed_by(/users/access_reviewer.id)>] true"#
        ));
        assert!(output.contains(
            r#"[<+APPROVE_IDENTITY_VERIFICATION>] true -> [<+modifies(/identity_verifications) +signed_by(/users/identity_reviewer.id)>] true"#
        ));
    }

    #[test]
    fn synthesis_list_includes_operational_approval_predicate_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            r#"[<+APPROVE_BUDGET>] true -> [<+modifies(/budgets) +signed_by(/users/budget_owner.id)>] true"#
        ));
        assert!(output.contains(
            r#"[<+APPROVE_PURCHASE_ORDER>] true -> [<+modifies(/purchase_orders) +signed_by(/users/procurement_manager.id)>] true"#
        ));
        assert!(output.contains(
            r#"[<+APPROVE_CONTRACT>] true -> [<+modifies(/contracts) +signed_by(/users/legal_reviewer.id)>] true"#
        ));
        assert!(output.contains(
            r#"[<+ONBOARD_VENDOR>] true -> [<+modifies(/vendors) +signed_by(/users/vendor_manager.id)>] true"#
        ));
        assert!(output.contains(
            r#"[<+APPROVE_EXPENSE>] true -> [<+modifies(/expenses) +signed_by(/users/finance_manager.id)>] true"#
        ));
    }

    #[test]
    fn synthesis_list_includes_financial_approval_predicate_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            r#"[<+APPROVE_REFUND>] true -> [<+modifies(/refunds) +signed_by(/users/refund_manager.id)>] true"#
        ));
        assert!(output.contains(
            r#"[<+APPROVE_CREDIT>] true -> [<+modifies(/credits) +signed_by(/users/credit_manager.id)>] true"#
        ));
        assert!(output.contains(
            r#"[<+APPROVE_PAYMENT>] true -> [<+modifies(/payments) +signed_by(/users/payment_approver.id)>] true"#
        ));
        assert!(output.contains(
            r#"[<+APPROVE_LOAN>] true -> [<+modifies(/loans) +signed_by(/users/loan_officer.id)>] true"#
        ));
        assert!(output.contains(
            r#"[<+APPROVE_CLAIM>] true -> [<+modifies(/claims) +signed_by(/users/claims_adjuster.id)>] true"#
        ));
    }

    #[test]
    fn synthesis_list_includes_records_lifecycle_predicate_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            r#"[<+APPROVE_DEPRECATION>] true -> [<+modifies(/deprecations) +signed_by(/users/product_manager.id)>] true"#
        ));
        assert!(output.contains(
            r#"[<+APPROVE_ARCHIVE>] true -> [<+modifies(/archives) +signed_by(/users/records_manager.id)>] true"#
        ));
        assert!(output.contains(
            r#"[<+APPROVE_RETENTION>] true -> [<+modifies(/retention) +signed_by(/users/records_counsel.id)>] true"#
        ));
        assert!(output.contains(
            r#"[<+APPROVE_POLICY>] true -> [<+modifies(/policies) +signed_by(/users/policy_owner.id)>] true"#
        ));
    }

    #[test]
    fn synthesis_list_includes_policy_approval_predicate_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            r#"[<+APPROVE_POLICY>] true -> [<+modifies(/policies) +signed_by(/users/policy_owner.id)>] true"#
        ));
        assert!(output.contains(
            r#"[<+APPROVE_CERTIFICATION>] true -> [<+modifies(/certifications) +signed_by(/users/certification_manager.id)>] true"#
        ));
        assert!(output.contains(
            r#"[<+APPROVE_ACCREDITATION>] true -> [<+modifies(/accreditations) +signed_by(/users/accreditation_manager.id)>] true"#
        ));
        assert!(output.contains(
            r#"[<+APPROVE_WAIVER>] true -> [<+modifies(/waivers) +signed_by(/users/waiver_authority.id)>] true"#
        ));
        assert!(output.contains(
            r#"[<+APPROVE_EXCEPTION>] true -> [<+modifies(/exceptions) +signed_by(/users/exception_owner.id)>] true"#
        ));
    }

    #[test]
    fn synthesis_list_includes_contract_change_predicate_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            r#"[<+APPROVE_EXTENSION>] true -> [<+modifies(/extensions) +signed_by(/users/extension_owner.id)>] true"#
        ));
        assert!(output.contains(
            r#"[<+APPROVE_AMENDMENT>] true -> [<+modifies(/amendments) +signed_by(/users/amendment_owner.id)>] true"#
        ));
        assert!(output.contains(
            r#"[<+APPROVE_ADDENDUM>] true -> [<+modifies(/addenda) +signed_by(/users/addendum_owner.id)>] true"#
        ));
        assert!(output.contains(
            r#"[<+APPROVE_SUPPLEMENT>] true -> [<+modifies(/supplements) +signed_by(/users/supplement_owner.id)>] true"#
        ));
        assert!(output.contains(
            r#"[<+APPROVE_APPENDIX>] true -> [<+modifies(/appendices) +signed_by(/users/appendix_owner.id)>] true"#
        ));
    }

    #[test]
    fn synthesis_list_includes_contract_attachment_predicate_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            r#"[<+APPROVE_RIDER>] true -> [<+modifies(/riders) +signed_by(/users/rider_owner.id)>] true"#
        ));
        assert!(output.contains(
            r#"[<+APPROVE_ENDORSEMENT>] true -> [<+modifies(/endorsements) +signed_by(/users/endorsement_owner.id)>] true"#
        ));
        assert!(output.contains(
            r#"[<+APPROVE_EXHIBIT>] true -> [<+modifies(/exhibits) +signed_by(/users/exhibit_owner.id)>] true"#
        ));
        assert!(output.contains(
            r#"[<+APPROVE_SCHEDULE>] true -> [<+modifies(/schedules) +signed_by(/users/schedule_owner.id)>] true"#
        ));
        assert!(output.contains(
            r#"[<+APPROVE_ATTACHMENT>] true -> [<+modifies(/attachments) +signed_by(/users/attachment_owner.id)>] true"#
        ));
    }

    #[test]
    fn synthesis_list_includes_contract_package_predicate_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            r#"[<+APPROVE_ANNEX>] true -> [<+modifies(/annexes) +signed_by(/users/annex_owner.id)>] true"#
        ));
        assert!(output.contains(
            r#"[<+APPROVE_ENCLOSURE>] true -> [<+modifies(/enclosures) +signed_by(/users/enclosure_owner.id)>] true"#
        ));
        assert!(output.contains(
            r#"[<+APPROVE_PACKAGE>] true -> [<+modifies(/packages) +signed_by(/users/package_owner.id)>] true"#
        ));
        assert!(output.contains(
            r#"[<+APPROVE_BUNDLE>] true -> [<+modifies(/bundles) +signed_by(/users/bundle_owner.id)>] true"#
        ));
        assert!(output.contains(
            r#"[<+APPROVE_DOSSIER>] true -> [<+modifies(/dossiers) +signed_by(/users/dossier_owner.id)>] true"#
        ));
    }

    #[test]
    fn synthesis_list_includes_case_intake_predicate_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            r#"[<+APPROVE_FILE>] true -> [<+modifies(/files) +signed_by(/users/file_owner.id)>] true"#
        ));
        assert!(output.contains(
            r#"[<+APPROVE_RECORD>] true -> [<+modifies(/records) +signed_by(/users/record_owner.id)>] true"#
        ));
        assert!(output.contains(
            r#"[<+APPROVE_CASE>] true -> [<+modifies(/cases) +signed_by(/users/case_owner.id)>] true"#
        ));
        assert!(output.contains(
            r#"[<+APPROVE_TICKET>] true -> [<+modifies(/tickets) +signed_by(/users/ticket_owner.id)>] true"#
        ));
        assert!(output.contains(
            r#"[<+APPROVE_PROPOSAL>] true -> [<+modifies(/proposals) +signed_by(/users/proposal_owner.id)>] true"#
        ));
        assert!(output.contains(
            r#"[<+APPROVE_REQUEST>] true -> [<+modifies(/requests) +signed_by(/users/request_owner.id)>] true"#
        ));
    }

    #[test]
    fn synthesis_list_includes_application_submission_predicate_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            r#"[<+APPROVE_APPLICATION>] true -> [<+modifies(/applications) +signed_by(/users/application_owner.id)>] true"#
        ));
        assert!(output.contains(
            r#"[<+APPROVE_SUBMISSION>] true -> [<+modifies(/submissions) +signed_by(/users/submission_owner.id)>] true"#
        ));
    }

    #[test]
    fn synthesis_list_includes_document_approval_predicate_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            r#"[<+APPROVE_DOCUMENT>] true -> [<+modifies(/documents) +signed_by(/users/document_owner.id)>] true"#
        ));
        assert!(output.contains(
            r#"[<+APPROVE_REPORT>] true -> [<+modifies(/reports) +signed_by(/users/report_owner.id)>] true"#
        ));
        assert!(output.contains(
            r#"[<+APPROVE_MEMO>] true -> [<+modifies(/memos) +signed_by(/users/memo_owner.id)>] true"#
        ));
        assert!(output.contains(
            r#"[<+APPROVE_NOTE>] true -> [<+modifies(/notes) +signed_by(/users/note_owner.id)>] true"#
        ));
        assert!(output.contains(
            r#"[<+APPROVE_COMMENT>] true -> [<+modifies(/comments) +signed_by(/users/comment_owner.id)>] true"#
        ));
    }

    #[test]
    fn synthesis_list_includes_feedback_approval_predicate_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            r#"[<+APPROVE_FEEDBACK>] true -> [<+modifies(/feedback) +signed_by(/users/feedback_owner.id)>] true"#
        ));
        assert!(output.contains(
            r#"[<+APPROVE_RATING>] true -> [<+modifies(/ratings) +signed_by(/users/rating_owner.id)>] true"#
        ));
        assert!(output.contains(
            r#"[<+APPROVE_REVIEW>] true -> [<+modifies(/reviews) +signed_by(/users/review_owner.id)>] true"#
        ));
        assert!(output.contains(
            r#"[<+APPROVE_SURVEY>] true -> [<+modifies(/surveys) +signed_by(/users/survey_owner.id)>] true"#
        ));
        assert!(output.contains(
            r#"[<+APPROVE_RESPONSE>] true -> [<+modifies(/responses) +signed_by(/users/response_owner.id)>] true"#
        ));
    }

    #[test]
    fn synthesis_list_includes_decision_planning_predicate_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            r#"[<+APPROVE_RESULT>] true -> [<+modifies(/results) +signed_by(/users/result_owner.id)>] true"#
        ));
        assert!(output.contains(
            r#"[<+APPROVE_OUTCOME>] true -> [<+modifies(/outcomes) +signed_by(/users/outcome_owner.id)>] true"#
        ));
        assert!(output.contains(
            r#"[<+APPROVE_DECISION>] true -> [<+modifies(/decisions) +signed_by(/users/decision_owner.id)>] true"#
        ));
        assert!(output.contains(
            r#"[<+APPROVE_PLAN>] true -> [<+modifies(/plans) +signed_by(/users/plan_owner.id)>] true"#
        ));
        assert!(output.contains(
            r#"[<+APPROVE_STRATEGY>] true -> [<+modifies(/strategies) +signed_by(/users/strategy_owner.id)>] true"#
        ));
    }

    #[test]
    fn synthesis_list_includes_goal_metric_predicate_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            r#"[<+APPROVE_OBJECTIVE>] true -> [<+modifies(/objectives) +signed_by(/users/objective_owner.id)>] true"#
        ));
        assert!(output.contains(
            r#"[<+APPROVE_TARGET>] true -> [<+modifies(/targets) +signed_by(/users/target_owner.id)>] true"#
        ));
        assert!(output.contains(
            r#"[<+APPROVE_GOAL>] true -> [<+modifies(/goals) +signed_by(/users/goal_owner.id)>] true"#
        ));
        assert!(output.contains(
            r#"[<+APPROVE_KPI>] true -> [<+modifies(/kpis) +signed_by(/users/kpi_owner.id)>] true"#
        ));
        assert!(output.contains(
            r#"[<+APPROVE_METRIC>] true -> [<+modifies(/metrics) +signed_by(/users/metric_owner.id)>] true"#
        ));
    }

    #[test]
    fn synthesis_list_includes_delivery_planning_predicate_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            r#"[<+APPROVE_OKR>] true -> [<+modifies(/okrs) +signed_by(/users/okr_owner.id)>] true"#
        ));
        assert!(output.contains(
            r#"[<+APPROVE_INITIATIVE>] true -> [<+modifies(/initiatives) +signed_by(/users/initiative_owner.id)>] true"#
        ));
        assert!(output.contains(
            r#"[<+APPROVE_EPIC>] true -> [<+modifies(/epics) +signed_by(/users/epic_owner.id)>] true"#
        ));
        assert!(output.contains(
            r#"[<+APPROVE_STORY>] true -> [<+modifies(/stories) +signed_by(/users/story_owner.id)>] true"#
        ));
        assert!(output.contains(
            r#"[<+APPROVE_TASK>] true -> [<+modifies(/tasks) +signed_by(/users/task_owner.id)>] true"#
        ));
    }

    #[test]
    fn synthesis_list_includes_defect_remediation_predicate_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            r#"[<+APPROVE_BUG>] true -> [<+modifies(/bugs) +signed_by(/users/bug_owner.id)>] true"#
        ));
        assert!(output.contains(
            r#"[<+APPROVE_ISSUE>] true -> [<+modifies(/issues) +signed_by(/users/issue_owner.id)>] true"#
        ));
        assert!(output.contains(
            r#"[<+APPROVE_DEFECT>] true -> [<+modifies(/defects) +signed_by(/users/defect_owner.id)>] true"#
        ));
        assert!(output.contains(
            r#"[<+APPROVE_PATCH>] true -> [<+modifies(/patches) +signed_by(/users/patch_owner.id)>] true"#
        ));
        assert!(output.contains(
            r#"[<+APPROVE_HOTFIX>] true -> [<+modifies(/hotfixes) +signed_by(/users/hotfix_owner.id)>] true"#
        ));
    }

    #[test]
    fn synthesis_list_includes_release_lifecycle_predicate_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            r#"[<+APPROVE_RELEASE_CANDIDATE>] true -> [<+modifies(/release_candidates) +signed_by(/users/release_manager.id)>] true"#
        ));
        assert!(output.contains(
            r#"[<+APPROVE_DEPLOYMENT>] true -> [<+modifies(/deployments) +signed_by(/users/deployment_owner.id)>] true"#
        ));
        assert!(output.contains(
            r#"[<+APPROVE_ROLLOUT>] true -> [<+modifies(/rollouts) +signed_by(/users/rollout_owner.id)>] true"#
        ));
        assert!(output.contains(
            r#"[<+APPROVE_LAUNCH>] true -> [<+modifies(/launches) +signed_by(/users/launch_owner.id)>] true"#
        ));
        assert!(output.contains(
            r#"[<+APPROVE_GENERAL_AVAILABILITY>] true -> [<+modifies(/general_availability) +signed_by(/users/ga_owner.id)>] true"#
        ));
    }

    #[test]
    fn synthesis_list_includes_production_operations_predicate_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            r#"[<+APPROVE_PRODUCTION>] true -> [<+modifies(/production) +signed_by(/users/production_owner.id)>] true"#
        ));
        assert!(output.contains(
            r#"[<+APPROVE_OPERATIONS>] true -> [<+modifies(/operations) +signed_by(/users/operations_owner.id)>] true"#
        ));
        assert!(output.contains(
            r#"[<+APPROVE_MAINTENANCE>] true -> [<+modifies(/maintenance) +signed_by(/users/maintenance_owner.id)>] true"#
        ));
        assert!(output.contains(
            r#"[<+APPROVE_SUPPORT>] true -> [<+modifies(/support) +signed_by(/users/support_owner.id)>] true"#
        ));
        assert!(output.contains(
            r#"[<+APPROVE_TRAINING>] true -> [<+modifies(/training) +signed_by(/users/training_owner.id)>] true"#
        ));
    }

    #[test]
    fn synthesis_list_includes_risk_and_compliance_predicate_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            r#"[<+CLOSE_INCIDENT>] true -> [<+modifies(/incidents) +signed_by(/users/incident_commander.id)>] true"#
        ));
        assert!(output.contains(
            r#"[<+FREEZE_CHANGE>] true -> [<+modifies(/releases) +signed_by(/users/release_manager.id)>] true"#
        ));
        assert!(output.contains(
            r#"[<+ACCEPT_RISK>] true -> [<+modifies(/risk) +signed_by(/users/risk_owner.id)>] true"#
        ));
        assert!(output.contains(
            r#"[<+APPROVE_SAFETY>] true -> [<+modifies(/safety) +signed_by(/users/safety_reviewer.id)>] true"#
        ));
        assert!(output.contains(
            r#"[<+ATTEST_COMPLIANCE>] true -> [<+modifies(/compliance) +signed_by(/users/compliance_officer.id)>] true"#
        ));
    }

    #[test]
    fn synthesis_list_includes_authorization_eventual_goal_examples() {
        let output = synthesis_list_text();

        assert!(output.contains("Authorization with eventual goals"));
        assert!(output.contains(
            "[<+RELEASE>] true -> (<+signed_by(/users/buyer.id)> true & (eventually(<+DEPOSIT> true) & eventually(<+DELIVER> true)))"
        ));
        assert!(output.contains(
            "[<+APPROVE>] true -> ([<+signed_by(/users/alice.id) +signed_by(/users/bob.id)>] true & eventually([<+DELIVER>] true))"
        ));
        assert!(output.contains(
            r#"[<+RELEASE>] true -> (<+oracle_attests(/oracles/delivery.id, \"delivered\", \"true\")> true & (eventually([<+DEPOSIT>] true) & eventually([<+DELIVER>] true)))"#
        ));
    }

    #[test]
    fn synthesis_list_includes_authorized_followup_obligation_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            r#"always([+USE_TOOL] true -> (<+signed_by(/users/tool_provider.id)> true & eventually([<+APPROVE_CAPABILITY>] true)))"#
        ));
        assert!(output.contains(
            r#"[+RELEASE] true -> (<+oracle_attests(/oracles/delivery.id, \"delivered\", \"true\")> true & eventually(<+DELIVER> true))"#
        ));
        assert!(output.contains(
            r#"[+RELEASE] true -> (<+oracle_attests(/oracles/delivery.id, \"delivered\", \"true\")> true & eventually([<+DELIVER>] true))"#
        ));
        assert!(output.contains(
            r#"[<+RELEASE>] true -> ([<+signed_by(/users/buyer.id)>] true & eventually([<+DELIVER>] true))"#
        ));
    }

    #[test]
    fn synthesis_list_includes_forbidden_after_guard_examples() {
        let output = synthesis_list_text();

        assert!(output.contains("Forbidden-after guards"));
        assert!(output.contains("[<+DISPUTE>] true -> always([-RELEASE] true)"));
        assert!(output
            .contains("[<+DISPUTE>] true -> (always([-RELEASE] true) & always([-REFUND] true))"));
        assert!(output.contains(
            "[<+DISPUTE>] true -> ([<+signed_by(/users/arbiter.id)>] true & always([-RELEASE] true))"
        ));
        assert!(output.contains(
            "[<+DISPUTE>] true -> ([<+signed_by(/users/arbiter.id)>] true & (always([-RELEASE] true) & always([-REFUND] true)))"
        ));
        assert!(output.contains(
            "[<+DISPUTE>] true -> ([<+signed_by(/users/alice.id) +signed_by(/users/bob.id)>] true & always([-RELEASE] true))"
        ));
        assert!(output.contains(
            "[<+DISPUTE>] true -> ([<+signed_by(/users/alice.id) +signed_by(/users/bob.id)>] true & (always([-RELEASE] true) & always([-REFUND] true)))"
        ));
        assert!(output.contains(
            r#"[<+DISPUTE>] true -> (<+oracle_attests(/oracles/dispute.id, \"opened\", \"true\")> true & always([-RELEASE] true))"#
        ));
        assert!(output.contains(
            r#"[<+DISPUTE>] true -> (<+oracle_attests(/oracles/dispute.id, \"opened\", \"true\")> true & (always([-RELEASE] true) & always([-REFUND] true)))"#
        ));
        assert!(output.contains(
            r#"[<+DISPUTE>] true -> ([<+oracle_attests(/oracles/dispute.id, \"opened\", \"true\")>] true & always([-RELEASE] true))"#
        ));
        assert!(output.contains(
            r#"[<+DISPUTE>] true -> ([<+oracle_attests(/oracles/dispute.id, \"opened\", \"true\")>] true & (always([-RELEASE] true) & always([-REFUND] true)))"#
        ));
    }

    #[test]
    fn synthesis_list_includes_authorized_forbidden_guard_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "[+DISPUTE] true -> (<+signed_by(/users/arbiter.id)> true & always([-RELEASE] true))"
        ));
        assert!(output.contains(
            "[+DISPUTE] true -> (<+signed_by(/users/arbiter.id)> true & (always([-RELEASE] true) & always([-REFUND] true)))"
        ));
        assert!(output.contains(
            "[+DISPUTE] true -> (<+signed_by(/users/alice.id) +signed_by(/users/bob.id)> true & always([-RELEASE] true))"
        ));
        assert!(output.contains(
            "[+DISPUTE] true -> (<+signed_by(/users/alice.id) +signed_by(/users/bob.id)> true & (always([-RELEASE] true) & always([-REFUND] true)))"
        ));
        assert!(output.contains(
            "[+DISPUTE] true -> ([<+signed_by(/users/arbiter.id)>] true & always([-RELEASE] true))"
        ));
        assert!(output.contains(
            "[+DISPUTE] true -> ([<+signed_by(/users/arbiter.id)>] true & (always([-RELEASE] true) & always([-REFUND] true)))"
        ));
        assert!(output.contains(
            r#"[+DISPUTE] true -> (<+oracle_attests(/oracles/dispute.id, \"opened\", \"true\")> true & always([-RELEASE] true))"#
        ));
        assert!(output.contains(
            r#"[+DISPUTE] true -> (<+oracle_attests(/oracles/dispute.id, \"opened\", \"true\")> true & (always([-RELEASE] true) & always([-REFUND] true)))"#
        ));
    }

    #[test]
    fn synthesis_list_includes_forbidden_lifecycle_guard_examples() {
        let output = synthesis_list_text();

        assert!(output
            .contains("always([+APPROVE_INVOICE] true -> always([-CHARGEBACK] true))"));
        assert!(output
            .contains("always([+ACCEPT_MILESTONE] true -> always([-REWORK] true))"));
        assert!(output
            .contains("always([+APPROVE_INSPECTION] true -> always([-DEFECT_CLAIM] true))"));
        assert!(output.contains(
            "always([+ATTEST_COMPLIANCE] true -> always([-NONCOMPLIANCE_FINDING] true))"
        ));
        assert!(output
            .contains("always([+APPROVE_SAFETY] true -> always([-UNSAFE_DEPLOYMENT] true))"));
        assert!(output
            .contains("always([+ACCEPT_RISK] true -> always([-UNMITIGATED_EXPOSURE] true))"));
        assert!(output
            .contains("always([+CLOSE_INCIDENT] true -> always([-REOPEN_INCIDENT] true))"));
        assert!(output.contains("always([+FREEZE_CHANGE] true -> always([-DEPLOY] true))"));
    }

    #[test]
    fn synthesis_list_includes_authorized_lifecycle_guard_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "[+APPROVE_INVOICE] true -> (<+signed_by(/users/finance_approver.id)> true & always([-CHARGEBACK] true))"
        ));
        assert!(output.contains(
            "[+ACCEPT_MILESTONE] true -> (<+signed_by(/users/client_reviewer.id)> true & always([-REWORK] true))"
        ));
        assert!(output.contains(
            "[+APPROVE_INSPECTION] true -> (<+signed_by(/users/inspector.id)> true & always([-DEFECT_CLAIM] true))"
        ));
        assert!(output.contains(
            r#"[+ATTEST_COMPLIANCE] true -> (<+oracle_attests(/oracles/compliance.id, \"status\", \"clear\")> true & always([-NONCOMPLIANCE_FINDING] true))"#
        ));
        assert!(output.contains(
            "[<+APPROVE_INVOICE>] true -> (<+signed_by(/users/finance_approver.id)> true & always([-CHARGEBACK] true))"
        ));
        assert!(output.contains(
            "[<+ACCEPT_MILESTONE>] true -> (<+signed_by(/users/client_reviewer.id)> true & always([-REWORK] true))"
        ));
        assert!(output.contains(
            "[<+APPROVE_INSPECTION>] true -> (<+signed_by(/users/inspector.id)> true & always([-DEFECT_CLAIM] true))"
        ));
        assert!(output.contains(
            r#"[<+ATTEST_COMPLIANCE>] true -> (<+oracle_attests(/oracles/compliance.id, \"status\", \"clear\")> true & always([-NONCOMPLIANCE_FINDING] true))"#
        ));
    }

    #[test]
    fn synthesis_list_includes_committed_authorized_lifecycle_guard_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "[+APPROVE_INVOICE] true -> ([<+signed_by(/users/finance_approver.id)>] true & always([-CHARGEBACK] true))"
        ));
        assert!(output.contains(
            "[+ACCEPT_MILESTONE] true -> ([<+signed_by(/users/client_reviewer.id)>] true & always([-REWORK] true))"
        ));
        assert!(output.contains(
            "[+APPROVE_INSPECTION] true -> ([<+signed_by(/users/inspector.id)>] true & always([-DEFECT_CLAIM] true))"
        ));
        assert!(output.contains(
            r#"[+ATTEST_COMPLIANCE] true -> ([<+oracle_attests(/oracles/compliance.id, \"status\", \"clear\")>] true & always([-NONCOMPLIANCE_FINDING] true))"#
        ));
        assert!(output.contains(
            "[<+APPROVE_INVOICE>] true -> ([<+signed_by(/users/finance_approver.id)>] true & always([-CHARGEBACK] true))"
        ));
        assert!(output.contains(
            "[<+ACCEPT_MILESTONE>] true -> ([<+signed_by(/users/client_reviewer.id)>] true & always([-REWORK] true))"
        ));
        assert!(output.contains(
            "[<+APPROVE_INSPECTION>] true -> ([<+signed_by(/users/inspector.id)>] true & always([-DEFECT_CLAIM] true))"
        ));
        assert!(output.contains(
            r#"[<+ATTEST_COMPLIANCE>] true -> ([<+oracle_attests(/oracles/compliance.id, \"status\", \"clear\")>] true & always([-NONCOMPLIANCE_FINDING] true))"#
        ));
    }

    #[test]
    fn synthesis_list_includes_authorized_operational_guard_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "[+APPROVE_SAFETY] true -> (<+signed_by(/users/safety_officer.id)> true & always([-UNSAFE_DEPLOYMENT] true))"
        ));
        assert!(output.contains(
            "[+ACCEPT_RISK] true -> (<+signed_by(/users/risk_owner.id)> true & always([-UNMITIGATED_EXPOSURE] true))"
        ));
        assert!(output.contains(
            "[+CLOSE_INCIDENT] true -> (<+signed_by(/users/incident_commander.id)> true & always([-REOPEN_INCIDENT] true))"
        ));
        assert!(output.contains(
            "[+FREEZE_CHANGE] true -> (<+signed_by(/users/change_manager.id)> true & always([-DEPLOY] true))"
        ));
        assert!(output.contains(
            "[<+APPROVE_SAFETY>] true -> (<+signed_by(/users/safety_officer.id)> true & always([-UNSAFE_DEPLOYMENT] true))"
        ));
        assert!(output.contains(
            "[<+ACCEPT_RISK>] true -> (<+signed_by(/users/risk_owner.id)> true & always([-UNMITIGATED_EXPOSURE] true))"
        ));
        assert!(output.contains(
            "[<+CLOSE_INCIDENT>] true -> (<+signed_by(/users/incident_commander.id)> true & always([-REOPEN_INCIDENT] true))"
        ));
        assert!(output.contains(
            "[<+FREEZE_CHANGE>] true -> (<+signed_by(/users/change_manager.id)> true & always([-DEPLOY] true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_committed_authorized_operational_guard_examples() {
        let output = synthesis_list_text();

        assert!(output.contains(
            "[+APPROVE_SAFETY] true -> ([<+signed_by(/users/safety_officer.id)>] true & always([-UNSAFE_DEPLOYMENT] true))"
        ));
        assert!(output.contains(
            "[+ACCEPT_RISK] true -> ([<+signed_by(/users/risk_owner.id)>] true & always([-UNMITIGATED_EXPOSURE] true))"
        ));
        assert!(output.contains(
            "[+CLOSE_INCIDENT] true -> ([<+signed_by(/users/incident_commander.id)>] true & always([-REOPEN_INCIDENT] true))"
        ));
        assert!(output.contains(
            "[+FREEZE_CHANGE] true -> ([<+signed_by(/users/change_manager.id)>] true & always([-DEPLOY] true))"
        ));
        assert!(output.contains(
            "[<+APPROVE_SAFETY>] true -> ([<+signed_by(/users/safety_officer.id)>] true & always([-UNSAFE_DEPLOYMENT] true))"
        ));
        assert!(output.contains(
            "[<+ACCEPT_RISK>] true -> ([<+signed_by(/users/risk_owner.id)>] true & always([-UNMITIGATED_EXPOSURE] true))"
        ));
        assert!(output.contains(
            "[<+CLOSE_INCIDENT>] true -> ([<+signed_by(/users/incident_commander.id)>] true & always([-REOPEN_INCIDENT] true))"
        ));
        assert!(output.contains(
            "[<+FREEZE_CHANGE>] true -> ([<+signed_by(/users/change_manager.id)>] true & always([-DEPLOY] true))"
        ));
    }

    #[test]
    fn synthesis_list_includes_basic_lifecycle_guard_examples() {
        let output = synthesis_list_text();

        assert!(output.contains("always([+CANCEL] true -> always([-DELIVER] true))"));
        assert!(output.contains("always([+REFUND] true -> always([-RELEASE] true))"));
        assert!(output.contains("always([+TIMEOUT] true -> always([-COMPLETE] true))"));
        assert!(output.contains("always([+ESCALATE] true -> always([-CLOSE] true))"));
        assert!(output.contains("always([+WITHDRAW] true -> always([-CLAIM] true))"));
        assert!(output.contains("always([+APPEAL] true -> always([-ENFORCE] true))"));
        assert!(output.contains("always([+REVOKE] true -> always([-USE] true))"));
        assert!(output.contains("always([+SUSPEND] true -> always([-ACCESS] true))"));
    }

    #[test]
    fn synthesis_list_includes_committed_basic_lifecycle_guard_examples() {
        let output = synthesis_list_text();

        assert!(output.contains("always([<+CANCEL>] true -> always([-DELIVER] true))"));
        assert!(output.contains("always([<+REFUND>] true -> always([-RELEASE] true))"));
        assert!(output.contains("always([<+TIMEOUT>] true -> always([-COMPLETE] true))"));
        assert!(output.contains("always([<+ESCALATE>] true -> always([-CLOSE] true))"));
        assert!(output.contains("always([<+WITHDRAW>] true -> always([-CLAIM] true))"));
        assert!(output.contains("always([<+APPEAL>] true -> always([-ENFORCE] true))"));
        assert!(output.contains("always([<+REVOKE>] true -> always([-USE] true))"));
        assert!(output.contains("always([<+SUSPEND>] true -> always([-ACCESS] true))"));
    }

    #[test]
    fn synthesis_list_includes_extended_lifecycle_guard_examples() {
        let output = synthesis_list_text();

        assert!(output.contains("always([+REINSTATE] true -> always([-SUSPEND] true))"));
        assert!(output.contains("always([+RENEW] true -> always([-EXPIRE] true))"));
        assert!(output.contains("always([+TERMINATE] true -> always([-RENEW] true))"));
        assert!(output.contains("always([+EXTEND] true -> always([-TERMINATE] true))"));
        assert!(output.contains("always([+ASSIGN] true -> always([-REASSIGN] true))"));
        assert!(output.contains("always([+CERTIFY] true -> always([-DEPLOY] true))"));
        assert!(output.contains("always([+PUBLISH] true -> always([-EMBARGO] true))"));
        assert!(output.contains("always([+REGISTER] true -> always([-DELETE] true))"));
        assert!(output.contains("always([+ACCEPT] true -> always([-REJECT] true))"));
        assert!(output.contains("always([+ACKNOWLEDGE] true -> always([-DISPUTE] true))"));
        assert!(
            output.contains("always([+CONFIRM_DELIVERY] true -> always([-REFUND] true))")
        );
    }

    #[test]
    fn synthesis_list_includes_committed_extended_lifecycle_guard_examples() {
        let output = synthesis_list_text();

        assert!(output.contains("always([<+REINSTATE>] true -> always([-SUSPEND] true))"));
        assert!(output.contains("always([<+RENEW>] true -> always([-EXPIRE] true))"));
        assert!(output.contains("always([<+TERMINATE>] true -> always([-RENEW] true))"));
        assert!(output.contains("always([<+EXTEND>] true -> always([-TERMINATE] true))"));
        assert!(output.contains("always([<+ASSIGN>] true -> always([-REASSIGN] true))"));
        assert!(output.contains("always([<+CERTIFY>] true -> always([-DEPLOY] true))"));
        assert!(output.contains("always([<+PUBLISH>] true -> always([-EMBARGO] true))"));
        assert!(output.contains("always([<+REGISTER>] true -> always([-DELETE] true))"));
        assert!(output.contains("always([<+ACCEPT>] true -> always([-REJECT] true))"));
        assert!(output.contains("always([<+ACKNOWLEDGE>] true -> always([-DISPUTE] true))"));
        assert!(
            output.contains("always([<+CONFIRM_DELIVERY>] true -> always([-REFUND] true))")
        );
    }

    #[test]
    fn existing_model_check_accepts_satisfied_proposed_formula() {
        let parsed = parse_formula_strings(&["always([<+APPROVE>] true)".to_string()]);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &parsed);
        let labels = vec!["approval_required".to_string()];

        let failed = existing_model_unsatisfied_formula_labels(&model, &parsed, &labels);

        assert!(failed.is_empty());
    }

    #[test]
    fn existing_model_check_reports_unsatisfied_proposed_formula() {
        let model_formulas = parse_formula_strings(&["always([<+APPROVE>] true)".to_string()]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &model_formulas);
        let proposed = parse_formula_strings(&["false".to_string()]);
        let labels = vec!["false_rule".to_string()];

        let failed = existing_model_unsatisfied_formula_labels(&model, &proposed, &labels);

        assert_eq!(failed, vec!["false_rule".to_string()]);
    }

    #[test]
    fn existing_model_loader_preserves_formula_declarations() {
        let model_formulas = parse_formula_strings(&["always([<+APPROVE>] true)".to_string()]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &model_formulas);
        let content = format!(
            "{}\n\nformula previous_rule {{\nalways([<+APPROVE>] true)\n}}\n",
            modality_lang::print_model(&model)
        );
        let path = std::env::temp_dir().join(format!(
            "modality-existing-model-{}.modality",
            std::process::id()
        ));
        std::fs::write(&path, content).unwrap();

        let loaded = load_existing_model_input(&path).unwrap();
        std::fs::remove_file(&path).unwrap();

        assert_eq!(loaded.model.name, "Contract");
        assert_eq!(loaded.formulas.len(), 1);
        assert_eq!(loaded.labels, vec!["existing `previous_rule`".to_string()]);
    }

    #[test]
    fn existing_model_loader_rejects_ambiguous_model_files() {
        let first_formulas = parse_formula_strings(&["always([<+APPROVE>] true)".to_string()]);
        let second_formulas = parse_formula_strings(&["always([<+DELIVER>] true)".to_string()]);
        let first =
            modality_lang::formula_synthesis::synthesize_from_formulas("First", &first_formulas);
        let second =
            modality_lang::formula_synthesis::synthesize_from_formulas("Second", &second_formulas);
        let path = std::env::temp_dir().join(format!(
            "modality-existing-model-ambiguous-{}.modality",
            std::process::id()
        ));
        std::fs::write(
            &path,
            format!(
                "{}\n\n{}",
                modality_lang::print_model(&first),
                modality_lang::print_model(&second)
            ),
        )
        .unwrap();

        let err = match load_existing_model_input(&path) {
            Ok(_) => panic!("ambiguous existing model file should be rejected"),
            Err(err) => err,
        };
        std::fs::remove_file(path).unwrap();

        assert!(err.to_string().contains("Expected exactly one model"));
    }

    #[test]
    fn existing_model_mode_rejects_other_synthesis_modes() {
        let mut opts = default_test_opts();
        opts.existing_model = Some(PathBuf::from("existing.modality"));
        opts.proposed_formula = Some("always([<+APPROVE>] true)".to_string());
        opts.template = Some("escrow".to_string());
        opts.describe = Some("ignored natural language request".to_string());

        let err = ensure_existing_model_mode_is_exclusive(&opts).unwrap_err();

        assert!(err.to_string().contains("--existing-model"));
        assert!(err.to_string().contains("--template"));
        assert!(err.to_string().contains("--describe"));
    }

    #[tokio::test]
    async fn existing_model_mode_rejects_llm_response_file_before_reading_path() {
        let missing_response_path = std::env::temp_dir().join(format!(
            "modality-synthesize-conflicting-llm-response-{}.md",
            std::process::id()
        ));

        let mut opts = default_test_opts();
        opts.existing_model = Some(PathBuf::from("existing.modality"));
        opts.proposed_formula = Some("always([<+APPROVE>] true)".to_string());
        opts.llm_response_file = Some(missing_response_path.clone());

        let err = run(&opts).await.unwrap_err();

        let message = err.to_string();
        assert!(message.contains(
            "--existing-model cannot be combined with other synthesis modes: --llm-response-file"
        ));
        assert!(!message.contains(&missing_response_path.display().to_string()));
    }

    #[tokio::test]
    async fn existing_model_mode_rejects_template_before_missing_existing_model() {
        let mut opts = default_test_opts();
        opts.template = Some("escrow".to_string());
        opts.proposed_formula = Some("always([<+APPROVE>] true)".to_string());

        let err = run(&opts).await.unwrap_err();

        let message = err.to_string();
        assert!(message.contains(
            "--existing-model cannot be combined with other synthesis modes: --template"
        ));
        assert!(!message.contains("--existing-model is required"));
    }

    #[tokio::test]
    async fn existing_model_mode_rejects_template_before_proposed_source_count() {
        let mut opts = default_test_opts();
        opts.existing_model = Some(PathBuf::from("existing.modality"));
        opts.proposed_formula = Some("always([<+APPROVE>] true)".to_string());
        opts.proposed_rule = Some(PathBuf::from("proposed.modality"));
        opts.template = Some("escrow".to_string());

        let err = run(&opts).await.unwrap_err();

        let message = err.to_string();
        assert!(message.contains(
            "--existing-model cannot be combined with other synthesis modes: --template"
        ));
        assert!(!message.contains("Use exactly one"));
    }

    #[test]
    fn existing_model_mode_requires_exactly_one_proposed_source() {
        let mut opts = default_test_opts();
        opts.existing_model = Some(PathBuf::from("existing.modality"));
        opts.proposed_formula = Some("always([<+APPROVE>] true)".to_string());
        opts.proposed_rule = Some(PathBuf::from("proposed.modality"));

        let err = run_existing_model_synthesis(&opts).unwrap_err();

        assert!(err.to_string().contains("Use exactly one"));
        assert!(err.to_string().contains("--proposed-formula"));
        assert!(err.to_string().contains("--proposed-rule"));
    }

    #[test]
    fn existing_model_mode_rejects_empty_proposed_rule_file() {
        let model_formulas = parse_formula_strings(&["always([<+APPROVE>] true)".to_string()]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &model_formulas);
        let existing_path = std::env::temp_dir().join(format!(
            "modality-existing-model-empty-rule-{}.modality",
            std::process::id()
        ));
        let proposed_rule_path = std::env::temp_dir().join(format!(
            "modality-existing-model-empty-proposed-rule-{}.modality",
            std::process::id()
        ));
        std::fs::write(
            &existing_path,
            format!(
                "{}\n\nformula previous_rule {{\nalways([<+APPROVE>] true)\n}}\n",
                modality_lang::print_model(&model)
            ),
        )
        .unwrap();
        std::fs::write(&proposed_rule_path, "\n\n").unwrap();

        let mut opts = default_test_opts();
        opts.existing_model = Some(existing_path.clone());
        opts.proposed_rule = Some(proposed_rule_path.clone());

        let err = run_existing_model_synthesis(&opts).unwrap_err();
        std::fs::remove_file(existing_path).unwrap();
        std::fs::remove_file(proposed_rule_path).unwrap();

        assert!(err
            .to_string()
            .contains("--verify requires every input formula to parse"));
        assert!(err.to_string().contains("<empty>"));
    }

    #[test]
    fn existing_model_mode_rejects_empty_proposed_formula() {
        let model_formulas = parse_formula_strings(&["always([<+APPROVE>] true)".to_string()]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &model_formulas);
        let existing_path = std::env::temp_dir().join(format!(
            "modality-existing-model-empty-inline-{}.modality",
            std::process::id()
        ));
        std::fs::write(
            &existing_path,
            format!(
                "{}\n\nformula previous_rule {{\nalways([<+APPROVE>] true)\n}}\n",
                modality_lang::print_model(&model)
            ),
        )
        .unwrap();

        let mut opts = default_test_opts();
        opts.existing_model = Some(existing_path.clone());
        opts.proposed_formula = Some("   \n".to_string());

        let err = run_existing_model_synthesis(&opts).unwrap_err();
        std::fs::remove_file(existing_path).unwrap();

        assert!(err
            .to_string()
            .contains("--verify requires every input formula to parse"));
        assert!(err.to_string().contains("<empty>"));
    }

    #[test]
    fn existing_model_mode_reports_missing_proposed_rule_path() {
        let model_formulas = parse_formula_strings(&["always([<+APPROVE>] true)".to_string()]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &model_formulas);
        let existing_path = std::env::temp_dir().join(format!(
            "modality-existing-model-missing-rule-{}.modality",
            std::process::id()
        ));
        let proposed_rule_path = std::env::temp_dir().join(format!(
            "modality-existing-model-missing-proposed-rule-{}.modality",
            std::process::id()
        ));
        std::fs::write(
            &existing_path,
            format!(
                "{}\n\nformula previous_rule {{\nalways([<+APPROVE>] true)\n}}\n",
                modality_lang::print_model(&model)
            ),
        )
        .unwrap();

        let mut opts = default_test_opts();
        opts.existing_model = Some(existing_path.clone());
        opts.proposed_rule = Some(proposed_rule_path.clone());

        let err = run_existing_model_synthesis(&opts).unwrap_err();
        std::fs::remove_file(existing_path).unwrap();

        let message = err.to_string();
        assert!(message.contains("Failed to read proposed rule file"));
        assert!(message.contains(&proposed_rule_path.display().to_string()));
    }

    #[test]
    fn existing_model_mode_reports_missing_existing_model_path() {
        let existing_path = std::env::temp_dir().join(format!(
            "modality-existing-model-missing-existing-{}.modality",
            std::process::id()
        ));

        let mut opts = default_test_opts();
        opts.existing_model = Some(existing_path.clone());
        opts.proposed_formula = Some("always([<+APPROVE>] true)".to_string());

        let err = run_existing_model_synthesis(&opts).unwrap_err();

        let message = err.to_string();
        assert!(message.contains("Failed to read existing model file"));
        assert!(message.contains(&existing_path.display().to_string()));
    }

    #[test]
    fn output_write_errors_include_target_path() {
        let output_path = std::env::temp_dir().join(format!(
            "modality-synthesize-output-directory-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&output_path).unwrap();

        let err = write_or_print_model("model Contract {\n  initial idle\n}\n", Some(&output_path))
            .unwrap_err();
        std::fs::remove_dir(&output_path).unwrap();

        let message = err.to_string();
        assert!(message.contains("Failed to write synthesized model"));
        assert!(message.contains(&output_path.display().to_string()));
    }

    #[test]
    fn existing_model_mode_writes_satisfied_existing_model_output() {
        let model_formulas = parse_formula_strings(&["always([<+APPROVE>] true)".to_string()]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &model_formulas);
        let existing_path = std::env::temp_dir().join(format!(
            "modality-existing-model-replacement-{}.modality",
            std::process::id()
        ));
        let output_path = std::env::temp_dir().join(format!(
            "modality-existing-model-output-{}.modality",
            std::process::id()
        ));
        std::fs::write(
            &existing_path,
            format!(
                "{}\n\nformula previous_rule {{\nalways([<+APPROVE>] true)\n}}\n",
                modality_lang::print_model(&model)
            ),
        )
        .unwrap();

        let mut opts = default_test_opts();
        opts.existing_model = Some(existing_path.clone());
        opts.proposed_formula =
            Some("[+APPROVE] true -> <+signed_by(/users/reviewer.id)> true".to_string());
        opts.output = Some(output_path.clone());

        run_existing_model_synthesis(&opts).unwrap();

        let output = std::fs::read_to_string(&output_path).unwrap();
        std::fs::remove_file(existing_path).unwrap();
        std::fs::remove_file(output_path).unwrap();

        assert!(output.contains("model Contract"));
        assert!(output.contains("+APPROVE"));
        assert!(output.contains("formula previous_rule"));
        assert!(output.contains("formula proposed_formula"));
        assert!(output.contains("signed_by(/users/reviewer.id)"));
        assert_eq!(
            modality_lang::parse_all_formulas_content_lalrpop(&output)
                .unwrap()
                .len(),
            2
        );
    }

    #[test]
    fn existing_model_mode_writes_json_with_formula_declarations() {
        let model_formulas = parse_formula_strings(&["always([<+APPROVE>] true)".to_string()]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &model_formulas);
        let existing_path = std::env::temp_dir().join(format!(
            "modality-existing-model-json-{}.modality",
            std::process::id()
        ));
        let output_path = std::env::temp_dir().join(format!(
            "modality-existing-model-json-output-{}.json",
            std::process::id()
        ));
        std::fs::write(
            &existing_path,
            format!(
                "{}\n\nformula previous_rule {{\nalways([<+APPROVE>] true)\n}}\n",
                modality_lang::print_model(&model)
            ),
        )
        .unwrap();

        let mut opts = default_test_opts();
        opts.existing_model = Some(existing_path.clone());
        opts.proposed_formula =
            Some("[+APPROVE] true -> <+signed_by(/users/reviewer.id)> true".to_string());
        opts.output = Some(output_path.clone());
        opts.format = "json".to_string();

        run_existing_model_synthesis(&opts).unwrap();

        let output = std::fs::read_to_string(&output_path).unwrap();
        std::fs::remove_file(existing_path).unwrap();
        std::fs::remove_file(output_path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

        assert_eq!(parsed["model"]["name"], "Contract");
        assert_eq!(parsed["formula_declarations"].as_array().unwrap().len(), 2);
        assert!(parsed["formula_declarations"][0]
            .as_str()
            .unwrap()
            .contains("formula previous_rule"));
        assert!(parsed["formula_declarations"][1]
            .as_str()
            .unwrap()
            .contains("formula proposed_formula"));
    }

    #[test]
    fn existing_model_mode_writes_replacement_candidate_output() {
        let model_formulas = parse_formula_strings(&["always([<+APPROVE>] true)".to_string()]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &model_formulas);
        let existing_path = std::env::temp_dir().join(format!(
            "modality-existing-model-candidate-{}.modality",
            std::process::id()
        ));
        let output_path = std::env::temp_dir().join(format!(
            "modality-existing-model-candidate-output-{}.modality",
            std::process::id()
        ));
        let proposed_rule_path = std::env::temp_dir().join(format!(
            "modality-existing-model-proposed-{}.modality",
            std::process::id()
        ));
        std::fs::write(
            &existing_path,
            format!(
                "{}\n\nformula previous_rule {{\nalways([<+APPROVE>] true)\n}}\n",
                modality_lang::print_model(&model)
            ),
        )
        .unwrap();
        std::fs::write(
            &proposed_rule_path,
            "formula delivery_required {\nalways([<+DELIVER>] true)\n}\n",
        )
        .unwrap();

        let mut opts = default_test_opts();
        opts.existing_model = Some(existing_path.clone());
        opts.proposed_rule = Some(proposed_rule_path.clone());
        opts.output = Some(output_path.clone());

        run_existing_model_synthesis(&opts).unwrap();

        let output = std::fs::read_to_string(&output_path).unwrap();
        std::fs::remove_file(existing_path).unwrap();
        std::fs::remove_file(proposed_rule_path).unwrap();
        std::fs::remove_file(output_path).unwrap();

        assert!(output.contains("model ContractCandidate"));
        assert!(output.contains("+APPROVE"));
        assert!(output.contains("+DELIVER"));
        assert!(output.contains("formula previous_rule"));
        assert!(output.contains("formula delivery_required"));
        assert_eq!(
            modality_lang::parse_all_formulas_content_lalrpop(&output)
                .unwrap()
                .len(),
            2
        );
    }

    #[tokio::test]
    async fn existing_model_mode_writes_json_replacement_candidate() {
        let model_formulas = parse_formula_strings(&["always([<+APPROVE>] true)".to_string()]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &model_formulas);
        let existing_path = std::env::temp_dir().join(format!(
            "modality-existing-model-json-candidate-{}.modality",
            std::process::id()
        ));
        let output_path = std::env::temp_dir().join(format!(
            "modality-existing-model-json-candidate-output-{}.json",
            std::process::id()
        ));
        let proposed_rule_path = std::env::temp_dir().join(format!(
            "modality-existing-model-json-proposed-{}.modality",
            std::process::id()
        ));
        std::fs::write(
            &existing_path,
            format!(
                "{}\n\nformula previous_rule {{\nalways([<+APPROVE>] true)\n}}\n",
                modality_lang::print_model(&model)
            ),
        )
        .unwrap();
        std::fs::write(
            &proposed_rule_path,
            "formula delivery_required {\nalways([<+DELIVER>] true)\n}\n",
        )
        .unwrap();

        let mut opts = default_test_opts();
        opts.existing_model = Some(existing_path.clone());
        opts.proposed_rule = Some(proposed_rule_path.clone());
        opts.output = Some(output_path.clone());
        opts.format = "json".to_string();

        run(&opts).await.unwrap();

        let output = std::fs::read_to_string(&output_path).unwrap();
        std::fs::remove_file(existing_path).unwrap();
        std::fs::remove_file(proposed_rule_path).unwrap();
        std::fs::remove_file(output_path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

        assert_eq!(parsed["model"]["name"], "ContractCandidate");
        assert_eq!(parsed["formula_declarations"].as_array().unwrap().len(), 2);
        assert!(parsed["formula_declarations"][0]
            .as_str()
            .unwrap()
            .contains("formula previous_rule"));
        assert!(parsed["formula_declarations"][1]
            .as_str()
            .unwrap()
            .contains("formula delivery_required"));

        let action_names = parsed["model"]["parts"][0]["transitions"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|transition| transition["properties"].as_array().unwrap())
            .map(|property| property["name"].as_str().unwrap())
            .collect::<Vec<_>>();

        assert!(action_names.contains(&"APPROVE"));
        assert!(action_names.contains(&"DELIVER"));
    }

    #[tokio::test]
    async fn existing_model_mode_writes_json_candidate_for_bare_rule_file() {
        let model_formulas = parse_formula_strings(&["always([<+APPROVE>] true)".to_string()]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &model_formulas);
        let existing_path = std::env::temp_dir().join(format!(
            "modality-existing-model-json-bare-rule-candidate-{}.modality",
            std::process::id()
        ));
        let proposed_rule_path = std::env::temp_dir().join(format!(
            "modality-existing-model-json-bare-rule-proposed-{}.modality",
            std::process::id()
        ));
        let output_path = std::env::temp_dir().join(format!(
            "modality-existing-model-json-bare-rule-output-{}.json",
            std::process::id()
        ));
        std::fs::write(
            &existing_path,
            format!(
                "{}\n\nformula previous_rule {{\nalways([<+APPROVE>] true)\n}}\n",
                modality_lang::print_model(&model)
            ),
        )
        .unwrap();
        std::fs::write(&proposed_rule_path, "always([<+DELIVER>] true)\n").unwrap();

        let mut opts = default_test_opts();
        opts.existing_model = Some(existing_path.clone());
        opts.proposed_rule = Some(proposed_rule_path.clone());
        opts.output = Some(output_path.clone());
        opts.format = "json".to_string();

        run(&opts).await.unwrap();

        let output = std::fs::read_to_string(&output_path).unwrap();
        std::fs::remove_file(existing_path).unwrap();
        std::fs::remove_file(proposed_rule_path).unwrap();
        std::fs::remove_file(output_path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

        assert_eq!(parsed["model"]["name"], "ContractCandidate");
        assert_eq!(parsed["formula_declarations"].as_array().unwrap().len(), 2);
        assert!(parsed["formula_declarations"][0]
            .as_str()
            .unwrap()
            .contains("formula previous_rule"));
        assert!(parsed["formula_declarations"][1]
            .as_str()
            .unwrap()
            .contains("formula proposed_rule"));
        assert!(parsed["formula_declarations"][1]
            .as_str()
            .unwrap()
            .contains("always([<+DELIVER>] true)"));

        let action_names = parsed["model"]["parts"][0]["transitions"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|transition| transition["properties"].as_array().unwrap())
            .map(|property| property["name"].as_str().unwrap())
            .collect::<Vec<_>>();

        assert!(action_names.contains(&"APPROVE"));
        assert!(action_names.contains(&"DELIVER"));
    }

    #[tokio::test]
    async fn existing_model_mode_writes_candidate_for_bare_rule_file() {
        let model_formulas = parse_formula_strings(&["always([<+APPROVE>] true)".to_string()]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &model_formulas);
        let existing_path = std::env::temp_dir().join(format!(
            "modality-existing-model-bare-rule-candidate-{}.modality",
            std::process::id()
        ));
        let proposed_rule_path = std::env::temp_dir().join(format!(
            "modality-existing-model-bare-rule-proposed-{}.modality",
            std::process::id()
        ));
        let output_path = std::env::temp_dir().join(format!(
            "modality-existing-model-bare-rule-output-{}.modality",
            std::process::id()
        ));
        std::fs::write(
            &existing_path,
            format!(
                "{}\n\nformula previous_rule {{\nalways([<+APPROVE>] true)\n}}\n",
                modality_lang::print_model(&model)
            ),
        )
        .unwrap();
        std::fs::write(&proposed_rule_path, "always([<+DELIVER>] true)\n").unwrap();

        let mut opts = default_test_opts();
        opts.existing_model = Some(existing_path.clone());
        opts.proposed_rule = Some(proposed_rule_path.clone());
        opts.output = Some(output_path.clone());

        run(&opts).await.unwrap();

        let output = std::fs::read_to_string(&output_path).unwrap();
        std::fs::remove_file(existing_path).unwrap();
        std::fs::remove_file(proposed_rule_path).unwrap();
        std::fs::remove_file(output_path).unwrap();

        assert!(output.contains("model ContractCandidate"));
        assert!(output.contains("+APPROVE"));
        assert!(output.contains("+DELIVER"));
        assert!(output.contains("formula previous_rule"));
        assert!(output.contains("formula proposed_rule"));
        assert_eq!(
            modality_lang::parse_all_models_content_lalrpop(&output)
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            modality_lang::parse_all_formulas_content_lalrpop(&output)
                .unwrap()
                .len(),
            2
        );
    }

    #[tokio::test]
    async fn existing_model_mode_writes_json_candidate_for_inline_formula() {
        let model_formulas = parse_formula_strings(&["always([<+APPROVE>] true)".to_string()]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &model_formulas);
        let existing_path = std::env::temp_dir().join(format!(
            "modality-existing-model-json-inline-candidate-{}.modality",
            std::process::id()
        ));
        let output_path = std::env::temp_dir().join(format!(
            "modality-existing-model-json-inline-candidate-output-{}.json",
            std::process::id()
        ));
        std::fs::write(
            &existing_path,
            format!(
                "{}\n\nformula previous_rule {{\nalways([<+APPROVE>] true)\n}}\n",
                modality_lang::print_model(&model)
            ),
        )
        .unwrap();

        let mut opts = default_test_opts();
        opts.existing_model = Some(existing_path.clone());
        opts.proposed_formula = Some("always([<+DELIVER>] true)".to_string());
        opts.output = Some(output_path.clone());
        opts.format = "json".to_string();

        run(&opts).await.unwrap();

        let output = std::fs::read_to_string(&output_path).unwrap();
        std::fs::remove_file(existing_path).unwrap();
        std::fs::remove_file(output_path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

        assert_eq!(parsed["model"]["name"], "ContractCandidate");
        assert_eq!(parsed["formula_declarations"].as_array().unwrap().len(), 2);
        assert!(parsed["formula_declarations"][0]
            .as_str()
            .unwrap()
            .contains("formula previous_rule"));
        assert!(parsed["formula_declarations"][1]
            .as_str()
            .unwrap()
            .contains("formula proposed_formula"));

        let action_names = parsed["model"]["parts"][0]["transitions"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|transition| transition["properties"].as_array().unwrap())
            .map(|property| property["name"].as_str().unwrap())
            .collect::<Vec<_>>();

        assert!(action_names.contains(&"APPROVE"));
        assert!(action_names.contains(&"DELIVER"));
    }

    #[tokio::test]
    async fn existing_model_mode_writes_candidate_for_inline_formula() {
        let model_formulas = parse_formula_strings(&["always([<+APPROVE>] true)".to_string()]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &model_formulas);
        let existing_path = std::env::temp_dir().join(format!(
            "modality-existing-model-inline-candidate-{}.modality",
            std::process::id()
        ));
        let output_path = std::env::temp_dir().join(format!(
            "modality-existing-model-inline-candidate-output-{}.modality",
            std::process::id()
        ));
        std::fs::write(
            &existing_path,
            format!(
                "{}\n\nformula previous_rule {{\nalways([<+APPROVE>] true)\n}}\n",
                modality_lang::print_model(&model)
            ),
        )
        .unwrap();

        let mut opts = default_test_opts();
        opts.existing_model = Some(existing_path.clone());
        opts.proposed_formula = Some("always([<+DELIVER>] true)".to_string());
        opts.output = Some(output_path.clone());

        run(&opts).await.unwrap();

        let output = std::fs::read_to_string(&output_path).unwrap();
        std::fs::remove_file(existing_path).unwrap();
        std::fs::remove_file(output_path).unwrap();

        assert!(output.contains("model ContractCandidate"));
        assert!(output.contains("+APPROVE"));
        assert!(output.contains("+DELIVER"));
        assert!(output.contains("formula previous_rule"));
        assert!(output.contains("formula proposed_formula"));
        assert_eq!(
            modality_lang::parse_all_models_content_lalrpop(&output)
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            modality_lang::parse_all_formulas_content_lalrpop(&output)
                .unwrap()
                .len(),
            2
        );
    }

    #[tokio::test]
    async fn existing_model_mode_writes_satisfied_inline_formula_output() {
        let model_formulas = parse_formula_strings(&["always([<+APPROVE>] true)".to_string()]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &model_formulas);
        let existing_path = std::env::temp_dir().join(format!(
            "modality-existing-model-satisfied-inline-{}.modality",
            std::process::id()
        ));
        let output_path = std::env::temp_dir().join(format!(
            "modality-existing-model-satisfied-inline-output-{}.modality",
            std::process::id()
        ));
        std::fs::write(
            &existing_path,
            format!(
                "{}\n\nformula previous_rule {{\nalways([<+APPROVE>] true)\n}}\n",
                modality_lang::print_model(&model)
            ),
        )
        .unwrap();

        let mut opts = default_test_opts();
        opts.existing_model = Some(existing_path.clone());
        opts.proposed_formula = Some("always([<+APPROVE>] true)".to_string());
        opts.output = Some(output_path.clone());

        run(&opts).await.unwrap();

        let output = std::fs::read_to_string(&output_path).unwrap();
        std::fs::remove_file(existing_path).unwrap();
        std::fs::remove_file(output_path).unwrap();

        assert!(output.contains("model Contract"));
        assert!(!output.contains("model ContractCandidate"));
        assert!(output.contains("+APPROVE"));
        assert!(output.contains("formula previous_rule"));
        assert!(output.contains("formula proposed_formula"));
        assert_eq!(
            modality_lang::parse_all_models_content_lalrpop(&output)
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            modality_lang::parse_all_formulas_content_lalrpop(&output)
                .unwrap()
                .len(),
            2
        );
    }

    #[tokio::test]
    async fn existing_model_mode_writes_json_for_satisfied_inline_formula() {
        let model_formulas = parse_formula_strings(&["always([<+APPROVE>] true)".to_string()]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &model_formulas);
        let existing_path = std::env::temp_dir().join(format!(
            "modality-existing-model-json-satisfied-{}.modality",
            std::process::id()
        ));
        let output_path = std::env::temp_dir().join(format!(
            "modality-existing-model-json-satisfied-output-{}.json",
            std::process::id()
        ));
        std::fs::write(
            &existing_path,
            format!(
                "{}\n\nformula previous_rule {{\nalways([<+APPROVE>] true)\n}}\n",
                modality_lang::print_model(&model)
            ),
        )
        .unwrap();

        let mut opts = default_test_opts();
        opts.existing_model = Some(existing_path.clone());
        opts.proposed_formula = Some("always([<+APPROVE>] true)".to_string());
        opts.output = Some(output_path.clone());
        opts.format = "json".to_string();

        run(&opts).await.unwrap();

        let output = std::fs::read_to_string(&output_path).unwrap();
        std::fs::remove_file(existing_path).unwrap();
        std::fs::remove_file(output_path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

        assert_eq!(parsed["model"]["name"], "Contract");
        assert_eq!(parsed["formula_declarations"].as_array().unwrap().len(), 2);
        assert!(parsed["formula_declarations"][0]
            .as_str()
            .unwrap()
            .contains("formula previous_rule"));
        assert!(parsed["formula_declarations"][1]
            .as_str()
            .unwrap()
            .contains("formula proposed_formula"));
    }

    #[tokio::test]
    async fn existing_model_mode_writes_satisfied_rule_file_output() {
        let model_formulas = parse_formula_strings(&["always([<+APPROVE>] true)".to_string()]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &model_formulas);
        let existing_path = std::env::temp_dir().join(format!(
            "modality-existing-model-satisfied-rule-{}.modality",
            std::process::id()
        ));
        let proposed_rule_path = std::env::temp_dir().join(format!(
            "modality-existing-model-satisfied-proposed-rule-{}.modality",
            std::process::id()
        ));
        let output_path = std::env::temp_dir().join(format!(
            "modality-existing-model-satisfied-rule-output-{}.modality",
            std::process::id()
        ));
        std::fs::write(
            &existing_path,
            format!(
                "{}\n\nformula previous_rule {{\nalways([<+APPROVE>] true)\n}}\n",
                modality_lang::print_model(&model)
            ),
        )
        .unwrap();
        std::fs::write(
            &proposed_rule_path,
            "formula approval_required {\nalways([<+APPROVE>] true)\n}\n",
        )
        .unwrap();

        let mut opts = default_test_opts();
        opts.existing_model = Some(existing_path.clone());
        opts.proposed_rule = Some(proposed_rule_path.clone());
        opts.output = Some(output_path.clone());

        run(&opts).await.unwrap();

        let output = std::fs::read_to_string(&output_path).unwrap();
        std::fs::remove_file(existing_path).unwrap();
        std::fs::remove_file(proposed_rule_path).unwrap();
        std::fs::remove_file(output_path).unwrap();

        assert!(output.contains("model Contract"));
        assert!(!output.contains("model ContractCandidate"));
        assert!(output.contains("+APPROVE"));
        assert!(output.contains("formula previous_rule"));
        assert!(output.contains("formula approval_required"));
        assert_eq!(
            modality_lang::parse_all_models_content_lalrpop(&output)
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            modality_lang::parse_all_formulas_content_lalrpop(&output)
                .unwrap()
                .len(),
            2
        );
    }

    #[tokio::test]
    async fn existing_model_mode_writes_json_for_satisfied_rule_file() {
        let model_formulas = parse_formula_strings(&["always([<+APPROVE>] true)".to_string()]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &model_formulas);
        let existing_path = std::env::temp_dir().join(format!(
            "modality-existing-model-json-satisfied-rule-{}.modality",
            std::process::id()
        ));
        let proposed_rule_path = std::env::temp_dir().join(format!(
            "modality-existing-model-json-satisfied-proposed-rule-{}.modality",
            std::process::id()
        ));
        let output_path = std::env::temp_dir().join(format!(
            "modality-existing-model-json-satisfied-rule-output-{}.json",
            std::process::id()
        ));
        std::fs::write(
            &existing_path,
            format!(
                "{}\n\nformula previous_rule {{\nalways([<+APPROVE>] true)\n}}\n",
                modality_lang::print_model(&model)
            ),
        )
        .unwrap();
        std::fs::write(
            &proposed_rule_path,
            "formula approval_required {\nalways([<+APPROVE>] true)\n}\n",
        )
        .unwrap();

        let mut opts = default_test_opts();
        opts.existing_model = Some(existing_path.clone());
        opts.proposed_rule = Some(proposed_rule_path.clone());
        opts.output = Some(output_path.clone());
        opts.format = "json".to_string();

        run(&opts).await.unwrap();

        let output = std::fs::read_to_string(&output_path).unwrap();
        std::fs::remove_file(existing_path).unwrap();
        std::fs::remove_file(proposed_rule_path).unwrap();
        std::fs::remove_file(output_path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

        assert_eq!(parsed["model"]["name"], "Contract");
        assert_eq!(parsed["formula_declarations"].as_array().unwrap().len(), 2);
        assert!(parsed["formula_declarations"][0]
            .as_str()
            .unwrap()
            .contains("formula previous_rule"));
        assert!(parsed["formula_declarations"][1]
            .as_str()
            .unwrap()
            .contains("formula approval_required"));
    }

    #[tokio::test]
    async fn existing_model_mode_writes_json_candidate_for_multiple_rule_formulas() {
        let model_formulas = parse_formula_strings(&["always([<+APPROVE>] true)".to_string()]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &model_formulas);
        let existing_path = std::env::temp_dir().join(format!(
            "modality-existing-model-json-multiple-rule-{}.modality",
            std::process::id()
        ));
        let proposed_rule_path = std::env::temp_dir().join(format!(
            "modality-existing-model-json-multiple-proposed-rule-{}.modality",
            std::process::id()
        ));
        let output_path = std::env::temp_dir().join(format!(
            "modality-existing-model-json-multiple-rule-output-{}.json",
            std::process::id()
        ));
        std::fs::write(
            &existing_path,
            format!(
                "{}\n\nformula previous_rule {{\nalways([<+APPROVE>] true)\n}}\n",
                modality_lang::print_model(&model)
            ),
        )
        .unwrap();
        std::fs::write(
            &proposed_rule_path,
            "formula delivery_required {\nalways([<+DELIVER>] true)\n}\n\nformula payment_required {\nalways([<+PAY>] true)\n}\n",
        )
        .unwrap();

        let mut opts = default_test_opts();
        opts.existing_model = Some(existing_path.clone());
        opts.proposed_rule = Some(proposed_rule_path.clone());
        opts.output = Some(output_path.clone());
        opts.format = "json".to_string();

        run(&opts).await.unwrap();

        let output = std::fs::read_to_string(&output_path).unwrap();
        std::fs::remove_file(existing_path).unwrap();
        std::fs::remove_file(proposed_rule_path).unwrap();
        std::fs::remove_file(output_path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

        assert_eq!(parsed["model"]["name"], "ContractCandidate");
        let declarations = parsed["formula_declarations"].as_array().unwrap();
        assert_eq!(declarations.len(), 3);
        assert!(declarations[0]
            .as_str()
            .unwrap()
            .contains("formula previous_rule"));
        assert!(declarations[1]
            .as_str()
            .unwrap()
            .contains("formula delivery_required"));
        assert!(declarations[2]
            .as_str()
            .unwrap()
            .contains("formula payment_required"));

        let action_names = parsed["model"]["parts"][0]["transitions"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|transition| transition["properties"].as_array().unwrap())
            .map(|property| property["name"].as_str().unwrap())
            .collect::<Vec<_>>();

        assert!(action_names.contains(&"APPROVE"));
        assert!(action_names.contains(&"DELIVER"));
        assert!(action_names.contains(&"PAY"));
    }

    #[tokio::test]
    async fn existing_model_mode_writes_candidate_for_multiple_rule_formulas() {
        let model_formulas = parse_formula_strings(&["always([<+APPROVE>] true)".to_string()]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &model_formulas);
        let existing_path = std::env::temp_dir().join(format!(
            "modality-existing-model-multiple-rule-{}.modality",
            std::process::id()
        ));
        let proposed_rule_path = std::env::temp_dir().join(format!(
            "modality-existing-model-multiple-proposed-rule-{}.modality",
            std::process::id()
        ));
        let output_path = std::env::temp_dir().join(format!(
            "modality-existing-model-multiple-rule-output-{}.modality",
            std::process::id()
        ));
        std::fs::write(
            &existing_path,
            format!(
                "{}\n\nformula previous_rule {{\nalways([<+APPROVE>] true)\n}}\n",
                modality_lang::print_model(&model)
            ),
        )
        .unwrap();
        std::fs::write(
            &proposed_rule_path,
            "formula delivery_required {\nalways([<+DELIVER>] true)\n}\n\nformula payment_required {\nalways([<+PAY>] true)\n}\n",
        )
        .unwrap();

        let mut opts = default_test_opts();
        opts.existing_model = Some(existing_path.clone());
        opts.proposed_rule = Some(proposed_rule_path.clone());
        opts.output = Some(output_path.clone());

        run(&opts).await.unwrap();

        let output = std::fs::read_to_string(&output_path).unwrap();
        std::fs::remove_file(existing_path).unwrap();
        std::fs::remove_file(proposed_rule_path).unwrap();
        std::fs::remove_file(output_path).unwrap();

        assert!(output.contains("model ContractCandidate"));
        assert!(output.contains("+APPROVE"));
        assert!(output.contains("+DELIVER"));
        assert!(output.contains("+PAY"));
        assert!(output.contains("formula previous_rule"));
        assert!(output.contains("formula delivery_required"));
        assert!(output.contains("formula payment_required"));
        assert_eq!(
            modality_lang::parse_all_formulas_content_lalrpop(&output)
                .unwrap()
                .len(),
            3
        );
    }

    #[test]
    fn formula_declaration_blocks_preserve_multiple_formula_sources() {
        let content = r#"
model Contract {
  part flow {
    idle --> idle: +APPROVE
  }
}

formula first_rule {
always([<+APPROVE>] true)
}

formula second_rule { eventually(<+DELIVER> true) }
"#;

        let declarations = formula_declaration_blocks(content);

        assert_eq!(declarations.len(), 2);
        assert!(declarations[0].contains("formula first_rule"));
        assert!(declarations[1].contains("formula second_rule"));
    }

    #[test]
    fn llm_response_loader_rejects_inline_and_file_together() {
        let response = "formula generated { true }".to_string();
        let path = PathBuf::from("response.md");

        let err = load_llm_response(Some(&response), Some(&path)).unwrap_err();

        assert!(err.to_string().contains("--llm-response-file"));
    }

    #[test]
    fn llm_response_loader_reports_missing_file_path() {
        let path = std::env::temp_dir().join(format!(
            "modality-synthesize-missing-response-{}.md",
            std::process::id()
        ));

        let err = load_llm_response(None, Some(&path)).unwrap_err();

        let message = err.to_string();
        assert!(message.contains("Failed to read LLM response file"));
        assert!(message.contains(&path.display().to_string()));
    }

    #[test]
    fn llm_response_file_round_trips_to_verified_synthesis() {
        let path = std::env::temp_dir().join(format!(
            "modality-synthesize-response-{}.md",
            std::process::id()
        ));
        std::fs::write(
            &path,
            r#"
```modality
F1: formula generated_1 {
always([<+APPROVE>] true)
}
F2: formula generated_2 {
[+APPROVE] true -> <+signed_by(/users/reviewer.id)> true
}
```
"#,
        )
        .unwrap();

        let response = load_llm_response(None, Some(&path)).unwrap().unwrap();
        std::fs::remove_file(&path).unwrap();

        let formula_strings = modality_lang::llm_synthesis::parse_llm_response(&response);
        assert_eq!(formula_strings.len(), 2);

        ensure_all_formula_strings_parsed(&formula_strings).unwrap();
        let formulas = parse_formula_strings(&formula_strings);
        assert_eq!(formulas.len(), 2);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[tokio::test]
    async fn llm_response_file_verify_writes_checked_model() {
        let response_path = std::env::temp_dir().join(format!(
            "modality-synthesize-response-run-{}.md",
            std::process::id()
        ));
        let output_path = std::env::temp_dir().join(format!(
            "modality-synthesize-response-output-{}.modality",
            std::process::id()
        ));
        std::fs::write(
            &response_path,
            r#"
```modality
formula generated_1 {
lfp(X, ((<+REVIEW> true) & ((<+WAIT> true) & <>((X)))) | (<+APPROVE> true))
}

formula generated_2 {
gfp(X, []((X)) & ([<+ARCHIVE>] true))
}
```
"#,
        )
        .unwrap();

        let mut opts = default_test_opts();
        opts.llm_response_file = Some(response_path.clone());
        opts.output = Some(output_path.clone());
        opts.verify = true;

        run(&opts).await.unwrap();

        let output = std::fs::read_to_string(&output_path).unwrap();
        std::fs::remove_file(response_path).unwrap();
        std::fs::remove_file(output_path).unwrap();

        let models = modality_lang::parse_all_models_content_lalrpop(&output).unwrap();
        assert_eq!(models.len(), 1);
        assert!(output.contains("model Contract"));
        assert!(output.contains("+APPROVE"));
        assert!(output.contains("+REVIEW"));
        assert!(output.contains("+WAIT"));
        assert!(output.contains("+ARCHIVE"));
    }

    #[tokio::test]
    async fn inline_llm_response_reports_expected_formula_shapes_when_empty() {
        let mut opts = default_test_opts();
        opts.llm_response =
            Some("Here is a summary, but no concrete Modality formulas.".to_string());

        let err = run(&opts).await.unwrap_err();

        let message = err.to_string();
        assert!(message.contains("No formulas found in LLM response"));
        assert!(message.contains("expected Modality formula declarations"));
        assert!(message.contains("F1:/F2: formula lines"));
    }

    #[tokio::test]
    async fn llm_response_file_verify_writes_json_model() {
        let response_path = std::env::temp_dir().join(format!(
            "modality-synthesize-response-json-run-{}.md",
            std::process::id()
        ));
        let output_path = std::env::temp_dir().join(format!(
            "modality-synthesize-response-output-{}.json",
            std::process::id()
        ));
        std::fs::write(
            &response_path,
            r#"
```modality
formula generated_1 {
lfp(X, ((<+REVIEW> true) & ((<+WAIT> true) & <>((X)))) | (<+APPROVE> true))
}

formula generated_2 {
gfp(X, []((X)) & ([<+ARCHIVE>] true))
}
```
"#,
        )
        .unwrap();

        let mut opts = default_test_opts();
        opts.llm_response_file = Some(response_path.clone());
        opts.output = Some(output_path.clone());
        opts.verify = true;
        opts.format = "json".to_string();

        run(&opts).await.unwrap();

        let output = std::fs::read_to_string(&output_path).unwrap();
        std::fs::remove_file(response_path).unwrap();
        std::fs::remove_file(output_path).unwrap();

        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert_eq!(parsed["name"], "Contract");
        let action_names = parsed["parts"][0]["transitions"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|transition| transition["properties"].as_array().unwrap())
            .map(|property| property["name"].as_str().unwrap())
            .collect::<Vec<_>>();

        assert!(action_names.contains(&"APPROVE"));
        assert!(action_names.contains(&"REVIEW"));
        assert!(action_names.contains(&"WAIT"));
        assert!(action_names.contains(&"ARCHIVE"));
    }

    #[tokio::test]
    async fn inline_llm_response_verify_writes_checked_model() {
        let output_path = std::env::temp_dir().join(format!(
            "modality-synthesize-inline-response-output-{}.modality",
            std::process::id()
        ));

        let mut opts = default_test_opts();
        opts.llm_response = Some(
            r#"
```modality
formula generated_1 {
lfp(X, ((<+REVIEW> true) & ((<+WAIT> true) & <>((X)))) | (<+APPROVE> true))
}

formula generated_2 {
gfp(X, []((X)) & ([<+ARCHIVE>] true))
}
```
"#
            .to_string(),
        );
        opts.output = Some(output_path.clone());
        opts.verify = true;

        run(&opts).await.unwrap();

        let output = std::fs::read_to_string(&output_path).unwrap();
        std::fs::remove_file(output_path).unwrap();

        let models = modality_lang::parse_all_models_content_lalrpop(&output).unwrap();
        assert_eq!(models.len(), 1);
        assert!(output.contains("model Contract"));
        assert!(output.contains("+APPROVE"));
        assert!(output.contains("+REVIEW"));
        assert!(output.contains("+WAIT"));
        assert!(output.contains("+ARCHIVE"));
    }

    #[tokio::test]
    async fn inline_llm_response_verify_writes_json_model() {
        let output_path = std::env::temp_dir().join(format!(
            "modality-synthesize-inline-response-output-{}.json",
            std::process::id()
        ));

        let mut opts = default_test_opts();
        opts.llm_response = Some(
            r#"
```modality
formula generated_1 {
lfp(X, ((<+REVIEW> true) & ((<+WAIT> true) & <>((X)))) | (<+APPROVE> true))
}

formula generated_2 {
gfp(X, []((X)) & ([<+ARCHIVE>] true))
}
```
"#
            .to_string(),
        );
        opts.output = Some(output_path.clone());
        opts.verify = true;
        opts.format = "json".to_string();

        run(&opts).await.unwrap();

        let output = std::fs::read_to_string(&output_path).unwrap();
        std::fs::remove_file(output_path).unwrap();

        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert_eq!(parsed["name"], "Contract");
        let action_names = parsed["parts"][0]["transitions"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|transition| transition["properties"].as_array().unwrap())
            .map(|property| property["name"].as_str().unwrap())
            .collect::<Vec<_>>();

        assert!(action_names.contains(&"APPROVE"));
        assert!(action_names.contains(&"REVIEW"));
        assert!(action_names.contains(&"WAIT"));
        assert!(action_names.contains(&"ARCHIVE"));
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
    fn verify_synthesized_model_accepts_compound_eventual_body_required_actions() {
        let formulas = parse_formula_strings(&[
            "[+RELEASE] true -> eventually((<+DEPOSIT> true & <+DELIVER> true))".to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_compound_eventual_body_committed_actions() {
        let formulas = parse_formula_strings(&[
            "[+RELEASE] true -> eventually(([<+DEPOSIT>] true & [<+DELIVER>] true))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_next_temporal_candidate_action() {
        let formulas = parse_formula_strings(&["next(<+APPROVE> true)".to_string()]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_until_temporal_candidate_actions() {
        let formulas =
            parse_formula_strings(&["<+WAIT> true until <+APPROVE> true".to_string()]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_mixed_permissive_committed_alternatives() {
        let formulas =
            parse_formula_strings(&["<+APPROVE> true | [<+REJECT>] true".to_string()]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_next_mixed_temporal_alternatives() {
        let formulas =
            parse_formula_strings(&["next((<+APPROVE> true | [<+REJECT>] true))".to_string()]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_eventual_mixed_temporal_alternatives() {
        let formulas = parse_formula_strings(&[
            "[+REQUEST] true -> eventually((<+APPROVE> true | [<+REJECT>] true))".to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_mixed_alternatives_with_signer_requirement() {
        let formulas = parse_formula_strings(&[
            "(<+APPROVE> true | [<+REJECT>] true) & ([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_mixed_alternatives_with_oracle_requirement() {
        let formulas = parse_formula_strings(&[
            "(<+APPROVE> true | [<+REJECT>] true) & ([+APPROVE] true -> <+oracle_attests(/oracles/review.id, \"approved\", \"true\")> true)"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);
        let output = format_synthesized_model(&model, "modality").unwrap();

        assert!(output.contains("+APPROVE"));
        assert!(output.contains("+REJECT"));
        assert!(output.contains("+oracle_attests(/oracles/review.id, approved, true)"));
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
    fn verify_synthesized_model_accepts_signer_and_compound_required_actions() {
        let formulas = parse_formula_strings(&[
            "[+RELEASE] true -> (<+signed_by(/users/buyer.id)> true & (eventually(<+DEPOSIT> true) & eventually(<+DELIVER> true)))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_multisig_and_compound_required_actions() {
        let formulas = parse_formula_strings(&[
            "[+APPROVE] true -> (<+signed_by(/users/alice.id) +signed_by(/users/bob.id)> true & (eventually(<+DEPOSIT> true) & eventually(<+DELIVER> true)))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_committed_action_signer_compound_required() {
        let formulas = parse_formula_strings(&[
            "[<+RELEASE>] true -> (<+signed_by(/users/buyer.id)> true & (eventually(<+DEPOSIT> true) & eventually(<+DELIVER> true)))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_committed_action_multisig_compound_required() {
        let formulas = parse_formula_strings(&[
            "[<+APPROVE>] true -> (<+signed_by(/users/alice.id) +signed_by(/users/bob.id)> true & (eventually(<+DEPOSIT> true) & eventually(<+DELIVER> true)))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_committed_signer_compound_required_actions() {
        let formulas = parse_formula_strings(&[
            "[+RELEASE] true -> ([<+signed_by(/users/buyer.id)>] true & (eventually(<+DEPOSIT> true) & eventually(<+DELIVER> true)))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_committed_multisig_compound_required_actions() {
        let formulas = parse_formula_strings(&[
            "[+APPROVE] true -> ([<+signed_by(/users/alice.id) +signed_by(/users/bob.id)>] true & (eventually(<+DEPOSIT> true) & eventually(<+DELIVER> true)))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_committed_action_committed_signer_compound_required() {
        let formulas = parse_formula_strings(&[
            "[<+RELEASE>] true -> ([<+signed_by(/users/buyer.id)>] true & (eventually(<+DEPOSIT> true) & eventually(<+DELIVER> true)))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_committed_action_committed_multisig_compound_required() {
        let formulas = parse_formula_strings(&[
            "[<+APPROVE>] true -> ([<+signed_by(/users/alice.id) +signed_by(/users/bob.id)>] true & (eventually(<+DEPOSIT> true) & eventually(<+DELIVER> true)))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_signer_compound_committed_required() {
        let formulas = parse_formula_strings(&[
            "[+RELEASE] true -> (<+signed_by(/users/buyer.id)> true & (eventually([<+DEPOSIT>] true) & eventually([<+DELIVER>] true)))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_multisig_compound_committed_required() {
        let formulas = parse_formula_strings(&[
            "[+APPROVE] true -> (<+signed_by(/users/alice.id) +signed_by(/users/bob.id)> true & (eventually([<+DEPOSIT>] true) & eventually([<+DELIVER>] true)))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_committed_action_signer_compound_committed_required() {
        let formulas = parse_formula_strings(&[
            "[<+RELEASE>] true -> (<+signed_by(/users/buyer.id)> true & (eventually([<+DEPOSIT>] true) & eventually([<+DELIVER>] true)))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_committed_action_multisig_compound_committed_required() {
        let formulas = parse_formula_strings(&[
            "[<+APPROVE>] true -> (<+signed_by(/users/alice.id) +signed_by(/users/bob.id)> true & (eventually([<+DEPOSIT>] true) & eventually([<+DELIVER>] true)))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_committed_signer_compound_committed_required() {
        let formulas = parse_formula_strings(&[
            "[+RELEASE] true -> ([<+signed_by(/users/buyer.id)>] true & (eventually([<+DEPOSIT>] true) & eventually([<+DELIVER>] true)))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_committed_multisig_compound_committed_required() {
        let formulas = parse_formula_strings(&[
            "[+APPROVE] true -> ([<+signed_by(/users/alice.id) +signed_by(/users/bob.id)>] true & (eventually([<+DEPOSIT>] true) & eventually([<+DELIVER>] true)))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_committed_action_committed_signer_compound_commitments() {
        let formulas = parse_formula_strings(&[
            "[<+RELEASE>] true -> ([<+signed_by(/users/buyer.id)>] true & (eventually([<+DEPOSIT>] true) & eventually([<+DELIVER>] true)))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_committed_action_committed_multisig_compound_commitments() {
        let formulas = parse_formula_strings(&[
            "[<+APPROVE>] true -> ([<+signed_by(/users/alice.id) +signed_by(/users/bob.id)>] true & (eventually([<+DEPOSIT>] true) & eventually([<+DELIVER>] true)))"
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
    fn verify_synthesized_model_accepts_oracle_attestation_example() {
        let formulas = parse_formula_strings(&[
            "[+RELEASE] true -> <+oracle_attests(/oracles/delivery.id, \"delivered\", \"true\")> true"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);
        let output = format_synthesized_model(&model, "modality").unwrap();

        assert!(output.contains("+RELEASE"));
        assert!(output.contains("+oracle_attests(/oracles/delivery.id, delivered, true)"));
        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_path_predicate_examples() {
        let formulas = parse_formula_strings(&[
            "always([+UPDATE] true -> <+any_signed(/members)> true)".to_string(),
            "always([+CHANGE_MEMBERS] true -> <+modifies(/members) +all_signed(/members)> true)"
                .to_string(),
            "always([+UPDATE_PROFILE] true -> <+any_signed(/members) -modifies(/members)> true)"
                .to_string(),
            "always([+CHANGE_CONFIG] true -> <+modifies(/config) +signed_by(/users/admin.id)> true)"
                .to_string(),
            "always([+CHANGE_PRIVATE] true -> <+modifies(/private) +all_signed(/members)> true)"
                .to_string(),
            "always([+SETTLE_ESCROW] true -> <+modifies(/escrow) +oracle_attests(/oracles/delivery.id, \"delivered\", \"true\")> true)"
                .to_string(),
            "[<+SETTLE_ESCROW>] true -> [<+modifies(/escrow) +oracle_attests(/oracles/delivery.id, \"delivered\", \"true\")>] true"
                .to_string(),
            "[<+ROTATE_KEY>] true -> [<+modifies(/keys) +signed_by(/users/security_admin.id)>] true"
                .to_string(),
            "[<+UPDATE_PROFILE>] true -> [<+any_signed(/members) -modifies(/members)>] true"
                .to_string(),
            "[<+CHANGE_MEMBERS>] true -> [<+modifies(/members) +all_signed(/members)>] true"
                .to_string(),
            "[<+CHANGE_CONFIG>] true -> [<+modifies(/config) +signed_by(/users/admin.id)>] true"
                .to_string(),
            "[<+CHANGE_PRIVATE>] true -> [<+modifies(/private) +all_signed(/members)>] true"
                .to_string(),
            "[<+EXECUTE_TREASURY>] true -> [<+modifies(/treasury) +threshold(\"2\", /treasury/signers)>] true"
                .to_string(),
            "[<+PUBLISH_AUDIT>] true -> [<+modifies(/audit) +signed_by(/users/auditor.id) +oracle_attests(/oracles/audit.id, \"passed\", \"true\")>] true"
                .to_string(),
            "[<+CLOSE_INCIDENT>] true -> [<+modifies(/incidents) +signed_by(/users/incident_commander.id)>] true"
                .to_string(),
            "[<+FREEZE_CHANGE>] true -> [<+modifies(/releases) +signed_by(/users/release_manager.id)>] true"
                .to_string(),
            "[<+ACCEPT_RISK>] true -> [<+modifies(/risk) +signed_by(/users/risk_owner.id)>] true"
                .to_string(),
            "[<+APPROVE_SAFETY>] true -> [<+modifies(/safety) +signed_by(/users/safety_reviewer.id)>] true"
                .to_string(),
            "[<+ATTEST_COMPLIANCE>] true -> [<+modifies(/compliance) +signed_by(/users/compliance_officer.id)>] true"
                .to_string(),
            "[<+CONFIRM_DELIVERY>] true -> [<+modifies(/delivery) +signed_by(/users/recipient.id)>] true"
                .to_string(),
            "[<+APPROVE_INVOICE>] true -> [<+modifies(/invoices) +signed_by(/users/payer.id)>] true"
                .to_string(),
            "[<+APPROVE_BUDGET>] true -> [<+modifies(/budgets) +signed_by(/users/budget_owner.id)>] true"
                .to_string(),
            "[<+APPROVE_PURCHASE_ORDER>] true -> [<+modifies(/purchase_orders) +signed_by(/users/procurement_manager.id)>] true"
                .to_string(),
            "[<+APPROVE_CONTRACT>] true -> [<+modifies(/contracts) +signed_by(/users/legal_reviewer.id)>] true"
                .to_string(),
            "[<+ONBOARD_VENDOR>] true -> [<+modifies(/vendors) +signed_by(/users/vendor_manager.id)>] true"
                .to_string(),
            "[<+APPROVE_TIME_OFF>] true -> [<+modifies(/time_off) +signed_by(/users/manager.id)>] true"
                .to_string(),
            "[<+APPROVE_EXPENSE>] true -> [<+modifies(/expenses) +signed_by(/users/finance_manager.id)>] true"
                .to_string(),
            "[<+APPROVE_TRAVEL>] true -> [<+modifies(/travel) +signed_by(/users/travel_manager.id)>] true"
                .to_string(),
            "[<+APPROVE_REIMBURSEMENT>] true -> [<+modifies(/reimbursements) +signed_by(/users/payroll_manager.id)>] true"
                .to_string(),
            "[<+APPROVE_REFUND>] true -> [<+modifies(/refunds) +signed_by(/users/refund_manager.id)>] true"
                .to_string(),
            "[<+APPROVE_CREDIT>] true -> [<+modifies(/credits) +signed_by(/users/credit_manager.id)>] true"
                .to_string(),
            "[<+APPROVE_ADJUSTMENT>] true -> [<+modifies(/adjustments) +signed_by(/users/controller.id)>] true"
                .to_string(),
            "[<+APPROVE_PAYMENT>] true -> [<+modifies(/payments) +signed_by(/users/payment_approver.id)>] true"
                .to_string(),
            "[<+APPROVE_DISCOUNT>] true -> [<+modifies(/discounts) +signed_by(/users/sales_manager.id)>] true"
                .to_string(),
            "[<+APPROVE_COMMISSION>] true -> [<+modifies(/commissions) +signed_by(/users/revenue_lead.id)>] true"
                .to_string(),
            "[<+APPROVE_GRANT>] true -> [<+modifies(/grants) +signed_by(/users/grants_manager.id)>] true"
                .to_string(),
            "[<+APPROVE_LOAN>] true -> [<+modifies(/loans) +signed_by(/users/loan_officer.id)>] true"
                .to_string(),
            "[<+APPROVE_CLAIM>] true -> [<+modifies(/claims) +signed_by(/users/claims_adjuster.id)>] true"
                .to_string(),
            "[<+APPROVE_WITHDRAWAL>] true -> [<+modifies(/withdrawals) +signed_by(/users/treasury_officer.id)>] true"
                .to_string(),
            "[<+APPROVE_SETTLEMENT>] true -> [<+modifies(/settlements) +signed_by(/users/settlement_manager.id)>] true"
                .to_string(),
            "[<+APPROVE_DIVIDEND>] true -> [<+modifies(/dividends) +signed_by(/users/board_secretary.id)>] true"
                .to_string(),
            "[<+APPROVE_ROYALTY>] true -> [<+modifies(/royalties) +signed_by(/users/rights_manager.id)>] true"
                .to_string(),
            "[<+APPROVE_LICENSE>] true -> [<+modifies(/licenses) +signed_by(/users/licensing_manager.id)>] true"
                .to_string(),
            "[<+APPROVE_RENEWAL>] true -> [<+modifies(/renewals) +signed_by(/users/account_manager.id)>] true"
                .to_string(),
            "[<+APPROVE_SUBSCRIPTION>] true -> [<+modifies(/subscriptions) +signed_by(/users/customer_success_manager.id)>] true"
                .to_string(),
            "[<+APPROVE_ENTITLEMENT>] true -> [<+modifies(/entitlements) +signed_by(/users/access_manager.id)>] true"
                .to_string(),
            "[<+APPROVE_DEPRECATION>] true -> [<+modifies(/deprecations) +signed_by(/users/product_manager.id)>] true"
                .to_string(),
            "[<+APPROVE_ARCHIVE>] true -> [<+modifies(/archives) +signed_by(/users/records_manager.id)>] true"
                .to_string(),
            "[<+APPROVE_RETENTION>] true -> [<+modifies(/retention) +signed_by(/users/records_counsel.id)>] true"
                .to_string(),
            "[<+APPROVE_POLICY>] true -> [<+modifies(/policies) +signed_by(/users/policy_owner.id)>] true"
                .to_string(),
            "[<+APPROVE_CERTIFICATION>] true -> [<+modifies(/certifications) +signed_by(/users/certification_manager.id)>] true"
                .to_string(),
            "[<+APPROVE_ACCREDITATION>] true -> [<+modifies(/accreditations) +signed_by(/users/accreditation_manager.id)>] true"
                .to_string(),
            "[<+APPROVE_WAIVER>] true -> [<+modifies(/waivers) +signed_by(/users/waiver_authority.id)>] true"
                .to_string(),
            "[<+APPROVE_EXCEPTION>] true -> [<+modifies(/exceptions) +signed_by(/users/exception_owner.id)>] true"
                .to_string(),
            "[<+APPROVE_VARIANCE>] true -> [<+modifies(/variances) +signed_by(/users/variance_owner.id)>] true"
                .to_string(),
            "[<+APPROVE_EXTENSION>] true -> [<+modifies(/extensions) +signed_by(/users/extension_owner.id)>] true"
                .to_string(),
            "[<+APPROVE_AMENDMENT>] true -> [<+modifies(/amendments) +signed_by(/users/amendment_owner.id)>] true"
                .to_string(),
            "[<+APPROVE_ADDENDUM>] true -> [<+modifies(/addenda) +signed_by(/users/addendum_owner.id)>] true"
                .to_string(),
            "[<+APPROVE_SUPPLEMENT>] true -> [<+modifies(/supplements) +signed_by(/users/supplement_owner.id)>] true"
                .to_string(),
            "[<+APPROVE_APPENDIX>] true -> [<+modifies(/appendices) +signed_by(/users/appendix_owner.id)>] true"
                .to_string(),
            "[<+APPROVE_RIDER>] true -> [<+modifies(/riders) +signed_by(/users/rider_owner.id)>] true"
                .to_string(),
            "[<+APPROVE_ENDORSEMENT>] true -> [<+modifies(/endorsements) +signed_by(/users/endorsement_owner.id)>] true"
                .to_string(),
            "[<+APPROVE_EXHIBIT>] true -> [<+modifies(/exhibits) +signed_by(/users/exhibit_owner.id)>] true"
                .to_string(),
            "[<+APPROVE_SCHEDULE>] true -> [<+modifies(/schedules) +signed_by(/users/schedule_owner.id)>] true"
                .to_string(),
            "[<+APPROVE_ATTACHMENT>] true -> [<+modifies(/attachments) +signed_by(/users/attachment_owner.id)>] true"
                .to_string(),
            "[<+APPROVE_ANNEX>] true -> [<+modifies(/annexes) +signed_by(/users/annex_owner.id)>] true"
                .to_string(),
            "[<+APPROVE_ENCLOSURE>] true -> [<+modifies(/enclosures) +signed_by(/users/enclosure_owner.id)>] true"
                .to_string(),
            "[<+APPROVE_PACKAGE>] true -> [<+modifies(/packages) +signed_by(/users/package_owner.id)>] true"
                .to_string(),
            "[<+APPROVE_BUNDLE>] true -> [<+modifies(/bundles) +signed_by(/users/bundle_owner.id)>] true"
                .to_string(),
            "[<+APPROVE_DOSSIER>] true -> [<+modifies(/dossiers) +signed_by(/users/dossier_owner.id)>] true"
                .to_string(),
            "[<+APPROVE_FILE>] true -> [<+modifies(/files) +signed_by(/users/file_owner.id)>] true"
                .to_string(),
            "[<+APPROVE_RECORD>] true -> [<+modifies(/records) +signed_by(/users/record_owner.id)>] true"
                .to_string(),
            "[<+APPROVE_CASE>] true -> [<+modifies(/cases) +signed_by(/users/case_owner.id)>] true"
                .to_string(),
            "[<+APPROVE_TICKET>] true -> [<+modifies(/tickets) +signed_by(/users/ticket_owner.id)>] true"
                .to_string(),
            "[<+APPROVE_PROPOSAL>] true -> [<+modifies(/proposals) +signed_by(/users/proposal_owner.id)>] true"
                .to_string(),
            "[<+APPROVE_REQUEST>] true -> [<+modifies(/requests) +signed_by(/users/request_owner.id)>] true"
                .to_string(),
            "[<+APPROVE_APPLICATION>] true -> [<+modifies(/applications) +signed_by(/users/application_owner.id)>] true"
                .to_string(),
            "[<+APPROVE_SUBMISSION>] true -> [<+modifies(/submissions) +signed_by(/users/submission_owner.id)>] true"
                .to_string(),
            "[<+APPROVE_DOCUMENT>] true -> [<+modifies(/documents) +signed_by(/users/document_owner.id)>] true"
                .to_string(),
            "[<+APPROVE_REPORT>] true -> [<+modifies(/reports) +signed_by(/users/report_owner.id)>] true"
                .to_string(),
            "[<+APPROVE_MEMO>] true -> [<+modifies(/memos) +signed_by(/users/memo_owner.id)>] true"
                .to_string(),
            "[<+APPROVE_NOTE>] true -> [<+modifies(/notes) +signed_by(/users/note_owner.id)>] true"
                .to_string(),
            "[<+APPROVE_COMMENT>] true -> [<+modifies(/comments) +signed_by(/users/comment_owner.id)>] true"
                .to_string(),
            "[<+APPROVE_REPLY>] true -> [<+modifies(/replies) +signed_by(/users/reply_owner.id)>] true"
                .to_string(),
            "[<+APPROVE_FEEDBACK>] true -> [<+modifies(/feedback) +signed_by(/users/feedback_owner.id)>] true"
                .to_string(),
            "[<+APPROVE_RATING>] true -> [<+modifies(/ratings) +signed_by(/users/rating_owner.id)>] true"
                .to_string(),
            "[<+APPROVE_REVIEW>] true -> [<+modifies(/reviews) +signed_by(/users/review_owner.id)>] true"
                .to_string(),
            "[<+APPROVE_SURVEY>] true -> [<+modifies(/surveys) +signed_by(/users/survey_owner.id)>] true"
                .to_string(),
            "[<+APPROVE_RESPONSE>] true -> [<+modifies(/responses) +signed_by(/users/response_owner.id)>] true"
                .to_string(),
            "[<+APPROVE_RESULT>] true -> [<+modifies(/results) +signed_by(/users/result_owner.id)>] true"
                .to_string(),
            "[<+APPROVE_OUTCOME>] true -> [<+modifies(/outcomes) +signed_by(/users/outcome_owner.id)>] true"
                .to_string(),
            "[<+APPROVE_DECISION>] true -> [<+modifies(/decisions) +signed_by(/users/decision_owner.id)>] true"
                .to_string(),
            "[<+APPROVE_PLAN>] true -> [<+modifies(/plans) +signed_by(/users/plan_owner.id)>] true"
                .to_string(),
            "[<+APPROVE_STRATEGY>] true -> [<+modifies(/strategies) +signed_by(/users/strategy_owner.id)>] true"
                .to_string(),
            "[<+APPROVE_OBJECTIVE>] true -> [<+modifies(/objectives) +signed_by(/users/objective_owner.id)>] true"
                .to_string(),
            "[<+APPROVE_TARGET>] true -> [<+modifies(/targets) +signed_by(/users/target_owner.id)>] true"
                .to_string(),
            "[<+APPROVE_GOAL>] true -> [<+modifies(/goals) +signed_by(/users/goal_owner.id)>] true"
                .to_string(),
            "[<+APPROVE_KPI>] true -> [<+modifies(/kpis) +signed_by(/users/kpi_owner.id)>] true"
                .to_string(),
            "[<+APPROVE_METRIC>] true -> [<+modifies(/metrics) +signed_by(/users/metric_owner.id)>] true"
                .to_string(),
            "[<+APPROVE_OKR>] true -> [<+modifies(/okrs) +signed_by(/users/okr_owner.id)>] true"
                .to_string(),
            "[<+APPROVE_INITIATIVE>] true -> [<+modifies(/initiatives) +signed_by(/users/initiative_owner.id)>] true"
                .to_string(),
            "[<+APPROVE_EPIC>] true -> [<+modifies(/epics) +signed_by(/users/epic_owner.id)>] true"
                .to_string(),
            "[<+APPROVE_STORY>] true -> [<+modifies(/stories) +signed_by(/users/story_owner.id)>] true"
                .to_string(),
            "[<+APPROVE_TASK>] true -> [<+modifies(/tasks) +signed_by(/users/task_owner.id)>] true"
                .to_string(),
            "[<+APPROVE_BUG>] true -> [<+modifies(/bugs) +signed_by(/users/bug_owner.id)>] true"
                .to_string(),
            "[<+APPROVE_ISSUE>] true -> [<+modifies(/issues) +signed_by(/users/issue_owner.id)>] true"
                .to_string(),
            "[<+APPROVE_DEFECT>] true -> [<+modifies(/defects) +signed_by(/users/defect_owner.id)>] true"
                .to_string(),
            "[<+APPROVE_PATCH>] true -> [<+modifies(/patches) +signed_by(/users/patch_owner.id)>] true"
                .to_string(),
            "[<+APPROVE_HOTFIX>] true -> [<+modifies(/hotfixes) +signed_by(/users/hotfix_owner.id)>] true"
                .to_string(),
            "[<+APPROVE_RELEASE_CANDIDATE>] true -> [<+modifies(/release_candidates) +signed_by(/users/release_manager.id)>] true"
                .to_string(),
            "[<+APPROVE_DEPLOYMENT>] true -> [<+modifies(/deployments) +signed_by(/users/deployment_owner.id)>] true"
                .to_string(),
            "[<+APPROVE_ROLLOUT>] true -> [<+modifies(/rollouts) +signed_by(/users/rollout_owner.id)>] true"
                .to_string(),
            "[<+APPROVE_LAUNCH>] true -> [<+modifies(/launches) +signed_by(/users/launch_owner.id)>] true"
                .to_string(),
            "[<+APPROVE_GENERAL_AVAILABILITY>] true -> [<+modifies(/general_availability) +signed_by(/users/ga_owner.id)>] true"
                .to_string(),
            "[<+APPROVE_PRODUCTION>] true -> [<+modifies(/production) +signed_by(/users/production_owner.id)>] true"
                .to_string(),
            "[<+APPROVE_OPERATIONS>] true -> [<+modifies(/operations) +signed_by(/users/operations_owner.id)>] true"
                .to_string(),
            "[<+APPROVE_MAINTENANCE>] true -> [<+modifies(/maintenance) +signed_by(/users/maintenance_owner.id)>] true"
                .to_string(),
            "[<+APPROVE_SUPPORT>] true -> [<+modifies(/support) +signed_by(/users/support_owner.id)>] true"
                .to_string(),
            "[<+APPROVE_TRAINING>] true -> [<+modifies(/training) +signed_by(/users/training_owner.id)>] true"
                .to_string(),
            "[<+APPROVE_COMPLIANCE>] true -> [<+modifies(/compliance) +signed_by(/users/compliance_owner.id)>] true"
                .to_string(),
            "[<+APPROVE_ONBOARDING>] true -> [<+modifies(/onboarding) +signed_by(/users/onboarding_owner.id)>] true"
                .to_string(),
            "[<+APPROVE_OFFBOARDING>] true -> [<+modifies(/offboarding) +signed_by(/users/offboarding_owner.id)>] true"
                .to_string(),
            "[<+APPROVE_DEPROVISIONING>] true -> [<+modifies(/deprovisioning) +signed_by(/users/access_owner.id)>] true"
                .to_string(),
            "[<+APPROVE_ACCESS_REVIEW>] true -> [<+modifies(/access_reviews) +signed_by(/users/access_reviewer.id)>] true"
                .to_string(),
            "[<+APPROVE_IDENTITY_VERIFICATION>] true -> [<+modifies(/identity_verifications) +signed_by(/users/identity_reviewer.id)>] true"
                .to_string(),
            "[<+ISSUE_CREDENTIAL>] true -> [<+modifies(/credentials) +signed_by(/users/credential_issuer.id)>] true"
                .to_string(),
            "[<+REVOKE_CREDENTIAL>] true -> [<+modifies(/credential_revocations) +signed_by(/users/credential_issuer.id)>] true"
                .to_string(),
            "[<+RENEW_CREDENTIAL>] true -> [<+modifies(/credential_renewals) +signed_by(/users/credential_issuer.id)>] true"
                .to_string(),
            "[<+EXPIRE_CREDENTIAL>] true -> [<+modifies(/credential_expirations) +signed_by(/users/credential_issuer.id)>] true"
                .to_string(),
            "[<+SUSPEND_CREDENTIAL>] true -> [<+modifies(/credential_suspensions) +signed_by(/users/credential_issuer.id)>] true"
                .to_string(),
            "[<+REINSTATE_CREDENTIAL>] true -> [<+modifies(/credential_reinstatements) +signed_by(/users/credential_issuer.id)>] true"
                .to_string(),
            "[<+VERIFY_CREDENTIAL>] true -> [<+modifies(/credential_verifications) +signed_by(/users/credential_verifier.id)>] true"
                .to_string(),
            "[<+PRESENT_CREDENTIAL>] true -> [<+modifies(/credential_presentations) +signed_by(/users/credential_holder.id)>] true"
                .to_string(),
            "[<+SHARE_CREDENTIAL>] true -> [<+modifies(/credential_shares) +signed_by(/users/credential_holder.id)>] true"
                .to_string(),
            "[<+EXPORT_CREDENTIAL>] true -> [<+modifies(/credential_exports) +signed_by(/users/credential_holder.id)>] true"
                .to_string(),
            "[<+REQUEST_CREDENTIAL>] true -> [<+modifies(/credential_requests) +signed_by(/users/credential_holder.id)>] true"
                .to_string(),
            "[<+ACCEPT_CREDENTIAL>] true -> [<+modifies(/credential_acceptances) +signed_by(/users/credential_holder.id)>] true"
                .to_string(),
            "[<+REJECT_CREDENTIAL>] true -> [<+modifies(/credential_rejections) +signed_by(/users/credential_holder.id)>] true"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("PathPolicy", &formulas);
        let output = format_synthesized_model(&model, "modality").unwrap();

        assert!(output.contains("+UPDATE +any_signed(/members)"));
        assert!(output.contains("+CHANGE_MEMBERS +modifies(/members) +all_signed(/members)"));
        assert!(output.contains("+UPDATE_PROFILE +any_signed(/members) -modifies(/members)"));
        assert!(output.contains("+CHANGE_CONFIG +signed_by(/users/admin.id) +modifies(/config)"));
        assert!(output.contains("+CHANGE_PRIVATE +modifies(/private) +all_signed(/members)"));
        assert!(output.contains(
            "+SETTLE_ESCROW +modifies(/escrow) +oracle_attests(/oracles/delivery.id, delivered, true)"
        ));
        assert!(output.contains(
            "+ROTATE_KEY +signed_by(/users/security_admin.id) +modifies(/keys)"
        ));
        assert!(output.contains("+UPDATE_PROFILE +any_signed(/members) -modifies(/members)"));
        assert!(output.contains("+CHANGE_MEMBERS +modifies(/members) +all_signed(/members)"));
        assert!(output.contains("+CHANGE_CONFIG +signed_by(/users/admin.id) +modifies(/config)"));
        assert!(output.contains("+CHANGE_PRIVATE +modifies(/private) +all_signed(/members)"));
        assert!(output
            .contains("+EXECUTE_TREASURY +modifies(/treasury) +threshold(\"2\", /treasury/signers)"));
        assert!(output.contains(
            "+PUBLISH_AUDIT +signed_by(/users/auditor.id) +modifies(/audit) +oracle_attests(/oracles/audit.id, passed, true)"
        ));
        assert!(output
            .contains("+CLOSE_INCIDENT +signed_by(/users/incident_commander.id) +modifies(/incidents)"));
        assert!(output.contains(
            "+FREEZE_CHANGE +signed_by(/users/release_manager.id) +modifies(/releases)"
        ));
        assert!(output.contains("+ACCEPT_RISK +signed_by(/users/risk_owner.id) +modifies(/risk)"));
        assert!(output.contains(
            "+APPROVE_SAFETY +signed_by(/users/safety_reviewer.id) +modifies(/safety)"
        ));
        assert!(output.contains(
            "+ATTEST_COMPLIANCE +signed_by(/users/compliance_officer.id) +modifies(/compliance)"
        ));
        assert!(output.contains(
            "+CONFIRM_DELIVERY +signed_by(/users/recipient.id) +modifies(/delivery)"
        ));
        assert!(
            output.contains("+APPROVE_INVOICE +signed_by(/users/payer.id) +modifies(/invoices)")
        );
        assert!(output.contains(
            "+APPROVE_BUDGET +signed_by(/users/budget_owner.id) +modifies(/budgets)"
        ));
        assert!(output.contains(
            "+APPROVE_PURCHASE_ORDER +signed_by(/users/procurement_manager.id) +modifies(/purchase_orders)"
        ));
        assert!(output.contains(
            "+APPROVE_CONTRACT +signed_by(/users/legal_reviewer.id) +modifies(/contracts)"
        ));
        assert!(
            output
                .contains("+ONBOARD_VENDOR +signed_by(/users/vendor_manager.id) +modifies(/vendors)")
        );
        assert!(
            output.contains("+APPROVE_TIME_OFF +signed_by(/users/manager.id) +modifies(/time_off)")
        );
        assert!(output.contains(
            "+APPROVE_EXPENSE +signed_by(/users/finance_manager.id) +modifies(/expenses)"
        ));
        assert!(output.contains(
            "+APPROVE_TRAVEL +signed_by(/users/travel_manager.id) +modifies(/travel)"
        ));
        assert!(output.contains(
            "+APPROVE_REIMBURSEMENT +signed_by(/users/payroll_manager.id) +modifies(/reimbursements)"
        ));
        assert!(output.contains(
            "+APPROVE_REFUND +signed_by(/users/refund_manager.id) +modifies(/refunds)"
        ));
        assert!(output.contains(
            "+APPROVE_CREDIT +signed_by(/users/credit_manager.id) +modifies(/credits)"
        ));
        assert!(output.contains(
            "+APPROVE_ADJUSTMENT +signed_by(/users/controller.id) +modifies(/adjustments)"
        ));
        assert!(output.contains(
            "+APPROVE_PAYMENT +signed_by(/users/payment_approver.id) +modifies(/payments)"
        ));
        assert!(output.contains(
            "+APPROVE_DISCOUNT +signed_by(/users/sales_manager.id) +modifies(/discounts)"
        ));
        assert!(output.contains(
            "+APPROVE_COMMISSION +signed_by(/users/revenue_lead.id) +modifies(/commissions)"
        ));
        assert!(output
            .contains("+APPROVE_GRANT +signed_by(/users/grants_manager.id) +modifies(/grants)"));
        assert!(
            output.contains("+APPROVE_LOAN +signed_by(/users/loan_officer.id) +modifies(/loans)")
        );
        assert!(output
            .contains("+APPROVE_CLAIM +signed_by(/users/claims_adjuster.id) +modifies(/claims)"));
        assert!(output.contains(
            "+APPROVE_WITHDRAWAL +signed_by(/users/treasury_officer.id) +modifies(/withdrawals)"
        ));
        assert!(output.contains(
            "+APPROVE_SETTLEMENT +signed_by(/users/settlement_manager.id) +modifies(/settlements)"
        ));
        assert!(output.contains(
            "+APPROVE_DIVIDEND +signed_by(/users/board_secretary.id) +modifies(/dividends)"
        ));
        assert!(output.contains(
            "+APPROVE_ROYALTY +signed_by(/users/rights_manager.id) +modifies(/royalties)"
        ));
        assert!(output.contains(
            "+APPROVE_LICENSE +signed_by(/users/licensing_manager.id) +modifies(/licenses)"
        ));
        assert!(output.contains(
            "+APPROVE_RENEWAL +signed_by(/users/account_manager.id) +modifies(/renewals)"
        ));
        assert!(output.contains(
            "+APPROVE_SUBSCRIPTION +signed_by(/users/customer_success_manager.id) +modifies(/subscriptions)"
        ));
        assert!(output.contains(
            "+APPROVE_ENTITLEMENT +signed_by(/users/access_manager.id) +modifies(/entitlements)"
        ));
        assert!(output.contains(
            "+APPROVE_DEPRECATION +signed_by(/users/product_manager.id) +modifies(/deprecations)"
        ));
        assert!(output.contains(
            "+APPROVE_ARCHIVE +signed_by(/users/records_manager.id) +modifies(/archives)"
        ));
        assert!(output.contains(
            "+APPROVE_RETENTION +signed_by(/users/records_counsel.id) +modifies(/retention)"
        ));
        assert!(output.contains(
            "+APPROVE_POLICY +signed_by(/users/policy_owner.id) +modifies(/policies)"
        ));
        assert!(output.contains(
            "+APPROVE_CERTIFICATION +signed_by(/users/certification_manager.id) +modifies(/certifications)"
        ));
        assert!(output.contains(
            "+APPROVE_ACCREDITATION +signed_by(/users/accreditation_manager.id) +modifies(/accreditations)"
        ));
        assert!(output.contains(
            "+APPROVE_WAIVER +signed_by(/users/waiver_authority.id) +modifies(/waivers)"
        ));
        assert!(output.contains(
            "+APPROVE_EXCEPTION +signed_by(/users/exception_owner.id) +modifies(/exceptions)"
        ));
        assert!(output.contains(
            "+APPROVE_VARIANCE +signed_by(/users/variance_owner.id) +modifies(/variances)"
        ));
        assert!(output.contains(
            "+APPROVE_EXTENSION +signed_by(/users/extension_owner.id) +modifies(/extensions)"
        ));
        assert!(output.contains(
            "+APPROVE_AMENDMENT +signed_by(/users/amendment_owner.id) +modifies(/amendments)"
        ));
        assert!(output.contains(
            "+APPROVE_ADDENDUM +signed_by(/users/addendum_owner.id) +modifies(/addenda)"
        ));
        assert!(output.contains(
            "+APPROVE_SUPPLEMENT +signed_by(/users/supplement_owner.id) +modifies(/supplements)"
        ));
        assert!(output.contains(
            "+APPROVE_APPENDIX +signed_by(/users/appendix_owner.id) +modifies(/appendices)"
        ));
        assert!(output.contains(
            "+APPROVE_RIDER +signed_by(/users/rider_owner.id) +modifies(/riders)"
        ));
        assert!(output.contains(
            "+APPROVE_ENDORSEMENT +signed_by(/users/endorsement_owner.id) +modifies(/endorsements)"
        ));
        assert!(output.contains(
            "+APPROVE_EXHIBIT +signed_by(/users/exhibit_owner.id) +modifies(/exhibits)"
        ));
        assert!(output.contains(
            "+APPROVE_SCHEDULE +signed_by(/users/schedule_owner.id) +modifies(/schedules)"
        ));
        assert!(output.contains(
            "+APPROVE_ATTACHMENT +signed_by(/users/attachment_owner.id) +modifies(/attachments)"
        ));
        assert!(
            output.contains("+APPROVE_ANNEX +signed_by(/users/annex_owner.id) +modifies(/annexes)")
        );
        assert!(output.contains(
            "+APPROVE_ENCLOSURE +signed_by(/users/enclosure_owner.id) +modifies(/enclosures)"
        ));
        assert!(output.contains(
            "+APPROVE_PACKAGE +signed_by(/users/package_owner.id) +modifies(/packages)"
        ));
        assert!(output.contains(
            "+APPROVE_BUNDLE +signed_by(/users/bundle_owner.id) +modifies(/bundles)"
        ));
        assert!(output.contains(
            "+APPROVE_DOSSIER +signed_by(/users/dossier_owner.id) +modifies(/dossiers)"
        ));
        assert!(output.contains(
            "+APPROVE_FILE +signed_by(/users/file_owner.id) +modifies(/files)"
        ));
        assert!(output.contains(
            "+APPROVE_RECORD +signed_by(/users/record_owner.id) +modifies(/records)"
        ));
        assert!(
            output.contains("+APPROVE_CASE +signed_by(/users/case_owner.id) +modifies(/cases)")
        );
        assert!(output.contains(
            "+APPROVE_TICKET +signed_by(/users/ticket_owner.id) +modifies(/tickets)"
        ));
        assert!(output.contains(
            "+APPROVE_PROPOSAL +signed_by(/users/proposal_owner.id) +modifies(/proposals)"
        ));
        assert!(output.contains(
            "+APPROVE_REQUEST +signed_by(/users/request_owner.id) +modifies(/requests)"
        ));
        assert!(output.contains(
            "+APPROVE_APPLICATION +signed_by(/users/application_owner.id) +modifies(/applications)"
        ));
        assert!(output.contains(
            "+APPROVE_SUBMISSION +signed_by(/users/submission_owner.id) +modifies(/submissions)"
        ));
        assert!(output.contains(
            "+APPROVE_DOCUMENT +signed_by(/users/document_owner.id) +modifies(/documents)"
        ));
        assert!(output
            .contains("+APPROVE_REPORT +signed_by(/users/report_owner.id) +modifies(/reports)"));
        assert!(
            output.contains("+APPROVE_MEMO +signed_by(/users/memo_owner.id) +modifies(/memos)")
        );
        assert!(
            output.contains("+APPROVE_NOTE +signed_by(/users/note_owner.id) +modifies(/notes)")
        );
        assert!(output.contains(
            "+APPROVE_COMMENT +signed_by(/users/comment_owner.id) +modifies(/comments)"
        ));
        assert!(output
            .contains("+APPROVE_REPLY +signed_by(/users/reply_owner.id) +modifies(/replies)"));
        assert!(output.contains(
            "+APPROVE_FEEDBACK +signed_by(/users/feedback_owner.id) +modifies(/feedback)"
        ));
        assert!(
            output.contains("+APPROVE_RATING +signed_by(/users/rating_owner.id) +modifies(/ratings)")
        );
        assert!(
            output.contains("+APPROVE_REVIEW +signed_by(/users/review_owner.id) +modifies(/reviews)")
        );
        assert!(
            output.contains("+APPROVE_SURVEY +signed_by(/users/survey_owner.id) +modifies(/surveys)")
        );
        assert!(output.contains(
            "+APPROVE_RESPONSE +signed_by(/users/response_owner.id) +modifies(/responses)"
        ));
        assert!(
            output.contains("+APPROVE_RESULT +signed_by(/users/result_owner.id) +modifies(/results)")
        );
        assert!(output.contains(
            "+APPROVE_OUTCOME +signed_by(/users/outcome_owner.id) +modifies(/outcomes)"
        ));
        assert!(output.contains(
            "+APPROVE_DECISION +signed_by(/users/decision_owner.id) +modifies(/decisions)"
        ));
        assert!(output.contains("+APPROVE_PLAN +signed_by(/users/plan_owner.id) +modifies(/plans)"));
        assert!(output.contains(
            "+APPROVE_STRATEGY +signed_by(/users/strategy_owner.id) +modifies(/strategies)"
        ));
        assert!(output.contains(
            "+APPROVE_OBJECTIVE +signed_by(/users/objective_owner.id) +modifies(/objectives)"
        ));
        assert!(output
            .contains("+APPROVE_TARGET +signed_by(/users/target_owner.id) +modifies(/targets)"));
        assert!(
            output.contains("+APPROVE_GOAL +signed_by(/users/goal_owner.id) +modifies(/goals)")
        );
        assert!(output.contains("+APPROVE_KPI +signed_by(/users/kpi_owner.id) +modifies(/kpis)"));
        assert!(
            output.contains("+APPROVE_METRIC +signed_by(/users/metric_owner.id) +modifies(/metrics)")
        );
        assert!(output.contains("+APPROVE_OKR +signed_by(/users/okr_owner.id) +modifies(/okrs)"));
        assert!(output.contains(
            "+APPROVE_INITIATIVE +signed_by(/users/initiative_owner.id) +modifies(/initiatives)"
        ));
        assert!(
            output.contains("+APPROVE_EPIC +signed_by(/users/epic_owner.id) +modifies(/epics)")
        );
        assert!(
            output.contains("+APPROVE_STORY +signed_by(/users/story_owner.id) +modifies(/stories)")
        );
        assert!(
            output.contains("+APPROVE_TASK +signed_by(/users/task_owner.id) +modifies(/tasks)")
        );
        assert!(output.contains("+APPROVE_BUG +signed_by(/users/bug_owner.id) +modifies(/bugs)"));
        assert!(
            output.contains("+APPROVE_ISSUE +signed_by(/users/issue_owner.id) +modifies(/issues)")
        );
        assert!(
            output.contains("+APPROVE_DEFECT +signed_by(/users/defect_owner.id) +modifies(/defects)")
        );
        assert!(
            output.contains("+APPROVE_PATCH +signed_by(/users/patch_owner.id) +modifies(/patches)")
        );
        assert!(
            output.contains("+APPROVE_HOTFIX +signed_by(/users/hotfix_owner.id) +modifies(/hotfixes)")
        );
        assert!(output.contains(
            "+APPROVE_RELEASE_CANDIDATE +signed_by(/users/release_manager.id) +modifies(/release_candidates)"
        ));
        assert!(output.contains(
            "+APPROVE_DEPLOYMENT +signed_by(/users/deployment_owner.id) +modifies(/deployments)"
        ));
        assert!(output.contains(
            "+APPROVE_ROLLOUT +signed_by(/users/rollout_owner.id) +modifies(/rollouts)"
        ));
        assert!(output.contains(
            "+APPROVE_LAUNCH +signed_by(/users/launch_owner.id) +modifies(/launches)"
        ));
        assert!(output.contains(
            "+APPROVE_GENERAL_AVAILABILITY +signed_by(/users/ga_owner.id) +modifies(/general_availability)"
        ));
        assert!(output.contains(
            "+APPROVE_PRODUCTION +signed_by(/users/production_owner.id) +modifies(/production)"
        ));
        assert!(output.contains(
            "+APPROVE_OPERATIONS +signed_by(/users/operations_owner.id) +modifies(/operations)"
        ));
        assert!(output.contains(
            "+APPROVE_MAINTENANCE +signed_by(/users/maintenance_owner.id) +modifies(/maintenance)"
        ));
        assert!(
            output.contains("+APPROVE_SUPPORT +signed_by(/users/support_owner.id) +modifies(/support)")
        );
        assert!(output.contains(
            "+APPROVE_TRAINING +signed_by(/users/training_owner.id) +modifies(/training)"
        ));
        assert!(output.contains(
            "+APPROVE_COMPLIANCE +signed_by(/users/compliance_owner.id) +modifies(/compliance)"
        ));
        assert!(output.contains(
            "+APPROVE_ONBOARDING +signed_by(/users/onboarding_owner.id) +modifies(/onboarding)"
        ));
        assert!(output.contains(
            "+APPROVE_OFFBOARDING +signed_by(/users/offboarding_owner.id) +modifies(/offboarding)"
        ));
        assert!(output.contains(
            "+APPROVE_DEPROVISIONING +signed_by(/users/access_owner.id) +modifies(/deprovisioning)"
        ));
        assert!(output.contains(
            "+APPROVE_ACCESS_REVIEW +signed_by(/users/access_reviewer.id) +modifies(/access_reviews)"
        ));
        assert!(output.contains(
            "+APPROVE_IDENTITY_VERIFICATION +signed_by(/users/identity_reviewer.id) +modifies(/identity_verifications)"
        ));
        assert!(output.contains(
            "+ISSUE_CREDENTIAL +signed_by(/users/credential_issuer.id) +modifies(/credentials)"
        ));
        assert!(output.contains(
            "+REVOKE_CREDENTIAL +signed_by(/users/credential_issuer.id) +modifies(/credential_revocations)"
        ));
        assert!(output.contains(
            "+RENEW_CREDENTIAL +signed_by(/users/credential_issuer.id) +modifies(/credential_renewals)"
        ));
        assert!(output.contains(
            "+EXPIRE_CREDENTIAL +signed_by(/users/credential_issuer.id) +modifies(/credential_expirations)"
        ));
        assert!(output.contains(
            "+SUSPEND_CREDENTIAL +signed_by(/users/credential_issuer.id) +modifies(/credential_suspensions)"
        ));
        assert!(output.contains(
            "+REINSTATE_CREDENTIAL +signed_by(/users/credential_issuer.id) +modifies(/credential_reinstatements)"
        ));
        assert!(output.contains(
            "+VERIFY_CREDENTIAL +signed_by(/users/credential_verifier.id) +modifies(/credential_verifications)"
        ));
        assert!(output.contains(
            "+PRESENT_CREDENTIAL +signed_by(/users/credential_holder.id) +modifies(/credential_presentations)"
        ));
        assert!(output.contains(
            "+SHARE_CREDENTIAL +signed_by(/users/credential_holder.id) +modifies(/credential_shares)"
        ));
        assert!(output.contains(
            "+EXPORT_CREDENTIAL +signed_by(/users/credential_holder.id) +modifies(/credential_exports)"
        ));
        assert!(output.contains(
            "+REQUEST_CREDENTIAL +signed_by(/users/credential_holder.id) +modifies(/credential_requests)"
        ));
        assert!(output.contains(
            "+ACCEPT_CREDENTIAL +signed_by(/users/credential_holder.id) +modifies(/credential_acceptances)"
        ));
        assert!(output.contains(
            "+REJECT_CREDENTIAL +signed_by(/users/credential_holder.id) +modifies(/credential_rejections)"
        ));
        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_oracle_and_followup_example() {
        let formulas = parse_formula_strings(&[
            "[+RELEASE] true -> (<+oracle_attests(/oracles/delivery.id, \"delivered\", \"true\")> true & eventually(<+DELIVER> true))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);
        let output = format_synthesized_model(&model, "modality").unwrap();

        assert!(output.contains("+RELEASE"));
        assert!(output.contains("+DELIVER"));
        assert!(output.contains("+oracle_attests(/oracles/delivery.id, delivered, true)"));
        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_oracle_and_compound_followups() {
        let formulas = parse_formula_strings(&[
            "[+RELEASE] true -> (<+oracle_attests(/oracles/delivery.id, \"delivered\", \"true\")> true & (eventually(<+DEPOSIT> true) & eventually(<+DELIVER> true)))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);
        let output = format_synthesized_model(&model, "modality").unwrap();

        assert!(output.contains("+RELEASE"));
        assert!(output.contains("+DEPOSIT"));
        assert!(output.contains("+DELIVER"));
        assert!(output.contains("+oracle_attests(/oracles/delivery.id, delivered, true)"));
        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_oracle_and_committed_followup_example() {
        let formulas = parse_formula_strings(&[
            "[+RELEASE] true -> (<+oracle_attests(/oracles/delivery.id, \"delivered\", \"true\")> true & eventually([<+DELIVER>] true))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);
        let output = format_synthesized_model(&model, "modality").unwrap();

        assert!(output.contains("+RELEASE"));
        assert!(output.contains("+DELIVER"));
        assert!(output.contains("+oracle_attests(/oracles/delivery.id, delivered, true)"));
        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_committed_action_oracle_and_followup() {
        let formulas = parse_formula_strings(&[
            "[<+RELEASE>] true -> (<+oracle_attests(/oracles/delivery.id, \"delivered\", \"true\")> true & eventually(<+DELIVER> true))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);
        let output = format_synthesized_model(&model, "modality").unwrap();

        assert!(output.contains("+RELEASE"));
        assert!(output.contains("+DELIVER"));
        assert!(output.contains("+oracle_attests(/oracles/delivery.id, delivered, true)"));
        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_committed_action_oracle_and_committed_followup() {
        let formulas = parse_formula_strings(&[
            "[<+RELEASE>] true -> (<+oracle_attests(/oracles/delivery.id, \"delivered\", \"true\")> true & eventually([<+DELIVER>] true))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);
        let output = format_synthesized_model(&model, "modality").unwrap();

        assert!(output.contains("+RELEASE"));
        assert!(output.contains("+DELIVER"));
        assert!(output.contains("+oracle_attests(/oracles/delivery.id, delivered, true)"));
        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_committed_action_oracle_compound_followups() {
        let formulas = parse_formula_strings(&[
            "[<+RELEASE>] true -> (<+oracle_attests(/oracles/delivery.id, \"delivered\", \"true\")> true & (eventually(<+DEPOSIT> true) & eventually(<+DELIVER> true)))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);
        let output = format_synthesized_model(&model, "modality").unwrap();

        assert!(output.contains("+RELEASE"));
        assert!(output.contains("+DEPOSIT"));
        assert!(output.contains("+DELIVER"));
        assert!(output.contains("+oracle_attests(/oracles/delivery.id, delivered, true)"));
        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_committed_action_oracle_compound_commitments() {
        let formulas = parse_formula_strings(&[
            "[<+RELEASE>] true -> (<+oracle_attests(/oracles/delivery.id, \"delivered\", \"true\")> true & (eventually([<+DEPOSIT>] true) & eventually([<+DELIVER>] true)))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);
        let output = format_synthesized_model(&model, "modality").unwrap();

        assert!(output.contains("+RELEASE"));
        assert!(output.contains("+DEPOSIT"));
        assert!(output.contains("+DELIVER"));
        assert!(output.contains("+oracle_attests(/oracles/delivery.id, delivered, true)"));
        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_oracle_and_compound_committed_followups() {
        let formulas = parse_formula_strings(&[
            "[+RELEASE] true -> (<+oracle_attests(/oracles/delivery.id, \"delivered\", \"true\")> true & (eventually([<+DEPOSIT>] true) & eventually([<+DELIVER>] true)))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);
        let output = format_synthesized_model(&model, "modality").unwrap();

        assert!(output.contains("+RELEASE"));
        assert!(output.contains("+DEPOSIT"));
        assert!(output.contains("+DELIVER"));
        assert!(output.contains("+oracle_attests(/oracles/delivery.id, delivered, true)"));
        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_oracle_and_forbidden_example() {
        let formulas = parse_formula_strings(&[
            "[+DISPUTE] true -> (<+oracle_attests(/oracles/dispute.id, \"opened\", \"true\")> true & always([-RELEASE] true))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);
        let output = format_synthesized_model(&model, "modality").unwrap();

        assert!(output.contains("+DISPUTE"));
        assert!(output.contains("-RELEASE"));
        assert!(output.contains("+oracle_attests(/oracles/dispute.id, opened, true)"));
        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_oracle_and_compound_forbidden_example() {
        let formulas = parse_formula_strings(&[
            "[+DISPUTE] true -> (<+oracle_attests(/oracles/dispute.id, \"opened\", \"true\")> true & (always([-RELEASE] true) & always([-REFUND] true)))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);
        let output = format_synthesized_model(&model, "modality").unwrap();

        assert!(output.contains("+DISPUTE"));
        assert!(output.contains("-RELEASE"));
        assert!(output.contains("-REFUND"));
        assert!(output.contains("+oracle_attests(/oracles/dispute.id, opened, true)"));
        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_committed_action_oracle_and_forbidden_example() {
        let formulas = parse_formula_strings(&[
            "[<+DISPUTE>] true -> (<+oracle_attests(/oracles/dispute.id, \"opened\", \"true\")> true & always([-RELEASE] true))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);
        let output = format_synthesized_model(&model, "modality").unwrap();

        assert!(output.contains("+DISPUTE"));
        assert!(output.contains("-RELEASE"));
        assert!(output.contains("+oracle_attests(/oracles/dispute.id, opened, true)"));
        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_committed_action_oracle_compound_forbidden() {
        let formulas = parse_formula_strings(&[
            "[<+DISPUTE>] true -> (<+oracle_attests(/oracles/dispute.id, \"opened\", \"true\")> true & (always([-RELEASE] true) & always([-REFUND] true)))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);
        let output = format_synthesized_model(&model, "modality").unwrap();

        assert!(output.contains("+DISPUTE"));
        assert!(output.contains("-RELEASE"));
        assert!(output.contains("-REFUND"));
        assert!(output.contains("+oracle_attests(/oracles/dispute.id, opened, true)"));
        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_committed_oracle_and_forbidden_example() {
        let formulas = parse_formula_strings(&[
            "[<+DISPUTE>] true -> ([<+oracle_attests(/oracles/dispute.id, \"opened\", \"true\")>] true & always([-RELEASE] true))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);
        let output = format_synthesized_model(&model, "modality").unwrap();

        assert!(output.contains("+DISPUTE"));
        assert!(output.contains("-RELEASE"));
        assert!(output.contains("+oracle_attests(/oracles/dispute.id, opened, true)"));
        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_committed_oracle_compound_forbidden() {
        let formulas = parse_formula_strings(&[
            "[<+DISPUTE>] true -> ([<+oracle_attests(/oracles/dispute.id, \"opened\", \"true\")>] true & (always([-RELEASE] true) & always([-REFUND] true)))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Contract", &formulas);
        let output = format_synthesized_model(&model, "modality").unwrap();

        assert!(output.contains("+DISPUTE"));
        assert!(output.contains("-RELEASE"));
        assert!(output.contains("-REFUND"));
        assert!(output.contains("+oracle_attests(/oracles/dispute.id, opened, true)"));
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
    fn verify_synthesized_model_accepts_agent_coordination_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+AGENT_A_TURN] true -> eventually(<+AGENT_B_TURN> true))".to_string(),
            "always([+AGENT_B_TURN] true -> eventually(<+AGENT_A_TURN> true))".to_string(),
            "always([+ASSIGN_TASK] true -> <+signed_by(/users/task_requester.id) +signed_by(/users/worker_agent.id)> true)"
                .to_string(),
            "always([+USE_TOOL] true -> (<+signed_by(/users/tool_provider.id)> true & eventually([<+APPROVE_CAPABILITY>] true)))"
                .to_string(),
        ]);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "AgentCoordination",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_escrow_progression_prompt_example() {
        let formulas = parse_formula_strings(&[
            "always([+DELIVER] true -> eventually(<+DEPOSIT> true))".to_string(),
            "always([+RELEASE] true -> eventually(<+DELIVER> true))".to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Escrow", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_procurement_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+CREATE_ORDER] true -> eventually(<+APPROVE_ORDER> true))".to_string(),
            "always([+APPROVE_ORDER] true -> eventually(<+FULFILL_ORDER> true))".to_string(),
            "always([+FULFILL_ORDER] true -> eventually(<+PAY_INVOICE> true))".to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Procurement", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_data_pipeline_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+INGEST_DATA] true -> eventually(<+VALIDATE_DATA> true))".to_string(),
            "always([+VALIDATE_DATA] true -> eventually(<+TRANSFORM_DATA> true))".to_string(),
            "always([+TRANSFORM_DATA] true -> eventually(<+PUBLISH_DATASET> true))".to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("DataPipeline", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_member_onboarding_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+INVITE_MEMBER] true -> eventually(<+ACCEPT_INVITE> true))".to_string(),
            "always([+ACCEPT_INVITE] true -> eventually(<+PROVISION_ACCESS> true))".to_string(),
            "always([+PROVISION_ACCESS] true -> eventually(<+COMPLETE_ONBOARDING> true))".to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("MemberOnboarding", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_release_rollout_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+PLAN_RELEASE] true -> eventually(<+APPROVE_QA> true))".to_string(),
            "always([+APPROVE_QA] true -> eventually(<+ROLLOUT_RELEASE> true))".to_string(),
            "always([+ROLLOUT_RELEASE] true -> eventually(<+MONITOR_RELEASE> true))".to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("ReleaseRollout", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_support_ticket_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+OPEN_TICKET] true -> eventually(<+ASSIGN_AGENT> true))".to_string(),
            "always([+ASSIGN_AGENT] true -> eventually(<+RESPOND_TICKET> true))".to_string(),
            "always([+RESPOND_TICKET] true -> eventually(<+RESOLVE_TICKET> true))".to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("SupportTicket", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_audit_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+START_AUDIT] true -> eventually(<+COLLECT_EVIDENCE> true))".to_string(),
            "always([+COLLECT_EVIDENCE] true -> eventually(<+REVIEW_EVIDENCE> true))".to_string(),
            "always([+REVIEW_EVIDENCE] true -> eventually(<+CLOSE_AUDIT> true))".to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("AuditWorkflow", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_expense_reimbursement_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+SUBMIT_EXPENSE] true -> eventually(<+APPROVE_EXPENSE> true))".to_string(),
            "always([+APPROVE_EXPENSE] true -> eventually(<+REIMBURSE_EXPENSE> true))".to_string(),
            "always([+REIMBURSE_EXPENSE] true -> eventually(<+CLOSE_EXPENSE> true))".to_string(),
        ]);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "ExpenseReimbursement",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_training_certification_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+ENROLL_TRAINING] true -> eventually(<+COMPLETE_TRAINING> true))".to_string(),
            "always([+COMPLETE_TRAINING] true -> eventually(<+PASS_ASSESSMENT> true))".to_string(),
            "always([+PASS_ASSESSMENT] true -> eventually(<+ISSUE_CERTIFICATE> true))".to_string(),
        ]);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "TrainingCertification",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_asset_maintenance_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+SCHEDULE_MAINTENANCE] true -> eventually(<+PERFORM_MAINTENANCE> true))".to_string(),
            "always([+PERFORM_MAINTENANCE] true -> eventually(<+VERIFY_MAINTENANCE> true))".to_string(),
            "always([+VERIFY_MAINTENANCE] true -> eventually(<+CLOSE_MAINTENANCE> true))".to_string(),
        ]);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "AssetMaintenance",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_backup_retention_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+SCHEDULE_BACKUP] true -> eventually(<+RUN_BACKUP> true))".to_string(),
            "always([+RUN_BACKUP] true -> eventually(<+VERIFY_BACKUP> true))".to_string(),
            "always([+VERIFY_BACKUP] true -> eventually(<+ARCHIVE_BACKUP> true))".to_string(),
        ]);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "BackupRetention",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_offboarding_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+REQUEST_OFFBOARDING] true -> eventually(<+REVOKE_ACCESS> true))".to_string(),
            "always([+REVOKE_ACCESS] true -> eventually(<+TRANSFER_OWNERSHIP> true))".to_string(),
            "always([+TRANSFER_OWNERSHIP] true -> eventually(<+CONFIRM_DEPROVISIONING> true))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Offboarding", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_contract_renewal_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+NOTICE_RENEWAL] true -> eventually(<+REVIEW_TERMS> true))".to_string(),
            "always([+REVIEW_TERMS] true -> eventually(<+APPROVE_RENEWAL> true))".to_string(),
            "always([+APPROVE_RENEWAL] true -> eventually(<+EXECUTE_RENEWAL> true))"
                .to_string(),
        ]);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "ContractRenewal",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_credential_issuance_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+REQUEST_CREDENTIAL] true -> eventually(<+VERIFY_IDENTITY> true))"
                .to_string(),
            "always([+VERIFY_IDENTITY] true -> eventually(<+ISSUE_CREDENTIAL> true))"
                .to_string(),
            "always([+ISSUE_CREDENTIAL] true -> eventually(<+ACCEPT_CREDENTIAL> true))"
                .to_string(),
        ]);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "CredentialIssuance",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_access_review_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+START_ACCESS_REVIEW] true -> eventually(<+COLLECT_ACCESS_EVIDENCE> true))"
                .to_string(),
            "always([+COLLECT_ACCESS_EVIDENCE] true -> eventually(<+APPROVE_ACCESS_REVIEW> true))"
                .to_string(),
            "always([+APPROVE_ACCESS_REVIEW] true -> eventually(<+REMEDIATE_ACCESS> true))"
                .to_string(),
        ]);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "AccessReview",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_claim_adjudication_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+SUBMIT_CLAIM] true -> eventually(<+REVIEW_CLAIM> true))".to_string(),
            "always([+REVIEW_CLAIM] true -> eventually(<+APPROVE_CLAIM> true))".to_string(),
            "always([+APPROVE_CLAIM] true -> eventually(<+PAY_CLAIM> true))".to_string(),
        ]);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "ClaimAdjudication",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_privacy_request_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+SUBMIT_PRIVACY_REQUEST] true -> eventually(<+VERIFY_SUBJECT> true))"
                .to_string(),
            "always([+VERIFY_SUBJECT] true -> eventually(<+FULFILL_PRIVACY_REQUEST> true))"
                .to_string(),
            "always([+FULFILL_PRIVACY_REQUEST] true -> eventually(<+CLOSE_PRIVACY_REQUEST> true))"
                .to_string(),
        ]);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "PrivacyRequest",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_data_deletion_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+REQUEST_DELETION] true -> eventually(<+CHECK_RETENTION_POLICY> true))"
                .to_string(),
            "always([+CHECK_RETENTION_POLICY] true -> eventually(<+DELETE_RECORDS> true))"
                .to_string(),
            "always([+DELETE_RECORDS] true -> eventually(<+CONFIRM_DELETION> true))".to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("DataDeletion", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_account_recovery_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+REQUEST_ACCOUNT_RECOVERY] true -> eventually(<+VERIFY_RECOVERY_FACTOR> true))"
                .to_string(),
            "always([+VERIFY_RECOVERY_FACTOR] true -> eventually(<+ROTATE_CREDENTIAL> true))"
                .to_string(),
            "always([+ROTATE_CREDENTIAL] true -> eventually(<+CONFIRM_ACCOUNT_RECOVERY> true))"
                .to_string(),
        ]);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "AccountRecovery",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_consent_change_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+REQUEST_CONSENT_CHANGE] true -> eventually(<+REVIEW_CONSENT_SCOPE> true))"
                .to_string(),
            "always([+REVIEW_CONSENT_SCOPE] true -> eventually(<+APPLY_CONSENT_CHANGE> true))"
                .to_string(),
            "always([+APPLY_CONSENT_CHANGE] true -> eventually(<+CONFIRM_CONSENT_CHANGE> true))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("ConsentChange", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_security_exception_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+OPEN_SECURITY_EXCEPTION] true -> eventually(<+ASSESS_EXCEPTION_RISK> true))"
                .to_string(),
            "always([+ASSESS_EXCEPTION_RISK] true -> eventually(<+APPROVE_EXCEPTION_MITIGATION> true))"
                .to_string(),
            "always([+APPROVE_EXCEPTION_MITIGATION] true -> eventually(<+CLOSE_SECURITY_EXCEPTION> true))"
                .to_string(),
        ]);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "SecurityException",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_vendor_risk_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+START_VENDOR_REVIEW] true -> eventually(<+COLLECT_VENDOR_QUESTIONNAIRE> true))"
                .to_string(),
            "always([+COLLECT_VENDOR_QUESTIONNAIRE] true -> eventually(<+ASSESS_VENDOR_RISK> true))"
                .to_string(),
            "always([+ASSESS_VENDOR_RISK] true -> eventually(<+APPROVE_VENDOR> true))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("VendorRisk", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_data_access_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+REQUEST_DATA_ACCESS] true -> eventually(<+VERIFY_ACCESS_PURPOSE> true))"
                .to_string(),
            "always([+VERIFY_ACCESS_PURPOSE] true -> eventually(<+APPROVE_DATA_ACCESS> true))"
                .to_string(),
            "always([+APPROVE_DATA_ACCESS] true -> eventually(<+LOG_ACCESS_GRANT> true))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("DataAccess", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_data_export_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+REQUEST_DATA_EXPORT] true -> eventually(<+CLASSIFY_EXPORT_DATA> true))"
                .to_string(),
            "always([+CLASSIFY_EXPORT_DATA] true -> eventually(<+APPROVE_DATA_EXPORT> true))"
                .to_string(),
            "always([+APPROVE_DATA_EXPORT] true -> eventually(<+TRANSMIT_EXPORT_PACKAGE> true))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("DataExport", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_data_sharing_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+REQUEST_DATA_SHARE] true -> eventually(<+VERIFY_RECIPIENT_AUTHORITY> true))"
                .to_string(),
            "always([+VERIFY_RECIPIENT_AUTHORITY] true -> eventually(<+APPROVE_DATA_SHARE> true))"
                .to_string(),
            "always([+APPROVE_DATA_SHARE] true -> eventually(<+RECORD_DATA_SHARE> true))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("DataSharing", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_data_use_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+REQUEST_DATA_USE] true -> eventually(<+REVIEW_USE_LIMITS> true))"
                .to_string(),
            "always([+REVIEW_USE_LIMITS] true -> eventually(<+APPROVE_DATA_USE> true))"
                .to_string(),
            "always([+APPROVE_DATA_USE] true -> eventually(<+LOG_DATA_USE> true))".to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("DataUse", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_retention_review_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+START_RETENTION_REVIEW] true -> eventually(<+CLASSIFY_RETENTION_RECORDS> true))"
                .to_string(),
            "always([+CLASSIFY_RETENTION_RECORDS] true -> eventually(<+APPROVE_RETENTION_PLAN> true))"
                .to_string(),
            "always([+APPROVE_RETENTION_PLAN] true -> eventually(<+ENFORCE_RETENTION_PLAN> true))"
                .to_string(),
        ]);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "RetentionReview",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_data_minimization_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+COLLECT_DATA] true -> eventually(<+MINIMIZE_DATASET> true))".to_string(),
            "always([+MINIMIZE_DATASET] true -> eventually(<+APPROVE_MINIMIZED_DATA> true))"
                .to_string(),
            "always([+APPROVE_MINIMIZED_DATA] true -> eventually(<+RECORD_MINIMIZATION> true))"
                .to_string(),
        ]);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "DataMinimization",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_data_anonymization_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+PREPARE_ANALYTICS_DATA] true -> eventually(<+ANONYMIZE_DATASET> true))"
                .to_string(),
            "always([+ANONYMIZE_DATASET] true -> eventually(<+VERIFY_ANONYMIZATION> true))"
                .to_string(),
            "always([+VERIFY_ANONYMIZATION] true -> eventually(<+RELEASE_ANONYMIZED_DATA> true))"
                .to_string(),
        ]);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "DataAnonymization",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_purpose_change_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+REQUEST_PURPOSE_CHANGE] true -> eventually(<+REVIEW_PURPOSE_COMPATIBILITY> true))"
                .to_string(),
            "always([+REVIEW_PURPOSE_COMPATIBILITY] true -> eventually(<+APPROVE_PURPOSE_CHANGE> true))"
                .to_string(),
            "always([+APPROVE_PURPOSE_CHANGE] true -> eventually(<+RECORD_PURPOSE_CHANGE> true))"
                .to_string(),
        ]);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "PurposeChange",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_lawful_basis_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+REQUEST_LAWFUL_BASIS_REVIEW] true -> eventually(<+ASSESS_LAWFUL_BASIS> true))"
                .to_string(),
            "always([+ASSESS_LAWFUL_BASIS] true -> eventually(<+APPROVE_PROCESSING_BASIS> true))"
                .to_string(),
            "always([+APPROVE_PROCESSING_BASIS] true -> eventually(<+RECORD_PROCESSING_BASIS> true))"
                .to_string(),
        ]);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "LawfulBasis",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_data_provenance_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+REGISTER_DATASET] true -> eventually(<+CAPTURE_PROVENANCE> true))"
                .to_string(),
            "always([+CAPTURE_PROVENANCE] true -> eventually(<+VERIFY_PROVENANCE> true))"
                .to_string(),
            "always([+VERIFY_PROVENANCE] true -> eventually(<+APPROVE_PROVENANCE_RECORD> true))"
                .to_string(),
        ]);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "DataProvenance",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_data_quality_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+PROFILE_DATASET] true -> eventually(<+VALIDATE_DATA_QUALITY> true))"
                .to_string(),
            "always([+VALIDATE_DATA_QUALITY] true -> eventually(<+APPROVE_QUALITY_REPORT> true))"
                .to_string(),
            "always([+APPROVE_QUALITY_REPORT] true -> eventually(<+PUBLISH_QUALITY_REPORT> true))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("DataQuality", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_data_classification_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+SUBMIT_DATASET] true -> eventually(<+CLASSIFY_DATASET> true))"
                .to_string(),
            "always([+CLASSIFY_DATASET] true -> eventually(<+APPROVE_DATA_CLASSIFICATION> true))"
                .to_string(),
            "always([+APPROVE_DATA_CLASSIFICATION] true -> eventually(<+RECORD_DATA_CLASSIFICATION> true))"
                .to_string(),
        ]);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "DataClassification",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_dpia_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+START_DPIA] true -> eventually(<+ASSESS_PRIVACY_RISK> true))".to_string(),
            "always([+ASSESS_PRIVACY_RISK] true -> eventually(<+APPROVE_DPIA> true))"
                .to_string(),
            "always([+APPROVE_DPIA] true -> eventually(<+RECORD_DPIA> true))".to_string(),
        ]);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas("Dpia", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_cross_border_transfer_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+REQUEST_CROSS_BORDER_TRANSFER] true -> eventually(<+ASSESS_TRANSFER_MECHANISM> true))"
                .to_string(),
            "always([+ASSESS_TRANSFER_MECHANISM] true -> eventually(<+APPROVE_CROSS_BORDER_TRANSFER> true))"
                .to_string(),
            "always([+APPROVE_CROSS_BORDER_TRANSFER] true -> eventually(<+RECORD_TRANSFER_ASSESSMENT> true))"
                .to_string(),
        ]);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "CrossBorderTransfer",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_subprocessor_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+REGISTER_SUBPROCESSOR] true -> eventually(<+ASSESS_SUBPROCESSOR_RISK> true))"
                .to_string(),
            "always([+ASSESS_SUBPROCESSOR_RISK] true -> eventually(<+APPROVE_SUBPROCESSOR> true))"
                .to_string(),
            "always([+APPROVE_SUBPROCESSOR] true -> eventually(<+RECORD_SUBPROCESSOR> true))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Subprocessor", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_data_localization_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+REQUEST_DATA_LOCALIZATION] true -> eventually(<+ASSESS_RESIDENCY_REQUIREMENT> true))"
                .to_string(),
            "always([+ASSESS_RESIDENCY_REQUIREMENT] true -> eventually(<+APPROVE_LOCALIZATION_PLAN> true))"
                .to_string(),
            "always([+APPROVE_LOCALIZATION_PLAN] true -> eventually(<+RECORD_LOCALIZATION_CONTROL> true))"
                .to_string(),
        ]);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "DataLocalization",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_model_card_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+SUBMIT_MODEL_CARD] true -> eventually(<+EVALUATE_MODEL_RISK> true))"
                .to_string(),
            "always([+EVALUATE_MODEL_RISK] true -> eventually(<+APPROVE_MODEL_DEPLOYMENT> true))"
                .to_string(),
            "always([+APPROVE_MODEL_DEPLOYMENT] true -> eventually(<+PUBLISH_MODEL_CARD> true))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("ModelCard", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_model_evaluation_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+REGISTER_EVALUATION_DATASET] true -> eventually(<+RUN_BIAS_EVALUATION> true))"
                .to_string(),
            "always([+RUN_BIAS_EVALUATION] true -> eventually(<+APPROVE_EVALUATION_REPORT> true))"
                .to_string(),
            "always([+APPROVE_EVALUATION_REPORT] true -> eventually(<+ARCHIVE_EVALUATION_EVIDENCE> true))"
                .to_string(),
        ]);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "ModelEvaluation",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_model_calibration_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+SCHEDULE_MODEL_CALIBRATION] true -> eventually(<+RUN_CALIBRATION_CHECK> true))"
                .to_string(),
            "always([+RUN_CALIBRATION_CHECK] true -> eventually(<+APPROVE_CALIBRATION_REPORT> true))"
                .to_string(),
            "always([+APPROVE_CALIBRATION_REPORT] true -> eventually(<+RECORD_CALIBRATION_RESULT> true))"
                .to_string(),
        ]);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "ModelCalibration",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_model_monitoring_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+START_MODEL_MONITORING] true -> eventually(<+DETECT_MODEL_DRIFT> true))"
                .to_string(),
            "always([+DETECT_MODEL_DRIFT] true -> eventually(<+APPROVE_MODEL_UPDATE> true))"
                .to_string(),
            "always([+APPROVE_MODEL_UPDATE] true -> eventually(<+RECORD_MONITORING_REVIEW> true))"
                .to_string(),
        ]);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "ModelMonitoring",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_model_incident_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+DETECT_MODEL_INCIDENT] true -> eventually(<+ASSESS_MODEL_IMPACT> true))"
                .to_string(),
            "always([+ASSESS_MODEL_IMPACT] true -> eventually(<+APPROVE_MODEL_ROLLBACK> true))"
                .to_string(),
            "always([+APPROVE_MODEL_ROLLBACK] true -> eventually(<+RECORD_MODEL_INCIDENT> true))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("ModelIncident", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_model_retirement_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+REQUEST_MODEL_RETIREMENT] true -> eventually(<+ASSESS_RETIREMENT_IMPACT> true))"
                .to_string(),
            "always([+ASSESS_RETIREMENT_IMPACT] true -> eventually(<+APPROVE_MODEL_RETIREMENT> true))"
                .to_string(),
            "always([+APPROVE_MODEL_RETIREMENT] true -> eventually(<+ARCHIVE_MODEL_ARTIFACTS> true))"
                .to_string(),
        ]);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "ModelRetirement",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_model_retraining_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+COLLECT_RETRAINING_DATA] true -> eventually(<+APPROVE_RETRAINING_PLAN> true))"
                .to_string(),
            "always([+APPROVE_RETRAINING_PLAN] true -> eventually(<+TRAIN_CANDIDATE_MODEL> true))"
                .to_string(),
            "always([+TRAIN_CANDIDATE_MODEL] true -> eventually(<+VALIDATE_CANDIDATE_MODEL> true))"
                .to_string(),
        ]);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "ModelRetraining",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_model_audit_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+LOG_MODEL_DECISION] true -> eventually(<+REVIEW_DECISION_TRACE> true))"
                .to_string(),
            "always([+REVIEW_DECISION_TRACE] true -> eventually(<+APPROVE_MODEL_AUDIT> true))"
                .to_string(),
            "always([+APPROVE_MODEL_AUDIT] true -> eventually(<+RECORD_AUDIT_EVIDENCE> true))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("ModelAudit", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_model_safety_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+START_MODEL_RED_TEAM] true -> eventually(<+REVIEW_RED_TEAM_FINDINGS> true))"
                .to_string(),
            "always([+REVIEW_RED_TEAM_FINDINGS] true -> eventually(<+APPROVE_SAFETY_MITIGATION> true))"
                .to_string(),
            "always([+APPROVE_SAFETY_MITIGATION] true -> eventually(<+RECORD_SAFETY_CASE> true))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("ModelSafety", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_model_version_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+REGISTER_MODEL_VERSION] true -> eventually(<+RUN_MODEL_VALIDATION> true))"
                .to_string(),
            "always([+RUN_MODEL_VALIDATION] true -> eventually(<+APPROVE_MODEL_VERSION> true))"
                .to_string(),
            "always([+APPROVE_MODEL_VERSION] true -> eventually(<+PROMOTE_MODEL_VERSION> true))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("ModelVersion", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_model_lineage_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+CAPTURE_MODEL_LINEAGE] true -> eventually(<+REVIEW_LINEAGE_REPORT> true))"
                .to_string(),
            "always([+REVIEW_LINEAGE_REPORT] true -> eventually(<+APPROVE_LINEAGE_RECORD> true))"
                .to_string(),
            "always([+APPROVE_LINEAGE_RECORD] true -> eventually(<+ARCHIVE_LINEAGE_RECORD> true))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("ModelLineage", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_model_artifact_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+SUBMIT_MODEL_ARTIFACT] true -> eventually(<+SCAN_MODEL_ARTIFACT> true))"
                .to_string(),
            "always([+SCAN_MODEL_ARTIFACT] true -> eventually(<+APPROVE_MODEL_ARTIFACT> true))"
                .to_string(),
            "always([+APPROVE_MODEL_ARTIFACT] true -> eventually(<+PUBLISH_MODEL_ARTIFACT> true))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("ModelArtifact", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_model_endpoint_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+REGISTER_MODEL_ENDPOINT] true -> eventually(<+RUN_ENDPOINT_SMOKE_TEST> true))"
                .to_string(),
            "always([+RUN_ENDPOINT_SMOKE_TEST] true -> eventually(<+APPROVE_ENDPOINT_ACTIVATION> true))"
                .to_string(),
            "always([+APPROVE_ENDPOINT_ACTIVATION] true -> eventually(<+ACTIVATE_MODEL_ENDPOINT> true))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("ModelEndpoint", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_model_canary_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+PLAN_MODEL_CANARY] true -> eventually(<+MONITOR_CANARY_METRICS> true))"
                .to_string(),
            "always([+MONITOR_CANARY_METRICS] true -> eventually(<+APPROVE_FULL_ROLLOUT> true))"
                .to_string(),
            "always([+APPROVE_FULL_ROLLOUT] true -> eventually(<+EXPAND_MODEL_TRAFFIC> true))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("ModelCanary", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_model_drift_response_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+DETECT_MODEL_DRIFT] true -> eventually(<+ASSESS_DRIFT_IMPACT> true))"
                .to_string(),
            "always([+ASSESS_DRIFT_IMPACT] true -> eventually(<+APPROVE_DRIFT_RESPONSE> true))"
                .to_string(),
            "always([+APPROVE_DRIFT_RESPONSE] true -> eventually(<+RECORD_DRIFT_RESPONSE> true))"
                .to_string(),
        ]);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "ModelDriftResponse",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_model_shadow_promotion_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+START_SHADOW_EVALUATION] true -> eventually(<+COMPARE_SHADOW_OUTPUT> true))"
                .to_string(),
            "always([+COMPARE_SHADOW_OUTPUT] true -> eventually(<+APPROVE_SHADOW_PROMOTION> true))"
                .to_string(),
            "always([+APPROVE_SHADOW_PROMOTION] true -> eventually(<+PROMOTE_SHADOW_MODEL> true))"
                .to_string(),
        ]);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "ModelShadowPromotion",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_model_rollback_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+DETECT_MODEL_REGRESSION] true -> eventually(<+ASSESS_ROLLBACK_RISK> true))"
                .to_string(),
            "always([+ASSESS_ROLLBACK_RISK] true -> eventually(<+APPROVE_MODEL_ROLLBACK_PLAN> true))"
                .to_string(),
            "always([+APPROVE_MODEL_ROLLBACK_PLAN] true -> eventually(<+EXECUTE_MODEL_ROLLBACK> true))"
                .to_string(),
        ]);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "ModelRollback",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_model_exception_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+REQUEST_MODEL_EXCEPTION] true -> eventually(<+ASSESS_MODEL_EXCEPTION> true))"
                .to_string(),
            "always([+ASSESS_MODEL_EXCEPTION] true -> eventually(<+APPROVE_MODEL_EXCEPTION> true))"
                .to_string(),
            "always([+APPROVE_MODEL_EXCEPTION] true -> eventually(<+RECORD_MODEL_EXCEPTION> true))"
                .to_string(),
        ]);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "ModelException",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_model_deprecation_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+REQUEST_MODEL_DEPRECATION] true -> eventually(<+ASSESS_DEPRECATION_IMPACT> true))"
                .to_string(),
            "always([+ASSESS_DEPRECATION_IMPACT] true -> eventually(<+APPROVE_MODEL_DEPRECATION> true))"
                .to_string(),
            "always([+APPROVE_MODEL_DEPRECATION] true -> eventually(<+RECORD_MODEL_DEPRECATION> true))"
                .to_string(),
        ]);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "ModelDeprecation",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_model_attestation_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+REQUEST_MODEL_ATTESTATION] true -> eventually(<+COLLECT_ATTESTATION_EVIDENCE> true))"
                .to_string(),
            "always([+COLLECT_ATTESTATION_EVIDENCE] true -> eventually(<+APPROVE_MODEL_ATTESTATION> true))"
                .to_string(),
            "always([+APPROVE_MODEL_ATTESTATION] true -> eventually(<+PUBLISH_MODEL_ATTESTATION> true))"
                .to_string(),
        ]);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "ModelAttestation",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_model_disclosure_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+REQUEST_MODEL_DISCLOSURE] true -> eventually(<+REVIEW_DISCLOSURE_SCOPE> true))"
                .to_string(),
            "always([+REVIEW_DISCLOSURE_SCOPE] true -> eventually(<+APPROVE_MODEL_DISCLOSURE> true))"
                .to_string(),
            "always([+APPROVE_MODEL_DISCLOSURE] true -> eventually(<+PUBLISH_MODEL_DISCLOSURE> true))"
                .to_string(),
        ]);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "ModelDisclosure",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_model_appeal_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+REQUEST_MODEL_APPEAL] true -> eventually(<+REVIEW_MODEL_APPEAL> true))"
                .to_string(),
            "always([+REVIEW_MODEL_APPEAL] true -> eventually(<+APPROVE_MODEL_APPEAL> true))"
                .to_string(),
            "always([+APPROVE_MODEL_APPEAL] true -> eventually(<+RECORD_MODEL_APPEAL> true))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("ModelAppeal", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_model_override_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+REQUEST_MODEL_OVERRIDE] true -> eventually(<+REVIEW_OVERRIDE_RISK> true))"
                .to_string(),
            "always([+REVIEW_OVERRIDE_RISK] true -> eventually(<+APPROVE_MODEL_OVERRIDE> true))"
                .to_string(),
            "always([+APPROVE_MODEL_OVERRIDE] true -> eventually(<+RECORD_OVERRIDE_AUDIT> true))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("ModelOverride", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_agent_action_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+REQUEST_AGENT_ACTION] true -> eventually(<+SIMULATE_AGENT_ACTION> true))"
                .to_string(),
            "always([+SIMULATE_AGENT_ACTION] true -> eventually(<+APPROVE_AGENT_ACTION> true))"
                .to_string(),
            "always([+APPROVE_AGENT_ACTION] true -> eventually(<+EXECUTE_AGENT_ACTION> true))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("AgentAction", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_tool_permission_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+REQUEST_TOOL_PERMISSION] true -> eventually(<+ASSESS_TOOL_RISK> true))"
                .to_string(),
            "always([+ASSESS_TOOL_RISK] true -> eventually(<+APPROVE_TOOL_PERMISSION> true))"
                .to_string(),
            "always([+APPROVE_TOOL_PERMISSION] true -> eventually(<+GRANT_TOOL_PERMISSION> true))"
                .to_string(),
        ]);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "ToolPermission",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_agent_task_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+DELEGATE_AGENT_TASK] true -> eventually(<+REVIEW_AGENT_OUTPUT> true))"
                .to_string(),
            "always([+REVIEW_AGENT_OUTPUT] true -> eventually(<+APPROVE_AGENT_OUTPUT> true))"
                .to_string(),
            "always([+APPROVE_AGENT_OUTPUT] true -> eventually(<+ARCHIVE_AGENT_TRACE> true))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("AgentTask", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_agent_policy_exception_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+REQUEST_AGENT_POLICY_EXCEPTION] true -> eventually(<+ASSESS_AGENT_POLICY_RISK> true))"
                .to_string(),
            "always([+ASSESS_AGENT_POLICY_RISK] true -> eventually(<+APPROVE_AGENT_POLICY_EXCEPTION> true))"
                .to_string(),
            "always([+APPROVE_AGENT_POLICY_EXCEPTION] true -> eventually(<+RECORD_AGENT_POLICY_EXCEPTION> true))"
                .to_string(),
        ]);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "AgentPolicyException",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_sandbox_session_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+REQUEST_SANDBOX_SESSION] true -> eventually(<+APPROVE_SANDBOX_BOUNDARY> true))"
                .to_string(),
            "always([+APPROVE_SANDBOX_BOUNDARY] true -> eventually(<+GRANT_SANDBOX_SESSION> true))"
                .to_string(),
            "always([+GRANT_SANDBOX_SESSION] true -> eventually(<+RECORD_SANDBOX_AUDIT> true))"
                .to_string(),
        ]);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "SandboxSession",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_agent_capability_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+REQUEST_AGENT_CAPABILITY] true -> eventually(<+EVALUATE_CAPABILITY_SCOPE> true))"
                .to_string(),
            "always([+EVALUATE_CAPABILITY_SCOPE] true -> eventually(<+APPROVE_AGENT_CAPABILITY> true))"
                .to_string(),
            "always([+APPROVE_AGENT_CAPABILITY] true -> eventually(<+ENABLE_AGENT_CAPABILITY> true))"
                .to_string(),
        ]);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "AgentCapability",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_agent_memory_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+PROPOSE_AGENT_MEMORY] true -> eventually(<+REVIEW_MEMORY_SCOPE> true))"
                .to_string(),
            "always([+REVIEW_MEMORY_SCOPE] true -> eventually(<+APPROVE_MEMORY_WRITE> true))"
                .to_string(),
            "always([+APPROVE_MEMORY_WRITE] true -> eventually(<+COMMIT_AGENT_MEMORY> true))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("AgentMemory", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_agent_handoff_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+REQUEST_AGENT_HANDOFF] true -> eventually(<+PACKAGE_AGENT_CONTEXT> true))"
                .to_string(),
            "always([+PACKAGE_AGENT_CONTEXT] true -> eventually(<+APPROVE_AGENT_HANDOFF> true))"
                .to_string(),
            "always([+APPROVE_AGENT_HANDOFF] true -> eventually(<+ACCEPT_AGENT_HANDOFF> true))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("AgentHandoff", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_external_tool_call_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+REQUEST_EXTERNAL_TOOL_CALL] true -> eventually(<+ASSESS_TOOL_CALL_RISK> true))"
                .to_string(),
            "always([+ASSESS_TOOL_CALL_RISK] true -> eventually(<+APPROVE_EXTERNAL_TOOL_CALL> true))"
                .to_string(),
            "always([+APPROVE_EXTERNAL_TOOL_CALL] true -> eventually(<+EXECUTE_EXTERNAL_TOOL_CALL> true))"
                .to_string(),
        ]);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "ExternalToolCall",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_agent_credential_rotation_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+REQUEST_AGENT_CREDENTIAL_ROTATION] true -> eventually(<+VERIFY_AGENT_IDENTITY> true))"
                .to_string(),
            "always([+VERIFY_AGENT_IDENTITY] true -> eventually(<+APPROVE_AGENT_CREDENTIAL_ROTATION> true))"
                .to_string(),
            "always([+APPROVE_AGENT_CREDENTIAL_ROTATION] true -> eventually(<+ROTATE_AGENT_CREDENTIAL> true))"
                .to_string(),
        ]);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "AgentCredentialRotation",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_agent_incident_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+REPORT_AGENT_INCIDENT] true -> eventually(<+CONTAIN_AGENT_SESSION> true))"
                .to_string(),
            "always([+CONTAIN_AGENT_SESSION] true -> eventually(<+APPROVE_AGENT_REMEDIATION> true))"
                .to_string(),
            "always([+APPROVE_AGENT_REMEDIATION] true -> eventually(<+RECORD_AGENT_INCIDENT> true))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("AgentIncident", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_agent_permission_revoke_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+REQUEST_AGENT_PERMISSION_REVOKE] true -> eventually(<+ASSESS_PERMISSION_DEPENDENCIES> true))"
                .to_string(),
            "always([+ASSESS_PERMISSION_DEPENDENCIES] true -> eventually(<+APPROVE_AGENT_PERMISSION_REVOKE> true))"
                .to_string(),
            "always([+APPROVE_AGENT_PERMISSION_REVOKE] true -> eventually(<+REVOKE_AGENT_PERMISSION> true))"
                .to_string(),
        ]);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "AgentPermissionRevoke",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_agent_data_egress_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+REQUEST_AGENT_DATA_EGRESS] true -> eventually(<+CLASSIFY_AGENT_OUTPUT> true))"
                .to_string(),
            "always([+CLASSIFY_AGENT_OUTPUT] true -> eventually(<+APPROVE_AGENT_DATA_EGRESS> true))"
                .to_string(),
            "always([+APPROVE_AGENT_DATA_EGRESS] true -> eventually(<+RELEASE_AGENT_OUTPUT> true))"
                .to_string(),
        ]);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "AgentDataEgress",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_agent_autonomy_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+REQUEST_AGENT_AUTONOMY] true -> eventually(<+ASSESS_AUTONOMY_RISK> true))"
                .to_string(),
            "always([+ASSESS_AUTONOMY_RISK] true -> eventually(<+APPROVE_AGENT_AUTONOMY> true))"
                .to_string(),
            "always([+APPROVE_AGENT_AUTONOMY] true -> eventually(<+ENABLE_AGENT_AUTONOMY> true))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("AgentAutonomy", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_agent_publication_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+PROPOSE_AGENT_PUBLICATION] true -> eventually(<+REVIEW_AGENT_CLAIMS> true))"
                .to_string(),
            "always([+REVIEW_AGENT_CLAIMS] true -> eventually(<+APPROVE_AGENT_PUBLICATION> true))"
                .to_string(),
            "always([+APPROVE_AGENT_PUBLICATION] true -> eventually(<+PUBLISH_AGENT_OUTPUT> true))"
                .to_string(),
        ]);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "AgentPublication",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_agent_secret_access_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+REQUEST_AGENT_SECRET_ACCESS] true -> eventually(<+REVIEW_SECRET_SCOPE> true))"
                .to_string(),
            "always([+REVIEW_SECRET_SCOPE] true -> eventually(<+APPROVE_AGENT_SECRET_ACCESS> true))"
                .to_string(),
            "always([+APPROVE_AGENT_SECRET_ACCESS] true -> eventually(<+GRANT_AGENT_SECRET_ACCESS> true))"
                .to_string(),
        ]);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "AgentSecretAccess",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_agent_model_access_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+REQUEST_AGENT_MODEL_ACCESS] true -> eventually(<+REVIEW_MODEL_ACCESS_SCOPE> true))"
                .to_string(),
            "always([+REVIEW_MODEL_ACCESS_SCOPE] true -> eventually(<+APPROVE_AGENT_MODEL_ACCESS> true))"
                .to_string(),
            "always([+APPROVE_AGENT_MODEL_ACCESS] true -> eventually(<+GRANT_AGENT_MODEL_ACCESS> true))"
                .to_string(),
        ]);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "AgentModelAccess",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_agent_spend_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+REQUEST_AGENT_SPEND] true -> eventually(<+ESTIMATE_AGENT_SPEND_RISK> true))"
                .to_string(),
            "always([+ESTIMATE_AGENT_SPEND_RISK] true -> eventually(<+APPROVE_AGENT_SPEND> true))"
                .to_string(),
            "always([+APPROVE_AGENT_SPEND] true -> eventually(<+EXECUTE_AGENT_SPEND> true))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("AgentSpend", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_agent_prompt_injection_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+DETECT_PROMPT_INJECTION] true -> eventually(<+QUARANTINE_AGENT_CONTEXT> true))"
                .to_string(),
            "always([+QUARANTINE_AGENT_CONTEXT] true -> eventually(<+APPROVE_CONTEXT_RESTORATION> true))"
                .to_string(),
            "always([+APPROVE_CONTEXT_RESTORATION] true -> eventually(<+RESTORE_AGENT_CONTEXT> true))"
                .to_string(),
        ]);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "AgentPromptInjection",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_agent_network_access_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+REQUEST_AGENT_NETWORK_ACCESS] true -> eventually(<+ASSESS_NETWORK_SCOPE> true))"
                .to_string(),
            "always([+ASSESS_NETWORK_SCOPE] true -> eventually(<+APPROVE_AGENT_NETWORK_ACCESS> true))"
                .to_string(),
            "always([+APPROVE_AGENT_NETWORK_ACCESS] true -> eventually(<+ENABLE_AGENT_NETWORK_ACCESS> true))"
                .to_string(),
        ]);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "AgentNetworkAccess",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_agent_state_export_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+REQUEST_AGENT_STATE_EXPORT] true -> eventually(<+REDACT_AGENT_STATE> true))"
                .to_string(),
            "always([+REDACT_AGENT_STATE] true -> eventually(<+APPROVE_AGENT_STATE_EXPORT> true))"
                .to_string(),
            "always([+APPROVE_AGENT_STATE_EXPORT] true -> eventually(<+EXPORT_AGENT_STATE> true))"
                .to_string(),
        ]);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "AgentStateExport",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_agent_dependency_update_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+PROPOSE_AGENT_DEPENDENCY_UPDATE] true -> eventually(<+SCAN_AGENT_DEPENDENCY> true))"
                .to_string(),
            "always([+SCAN_AGENT_DEPENDENCY] true -> eventually(<+APPROVE_AGENT_DEPENDENCY_UPDATE> true))"
                .to_string(),
            "always([+APPROVE_AGENT_DEPENDENCY_UPDATE] true -> eventually(<+APPLY_AGENT_DEPENDENCY_UPDATE> true))"
                .to_string(),
        ]);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "AgentDependencyUpdate",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_agent_identity_binding_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+REQUEST_AGENT_IDENTITY_BINDING] true -> eventually(<+VERIFY_AGENT_ATTESTATION> true))"
                .to_string(),
            "always([+VERIFY_AGENT_ATTESTATION] true -> eventually(<+APPROVE_AGENT_IDENTITY_BINDING> true))"
                .to_string(),
            "always([+APPROVE_AGENT_IDENTITY_BINDING] true -> eventually(<+BIND_AGENT_IDENTITY> true))"
                .to_string(),
        ]);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "AgentIdentityBinding",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_agent_runtime_migration_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+REQUEST_AGENT_RUNTIME_MIGRATION] true -> eventually(<+SNAPSHOT_AGENT_RUNTIME> true))"
                .to_string(),
            "always([+SNAPSHOT_AGENT_RUNTIME] true -> eventually(<+APPROVE_AGENT_RUNTIME_MIGRATION> true))"
                .to_string(),
            "always([+APPROVE_AGENT_RUNTIME_MIGRATION] true -> eventually(<+MIGRATE_AGENT_RUNTIME> true))"
                .to_string(),
        ]);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "AgentRuntimeMigration",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_agent_rollback_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+REQUEST_AGENT_ROLLBACK] true -> eventually(<+VERIFY_ROLLBACK_POINT> true))"
                .to_string(),
            "always([+VERIFY_ROLLBACK_POINT] true -> eventually(<+APPROVE_AGENT_ROLLBACK> true))"
                .to_string(),
            "always([+APPROVE_AGENT_ROLLBACK] true -> eventually(<+ROLLBACK_AGENT_STATE> true))"
                .to_string(),
        ]);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "AgentRollback",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_agent_telemetry_access_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+REQUEST_AGENT_TELEMETRY_ACCESS] true -> eventually(<+REDACT_AGENT_TELEMETRY> true))"
                .to_string(),
            "always([+REDACT_AGENT_TELEMETRY] true -> eventually(<+APPROVE_AGENT_TELEMETRY_ACCESS> true))"
                .to_string(),
            "always([+APPROVE_AGENT_TELEMETRY_ACCESS] true -> eventually(<+EXPORT_AGENT_TELEMETRY> true))"
                .to_string(),
        ]);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "AgentTelemetryAccess",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_agent_session_resume_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+REQUEST_AGENT_SESSION_RESUME] true -> eventually(<+VALIDATE_SESSION_CHECKPOINT> true))"
                .to_string(),
            "always([+VALIDATE_SESSION_CHECKPOINT] true -> eventually(<+APPROVE_AGENT_SESSION_RESUME> true))"
                .to_string(),
            "always([+APPROVE_AGENT_SESSION_RESUME] true -> eventually(<+RESUME_AGENT_SESSION> true))"
                .to_string(),
        ]);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "AgentSessionResume",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_agent_backup_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+REQUEST_AGENT_BACKUP] true -> eventually(<+VERIFY_BACKUP_SCOPE> true))"
                .to_string(),
            "always([+VERIFY_BACKUP_SCOPE] true -> eventually(<+APPROVE_AGENT_BACKUP> true))"
                .to_string(),
            "always([+APPROVE_AGENT_BACKUP] true -> eventually(<+CREATE_AGENT_BACKUP> true))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("AgentBackup", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_agent_log_retention_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+REQUEST_AGENT_LOG_RETENTION] true -> eventually(<+CLASSIFY_AGENT_LOGS> true))"
                .to_string(),
            "always([+CLASSIFY_AGENT_LOGS] true -> eventually(<+APPROVE_AGENT_LOG_RETENTION> true))"
                .to_string(),
            "always([+APPROVE_AGENT_LOG_RETENTION] true -> eventually(<+ENFORCE_AGENT_LOG_RETENTION> true))"
                .to_string(),
        ]);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "AgentLogRetention",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_agent_state_purge_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+REQUEST_AGENT_STATE_PURGE] true -> eventually(<+REVIEW_PURGE_SCOPE> true))"
                .to_string(),
            "always([+REVIEW_PURGE_SCOPE] true -> eventually(<+APPROVE_AGENT_STATE_PURGE> true))"
                .to_string(),
            "always([+APPROVE_AGENT_STATE_PURGE] true -> eventually(<+PURGE_AGENT_STATE> true))"
                .to_string(),
        ]);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "AgentStatePurge",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_agent_audit_disclosure_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+REQUEST_AGENT_AUDIT_DISCLOSURE] true -> eventually(<+REDACT_AGENT_AUDIT_LOG> true))"
                .to_string(),
            "always([+REDACT_AGENT_AUDIT_LOG] true -> eventually(<+APPROVE_AGENT_AUDIT_DISCLOSURE> true))"
                .to_string(),
            "always([+APPROVE_AGENT_AUDIT_DISCLOSURE] true -> eventually(<+DISCLOSE_AGENT_AUDIT_LOG> true))"
                .to_string(),
        ]);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "AgentAuditDisclosure",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_agent_environment_teardown_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+REQUEST_AGENT_ENVIRONMENT_TEARDOWN] true -> eventually(<+SNAPSHOT_AGENT_ENVIRONMENT> true))"
                .to_string(),
            "always([+SNAPSHOT_AGENT_ENVIRONMENT] true -> eventually(<+APPROVE_AGENT_ENVIRONMENT_TEARDOWN> true))"
                .to_string(),
            "always([+APPROVE_AGENT_ENVIRONMENT_TEARDOWN] true -> eventually(<+TEARDOWN_AGENT_ENVIRONMENT> true))"
                .to_string(),
        ]);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "AgentEnvironmentTeardown",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_agent_cache_invalidation_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+REQUEST_AGENT_CACHE_INVALIDATION] true -> eventually(<+ASSESS_CACHE_DEPENDENCIES> true))"
                .to_string(),
            "always([+ASSESS_CACHE_DEPENDENCIES] true -> eventually(<+APPROVE_AGENT_CACHE_INVALIDATION> true))"
                .to_string(),
            "always([+APPROVE_AGENT_CACHE_INVALIDATION] true -> eventually(<+INVALIDATE_AGENT_CACHE> true))"
                .to_string(),
        ]);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "AgentCacheInvalidation",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_agent_context_compaction_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+REQUEST_AGENT_CONTEXT_COMPACTION] true -> eventually(<+SUMMARIZE_AGENT_CONTEXT> true))"
                .to_string(),
            "always([+SUMMARIZE_AGENT_CONTEXT] true -> eventually(<+APPROVE_AGENT_CONTEXT_COMPACTION> true))"
                .to_string(),
            "always([+APPROVE_AGENT_CONTEXT_COMPACTION] true -> eventually(<+COMPACT_AGENT_CONTEXT> true))"
                .to_string(),
        ]);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "AgentContextCompaction",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_agent_workspace_handover_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+REQUEST_AGENT_WORKSPACE_HANDOVER] true -> eventually(<+INVENTORY_WORKSPACE_STATE> true))"
                .to_string(),
            "always([+INVENTORY_WORKSPACE_STATE] true -> eventually(<+APPROVE_AGENT_WORKSPACE_HANDOVER> true))"
                .to_string(),
            "always([+APPROVE_AGENT_WORKSPACE_HANDOVER] true -> eventually(<+HANDOVER_AGENT_WORKSPACE> true))"
                .to_string(),
        ]);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "AgentWorkspaceHandover",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_agent_knowledge_refresh_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+REQUEST_AGENT_KNOWLEDGE_REFRESH] true -> eventually(<+REVIEW_KNOWLEDGE_SOURCES> true))"
                .to_string(),
            "always([+REVIEW_KNOWLEDGE_SOURCES] true -> eventually(<+APPROVE_AGENT_KNOWLEDGE_REFRESH> true))"
                .to_string(),
            "always([+APPROVE_AGENT_KNOWLEDGE_REFRESH] true -> eventually(<+REFRESH_AGENT_KNOWLEDGE> true))"
                .to_string(),
        ]);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "AgentKnowledgeRefresh",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_agent_delegation_renewal_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+REQUEST_AGENT_DELEGATION_RENEWAL] true -> eventually(<+REVIEW_DELEGATION_SCOPE> true))"
                .to_string(),
            "always([+REVIEW_DELEGATION_SCOPE] true -> eventually(<+APPROVE_AGENT_DELEGATION_RENEWAL> true))"
                .to_string(),
            "always([+APPROVE_AGENT_DELEGATION_RENEWAL] true -> eventually(<+RENEW_AGENT_DELEGATION> true))"
                .to_string(),
        ]);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "AgentDelegationRenewal",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_agent_policy_drift_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+DETECT_AGENT_POLICY_DRIFT] true -> eventually(<+ASSESS_POLICY_DRIFT> true))"
                .to_string(),
            "always([+ASSESS_POLICY_DRIFT] true -> eventually(<+APPROVE_POLICY_DRIFT_REMEDIATION> true))"
                .to_string(),
            "always([+APPROVE_POLICY_DRIFT_REMEDIATION] true -> eventually(<+REMEDIATE_AGENT_POLICY> true))"
                .to_string(),
        ]);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "AgentPolicyDrift",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_agent_performance_review_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+REQUEST_AGENT_PERFORMANCE_REVIEW] true -> eventually(<+COLLECT_AGENT_METRICS> true))"
                .to_string(),
            "always([+COLLECT_AGENT_METRICS] true -> eventually(<+APPROVE_AGENT_PERFORMANCE_REVIEW> true))"
                .to_string(),
            "always([+APPROVE_AGENT_PERFORMANCE_REVIEW] true -> eventually(<+RECORD_AGENT_PERFORMANCE_REVIEW> true))"
                .to_string(),
        ]);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "AgentPerformanceReview",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_agent_budget_increase_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+REQUEST_AGENT_BUDGET_INCREASE] true -> eventually(<+ASSESS_AGENT_BUDGET_IMPACT> true))"
                .to_string(),
            "always([+ASSESS_AGENT_BUDGET_IMPACT] true -> eventually(<+APPROVE_AGENT_BUDGET_INCREASE> true))"
                .to_string(),
            "always([+APPROVE_AGENT_BUDGET_INCREASE] true -> eventually(<+APPLY_AGENT_BUDGET> true))"
                .to_string(),
        ]);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "AgentBudgetIncrease",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_agent_rate_limit_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+REQUEST_AGENT_RATE_LIMIT_CHANGE] true -> eventually(<+ASSESS_AGENT_RATE_LIMIT_RISK> true))"
                .to_string(),
            "always([+ASSESS_AGENT_RATE_LIMIT_RISK] true -> eventually(<+APPROVE_AGENT_RATE_LIMIT_CHANGE> true))"
                .to_string(),
            "always([+APPROVE_AGENT_RATE_LIMIT_CHANGE] true -> eventually(<+APPLY_AGENT_RATE_LIMIT> true))"
                .to_string(),
        ]);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "AgentRateLimit",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_agent_prompt_template_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+REQUEST_AGENT_PROMPT_TEMPLATE_CHANGE] true -> eventually(<+REVIEW_PROMPT_TEMPLATE_DIFF> true))"
                .to_string(),
            "always([+REVIEW_PROMPT_TEMPLATE_DIFF] true -> eventually(<+APPROVE_AGENT_PROMPT_TEMPLATE_CHANGE> true))"
                .to_string(),
            "always([+APPROVE_AGENT_PROMPT_TEMPLATE_CHANGE] true -> eventually(<+APPLY_AGENT_PROMPT_TEMPLATE> true))"
                .to_string(),
        ]);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "AgentPromptTemplate",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_agent_guardrail_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+REQUEST_AGENT_GUARDRAIL_CHANGE] true -> eventually(<+TEST_AGENT_GUARDRAIL> true))"
                .to_string(),
            "always([+TEST_AGENT_GUARDRAIL] true -> eventually(<+APPROVE_AGENT_GUARDRAIL_CHANGE> true))"
                .to_string(),
            "always([+APPROVE_AGENT_GUARDRAIL_CHANGE] true -> eventually(<+APPLY_AGENT_GUARDRAIL> true))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("AgentGuardrail", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_agent_evaluator_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+REQUEST_AGENT_EVALUATOR_CHANGE] true -> eventually(<+VALIDATE_AGENT_EVALUATOR> true))"
                .to_string(),
            "always([+VALIDATE_AGENT_EVALUATOR] true -> eventually(<+APPROVE_AGENT_EVALUATOR_CHANGE> true))"
                .to_string(),
            "always([+APPROVE_AGENT_EVALUATOR_CHANGE] true -> eventually(<+APPLY_AGENT_EVALUATOR> true))"
                .to_string(),
        ]);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "AgentEvaluator",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_human_review_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+REQUEST_HUMAN_REVIEW] true -> eventually(<+TRIAGE_REVIEW_REQUEST> true))"
                .to_string(),
            "always([+TRIAGE_REVIEW_REQUEST] true -> eventually(<+APPROVE_HUMAN_REVIEW> true))"
                .to_string(),
            "always([+APPROVE_HUMAN_REVIEW] true -> eventually(<+RECORD_REVIEW_OUTCOME> true))"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("HumanReview", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_decision_explanation_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+REQUEST_DECISION_EXPLANATION] true -> eventually(<+COLLECT_DECISION_FACTORS> true))"
                .to_string(),
            "always([+COLLECT_DECISION_FACTORS] true -> eventually(<+APPROVE_DECISION_EXPLANATION> true))"
                .to_string(),
            "always([+APPROVE_DECISION_EXPLANATION] true -> eventually(<+DELIVER_DECISION_EXPLANATION> true))"
                .to_string(),
        ]);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "DecisionExplanation",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_decision_correction_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+REQUEST_DECISION_CORRECTION] true -> eventually(<+REVIEW_DECISION_ERROR> true))"
                .to_string(),
            "always([+REVIEW_DECISION_ERROR] true -> eventually(<+APPROVE_DECISION_CORRECTION> true))"
                .to_string(),
            "always([+APPROVE_DECISION_CORRECTION] true -> eventually(<+RECORD_DECISION_CORRECTION> true))"
                .to_string(),
        ]);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "DecisionCorrection",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_decision_recourse_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+REQUEST_DECISION_RECOURSE] true -> eventually(<+REVIEW_RECOURSE_OPTIONS> true))"
                .to_string(),
            "always([+REVIEW_RECOURSE_OPTIONS] true -> eventually(<+APPROVE_RECOURSE_PLAN> true))"
                .to_string(),
            "always([+APPROVE_RECOURSE_PLAN] true -> eventually(<+RECORD_RECOURSE_OUTCOME> true))"
                .to_string(),
        ]);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "DecisionRecourse",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_adverse_action_notice_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+REQUEST_ADVERSE_ACTION_NOTICE] true -> eventually(<+COMPILE_NOTICE_EVIDENCE> true))"
                .to_string(),
            "always([+COMPILE_NOTICE_EVIDENCE] true -> eventually(<+APPROVE_ADVERSE_ACTION_NOTICE> true))"
                .to_string(),
            "always([+APPROVE_ADVERSE_ACTION_NOTICE] true -> eventually(<+DELIVER_ADVERSE_ACTION_NOTICE> true))"
                .to_string(),
        ]);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "AdverseActionNotice",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_automated_decision_contest_ordering_prompt_examples() {
        let formulas = parse_formula_strings(&[
            "always([+CONTEST_AUTOMATED_DECISION] true -> eventually(<+REVIEW_CONTEST_EVIDENCE> true))"
                .to_string(),
            "always([+REVIEW_CONTEST_EVIDENCE] true -> eventually(<+APPROVE_CONTEST_RESOLUTION> true))"
                .to_string(),
            "always([+APPROVE_CONTEST_RESOLUTION] true -> eventually(<+RECORD_CONTEST_RESOLUTION> true))"
                .to_string(),
        ]);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "AutomatedDecisionContest",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_dispute_resolution_prompt_example() {
        let formulas = parse_formula_strings(&[
            "always([+DISPUTE] true -> (always([-RELEASE] true) & always([-REFUND] true)))"
                .to_string(),
            "always([+RESOLVE_DISPUTE] true -> <+signed_by(/users/arbiter.id)> true)"
                .to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Dispute", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_cancellation_prompt_example() {
        let formulas = parse_formula_strings(&[
            "always([+CANCEL] true -> <+signed_by(/users/requester.id)> true)".to_string(),
            "always([+CANCEL] true -> always([-DELIVER] true))".to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Cancellation", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_refund_prompt_example() {
        let formulas = parse_formula_strings(&[
            "always([+REFUND] true -> <+signed_by(/users/seller.id)> true)".to_string(),
            "always([+REFUND] true -> always([-RELEASE] true))".to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Refund", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_review_approval_prompt_example() {
        let formulas = parse_formula_strings(&[
            "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)".to_string(),
            "always([+APPROVE] true -> always([-REJECT] true))".to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("ReviewApproval", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_review_rejection_prompt_example() {
        let formulas = parse_formula_strings(&[
            "always([+REJECT] true -> <+signed_by(/users/reviewer.id)> true)".to_string(),
            "always([+REJECT] true -> always([-APPROVE] true))".to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("ReviewRejection", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_timeout_prompt_example() {
        let formulas = parse_formula_strings(&[
            "always([+TIMEOUT] true -> <+oracle_attests(/oracles/clock.id, \"deadline_passed\", \"true\")> true)"
                .to_string(),
            "always([+TIMEOUT] true -> always([-COMPLETE] true))".to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Timeout", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_escalation_prompt_example() {
        let formulas = parse_formula_strings(&[
            "always([+ESCALATE] true -> <+signed_by(/users/manager.id)> true)".to_string(),
            "always([+ESCALATE] true -> always([-CLOSE] true))".to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Escalation", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_withdrawal_prompt_example() {
        let formulas = parse_formula_strings(&[
            "always([+WITHDRAW] true -> <+signed_by(/users/depositor.id)> true)".to_string(),
            "always([+WITHDRAW] true -> always([-CLAIM] true))".to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Withdrawal", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_appeal_prompt_example() {
        let formulas = parse_formula_strings(&[
            "always([+APPEAL] true -> <+signed_by(/users/appellant.id)> true)".to_string(),
            "always([+APPEAL] true -> always([-ENFORCE] true))".to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Appeal", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_revocation_prompt_example() {
        let formulas = parse_formula_strings(&[
            "always([+REVOKE] true -> <+signed_by(/users/issuer.id)> true)".to_string(),
            "always([+REVOKE] true -> always([-USE] true))".to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Revocation", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_suspension_prompt_example() {
        let formulas = parse_formula_strings(&[
            "always([+SUSPEND] true -> <+signed_by(/users/administrator.id)> true)".to_string(),
            "always([+SUSPEND] true -> always([-ACCESS] true))".to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Suspension", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_reinstatement_prompt_example() {
        let formulas = parse_formula_strings(&[
            "always([+REINSTATE] true -> <+signed_by(/users/administrator.id)> true)".to_string(),
            "always([+REINSTATE] true -> always([-SUSPEND] true))".to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Reinstatement", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_renewal_prompt_example() {
        let formulas = parse_formula_strings(&[
            "always([+RENEW] true -> <+signed_by(/users/holder.id)> true)".to_string(),
            "always([+RENEW] true -> always([-EXPIRE] true))".to_string(),
        ]);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas("Renewal", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_termination_prompt_example() {
        let formulas = parse_formula_strings(&[
            "always([+TERMINATE] true -> <+signed_by(/users/counterparty.id)> true)".to_string(),
            "always([+TERMINATE] true -> always([-RENEW] true))".to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Termination", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_extension_prompt_example() {
        let formulas = parse_formula_strings(&[
            "always([+EXTEND] true -> <+signed_by(/users/owner.id)> true)".to_string(),
            "always([+EXTEND] true -> always([-TERMINATE] true))".to_string(),
        ]);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas("Extension", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_assignment_prompt_example() {
        let formulas = parse_formula_strings(&[
            "always([+ASSIGN] true -> <+signed_by(/users/assigner.id)> true)".to_string(),
            "always([+ASSIGN] true -> always([-REASSIGN] true))".to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Assignment", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_certification_prompt_example() {
        let formulas = parse_formula_strings(&[
            "always([+CERTIFY] true -> <+signed_by(/users/auditor.id)> true)".to_string(),
            "always([+CERTIFY] true -> always([-DEPLOY] true))".to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Certification", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_publication_prompt_example() {
        let formulas = parse_formula_strings(&[
            "always([+PUBLISH] true -> <+signed_by(/users/editor.id)> true)".to_string(),
            "always([+PUBLISH] true -> always([-EMBARGO] true))".to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Publication", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_registration_prompt_example() {
        let formulas = parse_formula_strings(&[
            "always([+REGISTER] true -> <+signed_by(/users/registrar.id)> true)".to_string(),
            "always([+REGISTER] true -> always([-DELETE] true))".to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Registration", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_acceptance_prompt_example() {
        let formulas = parse_formula_strings(&[
            "always([+ACCEPT] true -> <+signed_by(/users/recipient.id)> true)".to_string(),
            "always([+ACCEPT] true -> always([-REJECT] true))".to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Acceptance", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_acknowledgement_prompt_example() {
        let formulas = parse_formula_strings(&[
            "always([+ACKNOWLEDGE] true -> <+signed_by(/users/recipient.id)> true)".to_string(),
            "always([+ACKNOWLEDGE] true -> always([-DISPUTE] true))".to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("Acknowledgement", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_delivery_confirmation_prompt_example() {
        let formulas = parse_formula_strings(&[
            "always([+CONFIRM_DELIVERY] true -> <+signed_by(/users/recipient.id)> true)"
                .to_string(),
            "always([+CONFIRM_DELIVERY] true -> always([-REFUND] true))".to_string(),
        ]);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "DeliveryConfirmation",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_invoice_approval_prompt_example() {
        let formulas = parse_formula_strings(&[
            "always([+APPROVE_INVOICE] true -> <+signed_by(/users/payer.id)> true)".to_string(),
            "always([+APPROVE_INVOICE] true -> always([-CHARGEBACK] true))".to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("InvoiceApproval", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_milestone_acceptance_prompt_example() {
        let formulas = parse_formula_strings(&[
            "always([+ACCEPT_MILESTONE] true -> <+signed_by(/users/verifier.id)> true)"
                .to_string(),
            "always([+ACCEPT_MILESTONE] true -> always([-REWORK] true))".to_string(),
        ]);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "MilestoneAcceptance",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_inspection_approval_prompt_example() {
        let formulas = parse_formula_strings(&[
            "always([+APPROVE_INSPECTION] true -> <+signed_by(/users/inspector.id)> true)"
                .to_string(),
            "always([+APPROVE_INSPECTION] true -> always([-DEFECT_CLAIM] true))".to_string(),
        ]);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "InspectionApproval",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_compliance_attestation_prompt_example() {
        let formulas = parse_formula_strings(&[
            "always([+ATTEST_COMPLIANCE] true -> <+signed_by(/users/compliance_officer.id)> true)"
                .to_string(),
            "always([+ATTEST_COMPLIANCE] true -> always([-NONCOMPLIANCE_FINDING] true))"
                .to_string(),
        ]);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "ComplianceAttestation",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_safety_approval_prompt_example() {
        let formulas = parse_formula_strings(&[
            "always([+APPROVE_SAFETY] true -> <+signed_by(/users/safety_reviewer.id)> true)"
                .to_string(),
            "always([+APPROVE_SAFETY] true -> always([-UNSAFE_DEPLOYMENT] true))".to_string(),
        ]);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "SafetyApproval",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_risk_acceptance_prompt_example() {
        let formulas = parse_formula_strings(&[
            "always([+ACCEPT_RISK] true -> <+signed_by(/users/risk_owner.id)> true)"
                .to_string(),
            "always([+ACCEPT_RISK] true -> always([-UNMITIGATED_EXPOSURE] true))".to_string(),
        ]);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "RiskAcceptance",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_incident_closure_prompt_example() {
        let formulas = parse_formula_strings(&[
            "always([+CLOSE_INCIDENT] true -> <+signed_by(/users/incident_commander.id)> true)"
                .to_string(),
            "always([+CLOSE_INCIDENT] true -> always([-REOPEN_INCIDENT] true))".to_string(),
        ]);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "IncidentClosure",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_change_freeze_prompt_example() {
        let formulas = parse_formula_strings(&[
            "always([+FREEZE_CHANGE] true -> <+signed_by(/users/release_manager.id)> true)"
                .to_string(),
            "always([+FREEZE_CHANGE] true -> always([-DEPLOY] true))".to_string(),
        ]);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("ChangeFreeze", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_nested_until_guard_formula() {
        let formulas = parse_formula_strings(&[
            "lfp(X, (<+APPROVE> true) | ((<+REVIEW> true) & ((<+WAIT> true) & <>X)))".to_string(),
        ]);
        assert_eq!(formulas.len(), 1);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "NestedUntilGuard",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_nested_committed_until_guard_formula() {
        let formulas = parse_formula_strings(&[
            "lfp(X, ([<+APPROVE>] true) | ((<+REVIEW> true) & ((<+WAIT> true) & <>X)))".to_string(),
        ]);
        assert_eq!(formulas.len(), 1);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "NestedCommittedUntilGuard",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_parenthesized_recursive_nested_committed_until_guard() {
        let formulas = parse_formula_strings(&[
            "lfp(X, ([<+APPROVE>] true) | ((<+REVIEW> true) & ((<+WAIT> true) & <>(X))))"
                .to_string(),
        ]);
        assert_eq!(formulas.len(), 1);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "ParenthesizedRecursiveNestedCommittedUntilGuard",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_parenthesized_recursive_nested_until_guard() {
        let formulas = parse_formula_strings(&[
            "lfp(X, (<+APPROVE> true) | ((<+REVIEW> true) & ((<+WAIT> true) & <>(X))))"
                .to_string(),
        ]);
        assert_eq!(formulas.len(), 1);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "ParenthesizedRecursiveNestedUntilGuard",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_guarded_recursive_branch_before_goal() {
        let formulas = parse_formula_strings(&[
            "lfp(X, ((<+REVIEW> true) & ((<+WAIT> true) & <>(X))) | (<+APPROVE> true))"
                .to_string(),
        ]);
        assert_eq!(formulas.len(), 1);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "GuardedRecursiveBranchBeforeGoal",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_nested_parenthesized_branch_before_goal() {
        let formulas = parse_formula_strings(&[
            "lfp(X, ((<+REVIEW> true) & ((<+WAIT> true) & <>((X)))) | (<+APPROVE> true))"
                .to_string(),
        ]);
        assert_eq!(formulas.len(), 1);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "NestedParenthesizedBranchBeforeGoal",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_raw_guarded_branch_before_goal() {
        let formulas = parse_formula_strings(&[
            "lfp(X, ((<+REVIEW> true) & ((<+WAIT> true) & <>X)) | (<+APPROVE> true))".to_string(),
        ]);
        assert_eq!(formulas.len(), 1);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "RawGuardedBranchBeforeGoal",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_guarded_recursive_branch_before_committed_goal() {
        let formulas = parse_formula_strings(&[
            "lfp(X, ((<+REVIEW> true) & ((<+WAIT> true) & <>(X))) | ([<+APPROVE>] true))"
                .to_string(),
        ]);
        assert_eq!(formulas.len(), 1);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "GuardedRecursiveBranchBeforeCommittedGoal",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_nested_parenthesized_branch_before_committed_goal() {
        let formulas = parse_formula_strings(&[
            "lfp(X, ((<+REVIEW> true) & ((<+WAIT> true) & <>((X)))) | ([<+APPROVE>] true))"
                .to_string(),
        ]);
        assert_eq!(formulas.len(), 1);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "NestedParenthesizedBranchBeforeCommittedGoal",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_raw_guarded_branch_before_committed_goal() {
        let formulas = parse_formula_strings(&[
            "lfp(X, ((<+REVIEW> true) & ((<+WAIT> true) & <>X)) | ([<+APPROVE>] true))".to_string(),
        ]);
        assert_eq!(formulas.len(), 1);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "RawGuardedBranchBeforeCommittedGoal",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_parenthesized_recursive_lfp_eventual_goal() {
        let formulas = parse_formula_strings(&["lfp(X, (<+APPROVE> true) | <>(X))".to_string()]);
        assert_eq!(formulas.len(), 1);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "ParenthesizedRecursiveLfpEventualGoal",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_raw_lfp_eventual_goal() {
        let formulas = parse_formula_strings(&["lfp(X, (<+APPROVE> true) | <>X)".to_string()]);
        assert_eq!(formulas.len(), 1);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "RawLfpEventualGoal",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_raw_committed_lfp_eventual_goal() {
        let formulas = parse_formula_strings(&["lfp(X, ([<+APPROVE>] true) | <>X)".to_string()]);
        assert_eq!(formulas.len(), 1);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "RawCommittedLfpEventualGoal",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_committed_recursive_lfp_eventual_goal() {
        let formulas =
            parse_formula_strings(&["lfp(X, ([<+APPROVE>] true) | [<>]X)".to_string()]);
        assert_eq!(formulas.len(), 1);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "CommittedRecursiveLfpEventualGoal",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_unlabeled_committed_lfp_recursion_before_availability() {
        let formulas = parse_formula_strings(&["lfp(X, [<>]X | ([<+APPROVE>] true))".to_string()]);
        assert_eq!(formulas.len(), 1);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "UnlabeledCommittedLfpRecursionBeforeAvailability",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_parenthesized_unlabeled_lfp_recursion_before_availability()
    {
        let formulas =
            parse_formula_strings(&["lfp(X, [<>](X) | ([<+APPROVE>] true))".to_string()]);
        assert_eq!(formulas.len(), 1);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "ParenthesizedUnlabeledLfpRecursionBeforeAvailability",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_nested_parenthesized_unlabeled_lfp_recursion_before_availability()
    {
        let formulas =
            parse_formula_strings(&["lfp(X, [<>]((X)) | ([<+APPROVE>] true))".to_string()]);
        assert_eq!(formulas.len(), 1);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "NestedParenthesizedUnlabeledLfpRecursionBeforeAvailability",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_parenthesized_committed_lfp_eventual_goal() {
        let formulas =
            parse_formula_strings(&["lfp(X, ([<+APPROVE>] true) | <>(X))".to_string()]);
        assert_eq!(formulas.len(), 1);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "ParenthesizedCommittedLfpEventualGoal",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_nested_parenthesized_committed_lfp_goal() {
        let formulas =
            parse_formula_strings(&["lfp(X, ([<+APPROVE>] true) | <>((X)))".to_string()]);
        assert_eq!(formulas.len(), 1);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "NestedParenthesizedCommittedLfpGoal",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_nested_parenthesized_lfp_eventual_goal() {
        let formulas =
            parse_formula_strings(&["lfp(X, (<+APPROVE> true) | <>((X)))".to_string()]);
        assert_eq!(formulas.len(), 1);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "NestedParenthesizedLfpEventualGoal",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_parsed_gfp_recursion() {
        let formulas =
            parse_formula_strings(&["gfp(X, ([<+APPROVE>] true) & [](X))".to_string()]);
        assert_eq!(formulas.len(), 1);
        let model =
            modality_lang::formula_synthesis::synthesize_from_formulas("ParsedGfp", &formulas);

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_raw_committed_gfp_recursion() {
        let formulas = parse_formula_strings(&["gfp(X, ([<+APPROVE>] true) & []X)".to_string()]);
        assert_eq!(formulas.len(), 1);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "RawCommittedGfp",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_unlabeled_committed_gfp_recursion() {
        let formulas = parse_formula_strings(&["gfp(X, ([<+APPROVE>] true) & [<>]X)".to_string()]);
        assert_eq!(formulas.len(), 1);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "UnlabeledCommittedGfp",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_unlabeled_committed_gfp_recursion_before_availability() {
        let formulas = parse_formula_strings(&["gfp(X, [<>]X & ([<+APPROVE>] true))".to_string()]);
        assert_eq!(formulas.len(), 1);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "UnlabeledCommittedGfpRecursionBeforeAvailability",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_parenthesized_unlabeled_committed_gfp_branch_order() {
        let formulas =
            parse_formula_strings(&["gfp(X, [<>](X) & ([<+APPROVE>] true))".to_string()]);
        assert_eq!(formulas.len(), 1);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "ParenthesizedUnlabeledCommittedGfpBranchOrder",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_nested_parenthesized_unlabeled_committed_gfp_branch_order()
    {
        let formulas =
            parse_formula_strings(&["gfp(X, [<>]((X)) & ([<+APPROVE>] true))".to_string()]);
        assert_eq!(formulas.len(), 1);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "NestedParenthesizedUnlabeledCommittedGfpBranchOrder",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_parenthesized_parsed_gfp_recursion() {
        let formulas =
            parse_formula_strings(&["gfp(X, ([<+APPROVE>] true) & []((X)))".to_string()]);
        assert_eq!(formulas.len(), 1);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "ParenthesizedParsedGfp",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_committed_gfp_recursion_before_availability() {
        let formulas = parse_formula_strings(&["gfp(X, []X & ([<+APPROVE>] true))".to_string()]);
        assert_eq!(formulas.len(), 1);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "CommittedGfpRecursionBeforeAvailability",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_parenthesized_committed_gfp_branch_order() {
        let formulas =
            parse_formula_strings(&["gfp(X, [](X) & ([<+APPROVE>] true))".to_string()]);
        assert_eq!(formulas.len(), 1);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "ParenthesizedCommittedGfpBranchOrder",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_parenthesized_committed_gfp_recursion_before_availability()
    {
        let formulas =
            parse_formula_strings(&["gfp(X, []((X)) & ([<+APPROVE>] true))".to_string()]);
        assert_eq!(formulas.len(), 1);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "ParenthesizedCommittedGfpRecursionBeforeAvailability",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_permissive_parsed_gfp_recursion() {
        let formulas = parse_formula_strings(&["gfp(X, (<+APPROVE> true) & []X)".to_string()]);
        assert_eq!(formulas.len(), 1);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "PermissiveParsedGfp",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_parenthesized_permissive_gfp_recursion() {
        let formulas = parse_formula_strings(&["gfp(X, (<+APPROVE> true) & [](X))".to_string()]);
        assert_eq!(formulas.len(), 1);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "ParenthesizedPermissiveGfp",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_permissive_gfp_recursion_before_availability() {
        let formulas = parse_formula_strings(&["gfp(X, []X & (<+APPROVE> true))".to_string()]);
        assert_eq!(formulas.len(), 1);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "PermissiveGfpRecursionBeforeAvailability",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_parenthesized_permissive_gfp_recursion_before_availability()
    {
        let formulas = parse_formula_strings(&["gfp(X, [](X) & (<+APPROVE> true))".to_string()]);
        assert_eq!(formulas.len(), 1);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "ParenthesizedPermissiveGfpRecursionBeforeAvailability",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_parenthesized_gfp_recursion_before_availability() {
        let formulas =
            parse_formula_strings(&["gfp(X, []((X)) & (<+APPROVE> true))".to_string()]);
        assert_eq!(formulas.len(), 1);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "ParenthesizedGfpRecursionBeforeAvailability",
            &formulas,
        );

        verify_synthesized_model(&model, &formulas).unwrap();
    }

    #[test]
    fn verify_synthesized_model_accepts_parenthesized_permissive_parsed_gfp_recursion() {
        let formulas =
            parse_formula_strings(&["gfp(X, (<+APPROVE> true) & []((X)))".to_string()]);
        assert_eq!(formulas.len(), 1);
        let model = modality_lang::formula_synthesis::synthesize_from_formulas(
            "ParenthesizedPermissiveParsedGfp",
            &formulas,
        );

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

        assert!(err.to_string().contains("1 unparsed"));
        assert!(err.to_string().contains("F2"));
        assert!(err.to_string().contains("parser:"));
    }

    #[test]
    fn unparsed_formula_labels_include_formula_preview() {
        let formulas = vec![
            "always([<+APPROVE>] true)".to_string(),
            "always(".to_string(),
        ];

        let unparsed = unparsed_formula_string_labels(&formulas);

        assert_eq!(unparsed.len(), 1);
        assert!(unparsed[0].starts_with("F2 `always(` (parser:"));
        assert!(unparsed[0].contains("Failed to parse formula"));
    }

    #[test]
    fn unparsed_formula_labels_mark_empty_formula_preview() {
        let formulas = vec![" \n\t ".to_string()];

        let unparsed = unparsed_formula_string_labels(&formulas);

        assert_eq!(unparsed.len(), 1);
        assert!(unparsed[0].starts_with("F1 `<empty>` (parser:"));
        assert!(unparsed[0].contains("Failed to parse formula"));
    }

    #[test]
    fn unparsed_formula_labels_truncate_long_formula_preview() {
        let formulas = vec![format!("always({}", "x".repeat(120))];

        let unparsed = unparsed_formula_string_labels(&formulas);

        assert!(unparsed[0].starts_with("F1 `always("));
        assert!(unparsed[0].contains("...` (parser:"));
    }

    #[test]
    fn unparsed_formula_labels_compact_multiline_formula_preview() {
        let formulas = vec!["formula Bad {\n  always(\n}".to_string()];

        let unparsed = unparsed_formula_string_labels(&formulas);

        assert_eq!(unparsed.len(), 1);
        assert!(unparsed[0].starts_with("F1 `formula Bad { always( }` (parser:"));
        assert!(unparsed[0].contains("Failed to parse formula"));
    }

    #[test]
    fn parsed_formula_labels_include_generated_input_preview() {
        let formulas = vec!["always([<+APPROVE>] true)".to_string()];

        let labels = parsed_formula_string_labels(&formulas);

        assert_eq!(labels, vec!["F1 `always([<+APPROVE>] true)`"]);
    }

    #[test]
    fn parsed_formula_labels_prefer_declared_formula_names() {
        let formulas = vec!["formula Approval { always([<+APPROVE>] true) }".to_string()];

        let labels = parsed_formula_string_labels(&formulas);

        assert_eq!(labels, vec!["F1 `Approval`"]);
    }

    #[test]
    fn parsed_formula_labels_number_multiple_declarations_from_one_input() {
        let formulas = vec![
            r#"
formula Approval {
always([<+APPROVE>] true)
}

formula ApprovalSigner {
[+APPROVE] true -> <+signed_by(/users/reviewer.id)> true
}
"#
            .to_string(),
        ];

        let labels = parsed_formula_string_labels(&formulas);

        assert_eq!(labels, vec!["F1.1 `Approval`", "F1.2 `ApprovalSigner`"]);
    }

    #[test]
    fn legacy_string_constraints_still_cover_unparseable_llm_output() {
        let formulas = vec![
            "[+RELEASE] true -> eventually(<+DELIVER> true)".to_string(),
            "[+RELEASE] true -> <+signed_by(/users/alice.id)> true".to_string(),
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

    #[test]
    fn legacy_string_constraints_still_accept_implies_output() {
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
