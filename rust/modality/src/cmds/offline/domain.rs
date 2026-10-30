//! Search domains: `domains.toml`.
//!
//! ```toml
//! depth = 12
//! [domain]
//! "/in/c.text" = ["0", "ab"]
//! "/in/s.text" = { alphabet = ["a", "b"], max_len = 3 }
//! [havoc]
//! "/out/done.bool" = [true, false]
//! ```

use anyhow::{anyhow, bail, Result};
use modality_lang::data::inductive::text_values;
use modality_lang::data::{Domain, PathType, Value};

/// A domain value that does not fit its path: where it is, and why.
pub struct DomainError {
    pub line: usize,
    pub col: usize,
    pub message: String,
    pub help: String,
}

fn raw_of(v: &toml::Value) -> Option<String> {
    Some(match v {
        toml::Value::String(s) => s.clone(),
        toml::Value::Integer(i) => i.to_string(),
        toml::Value::Boolean(b) => b.to_string(),
        toml::Value::Float(f) => f.to_string(),
        _ => return None,
    })
}

/// The position of the first occurrence of `needle` after `key` in `text`.
fn locate(text: &str, key: &str, needle: &str) -> (usize, usize) {
    let quoted = format!("\"{key}\"");
    let Some(k) = text.find(&quoted) else {
        return (1, 1);
    };
    let at = text[k..].find(needle).map(|i| k + i).unwrap_or(k);
    let before = &text[..at];
    let line = before.matches('\n').count() + 1;
    let col = before.rsplit('\n').next().unwrap_or("").chars().count() + 1;
    (line, col)
}

fn entries(
    text: &str,
    table: Option<&toml::Value>,
    errors: &mut Vec<DomainError>,
) -> Result<Vec<(String, Vec<Value>)>> {
    let Some(table) = table else {
        return Ok(Vec::new());
    };
    let table = table
        .as_table()
        .ok_or_else(|| anyhow!("a domain section is a table of paths"))?;
    let mut out = Vec::new();
    for (path, spec) in table {
        let ty = PathType::of(path);
        let values = match spec {
            toml::Value::Array(items) => {
                let mut vs = Vec::new();
                for item in items {
                    let raw = raw_of(item).ok_or_else(|| anyhow!("{path}: a value is a string, number or bool"))?;
                    match ty.parse_value(&raw) {
                        Ok(v) => vs.push(v),
                        Err(e) => {
                            let shown = match item {
                                toml::Value::String(s) => format!("\"{s}\""),
                                other => other.to_string(),
                            };
                            let (line, col) = locate(text, path, &shown);
                            let reason = if ty.num_kind().is_some() && raw.contains('/') {
                                "the type already rules this input out; remove it".to_string()
                            } else {
                                format!("{path} holds {} values", ty.name())
                            };
                            errors.push(DomainError {
                                line,
                                col,
                                message: format!("{shown} is not a{} {}", if ty == PathType::Integer { "n" } else { "" }, ty.name()),
                                help: reason,
                            });
                            let _ = e;
                        }
                    }
                }
                vs
            }
            toml::Value::Table(t) => {
                let alphabet: Vec<String> = t
                    .get("alphabet")
                    .and_then(|a| a.as_array())
                    .ok_or_else(|| anyhow!("{path}: give a list of values, or {{ alphabet = [...], max_len = N }}"))?
                    .iter()
                    .filter_map(|a| a.as_str().map(str::to_string))
                    .collect();
                let max_len = t
                    .get("max_len")
                    .and_then(|m| m.as_integer())
                    .ok_or_else(|| anyhow!("{path}: max_len is a whole number"))?;
                if !ty.is_textual() && ty != PathType::Any {
                    bail!("{path}: an alphabet is for text paths");
                }
                text_values(&alphabet, max_len.max(0) as usize)
            }
            _ => bail!("{path}: give a list of values, or {{ alphabet = [...], max_len = N }}"),
        };
        out.push((path.clone(), values));
    }
    // In the order the file lists them.
    out.sort_by_key(|(p, _)| text.find(&format!("\"{p}\"")).unwrap_or(usize::MAX));
    Ok(out)
}

pub fn parse(text: &str) -> Result<(Domain, Vec<DomainError>)> {
    let doc: toml::Value = toml::from_str(text).map_err(|e| anyhow!("{e}"))?;
    let depth = doc
        .get("depth")
        .and_then(|d| d.as_integer())
        .unwrap_or(12)
        .max(0) as usize;
    let mut errors = Vec::new();
    let values = entries(text, doc.get("domain"), &mut errors)?;
    let havoc = entries(text, doc.get("havoc"), &mut errors)?;
    Ok((
        Domain {
            depth,
            values,
            havoc,
        },
        errors,
    ))
}
