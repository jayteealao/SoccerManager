//! The AI manager (named mechanism). Before kick-off it picks a lineup, a bench, and the
//! tactics file's default tactics. During the match it checks every `check_interval_s`
//! simulated seconds and at once after an injury:
//!
//! - an injured player is replaced by the best-fitting substitute;
//! - from `fatigue_from_minute`, a tired outfield player (energy below `fatigue_energy`) is
//!   replaced, keeping one substitution back for an injury until
//!   `keep_for_injury_until_minute`;
//! - an outfield player keeping goal after the keeper was sent off is replaced by the best
//!   goalkeeper on the bench, while a substitution can still be made;
//! - trailing from `trailing_minute`, the mentality rises one step and pressing goes high,
//!   once per score;
//! - leading from `leading_minute`, the mentality drops one step and time wasting goes on,
//!   once per score.
//!
//! It draws no random numbers, so a match with the AI draws the same generator stream as
//! the rules alone would. Every choice goes through the same change queue a human manager
//! uses, and emits an `AiDecision` event with a short code.

use crate::data::attributes::AttributeSchema;
use crate::data::tactics::TacticsSchema;
use crate::data::team::Position;
use crate::modules::{ModuleCard, PreMatchModule};
use crate::tactics::Tactics;
use crate::team::{PLAYERS_PER_TEAM, SquadPlayer, Team};

/// Who manages a team during the match.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Manager {
    /// The AI manager: the pre-match setup and every in-match decision.
    Ai,
    /// A person: the AI's pre-match setup until a lineup message exists, and no in-match
    /// AI decisions.
    Human,
}

/// What an AI choice was.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiCode {
    MentalityUpTrailing,
    MentalityDownLeading,
    SubInjury,
    SubFatigue,
    SubKeeper,
}

impl AiCode {
    /// The `ai.decision` value.
    pub fn code(&self) -> &'static str {
        match self {
            AiCode::MentalityUpTrailing => "mentality-up-trailing",
            AiCode::MentalityDownLeading => "mentality-down-leading",
            AiCode::SubInjury => "sub-injury",
            AiCode::SubFatigue => "sub-fatigue",
            AiCode::SubKeeper => "sub-keeper",
        }
    }
}

/// The AI manager's memory: the score it last acted on while trailing and while leading.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AiState {
    pub trailing_acted: Option<[u32; 2]>,
    pub leading_acted: Option<[u32; 2]>,
    /// An injury asks for a check on this tick.
    pub due: bool,
}

/// The pre-match setup: tactics, the squad index of each slot, and the bench.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Setup {
    pub tactics: Tactics,
    pub lineup: [usize; PLAYERS_PER_TEAM],
    pub bench: Vec<usize>,
}

/// How well `p` fits role `role`: the weighted mean of the role's attributes.
pub fn role_fit(
    p: &SquadPlayer,
    role: usize,
    schema: &TacticsSchema,
    attrs: &AttributeSchema,
) -> f64 {
    let mut sum = 0.0;
    let mut weight = 0.0;
    for (name, w) in &schema.roles[role].attributes {
        if let Some(i) = attrs.index(name) {
            sum += w * f64::from(p.attributes.get(i));
            weight += w;
        }
    }
    if weight > 0.0 { sum / weight } else { 0.0 }
}

/// The best player in `candidates` for `role`: players whose position the role suits come
/// first, then any outfield player (or any player for a goalkeeper's role). Ties go to the
/// lower squad index.
pub(crate) fn best_for(
    team: &Team,
    candidates: &[usize],
    role: usize,
    schema: &TacticsSchema,
    attrs: &AttributeSchema,
) -> Option<usize> {
    let suits = &schema.roles[role].positions;
    let keeper_role = suits.contains(&Position::GK);
    let pick = |filter: &dyn Fn(Position) -> bool| {
        candidates
            .iter()
            .copied()
            .filter(|&s| filter(team.squad[s].position))
            .map(|s| (role_fit(&team.squad[s], role, schema, attrs), s))
            .fold(None::<(f64, usize)>, |best, (fit, s)| match best {
                Some((b, _)) if b >= fit => best,
                _ => Some((fit, s)),
            })
            .map(|(_, s)| s)
    };
    pick(&|p| suits.contains(&p))
        .or_else(|| pick(&|p| keeper_role || p != Position::GK))
        .or_else(|| pick(&|_| true))
}

/// The AI manager's pre-match setup for `team`: the tactics file's default tactics, the
/// best-fitting player for each slot in slot order, and a bench of `bench_size` with the best
/// remaining goalkeeper first and then the best remaining players by fit to their own
/// position's first role.
pub fn pre_match(team: &Team, schema: &TacticsSchema, attrs: &AttributeSchema) -> Setup {
    let tactics = Tactics::defaults(schema);
    let mut free: Vec<usize> = (0..team.squad.len()).collect();
    let mut lineup = [0usize; PLAYERS_PER_TEAM];
    for (slot, place) in lineup.iter_mut().enumerate() {
        let role = usize::from(tactics.roles[slot].role);
        // A validated squad holds at least eleven players.
        let pick = best_for(team, &free, role, schema, attrs).unwrap_or(free[0]);
        *place = pick;
        free.retain(|&s| s != pick);
    }
    let mut bench = Vec::new();
    let size = usize::from(schema.ai.bench_size);
    let keeper_role = schema.default_role(Position::GK);
    let keepers: Vec<usize> = free
        .iter()
        .copied()
        .filter(|&s| team.squad[s].position == Position::GK)
        .collect();
    if size > 0
        && let Some(gk) = best_for(team, &keepers, keeper_role, schema, attrs)
    {
        bench.push(gk);
        free.retain(|&s| s != gk);
    }
    let mut rest: Vec<(f64, usize)> = free
        .iter()
        .map(|&s| {
            let role = schema.default_role(team.squad[s].position);
            (role_fit(&team.squad[s], role, schema, attrs), s)
        })
        .collect();
    rest.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    bench.extend(
        rest.into_iter()
            .map(|(_, s)| s)
            .take(size.saturating_sub(bench.len())),
    );
    Setup {
        tactics,
        lineup,
        bench,
    }
}

