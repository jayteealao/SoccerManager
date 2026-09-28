//! The one match driver every streaming command shares: it steps the simulation, feeds the
//! tick sink, announces every stoppage through the sink's stoppage hook, and routes every
//! engine event, the running statistics and energy every simulated second, and the closing
//! statistics to a caller-chosen destination (a socket, a fixture, or both).
//!
//! When a page can queue changes, the driver also carries them into the engine: after every
//! wait at the gate it moves each change the socket admitted into the engine's own queue for
//! the home team, and it names the change on its verdict event by the identifier the page
//! was given, because the two queues number their changes independently.

use engine::gate::PlannedChange;
use engine::observe::{MatchFigures, round_to};
use engine::record::TickSink;
use engine::{
    Card, Change, ChangeId, Commentary, Commentator, EngineEvent, EngineEventKind, EventDetail,
    Simulation,
};
use protocol::{
    CardKind, ChangeKind, ChangeOutcome, ChangeState, Condition, EventType, MatchEvent,
    RosterEntry, ServerMessage, SlotRole, SquadEntry, Stats, SubstitutionRules, TeamRef, TeamSetup,
};
use std::cell::RefCell;

use stream::session::MatchState;
use stream::{Admitted, Gate, Inbox, StreamError};

/// The team a page manages.
const HOME: usize = 0;

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
    /// The changes the page queued, when a page can queue them.
    pub inbox: Option<&'a Inbox>,
    /// Every change the page queued in this run, kept across connections so a verdict after
    /// a reconnect still carries the page's identifier. `None` when nothing reconnects.
    pub page_changes: Option<&'a RefCell<Vec<PageChange>>>,
    /// Changes queued at fixed ticks, before the step that starts on each one's tick: a
    /// recording's change file, or a replay file's manager changes.
    pub planned: &'a [Planned],
}

/// A change queued at a fixed tick.
#[derive(Debug, Clone)]
pub enum Planned {
    /// Named by lineup slot and bench place, and resolved to players when it is queued, as
    /// the replay gate's change fixture queues its changes.
    Slot(PlannedChange),
    /// Named by squad index, as a replay file's change log holds it.
    Exact {
        tick: u32,
        team: usize,
        change: Change,
    },
}

impl Planned {
    fn tick(&self) -> u32 {
        match self {
            Planned::Slot(p) => p.tick,
            Planned::Exact { tick, .. } => *tick,
        }
    }

    fn queue(&self, sim: &mut Simulation) {
        match self {
            Planned::Slot(p) => {
                let change = p.to_change(sim);
                sim.queue_change(p.team, change);
            }
            Planned::Exact { team, change, .. } => {
                sim.queue_change(*team, change.clone());
            }
        }
    }
}

/// A change the page queued, as a run carries it across a lost connection.
#[derive(Debug, Clone)]
pub struct PageChange {
    /// The engine's identifier; `None` while the change still waits in the inbox.
    pub id: Option<ChangeId>,
    pub admitted: Admitted,
}

