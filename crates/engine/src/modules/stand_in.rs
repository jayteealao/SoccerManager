//! Test stand-ins: for each slot, a separate module registered under its own name that
//! forwards every call to the slot's default module and returns its result unchanged. The
//! swap test selects one through the slot file, like any other module, to prove that the
//! loop reaches a slot only through its selected module and that nothing outside the slot
//! depends on which module fills it.
//!
//! Test builds only (the `scenario` feature). A stand-in is never listed in
//! [`REGISTRY`]: [`registry_with`] returns a copy of the registry with one stand-in added.
//! Every stand-in, its card, and that copy are leaked, which is harmless in a test process.

use super::card::ModuleCard;
use super::game::{
    GameChanges, GameDay, PeopleModule, PresentationModule, SeasonModule, WorldModule,
};
use super::modifier::{Effect, Family, Modifier};
use super::registry::{ModuleRef, REGISTRY, Registration, SlotDecl};
use super::viewer::SkinModule;
use super::{
    AiPlan, BallModule, CarrierPlan, ChangesModule, ClockModule, CommentaryHookModule, Contest,
    CrossContest, Crossing, DecisionHookModule, DecisionModule, Deflection, DisciplineModule,
    FatigueModule, FoulsModule, InjuriesModule, LooseBall, ManagerModule, MatchView, OffsideModule,
    OptionDraft, OptionDraws, ParrySide, PeriodEnd, PossessionModule, PreMatchModule, Proposal,
    ROSTER, RestartPass, RestartsModule, RuleHookModule, RulesModule, Scored, ShootoutLineup,
    ShotDraws, ShotModule, SteeringModule, SubEntry, SubRequest, TackleChances, Targets,
};
use crate::ai::Setup;
use crate::ball::Ball;
use crate::data::Loaded;
use crate::data::attributes::AttributeSchema;
use crate::data::rules::RulePack;
use crate::data::rules::StoppageKind;
use crate::data::tactics::TacticsSchema;
use crate::decision::{Choice, Kick, Options};
use crate::error::EngineError;
use crate::fatigue::InjurySource;
use crate::math::{DVec2, DVec3};
use crate::plugin::{DecisionContext, FoulContext, LineContext, OptionOffsets};
use crate::rules::DeadBall;
use crate::rules::fouls::{Card, Tackle};
use crate::rules::offside::OffsideSet;
use crate::sim::EngineEvent;
use crate::tactics::change::RejectReason;
use crate::tactics::{Tactics, TacticsPatch};
use crate::team::Team;

/// A stand-in around the default module `T` of one slot. Each slot trait is implemented for
/// the stand-in around that trait's modules, and every method forwards.
pub struct StandIn<T: ?Sized + 'static>(pub &'static T);

/// The purpose every stand-in card states.
pub const PURPOSE: &str = "Test stand-in: forwards every call to the slot's default module.";

/// The calibration every stand-in card states.
pub const CALIBRATION: &str = "none: test stand-in, forwards to the default module";

/// The stand-in of `slot`: `<default name>-stand-in@1`, around the slot's default module,
/// with the default's card under the stand-in purpose and calibration and the same keys.
/// `None` for an undeclared slot.
pub fn registration(slot: &str) -> Option<Registration> {
    let default = REGISTRY.iter().find(|d| d.slot.id == slot)?.registrations[0];
    let card = leak(ModuleCard {
        purpose: PURPOSE,
        calibration: CALIBRATION,
        ..*default.card
    });
    Some(Registration {
        name: leak(format!("{}-stand-in", default.name)).as_str(),
        version: 1,
        module: wrap(default.module),
        card,
    })
}

/// A copy of [`REGISTRY`] with the stand-in of `slot` registered after the slot's own
/// modules. `None` for an undeclared slot.
pub fn registry_with(slot: &str) -> Option<Vec<SlotDecl>> {
    let stand_in = registration(slot)?;
    Some(
        REGISTRY
            .iter()
            .map(|decl| {
                let mut decl = *decl;
                if decl.slot.id == slot {
                    let mut list = decl.registrations.to_vec();
                    list.push(stand_in);
                    decl.registrations = leak(list).as_slice();
                }
                decl
            })
            .collect(),
    )
}

fn leak<T>(value: T) -> &'static T {
    Box::leak(Box::new(value))
}

