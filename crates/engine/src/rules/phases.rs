//! The phase machine (named mechanism): every phase of play has a name, and every change of
//! phase is one row of a declared table, with the Law or engine rule that allows it.
//!
//! The referee stores the phase as three values (`Phase`: live, a dead ball, full time), and
//! the gate hashes and the snapshot saves exactly those. The machine names the phase on top
//! of them: before kick-off, open play, a dead ball of each restart kind (corners, free
//! kicks and penalties are the set pieces), half-time (every break between periods), a
//! shoot-out kick being set and a shoot-out kick in flight, and full time. A stoppage is not
//! a phase of its own: it is the cause on the entry to a dead ball, because play never stays
//! stopped across a tick without a restart waiting.
//!
//! One writer changes the phase (`Simulation::enter_phase`). It names the target, checks the
//! row in debug builds, in the engine's tests and in debug mode, and records the change in
//! the engine's tests. The named phase is derived state: at every tick boundary it equals
//! [`derive`] of the stored phase and the shoot-out, so it is neither hashed nor saved.

use std::fmt;

use crate::data::rules::StoppageKind;

use super::Phase;

/// The name of a phase of play.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhaseName {
    /// The players are not yet placed for the opening kick-off.
    PreMatch,
    OpenPlay,
    /// The ball is dead and waits for a restart of this kind.
    DeadBall(StoppageKind),
    /// A break between two periods: half-time, or a break before or inside extra time.
    HalfTime,
    /// A kick of the penalty shoot-out is set at the mark.
    ShootoutKick,
    /// A kick of the penalty shoot-out is in flight.
    ShootoutLive,
    FullTime,
}

impl PhaseName {
    /// `true` for a set piece: a corner, a free kick, or a penalty.
    pub fn is_set_piece(self) -> bool {
        matches!(
            self,
            PhaseName::DeadBall(
                StoppageKind::Corner | StoppageKind::FreeKick | StoppageKind::Penalty
            )
        )
    }
}

impl fmt::Display for PhaseName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PhaseName::PreMatch => f.write_str("before kick-off"),
            PhaseName::OpenPlay => f.write_str("open play"),
            PhaseName::DeadBall(kind) => write!(f, "dead ball ({})", words(*kind)),
            PhaseName::HalfTime => f.write_str("half-time"),
            PhaseName::ShootoutKick => f.write_str("shoot-out kick set"),
            PhaseName::ShootoutLive => f.write_str("shoot-out kick in flight"),
            PhaseName::FullTime => f.write_str("full time"),
        }
    }
}

/// `kind` in words: its data-file code with spaces.
fn words(kind: StoppageKind) -> String {
    kind.code().replace('_', " ")
}

/// Why the phase changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cause {
    /// A period starts: the kick-off is placed and the ball is in play.
    KickOffTaken,
    /// The ball went dead; the kind is what stopped play (a goal, a throw-in, a goal kick or a
    /// corner after the ball left the pitch, a free kick or a penalty after a foul or an
    /// offside, or an injury).
    BallDead(StoppageKind),
    /// The taker restarted play.
    RestartTaken,
    /// The taker of the waiting restart changed: he was injured or substituted.
    TakerRenamed,
    /// A period's own time and its added time have passed and a break follows.
    PeriodEnd,
    /// The match is over: its last period ended, or a caller that drives the steps ended it.
    MatchEnd,
    /// A team fell below the rule pack's minimum, or the shoot-out passed its round limit.
    Abandoned,
    /// A level knockout match goes to the shoot-out; its first kick is set.
    ShootoutStart,
    /// The kicker struck a shoot-out kick.
    ShootoutKickTaken,
    /// The kick in progress ended and the next shoot-out kick is set.
    NextShootoutKick,
    /// The shoot-out is decided.
    ShootoutDecided,
}

impl fmt::Display for Cause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Cause::KickOffTaken => f.write_str("kick-off taken"),
            Cause::BallDead(kind) => write!(f, "ball dead: {}", words(*kind)),
            Cause::RestartTaken => f.write_str("restart taken"),
            Cause::TakerRenamed => f.write_str("taker renamed"),
            Cause::PeriodEnd => f.write_str("period end"),
            Cause::MatchEnd => f.write_str("match end"),
            Cause::Abandoned => f.write_str("abandoned"),
            Cause::ShootoutStart => f.write_str("shoot-out start"),
            Cause::ShootoutKickTaken => f.write_str("shoot-out kick taken"),
            Cause::NextShootoutKick => f.write_str("next shoot-out kick set"),
            Cause::ShootoutDecided => f.write_str("shoot-out decided"),
        }
    }
}

