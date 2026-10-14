//! The hook list: every place where play reads a team's plan. A tactical primitive (a
//! setting of the tactics file) reaches play only through the plan inputs it sets, and each
//! plan input is read at exactly one hook named here. The list is the fixed interface of the
//! choice model: a new choice model reads the same hooks, so a primitive and its tests keep
//! their meaning when the scoring inside a hook changes.
//!
//! Each hook names the [`crate::tactics::TeamPlan`] field it reads (its plan input), the kind
//! of read, and the code that reads it. A source test checks that every plan field a decision
//! site reads is the input of a hook in [`HOOKS`].

/// How a hook uses its plan input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookKind {
    /// A weight on an option's score.
    OptionWeight,
    /// A limit that switches a behaviour on or off.
    Threshold,
    /// A condition that starts a behaviour.
    Trigger,
    /// A time span inside which a behaviour holds.
    Window,
    /// An offset of where players stand.
    ShapeOffset,
}

/// One hook: its name, its kind, the plan input it reads and where.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hook {
    /// The name primitives use, `<area>.<what>`.
    pub name: &'static str,
    pub kind: HookKind,
    /// The [`crate::tactics::TeamPlan`] field the hook reads.
    pub input: &'static str,
    /// The code that reads the input.
    pub site: &'static str,
}

