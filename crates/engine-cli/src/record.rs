//! `engine-cli record`: write one whole match stream to a replay file, with no client. The
//! file holds every input of the match by value, the engine identity, and, at full time, the
//! applied-change log and the watchdog mark, so `resimulate` can play it again from the
//! file alone.

use std::path::Path;

use anyhow::{Context, bail};
use engine::gate::{PlannedChange, PlannedWhat};
use engine::observe::identity::{MatchId, data_dir, load_or_create_owner_id};
use protocol::{Hello, PROTOCOL_VERSION, ServerMessage};
use script::Backstop;
use stream::session::{FrameOut, FrameSink, MatchState};
use stream::{
    EngineIdentity, FORMAT_VERSION, ManagerKind, MatchSettings, Recorder, SharedRecorder,
    outcome_of,
};

use crate::cli::RecordOpts;
use crate::replay_inputs::{Built, InputFiles, build};
use crate::stream_run::{Drive, Planned, drive, hello_substitutions, hello_tactics, hello_teams};

pub fn run(content_dir: Option<&Path>, opts: &RecordOpts) -> anyhow::Result<i32> {
    let changes = match &opts.changes {
        Some(path) => read_changes(path)?,
        None => Vec::new(),
    };
    let inputs = InputFiles::read(
        content_dir,
        opts.team_a.as_deref(),
        opts.team_b.as_deref(),
        opts.script_pack.as_deref(),
    )?;
    // A team the change file names is managed by hand, as in the replay gate's change
    // fixture, so the computer manager makes no change of its own for it.
    let mut managers = [ManagerKind::Ai; 2];
    for change in &changes {
        managers[change.team] = ManagerKind::Human;
    }
    let settings = MatchSettings {
        seed: opts.seed,
        minutes: opts.minutes,
        knockout: opts.knockout,
        managers,
    };
    let Built {
        mut sim,
        commentary,
        keyframe_interval,
    } = build(&inputs, &settings, Backstop::default(), false)?;
    let owner_id = load_or_create_owner_id(&data_dir())?;
    let match_id = MatchId::now(opts.seed);
    let ticks = sim.config().max_ticks();
    let club_ids = [
        sim.config().teams[0].club_id.clone(),
        sim.config().teams[1].club_id.clone(),
    ];

    let recorder = SharedRecorder::new(Recorder::create_record(
        &opts.out,
        match_id.millis,
        EngineIdentity::current()?,
        settings,
        &inputs.0,
    )?);
    let mut sink = FrameSink::new(recorder.clone(), keyframe_interval);
    let mut messages = recorder.clone();

    // The hello is the fixture's first frame, so a replay forwards the recorded match rather
    // than describing itself. Without it a fixture carries no team name, no kit colour, and
    // no recorded keyframe interval, and a replayer can only invent all three. The simulation
    // comes first, because its pre-match setup settles the lineup the roster names.
    let hello = Hello {
        protocol_version: PROTOCOL_VERSION,
        engine_version: engine::version().to_string(),
        build_hash: engine::build_hash().to_string(),
        owner_id: owner_id.clone(),
        match_id: match_id.to_string(),
        seed: opts.seed,
        dt_ms: sim.config().tuning.dt * 1000.0,
        ticks_expected: ticks,
        keyframe_interval,
        teams: hello_teams(&sim, false),
        tactics: hello_tactics(&sim),
        substitutions: hello_substitutions(&sim),
        knockout: false,
    };
    messages.send(protocol::Frame::Text(
        serde_json::to_string(&ServerMessage::Hello(Box::new(hello)))
            .map_err(|source| protocol::ProtocolError::Json { source })?,
    ))?;

    let planned: Vec<Planned> = changes.into_iter().map(Planned::Slot).collect();
    let state = MatchState::default();
    let match_id = match_id.to_string();
    let driven = drive(
        &mut sim,
        &mut sink,
        &Drive {
            ticks,
            owner_id: &owner_id,
            match_id: &match_id,
            club_ids: [&club_ids[0], &club_ids[1]],
            state: &state,
            gate: None,
            commentary: &commentary,
            inbox: None,
            page_changes: None,
            planned: &planned,
            observe: None,
        },
        &mut |message: ServerMessage| {
            let text = serde_json::to_string(&message)
                .map_err(|source| protocol::ProtocolError::Json { source })?;
            messages.send(protocol::Frame::Text(text))
        },
    )?;

    drop(sink);
    drop(messages);
    let outcome = outcome_of(&sim);
    let changes_applied = outcome.changes.len();
    let summary = recorder.finish_record(outcome)?;
    println!(
        "{}",
        serde_json::json!({
            "fixture": opts.out.display().to_string(),
            "frames": summary.frames,
            "ticks": summary.ticks,
            "bytes": summary.bytes,
            "hash": summary.hash,
            "format": FORMAT_VERSION,
            "inputs_bytes": inputs.bytes(),
            "changes_applied": changes_applied,
        })
    );
    Ok(if driven.full_time { 0 } else { 2 })
}

/// One change of a change file, queued before the step that starts on its tick.
#[derive(serde::Deserialize)]
struct FileChange {
    tick: u32,
    team: usize,
    change: FileWhat,
}

/// What a change of a change file does.
#[derive(serde::Deserialize)]
#[serde(rename_all = "snake_case")]
enum FileWhat {
    /// The player in lineup `slot` off, the bench player at `bench` on.
    Substitution { slot: usize, bench: usize },
    /// The team's mentality set to this index of the tactics file.
    Mentality(u8),
}

impl From<FileChange> for PlannedChange {
    fn from(file: FileChange) -> Self {
        Self {
            tick: file.tick,
            team: file.team,
            change: match file.change {
                FileWhat::Substitution { slot, bench } => PlannedWhat::Substitution { slot, bench },
                FileWhat::Mentality(index) => PlannedWhat::Mentality(index),
            },
        }
    }
}

/// Reads a change file: a JSON array of changes, each queued before the step that starts on
/// its tick.
fn read_changes(path: &Path) -> anyhow::Result<Vec<PlannedChange>> {
    let shown = path.display();
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("cannot read the change file {shown}"))?;
    let changes: Vec<FileChange> = serde_json::from_str(&text)
        .with_context(|| format!("the change file {shown} is not a list of changes"))?;
    let changes: Vec<PlannedChange> = changes.into_iter().map(PlannedChange::from).collect();
    for (i, change) in changes.iter().enumerate() {
        if change.team > 1 {
            bail!(
                "the change file {shown}: change {i} names team {}; use 0 (home) or 1 (away)",
                change.team
            );
        }
    }
    Ok(changes)
}
