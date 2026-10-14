//! The AI manager (named mechanism). Before kick-off it picks a lineup by each player's
//! rating at the slots' positions, a bench that covers every line, and its tactics: the named
//! style the staff advice rates best for its squad when the content has styles, else the
//! tactics file's default tactics. The advice is an estimate with an error drawn from its own
//! seeded generator; nothing keeps it, and play never reads it. During the match it checks
//! every `check_interval_s` simulated seconds and at once after an injury:
//!
//! - an injured player is replaced by the substitute with the best rating at his slot;
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
use crate::data::styles::Styles;
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
        // A hidden value never counts: the fit reaches the page as a number.
        if let Some(i) = attrs.index(name).filter(|&i| !attrs.attributes[i].hidden) {
            sum += w * p.attributes.get(i).decimal();
            weight += w;
        }
    }
    if weight > 0.0 { sum / weight } else { 0.0 }
}

/// What a slot at `position` values player `p` at: his rating at the position, plus `margin`
/// when he is listed there. So another player takes a listed player's place only when he is
/// clearly better at it.
pub fn slot_value(p: &SquadPlayer, position: Position, margin: f64) -> f64 {
    let listed = if p.position == position { margin } else { 0.0 };
    p.positions[position as usize].decimal() + listed
}

/// A player's rating at his listed (natural) position.
fn natural(p: &SquadPlayer) -> f64 {
    p.positions[p.position as usize].decimal()
}

/// What a lineup choice weighs for player `squad` at a slot at `position` playing `role`: a
/// goalkeeper in goal and an outfield player outside it first, then the slot value, then the
/// role fit, then the lower squad index.
#[derive(Debug, Clone, Copy)]
struct Pick {
    score: f64,
    fit: f64,
    squad: usize,
}

/// Far below any rating: a goalkeeper outfield, or an outfield player in goal, comes last.
const WRONG_KIND: f64 = 100.0;

impl Pick {
    fn of(
        team: &Team,
        squad: usize,
        position: Position,
        role: usize,
        schema: &TacticsSchema,
        attrs: &AttributeSchema,
    ) -> Self {
        let p = &team.squad[squad];
        let kind = (p.position == Position::GK) == (position == Position::GK);
        let score = slot_value(p, position, schema.ai.out_of_position_margin)
            - if kind { 0.0 } else { WRONG_KIND };
        Self {
            score,
            fit: role_fit(p, role, schema, attrs),
            squad,
        }
    }

    /// Best first.
    fn order(&self, other: &Self) -> std::cmp::Ordering {
        other
            .score
            .total_cmp(&self.score)
            .then(other.fit.total_cmp(&self.fit))
            .then(self.squad.cmp(&other.squad))
    }
}

/// The best player in `candidates` for a slot at `position` playing `role`: a goalkeeper for a
/// goalkeeper's slot and an outfield player for any other while one is there, then the highest
/// slot value (his rating at the position, plus the tactics file's out-of-position margin when
/// he is listed there), then the best role fit, then the lower squad index. So a left-back
/// takes the left-back slot and a right-back the right, and a player listed elsewhere takes it
/// only when his rating there is higher by more than the margin.
pub(crate) fn best_for(
    team: &Team,
    candidates: &[usize],
    position: Position,
    role: usize,
    schema: &TacticsSchema,
    attrs: &AttributeSchema,
) -> Option<usize> {
    candidates
        .iter()
        .map(|&s| Pick::of(team, s, position, role, schema, attrs))
        .min_by(Pick::order)
        .map(|p| p.squad)
}

/// The AI manager's pre-match setup for `team` with the tactics file's default tactics:
/// [`pre_match_for`] with [`Tactics::defaults`].
pub fn pre_match(team: &Team, schema: &TacticsSchema, attrs: &AttributeSchema) -> Setup {
    pre_match_for(team, Tactics::defaults(schema), schema, attrs)
}

/// The line a position plays in: defence, midfield, or attack; `None` for the goalkeeper.
fn line_of(position: Position) -> Option<usize> {
    match position {
        Position::GK => None,
        Position::CB | Position::LB | Position::RB => Some(0),
        Position::DM | Position::CM | Position::AM => Some(1),
        Position::LW | Position::RW | Position::ST => Some(2),
    }
}

