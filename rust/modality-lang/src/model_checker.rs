use crate::ast::{Formula, FormulaExpr, Model, Part, Property, PropertySign, Transition};
use crate::theory::flow::{flow_seeded, Flow, FlowEdge};
use crate::theory::{
    standard, Constraint, Lit, NoState, Registry, StateView, Theory, TheoryVersion, Tri,
};
use crate::vars;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

/// Represents an internal LTS witness node (part name and node id).
///
/// Node ids are not user-facing contract states; transition labels carry the
/// contract meaning.
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

/// A transition whose own label set cannot hold on one edge, per the theory.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeadEdge {
    pub part_name: String,
    pub from: String,
    pub to: String,
    pub properties: Vec<Property>,
    /// Offending literals, sorted; from `Verdict::explain`.
    pub offending: Vec<String>,
}

/// Runtime status of one outgoing move, from accepted state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MoveStatus {
    /// Nothing in accepted state rules the edge out.
    Open,
    /// Some literal on the edge is false now.
    Blocked,
    /// The only open edge out of a node that has more than one.
    Forced,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Move {
    pub part_name: String,
    pub from: String,
    pub to: String,
    pub properties: Vec<Property>,
    pub status: MoveStatus,
    /// Literals that block the move, when `Blocked`.
    pub offending: Vec<String>,
}

/// The graph commits move on: every part's transitions and the top-level
/// ones, as one part over node names. A commit at a node may take any edge
/// out of it, whichever part the edge is written in. The part is named
/// after the parts it joins (`a+b`).
pub fn merged_model(model: &Model) -> Model {
    if model.parts.len() <= 1 && model.transitions.is_empty() {
        return model.clone();
    }
    let mut names: Vec<&str> = model.parts.iter().map(|p| p.name.as_str()).collect();
    if names.is_empty() {
        names.push("default");
    }
    let mut part = Part::new(names.join("+"));
    for t in model
        .parts
        .iter()
        .flat_map(|p| p.transitions.iter())
        .chain(model.transitions.iter())
    {
        part.add_transition(t.clone());
    }
    let mut merged = model.clone();
    merged.parts = vec![part];
    merged.transitions = Vec::new();
    merged
}

/// Where the first commit starts: `initial` when set, else the first edge of
/// [`merged_model`] (in file order) whose source no edge enters, else the
/// first edge's source, else `init`. Moving an edge between parts, without
/// reordering, keeps the start.
pub fn start_nodes(model: &Model) -> Vec<String> {
    if let Some(initial) = &model.initial {
        return vec![initial.clone()];
    }
    let merged = merged_model(model);
    let edges: Vec<&Transition> = merged
        .parts
        .iter()
        .flat_map(|p| p.transitions.iter())
        .collect();
    let entered: HashSet<&str> = edges.iter().map(|t| t.to.as_str()).collect();
    let start = edges
        .iter()
        .find(|t| !entered.contains(t.from.as_str()))
        .or_else(|| edges.first())
        .map(|t| t.from.clone())
        .unwrap_or_else(|| "init".to_string());
    vec![start]
}

/// `expr` with every negation pushed down to propositions, so each box and
/// diamond is evaluated where the formula asserts it. Edge matching errs in
/// one direction per operator: a box counts every edge a commit might take,
/// a diamond only edges the theory cannot rule out. Each is safe only
/// un-negated; under a negation it would err the wrong way.
fn negation_normal_form(expr: &FormulaExpr) -> FormulaExpr {
    Nnf::default().go(expr, false)
}

/// The label lists of the diamonds in `expr` (unlabeled for `eventually`,
/// `until`, and `next`), each once.
fn diamond_labels(expr: &FormulaExpr, out: &mut Vec<Vec<Property>>) {
    let mut add = |ls: &[Property]| {
        if !out.iter().any(|o| o.as_slice() == ls) {
            out.push(ls.to_vec());
        }
    };
    match expr {
        FormulaExpr::Diamond(ls, e) | FormulaExpr::DiamondBox(ls, e) => {
            add(ls);
            diamond_labels(e, out);
        }
        FormulaExpr::Eventually(e) | FormulaExpr::Next(e) => {
            add(&[]);
            diamond_labels(e, out);
        }
        FormulaExpr::Until(a, b) => {
            add(&[]);
            diamond_labels(a, out);
            diamond_labels(b, out);
        }
        FormulaExpr::And(a, b) | FormulaExpr::Or(a, b) | FormulaExpr::Implies(a, b) => {
            diamond_labels(a, out);
            diamond_labels(b, out);
        }
        FormulaExpr::Not(e)
        | FormulaExpr::Paren(e)
        | FormulaExpr::Box(_, e)
        | FormulaExpr::Lfp(_, e)
        | FormulaExpr::Gfp(_, e)
        | FormulaExpr::Always(e) => diamond_labels(e, out),
        FormulaExpr::True | FormulaExpr::False | FormulaExpr::Prop(_) | FormulaExpr::Var(_) => {}
    }
}

#[derive(Default)]
struct Nnf {
    /// Fixed-point variables in scope, each with whether its binder was
    /// negated: `!lfp(X, φ)` is `gfp(X, !φ[!X/X])`, so an occurrence of `X`
    /// under a negated binder flips back.
    binders: Vec<(String, bool)>,
    fresh: usize,
}

impl Nnf {
    fn sub(&mut self, e: &FormulaExpr, neg: bool) -> Box<FormulaExpr> {
        Box::new(self.go(e, neg))
    }