/// The same variant around a stand-in. A variant with no arm here fails to compile, so no
/// slot can miss its stand-in.
fn wrap(module: ModuleRef) -> ModuleRef {
    match module {
        ModuleRef::Fouls(m) => ModuleRef::Fouls(leak(StandIn(m))),
        ModuleRef::Offside(m) => ModuleRef::Offside(leak(StandIn(m))),
        ModuleRef::Shot(m) => ModuleRef::Shot(leak(StandIn(m))),
        ModuleRef::Fatigue(m) => ModuleRef::Fatigue(leak(StandIn(m))),
        ModuleRef::Steering(m) => ModuleRef::Steering(leak(StandIn(m))),
        ModuleRef::PreMatch(m) => ModuleRef::PreMatch(leak(StandIn(m))),
        ModuleRef::Modifier(m) => ModuleRef::Modifier(leak(StandIn(m))),
        ModuleRef::Clock(m) => ModuleRef::Clock(leak(StandIn(m))),
        ModuleRef::Restarts(m) => ModuleRef::Restarts(leak(StandIn(m))),
        ModuleRef::Discipline(m) => ModuleRef::Discipline(leak(StandIn(m))),
        ModuleRef::Injuries(m) => ModuleRef::Injuries(leak(StandIn(m))),
        ModuleRef::Ball(m) => ModuleRef::Ball(leak(StandIn(m))),
        ModuleRef::Possession(m) => ModuleRef::Possession(leak(StandIn(m))),
        ModuleRef::Decision(m) => ModuleRef::Decision(leak(StandIn(m))),
        ModuleRef::Manager(m) => ModuleRef::Manager(leak(StandIn(m))),
        ModuleRef::Changes(m) => ModuleRef::Changes(leak(StandIn(m))),
        ModuleRef::DecisionHook(m) => ModuleRef::DecisionHook(leak(StandIn(m))),
        ModuleRef::RuleHook(m) => ModuleRef::RuleHook(leak(StandIn(m))),
        ModuleRef::CommentaryHook(m) => ModuleRef::CommentaryHook(leak(StandIn(m))),
        ModuleRef::Rules(m) => ModuleRef::Rules(leak(StandIn(m))),
        ModuleRef::World(m) => ModuleRef::World(leak(StandIn(m))),
        ModuleRef::Season(m) => ModuleRef::Season(leak(StandIn(m))),
        ModuleRef::People(m) => ModuleRef::People(leak(StandIn(m))),
        ModuleRef::Presentation(m) => ModuleRef::Presentation(leak(StandIn(m))),
        ModuleRef::Skin(m) => ModuleRef::Skin(leak(StandIn(m))),
    }
}

impl FoulsModule for StandIn<dyn FoulsModule> {
    fn tackle_chances(
        &self,
        view: &MatchView<'_>,
        tackler: usize,
        carrier: usize,
    ) -> TackleChances {
        self.0.tackle_chances(view, tackler, carrier)
    }
    fn tackle_outcome(&self, chances: &TackleChances, draw: f64) -> Tackle {
        self.0.tackle_outcome(chances, draw)
    }
    fn card_thresholds(&self, view: &MatchView<'_>, offender: usize) -> [f64; 2] {
        self.0.card_thresholds(view, offender)
    }
    fn card(&self, view: &MatchView<'_>, offender: usize, draw: f64) -> Option<Card> {
        self.0.card(view, offender, draw)
    }
}

impl OffsideModule for StandIn<dyn OffsideModule> {
    fn on_kick(&self, view: &MatchView<'_>, passer: usize, team: usize) -> Proposal {
        self.0.on_kick(view, passer, team)
    }
    fn is_offence(&self, view: &MatchView<'_>, set: OffsideSet, toucher: usize) -> bool {
        self.0.is_offence(view, set, toucher)
    }
}

impl ShotModule for StandIn<dyn ShotModule> {
    fn xg(&self, view: &MatchView<'_>, from: DVec2, attack_x: f64) -> f64 {
        self.0.xg(view, from, attack_x)
    }
    fn quality(&self, view: &MatchView<'_>, from: DVec2, attack_x: f64) -> f64 {
        self.0.quality(view, from, attack_x)
    }
    fn on_target(&self, view: &MatchView<'_>, ball: Ball, attack_x: f64) -> bool {
        self.0.on_target(view, ball, attack_x)
    }
    fn save_chance(&self, view: &MatchView<'_>, quality: f64) -> f64 {
        self.0.save_chance(view, quality)
    }
}

