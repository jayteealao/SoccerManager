//! The one match driver every streaming command shares: it steps the simulation, feeds the
//! tick sink, announces every stoppage through the sink's stoppage hook, and routes every
//! engine event and the closing statistics to a caller-chosen destination (a socket, a
//! fixture, or both).

use engine::record::TickSink;
use engine::{Card, EngineEvent, EngineEventKind, Simulation};
use protocol::{CardKind, EventType, MatchEvent, ServerMessage, Stats};
use stream::session::MatchState;
use stream::{Gate, StreamError};

/// Where a driven match sends its messages.
pub type MessageRoute<'a> = &'a mut dyn FnMut(ServerMessage) -> Result<(), StreamError>;

/// Everything the driver needs beyond the simulation itself.
pub struct Drive<'a> {
    /// The most ticks to play. The match ends earlier at full time.
    pub ticks: u32,
    pub owner_id: &'a str,
    pub match_id: &'a str,
    /// The two club identifiers, home first, as `team.id` names them.
    pub club_ids: [&'a str; 2],
    pub state: &'a MatchState,
    /// The start and pause gate, when a client can pause this run.
    pub gate: Option<&'a Gate>,
}

/// What a driven match did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Driven {
    pub written: u32,
    /// `true` when the referee ended the match; `false` when the tick cap or a departed client
    /// ended it first.
    pub full_time: bool,
}

/// Runs the match to full time, or to `ticks` when that comes first.
pub fn drive<S: TickSink>(
    sim: &mut Simulation,
    sink: &mut S,
    opts: &Drive<'_>,
    route: MessageRoute<'_>,
) -> Result<Driven, StreamError> {
    let player_ids = sim.player_ids();
    let mut written = 0u32;
    while !sim.is_over() && written < opts.ticks {
        if let Some(gate) = opts.gate
            && !gate.wait_until_running()
        {
            break;
        }
        sim.step();
        let record = sim.record();
        opts.state.set_tick(record.tick);
        if let Err(err) = sink.on_tick(&record) {
            return gone(err, written);
        }
        written += 1;
        if let Some(stoppage) = sim.stoppage()
            && let Err(err) = sink.on_stoppage(&stoppage, sim)
        {
            return gone(err, written);
        }
        for event in sim.take_events() {
            opts.state.set_scores(event.scores);
            route(ServerMessage::Event(match_event(&event, opts, &player_ids)))?;
        }
    }
    let full_time = sim.is_over();
    sim.finish();
    for event in sim.take_events() {
        if let Err(err) = route(ServerMessage::Event(match_event(&event, opts, &player_ids))) {
            return closing_or_fail(err, written).map(|written| Driven { written, full_time });
        }
    }
    let summary = sim.summary();
    if let Err(err) = route(ServerMessage::Stats(Stats {
        tick: sim.tick(),
        minute: sim.minute().0,
        home_score: summary.goals[0],
        away_score: summary.goals[1],
        possession_changes: summary.possession_changes,
        ball_max_speed: summary.ball_max_speed,
        ball_idle_ticks: summary.ball_idle_ticks,
    })) {
        return closing_or_fail(err, written).map(|written| Driven { written, full_time });
    }
    Ok(Driven { written, full_time })
}

/// A sink failure mid-match: a departed client ends the run short of full time.
fn gone(err: engine::EngineError, written: u32) -> Result<Driven, StreamError> {
    finish_or_fail(err, written).map(|written| Driven {
        written,
        full_time: false,
    })
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

/// The same rule for the closing messages. A viewer that closes its page before full time
/// must not turn a finished run into a failure, and the caller still reports the short run.
fn closing_or_fail(err: StreamError, written: u32) -> Result<u32, StreamError> {
    match err {
        StreamError::ClientGone => {
            tracing::info!(
                signal = "socket.client_gone",
                written,
                reason = "the viewer disconnected before the closing messages"
            );
            Ok(written)
        }
        other => Err(other),
    }
}

/// One engine event as the `match-event` record kind names it.
fn match_event(event: &EngineEvent, opts: &Drive<'_>, player_ids: &[String]) -> MatchEvent {
    let event_type = match event.kind {
        EngineEventKind::KickOff => EventType::KickOff,
        EngineEventKind::Goal => EventType::Goal,
        EngineEventKind::HalfTime => EventType::HalfTime,
        EngineEventKind::FullTime => EventType::FullTime,
        EngineEventKind::Offside => EventType::Offside,
        EngineEventKind::Foul => EventType::Foul,
        EngineEventKind::Card => EventType::Card,
        EngineEventKind::ThrowIn => EventType::ThrowIn,
        EngineEventKind::Corner => EventType::Corner,
        EngineEventKind::GoalKick => EventType::GoalKick,
        EngineEventKind::FreeKick => EventType::FreeKick,
        EngineEventKind::Penalty => EventType::Penalty,
    };
    let id = |i: Option<usize>| i.and_then(|i| player_ids.get(i).cloned());
    MatchEvent::play(
        opts.owner_id,
        opts.match_id,
        event.tick,
        event_type,
        event.team.map(|t| opts.club_ids[t].to_string()),
        event.scores,
    )
    .at_minute(event.minute, event.minute_added)
    .player(id(event.player))
    .secondary(id(event.secondary))
    .card(event.card.map(card_kind))
    .advantage(event.advantage)
    .added_time(event.added_time_s)
}

/// The engine card as the protocol names it.
pub fn card_kind(card: Card) -> CardKind {
    match card {
        Card::Yellow => CardKind::Yellow,
        Card::SecondYellow => CardKind::SecondYellow,
        Card::Red => CardKind::Red,
    }
}