/// The AI manager's pre-match setup for `team` starting with `tactics`.
///
/// The lineup: every (player, slot) pair is valued as [`best_for`] values it, and the slots
/// fill from the best pair down; then any swap of two starters, or of a starter and a spare,
/// that raises the lineup's summed value is made until none does. So each slot holds the
/// player with the best rating at its position, a listed player keeps his place against a
/// close rating, and a player clearly better at another position than its listed player plays
/// there.
///
/// The bench of `bench_size`: the best spare goalkeeper; then for each line of the formation
/// (defence, midfield, attack) a spare listed at the line's position with the most slots, else
/// the spare with the best rating at one of the line's slot positions; then a full-back and a
/// winger when the formation plays them and the bench has none; then, one at a time, the spare
/// listed at the formation's position with the most slots for each bench player already listed
/// there, the best natural rating first. So the bench covers the positions most players play.
pub fn pre_match_for(
    team: &Team,
    tactics: Tactics,
    schema: &TacticsSchema,
    attrs: &AttributeSchema,
) -> Setup {
    let slots = &schema.formations[usize::from(tactics.formation)].slots;
    let n = team.squad.len();
    let pick = |squad: usize, slot: usize| {
        let role = usize::from(tactics.roles[slot].role);
        Pick::of(team, squad, slots[slot].position, role, schema, attrs)
    };
    let mut pairs: Vec<(Pick, usize)> = (0..n)
        .flat_map(|s| (0..PLAYERS_PER_TEAM).map(move |slot| (s, slot)))
        .map(|(s, slot)| (pick(s, slot), slot))
        .collect();
    pairs.sort_by(|a, b| a.0.order(&b.0).then(a.1.cmp(&b.1)));
    let mut taken = vec![false; n];
    let mut filled = [None::<usize>; PLAYERS_PER_TEAM];
    for (p, slot) in pairs {
        if !taken[p.squad] && filled[slot].is_none() {
            taken[p.squad] = true;
            filled[slot] = Some(p.squad);
        }
    }
    // A validated squad holds at least eleven players, so every slot is filled.
    let mut lineup: [usize; PLAYERS_PER_TEAM] =
        core::array::from_fn(|slot| filled[slot].unwrap_or(0));
    // Swaps that raise the summed value. Each made swap raises it strictly, so the loop ends.
    let score = |s: usize, slot: usize| pick(s, slot).score;
    const EPS: f64 = 1e-9;
    loop {
        let mut improved = false;
        for a in 0..PLAYERS_PER_TEAM {
            for b in a + 1..PLAYERS_PER_TEAM {
                let (pa, pb) = (lineup[a], lineup[b]);
                if score(pa, b) + score(pb, a) > score(pa, a) + score(pb, b) + EPS {
                    lineup.swap(a, b);
                    improved = true;
                }
            }
            for s in 0..n {
                if !lineup.contains(&s) && score(s, a) > score(lineup[a], a) + EPS {
                    lineup[a] = s;
                    improved = true;
                }
            }
        }
        if !improved {
            break;
        }
    }
    let mut free: Vec<usize> = (0..n).filter(|s| !lineup.contains(s)).collect();
    let size = usize::from(schema.ai.bench_size);
    let margin = schema.ai.out_of_position_margin;
    let mut bench: Vec<usize> = Vec::new();
    let take = |bench: &mut Vec<usize>, free: &mut Vec<usize>, s: usize| {
        bench.push(s);
        free.retain(|&f| f != s);
    };
    // The keeper.
    let keepers: Vec<usize> = free
        .iter()
        .copied()
        .filter(|&s| team.squad[s].position == Position::GK)
        .collect();
    let keeper_role = schema.default_role(Position::GK);
    if size > 0
        && let Some(gk) = best_for(team, &keepers, Position::GK, keeper_role, schema, attrs)
    {
        take(&mut bench, &mut free, gk);
    }
    // How many slots of the formation play at `p`, and how many bench players are listed there.
    let slots_at = |p: Position| slots.iter().filter(|s| s.position == p).count();
    let covered = |bench: &[usize], p: Position| {
        bench
            .iter()
            .filter(|&&s| team.squad[s].position == p)
            .count()
    };
    let outfield = |free: &[usize]| -> Vec<usize> {
        free.iter()
            .copied()
            .filter(|&s| team.squad[s].position != Position::GK)
            .collect()
    };
    // One spare for each line of the formation: one listed at the line's position with the
    // most slots first, then one listed in the line, then the best rating at one of the
    // line's slot positions.
    for line in 0..3 {
        let positions: Vec<Position> = slots
            .iter()
            .map(|s| s.position)
            .filter(|&p| line_of(p) == Some(line))
            .collect();
        if positions.is_empty() || bench.len() >= size {
            continue;
        }
        let best = outfield(&free)
            .into_iter()
            .map(|s| {
                let p = &team.squad[s];
                let listed = if positions.contains(&p.position) {
                    slots_at(p.position)
                } else {
                    0
                };
                let in_line = line_of(p.position) == Some(line);
                let value = positions
                    .iter()
                    .map(|&at| slot_value(p, at, margin))
                    .fold(f64::NEG_INFINITY, f64::max);
                (listed, in_line, value, s)
            })
            .min_by(|a, b| {
                b.0.cmp(&a.0)
                    .then(b.1.cmp(&a.1))
                    .then(b.2.total_cmp(&a.2))
                    .then(a.3.cmp(&b.3))
            });
        if let Some((_, _, _, s)) = best {
            take(&mut bench, &mut free, s);
        }
    }
    // Wide cover: a full-back and a winger, when the formation plays the position and the
    // bench has nobody listed at either side yet.
    for pair in [[Position::LB, Position::RB], [Position::LW, Position::RW]] {
        if bench.len() >= size
            || !pair.iter().any(|&p| slots_at(p) > 0)
            || pair.iter().any(|&p| covered(&bench, p) > 0)
        {
            continue;
        }
        let best = free
            .iter()
            .copied()
            .filter(|&s| {
                pair.contains(&team.squad[s].position) && slots_at(team.squad[s].position) > 0
            })
            .min_by(|&a, &b| {
                natural(&team.squad[b])
                    .total_cmp(&natural(&team.squad[a]))
                    .then(a.cmp(&b))
            });
        if let Some(s) = best {
            take(&mut bench, &mut free, s);
        }
    }
    // The rest, one at a time: the spare listed at the formation's position with the most slots
    // for each bench player already listed there, then the best natural rating.
    while bench.len() < size && !free.is_empty() {
        let need = |s: usize| {
            let p = team.squad[s].position;
            slots_at(p) as f64 / (1 + covered(&bench, p)) as f64
        };
        let best = free.iter().copied().min_by(|&a, &b| {
            need(b)
                .total_cmp(&need(a))
                .then(natural(&team.squad[b]).total_cmp(&natural(&team.squad[a])))
                .then(a.cmp(&b))
        });
        let Some(s) = best else {
            break;
        };
        take(&mut bench, &mut free, s);
    }
    Setup {
        tactics,
        lineup,
        bench,
    }
}

