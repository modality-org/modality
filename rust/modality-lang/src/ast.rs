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
    /// Initial state (for models with direct transitions, no parts)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initial: Option<String>,
    /// Direct transitions (for simpler syntax without parts)
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub transitions: Vec<Transition>,
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
    /// State propositions (node names)
    Prop(String),
    /// Boolean operations
    And(Box<FormulaExpr>, Box<FormulaExpr>),
    Or(Box<FormulaExpr>, Box<FormulaExpr>),
    Not(Box<FormulaExpr>),
    Implies(Box<FormulaExpr>, Box<FormulaExpr>),
    /// Parenthesized expressions
    Paren(Box<FormulaExpr>),
    /// Modal operators (action-labeled)
    /// <action> φ - "there exists an action-transition to a state satisfying φ"
    Diamond(Vec<Property>, Box<FormulaExpr>),
    /// [] φ - "all transitions lead to states satisfying φ" (unlabeled box)
    /// [action] φ - "all action-transitions lead to states satisfying φ"
    Box(Vec<Property>, Box<FormulaExpr>),
    /// [<action>] φ - "committed diamond": can do action AND cannot refuse
    /// Semantically equivalent to: [-action] false & <+action> φ
    /// This is the "must" form - committed to being able to take the action
    DiamondBox(Vec<Property>, Box<FormulaExpr>),
    /// Fixed point operators (modal mu-calculus)
    /// Bound variable reference
    Var(String),
    /// lfp(X, φ) - least fixed point: smallest set satisfying X = φ[X]
    /// Used for "eventually" style properties (inductive/reachability)
    Lfp(String, Box<FormulaExpr>),
    /// gfp(X, φ) - greatest fixed point: largest set satisfying X = φ[X]  
    /// Used for "always" style properties (coinductive/invariants)
    Gfp(String, Box<FormulaExpr>),
    /// Temporal operators (syntactic sugar - desugar to fixed points)
    /// eventually(f) ≡ lfp(X, <>X | f)
    Eventually(Box<FormulaExpr>),
    /// always(f) ≡ gfp(X, []X & f)
    Always(Box<FormulaExpr>),
    Until(Box<FormulaExpr>, Box<FormulaExpr>),
    Next(Box<FormulaExpr>),
}

impl FormulaExpr {
    /// Desugar `must P` into `[<+P>] true`
    /// For disjunctions: `must (P | Q)` → `[<+P>] true | [<+Q>] true`
    pub fn desugar_must(inner: FormulaExpr) -> FormulaExpr {
        match inner {
            // must (P | Q) → [<+P>] true | [<+Q>] true
            FormulaExpr::Or(left, right) => {
                FormulaExpr::Or(
                    Box::new(Self::desugar_must(*left)),
                    Box::new(Self::desugar_must(*right)),
                )
            }
            // must (P & Q) → [<+P>] true & [<+Q>] true
            FormulaExpr::And(left, right) => {
                FormulaExpr::And(
                    Box::new(Self::desugar_must(*left)),
                    Box::new(Self::desugar_must(*right)),
                )
            }
            // must (φ) - unwrap parens
            FormulaExpr::Paren(inner) => Self::desugar_must(*inner),
            // must P where P is a proposition → [<+P>] true
            FormulaExpr::Prop(name) => {
                FormulaExpr::DiamondBox(
                    vec![Property::new(PropertySign::Plus, name)],
                    Box::new(FormulaExpr::True),
                )
            }
            // For predicates like signed_by(X), wrap in diamond-box
            // must <+action> φ → [<+action>] φ (already diamond form)
            FormulaExpr::Diamond(props, phi) => {
                FormulaExpr::DiamondBox(props, phi)
            }
            // Other cases: wrap as-is (may need refinement)
            other => other,
        }
    }
    
