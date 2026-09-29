//! Every complete model, rule and formula block in the public docs parses
//! with the parser governance uses. Fragments (a bare formula, a grammar
//! sketch) are skipped: a block is checked when a line starts a `model`,
//! `rule` or `formula` block.

use std::path::{Path, PathBuf};

const SKIP_DIRS: &[&str] = &["progress", "resources", "rfcs", "roadmaps"];

fn markdown_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            let name = path.file_name().unwrap().to_string_lossy();
            if !SKIP_DIRS.contains(&name.as_ref()) {
                markdown_files(&path, out);
            }
        } else if path.extension().is_some_and(|e| e == "md") {
            out.push(path);
        }
    }
}

/// `(first line number, text)` of each fenced `modality` block.
fn modality_blocks(markdown: &str) -> Vec<(usize, String)> {
    let mut blocks = Vec::new();
    let mut current: Option<(usize, String)> = None;
    for (index, line) in markdown.lines().enumerate() {
        let fence = line.trim_start();
        match &mut current {
            None if fence.starts_with("```modality") => current = Some((index + 2, String::new())),
            Some(_) if fence.starts_with("```") => blocks.push(current.take().unwrap()),
            Some((_, text)) => {
                text.push_str(line);
                text.push('\n');
            }
            None => {}
        }
    }
    blocks
}

fn starts(block: &str, keyword: &str) -> bool {
    block.lines().any(|line| {
        let line = line.trim_start();
        let line = line.strip_prefix("export default ").unwrap_or(line);
        line.strip_prefix(keyword)
            .is_some_and(|rest| rest.starts_with(' ') || rest.starts_with('{'))
    })
}

/// A grammar sketch such as `model <name> { ... }`.
fn is_sketch(block: &str) -> bool {
    block.split('<').skip(1).any(|rest| {
        let name: String = rest.chars().take_while(|c| *c != '>').collect();
        !name.is_empty()
            && rest.len() > name.len()
            && name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
    })
}

fn check(block: &str) -> Result<(), String> {
    let has_rule = starts(block, "rule");
    let has_model = starts(block, "model");
    let has_formula = starts(block, "formula") && !has_rule;
    if has_rule {
        let rules: String = split_top_level(block)
            .into_iter()
            .filter(|item| starts(item, "rule"))
            .collect::<Vec<_>>()
            .join("\n");
        for rule in modality_lang::rule_file::parse_rule_file(&rules)? {
            for formula in rule.formulas {
                let decl = format!("formula r {{\n{}\n}}", formula.body);
                let parsed = modality_lang::FormulaParser::new()
                    .parse(&decl)
                    .map_err(|err| format!("formula `{}`: {err:?}", formula.body))?;
                modality_lang::vars::check_formula(&parsed)
                    .map_err(|err| format!("formula `{}`: {err}", formula.body))?;
            }
        }
    }
    if has_model {
        let models: String = split_top_level(block)
            .into_iter()
            .filter(|item| starts(item, "model"))
            .collect::<Vec<_>>()
            .join("\n");
        modality_lang::parse_all_models_content_lalrpop(&models).map(|_| ())?;
    }
    if has_formula {
        modality_lang::parse_all_formulas_content_lalrpop(block).map(|_| ())?;
    }
    Ok(())
}

/// Top-level `{ ... }` items with the lines before each, so a block that
/// shows a model and a rule is checked with each parser.
fn split_top_level(block: &str) -> Vec<String> {
    let mut items = Vec::new();
    let mut current = String::new();
    let mut depth = 0i32;
    for line in block.lines() {
        current.push_str(line);
        current.push('\n');
        let code = line.split("//").next().unwrap_or("");
        depth += code.matches('{').count() as i32 - code.matches('}').count() as i32;
        if depth == 0 && code.contains('}') {
            items.push(std::mem::take(&mut current));
        }
    }
    if !current.trim().is_empty() {
        items.push(current);
    }
    items
}

#[test]
fn every_documented_model_rule_and_formula_parses() {
    let docs = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs");
    let mut files = Vec::new();
    markdown_files(&docs, &mut files);
    files.sort();
    let mut checked = 0;
    let mut failures = Vec::new();
    for file in &files {
        let markdown = std::fs::read_to_string(file).unwrap();
        for (line, block) in modality_blocks(&markdown) {
            if !(starts(&block, "rule") || starts(&block, "model") || starts(&block, "formula"))
                || is_sketch(&block)
            {
                continue;
            }
            checked += 1;
            if let Err(err) = check(&block) {
                let name = file.strip_prefix(&docs).unwrap().display();
                failures.push(format!("docs/{name}:{line}: {err}"));
            }
        }
    }
    eprintln!("checked {checked} blocks");
    assert!(checked > 30, "only {checked} blocks found");
    assert!(failures.is_empty(), "{} blocks do not parse:\n{}", failures.len(), failures.join("\n"));
}
