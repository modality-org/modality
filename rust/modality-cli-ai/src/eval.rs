//! `modal ai eval`: grade AI-written rules against the rule suite.
//!
//! Three sources of answers: the suite's own references (`--references`,
//! which checks the suite), a JSONL file (`--answers`), or the configured
//! AI provider. Grading is the validator's, never the model's.

use anyhow::{bail, Context, Result};
use clap::{Parser, ValueEnum};
use modality_rule_suite::{
    grade_answer, render_contract, Answer, AnswerKind, Case, CaseGrade, Fixture, Grader, Split,
    Status, Suite,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::config::{self, AiConfig, Provider};
use crate::cursor_agent;
use crate::providers::{self, ReqwestPoster};

const MODEL_COOKBOOK: &str = include_str!("../../../docs/language/model-cookbook.md");

/// What every answer is asked to look like. The suite's cases never appear
/// here (a test checks the held-out ones).
pub const EVAL_SYSTEM: &str = r#"You write Modality rules for an existing contract.

Reply with one JSON object and nothing else:
{"kind": "rule" | "no_rule" | "question",
 "formula": "<one formula>",
 "witness": "<a model that meets the formula and the rules in force>",
 "assumption": "<the reading you chose, when the request is ambiguous>",
 "question": "<what you need to know, when you ask instead>",
 "explanation": "<one or two sentences>"}

The formula is the inner contents of a rule's `formula { ... }` block.
- A rule is checked from the state the commit that adds it reaches; that commit is never constrained by it. Use always(φ) for "from now on", "after this commit", or any standing requirement. Do not prefix it with [].
- Forbid a move with a box whose body is false. `[+modifies(/p) -signed_by(/k.id)] false` refuses any commit that writes under /p without that key's signature.
- The literals inside one box hold together. `[-signed_by(/a.id) -signed_by(/b.id)] false` refuses a commit that lacks both signatures, so either key may sign. Requiring both keys takes two boxes joined by &.
- Use only this contract's paths, its model's labels, commit methods (POST, RULE, MODEL, SEND, RECV, CREATE, INVOKE, REPOST) and standard predicates. Never invent an action name.
- num_*, bool_* and text_* predicates read accepted state. They never see a value written by the commit being checked.
- A rule is permanent, and it must hold together with the rules already in force.

The witness is a model in this form. Its first edge is unlabeled, and it allows every move the request allows:
model Contract {
  part flow {
    q0 --> q1
    q1 --> q1: +signed_by(/k.id)
  }
}

If the request is contradictory, or together with the rules in force would leave no later commit possible, reply with kind "no_rule" and say why.
If the request is ambiguous, either ask (kind "question") or give a rule and state the reading you chose in "assumption". Never pick a reading silently."#;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Content {
    /// The instructions above only
    None,
    /// The instructions and the formula and model cookbooks
    Cookbook,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum SplitArg {
    Dev,
    Heldout,
}

#[derive(Debug, Parser)]
#[command(
    about = "Grade AI-written rules against the rule suite",
    long_about = "Grade AI-written rules against the rule suite.\n\n\
Each case is a contract and a plain-language request. An answer is graded by the validator:\n\
G0 parses, G1 lint-clean and uses only the contract's vocabulary, G2 fits the rules in force,\n\
G3 not too strong (accepts the reference witness and its allowed moves), G4 not too weak\n\
(refuses every leak model), G5 agrees with the reference on probe models, G6 the answer's own\n\
witness admits the allowed moves. G0-G4 make an answer reasonable; G5 makes it exact.\n\n\
By default the configured provider (`modal ai set`) answers. --references checks the suite's\n\
own answers; --answers grades a JSONL file of {\"id\", \"wording\", \"kind\", \"formula\", ...}."
)]
pub struct Opts {
    /// Suite directory: one .toml file per fixture contract
    #[clap(long, default_value = "tests/ai-rules")]
    suite: PathBuf,

    /// Check the suite's reference answers instead of asking an AI
    #[clap(long, conflicts_with = "answers")]
    references: bool,

    /// Grade answers from a JSONL file instead of asking an AI
    #[clap(long)]
    answers: Option<PathBuf>,

    /// Only cases in this split
    #[clap(long, value_enum)]
    split: Option<SplitArg>,

    /// Only cases whose id starts with this (repeatable)
    #[clap(long = "case")]
    cases: Vec<String>,

    /// Only cases with this tag (repeatable)
    #[clap(long = "tag")]
    tags: Vec<String>,

    /// Ask with every paraphrase, not only the request
    #[clap(long)]
    paraphrases: bool,

    /// Ask each wording this many times (reports pass^k)
    #[clap(long, default_value_t = 1)]
    repeat: usize,

