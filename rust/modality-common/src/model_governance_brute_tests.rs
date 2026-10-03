//! The rule checker against brute force. Small models and rules over a
//! fixed vocabulary are checked the way governance checks a posted rule
//! (`validate_anchored_rule` from `q0`, with the accepted state there), and
//! compared with an oracle that enumerates every commit over the
//! vocabulary and follows accepted state from commit to commit, judging
//! each label with `predicate_holds`.
//!
//! The vocabulary is complete for its atoms: every accepted state and
//! commit agrees on every atom, and on the next state up to the atoms,
//! with one the oracle enumerates. So the oracle is exact, and an accepted
//! rule the oracle refutes is a rule no model run meets.

use super::*;
use modality_lang::ast::FormulaExpr as F;
use modality_lang::PropertySign;
use serde_json::json;

/// `(label text, how it appears in rules and models)`.
const ATOMS: &[&str] = &[
    "POST",
    "DELETE",
    "modifies(/a.bool)",
    "modifies(/x.num)",
    "modifies(/m)",
    "post_to_path(/x.num)",
    "sets(/x.num,\"4\")",
    "bool_true(/a.bool)",
    "bool_false(/a.bool)",
    "state_exists(/a.bool)",
    "num_gt(/x.num,\"3\")",
    "num_lt(/x.num,\"3\")",
    "num_eq(/x.num,\"2\")",
    "state_exists(/x.num)",
    "signed_by(/m/alice.id)",
    "signed_by(/m/bob.id)",
    "any_signed(/m)",
    "all_signed(/m)",
    "threshold(\"2\",/m)",
    "text_eq(/m/alice.id,\"KA\")",
];

const A_VALUES: &[Option<fn() -> Value>] = &[
    None,
    Some(|| json!(true)),
    Some(|| json!(false)),
    Some(|| json!("s")),
];
const X_VALUES: &[Option<fn() -> Value>] = &[
    None,
    Some(|| json!(1)),
    Some(|| json!(2)),
    Some(|| json!(3)),
    Some(|| json!(4)),
    Some(|| json!("s")),
];
const ALICE_VALUES: &[Option<fn() -> Value>] = &[None, Some(|| json!("KA")), Some(|| json!("KB"))];
const BOB_VALUES: &[Option<fn() -> Value>] = &[None, Some(|| json!("KB"))];

/// Every accepted state over the vocabulary's paths.
fn states() -> Vec<HashMap<String, Value>> {
    let mut out = Vec::new();
    for a in A_VALUES {
        for x in X_VALUES {
            for alice in ALICE_VALUES {
                for bob in BOB_VALUES {
                    let mut s = HashMap::new();
                    for (k, v) in [
                        ("a.bool", a),
                        ("x.num", x),
                        ("m/alice.id", alice),
                        ("m/bob.id", bob),
                    ] {
                        if let Some(v) = v {
                            s.insert(k.to_string(), v());
                        }
                    }
                    out.push(s);
                }
            }
        }
    }
    out
}

/// Every commit: per path nothing, a post of each value, or a delete; and
/// every set of signers.
fn commits() -> Vec<CommitFile> {
    fn choices(path: &str, values: &[Option<fn() -> Value>]) -> Vec<Option<(String, Value)>> {
        let mut out = vec![None, Some(("delete".to_string(), Value::Null))];
        out.extend(
            values
                .iter()
                .flatten()
                .map(|v| Some(("post".to_string(), v()))),
        );
        out.into_iter()
            .map(|c| c.map(|(m, v)| (format!("{m} /{path}"), v)))
            .collect()
    }
    let per_path = [
        choices("a.bool", A_VALUES),
        choices("x.num", X_VALUES),
        choices("m/alice.id", ALICE_VALUES),
        choices("m/bob.id", BOB_VALUES),
    ];
    let mut bodies: Vec<Vec<(String, Value)>> = vec![Vec::new()];
    for options in &per_path {
        bodies = bodies
            .into_iter()
            .flat_map(|body| {
                options.iter().map(move |o| {
                    let mut b = body.clone();
                    b.extend(o.clone());
                    b
                })
            })
            .collect();
    }
    let mut out = Vec::new();
    for body in &bodies {
        for signers in [&[][..], &["KA"], &["KB"], &["KA", "KB"]] {
            let mut c = CommitFile::new();
            for (action, value) in body {
                let (method, path) = action.split_once(' ').unwrap();
                c.add_action(method.to_string(), Some(path.to_string()), value.clone());
            }
            let sigs: serde_json::Map<String, Value> = signers
                .iter()
                .map(|k| (k.to_string(), json!("sig")))
                .collect();
            c.head.signatures = Some(Value::Object(sigs));
            out.push(c);
        }
    }
    out
}

