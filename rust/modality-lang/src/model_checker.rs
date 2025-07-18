use serde::{Serialize, Deserialize};
use crate::ast::{Model, Graph, Transition, Property, PropertySign, Formula, FormulaExpr};

/// Represents a state in the model (graph name and node name)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct State {
    pub graph_name: String,
    pub node_name: String,
}

/// Represents the result of model checking
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelCheckResult {
    pub formula: Formula,
    pub satisfying_states: Vec<State>,
    pub is_satisfied: bool,
}

/// Model checker for temporal modal formulas
pub struct ModelChecker {
    model: Model,
}

impl ModelChecker {
    /// Create a new model checker for the given model
    pub fn new(model: Model) -> Self {
        Self { model }
    }

    /// Check if a formula is satisfied by the model (requires at least one state from each graph)
    pub fn check_formula(&self, formula: &Formula) -> ModelCheckResult {
        let satisfying_states = self.evaluate_formula(&formula.expression);
        
        // Check if at least one state from each graph satisfies the formula
        let is_satisfied = self.check_satisfaction_per_graph(&satisfying_states);
        
        ModelCheckResult {
            formula: formula.clone(),
            satisfying_states: satisfying_states.clone(),
            is_satisfied,
        }
    }

    /// Check if any state satisfies the formula (original behavior)
    pub fn check_formula_any_state(&self, formula: &Formula) -> ModelCheckResult {
        let satisfying_states = self.evaluate_formula(&formula.expression);
        
        ModelCheckResult {
            formula: formula.clone(),
            satisfying_states: satisfying_states.clone(),
            is_satisfied: !satisfying_states.is_empty(),
        }
    }

    /// Check if at least one state from each graph satisfies the formula
    fn check_satisfaction_per_graph(&self, satisfying_states: &[State]) -> bool {
        // Get all graph names from the model
        let model_graphs: std::collections::HashSet<String> = self.model.graphs
            .iter()
            .map(|g| g.name.clone())
            .collect();
        
        // Get graph names from states that satisfy the formula
        let satisfying_graphs: std::collections::HashSet<String> = satisfying_states
            .iter()
            .map(|s| s.graph_name.clone())
            .collect();
        
        // Check if all graphs in the model have at least one satisfying state
        model_graphs.is_subset(&satisfying_graphs)
    }

    /// Evaluate a formula expression and return all satisfying states
    fn evaluate_formula(&self, expr: &FormulaExpr) -> Vec<State> {
        match expr {
            FormulaExpr::True => {
                // Current states satisfy true
                self.current_states()
            }
            FormulaExpr::False => {
                // No states satisfy false
                Vec::new()
            }
            FormulaExpr::And(left, right) => {
                let left_states = self.evaluate_formula(left);
                let right_states = self.evaluate_formula(right);
                self.intersect_states(&left_states, &right_states)
            }
            FormulaExpr::Or(left, right) => {
                let left_states = self.evaluate_formula(left);
                let right_states = self.evaluate_formula(right);
                self.union_states(&left_states, &right_states)
            }
            FormulaExpr::Not(expr) => {
                let expr_states = self.evaluate_formula(expr);
                let current_states = self.current_states();
                self.difference_states(&current_states, &expr_states)
            }
            FormulaExpr::Paren(expr) => {
                self.evaluate_formula(expr)
            }
            FormulaExpr::Diamond(properties, expr) => {
                self.evaluate_diamond(properties, expr)
            }
            FormulaExpr::Box(properties, expr) => {
                self.evaluate_box(properties, expr)
            }
        }
    }

    /// Evaluate diamond operator: <properties> phi
    fn evaluate_diamond(&self, properties: &[Property], expr: &FormulaExpr) -> Vec<State> {
        let target_states = self.evaluate_formula(expr);
        let mut result = Vec::new();

        for graph in &self.model.graphs {
            for transition in &graph.transitions {
                // Check if this transition has all the required properties
                if self.transition_satisfies_properties(transition, properties) {
                    // Check if the target state satisfies the inner formula
                    let from_state = State {
                        graph_name: graph.name.clone(),
                        node_name: transition.from.clone(),
                    };
                    
                    let to_state = State {
                        graph_name: graph.name.clone(),
                        node_name: transition.to.clone(),
                    };

                    // If the target state satisfies the formula, then the source state satisfies <properties> phi
                    if target_states.contains(&to_state) {
                        result.push(from_state);
                    }
                }
            }
        }

        result
    }

    /// Evaluate box operator: [properties] phi
    fn evaluate_box(&self, properties: &[Property], expr: &FormulaExpr) -> Vec<State> {
        let target_states = self.evaluate_formula(expr);
        let mut result = Vec::new();

        for graph in &self.model.graphs {
            for node in self.get_nodes_in_graph(graph) {
                let state = State {
                    graph_name: graph.name.clone(),
                    node_name: node.clone(),
                };

                // Check if ALL transitions from this state with all the properties lead to states satisfying phi
                let transitions_with_properties = self.get_transitions_from_node(graph, &node)
                    .into_iter()
                    .filter(|t| self.transition_satisfies_properties(t, properties))
                    .collect::<Vec<_>>();

                if transitions_with_properties.is_empty() {
                    // No transitions with these properties, so vacuously true
                    result.push(state);
                } else {
                    // Check if all target states satisfy the formula
                    let all_targets_satisfy = transitions_with_properties.iter().all(|t| {
                        let target_state = State {
                            graph_name: graph.name.clone(),
                            node_name: t.to.clone(),
                        };
                        target_states.contains(&target_state)
                    });

                    if all_targets_satisfy {
                        result.push(state);
                    }
                }
            }
        }

        result
    }

