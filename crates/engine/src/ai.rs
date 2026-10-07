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
use crate::data::rules::StoppageKind;
use crate::data::tactics::{PRESSING, TIME_WASTING, TacticsSchema};
use crate::data::team::Position;
use crate::modules::{AiPlan, ManagerModule, MatchView, ModuleCard, PreMatchModule};
use crate::player::Status;
use crate::tactics::change::{Change, ChangeId, ChangeQueue, QueuedChange};
use crate::tactics::{Tactics, TacticsPatch};
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

/// How well `p` fits role `role`: the weighted mean of the role's attributes, on the 1 to 20
/// scale. A lineup choice, not an action of play, so it reads the ratings directly.
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
            sum += w * p.attributes.get(i).decimal();
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

/// The AI manager's pre-match setup for `team` with the tactics file's default tactics:
/// [`pre_match_for`] with [`Tactics::defaults`].
pub fn pre_match(team: &Team, schema: &TacticsSchema, attrs: &AttributeSchema) -> Setup {
    pre_match_for(team, Tactics::defaults(schema), schema, attrs)
}

/// The AI manager's pre-match setup for `team` starting with `tactics`: the best-fitting
/// player for each slot's role in slot order, and a bench of `bench_size` with the best
/// remaining goalkeeper first and then the best remaining players by fit to their own
/// position's first role. So a side that starts in another formation fields players whose
/// position suits that formation's slots, as it does in the default one.
pub fn pre_match_for(
    team: &Team,
    tactics: Tactics,
    schema: &TacticsSchema,
    attrs: &AttributeSchema,
) -> Setup {
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
    fn setup_for(
        &self,
        team: &Team,
        start: Tactics,
        tactics: &TacticsSchema,
        attrs: &AttributeSchema,
    ) -> Setup {
        pre_match_for(team, start, tactics, attrs)
    }
}

