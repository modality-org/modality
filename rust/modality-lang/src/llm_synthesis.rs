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
    let mut xml_formula_block: Option<(String, Vec<String>)> = None;

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

        if let Some((tag, mut block_lines)) = xml_formula_block.take() {
            let lower = line.to_ascii_lowercase();
            let close_tag = format!("</{tag}>");
            if let Some(close_start) = lower.find(&close_tag) {
                let closing_line = line[..close_start].trim();
                if !closing_line.is_empty() {
                    block_lines.push(closing_line.to_string());
                }
                let joined_block = block_lines.join("\n");
                let formula = strip_formula_wrapping(joined_block.trim());
                let formula = extract_labeled_formula(formula).unwrap_or(formula);
                let formula = normalize_formula_candidate(formula);
                if is_raw_formula_line(&formula) {
                    push_formula_candidate(&mut formulas, &mut declaration_lines, &formula);
                }
            } else {
                block_lines.push(line.to_string());
                xml_formula_block = Some((tag, block_lines));
            }
            continue 'lines;
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

        if collect_json_event_line_formulas(line, &mut formulas) {
            continue 'lines;
        }

        if collect_json_field_line_formulas(line, &mut formulas) {
            continue 'lines;
        }

        if let Some(formula) = extract_plain_text_field_formula(line) {
            push_formula_candidate(&mut formulas, &mut declaration_lines, &formula);
            continue 'lines;
        }

        if let Some(formula) = extract_json_field_formula(line) {
            push_formula_candidate(&mut formulas, &mut declaration_lines, &formula);
            continue 'lines;
        }

        if let Some(formula) = extract_xml_tagged_formula(line) {
            push_formula_candidate(&mut formulas, &mut declaration_lines, &formula);
            continue 'lines;
        }

        if let Some((tag, content)) = extract_xml_formula_block_open(line) {
            let mut block_lines = Vec::new();
            if !content.is_empty() {
                block_lines.push(content.to_string());
            }
            xml_formula_block = Some((tag.to_string(), block_lines));
            continue 'lines;
        }

        let line = strip_formula_wrapping(line);

        if let Some(formula) = extract_markdown_table_formula(line) {
            push_formula_candidate(&mut formulas, &mut declaration_lines, formula);
            continue 'lines;
        }

        if let Some(formula) = extract_labeled_formula(line) {
            push_formula_candidate(&mut formulas, &mut declaration_lines, formula);
            continue 'lines;
        }

        // Also accept raw formula lines directly when no F1: prefix is present.
        if is_raw_formula_line(line) {
            push_formula_candidate(&mut formulas, &mut declaration_lines, line);
        } else {
            let formula = normalize_formula_candidate(line);
            if formula != line && is_raw_formula_line(&formula) {
                push_formula_candidate(&mut formulas, &mut declaration_lines, &formula);
            }
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
    match &value {
        serde_json::Value::String(value) => {
            collect_text_or_encoded_json_formulas(value, &mut formulas);
        }
        serde_json::Value::Array(items) if items.iter().all(serde_json::Value::is_string) => {
            collect_json_formulas(&value, &mut formulas, false, true);
        }
        _ => collect_json_formulas(&value, &mut formulas, false, false),
    }

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
                collect_text_or_encoded_json_formulas(formula, formulas);
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
                let key = normalize_llm_field_key(key);
                if matches!(
                    key.as_str(),
                    "formula"
                        | "formulas"
                        | "amendedformula"
                        | "amendmentformula"
                        | "bestformula"
                        | "acceptedformula"
                        | "candidateformula"
                        | "changeformula"
                        | "chosenformula"
                        | "correctionformula"
                        | "correctedformula"
                        | "draftformula"
                        | "editedformula"
                        | "errorformula"
                        | "failureformula"
                        | "fixformula"
                        | "fixedformula"
                        | "formulaamended"
                        | "formulaamendment"
                        | "formulacandidate"
                        | "formulachange"
                        | "formulacorrection"
                        | "formuladraft"
                        | "formulafix"
                        | "formulapatch"
                        | "formulaproposal"
                        | "formularevision"
                        | "formulaupdate"
                        | "formula_text"
                        | "formulatext"
                        | "finalformula"
                        | "generatedformula"
                        | "improvedformula"
                        | "outputformula"
                        | "patchformula"
                        | "patchedformula"
                        | "parseerrorformula"
                        | "proposalformula"
                        | "proposedformula"
                        | "recommendedformula"
                        | "refinedformula"
                        | "remediationformula"
                        | "replacementformula"
                        | "resolvedformula"
                        | "responseformula"
                        | "ruleamended"
                        | "ruleamendment"
                        | "rulecandidate"
                        | "rulechange"
                        | "rulecorrection"
                        | "ruledraft"
                        | "rulefix"
                        | "rulepatch"
                        | "ruleproposal"
                        | "rulerevision"
                        | "ruleupdate"
                        | "revisedformula"
                        | "revisionformula"
                        | "selectedformula"
                        | "solutionformula"
                        | "suggestedformula"
                        | "formulasuggested"
                        | "formulasuggestion"
                        | "rulesuggested"
                        | "rulesuggestion"
                        | "suggestionformula"
                        | "updateformula"
                        | "updatedformula"
                        | "validformula"
                        | "validationerrorformula"
                        | "validatedformula"
                        | "verifierformula"
                        | "verifiedformula"
                        | "expression"
                        | "expressions"
                        | "rule"
                        | "rules"
                        | "rule_text"
                        | "ruletext"
                ) {
                    collect_json_formulas(value, formulas, true, false);
                } else if matches!(
                    key.as_str(),
                    "content"
                        | "content_text"
                        | "contenttext"
                        | "text"
                        | "value"
                        | "blocks"
                        | "choices"
                        | "candidates"
                        | "alternatives"
                        | "chunks"
                        | "candidate"
                        | "data"
                        | "delta"
                        | "deltas"
                        | "items"
                        | "parts"
                        | "segments"
                        | "variants"
                        | "output"
                        | "outputs"
                        | "output_text"
                        | "outputtext"
                        | "completion"
                        | "completions"
                        | "completion_text"
                        | "completiontext"
                        | "response"
                        | "responses"
                        | "response_text"
                        | "responsetext"
                        | "answer"
                        | "answers"
                        | "answer_text"
                        | "answertext"
                        | "analysis"
                        | "analysistext"
                        | "amended"
                        | "amendedtext"
                        | "amendment"
                        | "amendmenttext"
                        | "amendments"
                        | "assistant_message"
                        | "assistant_output"
                        | "assistant_response"
                        | "assistantmessage"
                        | "assistantoutput"
                        | "assistantresponse"
                        | "accepted"
                        | "assessment"
                        | "assessmenttext"
                        | "body"
                        | "best"
                        | "change"
                        | "changed"
                        | "changedtext"
                        | "changes"
                        | "chosen"
                        | "critique"
                        | "critiquetext"
                        | "final"
                        | "final_answer"
                        | "final_message"
                        | "final_response"
                        | "finalanswer"
                        | "finalmessage"
                        | "finalresponse"
                        | "generation"
                        | "generations"
                        | "payload"
                        | "prediction"
                        | "predictions"
                        | "parsed"
                        | "result"
                        | "results"
                        | "structured"
                        | "structured_output"
                        | "structuredoutput"
                        | "message"
                        | "messages"
                        | "model_output"
                        | "model_response"
                        | "modeloutput"
                        | "modelresponse"
                        | "llm_output"
                        | "llm_response"
                        | "llmoutput"
                        | "llmresponse"
                        | "provider_output"
                        | "provider_response"
                        | "provideroutput"
                        | "providerresponse"
                        | "raw_output"
                        | "raw_response"
                        | "rawoutput"
                        | "rawresponse"
                        | "stdout"
                        | "stderr"
                        | "log"
                        | "logs"
                        | "logtext"
                        | "trace"
                        | "traces"
                        | "tracetext"
                        | "reply"
                        | "selected"
                        | "validated"
                        | "verified"
                        | "generated_text"
                        | "generatedtext"
                        | "correction"
                        | "corrections"
                        | "corrected"
                        | "correctedtext"
                        | "diagnostic"
                        | "diagnostics"
                        | "diagnostictext"
                        | "diagnosis"
                        | "diagnosistext"
                        | "detail"
                        | "details"
                        | "detailtext"
                        | "draft"
                        | "drafts"
                        | "drafttext"
                        | "edit"
                        | "edited"
                        | "editedtext"
                        | "edits"
                        | "error"
                        | "errormessage"
                        | "errors"
                        | "errortext"
                        | "explanation"
                        | "explanationtext"
                        | "failure"
                        | "failurereason"
                        | "failures"
                        | "failuretext"
                        | "fixed"
                        | "fixedtext"
                        | "feedback"
                        | "feedbacktext"
                        | "hint"
                        | "hints"
                        | "hinttext"
                        | "fix"
                        | "fixes"
                        | "improved"
                        | "improvedtext"
                        | "patch"
                        | "patched"
                        | "patchedtext"
                        | "patches"
                        | "parseerror"
                        | "parseerrortext"
                        | "proposed"
                        | "proposal"
                        | "proposaltext"
                        | "proposals"
                        | "recommended"
                        | "recommendedtext"
                        | "recommendation"
                        | "recommendationtext"
                        | "recommendations"
                        | "rationale"
                        | "rationaletext"
                        | "reason"
                        | "reasontext"
                        | "reasons"
                        | "reasoning"
                        | "reasoningtext"
                        | "refined"
                        | "refinedtext"
                        | "remediated"
                        | "remediatedtext"
                        | "remediation"
                        | "remediationtext"
                        | "remediations"
                        | "replacement"
                        | "replacementtext"
                        | "replacements"
                        | "repair"
                        | "repairtext"
                        | "repairs"
                        | "resolved"
                        | "resolvedtext"
                        | "review"
                        | "reviewtext"
                        | "revised"
                        | "revisedtext"
                        | "revision"
                        | "revisiontext"
                        | "revisions"
                        | "suggestion"
                        | "suggested"
                        | "suggestedtext"
                        | "suggestiontext"
                        | "suggestions"
                        | "solution"
                        | "solutiontext"
                        | "solutions"
                        | "update"
                        | "updated"
                        | "updatedtext"
                        | "updates"
                        | "validationerror"
                        | "validationerrortext"
                        | "verifiererror"
                        | "verifiererrortext"
                        | "verifieroutput"
                        | "verifierresponse"
                ) {
                    collect_json_text_formulas(value, formulas);
                } else if matches!(
                    key.as_str(),
                    "arguments" | "args" | "input" | "parameters" | "params"
                ) {
                    collect_json_encoded_formulas(value, formulas);
                } else if (formula_context || array_context)
                    && matches!(key.as_str(), "value" | "expression")
                {
                    collect_json_formulas(value, formulas, true, false);
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
        serde_json::Value::String(value) => collect_text_or_encoded_json_formulas(value, formulas),
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

fn collect_text_or_encoded_json_formulas(value: &str, formulas: &mut Vec<String>) {
    if let Ok(value) = serde_json::from_str(value) {
        let len = formulas.len();
        collect_json_formulas(&value, formulas, true, false);
        if formulas.len() != len {
            return;
        }
    }

    formulas.extend(parse_text_llm_response(value));
}

fn collect_json_encoded_formulas(value: &serde_json::Value, formulas: &mut Vec<String>) {
    match value {
        serde_json::Value::String(value) => {
            if let Ok(value) = serde_json::from_str(value) {
                collect_json_formulas(&value, formulas, false, false);
            } else {
                collect_text_or_encoded_json_formulas(value, formulas);
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

fn normalize_llm_field_key(key: &str) -> String {
    key.chars()
        .filter(|ch| !matches!(*ch, '_' | '-') && !ch.is_whitespace())
        .flat_map(char::to_lowercase)
        .collect()
}

fn collect_json_event_line_formulas(line: &str, formulas: &mut Vec<String>) -> bool {
    let Some(payload) = line.strip_prefix("data:") else {
        return false;
    };
    let payload = payload.trim();
    if payload.is_empty() || payload == "[DONE]" {
        return true;
    }

    if let Ok(value) = serde_json::from_str(payload) {
        collect_json_formulas(&value, formulas, false, false);
    } else {
        formulas.extend(parse_text_llm_response(payload));
    }

    true
}

fn collect_json_field_line_formulas(line: &str, formulas: &mut Vec<String>) -> bool {
    let line = strip_trailing_json_comma(line);
    if !line.contains(':') {
        return false;
    }

    let Ok(value) = serde_json::from_str(&format!("{{{line}}}")) else {
        return false;
    };
    let len = formulas.len();
    collect_json_formulas(&value, formulas, false, false);

    formulas.len() != len
}

fn push_formula_candidate(
    formulas: &mut Vec<String>,
    declaration_lines: &mut Vec<String>,
    formula: &str,
) {
    let formula = normalize_formula_candidate(formula);
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
    if matches!(lower_prefix.as_str(), "formula" | "rule" | "expression") {
        return true;
    }

    let label = ["formula", "rule", "expression"]
        .into_iter()
        .find_map(|prefix| lower_prefix.strip_prefix(prefix))
        .map(str::trim_start)
        .map(|label| label.trim_start_matches('#'));

    label.is_some_and(|label| {
        !label.is_empty() && label.chars().all(|c| c.is_ascii_digit())
    })
}

fn extract_labeled_formula(line: &str) -> Option<&str> {
    if let Some((prefix, formula)) = line.split_once(" - ") {
        if is_formula_prefix(prefix) {
            return Some(strip_labeled_formula_wrapping(formula.trim()));
        }
    }

    // Look for F1:, F2., Formula 3), Rule 4 =, etc. labels.
    for separator in [':', '.', ')', '='] {
        if let Some(separator_pos) = line.find(separator) {
            let prefix = &line[..separator_pos];
            if is_formula_prefix(prefix) {
                return Some(strip_labeled_formula_wrapping(
                    line[separator_pos + 1..].trim(),
                ));
            }
        }
    }

    None
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
    let line = strip_cdata_wrapping(line).unwrap_or(line);

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

fn normalize_formula_candidate(line: &str) -> String {
    let mut formula = strip_formula_wrapping(line).to_string();
    for _ in 0..3 {
        let Some(decoded) = decode_xml_formula_entities(&formula) else {
            break;
        };
        formula = decoded;
    }

    formula.trim().to_string()
}

fn decode_xml_formula_entities(line: &str) -> Option<String> {
    if !line.contains('&') {
        return None;
    }

    let mut decoded = String::with_capacity(line.len());
    let mut rest = line;
    let mut changed = false;

    while let Some(entity_start) = rest.find('&') {
        decoded.push_str(&rest[..entity_start]);
        let entity_body = &rest[entity_start + 1..];
        let Some(entity_end) = entity_body.find(';') else {
            decoded.push_str(&rest[entity_start..]);
            rest = "";
            break;
        };

        let entity = &entity_body[..entity_end];
        if let Some(ch) = decode_xml_formula_entity(entity) {
            decoded.push(ch);
            changed = true;
            rest = &entity_body[entity_end + 1..];
        } else {
            decoded.push('&');
            rest = entity_body;
        }
    }

    decoded.push_str(rest);

    changed.then_some(decoded)
}

fn decode_xml_formula_entity(entity: &str) -> Option<char> {
    match entity {
        "lt" => Some('<'),
        "gt" => Some('>'),
        "amp" => Some('&'),
        "quot" => Some('"'),
        "apos" => Some('\''),
        _ => decode_numeric_xml_formula_entity(entity),
    }
}

fn decode_numeric_xml_formula_entity(entity: &str) -> Option<char> {
    let value = entity.strip_prefix("#x").or_else(|| entity.strip_prefix("#X"));
    let value = if let Some(value) = value {
        u32::from_str_radix(value, 16).ok()?
    } else {
        let value = entity.strip_prefix('#')?;
        value.parse().ok()?
    };

    char::from_u32(value)
}

fn strip_cdata_wrapping(line: &str) -> Option<&str> {
    line.strip_prefix("<![CDATA[")
        .and_then(|line| line.strip_suffix("]]>"))
        .map(str::trim)
}

fn extract_json_field_formula(line: &str) -> Option<String> {
    let (key, value) = line.split_once(':')?;
    let key = normalize_llm_field_key(key.trim().trim_matches('"').trim_matches('\''));
    if !matches!(
        key.as_str(),
        "formula"
            | "formulas"
            | "amendedformula"
            | "amendmentformula"
            | "bestformula"
            | "acceptedformula"
            | "candidateformula"
            | "changeformula"
            | "chosenformula"
            | "correctionformula"
            | "correctedformula"
            | "draftformula"
            | "editedformula"
            | "errorformula"
            | "failureformula"
            | "fixformula"
            | "fixedformula"
            | "formulaamended"
            | "formulaamendment"
            | "formulacandidate"
            | "formulachange"
            | "formulacorrection"
            | "formuladraft"
            | "formulafix"
            | "formulapatch"
            | "formulaproposal"
            | "formularevision"
            | "formulaupdate"
            | "formulatext"
            | "finalformula"
            | "generatedformula"
            | "improvedformula"
            | "outputformula"
            | "patchformula"
            | "patchedformula"
            | "parseerrorformula"
            | "proposalformula"
            | "proposedformula"
            | "recommendedformula"
            | "refinedformula"
            | "remediationformula"
            | "replacementformula"
            | "resolvedformula"
            | "responseformula"
            | "ruleamended"
            | "ruleamendment"
            | "rulecandidate"
            | "rulechange"
            | "rulecorrection"
            | "ruledraft"
            | "rulefix"
            | "rulepatch"
            | "ruleproposal"
            | "rulerevision"
            | "ruleupdate"
            | "revisedformula"
            | "revisionformula"
            | "selectedformula"
            | "solutionformula"
            | "suggestedformula"
            | "formulasuggested"
            | "formulasuggestion"
            | "rulesuggested"
            | "rulesuggestion"
            | "suggestionformula"
            | "updateformula"
            | "updatedformula"
            | "validformula"
            | "validationerrorformula"
            | "validatedformula"
            | "verifierformula"
            | "verifiedformula"
            | "expression"
            | "expressions"
            | "rule"
            | "rules"
            | "ruletext"
    ) {
        return None;
    }

    let formula = normalize_formula_candidate(value.trim());
    if let Some(labeled_formula) = extract_labeled_formula(&formula) {
        let labeled_formula = normalize_formula_candidate(labeled_formula);
        if is_raw_formula_line(&labeled_formula) {
            return Some(labeled_formula);
        }
    }

    is_raw_formula_line(&formula).then_some(formula)
}

fn extract_plain_text_field_formula(line: &str) -> Option<String> {
    let (key, value) = split_plain_text_field(line)?;
    let key = normalize_llm_field_key(key.trim().trim_matches('"').trim_matches('\''));
    if !matches!(
        key.as_str(),
        "amendedformula"
            | "amendmentformula"
            | "bestformula"
            | "acceptedformula"
            | "candidateformula"
            | "changeformula"
            | "chosenformula"
            | "correctionformula"
            | "correctedformula"
            | "draftformula"
            | "editedformula"
            | "errorformula"
            | "failureformula"
            | "fixformula"
            | "fixedformula"
            | "formulaamended"
            | "formulaamendment"
            | "formulachange"
            | "formulacorrection"
            | "formuladraft"
            | "formulafix"
            | "formulapatch"
            | "formulaproposal"
            | "formularevision"
            | "formulaupdate"
            | "finalformula"
            | "generatedformula"
            | "improvedformula"
            | "outputformula"
            | "patchformula"
            | "patchedformula"
            | "parseerrorformula"
            | "proposalformula"
            | "proposedformula"
            | "recommendedformula"
            | "refinedformula"
            | "remediationformula"
            | "replacementformula"
            | "resolvedformula"
            | "responseformula"
            | "revisedformula"
            | "revisionformula"
            | "ruleamended"
            | "ruleamendment"
            | "rulechange"
            | "rulecorrection"
            | "ruledraft"
            | "rulefix"
            | "rulepatch"
            | "ruleproposal"
            | "rulerevision"
            | "ruleupdate"
            | "selectedformula"
            | "solutionformula"
            | "suggestedformula"
            | "formulasuggested"
            | "formulasuggestion"
            | "rulesuggested"
            | "rulesuggestion"
            | "suggestionformula"
            | "updateformula"
            | "updatedformula"
            | "validformula"
            | "validationerrorformula"
            | "validatedformula"
            | "verifierformula"
            | "verifiedformula"
            | "content"
            | "contenttext"
            | "text"
            | "output"
            | "outputs"
            | "outputtext"
            | "completion"
            | "completions"
            | "completiontext"
            | "response"
            | "responses"
            | "responsetext"
            | "answer"
            | "answers"
            | "answertext"
            | "analysis"
            | "analysistext"
            | "amended"
            | "amendedtext"
            | "amendment"
            | "amendmenttext"
            | "amendments"
            | "assistantmessage"
            | "assistantoutput"
            | "assistantresponse"
            | "accepted"
            | "assessment"
            | "assessmenttext"
            | "alternative"
            | "alternatives"
            | "result"
            | "block"
            | "blocks"
            | "body"
            | "best"
            | "candidate"
            | "candidates"
            | "change"
            | "changed"
            | "changedtext"
            | "changes"
            | "chosen"
            | "choice"
            | "choices"
            | "critique"
            | "critiquetext"
            | "chunk"
            | "chunks"
            | "delta"
            | "deltas"
            | "final"
            | "finalanswer"
            | "finalmessage"
            | "finalresponse"
            | "generated"
            | "generation"
            | "generations"
            | "item"
            | "items"
            | "part"
            | "parts"
            | "payload"
            | "prediction"
            | "predictions"
            | "message"
            | "modelresponse"
            | "modeloutput"
            | "llmoutput"
            | "llmresponse"
            | "provideroutput"
            | "providerresponse"
            | "rawoutput"
            | "rawresponse"
            | "stdout"
            | "stderr"
            | "log"
            | "logs"
            | "logtext"
            | "trace"
            | "traces"
            | "tracetext"
            | "reply"
            | "selected"
            | "segment"
            | "segments"
            | "validated"
            | "verified"
            | "variant"
            | "variants"
            | "generatedtext"
            | "correction"
            | "corrections"
            | "corrected"
            | "correctedtext"
            | "diagnostic"
            | "diagnostics"
            | "diagnostictext"
            | "diagnosis"
            | "diagnosistext"
            | "detail"
            | "details"
            | "detailtext"
            | "draft"
            | "drafts"
            | "drafttext"
            | "edit"
            | "edited"
            | "editedtext"
            | "edits"
            | "error"
            | "errormessage"
            | "errors"
            | "errortext"
            | "explanation"
            | "explanationtext"
            | "failure"
            | "failurereason"
            | "failures"
            | "failuretext"
            | "feedback"
            | "feedbacktext"
            | "fixed"
            | "fixedtext"
            | "hint"
            | "hints"
            | "hinttext"
            | "fix"
            | "fixes"
            | "improved"
            | "improvedtext"
            | "patch"
            | "patched"
            | "patchedtext"
            | "patches"
            | "parseerror"
            | "parseerrortext"
            | "proposed"
            | "proposal"
            | "proposaltext"
            | "proposals"
            | "recommended"
            | "recommendedtext"
            | "recommendation"
            | "recommendationtext"
            | "recommendations"
            | "rationale"
            | "rationaletext"
            | "reason"
            | "reasontext"
            | "reasons"
            | "reasoning"
            | "reasoningtext"
            | "refined"
            | "refinedtext"
            | "remediated"
            | "remediatedtext"
            | "remediation"
            | "remediationtext"
            | "remediations"
            | "replacement"
            | "replacementtext"
            | "replacements"
            | "repair"
            | "repairtext"
            | "repairs"
            | "resolved"
            | "resolvedtext"
            | "review"
            | "reviewtext"
            | "revised"
            | "revisedtext"
            | "revision"
            | "revisiontext"
            | "revisions"
            | "suggestion"
            | "suggested"
            | "suggestedtext"
            | "suggestiontext"
            | "suggestions"
            | "solution"
            | "solutiontext"
            | "solutions"
            | "update"
            | "updated"
            | "updatedtext"
            | "updates"
            | "validationerror"
            | "validationerrortext"
            | "verifiererror"
            | "verifiererrortext"
            | "verifieroutput"
            | "verifierresponse"
    ) {
        return None;
    }

    let formula = normalize_formula_candidate(value.trim());
    if let Some(labeled_formula) = extract_labeled_formula(&formula) {
        let labeled_formula = normalize_formula_candidate(labeled_formula);
        if is_raw_formula_line(&labeled_formula) {
            return Some(labeled_formula);
        }
    }

    is_raw_formula_line(&formula).then_some(formula)
}

fn split_plain_text_field(line: &str) -> Option<(&str, &str)> {
    match (line.find(':'), line.find('=')) {
        (Some(colon), Some(equals)) if equals < colon => {
            Some((&line[..equals], &line[equals + 1..]))
        }
        _ => line.split_once(':').or_else(|| line.split_once('=')),
    }
}

fn extract_xml_tagged_formula(line: &str) -> Option<String> {
    let line = line.trim();
    let lower = line.to_ascii_lowercase();

    for tag in [
        "formula",
        "formula_text",
        "formula-text",
        "formulatext",
        "rule",
        "rule_text",
        "rule-text",
        "ruletext",
        "expression",
    ] {
        if !lower.starts_with(&format!("<{tag}")) {
            continue;
        }

        let tag_end = "<".len() + tag.len();
        let Some(tag_boundary) = lower[tag_end..].chars().next() else {
            continue;
        };
        if tag_boundary != '>' && !tag_boundary.is_ascii_whitespace() {
            continue;
        }

        let Some(open_end) = lower.find('>') else {
            continue;
        };
        let close_tag = format!("</{tag}>");
        let Some(close_start) = lower.rfind(&close_tag) else {
            continue;
        };
        if close_start <= open_end {
            continue;
        }

        let formula = line[open_end + 1..close_start].trim();
        let formula = extract_labeled_formula(formula).unwrap_or(formula);
        let formula = normalize_formula_candidate(formula);
        if is_raw_formula_line(&formula) {
            return Some(formula);
        }
    }

    None
}

fn extract_xml_formula_block_open(line: &str) -> Option<(&str, &str)> {
    let line = line.trim();
    let lower = line.to_ascii_lowercase();

    for tag in [
        "formula",
        "formula_text",
        "formula-text",
        "formulatext",
        "rule",
        "rule_text",
        "rule-text",
        "ruletext",
        "expression",
    ] {
        if !lower.starts_with(&format!("<{tag}")) {
            continue;
        }

        let tag_end = "<".len() + tag.len();
        let Some(tag_boundary) = lower[tag_end..].chars().next() else {
            continue;
        };
        if tag_boundary != '>' && !tag_boundary.is_ascii_whitespace() {
            continue;
        }

        let Some(open_end) = lower.find('>') else {
            continue;
        };
        if lower[open_end + 1..].contains(&format!("</{tag}>")) {
            continue;
        }

        return Some((tag, line[open_end + 1..].trim()));
    }

    None
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
        || line.starts_with("<+")
        || line.starts_with("<-")
        || line.starts_with("<>")
        || line.starts_with("eventually")
        || (line.starts_with("formula ") && line.contains('{'))
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
    fn test_parse_llm_response_accepts_rule_and_expression_prefixes() {
        let response = r#"
Rule 1: always([+PAY] true -> eventually(<+WORK> true))
Expression 2: <+CANCEL> true
Rule: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+PAY] true -> eventually(<+WORK> true))",
                "<+CANCEL> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
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
    fn test_parse_llm_response_accepts_json_formula_text_fields() {
        let response = r#"
[
  {
    "label": "F1",
    "formula_text": "always([+PAY] true -> eventually(<+WORK> true))"
  },
  {
    "label": "F2",
    "formulaText": "<+CANCEL> true"
  },
  {
    "label": "F3",
    "rule_text": "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
  },
  {
    "label": "F4",
    "ruleText": "<+ESCALATE> true"
  }
]
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+PAY] true -> eventually(<+WORK> true))",
                "<+CANCEL> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
                "<+ESCALATE> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_formula_field_aliases() {
        let response = r#"
formula_text: always([+PAY] true -> eventually(<+WORK> true))
rule_text: <+CANCEL> true
expression: `always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)`
message: This is explanatory text, not a formula.
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+PAY] true -> eventually(<+WORK> true))",
                "<+CANCEL> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_plural_formula_field_aliases() {
        let response = r#"
formulas: always([+PAY] true -> eventually(<+WORK> true))
rules: <+CANCEL> true
expressions: `always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)`
content: This is explanatory text, not a formula.
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+PAY] true -> eventually(<+WORK> true))",
                "<+CANCEL> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_dash_separated_field_aliases() {
        let response = r#"
[
  {"formula-text": "always([+PAY] true -> eventually(<+WORK> true))"},
  {"rule-text": "<+CANCEL> true"},
  {"output-text": "Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"},
  {"answer-text": "Formula 4: <+ESCALATE> true"}
]
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+PAY] true -> eventually(<+WORK> true))",
                "<+CANCEL> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
                "<+ESCALATE> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_dash_separated_formula_fields() {
        let response = r#"
formula-text: always([+PAY] true -> eventually(<+WORK> true))
rule-text: <+CANCEL> true
expression: `always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)`
message: This is explanatory text, not a formula.
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+PAY] true -> eventually(<+WORK> true))",
                "<+CANCEL> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_provider_text_fields() {
        let response = r#"
