//! Rule files: the text of a `RULE` action.
//!
//! ```text
//! file    := (rule | "formula" NAME "{" FORMULA "}")+
//! rule    := ("export" "default" "rule" | "rule" NAME) "{" item* "}"
//! item    := "starting_at" "$PARENT" | "formula" NAME? "{" FORMULA "}"
//! ```
//!
//! Each rule has at most one `starting_at` and at least one `formula`; a
//! top-level `formula` is a rule of its own. `//` comments may appear
//! between tokens outside a formula. Anything else is an error, not
//! skipped: text a validator does not read is a rule nobody checks. A rule
//! is anchored at the commit that adds it, so `$PARENT` is the only anchor.

/// One `rule` block, or one top-level `formula`: its name (`None` for
/// `export default rule`) and its formulas, in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleBlock {
    pub name: Option<String>,
    pub formulas: Vec<RuleFormula>,
}

/// A formula's name, when it has one, and its body, trimmed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleFormula {
    pub name: Option<String>,
    pub body: String,
}

pub fn parse_rule_file(content: &str) -> Result<Vec<RuleBlock>, String> {
    let mut s = Scanner {
        src: content,
        pos: 0,
    };
    let mut rules = Vec::new();
    s.skip_trivia()?;
    while !s.at_end() {
        rules.push(s.rule()?);
        s.skip_trivia()?;
    }
    if rules.is_empty() {
        return Err(
            "a rule file needs `rule <name> { ... }` or `export default rule { ... }`".into(),
        );
    }
    Ok(rules)
}

struct Scanner<'a> {
    src: &'a str,
    pos: usize,
}