fn atom_property(atom: &str, sign: PropertySign) -> Property {
    let text = format!(
        "{}{atom}",
        if sign == PropertySign::Plus { '+' } else { '-' }
    );
    let model = format!("model M {{\n  part p {{\n    q0 --> q0: {text}\n  }}\n}}");
    let m = modality_lang::parse_all_models_content_lalrpop(&model)
        .unwrap_or_else(|e| panic!("{text}: {e}"))
        .remove(0);
    m.parts[0].transitions[0].properties[0].clone()
}

/// What the oracle knows: every state, and from each state the distinct
/// (atoms that hold, next state) pairs over every commit.
struct World {
    states: Vec<HashMap<String, Value>>,
    moves: Vec<Vec<(u32, usize)>>,
}

fn state_key(s: &HashMap<String, Value>) -> String {
    let mut keys: Vec<_> = s.iter().map(|(k, v)| format!("{k}={v}")).collect();
    keys.sort();
    keys.join(";")
}

fn world() -> World {
    let states = states();
    let index: HashMap<String, usize> = states
        .iter()
        .enumerate()
        .map(|(i, s)| (state_key(s), i))
        .collect();
    let atoms = atoms();
    let commits = commits();
    let moves = states
        .iter()
        .map(|s| {
            let mut seen = std::collections::BTreeSet::new();
            for c in &commits {
                let facts = CommitFacts::from_commit(c, s).under(TheoryVersion::V2);
                let mut mask = 0u32;
                for (i, p) in atoms.iter().enumerate() {
                    if facts.predicate_holds(p) {
                        mask |= 1 << i;
                    }
                }
                let mut next = s.clone();
                apply_commit_to_state(c, &mut next);
                seen.insert((mask, index[&state_key(&next)]));
            }
            seen.into_iter().collect()
        })
        .collect();
    World { states, moves }
}

type Label = (usize, bool);

struct Graph {
    nodes: Vec<String>,
    edges: Vec<(usize, usize, Vec<Label>)>,
}

fn holds_all(labels: &[Label], mask: u32) -> bool {
    labels.iter().all(|(i, pos)| (mask >> i & 1 == 1) == *pos)
}

fn atoms() -> &'static [Property] {
    static ATOM_PROPS: std::sync::OnceLock<Vec<Property>> = std::sync::OnceLock::new();
    ATOM_PROPS.get_or_init(|| {
        ATOMS
            .iter()
            .map(|a| atom_property(a, PropertySign::Plus))
            .collect()
    })
}

fn label_of(p: &Property) -> Label {
    let key = |q: &Property| (q.name.clone(), q.is_static(), predicate_args(q));
    let i = atoms()
        .iter()
        .position(|a| key(a) == key(p))
        .unwrap_or_else(|| panic!("not in the vocabulary: {p:?}"));
    (i, p.sign == PropertySign::Plus)
}

