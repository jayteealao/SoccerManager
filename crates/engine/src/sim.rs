//! The fixed-timestep simulation loop (named mechanism: tick count is a function of match
//! time alone). Order per tick: decisions, steering, ball, possession, referee and clock,
//! fatigue, the AI manager's check, the change queue when this tick opened a stoppage, and
//! overlap resolution. The referee (`rules`) owns the phase of play: live, a dead ball
//! waiting for its restart, or full time.

use sha2::{Digest, Sha256};

use crate::ai::{self, AiCode, AiState, Manager};
use crate::ball::Ball;
use crate::data::attributes::AttributeSchema;
use crate::data::rules::{RulePack, StoppageKind};
use crate::data::tactics::TacticsSchema;
use crate::data::team::TeamFile;
use crate::data::tuning::FatigueTuning;
use crate::data::{Content, hex12};
use crate::decision::Kick;
use crate::error::EngineError;
use crate::fatigue::InjurySource;
use crate::flags::ActiveFlags;
use crate::math::{DVec2, DVec3};
use crate::pitch;
use crate::player::Player;
use crate::plugin::{Plugins, ScriptNote};
use crate::record::{TickRecord, TickSink};
use crate::rng::EngineRng;
use crate::rules::fouls::{self, Card, Tackle};
use crate::rules::offside;
use crate::rules::{Phase, Referee, Stoppage};
use crate::steering;
use crate::tactics::Tactics;
use crate::tactics::change::{ChangeId, ChangeKind, ChangeQueue, RejectReason, SubLedger};
use crate::team::{PLAYERS_PER_TEAM, Team};
use crate::tuning::{Tuning, XgTuning};

/// Everything a match needs to start: the seed, the length, the tuning, the rule pack, the
/// tactics file, and the attribute schema from the content, the two teams with their
/// starters, who manages each team, and hashes of the files they came from.
#[derive(Debug, Clone)]
pub struct MatchConfig {
    pub seed: u64,
    pub minutes: u32,
    pub tuning: Tuning,
    pub rules: RulePack,
    pub tactics: TacticsSchema,
    pub attributes: AttributeSchema,
    pub fatigue: FatigueTuning,
    /// Who manages each team, home first. Both are AI-managed unless a caller says
    /// otherwise.
    pub managers: [Manager; 2],
    pub teams: [Team; 2],
    pub players: Vec<Player>,
    /// Twelve hex characters over the content files and the two team files.
    pub content_hash: String,
    /// SHA-256 of each team file as the engine read it, home first.
    pub team_digests: [[u8; 32]; 2],
    /// A knockout match that is level at the end of regulation time plays the rule pack's
    /// extra time and, still level, a penalty shoot-out. Off unless a caller says otherwise.
    pub knockout: bool,
    /// The flags that are on, resolved when the content loaded. Never changes in a match.
    pub flags: ActiveFlags,
}

impl MatchConfig {
    /// Builds a match from loaded content and two validated team files. `minutes` must be
    /// 1 to 200; the home team is `files[0]`. The AI manager's pre-match setup picks each
    /// team's lineup, bench, and tactics.
    pub fn new(
        seed: u64,
        minutes: u32,
        content: &Content,
        files: [&TeamFile; 2],
    ) -> Result<Self, EngineError> {
        if !(1..=200).contains(&minutes) {
            return Err(EngineError::InvalidConfig(format!(
                "minutes must be 1 to 200, got {minutes}"
            )));
        }
        let tuning = content.tuning.engine.clone();
        let (mut home, _) = Team::from_file(0, files[0], &content.attributes, &tuning)?;
        let (mut away, _) = Team::from_file(1, files[1], &content.attributes, &tuning)?;
        for team in [&mut home, &mut away] {
            let setup = ai::pre_match(team, &content.tactics, &content.attributes);
            team.lineup = setup.lineup;
            team.bench = setup.bench;
            team.set_tactics(setup.tactics, &content.tactics, &tuning);
        }
        let mut players = home.starters();
        players.extend(away.starters());
        let mut hasher = Sha256::new();
        hasher.update(content.digest);
        let mut team_digests = [[0u8; 32]; 2];
        for (digest, file) in team_digests.iter_mut().zip(files) {
            let json = serde_json::to_vec(file).map_err(|e| EngineError::Format(e.to_string()))?;
            *digest = Sha256::digest(&json).into();
            hasher.update(json);
        }
        Ok(Self {
            seed,
            minutes,
            tuning,
            rules: content.rules.clone(),
            tactics: content.tactics.clone(),
            attributes: content.attributes.clone(),
            fatigue: content.tuning.fatigue.clone(),
            managers: [Manager::Ai; 2],
            teams: [home, away],
            players,
            content_hash: hex12(&hasher.finalize()),
            team_digests,
            knockout: false,
            flags: content.flags.clone(),
        })
    }

    /// Folds a script pack's SHA-256 into the content hash, so a snapshot written with one
    /// pack resumes only with the same pack.
    pub fn fold_pack_hash(&mut self, pack_sha: &[u8; 32]) {
        let mut hasher = Sha256::new();
        hasher.update(self.content_hash.as_bytes());
        hasher.update(pack_sha);
        self.content_hash = hex12(&hasher.finalize());
    }

