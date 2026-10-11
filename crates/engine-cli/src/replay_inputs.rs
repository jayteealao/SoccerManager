//! The inputs of a recorded match, by value, and the one way a match is built from them.
//!
//! `record` reads the input files from disk once and stores their bytes in the replay
//! file; `resimulate` takes the same bytes back from the file. Both build the match through
//! [`build`], which reads no file, so the bytes a replay file stores are exactly the bytes
//! its match was played from, and a re-simulated match cannot read the checkout. Only
//! [`InputFiles::read`] touches the disk; a test fails when a path loader appears anywhere
//! else in this file or in `resimulate.rs`.

use std::path::Path;

use anyhow::{Context, bail};
use engine::data::traits::FROZEN_TRAITS_V1;
use engine::data::{
    ATTRIBUTES_FILE, COMMENTARY_FILE, ContentFiles, RULES_FILE, SLOTS_FILE, TACTICS_FILE,
    TEAM_A_FILE, TEAM_B_FILE, TRAITS_FILE, TUNING_FILE,
};
use engine::{Commentary, Content, MatchConfig, Simulation};
use script::{Backstop, LoadedPack, Pack};
use stream::{Fixture, InputFile, MatchSettings};

/// The roles every replay file holds, in the order the file stores them.
const CORE_ROLES: [&str; 7] = [
    "attributes",
    "tuning",
    "rules",
    "tactics",
    "commentary",
    "team_a",
    "team_b",
];

/// The slot file's role. It follows the core roles. A replay file written before the role
/// existed lacks it and plays with the built-in default selection, the only one then.
const SLOTS_ROLE: &str = "slots";

/// The trait catalogue's role. It follows the slot file. A replay file written before the
/// catalogue joined the content lacks it and builds with the catalogue as it first shipped
/// ([`FROZEN_TRAITS_V1`]); its team files hold no traits.
const TRAITS_ROLE: &str = "traits";

/// The script pack's roles. They come last and are present only when the match ran a pack.
const PACK_ROLES: [&str; 2] = ["pack_manifest", "pack_script"];

/// The input files of one match: the core roles, the slot file, the trait catalogue, and the
/// pack files, in the
/// order the replay file stores them.
pub struct InputFiles(pub Vec<InputFile>);

impl InputFiles {
    // path-loading: begin
    /// Resolves the content folder, the two team files, and the script pack as the
    /// streaming commands do, and reads each input file once.
    pub fn read(
        content_dir: Option<&Path>,
        team_a: Option<&Path>,
        team_b: Option<&Path>,
        script_pack: Option<&Path>,
    ) -> anyhow::Result<Self> {
        use engine::ContentDir;
        use engine::data::read_bytes;

        let dir = ContentDir::resolve(content_dir)?;
        let content = ContentFiles::read(&dir)?;
        let file = |role: &str, name: &str, bytes: Vec<u8>| InputFile {
            role: role.into(),
            name: name.into(),
            bytes,
        };
        let mut files = vec![
            file("attributes", ATTRIBUTES_FILE, content.attributes),
            file("tuning", TUNING_FILE, content.tuning),
            file("rules", RULES_FILE, content.rules),
            file("tactics", TACTICS_FILE, content.tactics),
            file(
                "commentary",
                COMMENTARY_FILE,
                read_bytes(&dir.path(COMMENTARY_FILE), COMMENTARY_FILE)?,
            ),
        ];
        for (role, flag, default) in [
            ("team_a", team_a, TEAM_A_FILE),
            ("team_b", team_b, TEAM_B_FILE),
        ] {
            let path = flag.map_or_else(|| dir.path(default), Path::to_path_buf);
            let name = dir.relative(&path);
            let bytes = read_bytes(&path, &name)?;
            files.push(file(role, &name, bytes));
        }
        files.push(file(
            SLOTS_ROLE,
            SLOTS_FILE,
            read_bytes(&dir.path(SLOTS_FILE), SLOTS_FILE)?,
        ));
        files.push(file(TRAITS_ROLE, TRAITS_FILE, content.traits));
        if let Some(pack_dir) = script_pack {
            // The folder checks (the entry stays inside the folder) run on the disk layout.
            let pack = Pack::read(pack_dir).inspect_err(|err| {
                tracing::error!(signal = "script.refused", reason = %err);
            })?;
            let manifest = pack_dir.join(script::pack::MANIFEST_FILE);
            let entry = pack_dir.join(&pack.manifest.entry);
            files.push(file(
                "pack_manifest",
                script::pack::MANIFEST_FILE,
                read_bytes(&manifest, script::pack::MANIFEST_FILE)?,
            ));
            files.push(file(
                "pack_script",
                &pack.manifest.entry,
                read_bytes(&entry, &pack.manifest.entry)?,
            ));
        }
        Ok(Self(files))
    }
    // path-loading: end

    /// The inputs a replay file holds, checked to be the roles a match needs.
    pub fn from_record(fixture: &Fixture) -> anyhow::Result<Self> {
        let roles: Vec<&str> = fixture.inputs.iter().map(|f| f.role.as_str()).collect();
        // The trait catalogue is stored only after the slot file, which every replay that has
        // the catalogue also has.
        let accepted = [(false, false), (true, false), (true, true)]
            .into_iter()
            .any(|(slots, traits)| {
                [false, true].into_iter().any(|pack| {
                    let mut wanted = CORE_ROLES.to_vec();
                    if slots {
                        wanted.push(SLOTS_ROLE);
                    }
                    if traits {
                        wanted.push(TRAITS_ROLE);
                    }
                    if pack {
                        wanted.extend(PACK_ROLES);
                    }
                    roles == wanted
                })
            });
        if !accepted {
            bail!(
                "the replay file's inputs are {} ; a match needs {}, then optionally {SLOTS_ROLE} \
                 and {TRAITS_ROLE}, then optionally {}",
                roles.join(", "),
                CORE_ROLES.join(", "),
                PACK_ROLES.join(" and ")
            );
        }
        Ok(Self(fixture.inputs.clone()))
    }

