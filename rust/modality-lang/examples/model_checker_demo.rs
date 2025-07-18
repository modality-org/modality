use modality_lang::{
    parse_content_lalrpop, 
    parse_all_formulas_content_lalrpop,
    ModelChecker,
    ast::{Model, Graph, Transition, Property, PropertySign, GraphState}
};

fn main() -> Result<(), String> {
    println!("=== Model Checker Demo with Model6 ===\n");

    // Manually construct Model6 with state information
    let mut model = Model::new("Model6".to_string());
    
    // Create graph g1
    let mut graph1 = Graph::new("g1".to_string());
    let mut t1 = Transition::new("n1".to_string(), "n2".to_string());
    t1.add_property(Property::new(PropertySign::Plus, "blue".to_string()));
    t1.add_property(Property::new(PropertySign::Minus, "red".to_string()));
    graph1.add_transition(t1);
    
    let mut t2 = Transition::new("n2".to_string(), "n3".to_string());
    t2.add_property(Property::new(PropertySign::Plus, "blue".to_string()));
    t2.add_property(Property::new(PropertySign::Minus, "green".to_string()));
    graph1.add_transition(t2);
    
    let mut t3 = Transition::new("n3".to_string(), "n1".to_string());
    t3.add_property(Property::new(PropertySign::Minus, "blue".to_string()));
    t3.add_property(Property::new(PropertySign::Plus, "red".to_string()));
    graph1.add_transition(t3);
    model.add_graph(graph1);
    
    // Create graph g2
    let mut graph2 = Graph::new("g2".to_string());
    let mut t4 = Transition::new("n1".to_string(), "n1".to_string());
    t4.add_property(Property::new(PropertySign::Plus, "yellow".to_string()));
    graph2.add_transition(t4);
    model.add_graph(graph2);
    
    // Add state information
    let state = vec![
        GraphState::new("g1".to_string(), vec!["n1".to_string(), "n2".to_string()]),
        GraphState::new("g2".to_string(), vec!["n1".to_string()])
    ];
    model.set_state(state);

    println!("📊 Model: {}", model.name);
    println!("   Graphs: {}", model.graphs.len());
    for graph in &model.graphs {
        println!("   - Graph '{}': {} transitions", graph.name, graph.transitions.len());
    }
    
    if let Some(state_info) = &model.state {
        println!("   📍 Current states:");
        for graph_state in state_info {
            println!("      - {}: {:?}", graph_state.graph_name, graph_state.current_nodes);
        }
    }
    println!();

    // Create test formulas
    let formulas_content = r#"
formula FormulaTrue: true
formula FormulaFalse: false
formula FormulaBooleanWff: (true or false) and true
formula FormulaDiamondBlueTrue: <+blue> true
formula FormulaBoxNegBlueFalse: [-blue] false
formula FormulaBlueYellowTest1: <+blue -yellow> true
formula FormulaBlueYellowTest2: <+blue +yellow> true
formula FormulaBlueYellowTest3: <+blue> true
"#;

    // Parse all formulas
    let formulas = parse_all_formulas_content_lalrpop(formulas_content)?;
    println!("📝 Found {} formulas:", formulas.len());
    for formula in &formulas {
        println!("   - {}", formula.name);
    }
    println!();

    // Create model checker
    let checker = ModelChecker::new(model);

    // Test each formula
    for formula in &formulas {
        println!("🔍 Checking formula: {}", formula.name);
        let result = checker.check_formula(formula);
        let result_any_state = checker.check_formula_any_state(formula);
        
        if result.is_satisfied {
            println!("   ✅ Formula is satisfied (per-graph)");
        } else {
            println!("   ❌ Formula is not satisfied (per-graph)");
        }
        
        if result_any_state.is_satisfied {
            println!("   ✅ Formula is satisfied (any state)");
        } else {
            println!("   ❌ Formula is not satisfied (any state)");
        }
        
        println!("   📍 Satisfying states ({}):", result.satisfying_states.len());
        for state in &result.satisfying_states {
            println!("      - {}.{}", state.graph_name, state.node_name);
        }
        println!();
    }

    // Test specific examples
    println!("=== Specific Examples ===\n");

    // Test <+blue> true
    let diamond_blue_true = formulas.iter()
        .find(|f| f.name == "FormulaDiamondBlueTrue")
        .expect("FormulaDiamondBlueTrue not found");
    
    println!("🔍 Testing <+blue> true:");
    let result = checker.check_formula(diamond_blue_true);
    let result_any_state = checker.check_formula_any_state(diamond_blue_true);
    
    if result.is_satisfied {
        println!("   ✅ <+blue> true is satisfied (per-graph)");
    } else {
        println!("   ❌ <+blue> true is not satisfied (per-graph)");
    }
    
    if result_any_state.is_satisfied {
        println!("   ✅ <+blue> true is satisfied (any state)");
    } else {
        println!("   ❌ <+blue> true is not satisfied (any state)");
    }
    
    println!("   📍 States where <+blue> true holds:");
    for state in &result.satisfying_states {
        println!("      - {}.{}", state.graph_name, state.node_name);
    }
    println!();

    // Test <+blue -yellow> true
    let blue_yellow_test1 = formulas.iter()
        .find(|f| f.name == "FormulaBlueYellowTest1")
        .expect("FormulaBlueYellowTest1 not found");
    
    println!("🔍 Testing <+blue -yellow> true:");
    let result = checker.check_formula(blue_yellow_test1);
    let result_any_state = checker.check_formula_any_state(blue_yellow_test1);
    
    if result.is_satisfied {
        println!("   ✅ <+blue -yellow> true is satisfied (per-graph)");
    } else {
        println!("   ❌ <+blue -yellow> true is not satisfied (per-graph)");
    }
    
    if result_any_state.is_satisfied {
        println!("   ✅ <+blue -yellow> true is satisfied (any state)");
    } else {
        println!("   ❌ <+blue -yellow> true is not satisfied (any state)");
    }
    
    println!("   📍 States where <+blue -yellow> true holds:");
    for state in &result.satisfying_states {
        println!("      - {}.{}", state.graph_name, state.node_name);
    }
    println!();

    // Test <+blue +yellow> true
    let blue_yellow_test2 = formulas.iter()
        .find(|f| f.name == "FormulaBlueYellowTest2")
        .expect("FormulaBlueYellowTest2 not found");
    
    println!("🔍 Testing <+blue +yellow> true:");
    let result = checker.check_formula(blue_yellow_test2);
    let result_any_state = checker.check_formula_any_state(blue_yellow_test2);
    
    if result.is_satisfied {
        println!("   ✅ <+blue +yellow> true is satisfied (per-graph)");
    } else {
        println!("   ❌ <+blue +yellow> true is not satisfied (per-graph)");
    }
    
    if result_any_state.is_satisfied {
        println!("   ✅ <+blue +yellow> true is satisfied (any state)");
    } else {
        println!("   ❌ <+blue +yellow> true is not satisfied (any state)");
    }
    
    println!("   📍 States where <+blue +yellow> true holds:");
    for state in &result.satisfying_states {
        println!("      - {}.{}", state.graph_name, state.node_name);
    }

    println!("\n=== Demo Complete ===");
    Ok(())
} 