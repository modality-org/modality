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
        print_synthesis_list();
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
                verify_synthesized_model_with_labels(
                    &model,
                    &parsed_input.formulas,
                    &parsed_input.labels,
                )?;
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
            r#"[+RELEASE] true -> eventually((<+DEPOSIT> true & <+DELIVER> true))"#,
            r#"[+RELEASE] true -> eventually(([<+DEPOSIT>] true & [<+DELIVER>] true))"#,
            r#"[+RELEASE] true -> (eventually(<+DEPOSIT> true) & eventually(<+DELIVER> true))"#,
            r#"[<+RELEASE>] true -> (eventually(<+DEPOSIT> true) & eventually(<+DELIVER> true))"#,
            r#"[+RELEASE] true -> (eventually([<+DEPOSIT>] true) & eventually([<+DELIVER>] true))"#,
            r#"[<+RELEASE>] true -> (eventually([<+DEPOSIT>] true) & eventually([<+DELIVER>] true))"#,
            r#"always([+AGENT_A_TURN] true -> eventually(<+AGENT_B_TURN> true))"#,
            r#"always([+AGENT_B_TURN] true -> eventually(<+AGENT_A_TURN> true))"#,
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
        ],
    },
];

fn print_synthesis_list() {
    println!("Available templates:\n");
    println!("  escrow              Two-party escrow with deposit/deliver/release");
    println!("  handshake           Mutual agreement requiring both signatures");
    println!("  mutual_cooperation  Cooperation game - both must cooperate, defection blocked");
    println!("  atomic_swap         Both parties commit before either can claim");
    println!("  multisig            N-of-M signature approval pattern");
    println!("  turn_taking         Alternating two-party turn cycle");
    println!("  service_agreement   Offer -> Accept -> Deliver -> Confirm -> Pay");
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
    for group in FORMULA_EXAMPLE_GROUPS {
        println!("\n  {}:", group.title);
        println!("    {}", group.description);
        for formula in group.formulas {
            println!(
                "    modality model synthesize --formulas \"{}\" --verify",
                escape_formula_for_command(formula)
            );
        }
    }
    println!("\nOr generate a prompt and synthesize an LLM response file:");
    println!(
        "  modality model synthesize --describe \"escrow where buyer deposits funds\" --generate-prompt"
    );
    println!("  modality model synthesize --llm-response-file response.md --verify");
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
        (None, Some(path)) => Ok(Some(std::fs::read_to_string(path)?)),
        (None, None) => Ok(None),
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
    fn llm_response_loader_rejects_inline_and_file_together() {
        let response = "formula generated { true }".to_string();
        let path = PathBuf::from("response.md");

        let err = load_llm_response(Some(&response), Some(&path)).unwrap_err();

        assert!(err.to_string().contains("--llm-response-file"));
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
