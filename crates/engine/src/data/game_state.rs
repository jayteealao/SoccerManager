//! The game-state block of the tactics file and the written form of a side's rules and
//! player instructions.
//!
//! The tactics file's `game_state` block sets the limits: the most rules a tactic holds, the
//! condition kinds with their ranges, the run-of-play window, the take-up time, and the
//! player override list (each primitive a player instruction may set, the plan inputs it sets
//! for the player, and the positions it reaches when it reaches only some).
//!
//! A side's rules and instructions are written by name (the same form in a style preset, a
//! match set-up, the hello and the replay record):
//!
//! ```json
//! {
//!   "rules": [
//!     { "when": [ { "kind": "score", "state": "leading", "by": 1 },
//!                 { "kind": "time_left", "minutes": 15 } ],
//!       "set":  [ { "primitive": "mentality", "level": "cautious" },
//!                 { "formation": "5-4-1" } ] }
//!   ],
//!   "instructions": [ { "slot": 9, "primitive": "final_third", "level": "shoot_early" } ]
//! }
//! ```
//!
//! [`resolve`] turns the written form into indices and refuses, by name: more rules than the
//! limit; a rule with no condition or no action; an unknown condition kind, primitive, level,
//! choice, formation or slot; one condition kind or one primitive twice in a rule; a field a
//! condition kind does not take, or a value outside its range; an instruction on a primitive
//! outside the override list (marking among them); two instructions on one slot and
//! primitive; and an instruction on a primitive whose entry names positions for a slot whose
//! position in a formation the tactic can take is not among them (forward runs reach only the
//! attack line).

use garde::Validate;
use serde::{Deserialize, Serialize};

use crate::data::tactics::{Refusal, TacticsSchema};
use crate::data::team::Position;
use crate::tactics::rules::{
    Action, CONDITION_KINDS, Condition, GameState, MAX_RULES, PlayerInstruction, Rule, Score,
};
use crate::team::PLAYERS_PER_TEAM;

/// The tactics file's game-state block.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct GameStateSchema {
    /// The most rules a tactic holds.
    #[garde(range(min = 1, max = 8))]
    pub max_rules: u8,
    /// The minutes of match time the run of play counts chances over.
    #[garde(range(min = 1, max = 45))]
    pub run_of_play_window_min: u32,
    /// Seconds a player at the bottom of the shape stage takes to take a rule's change up;
    /// a player at its top takes it up at once.
    #[garde(range(min = 0.0, max = 120.0))]
    pub take_up_s: f64,
    /// The condition kinds a rule may test, each with its unit and range.
    #[garde(length(min = 1, max = 6), dive)]
    pub conditions: Vec<ConditionKind>,
    /// The primitives a player instruction may set.
    #[garde(length(max = 16), dive)]
    pub player_overrides: Vec<PlayerOverride>,
}

/// One condition kind and the range of its value: goals for the score's `by`, minutes for
/// the time left, players for red cards, expected goals for the run of play, a share of
/// full energy for the team energy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct ConditionKind {
    #[garde(length(min = 1, max = 32))]
    pub kind: String,
    #[garde(length(min = 1, max = 16))]
    pub unit: String,
    #[garde(range(min = -10.0, max = 120.0))]
    pub min: f64,
    #[garde(range(min = -10.0, max = 120.0))]
    pub max: f64,
}

/// One primitive of the player override list: its name, the plan inputs an instruction
/// sets for the player (the others stay the side's), and the positions it reaches when it
/// reaches only some (empty: every position).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct PlayerOverride {
    #[garde(length(min = 1, max = 32))]
    pub primitive: String,
    #[garde(length(min = 1, max = 4))]
    pub inputs: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[garde(skip)]
    pub positions: Vec<Position>,
}

