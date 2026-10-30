//! Output shared by `modal check`, `run`, `prove` and `test`.

use modality_lang::data::label::{Kind, Label};
use modality_lang::data::{short_name, State, Term, Value};

pub const WIDTH: usize = 68;

/// `✓ name ......... holds`, the verdict at the right edge.
pub fn verdict_line(sym: &str, name: &str, word: &str) -> String {
    let left = format!("{sym} {name}");
    let used = left.chars().count() + word.chars().count();
    let pad = WIDTH.saturating_sub(used).max(2);
    format!("{left}{}{word}", " ".repeat(pad))
}

/// A summary with `exit N` at the right edge.
pub fn summary_line(text: &str, exit: i32) -> String {
    verdict_line("", text, &format!("exit {exit}"))
        .trim_start()
        .to_string()
}

pub fn thousands(n: u128) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// Short names for paths: `width` for `/in/width.integer` when unique.
#[derive(Clone)]
pub struct Names {
    all: Vec<String>,
}

impl Names {
    pub fn new(all: Vec<String>) -> Self {
        Names { all }
    }

    pub fn short(&self, path: &str) -> String {
        short_name(path, &self.all)
    }

    pub fn term(&self, t: &Term) -> String {
        t.show(&|p| self.short(p))
    }

    /// `width = 1, string = ""` for the given paths present in `state`.
    pub fn assignments(&self, state: &State, paths: &[String]) -> String {
        paths
            .iter()
            .filter_map(|p| state.get(p).map(|v| format!("{} = {v}", self.short(p))))
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// A guard as it reads: `added < pad`, `len(fill) = 1`.
    pub fn guard(&self, l: &Label) -> String {
        if let Kind::Pred { name, args } = &l.kind {
            let op = match name.as_str() {
                "num_eq" => Some(if l.plus { "=" } else { "≠" }),
                "num_lt" => Some(if l.plus { "<" } else { "≥" }),
                "num_lte" => Some(if l.plus { "≤" } else { ">" }),
                "num_gt" => Some(if l.plus { ">" } else { "≤" }),
                "num_gte" => Some(if l.plus { "≥" } else { "<" }),
                _ => None,
            };
            if let (Some(op), 2) = (op, args.len()) {
                return format!("{} {op} {}", self.term(&args[0]), self.term(&args[1]));
            }
        }
        let sign = if l.plus { "" } else { "not " };
        format!("{sign}{}", self.label(l))
    }

    /// The label as written, with short names: `num_eq(len(result), …)`.
    pub fn label(&self, l: &Label) -> String {
        let text = l.text();
        let mut out = text.clone();
        for p in &self.all {
            if out.contains(p.as_str()) {
                out = out.replace(p.as_str(), &self.short(p));
            }
        }
        out.trim_start_matches(['+', '-']).to_string()
    }
}

/// Why a data label is true or false of `state`, in words.
pub fn explain_pred(names: &Names, l: &Label, state: &State) -> Option<String> {
    let Kind::Pred { name, args } = &l.kind else {
        return None;
    };
    let shown: Vec<String> = args.iter().map(|a| names.term(a)).collect();
    let vals: Vec<Option<Value>> = args.iter().map(|a| a.eval(state)).collect();
    let v = |i: usize| {
        vals.get(i)
            .cloned()
            .flatten()
            .map(|v| v.to_string())
            .unwrap_or_else(|| "nothing".into())
    };
    let pair = |i: usize| {
        if matches!(args[i], Term::Lit(_)) {
            shown[i].clone()
        } else {
            format!("{} = {}", shown[i], v(i))
        }
    };
    let holds = modality_lang::data::label::pred_holds(name, args, state);
    Some(match name.as_str() {
        "num_eq" | "num_gt" | "num_gte" | "num_lt" | "num_lte" | "text_eq" => {
            if holds {
                format!("{}, {}", pair(0), pair(1))
            } else {
                format!("{}, but {}", pair(0), pair(1))
            }
        }
        "text_prefix_repeats" => {
            let k = vals.get(1).cloned().flatten().and_then(|v| v.as_num());
            let len = vals.first().cloned().flatten().and_then(|v| v.as_text().map(|t| t.chars().count()));
            match (k, len) {
                (Some(k), Some(n)) if k.is_integer() && k.num() > n as i128 => {
                    format!("{} has fewer than {} character{}", v(0), v(1), if k.num() == 1 { "" } else { "s" })
                }
                (Some(k), _) if k.num() == 1 && k.is_integer() => {
                    format!("the first character of {} is not {}", v(0), v(2))
                }
                _ => format!("the first {} characters of {} are not each {}", v(1), v(0), v(2)),
            }
        }
        "text_ends_with" => format!("{} does not end with {}", v(0), v(1)),
        "text_starts_with" => format!("{} does not start with {}", v(0), v(1)),
        "text_contains" => format!("{} does not contain {}", v(0), v(1)),
        _ => shown
            .iter()
            .enumerate()
            .map(|(i, _)| pair(i))
            .collect::<Vec<_>>()
            .join(", "),
    })
}

/// `"1 formula holds"`, `"3 formulas and 1 write undecided"`.
pub fn count_phrase(formulas: usize, writes: usize, word: &str) -> String {
    let f = format!("{formulas} formula{}", if formulas == 1 { "" } else { "s" });
    let mut s = if writes > 0 {
        if formulas > 0 {
            format!("{f} and {writes} write{}", if writes == 1 { "" } else { "s" })
        } else {
            format!("{writes} write{}", if writes == 1 { "" } else { "s" })
        }
    } else {
        f
    };
    let n = formulas + writes;
    if word == "hold" {
        s.push_str(if n == 1 { " holds" } else { " hold" });
    } else {
        s.push(' ');
        s.push_str(word);
    }
    s
}

/// A source line with its number, and a caret under `col..col+len`.
pub fn snippet(line_no: usize, line: &str, col: usize, len: usize, note: &str) -> Vec<String> {
    let w = line_no.to_string().len();
    let pad = " ".repeat(w);
    vec![
        format!("{pad} |"),
        format!("{line_no} | {line}"),
        format!(
            "{pad} | {}{} {note}",
            " ".repeat(col.saturating_sub(1)),
            "^".repeat(len.max(1))
        ),
        format!("{pad} |"),
    ]
}
