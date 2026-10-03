//! The suite's file format: one TOML file per fixture contract, holding
//! the contract and the plain-language rule requests made of it.

use anyhow::{anyhow, bail, Context, Result};
use modality_lang::TheoryVersion;
use serde::Deserialize;
use serde_json::Value;
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SuiteFile {
    pub fixture: Fixture,
    #[serde(default, rename = "case")]
    pub cases: Vec<Case>,
}

/// A contract as it stands before the request: what the AI is shown.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fixture {
    pub name: String,
    #[serde(default)]
    pub about: String,
    /// Predicate theory every check of this fixture runs under.
    #[serde(default = "default_theory")]
    pub theory: String,
    /// Accepted state, `path = value`. Signer `carol` signs with the
    /// placeholder key `KEY_CAROL`, so `.id` values name those keys.
    #[serde(default)]
    pub state: BTreeMap<String, Value>,
    /// Paths the contract uses that hold no value yet (for example
    /// `/notes`). Part of the vocabulary a rule may name.
    #[serde(default)]
    pub paths: Vec<String>,
    /// The governing model.
    pub model: String,
    /// Formulas already in force.
    #[serde(default)]
    pub rules: Vec<String>,
}

fn default_theory() -> String {
    "v3".to_string()
}

impl Fixture {
    pub fn theory(&self) -> Result<TheoryVersion> {
        self.theory
            .parse()
            .map_err(|e: String| anyhow!("fixture {}: {e}", self.name))
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Split {
    #[default]
    Dev,
    Heldout,
}

/// What a right answer is.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Expect {
    /// A rule that matches the one reference reading.
    #[default]
    Rule,
    /// No rule: the request is contradictory or would freeze the contract.
    NoRule,
    /// A question, or a rule whose stated assumption matches one reading.
    Clarify,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Case {
    pub id: String,
    #[serde(default)]
    pub split: Split,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default = "one")]
    pub difficulty: u8,
    #[serde(default)]
    pub expect: Expect,
    pub request: String,
    #[serde(default)]
    pub paraphrases: Vec<String>,
    /// The trap this case is about, for the person reading a failure.
    #[serde(default)]
    pub why: String,
    #[serde(default)]
    pub formula: Option<String>,
    #[serde(default)]
    pub witness: Option<String>,
    #[serde(default)]
    pub allow: Vec<Trace>,
    #[serde(default)]
    pub forbid: Vec<Trace>,
    /// `clarify` cases: one reference per reading.
    #[serde(default, rename = "reading")]
    pub readings: Vec<Reading>,
}

fn one() -> u8 {
    1
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reading {
    #[serde(default)]
    pub assumption: String,
    pub formula: String,
    /// A model that meets the formula and admits every `allow` move.
    pub witness: String,
    #[serde(default)]
    pub allow: Vec<Trace>,
    #[serde(default)]
    pub forbid: Vec<Trace>,
}

/// Commits applied in order right after the contract's genesis. In a
/// `forbid` trace every step but the last is allowed; the last is the
/// move the request forbids.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Trace {
    pub name: String,
    #[serde(default)]
    pub steps: Vec<Step>,
    /// Shorthand for one more, final step.
    #[serde(default)]
    pub signers: Vec<String>,
    #[serde(default)]
    pub post: BTreeMap<String, Value>,
    #[serde(default)]
    pub actions: Vec<ActionSpec>,
    /// `forbid` only: edge lines added to the witness so the move gets
    /// through. A right rule refuses that model.
    #[serde(default)]
    pub leak: Option<String>,
    /// `forbid` only: a whole leak model, when added edges are not enough.
    #[serde(default)]
    pub leak_model: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Step {
    #[serde(default)]
    pub signers: Vec<String>,
    #[serde(default)]
    pub post: BTreeMap<String, Value>,
    #[serde(default)]
    pub actions: Vec<ActionSpec>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActionSpec {
    pub method: String,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub value: Value,
}

impl Trace {
    pub fn all_steps(&self) -> Vec<Step> {
        let mut steps = self.steps.clone();
        if !self.signers.is_empty() || !self.post.is_empty() || !self.actions.is_empty() {
            steps.push(Step {
                signers: self.signers.clone(),
                post: self.post.clone(),
                actions: self.actions.clone(),
            });
        }
        steps
    }
}

impl Case {
    /// The reference readings: the case-level one, or each `[[case.reading]]`.
    pub fn readings(&self) -> Vec<Reading> {
        if !self.readings.is_empty() {
            return self.readings.clone();
        }
        match (&self.formula, &self.witness) {
            (Some(formula), Some(witness)) => vec![Reading {
                assumption: String::new(),
                formula: formula.clone(),
                witness: witness.clone(),
                allow: self.allow.clone(),
                forbid: self.forbid.clone(),
            }],
            _ => Vec::new(),
        }
    }

    /// The request, then each paraphrase.
    pub fn wordings(&self) -> Vec<&str> {
        std::iter::once(self.request.as_str())
            .chain(self.paraphrases.iter().map(String::as_str))
            .collect()
    }

    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|t| t == tag)
    }
}

