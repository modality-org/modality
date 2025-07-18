/// Represents a property with a sign (+ or -)
#[derive(Debug, Clone, PartialEq)]
pub struct Property {
    pub sign: PropertySign,
    pub name: String,
}

/// The sign of a property
#[derive(Debug, Clone, PartialEq)]
pub enum PropertySign {
    Plus,
    Minus,
}

/// Represents a transition between nodes
#[derive(Debug, Clone, PartialEq)]
pub struct Transition {
    pub from: String,
    pub to: String,
    pub properties: Vec<Property>,
}

/// Represents a graph within a model
#[derive(Debug, Clone, PartialEq)]
pub struct Graph {
    pub name: String,
    pub transitions: Vec<Transition>,
}

/// Represents a complete model
#[derive(Debug, Clone, PartialEq)]
pub struct Model {
    pub name: String,
    pub graphs: Vec<Graph>,
}

impl Model {
    /// Create a new model with the given name
    pub fn new(name: String) -> Self {
        Self {
            name,
            graphs: Vec::new(),
        }
    }

    /// Add a graph to this model
    pub fn add_graph(&mut self, graph: Graph) {
        self.graphs.push(graph);
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