/// A group of phases one row of the table names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Group {
    Is(PhaseName),
    /// A dead ball of any restart kind.
    AnyDeadBall,
    /// As a target only: the same dead ball the transition starts from.
    SameDeadBall,
}

impl Group {
    fn holds(self, name: PhaseName, from: PhaseName) -> bool {
        match self {
            Group::Is(n) => n == name,
            Group::AnyDeadBall => matches!(name, PhaseName::DeadBall(k) if RESTARTS.contains(&k)),
            Group::SameDeadBall => matches!(name, PhaseName::DeadBall(_)) && name == from,
        }
    }
}

/// The kind of a cause, without its detail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CauseKind {
    KickOffTaken,
    BallDead,
    RestartTaken,
    TakerRenamed,
    PeriodEnd,
    MatchEnd,
    Abandoned,
    ShootoutStart,
    ShootoutKickTaken,
    NextShootoutKick,
    ShootoutDecided,
}

impl Cause {
    pub fn kind(self) -> CauseKind {
        match self {
            Cause::KickOffTaken => CauseKind::KickOffTaken,
            Cause::BallDead(_) => CauseKind::BallDead,
            Cause::RestartTaken => CauseKind::RestartTaken,
            Cause::TakerRenamed => CauseKind::TakerRenamed,
            Cause::PeriodEnd => CauseKind::PeriodEnd,
            Cause::MatchEnd => CauseKind::MatchEnd,
            Cause::Abandoned => CauseKind::Abandoned,
            Cause::ShootoutStart => CauseKind::ShootoutStart,
            Cause::ShootoutKickTaken => CauseKind::ShootoutKickTaken,
            Cause::NextShootoutKick => CauseKind::NextShootoutKick,
            Cause::ShootoutDecided => CauseKind::ShootoutDecided,
        }
    }
}

/// The restart kinds a dead ball waits for (IFAB Laws 8 and 13 to 17): a goal and half-time
/// are causes, never restarts.
const RESTARTS: [StoppageKind; 7] = [
    StoppageKind::KickOff,
    StoppageKind::ThrowIn,
    StoppageKind::Corner,
    StoppageKind::GoalKick,
    StoppageKind::FreeKick,
    StoppageKind::Penalty,
    StoppageKind::Injury,
];

/// What may stop play, by restart: the restart a stoppage cause leads to.
fn restart_after(cause: StoppageKind) -> &'static [StoppageKind] {
    match cause {
        // A goal is restarted by a kick-off (Law 8).
        StoppageKind::Goal => &[StoppageKind::KickOff],
        // An injury in open play is restarted by a dropped ball (Law 8).
        StoppageKind::Injury => &[StoppageKind::Injury],
        // The ball leaving the pitch, a foul, and an offside are restarted by their own kind
        // (Laws 11, 12, 15, 16 and 17).
        StoppageKind::ThrowIn => &[StoppageKind::ThrowIn],
        StoppageKind::GoalKick => &[StoppageKind::GoalKick],
        StoppageKind::Corner => &[StoppageKind::Corner],
        StoppageKind::FreeKick => &[StoppageKind::FreeKick],
        StoppageKind::Penalty => &[StoppageKind::Penalty],
        StoppageKind::KickOff | StoppageKind::HalfTime => &[],
    }
}

/// One declared transition.
#[derive(Debug, Clone, Copy)]
pub struct Row {
    pub from: Group,
    pub to: Group,
    pub cause: CauseKind,
    /// The Law or engine rule that allows it.
    pub rule: &'static str,
}

const fn row(from: Group, to: Group, cause: CauseKind, rule: &'static str) -> Row {
    Row {
        from,
        to,
        cause,
        rule,
    }
}

use CauseKind as C;
use Group::{AnyDeadBall, Is, SameDeadBall};
use PhaseName as P;