    /// Expand DiamondBox to its semantic equivalent for model checking
    /// [<+P>] φ → [-P] false & <+P> φ
    pub fn expand_diamond_box(&self) -> FormulaExpr {
        match self {
            FormulaExpr::DiamondBox(props, phi) => {
                // Negate the properties for the box part
                let negated_props: Vec<Property> = props.iter().map(|p| {
                    Property::new(
                        match p.sign {
                            PropertySign::Plus => PropertySign::Minus,
                            PropertySign::Minus => PropertySign::Plus,
                        },
                        p.name.clone(),
                    )
                }).collect();
                
                // [-P] false & <+P> φ
                FormulaExpr::And(
                    Box::new(FormulaExpr::Box(negated_props, Box::new(FormulaExpr::False))),
                    Box::new(FormulaExpr::Diamond(props.clone(), Box::new(phi.expand_diamond_box()))),
                )
            }
            // Recursively expand in subformulas
            FormulaExpr::And(l, r) => FormulaExpr::And(
                Box::new(l.expand_diamond_box()),
                Box::new(r.expand_diamond_box()),
            ),
            FormulaExpr::Or(l, r) => FormulaExpr::Or(
                Box::new(l.expand_diamond_box()),
                Box::new(r.expand_diamond_box()),
            ),
            FormulaExpr::Not(inner) => FormulaExpr::Not(Box::new(inner.expand_diamond_box())),
            FormulaExpr::Implies(l, r) => FormulaExpr::Implies(
                Box::new(l.expand_diamond_box()),
                Box::new(r.expand_diamond_box()),
            ),
            FormulaExpr::Paren(inner) => FormulaExpr::Paren(Box::new(inner.expand_diamond_box())),
            FormulaExpr::Diamond(props, phi) => FormulaExpr::Diamond(
                props.clone(),
                Box::new(phi.expand_diamond_box()),
            ),
            FormulaExpr::Box(props, phi) => FormulaExpr::Box(
                props.clone(),
                Box::new(phi.expand_diamond_box()),
            ),
            FormulaExpr::Eventually(phi) => FormulaExpr::Eventually(Box::new(phi.expand_diamond_box())),
            FormulaExpr::Always(phi) => FormulaExpr::Always(Box::new(phi.expand_diamond_box())),
            FormulaExpr::Until(l, r) => FormulaExpr::Until(
                Box::new(l.expand_diamond_box()),
                Box::new(r.expand_diamond_box()),
            ),
            FormulaExpr::Next(phi) => FormulaExpr::Next(Box::new(phi.expand_diamond_box())),
            // Fixed point operators
            FormulaExpr::Lfp(var, phi) => FormulaExpr::Lfp(var.clone(), Box::new(phi.expand_diamond_box())),
            FormulaExpr::Gfp(var, phi) => FormulaExpr::Gfp(var.clone(), Box::new(phi.expand_diamond_box())),
            // Literals, props, and vars don't contain DiamondBox
            other => other.clone(),
        }
    }
    
    /// Desugar temporal operators to their fixed point equivalents
    /// always(f) ≡ gfp(X, []X & f)
    /// eventually(f) ≡ lfp(X, <>X | f)
    pub fn desugar_temporal(&self) -> FormulaExpr {
        match self {
            // always(f) → gfp(X, []X & f)
            FormulaExpr::Always(phi) => {
                let inner = phi.desugar_temporal();
                FormulaExpr::Gfp(
                    "X".to_string(),
                    Box::new(FormulaExpr::And(
                        Box::new(FormulaExpr::Box(vec![], Box::new(FormulaExpr::Var("X".to_string())))),
                        Box::new(inner),
                    )),
                )
            }
            // eventually(f) → lfp(X, <>X | f)
            FormulaExpr::Eventually(phi) => {
                let inner = phi.desugar_temporal();
                FormulaExpr::Lfp(
                    "X".to_string(),
                    Box::new(FormulaExpr::Or(
                        Box::new(FormulaExpr::Diamond(vec![], Box::new(FormulaExpr::Var("X".to_string())))),
                        Box::new(inner),
                    )),
                )
            }
            // until(p, q) → lfp(X, q | (p & <>X))
            FormulaExpr::Until(p, q) => {
                let p_inner = p.desugar_temporal();
                let q_inner = q.desugar_temporal();
                FormulaExpr::Lfp(
                    "X".to_string(),
                    Box::new(FormulaExpr::Or(
                        Box::new(q_inner),
                        Box::new(FormulaExpr::And(
                            Box::new(p_inner),
                            Box::new(FormulaExpr::Diamond(vec![], Box::new(FormulaExpr::Var("X".to_string())))),
                        )),
                    )),
                )
            }
            // Recursively desugar subformulas
            FormulaExpr::And(l, r) => FormulaExpr::And(
                Box::new(l.desugar_temporal()),
                Box::new(r.desugar_temporal()),
            ),
            FormulaExpr::Or(l, r) => FormulaExpr::Or(
                Box::new(l.desugar_temporal()),
                Box::new(r.desugar_temporal()),
            ),
            FormulaExpr::Not(inner) => FormulaExpr::Not(Box::new(inner.desugar_temporal())),
            FormulaExpr::Implies(l, r) => FormulaExpr::Implies(
                Box::new(l.desugar_temporal()),
                Box::new(r.desugar_temporal()),
            ),
            FormulaExpr::Paren(inner) => FormulaExpr::Paren(Box::new(inner.desugar_temporal())),
            FormulaExpr::Diamond(props, phi) => FormulaExpr::Diamond(
                props.clone(),
                Box::new(phi.desugar_temporal()),
            ),
            FormulaExpr::Box(props, phi) => FormulaExpr::Box(
                props.clone(),
                Box::new(phi.desugar_temporal()),
            ),
            FormulaExpr::DiamondBox(props, phi) => FormulaExpr::DiamondBox(
                props.clone(),
                Box::new(phi.desugar_temporal()),
            ),
            FormulaExpr::Next(phi) => FormulaExpr::Next(Box::new(phi.desugar_temporal())),
            FormulaExpr::Lfp(var, phi) => FormulaExpr::Lfp(var.clone(), Box::new(phi.desugar_temporal())),
            FormulaExpr::Gfp(var, phi) => FormulaExpr::Gfp(var.clone(), Box::new(phi.desugar_temporal())),
            // Literals, props, and vars pass through
            other => other.clone(),
        }
    }
}