    /// The two club identifiers, home first.
    pub fn club_ids(&self) -> [&str; 2] {
        [&self.teams[0].club_id, &self.teams[1].club_id]
    }

    /// The most ticks this match can last: regulation plus the cap on added time. A knockout
    /// match adds extra time at its caps and the rule pack's shoot-out allowance; a sudden
    /// death longer than the allowance plays on past it.
    pub fn max_ticks(&self) -> u32 {
        let regulation = crate::rules::clock::max_ticks(self.minutes, &self.rules);
        if !self.knockout {
            return regulation;
        }
        let delay = crate::rules::restart::delay_ticks(StoppageKind::Penalty, &self.tuning);
        regulation
            + crate::rules::clock::knockout_extra_ticks(self.minutes, &self.rules)
            + crate::rules::clock::shootout_allowance_ticks(&self.rules, delay)
    }

    /// The match is a knockout match: level at the end of regulation time, it plays extra
    /// time and then a penalty shoot-out.
    pub fn with_knockout(mut self) -> Self {
        self.knockout = true;
        self
    }

    /// `team` is managed by `manager`.
    pub fn with_manager(mut self, team: usize, manager: Manager) -> Self {
        self.managers[team] = manager;
        self
    }

    /// `team` starts with `tactics` instead of the AI manager's; its players take the new
    /// formation's places.
    pub fn with_tactics(mut self, team: usize, tactics: Tactics) -> Self {
        let schema = self.tactics.clone();
        self.teams[team].set_tactics(tactics, &schema, &self.tuning);
        let starters = self.teams[team].starters();
        let first = team * PLAYERS_PER_TEAM;
        self.players[first..first + PLAYERS_PER_TEAM].clone_from_slice(&starters);
        self
    }

    /// `team` starts with `lineup` (squad indices in slot order) and `bench` instead of the
    /// AI manager's, keeping its pre-match tactics; the starters are rebuilt in slot order.
    /// The content hash is unchanged, because a lineup is not a content file.
    pub fn with_setup(
        mut self,
        team: usize,
        lineup: [usize; PLAYERS_PER_TEAM],
        bench: Vec<usize>,
    ) -> Self {
        self.teams[team].lineup = lineup;
        self.teams[team].bench = bench;
        let starters = self.teams[team].starters();
        let first = team * PLAYERS_PER_TEAM;
        self.players[first..first + PLAYERS_PER_TEAM].clone_from_slice(&starters);
        self
    }
}

/// How a knockout match was decided.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecidedBy {
    Regulation,
    ExtraTime,
    Shootout,
}

impl DecidedBy {
    /// Every value, in declaration order.
    pub const ALL: [DecidedBy; 3] = [
        DecidedBy::Regulation,
        DecidedBy::ExtraTime,
        DecidedBy::Shootout,
    ];

    /// The value as the event contract spells it.
    pub fn code(&self) -> &'static str {
        match self {
            DecidedBy::Regulation => "regulation",
            DecidedBy::ExtraTime => "extra-time",
            DecidedBy::Shootout => "shoot-out",
        }
    }
}

/// What an engine event carries beyond the common fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventDetail {
    /// The verdict on a queued change: applied on the event's tick, or rejected with the
    /// reason.
    Change {
        id: ChangeId,
        kind: ChangeKind,
        reason: Option<RejectReason>,
    },
    /// Squad player `on` replaced squad player `off` in the event's roster slot.
    Substitution {
        off: usize,
        on: usize,
    },
    Injury {
        source: InjurySource,
    },
    Ai {
        code: AiCode,
    },
    /// A plugin hook failed or was switched off.
    Script(ScriptNote),
}

/// What happened during a tick that a consumer outside the engine must know about. The
/// engine names the fact; the protocol crate decides how it travels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineEventKind {
    /// Play restarts from the centre spot: at the start of a half or after a goal.
    KickOff,
    /// `team` scored; `scores` is the score after the goal.
    Goal,
    /// The end of the first half.
    HalfTime,
    /// The last tick of the match.
    FullTime,
    /// `player` of `team` was offside.
    Offside,
    /// `player` of `team` fouled `secondary`.
    Foul,
    /// `player` of `team` was shown `card`.
    Card,
    ThrowIn,
    Corner,
    GoalKick,
    FreeKick,
    Penalty,
    /// `player` of `team` was injured and left play. When the injury stopped play, the
    /// event names the dropped-ball spot.
    Injury,
    /// A substitute replaced `player` of `team`; the detail names both squad players.
    Substitution,
    /// The AI manager of `team` queued a change; the detail names the choice.
    AiDecision,
    /// A queued change of `team` applied on this tick.
    ChangeApplied,
    /// A queued change of `team` was rejected on this tick; the detail names the reason.
    ChangeRejected,
    /// A plugin hook failed or was switched off; the detail names the hook and the outcome.
    /// Play goes on with the engine's own choice.
    Script,
}

impl EngineEventKind {
    /// Every event kind, in declaration order.
    pub const ALL: [EngineEventKind; 18] = [
        EngineEventKind::KickOff,
        EngineEventKind::Goal,
        EngineEventKind::HalfTime,
        EngineEventKind::FullTime,
        EngineEventKind::Offside,
        EngineEventKind::Foul,
        EngineEventKind::Card,
        EngineEventKind::ThrowIn,
        EngineEventKind::Corner,
        EngineEventKind::GoalKick,
        EngineEventKind::FreeKick,
        EngineEventKind::Penalty,
        EngineEventKind::Injury,
        EngineEventKind::Substitution,
        EngineEventKind::AiDecision,
        EngineEventKind::ChangeApplied,
        EngineEventKind::ChangeRejected,
        EngineEventKind::Script,
    ];

