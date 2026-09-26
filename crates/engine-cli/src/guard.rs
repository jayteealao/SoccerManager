//! `engine-cli guard`: checks every commit in `BASE..HEAD` that changes the golden file
//! against its first parent with the rules of `engine::gate::guard`, and checks that each new
//! `regenerate` entry names a clean candidate commit that is an ancestor of its commit. It
//! prints one line per commit and exits 0 when every commit passes, 2 when a rule fails, and
//! 1 when git or a revision cannot be read. It plays no match.
//!
//! Git runs in the current folder through `std::process::Command`, as `build.rs` runs it; a
//! merge commit is compared with its first parent, the branch side of a pull request.

use std::process::Command;

use anyhow::{Context, bail};
use engine::gate::golden::{self, EntryKind};
use engine::gate::guard::{self, Fault};

use crate::cli::GuardOpts;

pub fn run(opts: &GuardOpts) -> anyhow::Result<i32> {
    let range = format!("{}..{}", opts.base, opts.head);
    let list = git(&["rev-list", "--reverse", &range, "--", &opts.golden])?;
    let commits: Vec<&str> = list.lines().filter(|l| !l.is_empty()).collect();
    if commits.is_empty() {
        eprintln!("guard: no commit in {range} changes {}", opts.golden);
        return Ok(0);
    }
    let mut failed = 0;
    for commit in &commits {
        let short = &commit[..commit.len().min(7)];
        let parent_rev = format!("{commit}^1");
        let parent = if git_ok(&["rev-parse", "--verify", "--quiet", &parent_rev])? {
            file_at(&parent_rev, &opts.golden)?
        } else {
            None
        };
        let child = file_at(commit, &opts.golden)?;
        let mut faults = guard::check(parent.as_deref(), child.as_deref());
        faults.extend(candidate_faults(
            commit,
            parent.as_deref(),
            child.as_deref(),
        )?);
        if faults.is_empty() {
            println!("{short} ok");
        } else {
            failed += 1;
            for f in &faults {
                println!("{short} {f}");
            }
        }
    }
    let verdict = if failed == 0 {
        "none fail".to_string()
    } else {
        format!("{failed} fail")
    };
    eprintln!(
        "guard: {} commits in {range} change {}; {verdict}",
        commits.len(),
        opts.golden
    );
    Ok(if failed == 0 { 0 } else { 2 })
}

/// A new `regenerate` entry's candidate must be a clean build of a commit that is an
/// ancestor of (or equal to) `commit`.
fn candidate_faults(
    commit: &str,
    parent: Option<&str>,
    child: Option<&str>,
) -> anyhow::Result<Vec<Fault>> {
    let Some(child) = child.and_then(|t| golden::read(t).ok()) else {
        return Ok(Vec::new());
    };
    let parent = parent.and_then(|t| golden::read(t).ok());
    let mut faults = Vec::new();
    for (i, e) in guard::new_entries(parent.as_ref(), &child) {
        if e.kind != EntryKind::Regenerate {
            continue;
        }
        let Some(candidate) = e.candidate.as_deref() else {
            continue;
        };
        let bad = |why: String| Fault {
            rule: 5,
            message: format!("ledger entry {i} (regenerate): candidate {candidate} {why}"),
        };
        if candidate.ends_with("-dirty") || candidate == "unknown" {
            faults.push(bad(
                "is not a clean build of a commit; regenerate on a clean tree".into(),
            ));
            continue;
        }
        let out = Command::new("git")
            .args(["merge-base", "--is-ancestor", candidate, commit])
            .output()
            .context("cannot run git")?;
        match out.status.code() {
            Some(0) => {}
            Some(1) => faults.push(bad(format!("is not an ancestor of {commit}"))),
            _ => faults.push(bad("is not a commit in this repository".into())),
        }
    }
    Ok(faults)
}

/// The golden file's text at `rev`, or `None` when the file is absent there.
fn file_at(rev: &str, path: &str) -> anyhow::Result<Option<String>> {
    let listed = git(&["ls-tree", "--name-only", rev, "--", path])?;
    if listed.trim().is_empty() {
        return Ok(None);
    }
    git(&["show", &format!("{rev}:{path}")]).map(Some)
}

/// The standard output of a git command that must succeed.
fn git(args: &[&str]) -> anyhow::Result<String> {
    let out = Command::new("git")
        .args(args)
        .output()
        .context("cannot run git")?;
    if !out.status.success() {
        bail!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    String::from_utf8(out.stdout).with_context(|| format!("git {} printed non-UTF-8", args[0]))
}

/// Whether a git command succeeds.
fn git_ok(args: &[&str]) -> anyhow::Result<bool> {
    Ok(Command::new("git")
        .args(args)
        .output()
        .context("cannot run git")?
        .status
        .success())
}