    /// Check if a transition satisfies a property
    fn transition_satisfies_property(&self, transition: &Transition, property: &Property) -> bool {
        transition.properties.iter().any(|p| p == property)
    }

    /// Check if a transition satisfies all properties in a list
    fn transition_satisfies_properties(&self, transition: &Transition, properties: &[Property]) -> bool {
        properties.iter().all(|property| {
            transition.properties.iter().any(|p| p == property)
        })
    }

    /// Get all nodes in a graph
    fn get_nodes_in_graph(&self, graph: &Graph) -> Vec<String> {
        let mut nodes = std::collections::HashSet::new();
        for transition in &graph.transitions {
            nodes.insert(transition.from.clone());
            nodes.insert(transition.to.clone());
        }
        nodes.into_iter().collect()
    }

    /// Get all transitions from a specific node in a graph
    fn get_transitions_from_node<'a>(&self, graph: &'a Graph, node: &str) -> Vec<&'a Transition> {
        graph.transitions.iter()
            .filter(|t| t.from == node)
            .collect()
    }

    /// Get all states in the model
    fn all_states(&self) -> Vec<State> {
        let mut states = Vec::new();
        for graph in &self.model.graphs {
            for node in self.get_nodes_in_graph(graph) {
                states.push(State {
                    graph_name: graph.name.clone(),
                    node_name: node,
                });
            }
        }
        states
    }

    /// Get current possible states (if state information is available)
    fn current_states(&self) -> Vec<State> {
        if let Some(state_info) = &self.model.state {
            let mut states = Vec::new();
            for graph_state in state_info {
                for node in &graph_state.current_nodes {
                    states.push(State {
                        graph_name: graph_state.graph_name.clone(),
                        node_name: node.clone(),
                    });
                }
            }
            states
        } else {
            // If no state information, return all states
            self.all_states()
        }
    }

    /// Intersect two sets of states
    fn intersect_states(&self, states1: &[State], states2: &[State]) -> Vec<State> {
        states1.iter()
            .filter(|s1| states2.contains(s1))
            .cloned()
            .collect()
    }

    /// Union two sets of states
    fn union_states(&self, states1: &[State], states2: &[State]) -> Vec<State> {
        let mut result = states1.to_vec();
        for state in states2 {
            if !result.contains(state) {
                result.push(state.clone());
            }
        }
        result
    }

    /// Difference of two sets of states (states1 - states2)
    fn difference_states(&self, states1: &[State], states2: &[State]) -> Vec<State> {
        states1.iter()
            .filter(|s| !states2.contains(s))
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Model, Graph, Transition, Property, PropertySign, Formula, FormulaExpr};

    fn create_test_model() -> Model {
        let mut model = Model::new("TestModel".to_string());
        
        let mut graph1 = Graph::new("g1".to_string());
        graph1.add_transition(Transition::new("n1".to_string(), "n2".to_string()));
        let mut t1 = Transition::new("n1".to_string(), "n2".to_string());
        t1.add_property(Property::new(PropertySign::Plus, "blue".to_string()));
        graph1.add_transition(t1);
        
        let mut t2 = Transition::new("n2".to_string(), "n3".to_string());
        t2.add_property(Property::new(PropertySign::Plus, "blue".to_string()));
        graph1.add_transition(t2);
        
        model.add_graph(graph1);
        model
    }

    #[test]
    fn test_evaluate_true() {
        let model = create_test_model();
        let checker = ModelChecker::new(model);
        let formula = Formula::new("True".to_string(), FormulaExpr::True);
        
        let result = checker.check_formula(&formula);
        assert!(result.is_satisfied);
        assert_eq!(result.satisfying_states.len(), 3); // n1, n2, n3
    }

    #[test]
    fn test_evaluate_false() {
        let model = create_test_model();
        let checker = ModelChecker::new(model);
        let formula = Formula::new("False".to_string(), FormulaExpr::False);
        
        let result = checker.check_formula(&formula);
        assert!(!result.is_satisfied);
        assert_eq!(result.satisfying_states.len(), 0);
    }

    #[test]
    fn test_evaluate_diamond() {
        let model = create_test_model();
        let checker = ModelChecker::new(model);
        
        let formula = Formula::new("DiamondBlueTrue".to_string(), 
            FormulaExpr::Diamond(
                vec![Property::new(PropertySign::Plus, "blue".to_string())],
                Box::new(FormulaExpr::True)
            )
        );
        
        let result = checker.check_formula(&formula);
        assert!(result.is_satisfied);
        // n1 should satisfy <+blue> true because it has a transition to n2 with +blue
        assert!(result.satisfying_states.iter().any(|s| s.node_name == "n1"));
    }
} 