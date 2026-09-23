//! The one match driver every streaming command shares: it steps the simulation, feeds the
//! tick sink, announces every stoppage through the sink's stoppage hook, and routes every
//! engine event, the running statistics and energy every simulated second, and the closing
//! statistics to a caller-chosen destination (a socket, a fixture, or both).

use engine::observe::{MatchFigures, round_to};
use engine::record::TickSink;
use engine::{
    Card, Commentary, Commentator, EngineEvent, EngineEventKind, EventDetail, Simulation,
};
use protocol::{
    CardKind, ChangeKind, ChangeOutcome, ChangeState, Condition, EventType, MatchEvent,
    RosterEntry, ServerMessage, Stats, TeamRef,
};
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
    /// The lines the commentator chooses from; every play event carries one.
    pub commentary: &'a Commentary,
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
    let mut ids = Ids::new(sim);
    let mut commentator = Commentator::for_match(opts.commentary, sim);
    // One simulated second of ticks: the cadence of the running statistics and energy.
    let ticks_per_second = ((1.0 / sim.tuning().dt).round() as u32).max(1);
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
            let line = commentator.line(&event);
            route(ServerMessage::Event(
                match_event(
                    &event,
                    opts.owner_id,
                    opts.match_id,
                    opts.club_ids,
                    &mut ids,
                )
                .commentary(line),
            ))?;
        }
        if record.tick.is_multiple_of(ticks_per_second) {
            route(ServerMessage::Stats(stats_message(sim)))?;
            route(ServerMessage::Condition(condition_message(sim)))?;
        }
    }
    let full_time = sim.is_over();
    sim.finish();
    for event in sim.take_events() {
        let line = commentator.line(&event);
        let message = match_event(
            &event,
            opts.owner_id,
            opts.match_id,
            opts.club_ids,
            &mut ids,
        )
        .commentary(line);
        if let Err(err) = route(ServerMessage::Event(message)) {
            return closing_or_fail(err, written).map(|written| Driven { written, full_time });
        }
    }
    if let Err(err) = route(ServerMessage::Stats(stats_message(sim))) {
        return closing_or_fail(err, written).map(|written| Driven { written, full_time });
    }
    Ok(Driven { written, full_time })
}

/// The running totals at the current tick. The shares and expected goals come from
/// `MatchFigures`, so they round exactly as the `match-stats` record does.
pub(crate) fn stats_message(sim: &Simulation) -> Stats {
    let summary = sim.summary();
    let figures = MatchFigures::new(&summary, sim.managers());
    Stats {
        tick: sim.tick(),
        minute: sim.minute().0,
        home_score: summary.goals[0],
        away_score: summary.goals[1],
        possession_changes: summary.possession_changes,
        ball_max_speed: summary.ball_max_speed,
        ball_idle_ticks: summary.ball_idle_ticks,
        possession_pct: figures.possession_pct,
        shots: summary.shots,
        shots_on_target: figures.shots_on_target,
        xg: figures.xg,
        passes: figures.passes,
        pass_accuracy_pct: figures.pass_accuracy_pct,
        fouls: summary.fouls,
        corners: summary.corners,
        offsides: summary.offsides,
    }
}

/// Every wire slot's energy at the current tick, home first, three decimals.
pub(crate) fn condition_message(sim: &Simulation) -> Condition {
    Condition {
        tick: sim.tick(),
        energy: sim
            .players()
            .iter()
            .map(|p| round_to(p.energy, 3))
            .collect(),
    }
}

