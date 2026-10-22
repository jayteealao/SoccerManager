//! The shipped game holds none of the real data: the packaging scripts never copy `gate/`,
//! and `content/` holds no record of the reference file, no target key, and no string equal
//! to a real club name from any list in `gate/real-names/` (every string of every file
//! there is a real name).

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde_json::Value;
use tempfile::TempDir;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The copy sources of a packaging script: what each `Copy-Item` (PowerShell) or `cp` (sh)
/// line copies into the staged package folder (`$stage`), as written.
fn copy_sources(script: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in script.lines().map(str::trim) {
        if !line.contains("$stage") {
            continue;
        }
        if let Some(rest) = line.strip_prefix("Copy-Item ") {
            // The first argument that is not a flag: a word, or `(Join-Path $repo 'x')`.
            let rest = rest.trim_start();
            let rest = rest.strip_prefix("-Recurse ").unwrap_or(rest).trim_start();
            if rest.starts_with('(') {
                let group = &rest[..rest.find(')').map_or(rest.len(), |i| i + 1)];
                out.push(group.split('\'').nth(1).unwrap_or(group).to_string());
            } else if let Some(word) = rest.split_whitespace().next() {
                out.push(word.to_string());
            }
        } else if let Some(rest) = line.strip_prefix("cp ") {
            // `cp [-R] a b dest`: every word but the flags and the last.
            let words: Vec<&str> = rest
                .split_whitespace()
                .filter(|w| !w.starts_with('-'))
                .collect();
            for w in &words[..words.len().saturating_sub(1)] {
                out.push(w.trim_matches('"').to_string());
            }
        }
    }
    out
}

/// The sources that name `gate` or a path under it.
fn gate_sources(script: &str) -> Vec<String> {
    copy_sources(script)
        .into_iter()
        .filter(|s| {
            s.replace('\\', "/")
                .split('/')
                .any(|part| part.eq_ignore_ascii_case("gate"))
        })
        .collect()
}

/// The repository item a source names, without the `$repo` prefix.
fn item(source: &str) -> String {
    source
        .trim_start_matches("$repo/")
        .replace('\\', "/")
        .to_string()
}

#[test]
fn neither_packaging_script_copies_the_gate_folder() {
    let known: BTreeSet<&str> = [
        "$exe",
        "$program",
        "$previous",
        "content",
        "viewer/dist-release",
        "LICENSE-MIT",
        "LICENSE-APACHE",
        "packaging/unix/soccermanager",
    ]
    .into_iter()
    .collect();
    for rel in ["packaging/windows/build.ps1", "packaging/unix/build.sh"] {
        let script = std::fs::read_to_string(repo().join(rel)).unwrap();
        assert_eq!(gate_sources(&script), Vec::<String>::new(), "{rel}");
        let sources = copy_sources(&script);
        assert!(
            sources.iter().any(|s| item(s) == "content"),
            "{rel}: {sources:?}"
        );
        for s in &sources {
            assert!(
                known.contains(item(s).as_str()),
                "{rel} copies {s}, a new item"
            );
        }
    }
}

#[test]
fn a_script_that_copies_the_gate_folder_fails() {
    let ps = "Copy-Item -Recurse (Join-Path $repo 'gate') (Join-Path $stage 'gate')";
    assert_eq!(copy_sources(ps), ["gate"]);
    let sh = "cp -R \"$repo/gate/real-targets.json\" \"$stage/$name/\"";
    assert_eq!(gate_sources(ps), ["gate"]);
    assert_eq!(gate_sources(sh), ["$repo/gate/real-targets.json"]);
    assert!(gate_sources("Copy-Item (Join-Path $repo 'content') $stage").is_empty());
    // A copy beside the installer, not into the package, is not a package item.
    assert!(copy_sources("Copy-Item (Join-Path $repo 'gate') $dist -Force").is_empty());
}

/// Every string value of a JSON document.
fn strings(v: &Value, out: &mut Vec<String>) {
    match v {
        Value::String(s) => out.push(s.clone()),
        Value::Array(a) => a.iter().for_each(|x| strings(x, out)),
        Value::Object(o) => o.values().for_each(|x| strings(x, out)),
        _ => {}
    }
}

/// What a content folder must not hold, from the `gate/` folder.
struct Forbidden {
    names: BTreeSet<String>,
    keys: BTreeSet<String>,
}

impl Forbidden {
    fn load(gate: &Path) -> Self {
        let mut names = BTreeSet::new();
        for e in std::fs::read_dir(gate.join("real-names")).unwrap() {
            let path = e.unwrap().path();
            if path.extension().is_some_and(|x| x == "json") {
                let v: Value =
                    serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
                let mut all = Vec::new();
                strings(&v, &mut all);
                names.extend(all.iter().map(|s| s.trim().to_lowercase()));
            }
        }
        let file: Value =
            serde_json::from_str(&std::fs::read_to_string(gate.join("real-targets.json")).unwrap())
                .unwrap();
        let keys = file["targets"]
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect();
        Self { names, keys }
    }
}

