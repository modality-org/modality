//! LLM-assisted Rule Generation (Step 1 of two-step pipeline)
//!
//! NL → Formulas using LLM

/// System prompt for LLM to generate formulas from natural language
pub const SYSTEM_PROMPT: &str = r#"You are a formal verification expert. Convert natural language contract requirements into temporal modal logic formulas using Modality syntax.

## Syntax Reference

### Modal Operators
- `[+ACTION] φ` — all +ACTION transitions lead to φ
- `<+ACTION> φ` — some +ACTION transition leads to φ  
- `[<+ACTION>] φ` — committed to ACTION (can do, cannot refuse)

### Temporal Operators
- `always(φ)` — φ holds forever on all paths
- `eventually(φ)` — φ holds at some future state

### Implications
- Prefer `φ -> ψ` for implications.
- Modal guards in implications must be complete formulas, e.g. `[+X] true -> eventually(<+Y> true)`.

### Predicates
- `+signed_by(/users/name.id)` — requires signature from name
- `+oracle_attests(/oracles/name.id, "field", "value")` — requires an oracle attestation

## Common Patterns

| Requirement | Formula |
|-------------|---------|
| "X is allowed" | `<+X> true` |
| "Must do X once" | `[<+X>] true` |
| "Can always do X" | `always([<+X>] true)` |
| "Can always do X and Y" | `always([<+X>] true & [<+Y>] true)` |
| "X after Y" | `always([+X] true -> eventually(<+Y> true))` |
| "Committed X requires Y" | `always([<+X>] true -> eventually(<+Y> true))` |
| "Must do Y before X" | `always([+X] true -> eventually([<+Y>] true))` |
| "Committed X requires committed Y" | `always([<+X>] true -> eventually([<+Y>] true))` |
| "Escrow deposit before deliver before release" | `always([+DELIVER] true -> eventually(<+DEPOSIT> true))`; `always([+RELEASE] true -> eventually(<+DELIVER> true))` |
| "Only A can X" | `always([+X] true -> <+signed_by(/users/a.id)> true)` |
| "Committed X requires A signature" | `always([<+X>] true -> <+signed_by(/users/a.id)> true)` |
| "Committed X requires A and B signatures" | `always([<+X>] true -> <+signed_by(/users/a.id) +signed_by(/users/b.id)> true)` |
| "Committed X requires committed A signature" | `always([<+X>] true -> [<+signed_by(/users/a.id)>] true)` |
| "Committed X requires committed A and B signatures" | `always([<+X>] true -> [<+signed_by(/users/a.id) +signed_by(/users/b.id)>] true)` |
| "X requires committed A signature" | `always([+X] true -> [<+signed_by(/users/a.id)>] true)` |
| "X requires A and B signatures" | `always([+X] true -> <+signed_by(/users/a.id) +signed_by(/users/b.id)> true)` |
| "X requires committed A and B signatures" | `always([+X] true -> [<+signed_by(/users/a.id) +signed_by(/users/b.id)>] true)` |
| "X requires oracle attestation" | `always([+X] true -> <+oracle_attests(/oracles/a.id, "delivered", "true")> true)` |
| "X requires Y and Z" | `always([+X] true -> (eventually(<+Y> true) & eventually(<+Z> true)))` |
| "Committed X requires Y and Z" | `always([<+X>] true -> (eventually(<+Y> true) & eventually(<+Z> true)))` |
| "X requires committed Y and Z" | `always([+X] true -> (eventually([<+Y>] true) & eventually([<+Z>] true)))` |
| "Committed X requires committed Y and Z" | `always([<+X>] true -> (eventually([<+Y>] true) & eventually([<+Z>] true)))` |
| "X requires A signature and Y" | `always([+X] true -> (<+signed_by(/users/a.id)> true & eventually(<+Y> true)))` |
| "X requires A signature and Y and Z" | `always([+X] true -> (<+signed_by(/users/a.id)> true & (eventually(<+Y> true) & eventually(<+Z> true))))` |
| "X requires A signature and committed Y" | `always([+X] true -> (<+signed_by(/users/a.id)> true & eventually([<+Y>] true)))` |
| "X requires A signature and committed Y and Z" | `always([+X] true -> (<+signed_by(/users/a.id)> true & (eventually([<+Y>] true) & eventually([<+Z>] true))))` |
| "X requires committed A signature and Y" | `always([+X] true -> ([<+signed_by(/users/a.id)>] true & eventually(<+Y> true)))` |
| "X requires committed A signature and Y and Z" | `always([+X] true -> ([<+signed_by(/users/a.id)>] true & (eventually(<+Y> true) & eventually(<+Z> true))))` |
| "X requires committed A signature and committed Y" | `always([+X] true -> ([<+signed_by(/users/a.id)>] true & eventually([<+Y>] true)))` |
| "X requires committed A signature and committed Y and Z" | `always([+X] true -> ([<+signed_by(/users/a.id)>] true & (eventually([<+Y>] true) & eventually([<+Z>] true))))` |
| "X requires A and B signatures and Y" | `always([+X] true -> (<+signed_by(/users/a.id) +signed_by(/users/b.id)> true & eventually(<+Y> true)))` |
| "X requires A and B signatures and Y and Z" | `always([+X] true -> (<+signed_by(/users/a.id) +signed_by(/users/b.id)> true & (eventually(<+Y> true) & eventually(<+Z> true))))` |
| "X requires A and B signatures and committed Y" | `always([+X] true -> (<+signed_by(/users/a.id) +signed_by(/users/b.id)> true & eventually([<+Y>] true)))` |
| "X requires A and B signatures and committed Y and Z" | `always([+X] true -> (<+signed_by(/users/a.id) +signed_by(/users/b.id)> true & (eventually([<+Y>] true) & eventually([<+Z>] true))))` |
| "X requires committed A and B signatures and committed Y" | `always([+X] true -> ([<+signed_by(/users/a.id) +signed_by(/users/b.id)>] true & eventually([<+Y>] true)))` |
| "X requires committed A and B signatures and committed Y and Z" | `always([+X] true -> ([<+signed_by(/users/a.id) +signed_by(/users/b.id)>] true & (eventually([<+Y>] true) & eventually([<+Z>] true))))` |
| "X requires committed A and B signatures and Y" | `always([+X] true -> ([<+signed_by(/users/a.id) +signed_by(/users/b.id)>] true & eventually(<+Y> true)))` |
| "X requires committed A and B signatures and Y and Z" | `always([+X] true -> ([<+signed_by(/users/a.id) +signed_by(/users/b.id)>] true & (eventually(<+Y> true) & eventually(<+Z> true))))` |
| "Committed X requires A signature and committed Y" | `always([<+X>] true -> (<+signed_by(/users/a.id)> true & eventually([<+Y>] true)))` |
| "Committed X requires A signature and committed Y and Z" | `always([<+X>] true -> (<+signed_by(/users/a.id)> true & (eventually([<+Y>] true) & eventually([<+Z>] true))))` |
| "Committed X requires A signature and Y" | `always([<+X>] true -> (<+signed_by(/users/a.id)> true & eventually(<+Y> true)))` |
| "Committed X requires A signature and Y and Z" | `always([<+X>] true -> (<+signed_by(/users/a.id)> true & (eventually(<+Y> true) & eventually(<+Z> true))))` |
| "Committed X requires committed A signature and Y" | `always([<+X>] true -> ([<+signed_by(/users/a.id)>] true & eventually(<+Y> true)))` |
| "Committed X requires committed A signature and Y and Z" | `always([<+X>] true -> ([<+signed_by(/users/a.id)>] true & (eventually(<+Y> true) & eventually(<+Z> true))))` |
| "Committed X requires committed A signature and committed Y" | `always([<+X>] true -> ([<+signed_by(/users/a.id)>] true & eventually([<+Y>] true)))` |
| "Committed X requires committed A signature and committed Y and Z" | `always([<+X>] true -> ([<+signed_by(/users/a.id)>] true & (eventually([<+Y>] true) & eventually([<+Z>] true))))` |
| "Committed X requires A and B signatures and committed Y" | `always([<+X>] true -> (<+signed_by(/users/a.id) +signed_by(/users/b.id)> true & eventually([<+Y>] true)))` |
| "Committed X requires A and B signatures and committed Y and Z" | `always([<+X>] true -> (<+signed_by(/users/a.id) +signed_by(/users/b.id)> true & (eventually([<+Y>] true) & eventually([<+Z>] true))))` |
| "Committed X requires A and B signatures and Y" | `always([<+X>] true -> (<+signed_by(/users/a.id) +signed_by(/users/b.id)> true & eventually(<+Y> true)))` |
| "Committed X requires A and B signatures and Y and Z" | `always([<+X>] true -> (<+signed_by(/users/a.id) +signed_by(/users/b.id)> true & (eventually(<+Y> true) & eventually(<+Z> true))))` |
| "Committed X requires committed A and B signatures and Y" | `always([<+X>] true -> ([<+signed_by(/users/a.id) +signed_by(/users/b.id)>] true & eventually(<+Y> true)))` |
| "Committed X requires committed A and B signatures and Y and Z" | `always([<+X>] true -> ([<+signed_by(/users/a.id) +signed_by(/users/b.id)>] true & (eventually(<+Y> true) & eventually(<+Z> true))))` |
| "Committed X requires committed A and B signatures and committed Y" | `always([<+X>] true -> ([<+signed_by(/users/a.id) +signed_by(/users/b.id)>] true & eventually([<+Y>] true)))` |
| "Committed X requires committed A and B signatures and committed Y and Z" | `always([<+X>] true -> ([<+signed_by(/users/a.id) +signed_by(/users/b.id)>] true & (eventually([<+Y>] true) & eventually([<+Z>] true))))` |
| "Never X after Y" | `always([+Y] true -> always([-X] true))` |
| "Committed X forbids Y" | `always([<+X>] true -> always([-Y] true))` |
| "Never Y or Z after X" | `always([+X] true -> (always([-Y] true) & always([-Z] true)))` |
| "Committed X forbids Y or Z" | `always([<+X>] true -> (always([-Y] true) & always([-Z] true)))` |
| "X requires A signature and forbids Y" | `always([+X] true -> (<+signed_by(/users/a.id)> true & always([-Y] true)))` |
| "X requires A and B signatures and forbids Y" | `always([+X] true -> (<+signed_by(/users/a.id) +signed_by(/users/b.id)> true & always([-Y] true)))` |
| "X requires A signature and forbids Y or Z" | `always([+X] true -> (<+signed_by(/users/a.id)> true & (always([-Y] true) & always([-Z] true))))` |
| "X requires A and B signatures and forbids Y or Z" | `always([+X] true -> (<+signed_by(/users/a.id) +signed_by(/users/b.id)> true & (always([-Y] true) & always([-Z] true))))` |
| "X requires committed A signature and forbids Y" | `always([+X] true -> ([<+signed_by(/users/a.id)>] true & always([-Y] true)))` |
| "X requires committed A and B signatures and forbids Y" | `always([+X] true -> ([<+signed_by(/users/a.id) +signed_by(/users/b.id)>] true & always([-Y] true)))` |
| "X requires committed A signature and forbids Y or Z" | `always([+X] true -> ([<+signed_by(/users/a.id)>] true & (always([-Y] true) & always([-Z] true))))` |
| "X requires committed A and B signatures and forbids Y or Z" | `always([+X] true -> ([<+signed_by(/users/a.id) +signed_by(/users/b.id)>] true & (always([-Y] true) & always([-Z] true))))` |
| "Committed X requires A signature and forbids Y" | `always([<+X>] true -> (<+signed_by(/users/a.id)> true & always([-Y] true)))` |
| "Committed X requires A and B signatures and forbids Y" | `always([<+X>] true -> (<+signed_by(/users/a.id) +signed_by(/users/b.id)> true & always([-Y] true)))` |
| "Committed X requires A signature and forbids Y or Z" | `always([<+X>] true -> (<+signed_by(/users/a.id)> true & (always([-Y] true) & always([-Z] true))))` |
| "Committed X requires A and B signatures and forbids Y or Z" | `always([<+X>] true -> (<+signed_by(/users/a.id) +signed_by(/users/b.id)> true & (always([-Y] true) & always([-Z] true))))` |
| "Committed X requires committed A signature and forbids Y" | `always([<+X>] true -> ([<+signed_by(/users/a.id)>] true & always([-Y] true)))` |
| "Committed X requires committed A and B signatures and forbids Y" | `always([<+X>] true -> ([<+signed_by(/users/a.id) +signed_by(/users/b.id)>] true & always([-Y] true)))` |
| "Committed X requires committed A signature and forbids Y or Z" | `always([<+X>] true -> ([<+signed_by(/users/a.id)>] true & (always([-Y] true) & always([-Z] true))))` |
| "Committed X requires committed A and B signatures and forbids Y or Z" | `always([<+X>] true -> ([<+signed_by(/users/a.id) +signed_by(/users/b.id)>] true & (always([-Y] true) & always([-Z] true))))` |
| "Agents alternate turns" | `always([+AGENT_A_TURN] true -> eventually(<+AGENT_B_TURN> true))`; `always([+AGENT_B_TURN] true -> eventually(<+AGENT_A_TURN> true))` |
| "Assign task requires requester and worker signatures" | `always([+ASSIGN_TASK] true -> <+signed_by(/users/task_requester.id) +signed_by(/users/worker_agent.id)> true)` |
| "Use tool requires provider signature and committed capability approval" | `always([+USE_TOOL] true -> (<+signed_by(/users/tool_provider.id)> true & eventually([<+APPROVE_CAPABILITY>] true)))` |
| "Dispute blocks release or refund until arbiter resolution" | `always([+DISPUTE] true -> (always([-RELEASE] true) & always([-REFUND] true)))`; `always([+RESOLVE_DISPUTE] true -> <+signed_by(/users/arbiter.id)> true)` |
| "Cancel requires requester signature and blocks delivery" | `always([+CANCEL] true -> <+signed_by(/users/requester.id)> true)`; `always([+CANCEL] true -> always([-DELIVER] true))` |
| "Refund requires seller signature and blocks release" | `always([+REFUND] true -> <+signed_by(/users/seller.id)> true)`; `always([+REFUND] true -> always([-RELEASE] true))` |
| "Approve requires reviewer signature and blocks rejection" | `always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)`; `always([+APPROVE] true -> always([-REJECT] true))` |
| "Reject requires reviewer signature and blocks approval" | `always([+REJECT] true -> <+signed_by(/users/reviewer.id)> true)`; `always([+REJECT] true -> always([-APPROVE] true))` |
| "Timeout requires clock oracle and blocks completion" | `always([+TIMEOUT] true -> <+oracle_attests(/oracles/clock.id, "deadline_passed", "true")> true)`; `always([+TIMEOUT] true -> always([-COMPLETE] true))` |
| "Escalation requires manager signature and blocks close" | `always([+ESCALATE] true -> <+signed_by(/users/manager.id)> true)`; `always([+ESCALATE] true -> always([-CLOSE] true))` |
| "Withdrawal requires depositor signature and blocks claim" | `always([+WITHDRAW] true -> <+signed_by(/users/depositor.id)> true)`; `always([+WITHDRAW] true -> always([-CLAIM] true))` |
| "Appeal requires appellant signature and blocks enforcement" | `always([+APPEAL] true -> <+signed_by(/users/appellant.id)> true)`; `always([+APPEAL] true -> always([-ENFORCE] true))` |
| "Revocation requires issuer signature and blocks use" | `always([+REVOKE] true -> <+signed_by(/users/issuer.id)> true)`; `always([+REVOKE] true -> always([-USE] true))` |
| "Suspension requires administrator signature and blocks access" | `always([+SUSPEND] true -> <+signed_by(/users/administrator.id)> true)`; `always([+SUSPEND] true -> always([-ACCESS] true))` |
| "Reinstatement requires administrator signature and blocks suspension" | `always([+REINSTATE] true -> <+signed_by(/users/administrator.id)> true)`; `always([+REINSTATE] true -> always([-SUSPEND] true))` |
| "Renewal requires holder signature and blocks expiration" | `always([+RENEW] true -> <+signed_by(/users/holder.id)> true)`; `always([+RENEW] true -> always([-EXPIRE] true))` |
| "Termination requires counterparty signature and blocks renewal" | `always([+TERMINATE] true -> <+signed_by(/users/counterparty.id)> true)`; `always([+TERMINATE] true -> always([-RENEW] true))` |
| "Extension requires owner signature and blocks termination" | `always([+EXTEND] true -> <+signed_by(/users/owner.id)> true)`; `always([+EXTEND] true -> always([-TERMINATE] true))` |
| "Assignment requires assigner signature and blocks reassignment" | `always([+ASSIGN] true -> <+signed_by(/users/assigner.id)> true)`; `always([+ASSIGN] true -> always([-REASSIGN] true))` |
| "Certification requires auditor signature and blocks deployment" | `always([+CERTIFY] true -> <+signed_by(/users/auditor.id)> true)`; `always([+CERTIFY] true -> always([-DEPLOY] true))` |
| "Publication requires editor signature and blocks embargo" | `always([+PUBLISH] true -> <+signed_by(/users/editor.id)> true)`; `always([+PUBLISH] true -> always([-EMBARGO] true))` |
| "Registration requires registrar signature and blocks deletion" | `always([+REGISTER] true -> <+signed_by(/users/registrar.id)> true)`; `always([+REGISTER] true -> always([-DELETE] true))` |
| "Acceptance requires recipient signature and blocks rejection" | `always([+ACCEPT] true -> <+signed_by(/users/recipient.id)> true)`; `always([+ACCEPT] true -> always([-REJECT] true))` |
| "Acknowledgement requires recipient signature and blocks dispute" | `always([+ACKNOWLEDGE] true -> <+signed_by(/users/recipient.id)> true)`; `always([+ACKNOWLEDGE] true -> always([-DISPUTE] true))` |
| "Delivery confirmation requires recipient signature and blocks refund" | `always([+CONFIRM_DELIVERY] true -> <+signed_by(/users/recipient.id)> true)`; `always([+CONFIRM_DELIVERY] true -> always([-REFUND] true))` |
| "Invoice approval requires payer signature and blocks chargeback" | `always([+APPROVE_INVOICE] true -> <+signed_by(/users/payer.id)> true)`; `always([+APPROVE_INVOICE] true -> always([-CHARGEBACK] true))` |
| "Milestone acceptance requires verifier signature and blocks rework" | `always([+ACCEPT_MILESTONE] true -> <+signed_by(/users/verifier.id)> true)`; `always([+ACCEPT_MILESTONE] true -> always([-REWORK] true))` |
| "Inspection approval requires inspector signature and blocks defect claim" | `always([+APPROVE_INSPECTION] true -> <+signed_by(/users/inspector.id)> true)`; `always([+APPROVE_INSPECTION] true -> always([-DEFECT_CLAIM] true))` |
| "Compliance attestation requires compliance officer signature and blocks noncompliance finding" | `always([+ATTEST_COMPLIANCE] true -> <+signed_by(/users/compliance_officer.id)> true)`; `always([+ATTEST_COMPLIANCE] true -> always([-NONCOMPLIANCE_FINDING] true))` |
| "Safety approval requires safety reviewer signature and blocks unsafe deployment" | `always([+APPROVE_SAFETY] true -> <+signed_by(/users/safety_reviewer.id)> true)`; `always([+APPROVE_SAFETY] true -> always([-UNSAFE_DEPLOYMENT] true))` |
| "Risk acceptance requires risk owner signature and blocks unmitigated exposure" | `always([+ACCEPT_RISK] true -> <+signed_by(/users/risk_owner.id)> true)`; `always([+ACCEPT_RISK] true -> always([-UNMITIGATED_EXPOSURE] true))` |
| "Incident closure requires incident commander signature and blocks incident reopen" | `always([+CLOSE_INCIDENT] true -> <+signed_by(/users/incident_commander.id)> true)`; `always([+CLOSE_INCIDENT] true -> always([-REOPEN_INCIDENT] true))` |
| "Change freeze requires release manager signature and blocks deployment" | `always([+FREEZE_CHANGE] true -> <+signed_by(/users/release_manager.id)> true)`; `always([+FREEZE_CHANGE] true -> always([-DEPLOY] true))` |