/// The set of (node, state) where `e` holds, over every commit.
fn eval(e: &F, g: &Graph, w: &World, env: &mut Vec<(String, Vec<bool>)>) -> Vec<bool> {
    let n = g.nodes.len() * w.states.len();
    let at = |node: usize, s: usize| node * w.states.len() + s;
    let step = |labels: &[Label], phi: &[bool], all: bool| -> Vec<bool> {
        let mut out = vec![all; n];
        for node in 0..g.nodes.len() {
            for s in 0..w.states.len() {
                let mut v = all;
                'moves: for &(mask, next) in &w.moves[s] {
                    if !holds_all(labels, mask) {
                        continue;
                    }
                    for (from, to, lab) in &g.edges {
                        if *from == node && holds_all(lab, mask) {
                            let ok = phi[at(*to, next)];
                            if all && !ok {
                                v = false;
                                break 'moves;
                            }
                            if !all && ok {
                                v = true;
                                break 'moves;
                            }
                        }
                    }
                }
                out[at(node, s)] = v;
            }
        }
        out
    };
    let labels = |ps: &[Property]| ps.iter().map(label_of).collect::<Vec<_>>();
    let fix = |var: &str, body: &F, start: bool, env: &mut Vec<(String, Vec<bool>)>| {
        let mut cur = vec![start; n];
        loop {
            env.push((var.to_string(), cur.clone()));
            let next = eval(body, g, w, env);
            env.pop();
            if next == cur {
                return cur;
            }
            cur = next;
        }
    };
    let zip = |a: Vec<bool>, b: Vec<bool>, f: fn(bool, bool) -> bool| {
        a.into_iter().zip(b).map(|(x, y)| f(x, y)).collect()
    };
    match e {
        F::True => vec![true; n],
        F::False => vec![false; n],
        F::Paren(a) => eval(a, g, w, env),
        F::Not(a) => eval(a, g, w, env).into_iter().map(|x| !x).collect(),
        F::And(a, b) => {
            let (x, y) = (eval(a, g, w, env), eval(b, g, w, env));
            zip(x, y, |x, y| x && y)
        }
        F::Or(a, b) => {
            let (x, y) = (eval(a, g, w, env), eval(b, g, w, env));
            zip(x, y, |x, y| x || y)
        }
        F::Implies(a, b) => {
            let (x, y) = (eval(a, g, w, env), eval(b, g, w, env));
            zip(x, y, |x, y| !x || y)
        }
        F::Box(ps, a) => step(&labels(ps), &eval(a, g, w, env), true),
        F::Diamond(ps, a) => step(&labels(ps), &eval(a, g, w, env), false),
        F::Next(a) => step(&[], &eval(a, g, w, env), false),
        F::DiamondBox(ps, a) => {
            let ls = labels(ps);
            let mut v = step(&ls, &eval(a, g, w, env), false);
            for (i, pos) in &ls {
                let refused = step(&[(*i, !pos)], &vec![false; n], true);
                v = zip(v, refused, |x, y| x && y);
            }
            v
        }
        F::Var(x) => env
            .iter()
            .rev()
            .find(|(v, _)| v == x)
            .map(|(_, s)| s.clone())
            .unwrap_or_else(|| panic!("free variable {x}")),
        F::Lfp(x, body) => fix(x, body, false, env),
        F::Gfp(x, body) => fix(x, body, true, env),
        F::Eventually(a) => {
            let body = F::Or(
                a.clone(),
                Box::new(F::Diamond(vec![], Box::new(F::Var("#e".into())))),
            );
            fix("#e", &body, false, env)
        }
        F::Always(a) => {
            let body = F::And(
                a.clone(),
                Box::new(F::Box(vec![], Box::new(F::Var("#a".into())))),
            );
            fix("#a", &body, true, env)
        }
        F::Until(a, b) => {
            let body = F::Or(
                b.clone(),
                Box::new(F::And(
                    a.clone(),
                    Box::new(F::Diamond(vec![], Box::new(F::Var("#u".into())))),
                )),
            );
            fix("#u", &body, false, env)
        }
        F::Prop(p) => panic!("node proposition {p}"),
    }
}

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

