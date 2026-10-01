//! Tactics changes: which changes a stoppage admits, the verdict on a substitution and where
//! the substitute enters, and the verdict on a tactics change (IFAB Law 3 and the rule
//! pack). The change queue in [`super::change`] applies each verdict.

use crate::data::rules::StoppageKind;
use crate::math::DVec2;
use crate::modules::{ChangesModule, MatchView, ModuleCard, SubEntry, SubRequest};
use crate::player::Status;
use crate::tactics::change::RejectReason;
use crate::tactics::{Tactics, TacticsPatch};
use crate::team::PLAYERS_PER_TEAM;

/// How far inside the touchline a substitute enters, in metres.
const ENTRY_MARGIN: f64 = 0.5;

/// Tactics changes version 1: the rule pack's admitting stoppages, limits, and windows.
pub struct ChangesV1;

impl ChangesModule for ChangesV1 {
    fn admits(&self, view: &MatchView<'_>, kind: StoppageKind) -> (bool, bool) {
        view.rules().admits(kind)
    }

    fn substitution(&self, view: &MatchView<'_>, r: &SubRequest) -> Result<SubEntry, RejectReason> {
        let (limit, windows) = view.substitution_limits();
        let rules = &view.rules().substitutions;
        let ledger = view.ledgers()[r.team];
        if ledger.used >= limit {
            return Err(RejectReason::LimitReached { limit });
        }
        let needs_window = !rules.exempt(r.kind) && ledger.window_at != Some(r.now);
        if needs_window && ledger.windows >= windows {
            return Err(RejectReason::NoWindowLeft { windows });
        }
        let side = &view.teams()[r.team];
        let slot = side
            .lineup
            .iter()
            .position(|&s| s == r.off)
            .ok_or(RejectReason::NotOnPitch { squad: r.off })?;
        let leaving = view.player(r.team * PLAYERS_PER_TEAM + slot);
        if leaving.status == Status::SentOff {
            return Err(RejectReason::SentOff { squad: r.off });
        }
        if !side.bench.contains(&r.on) {
            return Err(RejectReason::NotOnBench { squad: r.on });
        }
        // At half-time the substitute takes the leaving player's kick-off place; otherwise it
        // enters at the halfway line on the near touchline, on its own side.
        let (at, from_touchline) = if r.kind == StoppageKind::HalfTime && leaving.active() {
            (leaving.pos, false)
        } else {
            let k = r.entered as f64;
            (
                DVec2::new(
                    -side.attack_x * (1.0 + 1.5 * k),
                    -(view.pitch().half_width() - ENTRY_MARGIN),
                ),
                true,
            )
        };
        Ok(SubEntry {
            slot,
            at,
            needs_window,
            from_touchline,
        })
    }

    fn tactics(
        &self,
        view: &MatchView<'_>,
        team: usize,
        patch: &TacticsPatch,
        off_now: &[(usize, usize)],
    ) -> Result<Tactics, RejectReason> {
        let schema = view.tactics();
        if !patch.in_range(schema) {
            return Err(RejectReason::OutOfRange);
        }
        let side = &view.teams()[team];
        for (squad, _) in &patch.roles {
            if off_now.contains(&(team, *squad)) {
                return Err(RejectReason::LeftThePitch { squad: *squad });
            }
            let on_pitch = side
                .lineup
                .iter()
                .position(|s| s == squad)
                .is_some_and(|slot| view.player(team * PLAYERS_PER_TEAM + slot).active());
            if !on_pitch {
                return Err(RejectReason::NotOnPitch { squad: *squad });
            }
        }
        Ok(patch.applied_to(side.tactics, &side.lineup, schema))
    }
}

pub const CHANGES_V1_CARD: ModuleCard = ModuleCard {
    purpose: "Decides which changes a stoppage admits, whether a substitution or a tactics change applies or why not, and where a substitute enters.",
    inputs: "The rule pack, the substitution ledgers and limits, each team's lineup, bench, tactics, and attack direction, each player's status, activity, and position, and the tactics file.",
    outputs: "Whether a stoppage admits tactics changes and substitutions, the substitution's slot, entry point, and window use, the new tactics, or the reason for a rejection.",
    tuning: &["rules.substitutions", "rules.extra_time", "rules.stoppages"],
    calibration: "none: Law 3 change rules, no band",
    keys: &[],
};

/// The off version: no stoppage admits a change, so queued changes wait and expire at full
/// time. It is never asked for a verdict; if it were, it gives version 1's.
pub struct ChangesOff;

impl ChangesModule for ChangesOff {
    fn admits(&self, _: &MatchView<'_>, _: StoppageKind) -> (bool, bool) {
        (false, false)
    }

    fn substitution(&self, view: &MatchView<'_>, r: &SubRequest) -> Result<SubEntry, RejectReason> {
        ChangesV1.substitution(view, r)
    }

    fn tactics(
        &self,
        view: &MatchView<'_>,
        team: usize,
        patch: &TacticsPatch,
        off_now: &[(usize, usize)],
    ) -> Result<Tactics, RejectReason> {
        ChangesV1.tactics(view, team, patch, off_now)
    }
}

pub const CHANGES_OFF_CARD: ModuleCard = ModuleCard {
    purpose: "Admits no change at any stoppage.",
    inputs: "Nothing.",
    outputs: "No stoppage admits a change.",
    tuning: &["none"],
    calibration: "none: off version, no change applies",
    keys: &[],
};
