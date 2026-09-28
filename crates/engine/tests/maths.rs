//! The engine maths module. No engine source outside the platform backend calls a platform
//! transcendental function, `libm` resolves with its default features off and no
//! dependencies, and a default build selects the platform backend.

mod common;

use std::path::Path;
use std::process::Command;

use engine::math::{BACKEND, Backend};
use serde_json::Value;

/// The transcendental function names the search looks for.
const NAMES: [&str; 23] = [
    "sin", "cos", "tan", "sin_cos", "exp", "exp2", "exp_m1", "ln", "ln_1p", "log", "log2", "log10",
    "powi", "powf", "atan", "atan2", "asin", "acos", "hypot", "sinh", "cosh", "tanh", "cbrt",
];

/// The glam methods that call the platform maths library.
const GLAM_ANGLE: [&str; 4] = ["from_angle(", "to_angle(", "angle_to(", "rotate_towards("];

/// Every search pattern: the method form, the `f64` and `f32` path forms, and the glam
/// angle methods.
fn patterns() -> Vec<String> {
    let mut out = Vec::new();
    for name in NAMES {
        out.push(format!(".{name}("));
        out.push(format!("f64::{name}("));
        out.push(format!("f32::{name}("));
    }
    out.extend(GLAM_ANGLE.iter().map(|p| (*p).to_owned()));
    out
}

/// The patterns that `text` holds.
fn hits(text: &str, patterns: &[String]) -> Vec<String> {
    patterns
        .iter()
        .filter(|p| text.contains(p.as_str()))
        .cloned()
        .collect()
}

#[test]
fn no_platform_maths_outside_the_platform_backend() {
    const ALLOWED: &str = "math/platform.rs";
    let patterns = patterns();
    // Control 2: the pattern set flags the method and path forms and passes the libm calls.
    assert!(!hits("let a = x.atan2(y);", &patterns).is_empty());
    assert!(!hits("f64::exp(z)", &patterns).is_empty());
    assert!(hits("libm::exp(z)", &patterns).is_empty());
    assert!(hits("value.expect(\"a value\")", &patterns).is_empty());

    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut offenders = Vec::new();
    let mut control = false;
    for file in common::sources(&root) {
        let rel = file
            .strip_prefix(&root)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        let found = hits(&std::fs::read_to_string(&file).unwrap(), &patterns);
        if rel == ALLOWED {
            control = !found.is_empty();
        } else if !found.is_empty() {
            offenders.push(format!("{rel}: {found:?}"));
        }
    }
    // Control 1: the search finds the platform calls in the platform backend.
    assert!(control, "the search finds the platform calls in {ALLOWED}");
    assert!(
        offenders.is_empty(),
        "these files call platform maths outside {ALLOWED}: {offenders:#?}"
    );
}

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

#[test]
fn the_default_build_uses_the_platform_backend() {
    // Guard: the default build resolves the engine with the libm backend, so every CI build
    // plays the maths of the golden file's portable set.
    let doc = metadata();
    let features = &node(&doc, "engine")["features"];
    assert!(
        features
            .as_array()
            .expect("a feature list")
            .iter()
            .any(|f| f == "libm-maths"),
        "the default build resolves the engine with libm-maths: {features}"
    );
    let want = if cfg!(feature = "libm-maths") {
        Backend::Libm
    } else {
        Backend::Platform
    };
    assert_eq!(BACKEND, want);
}