pub const PRE_MATCH_V1_CARD: ModuleCard = ModuleCard {
    purpose: "Picks each team's lineup by role fit for the formation it starts in, a bench with the best spare goalkeeper first, and the tactics file's default tactics.",
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
    fn setup(&self, team: &Team, tactics: &TacticsSchema, attrs: &AttributeSchema) -> Setup {
        self.setup_for(team, Tactics::defaults(tactics), tactics, attrs)
    }
    fn setup_for(
        &self,
        team: &Team,
        start: Tactics,
        tactics: &TacticsSchema,
        _: &AttributeSchema,
    ) -> Setup {
        let mut lineup = [0usize; PLAYERS_PER_TEAM];
        for (slot, place) in lineup.iter_mut().enumerate() {
            *place = slot;
        }
        let size = usize::from(tactics.ai.bench_size);
        let bench = (PLAYERS_PER_TEAM..team.squad.len()).take(size).collect();
        Setup {
            tactics: start,
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

/// The AI manager's in-match checks, version 1 (the rules in this file's header). It reads
/// the view and works on a copy of the change queue: each change it proposes joins the copy
/// before the next read, so a later rule of the same check sees it, as the queue would.
pub struct AiManagerV1;

impl ManagerModule for AiManagerV1 {
    fn check(&self, view: &MatchView<'_>, team: usize, at_stoppage: bool) -> AiPlan {
        let mut check = Check {
            view,
            team,
            queue: view.queue().clone(),
            changes: Vec::new(),
        };
        let mut memory = view.ai_memory(team);
        let now = view.tick() + 1;
        let minute = view.referee().clock.minute(now).0;
        let ai = &view.tactics().ai;
        let limit = usize::from(view.substitution_limits().0);
        let used = usize::from(view.ledgers()[team].used);
        let players = view.players();
        let side = &view.teams()[team];
        // Injuries: every injured player on the lineup without a substitute queued. The check
        // on the injury's own stoppage always asks, so a refusal names its reason there; a
        // later check asks again only while the team could still make a substitution.
        let may_substitute = at_stoppage || check.substitution_possible();
        for slot in 0..PLAYERS_PER_TEAM {
            let i = team * PLAYERS_PER_TEAM + slot;
            let off = side.lineup[slot];
            if !may_substitute
                || players[i].status != Status::Injured
                || check.queue.has_substitution(team, off)
            {
                continue;
            }
            if let Some(on) = check.substitute_for(slot) {
                check.propose(Change::Substitution { off, on }, AiCode::SubInjury);
            }
        }
        // The goal: an outfield player keeping goal makes way for the bench keeper, unless a
        // goalkeeper is already on the way (for an injured keeper, say).
        let keeper_slot = side.keeper_slot();
        let keeper_off = side.lineup[keeper_slot];
        let keeper_coming = check
            .queue
            .incoming(team)
            .iter()
            .any(|&s| side.squad[s].position == Position::GK);
        if side.squad[keeper_off].position != Position::GK
            && !keeper_coming
            && !check.queue.has_substitution(team, keeper_off)
            && check.substitution_possible()
            && let Some(on) = check.bench_keeper()
        {
            check.propose(
                Change::Substitution {
                    off: keeper_off,
                    on,
                },
                AiCode::SubKeeper,
            );
        }
        // Fatigue.
        let windows_left = view.ledgers()[team].windows < view.substitution_limits().1;
        if minute >= ai.fatigue_from_minute && windows_left {
            let reserve = usize::from(minute < ai.keep_for_injury_until_minute);
            let mut tired: Vec<(f64, usize)> = (1..PLAYERS_PER_TEAM)
                .map(|slot| (slot, team * PLAYERS_PER_TEAM + slot))
                .filter(|&(slot, i)| {
                    let p = &players[i];
                    slot != keeper_slot
                        && p.active()
                        && p.energy < ai.fatigue_energy
                        && !check.queue.has_substitution(team, side.lineup[slot])
                })
                .map(|(slot, i)| (players[i].energy, slot))
                .collect();
            tired.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
            for (_, slot) in tired {
                let left = limit.saturating_sub(used + check.queue.substitutions(team));
                if left <= reserve {
                    break;
                }
                let off = side.lineup[slot];
                if let Some(on) = check.substitute_for(slot) {
                    check.propose(Change::Substitution { off, on }, AiCode::SubFatigue);
                }
            }
        }
        // The score.
        let score = view.goals();
        let (mine, theirs) = (score[team], score[1 - team]);
        let projected = check.projected_tactics();
        let schema = view.tactics();
        if mine < theirs && minute >= ai.trailing_minute && memory.trailing_acted != Some(score) {
            memory.trailing_acted = Some(score);
            let mut patch = TacticsPatch::default();
            if usize::from(projected.mentality) + 1 < schema.mentalities.len() {
                patch.mentality = Some(projected.mentality + 1);
            }
            let high = (schema.instructions.level_count(PRESSING) - 1) as u8;
            if projected.instructions[PRESSING] != high {
                patch.instructions[PRESSING] = Some(high);
            }
            if patch != TacticsPatch::default() {
                check.propose(Change::Tactics(patch), AiCode::MentalityUpTrailing);
            }
        } else if mine > theirs
            && minute >= ai.leading_minute
            && memory.leading_acted != Some(score)
        {
            memory.leading_acted = Some(score);
            let mut patch = TacticsPatch::default();
            if projected.mentality > 0 {
                patch.mentality = Some(projected.mentality - 1);
            }
            let on = (schema.instructions.level_count(TIME_WASTING) - 1) as u8;
            if projected.instructions[TIME_WASTING] != on {
                patch.instructions[TIME_WASTING] = Some(on);
            }
            if patch != TacticsPatch::default() {
                check.propose(Change::Tactics(patch), AiCode::MentalityDownLeading);
            }
        }
        AiPlan {
            minute,
            changes: check.changes,
            memory,
        }
    }
}

/// One check in progress: the view, the team, the copy of the queue with the check's own
/// proposals on it, and the proposals in order.
struct Check<'v, 'a> {
    view: &'v MatchView<'a>,
    team: usize,
    queue: ChangeQueue,
    changes: Vec<(Change, AiCode)>,
}

impl Check<'_, '_> {
    /// Proposes `change`: it joins the queue copy, as queuing it would.
    fn propose(&mut self, change: Change, code: AiCode) {
        let id = ChangeId {
            tick: self.view.tick(),
            n: self.queue.next,
        };
        self.queue.next += 1;
        self.queue.pending.push(QueuedChange {
            id,
            team: self.team,
            change: change.clone(),
        });
        self.changes.push((change, code));
    }

    /// `true` while a substitution for the team could still apply at some later stoppage:
    /// the limit is not used up by substitutions made and waiting, and a window is left or a
    /// window-exempt stoppage (half-time in the shipped pack) is still to come.
    fn substitution_possible(&self) -> bool {
        let rules = &self.view.rules().substitutions;
        let (limit, windows) = self.view.substitution_limits();
        let ledger = self.view.ledgers()[self.team];
        let taken = usize::from(ledger.used) + self.queue.substitutions(self.team);
        if taken >= usize::from(limit) {
            return false;
        }
        let exempt_ahead = rules
            .windows_exempt
            .iter()
            .any(|&kind| kind != StoppageKind::HalfTime || !self.view.referee().clock.last_half());
        ledger.windows < windows || exempt_ahead
    }

    /// The team's tactics once every waiting tactics change has applied.
    fn projected_tactics(&self) -> Tactics {
        let mut t = self.view.teams()[self.team].tactics;
        for q in self.queue.pending.iter().filter(|q| q.team == self.team) {
            if let Change::Tactics(p) = &q.change {
                if let Some(m) = p.mentality {
                    t.mentality = m;
                }
                for (level, set) in t.instructions.iter_mut().zip(p.instructions) {
                    if let Some(l) = set {
                        *level = l;
                    }
                }
            }
        }
        t
    }

    /// The best substitute on the bench for `slot`'s role, leaving out anyone already queued
    /// to come on.
    fn substitute_for(&self, slot: usize) -> Option<usize> {
        let side = &self.view.teams()[self.team];
        let incoming = self.queue.incoming(self.team);
        let free: Vec<usize> = side
            .bench
            .iter()
            .copied()
            .filter(|s| !incoming.contains(s))
            .collect();
        let role = usize::from(side.tactics.roles[slot].role);
        best_for(
            side,
            &free,
            role,
            self.view.tactics(),
            self.view.attributes(),
        )
    }

    /// The best goalkeeper on the bench by fit to the goalkeeper's role, leaving out anyone
    /// already queued to come on.
    fn bench_keeper(&self) -> Option<usize> {
        let side = &self.view.teams()[self.team];
        let incoming = self.queue.incoming(self.team);
        let keepers: Vec<usize> = side
            .bench
            .iter()
            .copied()
            .filter(|s| !incoming.contains(s) && side.squad[*s].position == Position::GK)
            .collect();
        let schema = self.view.tactics();
        let role = schema.default_role(Position::GK);
        best_for(side, &keepers, role, schema, self.view.attributes())
    }
}

pub const AI_MANAGER_V1_CARD: ModuleCard = ModuleCard {
    purpose: "Makes the AI manager's in-match choices: substitutions for injury, for the goal, and for fatigue, and the mentality change when trailing or leading.",
    inputs: "Each player's status, energy, and activity; the team's lineup, bench, squad, and tactics; the change queue; the substitution ledgers and limits; the score; the match clock; the manager's memory; the tactics file, the rule pack, and the attribute schema.",
    outputs: "The changes to queue in order, each with its reason code, and the manager's memory after the check.",
    tuning: &[
        "tactics.ai.trailing_minute",
        "tactics.ai.leading_minute",
        "tactics.ai.fatigue_energy",
        "tactics.ai.fatigue_from_minute",
        "tactics.ai.keep_for_injury_until_minute",
        "rules.substitutions",
        "rules.extra_time",
    ],
    calibration: "none: no substitution or mentality band in realism-bands.json",
    keys: &[],
};

/// The off version: no in-match decision; the manager's memory stays as it was.
pub struct ManagerOff;

impl ManagerModule for ManagerOff {
    fn check(&self, view: &MatchView<'_>, team: usize, _: bool) -> AiPlan {
        AiPlan {
            minute: view.referee().clock.minute(view.tick() + 1).0,
            changes: Vec::new(),
            memory: view.ai_memory(team),
        }
    }
}

pub const MANAGER_OFF_CARD: ModuleCard = ModuleCard {
    purpose: "Makes no in-match decision for an AI-managed team.",
    inputs: "The manager's memory and the match clock.",
    outputs: "No change, and the memory as it was.",
    tuning: &["none"],
    calibration: "none: off version, no in-match decision",
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