    /// The kind as the event contract spells it. Both change verdicts travel as
    /// `tactics-change`.
    pub fn code(&self) -> &'static str {
        match self {
            EngineEventKind::KickOff => "kick-off",
            EngineEventKind::Goal => "goal",
            EngineEventKind::HalfTime => "half-time",
            EngineEventKind::FullTime => "full-time",
            EngineEventKind::Offside => "offside",
            EngineEventKind::Foul => "foul",
            EngineEventKind::Card => "card",
            EngineEventKind::ThrowIn => "throw-in",
            EngineEventKind::Corner => "corner",
            EngineEventKind::GoalKick => "goal-kick",
            EngineEventKind::FreeKick => "free-kick",
            EngineEventKind::Penalty => "penalty",
            EngineEventKind::Injury => "injury",
            EngineEventKind::Substitution => "substitution",
            EngineEventKind::AiDecision => "ai-decision",
            EngineEventKind::ChangeApplied | EngineEventKind::ChangeRejected => "tactics-change",
            EngineEventKind::Script => "script",
        }
    }

    /// The event that announces a restart of `kind`.
    pub fn restart(kind: StoppageKind) -> Self {
        match kind {
            StoppageKind::ThrowIn => EngineEventKind::ThrowIn,
            StoppageKind::Corner => EngineEventKind::Corner,
            StoppageKind::GoalKick => EngineEventKind::GoalKick,
            StoppageKind::Penalty => EngineEventKind::Penalty,
            StoppageKind::FreeKick => EngineEventKind::FreeKick,
            StoppageKind::Injury => EngineEventKind::Injury,
            _ => EngineEventKind::KickOff,
        }
    }
}

/// One engine event, stamped with the tick it happened on and the minute the match clock
/// showed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EngineEvent {
    pub tick: u32,
    pub kind: EngineEventKind,
    /// The team the event belongs to, as a roster index, or `None` at half-time and full
    /// time. A foul, an offside, and a card belong to the offender's team; a restart belongs
    /// to the team that takes it.
    pub team: Option<usize>,
    /// The score after the event, home first.
    pub scores: [u32; 2],
    /// The minute of play, counted from 0, and the added minute in added time.
    pub minute: u32,
    pub minute_added: Option<u32>,
    /// The roster index of the player the event names: the offender, the booked player, the
    /// injured player, the player leaving on a substitution, the taker on a restart and a
    /// kick-off, and on a goal the player who kicked the ball last (a player of the other
    /// club on an own goal).
    pub player: Option<usize>,
    /// The fouled player.
    pub secondary: Option<usize>,
    pub card: Option<Card>,
    /// On a foul: `true` when play continued with advantage.
    pub advantage: Option<bool>,
    /// On half-time and full time: the seconds added to the half.
    pub added_time_s: Option<u32>,
    /// On a restart: where the ball was placed.
    pub spot: Option<DVec2>,
    pub detail: Option<EventDetail>,
    /// On the half-time break before an extra-time period and on its kick-off: the period
    /// that starts, counted from 0 (2 and 3 for the two periods of extra time).
    pub period: Option<u32>,
    /// On a shoot-out `penalty` event: the kicking team's round, from 1.
    pub shootout_round: Option<u32>,
    /// On the outcome event of a shoot-out kick: `true` when the kick scored.
    pub shootout_scored: Option<bool>,
    /// On the outcome event of a shoot-out kick and at full time after a shoot-out: the
    /// shoot-out score, home first.
    pub shootout_scores: Option<[u32; 2]>,
    /// At full time of a knockout match: how it was decided.
    pub decided_by: Option<DecidedBy>,
}

/// Aggregate counters kept during a match. Per-team arrays are home first; a foul, an
/// offside, and a card count against the offender's team, and a restart counts for the team
/// that takes it.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Summary {
    pub possession_changes: u32,
    pub ball_max_speed: f64,
    /// Ticks of open play with the ball at rest and nobody controlling it.
    pub ball_idle_ticks: u32,
    pub goals: [u32; 2],
    pub fouls: [u32; 2],
    pub offsides: [u32; 2],
    pub corners: [u32; 2],
    pub throw_ins: [u32; 2],
    pub goal_kicks: [u32; 2],
    pub free_kicks: [u32; 2],
    pub penalties: [u32; 2],
    /// Yellow cards, a second yellow included, and players sent off.
    pub yellow: [u32; 2],
    pub red: [u32; 2],
    /// Seconds added to each half.
    pub added_s: [u32; 2],
    /// Stoppages announced through the stoppage hook.
    pub stoppages: u32,
    pub dead_ball_ticks: u32,
    /// Times the offside positions were computed (once per kick in open play).
    pub offside_checks: u32,
    pub shots: [u32; 2],
    pub substitutions: [u32; 2],
    pub injuries: [u32; 2],
    /// Changes queued in the engine's queue, and their verdicts.
    pub changes_queued: u32,
    pub changes_applied: u32,
    pub changes_rejected: u32,
    pub ai_decisions: u32,
    /// Shots that scored or that the opposing goalkeeper held.
    pub shots_on_target: [u32; 2],
    /// The expected goals of every shot taken, summed per team.
    pub xg: [f64; 2],
    /// Passes played, and passes whose next controlling touch was a team-mate's.
    pub passes: [u32; 2],
    pub passes_completed: [u32; 2],
    /// Ticks of open play credited to the team that touched the ball last.
    pub possession_ticks: [u32; 2],
    /// Seconds added to each period of extra time.
    pub extra_added_s: [u32; 2],
    /// `true` once extra time started.
    pub extra_time: bool,
    /// The shoot-out score, home first, once a shoot-out started.
    pub shootout: Option<[u32; 2]>,
    /// Shoot-out kicks taken, both teams.
    pub shootout_kicks: u32,
    /// How a knockout match was decided, set at full time.
    pub decided_by: Option<DecidedBy>,
}

