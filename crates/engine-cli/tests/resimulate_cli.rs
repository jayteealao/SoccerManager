//! The replay file on the command line: `record` stores every input, the engine identity,
//! and the applied changes; `resimulate` reproduces the stored tick frames from the file
//! alone, with the content, team, and script files deleted; strict mode refuses another
//! engine and names the difference, comparison mode runs anyway and leaves the file as it
//! was, and a version-3 file is refused. The scheme is one number in the record, a snapshot,
//! the golden file, and the registry. A source guard keeps every path loader out of the
//! re-simulation path, so a replay never reads content from the checkout.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use engine::record::hex;
use sha2::{Digest, Sha256};
use stream::{ChangeSource, Fixture, LoggedChange, read_fixture, write_fixture};

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn temp(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("engine-cli-resim-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("the test folder is created");
    dir
}

/// The binary with its data folder in `data` and no content folder from the environment.
fn bin(data: &Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_engine-cli"));
    cmd.env("SM_DATA_DIR", data);
    cmd.env_remove("SM_CONTENT_DIR");
    cmd
}

fn sha256_file(path: &Path) -> String {
    hex(&Sha256::digest(std::fs::read(path).unwrap()))
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), &target).unwrap();
        }
    }
}

/// The changes of the fixture: at tick 3,000, the home team's slot-9 player off for its
/// first substitute and its mentality to 4, and an away substitution for a bench place the
/// team does not have, which the engine rejects.
const CHANGES: &str = r#"[
  { "tick": 3000, "team": 0, "change": { "substitution": { "slot": 9, "bench": 0 } } },
  { "tick": 3000, "team": 0, "change": { "mentality": 4 } },
  { "tick": 3000, "team": 1, "change": { "substitution": { "slot": 9, "bench": 99 } } }
]"#;

/// A recorded match: its folder, the copied content folder, and the replay file.
struct Recorded {
    dir: PathBuf,
    content: PathBuf,
    file: PathBuf,
    summary: serde_json::Value,
}

/// Copies the shipped content into a folder of its own and records `minutes` of seed 42
/// with the sample pack and the fixture's changes, from that copy only.
fn record(name: &str, minutes: u32) -> Recorded {
    let dir = temp(name);
    let content = dir.join("content");
    copy_dir(&repo().join("content"), &content);
    let changes = dir.join("changes.json");
    std::fs::write(&changes, CHANGES).unwrap();
    let file = dir.join("out").join("m.smfx");
    let out = bin(&dir.join("data"))
        .arg("--content-dir")
        .arg(&content)
        .args(["record", "--seed", "42", "--minutes", &minutes.to_string()])
        .arg("--script-pack")
        .arg(content.join("scripts/sample"))
        .arg("--changes")
        .arg(&changes)
        .arg("--out")
        .arg(&file)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "record: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let summary = serde_json::from_slice(&out.stdout).unwrap();
    Recorded {
        dir,
        content,
        file,
        summary,
    }
}

/// Runs `resimulate` on `file` from an empty working folder with `SM_CONTENT_DIR` naming a
/// folder that does not exist.
fn resimulate(dir: &Path, file: &Path, extra: &[&str]) -> Output {
    let empty = dir.join("empty-work-folder");
    std::fs::create_dir_all(&empty).unwrap();
    bin(&dir.join("data"))
        .current_dir(&empty)
        .env("SM_CONTENT_DIR", dir.join("no-such-content"))
        .arg("resimulate")
        .arg("--fixture")
        .arg(file)
        .args(extra)
        .output()
        .unwrap()
}

fn line(out: &Output) -> serde_json::Value {
    serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("{e}: {}", String::from_utf8_lossy(&out.stdout)))
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

/// The stored text frames that are a rejected change's verdict.
fn rejected_rows(fixture: &Fixture) -> usize {
    fixture
        .frames
        .iter()
        .filter_map(|f| match &f.frame {
            protocol::Frame::Text(text) => Some(text),
            protocol::Frame::Tick(_) => None,
        })
        .filter(|text| text.contains("\"change.state\":\"rejected\""))
        .count()
}

