//! Grades a candidate rule against a case. Every verdict comes from the
//! validator a node runs (`validate_pending_commit_with_theory`), under the
//! fixture's theory: a genesis commit carries the model, the state and the
//! rules, so the rules are checked from the node that commit reaches, and
//! later commits are allowed or refused by the model.

use crate::case::{Case, Fixture, Reading, Step, Trace};
use crate::model_text;
use anyhow::Result;
use modality_common::contract_store::CommitFile;
use modality_common::model_governance::{validate_pending_commit_with_theory, TheoryActivation};
use modality_lang::{
    lint_formula, parse_all_formulas_content_lalrpop, parse_all_models_content_lalrpop, Formula,
    FormulaExpr, FormulaLintOptions, LintSeverity, Property, PropertySource, TheoryVersion,
};
use serde::Serialize;
use serde_json::{json, Value};

/// Commit methods a static label can name (`+POST`, `-RULE`, ...).
pub const METHOD_LABELS: &[&str] = &[
    "POST", "RULE", "MODEL", "CREATE", "SEND", "RECV", "INVOKE", "REPOST", "GENESIS",
];

/// Predicates the commit evaluator reads. A name outside this list (and
/// outside the fixture's own model) is invented.
pub const EVALUATED_PREDICATES: &[&str] = &[
    "signed_by",
    "any_signed",
    "all_signed",
    "threshold",
    "modifies",
    "post_to_path",
    "post_to",
    "sets",
    "has_property",
    "state_exists",
    "text_eq",
    "text_contains",
    "text_starts_with",
    "text_ends_with",
    "amount_in_range",
    "num_eq",
    "num_gt",
    "num_gte",
    "num_lt",
    "num_lte",
    "bool_true",
    "bool_false",
    "sent_eq",
    "sent_lte",
    "sent_to",
    "posts_own_key",
    "emitted_by",
    "pays_senders",
    "keeps_product_per_share",
    "pays_memo_min",
    "mined_headers",
    "sets_from",
    "tracks",
    "keeps_product",
    "oracle_attests",
];

/// Cap on G5 probe models per reading.
pub const DEFAULT_PROBE_CAP: usize = 80;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Pass,
    Fail,
    Skip,
}

#[derive(Debug, Clone, Serialize)]
pub struct Check {
    pub grade: &'static str,
    pub status: Status,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RuleGrade {
    pub checks: Vec<Check>,
    /// G0–G4 pass.
    pub reasonable: bool,
    /// Reasonable, and G5 passes.
    pub exact: bool,
}

impl RuleGrade {
    pub fn status(&self, grade: &str) -> Status {
        self.checks
            .iter()
            .find(|c| c.grade == grade)
            .map(|c| c.status)
            .unwrap_or(Status::Skip)
    }

    /// `G0✓ G1✓ G2✓ G3✗ G4✓ G5✗ G6·`
    pub fn marks(&self) -> String {
        self.checks
            .iter()
            .map(|c| {
                let m = match c.status {
                    Status::Pass => "✓",
                    Status::Fail => "✗",
                    Status::Skip => "·",
                };
                format!("{}{m}", c.grade)
            })
            .collect::<Vec<_>>()
            .join(" ")
    }
}

pub struct Grader<'a> {
    pub fixture: &'a Fixture,
    pub theory: TheoryVersion,
    pub probe_cap: usize,
}

impl<'a> Grader<'a> {
    pub fn new(fixture: &'a Fixture) -> Result<Self> {
        Ok(Self {
            theory: fixture.theory()?,
            fixture,
            probe_cap: DEFAULT_PROBE_CAP,
        })
    }

    fn validate(&self, accepted: &[CommitFile], pending: &CommitFile) -> Result<(), String> {
        validate_pending_commit_with_theory(
            "",
            accepted,
            pending,
            None,
            None,
            None,
            TheoryActivation::always(self.theory),
        )
        .map_err(|e| short(&e.to_string()))
    }

