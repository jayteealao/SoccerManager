//! Game-state rules and player instructions: the parts of a side's tactic besides its base
//! tactics. A rule says "when ALL these conditions hold, set these primitives, the mentality
//! or the formation"; a side holds up to the tactics file's `game_state.max_rules`, in
//! priority order (the first wins a primitive two active rules set). A player instruction
//! sets one primitive of the override list for one formation slot.
//!
//! Everything here is pure: the situation a rule reads, the set of rules that hold, and the
//! tactics in force (the base with the active rules applied). The match loop evaluates the
//! rules each tick and queues a change when the active set moves
//! ([`crate::tactics::change`]).

use crate::data::primitives::settings;
use crate::data::tactics::{INSTRUCTIONS, TacticsSchema};
use crate::tactics::Tactics;
use crate::team::PLAYERS_PER_TEAM;

/// The most rules a side's active set can hold: the bits of one byte. The tactics file's
/// `game_state.max_rules` may set a lower limit.
pub const MAX_RULES: usize = 8;

/// The condition kinds a rule may test, as the tactics file and a rule name them.
pub const CONDITION_KINDS: [&str; 6] = [
    "score",
    "time_left",
    "red_cards_ours",
    "red_cards_theirs",
    "run_of_play",
    "team_energy",
];

/// The score a rule tests, from the side's own view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Score {
    /// Leading by this many goals or more.
    Leading(u8),
    /// Level.
    Level,
    /// Trailing by this many goals or more.
    Trailing(u8),
}

/// One condition of a rule.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Condition {
    Score(Score),
    /// This many minutes or less left to the end of the regulation time of the match (of
    /// extra time, once it has started); added time is not counted.
    TimeLeft {
        minutes: f64,
    },
    /// At least this many of the side's own players sent off.
    RedCardsOurs {
        at_least: u8,
    },
    /// At least this many of the other side's players sent off.
    RedCardsTheirs {
        at_least: u8,
    },
    /// The run of play: the expected goals of the side's shots less the other side's, over
    /// the window of the tactics file's `run_of_play_window_min` minutes, above (`above`) or
    /// below the value.
    RunOfPlay {
        above: bool,
        value: f64,
    },
    /// The mean energy of the side's outfield players on the pitch below the value (0 to 1).
    TeamEnergy {
        below: f64,
    },
}

impl Condition {
    /// The condition's kind, one of [`CONDITION_KINDS`].
    pub fn kind(&self) -> &'static str {
        match self {
            Condition::Score(_) => "score",
            Condition::TimeLeft { .. } => "time_left",
            Condition::RedCardsOurs { .. } => "red_cards_ours",
            Condition::RedCardsTheirs { .. } => "red_cards_theirs",
            Condition::RunOfPlay { .. } => "run_of_play",
            Condition::TeamEnergy { .. } => "team_energy",
        }
    }

    /// `true` when the condition holds in situation `s`.
    pub fn holds(&self, s: &Situation) -> bool {
        match *self {
            Condition::Score(Score::Leading(n)) => s.goal_difference >= i32::from(n),
            Condition::Score(Score::Level) => s.goal_difference == 0,
            Condition::Score(Score::Trailing(n)) => s.goal_difference <= -i32::from(n),
            Condition::TimeLeft { minutes } => s.minutes_left <= minutes,
            Condition::RedCardsOurs { at_least } => s.sent_off[0] >= at_least,
            Condition::RedCardsTheirs { at_least } => s.sent_off[1] >= at_least,
            Condition::RunOfPlay { above: true, value } => s.run_of_play > value,
            Condition::RunOfPlay {
                above: false,
                value,
            } => s.run_of_play < value,
            Condition::TeamEnergy { below } => s.energy < below,
        }
    }
}

// A condition's values come from the range-checked written form (or a range-checked change),
// so they are finite and compare as equal to themselves.
impl Eq for Condition {}

/// What a rule sets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Primitive `primitive` (an index into the tactics file's primitives) at level, choice
    /// or state `level`. The mentality is a primitive.
    Level { primitive: u8, level: u8 },
    /// Formation `0` (an index into the tactics file's formations); it waits for a stoppage.
    Formation(u8),
}

/// One rule: when every condition holds, every action applies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule {
    pub when: Vec<Condition>,
    pub set: Vec<Action>,
}

impl Rule {
    /// `true` when every condition holds in `s`.
    pub fn holds(&self, s: &Situation) -> bool {
        self.when.iter().all(|c| c.holds(s))
    }

    /// The formation the rule sets, if any.
    pub fn formation(&self) -> Option<u8> {
        self.set.iter().find_map(|a| match a {
            Action::Formation(f) => Some(*f),
            Action::Level { .. } => None,
        })
    }
}