    /// Stop after this many AI calls
    #[clap(long)]
    max_calls: Option<usize>,

    /// Modality material sent with each request
    #[clap(long, value_enum, default_value_t = Content::Cookbook)]
    content: Content,

    /// Model override for the configured provider
    #[clap(long)]
    model: Option<String>,

    /// API key override (otherwise env or saved config)
    #[clap(long)]
    api_key: Option<String>,

    /// Write a JSON report here
    #[clap(long)]
    report: Option<PathBuf>,

    /// Print every grade's notes, not only failures
    #[clap(long)]
    verbose: bool,
}

#[derive(Debug, Deserialize)]
struct AnswerLine {
    id: String,
    #[serde(default)]
    wording: usize,
    #[serde(flatten)]
    answer: Answer,
}

#[derive(Debug, Serialize)]
struct Run {
    case: String,
    fixture: String,
    tags: Vec<String>,
    wording: usize,
    attempt: usize,
    answer: Answer,
    #[serde(skip_serializing_if = "Option::is_none")]
    raw: Option<String>,
    grade: CaseGrade,
}

#[derive(Debug, Serialize)]
struct Report {
    suite: String,
    source: String,
    content: Option<Content>,
    runs: Vec<Run>,
    summary: Summary,
}

#[derive(Debug, Default, Serialize)]
struct Summary {
    runs: usize,
    reasonable: usize,
    exact: usize,
    /// Runs that let a forbidden move through on a `safety` case.
    safety_too_weak: usize,
    /// (case, wording) pairs whose every attempt passed, of all pairs.
    all_attempts_pass: Option<(usize, usize)>,
    by_tag: BTreeMap<String, (usize, usize)>,
}

pub async fn run(opts: &Opts) -> Result<()> {
    let suite = Suite::load(&opts.suite)?;
    if opts.references {
        return check_references(&suite, opts);
    }
    let (runs, source) = match &opts.answers {
        Some(path) => (
            grade_file(&suite, opts, path)?,
            format!("answers {}", path.display()),
        ),
        None => ask(&suite, opts).await?,
    };
    let summary = summarize(&runs, opts.repeat);
    print_summary(&summary);
    if let Some(path) = &opts.report {
        let report = Report {
            suite: opts.suite.display().to_string(),
            source,
            content: opts.answers.is_none().then_some(opts.content),
            runs,
            summary,
        };
        std::fs::write(path, serde_json::to_string_pretty(&report)?)
            .with_context(|| format!("Failed to write {}", path.display()))?;
        println!("Report: {}", path.display());
    }
    Ok(())
}

fn selected<'a>(suite: &'a Suite, opts: &Opts) -> Vec<(&'a Fixture, &'a Case)> {
    suite
        .cases()
        .filter(|(_, c)| match opts.split {
            Some(SplitArg::Dev) => c.split == Split::Dev,
            Some(SplitArg::Heldout) => c.split == Split::Heldout,
            None => true,
        })
        .filter(|(_, c)| opts.cases.is_empty() || opts.cases.iter().any(|p| c.id.starts_with(p)))
        .filter(|(_, c)| opts.tags.is_empty() || opts.tags.iter().any(|t| c.has_tag(t)))
        .collect()
}

fn check_references(suite: &Suite, opts: &Opts) -> Result<()> {
    let mut problems = Vec::new();
    let mut checked = 0;
    for (_, file) in &suite.files {
        let grader = Grader::new(&file.fixture)?;
        problems.extend(
            grader
                .check_fixture()
                .into_iter()
                .map(|p| format!("fixture {}: {p}", file.fixture.name)),
        );
    }
    for (fixture, case) in selected(suite, opts) {
        let grader = Grader::new(fixture)?;
        let found = grader.check_reference(case);
        checked += 1;
        println!(
            "{} {}",
            if found.is_empty() { "ok  " } else { "FAIL" },
            case.id
        );
        problems.extend(found);
    }
    if !problems.is_empty() {
        for p in &problems {
            eprintln!("  {p}");
        }
        bail!(
            "{} reference problem(s) in {checked} case(s)",
            problems.len()
        );
    }
    println!("All {checked} case references pass their own cases.");
    Ok(())
}