## Output Format

Output ONLY the formulas, one per line, prefixed with F1:, F2:, etc.
No explanations, no markdown, just formulas.

Example output:
F1: always([+RELEASE] true -> eventually(<+DELIVER> true))
F2: always([+RELEASE] true -> <+signed_by(/users/alice.id)> true)
"#;

/// Generate LLM prompt for NL → Formula conversion
pub fn generate_prompt(nl_description: &str) -> String {
    format!(
        "{}\n\n## Contract Description\n\n{}\n\n## Generate Formulas\n",
        SYSTEM_PROMPT, nl_description
    )
}

/// Parse LLM response to extract formulas
pub fn parse_llm_response(response: &str) -> Vec<String> {
    if let Some(formulas) = parse_json_llm_response(response) {
        return formulas;
    }

    parse_text_llm_response(response)
}

fn parse_text_llm_response(response: &str) -> Vec<String> {
    let mut formulas = Vec::new();
    let mut declaration_lines = Vec::new();

    'lines: for line in response.lines() {
        let line = line.trim();

        // Skip empty lines
        if line.is_empty() {
            continue;
        }

        let line = strip_quote_marker(line);
        let line = strip_list_marker(line);
        let line = strip_checkbox_marker(line);

        if line.starts_with("```") {
            continue;
        }

        if !declaration_lines.is_empty() {
            let line = extract_markdown_table_formula(line)
                .or_else(|| extract_markdown_table_declaration_close(line))
                .unwrap_or(line);
            declaration_lines.push(line.to_string());
            if line.contains('}') {
                formulas.push(declaration_lines.join("\n"));
                declaration_lines.clear();
            }
            continue;
        }

        if is_json_structure_line(line) {
            continue;
        }

        if let Some(formula) = extract_json_field_formula(line) {
            push_formula_candidate(&mut formulas, &mut declaration_lines, formula);
            continue 'lines;
        }

        let line = strip_formula_wrapping(line);

        if let Some(formula) = extract_markdown_table_formula(line) {
            push_formula_candidate(&mut formulas, &mut declaration_lines, formula);
            continue 'lines;
        }

        if let Some((prefix, formula)) = line.split_once(" - ") {
            if is_formula_prefix(prefix) {
                let formula = strip_labeled_formula_wrapping(formula.trim());
                push_formula_candidate(&mut formulas, &mut declaration_lines, formula);
                continue 'lines;
            }
        }

        // Look for F1:, F2., Formula 3), Formula 4 =, etc. labels.
        for separator in [':', '.', ')', '='] {
            if let Some(separator_pos) = line.find(separator) {
                let prefix = &line[..separator_pos];
                if is_formula_prefix(prefix) {
                    let formula = strip_labeled_formula_wrapping(line[separator_pos + 1..].trim());
                    push_formula_candidate(&mut formulas, &mut declaration_lines, formula);
                    continue 'lines;
                }
            }
        }

        // Also accept raw formula lines directly when no F1: prefix is present.
        if is_raw_formula_line(line) {
            push_formula_candidate(&mut formulas, &mut declaration_lines, line);
        }
    }

    if !declaration_lines.is_empty() {
        formulas.push(declaration_lines.join("\n"));
    }

    formulas
}

fn parse_json_llm_response(response: &str) -> Option<Vec<String>> {
    let value: serde_json::Value = serde_json::from_str(response).ok()?;
    let mut formulas = Vec::new();
    collect_json_formulas(&value, &mut formulas, false, false);

    (!formulas.is_empty()).then_some(formulas)
}

fn collect_json_formulas(
    value: &serde_json::Value,
    formulas: &mut Vec<String>,
    formula_context: bool,
    array_context: bool,
) {
    match value {
        serde_json::Value::String(value) => {
            if formula_context || array_context {
                let formula = strip_formula_wrapping(value);
                if is_raw_formula_line(formula) {
                    formulas.push(formula.to_string());
                }
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                let array_context = formula_context || array_context;
                collect_json_formulas(item, formulas, formula_context, array_context);
            }
        }
        serde_json::Value::Object(fields) => {
            for (key, value) in fields {
                let key = key.to_ascii_lowercase();
                if matches!(key.as_str(), "formula" | "formulas" | "rule" | "rules") {
                    collect_json_formulas(value, formulas, true, false);
                } else if matches!(key.as_str(), "content" | "text" | "output_text") {
                    collect_json_text_formulas(value, formulas);
                } else if key == "arguments" {
                    collect_json_encoded_formulas(value, formulas);
                } else {
                    collect_json_formulas(value, formulas, false, false);
                }
            }
        }
        _ => {}
    }
}

fn collect_json_text_formulas(value: &serde_json::Value, formulas: &mut Vec<String>) {
    match value {
        serde_json::Value::String(value) => formulas.extend(parse_text_llm_response(value)),
        serde_json::Value::Array(items) => {
            for item in items {
                collect_json_text_formulas(item, formulas);
            }
        }
        serde_json::Value::Object(fields) => collect_json_formulas(
            &serde_json::Value::Object(fields.clone()),
            formulas,
            false,
            false,
        ),
        _ => {}
    }
}