impl GameStateSchema {
    /// The checks that span the block and the tactics file: every condition kind known and
    /// listed once with its minimum at most its maximum; every override naming a primitive
    /// once, each input one the primitive sets and one a player's own plan carries
    /// ([`crate::tactics::hooks::PLAYER_INPUTS`]).
    pub fn check(&self, schema: &TacticsSchema) -> Result<(), Refusal> {
        if usize::from(self.max_rules) > MAX_RULES {
            return Err((
                "game_state.max_rules".into(),
                format!("{}; allowed 1 to {MAX_RULES}", self.max_rules),
            ));
        }
        for (i, c) in self.conditions.iter().enumerate() {
            let at = format!("game_state.conditions[{i}]");
            if !CONDITION_KINDS.contains(&c.kind.as_str()) {
                return Err((
                    at,
                    format!(
                        "{} is not a condition kind; the kinds are {}",
                        c.kind,
                        CONDITION_KINDS.join(", ")
                    ),
                ));
            }
            if self.conditions[..i].iter().any(|o| o.kind == c.kind) {
                return Err((at, format!("{} appears twice", c.kind)));
            }
            if c.min > c.max {
                return Err((at, format!("{}: min {} above max {}", c.kind, c.min, c.max)));
            }
        }
        for (i, o) in self.player_overrides.iter().enumerate() {
            let at = format!("game_state.player_overrides[{i}]");
            let Some(p) = schema.primitives.iter().find(|p| p.name == o.primitive) else {
                return Err((at, format!("{} is not a primitive", o.primitive)));
            };
            if self.player_overrides[..i]
                .iter()
                .any(|e| e.primitive == o.primitive)
            {
                return Err((at, format!("{} appears twice", o.primitive)));
            }
            for input in &o.inputs {
                if !p.inputs.iter().any(|ih| ih.input == *input) {
                    return Err((
                        format!("{at}.inputs"),
                        format!("{} does not set {input}", o.primitive),
                    ));
                }
                if !crate::tactics::hooks::PLAYER_INPUTS.contains(&input.as_str()) {
                    return Err((
                        format!("{at}.inputs"),
                        format!("{input} is read for the side as a whole, not per player"),
                    ));
                }
            }
        }
        Ok(())
    }

    /// The override entry of primitive `name`.
    pub fn override_of(&self, name: &str) -> Option<&PlayerOverride> {
        self.player_overrides.iter().find(|o| o.primitive == name)
    }

    /// The range of condition kind `kind`.
    pub fn condition(&self, kind: &str) -> Option<&ConditionKind> {
        self.conditions.iter().find(|c| c.kind == kind)
    }
}

/// A side's rules and instructions as written.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GameStateFile {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rules: Vec<RuleFile>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub instructions: Vec<InstructionFile>,
}

impl GameStateFile {
    /// `true` when the file holds no rule and no instruction.
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty() && self.instructions.is_empty()
    }
}

/// One rule as written.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuleFile {
    pub when: Vec<ConditionFile>,
    pub set: Vec<ActionFile>,
}

/// One condition as written: its kind and the fields that kind takes. `score` takes `state`
/// (`leading`, `level` or `trailing`) and, but for `level`, `by` (1 when absent);
/// `time_left` takes `minutes`; `red_cards_ours` and `red_cards_theirs` take `at_least` (1
/// when absent); `run_of_play` takes `above` or `below`; `team_energy` takes `below`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConditionFile {
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub by: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minutes: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at_least: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub above: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub below: Option<f64>,
}

/// One action as written: a primitive and its level, choice or state, or a formation.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActionFile {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub primitive: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub level: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub formation: Option<String>,
}

/// One player instruction as written: a formation slot (0 to 10), a primitive of the
/// override list and its level or choice.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InstructionFile {
    pub slot: u8,
    pub primitive: String,
    pub level: String,
}

/// Parses a written game state from JSON bytes; a refusal names the reason.
pub fn parse(bytes: &[u8]) -> Result<GameStateFile, Refusal> {
    serde_json::from_slice(bytes).map_err(|e| ("game_state".into(), e.to_string()))
}

