//! `engine-cli guard` over temporary git histories with synthetic golden files: a
//! clean history (bootstrap, add-machine-set, regenerate) exits 0; a hash change with no
//! entry fails at its own commit even when a later commit adds the entry; an add-machine-set
//! commit that also changes the existing set fails; a dirty regenerate candidate fails.

mod common;

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use engine::gate::golden::{self, GoldenFile};
use engine::gate::{self, Checkpoint, MatchHashes};

const MACHINE: &str = "aa-machine";

/// An empty git repository with one commit that has no golden file; returns its folder and
/// that commit.
fn repo(name: &str) -> (PathBuf, String) {
    let dir = std::env::temp_dir().join(format!("engine-cli-guard-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("gate")).unwrap();
    git(&dir, &["init", "--quiet"]);
    git(&dir, &["config", "core.autocrlf", "false"]);
    std::fs::write(dir.join("README.md"), "test repository\n").unwrap();
    let base = commit(&dir, "start");
    (dir, base)
}

fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .current_dir(dir)
        .args([
            "-c",
            "user.name=guard test",
            "-c",
            "user.email=guard@test.invalid",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap().trim().to_string()
}

/// Commits every change and returns the commit.
fn commit(dir: &Path, message: &str) -> String {
    git(dir, &["add", "--all"]);
    git(dir, &["commit", "--quiet", "--allow-empty", "-m", message]);
    git(dir, &["rev-parse", "HEAD"])
}

/// Writes `text` as the golden file and commits it.
fn commit_golden(dir: &Path, text: &str, message: &str) -> String {
    std::fs::write(dir.join("gate/golden.json"), text).unwrap();
    commit(dir, message)
}

fn guard(dir: &Path, base: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_engine-cli"))
        .current_dir(dir)
        .env("SM_LOG", "warn")
        .args(["guard", "--base", base])
        .output()
        .unwrap()
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn first() -> GoldenFile {
    let fixtures = gate::fixtures();
    let hash = |n: u32| format!("{n:064x}");
    let matches = fixtures
        .iter()
        .map(|f| MatchHashes {
            id: f.id.clone(),
            ticks: 2_500,
            final_hash: hash(3),
            checkpoints: vec![
                Checkpoint {
                    tick: 1_000,
                    hash: hash(1),
                },
                Checkpoint {
                    tick: 2_000,
                    hash: hash(2),
                },
                Checkpoint {
                    tick: 2_500,
                    hash: hash(3),
                },
            ],
        })
        .collect();
    GoldenFile::first(&fixtures, MACHINE, matches, golden::BOOTSTRAP_REASON)
}

fn changed(file: &GoldenFile, machine: &str, n: u32) -> Vec<MatchHashes> {
    let mut set = file.set_for(machine).unwrap().to_vec();
    set[0].final_hash = format!("{n:064x}");
    set[0].checkpoints.last_mut().unwrap().hash = set[0].final_hash.clone();
    set
}

/// `file` regenerated with a changed set for `MACHINE`, naming `candidate`.
fn regenerated(file: &GoldenFile, candidate: &str) -> GoldenFile {
    let (mut regen, _) = file.clone().regenerated(
        &gate::fixtures(),
        MACHINE,
        changed(file, MACHINE, 50),
        "a deliberate hash change",
    );
    regen.ledger.last_mut().unwrap().candidate = Some(candidate.to_string());
    regen
}

#[test]
fn a_clean_history_passes() {
    let (dir, base) = repo("clean");
    let one = first();
    commit_golden(&dir, &one.to_text(), "bootstrap");
    let two = common::with_machine_set(
        one.clone(),
        "zz-second",
        changed(&one, MACHINE, 9),
        "second machine",
    );
    let c2 = commit_golden(&dir, &two.to_text(), "add a machine set");
    std::fs::write(dir.join("code.txt"), "the change\n").unwrap();
    let code = commit(&dir, "the code change");
    let three = regenerated(&two, &code[..7]);
    commit_golden(&dir, &three.to_text(), "regenerate");

    let out = guard(&dir, &base);
    let stdout = text(&out.stdout);
    assert_eq!(out.status.code(), Some(0), "{stdout}{}", text(&out.stderr));
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 3, "{stdout}");
    assert!(lines.iter().all(|l| l.ends_with(" ok")), "{stdout}");
    assert!(stdout.contains(&format!("{} ok", &c2[..7])), "{stdout}");
    assert!(
        text(&out.stderr).contains("3 commits"),
        "{}",
        text(&out.stderr)
    );

    // A range with no golden-file change passes and says so.
    let out = guard(&dir, "HEAD");
    assert_eq!(out.status.code(), Some(0));
    assert!(text(&out.stderr).contains("no commit in HEAD..HEAD changes"));
}

#[test]
fn an_intermediate_hash_change_fails_even_when_a_later_commit_adds_the_entry() {
    let (dir, base) = repo("intermediate");
    let one = first();
    let c1 = commit_golden(&dir, &one.to_text(), "bootstrap");
    let mut silent = one.clone();
    silent
        .hash_sets
        .insert(MACHINE.into(), changed(&one, MACHINE, 50));
    let c2 = commit_golden(&dir, &silent.to_text(), "a silent hash change");
    let three = regenerated(&one, &c1[..7]);
    let c3 = commit_golden(&dir, &three.to_text(), "the entry, one commit late");

    let out = guard(&dir, &base);
    let stdout = text(&out.stdout);
    assert_eq!(out.status.code(), Some(2), "{stdout}{}", text(&out.stderr));
    assert!(stdout.contains(&format!("{} ok", &c1[..7])), "{stdout}");
    assert!(
        stdout.contains(&format!(
            "{} rule 5: hash set aa-machine changes with no new regenerate entry",
            &c2[..7]
        )),
        "{stdout}"
    );
    // The late commit is checked against the silent one, so its entry records no change.
    assert!(
        stdout.contains(&format!("{} rule 7: a regenerate entry is added", &c3[..7])),
        "{stdout}"
    );
    assert!(text(&out.stderr).contains("fail"), "{}", text(&out.stderr));

    // Checked from the silent commit on, the late commit still fails.
    let out = guard(&dir, &c2);
    assert_eq!(out.status.code(), Some(2), "{}", text(&out.stdout));
}

#[test]
fn an_add_machine_set_commit_that_changes_the_existing_set_fails() {
    let (dir, base) = repo("add-changes");
    let one = first();
    commit_golden(&dir, &one.to_text(), "bootstrap");
    let mut two = common::with_machine_set(
        one.clone(),
        "zz-second",
        changed(&one, MACHINE, 9),
        "second machine",
    );
    two.hash_sets
        .insert(MACHINE.into(), changed(&one, MACHINE, 77));
    two.set_differences = golden::set_differences(&two.hash_sets);
    let c2 = commit_golden(&dir, &two.to_text(), "add a set and change the old one");

    let out = guard(&dir, &base);
    let stdout = text(&out.stdout);
    assert_eq!(out.status.code(), Some(2), "{stdout}");
    assert!(
        stdout.contains(&format!(
            "{} rule 6: the add-machine-set entry for zz-second also: hash set aa-machine changes",
            &c2[..7]
        )),
        "{stdout}"
    );
}

#[test]
fn a_dirty_or_foreign_regenerate_candidate_fails() {
    let (dir, base) = repo("candidate");
    let one = first();
    let c1 = commit_golden(&dir, &one.to_text(), "bootstrap");
    let dirty = regenerated(&one, &format!("{}-dirty", &c1[..7]));
    let c2 = commit_golden(&dir, &dirty.to_text(), "regenerate on a dirty tree");

    let out = guard(&dir, &base);
    let stdout = text(&out.stdout);
    assert_eq!(out.status.code(), Some(2), "{stdout}");
    assert!(
        stdout.contains(&format!(
            "{} rule 5: ledger entry 1 (regenerate): candidate {}-dirty is not a clean build",
            &c2[..7],
            &c1[..7]
        )),
        "{stdout}"
    );

    let unknown = regenerated(&one, "0000000");
    std::fs::write(dir.join("gate/golden.json"), unknown.to_text()).unwrap();
    git(
        &dir,
        &["commit", "--quiet", "--all", "--amend", "-m", "unknown"],
    );
    let out = guard(&dir, &base);
    let stdout = text(&out.stdout);
    assert_eq!(out.status.code(), Some(2), "{stdout}");
    assert!(
        stdout.contains("candidate 0000000 is not a commit in this repository"),
        "{stdout}"
    );

    // A base that is not a revision exits 1.
    let out = guard(&dir, "no-such-revision");
    assert_eq!(out.status.code(), Some(1));
    assert!(
        text(&out.stderr).contains("git rev-list"),
        "{}",
        text(&out.stderr)
    );
}