impl FatigueModule for StandIn<dyn FatigueModule> {
    fn drain(&self, view: &MatchView<'_>, i: usize) -> f64 {
        self.0.drain(view, i)
    }
    fn injury_chance(&self, view: &MatchView<'_>, i: usize, source: InjurySource) -> f64 {
        self.0.injury_chance(view, i, source)
    }
}

impl SteeringModule for StandIn<dyn SteeringModule> {
    fn next_velocity(&self, view: &MatchView<'_>, i: usize) -> DVec2 {
        self.0.next_velocity(view, i)
    }
    fn separate(&self, view: &MatchView<'_>) -> Option<[DVec2; ROSTER]> {
        self.0.separate(view)
    }
}

impl PreMatchModule for StandIn<dyn PreMatchModule> {
    fn setup(&self, team: &Team, tactics: &TacticsSchema, attrs: &AttributeSchema) -> Setup {
        self.0.setup(team, tactics, attrs)
    }
}

impl Modifier for StandIn<dyn Modifier> {
    fn family(&self) -> Family {
        self.0.family()
    }
    fn effect(&self, view: &MatchView<'_>, i: usize) -> Effect {
        self.0.effect(view, i)
    }
}

impl ClockModule for StandIn<dyn ClockModule> {
    fn tally_seconds(&self, view: &MatchView<'_>, extra: bool) -> u32 {
        self.0.tally_seconds(view, extra)
    }
    fn added_seconds(&self, view: &MatchView<'_>, extra: bool, draw: f64) -> u32 {
        self.0.added_seconds(view, extra, draw)
    }
    fn period_end(&self, view: &MatchView<'_>) -> PeriodEnd {
        self.0.period_end(view)
    }
    fn extra_kick_off(&self, draw: f64) -> usize {
        self.0.extra_kick_off(draw)
    }
    fn abandoned(&self, view: &MatchView<'_>) -> Option<usize> {
        self.0.abandoned(view)
    }
    fn shootout_lineup(&self, view: &MatchView<'_>) -> ShootoutLineup {
        self.0.shootout_lineup(view)
    }
    fn shootout_first(&self, draw: f64) -> usize {
        self.0.shootout_first(draw)
    }
    fn shootout_end(&self, draw: f64) -> f64 {
        self.0.shootout_end(draw)
    }
    fn keeper_dive(&self, view: &MatchView<'_>, draw: f64) -> f64 {
        self.0.keeper_dive(view, draw)
    }
    fn shootout_save_hold(&self, view: &MatchView<'_>) -> f64 {
        self.0.shootout_save_hold(view)
    }
    fn shootout_decided(&self, view: &MatchView<'_>, scores: [u32; 2], taken: [u32; 2]) -> bool {
        self.0.shootout_decided(view, scores, taken)
    }
}

impl RestartsModule for StandIn<dyn RestartsModule> {
    fn taker(&self, view: &MatchView<'_>, kind: StoppageKind, team: usize, spot: DVec2) -> usize {
        self.0.taker(view, kind, team, spot)
    }
    fn delay_ticks(&self, view: &MatchView<'_>, kind: StoppageKind, team: usize) -> u32 {
        self.0.delay_ticks(view, kind, team)
    }
    fn shootout_delay_ticks(&self, view: &MatchView<'_>) -> u32 {
        self.0.shootout_delay_ticks(view)
    }
    fn target(&self, view: &MatchView<'_>, dead: &DeadBall, i: usize) -> DVec2 {
        self.0.target(view, dead, i)
    }
    fn ready(&self, view: &MatchView<'_>, dead: &DeadBall, now: u32) -> bool {
        self.0.ready(view, dead, now)
    }
    fn kick_off_position(&self, view: &MatchView<'_>, team: usize, slot: usize) -> DVec2 {
        self.0.kick_off_position(view, team, slot)
    }
}

impl DisciplineModule for StandIn<dyn DisciplineModule> {
    fn shown(&self, view: &MatchView<'_>, i: usize, card: Card) -> Option<Card> {
        self.0.shown(view, i, card)
    }
    fn sends_off(&self, card: Card) -> bool {
        self.0.sends_off(card)
    }
    fn more_severe(&self, a: Card, b: Card) -> Card {
        self.0.more_severe(a, b)
    }
}

