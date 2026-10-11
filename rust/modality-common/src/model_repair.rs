//! A governing model that lets a refused commit land.
//!
//! When the governing model refuses a commit, [`find_admitting_model`] looks
//! for the smallest change to it that lets the commit in: one new edge out
//! of a current node, labeled with the fewest labels the commit satisfies.
//! Fewer labels make a looser edge, so the first candidate that passes is
//! the least the model has to grow.
//!
//! The search decides nothing itself. Every candidate goes to `accepts`,
//! which the caller makes the full local verify of the commit carrying that
//! model: history replayed on it, every anchored rule checked against it, and
//! the commit taking one of its edges. So a model it returns is one the
//! network accepts with this commit, and a rule that forbids the commit
//! outright leaves it with nothing to return.
//!
//! The edge is inserted into the model's own text, in its own style, so the
//! rest of the file (comments included) is kept as written.

use anyhow::Result;
use modality_lang::{parse_content_lalrpop, Model, Property, PropertySign, PropertySource};

/// How far [`find_admitting_model`] looks.
#[derive(Debug, Clone, Copy)]
pub struct RepairLimits {
    /// Most labels on the new edge.
    pub max_labels: usize,
    /// Most candidate models handed to `accepts`.
    pub max_candidates: usize,
}

impl Default for RepairLimits {
    fn default() -> Self {
        Self {
            max_labels: 3,
            max_candidates: 2000,
        }
    }
}

/// A model that admits the commit: `model_text` is the original text with
/// `line` added to `part` (`None` for a model without parts).
#[derive(Debug, Clone)]
pub struct Repair {
    pub model_text: String,
    pub part: Option<String>,
    pub from: String,
    pub to: String,
    pub labels: Vec<Property>,
    pub line: String,
    /// Candidates tried, this one included.
    pub tried: usize,
}

/// What [`find_admitting_model`] found.
#[derive(Debug)]
pub enum RepairOutcome {
    Found(Repair),
    /// Nothing within the limits. `last_refusal` is why the candidate with
    /// the most labels was refused: the closest the search came.
    NotFound {
        tried: usize,
        last_refusal: Option<String>,
    },
}

