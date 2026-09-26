//! No dependency ships under the GPL or the AGPL. Every package in the Rust dependency tree
//! (`cargo metadata`) and in the browser suite's lock file (`e2e/package-lock.json`) names a
//! license expression, and at least one of the alternatives it offers is free of both. LGPL
//! is allowed.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// One node of a license expression.
#[derive(Debug)]
enum Expr {
    License(String),
    And(Vec<Expr>),
    Or(Vec<Expr>),
}

/// Splits an expression into words and parentheses. The legacy `/` separator of older crates
/// ("MIT/Apache-2.0") means OR.
fn tokens(text: &str) -> Vec<String> {
    text.replace('(', " ( ")
        .replace(')', " ) ")
        .replace('/', " OR ")
        .split_whitespace()
        .map(str::to_owned)
        .collect()
}

/// Parses `or := and (OR and)*`, `and := term (AND term)*`, `term := ( or ) | id [WITH id]`.
struct Parser {
    tokens: Vec<String>,
    at: usize,
}

impl Parser {
    fn peek(&self) -> Option<&str> {
        self.tokens.get(self.at).map(String::as_str)
    }

    fn next(&mut self) -> Result<String, String> {
        let token = self
            .tokens
            .get(self.at)
            .cloned()
            .ok_or("the expression ends early")?;
        self.at += 1;
        Ok(token)
    }

    fn or(&mut self) -> Result<Expr, String> {
        let mut parts = vec![self.and()?];
        while self.peek().is_some_and(|t| t.eq_ignore_ascii_case("OR")) {
            self.at += 1;
            parts.push(self.and()?);
        }
        Ok(if parts.len() == 1 {
            parts.remove(0)
        } else {
            Expr::Or(parts)
        })
    }

    fn and(&mut self) -> Result<Expr, String> {
        let mut parts = vec![self.term()?];
        while self.peek().is_some_and(|t| t.eq_ignore_ascii_case("AND")) {
            self.at += 1;
            parts.push(self.term()?);
        }
        Ok(if parts.len() == 1 {
            parts.remove(0)
        } else {
            Expr::And(parts)
        })
    }

    fn term(&mut self) -> Result<Expr, String> {
        let token = self.next()?;
        if token == "(" {
            let inner = self.or()?;
            if self.next()? != ")" {
                return Err("a parenthesis is not closed".into());
            }
            return Ok(inner);
        }
        if token == ")" || token.eq_ignore_ascii_case("OR") || token.eq_ignore_ascii_case("AND") {
            return Err(format!("unexpected {token}"));
        }
        // An exception narrows a license; it never makes a GPL license permissive.
        if self.peek().is_some_and(|t| t.eq_ignore_ascii_case("WITH")) {
            self.at += 1;
            self.next()?;
        }
        Ok(Expr::License(token))
    }
}

fn parse(text: &str) -> Result<Expr, String> {
    let mut parser = Parser {
        tokens: tokens(text),
        at: 0,
    };
    let expr = parser.or()?;
    if parser.at != parser.tokens.len() {
        return Err(format!("unexpected {}", parser.tokens[parser.at]));
    }
    Ok(expr)
}

/// GPL and AGPL in any version; LGPL is allowed.
fn copyleft(id: &str) -> bool {
    let id = id.to_ascii_uppercase();
    id.starts_with("GPL") || id.starts_with("AGPL")
}

/// True when the package can be used under terms free of the GPL and the AGPL.
fn acceptable(expr: &Expr) -> bool {
    match expr {
        Expr::License(id) => !copyleft(id),
        Expr::And(parts) => parts.iter().all(acceptable),
        Expr::Or(parts) => parts.iter().any(acceptable),
    }
}

/// `None` when the expression is acceptable, or the reason it is not.
fn verdict(license: Option<&str>) -> Option<String> {
    match license {
        None => Some("no license named".into()),
        Some(text) => match parse(text) {
            Err(e) => Some(format!("cannot read {text:?}: {e}")),
            Ok(expr) if acceptable(&expr) => None,
            Ok(_) => Some(format!(
                "{text} offers no alternative free of the GPL and the AGPL"
            )),
        },
    }
}

#[test]
fn expressions_are_judged_by_their_best_alternative() {
    assert!(verdict(Some("GPL-3.0-only")).is_some());
    assert!(verdict(Some("AGPL-3.0")).is_some());
    assert!(verdict(Some("MIT AND GPL-2.0")).is_some());
    assert!(verdict(Some("GPL-2.0 WITH Classpath-exception-2.0")).is_some());
    assert!(verdict(None).is_some());
    assert!(verdict(Some("MIT OR GPL-2.0")).is_none());
    assert!(verdict(Some("LGPL-2.1-or-later")).is_none());
    assert!(verdict(Some("MIT/Apache-2.0")).is_none());
    assert!(verdict(Some("(MIT OR Apache-2.0) AND Unicode-3.0")).is_none());
    assert!(verdict(Some("Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT")).is_none());
    assert!(verdict(Some("(MIT OR")).is_some());
}

fn cargo_packages() -> Vec<(String, Option<String>)> {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let out = Command::new(cargo)
        .args(["metadata", "--format-version", "1", "--offline"])
        .current_dir(repo())
        .output()
        .expect("cargo metadata runs");
    assert!(
        out.status.success(),
        "cargo metadata failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let doc: Value = serde_json::from_slice(&out.stdout).expect("cargo metadata prints JSON");
    doc["packages"]
        .as_array()
        .expect("a package list")
        .iter()
        .map(|p| {
            let name = format!(
                "{} {}",
                p["name"].as_str().unwrap_or("?"),
                p["version"].as_str().unwrap_or("?")
            );
            (name, p["license"].as_str().map(str::to_owned))
        })
        .collect()
}

fn npm_packages() -> Vec<(String, Option<String>)> {
    let path = repo().join("e2e/package-lock.json");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    let doc: Value = serde_json::from_str(&text).expect("the lock file is JSON");
    doc["packages"]
        .as_object()
        .expect("a package map")
        .iter()
        .map(|(key, p)| {
            let name = if key.is_empty() {
                "e2e".to_owned()
            } else {
                key.clone()
            };
            (name, p["license"].as_str().map(str::to_owned))
        })
        .collect()
}

#[test]
fn no_dependency_is_gpl_or_agpl() {
    let packages: Vec<_> = cargo_packages().into_iter().chain(npm_packages()).collect();
    assert!(
        packages.len() > 100,
        "only {} packages were read",
        packages.len()
    );
    let failing: Vec<String> = packages
        .iter()
        .filter_map(|(name, license)| {
            verdict(license.as_deref()).map(|why| format!("{name}: {why}"))
        })
        .collect();
    assert!(
        failing.is_empty(),
        "packages that fail the license rule:\n{}",
        failing.join("\n")
    );
}
