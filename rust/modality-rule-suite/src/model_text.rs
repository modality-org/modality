//! Line-level edits to a model's source: adding leak edges and making the
//! probe models G5 compares rules on. Edits are textual so the result reads
//! the way the case author wrote it; every result is re-parsed by the real
//! checker before it counts.

/// One `from --> to: literals` line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edge {
    pub line: usize,
    pub indent: String,
    pub from: String,
    pub to: String,
    pub literals: Vec<String>,
}

impl Edge {
    pub fn render(&self) -> String {
        if self.literals.is_empty() {
            format!("{}{} --> {}", self.indent, self.from, self.to)
        } else {
            format!(
                "{}{} --> {}: {}",
                self.indent,
                self.from,
                self.to,
                self.literals.join(" ")
            )
        }
    }
}

pub fn edges(model: &str) -> Vec<Edge> {
    model
        .lines()
        .enumerate()
        .filter_map(|(line, text)| parse_edge(line, text))
        .collect()
}

fn parse_edge(line: usize, text: &str) -> Option<Edge> {
    let (head, labels) = match text.split_once(':') {
        Some((h, l)) if h.contains("-->") => (h, l),
        _ => (text, ""),
    };
    let (from, to) = head.split_once("-->")?;
    let indent: String = text.chars().take_while(|c| c.is_whitespace()).collect();
    let from = from.trim();
    let to = to.trim();
    if from.is_empty() || to.is_empty() || from.contains(' ') || to.contains(' ') {
        return None;
    }
    Some(Edge {
        line,
        indent,
        from: from.to_string(),
        to: to.to_string(),
        literals: split_literals(labels),
    })
}

/// Split edge labels at top-level whitespace, keeping `f(a, "b c")` whole.
pub fn split_literals(labels: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut depth = 0i32;
    let mut quoted = false;
    for c in labels.chars() {
        match c {
            '"' => quoted = !quoted,
            '(' if !quoted => depth += 1,
            ')' if !quoted => depth -= 1,
            _ => {}
        }
        if c.is_whitespace() && depth == 0 && !quoted {
            if !current.is_empty() {
                out.push(std::mem::take(&mut current));
            }
        } else {
            current.push(c);
        }
    }
    if !current.is_empty() {
        out.push(current);
    }
    out
}

/// `+name(a, b)` as sign, name and arguments (`None` for a bare label).
pub fn parse_literal(lit: &str) -> Option<(char, String, Option<Vec<String>>)> {
    let mut chars = lit.chars();
    let sign = chars.next().filter(|c| *c == '+' || *c == '-')?;
    let rest: String = chars.collect();
    match rest.split_once('(') {
        Some((name, args)) => {
            let args = args.strip_suffix(')')?;
            Some((sign, name.to_string(), Some(split_args(args))))
        }
        None => Some((sign, rest, None)),
    }
}

fn split_args(args: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut depth = 0i32;
    let mut quoted = false;
    for c in args.chars() {
        match c {
            '"' => quoted = !quoted,
            '(' if !quoted => depth += 1,
            ')' if !quoted => depth -= 1,
            ',' if depth == 0 && !quoted => {
                out.push(current.trim().to_string());
                current.clear();
                continue;
            }
            _ => {}
        }
        current.push(c);
    }
    if !current.trim().is_empty() {
        out.push(current.trim().to_string());
    }
    out
}

pub fn render_literal(sign: char, name: &str, args: &Option<Vec<String>>) -> String {
    match args {
        Some(args) => format!("{sign}{name}({})", args.join(", ")),
        None => format!("{sign}{name}"),
    }
}

/// The model with `lines` added after its last edge, at that edge's indent.
pub fn with_edges(model: &str, lines: &str) -> Option<String> {
    let last = edges(model).into_iter().last()?;
    let mut out: Vec<String> = model.lines().map(str::to_string).collect();
    let added: Vec<String> = lines
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(|l| format!("{}{l}", last.indent))
        .collect();
    for (i, l) in added.into_iter().enumerate() {
        out.insert(last.line + 1 + i, l);
    }
    Some(out.join("\n") + "\n")
}

fn with_line(model: &str, line: usize, replacement: &str) -> String {
    let mut out: Vec<String> = model.lines().map(str::to_string).collect();
    out[line] = replacement.to_string();
    out.join("\n") + "\n"
}

/// A probe model and how it was made from the reference witness.
#[derive(Debug, Clone)]
pub struct Probe {
    pub label: String,
    pub model: String,
}