    fn genesis(&self, model: &str, extra: &[&str]) -> CommitFile {
        let mut c = CommitFile::new();
        c.add_action(
            "model".into(),
            Some("/model/default.modality".into()),
            json!(model),
        );
        for (path, value) in &self.fixture.state {
            c.add_action("post".into(), Some(path.clone()), value.clone());
        }
        let rules = self
            .fixture
            .rules
            .iter()
            .map(String::as_str)
            .chain(extra.iter().copied());
        for (i, formula) in rules.enumerate() {
            c.add_action(
                "rule".into(),
                Some(format!("/rules/r{i}.modality")),
                json!(rule_file(formula)),
            );
        }
        c
    }

    /// Does a contract with this model, the fixture's state and rules, and
    /// `extra` rules get past genesis?
    pub fn admits(&self, model: &str, extra: &[&str]) -> Result<(), String> {
        self.validate(&[], &self.genesis(model, extra))
    }

    /// Genesis, then every step of `trace`, each accepted.
    pub fn replay(&self, model: &str, extra: &[&str], trace: &Trace) -> Result<(), String> {
        let mut accepted = vec![self.genesis(model, extra)];
        self.validate(&[], &accepted[0])
            .map_err(|e| format!("genesis refused: {e}"))?;
        for (i, step) in trace.all_steps().iter().enumerate() {
            let commit = step_commit(step);
            self.validate(&accepted, &commit)
                .map_err(|e| format!("step {} refused: {e}", i + 1))?;
            accepted.push(commit);
        }
        Ok(())
    }

    pub fn leak_model(&self, reading: &Reading, trace: &Trace) -> Result<String, String> {
        if let Some(model) = &trace.leak_model {
            return Ok(model.clone());
        }
        let leak = trace.leak.as_deref().unwrap_or_default();
        model_text::with_edges(&reading.witness, leak).ok_or_else(|| {
            format!(
                "forbid `{}`: the witness has no edge to add the leak after",
                trace.name
            )
        })
    }

    pub fn grade_rule(&self, reading: &Reading, formula: &str, witness: Option<&str>) -> RuleGrade {
        let mut checks = Vec::new();
        let formula = formula.trim();

        // G0 well-formed
        let parsed = parse_formula(formula);
        checks.push(match &parsed {
            Ok(_) => pass("G0"),
            Err(e) => fail("G0", vec![e.clone()]),
        });
        let Ok(parsed) = parsed else {
            for g in ["G1", "G2", "G3", "G4", "G5", "G6"] {
                checks.push(skip(g));
            }
            return finish(checks);
        };

        // G1 clean
        let witness_model =
            witness.and_then(|w| parse_all_models_content_lalrpop(w).ok()?.into_iter().next());
        let mut notes: Vec<String> = lint_formula(&parsed, &FormulaLintOptions { witness_model })
            .into_iter()
            .filter(|d| d.severity == LintSeverity::Warning)
            .map(|d| format!("lint: {}", d.message))
            .collect();
        notes.extend(self.vocabulary_problems(&parsed));
        checks.push(verdict("G1", notes));

        let extra = [formula];

        // G2 fits
        let mut tried = Vec::new();
        let mut fits = None;
        if let Some(w) = witness {
            match self.admits(w, &extra) {
                Ok(()) => fits = Some("the answer's witness"),
                Err(e) => tried.push(format!("answer's witness refused: {e}")),
            }
        }
        if fits.is_none() {
            match self.admits(&reading.witness, &extra) {
                Ok(()) => fits = Some("the reference witness"),
                Err(e) => tried.push(format!("reference witness refused: {e}")),
            }
        }
        checks.push(match fits {
            Some(which) => Check {
                grade: "G2",
                status: Status::Pass,
                notes: vec![format!("met with the contract's rules by {which}")],
            },
            None => fail("G2", tried),
        });

        // G3 not too strong
        let mut notes = Vec::new();
        match self.admits(&reading.witness, &extra) {
            Err(e) => notes.push(format!(
                "refuses the reference witness, which only allows what the request allows: {e}"
            )),
            Ok(()) => {
                for t in &reading.allow {
                    if let Err(e) = self.replay(&reading.witness, &extra, t) {
                        notes.push(format!("allowed move `{}` refused: {e}", t.name));
                    }
                }
            }
        }
        checks.push(verdict("G3", notes));

        // G4 not too weak
        let labels: Vec<String> = properties(&parsed.expression)
            .into_iter()
            .filter(|p| p.is_static())
            .map(|p| p.name)
            .collect();
        let mut notes = Vec::new();
        for t in &reading.forbid {
            match self.leak_model(reading, t) {
                Err(e) => notes.push(e),
                Ok(leak) => {
                    if self.admits(&leak, &extra).is_ok() {
                        notes.push(format!(
                            "accepts a model that lets the forbidden move `{}` through",
                            t.name
                        ));
                    } else if let Some((label, _)) = self
                        .label_dodging_leaks(reading, t, &labels)
                        .into_iter()
                        .find(|(_, m)| self.admits(m, &extra).is_ok())
                    {
                        notes.push(format!(
                            "accepts a model that lets the forbidden move `{}` through on an edge marked `-{label}`; the move is not a {label} commit, so the rule never reaches it",
                            t.name
                        ));
                    }
                }
            }
        }
        checks.push(verdict("G4", notes));

        // G5 exact
        checks.push(self.agreement(reading, formula));

        // G6 own witness
        checks.push(match witness {
            None => skip("G6"),
            Some(w) => {
                let mut notes = Vec::new();
                match self.admits(w, &extra) {
                    Err(e) => {
                        notes.push(format!("the answer's witness does not meet the rule: {e}"))
                    }
                    Ok(()) => {
                        for t in &reading.allow {
                            if let Err(e) = self.replay(w, &extra, t) {
                                notes.push(format!(
                                    "the answer's witness refuses allowed move `{}`: {e}",
                                    t.name
                                ));
                            }
                        }
                    }
                }
                verdict("G6", notes)
            }
        });

        finish(checks)
    }