fn random_labels(rng: &mut Rng, max: usize) -> String {
    (0..rng.below(max + 1))
        .map(|_| {
            let sign = if rng.below(2) == 0 { '+' } else { '-' };
            format!("{sign}{}", ATOMS[rng.below(ATOMS.len())])
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn random_rule(rng: &mut Rng, depth: usize) -> String {
    let leaf = |rng: &mut Rng| if rng.below(2) == 0 { "true" } else { "false" }.to_string();
    if depth == 0 {
        return leaf(rng);
    }
    let sub = |rng: &mut Rng| random_rule(rng, depth - 1);
    match rng.below(10) {
        0 => leaf(rng),
        1 | 2 => format!("[{}] {}", random_labels(rng, 2), sub(rng)),
        3 | 4 => format!("<{}> {}", random_labels(rng, 2), sub(rng)),
        5 => {
            let l = random_labels(rng, 2);
            let l = if l.is_empty() { "+POST".to_string() } else { l };
            format!("[<{l}>] {}", sub(rng))
        }
        6 => format!("({} & {})", sub(rng), sub(rng)),
        7 => format!("({} | {})", sub(rng), sub(rng)),
        8 => format!("always({})", sub(rng)),
        _ => format!("eventually({})", sub(rng)),
    }
}

fn random_model(rng: &mut Rng) -> (String, Graph) {
    let nodes = 1 + rng.below(3);
    let mut text = String::new();
    let mut edges = Vec::new();
    for i in 0..1 + rng.below(4) {
        let from = if i == 0 { 0 } else { rng.below(nodes) };
        let to = rng.below(nodes);
        let mut labels = random_labels(rng, 3);
        if labels.is_empty() {
            labels = "+POST".to_string();
        }
        text.push_str(&format!("    q{from} --> q{to}: {labels}\n"));
        edges.push((from, to, labels));
    }
    let model = format!("model M {{\n  part p {{\n{text}  }}\n}}");
    let parsed = modality_lang::parse_all_models_content_lalrpop(&model)
        .unwrap()
        .remove(0);
    let graph = Graph {
        nodes: (0..nodes).map(|i| format!("q{i}")).collect(),
        edges: parsed.parts[0]
            .transitions
            .iter()
            .map(|t| {
                let idx = |n: &str| n[1..].parse::<usize>().unwrap();
                (
                    idx(&t.from),
                    idx(&t.to),
                    t.properties.iter().map(label_of).collect(),
                )
            })
            .collect(),
    };
    (model, graph)
}

fn accepted(
    model: &Model,
    formula: &Formula,
    theory: TheoryVersion,
    state: &HashMap<String, Value>,
) -> bool {
    let rule = AnchoredRule {
        formula: formula.clone(),
        formula_source: String::new(),
        anchor_commit: 0,
        anchor_states: ["q0".to_string()].into(),
    };
    validate_anchored_rule(model, &rule, theory, state).is_ok()
}

/// Under `V2`, a rule governance accepts from `q0` in accepted state `s`
/// holds there: every run of commits from `(q0, s)` meets it.
#[test]
fn v2_accepts_only_rules_every_run_meets() {
    let w = world();
    let rounds: u32 = std::env::var("MODALITY_BRUTE_ROUNDS")
        .ok()
        .and_then(|r| r.parse().ok())
        .unwrap_or(400);
    let mut rng = Rng(0x000B_207E_F0CE_0001);
    let (mut accepts, mut refused_true, mut failures) = (0, 0, Vec::new());
    for round in 0..rounds {
        let (model_text, graph) = random_model(&mut rng);
        let model = modality_lang::parse_content_lalrpop(&model_text).unwrap();
        let rule = random_rule(&mut rng, 3);
        let Ok(mut formulas) =
            modality_lang::parse_all_formulas_content_lalrpop(&format!("formula r {{ {rule} }}"))
        else {
            continue;
        };
        let formula = formulas.remove(0);
        let truth = eval(&formula.expression, &graph, &w, &mut Vec::new());
        let s = rng.below(w.states.len());
        let holds = truth[s];
        if accepted(&model, &formula, TheoryVersion::V2, &w.states[s]) {
            accepts += 1;
            if !holds {
                failures.push(format!(
                    "round {round}: accepted but a run breaks it\nrule: {rule}\nstate: {:?}\n{model_text}",
                    w.states[s]
                ));
            }
        } else if holds {
            refused_true += 1;
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {accepts} accepted rules fail:\n{}",
        failures.len(),
        failures
            .iter()
            .take(5)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n\n")
    );
    assert!(accepts > rounds as usize / 10, "accepted {accepts}");
    eprintln!("accepted {accepts}; refused though true {refused_true}");
}