/// The expected goals of a shot from `from` at the goal a team attacking `attack_x` aims
/// at: a logistic function of the distance to the goal centre and of the angle the goal
/// mouth subtends, with the coefficients from the tuning file.
pub fn shot_xg(from: DVec2, attack_x: f64, t: &XgTuning) -> f64 {
    let x = pitch::HALF_LENGTH * attack_x;
    let half = pitch::GOAL_WIDTH / 2.0;
    let a = DVec2::new(x, half) - from;
    let b = DVec2::new(x, -half) - from;
    let angle = a.perp_dot(b).atan2(a.dot(b)).abs();
    let distance = (pitch::goal_centre(attack_x) - from).length();
    let z = t.intercept + t.distance_coef * distance + t.angle_coef * angle;
    1.0 / (1.0 + (-z).exp())
}

/// The decision hook's offsets for one carrier, in force until `until`.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ScriptCache {
    pub carrier: usize,
    pub until: u32,
    pub offsets: crate::plugin::OptionOffsets,
}

/// One running match.
pub struct Simulation {
    pub(crate) config: MatchConfig,
    pub(crate) teams: [Team; 2],
    pub(crate) players: Vec<Player>,
    pub(crate) ball: Ball,
    pub(crate) rng: EngineRng,
    pub(crate) tick: u32,
    pub(crate) carrier: Option<usize>,
    pub(crate) control_since: u32,
    pub(crate) last_touch: Option<usize>,
    /// The roster index of the player who kicked the ball last in this spell of play. Every
    /// dead ball clears it, and a snapshot is written only when play stops, so the snapshot
    /// never needs it.
    pub(crate) last_kicker: Option<usize>,
    pub(crate) summary: Summary,
    pub(crate) referee: Referee,
    pub(crate) restart: bool,
    pub(crate) events: Vec<EngineEvent>,
    pub(crate) stoppage: Option<Stoppage>,
    /// The team shapes in force from each tick on: the start, each half-time, and each
    /// sending-off. The validator reads it.
    pub(crate) timeline: Vec<(u32, [Team; 2])>,
    pub(crate) managers: [Manager; 2],
    pub(crate) queue: ChangeQueue,
    pub(crate) ledgers: [SubLedger; 2],
    pub(crate) ai: [AiState; 2],
    /// `true` once a goalkeeper failed to hold the ball in flight: the tuned catch chance is
    /// one roll per flight, not one per tick the ball spends within reach.
    pub(crate) keeper_beaten: bool,
    /// The team whose pass or shot is in flight and not yet resolved. Every dead ball clears
    /// both, like `last_kicker`, so the snapshot never needs them.
    pub(crate) pass_in_flight: Option<usize>,
    pub(crate) shot_in_flight: Option<usize>,
    /// Test seam: the outcomes the next shoot-out kicks are given, whatever the ball does.
    #[cfg(feature = "scenario")]
    pub(crate) forced_kicks: std::collections::VecDeque<bool>,
    /// The plugin hooks, none unless a caller attaches them.
    pub(crate) plugins: Plugins,
    /// The decision hook's offsets for the current carrier. A stoppage and a new carrier
    /// clear it, so it never needs to be in the snapshot.
    pub(crate) script_cache: Option<ScriptCache>,
    finished: bool,
    scratch: Vec<DVec2>,
}

impl Simulation {
    /// Places both teams for kick-off.
    pub fn new(config: MatchConfig) -> Result<Self, EngineError> {
        let mut sim = Self::blank(config)?;
        sim.place_kick_off(0);
        Ok(sim)
    }

    /// A match with its state as the configuration gives it and nobody placed.
    pub(crate) fn blank(config: MatchConfig) -> Result<Self, EngineError> {
        if config.players.len() != 2 * PLAYERS_PER_TEAM {
            return Err(EngineError::InvalidConfig(format!(
                "a match needs {} players, got {}",
                2 * PLAYERS_PER_TEAM,
                config.players.len()
            )));
        }
        let teams = config.teams.clone();
        let players = config.players.clone();
        let rng = EngineRng::from_seed(config.seed);
        let referee = Referee::new(config.minutes, &config.rules, config.knockout);
        Ok(Self {
            timeline: vec![(0, teams.clone())],
            teams,
            players,
            ball: Ball::at(DVec2::ZERO),
            rng,
            tick: 0,
            carrier: None,
            control_since: 0,
            last_touch: None,
            last_kicker: None,
            summary: Summary::default(),
            referee,
            restart: false,
            events: Vec::new(),
            stoppage: None,
            managers: config.managers,
            queue: ChangeQueue::default(),
            ledgers: [SubLedger::default(); 2],
            ai: [AiState::default(); 2],
            keeper_beaten: false,
            pass_in_flight: None,
            shot_in_flight: None,
            #[cfg(feature = "scenario")]
            forced_kicks: std::collections::VecDeque::new(),
            plugins: Plugins::default(),
            script_cache: None,
            finished: false,
            scratch: Vec::with_capacity(2 * PLAYERS_PER_TEAM),
            config,
        })
    }