/// One player instruction: primitive `primitive` (an index into the tactics file's
/// primitives) at level or choice `level` for the player in formation slot `slot`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerInstruction {
    pub slot: u8,
    pub primitive: u8,
    pub level: u8,
}

/// A side's rules, in priority order, and its player instructions.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GameState {
    pub rules: Vec<Rule>,
    pub instructions: Vec<PlayerInstruction>,
}

impl GameState {
    /// `true` when the side has no rule and no instruction.
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty() && self.instructions.is_empty()
    }

    /// The instructions of the player in `slot`, in the order given.
    pub fn instructions_for(&self, slot: usize) -> impl Iterator<Item = &PlayerInstruction> {
        self.instructions
            .iter()
            .filter(move |i| usize::from(i.slot) == slot)
    }

    /// `true` when some instruction names `slot`.
    pub fn instructs(&self, slot: usize) -> bool {
        self.instructions_for(slot).next().is_some()
    }

    /// The bits of the rules that hold in `s`: bit `k` for rule `k`.
    pub fn active(&self, s: &Situation) -> u8 {
        self.rules
            .iter()
            .take(MAX_RULES)
            .enumerate()
            .filter(|(_, r)| r.holds(s))
            .fold(0, |bits, (k, _)| bits | (1 << k))
    }
}

/// What a side's rules read: the match from that side's view.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Situation {
    /// The side's goals less the other side's.
    pub goal_difference: i32,
    /// Minutes left to the end of the regulation time of the match, or of extra time once it
    /// has started; 0 in added time.
    pub minutes_left: f64,
    /// Players sent off: the side's own, then the other side's.
    pub sent_off: [u8; 2],
    /// Expected goals for less expected goals against over the run-of-play window.
    pub run_of_play: f64,
    /// The mean energy of the side's outfield players on the pitch, 0 to 1.
    pub energy: f64,
}

impl Tactics {
    /// These tactics with primitive `primitive` (an index into the tactics file's
    /// primitives) at level, choice or state `level`: a team instruction's level, the
    /// mentality, or the setting byte of a primitive that owns its levels or choices. A
    /// primitive whose level block the tactics do not store is left as it is.
    pub fn with_level(mut self, primitive: usize, level: u8, schema: &TacticsSchema) -> Self {
        let Some(p) = schema.primitives.get(primitive) else {
            return self;
        };
        if p.is_setting() {
            if let Some(k) = settings(schema).position(|s| s.name == p.name) {
                self.settings[k] = level;
            }
            return self;
        }
        match p.levels.as_deref() {
            Some("mentalities") => self.mentality = level,
            Some(block) => {
                if let Some(i) = block
                    .strip_prefix("instructions.")
                    .and_then(|name| INSTRUCTIONS.iter().position(|n| *n == name))
                {
                    self.instructions[i] = level;
                }
            }
            None => {}
        }
        self
    }

    /// The level, choice or state of primitive `primitive` in these tactics: the inverse of
    /// [`Tactics::with_level`]. `None` for a primitive the tactics do not store.
    pub fn level(&self, primitive: usize, schema: &TacticsSchema) -> Option<u8> {
        let p = schema.primitives.get(primitive)?;
        if p.is_setting() {
            let k = settings(schema).position(|s| s.name == p.name)?;
            return self.settings.get(k).copied();
        }
        match p.levels.as_deref() {
            Some("mentalities") => Some(self.mentality),
            Some(block) => block
                .strip_prefix("instructions.")
                .and_then(|name| INSTRUCTIONS.iter().position(|n| *n == name))
                .map(|i| self.instructions[i]),
            None => None,
        }
    }
}

/// The tactics in force for base tactics `base` with the rules of `rules` whose bit `active`
/// holds: each active rule applied from the lowest in the list to the highest, so the higher
/// rule wins a primitive or the formation two of them set.
pub fn effective(base: &Tactics, rules: &[Rule], active: u8, schema: &TacticsSchema) -> Tactics {
    let mut t = *base;
    for (k, rule) in rules.iter().enumerate().take(MAX_RULES).rev() {
        if active & (1 << k) == 0 {
            continue;
        }
        for action in &rule.set {
            t = match *action {
                Action::Level { primitive, level } => {
                    t.with_level(usize::from(primitive), level, schema)
                }
                Action::Formation(f) => {
                    let mut moved = t;
                    moved.set_formation(f, schema);
                    moved
                }
            };
        }
    }
    t
}

/// The tactics that apply at once for a rule change: the tactics in force after the change
/// (`target`) with the formation and roles of the tactics in force now (`now`), which wait
/// for a stoppage.
pub fn at_once(target: &Tactics, now: &Tactics) -> Tactics {
    Tactics {
        formation: now.formation,
        roles: now.roles,
        ..*target
    }
}

