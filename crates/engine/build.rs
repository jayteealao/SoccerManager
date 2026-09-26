//! Embeds the git build hash into the crate as `ENGINE_BUILD_HASH`.

use std::process::Command;

fn git(args: &[&str]) -> Option<String> {
    let output = Command::new("git").args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn main() {
    let hash = match git(&["rev-parse", "--short", "HEAD"]) {
        Some(h) if !h.is_empty() => {
            let dirty = git(&["status", "--porcelain"])
                .map(|s| !s.is_empty())
                .unwrap_or(false);
            if dirty { format!("{h}-dirty") } else { h }
        }
        _ => "unknown".to_string(),
    };
    println!("cargo:rustc-env=ENGINE_BUILD_HASH={hash}");
    println!("cargo:rerun-if-changed=../../.git/HEAD");
    println!("cargo:rerun-if-changed=../../.git/refs/heads");
    println!("cargo:rerun-if-changed=../../.git/index");
}
