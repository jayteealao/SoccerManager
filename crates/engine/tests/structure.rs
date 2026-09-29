//! The structure rule of the module contract: only the central loop writes match state. No
//! engine part outside the loop files has a method on `Simulation` that takes `&mut self`,
//! and no function outside them takes a `&mut Simulation`. A source scan checks it, because
//! a compile test cannot say "outside these files"; the compile-fail test already proves a
//! module cannot write through its view.

use std::path::Path;

/// The central loop: the tick loop and its passes, the referee's loop, the change queue, and
/// the one writer of module proposals.
const LOOP_FILES: &[&str] = &[
    "sim.rs",
    "sim/",
    "rules/mod.rs",
    "tactics/change.rs",
    "modules/proposal.rs",
];

/// Not engine parts: the snapshot reader restores state, and the scenario builder and the
/// gate's fault and change harness are test seams.
const EXCEPTIONS: &[&str] = &["snapshot.rs", "scenario.rs", "gate/"];

/// Every function in `files` (path relative to `src`, with `/`, and the source text) that
/// writes match state outside the loop files: `file: function` for each.
fn writers_outside_the_loop(files: &[(String, String)]) -> Vec<String> {
    let allowed = |path: &str| {
        LOOP_FILES
            .iter()
            .chain(EXCEPTIONS)
            .any(|a| path == *a || (a.ends_with('/') && path.starts_with(a)))
    };
    let mut found = Vec::new();
    for (path, text) in files {
        if allowed(path) {
            continue;
        }
        let mut in_impl = false;
        let mut signature: Option<String> = None;
        for line in text.lines() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("//") {
                continue;
            }
            // rustfmt puts an impl block's opening line and closing brace at column 0.
            if line.starts_with("impl ") && line.contains(" Simulation") && line.ends_with('{') {
                in_impl = true;
                continue;
            }
            if line == "}" {
                in_impl = false;
                continue;
            }
            if signature.is_none() && is_fn_start(trimmed) {
                signature = Some(String::new());
            }
            if let Some(sig) = signature.as_mut() {
                sig.push_str(trimmed);
                sig.push(' ');
                if trimmed.contains('{') || trimmed.ends_with(';') {
                    let sig = signature.take().unwrap_or_default();
                    let head = sig.split('{').next().unwrap_or_default();
                    let writes =
                        (in_impl && head.contains("&mut self")) || head.contains("&mut Simulation");
                    if writes {
                        found.push(format!("{path}: {}", fn_name(head)));
                    }
                }
            }
        }
    }
    found
}

/// `true` when `line` opens a function signature.
fn is_fn_start(line: &str) -> bool {
    let mut rest = line;
    for prefix in [
        "pub(crate) ",
        "pub(super) ",
        "pub ",
        "const ",
        "async ",
        "unsafe ",
    ] {
        rest = rest.strip_prefix(prefix).unwrap_or(rest);
    }
    rest.starts_with("fn ")
}

fn fn_name(signature: &str) -> &str {
    signature
        .split("fn ")
        .nth(1)
        .and_then(|rest| rest.split(['(', '<']).next())
        .unwrap_or(signature)
        .trim()
}

/// Every `.rs` file under `dir`, with its path relative to `root`.
fn sources(root: &Path, dir: &Path, out: &mut Vec<(String, String)>) {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            sources(root, &path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            let rel = path
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            out.push((rel, std::fs::read_to_string(&path).unwrap()));
        }
    }
}

#[test]
fn no_engine_part_writes_match_state_outside_the_loop() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    sources(&root, &root, &mut files);
    assert!(
        files.iter().any(|(p, _)| p == "sim/possession.rs"),
        "the scan found the loop files"
    );
    assert!(files.len() > 40, "only {} source files", files.len());
    let found = writers_outside_the_loop(&files);
    assert!(
        found.is_empty(),
        "engine parts that write match state outside the central loop:\n{}",
        found.join("\n")
    );
}

#[test]
fn the_scan_refuses_a_part_that_writes() {
    let part = "use crate::sim::Simulation;\n\
                \n\
                impl Simulation {\n\
                \x20   /// Reads.\n\
                \x20   pub fn looks(&self) -> u32 {\n\
                \x20       self.tick\n\
                \x20   }\n\
                \n\
                \x20   // fn commented(&mut self) {}\n\
                \x20   pub(crate) fn sneaks(\n\
                \x20       &mut self,\n\
                \x20       i: usize,\n\
                \x20   ) {\n\
                \x20       self.players[i].target = DVec2::ZERO;\n\
                \x20   }\n\
                }\n\
                \n\
                pub fn pokes(sim: &mut Simulation) {\n\
                \x20   sim.tick += 1;\n\
                }\n";
    let files = vec![
        ("possession.rs".to_string(), part.to_string()),
        ("sim/possession.rs".to_string(), part.to_string()),
        ("snapshot.rs".to_string(), part.to_string()),
    ];
    assert_eq!(
        writers_outside_the_loop(&files),
        ["possession.rs: sneaks", "possession.rs: pokes"]
    );
}