/// `true` when `a` and `b` differ in their formation or a slot's role or duty: the part of
/// a rule change that waits for a stoppage.
pub fn shape_differs(a: &Tactics, b: &Tactics) -> bool {
    a.formation != b.formation || (0..PLAYERS_PER_TEAM).any(|s| a.roles[s] != b.roles[s])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::test_support::shipped_content;

    fn situation() -> Situation {
        Situation {
            goal_difference: 0,
            minutes_left: 60.0,
            sent_off: [0, 0],
            run_of_play: 0.0,
            energy: 0.9,
        }
    }

    #[test]
    fn each_condition_holds_only_inside_its_bound() {
        let s = situation();
        let lead = Condition::Score(Score::Leading(1));
        assert!(!lead.holds(&s));
        assert!(lead.holds(&Situation {
            goal_difference: 2,
            ..s
        }));
        assert!(Condition::Score(Score::Level).holds(&s));
        let trail = Condition::Score(Score::Trailing(2));
        assert!(!trail.holds(&Situation {
            goal_difference: -1,
            ..s
        }));
        assert!(trail.holds(&Situation {
            goal_difference: -2,
            ..s
        }));
        let late = Condition::TimeLeft { minutes: 15.0 };
        assert!(!late.holds(&s));
        assert!(late.holds(&Situation {
            minutes_left: 15.0,
            ..s
        }));
        let ours = Condition::RedCardsOurs { at_least: 1 };
        let theirs = Condition::RedCardsTheirs { at_least: 1 };
        assert!(!ours.holds(&s) && !theirs.holds(&s));
        assert!(ours.holds(&Situation {
            sent_off: [1, 0],
            ..s
        }));
        assert!(theirs.holds(&Situation {
            sent_off: [0, 1],
            ..s
        }));
        let on_top = Condition::RunOfPlay {
            above: true,
            value: 0.5,
        };
        let under = Condition::RunOfPlay {
            above: false,
            value: -0.5,
        };
        assert!(!on_top.holds(&s) && !under.holds(&s));
        assert!(on_top.holds(&Situation {
            run_of_play: 0.6,
            ..s
        }));
        assert!(under.holds(&Situation {
            run_of_play: -0.6,
            ..s
        }));
        let tired = Condition::TeamEnergy { below: 0.6 };
        assert!(!tired.holds(&s));
        assert!(tired.holds(&Situation { energy: 0.55, ..s }));
    }

    #[test]
    fn a_rule_holds_only_when_all_its_conditions_hold() {
        let rule = Rule {
            when: vec![
                Condition::Score(Score::Leading(1)),
                Condition::TimeLeft { minutes: 15.0 },
            ],
            set: vec![Action::Level {
                primitive: 1,
                level: 1,
            }],
        };
        let s = situation();
        assert!(!rule.holds(&Situation {
            goal_difference: 1,
            ..s
        }));
        assert!(!rule.holds(&Situation {
            minutes_left: 10.0,
            ..s
        }));
        assert!(rule.holds(&Situation {
            goal_difference: 1,
            minutes_left: 10.0,
            ..s
        }));
    }

    #[test]
    fn the_higher_rule_wins_a_primitive_two_active_rules_set() {
        let content = shipped_content();
        let schema = &content.tactics;
        let mentality = schema
            .primitives
            .iter()
            .position(|p| p.name == "mentality")
            .unwrap() as u8;
        let rule = |level: u8| Rule {
            when: vec![Condition::Score(Score::Level)],
            set: vec![Action::Level {
                primitive: mentality,
                level,
            }],
        };
        let rules = vec![rule(0), rule(4)];
        let base = Tactics::defaults(schema);
        assert_eq!(effective(&base, &rules, 0b11, schema).mentality, 0);
        assert_eq!(effective(&base, &rules, 0b10, schema).mentality, 4);
        assert_eq!(effective(&base, &rules, 0, schema), base);
    }

    #[test]
    fn with_level_reaches_instructions_the_mentality_and_settings() {
        let content = shipped_content();
        let schema = &content.tactics;
        let base = Tactics::defaults(schema);
        let index = |name: &str| {
            schema
                .primitives
                .iter()
                .position(|p| p.name == name)
                .unwrap()
        };
        let t = base.with_level(index("pressing"), 4, schema);
        assert_eq!(t.instructions[crate::data::tactics::PRESSING], 4);
        let t = base.with_level(index("mentality"), 1, schema);
        assert_eq!(t.mentality, 1);
        let k = settings(schema)
            .position(|p| p.name == "dribbling")
            .unwrap();
        let t = base.with_level(index("dribbling"), 4, schema);
        assert_eq!(t.settings[k], 4);
        assert_eq!(
            Tactics {
                settings: base.settings,
                ..t
            },
            base
        );
    }
}