/// Every transition the machine allows. A shoot-out kick is decided only once it is in
/// flight, nobody is injured or substituted during the shoot-out, and a held card is shown
/// before the next kick is set, so a set shoot-out kick leaves only for its flight.
pub const TRANSITIONS: &[Row] = &[
    row(
        Is(P::PreMatch),
        Is(P::OpenPlay),
        C::KickOffTaken,
        "Law 8: the kick-off starts the match",
    ),
    row(
        Is(P::HalfTime),
        Is(P::OpenPlay),
        C::KickOffTaken,
        "Law 8: a kick-off starts each period",
    ),
    row(
        Is(P::OpenPlay),
        AnyDeadBall,
        C::BallDead,
        "Laws 8 to 17: the ball goes dead and a restart waits",
    ),
    row(
        AnyDeadBall,
        Is(P::OpenPlay),
        C::RestartTaken,
        "Laws 8 to 17: the restart puts the ball in play",
    ),
    row(
        AnyDeadBall,
        SameDeadBall,
        C::TakerRenamed,
        "engine rule: an injured or substituted taker is replaced",
    ),
    row(
        Is(P::OpenPlay),
        Is(P::HalfTime),
        C::PeriodEnd,
        "Law 7: the period ends",
    ),
    row(
        AnyDeadBall,
        Is(P::HalfTime),
        C::PeriodEnd,
        "Law 7: the period ends while the ball is dead",
    ),
    row(
        Is(P::OpenPlay),
        Is(P::FullTime),
        C::MatchEnd,
        "Law 7: the match ends",
    ),
    row(
        AnyDeadBall,
        Is(P::FullTime),
        C::MatchEnd,
        "Law 7: the match ends while the ball is dead",
    ),
    row(
        Is(P::FullTime),
        Is(P::FullTime),
        C::MatchEnd,
        "engine rule: full time is recorded after the last period ends",
    ),
    row(
        Is(P::OpenPlay),
        Is(P::FullTime),
        C::Abandoned,
        "Law 3: a team below the minimum cannot continue",
    ),
    row(
        AnyDeadBall,
        Is(P::FullTime),
        C::Abandoned,
        "Law 3: a team below the minimum cannot continue",
    ),
    row(
        Is(P::HalfTime),
        Is(P::FullTime),
        C::Abandoned,
        "Law 3: a card held for the break leaves a team below the minimum",
    ),
    row(
        Is(P::OpenPlay),
        Is(P::ShootoutKick),
        C::ShootoutStart,
        "Law 10: a level knockout match goes to kicks from the penalty mark",
    ),
    row(
        AnyDeadBall,
        Is(P::ShootoutKick),
        C::ShootoutStart,
        "Law 10: a level knockout match goes to kicks from the penalty mark",
    ),
    row(
        Is(P::ShootoutKick),
        Is(P::ShootoutLive),
        C::ShootoutKickTaken,
        "Law 10: the kicker strikes the ball",
    ),
    row(
        Is(P::ShootoutLive),
        Is(P::ShootoutKick),
        C::NextShootoutKick,
        "Law 10: the next kick is set",
    ),
    row(
        Is(P::ShootoutLive),
        Is(P::FullTime),
        C::ShootoutDecided,
        "Law 10: the shoot-out is decided",
    ),
    row(
        Is(P::ShootoutLive),
        Is(P::FullTime),
        C::Abandoned,
        "engine rule: the shoot-out passed its round limit, or a held card left a team below the minimum",
    ),
];

/// The index in [`TRANSITIONS`] of the row that allows `from` to `to` for `cause`, or `None`
/// when no row does. A dead ball entered by `BallDead` must be the restart its cause leads to.
pub fn declared(from: PhaseName, to: PhaseName, cause: Cause) -> Option<usize> {
    if let (Cause::BallDead(by), PhaseName::DeadBall(kind)) = (cause, to)
        && !restart_after(by).contains(&kind)
    {
        return None;
    }
    TRANSITIONS
        .iter()
        .position(|r| r.cause == cause.kind() && r.from.holds(from, from) && r.to.holds(to, from))
}