/// The band file's text with each band's target pointers (`target.target` and
/// `real.source.target`) taken out.
fn without_target_pointers(text: &str) -> String {
    let Ok(mut v) = serde_json::from_str::<Value>(text) else {
        return text.to_string();
    };
    if let Some(bands) = v["bands"].as_array_mut() {
        for b in bands {
            if let Some(target) = b.get_mut("target").and_then(Value::as_object_mut) {
                target.remove("target");
            }
            if let Some(source) = b.pointer_mut("/real/source").and_then(Value::as_object_mut) {
                source.remove("target");
            }
        }
    }
    v.to_string()
}

/// Every fault of the content folder `dir`: a reference-file record, a target key, or a
/// string equal to a real club name.
fn content_faults(dir: &Path, forbidden: &Forbidden) -> Vec<String> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        for e in std::fs::read_dir(dir).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                walk(&p, out);
            } else {
                out.push(p);
            }
        }
    }
    fn records(v: &Value) -> bool {
        match v {
            Value::Object(o) => {
                (o.contains_key("event_definition") && o.contains_key("unit_of_observation"))
                    || o.values().any(records)
            }
            Value::Array(a) => a.iter().any(records),
            _ => false,
        }
    }
    let mut files = Vec::new();
    walk(dir, &mut files);
    let mut faults = Vec::new();
    for path in files {
        let rel = path.strip_prefix(dir).unwrap().display().to_string();
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        // A band of the band file may name its reference-file target by key: a pointer that
        // calibrate resolves from the checkout, never a figure. Every other key is a fault.
        let scanned = if rel == "realism-bands.json" {
            without_target_pointers(&text)
        } else {
            text.clone()
        };
        for key in &forbidden.keys {
            if scanned.contains(key.as_str()) {
                faults.push(format!("{rel}: target key {key}"));
            }
        }
        let Ok(v) = serde_json::from_str::<Value>(&text) else {
            continue;
        };
        if records(&v) {
            faults.push(format!("{rel}: a reference-file record"));
        }
        let mut all = Vec::new();
        strings(&v, &mut all);
        for s in all {
            if forbidden.names.contains(&s.trim().to_lowercase()) {
                faults.push(format!("{rel}: real club name {s:?}"));
            }
        }
    }
    faults
}

#[test]
fn the_shipped_content_holds_no_real_figure_and_no_real_club_name() {
    let forbidden = Forbidden::load(&repo().join("gate"));
    assert!(
        forbidden.names.len() > 400,
        "{} names",
        forbidden.names.len()
    );
    assert!(forbidden.keys.len() > 500);
    let faults = content_faults(&repo().join("content"), &forbidden);
    assert!(faults.is_empty(), "{faults:#?}");
}

#[test]
fn a_content_folder_with_a_record_a_key_or_a_real_name_fails() {
    let forbidden = Forbidden::load(&repo().join("gate"));
    let tmp = TempDir::new().unwrap();
    let content = tmp.path();
    std::fs::create_dir_all(content.join("teams")).unwrap();
    // Facsimile names that hold a real name inside them pass.
    std::fs::write(
        content.join("teams/a.json"),
        r#"{"name": "Salhaven Rangers", "players": [{"name": "Lesef Morton"}]}"#,
    )
    .unwrap();
    assert!(content_faults(content, &forbidden).is_empty());

    std::fs::write(content.join("teams/b.json"), r#"{"name": "Rangers"}"#).unwrap();
    std::fs::write(
        content.join("leak.json"),
        r#"{"x": {"value": 2.9, "event_definition": "goals", "unit_of_observation": "match"}}"#,
    )
    .unwrap();
    std::fs::write(
        content.join("bands.json"),
        r#"{"real": "fd.E0.goals_per_match"}"#,
    )
    .unwrap();
    let faults = content_faults(content, &forbidden);
    assert!(
        faults
            .iter()
            .any(|f| f.contains("real club name \"Rangers\"")),
        "{faults:?}"
    );
    assert!(
        faults.iter().any(|f| f.contains("a reference-file record")),
        "{faults:?}"
    );
    assert!(
        faults
            .iter()
            .any(|f| f.contains("target key fd.E0.goals_per_match")),
        "{faults:?}"
    );

    // The band file may point a band at its target by key, and nowhere else.
    let pointer = TempDir::new().unwrap();
    std::fs::write(
        pointer.path().join("realism-bands.json"),
        r#"{"bands": [{"band": "g", "real": {"source": {"target": "fd.E0.goals_per_match"}}}]}"#,
    )
    .unwrap();
    assert!(content_faults(pointer.path(), &forbidden).is_empty());
    std::fs::write(
        pointer.path().join("realism-bands.json"),
        r#"{"bands": [{"band": "g", "margin": {"reason": "from fd.E0.goals_per_match"}}]}"#,
    )
    .unwrap();
    assert!(!content_faults(pointer.path(), &forbidden).is_empty());
}