    /// G5: the candidate and the reference accept and refuse the same probe
    /// models (leaks and small edits of the witness).
    fn agreement(&self, reading: &Reading, formula: &str) -> Check {
        if squash(formula) == squash(&reading.formula) {
            return Check {
                grade: "G5",
                status: Status::Pass,
                notes: vec!["same text as the reference".into()],
            };
        }
        let mut probes: Vec<model_text::Probe> = reading
            .forbid
            .iter()
            .filter_map(|t| {
                self.leak_model(reading, t).ok().map(|m| model_text::Probe {
                    label: format!("leak for `{}`", t.name),
                    model: m,
                })
            })
            .collect();
        probes.push(model_text::Probe {
            label: "reference witness".into(),
            model: reading.witness.clone(),
        });
        probes.extend(model_text::mutations(
            &reading.witness,
            &self.signer_paths(),
            self.probe_cap,
        ));

        let mut compared = 0;
        let mut notes = Vec::new();
        for p in &probes {
            if self.admits(&p.model, &[]).is_err() {
                continue;
            }
            compared += 1;
            let r = self.admits(&p.model, &[reading.formula.as_str()]).is_ok();
            let a = self.admits(&p.model, &[formula]).is_ok();
            if r != a {
                let side = if a {
                    "too weak here"
                } else {
                    "too strong here"
                };
                notes.push(format!(
                    "{}: reference {}, answer {} ({side})",
                    p.label,
                    word(r),
                    word(a)
                ));
            }
        }
        let status = if notes.is_empty() {
            Status::Pass
        } else {
            Status::Fail
        };
        notes.insert(
            0,
            format!("{} disagreements on {compared} probe models", notes.len()),
        );
        Check {
            grade: "G5",
            status,
            notes,
        }
    }