impl Model {
    /// Create a new model with the given name
    pub fn new(name: String) -> Self {
        Self {
            name,
            parts: Vec::new(),
            state: None,
            initial: None,
            transitions: Vec::new(),
        }
    }

    /// Create a new model with initial state and direct transitions
    pub fn new_simple(name: String, initial: String, transitions: Vec<Transition>) -> Self {
        Self {
            name,
            parts: Vec::new(),
            state: None,
            initial: Some(initial),
            transitions,
        }
    }

    /// Add a part to this model
    pub fn add_part(&mut self, part: Part) {
        self.parts.push(part);
    }

    /// Add a direct transition (for simple models without parts)
    pub fn add_transition(&mut self, transition: Transition) {
        self.transitions.push(transition);
    }

    /// Set the initial state
    pub fn set_initial(&mut self, state: String) {
        self.initial = Some(state);
    }

    /// Set the state information for this model
    pub fn set_state(&mut self, state: Vec<PartState>) {
        self.state = Some(state);
    }

    /// Get all transitions (from parts or direct)
    pub fn all_transitions(&self) -> Vec<&Transition> {
        let mut result: Vec<&Transition> = self.transitions.iter().collect();
        for part in &self.parts {
            result.extend(part.transitions.iter());
        }
        result
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

/// A rule that applies only to the commit it's attached to
/// Used for threshold signatures and other commit-time validation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuleForThisCommit {
    /// The formula expression (e.g., signed_by_n(2, [...]))
    pub expression: CommitRuleExpr,
}

/// Expressions valid in a rule_for_this_commit
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CommitRuleExpr {
    /// signed_by_n(n, [signer1, signer2, ...]) - threshold signature
    SignedByN { required: usize, signers: Vec<String> },
    /// signed_by(path) - single required signature
    SignedBy(String),
    /// Conjunction
    And(Box<CommitRuleExpr>, Box<CommitRuleExpr>),
    /// Disjunction
    Or(Box<CommitRuleExpr>, Box<CommitRuleExpr>),
}

impl RuleForThisCommit {
    pub fn new(expression: CommitRuleExpr) -> Self {
        Self { expression }
    }
    
    pub fn signed_by(signer: String) -> Self {
        Self {
            expression: CommitRuleExpr::SignedBy(signer),
        }
    }
    
    pub fn signed_by_n(required: usize, signers: Vec<String>) -> Self {
        Self {
            expression: CommitRuleExpr::SignedByN { required, signers },
        }
    }
}

/// Represents a top-level item in a modality file
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TopLevelItem {
    Model(Model),
    Formula(Formula),
    Action(Action),
    Test(Test),
    Contract(Contract),
    RuleForThisCommit(RuleForThisCommit),
}

/// Helper enum for parsing model body items
#[derive(Debug, Clone, PartialEq)]
pub enum ModelBodyItem {
    Initial(String),
    Transition(Transition),
    Part(Part),
}

/// A contract is an append-only log of commits
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Contract {
    pub name: String,
    pub commits: Vec<ContractCommit>,
}

/// A commit in a contract log
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContractCommit {
    pub signed_by: String,
    pub signature: String,
    pub model: Option<Model>,
    pub statements: Vec<CommitStatement>,
}

/// Statements within a commit
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CommitStatement {
    SignedBy { party: String, signature: String },
    Model(Model),
    /// Add a rule (transitions as +ADD_RULE in model)
    AddRule(FormulaExpr),
    /// Domain action
    Do(Vec<Property>),
}

impl Contract {
    pub fn new(name: String) -> Self {
        Self {
            name,
            commits: Vec::new(),
        }
    }
    
    pub fn add_commit(&mut self, commit: ContractCommit) {
        self.commits.push(commit);
    }
}

impl ContractCommit {
    pub fn new(signed_by: String, signature: String) -> Self {
        Self {
            signed_by,
            signature,
            model: None,
            statements: Vec::new(),
        }
    }
    
    pub fn with_model(signed_by: String, signature: String, model: Model) -> Self {
        Self {
            signed_by,
            signature,
            model: Some(model),
            statements: Vec::new(),
        }
    }
    
    pub fn add_statement(&mut self, stmt: CommitStatement) {
        self.statements.push(stmt);
    }
} 