    /// Sends player `i` off before kick-off, as a card shown would: the player loses the
    /// ball, the team is laid out again, and its manager reviews the shape. The team shapes
    /// start again from the current tick, so the validator reads the reduced side from the
    /// start. Calibration's controlled sending-off experiment uses it.
    pub fn send_off_before_kickoff(&mut self, i: usize) {
        if self.carrier == Some(i) {
            self.carrier = None;
        }
        crate::rules::discipline::send_off(&mut self.players, &mut self.teams, i);
        let team = self.players[i].team;
        self.ai[team].due = true;
        self.timeline = vec![(self.tick, self.teams.clone())];
    }

    pub fn tuning(&self) -> &Tuning {
        &self.config.tuning
    }

    /// The roster index of the player keeping goal for `team`: slot 0, a goalkeeper who came
    /// on, or an outfield player standing in (`Team::keeper_slot`).
    pub fn keeper(&self, team: usize) -> usize {
        team * PLAYERS_PER_TEAM + self.teams[team].keeper_slot()
    }

    /// Attaches plugin hooks. Call it before the first step, or right after a resume.
    pub fn set_plugins(&mut self, plugins: Plugins) {
        self.plugins = plugins;
        self.script_cache = None;
    }

    /// The attached plugin hooks, their pack, and their counters.
    pub fn plugins(&self) -> &Plugins {
        &self.plugins
    }

    /// Offers `native`, the line the commentator chose for `event`, to the commentary hook.
    /// Returns the line to use and, when the hook failed, the `script` events to record on
    /// the event's tick. Without a commentary hook the line comes back as it went in.
    pub fn offer_line(
        &mut self,
        event: &EngineEvent,
        native: Option<String>,
    ) -> (Option<String>, Vec<EngineEvent>) {
        let (Some(native), Some(hook)) = (native.as_deref(), self.plugins.commentary.as_mut())
        else {
            return (native, Vec::new());
        };
        let ctx = crate::plugin::LineContext {
            tick: event.tick,
            minute: event.minute,
            kind: event.kind.code(),
            team: event.team,
            scores: event.scores,
        };
        let outcome = hook.line(&ctx, native);
        let (line, notes) = self
            .plugins
            .settle(crate::plugin::HookPoint::Commentary, outcome);
        let events = notes
            .into_iter()
            .map(|note| self.script_event_at(event.tick, note))
            .collect();
        (Some(line.unwrap_or_else(|| native.to_string())), events)
    }

    /// Records the notes a hook call produced as `script` events on the tick this step
    /// produces.
    pub(crate) fn push_script_notes(&mut self, notes: Vec<ScriptNote>) {
        for note in notes {
            let event = self.script_event_at(self.tick + 1, note);
            self.events.push(event);
        }
    }

    fn script_event_at(&self, tick: u32, note: ScriptNote) -> EngineEvent {
        let mut event = self.event_at(tick, EngineEventKind::Script, None);
        event.detail = Some(EventDetail::Script(note));
        event
    }

    pub fn config(&self) -> &MatchConfig {
        &self.config
    }

    pub fn teams(&self) -> [Team; 2] {
        self.teams.clone()
    }

    /// The team shapes in force from each tick on, for the validator.
    pub fn team_timeline(&self) -> &[(u32, [Team; 2])] {
        &self.timeline
    }

    pub fn players(&self) -> &[Player] {
        &self.players
    }

    /// Each player's identifier from the team file, in roster order. A substitution changes
    /// the identifier in the substitute's roster slot.
    pub fn player_ids(&self) -> Vec<String> {
        self.players
            .iter()
            .map(|p| self.teams[p.team].player_ids[p.squad].clone())
            .collect()
    }

    /// Each player's display name from the team file, in roster order. A substitution
    /// changes the name in the substitute's roster slot.
    pub fn player_names(&self) -> Vec<String> {
        self.players
            .iter()
            .map(|p| self.teams[p.team].player_names[p.squad].clone())
            .collect()
    }

    /// The seed the match was built from.
    pub fn seed(&self) -> u64 {
        self.config.seed
    }

    /// The regulation length of the match in minutes.
    pub fn minutes(&self) -> u32 {
        self.config.minutes
    }

    /// Who manages each team, home first.
    pub fn managers(&self) -> [Manager; 2] {
        self.managers
    }

    pub fn tick(&self) -> u32 {
        self.tick
    }

    pub fn summary(&self) -> Summary {
        self.summary
    }

    /// The current half, from 0.
    pub fn half(&self) -> u32 {
        self.referee.clock.half
    }