/// Carries the page's changes onto a match rewound to `resume_at`. A change the engine queued
/// before that tick is in the resumed match and keeps its identifier. A change queued later,
/// or still in the inbox, is queued again when the events file kept its `queued` row (the
/// page was told it is queued); any other goes with its row.
pub fn carry_page_changes(sim: &mut Simulation, changes: &mut Vec<PageChange>, resume_at: u32) {
    changes.retain(|c| c.id.is_some_and(|id| id.tick < resume_at) || c.admitted.tick <= resume_at);
    for change in changes.iter_mut() {
        if change.id.is_none_or(|id| id.tick >= resume_at) {
            change.id = Some(sim.queue_change(HOME, change.admitted.change.clone()));
        }
    }
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
    if let Some(carried) = opts.page_changes {
        ids.page.extend(
            carried
                .borrow()
                .iter()
                .filter_map(|c| c.id.map(|id| (id, c.admitted.queue_id.clone()))),
        );
    }
    let mut commentator = Commentator::for_match(opts.commentary, sim);
    // One simulated second of ticks: the cadence of the running statistics and energy.
    let ticks_per_second = ((1.0 / sim.tuning().dt).round() as u32).max(1);
    let mut written = 0u32;
    // A sudden death can run past the announced maximum; it is played to its end.
    while !sim.is_over() && (written < opts.ticks || sim.in_shootout()) {
        // A stopped gate means the viewer left: the match did not end, so no full-time
        // event or closing statistics are written for it.
        if let Some(gate) = opts.gate
            && !gate.wait_for_room(sim.tick())
        {
            tracing::info!(
                signal = "socket.client_gone",
                written,
                reason = "the viewer left while the match waited"
            );
            return Ok(Driven {
                written,
                full_time: false,
            });
        }
        let now = sim.tick();
        for planned in opts.planned.iter().filter(|p| p.tick() == now) {
            planned.queue(sim);
        }
        // A change queued while the match was paused is queued here, on the first running
        // tick, so it waits for the next stoppage and never applies on the resume tick.
        if let Some(inbox) = opts.inbox {
            for admitted in inbox.drain() {
                let id = sim.queue_change(HOME, admitted.change.clone());
                ids.page.push((id, admitted.queue_id.clone()));
                if let Some(carried) = opts.page_changes {
                    carried.borrow_mut().push(PageChange {
                        id: Some(id),
                        admitted,
                    });
                }
            }
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
            for row in rows(
                sim,
                &mut commentator,
                &event,
                opts.owner_id,
                opts.match_id,
                opts.club_ids,
                &mut ids,
            ) {
                route(ServerMessage::Event(Box::new(row)))?;
            }
        }
        if record.tick.is_multiple_of(ticks_per_second) {
            route(ServerMessage::Stats(stats_message(sim)))?;
            route(ServerMessage::Condition(condition_message(sim)))?;
        }
    }
    let full_time = sim.is_over();
    sim.finish();
    for event in sim.take_events() {
        for message in rows(
            sim,
            &mut commentator,
            &event,
            opts.owner_id,
            opts.match_id,
            opts.club_ids,
            &mut ids,
        ) {
            if let Err(err) = route(ServerMessage::Event(Box::new(message))) {
                return closing_or_fail(err, written).map(|written| Driven { written, full_time });
            }
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
/// because the computer manager's pre-match setup there settles the lineup. When the page
/// picks the home lineup (`page_lineup`), the home team also carries its squad, with each
/// player's fit to every role, and the computer manager's setup to start the editor from.
pub(crate) fn hello_teams(sim: &Simulation, page_lineup: bool) -> [TeamRef; 2] {
    let config = sim.config();
    let fitness = config.attributes.index("natural_fitness");
    let mut index = 0usize;
    sim.teams().map(|team| {
        let editable = page_lineup && index == HOME;
        index += 1;
        let squad = if editable {
            team.squad
                .iter()
                .enumerate()
                .map(|(s, player)| SquadEntry {
                    id: team.player_ids[s].clone(),
                    name: team.player_names[s].clone(),
                    shirt: player.shirt,
                    position: player.position.code().to_string(),
                    natural_fitness: fitness.map_or(0, |i| player.attributes.get(i)),
                    role_fit: (0..config.tactics.roles.len())
                        .map(|role| {
                            let fit = engine::ai::role_fit(
                                player,
                                role,
                                &config.tactics,
                                &config.attributes,
                            );
                            fit.round().clamp(0.0, 100.0) as u8
                        })
                        .collect(),
                })
                .collect()
        } else {
            Vec::new()
        };
        let setup = editable.then(|| TeamSetup {
            lineup: team.lineup.iter().map(|&s| wire_index(s)).collect(),
            bench: team.bench.iter().map(|&s| wire_index(s)).collect(),
            formation: team.tactics.formation,
            mentality: team.tactics.mentality,
            instructions: team.tactics.instructions.to_vec(),
            roles: team
                .tactics
                .roles
                .iter()
                .map(|rd| SlotRole {
                    role: rd.role,
                    duty: rd.duty,
                })
                .collect(),
        });
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
            squad,
            setup,
        }
    })
}

/// A squad index as the wire writes it. A validated team file holds far fewer players.
fn wire_index(squad: usize) -> u16 {
    u16::try_from(squad).unwrap_or(u16::MAX)
}

/// The loaded tactics file as the hello carries it, so the page renders its tactics panel
/// from the same file the engine reads. A JSON object's keys carry no order on the wire, so
/// `instruction_order` names the instructions in the order a level list indexes them.
pub(crate) fn hello_tactics(sim: &Simulation) -> serde_json::Value {
    let mut tactics =
        serde_json::to_value(&sim.config().tactics).unwrap_or(serde_json::Value::Null);
    if let Some(object) = tactics.as_object_mut() {
        object.insert(
            "instruction_order".into(),
            serde_json::json!(engine::data::tactics::INSTRUCTIONS),
        );
    }
    tactics
}

/// The rule pack's substitution limits.
pub(crate) fn hello_substitutions(sim: &Simulation) -> SubstitutionRules {
    let rules = &sim.config().rules.substitutions;
    SubstitutionRules {
        limit: rules.limit,
        windows: rules.windows,
    }
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
    /// Each change the page queued: the engine's identifier and the one the page was given.
    page: Vec<(ChangeId, String)>,
    /// `true` once the first kick-off named the script pack.
    pack_named: bool,
}

impl Ids {
    /// The identifiers at the start of `sim`.
    pub(crate) fn new(sim: &Simulation) -> Self {
        Self {
            roster: sim.player_ids(),
            squads: sim.teams().map(|t| t.player_ids),
            page: Vec::new(),
            pack_named: false,
        }
    }

    /// The identifier a change's verdict carries: the page's, for a change the page queued.
    fn queue_id(&self, id: ChangeId) -> String {
        self.page
            .iter()
            .find(|(engine, _)| *engine == id)
            .map_or_else(|| id.to_string(), |(_, page)| page.clone())
    }
}

/// The event type the wire names an engine event kind by. Both change verdicts travel as
/// `tactics-change`.
pub(crate) fn event_type(kind: EngineEventKind) -> EventType {
    match kind {
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
        EngineEventKind::Script => EventType::Script,
        EngineEventKind::ChangeApplied | EngineEventKind::ChangeRejected => {
            EventType::TacticsChange
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
            queue_id: Some(ids.queue_id(id)),
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
    let event_type = event_type(event.kind);
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
    .period(event.period)
    .shootout(
        event.shootout_round,
        event.shootout_scored,
        event.shootout_scores,
    )
    .decided_by(event.decided_by.map(|d| d.code().to_string()))
}

/// One engine event as the rows it becomes: the event with its commentary line, which the
/// script pack's commentary hook may rewrite, then a `script` row for each hook failure that
/// rewrite caused. The first kick-off names the script pack, and a `script` row names the
/// hook, the outcome, and why.
pub(crate) fn rows(
    sim: &mut Simulation,
    commentator: &mut Commentator,
    event: &EngineEvent,
    owner_id: &str,
    match_id: &str,
    club_ids: [&str; 2],
    ids: &mut Ids,
) -> Vec<MatchEvent> {
    let (line, failures) = sim.offer_line(event, commentator.line(event));
    let mut out = Vec::with_capacity(1 + failures.len());
    for (e, line) in std::iter::once((event, line)).chain(failures.iter().map(|f| (f, None))) {
        let mut row = match_event(e, owner_id, match_id, club_ids, ids).commentary(line);
        if e.kind == EngineEventKind::KickOff && !ids.pack_named {
            ids.pack_named = true;
            row = row.script_pack(sim.plugins().pack.clone());
        }
        if let Some(EventDetail::Script(note)) = e.detail {
            let detail = sim.plugins().detail(&note);
            row = row.script(note.hook.code(), note.outcome.code(), detail);
        }
        out.push(row);
    }
    out
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
        crate::content::load(Some(&dir), None, None, None).unwrap()
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
                inbox: None,
                page_changes: None,
                planned: &[],
            },
            &mut |m: ServerMessage| {
                messages.push(m);
                Ok(())
            },
        )
        .unwrap();
        (messages, sim)
    }

    /// A level knockout match streams its shoot-out to full time even past the tick cap, and
    /// the kicks and the decision reach the wire as event fields.
    #[test]
    fn a_shootout_streams_to_full_time_past_the_tick_cap() {
        let loaded = loaded();
        let [a, b] = &loaded.teams;
        let mut config = MatchConfig::new(42, 1, &loaded.content, [a, b])
            .unwrap()
            .with_knockout();
        // Nobody decides, so the minute ends level.
        config.tuning.decision_interval_ticks = u32::MAX;
        let mut sim = Simulation::new(config).unwrap();
        let state = MatchState::default();
        let mut messages = Vec::new();
        let driven = drive(
            &mut sim,
            &mut VecSink::default(),
            &Drive {
                // Regulation only: the shoot-out runs past the cap.
                ticks: 3_000,
                owner_id: "0123456789abcdef0123456789abcdef",
                match_id: "000000000000002a-1",
                club_ids: ["club-a", "club-b"],
                state: &state,
                gate: None,
                commentary: &loaded.commentary,
                inbox: None,
                page_changes: None,
                planned: &[],
            },
            &mut |m: ServerMessage| {
                messages.push(m);
                Ok(())
            },
        )
        .unwrap();
        assert!(driven.full_time);
        assert!(driven.written > 3_000);
        let events: Vec<MatchEvent> = messages
            .into_iter()
            .filter_map(|m| match m {
                ServerMessage::Event(e) => Some(*e),
                _ => None,
            })
            .collect();
        let outcomes: Vec<&MatchEvent> = events
            .iter()
            .filter(|e| e.shootout_scored.is_some())
            .collect();
        assert!(!outcomes.is_empty());
        for e in &outcomes {
            assert_eq!(e.event_type, EventType::Penalty);
            assert!(e.commentary.is_none(), "a shoot-out kick gets no line");
            assert!(e.shootout_round.is_some() && e.shootout_scores.is_some());
        }
        let full_time = events.last().unwrap();
        assert_eq!(full_time.event_type, EventType::FullTime);
        assert_eq!(full_time.decided_by.as_deref(), Some("shoot-out"));
        let json = serde_json::to_string(full_time).unwrap();
        assert!(
            json.contains("\"result.decided_by\":\"shoot-out\""),
            "{json}"
        );
        assert!(json.contains("\"shootout.scores\":["), "{json}");
    }

    #[test]
    fn the_hello_roster_lists_the_starters_in_slot_order_then_the_bench() {
        let loaded = loaded();
        let [a, b] = &loaded.teams;
        let config = MatchConfig::new(42, 2, &loaded.content, [a, b]).unwrap();
        let sim = Simulation::new(config).unwrap();
        let teams = hello_teams(&sim, false);
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
                ServerMessage::Event(e) => Some(&**e),
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

    #[test]
    fn a_page_lineup_hello_carries_the_home_squad_setup_and_instruction_order() {
        let loaded = loaded();
        let [a, b] = &loaded.teams;
        let config = MatchConfig::new(42, 2, &loaded.content, [a, b]).unwrap();
        let sim = Simulation::new(config).unwrap();
        let teams = hello_teams(&sim, true);
        let home = &sim.teams()[HOME];
        assert_eq!(teams[HOME].squad.len(), home.squad.len());
        assert!(teams[1].squad.is_empty() && teams[1].setup.is_none());
        let roles = sim.config().tactics.roles.len();
        for entry in &teams[HOME].squad {
            assert_eq!(entry.role_fit.len(), roles);
            assert!(entry.role_fit.iter().all(|&f| f <= 100), "{entry:?}");
            assert!(entry.natural_fitness > 0, "{entry:?}");
        }
        let setup = teams[HOME]
            .setup
            .as_ref()
            .expect("the editor starts from the setup");
        let lineup: Vec<usize> = setup.lineup.iter().map(|&s| usize::from(s)).collect();
        assert_eq!(lineup, home.lineup.to_vec());
        assert_eq!(setup.roles.len(), 11);
        assert_eq!(setup.instructions, home.tactics.instructions.to_vec());
        assert!(hello_teams(&sim, false)[HOME].setup.is_none());
        let tactics = hello_tactics(&sim);
        assert_eq!(
            tactics["instruction_order"],
            serde_json::json!(engine::data::tactics::INSTRUCTIONS)
        );
        assert_eq!(
            hello_substitutions(&sim).limit,
            sim.config().rules.substitutions.limit
        );
    }

    /// Keeps the tick and kind of every stoppage the match opened.
    #[derive(Default)]
    struct Stoppages(Vec<(u32, engine::StoppageKind)>);

    impl TickSink for Stoppages {
        fn on_tick(&mut self, _: &engine::record::TickRecord) -> Result<(), engine::EngineError> {
            Ok(())
        }

        fn on_stoppage(
            &mut self,
            stoppage: &engine::Stoppage,
            sim: &Simulation,
        ) -> Result<(), engine::EngineError> {
            self.0.push((sim.tick(), stoppage.kind));
            Ok(())
        }
    }

    /// The page's two changes: an attacking mentality and the first substitute for the
    /// home striker in slot 10.
    fn page_changes(sim: &Simulation) -> Vec<stream::Admitted> {
        let home = &sim.teams()[HOME];
        vec![
            stream::Admitted {
                queue_id: "q-7-0".into(),
                change: engine::Change::Tactics(engine::TacticsPatch::mentality(4)),
                tick: 7,
            },
            stream::Admitted {
                queue_id: "q-7-1".into(),
                change: engine::Change::Substitution {
                    off: home.lineup[10],
                    on: home.bench[0],
                },
                tick: 7,
            },
        ]
    }

    fn verdicts(messages: &[ServerMessage]) -> Vec<&MatchEvent> {
        messages
            .iter()
            .filter_map(|m| match m {
                ServerMessage::Event(e) if e.event_type == EventType::TacticsChange => Some(&**e),
                _ => None,
            })
            .filter(|e| e.team_id.as_deref() == Some("club-a"))
            .collect()
    }

    #[test]
    fn every_engine_event_kind_spells_the_wire_type_it_travels_as() {
        for kind in EngineEventKind::ALL {
            assert_eq!(kind.code(), event_type(kind).code(), "{kind:?}");
        }
    }

    #[test]
    fn the_wire_minute_counts_the_ticks_the_engine_clock_counts() {
        assert_eq!(
            protocol::event::TICKS_PER_MINUTE,
            engine::rules::clock::TICKS_PER_MINUTE
        );
    }

    #[test]
    fn a_viewer_that_leaves_while_the_match_waits_gets_no_full_time() {
        let loaded = loaded();
        let [a, b] = &loaded.teams;
        let config = MatchConfig::new(42, 2, &loaded.content, [a, b]).unwrap();
        let ticks = config.max_ticks();
        let mut sim = Simulation::new(config).unwrap();
        let gate = Gate::new();
        gate.stop();
        let state = MatchState::default();
        let mut messages = Vec::new();
        let driven = drive(
            &mut sim,
            &mut VecSink::default(),
            &Drive {
                ticks,
                owner_id: "0123456789abcdef0123456789abcdef",
                match_id: "000000000000002a-1",
                club_ids: ["club-a", "club-b"],
                state: &state,
                gate: Some(&gate),
                commentary: &loaded.commentary,
                inbox: None,
                page_changes: None,
                planned: &[],
            },
            &mut |m: ServerMessage| {
                messages.push(m);
                Ok(())
            },
        )
        .unwrap();
        assert!(!driven.full_time);
        assert!(
            !messages.iter().any(|m| matches!(
                m,
                ServerMessage::Event(e) if e.event_type == EventType::FullTime
            )),
            "a match the viewer left must not record a full time"
        );
    }

    #[test]
    fn a_rewind_keeps_page_changes_the_page_was_told_about_and_drops_the_rest() {
        let loaded = loaded();
        let [a, b] = &loaded.teams;
        let config = MatchConfig::new(42, 3, &loaded.content, [a, b])
            .unwrap()
            .with_manager(HOME, engine::Manager::Human);
        let mut sim = Simulation::new(config).unwrap();
        let admitted = page_changes(&sim);
        let change = |queue_id: &str, tick: u32, id: Option<ChangeId>| PageChange {
            id,
            admitted: Admitted {
                queue_id: queue_id.into(),
                tick,
                ..admitted[0].clone()
            },
        };
        let resume_at = 100;
        let mut changes = vec![
            // Queued into the engine before the snapshot: the resumed match holds it.
            change("q-50-0", 50, Some(ChangeId { tick: 60, n: 0 })),
            // Admitted at the snapshot tick, queued into the engine after it: queued again.
            change("q-100-1", 100, Some(ChangeId { tick: 100, n: 1 })),
            // Still in the inbox when the connection dropped: queued now.
            change("q-90-2", 90, None),
            // Admitted after the snapshot: its row is gone, and so is it.
            change("q-120-3", 120, Some(ChangeId { tick: 120, n: 2 })),
        ];
        let before = sim.pending_changes().len();
        carry_page_changes(&mut sim, &mut changes, resume_at);
        let kept: Vec<&str> = changes
            .iter()
            .map(|c| c.admitted.queue_id.as_str())
            .collect();
        assert_eq!(kept, ["q-50-0", "q-100-1", "q-90-2"]);
        assert_eq!(changes[0].id, Some(ChangeId { tick: 60, n: 0 }));
        assert!(changes.iter().all(|c| c.id.is_some()));
        assert_eq!(sim.pending_changes().len(), before + 2);
    }

    #[test]
    fn a_page_change_reaches_the_engine_and_its_verdict_carries_the_page_identifier() {
        let loaded = loaded();
        let [a, b] = &loaded.teams;
        let config = MatchConfig::new(42, 3, &loaded.content, [a, b])
            .unwrap()
            .with_manager(HOME, engine::Manager::Human);
        let ticks = config.max_ticks();
        let mut sim = Simulation::new(config).unwrap();
        let inbox = Inbox::default();
        for change in page_changes(&sim) {
            inbox.push(change);
        }
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
                inbox: Some(&inbox),
                page_changes: None,
                planned: &[],
            },
            &mut |m: ServerMessage| {
                messages.push(m);
                Ok(())
            },
        )
        .unwrap();
        let verdicts = verdicts(&messages);
        let mut ids: Vec<&str> = verdicts
            .iter()
            .filter_map(|e| e.change_queue_id.as_deref())
            .collect();
        ids.sort_unstable();
        assert_eq!(ids, ["q-7-0", "q-7-1"], "{verdicts:?}");
        for e in &verdicts {
            assert_eq!(e.change_state, Some(ChangeState::Applied), "{e:?}");
            assert_eq!(e.change_applied_tick, Some(e.tick));
        }
        let home = &sim.teams()[HOME];
        assert_eq!(home.tactics.mentality, 4);
        let starters = MatchConfig::new(42, 3, &loaded.content, [a, b])
            .unwrap()
            .teams[HOME]
            .lineup;
        assert_ne!(home.lineup[10], starters[10], "the substitute took slot 10");
    }

    /// Waits until the producer stops moving and returns the tick it stopped on.
    fn settled(state: &MatchState) -> u32 {
        let mut at = state.tick();
        loop {
            std::thread::sleep(std::time::Duration::from_millis(20));
            let now = state.tick();
            if now == at {
                return at;
            }
            at = now;
        }
    }

    #[test]
    fn a_reported_drawn_tick_holds_the_engine_within_the_lead_bound() {
        let loaded = loaded();
        let [a, b] = &loaded.teams;
        let config = MatchConfig::new(42, 3, &loaded.content, [a, b])
            .unwrap()
            .with_manager(HOME, engine::Manager::Human);
        let ticks = config.max_ticks();
        let mut sim = Simulation::new(config).unwrap();
        let gate = Gate::new();
        gate.set_lead_bound(500);
        gate.set_seen(1_000);
        let state = MatchState::default();
        std::thread::scope(|scope| {
            let run = scope.spawn(|| {
                drive(
                    &mut sim,
                    &mut VecSink::default(),
                    &Drive {
                        ticks,
                        owner_id: "0123456789abcdef0123456789abcdef",
                        match_id: "000000000000002a-1",
                        club_ids: ["club-a", "club-b"],
                        state: &state,
                        gate: Some(&gate),
                        commentary: &loaded.commentary,
                        inbox: None,
                        page_changes: None,
                        planned: &[],
                    },
                    &mut |_: ServerMessage| Ok(()),
                )
            });
            while state.tick() < 1_400 {
                std::thread::yield_now();
            }
            assert_eq!(
                settled(&state),
                1_500,
                "the engine stops 500 ticks past the drawn tick"
            );
            gate.set_seen(2_000);
            while state.tick() < 2_400 {
                std::thread::yield_now();
            }
            assert_eq!(
                settled(&state),
                2_500,
                "a newer drawn tick releases it by as much"
            );
            gate.stop();
            run.join().unwrap().unwrap();
        });
    }

    #[test]
    fn a_change_made_while_paused_applies_at_the_next_stoppage_not_on_resume() {
        let loaded = loaded();
        let [a, b] = &loaded.teams;
        let config = MatchConfig::new(42, 3, &loaded.content, [a, b])
            .unwrap()
            .with_manager(HOME, engine::Manager::Human);
        let ticks = config.max_ticks();
        let mut sim = Simulation::new(config).unwrap();
        let changes = page_changes(&sim);
        let gate = Gate::new();
        let inbox = Inbox::default();
        let state = MatchState::default();
        let mut messages = Vec::new();
        let mut stoppages = Stoppages::default();
        let paused_at = std::thread::scope(|scope| {
            let run = scope.spawn(|| {
                drive(
                    &mut sim,
                    &mut stoppages,
                    &Drive {
                        ticks,
                        owner_id: "0123456789abcdef0123456789abcdef",
                        match_id: "000000000000002a-1",
                        club_ids: ["club-a", "club-b"],
                        state: &state,
                        gate: Some(&gate),
                        commentary: &loaded.commentary,
                        inbox: Some(&inbox),
                        page_changes: None,
                        planned: &[],
                    },
                    &mut |m: ServerMessage| {
                        messages.push(m);
                        Ok(())
                    },
                )
            });
            while state.tick() < 2_000 {
                std::thread::yield_now();
            }
            gate.set_running(false);
            // The producer finishes the tick it is on, then waits at the gate.
            let mut paused_at = state.tick();
            loop {
                std::thread::sleep(std::time::Duration::from_millis(20));
                let now = state.tick();
                if now == paused_at {
                    break;
                }
                paused_at = now;
            }
            for change in changes {
                inbox.push(change);
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
            assert_eq!(state.tick(), paused_at, "a paused match does not move");
            gate.set_running(true);
            run.join().unwrap().unwrap();
            paused_at
        });
        let verdicts = verdicts(&messages);
        assert_eq!(verdicts.len(), 2, "{verdicts:?}");
        // The first stoppage after the pause that admits a change.
        let next = stoppages
            .0
            .iter()
            .find(|(tick, kind)| *tick > paused_at && *kind != engine::StoppageKind::Penalty)
            .map(|(tick, _)| *tick)
            .expect("a three-minute match has a stoppage after the pause");
        for e in verdicts {
            assert_eq!(e.change_queued_tick, Some(paused_at), "{e:?}");
            assert_eq!(e.change_state, Some(ChangeState::Applied), "{e:?}");
            assert!(e.tick > paused_at, "applied on resume: {e:?}");
            assert_eq!(e.tick, next, "not at the next stoppage: {e:?}");
        }
    }
}
