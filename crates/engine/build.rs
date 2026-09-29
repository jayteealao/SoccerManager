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
            let dirty = git(&["status", "--porcelain", "--untracked-files=no"])
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
    // In a linked worktree `.git` is a file, so the git folders are asked for, not assumed.
    // HEAD and the index belong to this worktree; the branch refs are shared.
    let git_dir = git(&["rev-parse", "--git-dir"]).unwrap_or_else(|| "../../.git".to_string());
    let common_dir = git(&["rev-parse", "--git-common-dir"]).unwrap_or_else(|| git_dir.clone());
    println!("cargo:rerun-if-changed={git_dir}/HEAD");
    println!("cargo:rerun-if-changed={common_dir}/refs/heads");
    println!("cargo:rerun-if-changed={git_dir}/index");
    // The dirty mark reads tracked files, so an edit to one must run this script again.
    println!("cargo:rerun-if-changed=../../crates");
    println!("cargo:rerun-if-changed=../../Cargo.lock");
}