    /// The minute of play the match clock shows now, and the added minute in added time.
    pub fn minute(&self) -> (u32, Option<u32>) {
        self.referee.clock.minute(self.tick)
    }

    /// `true` while the penalty shoot-out is being played.
    pub fn in_shootout(&self) -> bool {
        self.referee.shootout.is_some() && !self.is_over()
    }

    /// `true` once the match has ended at full time or been abandoned.
    pub fn is_over(&self) -> bool {
        self.referee.phase == Phase::FullTime
    }

    /// `true` when the match ended because a team fell below the rule pack's minimum.
    pub fn abandoned(&self) -> bool {
        self.referee.abandoned
    }

    /// The player controlling the ball, if any.
    pub fn carrier(&self) -> Option<usize> {
        self.carrier
    }

    /// The dead ball waiting for its restart, if play is stopped.
    pub fn dead_ball(&self) -> Option<crate::rules::DeadBall> {
        match self.referee.phase {
            Phase::DeadBall(dead) => Some(dead),
            _ => None,
        }
    }

    /// The stoppage the last tick announced, if it announced one.
    pub fn stoppage(&self) -> Option<Stoppage> {
        self.stoppage
    }

    /// Plays to full time, emitting one record per tick and announcing every stoppage
    /// through the sink's stoppage hook, then closes the match.
    pub fn run<S: TickSink>(&mut self, sink: &mut S) -> Result<(), EngineError> {
        while !self.is_over() {
            self.step();
            sink.on_tick(&self.record())?;
            if let Some(stoppage) = self.stoppage {
                sink.on_stoppage(&stoppage, self)?;
            }
        }
        self.finish();
        Ok(())
    }

    /// Records full time. A caller that drives `step` itself calls this after the last tick;
    /// a second call records nothing.
    pub fn finish(&mut self) {
        if self.finished {
            return;
        }
        self.finished = true;
        self.referee.phase = Phase::FullTime;
        let added = self
            .referee
            .clock
            .plays_added
            .then(|| self.period_added_s(self.referee.clock.half));
        let mut event = self.event_at(self.tick, EngineEventKind::FullTime, None);
        event.added_time_s = added;
        event.shootout_scores = self.summary.shootout;
        event.decided_by = self.summary.decided_by;
        self.events.push(event);
    }

    /// Takes every event recorded since the last call, in tick order.
    pub fn take_events(&mut self) -> Vec<EngineEvent> {
        std::mem::take(&mut self.events)
    }

    /// Advances the match by one tick. Does nothing once the match is over.
    pub fn step(&mut self) {
        if self.is_over() {
            return;
        }
        self.restart = false;
        self.stoppage = None;
        let t = self.config.tuning.clone();
        match self.referee.phase {
            // During a shoot-out kick nobody decides: every player keeps the target the kick
            // was set up with, and the keeper the dive it committed to.
            Phase::Live if self.referee.shootout.is_some() => {}
            Phase::Live => {
                if let Some(team) = self.last_touch {
                    self.summary.possession_ticks[team] += 1;
                }
                if self.tick.is_multiple_of(t.decision_interval_ticks)
                    && let Some(kick) = self.decide()
                {
                    self.apply_kick(kick, &t, true);
                }
            }
            Phase::DeadBall(dead) => {
                self.summary.dead_ball_ticks += 1;
                self.dead_ball_tick(&dead, &t);
            }
            Phase::FullTime => {}
        }
        steering::step_all(&mut self.players, &mut self.scratch, &t);
        if self.referee.phase == Phase::Live {
            self.move_ball(&t);
        }
        if self.referee.phase == Phase::Live {
            self.resolve_possession(&t);
        }
        if self.referee.phase == Phase::Live && self.referee.shootout.is_some() {
            self.shootout_kick_expiry();
        }
        self.check_clock();
        if !self.is_over() {
            self.fatigue_tick();
            // Nobody manages a team during the shoot-out.
            if self.referee.shootout.is_none() {
                self.ai_tick();
            }
        }
        if let Some(stoppage) = self.stoppage {
            self.apply_changes(stoppage);
            self.script_cache = None;
        }
        steering::resolve_overlaps(&mut self.players, &t);
        self.tick += 1;
    }

    /// The current tick as a record with 32-bit positions.
    pub fn record(&self) -> TickRecord {
        let mut players = [[0.0f32; 2]; 2 * PLAYERS_PER_TEAM];
        for (slot, p) in players.iter_mut().zip(self.players.iter()) {
            *slot = [p.pos.x as f32, p.pos.y as f32];
        }
        TickRecord {
            tick: self.tick,
            ball: [
                self.ball.pos.x as f32,
                self.ball.pos.y as f32,
                self.ball.pos.z as f32,
            ],
            players,
            restart: self.restart,
        }
    }

    /// An event of `kind` on the tick this step produces.
    pub(crate) fn event(&self, kind: EngineEventKind, team: Option<usize>) -> EngineEvent {
        self.event_at(self.tick + 1, kind, team)
    }

