//! The bisect build cache: engine binaries built from commits, each keyed by the commit, the
//! toolchain, the target, the cargo profile, and the features, and each stored with the
//! SHA-256 of its executable. A cached executable is re-used only when its entry names the
//! same key and the executable still has the recorded SHA-256; anything else builds again.
//!
//! Layout under the cache folder: `entries/<id>/engine-cli[.exe]` and
//! `entries/<id>/entry.json`, where `<id>` is the SHA-256 of the key's JSON; `checkouts/<id>`
//! holds a detached worktree only while it builds; `target/` is the cargo target folder all
//! builds share.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// What a cached build is keyed by.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CacheKey {
    /// The full commit hash.
    pub commit: String,
    /// The compiler release and its commit hash, as `rustc -vV` reports them.
    pub toolchain: String,
    /// The target triple the build runs on (the compiler's host).
    pub target: String,
    pub profile: String,
    /// The cargo features, sorted and comma-joined; empty for the defaults.
    pub features: String,
}

impl CacheKey {
    /// The entry id: the SHA-256 of the key's JSON, whose field order is fixed.
    pub fn id(&self) -> String {
        let json = serde_json::to_string(self).expect("a key always serialises");
        stream::record::hex(&Sha256::digest(json.as_bytes()))
    }
}

/// The features as a key holds them: sorted, de-duplicated, comma-joined.
pub fn features_key(features: &str) -> String {
    let mut all: Vec<&str> = features
        .split([',', ' '])
        .map(str::trim)
        .filter(|f| !f.is_empty())
        .collect();
    all.sort_unstable();
    all.dedup();
    all.join(",")
}

/// A cache entry's record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    key: CacheKey,
    executable_sha256: String,
}

/// Builds the engine executable of a key into a file.
pub trait Builder {
    /// Builds `key` and writes the executable to `out`. On failure, the reason in words.
    fn build(&self, key: &CacheKey, out: &Path) -> Result<(), String>;
}

/// An executable the cache handed out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cached {
    pub path: PathBuf,
    pub executable_sha256: String,
    /// `true` when the entry was re-used, `false` when it was built now.
    pub reused: bool,
}

/// The executable file name inside an entry.
pub fn executable_name() -> String {
    format!("engine-cli{}", std::env::consts::EXE_SUFFIX)
}

/// The SHA-256 of a file, or `None` when it cannot be read.
pub fn file_sha256(path: &Path) -> Option<String> {
    let mut file = fs::File::open(path).ok()?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 1 << 16];
    loop {
        let n = file.read(&mut buf).ok()?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Some(stream::record::hex(&hasher.finalize()))
}

/// The executable of `key` from the cache under `root`: the entry when its record names
/// `key` and its executable's SHA-256 matches, otherwise a new build through `builder`.
pub fn lookup(root: &Path, key: &CacheKey, builder: &dyn Builder) -> Result<Cached, String> {
    let dir = root.join("entries").join(key.id());
    let exe = dir.join(executable_name());
    let record = dir.join("entry.json");
    let entry: Option<Entry> = fs::read(&record)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok());
    if let Some(entry) = entry
        && entry.key == *key
        && file_sha256(&exe).as_deref() == Some(entry.executable_sha256.as_str())
    {
        return Ok(Cached {
            path: exe,
            executable_sha256: entry.executable_sha256,
            reused: true,
        });
    }
    let _ = fs::remove_file(&record);
    let _ = fs::remove_file(&exe);
    fs::create_dir_all(&dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
    builder.build(key, &exe)?;
    let sha = file_sha256(&exe)
        .ok_or_else(|| format!("the build wrote no executable at {}", exe.display()))?;
    let entry = Entry {
        key: key.clone(),
        executable_sha256: sha.clone(),
    };
    let json = serde_json::to_vec_pretty(&entry).expect("an entry always serialises");
    let partial = dir.join("entry.json.partial");
    fs::write(&partial, json)
        .and_then(|()| fs::rename(&partial, &record))
        .map_err(|e| format!("cannot write {}: {e}", record.display()))?;
    Ok(Cached {
        path: exe,
        executable_sha256: sha,
        reused: false,
    })
}

/// The full hash of the commit `rev` names in `repo`.
pub fn resolve_commit(repo: &Path, rev: &str) -> Result<String, String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["rev-parse", "--verify", "--quiet"])
        .arg(format!("{rev}^{{commit}}"))
        .stdin(Stdio::null())
        .output()
        .map_err(|e| format!("cannot run git: {e}"))?;
    let hash = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if !out.status.success() || hash.is_empty() {
        return Err(format!("{rev} names no commit in {}", repo.display()));
    }
    Ok(hash)
}

