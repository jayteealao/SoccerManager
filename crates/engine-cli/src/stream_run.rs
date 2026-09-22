//! The one match driver every streaming command shares: it steps the simulation, feeds the
//! tick sink, and routes every engine event and the closing statistics to a caller-chosen
//! destination (a socket, a fixture, or both).

use engine::record::TickSink;
use engine::{EngineEvent, EngineEventKind, Simulation};
use protocol::{EventType, MatchEvent, ServerMessage, Stats};
use stream::session::MatchState;
use stream::{Gate, StreamError};

/// Simulated ticks in one minute of play.
const TICKS_PER_MINUTE: u32 = 50 * 60;

/// Where a driven match sends its messages.
pub type MessageRoute<'a> = &'a mut dyn FnMut(ServerMessage) -> Result<(), StreamError>;

/// Everything the driver needs beyond the simulation itself.
pub struct Drive<'a> {
    pub ticks: u32,
    pub owner_id: &'a str,
    pub match_id: &'a str,
    /// The two club identifiers, home first, as `team.id` names them.
    pub club_ids: [&'a str; 2],
    pub state: &'a MatchState,
    /// The start and pause gate, when a client can pause this run.
    pub gate: Option<&'a Gate>,
}

/// Runs the match. Returns the number of ticks written, which is short of `ticks` only when
/// the client went away.
pub fn drive<S: TickSink>(
    sim: &mut Simulation,
    sink: &mut S,
    opts: &Drive<'_>,
    route: MessageRoute<'_>,
) -> Result<u32, StreamError> {
    let mut written = 0u32;
    for _ in 0..opts.ticks {
        if let Some(gate) = opts.gate
            && !gate.wait_until_running()
        {
            break;
        }
        sim.step();
        let record = sim.record();
        opts.state.set_tick(record.tick);
        if let Err(err) = sink.on_tick(&record) {
            return finish_or_fail(err, written);
        }
        written += 1;
        for event in sim.take_events() {
            opts.state.set_scores(event.scores);
            route(ServerMessage::Event(match_event(&event, opts)))?;
        }
    }
    sim.finish();
    for event in sim.take_events() {
        route(ServerMessage::Event(match_event(&event, opts)))?;
    }
    let summary = sim.summary();
    route(ServerMessage::Stats(Stats {
        tick: sim.tick(),
        minute: sim.tick() / TICKS_PER_MINUTE,
        home_score: summary.goals[0],
        away_score: summary.goals[1],
        possession_changes: summary.possession_changes,
        ball_max_speed: summary.ball_max_speed,
        ball_idle_ticks: summary.ball_idle_ticks,
    }))?;
    Ok(written)
}

/// A client that closes early ends the run; anything else is a real failure.
fn finish_or_fail(err: engine::EngineError, written: u32) -> Result<u32, StreamError> {
    match err {
        engine::EngineError::Sink(reason) => {
            tracing::info!(signal = "socket.client_gone", written, reason = %reason);
            Ok(written)
        }
        other => Err(StreamError::from(other)),
    }
}

/// One engine event as the `match-event` record kind names it.
fn match_event(event: &EngineEvent, opts: &Drive<'_>) -> MatchEvent {
    let event_type = match event.kind {
        EngineEventKind::KickOff => EventType::KickOff,
        EngineEventKind::Goal => EventType::Goal,
        EngineEventKind::FullTime => EventType::FullTime,
    };
    MatchEvent::play(
        opts.owner_id,
        opts.match_id,
        event.tick,
        event_type,
        event.team.map(|t| opts.club_ids[t].to_string()),
        event.scores,
    )
}
