//! Model Printer - Serialize models back to Modality syntax
//!
//! Supports the new @Signer shorthand for cleaner output.

#![allow(clippy::collapsible_match, clippy::redundant_closure)]

use crate::ast::{Model, Part, Property, PropertySign, Transition};

/// Print a model to Modality syntax
pub fn print_model(model: &Model) -> String {
    let mut output = String::new();
    output.push_str(&format!("model {} {{\n", model.name));

    for part in &model.parts {
        output.push_str(&print_part(part, 2));
    }

    output.push_str("}\n");
    output
}

/// Print a part with indentation
fn print_part(part: &Part, indent: usize) -> String {
    let mut output = String::new();
    let spaces = " ".repeat(indent);

    output.push_str(&format!("{}part {} {{\n", spaces, part.name));

    for transition in &part.transitions {
        output.push_str(&print_transition(transition, indent + 2));
    }

    output.push_str(&format!("{}}}\n", spaces));
    output
}

/// Print a transition with properties
fn print_transition(transition: &Transition, indent: usize) -> String {
    let spaces = " ".repeat(indent);

    if transition.properties.is_empty() {
        format!("{}{} --> {}\n", spaces, transition.from, transition.to)
    } else {
        let props = print_properties(&transition.properties);
        format!(
            "{}{} --> {}: {}\n",
            spaces, transition.from, transition.to, props
        )
    }
}

/// Print properties with @Signer shorthand detection
fn print_properties(props: &[Property]) -> String {
    props
        .iter()
        .map(|p| print_property(p))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Print a single property, handling predicates
fn print_property(prop: &Property) -> String {
    let sign = match prop.sign {
        PropertySign::Plus => "+",
        PropertySign::Minus => "-",
    };

    // Check if this is a predicate property
    if let Some(ref source) = prop.source {
        if let crate::ast::PropertySource::Predicate { args, .. } = source {
            // Extract the arg from the JSON
            if let Some(arg) = args.get("arg").and_then(|v| v.as_str()) {
                return format!("{}{}({})", sign, prop.name, print_predicate_arg(arg));
            }
            if let Some(args) = args.get("args").and_then(|v| v.as_array()) {
                let rendered_args = args
                    .iter()
                    .filter_map(|arg| arg.as_str())
                    .map(print_predicate_arg)
                    .collect::<Vec<_>>()
                    .join(", ");
                if !rendered_args.is_empty() {
                    return format!("{}{}({})", sign, prop.name, rendered_args);
                }
            }
        }
    }

    format!("{}{}", sign, prop.name)
}

fn print_predicate_arg(arg: &str) -> String {
    if is_identifier(arg) || is_path_literal(arg) {
        arg.to_string()
    } else {
        format!("\"{}\"", arg.replace('\\', "\\\\").replace('"', "\\\""))
    }
}

fn is_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    match chars.next() {
        Some(first) if first == '_' || first.is_ascii_alphabetic() => {}
        _ => return false,
    }
    chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
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
    use crate::synthesis::templates;

    #[test]
    fn test_print_mutual_cooperation() {
        let model = templates::mutual_cooperation("Alice", "Bob");
        let output = print_model(&model);

        assert!(output.contains("model MutualCooperation"));
        assert!(output.contains("+SIGNED_BY_ALICE"));
        assert!(output.contains("+SIGNED_BY_BOB"));
        assert!(output.contains("-DEFECT"));
    }

    #[test]
    fn test_print_escrow() {
        let model = templates::escrow("Buyer", "Seller");
        let output = print_model(&model);

        assert!(output.contains("model Escrow"));
        assert!(output.contains("+DEPOSIT"));
        assert!(output.contains("+DELIVER"));
        assert!(output.contains("+RELEASE"));
    }

    #[test]
    fn test_property_printing() {
        let prop = Property::new(PropertySign::Plus, "SIGNED_BY_ALICE".to_string());
        assert_eq!(print_property(&prop), "+SIGNED_BY_ALICE");

        let prop2 = Property::new(PropertySign::Plus, "DEPOSIT".to_string());
        assert_eq!(print_property(&prop2), "+DEPOSIT");

        let prop3 = Property::new(PropertySign::Minus, "DEFECT".to_string());
        assert_eq!(print_property(&prop3), "-DEFECT");
    }

    #[test]
    fn test_predicate_printing() {
        let prop =
            Property::new_predicate_from_call("signed_by".to_string(), "alice_pubkey".to_string());
        assert_eq!(print_property(&prop), "+signed_by(alice_pubkey)");
    }

    #[test]
    fn test_multi_argument_predicate_printing() {
        let prop = Property::new_predicate_from_call_args(
            "oracle_attests".to_string(),
            vec![
                "/oracles/delivery.id".to_string(),
                "delivered".to_string(),
                "true".to_string(),
            ],
        );
        assert_eq!(
            print_property(&prop),
            "+oracle_attests(/oracles/delivery.id, delivered, true)"
        );
    }

    #[test]
    fn test_numeric_like_predicate_args_are_quoted() {
        let prop = Property::new_predicate_from_call_args(
            "threshold".to_string(),
            vec!["2".to_string(), "/treasury/signers".to_string()],
        );
        assert_eq!(
            print_property(&prop),
            "+threshold(\"2\", /treasury/signers)"
        );
    }

    #[test]
    fn test_printed_numeric_predicate_args_parse_again() {
        let mut model = Model::new("ThresholdContract".to_string());
        let mut part = Part::new("flow".to_string());
        let mut transition = Transition::new("pending".to_string(), "executed".to_string());
        transition.add_property(Property::new(PropertySign::Plus, "EXECUTE".to_string()));
        transition.add_property(Property::new_predicate_from_call_args(
            "threshold".to_string(),
            vec!["2".to_string(), "/treasury/signers".to_string()],
        ));
        part.add_transition(transition);
        model.add_part(part);

        let output = print_model(&model);
        assert!(output.contains("+threshold(\"2\", /treasury/signers)"));
        crate::lalrpop_parser::parse_all_models_content_lalrpop(&output)
            .expect("printed model should parse");
    }
}
