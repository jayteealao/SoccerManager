//! The documentation set exists, the command-line reference names every flag of every
//! command, and every command the tutorial and the how-to run is a real command.

use std::path::{Path, PathBuf};
use std::process::Command;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(relative: &str) -> String {
    let path = repo().join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
}

fn help(args: &[&str]) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_engine-cli"))
        .args(args)
        .output()
        .unwrap();
    assert!(out.status.success(), "{args:?} failed");
    String::from_utf8(out.stdout).unwrap()
}

/// The commands `engine-cli --help` lists, without `help` itself.
fn commands() -> Vec<String> {
    let text = help(&["--help"]);
    let list = text.split("Commands:").nth(1).expect("a command list");
    let list = list.split("Options:").next().unwrap();
    list.lines()
        .filter_map(|line| line.split_whitespace().next())
        .filter(|name| *name != "help")
        .map(str::to_owned)
        .collect()
}

/// Every `--flag` a help text names.
fn flags(text: &str) -> Vec<String> {
    let mut found: Vec<String> = text
        .split(|c: char| c.is_whitespace() || c == ',' || c == '[' || c == ']')
        .filter(|word| word.starts_with("--") && word.len() > 2)
        .map(|word| {
            word.trim_end_matches(|c: char| !c.is_ascii_alphanumeric())
                .to_owned()
        })
        .filter(|word| word.len() > 2)
        .collect();
    found.sort();
    found.dedup();
    found
}

/// The text of the `## <name>` section of a Markdown document.
fn section<'a>(text: &'a str, name: &str) -> Option<&'a str> {
    let heading = format!("\n## {name}\n");
    let start = text.find(&heading)? + heading.len();
    let rest = &text[start..];
    Some(rest.find("\n## ").map_or(rest, |end| &rest[..end]))
}

#[test]
fn every_document_in_the_set_exists() {
    for path in [
        "README.md",
        "docs/tutorials/first-match.md",
        "docs/how-to/modding.md",
        "docs/how-to/calibration.md",
        "docs/reference/cli.md",
        "docs/reference/data-files.md",
        "docs/reference/protocol.md",
        "docs/explanation/engine.md",
    ] {
        assert!(read(path).trim().len() > 200, "{path} is empty or missing");
    }
}

#[test]
fn the_reference_names_every_flag_of_every_command() {
    let doc = read("docs/reference/cli.md");
    let global = section(&doc, "Global flags").expect("a Global flags section");
    let names = commands();
    assert!(names.len() >= 9, "only {} commands listed", names.len());
    for name in &names {
        let text =
            section(&doc, name).unwrap_or_else(|| panic!("cli.md has no section for {name}"));
        for flag in flags(&help(&[name, "--help"])) {
            assert!(
                text.contains(&format!("`{flag}`")) || global.contains(&format!("`{flag}`")),
                "cli.md does not name {flag} of {name}"
            );
        }
    }
}

#[test]
fn every_command_in_the_guides_is_a_real_command() {
    let names = commands();
    for path in [
        "docs/tutorials/first-match.md",
        "docs/how-to/modding.md",
        "docs/how-to/calibration.md",
        "README.md",
    ] {
        let text = read(path);
        let used: Vec<&str> = text
            .split("engine-cli ")
            .skip(1)
            .filter_map(|rest| rest.split_whitespace().next())
            .filter(|word| word.chars().all(|c| c.is_ascii_lowercase()))
            .collect();
        assert!(
            !used.is_empty() || path == "README.md",
            "{path} runs no command"
        );
        for word in used {
            assert!(
                names.iter().any(|n| n == word),
                "{path} runs engine-cli {word}, which does not exist"
            );
        }
    }
}