impl InjuriesModule for StandIn<dyn InjuriesModule> {
    fn leaves(&self, view: &MatchView<'_>, i: usize, source: InjurySource) -> bool {
        self.0.leaves(view, i, source)
    }
    fn dropped_ball(&self, view: &MatchView<'_>, team: usize) -> (usize, DVec2) {
        self.0.dropped_ball(view, team)
    }
}

impl BallModule for StandIn<dyn BallModule> {
    fn carry(&self, view: &MatchView<'_>, c: usize) -> Ball {
        self.0.carry(view, c)
    }
    fn integrate(&self, view: &MatchView<'_>, ball: Ball) -> Ball {
        self.0.integrate(view, ball)
    }
    fn kick(&self, view: &MatchView<'_>, ball: Ball, dir: DVec2, speed: f64, loft: f64) -> Ball {
        self.0.kick(view, ball, dir, speed, loft)
    }
    fn crossing(&self, view: &MatchView<'_>, prev: DVec2, ball: &Ball) -> Crossing {
        self.0.crossing(view, prev, ball)
    }
    fn deflect(&self, view: &MatchView<'_>, how: Deflection, angle: f64, loft: f64) -> DVec3 {
        self.0.deflect(view, how, angle, loft)
    }
}

impl PossessionModule for StandIn<dyn PossessionModule> {
    fn shot_contest(&self, view: &MatchView<'_>) -> Option<usize> {
        self.0.shot_contest(view)
    }
    fn blockers(&self, view: &MatchView<'_>, shooter: usize) -> Contest {
        self.0.blockers(view, shooter)
    }
    fn save_reach(&self, view: &MatchView<'_>, shooter: usize) -> Option<usize> {
        self.0.save_reach(view, shooter)
    }
    fn save_hold(&self, view: &MatchView<'_>) -> f64 {
        self.0.save_hold(view)
    }
    fn parry_side(&self, view: &MatchView<'_>, k: usize) -> ParrySide {
        self.0.parry_side(view, k)
    }
    fn parry_side_from_draw(&self, draw: f64, threshold: f64) -> f64 {
        self.0.parry_side_from_draw(draw, threshold)
    }
    fn cross_clearers(&self, view: &MatchView<'_>) -> Option<CrossContest> {
        self.0.cross_clearers(view)
    }
    fn clearance_line(&self, view: &MatchView<'_>, i: usize, wide: bool) -> (DVec2, f64) {
        self.0.clearance_line(view, i, wide)
    }
    fn loose_ball(&self, view: &MatchView<'_>) -> Option<LooseBall> {
        self.0.loose_ball(view)
    }
    fn tacklers(&self, view: &MatchView<'_>, c: usize) -> Option<u32> {
        self.0.tacklers(view, c)
    }
}

impl DecisionModule for StandIn<dyn DecisionModule> {
    fn targets(&self, view: &MatchView<'_>) -> Targets {
        self.0.targets(view)
    }
    fn options(&self, view: &MatchView<'_>, c: usize) -> OptionDraft {
        self.0.options(view, c)
    }
    fn scored(&self, draft: &OptionDraft, draws: &OptionDraws) -> Scored {
        self.0.scored(draft, draws)
    }
    fn choose(
        &self,
        view: &MatchView<'_>,
        c: usize,
        options: &Options,
        offsets: Option<OptionOffsets>,
    ) -> (Options, Choice) {
        self.0.choose(view, c, options, offsets)
    }
    fn plan(
        &self,
        view: &MatchView<'_>,
        c: usize,
        options: &Options,
        choice: Choice,
    ) -> CarrierPlan {
        self.0.plan(view, c, options, choice)
    }
    fn pass_kick(&self, view: &MatchView<'_>, c: usize, j: usize, aim: f64) -> Kick {
        self.0.pass_kick(view, c, j, aim)
    }
    fn clear_kick(&self, view: &MatchView<'_>, c: usize, wide: bool, aim: f64) -> Kick {
        self.0.clear_kick(view, c, wide, aim)
    }
    fn shot_draws_side(&self, view: &MatchView<'_>, keeper: usize) -> bool {
        self.0.shot_draws_side(view, keeper)
    }
    fn shot_kick(
        &self,
        view: &MatchView<'_>,
        c: usize,
        goal: DVec2,
        keeper: usize,
        spread_scale: f64,
        draws: &ShotDraws,
    ) -> Kick {
        self.0.shot_kick(view, c, goal, keeper, spread_scale, draws)
    }
    fn restart_pass(&self, view: &MatchView<'_>, taker: usize, kind: StoppageKind) -> RestartPass {
        self.0.restart_pass(view, taker, kind)
    }
}