fn grade_file(suite: &Suite, opts: &Opts, path: &Path) -> Result<Vec<Run>> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read {}", path.display()))?;
    let mut lines: Vec<AnswerLine> = Vec::new();
    for (i, line) in text
        .lines()
        .enumerate()
        .filter(|(_, l)| !l.trim().is_empty())
    {
        let mut parsed: AnswerLine = serde_json::from_str(line)
            .with_context(|| format!("{}:{}: not an answer line", path.display(), i + 1))?;
        parsed.answer.normalize();
        lines.push(parsed);
    }
    let mut runs = Vec::new();
    for (fixture, case) in selected(suite, opts) {
        let grader = Grader::new(fixture)?;
        let mine: Vec<&AnswerLine> = lines.iter().filter(|l| l.id == case.id).collect();
        if mine.is_empty() {
            println!("MISS {}", case.id);
            continue;
        }
        let mut attempts: BTreeMap<usize, usize> = BTreeMap::new();
        for line in mine {
            let attempt = attempts.entry(line.wording).or_default();
            let grade = grade_answer(&grader, case, &line.answer);
            let run = Run {
                case: case.id.clone(),
                fixture: fixture.name.clone(),
                tags: case.tags.clone(),
                wording: line.wording,
                attempt: *attempt,
                answer: line.answer.clone(),
                raw: None,
                grade,
            };
            *attempt += 1;
            print_run(&run, opts.verbose);
            runs.push(run);
        }
    }
    Ok(runs)
}

async fn ask(suite: &Suite, opts: &Opts) -> Result<(Vec<Run>, String)> {
    let mut config: AiConfig = config::load_required()?;
    if let Some(model) = &opts.model {
        config.model = Some(model.clone());
    }
    let provider = config.provider()?;
    let source = format!("{} {}", provider.as_str(), config.model()?);
    println!("Asking {source}, content: {:?}", opts.content);
    let system = match opts.content {
        Content::None => EVAL_SYSTEM.to_string(),
        Content::Cookbook => format!(
            "{EVAL_SYSTEM}\n\n---\n\n{}\n\n---\n\n{MODEL_COOKBOOK}",
            cursor_agent::EMBEDDED_FORMULA_COOKBOOK
        ),
    };
    let workspace = std::env::temp_dir().join(format!("modal-ai-eval-{}", std::process::id()));
    std::fs::create_dir_all(&workspace)?;
    let poster = ReqwestPoster::new()?;

    let mut runs = Vec::new();
    let mut calls = 0usize;
    let mut errors_in_a_row = 0;
    'cases: for (fixture, case) in selected(suite, opts) {
        let grader = Grader::new(fixture)?;
        let wordings = if opts.paraphrases {
            case.wordings()
        } else {
            vec![case.request.as_str()]
        };
        for (w, wording) in wordings.iter().enumerate() {
            for attempt in 0..opts.repeat.max(1) {
                if opts.max_calls.is_some_and(|max| calls >= max) {
                    println!("Stopped at --max-calls {calls}.");
                    break 'cases;
                }
                calls += 1;
                let user = format!("{}\nRequest:\n{wording}\n", render_contract(fixture));
                let reply = if provider == Provider::CursorAgent {
                    cursor_agent::run_prompt(
                        &config,
                        &format!("{system}\n\n---\n\n{user}"),
                        opts.api_key.as_deref(),
                        &workspace,
                    )
                    .await
                } else {
                    providers::complete_with_system(
                        &config,
                        &system,
                        &user,
                        opts.api_key.as_deref(),
                        &poster,
                    )
                    .await
                };
                let (answer, raw) = match reply {
                    Ok(raw) => {
                        errors_in_a_row = 0;
                        (read_answer(&raw), Some(raw))
                    }
                    Err(e) => {
                        errors_in_a_row += 1;
                        eprintln!("  {}: provider error: {e:#}", case.id);
                        if errors_in_a_row >= 3 {
                            bail!("Three provider errors in a row; stopping.");
                        }
                        (Answer::default(), Some(format!("provider error: {e:#}")))
                    }
                };
                let grade = grade_answer(&grader, case, &answer);
                let run = Run {
                    case: case.id.clone(),
                    fixture: fixture.name.clone(),
                    tags: case.tags.clone(),
                    wording: w,
                    attempt,
                    answer,
                    raw,
                    grade,
                };
                print_run(&run, opts.verbose);
                runs.push(run);
            }
        }
    }
    let _ = std::fs::remove_dir_all(&workspace);
    Ok((runs, source))
}

/// A JSON answer, or failing that a bare formula.
fn read_answer(raw: &str) -> Answer {
    if let Some(answer) = Answer::from_json_text(raw) {
        return answer;
    }
    match providers::extract_formula(raw) {
        Ok(formula) => Answer::rule(&formula),
        Err(_) => Answer::default(),
    }
}

