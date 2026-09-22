//! The commentary file: English lines grouped into template sets per event kind, each set with
//! the conditions under which it applies. The file is loaded through the same fail-closed
//! loader as the other content, and the checks here make two properties hold for every file
//! the engine accepts: every commented event kind has at least three lines that need no
//! condition, and no line uses a placeholder its event kind cannot fill.

use garde::Validate;
use serde::Deserialize;

use super::render::{self, PLACEHOLDERS};
use crate::ai::AiCode;
use crate::data::{self, COMMENTARY_FILE, ContentDir};
use crate::error::EngineError;
use crate::rules::fouls::Card;
use crate::sim::EngineEventKind;

/// The schema version of `commentary/en.json` this build reads.
pub const COMMENTARY_VERSION: u32 = 1;
/// The fewest lines with no condition each commented event kind needs.
pub const MIN_PLAIN_LINES: usize = 3;

/// The event kinds that get a commentary line: every kind except the verdicts on a queued
/// change, which acknowledge the manager's own request.
pub const COMMENTED: [EngineEventKind; 15] = [
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
];

/// `true` when events of `kind` get a line.
pub fn commented(kind: EngineEventKind) -> bool {
    COMMENTED.contains(&kind)
}

/// The placeholders every event of `kind` fills.
pub fn guaranteed(kind: EngineEventKind) -> Vec<&'static str> {
    let mut names = vec![
        "home",
        "away",
        "home_score",
        "away_score",
        "score",
        "minute",
    ];
    let whistle = matches!(kind, EngineEventKind::HalfTime | EngineEventKind::FullTime);
    if !whistle {
        names.extend(["team", "opponent"]);
    }
    if !whistle && kind != EngineEventKind::AiDecision {
        names.push("player");
    }
    if matches!(kind, EngineEventKind::Foul | EngineEventKind::Substitution) {
        names.push("other_player");
    }
    names
}

/// The placeholders an event of `kind` can fill: the guaranteed ones, plus the added minute,
/// which only an event in added time has.
pub fn fillable(kind: EngineEventKind) -> Vec<&'static str> {
    let mut names = guaranteed(kind);
    names.push("added_minutes");
    names
}

/// The part of the match an event falls in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MinuteBand {
    /// Before one sixth of the match.
    Early,
    /// From eight ninths of the match, or in added time of the last half.
    Late,
    AddedTime,
    FirstHalf,
    SecondHalf,
}

/// The score from the side of the event's club (the home club at half-time and full time).
/// The goal states hold on a goal only and describe what the goal did.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ScoreState {
    Level,
    Leading,
    Trailing,
    /// A lead of 3 or more.
    Rout,
    /// The first goal of the match.
    Opener,
    /// The goal levels the score.
    Equaliser,
    /// The goal turns a level score into a lead.
    GoAhead,
    /// The scoring club already led.
    ExtendsLead,
    /// The scoring club still trails.
    Consolation,
}

/// Recent form of the event's club.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Form {
    /// The club scored 2 or more goals in the last sixth of the match.
    Hot,
    /// The club conceded 2 or more goals in the last sixth of the match.
    Cold,
    /// The player the event names already holds a yellow card.
    Booked,
}

/// How often events of the same kind happened in the last ten minutes of play.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Repeat {
    /// None before this one.
    First,
    /// At least one before this one.
    Again,
    /// At least two before this one.
    Streak,
}

/// The card on a card event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CardCondition {
    Yellow,
    SecondYellow,
    Red,
}

impl CardCondition {
    pub fn matches(self, card: Card) -> bool {
        matches!(
            (self, card),
            (CardCondition::Yellow, Card::Yellow)
                | (CardCondition::SecondYellow, Card::SecondYellow)
                | (CardCondition::Red, Card::Red)
        )
    }
}

/// The AI manager's choice on an `ai-decision` event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DecisionCondition {
    MentalityUpTrailing,
    MentalityDownLeading,
    SubInjury,
    SubFatigue,
}

impl DecisionCondition {
    pub fn matches(self, code: AiCode) -> bool {
        matches!(
            (self, code),
            (
                DecisionCondition::MentalityUpTrailing,
                AiCode::MentalityUpTrailing
            ) | (
                DecisionCondition::MentalityDownLeading,
                AiCode::MentalityDownLeading
            ) | (DecisionCondition::SubInjury, AiCode::SubInjury)
                | (DecisionCondition::SubFatigue, AiCode::SubFatigue)
        )
    }
}

/// The conditions of a template set. A set with no condition applies to every event of its
/// kind.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct When {
    pub minute: Option<MinuteBand>,
    pub score: Option<ScoreState>,
    pub form: Option<Form>,
    pub repeat: Option<Repeat>,
    pub card: Option<CardCondition>,
    /// On a foul: `true` when play continued with advantage.
    pub advantage: Option<bool>,
    /// On a goal: `true` when the player who kicked the ball last plays for the other club.
    pub own_goal: Option<bool>,
    pub decision: Option<DecisionCondition>,
}