/// The two clubs as the hello names them, home first, each with its roster: the 11 starters
/// in wire-slot order, then the named bench in bench order. Call it after `Simulation::new`,
/// because the computer manager's pre-match setup there settles the lineup.
pub(crate) fn hello_teams(sim: &Simulation) -> [TeamRef; 2] {
    sim.teams().map(|team| {
        let entry = |squad: usize| RosterEntry {
            id: team.player_ids[squad].clone(),
            name: team.player_names[squad].clone(),
            shirt: team.squad[squad].shirt,
            position: team.squad[squad].position.code().to_string(),
            squad_index: u32::try_from(squad).unwrap_or(u32::MAX),
        };
        let roster = team
            .lineup
            .iter()
            .chain(team.bench.iter())
            .map(|&squad| entry(squad))
            .collect();
        TeamRef {
            id: team.club_id.clone(),
            name: team.name.clone(),
            kit_primary: team.kit.primary.clone(),
            kit_secondary: team.kit.secondary.clone(),
            roster,
        }
    })
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

/// The player identifiers events name: each roster slot's current player, and each team's
/// squad in file order. A substitution event updates its roster slot, so an earlier event on
/// the same tick still names the player who left.
pub(crate) struct Ids {
    roster: Vec<String>,
    squads: [Vec<String>; 2],
}

impl Ids {
    /// The identifiers at the start of `sim`.
    pub(crate) fn new(sim: &Simulation) -> Self {
        Self {
            roster: sim.player_ids(),
            squads: sim.teams().map(|t| t.player_ids),
        }
    }
}

/// One engine event as the `match-event` record kind names it. `club_ids` are the two
/// `team.id` values, home first.
pub(crate) fn match_event(
    event: &EngineEvent,
    owner_id: &str,
    match_id: &str,
    club_ids: [&str; 2],
    ids: &mut Ids,
) -> MatchEvent {
    let team_id = event.team.map(|t| club_ids[t].to_string());
    let squad_id = |team: Option<usize>, s: usize| team.and_then(|t| ids.squads[t].get(s).cloned());
    if let Some(EventDetail::Change { id, kind, reason }) = event.detail {
        let kind = match kind {
            engine::ChangeKind::Tactics => ChangeKind::Tactics,
            engine::ChangeKind::Substitution => ChangeKind::Substitution,
        };
        let squads = event.team.map_or(&[][..], |t| &ids.squads[t][..]);
        let outcome = ChangeOutcome {
            kind: Some(kind),
            queue_id: Some(id.to_string()),
            state: if reason.is_some() {
                ChangeState::Rejected
            } else {
                ChangeState::Applied
            },
            rejected_reason: reason.map(|r| r.text(squads)),
        };
        return MatchEvent::change(owner_id, match_id, event.tick, event.scores, outcome)
            .at_minute(event.minute, event.minute_added)
            .team(team_id)
            .queued_at(id.tick)
            .applied_tick(reason.is_none().then_some(event.tick));
    }
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
        EngineEventKind::Injury => EventType::Injury,
        EngineEventKind::Substitution => EventType::Substitution,
        EngineEventKind::AiDecision => EventType::AiDecision,
        // A verdict always carries its change detail and returned above.
        EngineEventKind::ChangeApplied | EngineEventKind::ChangeRejected => {
            EventType::TacticsChange
        }
    };
    let (player, secondary) = match event.detail {
        Some(EventDetail::Substitution { off, on }) => {
            let (off_id, on_id) = (squad_id(event.team, off), squad_id(event.team, on));
            if let (Some(i), Some(on_id)) = (event.player, on_id.clone()) {
                ids.roster[i] = on_id;
            }
            (off_id, on_id)
        }
        _ => {
            let id = |i: Option<usize>| i.and_then(|i| ids.roster.get(i).cloned());
            (id(event.player), id(event.secondary))
        }
    };
    let ai_decision = match event.detail {
        Some(EventDetail::Ai { code }) => Some(code.code().to_string()),
        _ => None,
    };
    MatchEvent::play(
        owner_id,
        match_id,
        event.tick,
        event_type,
        team_id,
        event.scores,
    )
    .at_minute(event.minute, event.minute_added)
    .player(player)
    .secondary(secondary)
    .card(event.card.map(card_kind))
    .advantage(event.advantage)
    .added_time(event.added_time_s)
    .ai_decision(ai_decision)
}