    fn go(&mut self, expr: &FormulaExpr, neg: bool) -> FormulaExpr {
        use FormulaExpr as F;
        match expr {
            F::True | F::False => {
                if neg == matches!(expr, F::True) {
                    F::False
                } else {
                    F::True
                }
            }
            F::Prop(name) | F::Var(name) => {
                let flip = self
                    .binders
                    .iter()
                    .rev()
                    .find(|(v, _)| v == name)
                    .is_some_and(|(_, f)| *f);
                if neg != flip {
                    F::Not(Box::new(expr.clone()))
                } else {
                    expr.clone()
                }
            }
            F::Not(e) => self.go(e, !neg),
            F::Paren(e) => self.go(e, neg),
            F::And(l, r) if neg => F::Or(self.sub(l, true), self.sub(r, true)),
            F::And(l, r) => F::And(self.sub(l, false), self.sub(r, false)),
            F::Or(l, r) if neg => F::And(self.sub(l, true), self.sub(r, true)),
            F::Or(l, r) => F::Or(self.sub(l, false), self.sub(r, false)),
            F::Implies(l, r) if neg => F::And(self.sub(l, false), self.sub(r, true)),
            F::Implies(l, r) => F::Or(self.sub(l, true), self.sub(r, false)),
            F::Diamond(ps, e) if neg => F::Box(ps.clone(), self.sub(e, true)),
            F::Diamond(ps, e) => F::Diamond(ps.clone(), self.sub(e, false)),
            F::Box(ps, e) if neg => F::Diamond(ps.clone(), self.sub(e, true)),
            F::Box(ps, e) => F::Box(ps.clone(), self.sub(e, false)),
            F::DiamondBox(..) => self.go(&expr.expand_diamond_box(), neg),
            F::Eventually(e) if neg => F::Always(self.sub(e, true)),
            F::Eventually(e) => F::Eventually(self.sub(e, false)),
            F::Always(e) if neg => F::Eventually(self.sub(e, true)),
            F::Always(e) => F::Always(self.sub(e, false)),
            F::Next(e) if neg => F::Box(Vec::new(), self.sub(e, true)),
            F::Next(e) => F::Next(self.sub(e, false)),
            // !until(l, r) = gfp(X, !r & (!l | []X)). `#` keeps X apart from
            // every name a rule can write.
            F::Until(l, r) if neg => {
                let x = format!("#until{}", self.fresh);
                self.fresh += 1;
                let not_r = self.sub(r, true);
                let not_l = self.sub(l, true);
                let step = F::Box(Vec::new(), Box::new(F::Var(x.clone())));
                F::Gfp(
                    x,
                    Box::new(F::And(not_r, Box::new(F::Or(not_l, Box::new(step))))),
                )
            }
            F::Until(l, r) => F::Until(self.sub(l, false), self.sub(r, false)),
            F::Lfp(x, e) | F::Gfp(x, e) => {
                self.binders.push((x.clone(), neg));
                let body = self.sub(e, neg);
                self.binders.pop();
                if matches!(expr, F::Lfp(..)) != neg {
                    F::Lfp(x.clone(), body)
                } else {
                    F::Gfp(x.clone(), body)
                }
            }
        }
    }
}

/// Model checker for temporal modal formulas
pub struct ModelChecker {
    model: Model,
    version: TheoryVersion,
    registry: Option<Arc<dyn Registry + Send + Sync>>,
    state: Option<Arc<dyn StateView + Send + Sync>>,
    /// `(part index, transition index)` of edges the theory proved dead.
    dead: HashSet<(usize, usize)>,
    /// Under `V2`, edges state flow proves no run from `start` takes.
    never: HashSet<(usize, usize)>,
    /// Under `V2`, what holds on arrival at each node on every run from
    /// `start`. Edge matching reads it: a commit from the node meets it.
    facts: HashMap<String, Vec<Lit>>,
    /// The node a `V2` rule check is evaluated from.
    start: Option<String>,
    /// Under `V2`, the copy of the rule's node that takes the first step,
    /// and the node: a proposition naming the node holds at the copy too.
    first_step: Option<(String, String)>,
    /// Accepted state at the node a rule check starts from. `V2` state flow
    /// reads it, as what every run from there starts knowing, and so does
    /// diamond matching on the first step; later steps do not, since state
    /// changes along a run.
    anchor_state: Option<Arc<dyn StateView + Send + Sync>>,
    /// Fixed-point variables being evaluated, bound to their states.
    bound: Mutex<HashMap<String, Vec<State>>>,
}

impl ModelChecker {
    /// Create a new model checker for the given model.
    ///
    /// Uses `TheoryVersion::V0`: atoms are opaque, matching is structural,
    /// no edge is pruned. This is the behaviour every existing caller
    /// relies on.
    pub fn new(model: Model) -> Self {
        Self::with_theory(model, TheoryVersion::V0, None, None)
    }

    /// Model checker under a theory version with the standard registry and
    /// no state view.
    pub fn with_version(model: Model, version: TheoryVersion) -> Self {
        Self::with_theory(model, version, None, None)
    }

    /// Model checker under a theory version. `None` registry means the
    /// standard declarations; `None` state means nothing is known about
    /// accepted state (structural queries only).
    pub fn with_theory(
        model: Model,
        version: TheoryVersion,
        registry: Option<Box<dyn Registry + Send + Sync>>,
        state: Option<Box<dyn StateView + Send + Sync>>,
    ) -> Self {
        Self::with_shared(
            model,
            version,
            registry.map(Arc::from),
            state.map(Arc::from),
        )
    }

    fn with_shared(
        model: Model,
        version: TheoryVersion,
        registry: Option<Arc<dyn Registry + Send + Sync>>,
        state: Option<Arc<dyn StateView + Send + Sync>>,
    ) -> Self {
        let mut checker = Self {
            model,
            version,
            registry,
            state,
            dead: HashSet::new(),
            never: HashSet::new(),
            facts: HashMap::new(),
            start: None,
            first_step: None,
            anchor_state: None,
            bound: Mutex::new(HashMap::new()),
        };
        checker.dead = checker.compute_dead();
        checker
    }