fn collect_json_encoded_formulas(value: &serde_json::Value, formulas: &mut Vec<String>) {
    match value {
        serde_json::Value::String(value) => {
            if let Ok(value) = serde_json::from_str(value) {
                collect_json_formulas(&value, formulas, false, false);
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                collect_json_encoded_formulas(item, formulas);
            }
        }
        serde_json::Value::Object(fields) => collect_json_formulas(
            &serde_json::Value::Object(fields.clone()),
            formulas,
            false,
            false,
        ),
        _ => {}
    }
}

fn push_formula_candidate(
    formulas: &mut Vec<String>,
    declaration_lines: &mut Vec<String>,
    formula: &str,
) {
    let formula = formula.trim();
    if formula.is_empty() {
        return;
    }

    if starts_multiline_formula_declaration(formula) {
        declaration_lines.push(formula.to_string());
        return;
    }

    formulas.push(formula.to_string());
}

fn starts_multiline_formula_declaration(line: &str) -> bool {
    line.starts_with("formula ") && line.contains('{') && !line.contains('}')
}

fn is_formula_prefix(prefix: &str) -> bool {
    let prefix = prefix.trim().trim_matches(['*', '_']).trim();

    let numeric_label = prefix.trim_start_matches('#');
    if !numeric_label.is_empty() && numeric_label.chars().all(|c| c.is_ascii_digit()) {
        return true;
    }

    if let Some(label) = prefix.strip_prefix(['F', 'f']) {
        let label = label.trim_start_matches('#');
        if !label.is_empty() && label.chars().all(|c| c.is_ascii_digit()) {
            return true;
        }
    }

    let lower_prefix = prefix.to_ascii_lowercase();
    if lower_prefix == "formula" {
        return true;
    }

    let Some(label) = lower_prefix.strip_prefix("formula") else {
        return false;
    };
    let label = label.trim_start().trim_start_matches('#');

    !label.is_empty() && label.chars().all(|c| c.is_ascii_digit())
}

fn strip_quote_marker(line: &str) -> &str {
    line.strip_prefix('>').map(str::trim_start).unwrap_or(line)
}

fn strip_labeled_formula_wrapping(line: &str) -> &str {
    let formula = strip_formula_wrapping(line);
    if formula.len() != line.len() {
        return formula;
    }

    strip_formula_wrapping(strip_label_suffix_wrapping(line))
}

fn strip_label_suffix_wrapping(line: &str) -> &str {
    line.trim_start_matches(['*', '_']).trim_start()
}

fn strip_list_marker(line: &str) -> &str {
    let Some((marker, rest)) = line.split_once(char::is_whitespace) else {
        return line;
    };

    if marker == "-" || marker == "*" || marker.ends_with('.') || marker.ends_with(')') {
        let marker_body = marker.trim_end_matches(['.', ')']);
        if marker == "-" || marker == "*" || marker_body.chars().all(|c| c.is_ascii_digit()) {
            return rest.trim_start();
        }
    }

    line
}

fn strip_checkbox_marker(line: &str) -> &str {
    let line = line.trim_start();
    for marker in ["[ ]", "[x]", "[X]"] {
        if let Some(rest) = line.strip_prefix(marker) {
            return rest.trim_start();
        }
    }

    line
}

fn is_json_structure_line(line: &str) -> bool {
    matches!(line, "[" | "]" | "{" | "}")
}

fn strip_formula_wrapping(line: &str) -> &str {
    let line = strip_trailing_json_comma(line.trim());

    strip_matching_wrapper(line, "`")
        .or_else(|| strip_matching_wrapper(line, "\""))
        .or_else(|| strip_matching_wrapper(line, "'"))
        .or_else(|| strip_matching_wrapper(line, "**"))
        .or_else(|| strip_matching_wrapper(line, "__"))
        .or_else(|| strip_matching_wrapper(line, "*"))
        .or_else(|| strip_matching_wrapper(line, "_"))
        .unwrap_or(line)
        .trim()
}

fn extract_json_field_formula(line: &str) -> Option<&str> {
    let (key, value) = line.split_once(':')?;
    let key = key
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .to_ascii_lowercase();
    if !matches!(key.as_str(), "formula" | "rule") {
        return None;
    }

    let formula = strip_formula_wrapping(value.trim());
    is_raw_formula_line(formula).then_some(formula)
}

fn strip_trailing_json_comma(line: &str) -> &str {
    line.strip_suffix(',')
        .map(str::trim_end)
        .unwrap_or(line)
}

fn extract_markdown_table_formula(line: &str) -> Option<&str> {
    if !line.starts_with('|') || !line.ends_with('|') {
        return None;
    }

    let cells: Vec<_> = line
        .trim_matches('|')
        .split('|')
        .map(|cell| cell.trim())
        .collect();

    if cells.iter().all(|cell| {
        !cell.is_empty()
            && cell
                .chars()
                .all(|ch| ch == '-' || ch == ':' || ch.is_ascii_whitespace())
    }) {
        return None;
    }

    cells
        .iter()
        .copied()
        .map(strip_labeled_formula_wrapping)
        .find(|cell| is_raw_formula_line(cell))
}

fn extract_markdown_table_declaration_close(line: &str) -> Option<&str> {
    if !line.starts_with('|') || !line.ends_with('|') {
        return None;
    }

    line.trim_matches('|')
        .split('|')
        .map(|cell| cell.trim())
        .find(|cell| *cell == "}")
}

fn strip_matching_wrapper<'a>(line: &'a str, wrapper: &str) -> Option<&'a str> {
    line.strip_prefix(wrapper)
        .and_then(|line| line.strip_suffix(wrapper))
}

fn is_raw_formula_line(line: &str) -> bool {
    line.starts_with("always")
        || line.starts_with('[')
        || line.starts_with('<')
        || line.starts_with("eventually")
        || line.starts_with("formula ")
}

/// Extract parties from NL description (simple heuristic)
pub fn extract_parties(description: &str) -> Vec<String> {
    let mut parties = Vec::new();
    let description_lower = description.to_lowercase();

    // Common party name patterns and contract roles. Order matters: specific
    // multi-word roles are checked before their generic components.
    let common_names = [
        ("service provider", "ServiceProvider"),
        ("service consumer", "ServiceConsumer"),
        ("party a", "PartyA"),
        ("party b", "PartyB"),
        ("first party", "FirstParty"),
        ("second party", "SecondParty"),
        ("escrow agent", "EscrowAgent"),
        ("data controller", "DataController"),
        ("data processor", "DataProcessor"),
        ("data subject", "DataSubject"),
        ("data recipient", "DataRecipient"),
        ("data exporter", "DataExporter"),
        ("data importer", "DataImporter"),
        ("platform operator", "PlatformOperator"),
        ("marketplace operator", "MarketplaceOperator"),
        ("travel agent", "TravelAgent"),
        ("grid operator", "GridOperator"),
        ("network operator", "NetworkOperator"),
        ("roaming partner", "RoamingPartner"),
        ("labor union", "LaborUnion"),
        ("research institution", "ResearchInstitution"),
        ("arbitration tribunal", "Tribunal"),
        ("regulatory agency", "RegulatoryAgency"),
        ("tax authority", "TaxAuthority"),
        ("revenue agency", "RevenueAgency"),
        ("withholding agent", "WithholdingAgent"),
        ("account holder", "AccountHolder"),
        ("payment processor", "PaymentProcessor"),
        ("card issuer", "CardIssuer"),
        ("securities exchange", "SecuritiesExchange"),
        ("clearing house", "Clearinghouse"),
        ("clearinghouse", "Clearinghouse"),
        ("asset custodian", "AssetCustodian"),
        ("property manager", "PropertyManager"),
        ("title company", "TitleCompany"),
        ("escrow officer", "EscrowOfficer"),
        ("carbon registry", "CarbonRegistry"),
        ("credit buyer", "CreditBuyer"),
        ("credit seller", "CreditSeller"),
        ("project developer", "ProjectDeveloper"),
        ("patent office", "PatentOffice"),
        ("patent owner", "PatentOwner"),
        ("trademark owner", "TrademarkOwner"),
        ("rights holder", "RightsHolder"),
        ("environmental agency", "EnvironmentalAgency"),
        ("permit holder", "PermitHolder"),
        ("remediation contractor", "RemediationContractor"),
        ("monitoring lab", "MonitoringLab"),
        ("compliance officer", "ComplianceOfficer"),
        ("certification body", "CertificationBody"),
        ("audit committee", "AuditCommittee"),
        ("identity provider", "IdentityProvider"),
        ("relying party", "RelyingParty"),
        ("kyc provider", "KycProvider"),
        ("beneficial owner", "BeneficialOwner"),
        ("model provider", "ModelProvider"),
        ("model user", "ModelUser"),
        ("safety reviewer", "SafetyReviewer"),
        ("red team", "RedTeam"),
        ("agent coordinator", "AgentCoordinator"),
        ("task requester", "TaskRequester"),
        ("worker agent", "WorkerAgent"),
        ("tool provider", "ToolProvider"),
        ("buyer", "Buyer"),
        ("seller", "Seller"),
        ("offeror", "Offeror"),
        ("offeree", "Offeree"),
        ("promisor", "Promisor"),
        ("promisee", "Promisee"),
        ("provider", "Provider"),
        ("consumer", "Consumer"),
        ("patient", "Patient"),
        ("clinician", "Clinician"),
        ("physician", "Physician"),
        ("caregiver", "Caregiver"),
        ("student", "Student"),
        ("instructor", "Instructor"),
        ("teacher", "Instructor"),
        ("institution", "Institution"),
        ("traveler", "Traveler"),
        ("guest", "Guest"),
        ("host", "Host"),
        ("employer", "Employer"),
        ("employee", "Employee"),
        ("worker", "Worker"),
        ("publisher", "Publisher"),
        ("author", "Author"),
        ("editor", "Editor"),
        ("advertiser", "Advertiser"),
        ("sponsor", "Sponsor"),
        ("investigator", "Investigator"),
        ("participant", "Participant"),
        ("evaluator", "Evaluator"),
        ("plaintiff", "Plaintiff"),
        ("defendant", "Defendant"),
        ("counsel", "Counsel"),
        ("court", "Court"),
        ("claimant", "Claimant"),
        ("respondent", "Respondent"),
        ("tribunal", "Tribunal"),
        ("auditor", "Auditor"),
        ("auditee", "Auditee"),
        ("regulator", "Regulator"),
        ("applicant", "Applicant"),
        ("permittee", "Permittee"),
        ("taxpayer", "Taxpayer"),
        ("bank", "Bank"),
        ("cardholder", "Cardholder"),
        ("investor", "Investor"),
        ("underwriter", "Underwriter"),
        ("realtor", "Realtor"),
        ("utility", "Utility"),
        ("generator", "Generator"),
        ("offtaker", "Offtaker"),
        ("client", "Client"),
        ("contractor", "Contractor"),
        ("subcontractor", "Subcontractor"),
        ("architect", "Architect"),
        ("engineer", "Engineer"),
        ("broker", "Broker"),
        ("registrar", "Registrar"),
        ("registrant", "Registrant"),
        ("principal", "Principal"),
        ("agent", "Agent"),
        ("depositor", "Depositor"),
        ("deliverer", "Deliverer"),
        ("recipient", "Recipient"),
        ("sender", "Sender"),
        ("receiver", "Receiver"),
        ("auctioneer", "Auctioneer"),
        ("bidder", "Bidder"),
        ("payer", "Payer"),
        ("payee", "Payee"),
        ("borrower", "Borrower"),
        ("lender", "Lender"),
        ("debtor", "Debtor"),
        ("creditor", "Creditor"),
        ("obligor", "Obligor"),
        ("obligee", "Obligee"),
        ("pledgor", "Pledgor"),
        ("pledgee", "Pledgee"),
        ("mortgagor", "Mortgagor"),
        ("mortgagee", "Mortgagee"),
        ("trustor", "Trustor"),
        ("trustee", "Trustee"),
        ("beneficiary", "Beneficiary"),
        ("insurer", "Insurer"),
        ("insured", "Insured"),
        ("licensor", "Licensor"),
        ("licensee", "Licensee"),
        ("grantor", "Grantor"),
        ("grantee", "Grantee"),
        ("assignor", "Assignor"),
        ("assignee", "Assignee"),
        ("issuer", "Issuer"),
        ("holder", "Holder"),
        ("arbiter", "Arbiter"),
        ("arbitrator", "Arbiter"),
        ("mediator", "Arbiter"),
        ("reviewer", "Reviewer"),
        ("auditor", "Reviewer"),
        ("inspector", "Reviewer"),
        ("oracle", "Oracle"),
        ("verifier", "Verifier"),
        ("validator", "Verifier"),
        ("subscriber", "Subscriber"),
        ("moderator", "Moderator"),
        ("admin", "Admin"),
        ("proposer", "Proposer"),
        ("voter", "Voter"),
        ("delegate", "Delegate"),
        ("approver", "Approver"),
        ("authorizer", "Approver"),
        ("manager", "Approver"),
        ("supervisor", "Approver"),
        ("steward", "Steward"),
        ("custodian", "Steward"),
        ("governor", "Steward"),
        ("owner", "Owner"),
        ("user", "User"),
        ("vendor", "Vendor"),
        ("merchant", "Merchant"),
        ("supplier", "Supplier"),
        ("purchaser", "Purchaser"),
        ("manufacturer", "Manufacturer"),
        ("distributor", "Distributor"),
        ("reseller", "Reseller"),
        ("retailer", "Retailer"),
        ("wholesaler", "Wholesaler"),
        ("shipper", "Shipper"),
        ("carrier", "Carrier"),
        ("consignor", "Consignor"),
        ("consignee", "Consignee"),
        ("bailor", "Bailor"),
        ("bailee", "Bailee"),
        ("franchisor", "Franchisor"),
        ("franchisee", "Franchisee"),
        ("ship owner", "Shipowner"),
        ("shipowner", "Shipowner"),
        ("charterer", "Charterer"),
        ("indemnitor", "Indemnitor"),
        ("indemnitee", "Indemnitee"),
        ("guarantor", "Guarantor"),
        ("principal", "Principal"),
        ("warrantor", "Warrantor"),
        ("warrantee", "Warrantee"),
        ("donor", "Donor"),
        ("donee", "Donee"),
        ("customer", "Customer"),
        ("employee", "Employee"),
        ("employer", "Employer"),
        ("lessor", "Lessor"),
        ("lessee", "Lessee"),
        ("tenant", "Tenant"),
        ("landlord", "Landlord"),
        ("alice", "Alice"),
        ("bob", "Bob"),
        ("carol", "Carol"),
        ("dave", "Dave"),
        ("eve", "Eve"),
        ("frank", "Frank"),
    ];

    for (lower, proper) in common_names {
        if contains_party_pattern(&description_lower, lower) && !parties.contains(&proper.to_string()) {
            parties.push(proper.to_string());
        }
    }

    // Default if none found
    if parties.is_empty() {
        parties.push("PartyA".to_string());
        parties.push("PartyB".to_string());
    }

    parties
}

