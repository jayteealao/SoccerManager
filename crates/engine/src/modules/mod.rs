//! The module contract. Each engine part that moved onto it is a module behind a typed
//! interface, registered by name and version in one ordered table ([`registry::REGISTRY`])
//! and chosen per slot by the slot file (`content/slots.json`).
//!
//! A module reads the match through a [`MatchView`], which has private fields and read-only
//! accessors, so the compiler refuses a module that writes match state. A module that needs
//! a change returns a [`Proposal`]; only the central loop applies it. A module never draws:
//! the loop draws on the action keys the module's card lists, and the module maps the draw
//! to an outcome. So every stream id, every draw, and every draw order stay as they were.

pub mod card;
pub mod config;
pub mod fast_events;
pub mod fast_model;
pub mod game;
pub mod modifier;
pub mod proposal;
pub mod registry;
#[cfg(feature = "scenario")]
pub mod stand_in;
pub mod view;
pub mod viewer;

use std::fmt;

use crate::ai::{AiCode, AiState, Setup};
use crate::ball::Ball;
use crate::data::Loaded;
use crate::data::attributes::AttributeSchema;
use crate::data::rules::RulePack;
use crate::data::rules::StoppageKind;
use crate::data::tactics::TacticsSchema;
use crate::decision::{Choice, Kick, MAX_PRESSERS, Options};
use crate::error::EngineError;
use crate::fatigue::InjurySource;
use crate::math::{DVec2, DVec3};
use crate::pitch::Exit;
use crate::plugin::{DecisionContext, FoulContext, LineContext, OptionOffsets};
use crate::rules::DeadBall;
use crate::rules::fouls::{Card, Tackle};
use crate::rules::offside::OffsideSet;
use crate::sim::{DecidedBy, EngineEvent};
use crate::tactics::change::{Change, RejectReason};
use crate::tactics::{Tactics, TacticsPatch};
use crate::team::{PLAYERS_PER_TEAM, Team};

pub use card::{CardError, MOVED_KEYS, ModuleCard, OwnershipError, check_card, check_ownership};
pub use config::{SLOTS_VERSION, SlotEntry, SlotFile, resolve};
pub use game::{GameChanges, GameDay, PeopleModule, PresentationModule, SeasonModule, WorldModule};
pub use modifier::Modifiers;
pub use proposal::Proposal;
pub use registry::{MODIFIER_COUNT, ModuleRef, REGISTRY, Registration, SLOT_COUNT, SlotDecl};
pub use view::MatchView;
pub use viewer::{SKIN_NAMES, SkinModule};

/// A slot: a place in the engine that one module fills.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Slot {
    /// The dotted id the slot file names, such as `engine.fouls`.
    pub id: &'static str,
    /// A required slot refuses `off`.
    pub required: bool,
}

/// The chances of one tackle attempt, computed before its one draw.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TackleChances {
    /// A clean win below this.
    pub p_win: f64,
    /// A foul in the next band of this width.
    pub p_foul: f64,
    /// The share of the foul band after which the fouled team loses the ball.
    pub ball_loss: f64,
}