/// The pre-match setup version 1: the AI manager's [`pre_match`], and its pick of a style
/// from the staff advice ([`crate::advice::advise`]).
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
    fn advise(
        &self,
        team: &Team,
        styles: &Styles,
        tactics: &TacticsSchema,
        attrs: &AttributeSchema,
        seed: u64,
        side: usize,
    ) -> Option<Tactics> {
        crate::advice::advise(team, styles, tactics, attrs, seed, side)
    }
}

pub const PRE_MATCH_V1_CARD: ModuleCard = ModuleCard {
    purpose: "Picks each team's lineup for the formation it starts in by each player's rating at the slot's position (a listed player keeps his place unless another is clearly better there), a bench that covers the goal, every line and the wide positions, and for an AI-managed side the named style the staff advice rates best for its squad.",
    inputs: "The team's squad with positions, position ratings and attributes, the tactics schema's roles and AI settings, the attribute schema, the named styles, the match seed and the side.",
    outputs: "The setup: the tactics, the squad index of each slot, and the bench; for an AI-managed side, the tactics of the style it picks.",
    tuning: &[
        "tactics.roles",
        "tactics.ai.bench_size",
        "tactics.ai.out_of_position_margin",
        "styles",
    ],
    calibration: "none: lineup and style choice, no realism band",
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
            // One level above the default pressing, never lower than the side already presses.
            let top = (schema.instructions.level_count(PRESSING) - 1) as u8;
            let high = (schema.instructions.defaults()[PRESSING] + 1).min(top);
            if projected.instructions[PRESSING] < high {
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

    /// The best substitute on the bench for `slot`'s position and role, leaving out anyone
    /// already queued to come on: the highest slot value as [`best_for`] weighs it, then the
    /// one whose own position the formation needs least for each bench player listed there,
    /// then the best role fit, then the lower squad index.
    fn substitute_for(&self, slot: usize) -> Option<usize> {
        let side = &self.view.teams()[self.team];
        let incoming = self.queue.incoming(self.team);
        let free: Vec<usize> = side
            .bench
            .iter()
            .copied()
            .filter(|s| !incoming.contains(s))
            .collect();
        let schema = self.view.tactics();
        let position = schema.formations[usize::from(side.tactics.formation)].slots[slot].position;
        let role = usize::from(side.tactics.roles[slot].role);
        let slots = &schema.formations[usize::from(side.tactics.formation)].slots;
        // Among equal slot values, the one whose own position the formation needs least for
        // each bench player listed there comes on, keeping the cover a later change may need.
        let need = |s: usize| {
            let p = side.squad[s].position;
            let on_pitch = slots.iter().filter(|x| x.position == p).count();
            let cover = free
                .iter()
                .filter(|&&b| side.squad[b].position == p)
                .count();
            on_pitch as f64 / cover.max(1) as f64
        };
        free.iter()
            .map(|&s| {
                let pick = Pick::of(side, s, position, role, schema, self.view.attributes());
                (pick, need(s))
            })
            .min_by(|a, b| {
                b.0.score
                    .total_cmp(&a.0.score)
                    .then(a.1.total_cmp(&b.1))
                    .then(a.0.order(&b.0))
            })
            .map(|(p, _)| p.squad)
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
        best_for(
            side,
            &keepers,
            Position::GK,
            role,
            schema,
            self.view.attributes(),
        )
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

    #[test]
    fn full_backs_and_wingers_start_on_their_own_side_whatever_their_fit() {
        use crate::rating::Rating;
        let content = shipped_content();
        let wide = [Position::LB, Position::RB, Position::LW, Position::RW];
        for file in default_teams(&content) {
            // One side's full-backs and wingers fit their role far better than the other's.
            for strong in [[Position::RB, Position::RW], [Position::LB, Position::LW]] {
                let mut file = file.clone();
                for p in &mut file.players {
                    let tenths = if strong.contains(&p.position) {
                        200
                    } else if wide.contains(&p.position) {
                        10
                    } else {
                        continue;
                    };
                    p.attributes
                        .values_mut()
                        .for_each(|v| *v = Rating::from_tenths(tenths));
                }
                let (team, _) =
                    Team::from_file(0, &file, &content.attributes, &content.tuning.engine).unwrap();
                let setup = pre_match(&team, &content.tactics, &content.attributes);
                let slots = &content.tactics.formations[usize::from(setup.tactics.formation)].slots;
                // Each shipped squad has a natural player for every slot of the default
                // formation, so every starter plays his listed position.
                for (slot, &s) in setup.lineup.iter().enumerate() {
                    assert_eq!(team.squad[s].position, slots[slot].position, "slot {slot}");
                }
                // A substitute for a wide slot is picked the same way.
                let all: Vec<usize> = (0..team.squad.len()).collect();
                for position in wide {
                    let role = content.tactics.default_role(position);
                    let pick = best_for(
                        &team,
                        &all,
                        position,
                        role,
                        &content.tactics,
                        &content.attributes,
                    )
                    .unwrap();
                    assert_eq!(team.squad[pick].position, position);
                }
            }
        }
    }
}
