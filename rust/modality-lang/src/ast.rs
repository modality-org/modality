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

/// Represents a part within a model
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Part {
    pub name: String,
    pub transitions: Vec<Transition>,
}

/// Represents the current state of a part
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PartState {
    pub part_name: String,
    pub current_nodes: Vec<String>,
}

/// Represents a complete model
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Model {
    pub name: String,
    pub parts: Vec<Part>,
    pub state: Option<Vec<PartState>>,
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
    Diamond(Vec<Property>, Box<FormulaExpr>),
    Box(Vec<Property>, Box<FormulaExpr>),
}

impl Model {
    /// Create a new model with the given name
    pub fn new(name: String) -> Self {
        Self {
            name,
            parts: Vec::new(),
            state: None,
        }
    }

    /// Add a part to this model
    pub fn add_part(&mut self, part: Part) {
        self.parts.push(part);
    }

    /// Set the state information for this model
    pub fn set_state(&mut self, state: Vec<PartState>) {
        self.state = Some(state);
    }
}

impl Part {
    /// Create a new part with the given name
    pub fn new(name: String) -> Self {
        Self {
            name,
            transitions: Vec::new(),
        }
    }

    /// Add a transition to this part
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

impl PartState {
    /// Create a new part state
    pub fn new(part_name: String, current_nodes: Vec<String>) -> Self {
        Self {
            part_name,
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