fn print_run(run: &Run, verbose: bool) {
    let g = &run.grade;
    let head = if g.exact {
        "EXACT"
    } else if g.pass {
        "PASS "
    } else {
        "FAIL "
    };
    let marks = g
        .rule
        .as_ref()
        .map(|r| r.marks())
        .unwrap_or_else(|| format!("{:?}", g.kind));
    println!(
        "{head} {} [wording {} attempt {}] {marks}",
        run.case, run.wording, run.attempt
    );
    if g.pass && !verbose {
        return;
    }
    if let Some(f) = &run.answer.formula {
        println!("      formula: {f}");
    }
    if run.answer.kind != AnswerKind::Rule {
        if let Some(text) = run
            .answer
            .question
            .as_ref()
            .or(run.answer.explanation.as_ref())
        {
            println!(
                "      {}: {text}",
                if run.answer.kind == AnswerKind::Question {
                    "asked"
                } else {
                    "said"
                }
            );
        }
    }
    for n in &g.notes {
        println!("      {n}");
    }
    if let Some(r) = &g.rule {
        for c in &r.checks {
            if c.status == Status::Fail || (verbose && !c.notes.is_empty()) {
                for n in &c.notes {
                    println!("      {}: {n}", c.grade);
                }
            }
        }
    }
}

fn summarize(runs: &[Run], repeat: usize) -> Summary {
    let mut s = Summary {
        runs: runs.len(),
        ..Summary::default()
    };
    let mut pairs: BTreeMap<(String, usize), bool> = BTreeMap::new();
    for r in runs {
        s.reasonable += r.grade.pass as usize;
        s.exact += r.grade.exact as usize;
        if r.tags.iter().any(|t| t == "safety") && r.grade.too_weak() {
            s.safety_too_weak += 1;
        }
        for t in &r.tags {
            let e = s.by_tag.entry(t.clone()).or_default();
            e.0 += r.grade.pass as usize;
            e.1 += 1;
        }
        let all = pairs.entry((r.case.clone(), r.wording)).or_insert(true);
        *all &= r.grade.pass;
    }
    if repeat > 1 {
        s.all_attempts_pass = Some((pairs.values().filter(|v| **v).count(), pairs.len()));
    }
    s
}

fn print_summary(s: &Summary) {
    if s.runs == 0 {
        println!("No runs.");
        return;
    }
    let pct = |n: usize, d: usize| {
        if d == 0 {
            0.0
        } else {
            100.0 * n as f64 / d as f64
        }
    };
    println!();
    println!(
        "Reasonable: {}/{} ({:.0}%)   Exact: {}/{} ({:.0}%)",
        s.reasonable,
        s.runs,
        pct(s.reasonable, s.runs),
        s.exact,
        s.runs,
        pct(s.exact, s.runs)
    );
    if let Some((ok, all)) = s.all_attempts_pass {
        println!("Every attempt passed: {ok}/{all} (case, wording) pairs");
    }
    println!(
        "Too weak on safety cases: {}{}",
        s.safety_too_weak,
        if s.safety_too_weak > 0 {
            "  (a model with any is not a default)"
        } else {
            ""
        }
    );
    for (tag, (ok, all)) in &s.by_tag {
        println!("  {tag:<14} {ok}/{all}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_a_bare_formula_when_there_is_no_json() {
        let a = read_answer("always([-signed_by(/parties/alice.id)] false)");
        assert_eq!(a.kind, AnswerKind::Rule);
        assert_eq!(
            a.formula.as_deref(),
            Some("always([-signed_by(/parties/alice.id)] false)")
        );
    }

    #[test]
    fn the_checked_in_suite_references_pass_through_the_cli_path() {
        let suite_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/ai-rules");
        let opts = Opts::parse_from([
            "eval",
            "--references",
            "--suite",
            suite_dir.to_str().unwrap(),
        ]);
        let suite = Suite::load(&opts.suite).unwrap();
        check_references(&suite, &opts).unwrap();
    }

    #[test]
    fn grades_an_answers_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("answers.jsonl");
        std::fs::write(
            &path,
            concat!(
                r#"{"id": "fc-alice-signs", "formula": "always([-signed_by(/parties/alice.id)] false)"}"#,
                "\n",
                r#"{"id": "fc-either-signs", "formula": "always([-signed_by(/parties/alice.id)] false)"}"#,
                "\n",
            ),
        )
        .unwrap();
        let suite_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/ai-rules");
        let opts = Opts::parse_from([
            "eval",
            "--suite",
            suite_dir.to_str().unwrap(),
            "--answers",
            path.to_str().unwrap(),
            "--case",
            "fc-alice",
            "--case",
            "fc-either",
        ]);
        let suite = Suite::load(&opts.suite).unwrap();
        let runs = grade_file(&suite, &opts, &path).unwrap();
        let by_id: BTreeMap<_, _> = runs
            .iter()
            .map(|r| (r.case.as_str(), r.grade.pass))
            .collect();
        assert_eq!(by_id.get("fc-alice-signs"), Some(&true));
        assert_eq!(
            by_id.get("fc-either-signs"),
            Some(&false),
            "Alice-only is too strong for either"
        );
    }
}