/// Resolves `file` against the tactics file. `formations` are the formations the tactic
/// stands in (its base formation, or a style's formations); a rule's formation joins them for
/// the position check of an instruction.
pub fn resolve(
    file: &GameStateFile,
    schema: &TacticsSchema,
    formations: &[u8],
) -> Result<GameState, Refusal> {
    if file.is_empty() {
        return Ok(GameState::default());
    }
    let Some(block) = schema.game_state.as_ref() else {
        return Err((
            "game_state".into(),
            "the tactics file has no game_state block, so it takes no rule or instruction".into(),
        ));
    };
    if file.rules.len() > usize::from(block.max_rules) {
        return Err((
            "rules".into(),
            format!(
                "{} rules; a tactic holds at most {}",
                file.rules.len(),
                block.max_rules
            ),
        ));
    }
    let mut rules = Vec::with_capacity(file.rules.len());
    for (r, rule) in file.rules.iter().enumerate() {
        rules.push(resolve_rule(rule, r, block, schema)?);
    }
    let mut stands_in: Vec<u8> = formations.to_vec();
    stands_in.extend(rules.iter().filter_map(Rule::formation));
    let mut instructions = Vec::with_capacity(file.instructions.len());
    for (k, ins) in file.instructions.iter().enumerate() {
        let at = format!("instructions[{k}]");
        if usize::from(ins.slot) >= PLAYERS_PER_TEAM {
            return Err((
                format!("{at}.slot"),
                format!(
                    "slot {} is not a slot; slots run 0 to {}",
                    ins.slot,
                    PLAYERS_PER_TEAM - 1
                ),
            ));
        }
        let Some(entry) = block.override_of(&ins.primitive) else {
            let known = schema.primitives.iter().any(|p| p.name == ins.primitive);
            let reason = if known {
                format!(
                    "{} is not on the player override list; a player instruction may set {}",
                    ins.primitive,
                    block
                        .player_overrides
                        .iter()
                        .map(|o| o.primitive.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            } else {
                format!(
                    "{} is not a primitive and not on the player override list",
                    ins.primitive
                )
            };
            return Err((format!("{at}.primitive"), reason));
        };
        let (primitive, level) = level_of(schema, &ins.primitive, &ins.level, &at)?;
        if file.instructions[..k]
            .iter()
            .any(|o| o.slot == ins.slot && o.primitive == ins.primitive)
        {
            return Err((
                at,
                format!(
                    "slot {} has two instructions on {}",
                    ins.slot, ins.primitive
                ),
            ));
        }
        if !entry.positions.is_empty() {
            for &f in &stands_in {
                let Some(formation) = schema.formations.get(usize::from(f)) else {
                    continue;
                };
                let position = formation.slots[usize::from(ins.slot)].position;
                if !entry.positions.contains(&position) {
                    return Err((
                        format!("{at}.slot"),
                        format!(
                            "{} reaches only {}; slot {} is {} in {}",
                            ins.primitive,
                            entry
                                .positions
                                .iter()
                                .map(|p| p.code())
                                .collect::<Vec<_>>()
                                .join(", "),
                            ins.slot,
                            position.code(),
                            formation.name
                        ),
                    ));
                }
            }
        }
        instructions.push(PlayerInstruction {
            slot: ins.slot,
            primitive,
            level,
        });
    }
    Ok(GameState {
        rules,
        instructions,
    })
}

/// The index of primitive `name` and of its level, choice or state `level`.
fn level_of(
    schema: &TacticsSchema,
    name: &str,
    level: &str,
    at: &str,
) -> Result<(u8, u8), Refusal> {
    let Some(p) = schema.primitives.iter().position(|p| p.name == name) else {
        return Err((
            format!("{at}.primitive"),
            format!("{name} is not a primitive"),
        ));
    };
    let names = schema.primitives[p].level_names(schema);
    let Some(l) = names.iter().position(|n| *n == level) else {
        return Err((
            format!("{at}.level"),
            format!(
                "{level} is not a level of {name}; its levels are {}",
                names.join(", ")
            ),
        ));
    };
    Ok((p as u8, l as u8))
}

fn resolve_rule(
    rule: &RuleFile,
    r: usize,
    block: &GameStateSchema,
    schema: &TacticsSchema,
) -> Result<Rule, Refusal> {
    let at = format!("rules[{r}]");
    if rule.when.is_empty() {
        return Err((format!("{at}.when"), "a rule needs a condition".into()));
    }
    if rule.set.is_empty() {
        return Err((format!("{at}.set"), "a rule needs an action".into()));
    }
    let mut when = Vec::with_capacity(rule.when.len());
    for (c, cond) in rule.when.iter().enumerate() {
        let at = format!("{at}.when[{c}]");
        if rule.when[..c].iter().any(|o| o.kind == cond.kind) {
            return Err((at, format!("{} appears twice in the rule", cond.kind)));
        }
        when.push(resolve_condition(cond, &at, block)?);
    }
    let mut set = Vec::with_capacity(rule.set.len());
    for (a, action) in rule.set.iter().enumerate() {
        let at = format!("{at}.set[{a}]");
        let resolved = match (&action.primitive, &action.level, &action.formation) {
            (Some(name), Some(level), None) => {
                if rule.set[..a]
                    .iter()
                    .any(|o| o.primitive.as_deref() == Some(name))
                {
                    return Err((at, format!("{name} appears twice in the rule")));
                }
                let (primitive, level) = level_of(schema, name, level, &at)?;
                Action::Level { primitive, level }
            }
            (None, None, Some(name)) => {
                if rule.set[..a].iter().any(|o| o.formation.is_some()) {
                    return Err((at, "the formation appears twice in the rule".into()));
                }
                let f = schema.formation_index(name).ok_or_else(|| {
                    (
                        format!("{at}.formation"),
                        format!("{name} is not a formation"),
                    )
                })?;
                Action::Formation(f as u8)
            }
            _ => {
                return Err((
                    at,
                    "an action names a primitive and its level, or a formation".into(),
                ));
            }
        };
        set.push(resolved);
    }
    Ok(Rule { when, set })
}

fn resolve_condition(
    c: &ConditionFile,
    at: &str,
    block: &GameStateSchema,
) -> Result<Condition, Refusal> {
    let Some(range) = block.condition(&c.kind) else {
        return Err((
            format!("{at}.kind"),
            format!(
                "{} is not a condition kind; the kinds are {}",
                c.kind,
                block
                    .conditions
                    .iter()
                    .map(|k| k.kind.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        ));
    };
    let fields = [
        ("state", c.state.is_some()),
        ("by", c.by.is_some()),
        ("minutes", c.minutes.is_some()),
        ("at_least", c.at_least.is_some()),
        ("above", c.above.is_some()),
        ("below", c.below.is_some()),
    ];
    let takes: &[&str] = match c.kind.as_str() {
        "score" => &["state", "by"],
        "time_left" => &["minutes"],
        "red_cards_ours" | "red_cards_theirs" => &["at_least"],
        "run_of_play" => &["above", "below"],
        _ => &["below"],
    };
    if let Some((field, _)) = fields.iter().find(|(f, set)| *set && !takes.contains(f)) {
        return Err((
            format!("{at}.{field}"),
            format!("{} takes {}, not {field}", c.kind, takes.join(" or ")),
        ));
    }
    let within = |field: &str, v: f64| -> Result<f64, Refusal> {
        if (range.min..=range.max).contains(&v) {
            Ok(v)
        } else {
            Err((
                format!("{at}.{field}"),
                format!("{v} {}; allowed {} to {}", range.unit, range.min, range.max),
            ))
        }
    };
    let missing = |field: &str| (format!("{at}.{field}"), format!("{} needs {field}", c.kind));
    Ok(match c.kind.as_str() {
        "score" => {
            let state = c.state.as_deref().ok_or_else(|| missing("state"))?;
            match state {
                "level" => {
                    if c.by.is_some() {
                        return Err((format!("{at}.by"), "a level score takes no by".into()));
                    }
                    Condition::Score(Score::Level)
                }
                "leading" | "trailing" => {
                    let by = c.by.unwrap_or(1);
                    within("by", f64::from(by))?;
                    Condition::Score(if state == "leading" {
                        Score::Leading(by)
                    } else {
                        Score::Trailing(by)
                    })
                }
                other => {
                    return Err((
                        format!("{at}.state"),
                        format!(
                            "{other} is not a score state; the states are leading, level and trailing"
                        ),
                    ));
                }
            }
        }
        "time_left" => Condition::TimeLeft {
            minutes: within("minutes", c.minutes.ok_or_else(|| missing("minutes"))?)?,
        },
        "red_cards_ours" | "red_cards_theirs" => {
            let n = c.at_least.unwrap_or(1);
            within("at_least", f64::from(n))?;
            if c.kind == "red_cards_ours" {
                Condition::RedCardsOurs { at_least: n }
            } else {
                Condition::RedCardsTheirs { at_least: n }
            }
        }
        "run_of_play" => match (c.above, c.below) {
            (Some(v), None) => Condition::RunOfPlay {
                above: true,
                value: within("above", v)?,
            },
            (None, Some(v)) => Condition::RunOfPlay {
                above: false,
                value: within("below", v)?,
            },
            _ => {
                return Err((
                    at.to_string(),
                    "run_of_play takes one of above or below".into(),
                ));
            }
        },
        _ => Condition::TeamEnergy {
            below: within("below", c.below.ok_or_else(|| missing("below"))?)?,
        },
    })
}

/// The written form of a resolved game state, by name.
pub fn written(state: &GameState, schema: &TacticsSchema) -> GameStateFile {
    let level = |primitive: u8, level: u8| {
        let p = &schema.primitives[usize::from(primitive)];
        let names = p.level_names(schema);
        (p.name.clone(), names[usize::from(level)].to_string())
    };
    let rules = state
        .rules
        .iter()
        .map(|rule| RuleFile {
            when: rule.when.iter().map(condition_file).collect(),
            set: rule
                .set
                .iter()
                .map(|a| match *a {
                    Action::Level {
                        primitive,
                        level: l,
                    } => {
                        let (p, l) = level(primitive, l);
                        ActionFile {
                            primitive: Some(p),
                            level: Some(l),
                            formation: None,
                        }
                    }
                    Action::Formation(f) => ActionFile {
                        formation: Some(schema.formations[usize::from(f)].name.clone()),
                        ..ActionFile::default()
                    },
                })
                .collect(),
        })
        .collect();
    let instructions = state
        .instructions
        .iter()
        .map(|i| {
            let (primitive, level) = level(i.primitive, i.level);
            InstructionFile {
                slot: i.slot,
                primitive,
                level,
            }
        })
        .collect();
    GameStateFile {
        rules,
        instructions,
    }
}

/// Each side's rules and player instructions in their written form, home first, as the hello
/// and the replay record carry them: `None` for a side with neither, and `None` in all when
/// neither side has any, so a match without them writes nothing.
pub fn sides_value(
    states: [&GameState; 2],
    schema: &TacticsSchema,
) -> Option<[Option<serde_json::Value>; 2]> {
    if states.iter().all(|s| s.is_empty()) {
        return None;
    }
    Some(states.map(|s| {
        (!s.is_empty()).then(|| {
            serde_json::to_value(written(s, schema)).expect("a written game state is JSON")
        })
    }))
}

/// A side's written rules and instructions from the hello or the replay record, resolved
/// against the tactics file as a tactic of `formations` resolves them.
pub fn from_value(
    value: &serde_json::Value,
    schema: &TacticsSchema,
    formations: &[u8],
) -> Result<GameState, Refusal> {
    let file: GameStateFile = serde_json::from_value(value.clone())
        .map_err(|e| ("game_state".to_string(), e.to_string()))?;
    resolve(&file, schema, formations)
}

fn condition_file(c: &Condition) -> ConditionFile {
    let base = ConditionFile {
        kind: c.kind().to_string(),
        ..ConditionFile::default()
    };
    match *c {
        Condition::Score(Score::Level) => ConditionFile {
            state: Some("level".into()),
            ..base
        },
        Condition::Score(Score::Leading(by)) => ConditionFile {
            state: Some("leading".into()),
            by: Some(by),
            ..base
        },
        Condition::Score(Score::Trailing(by)) => ConditionFile {
            state: Some("trailing".into()),
            by: Some(by),
            ..base
        },
        Condition::TimeLeft { minutes } => ConditionFile {
            minutes: Some(minutes),
            ..base
        },
        Condition::RedCardsOurs { at_least } | Condition::RedCardsTheirs { at_least } => {
            ConditionFile {
                at_least: Some(at_least),
                ..base
            }
        }
        Condition::RunOfPlay { above: true, value } => ConditionFile {
            above: Some(value),
            ..base
        },
        Condition::RunOfPlay {
            above: false,
            value,
        } => ConditionFile {
            below: Some(value),
            ..base
        },
        Condition::TeamEnergy { below } => ConditionFile {
            below: Some(below),
            ..base
        },
    }
}