/// Every fixture file of a suite directory, in file-name order.
#[derive(Debug, Clone)]
pub struct Suite {
    pub dir: PathBuf,
    pub files: Vec<(PathBuf, SuiteFile)>,
}

impl Suite {
    pub fn load(dir: &Path) -> Result<Self> {
        let mut paths: Vec<PathBuf> = std::fs::read_dir(dir)
            .with_context(|| format!("Failed to read suite directory {}", dir.display()))?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("toml"))
            .collect();
        paths.sort();
        if paths.is_empty() {
            bail!("No .toml fixture files in {}", dir.display());
        }
        let mut files = Vec::new();
        for path in paths {
            let text = std::fs::read_to_string(&path)
                .with_context(|| format!("Failed to read {}", path.display()))?;
            let file: SuiteFile = toml::from_str(&text)
                .with_context(|| format!("Invalid suite file {}", path.display()))?;
            files.push((path, file));
        }
        let suite = Suite {
            dir: dir.to_path_buf(),
            files,
        };
        suite.check_structure()?;
        Ok(suite)
    }

    pub fn cases(&self) -> impl Iterator<Item = (&Fixture, &Case)> {
        self.files
            .iter()
            .flat_map(|(_, f)| f.cases.iter().map(move |c| (&f.fixture, c)))
    }

    /// Shape errors a reader would otherwise find as confusing grades.
    fn check_structure(&self) -> Result<()> {
        let mut problems = Vec::new();
        let mut fixtures = HashSet::new();
        let mut ids = HashSet::new();
        for (path, file) in &self.files {
            let fx = &file.fixture;
            let at = |id: &str| format!("{} {id}", path.display());
            if !fixtures.insert(fx.name.clone()) {
                problems.push(format!(
                    "{}: fixture name `{}` repeats",
                    path.display(),
                    fx.name
                ));
            }
            if let Err(e) = fx.theory() {
                problems.push(e.to_string());
            }
            for case in &file.cases {
                if !ids.insert(case.id.clone()) {
                    problems.push(format!("{}: case id repeats", at(&case.id)));
                }
                let readings = case.readings();
                match case.expect {
                    Expect::Rule if readings.len() != 1 => problems.push(format!(
                        "{}: expect = rule needs `formula` and `witness`",
                        at(&case.id)
                    )),
                    Expect::Clarify if readings.len() < 2 => problems.push(format!(
                        "{}: expect = clarify needs two or more [[case.reading]]",
                        at(&case.id)
                    )),
                    Expect::NoRule if !readings.is_empty() => problems.push(format!(
                        "{}: expect = no_rule has no reference reading",
                        at(&case.id)
                    )),
                    _ => {}
                }
                if case.expect == Expect::Clarify
                    && readings.iter().any(|r| r.assumption.trim().is_empty())
                {
                    problems.push(format!(
                        "{}: every reading needs an assumption",
                        at(&case.id)
                    ));
                }
                for (i, r) in readings.iter().enumerate() {
                    if r.allow.is_empty() {
                        problems.push(format!(
                            "{} reading {i}: needs at least one `allow` move, or the rule may freeze the contract unnoticed",
                            at(&case.id)
                        ));
                    }
                    for t in &r.allow {
                        if t.leak.is_some() || t.leak_model.is_some() {
                            problems.push(format!(
                                "{} allow `{}`: has a leak",
                                at(&case.id),
                                t.name
                            ));
                        }
                        if t.all_steps().is_empty() {
                            problems.push(format!("{} allow `{}`: no steps", at(&case.id), t.name));
                        }
                    }
                    for t in &r.forbid {
                        if t.leak.is_some() == t.leak_model.is_some() {
                            problems.push(format!(
                                "{} forbid `{}`: needs exactly one of `leak` or `leak_model`",
                                at(&case.id),
                                t.name
                            ));
                        }
                        if t.all_steps().is_empty() {
                            problems.push(format!(
                                "{} forbid `{}`: no steps",
                                at(&case.id),
                                t.name
                            ));
                        }
                    }
                }
            }
        }
        if problems.is_empty() {
            Ok(())
        } else {
            bail!(
                "Suite {} has shape errors:\n  {}",
                self.dir.display(),
                problems.join("\n  ")
            )
        }
    }
}