/// Tackles, fouls, and cards. The loop draws the `Tackle` key against
/// `[p_win, p_win + p_foul]` and the `FoulCard` key against [`FoulsModule::card_thresholds`].
pub trait FoulsModule: Send + Sync + 'static {
    /// The chances of one tackle attempt by `tackler` on `carrier`.
    fn tackle_chances(&self, view: &MatchView<'_>, tackler: usize, carrier: usize)
    -> TackleChances;
    /// What the attempt did, given its draw.
    fn tackle_outcome(&self, chances: &TackleChances, draw: f64) -> Tackle;
    /// The two cumulative thresholds of the card draw for a foul by `offender`.
    fn card_thresholds(&self, view: &MatchView<'_>, offender: usize) -> [f64; 2];
    /// The card, if any, for a foul by `offender`, given the card draw.
    fn card(&self, view: &MatchView<'_>, offender: usize, draw: f64) -> Option<Card>;
}

/// Offside (IFAB Law 11). It draws nothing.
pub trait OffsideModule: Send + Sync + 'static {
    /// `passer` of `team` plays the ball in open play: the offside positions it fixes.
    fn on_kick(&self, view: &MatchView<'_>, passer: usize, team: usize) -> Proposal;
    /// `true` when `toucher` commits an offence, given the positions fixed at the kick.
    fn is_offence(&self, view: &MatchView<'_>, set: OffsideSet, toucher: usize) -> bool;
}

/// Shots: expected goals, chance quality, whether a shot's flight is on target, and the
/// keeper's save chance. The loop draws the `Save` and `ShootoutSave` keys against
/// [`ShotModule::save_chance`].
pub trait ShotModule: Send + Sync + 'static {
    /// The expected goals of a shot from `from` at the goal a team attacking `attack_x` aims at.
    fn xg(&self, view: &MatchView<'_>, from: DVec2, attack_x: f64) -> f64;
    /// The quality of that shot, which sets the keeper's save chance.
    fn quality(&self, view: &MatchView<'_>, from: DVec2, attack_x: f64) -> f64;
    /// `true` when `ball`, just kicked and left alone, goes in under the bar.
    fn on_target(&self, view: &MatchView<'_>, ball: Ball, attack_x: f64) -> bool;
    /// The chance that keeper `keeper` saves a shot of `quality` heading on target, struck by
    /// `shooter` when the loop knows him.
    fn save_chance(
        &self,
        view: &MatchView<'_>,
        quality: f64,
        keeper: usize,
        shooter: Option<usize>,
    ) -> f64;
}

/// Fatigue and injury chances. The loop writes energy, and draws the `InjuryMinute` and
/// `InjuryTackle` keys against [`FatigueModule::injury_chance`]. The effect of energy on the
/// effective values is the fatigue modifier ([`modifier`]). The slot also owns the
/// `FormMatch` and `FormPeriod` keys, which the loop draws for each player's form offset
/// beside his energy.
pub trait FatigueModule: Send + Sync + 'static {
    /// The energy player `i` loses in one tick.
    fn drain(&self, view: &MatchView<'_>, i: usize) -> f64;
    /// The energy every player on the pitch loses in one tick, in roster order, into `out`
    /// (zero for a player off the pitch). The default asks [`Self::drain`] once per player,
    /// so the loop makes one call through the slot per tick, not one per player.
    fn drains(&self, view: &MatchView<'_>, out: &mut [f64; ROSTER]) {
        for (i, p) in view.players().iter().enumerate().take(ROSTER) {
            out[i] = if p.active() { self.drain(view, i) } else { 0.0 };
        }
    }
    /// The chance that one injury roll of `source` injures player `i`.
    fn injury_chance(&self, view: &MatchView<'_>, i: usize, source: InjurySource) -> f64;
}

/// Steering: the velocity each player moves at next. It draws nothing. The loop computes
/// every velocity from the old state, then applies them all.
pub trait SteeringModule: Send + Sync + 'static {
    /// The velocity of player `i` after one tick of steering toward its target.
    fn next_velocity(&self, view: &MatchView<'_>, i: usize) -> DVec2;
    /// Every player's velocity after one tick, in roster order, into `out` (zero for a player
    /// off the pitch). The default asks [`Self::next_velocity`] once per player, so the loop
    /// makes one call through the slot per tick, not one per player.
    fn next_velocities(&self, view: &MatchView<'_>, out: &mut Vec<DVec2>) {
        out.clear();
        for (i, p) in view.players().iter().enumerate() {
            out.push(if p.active() {
                self.next_velocity(view, i)
            } else {
                DVec2::ZERO
            });
        }
    }
    /// Every player's position once players closer than the minimum distance are pushed
    /// apart, in roster order; `None` when no pair is that close, so nobody moves.
    fn separate(&self, view: &MatchView<'_>) -> Option<[DVec2; ROSTER]>;
}

/// The pre-match setup of a team: its lineup, bench, and tactics. It runs before a match
/// exists, so it reads the team and the content, and draws nothing.
pub trait PreMatchModule: Send + Sync + 'static {
    /// The setup for `team`.
    fn setup(&self, team: &Team, tactics: &TacticsSchema, attrs: &AttributeSchema) -> Setup;
    /// The setup for `team` starting with `start` (an AI-managed side started in another
    /// formation): the lineup and bench picked for `start`'s slots and roles.
    fn setup_for(
        &self,
        team: &Team,
        start: Tactics,
        tactics: &TacticsSchema,
        attrs: &AttributeSchema,
    ) -> Setup;
}

/// What happens when a period's time, added time included, has run out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeriodEnd {
    /// A break before the next period; `recover` gives energy back (regulation half-time).
    Break { recover: bool },
    /// A level knockout match goes to the penalty shoot-out.
    Shootout,
    /// The match ends; a knockout match records how it was decided.
    FullTime { decided_by: Option<DecidedBy> },
}

/// Who takes part in a penalty shoot-out: each team's kickers in order, and its keeper.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShootoutLineup {
    pub order: [Vec<usize>; 2],
    pub keepers: [usize; 2],
}

/// The clock and match end: added time, what follows each period, the extra-time kick-off
/// toss, abandonment, and the penalty shoot-out. The loop draws the `AddedTime`,
/// `ExtraTimeAdded`, `ExtraKickOff`, `ShootoutFirstTeam`, `ShootoutEnd`, `KeeperDive`, and
/// `ShootoutSaveHold` keys and passes each draw in.
pub trait ClockModule: Send + Sync + 'static {
    /// The seconds the rule pack allows for the stoppages and cards of the current period;
    /// `extra` prices an extra-time period.
    fn tally_seconds(&self, view: &MatchView<'_>, extra: bool) -> u32;
    /// The seconds added to the current period, given the added-time draw.
    fn added_seconds(&self, view: &MatchView<'_>, extra: bool, draw: f64) -> u32;
    /// What follows the current period, once its time has run out.
    fn period_end(&self, view: &MatchView<'_>) -> PeriodEnd;
    /// The team that kicks off extra time, given the toss draw.
    fn extra_kick_off(&self, draw: f64) -> usize;
    /// The first team with too few players on the pitch to go on, or `None`.
    fn abandoned(&self, view: &MatchView<'_>) -> Option<usize>;
    /// The kickers and keepers of the shoot-out.
    fn shootout_lineup(&self, view: &MatchView<'_>) -> ShootoutLineup;
    /// The team that kicks first, given the toss draw.
    fn shootout_first(&self, draw: f64) -> usize;
    /// The end every kick is taken at (the sign of the goal line's `x`), given the toss draw.
    fn shootout_end(&self, draw: f64) -> f64;
    /// The side the keeper dives to (-1, 0, or +1), given the dive draw in `[-1, 1)`.
    fn keeper_dive(&self, view: &MatchView<'_>, draw: f64) -> f64;
    /// The `ShootoutSaveHold` threshold: a draw below it holds a saved shoot-out kick.
    fn shootout_save_hold(&self, view: &MatchView<'_>, keeper: usize) -> f64;
    /// `true` once the shoot-out has a winner.
    fn shootout_decided(&self, view: &MatchView<'_>, scores: [u32; 2], taken: [u32; 2]) -> bool;
}

/// Restarts and dead balls: the taker, the wait, where each player stands while the ball
/// is dead, when the restart may be taken, and the kick-off positions. It draws nothing.
pub trait RestartsModule: Send + Sync + 'static {
    /// The player of `team` who takes a restart of `kind` at `spot`.
    fn taker(&self, view: &MatchView<'_>, kind: StoppageKind, team: usize, spot: DVec2) -> usize;
    /// Ticks a restart of `kind` by `team` waits before it may be taken.
    fn delay_ticks(&self, view: &MatchView<'_>, kind: StoppageKind, team: usize) -> u32;
    /// Ticks a shoot-out kick waits before it may be taken.
    fn shootout_delay_ticks(&self, view: &MatchView<'_>) -> u32;
    /// Where player `i` stands while `dead` waits.
    fn target(&self, view: &MatchView<'_>, dead: &DeadBall, i: usize) -> DVec2;
    /// `true` when `dead` may be taken at tick `now`.
    fn ready(&self, view: &MatchView<'_>, dead: &DeadBall, now: u32) -> bool;
    /// Where the player in `slot` of `team` stands for a kick-off by `kicking`.
    fn kick_off_position(
        &self,
        view: &MatchView<'_>,
        team: usize,
        slot: usize,
        kicking: usize,
    ) -> DVec2;
}

/// Discipline: which card a player is shown, whether it sends the player off, and the more
/// severe of two cards. It draws nothing; the fouls module decides a foul's card.
pub trait DisciplineModule: Send + Sync + 'static {
    /// The card player `i` is shown for `card`, or `None` when no card is shown.
    fn shown(&self, view: &MatchView<'_>, i: usize, card: Card) -> Option<Card>;
    /// `true` when `card` sends the player off.
    fn sends_off(&self, card: Card) -> bool;
    /// The more severe of `a` and `b`; `a` on a tie.
    fn more_severe(&self, a: Card, b: Card) -> Card;
}

/// Injuries: whether an injury takes a player off, and the dropped ball it causes in open
/// play. It draws nothing; the fatigue module sets the injury chances.
pub trait InjuriesModule: Send + Sync + 'static {
    /// `true` when an injury of `source` takes player `i` off.
    fn leaves(&self, view: &MatchView<'_>, i: usize, source: InjurySource) -> bool;
    /// The team given the dropped ball after an injury to a player of `team` in open play,
    /// and the spot.
    fn dropped_ball(&self, view: &MatchView<'_>, team: usize) -> (usize, DVec2);
}

/// The players on the pitch in a match, both teams.
pub const ROSTER: usize = 2 * PLAYERS_PER_TEAM;

/// The most team-mates a carrier can pass to.
pub const MATES: usize = PLAYERS_PER_TEAM - 1;

/// Where the ball crossed a line in open play.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Crossing {
    /// Into the goal `team` attacks, under the bar.
    Goal(usize),
    /// Over a touchline or a goal line.
    Out(Exit),
    None,
}

/// How a ball is turned away: blocked back the way it came, parried along the goal line to
/// `side`, or cleared along `away` with up to `spread` either way.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Deflection {
    Block,
    Parry { side: f64 },
    Clear { away: DVec2, spread: f64 },
}

/// Ball physics: the carried ball, flight and roll, a kick, the lines it crosses, and the
/// turn of a deflected ball. The loop draws the `BlockDeflect`, `ParryAngle`, `ParryLoft`,
/// `CrossAngle`, and `CrossLoft` keys and passes each raw draw in.
pub trait BallModule: Send + Sync + 'static {
    /// The ball after one tick at the feet of carrier `c`.
    fn carry(&self, view: &MatchView<'_>, c: usize) -> Ball;
    /// `ball` after one tick of flight or roll.
    fn integrate(&self, view: &MatchView<'_>, ball: Ball) -> Ball;
    /// `ball` kicked along the ground unit vector `dir` at `speed` with vertical speed `loft`.
    fn kick(&self, view: &MatchView<'_>, ball: Ball, dir: DVec2, speed: f64, loft: f64) -> Ball;
    /// The line `ball` crossed since it was at `prev`, in open play.
    fn crossing(&self, view: &MatchView<'_>, prev: DVec2, ball: &Ball) -> Crossing;
    /// The velocity of the ball in the view after `how`, given the raw angle and loft draws.
    fn deflect(&self, view: &MatchView<'_>, how: Deflection, angle: f64, loft: f64) -> DVec3;
}

/// The nearest player who can reach a loose ball.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LooseBall {
    /// The distance and roster index of the nearest player within reach, if any.
    pub best: Option<(f64, usize)>,
    /// `true` when the ball is too fast to control: only a keeper may reach it.
    pub fast: bool,
    /// The chance a keeper holds a fast ball.
    pub catch_chance: f64,
    /// The chance the nearest player controls a ball he meets above head height, against
    /// the best header of an opponent within reach; `None` when the ball is not a header.
    pub header: Option<f64>,
}

/// The defenders (one bit per roster index, read from bit 0 up) who may try to stop the
/// ball, and each one's chance.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Contest {
    pub mask: u32,
    pub chance: f64,
}

/// A fast pass in the penalty area that the defending side may clear.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CrossContest {
    /// The defending team.
    pub team: usize,
    pub contest: Contest,
    /// The chance a clearance goes wide; `None` when it never does.
    pub wide_chance: Option<f64>,
}

/// Which way a keeper parries: fixed by the ball's side of the goal, or drawn against the
/// threshold when the ball is dead centre.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ParrySide {
    Fixed(f64),
    Draw(f64),
}

/// Possession: who reaches a loose ball, who may block a shot, whether the keeper reaches a
/// shot on target and holds it, the side of a parry, who may clear a cross and along which
/// line, and who may tackle the carrier. The loop draws the `Block`, `SaveHold`, `ParrySide`,
/// `CrossClear`, `CrossWide`, and `KeeperCatch` keys.
pub trait PossessionModule: Send + Sync + 'static {
    /// The team whose shot is in flight and fast enough to be contested, or `None`.
    fn shot_contest(&self, view: &MatchView<'_>) -> Option<usize>;
    /// The outfield defenders who may try to block the shot of `shooter`.
    fn blockers(&self, view: &MatchView<'_>, shooter: usize) -> Contest;
    /// The keeper who can reach the shot of `shooter` on target, or `None`.
    fn save_reach(&self, view: &MatchView<'_>, shooter: usize) -> Option<usize>;
    /// The `SaveHold` threshold: a draw below it holds a save.
    fn save_hold(&self, view: &MatchView<'_>, keeper: usize) -> f64;
    /// The side keeper `k` parries to.
    fn parry_side(&self, view: &MatchView<'_>, k: usize) -> ParrySide;
    /// The parry side for a drawn side, given the draw and the threshold.
    fn parry_side_from_draw(&self, draw: f64, threshold: f64) -> f64;
    /// The defenders who may try to clear the pass in flight, or `None`.
    fn cross_clearers(&self, view: &MatchView<'_>) -> Option<CrossContest>;
    /// The line and the spread of a clearance by player `i`, wide or not.
    fn clearance_line(&self, view: &MatchView<'_>, i: usize, wide: bool) -> (DVec2, f64);
    /// Who reaches the loose ball, or `None` when it is above reach height.
    fn loose_ball(&self, view: &MatchView<'_>) -> Option<LooseBall>;
    /// The opponents who may tackle carrier `c`, or `None` inside the control cooldown.
    fn tacklers(&self, view: &MatchView<'_>, c: usize) -> Option<u32>;
}

/// What the goal-side cover found, for the debug trace.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CoverTrace {
    NoAttacker,
    NoDefender {
        attacker: usize,
    },
    Covered {
        attacker: usize,
        defender: usize,
        distance: f64,
    },
}

/// What the targets pass decided, for the debug trace.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TargetsTrace {
    /// The carrier's side attacks: the pressers of the defending side and the cover.
    Carrier {
        def: usize,
        count: usize,
        reach: f64,
        pressers: [(f64, usize); MAX_PRESSERS],
        cover: CoverTrace,
    },
    /// The ball is loose: each team's chaser and keeper.
    Loose {
        nearest: [usize; 2],
        nearest_dist: [f64; 2],
        keepers: [Option<usize>; 2],
    },
}

/// Every player's target for this tick, in roster order.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Targets {
    pub targets: [DVec2; ROSTER],
    pub trace: TargetsTrace,
}

/// The carrier's option scores before their noise term.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OptionDraft {
    /// The amplitude of each score's noise.
    pub noise: f64,
    pub shot: Option<f64>,
    /// The open team-mates in roster order and their pass scores.
    pub passes: [(usize, f64); MATES],
    pub pass_count: usize,
    /// The dribble and hold scores; `None` for a keeper. The dribble score is `None` when
    /// an opponent is close and the carrier does not try a take-on (the skill gate).
    pub dribble_hold: Option<(Option<f64>, f64)>,
    pub clear: f64,
    /// The carry cost subtracted from the best pass after the pick.
    pub carry: f64,
    pub nearest_opp_pos: DVec2,
}

/// The raw noise draws of one carrier's options, in the draft's order.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct OptionDraws {
    pub shot: f64,
    pub passes: [f64; MATES],
    pub dribble_hold: (f64, f64),
    pub clear: f64,
}

/// The scored options and every pass candidate's score, in roster order.
#[derive(Debug, Clone, Copy)]
pub struct Scored {
    pub options: Options,
    pub candidates: [(usize, f64); MATES],
    pub count: usize,
}

/// What the carrier does with its choice.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CarrierPlan {
    /// Shoot at `goal`, which `keeper` defends.
    Shot { goal: DVec2, keeper: usize },
    /// Pass to team-mate `j`.
    Pass { j: usize },
    /// Clear; `wide_chance` is set when the clearance may go wide.
    Clear { wide_chance: Option<f64> },
    /// Hold or dribble toward `target`.
    Move { target: DVec2 },
}

/// The raw draws of a shot: the side (only when the keeper is off the pitch), the aim, the
/// spread, and the loft.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShotDraws {
    pub side: Option<bool>,
    pub aim: f64,
    pub spread: f64,
    pub loft: f64,
}

/// A restart kick and what the taker considered, for the debug trace.
#[derive(Debug, Clone, Copy)]
pub struct RestartPass {
    pub kick: Kick,
    pub target: DVec2,
    pub fallback: bool,
    pub candidates: [(usize, f64); MATES],
    pub count: usize,
}

/// The decision maker: every player's target, the carrier's options, the choice, the kick,
/// the shot kick, and the restart pass. The loop draws the `ShotScore`, `PassScore`,
/// `DribbleScore`, `HoldScore`, `ClearScore`, `PassAim`, `ClearWide`, `ClearAim`, `ShotSide`,
/// `ShotAim`, `ShotSpread`, and `ShotLoft` keys and passes each raw draw in.
pub trait DecisionModule: Send + Sync + 'static {
    /// Every player's target for this tick.
    fn targets(&self, view: &MatchView<'_>) -> Targets;
    /// Carrier `c`'s option scores before their noise.
    fn options(&self, view: &MatchView<'_>, c: usize) -> OptionDraft;
    /// The options once each noise draw is added.
    fn scored(&self, draft: &OptionDraft, draws: &OptionDraws) -> Scored;
    /// The options after the decision hook's offsets, and the choice.
    fn choose(
        &self,
        view: &MatchView<'_>,
        c: usize,
        options: &Options,
        offsets: Option<OptionOffsets>,
    ) -> (Options, Choice);
    /// What carrier `c` does with `choice`.
    fn plan(
        &self,
        view: &MatchView<'_>,
        c: usize,
        options: &Options,
        choice: Choice,
    ) -> CarrierPlan;
    /// Carrier `c`'s pass to `j`, given the aim draw.
    fn pass_kick(&self, view: &MatchView<'_>, c: usize, j: usize, aim: f64) -> Kick;
    /// Carrier `c`'s clearance, wide or not, given the aim draw.
    fn clear_kick(&self, view: &MatchView<'_>, c: usize, wide: bool, aim: f64) -> Kick;
    /// `true` when a shot at a goal `keeper` defends draws its side.
    fn shot_draws_side(&self, view: &MatchView<'_>, keeper: usize) -> bool;
    /// Player `c`'s shot at `goal`, which `keeper` defends, given the draws.
    fn shot_kick(
        &self,
        view: &MatchView<'_>,
        c: usize,
        goal: DVec2,
        keeper: usize,
        spread_scale: f64,
        draws: &ShotDraws,
    ) -> Kick;
    /// The kick that takes a restart of `kind`.
    fn restart_pass(&self, view: &MatchView<'_>, taker: usize, kind: StoppageKind) -> RestartPass;
    /// The chance that player `i`, on the side without the ball, lapses in this minute of
    /// play. The loop draws the `Lapse` key against it.
    fn lapse_chance(&self, view: &MatchView<'_>, i: usize) -> f64;
}

/// One check of the AI manager: the minute, the changes to queue in order, and the manager's
/// memory after the check.
#[derive(Debug, Clone, PartialEq)]
pub struct AiPlan {
    pub minute: u32,
    pub changes: Vec<(Change, AiCode)>,
    pub memory: AiState,
}

/// The AI manager's in-match checks. It draws nothing.
pub trait ManagerModule: Send + Sync + 'static {
    /// One check for `team`; `at_stoppage` marks the check an injury or a goal asks for.
    fn check(&self, view: &MatchView<'_>, team: usize, at_stoppage: bool) -> AiPlan;
}

/// Where a substitute enters.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SubEntry {
    /// The lineup slot the substitute takes.
    pub slot: usize,
    pub at: DVec2,
    /// `true` when the substitution uses a new window.
    pub needs_window: bool,
    /// `true` when the substitute enters at the touchline, one place along from the last.
    pub from_touchline: bool,
}

/// One substitution asked for at a stoppage.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SubRequest {
    pub team: usize,
    /// The squad indices of the player leaving and the player coming on.
    pub off: usize,
    pub on: usize,
    pub kind: StoppageKind,
    /// The tick of the stoppage.
    pub now: u32,
    /// Substitutes of the team who already entered at the touchline at this stoppage.
    pub entered: usize,
}

/// Tactics changes: which changes a stoppage admits, the substitution verdict and where the
/// substitute enters, and the tactics verdict. It draws nothing.
pub trait ChangesModule: Send + Sync + 'static {
    /// Whether a stoppage of `kind` admits tactics changes and substitutions.
    fn admits(&self, view: &MatchView<'_>, kind: StoppageKind) -> (bool, bool);
    /// The verdict on one substitution, and where the substitute enters.
    fn substitution(
        &self,
        view: &MatchView<'_>,
        request: &SubRequest,
    ) -> Result<SubEntry, RejectReason>;
    /// `team`'s tactics after `patch`, or why not; `off_now` names the players the same
    /// stoppage substituted off.
    fn tactics(
        &self,
        view: &MatchView<'_>,
        team: usize,
        patch: &TacticsPatch,
        off_now: &[(usize, usize)],
    ) -> Result<Tactics, RejectReason>;
}

/// The decision hook's slot: the adapter between the loop and an attached decision hook. The
/// hook itself is per-match state the loop keeps (`Plugins`); this module only builds what
/// the hook sees. It draws nothing.
pub trait DecisionHookModule: Send + Sync + 'static {
    /// What the decision hook sees about carrier `carrier`, or `None` to consult no hook.
    fn context(&self, view: &MatchView<'_>, carrier: usize) -> Option<DecisionContext>;
}

/// The rule hook's slot: builds what an attached rule hook sees when the referee has judged
/// a foul. It draws nothing.
pub trait RuleHookModule: Send + Sync + 'static {
    /// What the rule hook sees about a foul by `offender`, or `None` to consult no hook.
    fn context(
        &self,
        view: &MatchView<'_>,
        offender: usize,
        advantage: bool,
        penalty: bool,
    ) -> Option<FoulContext>;
}

/// The commentary hook's slot: builds what an attached commentary hook sees for one event
/// that has a line. It draws nothing.
pub trait CommentaryHookModule: Send + Sync + 'static {
    /// What the commentary hook sees for `event`, or `None` to consult no hook.
    fn context(&self, view: &MatchView<'_>, event: &EngineEvent) -> Option<LineContext>;
}

/// The rule pack slot (`game.rules`): loads the rule pack a match plays under. It runs where
/// content is built, before any match exists, and draws nothing.
pub trait RulesModule: Send + Sync + 'static {
    /// The rule pack, given the bytes of the content folder's rule file.
    fn load(&self, written: &[u8]) -> Result<Loaded<RulePack>, EngineError>;
}

/// One slot's choice: the slot id, the module name, and its version (0 for `off`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Picked {
    pub slot: &'static str,
    pub module: &'static str,
    pub version: u32,
}

/// The modules one match runs, resolved once from the slot file. Modules are stateless, so
/// the references are shared and nothing about them enters a snapshot.
#[derive(Clone, Copy)]
pub struct ResolvedModules {
    pub fouls: &'static dyn FoulsModule,
    pub offside: &'static dyn OffsideModule,
    pub shot: &'static dyn ShotModule,
    pub fatigue: &'static dyn FatigueModule,
    pub steering: &'static dyn SteeringModule,
    pub pre_match: &'static dyn PreMatchModule,
    /// The four modifier slots, in registry order.
    pub modifiers: Modifiers,
    pub clock: &'static dyn ClockModule,
    pub restarts: &'static dyn RestartsModule,
    pub discipline: &'static dyn DisciplineModule,
    pub injuries: &'static dyn InjuriesModule,
    pub ball: &'static dyn BallModule,
    pub possession: &'static dyn PossessionModule,
    pub decision: &'static dyn DecisionModule,
    pub manager: &'static dyn ManagerModule,
    pub changes: &'static dyn ChangesModule,
    pub decision_hook: &'static dyn DecisionHookModule,
    pub rule_hook: &'static dyn RuleHookModule,
    pub commentary_hook: &'static dyn CommentaryHookModule,
    pub rule_pack: &'static dyn RulesModule,
    pub world: &'static dyn WorldModule,
    pub season: &'static dyn SeasonModule,
    pub people: &'static dyn PeopleModule,
    pub presentation: &'static dyn PresentationModule,
    pub skin: &'static dyn SkinModule,
    /// Private: only `fast_model::resolve` reaches it, for the fit and check commands.
    fast_model: &'static dyn fast_model::FastModel,
    picked: [Picked; SLOT_COUNT],
}

impl ResolvedModules {
    /// What each slot holds, in registry order.
    pub fn picked(&self) -> &[Picked] {
        &self.picked
    }

    /// The module chosen for `slot`, or `None` for an undeclared slot.
    pub fn picked_for(&self, slot: &str) -> Option<Picked> {
        self.picked.iter().copied().find(|p| p.slot == slot)
    }
}

impl fmt::Debug for ResolvedModules {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut list = f.debug_map();
        for p in &self.picked {
            list.entry(&p.slot, &format_args!("{}@{}", p.module, p.version));
        }
        list.finish()
    }
}

impl PartialEq for ResolvedModules {
    fn eq(&self, other: &Self) -> bool {
        self.picked == other.picked
    }
}