    /// Leak models whose leak edges also say `-L`, for each label `L` the
    /// answer names that the forbidden move does not carry. The move still
    /// gets through them, so a right rule refuses them too. A rule guarded
    /// by an invented action (`[+SIGN ...] false`) does not.
    fn label_dodging_leaks(
        &self,
        reading: &Reading,
        trace: &Trace,
        labels: &[String],
    ) -> Vec<(String, String)> {
        let Some(leak) = &trace.leak else {
            return Vec::new();
        };
        let Some(last) = trace.all_steps().pop() else {
            return Vec::new();
        };
        let mut methods: Vec<String> = last
            .actions
            .iter()
            .map(|a| a.method.to_uppercase())
            .collect();
        if !last.post.is_empty() {
            methods.push("POST".into());
        }
        let mut out = Vec::new();
        for label in labels {
            if methods.contains(label) || out.iter().any(|(l, _): &(String, String)| l == label) {
                continue;
            }
            let dodged: String = leak
                .lines()
                .map(str::trim)
                .filter(|l| !l.is_empty())
                .map(|l| {
                    if l.contains(':') {
                        format!("{l} -{label}")
                    } else {
                        format!("{l}: -{label}")
                    }
                })
                .collect::<Vec<_>>()
                .join("\n");
            if let Some(model) = model_text::with_edges(&reading.witness, &dodged) {
                if self.replay(&model, &[], trace).is_ok() {
                    out.push((label.clone(), model));
                }
            }
        }
        out
    }

    fn signer_paths(&self) -> Vec<String> {
        self.fixture
            .state
            .keys()
            .filter(|p| p.ends_with(".id"))
            .cloned()
            .collect()
    }

    /// Labels, predicates and paths outside what the contract shows.
    fn vocabulary_problems(&self, formula: &Formula) -> Vec<String> {
        let mut known_paths: Vec<String> = self.fixture.state.keys().cloned().collect();
        known_paths.extend(self.fixture.paths.iter().cloned());
        let mut model_labels = Vec::new();
        let mut model_predicates = Vec::new();
        for edge in model_text::edges(&self.fixture.model) {
            for lit in &edge.literals {
                if let Some((_, name, args)) = model_text::parse_literal(lit) {
                    match args {
                        None => model_labels.push(name),
                        Some(args) => {
                            known_paths.extend(args.into_iter().filter(|a| a.starts_with('/')));
                            model_predicates.push(name);
                        }
                    }
                }
            }
        }
        for rule in &self.fixture.rules {
            if let Ok(f) = parse_formula(rule) {
                for p in properties(&f.expression) {
                    known_paths.extend(path_args(&p));
                }
            }
        }

        let mut problems = Vec::new();
        for p in properties(&formula.expression) {
            if p.is_static() {
                if !METHOD_LABELS.contains(&p.name.as_str()) && !model_labels.contains(&p.name) {
                    problems.push(format!(
                        "`{}` is not a commit method or a label of this contract's model (invented action?)",
                        p.name
                    ));
                }
                continue;
            }
            if !EVALUATED_PREDICATES.contains(&p.name.as_str())
                && !model_predicates.contains(&p.name)
            {
                problems.push(format!(
                    "`{}` is not a predicate the validator evaluates",
                    p.name
                ));
            }
            for path in path_args(&p) {
                if !path_known(&path, &known_paths) {
                    problems.push(format!("path `{path}` is not in this contract"));
                }
            }
        }
        problems.sort();
        problems.dedup();
        problems
    }

    /// Problems with the fixture itself.
    pub fn check_fixture(&self) -> Vec<String> {
        let mut problems = Vec::new();
        for rule in &self.fixture.rules {
            if let Err(e) = parse_formula(rule) {
                problems.push(format!("fixture rule does not parse: {e}"));
            }
        }
        if let Err(e) = self.admits(&self.fixture.model, &[]) {
            problems.push(format!("fixture model and rules refused at genesis: {e}"));
        }
        problems
    }

