//! Embeds the git build hash into the crate as `ENGINE_BUILD_HASH`, and the full commit and
//! the dirty mark it is made from as `ENGINE_COMMIT` and `ENGINE_DIRTY`.

use std::process::Command;

fn git(args: &[&str]) -> Option<String> {
    let output = Command::new("git").args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn main() {
    let short = git(&["rev-parse", "--short", "HEAD"]).filter(|h| !h.is_empty());
    let full = git(&["rev-parse", "HEAD"]).filter(|h| !h.is_empty());
    let (hash, commit, dirty) = match (short, full) {
        (Some(short), Some(full)) => {
            let dirty = git(&["status", "--porcelain"])
                .map(|s| !s.is_empty())
                .unwrap_or(false);
            let hash = if dirty {
                format!("{short}-dirty")
            } else {
                short
            };
            (hash, full, dirty)
        }
        _ => ("unknown".to_string(), "unknown".to_string(), false),
    };
    println!("cargo:rustc-env=ENGINE_BUILD_HASH={hash}");
    println!("cargo:rustc-env=ENGINE_COMMIT={commit}");
    println!("cargo:rustc-env=ENGINE_DIRTY={dirty}");
    println!("cargo:rerun-if-changed=../../.git/HEAD");
    println!("cargo:rerun-if-changed=../../.git/refs/heads");
    println!("cargo:rerun-if-changed=../../.git/index");
}
