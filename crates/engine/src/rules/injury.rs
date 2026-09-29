//! Injuries (IFAB Law 8): whether an injury takes a player off, and the dropped ball that
//! stops open play for it. The fatigue module sets the chance of each injury roll and the
//! loop takes the rolls; this module decides what an injury does.

use crate::fatigue::InjurySource;
use crate::math::DVec2;
use crate::modules::{InjuriesModule, MatchView, ModuleCard};
use crate::pitch;

/// Injuries, version 1: an injured player on the pitch leaves at once, and in open play the
/// ball is dropped at its spot for the team that touched it last (the injured player's team
/// when nobody has).
pub struct InjuriesV1;

impl InjuriesModule for InjuriesV1 {
    fn leaves(&self, view: &MatchView<'_>, i: usize, _: InjurySource) -> bool {
        view.player(i).active()
    }

    fn dropped_ball(&self, view: &MatchView<'_>, team: usize) -> (usize, DVec2) {
        (
            view.last_touch().unwrap_or(team),
            pitch::clamp(view.ball().xy(), 0.5),
        )
    }
}

pub const INJURIES_V1_CARD: ModuleCard = ModuleCard {
    purpose: "Decides whether an injury takes a player off, and the dropped ball it causes in open play.",
    inputs: "The injured player's status, the team that touched the ball last, and the ball's position.",
    outputs: "Whether the player leaves, and the dropped ball's team and spot; the loop takes the player off and stops play.",
    tuning: &["none"],
    calibration: "none: no injury band in realism-bands.json",
    keys: &[],
};

/// Injuries switched off: no injury takes a player off. The injury rolls still draw on
/// their keys.
pub struct InjuriesOff;

impl InjuriesModule for InjuriesOff {
    fn leaves(&self, _: &MatchView<'_>, _: usize, _: InjurySource) -> bool {
        false
    }

    fn dropped_ball(&self, view: &MatchView<'_>, team: usize) -> (usize, DVec2) {
        InjuriesV1.dropped_ball(view, team)
    }
}

pub const INJURIES_OFF_CARD: ModuleCard = ModuleCard {
    purpose: "Injuries switched off: no injury takes a player off.",
    inputs: "Nothing.",
    outputs: "Never leaves; the dropped ball of version 1, which no injury reaches.",
    tuning: &["none"],
    calibration: "none: off version, no injury takes a player off",
    keys: &[],
};