#[test]
fn a_replay_file_alone_reproduces_its_tick_frames_with_every_source_file_deleted() {
    let recorded = record("ac39", 90);
    // The fixture first: two home changes at one tick and one stoppage, the substitution
    // first, no away entry, and one rejected verdict row in the stored text frames.
    let fixture = read_fixture(&recorded.file).unwrap();
    let log = &fixture.record.as_ref().unwrap().changes;
    assert_eq!(log.len(), 2, "{log:?}");
    assert!(matches!(log[0].change, LoggedChange::Substitution { .. }));
    assert!(matches!(log[1].change, LoggedChange::Tactics { .. }));
    assert_eq!(
        (log[0].tick, &log[0].stoppage),
        (log[1].tick, &log[1].stoppage)
    );
    assert!(
        log.iter()
            .all(|c| c.team == 0 && c.source == ChangeSource::Manager)
    );
    assert_eq!(rejected_rows(&fixture), 1);
    assert_eq!(fixture.inputs.len(), 10);

    // Every source file goes: the content folder with its teams and the script pack, and
    // the change file.
    std::fs::remove_dir_all(&recorded.content).unwrap();
    std::fs::remove_file(recorded.dir.join("changes.json")).unwrap();
    assert!(!recorded.content.exists());

    let out = resimulate(&recorded.dir, &recorded.file, &[]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    let line = line(&out);
    assert_eq!(line["verdict"], "identical", "{line}");
    assert_eq!(line["stored_sha256"], line["resimulated_sha256"], "{line}");
    assert_eq!(line["changes_applied"], 2, "{line}");
    assert_eq!(line["first_difference"], serde_json::Value::Null, "{line}");
    assert_eq!(line["tick_frames"], recorded.summary["ticks"], "{line}");
    let _ = std::fs::remove_dir_all(&recorded.dir);
}

/// Copies the shipped content into `dir/content`, sets the slot file to `slots`, records
/// `minutes` of seed 42 from that copy only, and returns the replay file.
fn record_with_slots(dir: &Path, name: &str, slots: &str, minutes: u32) -> PathBuf {
    let content = dir.join(format!("content-{name}"));
    copy_dir(&repo().join("content"), &content);
    std::fs::write(content.join("slots.json"), slots).unwrap();
    let file = dir.join("out").join(format!("{name}.smfx"));
    let out = bin(&dir.join("data"))
        .arg("--content-dir")
        .arg(&content)
        .args(["record", "--seed", "42", "--minutes", &minutes.to_string()])
        .arg("--out")
        .arg(&file)
        .output()
        .unwrap();
    assert!(out.status.success(), "record: {}", stderr(&out));
    std::fs::remove_dir_all(&content).unwrap();
    file
}

#[test]
fn a_match_recorded_with_a_module_off_replays_with_it_off() {
    let dir = temp("slots-off");
    let default = r#"{"schema_version":1,"slots":{"engine.fouls":{"module":"fouls","version":1},"engine.offside":{"module":"offside","version":1},"engine.shot":{"module":"shot","version":1},"engine.fatigue":{"module":"fatigue","version":1},"engine.steering":{"module":"steering","version":1},"engine.pre-match":{"module":"pre-match","version":1},"engine.modifier.fatigue":{"module":"fatigue-curve","version":1},"engine.modifier.pressure":{"module":"pressure","version":1},"engine.modifier.momentum":{"module":"momentum","version":1},"engine.modifier.weather":{"module":"weather","version":1},"engine.clock":{"module":"clock","version":1},"engine.restarts":{"module":"restarts","version":1},"engine.discipline":{"module":"discipline","version":1},"engine.injuries":{"module":"injuries","version":1},"engine.ball":{"module":"ball","version":1},"engine.possession":{"module":"possession","version":1},"engine.decision":{"module":"decision","version":1},"engine.manager":{"module":"ai-manager","version":1},"engine.changes":{"module":"changes","version":1},"engine.hook.decision":{"module":"decision-hook","version":1},"engine.hook.rule":{"module":"rule-hook","version":1},"engine.hook.commentary":{"module":"commentary-hook","version":1},"game.rules":{"module":"rule-pack","version":1},"game.world":{"module":"world-stub","version":1},"game.season":{"module":"season-stub","version":1},"game.people":{"module":"people-stub","version":1},"game.presentation":{"module":"presentation-stub","version":1},"viewer.skin":{"module":"broadcast-blue","version":1},"engine.fast-model":{"module":"fitted-scores","version":1}}}"#;
    let fouls_off = r#"{"schema_version":1,"slots":{"engine.fouls":{"module":"off"},"engine.offside":{"module":"offside","version":1},"engine.shot":{"module":"shot","version":1},"engine.fatigue":{"module":"fatigue","version":1},"engine.steering":{"module":"steering","version":1},"engine.pre-match":{"module":"pre-match","version":1},"engine.modifier.fatigue":{"module":"fatigue-curve","version":1},"engine.modifier.pressure":{"module":"pressure","version":1},"engine.modifier.momentum":{"module":"momentum","version":1},"engine.modifier.weather":{"module":"weather","version":1},"engine.clock":{"module":"clock","version":1},"engine.restarts":{"module":"restarts","version":1},"engine.discipline":{"module":"discipline","version":1},"engine.injuries":{"module":"injuries","version":1},"engine.ball":{"module":"ball","version":1},"engine.possession":{"module":"possession","version":1},"engine.decision":{"module":"decision","version":1},"engine.manager":{"module":"ai-manager","version":1},"engine.changes":{"module":"changes","version":1},"engine.hook.decision":{"module":"decision-hook","version":1},"engine.hook.rule":{"module":"rule-hook","version":1},"engine.hook.commentary":{"module":"commentary-hook","version":1},"game.rules":{"module":"rule-pack","version":1},"game.world":{"module":"world-stub","version":1},"game.season":{"module":"season-stub","version":1},"game.people":{"module":"people-stub","version":1},"game.presentation":{"module":"presentation-stub","version":1},"viewer.skin":{"module":"broadcast-blue","version":1},"engine.fast-model":{"module":"fitted-scores","version":1}}}"#;
    let off = record_with_slots(&dir, "off", fouls_off, 30);
    let on = record_with_slots(&dir, "on", default, 30);

    // The slot file is an input of the replay, byte for byte.
    for (file, slots) in [(&off, fouls_off), (&on, default)] {
        let fixture = read_fixture(file).unwrap();
        let held = fixture
            .inputs
            .iter()
            .find(|f| f.role == "slots")
            .expect("the replay file holds the slot file");
        assert_eq!(held.name, "slots.json");
        assert_eq!(held.bytes, slots.as_bytes());
    }

    // Each replays identically from the file alone, so the off match is re-simulated with
    // fouls off and not with the built-in default. The two matches differ, so a replay that
    // took the default for both would fail one of them.
    let mut stored = Vec::new();
    for file in [&off, &on] {
        let out = resimulate(&dir, file, &[]);
        assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
        let line = line(&out);
        assert_eq!(line["verdict"], "identical", "{line}");
        stored.push(line["stored_sha256"].clone());
    }
    assert_ne!(stored[0], stored[1], "fouls off must change the match");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_new_replay_file_holds_the_engine_identity_every_input_and_every_applied_change() {
    let recorded = record("ac40", 3);
    let fixture = read_fixture(&recorded.file).unwrap();
    assert_eq!(fixture.format, stream::FORMAT_VERSION);
    let record = fixture.record.as_ref().unwrap();

    let engine = &record.engine;
    assert_eq!(engine.commit, engine::commit());
    assert_eq!(engine.dirty, engine::dirty());
    assert_eq!(engine.crate_version, engine::version());
    assert_eq!(engine.scheme, engine::rng::STREAM_SCHEME);
    assert_eq!(engine.maths, engine::trace::MATHS);
    assert_eq!(engine.build, engine::build_hash());
    assert_eq!(
        engine.executable_sha256,
        sha256_file(Path::new(env!("CARGO_BIN_EXE_engine-cli")))
    );

    assert_eq!(record.settings.seed, 42);
    assert_eq!(record.settings.minutes, 3);
    assert!(!record.settings.knockout);
    // Both teams are named in the change file, so both are managed by hand.
    assert_eq!(
        record.settings.managers,
        [stream::ManagerKind::Human, stream::ManagerKind::Human]
    );

    // Every input, byte for byte, against the file it was read from.
    let sources = [
        ("attributes", "attributes.json"),
        ("tuning", "tuning.json"),
        ("rules", "rules/default.json"),
        ("tactics", "tactics.json"),
        ("commentary", "commentary/en.json"),
        ("team_a", "teams/default-a.json"),
        ("team_b", "teams/default-b.json"),
        ("slots", "slots.json"),
        ("pack_manifest", "scripts/sample/pack.json"),
        ("pack_script", "scripts/sample/main.rhai"),
    ];
    assert_eq!(fixture.inputs.len(), sources.len());
    let mut total = 0u64;
    for ((role, source), (file, listed)) in sources
        .iter()
        .zip(fixture.inputs.iter().zip(&record.inputs))
    {
        let bytes = std::fs::read(recorded.content.join(source)).unwrap();
        assert_eq!(file.role, *role);
        assert_eq!(listed.role, *role);
        assert_eq!(file.bytes, bytes, "{role}");
        assert_eq!(listed.sha256, hex(&Sha256::digest(&bytes)), "{role}");
        assert_eq!(listed.bytes, bytes.len() as u64, "{role}");
        total += bytes.len() as u64;
    }
    assert_eq!(record.inputs_bytes, total);
    assert_eq!(recorded.summary["inputs_bytes"], total);
    assert_eq!(recorded.summary["format"], 4);

    // Every applied change with its tick, stoppage, and order.
    let log = &record.changes;
    assert_eq!(recorded.summary["changes_applied"], log.len());
    assert_eq!(log.len(), 2, "{log:?}");
    for (order, entry) in log.iter().enumerate() {
        assert_eq!(entry.order as usize, order);
        assert_eq!(entry.queued_tick, 3_000);
        assert!(entry.tick > 3_000, "{entry:?}");
        assert!(!entry.stoppage.is_empty(), "{entry:?}");
    }
    assert_eq!(
        log[1].change,
        LoggedChange::Tactics {
            formation: None,
            mentality: Some(4),
            instructions: [None; 6],
            roles: Vec::new(),
        }
    );
    let _ = std::fs::remove_dir_all(&recorded.dir);
}

#[test]
fn the_scheme_is_one_number_in_the_record_a_snapshot_the_golden_file_and_the_registry() {
    use engine::data::{TEAM_A_FILE, TEAM_B_FILE};
    use engine::snapshot::Snapshot;
    use engine::{Content, ContentDir, MatchConfig, Simulation};

    let recorded = record("ac26", 1);
    let record_scheme = read_fixture(&recorded.file)
        .unwrap()
        .record
        .unwrap()
        .engine
        .scheme;

    let dir = ContentDir::at(repo().join("content"));
    let content = Content::load(&dir).unwrap();
    let a = content
        .load_team(&dir, &dir.path(TEAM_A_FILE))
        .unwrap()
        .value;
    let b = content
        .load_team(&dir, &dir.path(TEAM_B_FILE))
        .unwrap()
        .value;
    let config = || MatchConfig::new(42, 1, &content, [&a, &b]).unwrap();
    let mut sim = Simulation::new(config()).unwrap();
    for _ in 0..500 {
        sim.step();
    }
    let snapshot = Snapshot::capture(&sim, [0; 16], 1);
    let snapshot = Snapshot::from_bytes(&snapshot.to_bytes(), "match.snap").unwrap();
    let snapshot_scheme = Simulation::from_snapshot(config(), &snapshot)
        .unwrap()
        .stream_scheme();

    let golden = engine::gate::golden::load_lenient(&repo().join("gate/golden.json")).unwrap();
    let golden_scheme = golden
        .ledger
        .iter()
        .rev()
        .find(|e| e.kind != engine::gate::golden::EntryKind::AddMachineSet)
        .unwrap()
        .scheme;

    let registry_scheme = engine::streams::KEYED_SCHEME;
    assert_eq!(
        [record_scheme, snapshot_scheme, golden_scheme],
        [registry_scheme; 3]
    );
    let _ = std::fs::remove_dir_all(&recorded.dir);
}

/// The recorded file rewritten with `patch` applied to its record.
fn patched(
    recorded: &Recorded,
    name: &str,
    patch: impl FnOnce(&mut stream::ReplayRecord),
) -> PathBuf {
    let mut fixture = read_fixture(&recorded.file).unwrap();
    patch(fixture.record.as_mut().unwrap());
    let path = recorded.dir.join(name);
    write_fixture(&path, &fixture).unwrap();
    path
}

#[test]
fn strict_mode_refuses_another_engine_by_name_and_runs_a_dirty_record_with_a_warning() {
    let recorded = record("ac41", 2);
    let other = "0".repeat(64);

    let file = patched(&recorded, "sha.smfx", |r| {
        r.engine.executable_sha256 = other.clone()
    });
    let out = resimulate(&recorded.dir, &file, &[]);
    assert_eq!(out.status.code(), Some(1), "{}", stderr(&out));
    assert!(
        stderr(&out).contains(&format!(
            "executable SHA-256 differs: the record has {other}"
        )),
        "{}",
        stderr(&out)
    );
    assert!(out.stdout.is_empty());

    let file = patched(&recorded, "scheme.smfx", |r| r.engine.scheme = 9);
    let out = resimulate(&recorded.dir, &file, &[]);
    assert_eq!(out.status.code(), Some(1), "{}", stderr(&out));
    assert!(
        stderr(&out).contains(&format!(
            "scheme 9 differs from this build's {}",
            engine::rng::STREAM_SCHEME
        )),
        "{}",
        stderr(&out)
    );

    let binary = sha256_file(Path::new(env!("CARGO_BIN_EXE_engine-cli")));
    let file = patched(&recorded, "dirty.smfx", |r| {
        r.engine.dirty = true;
        r.engine.executable_sha256 = binary.clone();
    });
    let out = resimulate(&recorded.dir, &file, &[]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    assert!(
        stderr(&out).contains("warning: recorded on a dirty build")
            && stderr(&out).contains("cannot be rebuilt"),
        "{}",
        stderr(&out)
    );
    assert_eq!(line(&out)["verdict"], "identical");

    // Controls: a clean record of this binary runs with no identity message, and so does
    // the file as recorded.
    let file = patched(&recorded, "clean.smfx", |r| r.engine.dirty = false);
    let out = resimulate(&recorded.dir, &file, &[]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    assert!(
        !stderr(&out).contains("differs") && !stderr(&out).contains("dirty build"),
        "{}",
        stderr(&out)
    );
    let out = resimulate(&recorded.dir, &recorded.file, &[]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    assert!(!stderr(&out).contains("differs"), "{}", stderr(&out));
    let _ = std::fs::remove_dir_all(&recorded.dir);
}

#[test]
fn comparison_mode_runs_on_another_identity_reports_both_and_leaves_the_file_as_it_was() {
    let recorded = record("acrr1", 2);
    let file = patched(&recorded, "other.smfx", |r| {
        r.engine.executable_sha256 = "f".repeat(64)
    });
    let before = sha256_file(&file);
    let out = resimulate(&recorded.dir, &file, &["--compare"]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    let err = stderr(&out);
    let scheme = engine::rng::STREAM_SCHEME;
    assert!(
        err.contains(&format!("engine (record): sha256 {}", "f".repeat(64))),
        "{err}"
    );
    assert!(err.contains("engine (this binary): sha256 "), "{err}");
    assert!(
        err.contains(&format!(
            "scheme {scheme} (record), scheme {scheme} (this binary)"
        )),
        "{err}"
    );
    let line = line(&out);
    assert_eq!(line["mode"], "compare");
    assert_eq!(line["verdict"], "identical", "{line}");
    assert_eq!(
        line["engines"]["record"]["executable_sha256"],
        "f".repeat(64)
    );
    assert_eq!(
        line["engines"]["this_binary"]["executable_sha256"],
        sha256_file(Path::new(env!("CARGO_BIN_EXE_engine-cli")))
    );
    assert_eq!(
        sha256_file(&file),
        before,
        "comparison mode rewrote the file"
    );
    let _ = std::fs::remove_dir_all(&recorded.dir);
}

#[test]
fn a_version_three_file_is_refused_because_it_holds_no_inputs() {
    let dir = temp("ac43");
    let legacy = repo().join("viewer/tests/data/one-minute.smfx");
    let out = resimulate(&dir, &legacy, &[]);
    assert_eq!(out.status.code(), Some(1));
    let err = stderr(&out);
    assert!(
        err.contains("one-minute.smfx") && err.contains("holds no inputs"),
        "{err}"
    );
    assert!(out.stdout.is_empty());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_content_folder_flag_is_refused_before_any_file_is_read() {
    let dir = temp("content-flag");
    let out = bin(&dir)
        .arg("--content-dir")
        .arg(repo().join("content"))
        .args(["resimulate", "--fixture"])
        .arg(dir.join("absent.smfx"))
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    let err = stderr(&out);
    assert!(
        err.contains("reads every input from the replay file"),
        "{err}"
    );
    assert!(
        !err.contains("absent.smfx"),
        "the file was read first: {err}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// The path loaders, and the region of `replay_inputs.rs` allowed to call them.
const PATH_LOADERS: [&str; 10] = [
    "ContentDir",
    "Content::load",
    "ContentFiles::read",
    "Commentary::load",
    "load_team",
    "Pack::read",
    "LoadedPack::load",
    "content::load",
    "read_bytes",
    "fs::read",
];
const BEGIN: &str = "// path-loading: begin";
const END: &str = "// path-loading: end";

/// Every path loader `source` names outside the allowed region.
fn path_loaders(source: &str) -> Vec<&'static str> {
    let mut outside = String::new();
    let mut rest = source;
    while let Some(start) = rest.find(BEGIN) {
        outside.push_str(&rest[..start]);
        rest = rest[start..]
            .find(END)
            .map_or("", |end| &rest[start + end..]);
    }
    outside.push_str(rest);
    PATH_LOADERS
        .into_iter()
        .filter(|loader| outside.contains(loader))
        .collect()
}

#[test]
fn the_resimulation_path_names_no_path_loader() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let inputs = std::fs::read_to_string(src.join("replay_inputs.rs")).unwrap();
    let command = std::fs::read_to_string(src.join("resimulate.rs")).unwrap();
    assert_eq!(
        path_loaders(&inputs),
        Vec::<&str>::new(),
        "replay_inputs.rs"
    );
    assert_eq!(path_loaders(&command), Vec::<&str>::new(), "resimulate.rs");
    // Controls: the guard sees the loaders inside the allowed region once the markers go,
    // and a loader added after the region.
    assert!(inputs.contains(BEGIN) && inputs.contains(END));
    let unmarked = inputs.replace(BEGIN, "").replace(END, "");
    assert!(
        path_loaders(&unmarked).contains(&"ContentDir"),
        "{:?}",
        path_loaders(&unmarked)
    );
    assert!(path_loaders(&unmarked).contains(&"Pack::read"));
    let added = format!("{inputs}\nfn sneak() {{ let _ = engine::Content::load(&dir); }}\n");
    assert_eq!(path_loaders(&added), ["Content::load"]);
}

/// Controls for the comparison itself: a changed stored tick frame and a changed log entry
/// are each reported as a difference, with exit code 2.
#[test]
fn a_changed_tick_frame_or_log_entry_is_reported_as_a_difference() {
    let recorded = record("differs", 3);

    let mut fixture = read_fixture(&recorded.file).unwrap();
    let (index, at) = fixture
        .frames
        .iter()
        .enumerate()
        .filter(|(_, f)| !f.frame.is_text())
        .map(|(i, _)| i)
        .enumerate()
        .nth(4_000)
        .unwrap();
    let mut bytes = fixture.frames[at].frame.payload().to_vec();
    let last = bytes.len() - 1;
    bytes[last] ^= 0x01;
    fixture.frames[at].frame =
        protocol::Frame::Tick(protocol::TickFrame::from_bytes(&bytes).unwrap());
    let file = recorded.dir.join("frame.smfx");
    write_fixture(&file, &fixture).unwrap();
    let out = resimulate(&recorded.dir, &file, &[]);
    assert_eq!(out.status.code(), Some(2), "{}", stderr(&out));
    let found = line(&out);
    assert_eq!(found["verdict"], "differs", "{found}");
    assert_eq!(found["first_difference"]["frame"], index, "{found}");
    assert_ne!(found["stored_sha256"], found["resimulated_sha256"]);

    let file = patched(&recorded, "log.smfx", |r| r.changes[1].tick += 1);
    let out = resimulate(&recorded.dir, &file, &[]);
    assert_eq!(out.status.code(), Some(2), "{}", stderr(&out));
    let found = line(&out);
    assert_eq!(
        found["first_difference"],
        serde_json::json!({ "change": 1 }),
        "{found}"
    );
    assert_eq!(found["stored_sha256"], found["resimulated_sha256"]);
    let _ = std::fs::remove_dir_all(&recorded.dir);
}

/// The committed version-4 replay file: one minute of seed 42, 3,000 tick frames.
fn committed_v4() -> PathBuf {
    repo().join("viewer/tests/data/one-minute-v4.smfx")
}

/// The lines of a state digest file.
fn digest_lines(path: &Path) -> Vec<String> {
    std::fs::read_to_string(path)
        .unwrap()
        .lines()
        .map(str::to_string)
        .collect()
}

/// The SHA-256 of the fields of a state fields file, joined in order.
fn joined_fields_sha(path: &Path) -> String {
    let fields: serde_json::Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let mut hasher = Sha256::new();
    for field in fields["fields"].as_array().unwrap() {
        let text = field["hex"].as_str().unwrap();
        let bytes: Vec<u8> = (0..text.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&text[i..i + 2], 16).unwrap())
            .collect();
        hasher.update(bytes);
    }
    hex(&hasher.finalize())
}

#[test]
fn state_digests_give_one_line_per_tick_and_the_same_file_twice() {
    // A minute of seed 42 recorded by this build: a file recorded by an older build plays on
    // the build that recorded it, and this build's results differ from it.
    let recorded = record("digests-rec", 1);
    let file = recorded.file.clone();
    let dir = temp("digests");
    let (a, b) = (dir.join("a.digests"), dir.join("b.digests"));
    for path in [&a, &b] {
        let out = resimulate(
            &dir,
            &file,
            &["--compare", "--state-digests", path.to_str().unwrap()],
        );
        assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
        assert_eq!(line(&out)["verdict"], "identical");
    }
    assert_eq!(std::fs::read(&a).unwrap(), std::fs::read(&b).unwrap());
    let lines = digest_lines(&a);
    let header: serde_json::Value = serde_json::from_str(&lines[0]).unwrap();
    assert_eq!(header["state_digests"], 1);
    assert_eq!(header["inventory"], 1);
    assert_eq!(header["gate_schema"], 1);
    assert_eq!(header["scheme"], header["engine"]["scheme"]);
    let ticks: Vec<&str> = lines[1..lines.len() - 2]
        .iter()
        .map(|l| l.as_str())
        .collect();
    assert_eq!(ticks.len(), 3_000);
    for (i, l) in ticks.iter().enumerate() {
        let (tick, digest) = l.split_once(' ').unwrap();
        assert_eq!(tick.parse::<usize>().unwrap(), i + 1);
        assert_eq!(digest.len(), 64);
    }
    assert!(lines[lines.len() - 2].starts_with("finish "));
    assert_eq!(lines[lines.len() - 1], "end 3000 true");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_tick_digest_is_the_sha256_of_that_ticks_fields_and_at_tick_stops_there() {
    // A minute of seed 42 recorded by this build: a file recorded by an older build plays on
    // the build that recorded it, and this build's results differ from it.
    let recorded = record("fields-rec", 1);
    let file = recorded.file.clone();
    let dir = temp("fields");
    let digests = dir.join("m.digests");
    let out = resimulate(
        &dir,
        &file,
        &["--compare", "--state-digests", digests.to_str().unwrap()],
    );
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    let lines = digest_lines(&digests);
    for tick in [1u32, 1_500, 3_000] {
        let fields = dir.join(format!("{tick}.json"));
        let at = tick.to_string();
        let out = resimulate(
            &dir,
            &file,
            &[
                "--compare",
                "--state-fields",
                fields.to_str().unwrap(),
                "--at-tick",
                &at,
            ],
        );
        let verdict = line(&out);
        assert_eq!(verdict["stopped_at"], tick, "{}", stderr(&out));
        let written: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&fields).unwrap()).unwrap();
        assert_eq!(written["tick"], tick);
        assert_eq!(written["fields"][0]["name"], "tick");
        let expected = format!("{tick} {}", joined_fields_sha(&fields));
        assert_eq!(lines[tick as usize], expected, "tick {tick}");
    }
    // A tick past the end writes no fields and fails.
    let fields = dir.join("late.json");
    let out = resimulate(
        &dir,
        &file,
        &[
            "--compare",
            "--state-fields",
            fields.to_str().unwrap(),
            "--at-tick",
            "9999",
        ],
    );
    assert_eq!(out.status.code(), Some(1), "{}", stderr(&out));
    assert!(
        stderr(&out).contains("before tick 9999"),
        "{}",
        stderr(&out)
    );
    assert!(!fields.exists());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_debug_trace_of_a_resimulation_counts_every_draw_and_stops_at_the_tick() {
    let dir = temp("trace");
    let (trace, fields) = (dir.join("t.jsonl"), dir.join("f.json"));
    let out = resimulate(
        &dir,
        &committed_v4(),
        &[
            "--compare",
            "--debug-trace",
            trace.to_str().unwrap(),
            "--state-fields",
            fields.to_str().unwrap(),
            "--at-tick",
            "1500",
        ],
    );
    let err = stderr(&out);
    assert_eq!(out.status.code(), Some(2), "{err}");
    assert_eq!(line(&out)["stopped_at"], 1500);
    let counts = err
        .lines()
        .find(|l| l.starts_with("debug trace: "))
        .unwrap_or_else(|| panic!("{err}"));
    let draws = |word: &str| -> u64 {
        let at = counts.find(word).unwrap() + word.len();
        counts[at..]
            .chars()
            .take_while(char::is_ascii_digit)
            .collect::<String>()
            .parse()
            .unwrap()
    };
    let (recorded, registry) = (
        counts
            .split(", ")
            .nth(1)
            .unwrap()
            .split(' ')
            .next()
            .unwrap()
            .parse::<u64>()
            .unwrap(),
        draws("(registry "),
    );
    assert!(recorded > 0);
    assert_eq!(recorded, registry, "{counts}");
    let text = std::fs::read_to_string(&trace).unwrap();
    let mut records = text.lines();
    let header: serde_json::Value = serde_json::from_str(records.next().unwrap()).unwrap();
    assert_eq!(header["seed"], 42);
    let ticks: Vec<u64> = records
        .map(|l| {
            let r: serde_json::Value = serde_json::from_str(l).unwrap();
            assert!(r.get("k").is_some(), "{l}");
            r["t"].as_u64().unwrap()
        })
        .collect();
    assert!(ticks.contains(&1500));
    assert_eq!(ticks.iter().max(), Some(&1500));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_resimulation_stopped_at_a_tick_writes_no_full_time_record_to_its_trace() {
    // A minute of seed 42 recorded by this build: a file recorded by an older build plays on
    // the build that recorded it, and this build's results differ from it.
    let recorded = record("stopped-trace-rec", 1);
    let file = recorded.file.clone();
    let dir = temp("stopped-trace");
    let (stopped, whole) = (dir.join("stopped.jsonl"), dir.join("whole.jsonl"));
    let fields = dir.join("f.json");
    let out = resimulate(
        &dir,
        &file,
        &[
            "--compare",
            "--debug-trace",
            stopped.to_str().unwrap(),
            "--state-fields",
            fields.to_str().unwrap(),
            "--at-tick",
            "2000",
        ],
    );
    assert_eq!(out.status.code(), Some(2), "{}", stderr(&out));
    let out = resimulate(
        &dir,
        &file,
        &["--compare", "--debug-trace", whole.to_str().unwrap()],
    );
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    let full_time = |path: &Path| {
        std::fs::read_to_string(path)
            .unwrap()
            .lines()
            .filter(|l| l.contains("\"point\":\"full_time\""))
            .count()
    };
    assert_eq!(full_time(&stopped), 0);
    // Control: the whole match records its full time.
    assert_eq!(full_time(&whole), 1);
    // The records of the stopped run are the whole run's records up to its tick.
    let upto = |path: &Path, last: u64| -> Vec<String> {
        std::fs::read_to_string(path)
            .unwrap()
            .lines()
            .skip(1)
            .filter(|l| {
                let r: serde_json::Value = serde_json::from_str(l).unwrap();
                r["t"].as_u64().unwrap() <= last
            })
            .map(str::to_string)
            .collect()
    };
    assert_eq!(upto(&stopped, 2000), upto(&whole, 2000));
    let _ = std::fs::remove_dir_all(&dir);
}