/// The toolchain and the target a build of `commit` uses: `rustc -vV` under the channel the
/// commit's `rust-toolchain.toml` pins, or, when it pins none, as run in `cache`, where
/// the build's checkout will be.
pub fn toolchain_of(repo: &Path, commit: &str, cache: &Path) -> Result<(String, String), String> {
    let pinned = Command::new("git")
        .arg("-C")
        .arg(repo)
        .arg("show")
        .arg(format!("{commit}:rust-toolchain.toml"))
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| channel(&String::from_utf8_lossy(&o.stdout)));
    fs::create_dir_all(cache).map_err(|e| format!("cannot create {}: {e}", cache.display()))?;
    let mut rustc = Command::new("rustc");
    rustc.arg("-vV").current_dir(cache).stdin(Stdio::null());
    if let Some(channel) = &pinned {
        rustc.env("RUSTUP_TOOLCHAIN", channel);
    }
    let out = rustc
        .output()
        .map_err(|e| format!("cannot run rustc: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "rustc -vV failed for the toolchain {}: {}",
            pinned.as_deref().unwrap_or("(default)"),
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    parse_rustc(&String::from_utf8_lossy(&out.stdout))
        .ok_or_else(|| "rustc -vV printed no release, commit-hash, or host".to_string())
}

/// The `channel` of a `rust-toolchain.toml`.
fn channel(toml: &str) -> Option<String> {
    toml.lines().find_map(|line| {
        let (name, value) = line.split_once('=')?;
        (name.trim() == "channel").then(|| value.trim().trim_matches('"').to_string())
    })
}

/// `(toolchain, target)` from `rustc -vV`: `<release> <commit-hash>` and the host.
fn parse_rustc(text: &str) -> Option<(String, String)> {
    let field = |name: &str| {
        text.lines()
            .find_map(|l| l.strip_prefix(name).map(|v| v.trim().to_string()))
    };
    Some((
        format!("{} {}", field("release:")?, field("commit-hash:")?),
        field("host:")?,
    ))
}

/// Builds a commit in a detached worktree under the cache, with one cargo target folder
/// shared by every build, and removes the worktree in every case.
pub struct GitWorktreeBuilder {
    pub repo: PathBuf,
    pub cache: PathBuf,
}

impl Builder for GitWorktreeBuilder {
    fn build(&self, key: &CacheKey, out: &Path) -> Result<(), String> {
        let checkout = self.cache.join("checkouts").join(key.id());
        remove_worktree(&self.repo, &checkout);
        let added = Command::new("git")
            .arg("-C")
            .arg(&self.repo)
            .args(["worktree", "add", "--detach"])
            .arg(&checkout)
            .arg(&key.commit)
            .stdin(Stdio::null())
            .output()
            .map_err(|e| format!("cannot run git: {e}"))?;
        if !added.status.success() {
            remove_worktree(&self.repo, &checkout);
            return Err(format!(
                "cannot check out {}: {}",
                key.commit,
                tail(&added.stderr, 40)
            ));
        }
        let built = self.cargo(key, &checkout, out);
        remove_worktree(&self.repo, &checkout);
        built
    }
}

impl GitWorktreeBuilder {
    fn cargo(&self, key: &CacheKey, checkout: &Path, out: &Path) -> Result<(), String> {
        let target = self.cache.join("target");
        let mut cargo = Command::new("cargo");
        cargo
            .args(["build", "--locked", "-p", "engine-cli", "--profile"])
            .arg(&key.profile)
            .arg("--target-dir")
            .arg(&target)
            .current_dir(checkout)
            .stdin(Stdio::null())
            .stdout(Stdio::null());
        if !key.features.is_empty() {
            cargo.arg("--features").arg(&key.features);
        }
        let result = cargo
            .output()
            .map_err(|e| format!("cannot run cargo: {e}"))?;
        if !result.status.success() {
            return Err(format!(
                "the build of {} failed:\n{}",
                key.commit,
                tail(&result.stderr, 40)
            ));
        }
        let folder = match key.profile.as_str() {
            "dev" | "test" => "debug",
            "bench" => "release",
            other => other,
        };
        let built = target.join(folder).join(executable_name());
        fs::copy(&built, out)
            .map(|_| ())
            .map_err(|e| format!("cannot copy {}: {e}", built.display()))
    }
}

/// Removes the worktree at `checkout`, if any, and its folder.
fn remove_worktree(repo: &Path, checkout: &Path) {
    if checkout.exists() {
        let _ = Command::new("git")
            .arg("-C")
            .arg(repo)
            .args(["worktree", "remove", "--force"])
            .arg(checkout)
            .stdin(Stdio::null())
            .output();
        let _ = fs::remove_dir_all(checkout);
    }
    let _ = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["worktree", "prune"])
        .stdin(Stdio::null())
        .output();
}

