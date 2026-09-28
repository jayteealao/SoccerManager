//! The engine maths library: `libm` resolves with its default features off and no
//! dependencies, and only the engine depends on it. The clippy rule in
//! `crates/engine/clippy.toml` keeps platform maths out of the engine crate.

use std::path::Path;
use std::process::Command;

use serde_json::Value;

/// The workspace's `cargo metadata`, resolved with each member's default features.
fn metadata() -> Value {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let out = Command::new(cargo)
        .args(["metadata", "--format-version", "1", "--offline", "--locked"])
        .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
        .output()
        .expect("cargo metadata runs");
    assert!(
        out.status.success(),
        "cargo metadata failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).expect("cargo metadata prints JSON")
}

/// The resolve node of the package named `name`.
fn node<'a>(doc: &'a Value, name: &str) -> &'a Value {
    let id = &doc["packages"]
        .as_array()
        .expect("a package list")
        .iter()
        .find(|p| p["name"] == name)
        .unwrap_or_else(|| panic!("{name} is a package"))["id"];
    doc["resolve"]["nodes"]
        .as_array()
        .expect("a resolve graph")
        .iter()
        .find(|n| n["id"] == *id)
        .unwrap_or_else(|| panic!("{name} is in the resolve graph"))
}

#[test]
fn libm_resolves_without_default_features() {
    let doc = metadata();
    let libm: Vec<&Value> = doc["packages"]
        .as_array()
        .expect("a package list")
        .iter()
        .filter(|p| p["name"] == "libm")
        .collect();
    assert_eq!(libm.len(), 1, "exactly one libm package");
    assert_eq!(libm[0]["version"], "0.2.16");
    let id = libm[0]["id"].as_str().expect("a package id");

    let nodes = doc["resolve"]["nodes"].as_array().expect("a resolve graph");
    let node = node(&doc, "libm");
    assert_eq!(
        node["features"],
        Value::Array(vec![]),
        "libm resolves with no features (no default, no arch)"
    );
    assert_eq!(
        node["dependencies"],
        Value::Array(vec![]),
        "libm has no dependencies"
    );

    let users: Vec<String> = nodes
        .iter()
        .filter(|n| {
            n["dependencies"]
                .as_array()
                .is_some_and(|d| d.iter().any(|d| d == id))
        })
        .map(|n| {
            let user = n["id"].as_str().unwrap_or("?");
            doc["packages"]
                .as_array()
                .unwrap()
                .iter()
                .find(|p| p["id"] == user)
                .and_then(|p| p["name"].as_str())
                .unwrap_or(user)
                .to_owned()
        })
        .collect();
    assert_eq!(users, ["engine"], "only the engine depends on libm");
}