/// Every hook, in a fixed order.
pub const HOOKS: [Hook; 36] = [
    Hook {
        name: "pass.recycle",
        kind: HookKind::OptionWeight,
        input: "patience",
        site: "decision.rs: the pass score of a short pass that does not go forward",
    },
    Hook {
        name: "option.loss_danger",
        kind: HookKind::OptionWeight,
        input: "loss_weight",
        site: "decision.rs: a pass, dribble or hold loses loss weight x failure chance x danger",
    },
    Hook {
        name: "pass.directness",
        kind: HookKind::OptionWeight,
        input: "directness",
        site: "decision.rs: the pass score in proportion to the pass's gain toward goal",
    },
    Hook {
        name: "shape.block_depth",
        kind: HookKind::ShapeOffset,
        input: "block_depth",
        site: "team.rs: every anchor out of possession moves up the pitch",
    },
    Hook {
        name: "shape.width",
        kind: HookKind::ShapeOffset,
        input: "width",
        site: "team.rs: a scale on every in-possession anchor's distance from the centre line; \
               decision.rs: the score of a pass to a team-mate in a wide channel (second input)",
    },
    Hook {
        name: "shape.rest_defence",
        kind: HookKind::Threshold,
        input: "rest_defence",
        site: "sim/movement.rs, shape.rs: the fewest outfield players kept goal-side",
    },
    Hook {
        name: "shape.rest_guard",
        kind: HookKind::Threshold,
        input: "rest",
        site: "shape.rs: how the rest-defence guard holds players back",
    },
    Hook {
        name: "shape.stand_options",
        kind: HookKind::ShapeOffset,
        input: "sizes",
        site: "shape.rs: the sizes of the stand options",
    },
    Hook {
        name: "shape.slots",
        kind: HookKind::ShapeOffset,
        input: "slots",
        site: "shape.rs, decision.rs: each slot's stand, role and duty offsets",
    },
    Hook {
        name: "shape.back_line_gap",
        kind: HookKind::Threshold,
        input: "back_line_gap",
        site: "decision.rs, team.rs: the widest gap between two back-line neighbours",
    },
    Hook {
        name: "shape.organise_back_line",
        kind: HookKind::ShapeOffset,
        input: "organise_back_line",
        site: "team.rs: the back line's share of the line's organisation",
    },
    Hook {
        name: "press.count",
        kind: HookKind::Trigger,
        input: "press_count",
        site: "team.rs: how many players press the carrier",
    },
    Hook {
        name: "press.distance",
        kind: HookKind::Threshold,
        input: "press_distance",
        site: "decision.rs: how far from the carrier a player starts to press",
    },
    Hook {
        name: "restart.time_wasting",
        kind: HookKind::Window,
        input: "time_wasting",
        site: "rules/restart.rs, sim/pace.rs: the drawn restart wait while leading",
    },
    Hook {
        name: "pace.lift",
        kind: HookKind::Trigger,
        input: "pace_lift",
        site: "sim/pace.rs: pressing jobs and support jobs rise one urgency grade",
    },
    Hook {
        name: "trait.conflict",
        kind: HookKind::Trigger,
        input: "held",
        site: "sim/overrides.rs: a trait of play against the plan's held settings",
    },
    Hook {
        name: "trait.duty_depth.attack",
        kind: HookKind::ShapeOffset,
        input: "attack_depth",
        site: "sim/overrides.rs: an override that plays a player at the attack duty's depth",
    },
    Hook {
        name: "trait.duty_depth.defend",
        kind: HookKind::ShapeOffset,
        input: "defend_depth",
        site: "sim/overrides.rs: an override that plays a player at the defend duty's depth",
    },
    Hook {
        name: "keeper.chase_range",
        kind: HookKind::Threshold,
        input: "keeper_rush_off",
        site: "decision.rs: the keeper's chase range under the line height",
    },
    Hook {
        name: "pass.build_up",
        kind: HookKind::OptionWeight,
        input: "build_up",
        site: "decision.rs: a long pass out of the own third (the keeper's too) and a goal \
               kick's pick and reach, by its length",
    },
    Hook {
        name: "pass.channel.left",
        kind: HookKind::OptionWeight,
        input: "channel_left",
        site: "decision.rs: a pass into the side's left channel of the middle or final third",
    },
    Hook {
        name: "pass.channel.centre",
        kind: HookKind::OptionWeight,
        input: "channel_centre",
        site: "decision.rs: a pass into the centre channel of the middle or final third",
    },
    Hook {
        name: "pass.channel.right",
        kind: HookKind::OptionWeight,
        input: "channel_right",
        site: "decision.rs: a pass into the side's right channel of the middle or final third",
    },
    Hook {
        name: "shot.outside_box",
        kind: HookKind::OptionWeight,
        input: "long_shot",
        site: "decision.rs: the shot score from outside the penalty area",
    },
    Hook {
        name: "shot.range",
        kind: HookKind::Threshold,
        input: "shot_reach_m",
        site: "decision.rs: how far from goal a carrier considers a shot",
    },
    Hook {
        name: "dribble.weight",
        kind: HookKind::OptionWeight,
        input: "dribble",
        site: "decision.rs: the dribble score",
    },
    Hook {
        name: "pass.cross",
        kind: HookKind::OptionWeight,
        input: "crossing",
        site: "decision.rs: the score of a cross",
    },
    Hook {
        name: "pass.cross_early",
        kind: HookKind::OptionWeight,
        input: "early_cross",
        site: "decision.rs: the score of a cross from deep, before the byline",
    },
    Hook {
        name: "cross.trajectory",
        kind: HookKind::Trigger,
        input: "cross_loft",
        site: "decision.rs: a cross kicked driven along the ground or floated",
    },
    Hook {
        name: "option.noise",
        kind: HookKind::OptionWeight,
        input: "freedom",
        site: "decision.rs: a factor on the noise of every option's score",
    },
    Hook {
        name: "run.start",
        kind: HookKind::Threshold,
        input: "run_start_m",
        site: "decision.rs: how far from the defensive line a forward may start a timed run, in \
               behind above 0 or coming short below 0",
    },
    Hook {
        name: "shape.fullback_run",
        kind: HookKind::Trigger,
        input: "fullback_run",
        site: "shape.rs: the overlap or underlap first in a full-back's menu, lane or not",
    },
    Hook {
        name: "shape.fullback_reach",
        kind: HookKind::Threshold,
        input: "fullback_reach",
        site: "shape.rs: how far ahead and how near the ball a full-back's run may start",
    },
    Hook {
        name: "regain.counter",
        kind: HookKind::OptionWeight,
        input: "counter",
        site: "decision.rs: while the side's regain window is open, a pass's gain toward goal \
               scores more, the hold less, and attack-line runners may start a run in behind \
               farther from the line",
    },
    Hook {
        name: "loss.counter_press",
        kind: HookKind::Threshold,
        input: "counter_press",
        site: "decision.rs: while the side's counter-press window is open, a factor on how far \
               from the carrier its nearest players start to press",
    },
    Hook {
        name: "loss.counter_press_window",
        kind: HookKind::Window,
        input: "counter_press_s",
        site: "decision.rs: how many seconds after an open-play loss the counter-press window \
               stays open",
    },
];

/// Hooks that read a second plan input beside their first: the keeper's chase range reads
/// both line-height switches, and width spreads the shape and weighs a pass to a wide
/// team-mate.
pub const SECOND_INPUTS: [(&str, &str); 2] = [
    ("keeper.chase_range", "keeper_stay_off"),
    ("shape.width", "wide_target"),
];

/// The hook named `name`.
pub fn hook(name: &str) -> Option<&'static Hook> {
    HOOKS.iter().find(|h| h.name == name)
}

/// Every plan input some hook reads.
pub fn inputs() -> impl Iterator<Item = &'static str> {
    HOOKS
        .iter()
        .map(|h| h.input)
        .chain(SECOND_INPUTS.iter().map(|(_, i)| *i))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hook_names_are_unique_and_each_input_has_one_hook() {
        for (i, h) in HOOKS.iter().enumerate() {
            assert!(HOOKS[..i].iter().all(|o| o.name != h.name), "{}", h.name);
            assert!(HOOKS[..i].iter().all(|o| o.input != h.input), "{}", h.input);
        }
        for (name, input) in SECOND_INPUTS {
            assert!(hook(name).is_some());
            assert!(HOOKS.iter().all(|h| h.input != input));
        }
    }
}
