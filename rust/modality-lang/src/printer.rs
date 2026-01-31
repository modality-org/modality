//! Model Printer - Serialize models back to Modality syntax
//!
//! Supports the new @Signer shorthand for cleaner output.

use crate::ast::{Model, Part, Transition, Property, PropertySign};

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
        format!("{}{} --> {}: {}\n", spaces, transition.from, transition.to, props)
    }
}

/// Print properties with @Signer shorthand detection
fn print_properties(props: &[Property]) -> String {
    props.iter()
        .map(|p| print_property(p))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Print a single property, using @Signer shorthand when applicable
fn print_property(prop: &Property) -> String {
    // Check if this is a SIGNED_BY_ property that can use shorthand
    if prop.sign == PropertySign::Plus && prop.name.starts_with("SIGNED_BY_") {
        let signer = &prop.name[10..]; // Remove "SIGNED_BY_" prefix
        // Convert to title case for the shorthand
        let signer_pretty = signer
            .chars()
            .enumerate()
            .map(|(i, c)| {
                if i == 0 { c.to_ascii_uppercase() }
                else { c.to_ascii_lowercase() }
            })
            .collect::<String>();
        format!("@{}", signer_pretty)
    } else {
        let sign = match prop.sign {
            PropertySign::Plus => "+",
            PropertySign::Minus => "-",
        };
        format!("{}{}", sign, prop.name)
    }
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
        assert!(output.contains("@Alice"));
        assert!(output.contains("@Bob"));
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
    fn test_signer_shorthand() {
        let prop = Property::new(PropertySign::Plus, "SIGNED_BY_ALICE".to_string());
        assert_eq!(print_property(&prop), "@Alice");
        
        let prop2 = Property::new(PropertySign::Plus, "DEPOSIT".to_string());
        assert_eq!(print_property(&prop2), "+DEPOSIT");
        
        let prop3 = Property::new(PropertySign::Minus, "DEFECT".to_string());
        assert_eq!(print_property(&prop3), "-DEFECT");
    }
}