impl<'a> Scanner<'a> {
    fn rest(&self) -> &'a str {
        &self.src[self.pos..]
    }

    fn at_end(&self) -> bool {
        self.pos >= self.src.len()
    }

    fn line(&self) -> usize {
        self.src[..self.pos].matches('\n').count() + 1
    }

    fn error<T>(&self, what: &str) -> Result<T, String> {
        let found: String = self.rest().chars().take(24).collect();
        let found = if found.is_empty() {
            "end of file".to_string()
        } else {
            format!(
                "`{}`",
                found.split_whitespace().collect::<Vec<_>>().join(" ")
            )
        };
        Err(format!(
            "line {}: expected {what}, found {found}",
            self.line()
        ))
    }

    fn skip_trivia(&mut self) -> Result<(), String> {
        loop {
            let trimmed = self.rest().trim_start();
            self.pos = self.src.len() - trimmed.len();
            if trimmed.starts_with("//") {
                self.pos += trimmed.find('\n').unwrap_or(trimmed.len());
            } else if trimmed.starts_with("/*") {
                return self.error("a rule item (block comments are not rule syntax)");
            } else {
                return Ok(());
            }
        }
    }

    fn word(&mut self) -> Option<&'a str> {
        let rest = self.rest();
        let len = rest
            .char_indices()
            .find(|&(i, c)| !(c == '_' || c.is_ascii_alphabetic() || (i > 0 && c.is_ascii_digit())))
            .map_or(rest.len(), |(i, _)| i);
        if len == 0 {
            return None;
        }
        self.pos += len;
        Some(&rest[..len])
    }

    fn name(&mut self, what: &str) -> Result<String, String> {
        self.skip_trivia()?;
        match self.word() {
            Some(n) => Ok(n.to_string()),
            None => self.error(what),
        }
    }

    fn keyword(&mut self, kw: &str) -> Result<(), String> {
        self.skip_trivia()?;
        let start = self.pos;
        match self.word() {
            Some(w) if w == kw => Ok(()),
            _ => {
                self.pos = start;
                self.error(&format!("`{kw}`"))
            }
        }
    }

    fn punct(&mut self, c: char) -> Result<(), String> {
        self.skip_trivia()?;
        if self.rest().starts_with(c) {
            self.pos += c.len_utf8();
            Ok(())
        } else {
            self.error(&format!("`{c}`"))
        }
    }

    fn rule(&mut self) -> Result<RuleBlock, String> {
        let start = self.pos;
        let name = match self.word() {
            Some("export") => {
                self.keyword("default")?;
                self.keyword("rule")?;
                None
            }
            Some("rule") => Some(self.name("a rule name")?),
            Some("formula") => {
                let name = self.name("a formula name")?;
                self.punct('{')?;
                let body = self.formula_body()?;
                return Ok(RuleBlock {
                    name: Some(name.clone()),
                    formulas: vec![RuleFormula {
                        name: Some(name),
                        body,
                    }],
                });
            }
            _ => {
                self.pos = start;
                return self
                    .error("`rule <name> {`, `export default rule {`, or `formula <name> {`");
            }
        };
        self.punct('{')?;
        let mut formulas = Vec::new();
        let mut anchored = false;
        loop {
            self.skip_trivia()?;
            if self.rest().starts_with('}') {
                self.pos += 1;
                break;
            }
            let item = self.pos;
            match self.word() {
                Some("formula") => {
                    self.skip_trivia()?;
                    let name = self.word().map(str::to_string);
                    self.punct('{')?;
                    let body = self.formula_body()?;
                    formulas.push(RuleFormula { name, body });
                }
                Some("starting_at") if !anchored => {
                    anchored = true;
                    self.skip_trivia()?;
                    let after = self
                        .rest()
                        .get("$PARENT".len()..)
                        .and_then(|r| r.chars().next());
                    if !self.rest().starts_with("$PARENT")
                        || after.is_some_and(|c| c == '_' || c.is_ascii_alphanumeric())
                    {
                        return self
                            .error("`$PARENT` (a rule is anchored at the commit that adds it)");
                    }
                    self.pos += "$PARENT".len();
                }
                Some("starting_at") => {
                    self.pos = item;
                    return self.error("one `starting_at` per rule");
                }
                _ => {
                    self.pos = item;
                    return self.error("`formula {`, `starting_at $PARENT`, or `}`");
                }
            }
        }
        if formulas.is_empty() {
            self.pos = start;
            return self.error("a rule with at least one `formula { ... }`");
        }
        Ok(RuleBlock { name, formulas })
    }

    /// The text up to the `}` that closes the formula, skipping braces in
    /// string literals.
    fn formula_body(&mut self) -> Result<String, String> {
        let start = self.pos;
        let mut depth = 1usize;
        let mut chars = self.rest().char_indices();
        while let Some((i, c)) = chars.next() {
            match c {
                '"' => loop {
                    match chars.next() {
                        Some((_, '\\')) => {
                            chars.next();
                        }
                        Some((_, '"')) => break,
                        Some(_) => {}
                        None => {
                            self.pos = start;
                            return self.error("a closing `\"` in the formula");
                        }
                    }
                },
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        let body = self.src[start..start + i].trim().to_string();
                        self.pos = start + i + 1;
                        if body.is_empty() {
                            self.pos = start;
                            return self.error("a formula");
                        }
                        return Ok(body);
                    }
                }
                _ => {}
            }
        }
        self.pos = start;
        self.error("the `}` that closes this formula")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn formulas(src: &str) -> Vec<String> {
        parse_rule_file(src)
            .unwrap()
            .into_iter()
            .flat_map(|r| r.formulas)
            .map(|f| f.body)
            .collect()
    }

    #[test]
    fn reads_every_documented_shape() {
        assert_eq!(
            formulas("export default rule {\n  starting_at $PARENT\n  formula {\n    always(true)\n  }\n}\n"),
            vec!["always(true)"]
        );
        assert_eq!(formulas("rule r { formula { true } }"), vec!["true"]);
        let two = parse_rule_file(
            "// two rules\nrule a { formula { true } formula { false } }\nrule b {\n  formula { <+POST> true } // why\n}",
        )
        .unwrap();
        assert_eq!(two[0].name.as_deref(), Some("a"));
        let bodies = |r: &RuleBlock| {
            r.formulas
                .iter()
                .map(|f| f.body.clone())
                .collect::<Vec<_>>()
        };
        assert_eq!(bodies(&two[0]), vec!["true", "false"]);
        assert_eq!(bodies(&two[1]), vec!["<+POST> true"]);
        assert_eq!(
            formulas(r#"rule r { formula { <+text_eq(/t.text, "a}b")> true } }"#),
            vec![r#"<+text_eq(/t.text, "a}b")> true"#]
        );
    }

    #[test]
    fn reads_named_and_top_level_formulas() {
        let named = parse_rule_file("rule r {\n  formula SimpleRule {\n    true\n  }\n}").unwrap();
        assert_eq!(named[0].formulas[0].name.as_deref(), Some("SimpleRule"));
        assert_eq!(named[0].formulas[0].body, "true");
        let acme = parse_rule_file(include_str!(
            "../../../experiments/ietf-autoformalization/rfc8555-acme/rules/governance.modality"
        ))
        .unwrap();
        assert_eq!(acme.len(), 14);
        assert_eq!(
            acme[0].name.as_deref(),
            Some("finalize_requires_authorization")
        );
        assert!(acme.iter().all(|r| r.formulas.len() == 1));
    }

    #[test]
    fn refuses_what_it_does_not_read() {
        for (src, why) in [
            ("", "a rule file needs"),
            (
                "export default rule {\n  formla {\n    false\n  }\n}\n",
                "line 2: expected `formula {`",
            ),
            ("rule r { }", "at least one `formula"),
            (
                "rule r { formula { true }",
                "expected `formula {`, `starting_at $PARENT`, or `}`, found end of file",
            ),
            (
                "rule r { formula { true } } trailing",
                "expected `rule <name> {`",
            ),
            ("rule r { starting_at $ROOT formula { true } }", "`$PARENT`"),
            (
                "rule r { starting_at $PARENT starting_at $PARENT formula { true } }",
                "one `starting_at`",
            ),
            ("rule r { formula { } }", "expected a formula"),
            ("rule r { formula { \"open } }", "closing `\"`"),
            ("/* c */ rule r { formula { true } }", "block comments"),
            (
                "// the formula { false } is here\nrul r { formula { true } }",
                "expected `rule <name> {`",
            ),
        ] {
            let err = parse_rule_file(src).expect_err(src);
            assert!(err.contains(why), "{src:?}: {err}");
        }
    }
}
