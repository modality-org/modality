use crate::alphabet::{
    candidate_labels, extract_alphabet, formulas_skip_first_step, is_top_level_false,
};
use crate::verify::formulas_satisfied;
use crate::{SynthesisOptions, SynthesisResult};
use modality_lang::{FormulaExpr, Model, Part, Property, Transition};

pub fn synthesize(formulas: &[FormulaExpr], opts: SynthesisOptions) -> SynthesisResult {
    if formulas.is_empty() {
        return SynthesisResult::Unsat {
            max_states: opts.max_states,
            reason: "no formulas provided".to_string(),
            last_candidate: None,
        };
    }

    if formulas.iter().any(is_top_level_false) {
        return SynthesisResult::Unsat {
            max_states: opts.max_states,
            reason: "formula is false".to_string(),
            last_candidate: None,
        };
    }

    let max_states = opts.max_states.max(1);
    let max_transitions = opts.max_transitions.max(1);
    let alphabet = extract_alphabet(formulas);
    let labels = candidate_labels(&alphabet);
    let skip_first_step = formulas_skip_first_step(formulas);
    let mut last_candidate = None;

    if !skip_first_step {
        if let Some(model) =
            grounded_single_step_witness(&opts.name, &alphabet.properties, formulas)
        {
            return SynthesisResult::Witness(model);
        }
    }

    for n in 1..=max_states {
        for skeleton in skeletons(n) {
            if skeleton.len() > max_transitions {
                continue;
            }
            for labeled in label_skeleton(&skeleton, &labels) {
                if skip_first_step && !initial_step_unlabeled(&labeled) {
                    continue;
                }
                let model = build_model(&opts.name, n, &labeled);
                last_candidate = Some(model.clone());
                if formulas_satisfied(&model, formulas) {
                    return SynthesisResult::Witness(model);
                }
            }
        }
    }

    SynthesisResult::Unsat {
        max_states,
        reason: format!(
            "no satisfying witness found within {max_states} states and {max_transitions} transitions"
        ),
        last_candidate,
    }
}

fn grounded_single_step_witness(
    name: &str,
    properties: &[Property],
    formulas: &[FormulaExpr],
) -> Option<Model> {
    let label = positive_label_union(properties);
    if label.is_empty() {
        return None;
    }

    let model = build_model(name, 1, &[(0, 0, label)]);
    formulas_satisfied(&model, formulas).then_some(model)
}

fn positive_label_union(properties: &[Property]) -> Vec<Property> {
    let mut label = Vec::new();
    for property in properties {
        if property.sign != modality_lang::PropertySign::Plus {
            continue;
        }
        if !label.iter().any(|existing| existing == property) {
            label.push(property.clone());
        }
    }
    label
}

fn initial_step_unlabeled(edges: &[(usize, usize, Vec<Property>)]) -> bool {
    let outgoing: Vec<_> = edges.iter().filter(|(from, _, _)| *from == 0).collect();
    !outgoing.is_empty() && outgoing.iter().all(|(_, _, label)| label.is_empty())
}

type Edge = (usize, usize);

fn skeletons(n: usize) -> Vec<Vec<Edge>> {
    let mut graphs = Vec::new();

    // Linear chain with a terminal self-loop. For n=1 this is a single loop.
    let mut chain = Vec::new();
    for index in 0..n.saturating_sub(1) {
        chain.push((index, index + 1));
    }
    chain.push((n - 1, n - 1));
    graphs.push(chain);

    if n >= 2 {
        graphs.push(vec![(0, 1), (1, 1)]);
        graphs.push(vec![(0, 1), (1, 0)]);
    }

    let self_loops: Vec<Edge> = (0..n).map(|index| (index, index)).collect();
    if !graphs.iter().any(|graph| graph == &self_loops) {
        graphs.push(self_loops);
    }

    graphs
}

fn label_skeleton(
    skeleton: &[Edge],
    labels: &[Vec<Property>],
) -> Vec<Vec<(usize, usize, Vec<Property>)>> {
    if skeleton.is_empty() {
        return Vec::new();
    }

    let mut labeled_graphs = Vec::new();

    if skeleton.len() == 1 {
        for label in labels {
            labeled_graphs.push(vec![(skeleton[0].0, skeleton[0].1, label.clone())]);
        }
        return labeled_graphs;
    }

    // All edges empty, then one distinguished edge labeled, rest empty.
    push_assignment(
        &mut labeled_graphs,
        skeleton,
        &vec![Vec::new(); skeleton.len()],
    );
    for edge_index in 0..skeleton.len() {
        for label in labels.iter().filter(|label| !label.is_empty()) {
            let mut assignment = vec![Vec::new(); skeleton.len()];
            assignment[edge_index] = label.clone();
            push_assignment(&mut labeled_graphs, skeleton, &assignment);
        }
    }

    // Every edge the same non-empty label.
    for label in labels.iter().filter(|label| !label.is_empty()) {
        let assignment = vec![label.clone(); skeleton.len()];
        push_assignment(&mut labeled_graphs, skeleton, &assignment);
    }

    // For two-edge graphs, also try every pair of labels (including empty).
    if skeleton.len() == 2 && labels.len() <= 16 {
        for left in labels {
            for right in labels {
                push_assignment(
                    &mut labeled_graphs,
                    skeleton,
                    &[left.clone(), right.clone()],
                );
            }
        }
    }

    labeled_graphs
}

fn push_assignment(
    out: &mut Vec<Vec<(usize, usize, Vec<Property>)>>,
    skeleton: &[Edge],
    labels: &[Vec<Property>],
) {
    let labeled: Vec<(usize, usize, Vec<Property>)> = skeleton
        .iter()
        .zip(labels.iter())
        .map(|((from, to), label)| (*from, *to, label.clone()))
        .collect();
    if !out.iter().any(|existing| existing == &labeled) {
        out.push(labeled);
    }
}

fn build_model(name: &str, n: usize, edges: &[(usize, usize, Vec<Property>)]) -> Model {
    let mut part = Part::new("flow".to_string());
    for (from, to, properties) in edges {
        let mut transition = Transition::new(node_name(*from), node_name(*to));
        for property in properties {
            transition.add_property(property.clone());
        }
        part.add_transition(transition);
    }

    // Isolated prefix nodes would otherwise vanish from the checker, which
    // only discovers ids that appear on transitions. Keep q0 present.
    if n >= 1 && part.transitions.is_empty() {
        part.add_transition(Transition::new(node_name(0), node_name(0)));
    }

    let mut model = Model::new(name.to_string());
    model.set_initial("q0".to_string());
    model.add_part(part);
    model
}

fn node_name(index: usize) -> String {
    format!("q{index}")
}
