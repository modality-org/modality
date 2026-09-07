use modality_lang::{FormulaExpr, Property, PropertySign};

/// Properties and modality bags mentioned in a formula set, in left-to-right order.
#[derive(Debug, Clone, Default)]
pub struct Alphabet {
    pub properties: Vec<Property>,
    pub bags: Vec<Vec<Property>>,
}

pub fn extract_alphabet(formulas: &[FormulaExpr]) -> Alphabet {
    let mut alphabet = Alphabet::default();
    for formula in formulas {
        walk(formula, &mut alphabet);
    }
    alphabet
}

/// Candidate edge labels: empty first, then plus-polarity singletons, then plus-polarity bags.
pub fn candidate_labels(alphabet: &Alphabet) -> Vec<Vec<Property>> {
    let mut labels = Vec::new();
    push_unique_bag(&mut labels, Vec::new());

    for property in &alphabet.properties {
        push_unique_bag(&mut labels, vec![as_plus(property)]);
    }

    for bag in &alphabet.bags {
        let plus_bag: Vec<Property> = bag.iter().map(as_plus).collect();
        push_unique_bag(&mut labels, plus_bag);
        push_unique_bag(&mut labels, bag.clone());
    }

    labels
}

fn walk(expr: &FormulaExpr, alphabet: &mut Alphabet) {
    match expr {
        FormulaExpr::True | FormulaExpr::False | FormulaExpr::Prop(_) | FormulaExpr::Var(_) => {}
        FormulaExpr::And(left, right)
        | FormulaExpr::Or(left, right)
        | FormulaExpr::Implies(left, right)
        | FormulaExpr::Until(left, right) => {
            walk(left, alphabet);
            walk(right, alphabet);
        }
        FormulaExpr::Not(inner)
        | FormulaExpr::Paren(inner)
        | FormulaExpr::Eventually(inner)
        | FormulaExpr::Always(inner)
        | FormulaExpr::Next(inner)
        | FormulaExpr::Lfp(_, inner)
        | FormulaExpr::Gfp(_, inner) => walk(inner, alphabet),
        FormulaExpr::Diamond(props, inner)
        | FormulaExpr::Box(props, inner)
        | FormulaExpr::DiamondBox(props, inner) => {
            for prop in props {
                push_unique_property(&mut alphabet.properties, prop.clone());
            }
            if !props.is_empty() {
                push_unique_bag(&mut alphabet.bags, props.clone());
            }
            walk(inner, alphabet);
        }
    }
}

fn as_plus(property: &Property) -> Property {
    let mut plus = property.clone();
    plus.sign = PropertySign::Plus;
    plus
}

fn push_unique_property(properties: &mut Vec<Property>, property: Property) {
    if !properties.iter().any(|existing| existing == &property) {
        properties.push(property);
    }
}

pub(crate) fn push_unique_bag(bags: &mut Vec<Vec<Property>>, bag: Vec<Property>) {
    if !bags.iter().any(|existing| bags_eq(existing, &bag)) {
        bags.push(bag);
    }
}

fn bags_eq(left: &[Property], right: &[Property]) -> bool {
    left.len() == right.len() && left.iter().zip(right.iter()).all(|(a, b)| a == b)
}

pub(crate) fn is_top_level_false(expr: &FormulaExpr) -> bool {
    match expr {
        FormulaExpr::False => true,
        FormulaExpr::Paren(inner) => is_top_level_false(inner),
        _ => false,
    }
}

/// `[] φ` and `next(φ)` constrain successors, not the starting state's current step.
pub(crate) fn is_top_level_skip_first(expr: &FormulaExpr) -> bool {
    match expr {
        FormulaExpr::Next(_) => true,
        FormulaExpr::Box(props, _) if props.is_empty() => true,
        FormulaExpr::Paren(inner) => is_top_level_skip_first(inner),
        _ => false,
    }
}

pub(crate) fn formulas_skip_first_step(formulas: &[FormulaExpr]) -> bool {
    !formulas.is_empty() && formulas.iter().all(is_top_level_skip_first)
}
