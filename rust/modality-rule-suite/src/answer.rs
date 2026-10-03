//! An answer to a case, and the case-level verdict on it.

use crate::case::{Case, Expect};
use crate::grade::{Grader, RuleGrade};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnswerKind {
    #[default]
    Rule,
    /// The request is contradictory or would freeze the contract.
    NoRule,
    /// The request is ambiguous; the answer asks instead of guessing.
    Question,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Answer {
    #[serde(default)]
    pub kind: AnswerKind,
    #[serde(default)]
    pub formula: Option<String>,
    #[serde(default)]
    pub witness: Option<String>,
    #[serde(default)]
    pub assumption: Option<String>,
    #[serde(default)]
    pub question: Option<String>,
    #[serde(default)]
    pub explanation: Option<String>,
}

impl Answer {
    /// The first JSON object in `raw`, fences and prose around it ignored.
    pub fn from_json_text(raw: &str) -> Option<Answer> {
        let start = raw.find('{')?;
        let end = raw.rfind('}')?;
        let mut answer: Answer = serde_json::from_str(raw.get(start..=end)?).ok()?;
        answer.normalize();
        Some(answer)
    }

    pub fn rule(formula: &str) -> Answer {
        let mut a = Answer {
            formula: Some(formula.to_string()),
            ..Answer::default()
        };
        a.normalize();
        a
    }

    pub fn normalize(&mut self) {
        let clean = |s: &mut Option<String>| {
            if s.as_deref().map(str::trim).is_some_and(str::is_empty) {
                *s = None;
            }
        };
        self.formula = self.formula.as_deref().map(normalize_formula);
        self.witness = self.witness.as_deref().map(|w| strip_fences(w).to_string());
        clean(&mut self.formula);
        clean(&mut self.witness);
        clean(&mut self.assumption);
        clean(&mut self.question);
    }
}

fn strip_fences(s: &str) -> &str {
    let s = s.trim();
    let s = s
        .strip_prefix("```modality")
        .or_else(|| s.strip_prefix("```"))
        .unwrap_or(s);
    s.strip_suffix("```")
        .unwrap_or(s)
        .trim()
        .trim_matches('`')
        .trim()
}

/// The inner formula, when an answer wraps it in `formula { … }` or a rule.
pub fn normalize_formula(s: &str) -> String {
    let s = strip_fences(s);
    if let Some(at) = s.find("formula") {
        let rest = &s[at + "formula".len()..];
        if let Some(open) = rest.find('{') {
            let body = &rest[open + 1..];
            let mut depth = 1;
            for (i, c) in body.char_indices() {
                match c {
                    '{' => depth += 1,
                    '}' => {
                        depth -= 1;
                        if depth == 0 {
                            return body[..i].trim().to_string();
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    s.trim_end_matches(';').trim().to_string()
}

#[derive(Debug, Clone, Serialize)]
pub struct CaseGrade {
    pub pass: bool,
    pub exact: bool,
    pub kind: AnswerKind,
    /// The reading the rule was graded against.
    pub reading: Option<usize>,
    pub rule: Option<RuleGrade>,
    pub notes: Vec<String>,
}

impl CaseGrade {
    /// G4 failed: the rule lets through a move the request forbids.
    pub fn too_weak(&self) -> bool {
        self.rule
            .as_ref()
            .is_some_and(|g| g.status("G4") == crate::grade::Status::Fail)
    }
}

pub fn grade_answer(grader: &Grader, case: &Case, answer: &Answer) -> CaseGrade {
    let readings = case.readings();
    let mut notes = Vec::new();
    let graded = |i: usize| {
        answer
            .formula
            .as_deref()
            .map(|f| grader.grade_rule(&readings[i], f, answer.witness.as_deref()))
    };
    let missing_formula = answer.kind == AnswerKind::Rule && answer.formula.is_none();
    if missing_formula {
        notes.push("answer has no formula".into());
    }

    match case.expect {
        Expect::Rule => {
            if answer.kind != AnswerKind::Rule {
                notes.push(format!("expected a rule, got {:?}", answer.kind));
            }
            let rule = if answer.kind == AnswerKind::Rule {
                graded(0)
            } else {
                None
            };
            let pass = rule.as_ref().is_some_and(|g| g.reasonable);
            let exact = rule.as_ref().is_some_and(|g| g.exact);
            CaseGrade {
                pass,
                exact,
                kind: answer.kind,
                reading: rule.as_ref().map(|_| 0),
                rule,
                notes,
            }
        }
        Expect::NoRule => {
            let pass = answer.kind == AnswerKind::NoRule;
            if !pass {
                notes.push(
                    "the request is contradictory or would freeze the contract; the answer should say so, not give a rule"
                        .into(),
                );
            }
            CaseGrade {
                pass,
                exact: pass,
                kind: answer.kind,
                reading: None,
                rule: None,
                notes,
            }
        }
        Expect::Clarify => match answer.kind {
            AnswerKind::Question => CaseGrade {
                pass: true,
                exact: true,
                kind: answer.kind,
                reading: None,
                rule: None,
                notes,
            },
            AnswerKind::NoRule => {
                notes.push("the request is ambiguous, not impossible".into());
                CaseGrade {
                    pass: false,
                    exact: false,
                    kind: answer.kind,
                    reading: None,
                    rule: None,
                    notes,
                }
            }
            AnswerKind::Rule => {
                let grades: Vec<(usize, RuleGrade)> = (0..readings.len())
                    .filter_map(|i| graded(i).map(|g| (i, g)))
                    .collect();
                let best = grades
                    .iter()
                    .max_by_key(|(i, g)| (g.exact, g.reasonable, std::cmp::Reverse(*i)))
                    .cloned();
                let stated = answer.assumption.is_some();
                if !stated {
                    notes.push(
                        "picked a reading of an ambiguous request without saying which".into(),
                    );
                }
                let reasonable = best.as_ref().is_some_and(|(_, g)| g.reasonable);
                let exact = best.as_ref().is_some_and(|(_, g)| g.exact);
                CaseGrade {
                    pass: stated && reasonable,
                    exact: stated && exact,
                    kind: answer.kind,
                    reading: best.as_ref().map(|(i, _)| *i),
                    rule: best.map(|(_, g)| g),
                    notes,
                }
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_json_answers_inside_prose_and_fences() {
        let raw = "Here you go:\n```json\n{\"kind\": \"rule\", \"formula\": \"always([-signed_by(/a.id)] false)\", \"witness\": \"\"}\n```";
        let a = Answer::from_json_text(raw).unwrap();
        assert_eq!(a.kind, AnswerKind::Rule);
        assert_eq!(
            a.formula.as_deref(),
            Some("always([-signed_by(/a.id)] false)")
        );
        assert!(a.witness.is_none());
    }

    #[test]
    fn unwraps_rule_and_formula_blocks() {
        assert_eq!(
            normalize_formula("rule r {\n  formula {\n    always([] false)\n  }\n}"),
            "always([] false)"
        );
        assert_eq!(normalize_formula("`always([] false)`"), "always([] false)");
    }
}
