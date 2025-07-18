use serde::{Serialize, Deserialize};

/// Represents a property with a sign (+ or -)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Property {
    pub sign: PropertySign,
    pub name: String,
}

/// The sign of a property
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PropertySign {
    Plus,
    Minus,
}

/// Represents a transition between nodes
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Transition {
    pub from: String,
    pub to: String,
    pub properties: Vec<Property>,
}

/// Represents a graph within a model
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Graph {
    pub name: String,
    pub transitions: Vec<Transition>,
}

/// Represents the current state of a graph
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GraphState {
    pub graph_name: String,
    pub current_nodes: Vec<String>,
}

/// Represents a complete model
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Model {
    pub name: String,
    pub graphs: Vec<Graph>,
    pub state: Option<Vec<GraphState>>,
}

/// Represents a temporal modal formula
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Formula {
    pub name: String,
    pub expression: FormulaExpr,
}

/// Represents a formula expression
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FormulaExpr {
    /// Boolean literals
    True,
    False,
    /// Boolean operations
    And(Box<FormulaExpr>, Box<FormulaExpr>),
    Or(Box<FormulaExpr>, Box<FormulaExpr>),
    Not(Box<FormulaExpr>),
    /// Parenthesized expressions
    Paren(Box<FormulaExpr>),
    /// Modal operators
    Diamond(Property, Box<FormulaExpr>),
    Box(Property, Box<FormulaExpr>),
}

impl Model {
    /// Create a new model with the given name
    pub fn new(name: String) -> Self {
        Self {
            name,
            graphs: Vec::new(),
            state: None,
        }
    }

    /// Add a graph to this model
    pub fn add_graph(&mut self, graph: Graph) {
        self.graphs.push(graph);
    }

    /// Set the state information for this model
    pub fn set_state(&mut self, state: Vec<GraphState>) {
        self.state = Some(state);
    }
}

impl Graph {
    /// Create a new graph with the given name
    pub fn new(name: String) -> Self {
        Self {
            name,
            transitions: Vec::new(),
        }
    }

    /// Add a transition to this graph
    pub fn add_transition(&mut self, transition: Transition) {
        self.transitions.push(transition);
    }
}

impl Transition {
    /// Create a new transition
    pub fn new(from: String, to: String) -> Self {
        Self {
            from,
            to,
            properties: Vec::new(),
        }
    }

    /// Add a property to this transition
    pub fn add_property(&mut self, property: Property) {
        self.properties.push(property);
    }
}

impl Property {
    /// Create a new property
    pub fn new(sign: PropertySign, name: String) -> Self {
        Self { sign, name }
    }
}

impl GraphState {
    /// Create a new graph state
    pub fn new(graph_name: String, current_nodes: Vec<String>) -> Self {
        Self {
            graph_name,
            current_nodes,
        }
    }
}

impl Formula {
    /// Create a new formula
    pub fn new(name: String, expression: FormulaExpr) -> Self {
        Self { name, expression }
    }
}

/// Represents a top-level item in a modality file
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TopLevelItem {
    Model(Model),
    Formula(Formula),
} 