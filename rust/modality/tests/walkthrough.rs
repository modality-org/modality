//! The model-checking and proving walkthroughs, run command by command.
//!
//! A `.walk` file is a sequence of blocks, all in one fresh directory:
//!
//! ```text
//! === file leftpad.modality
//! <contents>
//! === run check leftpad.modality
//! <expected stdout, then a last line `exit N`>
//! ```
//!
//! `UPDATE_WALKTHROUGH=1 cargo test -p modality --test walkthrough` writes
//! the outputs back into the files instead of comparing.

use std::path::Path;
use std::process::Command;

enum Block {
    File { name: String, body: String },
    Run { args: String, expected: String },
}

fn blocks(text: &str) -> Vec<Block> {
    let mut out = Vec::new();
    let mut cur: Option<(String, Vec<&str>)> = None;
    let flush = |cur: &mut Option<(String, Vec<&str>)>, out: &mut Vec<Block>| {
        if let Some((head, lines)) = cur.take() {
            let mut body = lines.join("\n");
            if !body.is_empty() {
                body.push('\n');
            }
            if let Some(name) = head.strip_prefix("file ") {
                out.push(Block::File { name: name.trim().to_string(), body });
            } else if let Some(args) = head.strip_prefix("run ") {
                out.push(Block::Run { args: args.trim().to_string(), expected: body });
            }
        }
    };
    for line in text.lines() {
        if let Some(head) = line.strip_prefix("=== ") {
            flush(&mut cur, &mut out);
            cur = Some((head.to_string(), Vec::new()));
        } else if let Some((_, lines)) = cur.as_mut() {
            lines.push(line);
        }
    }
    flush(&mut cur, &mut out);
    out
}

/// Arguments split on spaces, except inside double quotes.
fn split_args(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    for c in s.chars() {
        match c {
            '"' => quoted = !quoted,
            ' ' if !quoted => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
            }
            c => cur.push(c),
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

fn walk(name: &str) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/walkthrough").join(name);
    let text = std::fs::read_to_string(&path).unwrap();
    let dir = std::env::temp_dir().join(format!("modality-walk-{}-{}", name, std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let update = std::env::var("UPDATE_WALKTHROUGH").is_ok();
    let mut rewritten = String::new();
    let mut failures = Vec::new();
    for b in blocks(&text) {
        match b {
            Block::File { name, body } => {
                let file = dir.join(&name);
                std::fs::create_dir_all(file.parent().unwrap()).unwrap();
                std::fs::write(file, &body).unwrap();
                rewritten.push_str(&format!("=== file {name}\n{body}"));
            }
            Block::Run { args, expected } => {
                let out = Command::new(env!("CARGO_BIN_EXE_modality"))
                    .args(split_args(&args))
                    .current_dir(&dir)
                    .output()
                    .unwrap();
                let mut got = String::from_utf8_lossy(&out.stdout).to_string();
                if !out.status.success() || !out.stderr.is_empty() {
                    got.push_str(&String::from_utf8_lossy(&out.stderr));
                }
                got.push_str(&format!("exit {}\n", out.status.code().unwrap_or(-1)));
                if got != expected {
                    failures.push(format!("$ modal {args}\n--- expected\n{expected}--- got\n{got}"));
                }
                rewritten.push_str(&format!("=== run {args}\n{got}"));
            }
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
    if update {
        std::fs::write(&path, rewritten).unwrap();
        return;
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn model_checking_walkthrough() {
    walk("model-checking.walk");
}

#[test]
fn proving_walkthrough() {
    walk("proving.walk");
}