    fn event_at(&self, tick: u32, kind: EngineEventKind, team: Option<usize>) -> EngineEvent {
        // The clock stops when the shoot-out starts: its events show the last minute of play.
        let (minute, minute_added) = if self.referee.shootout.is_some() {
            (self.referee.clock.end_minute(), None)
        } else {
            self.referee.clock.minute(tick)
        };
        EngineEvent {
            tick,
            kind,
            team,
            scores: self.summary.goals,
            minute,
            minute_added,
            player: None,
            secondary: None,
            card: None,
            advantage: None,
            added_time_s: None,
            spot: None,
            detail: None,
            period: None,
            shootout_round: None,
            shootout_scored: None,
            shootout_scores: None,
            decided_by: None,
        }
    }

    /// Kicks the ball for the carrier. In open play the kick fixes who is in an offside
    /// position; a throw-in, a goal kick, and a corner fix nobody.
    pub(crate) fn apply_kick(&mut self, kick: Kick, t: &Tuning, offside_counts: bool) {
        let (dir, speed, loft) = match kick {
            Kick::Pass { dir, speed, loft } | Kick::Shot { dir, speed, loft } => (dir, speed, loft),
        };
        if let Some(c) = self.carrier {
            let team = self.players[c].team;
            self.pass_in_flight = None;
            self.shot_in_flight = None;
            if matches!(kick, Kick::Shot { .. }) {
                self.summary.shots[team] += 1;
                let xg = shot_xg(self.ball.xy(), self.teams[team].attack_x, &t.xg);
                self.summary.xg[team] += xg;
                self.shot_in_flight = Some(team);
            } else {
                self.summary.passes[team] += 1;
                self.pass_in_flight = Some(team);
            }
            self.last_touch = Some(team);
            self.last_kicker = Some(c);
            self.referee.offside = if offside_counts {
                self.summary.offside_checks += 1;
                offside::offside_set(c, self.ball.pos.x, &self.players, self.teams[team].attack_x)
            } else {
                0
            };
        }
        self.ball.kick(dir, speed, loft, t);
        self.carrier = None;
        self.keeper_beaten = false;
    }

    fn move_ball(&mut self, t: &Tuning) {
        match self.carrier {
            Some(c) => {
                let p = &self.players[c];
                let at = pitch::clamp(p.pos + p.facing * 0.5, 0.1);
                let step = crate::math::clamp_len(at - self.ball.xy(), t.carry_step);
                let next = self.ball.xy() + step;
                self.ball.pos = DVec3::new(next.x, next.y, 0.0);
                self.ball.vel = DVec3::new(p.vel.x, p.vel.y, 0.0);
            }
            None => {
                let prev = self.ball.xy();
                self.ball.integrate(t);
                let xy = self.ball.xy();
                if self.referee.shootout.is_some() {
                    self.shootout_ball(prev, xy, t);
                    return;
                }
                for team in 0..2 {
                    if pitch::in_goal(prev, xy, self.teams[team].attack_x)
                        && self.ball.pos.z < t.crossbar_height
                    {
                        self.goal(team);
                        return;
                    }
                }
                if let Some(exit) = pitch::exit(prev, xy) {
                    self.ball_out(exit);
                    return;
                }
            }
        }
        let speed = self.ball.speed();
        if speed > self.summary.ball_max_speed {
            self.summary.ball_max_speed = speed;
        }
        if speed == 0.0 && self.carrier.is_none() {
            self.summary.ball_idle_ticks += 1;
        }
    }

    fn resolve_possession(&mut self, t: &Tuning) {
        if self.referee.shootout.is_some() {
            self.shootout_save(t);
            return;
        }
        let ball_xy = self.ball.xy();
        match self.carrier {
            None => {
                if self.ball.pos.z > t.reach_height {
                    return;
                }
                let fast = self.ball.speed() > t.control_speed;
                let keepers = [self.keeper(0), self.keeper(1)];
                let mut best: Option<(f64, usize)> = None;
                for (i, p) in self.players.iter().enumerate() {
                    let keeper = i == keepers[p.team];
                    if !p.active() || (fast && !keeper) {
                        continue;
                    }
                    let reach = if keeper {
                        t.keeper_reach
                    } else {
                        t.reach_radius
                    };
                    let d = (p.pos - ball_xy).length();
                    if d < reach && best.is_none_or(|(bd, _)| d < bd) {
                        best = Some((d, i));
                    }
                }
                if let Some((_, i)) = best {
                    if fast {
                        if self.keeper_beaten {
                            return;
                        }
                        if !self.rng.chance(t.keeper_catch_chance) {
                            self.keeper_beaten = true;
                            return;
                        }
                    }
                    self.gain(i, t);
                }
            }
            Some(c) => {
                if self.tick.saturating_sub(self.control_since) < t.control_cooldown_ticks {
                    return;
                }
                let carrier = self.players[c];
                for i in 0..self.players.len() {
                    let p = self.players[i];
                    if !p.active()
                        || p.team == carrier.team
                        || self.tick < p.foul_ready
                        || (p.pos - ball_xy).length() > t.reach_radius
                    {
                        continue;
                    }
                    let p_win = fouls::win_chance(&p.derived, &carrier.derived, t);
                    let p_foul = fouls::foul_chance(&p.derived, p.yellow, t);
                    let draw = self.rng.referee_draw();
                    match fouls::tackle_outcome(p_win, p_foul, t.foul_ball_loss, draw) {
                        Tackle::Win => {
                            self.gain(i, t);
                            self.tackle_injury_roll(c);
                            return;
                        }
                        Tackle::Foul { ball_lost } => {
                            self.foul(i, c, ball_lost, t);
                            self.tackle_injury_roll(c);
                            return;
                        }
                        Tackle::Miss => {}
                    }
                }
            }
        }
    }