    /// Under `V2`, rule checks start their state flow from `state`: the
    /// accepted state at the node they are checked at. Sound only when it
    /// is that state, as at `RULE`-add and on replay.
    pub fn with_anchor_state(mut self, state: Box<dyn StateView + Send + Sync>) -> Self {
        self.anchor_state = Some(Arc::from(state));
        self
    }

    /// A checker for another model with the same theory, registry, state
    /// view, anchor state and first-step copy.
    fn derived(&self, model: Model) -> ModelChecker {
        let mut checker = ModelChecker::with_shared(
            model,
            self.version,
            self.registry.clone(),
            self.state.clone(),
        );
        checker.anchor_state = self.anchor_state.clone();
        checker.first_step = self.first_step.clone();
        checker
    }

    pub fn theory_version(&self) -> TheoryVersion {
        self.version
    }

    fn registry(&self) -> &dyn Registry {
        match &self.registry {
            Some(r) => r.as_ref(),
            None => standard(),
        }
    }

    /// Under `V2`, the anchor state when `node` is the copy of the rule's
    /// node that takes the first step: that step is taken in exactly this
    /// state, so a diamond there is decided against it.
    fn anchor_step(&self, node: &str) -> Option<&(dyn StateView + Send + Sync)> {
        if self.version != TheoryVersion::V2 {
            return None;
        }
        let (copy, _) = self.first_step.as_ref()?;
        (copy == node).then_some(())?;
        self.anchor_state.as_deref()
    }