/// The name of the stored phase `phase` while a shoot-out is (`shootout`) or is not in
/// progress.
pub fn derive(phase: Phase, shootout: bool) -> PhaseName {
    match (phase, shootout) {
        (Phase::FullTime, _) => PhaseName::FullTime,
        (Phase::Live, false) => PhaseName::OpenPlay,
        (Phase::Live, true) => PhaseName::ShootoutLive,
        (Phase::DeadBall(dead), false) => PhaseName::DeadBall(dead.kind),
        (Phase::DeadBall(_), true) => PhaseName::ShootoutKick,
    }
}

/// One recorded change of phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Step {
    /// The tick the change belongs to: the tick the step produces.
    pub tick: u32,
    pub from: PhaseName,
    pub to: PhaseName,
    pub cause: Cause,
}

impl fmt::Display for Step {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "tick {}: {} -> {} ({})",
            self.tick, self.from, self.to, self.cause
        )
    }
}

/// Every step of `log` that no row of the table allows, one line each.
pub fn undeclared(log: &[Step]) -> Vec<String> {
    log.iter()
        .filter(|s| declared(s.from, s.to, s.cause).is_none())
        .map(|s| format!("undeclared phase transition: {s}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::DVec2;
    use crate::rules::DeadBall;

    fn dead(kind: StoppageKind) -> Phase {
        Phase::DeadBall(DeadBall {
            kind,
            team: 0,
            spot: DVec2::ZERO,
            direct: true,
            since: 0,
            ready_at: 0,
            taker: 0,
        })
    }

    #[test]
    fn the_stored_phase_names_one_phase() {
        assert_eq!(derive(Phase::Live, false), PhaseName::OpenPlay);
        assert_eq!(derive(Phase::Live, true), PhaseName::ShootoutLive);
        assert_eq!(
            derive(dead(StoppageKind::Corner), false),
            PhaseName::DeadBall(StoppageKind::Corner)
        );
        assert_eq!(
            derive(dead(StoppageKind::Penalty), true),
            PhaseName::ShootoutKick
        );
        assert_eq!(derive(Phase::FullTime, true), PhaseName::FullTime);
    }

    #[test]
    fn the_set_pieces_are_corners_free_kicks_and_penalties() {
        let set: Vec<_> = RESTARTS
            .iter()
            .filter(|k| PhaseName::DeadBall(**k).is_set_piece())
            .collect();
        assert_eq!(
            set,
            [
                &StoppageKind::Corner,
                &StoppageKind::FreeKick,
                &StoppageKind::Penalty
            ]
        );
    }

    #[test]
    fn a_dead_ball_must_be_the_restart_its_cause_leads_to() {
        let goal = Cause::BallDead(StoppageKind::Goal);
        let kick_off = PhaseName::DeadBall(StoppageKind::KickOff);
        assert!(declared(PhaseName::OpenPlay, kick_off, goal).is_some());
        let throw_in = PhaseName::DeadBall(StoppageKind::ThrowIn);
        assert!(declared(PhaseName::OpenPlay, throw_in, goal).is_none());
        // Half-time is a phase of its own, never a dead ball.
        let half = PhaseName::DeadBall(StoppageKind::HalfTime);
        assert!(
            declared(
                PhaseName::OpenPlay,
                half,
                Cause::BallDead(StoppageKind::HalfTime)
            )
            .is_none()
        );
    }

    #[test]
    fn a_renamed_taker_keeps_the_same_dead_ball() {
        let corner = PhaseName::DeadBall(StoppageKind::Corner);
        let free_kick = PhaseName::DeadBall(StoppageKind::FreeKick);
        assert!(declared(corner, corner, Cause::TakerRenamed).is_some());
        assert!(declared(corner, free_kick, Cause::TakerRenamed).is_none());
    }

    #[test]
    fn full_time_never_returns_to_play() {
        assert!(
            declared(
                PhaseName::FullTime,
                PhaseName::OpenPlay,
                Cause::KickOffTaken
            )
            .is_none()
        );
        let planted = [Step {
            tick: 9,
            from: PhaseName::FullTime,
            to: PhaseName::OpenPlay,
            cause: Cause::KickOffTaken,
        }];
        assert_eq!(
            undeclared(&planted),
            ["undeclared phase transition: tick 9: full time -> open play (kick-off taken)"]
        );
    }
}
