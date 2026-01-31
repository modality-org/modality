use serde::{Serialize, Deserialize};

/// Represents a property with a sign (+ or -)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Property {
    pub sign: PropertySign,
    pub name: String,
    /// Optional source for the property value (static or predicate)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<PropertySource>,
}

/// The sign of a property
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PropertySign {
    Plus,
    Minus,
}

/// Source of a property's value
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PropertySource {
    /// Static property (manually assigned)
    Static,
    /// Predicate-based property (computed via WASM)
    Predicate {
        /// Path to the WASM module (e.g., "/_code/modal/signed_by.wasm")
        path: String,
        /// Arguments to pass to the predicate (JSON)
        args: serde_json::Value,
    },
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

/// Represents an action declaration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Action {
    pub name: String,
    pub properties: Vec<Property>,
}

/// Represents an action function call
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActionCall {
    pub argument: String,
}

/// Represents a test declaration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Test {
    pub name: Option<String>,
    pub statements: Vec<TestStatement>,
}

/// Represents a statement within a test
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TestStatement {
    Assignment(String, String), // variable = expression
    Commit(String), // commit(action)
    ActionCall(String), // action("+hello")
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
    /// Create a new static property
    pub fn new(sign: PropertySign, name: String) -> Self {
        Self { 
            sign, 
            name,
            source: Some(PropertySource::Static),
        }
    }

    /// Create a new predicate-based property
    pub fn new_predicate(sign: PropertySign, name: String, path: String, args: serde_json::Value) -> Self {
        Self {
            sign,
            name,
            source: Some(PropertySource::Predicate { path, args }),
        }
    }

    /// Create a predicate property from a function call in the grammar
    /// e.g., +signed_by("alice_pubkey") -> predicate that verifies signature
    pub fn new_predicate_from_call(name: String, arg: String) -> Self {
        let path = format!("/_code/modal/{}.wasm", name);
        let args = serde_json::json!({ "arg": arg });
        Self {
            sign: PropertySign::Plus,
            name,
            source: Some(PropertySource::Predicate { path, args }),
        }
    }

    /// Create a negated predicate property from a function call
    /// e.g., -signed_by("alice_pubkey") -> requires signature NOT present
    pub fn new_predicate_from_call_negated(name: String, arg: String) -> Self {
        let path = format!("/_code/modal/{}.wasm", name);
        let args = serde_json::json!({ "arg": arg });
        Self {
            sign: PropertySign::Minus,
            name,
            source: Some(PropertySource::Predicate { path, args }),
        }
    }

    /// Check if this is a static property
    pub fn is_static(&self) -> bool {
        matches!(self.source, Some(PropertySource::Static) | None)
    }

    /// Check if this is a predicate-based property
    pub fn is_predicate(&self) -> bool {
        matches!(self.source, Some(PropertySource::Predicate { .. }))
    }

    /// Get the predicate path and args if this is a predicate property
    pub fn get_predicate(&self) -> Option<(&str, &serde_json::Value)> {
        match &self.source {
            Some(PropertySource::Predicate { path, args }) => Some((path.as_str(), args)),
            _ => None,
        }
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

impl Action {
    /// Create a new action
    pub fn new(name: String, properties: Vec<Property>) -> Self {
        Self { name, properties }
    }
}

impl ActionCall {
    /// Create a new action call
    pub fn new(argument: String) -> Self {
        Self { argument }
    }
}

impl Test {
    /// Create a new test
    pub fn new(name: Option<String>) -> Self {
        Self {
            name,
            statements: Vec::new(),
        }
    }

    /// Create a new test with statements
    pub fn with_statements(name: Option<String>, statements: Vec<TestStatement>) -> Self {
        Self { name, statements }
    }

    /// Add a statement to this test
    pub fn add_statement(&mut self, statement: TestStatement) {
        self.statements.push(statement);
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
    Action(Action),
    Test(Test),
} 