    fn theory(&self) -> Theory<'_> {
        let registry = self.registry();
        let state: &dyn StateView = match &self.state {
            Some(s) => s.as_ref(),
            None => &NoState,
        };
        Theory::new(self.version, registry, state)
    }

    /// Edges whose label set is provably unsatisfiable. Empty under `V0`.
    /// Dead edges are computed without state: they are dead in every
    /// state, which is what makes pruning them sound for a MODEL commit.
    fn compute_dead(&self) -> HashSet<(usize, usize)> {
        let mut dead = HashSet::new();
        if self.version == TheoryVersion::V0 {
            return dead;
        }
        let registry: &dyn Registry = match &self.registry {
            Some(r) => r.as_ref(),
            None => standard(),
        };
        let theory = Theory::new(self.version, registry, &NoState);
        for (pi, part) in self.model.parts.iter().enumerate() {
            for (ti, transition) in part.transitions.iter().enumerate() {
                if self
                    .edge_instances(transition)
                    .iter()
                    .all(|props| theory.consistent(props).tri == Tri::False)
                {
                    dead.insert((pi, ti));
                }
            }
        }
        dead
    }

    /// The label sets an edge stands for: itself, or with variables, one per
    /// assignment of the model's names plus fresh ones.
    fn edge_instances(&self, transition: &Transition) -> Vec<Vec<Property>> {
        if !vars::any_vars(&transition.properties) {
            return vec![transition.properties.clone()];
        }
        let props = vars::model_props(&self.model);
        let segments = vars::segments(props.iter().copied());
        let edge_vars: Vec<String> = vars::vars_of(&transition.properties).into_iter().collect();
        let names = vars::universe(&segments, edge_vars.len());
        let fill = Self::fill_segments(segments, &names, props.iter().copied());
        vars::assignments(&edge_vars, &names)
            .iter()
            .map(|env| {
                transition
                    .properties
                    .iter()
                    .flat_map(|p| vars::expand(p, env, &fill))
                    .collect()
            })
            .collect()
    }

    /// Segments to fill holes with: every mentioned one, and every name
    /// with every suffix a variable carries.
    fn fill_segments<'a>(
        mut segments: std::collections::BTreeSet<String>,
        names: &[String],
        props: impl IntoIterator<Item = &'a Property>,
    ) -> std::collections::BTreeSet<String> {
        let suffixes = vars::var_suffixes(props);
        for n in names {
            for suf in &suffixes {
                segments.insert(format!("{n}{suf}"));
            }
        }
        segments
    }

    /// With variables in the model or the formula: a checker for the model
    /// instantiated over every name it and the formula mention, plus one
    /// fresh name per variable, and the formula once per assignment of
    /// those names to its variables. `None` without variables.
    fn instances(&self, expr: &FormulaExpr) -> Option<(ModelChecker, Vec<FormulaExpr>)> {
        let fprops = vars::formula_props(expr);
        let mprops = vars::model_props(&self.model);
        if !vars::any_vars(fprops.iter().copied()) && !vars::any_vars(mprops.iter().copied()) {
            return None;
        }
        let rule_vars: Vec<String> = vars::vars_of(fprops.iter().copied()).into_iter().collect();
        let edge_vars = self
            .model
            .parts
            .iter()
            .flat_map(|p| p.transitions.iter())
            .chain(self.model.transitions.iter())
            .map(|t| vars::vars_of(&t.properties).len())
            .max()
            .unwrap_or(0);
        let segments = vars::segments(fprops.iter().copied().chain(mprops.iter().copied()));
        let names = vars::universe(&segments, rule_vars.len() + edge_vars);
        let fill = Self::fill_segments(
            segments,
            &names,
            fprops.iter().copied().chain(mprops.iter().copied()),
        );
        let mut ground = self.derived(vars::ground_model(&self.model, &names, &fill));
        if let Some(start) = &self.start {
            if !vars::any_vars(mprops.iter().copied()) {
                ground.scope_to(start);
            }
        }
        let formulas = vars::assignments(&rule_vars, &names)
            .iter()
            .map(|env| vars::substitute_formula(expr, env))
            .collect();
        Some((ground, formulas))
    }

    /// States satisfying the formula; with variables, every instance of it.
    /// A rule with a hole is not decided and satisfies nothing.
    fn satisfying(&self, expr: &FormulaExpr) -> Vec<State> {
        let expr = &negation_normal_form(expr);
        if vars::formula_props(expr).into_iter().any(vars::has_holes) {
            return Vec::new();
        }
        let Some((ground, formulas)) = self.instances(expr) else {
            return self.evaluate_formula(expr);
        };
        let mut out: Option<Vec<State>> = None;
        for f in &formulas {
            let states = ground.evaluate_formula(f);
            out = Some(match out {
                None => states,
                Some(prev) => ground.intersect_states(&prev, &states),
            });
        }
        out.unwrap_or_default()
    }

    fn part_index(&self, part: &Part) -> Option<usize> {
        self.model.parts.iter().position(|p| std::ptr::eq(p, part))
    }

    /// Transitions of a part that the theory has not proved dead, or, from
    /// a `V2` start node, never taken.
    fn live_transitions<'a>(&self, part: &'a Part) -> Vec<&'a Transition> {
        if self.dead.is_empty() && self.never.is_empty() {
            return part.transitions.iter().collect();
        }
        let pi = self.part_index(part);
        part.transitions
            .iter()
            .enumerate()
            .filter(|(ti, _)| {
                pi.is_none_or(|pi| {
                    !self.dead.contains(&(pi, *ti)) && !self.never.contains(&(pi, *ti))
                })
            })
            .map(|(_, t)| t)
            .collect()
    }

    /// Every dead edge, with the literals that kill it. Sorted by part,
    /// then by position in the part.
    pub fn dead_transitions(&self) -> Vec<DeadEdge> {
        let theory = Theory::new(
            self.version,
            match &self.registry {
                Some(r) => r.as_ref(),
                None => standard(),
            },
            &NoState,
        );
        let mut keys: Vec<&(usize, usize)> = self.dead.iter().collect();
        keys.sort();
        keys.into_iter()
            .map(|(pi, ti)| {
                let part = &self.model.parts[*pi];
                let t = &part.transitions[*ti];
                DeadEdge {
                    part_name: part.name.clone(),
                    from: t.from.clone(),
                    to: t.to.clone(),
                    properties: t.properties.clone(),
                    offending: self
                        .edge_instances(t)
                        .first()
                        .map(|props| theory.consistent(props).explain())
                        .unwrap_or_default(),
                }
            })
            .collect()
    }

    /// Edges no commit takes once the contract is under way from `initial`:
    /// the facts every way into their node carries contradict their labels
    /// (theory state flow, `theory::flow`). A lint under `V1`; under `V2` a
    /// rule check from a node drops the edges this finds from it. The
    /// offending literals include the carried facts. Empty under `V0`.
    pub fn dead_after_step(&self, initial: &[String]) -> Vec<DeadEdge> {
        self.flow_edges(self.flow_from(initial, None))
    }

    /// Live edges that a diamond of `formula` does not count because the
    /// theory can show neither that a commit takes the edge with the
    /// diamond's labels nor that none does (`Unknown`), each with those
    /// labels. Empty under `V0`.
    pub fn undecided_for(&self, formula: &Formula) -> Vec<(DeadEdge, Vec<Property>)> {
        if self.version == TheoryVersion::V0 {
            return Vec::new();
        }
        let mut labels: Vec<Vec<Property>> = Vec::new();
        diamond_labels(&negation_normal_form(&formula.expression), &mut labels);
        let theory = self.theory();
        let mut out = Vec::new();
        for part in &self.model.parts {
            for t in self.live_transitions(part) {
                for ls in &labels {
                    let undecided = self.edge_instances(t).into_iter().any(|mut with| {
                        with.extend(ls.iter().cloned());
                        theory.consistent(&with).tri == Tri::Unknown
                    });
                    if undecided {
                        let edge = DeadEdge {
                            part_name: part.name.clone(),
                            from: t.from.clone(),
                            to: t.to.clone(),
                            properties: t.properties.clone(),
                            offending: Vec::new(),
                        };
                        out.push((edge, ls.clone()));
                    }
                }
            }
        }
        out
    }

    fn flow_edges(&self, found: Vec<((usize, usize), Vec<String>)>) -> Vec<DeadEdge> {
        found
            .into_iter()
            .map(|((pi, ti), offending)| {
                let part = &self.model.parts[pi];
                let t = &part.transitions[ti];
                DeadEdge {
                    part_name: part.name.clone(),
                    from: t.from.clone(),
                    to: t.to.clone(),
                    properties: t.properties.clone(),
                    offending,
                }
            })
            .collect()
    }

    /// State flow over every part's edges from `initial`, where every run
    /// starts in `seed_state` (knowing nothing when `None`): the position of
    /// each edge dead after a step, and why.
    fn flow_from(
        &self,
        initial: &[String],
        seed_state: Option<&(dyn StateView + Send + Sync)>,
    ) -> Vec<((usize, usize), Vec<String>)> {
        if self.version == TheoryVersion::V0 {
            return Vec::new();
        }
        let (flow, at) = self.run_flow(initial, seed_state);
        flow.dead_after
            .into_iter()
            .map(|(i, why)| (at[i], why.iter().map(ToString::to_string).collect()))
            .collect()
    }

    /// The flow itself, and each flow edge's `(part, transition)` position.
    /// The seed is what `seed_state` says about every path an edge mentions
    /// (Lean: `stateFacts_seed`).
    fn run_flow(
        &self,
        initial: &[String],
        seed_state: Option<&(dyn StateView + Send + Sync)>,
    ) -> (Flow, Vec<(usize, usize)>) {
        let registry: &dyn Registry = match &self.registry {
            Some(r) => r.as_ref(),
            None => standard(),
        };
        let theory = Theory::new(self.version, registry, &NoState);
        let mut at = Vec::new();
        let mut edges = Vec::new();
        for (pi, part) in self.model.parts.iter().enumerate() {
            for (ti, t) in part.transitions.iter().enumerate() {
                let lits =
                    (!vars::any_vars(&t.properties)).then(|| theory.expand_all(&t.properties).0);
                edges.push(FlowEdge {
                    from: t.from.clone(),
                    to: t.to.clone(),
                    lits,
                });
                at.push((pi, ti));
            }
        }
        let seed = match seed_state {
            Some(state) => {
                let mut mentioned: Vec<Lit> = edges
                    .iter()
                    .filter_map(|e| e.lits.clone())
                    .flatten()
                    .collect();
                // An external predicate reads the key its first argument
                // names; knowing it is what lets a later step count on it.
                let keys: Vec<Lit> = mentioned
                    .iter()
                    .filter_map(|l| match &l.c {
                        Constraint::Opaque { name, args } if registry.external(name) => {
                            args.first().map(|k| {
                                Lit::pos(Constraint::Exists {
                                    path: crate::theory::sort::norm_path(k),
                                })
                            })
                        }
                        _ => None,
                    })
                    .collect();
                mentioned.extend(keys);
                Theory::new(self.version, registry, state).state_facts(&mentioned)
            }
            None => Vec::new(),
        };
        (flow_seeded(&edges, initial, &seed), at)
    }

    /// Under `V2`, the flow a rule check from `node` runs, from the anchor
    /// state when there is one. `None` otherwise, and for a model with
    /// variables: its instances stand for edges with names the flow does
    /// not see.
    fn v2_flow(&self, node: &str) -> Option<(Flow, Vec<(usize, usize)>)> {
        if self.version != TheoryVersion::V2
            || vars::any_vars(vars::model_props(&self.model).iter().copied())
        {
            return None;
        }
        Some(self.run_flow(&[node.to_string()], self.anchor_state.as_deref()))
    }

    /// Scope a `V2` rule check to `node`: drop the edges no run from there
    /// takes, and keep what is known at each node for edge matching.
    fn scope_to(&mut self, node: &str) {
        self.start = Some(node.to_string());
        if let Some((flow, at)) = self.v2_flow(node) {
            self.never = flow.dead_after.iter().map(|(i, _)| at[*i]).collect();
            self.facts = flow.facts.into_iter().collect();
        }
    }

    /// Under `V2`, the edges a rule check from `node` drops, with why.
    pub fn never_taken_from(&self, node: &str) -> Vec<DeadEdge> {
        let Some((flow, at)) = self.v2_flow(node) else {
            return Vec::new();
        };
        self.flow_edges(
            flow.dead_after
                .into_iter()
                .map(|(i, why)| (at[i], why.iter().map(ToString::to_string).collect()))
                .collect(),
        )
    }

    /// Runtime necessity: classify every outgoing edge of `node` (in every
    /// part that has it) against the state view. Under `V0`, or with no
    /// state view, every live edge is `Open`. A move is `Forced` when it is
    /// the only open edge out of `node` in any part: a commit may take any.
    pub fn classify_moves(&self, node: &str) -> Vec<Move> {
        let theory = self.theory();
        let mut out = Vec::new();
        for part in &self.model.parts {
            let edges: Vec<&Transition> = self
                .live_transitions(part)
                .into_iter()
                .filter(|t| t.from == node)
                .collect();
            let mut moves: Vec<Move> = edges
                .iter()
                .map(|t| {
                    // An edge with variables needs the names in state to
                    // classify; it is reported open.
                    let v = if vars::any_vars(&t.properties) {
                        crate::theory::Verdict::unknown()
                    } else {
                        theory.consistent(&t.properties)
                    };
                    let (status, offending) = if v.tri == Tri::False {
                        (MoveStatus::Blocked, v.explain())
                    } else {
                        (MoveStatus::Open, Vec::new())
                    };
                    Move {
                        part_name: part.name.clone(),
                        from: t.from.clone(),
                        to: t.to.clone(),
                        properties: t.properties.clone(),
                        status,
                        offending,
                    }
                })
                .collect();
            out.append(&mut moves);
        }
        if out.len() > 1 && out.iter().filter(|m| m.status == MoveStatus::Open).count() == 1 {
            for m in &mut out {
                if m.status == MoveStatus::Open {
                    m.status = MoveStatus::Forced;
                }
            }
        }
        out
    }

    /// Check a formula the way the network checks a rule on a new contract:
    /// on [`merged_model`], at every one of [`start_nodes`].
    pub fn check_formula(&self, formula: &Formula) -> ModelCheckResult {
        let mut result: Option<ModelCheckResult> = None;
        for start in start_nodes(&self.model) {
            let at = self.check_formula_at_state(formula, &start);
            match &mut result {
                None => result = Some(at),
                Some(r) => r.is_satisfied &= at.is_satisfied,
            }
        }
        result.expect("start_nodes is never empty")
    }

    /// Check if any witness node satisfies the formula (original behavior)
    pub fn check_formula_any_state(&self, formula: &Formula) -> ModelCheckResult {
        let satisfying_states = self.satisfying(&formula.expression);

        ModelCheckResult {
            formula: formula.clone(),
            satisfying_states: satisfying_states.clone(),
            is_satisfied: !satisfying_states.is_empty(),
        }
    }

    /// Check if a formula is satisfied starting from a specific witness node id
    ///
    /// Returns satisfied if the named witness node is among the nodes that satisfy the formula.
    /// The formula is evaluated on [`merged_model`], the graph commits move on, so on a model
    /// with several parts it must hold for the edges of every part out of the node.
    ///
    /// Under `V2` the check runs from `state_name`, knowing what the anchor
    /// state says there when one is set ([`Self::with_anchor_state`]) and
    /// nothing otherwise. It drops the edges no run from it takes, and
    /// matches edges against what every run knows at their node.
    pub fn check_formula_at_state(&self, formula: &Formula, state_name: &str) -> ModelCheckResult {
        if self.model.parts.len() > 1 || !self.model.transitions.is_empty() {
            return self
                .derived(merged_model(&self.model))
                .check_formula_at_state(formula, state_name);
        }
        if self.version == TheoryVersion::V2 && self.start.as_deref() != Some(state_name) {
            // The first step from the node is taken in the anchor state; a
            // return to the node is not. A copy of the node with its edges
            // out and none in keeps the seed to the first step.
            let first = format!("{state_name}#first");
            let mut unfolded = self.model.clone();
            let mut from_copy = false;
            if let Some(part) = unfolded.parts.first_mut() {
                let out: Vec<Transition> = part
                    .transitions
                    .iter()
                    .filter(|t| t.from == state_name)
                    .map(|t| Transition {
                        from: first.clone(),
                        ..t.clone()
                    })
                    .collect();
                from_copy = !out.is_empty();
                part.transitions.extend(out);
            }
            if !from_copy {
                let mut scoped = self.derived(self.model.clone());
                scoped.scope_to(state_name);
                return scoped.check_formula_at_state(formula, state_name);
            }
            let mut scoped = self.derived(unfolded);
            scoped.first_step = Some((first.clone(), state_name.to_string()));
            scoped.scope_to(&first);
            let mut result = scoped.check_formula_at_state(formula, &first);
            result.satisfying_states.retain(|s| s.node_name != first);
            return result;
        }
        let satisfying_states = self.satisfying(&formula.expression);

        // Check if any satisfying state has this node name
        let is_satisfied = satisfying_states.iter().any(|s| s.node_name == state_name);

        ModelCheckResult {
            formula: formula.clone(),
            satisfying_states,
            is_satisfied,
        }
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
                if let Some(states) = self.bound_states(name) {
                    return states;
                }
                // Witness nodes where the opaque node id matches the proposition.
                self.all_states()
                    .into_iter()
                    .filter(|s| {
                        s.node_name == *name
                            || self
                                .first_step
                                .as_ref()
                                .is_some_and(|(copy, node)| s.node_name == *copy && node == name)
                    })
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
            FormulaExpr::Paren(expr) => self.evaluate_formula(expr),
            FormulaExpr::Diamond(properties, expr) => self.evaluate_diamond(properties, expr),
            FormulaExpr::Box(properties, expr) => self.evaluate_box(properties, expr),
            FormulaExpr::DiamondBox(properties, expr) => {
                // [<action>] φ = [-action] false & <+action> φ
                // Expand and evaluate
                let expanded =
                    FormulaExpr::DiamondBox(properties.clone(), expr.clone()).expand_diamond_box();
                self.evaluate_formula(&expanded)
            }
            FormulaExpr::Eventually(expr) => self.evaluate_eventually(expr),
            FormulaExpr::Always(expr) => self.evaluate_always(expr),
            FormulaExpr::Until(left, right) => self.evaluate_until(left, right),
            FormulaExpr::Next(expr) => self.evaluate_next(expr),
            FormulaExpr::Var(name) => {
                if let Some(states) = self.bound_states(name) {
                    return states;
                }
                self.all_states()
                    .into_iter()
                    .filter(|s| s.node_name == *name)
                    .collect()
            }
            FormulaExpr::Lfp(var, expr) => self.evaluate_lfp(var, expr),
            FormulaExpr::Gfp(var, expr) => self.evaluate_gfp(var, expr),
        }
    }

    /// The edges an unlabeled diamond counts: `eventually`, `until` and
    /// `next` step only where some commit can.
    fn diamond_steps<'a>(&self, part: &'a Part) -> Vec<&'a Transition> {
        self.live_transitions(part)
            .into_iter()
            .filter(|t| self.transition_satisfies_properties(t, &[], false))
            .collect()
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
                for transition in self.diamond_steps(part) {
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
                    for transition in self.live_transitions(part) {
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
                for transition in self.diamond_steps(part) {
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
            for transition in self.diamond_steps(part) {
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
            let new_result = self.evaluate_bound(var, &result, expr);

            // Check if we've reached a fixed point
            changed =
                new_result.len() != result.len() || !new_result.iter().all(|s| result.contains(s));
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
            let new_result = self.evaluate_bound(var, &result, expr);

            // Intersect with current result (gfp is monotonically decreasing)
            let intersection = self.intersect_states(&result, &new_result);

            // Check if we've reached a fixed point
            changed = intersection.len() != result.len();
            result = intersection;
        }

        result
    }

    /// Evaluate `expr` with `var` bound to exactly `states`. A bound
    /// variable names states, part and node, not node names: on a model with
    /// several parts a node name would also pick the same name in every
    /// other part.
    fn evaluate_bound(&self, var: &str, states: &[State], expr: &FormulaExpr) -> Vec<State> {
        let outer = self.bind(var, Some(states.to_vec()));
        let result = self.evaluate_formula(expr);
        self.bind(var, outer);
        result
    }

    fn bind(&self, var: &str, states: Option<Vec<State>>) -> Option<Vec<State>> {
        let mut bound = self.bound.lock().unwrap_or_else(|e| e.into_inner());
        match states {
            Some(states) => bound.insert(var.to_string(), states),
            None => bound.remove(var),
        }
    }

    fn bound_states(&self, name: &str) -> Option<Vec<State>> {
        self.bound
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(name)
            .cloned()
    }

    /// Evaluate diamond operator: <properties> phi
    fn evaluate_diamond(&self, properties: &[Property], expr: &FormulaExpr) -> Vec<State> {
        let target_states = self.evaluate_formula(expr);
        let mut result = Vec::new();

        for part in &self.model.parts {
            for transition in self.live_transitions(part) {
                // Check if this transition has all the required properties
                if self.transition_satisfies_properties(transition, properties, false) {
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
                let transitions_with_properties = self
                    .get_transitions_from_node(part, &node)
                    .into_iter()
                    .filter(|t| self.transition_satisfies_properties(t, properties, true))
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
    ///
    /// Under a theory version above `V0`, two steps come first:
    /// - the edge's atoms entail the property → usable (`x>7` for `x>5`)
    /// - the edge's atoms plus the property are inconsistent → not usable
    ///
    /// and the structural rule above decides the rest. The edge is also not
    /// usable when its atoms and all the labels together are inconsistent:
    /// one commit has to meet every label at once. Under `V2` that includes
    /// what is known at the edge's node on every run from the rule's start
    /// (Lean: `dead_after_sound` on the edge's and the labels' literals).
    ///
    /// `whole_atom` is set for boxes: "mentions" then means the same predicate
    /// with the same arguments. A box has to range over every edge a commit
    /// could take, and a commit signed by Alice and Bob takes an edge that only
    /// names `+signed_by(bob)`; matching by name alone would skip that edge and
    /// accept a rule that such a commit breaks.
    fn transition_satisfies_properties(
        &self,
        transition: &Transition,
        properties: &[Property],
        whole_atom: bool,
    ) -> bool {
        let theory = if self.version == TheoryVersion::V0 {
            None
        } else {
            Some(self.theory())
        };
        if let (Some(theory), false) = (&theory, whole_atom) {
            let mut with = transition.properties.clone();
            with.extend(properties.iter().cloned());
            if let Some(anchor) = self.anchor_step(&transition.from) {
                return Theory::new(self.version, self.registry(), anchor)
                    .consistent(&with)
                    .tri
                    == Tri::True;
            }
            if theory.consistent(&with).tri != Tri::True {
                return false;
            }
            let known: &[Lit] = self.facts.get(&transition.from).map_or(&[], |f| f);
            let (mut lits, exact) = theory.expand_all(&with);
            if self.version == TheoryVersion::V2 && !theory.robust(&lits, known) {
                return false;
            }
            if known.is_empty() {
                return true;
            }
            lits.extend(known.iter().cloned());
            return theory.consistent_lits(&lits, exact).tri == Tri::True;
        }
        if let Some(theory) = &theory {
            let mut with = transition.properties.clone();
            with.extend(properties.iter().cloned());
            if theory.consistent(&with).tri == Tri::False {
                return false;
            }
            if let Some(known) = self.facts.get(&transition.from).filter(|f| !f.is_empty()) {
                let mut lits = theory.expand_all(&with).0;
                lits.extend(known.iter().cloned());
                if theory.consistent_lits(&lits, false).tri == Tri::False {
                    return false;
                }
            }
        }
        properties.iter().all(|property| {
            // Check if transition explicitly has this property
            let has_explicit = transition.properties.iter().any(|p| p == property);
            if has_explicit {
                return true;
            }

            if let Some(theory) = &theory {
                match theory.entails(&transition.properties, property) {
                    Tri::True => return true,
                    Tri::False | Tri::Unknown => {}
                }
                let mut with = transition.properties.clone();
                with.push(property.clone());
                if theory.consistent(&with).tri == Tri::False {
                    return false;
                }
            }

            // If transition doesn't mention this property at all, it's usable
            let property_name = &property.name;
            let mentions_property = transition.properties.iter().any(|p| {
                p.name == *property_name
                    && (!whole_atom
                        || p.get_predicate().map(|(_, args)| args)
                            == property.get_predicate().map(|(_, args)| args))
            });
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
        self.live_transitions(part)
            .into_iter()
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
        states1
            .iter()
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
        states1
            .iter()
            .filter(|s| !states2.contains(s))
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Formula, FormulaExpr, Model, Part, Property, PropertySign, Transition};

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

        let formula = Formula::new(
            "DiamondBlueTrue".to_string(),
            FormulaExpr::Diamond(
                vec![Property::new(PropertySign::Plus, "blue".to_string())],
                Box::new(FormulaExpr::True),
            ),
        );

        let result = checker.check_formula(&formula);
        assert!(result.is_satisfied);
        // n1 should satisfy <+blue> true because it has a transition to n2 with +blue
        assert!(result.satisfying_states.iter().any(|s| s.node_name == "n1"));
    }

    #[test]
    fn test_gfp_substitutes_parsed_prop_variable_references() {
        let mut model = Model::new("CommittedLoop".to_string());
        let mut part = Part::new("flow".to_string());
        let mut transition = Transition::new("q0".to_string(), "q0".to_string());
        transition.add_property(Property::new(PropertySign::Plus, "APPROVE".to_string()));
        part.add_transition(transition);
        model.add_part(part);
        let checker = ModelChecker::new(model);

        let formula = Formula::new(
            "Invariant".to_string(),
            FormulaExpr::Gfp(
                "X".to_string(),
                Box::new(FormulaExpr::And(
                    Box::new(FormulaExpr::DiamondBox(
                        vec![Property::new(PropertySign::Plus, "APPROVE".to_string())],
                        Box::new(FormulaExpr::True),
                    )),
                    Box::new(FormulaExpr::Box(
                        Vec::new(),
                        Box::new(FormulaExpr::Prop("X".to_string())),
                    )),
                )),
            ),
        );

        let result = checker.check_formula(&formula);

        assert!(result.is_satisfied);
        assert!(result.satisfying_states.iter().any(|s| s.node_name == "q0"));
    }

    #[test]
    fn test_gfp_accepts_unlabeled_committed_recursive_step() {
        let mut model = Model::new("CommittedUnlabeledLoop".to_string());
        let mut part = Part::new("flow".to_string());
        let mut transition = Transition::new("q0".to_string(), "q0".to_string());
        transition.add_property(Property::new(PropertySign::Plus, "APPROVE".to_string()));
        part.add_transition(transition);
        model.add_part(part);
        let checker = ModelChecker::new(model);

        let formula = Formula::new(
            "Invariant".to_string(),
            FormulaExpr::Gfp(
                "X".to_string(),
                Box::new(FormulaExpr::And(
                    Box::new(FormulaExpr::DiamondBox(
                        vec![Property::new(PropertySign::Plus, "APPROVE".to_string())],
                        Box::new(FormulaExpr::True),
                    )),
                    Box::new(FormulaExpr::DiamondBox(
                        Vec::new(),
                        Box::new(FormulaExpr::Prop("X".to_string())),
                    )),
                )),
            ),
        );

        let result = checker.check_formula(&formula);

        assert!(result.is_satisfied);
        assert!(result.satisfying_states.iter().any(|s| s.node_name == "q0"));
    }

    #[test]
    fn test_lfp_substitutes_parsed_prop_variable_references() {
        let model = create_test_model();
        let checker = ModelChecker::new(model);

        let formula = Formula::new(
            "Reachable".to_string(),
            FormulaExpr::Lfp(
                "X".to_string(),
                Box::new(FormulaExpr::Or(
                    Box::new(FormulaExpr::Prop("n3".to_string())),
                    Box::new(FormulaExpr::Diamond(
                        Vec::new(),
                        Box::new(FormulaExpr::Prop("X".to_string())),
                    )),
                )),
            ),
        );

        let result = checker.check_formula(&formula);

        assert!(result.is_satisfied);
        assert!(result.satisfying_states.iter().any(|s| s.node_name == "n1"));
        assert!(result.satisfying_states.iter().any(|s| s.node_name == "n2"));
        assert!(result.satisfying_states.iter().any(|s| s.node_name == "n3"));
    }

    /// A label-free formula whose fixed-point variables occur only
    /// positively. `binders` holds each variable in scope with the polarity
    /// it was bound at.
    fn random_formula(
        next: &mut impl FnMut() -> u64,
        depth: u32,
        pos: bool,
        binders: &mut Vec<(String, bool)>,
    ) -> FormulaExpr {
        use FormulaExpr as F;
        let node = |n: u64| F::Prop(format!("q{}", n % 4));
        if depth == 0 {
            return match next() % 4 {
                0 => F::True,
                1 => F::False,
                2 => node(next()),
                _ => match binders.iter().rev().find(|(_, p)| *p == pos) {
                    Some((v, _)) => F::Var(v.clone()),
                    None => node(next()),
                },
            };
        }
        let pick = next() % 14;
        let least = next().is_multiple_of(2);
        if pick >= 12 {
            return random_formula(next, 0, pos, binders);
        }
        let mut sub = |pos: bool, binders: &mut Vec<(String, bool)>| {
            Box::new(random_formula(next, depth - 1, pos, binders))
        };
        match pick {
            0 => F::Not(sub(!pos, binders)),
            1 => F::And(sub(pos, binders), sub(pos, binders)),
            2 => F::Or(sub(pos, binders), sub(pos, binders)),
            3 => F::Implies(sub(!pos, binders), sub(pos, binders)),
            4 => F::Diamond(Vec::new(), sub(pos, binders)),
            5 => F::Box(Vec::new(), sub(pos, binders)),
            6 => F::Eventually(sub(pos, binders)),
            7 => F::Always(sub(pos, binders)),
            8 => F::Until(sub(pos, binders), sub(pos, binders)),
            9 => F::Next(sub(pos, binders)),
            _ => {
                let x = format!("X{}", binders.len());
                binders.push((x.clone(), pos));
                let body = sub(pos, binders);
                binders.pop();
                if least {
                    F::Lfp(x, body)
                } else {
                    F::Gfp(x, body)
                }
            }
        }
    }

    /// With no labels edge matching is exact, so pushing negation inward
    /// must not change a formula's states, and a negated formula must hold
    /// exactly where the formula does not.
    #[test]
    fn negation_normal_form_keeps_meaning_where_matching_is_exact() {
        let mut seed: u64 = 0x2545_f491_4f6c_dd1d;
        let mut next = move || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };
        let names = |states: Vec<State>| {
            let mut v: Vec<String> = states.into_iter().map(|s| s.node_name).collect();
            v.sort();
            v.dedup();
            v
        };
        for round in 0..3000 {
            let mut part = Part::new("p".to_string());
            for _ in 0..1 + next() % 6 {
                part.add_transition(Transition::new(
                    format!("q{}", next() % 4),
                    format!("q{}", next() % 4),
                ));
            }
            let mut model = Model::new("M".to_string());
            model.add_part(part);
            let checker = ModelChecker::new(model);
            let f = random_formula(&mut next, 4, true, &mut Vec::new());
            let direct = names(checker.evaluate_formula(&f));
            assert_eq!(
                names(checker.satisfying(&f)),
                direct,
                "round {round}: {f:?}"
            );
            let not = FormulaExpr::Not(Box::new(f.clone()));
            let all = names(checker.all_states());
            let complement: Vec<String> = all
                .iter()
                .filter(|n| !direct.contains(n))
                .cloned()
                .collect();
            assert_eq!(
                names(checker.satisfying(&not)),
                complement,
                "round {round}: {f:?}"
            );
        }
    }
}