    /// Problems with a case's own reference answers. A suite whose
    /// references fail grades nothing.
    pub fn check_reference(&self, case: &Case) -> Vec<String> {
        let mut problems = Vec::new();
        for (i, reading) in case.readings().iter().enumerate() {
            let at = format!("{} reading {i}", case.id);
            let grade = self.grade_rule(reading, &reading.formula, Some(&reading.witness));
            for c in grade.checks.iter().filter(|c| c.status == Status::Fail) {
                problems.push(format!(
                    "{at}: reference fails {}: {}",
                    c.grade,
                    c.notes.join("; ")
                ));
            }
            for t in &reading.forbid {
                let leak = match self.leak_model(reading, t) {
                    Ok(l) => l,
                    Err(e) => {
                        problems.push(format!("{at}: {e}"));
                        continue;
                    }
                };
                if let Err(e) = self.admits(&leak, &[]) {
                    problems.push(format!(
                        "{at}: leak for `{}` is refused by the contract's own rules, so it tests nothing: {e}",
                        t.name
                    ));
                } else if let Err(e) = self.replay(&leak, &[], t) {
                    problems.push(format!(
                        "{at}: leak for `{}` does not let the move through: {e}",
                        t.name
                    ));
                }
                if self
                    .replay(&reading.witness, &[reading.formula.as_str()], t)
                    .is_ok()
                {
                    problems.push(format!(
                        "{at}: the reference witness lets the forbidden move `{}` through",
                        t.name
                    ));
                }
            }
        }
        problems
    }
}

pub fn rule_file(formula: &str) -> String {
    format!("export default rule {{\n  formula {{\n    {formula}\n  }}\n}}\n")
}

/// Signer `carol` signs with the placeholder key `KEY_CAROL`.
pub fn signer_key(name: &str) -> String {
    if name.starts_with("KEY_") {
        name.to_string()
    } else {
        format!("KEY_{}", name.to_uppercase())
    }
}

fn step_commit(step: &Step) -> CommitFile {
    let mut c = CommitFile::new();
    for (path, value) in &step.post {
        c.add_action("post".into(), Some(path.clone()), value.clone());
    }
    for a in &step.actions {
        c.add_action(a.method.clone(), a.path.clone(), a.value.clone());
    }
    if !step.signers.is_empty() {
        let sigs: serde_json::Map<String, Value> = step
            .signers
            .iter()
            .map(|s| (signer_key(s), json!("sig")))
            .collect();
        c.head.signatures = Some(Value::Object(sigs));
    }
    c
}

pub fn parse_formula(formula: &str) -> Result<Formula, String> {
    let text = format!("formula r {{\n  {formula}\n}}\n");
    let mut all =
        parse_all_formulas_content_lalrpop(&text).map_err(|e| format!("does not parse: {e}"))?;
    match all.len() {
        1 => Ok(all.remove(0)),
        n => Err(format!("expected one formula, found {n}")),
    }
}

fn properties(e: &FormulaExpr) -> Vec<Property> {
    let mut out = Vec::new();
    collect(e, &mut out);
    out
}

fn collect(e: &FormulaExpr, out: &mut Vec<Property>) {
    match e {
        FormulaExpr::True | FormulaExpr::False | FormulaExpr::Prop(_) | FormulaExpr::Var(_) => {}
        FormulaExpr::And(a, b)
        | FormulaExpr::Or(a, b)
        | FormulaExpr::Implies(a, b)
        | FormulaExpr::Until(a, b) => {
            collect(a, out);
            collect(b, out);
        }
        FormulaExpr::Not(a)
        | FormulaExpr::Paren(a)
        | FormulaExpr::Eventually(a)
        | FormulaExpr::Always(a)
        | FormulaExpr::Next(a)
        | FormulaExpr::Lfp(_, a)
        | FormulaExpr::Gfp(_, a) => collect(a, out),
        FormulaExpr::Diamond(props, a)
        | FormulaExpr::Box(props, a)
        | FormulaExpr::DiamondBox(props, a) => {
            out.extend(props.iter().cloned());
            collect(a, out);
        }
    }
}

fn path_args(p: &Property) -> Vec<String> {
    let Some(PropertySource::Predicate { args, .. }) = &p.source else {
        return Vec::new();
    };
    let values: Vec<&Value> = match (args.get("arg"), args.get("args")) {
        (Some(a), _) => vec![a],
        (_, Some(Value::Array(a))) => a.iter().collect(),
        _ => Vec::new(),
    };
    values
        .into_iter()
        .filter_map(Value::as_str)
        .filter(|s| s.starts_with('/'))
        .map(str::to_string)
        .collect()
}

