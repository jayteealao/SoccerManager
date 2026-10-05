//! The engine id a fit records: the identity of the golden file's results. The golden file
//! changes only through a regeneration with a ledger entry, so the id changes exactly when
//! the engine's results change, and never on a commit that changes no result.
//!
//! `golden-<last ledger index>-<build>-<digest>`: the build is the last entry's candidate
//! (the commit whose code made the hashes) or, for a bootstrap, its build; the digest is the
//! first 12 hex characters of SHA-256 over the portable hash set's fixture ids and final
//! hashes.

use std::path::Path;

use anyhow::{Context, bail};
use engine::ContentDir;
use engine::gate::golden::{self, GoldenFile};
use engine::gate::{self, Inputs, PackInputs, Verdict};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// The engine id and the parts it is made of.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EngineId {
    pub id: String,
    pub ledger_index: usize,
    pub build: String,
    pub digest: String,
}

/// The engine id of the results `file` records.
pub fn of_golden(file: &GoldenFile) -> anyhow::Result<EngineId> {
    let Some((index, last)) = file.ledger.iter().enumerate().next_back() else {
        bail!("the golden file has no ledger entry");
    };
    let build = last.candidate.clone().unwrap_or_else(|| last.build.clone());
    let set = file.set_for(golden::PORTABLE)?;
    let mut hasher = Sha256::new();
    for m in set {
        hasher.update(format!("{}={}\n", m.id, m.final_hash).as_bytes());
    }
    let hex: String = hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let digest = hex[..12].to_string();
    Ok(EngineId {
        id: format!("golden-{index}-{build}-{digest}"),
        ledger_index: index,
        build,
        digest,
    })
}

/// The engine id of the golden file at `path`.
pub fn of_path(path: &Path) -> anyhow::Result<EngineId> {
    let file = golden::load_lenient(path)
        .with_context(|| format!("cannot read the golden file {}", path.display()))?;
    of_golden(&file)
}

/// Replays the gate fixtures named in `only` (all 22 when empty) and stops at the first
/// whose hashes differ from the golden file at `path`, so the id a fit records is the id of
/// the engine that plays it.
pub fn confirm_engine(
    content_dir: Option<&Path>,
    path: &Path,
    only: &[String],
) -> anyhow::Result<usize> {
    let all = gate::fixtures();
    for id in only {
        if !all.iter().any(|f| &f.id == id) {
            let known: Vec<&str> = all.iter().map(|f| f.id.as_str()).collect();
            bail!("no gate fixture {id:?}; known: {}", known.join(", "));
        }
    }
    let file = golden::load(path, &all)
        .with_context(|| format!("cannot read the golden file {}", path.display()))?;
    let set = file.set_for(&golden::set_key())?.to_vec();
    let selected: Vec<_> = all
        .iter()
        .filter(|f| only.is_empty() || only.contains(&f.id))
        .collect();
    let loaded = crate::content::load(content_dir, None, None, None)?;
    let pack = if selected.iter().any(|f| f.pack.is_some()) {
        let dir = ContentDir::resolve(content_dir)?;
        let pack_dir = dir.path(&format!("scripts/{}", gate::KNOCKOUT_PACK));
        Some(
            script::LoadedPack::load(&pack_dir)
                .with_context(|| format!("the {} script pack", gate::KNOCKOUT_PACK))?,
        )
    } else {
        None
    };
    let hooks = || {
        pack.as_ref()
            .map(script::LoadedPack::plugins)
            .unwrap_or_default()
    };
    let inputs = Inputs {
        content: &loaded.content,
        teams: [&loaded.teams[0], &loaded.teams[1]],
        pack: pack.as_ref().map(|p| PackInputs {
            id: &p.pack.manifest.id,
            sha: *p.sha(),
            hooks: &hooks,
        }),
    };
    for fixture in &selected {
        let played = gate::play_fixture(fixture, &inputs)?;
        let expected = set
            .iter()
            .find(|m| m.id == fixture.id)
            .expect("the strict load found every fixture");
        if gate::compare(expected, &played.hashes) != Verdict::Same {
            bail!(
                "gate fixture {} differs from {}: this engine does not play the golden \
                 results, so a fit made now would record the wrong engine id",
                fixture.id,
                path.display()
            );
        }
    }
    tracing::info!(
        signal = "fast_model.gate_confirmed",
        fixtures = selected.len()
    );
    Ok(selected.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn shipped() -> GoldenFile {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../gate/golden.json");
        golden::load_lenient(&path).unwrap()
    }

    #[test]
    fn the_id_names_the_last_ledger_entry_and_its_build() {
        let file = shipped();
        let id = of_golden(&file).unwrap();
        assert_eq!(id.ledger_index, file.ledger.len() - 1);
        assert!(id.id.starts_with(&format!("golden-{}-", id.ledger_index)));
        assert_eq!(id.digest.len(), 12);
        assert_eq!(
            id,
            of_golden(&file.clone()).unwrap(),
            "the same file, the same id"
        );
    }

    #[test]
    fn a_ledger_entry_or_a_final_hash_changes_the_id_and_nothing_else_does() {
        let file = shipped();
        let id = of_golden(&file).unwrap();

        let mut more = file.clone();
        let mut entry = more.ledger.last().unwrap().clone();
        entry.candidate = Some("abc1234".into());
        more.ledger.push(entry);
        assert_ne!(of_golden(&more).unwrap().id, id.id);

        let mut hash = file.clone();
        let set = hash.hash_sets.get_mut(golden::PORTABLE).unwrap();
        set[0].final_hash = "0".repeat(64);
        let changed = of_golden(&hash).unwrap();
        assert_ne!(changed.digest, id.digest);
        assert_eq!(changed.ledger_index, id.ledger_index);

        let mut checkpoints = file.clone();
        let set = checkpoints.hash_sets.get_mut(golden::PORTABLE).unwrap();
        set[0].checkpoints.clear();
        assert_eq!(
            of_golden(&checkpoints).unwrap(),
            id,
            "checkpoints are not results"
        );
    }
}