impl ManagerModule for StandIn<dyn ManagerModule> {
    fn check(&self, view: &MatchView<'_>, team: usize, at_stoppage: bool) -> AiPlan {
        self.0.check(view, team, at_stoppage)
    }
}

impl ChangesModule for StandIn<dyn ChangesModule> {
    fn admits(&self, view: &MatchView<'_>, kind: StoppageKind) -> (bool, bool) {
        self.0.admits(view, kind)
    }
    fn substitution(
        &self,
        view: &MatchView<'_>,
        request: &SubRequest,
    ) -> Result<SubEntry, RejectReason> {
        self.0.substitution(view, request)
    }
    fn tactics(
        &self,
        view: &MatchView<'_>,
        team: usize,
        patch: &TacticsPatch,
        off_now: &[(usize, usize)],
    ) -> Result<Tactics, RejectReason> {
        self.0.tactics(view, team, patch, off_now)
    }
}

impl DecisionHookModule for StandIn<dyn DecisionHookModule> {
    fn context(&self, view: &MatchView<'_>, carrier: usize) -> Option<DecisionContext> {
        self.0.context(view, carrier)
    }
}

impl RuleHookModule for StandIn<dyn RuleHookModule> {
    fn context(
        &self,
        view: &MatchView<'_>,
        offender: usize,
        advantage: bool,
        penalty: bool,
    ) -> Option<FoulContext> {
        self.0.context(view, offender, advantage, penalty)
    }
}

impl CommentaryHookModule for StandIn<dyn CommentaryHookModule> {
    fn context(&self, view: &MatchView<'_>, event: &EngineEvent) -> Option<LineContext> {
        self.0.context(view, event)
    }
}

impl RulesModule for StandIn<dyn RulesModule> {
    fn load(&self, written: &[u8]) -> Result<Loaded<RulePack>, EngineError> {
        self.0.load(written)
    }
}

impl WorldModule for StandIn<dyn WorldModule> {
    fn on_day(&self, day: GameDay) -> GameChanges {
        self.0.on_day(day)
    }
}

impl SeasonModule for StandIn<dyn SeasonModule> {
    fn on_day(&self, day: GameDay) -> GameChanges {
        self.0.on_day(day)
    }
}

impl PeopleModule for StandIn<dyn PeopleModule> {
    fn on_day(&self, day: GameDay) -> GameChanges {
        self.0.on_day(day)
    }
}

impl PresentationModule for StandIn<dyn PresentationModule> {
    fn on_day(&self, day: GameDay) -> GameChanges {
        self.0.on_day(day)
    }
}

impl SkinModule for StandIn<dyn SkinModule> {
    fn skin(&self) -> &'static str {
        self.0.skin()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::{SlotEntry, SlotFile, resolve};

    #[test]
    fn every_slot_has_a_stand_in_that_resolves() {
        for decl in REGISTRY {
            let slot = decl.slot.id;
            let stand_in = registration(slot).expect("a declared slot has a stand-in");
            let name = format!("{}-stand-in", decl.registrations[0].name);
            assert_eq!((stand_in.name, stand_in.version), (name.as_str(), 1));
            assert_eq!(stand_in.card.keys, decl.registrations[0].card.keys);
            let mut file = SlotFile::builtin_default();
            file.slots.insert(
                slot.to_string(),
                SlotEntry {
                    module: name.clone(),
                    version: Some(1),
                },
            );
            let registry = registry_with(slot).unwrap();
            let picked = resolve(&file, &registry)
                .unwrap_or_else(|e| panic!("{slot}: {e}"))
                .picked_for(slot)
                .unwrap();
            assert_eq!((picked.module, picked.version), (name.as_str(), 1));
            // The shipped registry does not know the stand-in.
            assert!(resolve(&file, REGISTRY).is_err(), "{slot}");
        }
        assert!(registration("engine.nowhere").is_none());
        assert!(registry_with("engine.nowhere").is_none());
    }
}