fn segments(path: &str) -> Vec<&str> {
    path.split('/').filter(|s| !s.is_empty()).collect()
}

fn segment_matches(a: &str, b: &str) -> bool {
    a == b || a.contains('$') || b.contains('$')
}

/// `prefix` is `full` or an ancestor of it, with `$k` matching any segment.
fn is_ancestor_or_same(prefix: &[&str], full: &[&str]) -> bool {
    prefix.len() <= full.len() && prefix.iter().zip(full).all(|(a, b)| segment_matches(a, b))
}

/// A path is known when it is a contract path, an ancestor of one, or a
/// new name beside one (`/members/eve.id` next to `/members/carol.id`).
fn path_known(path: &str, known: &[String]) -> bool {
    let p = segments(path);
    if p.is_empty() {
        return true;
    }
    known.iter().any(|k| {
        let k = segments(k);
        is_ancestor_or_same(&p, &k) || (p.len() > 1 && is_ancestor_or_same(&p[..p.len() - 1], &k))
    })
}

fn squash(s: &str) -> String {
    s.chars().filter(|c| !c.is_whitespace()).collect()
}

fn word(accepts: bool) -> &'static str {
    if accepts {
        "accepts"
    } else {
        "refuses"
    }
}

fn short(e: &str) -> String {
    const MAX: usize = 400;
    let one_line = e.replace('\n', " ");
    if one_line.chars().count() > MAX {
        format!("{}…", one_line.chars().take(MAX).collect::<String>())
    } else {
        one_line
    }
}

fn pass(grade: &'static str) -> Check {
    Check {
        grade,
        status: Status::Pass,
        notes: Vec::new(),
    }
}

fn fail(grade: &'static str, notes: Vec<String>) -> Check {
    Check {
        grade,
        status: Status::Fail,
        notes,
    }
}

fn skip(grade: &'static str) -> Check {
    Check {
        grade,
        status: Status::Skip,
        notes: Vec::new(),
    }
}

fn verdict(grade: &'static str, notes: Vec<String>) -> Check {
    if notes.is_empty() {
        pass(grade)
    } else {
        fail(grade, notes)
    }
}

fn finish(checks: Vec<Check>) -> RuleGrade {
    let ok = |g: &str| {
        checks
            .iter()
            .any(|c| c.grade == g && c.status == Status::Pass)
    };
    let reasonable = ["G0", "G1", "G2", "G3", "G4"].iter().all(|g| ok(g));
    let exact = reasonable && ok("G5");
    RuleGrade {
        checks,
        reasonable,
        exact,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every predicate the commit evaluator matches on is in the list G1
    /// accepts, so a new standard predicate is never graded as invented.
    #[test]
    fn evaluated_predicates_match_the_evaluator() {
        let src = include_str!("../../modality-common/src/model_governance.rs");
        let start = src.find("fn predicate_holds").expect("predicate_holds");
        let body = &src[start..];
        let end = body.find("\n    fn ").unwrap_or(body.len());
        let mut missing = Vec::new();
        for line in body[..end].lines() {
            let t = line.trim_start();
            if !(t.starts_with('"') && t.contains("=>")) {
                continue;
            }
            for name in t.split("=>").next().unwrap().split('|') {
                let name = name.trim().trim_matches('"');
                if !name.is_empty() && !EVALUATED_PREDICATES.contains(&name) {
                    missing.push(name.to_string());
                }
            }
        }
        assert!(
            missing.is_empty(),
            "add to EVALUATED_PREDICATES: {missing:?}"
        );
    }

    #[test]
    fn paths_beside_or_above_contract_paths_are_known() {
        let known = vec!["/members/carol.id".to_string(), "/notes".to_string()];
        assert!(path_known("/members", &known));
        assert!(path_known("/members/eve.id", &known));
        assert!(path_known("/notes/a.text", &known));
        assert!(path_known("/members/$k.id", &known));
        assert!(!path_known("/users/alice.id", &known));
        assert!(path_known("/", &known));
    }
}