fn contains_party_pattern(text: &str, pattern: &str) -> bool {
    text.match_indices(pattern).any(|(start, matched)| {
        let end = start + matched.len();
        is_party_boundary(text[..start].chars().next_back())
            && is_party_boundary(text[end..].chars().next())
    })
}

fn is_party_boundary(ch: Option<char>) -> bool {
    ch.is_none_or(|ch| !ch.is_ascii_alphanumeric())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_llm_response() {
        let response = r#"
F1: always([+RELEASE] true -> eventually(<+DELIVER> true))
F2: always([+RELEASE] true -> <+signed_by(/users/alice.id)> true)
F3: always([+DELIVER] true -> <+signed_by(/users/bob.id)> true)
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(formulas.len(), 3);
        assert!(formulas[0].contains("RELEASE"));
        assert!(formulas[1].contains("signed_by"));
    }

    #[test]
    fn test_parse_llm_response_accepts_lowercase_prefix() {
        let response = "f1: always([+RELEASE] true -> eventually(<+DELIVER> true))";

        let formulas = parse_llm_response(response);
        assert_eq!(formulas.len(), 1);
        assert_eq!(
            formulas[0],
            "always([+RELEASE] true -> eventually(<+DELIVER> true))"
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_formula_prefix() {
        let response = "Formula 1: always([+RELEASE] true -> eventually(<+DELIVER> true))";

        let formulas = parse_llm_response(response);
        assert_eq!(formulas.len(), 1);
        assert_eq!(
            formulas[0],
            "always([+RELEASE] true -> eventually(<+DELIVER> true))"
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_unnumbered_formula_prefix() {
        let response = "Formula: always([+RELEASE] true -> eventually(<+DELIVER> true))";

        let formulas = parse_llm_response(response);
        assert_eq!(formulas.len(), 1);
        assert_eq!(
            formulas[0],
            "always([+RELEASE] true -> eventually(<+DELIVER> true))"
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_label_separators() {
        let response = r#"
F1. always([+PAY] true -> eventually(<+WORK> true))
Formula 2) <+CANCEL> true
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(formulas.len(), 2);
        assert_eq!(
            formulas[0],
            "always([+PAY] true -> eventually(<+WORK> true))"
        );
        assert_eq!(formulas[1], "<+CANCEL> true");
    }

    #[test]
    fn test_parse_llm_response_accepts_dash_label_separator() {
        let response = r#"
F1 - always([+PAY] true -> eventually(<+WORK> true))
Formula 2 - <+CANCEL> true
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(formulas.len(), 2);
        assert_eq!(
            formulas[0],
            "always([+PAY] true -> eventually(<+WORK> true))"
        );
        assert_eq!(formulas[1], "<+CANCEL> true");
    }

    #[test]
    fn test_parse_llm_response_accepts_equals_label_separator() {
        let response = r#"
F1 = always([+PAY] true -> eventually(<+WORK> true))
Formula 2 = <+CANCEL> true
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(formulas.len(), 2);
        assert_eq!(
            formulas[0],
            "always([+PAY] true -> eventually(<+WORK> true))"
        );
        assert_eq!(formulas[1], "<+CANCEL> true");
    }

    #[test]
    fn test_parse_llm_response_accepts_hash_numbered_labels() {
        let response = r#"
F#1: always([+PAY] true -> eventually(<+WORK> true))
Formula #2: <+CANCEL> true
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(formulas.len(), 2);
        assert_eq!(
            formulas[0],
            "always([+PAY] true -> eventually(<+WORK> true))"
        );
        assert_eq!(formulas[1], "<+CANCEL> true");
    }

    #[test]
    fn test_parse_llm_response_accepts_numeric_labels() {
        let response = r#"
1: always([+PAY] true -> eventually(<+WORK> true))
2 = <+CANCEL> true
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(formulas.len(), 2);
        assert_eq!(
            formulas[0],
            "always([+PAY] true -> eventually(<+WORK> true))"
        );
        assert_eq!(formulas[1], "<+CANCEL> true");
    }

    #[test]
    fn test_parse_llm_response_no_prefix() {
        let response = r#"
always([+PAY] true -> eventually(<+WORK> true))
[+EXECUTE] true -> <+signed_by(/users/admin.id)> true
<+CANCEL> true
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(formulas.len(), 3);
        assert_eq!(formulas[2], "<+CANCEL> true");
    }

    #[test]
    fn test_parse_llm_response_accepts_formula_declaration() {
        let response = "formula generated_1 { always([+PAY] true -> eventually(<+WORK> true)) }";

        let formulas = parse_llm_response(response);
        assert_eq!(formulas.len(), 1);
        assert_eq!(
            formulas[0],
            "formula generated_1 { always([+PAY] true -> eventually(<+WORK> true)) }"
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_multiline_formula_declaration() {
        let response = r#"
```modality
F1: formula generated_1 {
  always([+PAY] true -> eventually(<+WORK> true))
}
```
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(formulas.len(), 1);
        assert_eq!(
            formulas[0],
            "formula generated_1 {\nalways([+PAY] true -> eventually(<+WORK> true))\n}"
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_multiple_multiline_formula_declarations() {
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

        let formulas = parse_llm_response(response);
        assert_eq!(formulas.len(), 2);
        assert_eq!(formulas[0], "formula generated_1 {\nalways([<+APPROVE>] true)\n}");
        assert_eq!(
            formulas[1],
            "formula generated_2 {\n[+APPROVE] true -> <+signed_by(/users/reviewer.id)> true\n}"
        );
    }

    #[test]
    fn test_parse_llm_response_strips_list_markers() {
        let response = r#"
- always([+PAY] true -> eventually(<+WORK> true))
1. [+EXECUTE] true -> <+signed_by(/users/admin.id)> true
2) <+CANCEL> true
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(formulas.len(), 3);
        assert_eq!(
            formulas[0],
            "always([+PAY] true -> eventually(<+WORK> true))"
        );
        assert_eq!(
            formulas[1],
            "[+EXECUTE] true -> <+signed_by(/users/admin.id)> true"
        );
        assert_eq!(formulas[2], "<+CANCEL> true");
    }

    #[test]
    fn test_parse_llm_response_strips_list_markers_before_prefixes() {
        let response = r#"
- F1: always([+PAY] true -> eventually(<+WORK> true))
1. Formula 2: <+CANCEL> true
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(formulas.len(), 2);
        assert_eq!(
            formulas[0],
            "always([+PAY] true -> eventually(<+WORK> true))"
        );
        assert_eq!(formulas[1], "<+CANCEL> true");
    }

    #[test]
    fn test_parse_llm_response_strips_checklist_markers() {
        let response = r#"
- [ ] F1: always([+PAY] true -> eventually(<+WORK> true))
- [x] Formula 2: <+CANCEL> true
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(formulas.len(), 2);
        assert_eq!(
            formulas[0],
            "always([+PAY] true -> eventually(<+WORK> true))"
        );
        assert_eq!(formulas[1], "<+CANCEL> true");
    }

    #[test]
    fn test_parse_llm_response_strips_quote_markers() {
        let response = r#"
> F1: always([+PAY] true -> eventually(<+WORK> true))
> - <+CANCEL> true
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(formulas.len(), 2);
        assert_eq!(
            formulas[0],
            "always([+PAY] true -> eventually(<+WORK> true))"
        );
        assert_eq!(formulas[1], "<+CANCEL> true");
    }

    #[test]
    fn test_parse_llm_response_accepts_emphasized_labels() {
        let response = r#"
**F1:** always([+PAY] true -> eventually(<+WORK> true))
__Formula 2__: <+CANCEL> true
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(formulas.len(), 2);
        assert_eq!(
            formulas[0],
            "always([+PAY] true -> eventually(<+WORK> true))"
        );
        assert_eq!(formulas[1], "<+CANCEL> true");
    }

    #[test]
    fn test_parse_llm_response_strips_inline_code_wrapping() {
        let response = r#"
F1: `always([+PAY] true -> eventually(<+WORK> true))`
- `<+CANCEL> true`
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(formulas.len(), 2);
        assert_eq!(
            formulas[0],
            "always([+PAY] true -> eventually(<+WORK> true))"
        );
        assert_eq!(formulas[1], "<+CANCEL> true");
    }

    #[test]
    fn test_parse_llm_response_strips_quote_wrapping() {
        let response = r#"
F1: "always([+PAY] true -> eventually(<+WORK> true))"
- "<+CANCEL> true"
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(formulas.len(), 2);
        assert_eq!(
            formulas[0],
            "always([+PAY] true -> eventually(<+WORK> true))"
        );
        assert_eq!(formulas[1], "<+CANCEL> true");
    }

    #[test]
    fn test_parse_llm_response_strips_single_quote_wrapping() {
        let response = r#"
F1: 'always([+PAY] true -> eventually(<+WORK> true))'
- '<+CANCEL> true'
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(formulas.len(), 2);
        assert_eq!(
            formulas[0],
            "always([+PAY] true -> eventually(<+WORK> true))"
        );
        assert_eq!(formulas[1], "<+CANCEL> true");
    }

    #[test]
    fn test_parse_llm_response_strips_json_string_commas() {
        let response = r#"
[
  "always([+PAY] true -> eventually(<+WORK> true))",
  "<+CANCEL> true"
]
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(formulas.len(), 2);
        assert_eq!(
            formulas[0],
            "always([+PAY] true -> eventually(<+WORK> true))"
        );
        assert_eq!(formulas[1], "<+CANCEL> true");
    }

    #[test]
    fn test_parse_llm_response_strips_labeled_json_string_commas() {
        let response = r#"
F1: "always([+PAY] true -> eventually(<+WORK> true))",
Formula 2: "<+CANCEL> true",
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(formulas.len(), 2);
        assert_eq!(
            formulas[0],
            "always([+PAY] true -> eventually(<+WORK> true))"
        );
        assert_eq!(formulas[1], "<+CANCEL> true");
    }

    #[test]
    fn test_parse_llm_response_accepts_json_formula_fields() {
        let response = r#"
[
  {
    "label": "F1",
    "formula": "always([+PAY] true -> eventually(<+WORK> true))"
  },
  {
    "label": "F2",
    "formula": "<+CANCEL> true"
  }
]
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(formulas.len(), 2);
        assert_eq!(
            formulas[0],
            "always([+PAY] true -> eventually(<+WORK> true))"
        );
        assert_eq!(formulas[1], "<+CANCEL> true");
    }

    #[test]
    fn test_parse_llm_response_accepts_json_formulas_field() {
        let response = r#"
{
  "formulas": [
    "always([+PAY] true -> eventually(<+WORK> true))",
    "<+CANCEL> true"
  ],
  "notes": [
    "This explanatory string should not be parsed as a formula."
  ]
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(formulas.len(), 2);
        assert_eq!(
            formulas[0],
            "always([+PAY] true -> eventually(<+WORK> true))"
        );
        assert_eq!(formulas[1], "<+CANCEL> true");
    }

    #[test]
    fn test_parse_llm_response_accepts_fenced_json_formula_fields() {
        let response = r#"
```json
[
  {
    "label": "F1",
    "formula": "always([+PAY] true -> eventually(<+WORK> true))"
  },
  {
    "label": "F2",
    "formula": "<+CANCEL> true"
  }
]
```
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(formulas.len(), 2);
        assert_eq!(
            formulas[0],
            "always([+PAY] true -> eventually(<+WORK> true))"
        );
        assert_eq!(formulas[1], "<+CANCEL> true");
    }

    #[test]
    fn test_parse_llm_response_ignores_fenced_json_non_formula_fields() {
        let response = r#"
```json
{
  "notes": "always write an explanation",
  "formula": "<+CANCEL> true"
}
```
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(formulas, vec!["<+CANCEL> true"]);
    }

    #[test]
    fn test_parse_llm_response_accepts_json_message_content() {
        let response = r#"
{
  "choices": [
    {
      "message": {
        "role": "assistant",
        "content": "F1: always([+PAY] true -> eventually(<+WORK> true))\nF2: <+CANCEL> true"
      }
    }
  ]
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(formulas.len(), 2);
        assert_eq!(
            formulas[0],
            "always([+PAY] true -> eventually(<+WORK> true))"
        );
        assert_eq!(formulas[1], "<+CANCEL> true");
    }

    #[test]
    fn test_parse_llm_response_accepts_json_output_text() {
        let response = r#"
{
  "output": [
    {
      "content": [
        {
          "type": "output_text",
          "text": "F1: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
        }
      ]
    }
  ]
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec!["always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_tool_arguments() {
        let response = r#"
{
  "tool_calls": [
    {
      "function": {
        "name": "emit_formulas",
        "arguments": "{\"formulas\":[\"always([+PAY] true -> eventually(<+WORK> true))\",\"<+CANCEL> true\"]}"
      }
    }
  ]
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(formulas.len(), 2);
        assert_eq!(
            formulas[0],
            "always([+PAY] true -> eventually(<+WORK> true))"
        );
        assert_eq!(formulas[1], "<+CANCEL> true");
    }

    #[test]
    fn test_parse_llm_response_strips_markdown_emphasis_wrapping() {
        let response = r#"
F1: **always([+PAY] true -> eventually(<+WORK> true))**
- _<+CANCEL> true_
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(formulas.len(), 2);
        assert_eq!(
            formulas[0],
            "always([+PAY] true -> eventually(<+WORK> true))"
        );
        assert_eq!(formulas[1], "<+CANCEL> true");
    }

    #[test]
    fn test_parse_llm_response_accepts_markdown_table_rows() {
        let response = r#"
| Label | Formula |
| --- | --- |
| F1 | always([+PAY] true -> eventually(<+WORK> true)) |
| Formula 2 | `<+CANCEL> true` |
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(formulas.len(), 2);
        assert_eq!(
            formulas[0],
            "always([+PAY] true -> eventually(<+WORK> true))"
        );
        assert_eq!(formulas[1], "<+CANCEL> true");
    }

    #[test]
    fn test_parse_llm_response_accepts_table_formula_declarations() {
        let response = r#"
| Label | Formula |
| --- | --- |
| F1 | formula generated_1 { |
| | always([<+APPROVE>] true) |
| | } |
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(formulas.len(), 1);
        assert_eq!(formulas[0], "formula generated_1 {\nalways([<+APPROVE>] true)\n}");
    }

    #[test]
    fn test_extract_parties() {
        let parties = extract_parties("Alice wants to buy from Bob");
        assert!(parties.contains(&"Alice".to_string()));
        assert!(parties.contains(&"Bob".to_string()));
    }

    #[test]
    fn test_extract_specific_service_party_roles() {
        let parties = extract_parties(
            "Service provider and service consumer agree that party A pays party B",
        );

        assert!(parties.contains(&"ServiceProvider".to_string()));
        assert!(parties.contains(&"ServiceConsumer".to_string()));
        assert!(parties.contains(&"PartyA".to_string()));
        assert!(parties.contains(&"PartyB".to_string()));
    }

    #[test]
    fn test_extract_verification_party_roles() {
        let parties =
            extract_parties("Auditor and validator inspect delivery before arbitrator resolution");

        assert!(parties.contains(&"Reviewer".to_string()));
        assert!(parties.contains(&"Verifier".to_string()));
        assert!(parties.contains(&"Arbiter".to_string()));
    }

    #[test]
    fn test_extract_approval_party_roles() {
        let parties = extract_parties(
            "Manager authorization and supervisor approval require custodian oversight",
        );

        assert_eq!(
            parties
                .iter()
                .filter(|party| party.as_str() == "Approver")
                .count(),
            1
        );
        assert!(parties.contains(&"Steward".to_string()));
    }

    #[test]
    fn test_extract_payment_party_roles() {
        let parties = extract_parties("Payer deposits funds before the payee releases receipt");

        assert!(parties.contains(&"Payer".to_string()));
        assert!(parties.contains(&"Payee".to_string()));
    }

    #[test]
    fn test_extract_contract_formation_party_roles() {
        let parties =
            extract_parties("Offeror sends terms after promisor accepts duties to promisee and offeree");

        assert!(parties.contains(&"Offeror".to_string()));
        assert!(parties.contains(&"Offeree".to_string()));
        assert!(parties.contains(&"Promisor".to_string()));
        assert!(parties.contains(&"Promisee".to_string()));
    }

    #[test]
    fn test_extract_loan_party_roles() {
        let parties = extract_parties("Borrower repays the lender after collateral release");

        assert!(parties.contains(&"Borrower".to_string()));
        assert!(parties.contains(&"Lender".to_string()));
    }

    #[test]
    fn test_extract_debt_party_roles() {
        let parties = extract_parties("Debtor pays creditor before lien release");

        assert!(parties.contains(&"Debtor".to_string()));
        assert!(parties.contains(&"Creditor".to_string()));
    }

    #[test]
    fn test_extract_obligation_party_roles() {
        let parties = extract_parties("Obligor performs covenant before obligee releases waiver");

        assert!(parties.contains(&"Obligor".to_string()));
        assert!(parties.contains(&"Obligee".to_string()));
    }

    #[test]
    fn test_extract_pledge_party_roles() {
        let parties = extract_parties("Pledgor repays loan before pledgee releases collateral");

        assert!(parties.contains(&"Pledgor".to_string()));
        assert!(parties.contains(&"Pledgee".to_string()));
    }

    #[test]
    fn test_extract_mortgage_party_roles() {
        let parties = extract_parties("Mortgagor cures default before mortgagee releases lien");

        assert!(parties.contains(&"Mortgagor".to_string()));
        assert!(parties.contains(&"Mortgagee".to_string()));
    }

    #[test]
    fn test_extract_trust_party_roles() {
        let parties = extract_parties(
            "Trustor appoints trustee before beneficiary receives distribution",
        );

        assert!(parties.contains(&"Trustor".to_string()));
        assert!(parties.contains(&"Trustee".to_string()));
        assert!(parties.contains(&"Beneficiary".to_string()));
    }

    #[test]
    fn test_extract_insurance_party_roles() {
        let parties = extract_parties("Insurer approves claims before insured receives payout");

        assert!(parties.contains(&"Insurer".to_string()));
        assert!(parties.contains(&"Insured".to_string()));
    }

    #[test]
    fn test_extract_license_party_roles() {
        let parties = extract_parties("Licensor grants rights after the licensee signs terms");

        assert!(parties.contains(&"Licensor".to_string()));
        assert!(parties.contains(&"Licensee".to_string()));
    }

    #[test]
    fn test_extract_grant_party_roles() {
        let parties = extract_parties("Grantor transfers rights after the grantee accepts terms");

        assert!(parties.contains(&"Grantor".to_string()));
        assert!(parties.contains(&"Grantee".to_string()));
    }

    #[test]
    fn test_extract_assignment_party_roles() {
        let parties = extract_parties("Assignor transfers claims after assignee signs notice");

        assert!(parties.contains(&"Assignor".to_string()));
        assert!(parties.contains(&"Assignee".to_string()));
    }

    #[test]
    fn test_extract_credential_party_roles() {
        let parties = extract_parties("Issuer revokes credential after the holder fails renewal");

        assert!(parties.contains(&"Issuer".to_string()));
        assert!(parties.contains(&"Holder".to_string()));
    }

    #[test]
    fn test_extract_lease_party_roles() {
        let parties = extract_parties("Lessor permits access after lessee deposits collateral");

        assert!(parties.contains(&"Lessor".to_string()));
        assert!(parties.contains(&"Lessee".to_string()));
    }

    #[test]
    fn test_extract_procurement_party_roles() {
        let parties = extract_parties("Supplier ships goods after purchaser funds escrow");

        assert!(parties.contains(&"Supplier".to_string()));
        assert!(parties.contains(&"Purchaser".to_string()));
    }

    #[test]
    fn test_extract_healthcare_party_roles() {
        let parties = extract_parties(
            "Patient authorizes caregiver access after clinician and physician approve treatment",
        );

        assert!(parties.contains(&"Patient".to_string()));
        assert!(parties.contains(&"Caregiver".to_string()));
        assert!(parties.contains(&"Clinician".to_string()));
        assert!(parties.contains(&"Physician".to_string()));
    }

    #[test]
    fn test_extract_education_party_roles() {
        let parties =
            extract_parties("Student submits assignment after instructor and institution approve enrollment");

        assert!(parties.contains(&"Student".to_string()));
        assert!(parties.contains(&"Instructor".to_string()));
        assert!(parties.contains(&"Institution".to_string()));
    }

    #[test]
    fn test_extract_travel_party_roles() {
        let parties =
            extract_parties("Traveler books stay after guest, host, and travel agent confirm itinerary");

        assert!(parties.contains(&"Traveler".to_string()));
        assert!(parties.contains(&"Guest".to_string()));
        assert!(parties.contains(&"Host".to_string()));
        assert!(parties.contains(&"TravelAgent".to_string()));
    }

    #[test]
    fn test_extract_energy_party_roles() {
        let parties =
            extract_parties("Grid operator dispatches power after utility, generator, and offtaker agree");

        assert!(parties.contains(&"GridOperator".to_string()));
        assert!(parties.contains(&"Utility".to_string()));
        assert!(parties.contains(&"Generator".to_string()));
        assert!(parties.contains(&"Offtaker".to_string()));
    }

    #[test]
    fn test_extract_telecom_party_roles() {
        let parties = extract_parties(
            "Network operator activates service after subscriber and roaming partner accept terms",
        );

        assert!(parties.contains(&"NetworkOperator".to_string()));
        assert!(parties.contains(&"Subscriber".to_string()));
        assert!(parties.contains(&"RoamingPartner".to_string()));
    }

    #[test]
    fn test_extract_employment_party_roles() {
        let parties = extract_parties(
            "Employer schedules training after employee, worker, and labor union approve policy",
        );

        assert!(parties.contains(&"Employer".to_string()));
        assert!(parties.contains(&"Employee".to_string()));
        assert!(parties.contains(&"Worker".to_string()));
        assert!(parties.contains(&"LaborUnion".to_string()));
    }

    #[test]
    fn test_extract_publishing_party_roles() {
        let parties =
            extract_parties("Publisher releases article after author, editor, and advertiser approve copy");

        assert!(parties.contains(&"Publisher".to_string()));
        assert!(parties.contains(&"Author".to_string()));
        assert!(parties.contains(&"Editor".to_string()));
        assert!(parties.contains(&"Advertiser".to_string()));
    }

    #[test]
    fn test_extract_research_party_roles() {
        let parties = extract_parties(
            "Sponsor funds trial after investigator, participant, and research institution approve protocol",
        );

        assert!(parties.contains(&"Sponsor".to_string()));
        assert!(parties.contains(&"Investigator".to_string()));
        assert!(parties.contains(&"Participant".to_string()));
        assert!(parties.contains(&"ResearchInstitution".to_string()));
    }

    #[test]
    fn test_extract_litigation_party_roles() {
        let parties =
            extract_parties("Plaintiff settles claim after defendant, counsel, and court approve order");

        assert!(parties.contains(&"Plaintiff".to_string()));
        assert!(parties.contains(&"Defendant".to_string()));
        assert!(parties.contains(&"Counsel".to_string()));
        assert!(parties.contains(&"Court".to_string()));
    }

    #[test]
    fn test_extract_arbitration_party_roles() {
        let parties = extract_parties(
            "Claimant files notice after respondent, arbitrator, and arbitration tribunal approve award",
        );

        assert!(parties.contains(&"Claimant".to_string()));
        assert!(parties.contains(&"Respondent".to_string()));
        assert!(parties.contains(&"Arbiter".to_string()));
        assert!(parties.contains(&"Tribunal".to_string()));
    }

    #[test]
    fn test_extract_regulatory_party_roles() {
        let parties = extract_parties(
            "Regulator grants permit after applicant, permittee, and regulatory agency approve filing",
        );

        assert!(parties.contains(&"Regulator".to_string()));
        assert!(parties.contains(&"Applicant".to_string()));
        assert!(parties.contains(&"Permittee".to_string()));
        assert!(parties.contains(&"RegulatoryAgency".to_string()));
    }

    #[test]
    fn test_extract_tax_party_roles() {
        let parties = extract_parties(
            "Taxpayer remits return after tax authority, withholding agent, and revenue agency approve filing",
        );

        assert!(parties.contains(&"Taxpayer".to_string()));
        assert!(parties.contains(&"TaxAuthority".to_string()));
        assert!(parties.contains(&"WithholdingAgent".to_string()));
        assert!(parties.contains(&"RevenueAgency".to_string()));
    }

    #[test]
    fn test_extract_finance_party_roles() {
        let parties = extract_parties(
            "Bank settles transfer after account holder, cardholder, card issuer, and payment processor approve charge",
        );

        assert!(parties.contains(&"Bank".to_string()));
        assert!(parties.contains(&"AccountHolder".to_string()));
        assert!(parties.contains(&"Cardholder".to_string()));
        assert!(parties.contains(&"CardIssuer".to_string()));
        assert!(parties.contains(&"PaymentProcessor".to_string()));
    }

    #[test]
    fn test_extract_securities_party_roles() {
        let parties = extract_parties(
            "Investor subscribes after underwriter, securities exchange, clearinghouse, and asset custodian approve settlement",
        );

        assert!(parties.contains(&"Investor".to_string()));
        assert!(parties.contains(&"Underwriter".to_string()));
        assert!(parties.contains(&"SecuritiesExchange".to_string()));
        assert!(parties.contains(&"Clearinghouse".to_string()));
        assert!(parties.contains(&"AssetCustodian".to_string()));
    }

    #[test]
    fn test_extract_real_estate_party_roles() {
        let parties = extract_parties(
            "Landlord transfers keys after tenant, realtor, property manager, title company, and escrow officer approve closing",
        );

        assert!(parties.contains(&"Landlord".to_string()));
        assert!(parties.contains(&"Tenant".to_string()));
        assert!(parties.contains(&"Realtor".to_string()));
        assert!(parties.contains(&"PropertyManager".to_string()));
        assert!(parties.contains(&"TitleCompany".to_string()));
        assert!(parties.contains(&"EscrowOfficer".to_string()));
    }

    #[test]
    fn test_extract_carbon_market_party_roles() {
        let parties = extract_parties(
            "Credit buyer retires offsets after credit seller, project developer, and carbon registry approve issuance",
        );

        assert!(parties.contains(&"CreditBuyer".to_string()));
        assert!(parties.contains(&"CreditSeller".to_string()));
        assert!(parties.contains(&"ProjectDeveloper".to_string()));
        assert!(parties.contains(&"CarbonRegistry".to_string()));
    }

    #[test]
    fn test_extract_ip_party_roles() {
        let parties = extract_parties(
            "Patent owner licenses invention after patent office, trademark owner, and rights holder approve filing",
        );

        assert!(parties.contains(&"PatentOwner".to_string()));
        assert!(parties.contains(&"PatentOffice".to_string()));
        assert!(parties.contains(&"TrademarkOwner".to_string()));
        assert!(parties.contains(&"RightsHolder".to_string()));
    }

    #[test]
    fn test_extract_environmental_party_roles() {
        let parties = extract_parties(
            "Permit holder reports remediation work after environmental agency, remediation contractor, and monitoring lab approve cleanup",
        );

        assert!(parties.contains(&"PermitHolder".to_string()));
        assert!(parties.contains(&"EnvironmentalAgency".to_string()));
        assert!(parties.contains(&"RemediationContractor".to_string()));
        assert!(parties.contains(&"MonitoringLab".to_string()));
    }

    #[test]
    fn test_extract_audit_party_roles() {
        let parties = extract_parties(
            "Auditor files attestation after auditee, compliance officer, certification body, and audit committee approve controls",
        );

        assert!(parties.contains(&"Auditor".to_string()));
        assert!(parties.contains(&"Auditee".to_string()));
        assert!(parties.contains(&"ComplianceOfficer".to_string()));
        assert!(parties.contains(&"CertificationBody".to_string()));
        assert!(parties.contains(&"AuditCommittee".to_string()));
    }

    #[test]
    fn test_extract_kyc_party_roles() {
        let parties = extract_parties(
            "Relying party accepts onboarding after identity provider, KYC provider, and beneficial owner approve verification",
        );

        assert!(parties.contains(&"RelyingParty".to_string()));
        assert!(parties.contains(&"IdentityProvider".to_string()));
        assert!(parties.contains(&"KycProvider".to_string()));
        assert!(parties.contains(&"BeneficialOwner".to_string()));
    }

    #[test]
    fn test_extract_model_governance_party_roles() {
        let parties = extract_parties(
            "Model provider releases weights after model user, evaluator, safety reviewer, and red team approve deployment",
        );

        assert!(parties.contains(&"ModelProvider".to_string()));
        assert!(parties.contains(&"ModelUser".to_string()));
        assert!(parties.contains(&"Evaluator".to_string()));
        assert!(parties.contains(&"SafetyReviewer".to_string()));
        assert!(parties.contains(&"RedTeam".to_string()));
    }

    #[test]
    fn test_extract_agent_coordination_party_roles() {
        let parties = extract_parties(
            "Agent coordinator assigns work after task requester, worker agent, and tool provider approve capability terms",
        );

        assert!(parties.contains(&"AgentCoordinator".to_string()));
        assert!(parties.contains(&"TaskRequester".to_string()));
        assert!(parties.contains(&"WorkerAgent".to_string()));
        assert!(parties.contains(&"ToolProvider".to_string()));
    }

    #[test]
    fn test_extract_construction_party_roles() {
        let parties = extract_parties(
            "Owner accepts plans after architect, engineer, contractor, and subcontractor certify work",
        );

        assert!(parties.contains(&"Owner".to_string()));
        assert!(parties.contains(&"Architect".to_string()));
        assert!(parties.contains(&"Engineer".to_string()));
        assert!(parties.contains(&"Contractor".to_string()));
        assert!(parties.contains(&"Subcontractor".to_string()));
    }

    #[test]
    fn test_extract_supply_chain_party_roles() {
        let parties = extract_parties(
            "Manufacturer ships goods to distributor before reseller, retailer, and wholesaler confirm allocation",
        );

        assert!(parties.contains(&"Manufacturer".to_string()));
        assert!(parties.contains(&"Distributor".to_string()));
        assert!(parties.contains(&"Reseller".to_string()));
        assert!(parties.contains(&"Retailer".to_string()));
        assert!(parties.contains(&"Wholesaler".to_string()));
    }

    #[test]
    fn test_extract_logistics_party_roles() {
        let parties =
            extract_parties("Shipper tenders goods to carrier before consignee confirms receipt");

        assert!(parties.contains(&"Shipper".to_string()));
        assert!(parties.contains(&"Carrier".to_string()));
        assert!(parties.contains(&"Consignee".to_string()));
    }

    #[test]
    fn test_extract_bailment_party_roles() {
        let parties = extract_parties("Bailor deposits equipment before bailee returns custody");

        assert!(parties.contains(&"Bailor".to_string()));
        assert!(parties.contains(&"Bailee".to_string()));
    }

    #[test]
    fn test_extract_franchise_party_roles() {
        let parties = extract_parties("Franchisor approves opening before franchisee pays fees");

        assert!(parties.contains(&"Franchisor".to_string()));
        assert!(parties.contains(&"Franchisee".to_string()));
    }

    #[test]
    fn test_extract_charter_party_roles() {
        let parties = extract_parties("Ship owner delivers vessel before charterer remits hire");

        assert!(parties.contains(&"Shipowner".to_string()));
        assert!(parties.contains(&"Charterer".to_string()));
    }

    #[test]
    fn test_extract_indemnity_party_roles() {
        let parties = extract_parties("Indemnitor reimburses losses after indemnitee files claim");

        assert!(parties.contains(&"Indemnitor".to_string()));
        assert!(parties.contains(&"Indemnitee".to_string()));
    }

    #[test]
    fn test_extract_guarantee_party_roles() {
        let parties = extract_parties("Guarantor pays if principal defaults on obligation");

        assert!(parties.contains(&"Guarantor".to_string()));
        assert!(parties.contains(&"Principal".to_string()));
    }

    #[test]
    fn test_extract_warranty_party_roles() {
        let parties = extract_parties("Warrantor repairs defects after warrantee reports failure");

        assert!(parties.contains(&"Warrantor".to_string()));
        assert!(parties.contains(&"Warrantee".to_string()));
    }

    #[test]
    fn test_extract_gift_party_roles() {
        let parties = extract_parties("Donor transfers artwork after donee accepts conditions");

        assert!(parties.contains(&"Donor".to_string()));
        assert!(parties.contains(&"Donee".to_string()));
    }

    #[test]
    fn test_extract_brokerage_party_roles() {
        let parties = extract_parties("Broker executes trade after client approves order");

        assert!(parties.contains(&"Broker".to_string()));
        assert!(parties.contains(&"Client".to_string()));
    }

    #[test]
    fn test_extract_escrow_agent_party_roles() {
        let parties =
            extract_parties("Escrow agent releases funds after buyer accepts seller delivery");

        assert!(parties.contains(&"EscrowAgent".to_string()));
        assert!(parties.contains(&"Buyer".to_string()));
        assert!(parties.contains(&"Seller".to_string()));
    }

    #[test]
    fn test_extract_registry_party_roles() {
        let parties = extract_parties("Registrar renews domain after registrant pays fee");

        assert!(parties.contains(&"Registrar".to_string()));
        assert!(parties.contains(&"Registrant".to_string()));
    }

    #[test]
    fn test_extract_auction_party_roles() {
        let parties = extract_parties("Auctioneer awards lot after bidder satisfies reserve");

        assert!(parties.contains(&"Auctioneer".to_string()));
        assert!(parties.contains(&"Bidder".to_string()));
    }

    #[test]
    fn test_extract_platform_party_roles() {
        let parties = extract_parties(
            "Platform operator escrows listing before marketplace operator releases vendor payout",
        );

        assert!(parties.contains(&"PlatformOperator".to_string()));
        assert!(parties.contains(&"MarketplaceOperator".to_string()));
        assert!(parties.contains(&"Vendor".to_string()));
    }

    #[test]
    fn test_extract_governance_party_roles() {
        let parties = extract_parties("Proposer submits budget before voter and delegate approve");

        assert!(parties.contains(&"Proposer".to_string()));
        assert!(parties.contains(&"Voter".to_string()));
        assert!(parties.contains(&"Delegate".to_string()));
    }

    #[test]
    fn test_extract_data_processing_party_roles() {
        let parties = extract_parties(
            "Data exporter transfers data subject records to data importer after data controller approves data processor export to data recipient",
        );

        assert!(parties.contains(&"DataController".to_string()));
        assert!(parties.contains(&"DataProcessor".to_string()));
        assert!(parties.contains(&"DataSubject".to_string()));
        assert!(parties.contains(&"DataRecipient".to_string()));
        assert!(parties.contains(&"DataExporter".to_string()));
        assert!(parties.contains(&"DataImporter".to_string()));
    }

    #[test]
    fn test_extract_party_roles_require_token_boundaries() {
        let parties = extract_parties("Stakeholder signs after shareholder review");

        assert!(!parties.contains(&"Holder".to_string()));
        assert!(parties.contains(&"PartyA".to_string()));
        assert!(parties.contains(&"PartyB".to_string()));
    }

    #[test]
    fn test_prompt_includes_multi_signer_authorization_pattern() {
        let prompt = generate_prompt("Approval requires Alice and Bob signatures");

        assert!(prompt.contains("<+signed_by(/users/a.id) +signed_by(/users/b.id)> true"));
        assert!(prompt.contains("[<+signed_by(/users/a.id) +signed_by(/users/b.id)>] true"));
    }

    #[test]
    fn test_prompt_includes_oracle_attestation_pattern() {
        let prompt = generate_prompt("Release requires oracle attestation");

        assert!(prompt.contains(
            r#"always([+X] true -> <+oracle_attests(/oracles/a.id, "delivered", "true")> true)"#
        ));
    }

    #[test]
    fn test_prompt_includes_parser_backed_implication_guidance() {
        let prompt = generate_prompt("Release requires delivery");

        assert!(prompt.contains("Prefer `φ -> ψ` for implications."));
        assert!(prompt.contains("[+X] true -> eventually(<+Y> true)"));
    }

    #[test]
    fn test_prompt_includes_committed_action_authorization_pattern() {
        let prompt = generate_prompt("Committed release requires buyer signature");

        assert!(prompt.contains("always([<+X>] true -> <+signed_by(/users/a.id)> true)"));
        assert!(prompt.contains(
            "always([<+X>] true -> <+signed_by(/users/a.id) +signed_by(/users/b.id)> true)"
        ));
        assert!(prompt.contains("always([<+X>] true -> [<+signed_by(/users/a.id)>] true)"));
        assert!(prompt.contains(
            "always([<+X>] true -> [<+signed_by(/users/a.id) +signed_by(/users/b.id)>] true)"
        ));
        assert!(prompt.contains(
            "always([+X] true -> (<+signed_by(/users/a.id)> true & eventually(<+Y> true)))"
        ));
        assert!(prompt.contains(
            "always([+X] true -> (<+signed_by(/users/a.id)> true & (eventually(<+Y> true) & eventually(<+Z> true))))"
        ));
        assert!(prompt.contains(
            "always([+X] true -> (<+signed_by(/users/a.id)> true & eventually([<+Y>] true)))"
        ));
        assert!(prompt.contains(
            "always([+X] true -> (<+signed_by(/users/a.id)> true & (eventually([<+Y>] true) & eventually([<+Z>] true))))"
        ));
        assert!(prompt.contains(
            "always([+X] true -> ([<+signed_by(/users/a.id)>] true & eventually(<+Y> true)))"
        ));
        assert!(prompt.contains(
            "always([+X] true -> ([<+signed_by(/users/a.id)>] true & (eventually(<+Y> true) & eventually(<+Z> true))))"
        ));
        assert!(prompt.contains(
            "always([+X] true -> ([<+signed_by(/users/a.id)>] true & eventually([<+Y>] true)))"
        ));
        assert!(prompt.contains(
            "always([+X] true -> ([<+signed_by(/users/a.id)>] true & (eventually([<+Y>] true) & eventually([<+Z>] true))))"
        ));
        assert!(prompt.contains(
            "always([+X] true -> (<+signed_by(/users/a.id) +signed_by(/users/b.id)> true & eventually(<+Y> true)))"
        ));
        assert!(prompt.contains(
            "always([+X] true -> (<+signed_by(/users/a.id) +signed_by(/users/b.id)> true & (eventually(<+Y> true) & eventually(<+Z> true))))"
        ));
        assert!(prompt.contains(
            "always([+X] true -> (<+signed_by(/users/a.id) +signed_by(/users/b.id)> true & eventually([<+Y>] true)))"
        ));
        assert!(prompt.contains(
            "always([+X] true -> (<+signed_by(/users/a.id) +signed_by(/users/b.id)> true & (eventually([<+Y>] true) & eventually([<+Z>] true))))"
        ));
        assert!(prompt.contains(
            "always([+X] true -> ([<+signed_by(/users/a.id) +signed_by(/users/b.id)>] true & eventually([<+Y>] true)))"
        ));
        assert!(prompt.contains(
            "always([+X] true -> ([<+signed_by(/users/a.id) +signed_by(/users/b.id)>] true & (eventually([<+Y>] true) & eventually([<+Z>] true))))"
        ));
        assert!(prompt.contains(
            "always([+X] true -> ([<+signed_by(/users/a.id) +signed_by(/users/b.id)>] true & eventually(<+Y> true)))"
        ));
        assert!(prompt.contains(
            "always([+X] true -> ([<+signed_by(/users/a.id) +signed_by(/users/b.id)>] true & (eventually(<+Y> true) & eventually(<+Z> true))))"
        ));
        assert!(prompt.contains(
            "always([<+X>] true -> (<+signed_by(/users/a.id)> true & eventually([<+Y>] true)))"
        ));
        assert!(prompt.contains(
            "always([<+X>] true -> (<+signed_by(/users/a.id)> true & (eventually([<+Y>] true) & eventually([<+Z>] true))))"
        ));
        assert!(prompt.contains(
            "always([<+X>] true -> (<+signed_by(/users/a.id)> true & eventually(<+Y> true)))"
        ));
        assert!(prompt.contains(
            "always([<+X>] true -> (<+signed_by(/users/a.id)> true & (eventually(<+Y> true) & eventually(<+Z> true))))"
        ));
        assert!(prompt.contains(
            "always([<+X>] true -> ([<+signed_by(/users/a.id)>] true & eventually(<+Y> true)))"
        ));
        assert!(prompt.contains(
            "always([<+X>] true -> ([<+signed_by(/users/a.id)>] true & (eventually(<+Y> true) & eventually(<+Z> true))))"
        ));
        assert!(prompt.contains(
            "always([<+X>] true -> ([<+signed_by(/users/a.id)>] true & eventually([<+Y>] true)))"
        ));
        assert!(prompt.contains(
            "always([<+X>] true -> ([<+signed_by(/users/a.id)>] true & (eventually([<+Y>] true) & eventually([<+Z>] true))))"
        ));
        assert!(prompt.contains(
            "always([<+X>] true -> (<+signed_by(/users/a.id) +signed_by(/users/b.id)> true & eventually([<+Y>] true)))"
        ));
        assert!(prompt.contains(
            "always([<+X>] true -> (<+signed_by(/users/a.id) +signed_by(/users/b.id)> true & (eventually([<+Y>] true) & eventually([<+Z>] true))))"
        ));
        assert!(prompt.contains(
            "always([<+X>] true -> (<+signed_by(/users/a.id) +signed_by(/users/b.id)> true & eventually(<+Y> true)))"
        ));
        assert!(prompt.contains(
            "always([<+X>] true -> (<+signed_by(/users/a.id) +signed_by(/users/b.id)> true & (eventually(<+Y> true) & eventually(<+Z> true))))"
        ));
        assert!(prompt.contains(
            "always([<+X>] true -> ([<+signed_by(/users/a.id) +signed_by(/users/b.id)>] true & eventually(<+Y> true)))"
        ));
        assert!(prompt.contains(
            "always([<+X>] true -> ([<+signed_by(/users/a.id) +signed_by(/users/b.id)>] true & (eventually(<+Y> true) & eventually(<+Z> true))))"
        ));
        assert!(prompt.contains(
            "always([<+X>] true -> ([<+signed_by(/users/a.id) +signed_by(/users/b.id)>] true & eventually([<+Y>] true)))"
        ));
        assert!(prompt.contains(
            "always([<+X>] true -> ([<+signed_by(/users/a.id) +signed_by(/users/b.id)>] true & (eventually([<+Y>] true) & eventually([<+Z>] true))))"
        ));
    }

    #[test]
    fn test_prompt_includes_direct_diamond_patterns() {
        let prompt = generate_prompt("Approval is always allowed");

        assert!(prompt.contains("`<+X> true`"));
        assert!(prompt.contains("`[<+X>] true`"));
        assert!(prompt.contains("`always([<+X>] true)`"));
        assert!(prompt.contains("`always([<+X>] true & [<+Y>] true)`"));
    }

    #[test]
    fn test_prompt_includes_committed_goal_patterns() {
        let prompt = generate_prompt("Release requires committed delivery and reviewer signature");

        assert!(prompt.contains("always([<+X>] true -> eventually(<+Y> true))"));
        assert!(prompt.contains("always([<+X>] true -> eventually([<+Y>] true))"));
        assert!(prompt.contains("eventually([<+Y>] true)"));
        assert!(prompt
            .contains("always([<+X>] true -> (eventually(<+Y> true) & eventually(<+Z> true)))"));
        assert!(prompt.contains(
            "always([<+X>] true -> (eventually([<+Y>] true) & eventually([<+Z>] true)))"
        ));
        assert!(prompt.contains("eventually([<+Y>] true) & eventually([<+Z>] true)"));
        assert!(prompt.contains("[<+signed_by(/users/a.id)>] true"));
    }

    #[test]
    fn test_prompt_includes_compound_forbidden_after_pattern() {
        let prompt = generate_prompt("Never release or refund after dispute");

        assert!(prompt.contains("always([+X] true -> (always([-Y] true) & always([-Z] true)))"));
        assert!(prompt.contains("always([<+X>] true -> always([-Y] true))"));
        assert!(prompt.contains("always([<+X>] true -> (always([-Y] true) & always([-Z] true)))"));
        assert!(prompt
            .contains("always([+X] true -> (<+signed_by(/users/a.id)> true & always([-Y] true)))"));
        assert!(prompt.contains(
            "always([+X] true -> (<+signed_by(/users/a.id) +signed_by(/users/b.id)> true & always([-Y] true)))"
        ));
        assert!(prompt.contains(
            "always([+X] true -> (<+signed_by(/users/a.id)> true & (always([-Y] true) & always([-Z] true))))"
        ));
        assert!(prompt.contains(
            "always([+X] true -> (<+signed_by(/users/a.id) +signed_by(/users/b.id)> true & (always([-Y] true) & always([-Z] true))))"
        ));
        assert!(prompt.contains(
            "always([<+X>] true -> (<+signed_by(/users/a.id)> true & always([-Y] true)))"
        ));
        assert!(prompt.contains(
            "always([<+X>] true -> (<+signed_by(/users/a.id) +signed_by(/users/b.id)> true & always([-Y] true)))"
        ));
        assert!(prompt.contains(
            "always([<+X>] true -> (<+signed_by(/users/a.id)> true & (always([-Y] true) & always([-Z] true))))"
        ));
        assert!(prompt.contains(
            "always([<+X>] true -> (<+signed_by(/users/a.id) +signed_by(/users/b.id)> true & (always([-Y] true) & always([-Z] true))))"
        ));
        assert!(prompt.contains(
            "always([+X] true -> ([<+signed_by(/users/a.id)>] true & always([-Y] true)))"
        ));
        assert!(prompt.contains(
            "always([+X] true -> ([<+signed_by(/users/a.id) +signed_by(/users/b.id)>] true & always([-Y] true)))"
        ));
        assert!(prompt.contains(
            "always([<+X>] true -> ([<+signed_by(/users/a.id)>] true & always([-Y] true)))"
        ));
        assert!(prompt.contains(
            "always([<+X>] true -> ([<+signed_by(/users/a.id) +signed_by(/users/b.id)>] true & always([-Y] true)))"
        ));
        assert!(prompt.contains(
            "always([+X] true -> ([<+signed_by(/users/a.id)>] true & (always([-Y] true) & always([-Z] true))))"
        ));
        assert!(prompt.contains(
            "always([+X] true -> ([<+signed_by(/users/a.id) +signed_by(/users/b.id)>] true & (always([-Y] true) & always([-Z] true))))"
        ));
        assert!(prompt.contains(
            "always([<+X>] true -> ([<+signed_by(/users/a.id)>] true & (always([-Y] true) & always([-Z] true))))"
        ));
        assert!(prompt.contains(
            "always([<+X>] true -> ([<+signed_by(/users/a.id) +signed_by(/users/b.id)>] true & (always([-Y] true) & always([-Z] true))))"
        ));
    }

    #[test]
    fn test_prompt_includes_agent_coordination_patterns() {
        let prompt = generate_prompt("Agent coordinator assigns work to a worker agent");

        assert!(prompt.contains(
            "always([+AGENT_A_TURN] true -> eventually(<+AGENT_B_TURN> true))"
        ));
        assert!(prompt.contains(
            "always([+AGENT_B_TURN] true -> eventually(<+AGENT_A_TURN> true))"
        ));
        assert!(prompt.contains(
            "always([+ASSIGN_TASK] true -> <+signed_by(/users/task_requester.id) +signed_by(/users/worker_agent.id)> true)"
        ));
        assert!(prompt.contains(
            "always([+USE_TOOL] true -> (<+signed_by(/users/tool_provider.id)> true & eventually([<+APPROVE_CAPABILITY>] true)))"
        ));
    }

    #[test]
    fn test_prompt_includes_escrow_progression_pattern() {
        let prompt = generate_prompt("Escrow deposit before delivery before release");

        assert!(prompt.contains("always([+DELIVER] true -> eventually(<+DEPOSIT> true))"));
        assert!(prompt.contains("always([+RELEASE] true -> eventually(<+DELIVER> true))"));
    }

    #[test]
    fn test_prompt_includes_dispute_resolution_pattern() {
        let prompt = generate_prompt("Dispute blocks release or refund until arbiter resolution");

        assert!(prompt.contains(
            "always([+DISPUTE] true -> (always([-RELEASE] true) & always([-REFUND] true)))"
        ));
        assert!(prompt.contains(
            "always([+RESOLVE_DISPUTE] true -> <+signed_by(/users/arbiter.id)> true)"
        ));
    }

    #[test]
    fn test_prompt_includes_cancellation_pattern() {
        let prompt = generate_prompt("Cancel requires requester signature and blocks delivery");

        assert!(prompt
            .contains("always([+CANCEL] true -> <+signed_by(/users/requester.id)> true)"));
        assert!(prompt.contains("always([+CANCEL] true -> always([-DELIVER] true))"));
    }

    #[test]
    fn test_prompt_includes_refund_pattern() {
        let prompt = generate_prompt("Refund requires seller signature and blocks release");

        assert!(
            prompt.contains("always([+REFUND] true -> <+signed_by(/users/seller.id)> true)")
        );
        assert!(prompt.contains("always([+REFUND] true -> always([-RELEASE] true))"));
    }

    #[test]
    fn test_prompt_includes_review_approval_pattern() {
        let prompt = generate_prompt("Approve requires reviewer signature and blocks rejection");

        assert!(
            prompt.contains("always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)")
        );
        assert!(prompt.contains("always([+APPROVE] true -> always([-REJECT] true))"));
    }

    #[test]
    fn test_prompt_includes_review_rejection_pattern() {
        let prompt = generate_prompt("Reject requires reviewer signature and blocks approval");

        assert!(
            prompt.contains("always([+REJECT] true -> <+signed_by(/users/reviewer.id)> true)")
        );
        assert!(prompt.contains("always([+REJECT] true -> always([-APPROVE] true))"));
    }

    #[test]
    fn test_prompt_includes_timeout_pattern() {
        let prompt = generate_prompt("Timeout requires clock oracle and blocks completion");

        assert!(prompt.contains(
            "always([+TIMEOUT] true -> <+oracle_attests(/oracles/clock.id, \"deadline_passed\", \"true\")> true)"
        ));
        assert!(prompt.contains("always([+TIMEOUT] true -> always([-COMPLETE] true))"));
    }

    #[test]
    fn test_prompt_includes_escalation_pattern() {
        let prompt = generate_prompt("Escalation requires manager signature and blocks close");

        assert!(
            prompt.contains("always([+ESCALATE] true -> <+signed_by(/users/manager.id)> true)")
        );
        assert!(prompt.contains("always([+ESCALATE] true -> always([-CLOSE] true))"));
    }

    #[test]
    fn test_prompt_includes_withdrawal_pattern() {
        let prompt = generate_prompt("Withdrawal requires depositor signature and blocks claim");

        assert!(
            prompt.contains("always([+WITHDRAW] true -> <+signed_by(/users/depositor.id)> true)")
        );
        assert!(prompt.contains("always([+WITHDRAW] true -> always([-CLAIM] true))"));
    }

    #[test]
    fn test_prompt_includes_appeal_pattern() {
        let prompt = generate_prompt("Appeal requires appellant signature and blocks enforcement");

        assert!(
            prompt.contains("always([+APPEAL] true -> <+signed_by(/users/appellant.id)> true)")
        );
        assert!(prompt.contains("always([+APPEAL] true -> always([-ENFORCE] true))"));
    }

    #[test]
    fn test_prompt_includes_revocation_pattern() {
        let prompt = generate_prompt("Revocation requires issuer signature and blocks use");

        assert!(
            prompt.contains("always([+REVOKE] true -> <+signed_by(/users/issuer.id)> true)")
        );
        assert!(prompt.contains("always([+REVOKE] true -> always([-USE] true))"));
    }

    #[test]
    fn test_prompt_includes_suspension_pattern() {
        let prompt = generate_prompt("Suspension requires administrator signature and blocks access");

        assert!(prompt.contains(
            "always([+SUSPEND] true -> <+signed_by(/users/administrator.id)> true)"
        ));
        assert!(prompt.contains("always([+SUSPEND] true -> always([-ACCESS] true))"));
    }

    #[test]
    fn test_prompt_includes_reinstatement_pattern() {
        let prompt =
            generate_prompt("Reinstatement requires administrator signature and blocks suspension");

        assert!(prompt.contains(
            "always([+REINSTATE] true -> <+signed_by(/users/administrator.id)> true)"
        ));
        assert!(prompt.contains("always([+REINSTATE] true -> always([-SUSPEND] true))"));
    }

    #[test]
    fn test_prompt_includes_renewal_pattern() {
        let prompt = generate_prompt("Renewal requires holder signature and blocks expiration");

        assert!(
            prompt.contains("always([+RENEW] true -> <+signed_by(/users/holder.id)> true)")
        );
        assert!(prompt.contains("always([+RENEW] true -> always([-EXPIRE] true))"));
    }

    #[test]
    fn test_prompt_includes_termination_pattern() {
        let prompt =
            generate_prompt("Termination requires counterparty signature and blocks renewal");

        assert!(prompt.contains(
            "always([+TERMINATE] true -> <+signed_by(/users/counterparty.id)> true)"
        ));
        assert!(prompt.contains("always([+TERMINATE] true -> always([-RENEW] true))"));
    }

    #[test]
    fn test_prompt_includes_extension_pattern() {
        let prompt = generate_prompt("Extension requires owner signature and blocks termination");

        assert!(prompt.contains("always([+EXTEND] true -> <+signed_by(/users/owner.id)> true)"));
        assert!(prompt.contains("always([+EXTEND] true -> always([-TERMINATE] true))"));
    }

    #[test]
    fn test_prompt_includes_assignment_pattern() {
        let prompt = generate_prompt("Assignment requires assigner signature and blocks reassignment");

        assert!(prompt.contains(
            "always([+ASSIGN] true -> <+signed_by(/users/assigner.id)> true)"
        ));
        assert!(prompt.contains("always([+ASSIGN] true -> always([-REASSIGN] true))"));
    }

    #[test]
    fn test_prompt_includes_certification_pattern() {
        let prompt =
            generate_prompt("Certification requires auditor signature and blocks deployment");

        assert!(prompt.contains(
            "always([+CERTIFY] true -> <+signed_by(/users/auditor.id)> true)"
        ));
        assert!(prompt.contains("always([+CERTIFY] true -> always([-DEPLOY] true))"));
    }

    #[test]
    fn test_prompt_includes_publication_pattern() {
        let prompt = generate_prompt("Publication requires editor signature and blocks embargo");

        assert!(prompt.contains(
            "always([+PUBLISH] true -> <+signed_by(/users/editor.id)> true)"
        ));
        assert!(prompt.contains("always([+PUBLISH] true -> always([-EMBARGO] true))"));
    }

    #[test]
    fn test_prompt_includes_registration_pattern() {
        let prompt = generate_prompt("Registration requires registrar signature and blocks deletion");

        assert!(prompt.contains(
            "always([+REGISTER] true -> <+signed_by(/users/registrar.id)> true)"
        ));
        assert!(prompt.contains("always([+REGISTER] true -> always([-DELETE] true))"));
    }

    #[test]
    fn test_prompt_includes_acceptance_pattern() {
        let prompt = generate_prompt("Acceptance requires recipient signature and blocks rejection");

        assert!(prompt.contains(
            "always([+ACCEPT] true -> <+signed_by(/users/recipient.id)> true)"
        ));
        assert!(prompt.contains("always([+ACCEPT] true -> always([-REJECT] true))"));
    }

    #[test]
    fn test_prompt_includes_acknowledgement_pattern() {
        let prompt =
            generate_prompt("Acknowledgement requires recipient signature and blocks dispute");

        assert!(prompt.contains(
            "always([+ACKNOWLEDGE] true -> <+signed_by(/users/recipient.id)> true)"
        ));
        assert!(prompt.contains("always([+ACKNOWLEDGE] true -> always([-DISPUTE] true))"));
    }

    #[test]
    fn test_prompt_includes_delivery_confirmation_pattern() {
        let prompt =
            generate_prompt("Delivery confirmation requires recipient signature and blocks refund");

        assert!(prompt.contains(
            "always([+CONFIRM_DELIVERY] true -> <+signed_by(/users/recipient.id)> true)"
        ));
        assert!(prompt.contains("always([+CONFIRM_DELIVERY] true -> always([-REFUND] true))"));
    }

    #[test]
    fn test_prompt_includes_invoice_approval_pattern() {
        let prompt = generate_prompt("Invoice approval requires payer signature and blocks chargeback");

        assert!(prompt.contains(
            "always([+APPROVE_INVOICE] true -> <+signed_by(/users/payer.id)> true)"
        ));
        assert!(prompt.contains("always([+APPROVE_INVOICE] true -> always([-CHARGEBACK] true))"));
    }

    #[test]
    fn test_prompt_includes_milestone_acceptance_pattern() {
        let prompt =
            generate_prompt("Milestone acceptance requires verifier signature and blocks rework");

        assert!(prompt.contains(
            "always([+ACCEPT_MILESTONE] true -> <+signed_by(/users/verifier.id)> true)"
        ));
        assert!(prompt.contains("always([+ACCEPT_MILESTONE] true -> always([-REWORK] true))"));
    }

    #[test]
    fn test_prompt_includes_inspection_approval_pattern() {
        let prompt =
            generate_prompt("Inspection approval requires inspector signature and blocks defect claim");

        assert!(prompt.contains(
            "always([+APPROVE_INSPECTION] true -> <+signed_by(/users/inspector.id)> true)"
        ));
        assert!(prompt.contains(
            "always([+APPROVE_INSPECTION] true -> always([-DEFECT_CLAIM] true))"
        ));
    }

    #[test]
    fn test_prompt_includes_compliance_attestation_pattern() {
        let prompt = generate_prompt(
            "Compliance attestation requires compliance officer signature and blocks noncompliance finding",
        );

        assert!(prompt.contains(
            "always([+ATTEST_COMPLIANCE] true -> <+signed_by(/users/compliance_officer.id)> true)"
        ));
        assert!(prompt.contains(
            "always([+ATTEST_COMPLIANCE] true -> always([-NONCOMPLIANCE_FINDING] true))"
        ));
    }

    #[test]
    fn test_prompt_includes_safety_approval_pattern() {
        let prompt =
            generate_prompt("Safety approval requires safety reviewer signature and blocks unsafe deployment");

        assert!(prompt.contains(
            "always([+APPROVE_SAFETY] true -> <+signed_by(/users/safety_reviewer.id)> true)"
        ));
        assert!(prompt.contains(
            "always([+APPROVE_SAFETY] true -> always([-UNSAFE_DEPLOYMENT] true))"
        ));
    }

    #[test]
    fn test_prompt_includes_risk_acceptance_pattern() {
        let prompt =
            generate_prompt("Risk acceptance requires risk owner signature and blocks unmitigated exposure");

        assert!(prompt.contains(
            "always([+ACCEPT_RISK] true -> <+signed_by(/users/risk_owner.id)> true)"
        ));
        assert!(prompt.contains(
            "always([+ACCEPT_RISK] true -> always([-UNMITIGATED_EXPOSURE] true))"
        ));
    }

    #[test]
    fn test_prompt_includes_incident_closure_pattern() {
        let prompt = generate_prompt(
            "Incident closure requires incident commander signature and blocks incident reopen",
        );

        assert!(prompt.contains(
            "always([+CLOSE_INCIDENT] true -> <+signed_by(/users/incident_commander.id)> true)"
        ));
        assert!(prompt.contains(
            "always([+CLOSE_INCIDENT] true -> always([-REOPEN_INCIDENT] true))"
        ));
    }

    #[test]
    fn test_prompt_includes_change_freeze_pattern() {
        let prompt =
            generate_prompt("Change freeze requires release manager signature and blocks deployment");

        assert!(prompt.contains(
            "always([+FREEZE_CHANGE] true -> <+signed_by(/users/release_manager.id)> true)"
        ));
        assert!(prompt.contains("always([+FREEZE_CHANGE] true -> always([-DEPLOY] true))"));
    }
}