content: F1: always([+PAY] true -> eventually(<+WORK> true))
output-text: Formula 2: <+CANCEL> true
message: This is explanatory text, not a formula.
finalAnswer: `always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)`
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+PAY] true -> eventually(<+WORK> true))",
                "<+CANCEL> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_expression_fields() {
        let response = r#"
{
  "expression": "always([+PAY] true -> eventually(<+WORK> true))",
  "expressions": [
    {"value": "<+CANCEL> true"},
    {"expression": "[<+REFUND>] true"}
  ]
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+PAY] true -> eventually(<+WORK> true))",
                "<+CANCEL> true",
                "[<+REFUND>] true"
            ]
        );
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
    fn test_parse_llm_response_accepts_top_level_json_formula_array() {
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
    fn test_parse_llm_response_accepts_top_level_encoded_json_formula_array() {
        let response =
            r#""[\"always([+PAY] true -> eventually(<+WORK> true))\",\"<+CANCEL> true\"]""#;

        let formulas = parse_llm_response(response);
        assert_eq!(formulas.len(), 2);
        assert_eq!(
            formulas[0],
            "always([+PAY] true -> eventually(<+WORK> true))"
        );
        assert_eq!(formulas[1], "<+CANCEL> true");
    }

    #[test]
    fn test_parse_llm_response_accepts_labeled_json_formula_values() {
        let response = r#"
{
  "formulas": [
    "F1: always([+PAY] true -> eventually(<+WORK> true))",
    "Formula 2: <+CANCEL> true"
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
    fn test_parse_llm_response_accepts_json_encoded_formulas_value() {
        let response = r#"
{
  "formulas": "[\"always([+PAY] true -> eventually(<+WORK> true))\",\"<+CANCEL> true\"]"
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
    fn test_parse_llm_response_accepts_json_formula_object_values() {
        let response = r#"
{
  "formulas": [
    {
      "name": "payment",
      "value": "always([+PAY] true -> eventually(<+WORK> true))"
    },
    {
      "name": "cancel",
      "expression": "<+CANCEL> true"
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
    fn test_parse_llm_response_accepts_fenced_json_provider_text_fields() {
        let response = r#"
```json
{
  "content": "F1: always([+PAY] true -> eventually(<+WORK> true))",
  "message": "Explanation only.",
  "output_text": "Formula 2: <+CANCEL> true"
}
```
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+PAY] true -> eventually(<+WORK> true))",
                "<+CANCEL> true"
            ]
        );
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
    fn test_parse_llm_response_accepts_json_encoded_message_content() {
        let response = r#"
{
  "choices": [
    {
      "message": {
        "role": "assistant",
        "content": "{\"formulas\":[\"always([+PAY] true -> eventually(<+WORK> true))\",\"<+CANCEL> true\"]}"
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
        },
        {
          "type": "output_text",
          "value": "F2: <+ESCALATE> true"
        }
      ]
    }
  ]
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
                "<+ESCALATE> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_output_string() {
        let response = r#"
{
  "output": "F1: always([+PAY] true -> eventually(<+WORK> true))\nF2: <+CANCEL> true"
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
    fn test_parse_llm_response_accepts_json_completion_text() {
        let response = r#"
{
  "completion": "F1: always([+PAY] true -> eventually(<+WORK> true))\nF2: <+CANCEL> true"
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
    fn test_parse_llm_response_accepts_json_response_text() {
        let response = r#"
{
  "response": "F1: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec!["always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_answer_and_result_text() {
        let response = r#"
{
  "answer": "F1: always([+PAY] true -> eventually(<+WORK> true))",
  "result": "F2: <+CANCEL> true"
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+PAY] true -> eventually(<+WORK> true))",
                "<+CANCEL> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_body_and_payload_text() {
        let response = r#"
{
  "body": "F1: always([+PAY] true -> eventually(<+WORK> true))",
  "payload": {
    "text": "F2: <+CANCEL> true"
  }
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+PAY] true -> eventually(<+WORK> true))",
                "<+CANCEL> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_final_answer_text() {
        let response = r#"
{
  "final_answer": "F1: always([+PAY] true -> eventually(<+WORK> true))",
  "finalAnswer": "F2: <+CANCEL> true",
  "final": "Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
                "<+CANCEL> true",
                "always([+PAY] true -> eventually(<+WORK> true))"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_final_response_aliases() {
        let response = r#"
{
  "final_response": "F1: always([+SHIP] true -> eventually(<+PAY> true))",
  "finalMessage": "Plain explanation.\nFormula 2: <+REFUND> true",
  "assistant_response": "No valid formula in this explanation.",
  "assistantMessage": "Formula 3: <+ESCALATE> true",
  "modelOutput": "F4: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "<+ESCALATE> true",
                "<+REFUND> true",
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_provider_response_aliases() {
        let response = r#"
{
  "assistant_output": "F1: always([+SHIP] true -> eventually(<+PAY> true))",
  "model_response": "Plain explanation.\nFormula 2: <+REFUND> true",
  "llm_response": "No valid formula in this explanation.",
  "providerOutput": "Formula 3: <+ESCALATE> true",
  "raw_output": "F4: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "<+ESCALATE> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_message_reply_and_generated_text() {
        let response = r#"
{
  "message": "F1: always([+PAY] true -> eventually(<+WORK> true))",
  "reply": "F2: <+CANCEL> true",
  "generated_text": "Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
                "always([+PAY] true -> eventually(<+WORK> true))",
                "<+CANCEL> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_camel_case_text_fields() {
        let response = r#"
{
  "contentText": "F4: always([+CONTENT] true -> eventually(<+VERIFY> true))",
  "generatedText": "F1: always([+PAY] true -> eventually(<+WORK> true))",
  "outputText": "F2: <+CANCEL> true",
  "responseText": "Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+CONTENT] true -> eventually(<+VERIFY> true))",
                "always([+PAY] true -> eventually(<+WORK> true))",
                "<+CANCEL> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_provider_text_arrays() {
        let response = r#"
{
  "alternatives": [
    "Formula 16: always([+ALTERNATE] true -> eventually(<+SELECT> true))"
  ],
  "choices": [
    "F1: always([+PAY] true -> eventually(<+WORK> true))"
  ],
  "answers": [
    "Formula 13: always([+ANSWER] true -> eventually(<+CHECK> true))"
  ],
  "candidates": [
    "F2: <+CANCEL> true"
  ],
  "completions": [
    "Formula 14: <+COMPLETE> true"
  ],
  "blocks": [
    "Formula 8: always([+DEPLOY] true -> eventually(<+ROLLBACK> true))"
  ],
  "chunks": [
    "Formula 9: <+RETRY> true"
  ],
  "data": [
    "The generated rule is ready.",
    "Formula 4: <+ESCALATE> true"
  ],
  "generations": [
    "Formula 10: always([+SHIP] true -> eventually(<+CONFIRM> true))"
  ],
  "items": [
    "Formula 5: always([+REVIEW] true -> eventually(<+APPROVE> true))"
  ],
  "messages": [
    { "content": "Formula 11: <+NOTIFY> true" }
  ],
  "parts": [
    "Formula 6: <+ARCHIVE> true"
  ],
  "results": [
    "Explanation only.",
    "Formula 12: always([+EXPORT] true -> <+signed_by(/users/exporter.id)> true)"
  ],
  "responses": [
    "Formula 15: always([+RESPOND] true -> <+signed_by(/users/responder.id)> true)"
  ],
  "segments": [
    "Formula 7: always([+AUDIT] true -> eventually(<+REPORT> true))"
  ],
  "outputs": [
    "Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
  ],
  "variants": [
    "Formula 17: <+VARIANT> true"
  ]
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+ALTERNATE] true -> eventually(<+SELECT> true))",
                "always([+ANSWER] true -> eventually(<+CHECK> true))",
                "always([+DEPLOY] true -> eventually(<+ROLLBACK> true))",
                "<+CANCEL> true",
                "always([+PAY] true -> eventually(<+WORK> true))",
                "<+RETRY> true",
                "<+COMPLETE> true",
                "<+ESCALATE> true",
                "always([+SHIP] true -> eventually(<+CONFIRM> true))",
                "always([+REVIEW] true -> eventually(<+APPROVE> true))",
                "<+NOTIFY> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
                "<+ARCHIVE> true",
                "always([+RESPOND] true -> <+signed_by(/users/responder.id)> true)",
                "always([+EXPORT] true -> <+signed_by(/users/exporter.id)> true)",
                "always([+AUDIT] true -> eventually(<+REPORT> true))",
                "<+VARIANT> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_provider_delta_text() {
        let response = r#"
{
  "delta": "F1: always([+STREAM] true -> eventually(<+FINAL> true))",
  "deltas": [
    "Partial explanation.",
    "Formula 2: <+COMMIT> true"
  ]
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+STREAM] true -> eventually(<+FINAL> true))",
                "<+COMMIT> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_stream_data_lines() {
        let response = r#"
event: message.delta
data: {"choices":[{"delta":{"content":"F1: always([+STREAM] true -> eventually(<+FINAL> true))"}}]}
data: {"output":[{"content":[{"type":"output_text","text":"Formula 2: <+COMMIT> true"}]}]}
data: {"message":"Explanation only."}
data: [DONE]
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+STREAM] true -> eventually(<+FINAL> true))",
                "<+COMMIT> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_xml_tagged_formulas() {
        let response = r#"
<formulas>
<formula>always([+TAGGED] true -> eventually(<+REVIEW> true))</formula>
<formula_text name="commit"><+COMMIT> true</formula_text>
<rule>`always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)`</rule>
</formulas>
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+TAGGED] true -> eventually(<+REVIEW> true))",
                "<+COMMIT> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_multiline_xml_tagged_formulas() {
        let response = r#"
<formulas>
<formula>
always([+TAGGED] true -> eventually(<+REVIEW> true))
</formula>
<formula_text name="commit">
<+COMMIT> true
</formula_text>
<rule>
`always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)`
</rule>
</formulas>
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+TAGGED] true -> eventually(<+REVIEW> true))",
                "<+COMMIT> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_dash_separated_xml_tags() {
        let response = r#"
<formula-text>always([+PAY] true -> eventually(<+WORK> true))</formula-text>
<rule-text>
<+CANCEL> true
</rule-text>
<expression>always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)</expression>
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+PAY] true -> eventually(<+WORK> true))",
                "<+CANCEL> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_camel_case_xml_tags() {
        let response = r#"
<formulaText>always([+PAY] true -> eventually(<+WORK> true))</formulaText>
<ruleText>
<+CANCEL> true
</ruleText>
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+PAY] true -> eventually(<+WORK> true))",
                "<+CANCEL> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_labeled_xml_tagged_formulas() {
        let response = r#"
<formula>F1: always([+PAY] true -> eventually(<+WORK> true))</formula>
<rule>Formula 2: <+CANCEL> true</rule>
<formula_text>F3 - always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)</formula_text>
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+PAY] true -> eventually(<+WORK> true))",
                "<+CANCEL> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_cdata_wrapped_formulas() {
        let response = r#"
<formula><![CDATA[always([+PAY] true -> eventually(<+WORK> true))]]></formula>
<rule>
<![CDATA[<+CANCEL> true]]>
</rule>
formula_text: <![CDATA[always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)]]>
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+PAY] true -> eventually(<+WORK> true))",
                "<+CANCEL> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_xml_escaped_formulas() {
        let response = r#"
F1: always([+PAY] true -&gt; eventually(&lt;+WORK&gt; true))
<formula>&lt;+CANCEL&gt; true</formula>
formula_text: always([+APPROVE] true -&gt; &lt;+signed_by(/users/reviewer.id)&gt; true)
{
  "ruleText": "&lt;+ESCALATE&gt; true"
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+PAY] true -> eventually(<+WORK> true))",
                "<+CANCEL> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
                "<+ESCALATE> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_numeric_xml_escaped_formulas() {
        let response = r#"
