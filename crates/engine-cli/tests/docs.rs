//! The documentation set exists, the command-line reference names every flag of every
//! command, every command the tutorial and the how-to guides run is a real command, and the
//! engine module reference lists every slot of the registry as the program declares it.

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
        "docs/how-to/replay-gate.md",
        "docs/how-to/find-a-divergence.md",
        "docs/reference/cli.md",
        "docs/reference/data-files.md",
        "docs/reference/protocol.md",
        "docs/explanation/engine.md",
        "docs/reference/engine-modules.md",
        "docs/how-to/engine-modules.md",
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
        "docs/how-to/replay-gate.md",
        "docs/how-to/find-a-divergence.md",
        "docs/how-to/engine-modules.md",
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

/// One row of the module reference's slot table: the slot id, whether it is required, the
/// default module as `name@version`, the other versions, and whether it has an off version.
#[derive(Debug, PartialEq)]
struct SlotRow {
    slot: String,
    required: bool,
    default: String,
    others: Vec<String>,
    off: bool,
}

/// The rows of the `## Slots` table of the module reference.
fn slot_rows(doc: &str) -> Vec<SlotRow> {
    let table = section(doc, "Slots").expect("a Slots section");
    let code = |cell: &str| cell.trim().trim_matches('`').to_owned();
    table
        .lines()
        .filter(|line| line.starts_with("| `"))
        .map(|line| {
            let cells: Vec<&str> = line.trim_matches('|').split('|').collect();
            assert!(cells.len() >= 5, "a short slot row: {line}");
            let others = match cells[3].trim() {
                "none" => Vec::new(),
                list => list.split(',').map(code).collect(),
            };
            SlotRow {
                slot: code(cells[0]),
                required: cells[1].trim() == "yes",
                default: code(cells[2]),
                others,
                off: cells[4].trim() != "none",
            }
        })
        .collect()
}

/// The rows the registry declares, in its order, as a release program declares them: a test
/// build of the whole workspace also registers the faulty modules the gate tests select
/// (`<name>-faulty`), which no release program contains and the reference does not list.
fn registry_rows() -> Vec<SlotRow> {
    engine::modules::REGISTRY
        .iter()
        .map(|decl| {
            let named = |r: &engine::modules::Registration| format!("{}@{}", r.name, r.version);
            SlotRow {
                slot: decl.slot.id.to_owned(),
                required: decl.slot.required,
                default: named(&decl.registrations[0]),
                others: decl.registrations[1..]
                    .iter()
                    .filter(|r| !r.name.ends_with("-faulty"))
                    .map(named)
                    .collect(),
                off: decl.off.is_some(),
            }
        })
        .collect()
}

#[test]
fn the_module_reference_names_every_slot() {
    let doc = read("docs/reference/engine-modules.md");
    let expected = registry_rows();
    assert_eq!(expected.len(), engine::modules::SLOT_COUNT);
    assert_eq!(
        slot_rows(&doc),
        expected,
        "the slot table differs from the registry"
    );

    // Control: a planted row for a slot the registry does not declare must fail the same
    // comparison, so the check cannot pass on a table it does not read.
    let planted = doc.replacen(
        "| `engine.fouls` |",
        "| `engine.planted` | no | `planted@1` | none | `off` | none | none: control |
| `engine.fouls` |",
        1,
    );
    assert_ne!(planted, doc, "the control row was not planted");
    assert_ne!(
        slot_rows(&planted),
        expected,
        "a planted row went unnoticed"
    );
}

#[test]
fn the_module_how_to_runs_real_commands() {
    let doc = read("docs/how-to/engine-modules.md");
    // Every test target the guide runs is a test file of the engine crate.
    let targets: Vec<&str> = doc
        .split("cargo test -p engine --test ")
        .skip(1)
        .filter_map(|rest| rest.split_whitespace().next())
        .collect();
    assert!(targets.len() >= 2, "the how-to runs no engine test");
    for target in targets {
        let file = format!("crates/engine/tests/{target}.rs");
        assert!(
            repo().join(&file).is_file(),
            "the how-to runs {file}, which does not exist"
        );
    }
    // The files it edits exist, and the test whose count it raises is in the file it names.
    for file in [
        "crates/engine/src/modules/modifier/mod.rs",
        "crates/engine/src/modules/registry.rs",
        "crates/engine/tests/modules.rs",
    ] {
        assert!(
            doc.contains(&format!("`{file}`")),
            "the how-to does not name {file}"
        );
        assert!(repo().join(file).is_file(), "{file} does not exist");
    }
    assert!(
        read("crates/engine/tests/modules.rs")
            .contains("fn every_modifier_slot_names_its_family()")
    );
    // The slot it extends is declared, optional, and holds a modifier.
    let weather = engine::modules::REGISTRY
        .iter()
        .find(|d| d.slot.id == "engine.modifier.weather")
        .expect("the how-to's slot is declared");
    assert!(!weather.slot.required && weather.off.is_some());
    assert!(matches!(
        weather.registrations[0].module,
        engine::modules::ModuleRef::Modifier(_)
    ));
}