/// The engine card as the protocol names it.
pub fn card_kind(card: Card) -> CardKind {
    match card {
        Card::Yellow => CardKind::Yellow,
        Card::SecondYellow => CardKind::SecondYellow,
        Card::Red => CardKind::Red,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine::MatchConfig;
    use engine::record::VecSink;
    use std::path::Path;

    fn loaded() -> crate::content::Loaded {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
        crate::content::load(Some(&dir), None, None).unwrap()
    }

    /// Every message of a seeded match of `minutes`, and the simulation after full time.
    fn driven(minutes: u32) -> (Vec<ServerMessage>, Simulation) {
        let loaded = loaded();
        let [a, b] = &loaded.teams;
        let config = MatchConfig::new(42, minutes, &loaded.content, [a, b]).unwrap();
        let ticks = config.max_ticks();
        let mut sim = Simulation::new(config).unwrap();
        let state = MatchState::default();
        let mut messages = Vec::new();
        drive(
            &mut sim,
            &mut VecSink::default(),
            &Drive {
                ticks,
                owner_id: "0123456789abcdef0123456789abcdef",
                match_id: "000000000000002a-1",
                club_ids: ["club-a", "club-b"],
                state: &state,
                gate: None,
                commentary: &loaded.commentary,
            },
            &mut |m: ServerMessage| {
                messages.push(m);
                Ok(())
            },
        )
        .unwrap();
        (messages, sim)
    }

    #[test]
    fn the_hello_roster_lists_the_starters_in_slot_order_then_the_bench() {
        let loaded = loaded();
        let [a, b] = &loaded.teams;
        let config = MatchConfig::new(42, 2, &loaded.content, [a, b]).unwrap();
        let sim = Simulation::new(config).unwrap();
        let teams = hello_teams(&sim);
        let wire_ids = sim.player_ids();
        for (t, team) in sim.teams().iter().enumerate() {
            let roster = &teams[t].roster;
            assert_eq!(roster.len(), 11 + team.bench.len());
            for (slot, entry) in roster.iter().take(11).enumerate() {
                assert_eq!(entry.id, wire_ids[t * 11 + slot], "team {t} slot {slot}");
                assert_eq!(entry.squad_index as usize, team.lineup[slot]);
            }
            for (entry, &squad) in roster.iter().skip(11).zip(team.bench.iter()) {
                assert_eq!(entry.squad_index as usize, squad);
            }
            for entry in roster {
                assert!(team.player_ids.contains(&entry.id), "{}", entry.id);
                assert!(!entry.position.is_empty() && entry.shirt > 0, "{entry:?}");
            }
            assert_eq!(teams[t].id, team.club_id);
        }
    }

    #[test]
    fn statistics_and_energy_are_sent_every_simulated_second_and_statistics_at_full_time() {
        let (messages, sim) = driven(2);
        let stats: Vec<&Stats> = messages
            .iter()
            .filter_map(|m| match m {
                ServerMessage::Stats(s) => Some(s),
                _ => None,
            })
            .collect();
        let conditions: Vec<&Condition> = messages
            .iter()
            .filter_map(|m| match m {
                ServerMessage::Condition(c) => Some(c),
                _ => None,
            })
            .collect();
        let played = sim.tick();
        assert_eq!(stats.len() as u32, played / 50 + 1, "{played} ticks");
        assert_eq!(conditions.len() as u32, played / 50);
        assert!(stats.windows(2).all(|w| w[0].tick <= w[1].tick));
        let last = stats.last().unwrap();
        let summary = sim.summary();
        let figures = MatchFigures::new(&summary, sim.managers());
        assert_eq!(last.tick, played);
        assert_eq!(last.possession_pct, figures.possession_pct);
        assert_eq!(last.shots, summary.shots);
        assert_eq!(last.shots_on_target, figures.shots_on_target);
        assert_eq!(last.xg, figures.xg);
        assert_eq!(last.passes, figures.passes);
        assert_eq!(last.pass_accuracy_pct, figures.pass_accuracy_pct);
        assert_eq!(last.fouls, summary.fouls);
        assert_eq!(last.corners, summary.corners);
        assert_eq!(last.offsides, summary.offsides);
        assert_eq!([last.home_score, last.away_score], summary.goals);
        for c in &conditions {
            assert!(c.tick.is_multiple_of(50), "{}", c.tick);
            assert_eq!(c.energy.len(), 22);
            assert!(c.energy.iter().all(|e| (0.0..=1.0).contains(e)), "{c:?}");
        }
        assert!(conditions.last().unwrap().energy.iter().any(|&e| e < 1.0));
    }

    #[test]
    fn every_play_event_carries_a_commentary_line() {
        let (messages, _) = driven(3);
        let events: Vec<&MatchEvent> = messages
            .iter()
            .filter_map(|m| match m {
                ServerMessage::Event(e) => Some(e),
                _ => None,
            })
            .collect();
        assert!(events.len() > 3, "{} events", events.len());
        for e in events {
            if e.event_type == EventType::TacticsChange {
                assert_eq!(e.commentary, None);
            } else {
                let line = e.commentary.as_deref().unwrap_or_default();
                assert!(!line.trim().is_empty(), "{e:?}");
            }
        }
    }
}