F1: always([+PAY] true &#45;&#62; eventually(&#60;+WORK&#62; true))
<formula>&#x3C;+CANCEL&#x3E; true</formula>
formula_text: &amp;lt;+ESCALATE&amp;gt; true
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+PAY] true -> eventually(<+WORK> true))",
                "<+CANCEL> true",
                "<+ESCALATE> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_multiline_xml_escaped_formulas() {
        let response = r#"
<formula>
F1: always([+PAY] true -&gt; eventually(&lt;+WORK&gt; true))
</formula>
<rule>
Formula 2: &amp;lt;+ESCALATE&amp;gt; true
</rule>
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+PAY] true -> eventually(<+WORK> true))",
                "<+ESCALATE> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_provider_generation_aliases() {
        let response = r#"
{
  "generation": "F1: always([+GENERATE] true -> eventually(<+REVIEW> true))",
  "candidate": "Candidate text\nFormula 2: <+APPROVE> true",
  "predictions": [
    "Explanatory prediction.",
    "F3: always([+PUBLISH] true -> <+signed_by(/users/editor.id)> true)"
  ]
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "<+APPROVE> true",
                "always([+GENERATE] true -> eventually(<+REVIEW> true))",
                "always([+PUBLISH] true -> <+signed_by(/users/editor.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_snake_case_provider_text_fields() {
        let response = r#"
{
  "answer_text": "F1: always([+ANSWER] true -> eventually(<+CHECK> true))",
  "completion_text": "F2: <+COMPLETE> true",
  "response_text": "Plain explanation.\nFormula 3: always([+RESPOND] true -> <+signed_by(/users/responder.id)> true)"
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+ANSWER] true -> eventually(<+CHECK> true))",
                "<+COMPLETE> true",
                "always([+RESPOND] true -> <+signed_by(/users/responder.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_structured_output_aliases() {
        let response = r#"
{
  "parsed": {
    "rules": [
      "F1: always([+PARSE] true -> eventually(<+CHECK> true))"
    ]
  },
  "structured_output": [
    "Explanation only.",
    "Formula 2: <+STRUCTURE> true"
  ],
  "structured": {
    "items": [
      "Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
    ]
  }
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+PARSE] true -> eventually(<+CHECK> true))",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
                "<+STRUCTURE> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_correction_fields() {
        let response = r#"
{
  "diagnostic": "Parse error: expected formula body.",
  "corrected_formula": "always([+PAY] true -> eventually(<+DELIVER> true))",
  "suggestions": [
    "Explanation only.",
    "Formula 2: <+CANCEL> true"
  ],
  "revision": {
    "fixed formula": "F3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
  }
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+PAY] true -> eventually(<+DELIVER> true))",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
                "<+CANCEL> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_repair_proposal_fields() {
        let response = r#"
{
  "proposal_formula": "always([+PAY] true -> eventually(<+DELIVER> true))",
  "recommendations": [
    "Explanation only.",
    "Formula 2: <+CANCEL> true"
  ],
  "remediation": {
    "repair": "F3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
  },
  "fixes": [
    { "recommended formula": "F4: <+ESCALATE> true" }
  ]
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "<+ESCALATE> true",
                "always([+PAY] true -> eventually(<+DELIVER> true))",
                "<+CANCEL> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_repair_status_fields() {
        let response = r#"
{
  "corrected": "always([+SHIP] true -> eventually(<+PAY> true))",
  "recommended": "Formula 2: <+REFUND> true",
  "revised": {
    "text": "F3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
  },
  "remediated": "This response only explains the repair."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_repair_edit_fields() {
        let response = r#"
{
  "updated_formula": "always([+SHIP] true -> eventually(<+PAY> true))",
  "edited_formula": "Formula 2: <+REFUND> true",
  "patched": {
    "text": "F3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
  },
  "replacement": "This response only explains the replacement."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
                "always([+SHIP] true -> eventually(<+PAY> true))"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_repair_text_fields() {
        let response = r#"
{
  "corrected_text": "always([+SHIP] true -> eventually(<+PAY> true))",
  "repair_text": "Formula 2: <+REFUND> true",
  "updated text": {
    "text": "F3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
  },
  "replacement_text": "This response only explains the replacement."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_refinement_fields() {
        let response = r#"
{
  "improved_formula": "always([+SHIP] true -> eventually(<+PAY> true))",
  "refined": "Formula 2: <+REFUND> true",
  "resolved_text": {
    "text": "F3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
  },
  "improved_text": "This response only explains the improvement."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_feedback_fields() {
        let response = r#"
{
  "feedback": "always([+SHIP] true -> eventually(<+PAY> true))",
  "analysis": "This only explains why the first draft failed.",
  "critique_text": "Formula 2: <+REFUND> true",
  "assessment": {
    "review": "F3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
  }
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
                "<+REFUND> true",
                "always([+SHIP] true -> eventually(<+PAY> true))"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_solution_fields() {
        let response = r#"
{
  "solution_formula": "always([+SHIP] true -> eventually(<+PAY> true))",
  "diagnosis_text": "Formula 2: <+REFUND> true",
  "solution": {
    "text": "F3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
  },
  "diagnosis": "This only explains the parse failure."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
                "always([+SHIP] true -> eventually(<+PAY> true))"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_reasoning_fields() {
        let response = r#"
{
  "explanation": "Formula 1: always([+SHIP] true -> eventually(<+PAY> true))",
  "explanation_text": "This only explains why the repair was needed.",
  "rationale": "F2: <+REFUND> true",
  "reasoning": {
    "text": "Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
  }
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_error_feedback_fields() {
        let response = r#"
{
  "error": "Formula 1: always([+SHIP] true -> eventually(<+PAY> true))",
  "error_message": "This only explains why parsing failed.",
  "validation_error": {
    "text": "F2: <+REFUND> true"
  },
  "verifier_output": "Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_log_output_fields() {
        let response = r#"
{
  "stdout": "Formula 1: always([+SHIP] true -> eventually(<+PAY> true))",
  "stderr": "This only contains verifier diagnostics.",
  "logs": [
    "F2: <+REFUND> true",
    "trace text without a formula"
  ],
  "trace": {
    "text": "Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
  }
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "<+REFUND> true",
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_error_detail_fields() {
        let response = r#"
{
  "detail": "Formula 1: always([+SHIP] true -> eventually(<+PAY> true))",
  "details": [
    "F2: <+REFUND> true",
    "details without a formula"
  ],
  "reason": {
    "text": "Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
  },
  "hint": "This only suggests trying a simpler rule."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_candidate_field_order_aliases() {
        let response = r#"
{
  "formula_candidate": "Formula 1: always([+SHIP] true -> eventually(<+PAY> true))",
  "rule_candidate": "F2: <+REFUND> true",
  "formula candidate": "This candidate is only explained in prose."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_proposal_field_order_aliases() {
        let response = r#"
{
  "formula_proposal": "Formula 1: always([+SHIP] true -> eventually(<+PAY> true))",
  "rule_proposal": "F2: <+REFUND> true",
  "formula proposal": "This proposal is only explained in prose."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_draft_field_order_aliases() {
        let response = r#"
{
  "formula_draft": "Formula 1: always([+SHIP] true -> eventually(<+PAY> true))",
  "draft_formula": "F2: <+REFUND> true",
  "rule_draft": "Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
  "draft": "This draft is only explained in prose."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "<+REFUND> true",
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_revision_field_order_aliases() {
        let response = r#"
{
  "formula_revision": "Formula 1: always([+SHIP] true -> eventually(<+PAY> true))",
  "revision_formula": "F2: <+REFUND> true",
  "rule_revision": "Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
  "revision": "This revision is only explained in prose."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_fix_field_order_aliases() {
        let response = r#"
{
  "formula_fix": "Formula 1: always([+SHIP] true -> eventually(<+PAY> true))",
  "fix_formula": "F2: <+REFUND> true",
  "rule_fix": "Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
  "fix": "This fix is only explained in prose."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "<+REFUND> true",
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_amendment_field_order_aliases() {
        let response = r#"
{
  "formula_amendment": "Formula 1: always([+SHIP] true -> eventually(<+PAY> true))",
  "amendment_formula": "F2: <+REFUND> true",
  "rule_amendment": "Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
  "amendment": "This amendment is only explained in prose."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "<+REFUND> true",
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_patch_field_order_aliases() {
        let response = r#"
{
  "formula_patch": "Formula 1: always([+SHIP] true -> eventually(<+PAY> true))",
  "patch_formula": "F2: <+REFUND> true",
  "rule_patch": "Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
  "patch": "This patch is only explained in prose."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_update_field_order_aliases() {
        let response = r#"
{
  "formula_update": "Formula 1: always([+SHIP] true -> eventually(<+PAY> true))",
  "update_formula": "F2: <+REFUND> true",
  "rule_update": "Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
  "update": "This update is only explained in prose."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
                "<+REFUND> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_change_field_order_aliases() {
        let response = r#"
{
  "formula_change": "Formula 1: always([+SHIP] true -> eventually(<+PAY> true))",
  "change_formula": "F2: <+REFUND> true",
  "rule_change": "Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
  "change": "This change is only explained in prose."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "<+REFUND> true",
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_correction_field_order_aliases() {
        let response = r#"
{
  "formula_correction": "Formula 1: always([+SHIP] true -> eventually(<+PAY> true))",
  "correction_formula": "F2: <+REFUND> true",
  "rule_correction": "Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
  "correction": "This correction is only explained in prose."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "<+REFUND> true",
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_suggestion_field_order_aliases() {
        let response = r#"
{
  "formula_suggestion": "Formula 1: always([+SHIP] true -> eventually(<+PAY> true))",
  "suggestion_formula": "F2: <+REFUND> true",
  "rule_suggestion": "Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
  "suggestion": "This suggestion is only explained in prose."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
                "<+REFUND> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_candidate_formula_fields() {
        let response = r#"
{
  "best_formula": "always([+PAY] true -> eventually(<+DELIVER> true))",
  "candidate_formula": "F2: <+CANCEL> true",
  "selected formula": "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
  "validated_formula": "<+ESCALATE> true",
  "chosen_formula": "Formula 5: always([+MERGE] true -> <+signed_by(/users/maintainer.id)> true)",
  "accepted formula": "explanation without a formula",
  "verified_formula": "F6: <+DEPLOY> true"
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+PAY] true -> eventually(<+DELIVER> true))",
                "<+CANCEL> true",
                "always([+MERGE] true -> <+signed_by(/users/maintainer.id)> true)",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
                "<+ESCALATE> true",
                "<+DEPLOY> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_response_formula_fields() {
        let response = r#"
{
  "generated_formula": "always([+PAY] true -> eventually(<+DELIVER> true))",
  "final_formula": "F2: <+CANCEL> true",
  "output formula": "explanation without a formula",
  "response_formula": "Formula 4: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "<+CANCEL> true",
                "always([+PAY] true -> eventually(<+DELIVER> true))",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_status_text_fields() {
        let response = r#"
{
  "best": "always([+PAY] true -> eventually(<+DELIVER> true))",
  "chosen": "F2: <+CANCEL> true",
  "accepted": "explanation without a formula",
  "selected": "Formula 4: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
  "validated": "<+ESCALATE> true",
  "verified": "This candidate is syntactically valid."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+PAY] true -> eventually(<+DELIVER> true))",
                "<+CANCEL> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
                "<+ESCALATE> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_correction_fields() {
        let response = r#"
diagnostic: parser expected a modal expression
corrected formula: always([+SHIP] true -> eventually(<+PAY> true))
suggestion: Formula 2: <+REFUND> true
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_repair_proposal_fields() {
        let response = r#"
recommendation: retry with a committed modal action
proposal formula: always([+SHIP] true -> eventually(<+PAY> true))
repair: Formula 2: <+REFUND> true
remediation: emit a formula label before prose
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_repair_status_fields() {
        let response = r#"
corrected: always([+SHIP] true -> eventually(<+PAY> true))
recommended: Formula 2: <+REFUND> true
revised: F3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)
remediated: this response only explains the repair
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_repair_edit_fields() {
        let response = r#"
updated formula: always([+SHIP] true -> eventually(<+PAY> true))
edit: Formula 2: <+REFUND> true
patch: F3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)
replacement: this response only explains the replacement
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_labeled_plain_repair_formula_fields() {
        let response = r#"
corrected formula: Formula 1: always([+SHIP] true -> eventually(<+PAY> true))
proposal formula: F2: <+REFUND> true
updated formula: Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_repair_text_fields() {
        let response = r#"
corrected text: always([+SHIP] true -> eventually(<+PAY> true))
repair text: Formula 2: <+REFUND> true
updated text: F3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)
replacement text: this response only explains the replacement
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_equals_separated_plain_repair_fields() {
        let response = r#"
corrected formula = always([+SHIP] true -> eventually(<+PAY> true))
repair = Formula 2: <+REFUND> true
updated text = F3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)
replacement = this response only explains the replacement
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_refinement_fields() {
        let response = r#"
improved formula: always([+SHIP] true -> eventually(<+PAY> true))
refined = Formula 2: <+REFUND> true
resolved text: F3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)
improved text: this response only explains the improvement
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_feedback_fields() {
        let response = r#"
feedback: always([+SHIP] true -> eventually(<+PAY> true))
analysis: this only explains why the first draft failed
critique text: Formula 2: <+REFUND> true
review: F3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)
assessment: prose only
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_solution_fields() {
        let response = r#"
diagnosis text: Formula 1: always([+SHIP] true -> eventually(<+PAY> true))
solution = Formula 2: <+REFUND> true
solution formula: F3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)
diagnosis: this only explains the parse failure
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_reasoning_fields() {
        let response = r#"
explanation: Formula 1: always([+SHIP] true -> eventually(<+PAY> true))
rationale text: Formula 2: <+REFUND> true
reasoning = F3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)
reasoning text: this only explains the repair
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_error_feedback_fields() {
        let response = r#"
error: Formula 1: always([+SHIP] true -> eventually(<+PAY> true))
error message: this only explains why parsing failed
validation error = F2: <+REFUND> true
verifier output: Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_log_output_fields() {
        let response = r#"
stdout: Formula 1: always([+SHIP] true -> eventually(<+PAY> true))
stderr: verifier diagnostics without a formula
logs: F2: <+REFUND> true
trace = Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_error_detail_fields() {
        let response = r#"
detail: Formula 1: always([+SHIP] true -> eventually(<+PAY> true))
details: F2: <+REFUND> true
reason = Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)
hint text: this only suggests trying a simpler rule
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_candidate_field_order_aliases() {
        let response = r#"
formula candidate: Formula 1: always([+SHIP] true -> eventually(<+PAY> true))
rule candidate: F2: <+REFUND> true
formula candidate = this candidate is only explained in prose
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_proposal_field_order_aliases() {
        let response = r#"
formula proposal: Formula 1: always([+SHIP] true -> eventually(<+PAY> true))
rule proposal: F2: <+REFUND> true
formula proposal = this proposal is only explained in prose
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_draft_field_order_aliases() {
        let response = r#"
formula draft: Formula 1: always([+SHIP] true -> eventually(<+PAY> true))
draft formula: F2: <+REFUND> true
rule draft: Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)
draft = this draft is only explained in prose
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_revision_field_order_aliases() {
        let response = r#"
formula revision: Formula 1: always([+SHIP] true -> eventually(<+PAY> true))
revision formula: F2: <+REFUND> true
rule revision: Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)
revision = this revision is only explained in prose
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_fix_field_order_aliases() {
        let response = r#"
formula fix: Formula 1: always([+SHIP] true -> eventually(<+PAY> true))
fix formula: F2: <+REFUND> true
rule fix: Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)
fix = this fix is only explained in prose
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_amendment_field_order_aliases() {
        let response = r#"
formula amendment: Formula 1: always([+SHIP] true -> eventually(<+PAY> true))
amendment formula: F2: <+REFUND> true
rule amendment: Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)
amendment = this amendment is only explained in prose
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_patch_field_order_aliases() {
        let response = r#"
formula patch: Formula 1: always([+SHIP] true -> eventually(<+PAY> true))
patch formula: F2: <+REFUND> true
rule patch: Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)
patch = this patch is only explained in prose
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_update_field_order_aliases() {
        let response = r#"
formula update: Formula 1: always([+SHIP] true -> eventually(<+PAY> true))
update formula: F2: <+REFUND> true
rule update: Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)
update = this update is only explained in prose
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_change_field_order_aliases() {
        let response = r#"
formula change: Formula 1: always([+SHIP] true -> eventually(<+PAY> true))
change formula: F2: <+REFUND> true
rule change: Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)
change = this change is only explained in prose
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_correction_field_order_aliases() {
        let response = r#"
formula correction: Formula 1: always([+SHIP] true -> eventually(<+PAY> true))
correction formula: F2: <+REFUND> true
rule correction: Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)
correction = this correction is only explained in prose
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_suggestion_field_order_aliases() {
        let response = r#"
formula suggestion: Formula 1: always([+SHIP] true -> eventually(<+PAY> true))
suggestion formula: F2: <+REFUND> true
rule suggestion: Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)
suggestion = this suggestion is only explained in prose
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_candidate_formula_fields() {
        let response = r#"
best formula: always([+SHIP] true -> eventually(<+PAY> true))
candidate formula: F2: <+REFUND> true
selected formula: explanation without a formula
validated formula: <+ESCALATE> true
chosen formula: Formula 4: always([+MERGE] true -> <+signed_by(/users/maintainer.id)> true)
accepted formula: this is only prose
verified formula: F5: <+DEPLOY> true
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "<+ESCALATE> true",
                "always([+MERGE] true -> <+signed_by(/users/maintainer.id)> true)",
                "<+DEPLOY> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_response_formula_fields() {
        let response = r#"
generated formula: always([+SHIP] true -> eventually(<+PAY> true))
final formula: Formula 2: <+REFUND> true
output formula: this is only prose
response formula: F4: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_status_text_fields() {
        let response = r#"
best: always([+SHIP] true -> eventually(<+PAY> true))
chosen: Formula 2: <+REFUND> true
accepted: explanation without a formula
selected: F4: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)
validated: <+ESCALATE> true
verified: this candidate passed validation
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
                "<+ESCALATE> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_candidate_text_fields() {
        let response = r#"
candidate: always([+SHIP] true -> eventually(<+PAY> true))
alternative: Formula 2: <+REFUND> true
choice: explanation without a formula
chunk: F4: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)
part: <+ESCALATE> true
segment: prose only
variant: Formula 7: <+DEPLOY> true
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
                "<+ESCALATE> true",
                "<+DEPLOY> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_batch_text_fields() {
        let response = r#"
answers: always([+SHIP] true -> eventually(<+PAY> true))
completions: Formula 2: <+REFUND> true
responses: explanation without a formula
blocks: F4: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)
deltas: <+ESCALATE> true
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
                "<+ESCALATE> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_final_response_aliases() {
        let response = r#"
final response: always([+SHIP] true -> eventually(<+PAY> true))
final message: Formula 2: <+REFUND> true
assistant response: explanation without a formula
assistant message: Formula 3: <+ESCALATE> true
model output: F4: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "<+ESCALATE> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_provider_response_aliases() {
        let response = r#"
assistant output: always([+SHIP] true -> eventually(<+PAY> true))
model response: Formula 2: <+REFUND> true
llm response: explanation without a formula
provider output: Formula 3: <+ESCALATE> true
raw output: F4: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "<+ESCALATE> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_ignores_json_message_without_formula() {
        let response = r#"
{
  "message": "I found two rules and will explain them below.",
  "formula": "<+CANCEL> true"
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(formulas, vec!["<+CANCEL> true"]);
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
    fn test_parse_llm_response_accepts_json_tool_input() {
        let response = r#"
{
  "content": [
    {
      "type": "tool_use",
      "name": "emit_formulas",
      "input": "{\"rules\":[\"always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)\",\"<+ESCALATE> true\"]}"
    }
  ]
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(formulas.len(), 2);
        assert_eq!(
            formulas[0],
            "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
        );
        assert_eq!(formulas[1], "<+ESCALATE> true");
    }

    #[test]
    fn test_parse_llm_response_accepts_json_tool_payload_aliases() {
        let response = r#"
{
  "tool_calls": [
    {
      "function": {
        "name": "emit_formulas",
        "args": "{\"formulas\":[\"always([+PAY] true -> eventually(<+WORK> true))\"]}"
      }
    },
    {
      "function": {
        "name": "emit_more_formulas",
        "params": "{\"rules\":[\"<+CANCEL> true\"]}"
      }
    },
    {
      "function": {
        "name": "emit_structured_formulas",
        "parameters": {
          "formulas": [
            "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
          ]
        }
      }
    }
  ]
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+PAY] true -> eventually(<+WORK> true))",
                "<+CANCEL> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_json_tool_payload_strings() {
        let response = r#"
{
  "function_call": {
    "name": "emit_formulas",
    "arguments": "F1: always([+PAY] true -> eventually(<+WORK> true))"
  },
  "tool_use": {
    "name": "emit_more_formulas",
    "input": "Explanation only.\nFormula 2: <+CANCEL> true"
  },
  "parameters": "Plain non-formula argument text."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+PAY] true -> eventually(<+WORK> true))",
                "<+CANCEL> true"
            ]
        );
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
