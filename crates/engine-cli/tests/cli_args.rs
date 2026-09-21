//! AC-f: an invalid seed exits non-zero with a message naming the argument; a valid seed exits zero.

use std::process::Command;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_engine-cli"))
}

#[test]
fn invalid_seed_exits_non_zero_and_names_the_argument() {
    let out = bin()
        .args(["simulate", "--seed", "abc", "--ticks-out", "unused.ticks"])
        .output()
        .unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("--seed"),
        "stderr did not name --seed: {stderr}"
    );
}

#[test]
fn valid_seed_exits_zero() {
    let path = std::env::temp_dir().join(format!("engine-cli-args-{}.ticks", std::process::id()));
    let out = bin()
        .args(["simulate", "--seed", "42", "--minutes", "1", "--ticks-out"])
        .arg(&path)
        .output()
        .unwrap();
    let _ = std::fs::remove_file(&path);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("\"record.kind\":\"match-stats\""),
        "stdout: {stdout}"
    );
}