/// Search for a one-edge change to `model_text` that `accepts` takes.
///
/// `current_states` are the nodes the contract is at; `labels` are the
/// labels the commit satisfies, in the order to prefer them.
pub fn find_admitting_model(
    model_text: &str,
    current_states: &[String],
    labels: &[Property],
    limits: RepairLimits,
    mut accepts: impl FnMut(&str) -> Result<()>,
) -> Result<RepairOutcome> {
    let model = parse_content_lalrpop(model_text)
        .map_err(|e| anyhow::anyhow!("Invalid governing model syntax: {e}"))?;
    let legacy = model_text.contains("-->");
    let mut tried = 0;
    let mut last_refusal: Option<(usize, String)> = None;

    for size in 0..=limits.max_labels.min(labels.len()) {
        for chosen in combinations(labels.len(), size) {
            let edge_labels: Vec<Property> = chosen.iter().map(|&i| labels[i].clone()).collect();
            if contradicts(&edge_labels) {
                continue;
            }
            for from in current_states {
                let Some((part, nodes)) = home_of(&model, from) else {
                    continue;
                };
                for to in targets(from, &nodes) {
                    if tried >= limits.max_candidates {
                        return Ok(not_found(tried, last_refusal));
                    }
                    let line = edge_line(from, &to, &edge_labels, legacy);
                    let Some(text) = insert_edge(model_text, part.as_deref(), &line) else {
                        continue;
                    };
                    if !adds_exactly(&model, &text, part.as_deref(), from, &to, &edge_labels) {
                        continue;
                    }
                    tried += 1;
                    match accepts(&text) {
                        Ok(()) => {
                            return Ok(RepairOutcome::Found(Repair {
                                model_text: text,
                                part,
                                from: from.clone(),
                                to,
                                labels: edge_labels,
                                line,
                                tried,
                            }))
                        }
                        Err(err) => {
                            if last_refusal.as_ref().is_none_or(|(n, _)| size >= *n) {
                                last_refusal = Some((size, err.to_string()));
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(not_found(tried, last_refusal))
}

fn not_found(tried: usize, last_refusal: Option<(usize, String)>) -> RepairOutcome {
    RepairOutcome::NotFound {
        tried,
        last_refusal: last_refusal.map(|(_, e)| e),
    }
}

/// Index sets of `k` out of `n`, smallest indices first.
fn combinations(n: usize, k: usize) -> Vec<Vec<usize>> {
    fn go(start: usize, n: usize, k: usize, cur: &mut Vec<usize>, out: &mut Vec<Vec<usize>>) {
        if cur.len() == k {
            out.push(cur.clone());
            return;
        }
        for i in start..n {
            cur.push(i);
            go(i + 1, n, k, cur, out);
            cur.pop();
        }
    }
    let mut out = Vec::new();
    go(0, n, k, &mut Vec::new(), &mut out);
    out
}

/// `+p` beside `-p`: an edge no commit takes.
fn contradicts(labels: &[Property]) -> bool {
    labels.iter().any(|a| {
        labels
            .iter()
            .any(|b| a.sign != b.sign && a.name == b.name && a.source == b.source)
    })
}

/// The part a node is in (the first, if several), and that part's nodes in
/// order of appearance. `None` part: a model whose transitions sit directly
/// in it.
fn home_of(model: &Model, node: &str) -> Option<(Option<String>, Vec<String>)> {
    fn nodes_of<'a>(ts: impl Iterator<Item = &'a modality_lang::Transition>) -> Vec<String> {
        let mut nodes = Vec::new();
        for t in ts {
            for n in [&t.from, &t.to] {
                if !nodes.contains(n) {
                    nodes.push(n.clone());
                }
            }
        }
        nodes
    }
    for part in &model.parts {
        let nodes = nodes_of(part.transitions.iter());
        if nodes.iter().any(|n| n == node) {
            return Some((Some(part.name.clone()), nodes));
        }
    }
    let nodes = nodes_of(model.transitions.iter());
    let starts_here = model.initial.as_deref() == Some(node);
    (nodes.iter().any(|n| n == node) || starts_here).then_some((None, nodes))
}

/// Where the new edge may go, best first: back to `from` (the contract stays
/// where it is and can go on), to another node of the part, then to a new
/// node.
fn targets(from: &str, nodes: &[String]) -> Vec<String> {
    let mut out = vec![from.to_string()];
    out.extend(nodes.iter().filter(|n| *n != from).cloned());
    let fresh = (0..)
        .map(|i| format!("q{i}"))
        .find(|n| !nodes.contains(n))
        .expect("an unused node name");
    out.push(fresh);
    out
}

/// `from -> to [labels]`, or `from --> to: labels` in a model written with
/// `-->`.
fn edge_line(from: &str, to: &str, labels: &[Property], legacy: bool) -> String {
    let labels = labels.iter().map(label_text).collect::<Vec<_>>().join(" ");
    match (legacy, labels.is_empty()) {
        (true, true) => format!("{from} --> {to}"),
        (true, false) => format!("{from} --> {to}: {labels}"),
        (false, true) => format!("{from} -> {to}"),
        (false, false) => format!("{from} -> {to} [{labels}]"),
    }
}

/// A label as it is written: `+POST`, `-signed_by(/parties/bob.id)`,
/// `+threshold("2", /signers)`.
pub fn label_text(p: &Property) -> String {
    let sign = match p.sign {
        PropertySign::Plus => "+",
        PropertySign::Minus => "-",
    };
    let Some(PropertySource::Predicate { args, .. }) = &p.source else {
        return format!("{sign}{}", p.name);
    };
    let values: Vec<&serde_json::Value> = match args.get("arg") {
        Some(arg) => vec![arg],
        None => args
            .get("args")
            .and_then(|a| a.as_array())
            .map(|a| a.iter().collect())
            .unwrap_or_default(),
    };
    let args = values
        .iter()
        .map(|v| {
            let s = v
                .as_str()
                .map(str::to_string)
                .unwrap_or_else(|| v.to_string());
            let bare = s.starts_with('/') && !s.contains([' ', ',', ')'])
                || (s
                    .chars()
                    .next()
                    .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
                    && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'));
            if bare {
                s
            } else {
                format!("\"{s}\"")
            }
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!("{sign}{}({args})", p.name)
}

/// Whether `text` parses to `before` plus exactly the edge asked for, so
/// what is committed is what was searched.
fn adds_exactly(
    before: &Model,
    text: &str,
    part: Option<&str>,
    from: &str,
    to: &str,
    labels: &[Property],
) -> bool {
    let Ok(mut after) = parse_content_lalrpop(text) else {
        return false;
    };
    let transitions = match part {
        Some(name) => match after.parts.iter_mut().find(|p| p.name == name) {
            Some(p) => &mut p.transitions,
            None => return false,
        },
        None => &mut after.transitions,
    };
    let Some(added) = transitions.pop() else {
        return false;
    };
    added.from == from && added.to == to && added.properties == labels && &after == before
}

/// `text` with `line` added as the last edge of `part` (or of the model,
/// for `None`), indented like its neighbours.
fn insert_edge(text: &str, part: Option<&str>, line: &str) -> Option<String> {
    let close = block_close(text, part)?;
    let line_start = text[..close].rfind('\n').map_or(0, |i| i + 1);
    let brace_alone = text[line_start..close].trim().is_empty();
    let closing_indent = &text[line_start..line_start + leading_ws(&text[line_start..])];
    let indent = text[..line_start]
        .lines()
        .rev()
        .find(|l| !l.trim().is_empty())
        .filter(|l| !l.trim_end().ends_with('{'))
        .map(|l| l[..leading_ws(l)].to_string())
        .unwrap_or_else(|| format!("{closing_indent}  "));
    let mut out = String::with_capacity(text.len() + line.len() + 8);
    if brace_alone {
        out.push_str(&text[..line_start]);
        out.push_str(&format!("{indent}{line}\n"));
        out.push_str(&text[line_start..]);
    } else {
        out.push_str(text[..close].trim_end());
        out.push_str(&format!("\n{indent}{line}\n{closing_indent}"));
        out.push_str(&text[close..]);
    }
    Some(out)
}

fn leading_ws(s: &str) -> usize {
    s.len() - s.trim_start_matches([' ', '\t']).len()
}

/// Byte offset of the `}` closing `part NAME { ... }`, or the model's block
/// for `None`. Skips comments and strings.
fn block_close(text: &str, part: Option<&str>) -> Option<usize> {
    let code = blank_comments_and_strings(text);
    let keyword_at = match part {
        Some(name) => find_word_pair(&code, "part", name)?,
        None => find_word(&code, "model")?,
    };
    let open = keyword_at + code[keyword_at..].find('{')?;
    let mut depth = 0usize;
    for (i, c) in code[open..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(open + i);
                }
            }
            _ => {}
        }
    }
    None
}

/// `text` with comments and string contents replaced by spaces, offsets kept.
fn blank_comments_and_strings(text: &str) -> String {
    let b = text.as_bytes();
    let mut out = b.to_vec();
    let mut i = 0;
    while i < b.len() {
        if b[i..].starts_with(b"//") {
            while i < b.len() && b[i] != b'\n' {
                out[i] = b' ';
                i += 1;
            }
        } else if b[i..].starts_with(b"/*") {
            while i < b.len() && !b[i..].starts_with(b"*/") {
                if b[i] != b'\n' {
                    out[i] = b' ';
                }
                i += 1;
            }
            for _ in 0..2.min(b.len() - i) {
                out[i] = b' ';
                i += 1;
            }
        } else if b[i] == b'"' {
            i += 1;
            while i < b.len() && b[i] != b'"' {
                if b[i] != b'\n' {
                    out[i] = b' ';
                }
                i += 1;
            }
            i += 1;
        } else {
            i += 1;
        }
    }
    // Only ASCII bytes were replaced with ASCII spaces.
    String::from_utf8(out).unwrap_or_else(|_| text.to_string())
}

fn is_word_byte(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_'
}

fn find_word(code: &str, word: &str) -> Option<usize> {
    let b = code.as_bytes();
    code.match_indices(word).map(|(i, _)| i).find(|&i| {
        (i == 0 || !is_word_byte(b[i - 1]))
            && b.get(i + word.len()).is_none_or(|&c| !is_word_byte(c))
    })
}

/// Offset of `first` followed (after whitespace) by the word `second`.
fn find_word_pair(code: &str, first: &str, second: &str) -> Option<usize> {
    let b = code.as_bytes();
    let mut from = 0;
    while let Some(rel) = find_word(&code[from..], first) {
        let at = from + rel;
        let rest = &code[at + first.len()..];
        let name = rest.trim_start();
        let name_at = code.len() - name.len();
        if name.starts_with(second)
            && b.get(name_at + second.len())
                .is_none_or(|&c| !is_word_byte(c))
        {
            return Some(at);
        }
        from = at + first.len();
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const LEGACY: &str = "model Contract {\n  part flow {\n    q0 --> q1\n    q1 --> q1: +signed_by(/parties/alice.id)\n  }\n}\n";

    fn signed_by(path: &str, sign: PropertySign) -> Property {
        let mut p = Property::new_predicate_from_call_args("signed_by".into(), vec![path.into()]);
        p.sign = sign;
        p
    }

    #[test]
    fn inserts_in_the_file_style_and_keeps_the_rest() {
        let text = "// keep me\nmodel M {\n  part p {\n    a -> b [+GO] // and me\n  }\n}\n";
        let out = insert_edge(text, Some("p"), "b -> b [+STOP]").unwrap();
        assert_eq!(
            out,
            "// keep me\nmodel M {\n  part p {\n    a -> b [+GO] // and me\n    b -> b [+STOP]\n  }\n}\n"
        );
        let one_line = insert_edge("model M { part p { a -> b } }", Some("p"), "b -> a").unwrap();
        assert!(parse_content_lalrpop(&one_line).is_ok(), "{one_line}");
    }

    #[test]
    fn skips_braces_in_comments_and_strings() {
        let text = "model M {\n  part p {\n    // } not this\n    a -> b [+x(\"}\")]\n  }\n}\n";
        let out = insert_edge(text, Some("p"), "b -> b").unwrap();
        assert!(out.contains("    b -> b\n  }\n}"), "{out}");
    }

    #[test]
    fn labels_round_trip_through_the_parser() {
        let parse = |label: &str| {
            let text = format!("model M {{\n  part p {{\n    a -> a [{label}]\n  }}\n}}\n");
            let model = parse_content_lalrpop(&text).unwrap_or_else(|e| panic!("{label}: {e}"));
            model.parts[0].transitions[0].properties[0].clone()
        };
        for label in [
            "+signed_by(/parties/bob.id)",
            "-modifies(/members)",
            "+threshold(\"2\", /signers)",
            "+emitted_by(/__programs__/pool.wasm, \"bc63f0cb\")",
            "+pays_senders(\"12D3KooW:tokA\")",
            "+CREATE",
        ] {
            let p = parse(label);
            assert_eq!(
                parse(&label_text(&p)),
                p,
                "{label} printed as {}",
                label_text(&p)
            );
        }
        assert_eq!(
            label_text(&parse("+threshold(\"2\", /signers)")),
            "+threshold(\"2\", /signers)"
        );
    }

    #[test]
    fn finds_the_weakest_edge_that_is_accepted() {
        let bob = signed_by("/parties/bob.id", PropertySign::Plus);
        let not_alice = signed_by("/parties/alice.id", PropertySign::Minus);
        let mut seen = Vec::new();
        // Stand-in for the verifier: the rule needs a signer on every edge.
        let outcome = find_admitting_model(
            LEGACY,
            &["q1".into()],
            &[bob.clone(), not_alice],
            RepairLimits::default(),
            |text| {
                seen.push(text.to_string());
                if text.contains("q1 --> q1: +signed_by(/parties/bob.id)\n") {
                    Ok(())
                } else {
                    anyhow::bail!("unsigned edge")
                }
            },
        )
        .unwrap();
        let RepairOutcome::Found(repair) = outcome else {
            panic!("expected a repair");
        };
        assert_eq!(repair.line, "q1 --> q1: +signed_by(/parties/bob.id)");
        assert_eq!(repair.labels, vec![bob]);
        // Unlabeled edges were tried first: q1 to itself, to q0, to a new node.
        assert_eq!(repair.tried, 4);
        assert!(seen[0].contains("    q1 --> q1\n"), "{}", seen[0]);
        assert!(repair.model_text.ends_with(
            "    q1 --> q1: +signed_by(/parties/alice.id)\n    q1 --> q1: +signed_by(/parties/bob.id)\n  }\n}\n"
        ));
    }

    #[test]
    fn reports_the_closest_refusal_when_nothing_passes() {
        let bob = signed_by("/parties/bob.id", PropertySign::Plus);
        let outcome = find_admitting_model(
            LEGACY,
            &["q1".into()],
            &[bob],
            RepairLimits::default(),
            |text| anyhow::bail!("refused {}", text.lines().count()),
        )
        .unwrap();
        let RepairOutcome::NotFound {
            tried,
            last_refusal,
        } = outcome
        else {
            panic!("nothing should pass");
        };
        assert_eq!(tried, 6);
        assert!(last_refusal.unwrap().starts_with("refused"));
    }

    #[test]
    fn never_pairs_a_label_with_its_negation() {
        let plus = signed_by("/a.id", PropertySign::Plus);
        let minus = signed_by("/a.id", PropertySign::Minus);
        assert!(contradicts(&[plus.clone(), minus]));
        assert!(!contradicts(&[plus]));
    }
}