    /// Player `i` touches the ball first. A player in an offside position is penalised
    /// instead of gaining the ball.
    fn gain(&mut self, i: usize, _t: &Tuning) {
        if offside::is_offence(self.referee.offside, i) {
            self.offside_offence(i);
            return;
        }
        self.referee.offside = 0;
        self.keeper_beaten = false;
        let team = self.players[i].team;
        if self.pass_in_flight.take() == Some(team) {
            self.summary.passes_completed[team] += 1;
        }
        if let Some(shooter) = self.shot_in_flight.take()
            && shooter != team
            && i == self.keeper(team)
        {
            self.summary.shots_on_target[shooter] += 1;
        }
        if self.last_touch != Some(team) {
            self.summary.possession_changes += 1;
        }
        self.last_touch = Some(team);
        self.carrier = Some(i);
        self.control_since = self.tick;
        self.script_cache = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::test_support::shipped_config;
    use crate::record::VecSink;

    #[test]
    fn five_minutes_produce_fifteen_thousand_ticks() {
        let mut sim = Simulation::new(shipped_config(42, 5).unwrap()).unwrap();
        let mut sink = VecSink::default();
        sim.run(&mut sink).unwrap();
        assert_eq!(sink.records.len(), 15_000);
        assert_eq!(sink.records[0].tick, 1);
        assert!(
            sim.summary().possession_changes > 0,
            "nobody ever gained possession"
        );
    }

    #[test]
    fn a_chosen_setup_puts_the_chosen_players_in_slot_order() {
        let config = shipped_config(42, 1).unwrap();
        let hash = config.content_hash.clone();
        let lineup = [11, 1, 2, 3, 4, 5, 6, 7, 8, 9, 20];
        let tactics = config.teams[0].tactics;
        let config = config.with_setup(0, lineup, vec![0, 10, 12]);
        for (slot, &squad) in lineup.iter().enumerate() {
            assert_eq!(config.players[slot].squad, squad, "slot {slot}");
            assert_eq!(config.players[slot].slot, slot);
        }
        assert_eq!(config.teams[0].bench, vec![0, 10, 12]);
        assert_eq!(
            config.teams[0].tactics, tactics,
            "the pre-match tactics are kept"
        );
        assert_eq!(config.content_hash, hash);
        let sim = Simulation::new(config).unwrap();
        assert_eq!(sim.teams()[0].lineup, lineup);
    }

    #[test]
    fn zero_minutes_is_rejected() {
        assert!(shipped_config(1, 0).is_err());
    }

    #[test]
    fn the_config_names_both_clubs_and_the_content() {
        let config = shipped_config(1, 1).unwrap();
        assert_ne!(config.club_ids()[0], config.club_ids()[1]);
        assert_eq!(config.content_hash.len(), 12);
        assert_eq!(config.players.len(), 22);
        assert_ne!(config.team_digests[0], config.team_digests[1]);
        assert_eq!(config.max_ticks(), 3_000);
    }

    #[test]
    fn a_knockout_match_announces_extra_time_and_ten_rounds_of_kicks() {
        let config = shipped_config(1, 90).unwrap();
        assert_eq!(config.max_ticks(), 360_000);
        let knockout = config.with_knockout();
        assert_eq!(knockout.max_ticks(), 360_000 + 120_000 + 50_000);
        // A shortened knockout match plays no extra time, only the shoot-out.
        let short = shipped_config(1, 5).unwrap().with_knockout();
        assert_eq!(short.max_ticks(), 15_000 + 50_000);
    }

    /// Steps the seed-42 90-minute match, calling `each` after every tick.
    fn full_match(mut each: impl FnMut(&Simulation)) -> Simulation {
        let mut sim = Simulation::new(shipped_config(42, 90).unwrap()).unwrap();
        while !sim.is_over() {
            sim.step();
            each(&sim);
        }
        sim.finish();
        sim
    }

    #[test]
    fn the_last_kicker_is_clear_whenever_play_stops() {
        let mut stoppages = 0;
        full_match(|sim| {
            if sim.stoppage().is_some() {
                stoppages += 1;
                assert_eq!(sim.last_kicker, None, "tick {}", sim.tick());
            }
        });
        // The seed-42 match stops play 39 times (58 before the defending rework, which cut its
        // fouls, throw-ins and offsides); the floor only guards against an empty check.
        assert!(stoppages > 30, "only {stoppages} stoppages");
    }

    #[test]
    fn every_play_event_names_a_player() {
        let events = full_match(|_| {}).take_events();
        let goals = events
            .iter()
            .filter(|e| e.kind == EngineEventKind::Goal)
            .count();
        assert!(goals > 0, "the match scored no goal");
        for e in &events {
            // The manager's own events and the whistles at the end of a half name nobody.
            let named = !matches!(
                e.kind,
                EngineEventKind::HalfTime
                    | EngineEventKind::FullTime
                    | EngineEventKind::AiDecision
                    | EngineEventKind::ChangeApplied
                    | EngineEventKind::ChangeRejected
            );
            if named {
                assert!(e.player.is_some(), "{e:?}");
            }
        }
    }
}