/// The last `n` lines of a process's output.
fn tail(bytes: &[u8], n: usize) -> String {
    let text = String::from_utf8_lossy(bytes);
    let lines: Vec<&str> = text.lines().collect();
    lines[lines.len().saturating_sub(n)..].join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    /// Writes a small executable stand-in and counts its calls; fails when told to.
    struct FakeBuilder {
        calls: Cell<u32>,
        fail: bool,
    }

    impl FakeBuilder {
        fn new() -> Self {
            Self {
                calls: Cell::new(0),
                fail: false,
            }
        }
    }

    impl Builder for FakeBuilder {
        fn build(&self, key: &CacheKey, out: &Path) -> Result<(), String> {
            self.calls.set(self.calls.get() + 1);
            if self.fail {
                return Err("error[E0425]: cannot find value `x` in this scope".into());
            }
            fs::write(out, format!("built {}", key.id())).map_err(|e| e.to_string())
        }
    }

    fn key() -> CacheKey {
        CacheKey {
            commit: "0123456789abcdef0123456789abcdef01234567".into(),
            toolchain: "1.92.0 ded5c06cf21d2b93bffd5d884aa6e96934ee4234".into(),
            target: "x86_64-pc-windows-msvc".into(),
            profile: "release".into(),
            features: String::new(),
        }
    }

    fn root(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "engine-cli-bisect-cache-{name}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn the_second_lookup_reuses_the_first_build() {
        let root = root("reuse");
        let builder = FakeBuilder::new();
        let first = lookup(&root, &key(), &builder).unwrap();
        assert!(!first.reused);
        let second = lookup(&root, &key(), &builder).unwrap();
        assert!(second.reused);
        assert_eq!(second.path, first.path);
        assert_eq!(second.executable_sha256, first.executable_sha256);
        assert_eq!(builder.calls.get(), 1);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn another_toolchain_profile_or_feature_set_builds_again() {
        let root = root("key");
        let builder = FakeBuilder::new();
        lookup(&root, &key(), &builder).unwrap();
        let others = [
            CacheKey {
                toolchain: "1.93.0 0000000000000000000000000000000000000000".into(),
                ..key()
            },
            CacheKey {
                profile: "dev".into(),
                ..key()
            },
            CacheKey {
                features: "debug-trace".into(),
                ..key()
            },
        ];
        for (i, other) in others.iter().enumerate() {
            let got = lookup(&root, other, &builder).unwrap();
            assert!(!got.reused, "{other:?}");
            assert_eq!(builder.calls.get(), 2 + i as u32);
        }
        assert!(lookup(&root, &key(), &builder).unwrap().reused);
        assert_eq!(builder.calls.get(), 4);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_changed_executable_builds_again() {
        let root = root("tamper");
        let builder = FakeBuilder::new();
        let first = lookup(&root, &key(), &builder).unwrap();
        let mut bytes = fs::read(&first.path).unwrap();
        bytes[0] ^= 1;
        fs::write(&first.path, bytes).unwrap();
        let again = lookup(&root, &key(), &builder).unwrap();
        assert!(!again.reused);
        assert_eq!(again.executable_sha256, first.executable_sha256);
        assert_eq!(builder.calls.get(), 2);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn an_entry_that_names_another_key_builds_again() {
        let root = root("other-entry");
        let builder = FakeBuilder::new();
        let first = lookup(&root, &key(), &builder).unwrap();
        let record = first.path.with_file_name("entry.json");
        let other = Entry {
            key: CacheKey {
                commit: "fedcba9876543210fedcba9876543210fedcba98".into(),
                ..key()
            },
            executable_sha256: first.executable_sha256.clone(),
        };
        fs::write(&record, serde_json::to_vec(&other).unwrap()).unwrap();
        assert!(!lookup(&root, &key(), &builder).unwrap().reused);
        assert_eq!(builder.calls.get(), 2);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_failed_build_is_an_error_with_its_reason_and_no_entry() {
        let root = root("fail");
        let builder = FakeBuilder {
            fail: true,
            ..FakeBuilder::new()
        };
        let err = lookup(&root, &key(), &builder).unwrap_err();
        assert!(err.contains("cannot find value"), "{err}");
        let dir = root.join("entries").join(key().id());
        assert!(!dir.join("entry.json").exists());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn features_are_sorted_and_joined_and_rustc_output_is_read() {
        assert_eq!(features_key("b, a,,a"), "a,b");
        assert_eq!(features_key(""), "");
        let text = "rustc 1.92.0 (ded5c06cf 2025-12-08)\nbinary: rustc\n\
                    commit-hash: ded5c06cf21d2b93bffd5d884aa6e96934ee4234\n\
                    host: x86_64-pc-windows-msvc\nrelease: 1.92.0\n";
        assert_eq!(
            parse_rustc(text),
            Some((
                "1.92.0 ded5c06cf21d2b93bffd5d884aa6e96934ee4234".into(),
                "x86_64-pc-windows-msvc".into()
            ))
        );
        assert_eq!(
            channel("[toolchain]\nchannel = \"1.92.0\"\n").as_deref(),
            Some("1.92.0")
        );
    }
}