/// Small edits of `model`: drop or flip one literal, swap one signer key
/// for another, weaken `all_signed` to `any_signed` and back, widen a path
/// to its parent, add an unlabeled self-loop. Deduplicated, in a fixed
/// order, at most `cap`.
pub fn mutations(model: &str, signer_paths: &[String], cap: usize) -> Vec<Probe> {
    let mut probes: Vec<Probe> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut push = |label: String, text: String, probes: &mut Vec<Probe>| {
        if text != model && seen.insert(text.clone()) {
            probes.push(Probe { label, model: text });
        }
    };
    let all = edges(model);
    for edge in &all {
        let shown = edge.render().trim().to_string();
        for (i, lit) in edge.literals.iter().enumerate() {
            let mut dropped = edge.clone();
            dropped.literals.remove(i);
            push(
                format!("drop `{lit}` from `{shown}`"),
                with_line(model, edge.line, &dropped.render()),
                &mut probes,
            );
            let Some((sign, name, args)) = parse_literal(lit) else {
                continue;
            };
            let flipped_sign = if sign == '+' { '-' } else { '+' };
            let mut flipped = edge.clone();
            flipped.literals[i] = render_literal(flipped_sign, &name, &args);
            push(
                format!("flip `{lit}` in `{shown}`"),
                with_line(model, edge.line, &flipped.render()),
                &mut probes,
            );
            let swap_name = match name.as_str() {
                "all_signed" => Some("any_signed"),
                "any_signed" => Some("all_signed"),
                _ => None,
            };
            if let Some(other) = swap_name {
                let mut swapped = edge.clone();
                swapped.literals[i] = render_literal(sign, other, &args);
                push(
                    format!("`{name}` to `{other}` in `{shown}`"),
                    with_line(model, edge.line, &swapped.render()),
                    &mut probes,
                );
            }
            if let Some(args) = &args {
                if name == "signed_by" {
                    for other in signer_paths.iter().filter(|p| Some(*p) != args.first()) {
                        let mut swapped = edge.clone();
                        let mut new_args = args.clone();
                        new_args[0] = other.clone();
                        swapped.literals[i] = render_literal(sign, &name, &Some(new_args));
                        push(
                            format!("sign with `{other}` instead in `{shown}`"),
                            with_line(model, edge.line, &swapped.render()),
                            &mut probes,
                        );
                    }
                }
                if matches!(name.as_str(), "modifies" | "post_to_path") {
                    if let Some(parent) = args.first().and_then(|p| parent_path(p)) {
                        let mut widened = edge.clone();
                        let mut new_args = args.clone();
                        new_args[0] = parent.clone();
                        widened.literals[i] = render_literal(sign, &name, &Some(new_args));
                        push(
                            format!("widen `{lit}` to `{parent}` in `{shown}`"),
                            with_line(model, edge.line, &widened.render()),
                            &mut probes,
                        );
                    }
                }
            }
        }
    }
    let mut targets: Vec<&str> = all.iter().map(|e| e.to.as_str()).collect();
    targets.sort();
    targets.dedup();
    for node in targets {
        if let Some(text) = with_edges(model, &format!("{node} --> {node}")) {
            push(
                format!("add unlabeled `{node} --> {node}`"),
                text,
                &mut probes,
            );
        }
    }
    probes.truncate(cap);
    probes
}

fn parent_path(path: &str) -> Option<String> {
    let trimmed = path.trim_end_matches('/');
    let (parent, _) = trimmed.rsplit_once('/')?;
    if parent.is_empty() {
        None
    } else {
        Some(parent.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const W: &str = "model Contract {\n  part flow {\n    q0 --> q1\n    q1 --> q1: +any_signed(/members) -modifies(/members)\n  }\n}\n";

    #[test]
    fn parses_edges_and_literals() {
        let e = edges(W);
        assert_eq!(e.len(), 2);
        assert!(e[0].literals.is_empty());
        assert_eq!(
            e[1].literals,
            vec!["+any_signed(/members)", "-modifies(/members)"]
        );
        assert_eq!(
            split_literals(r#"+num_gt(/x.num, "5") -text_eq(/s.text, "a b")"#),
            vec![r#"+num_gt(/x.num, "5")"#, r#"-text_eq(/s.text, "a b")"#]
        );
        assert_eq!(
            parse_literal(r#"+num_gt(/x.num, "5")"#),
            Some((
                '+',
                "num_gt".into(),
                Some(vec!["/x.num".into(), "\"5\"".into()])
            ))
        );
        assert_eq!(parse_literal("-POST"), Some(('-', "POST".into(), None)));
    }

    #[test]
    fn adds_edges_after_the_last_one() {
        let leaked = with_edges(W, "q1 --> q1: +any_signed(/members)").unwrap();
        assert!(leaked.contains("-modifies(/members)\n    q1 --> q1: +any_signed(/members)\n  }"));
    }

    #[test]
    fn mutations_are_distinct_edits() {
        let probes = mutations(W, &[], 100);
        let labels: Vec<_> = probes.iter().map(|p| p.label.as_str()).collect();
        assert!(labels
            .iter()
            .any(|l| l.starts_with("drop `-modifies(/members)`")));
        assert!(labels
            .iter()
            .any(|l| l.starts_with("`any_signed` to `all_signed`")));
        assert!(labels
            .iter()
            .any(|l| l.starts_with("add unlabeled `q1 --> q1`")));
        assert!(probes.iter().all(|p| p.model != W));
    }
}