impl When {
    /// How many conditions the set names; a higher count is more specific.
    pub fn specificity(&self) -> usize {
        [
            self.minute.is_some(),
            self.score.is_some(),
            self.form.is_some(),
            self.repeat.is_some(),
            self.card.is_some(),
            self.advantage.is_some(),
            self.own_goal.is_some(),
            self.decision.is_some(),
        ]
        .into_iter()
        .filter(|&b| b)
        .count()
    }
}

/// One template set as the file writes it.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TemplateSet {
    /// The event kind, spelled as the event contract spells it (`goal`, `throw-in`).
    pub event: String,
    #[serde(default)]
    pub when: When,
    pub lines: Vec<String>,
}

/// The commentary file.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct CommentaryFile {
    #[garde(skip)]
    pub schema_version: u32,
    /// The language of the lines, as a tag (`en`).
    #[garde(length(min = 1, max = 16))]
    pub language: String,
    #[garde(custom(valid_templates))]
    pub templates: Vec<TemplateSet>,
}

/// The commented event kind a file spells as `code`.
fn kind_of(code: &str) -> Option<EngineEventKind> {
    COMMENTED.into_iter().find(|k| k.code() == code)
}

fn valid_templates(sets: &[TemplateSet], _ctx: &()) -> garde::Result {
    let fail = |text: String| Err(garde::Error::new(text));
    for (s, set) in sets.iter().enumerate() {
        let Some(kind) = kind_of(&set.event) else {
            return fail(format!(
                "templates[{s}]: event {} is not a commented event kind",
                set.event
            ));
        };
        if set.lines.is_empty() {
            return fail(format!("templates[{s}] ({}): no lines", set.event));
        }
        let can = fillable(kind);
        for (l, line) in set.lines.iter().enumerate() {
            let at = format!("templates[{s}].lines[{l}] ({})", set.event);
            if line.trim().is_empty() {
                return fail(format!("{at}: the line is empty"));
            }
            let names =
                render::placeholders(line).map_err(|e| garde::Error::new(format!("{at}: {e}")))?;
            for name in names {
                if !PLACEHOLDERS.contains(&name) {
                    return fail(format!("{at}: unknown placeholder {{{name}}}"));
                }
                if !can.contains(&name) {
                    return fail(format!(
                        "{at}: placeholder {{{name}}} cannot be filled on {}",
                        set.event
                    ));
                }
            }
        }
    }
    for kind in COMMENTED {
        let sure = guaranteed(kind);
        let plain = sets
            .iter()
            .filter(|set| set.event == kind.code() && set.when.specificity() == 0)
            .flat_map(|set| set.lines.iter())
            .filter(|line| {
                render::placeholders(line).is_ok_and(|names| names.iter().all(|n| sure.contains(n)))
            })
            .count();
        if plain < MIN_PLAIN_LINES {
            return fail(format!(
                "event {} has {plain} lines with no condition that always fill; at least {MIN_PLAIN_LINES} are needed",
                kind.code()
            ));
        }
    }
    Ok(())
}

/// One loaded template set, ready for selection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Set {
    pub kind: EngineEventKind,
    pub when: When,
    pub lines: Vec<String>,
    pub specificity: usize,
}

/// The loaded commentary: every template set with its event kind resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Commentary {
    pub language: String,
    pub sets: Vec<Set>,
}

impl Commentary {
    /// Loads and checks `commentary/en.json` from `dir`. A refusal names the file, the
    /// template set and line, and the event kind or placeholder at fault.
    pub fn load(dir: &ContentDir) -> Result<Self, EngineError> {
        Self::load_path(&dir.path(COMMENTARY_FILE), COMMENTARY_FILE)
    }

    /// Loads and checks a commentary file at `path`; `shown` is the path errors name.
    pub fn load_path(path: &std::path::Path, shown: &str) -> Result<Self, EngineError> {
        let file =
            data::load_json::<CommentaryFile>("commentary", path, shown, COMMENTARY_VERSION, &())?
                .value;
        Ok(Self::from_file(file))
    }

    /// Resolves a checked file. A set whose event is not commented is left out; the checks
    /// in the loader refuse such a file first.
    pub fn from_file(file: CommentaryFile) -> Self {
        let sets = file
            .templates
            .into_iter()
            .filter_map(|set| {
                Some(Set {
                    kind: kind_of(&set.event)?,
                    specificity: set.when.specificity(),
                    when: set.when,
                    lines: set.lines,
                })
            })
            .collect();
        Self {
            language: file.language,
            sets,
        }
    }
}