/// The pre-match setup version 1: the AI manager's [`pre_match`].
pub struct PreMatchV1;

impl PreMatchModule for PreMatchV1 {
    fn setup(&self, team: &Team, tactics: &TacticsSchema, attrs: &AttributeSchema) -> Setup {
        pre_match(team, tactics, attrs)
    }
}

pub const PRE_MATCH_V1_CARD: ModuleCard = ModuleCard {
    purpose: "Picks each team's lineup by role fit, a bench with the best spare goalkeeper first, and the tactics file's default tactics.",
    inputs: "The team's squad with positions and attributes, the tactics schema's roles and AI bench size, and the attribute schema.",
    outputs: "The setup: the tactics, the squad index of each slot, and the bench.",
    tuning: &["tactics.roles", "tactics.ai.bench_size"],
    calibration: "none: lineup choice, no realism band",
    keys: &[],
};

/// The pre-match setup switched off: the squad in its file order, the first eleven start,
/// the next `bench_size` sit on the bench, and the tactics file's default tactics.
pub struct PreMatchOff;

impl PreMatchModule for PreMatchOff {
    fn setup(&self, team: &Team, tactics: &TacticsSchema, _: &AttributeSchema) -> Setup {
        let mut lineup = [0usize; PLAYERS_PER_TEAM];
        for (slot, place) in lineup.iter_mut().enumerate() {
            *place = slot;
        }
        let size = usize::from(tactics.ai.bench_size);
        let bench = (PLAYERS_PER_TEAM..team.squad.len()).take(size).collect();
        Setup {
            tactics: Tactics::defaults(tactics),
            lineup,
            bench,
        }
    }
}

pub const PRE_MATCH_OFF_CARD: ModuleCard = ModuleCard {
    purpose: "The pre-match setup switched off: the squad in file order fills the eleven slots and then the bench, with the default tactics.",
    inputs: "The team's squad size and the tactics schema's AI bench size.",
    outputs: "The setup in squad order with the default tactics.",
    tuning: &["tactics.ai.bench_size"],
    calibration: "none: off version, no lineup choice",
    keys: &[],
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::test_support::{default_teams, shipped_content};

    #[test]
    fn the_pre_match_lineup_puts_a_goalkeeper_in_goal_and_one_on_the_bench() {
        let content = shipped_content();
        let [a, _] = default_teams(&content);
        let (team, _) =
            Team::from_file(0, &a, &content.attributes, &content.tuning.engine).unwrap();
        let setup = pre_match(&team, &content.tactics, &content.attributes);
        assert_eq!(team.squad[setup.lineup[0]].position, Position::GK);
        assert_eq!(setup.bench.len(), 7);
        assert_eq!(team.squad[setup.bench[0]].position, Position::GK);
        let mut all: Vec<usize> = setup.lineup.to_vec();
        all.extend(&setup.bench);
        let n = all.len();
        all.sort_unstable();
        all.dedup();
        assert_eq!(all.len(), n, "a player is named twice");
        for slot in 1..PLAYERS_PER_TEAM {
            assert_ne!(team.squad[setup.lineup[slot]].position, Position::GK);
        }
    }

    #[test]
    fn the_pre_match_module_gives_the_ai_setup_and_its_off_version_squad_order() {
        let content = shipped_content();
        for (i, file) in default_teams(&content).iter().enumerate() {
            let (team, _) =
                Team::from_file(i, file, &content.attributes, &content.tuning.engine).unwrap();
            assert_eq!(
                PreMatchV1.setup(&team, &content.tactics, &content.attributes),
                pre_match(&team, &content.tactics, &content.attributes)
            );
            let off = PreMatchOff.setup(&team, &content.tactics, &content.attributes);
            let mut starters = off.lineup.to_vec();
            starters.sort_unstable();
            starters.dedup();
            assert_eq!(starters.len(), PLAYERS_PER_TEAM, "eleven distinct starters");
            assert_eq!(off.lineup, core::array::from_fn(|slot| slot));
            assert!(off.bench.iter().all(|s| !off.lineup.contains(s)));
            assert_eq!(off.tactics, Tactics::defaults(&content.tactics));
        }
    }

    #[test]
    fn the_pre_match_setup_is_the_same_every_time() {
        let content = shipped_content();
        let [a, _] = default_teams(&content);
        let (team, _) =
            Team::from_file(0, &a, &content.attributes, &content.tuning.engine).unwrap();
        let one = pre_match(&team, &content.tactics, &content.attributes);
        let two = pre_match(&team, &content.tactics, &content.attributes);
        assert_eq!(one, two);
    }
}
