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
        let line = strip_formula_wrapping(line);

        if line.starts_with("```") {
            continue;
        }

        if !declaration_lines.is_empty() {
            declaration_lines.push(line.to_string());
            if line.contains('}') {
                formulas.push(declaration_lines.join("\n"));
                declaration_lines.clear();
            }
            continue;
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

fn strip_formula_wrapping(line: &str) -> &str {
    strip_matching_wrapper(line.trim(), "`")
        .or_else(|| strip_matching_wrapper(line.trim(), "**"))
        .or_else(|| strip_matching_wrapper(line.trim(), "__"))
        .or_else(|| strip_matching_wrapper(line.trim(), "*"))
        .or_else(|| strip_matching_wrapper(line.trim(), "_"))
        .unwrap_or(line)
        .trim()
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
        ("buyer", "Buyer"),
        ("seller", "Seller"),
        ("provider", "Provider"),
        ("consumer", "Consumer"),
        ("client", "Client"),
        ("contractor", "Contractor"),
        ("principal", "Principal"),
        ("agent", "Agent"),
        ("depositor", "Depositor"),
        ("deliverer", "Deliverer"),
        ("recipient", "Recipient"),
        ("sender", "Sender"),
        ("receiver", "Receiver"),
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
        ("shipper", "Shipper"),
        ("carrier", "Carrier"),
        ("consignor", "Consignor"),
        ("consignee", "Consignee"),
        ("bailor", "Bailor"),
        ("bailee", "Bailee"),
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
}