    fn get(&self, role: &str) -> Option<&InputFile> {
        self.0.iter().find(|f| f.role == role)
    }

    fn role(&self, role: &str) -> anyhow::Result<&InputFile> {
        self.get(role)
            .with_context(|| format!("the inputs hold no {role} file"))
    }

    /// The sum of the input files' sizes.
    pub fn bytes(&self) -> u64 {
        self.0.iter().map(|f| f.bytes.len() as u64).sum()
    }
}

/// A match built from its inputs, ready to play.
pub struct Built {
    pub sim: Simulation,
    pub commentary: Commentary,
    pub keyframe_interval: u32,
}

/// Builds the match `settings` names from `inputs`, reading no file. The script pack's
/// calls run under `backstop`. `traced` plays the match with debug mode on.
pub fn build(
    inputs: &InputFiles,
    settings: &MatchSettings,
    backstop: Backstop,
    traced: bool,
) -> anyhow::Result<Built> {
    let content = Content::from_files(&ContentFiles {
        attributes: inputs.role("attributes")?.bytes.clone(),
        tuning: inputs.role("tuning")?.bytes.clone(),
        rules: inputs.role("rules")?.bytes.clone(),
        tactics: inputs.role("tactics")?.bytes.clone(),
        // A file without the trait catalogue builds with the catalogue as it first shipped.
        traits: inputs
            .get(TRAITS_ROLE)
            .map_or_else(|| FROZEN_TRAITS_V1.as_bytes().to_vec(), |f| f.bytes.clone()),
    })?;
    // A file without the slot file plays with the built-in default selection.
    let content = match inputs.get(SLOTS_ROLE) {
        Some(slots) => content.with_slots(&slots.bytes)?,
        None => content,
    };
    let commentary = inputs.role("commentary")?;
    let commentary = Commentary::from_bytes(&commentary.bytes, &commentary.name)?;
    let team = |role: &str| -> anyhow::Result<_> {
        let file = inputs.role(role)?;
        Ok(content.team_from_bytes(&file.bytes, &file.name)?.value)
    };
    let (team_a, team_b) = (team("team_a")?, team("team_b")?);
    let pack = match (inputs.get("pack_manifest"), inputs.get("pack_script")) {
        (Some(manifest), Some(entry)) => {
            let pack = Pack::from_bytes(&manifest.bytes, &entry.bytes, Path::new("script-pack"))
                .and_then(|pack| {
                    if pack.manifest.entry == entry.name {
                        LoadedPack::from_pack(pack, backstop)
                    } else {
                        Err(script::ScriptError::Refused {
                            path: manifest.name.clone(),
                            field: "entry".into(),
                            reason: format!("names {}, not {}", pack.manifest.entry, entry.name),
                        })
                    }
                })
                .inspect_err(|err| {
                    tracing::error!(signal = "script.refused", reason = %err);
                })?;
            Some(pack)
        }
        (None, None) => None,
        _ => bail!("the inputs hold half a script pack"),
    };
    let mut config = MatchConfig::new(
        settings.seed,
        settings.minutes,
        &content,
        [&team_a, &team_b],
    )?;
    if settings.knockout {
        config = config.with_knockout();
    }
    for (team, kind) in settings.managers.iter().enumerate() {
        config = config.with_manager(team, kind.manager());
    }
    let keyframe_interval = content.tuning.stream.keyframe_interval;
    let sim = Simulation::start(
        config,
        pack.as_ref().map(|pack| (pack.sha(), pack.plugins())),
        traced,
    )?;
    Ok(Built {
        sim,
        commentary,
        keyframe_interval,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shipped() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")
    }

    fn settings() -> MatchSettings {
        MatchSettings {
            seed: 42,
            minutes: 1,
            knockout: false,
            managers: [stream::ManagerKind::Ai, stream::ManagerKind::Ai],
        }
    }

    /// A recorded match stores the trait catalogue after the slot file; a replay written before
    /// the catalogue joined the content lacks it, is accepted, and builds with the catalogue as
    /// it first shipped, which is the shipped one today, so the match is the same match.
    #[test]
    fn a_replay_without_the_trait_catalogue_builds_with_the_frozen_copy() {
        let inputs = InputFiles::read(Some(&shipped()), None, None, None).unwrap();
        let roles: Vec<&str> = inputs.0.iter().map(|f| f.role.as_str()).collect();
        assert_eq!(roles[7..], [SLOTS_ROLE, TRAITS_ROLE]);
        let held = inputs.get(TRAITS_ROLE).unwrap();
        assert_eq!(held.name, TRAITS_FILE);
        assert_eq!(held.bytes, FROZEN_TRAITS_V1.as_bytes());

        let older = InputFiles(
            inputs
                .0
                .iter()
                .filter(|f| f.role != TRAITS_ROLE)
                .cloned()
                .collect(),
        );
        let with = build(&inputs, &settings(), Backstop::Never, false).unwrap();
        let without = build(&older, &settings(), Backstop::Never, false).unwrap();
        assert_eq!(
            with.sim.config().content_hash,
            without.sim.config().content_hash
        );
    }
}
