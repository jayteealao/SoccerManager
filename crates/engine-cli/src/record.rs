//! `engine-cli record`: write one whole match stream to a fixture file, with no client.

use std::path::Path;

use engine::observe::identity::{MatchId, data_dir, load_or_create_owner_id};
use engine::{MatchConfig, Simulation};
use protocol::{Hello, PROTOCOL_VERSION, ServerMessage};
use stream::session::{FrameOut, FrameSink, MatchState};
use stream::{Recorder, SharedRecorder};

use crate::cli::RecordOpts;
use crate::stream_run::{Drive, drive, hello_teams};

pub fn run(content_dir: Option<&Path>, opts: &RecordOpts) -> anyhow::Result<i32> {
    let loaded = crate::content::load(content_dir, opts.team_a.as_deref(), opts.team_b.as_deref())?;
    let [team_a, team_b] = &loaded.teams;
    let config = MatchConfig::new(opts.seed, opts.minutes, &loaded.content, [team_a, team_b])?;
    let keyframe_interval = loaded.content.tuning.stream.keyframe_interval;
    let owner_id = load_or_create_owner_id(&data_dir())?;
    let match_id = MatchId::now(opts.seed);
    let ticks = config.max_ticks();
    let club_ids = [
        config.teams[0].club_id.clone(),
        config.teams[1].club_id.clone(),
    ];

    let recorder = SharedRecorder::new(Recorder::create(&opts.out, match_id.millis, opts.seed)?);
    let mut sink = FrameSink::new(recorder.clone(), keyframe_interval);
    let mut messages = recorder.clone();

    // The hello is the fixture's first entry, so a replay forwards the recorded match rather
    // than describing itself. Without it a fixture carries no team name, no kit colour, and
    // no recorded keyframe interval, and a replayer can only invent all three. The simulation
    // comes first, because its pre-match setup settles the lineup the roster names.
    let dt = config.tuning.dt;
    let mut sim = Simulation::new(config)?;
    let hello = Hello {
        protocol_version: PROTOCOL_VERSION,
        engine_version: engine::version().to_string(),
        build_hash: engine::build_hash().to_string(),
        owner_id: owner_id.clone(),
        match_id: match_id.to_string(),
        seed: opts.seed,
        dt_ms: dt * 1000.0,
        ticks_expected: ticks,
        keyframe_interval,
        teams: hello_teams(&sim),
    };
    messages.send(protocol::Frame::Text(
        serde_json::to_string(&ServerMessage::Hello(hello))
            .map_err(|source| protocol::ProtocolError::Json { source })?,
    ))?;

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
            commentary: &loaded.commentary,
        },
        &mut |message: ServerMessage| {
            let text = serde_json::to_string(&message)
                .map_err(|source| protocol::ProtocolError::Json { source })?;
            messages.send(protocol::Frame::Text(text))
        },
    )?;

    drop(sink);
    drop(messages);
    let summary = recorder.finish()?;
    println!(
        "{}",
        serde_json::json!({
            "fixture": opts.out.display().to_string(),
            "frames": summary.frames,
            "ticks": summary.ticks,
            "bytes": summary.bytes,
            "hash": summary.hash,
        })
    );
    Ok(if driven.full_time { 0 } else { 2 })
}
