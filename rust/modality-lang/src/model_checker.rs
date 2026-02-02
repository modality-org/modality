use serde::{Serialize, Deserialize};
use crate::ast::{Model, Part, Transition, Property, Formula, FormulaExpr};

/// Represents a state in the model (part name and node name)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct State {
    pub part_name: String,
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
        
        // Check if at least one state from each part satisfies the formula
        let is_satisfied = self.check_satisfaction_per_part(&satisfying_states);
        
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

    /// Check if at least one state from each part satisfies the formula
    fn check_satisfaction_per_part(&self, satisfying_states: &[State]) -> bool {
        // Get all part names from the model
        let model_parts: std::collections::HashSet<String> = self.model.parts
            .iter()
            .map(|p| p.name.clone())
            .collect();
        
        // Get part names from states that satisfy the formula
        let satisfying_parts: std::collections::HashSet<String> = satisfying_states
            .iter()
            .map(|s| s.part_name.clone())
            .collect();
        
        // Check if all parts in the model have at least one satisfying state
        model_parts.is_subset(&satisfying_parts)
    }

    /// Evaluate a formula expression and return all satisfying states
    fn evaluate_formula(&self, expr: &FormulaExpr) -> Vec<State> {
        match expr {
            FormulaExpr::True => {
                // All states satisfy true
                self.all_states()
            }
            FormulaExpr::False => {
                // No states satisfy false
                Vec::new()
            }
            FormulaExpr::Prop(name) => {
                // States where current node name matches the proposition
                self.all_states()
                    .into_iter()
                    .filter(|s| s.node_name == *name)
                    .collect()
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
                let all_states = self.all_states();
                self.difference_states(&all_states, &expr_states)
            }
            FormulaExpr::Implies(left, right) => {
                // P -> Q is equivalent to !P | Q
                let not_left = FormulaExpr::Not(left.clone());
                let or_expr = FormulaExpr::Or(Box::new(not_left), right.clone());
                self.evaluate_formula(&or_expr)
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
            FormulaExpr::DiamondBox(properties, expr) => {
                // [<action>] φ = [-action] false & <+action> φ
                // Expand and evaluate
                let expanded = FormulaExpr::DiamondBox(properties.clone(), expr.clone()).expand_diamond_box();
                self.evaluate_formula(&expanded)
            }
            FormulaExpr::Eventually(expr) => {
                self.evaluate_eventually(expr)
            }
            FormulaExpr::Always(expr) => {
                self.evaluate_always(expr)
            }
            FormulaExpr::Until(left, right) => {
                self.evaluate_until(left, right)
            }
            FormulaExpr::Next(expr) => {
                self.evaluate_next(expr)
            }
            FormulaExpr::Var(name) => {
                // Variable lookup - should be handled in fixed point context
                // For now, treat as proposition (will be substituted during fixed point eval)
                self.all_states()
                    .into_iter()
                    .filter(|s| s.node_name == *name)
                    .collect()
            }
            FormulaExpr::Lfp(var, expr) => {
                self.evaluate_lfp(var, expr)
            }
            FormulaExpr::Gfp(var, expr) => {
                self.evaluate_gfp(var, expr)
            }
        }
    }
    
    /// Evaluate eventually(P): states from which a P-state is reachable
    /// Uses backward reachability (least fixed point)
    fn evaluate_eventually(&self, expr: &FormulaExpr) -> Vec<State> {
        let target_states = self.evaluate_formula(expr);
        let mut result = target_states.clone();
        let mut changed = true;
        
        // Fixed point: keep adding states that can reach the result set
        while changed {
            changed = false;
            let current_result = result.clone();
            
            for part in &self.model.parts {
                for transition in &part.transitions {
                    let from_state = State {
                        part_name: part.name.clone(),
                        node_name: transition.from.clone(),
                    };
                    let to_state = State {
                        part_name: part.name.clone(),
                        node_name: transition.to.clone(),
                    };
                    
                    // If to_state is in result and from_state is not, add from_state
                    if current_result.contains(&to_state) && !result.contains(&from_state) {
                        result.push(from_state);
                        changed = true;
                    }
                }
            }
        }
        
        result
    }
    
    /// Evaluate always(P): states where P holds on all reachable states
    /// Uses forward reachability check (greatest fixed point)
    fn evaluate_always(&self, expr: &FormulaExpr) -> Vec<State> {
        let p_states = self.evaluate_formula(expr);
        let all_states = self.all_states();
        
        // Start with all states, remove those that can reach a non-P state
        let mut result = all_states.clone();
        let mut changed = true;
        
        while changed {
            changed = false;
            let current_result = result.clone();
            
            for state in &current_result {
                // Check if this state satisfies P
                if !p_states.contains(state) {
                    if result.contains(state) {
                        result.retain(|s| s != state);
                        changed = true;
                    }
                    continue;
                }
                
                // Check if any outgoing transition leads to a state not in result
                let part = self.model.parts.iter().find(|p| p.name == state.part_name);
                if let Some(part) = part {
                    for transition in &part.transitions {
                        if transition.from == state.node_name {
                            let to_state = State {
                                part_name: part.name.clone(),
                                node_name: transition.to.clone(),
                            };
                            if !current_result.contains(&to_state) {
                                if result.contains(state) {
                                    result.retain(|s| s != state);
                                    changed = true;
                                }
                                break;
                            }
                        }
                    }
                }
            }
        }
        
        result
    }
    
    /// Evaluate P until Q: P holds until Q becomes true
    /// Least fixed point: Q or (P and exists next state in Until(P,Q))
    fn evaluate_until(&self, left: &FormulaExpr, right: &FormulaExpr) -> Vec<State> {
        let p_states = self.evaluate_formula(left);
        let q_states = self.evaluate_formula(right);
        
        // Start with Q states
        let mut result = q_states.clone();
        let mut changed = true;
        
        while changed {
            changed = false;
            let current_result = result.clone();
            
            for part in &self.model.parts {
                for transition in &part.transitions {
                    let from_state = State {
                        part_name: part.name.clone(),
                        node_name: transition.from.clone(),
                    };
                    let to_state = State {
                        part_name: part.name.clone(),
                        node_name: transition.to.clone(),
                    };
                    
                    // If from_state satisfies P, to_state is in result, and from_state not in result
                    if p_states.contains(&from_state) 
                        && current_result.contains(&to_state) 
                        && !result.contains(&from_state) 
                    {
                        result.push(from_state);
                        changed = true;
                    }
                }
            }
        }
        
        result
    }
    
    /// Evaluate next(P): states with a transition to a P-state
    fn evaluate_next(&self, expr: &FormulaExpr) -> Vec<State> {
        let target_states = self.evaluate_formula(expr);
        let mut result = Vec::new();
        
        for part in &self.model.parts {
            for transition in &part.transitions {
                let from_state = State {
                    part_name: part.name.clone(),
                    node_name: transition.from.clone(),
                };
                let to_state = State {
                    part_name: part.name.clone(),
                    node_name: transition.to.clone(),
                };
                
                if target_states.contains(&to_state) && !result.contains(&from_state) {
                    result.push(from_state);
                }
            }
        }
        
        result
    }
    
    /// Evaluate lfp(X, φ): least fixed point
    /// Start with empty set, iterate until fixed point
    fn evaluate_lfp(&self, var: &str, expr: &FormulaExpr) -> Vec<State> {
        let mut result: Vec<State> = Vec::new();
        let mut changed = true;
        
        while changed {
            // Substitute current result for variable X in the formula
            let substituted = self.substitute_var(expr, var, &result);
            let new_result = self.evaluate_formula(&substituted);
            
            // Check if we've reached a fixed point
            changed = new_result.len() != result.len() || 
                      !new_result.iter().all(|s| result.contains(s));
            result = new_result;
        }
        
        result
    }
    
    /// Evaluate gfp(X, φ): greatest fixed point
    /// Start with all states, iterate until fixed point
    fn evaluate_gfp(&self, var: &str, expr: &FormulaExpr) -> Vec<State> {
        let mut result = self.all_states();
        let mut changed = true;
        
        while changed {
            // Substitute current result for variable X in the formula
            let substituted = self.substitute_var(expr, var, &result);
            let new_result = self.evaluate_formula(&substituted);
            
            // Intersect with current result (gfp is monotonically decreasing)
            let intersection = self.intersect_states(&result, &new_result);
            
            // Check if we've reached a fixed point
            changed = intersection.len() != result.len();
            result = intersection;
        }
        
        result
    }
    
    /// Substitute a variable with a set of states in a formula
    /// Returns a formula where Var(name) is replaced with an Or of Prop(state_name)
    fn substitute_var(&self, expr: &FormulaExpr, var: &str, states: &[State]) -> FormulaExpr {
        match expr {
            FormulaExpr::Var(name) if name == var => {
                // Replace variable with disjunction of state propositions
                if states.is_empty() {
                    FormulaExpr::False
                } else {
                    states.iter().skip(1).fold(
                        FormulaExpr::Prop(states[0].node_name.clone()),
                        |acc, s| FormulaExpr::Or(
                            Box::new(acc),
                            Box::new(FormulaExpr::Prop(s.node_name.clone()))
                        )
                    )
                }
            }
            FormulaExpr::And(l, r) => FormulaExpr::And(
                Box::new(self.substitute_var(l, var, states)),
                Box::new(self.substitute_var(r, var, states)),
            ),
            FormulaExpr::Or(l, r) => FormulaExpr::Or(
                Box::new(self.substitute_var(l, var, states)),
                Box::new(self.substitute_var(r, var, states)),
            ),
            FormulaExpr::Not(inner) => FormulaExpr::Not(
                Box::new(self.substitute_var(inner, var, states))
            ),
            FormulaExpr::Implies(l, r) => FormulaExpr::Implies(
                Box::new(self.substitute_var(l, var, states)),
                Box::new(self.substitute_var(r, var, states)),
            ),
            FormulaExpr::Paren(inner) => FormulaExpr::Paren(
                Box::new(self.substitute_var(inner, var, states))
            ),
            FormulaExpr::Diamond(props, phi) => FormulaExpr::Diamond(
                props.clone(),
                Box::new(self.substitute_var(phi, var, states)),
            ),
            FormulaExpr::Box(props, phi) => FormulaExpr::Box(
                props.clone(),
                Box::new(self.substitute_var(phi, var, states)),
            ),
            FormulaExpr::DiamondBox(props, phi) => FormulaExpr::DiamondBox(
                props.clone(),
                Box::new(self.substitute_var(phi, var, states)),
            ),
            FormulaExpr::Eventually(phi) => FormulaExpr::Eventually(
                Box::new(self.substitute_var(phi, var, states))
            ),
            FormulaExpr::Always(phi) => FormulaExpr::Always(
                Box::new(self.substitute_var(phi, var, states))
            ),
            FormulaExpr::Until(l, r) => FormulaExpr::Until(
                Box::new(self.substitute_var(l, var, states)),
                Box::new(self.substitute_var(r, var, states)),
            ),
            FormulaExpr::Next(phi) => FormulaExpr::Next(
                Box::new(self.substitute_var(phi, var, states))
            ),
            // Nested fixed points: only substitute if var is different
            FormulaExpr::Lfp(v, phi) if v != var => FormulaExpr::Lfp(
                v.clone(),
                Box::new(self.substitute_var(phi, var, states)),
            ),
            FormulaExpr::Gfp(v, phi) if v != var => FormulaExpr::Gfp(
                v.clone(),
                Box::new(self.substitute_var(phi, var, states)),
            ),
            // Don't substitute bound variables or literals
            other => other.clone(),
        }
    }

    /// Evaluate diamond operator: <properties> phi
    fn evaluate_diamond(&self, properties: &[Property], expr: &FormulaExpr) -> Vec<State> {
        let target_states = self.evaluate_formula(expr);
        let mut result = Vec::new();

        for part in &self.model.parts {
            for transition in &part.transitions {
                // Check if this transition has all the required properties
                if self.transition_satisfies_properties(transition, properties) {
                    // Check if the target state satisfies the inner formula
                    let from_state = State {
                        part_name: part.name.clone(),
                        node_name: transition.from.clone(),
                    };
                    
                    let to_state = State {
                        part_name: part.name.clone(),
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

        for part in &self.model.parts {
            for node in self.get_nodes_in_part(part) {
                let state = State {
                    part_name: part.name.clone(),
                    node_name: node.clone(),
                };

                // Check if ALL transitions from this state with all the properties lead to states satisfying phi
                let transitions_with_properties = self.get_transitions_from_node(part, &node)
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
                            part_name: part.name.clone(),
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
    #[allow(dead_code)]
    fn transition_satisfies_property(&self, transition: &Transition, property: &Property) -> bool {
        transition.properties.iter().any(|p| p == property)
    }

    /// Check if a transition satisfies all properties in a list
    /// A transition satisfies a property if:
    /// - For +property: transition explicitly has +property OR doesn't mention property at all
    /// - For -property: transition explicitly has -property OR doesn't mention property at all
    fn transition_satisfies_properties(&self, transition: &Transition, properties: &[Property]) -> bool {
        properties.iter().all(|property| {
            // Check if transition explicitly has this property
            let has_explicit = transition.properties.iter().any(|p| p == property);
            if has_explicit {
                return true;
            }
            
            // If transition doesn't mention this property at all, it's usable
            let property_name = &property.name;
            let mentions_property = transition.properties.iter().any(|p| p.name == *property_name);
            !mentions_property
        })
    }

    /// Get all nodes in a part
    fn get_nodes_in_part(&self, part: &Part) -> Vec<String> {
        let mut nodes = std::collections::HashSet::new();
        for transition in &part.transitions {
            nodes.insert(transition.from.clone());
            nodes.insert(transition.to.clone());
        }
        nodes.into_iter().collect()
    }

    /// Get all transitions from a specific node in a part
    fn get_transitions_from_node<'a>(&self, part: &'a Part, node: &str) -> Vec<&'a Transition> {
        part.transitions.iter()
            .filter(|t| t.from == node)
            .collect()
    }

    /// Get all states in the model
    fn all_states(&self) -> Vec<State> {
        let mut states = Vec::new();
        for part in &self.model.parts {
            for node in self.get_nodes_in_part(part) {
                states.push(State {
                    part_name: part.name.clone(),
                    node_name: node,
                });
            }
        }
        states
    }

    /// Get current possible states (if state information is available)
    // TODO: Expose for debugging/introspection API
    #[allow(dead_code)]
    fn current_states(&self) -> Vec<State> {
        if let Some(state_info) = &self.model.state {
            let mut states = Vec::new();
            for part_state in state_info {
                for node in &part_state.current_nodes {
                    states.push(State {
                        part_name: part_state.part_name.clone(),
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
    use crate::ast::{Model, Part, Transition, Property, PropertySign, Formula, FormulaExpr};

    fn create_test_model() -> Model {
        let mut model = Model::new("TestModel".to_string());
        
        let mut graph1 = Part::new("g1".to_string());
        graph1.add_transition(Transition::new("n1".to_string(), "n2".to_string()));
        let mut t1 = Transition::new("n1".to_string(), "n2".to_string());
        t1.add_property(Property::new(PropertySign::Plus, "blue".to_string()));
        graph1.add_transition(t1);
        
        let mut t2 = Transition::new("n2".to_string(), "n3".to_string());
        t2.add_property(Property::new(PropertySign::Plus, "blue".to_string()));
        graph1.add_transition(t2);
        
        model.add_part(graph1);
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