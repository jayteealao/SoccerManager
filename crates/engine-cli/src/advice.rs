//! The assistant's advice for the page's team. The computer manager's own check runs for the
//! team a person manages, as advice only: it reads the match through the shared read-only view
//! and a copy of the change queue, draws no random number, and writes nothing, so the match
//! plays exactly as it would without it. Its picks travel as an `advice` server message, which
//! no record or replay keeps; the page queues a pick only when the person accepts it.

use engine::ai::{AiCode, AiManagerV1};
use engine::modules::ManagerModule;
use engine::{Card, Change, EngineEvent, EngineEventKind, Manager, Simulation, TacticsPatch};
use protocol::{Advice, AdvicePick, PatchWire, RoleWire};

/// The team a page manages.
const HOME: usize = 0;

/// The advice drive of one run: the picks last sent.
#[derive(Debug, Default)]
pub(crate) struct Advisor {
    /// The picks of the last `advice` message; a check that finds the same picks sends none.
    sent: Vec<AdvicePick>,
}

impl Advisor {
    /// The advice after a step that recorded `events`, when a check is due and its picks
    /// differ from the last ones sent. A check is due, as the computer manager's own, on the
    /// tick a stoppage that admits a substitution opened, and on a tick whose events hold a
    /// goal, an injury to the page's team, or a sending-off of one of its players. The picks
    /// are substitutions only. No check runs for a team the computer manages, in a penalty
    /// shoot-out, or after full time.
    pub(crate) fn after_step(
        &mut self,
        sim: &Simulation,
        events: &[EngineEvent],
    ) -> Option<Advice> {
        if sim.managers()[HOME] != Manager::Human || sim.is_over() || sim.in_shootout() {
            return None;
        }
        let view = sim.view();
        if !events.iter().any(asks_for_check) && !view.substitutions_admitted() {
            return None;
        }
        let plan = AiManagerV1.check(&view, HOME, true);
        let minute = view.referee().clock.minute(sim.tick() + 1).0;
        let picks: Vec<AdvicePick> = plan
            .changes
            .iter()
            .map(|(change, code)| pick(change, *code))
            .collect();
        if picks == self.sent {
            return None;
        }
        self.sent.clone_from(&picks);
        Some(Advice {
            tick: sim.tick(),
            minute,
            picks,
        })
    }
}

/// `true` for an event after which the computer manager checks at once: a goal, an injury to
/// the page's team, or a sending-off of one of its players.
fn asks_for_check(event: &EngineEvent) -> bool {
    match event.kind {
        EngineEventKind::Goal => true,
        EngineEventKind::Injury => event.team == Some(HOME),
        EngineEventKind::Card => {
            event.team == Some(HOME) && matches!(event.card, Some(Card::SecondYellow | Card::Red))
        }
        _ => false,
    }
}

/// One proposed change as the wire names it.
fn pick(change: &Change, code: AiCode) -> AdvicePick {
    match change {
        Change::Substitution { off, on } => AdvicePick {
            code: code.code().into(),
            kind: "substitution".into(),
            off: Some(wire_index(*off)),
            on: Some(wire_index(*on)),
            patch: None,
        },
        Change::Tactics(patch) => AdvicePick {
            code: code.code().into(),
            kind: "tactics".into(),
            off: None,
            on: None,
            patch: Some(patch_wire(patch)),
        },
        // The computer manager proposes no rule, level or game-state change; the side's rules
        // make their own, and the board makes the others.
        Change::Rule { .. } | Change::Instructions(_) | Change::GameState(_) => AdvicePick {
            code: code.code().into(),
            kind: "rule".into(),
            off: None,
            on: None,
            patch: None,
        },
    }
}

/// A tactics change as the wire writes it: the reverse of the socket's reading.
fn patch_wire(patch: &TacticsPatch) -> PatchWire {
    PatchWire {
        formation: patch.formation,
        mentality: patch.mentality,
        instructions: patch
            .instructions
            .iter()
            .any(Option::is_some)
            .then_some(patch.instructions),
        roles: patch
            .roles
            .iter()
            .map(|(squad, rd)| RoleWire {
                squad: wire_index(*squad),
                role: rd.role,
                duty: rd.duty,
            })
            .collect(),
    }
}

/// A squad index as the wire writes it. A validated team file holds far fewer players.
fn wire_index(squad: usize) -> u16 {
    u16::try_from(squad).unwrap_or(u16::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine::MatchConfig;
    use engine::gate::StateWriter;
    use std::hash::{DefaultHasher, Hasher};
    use std::path::Path;

    /// The advice drive the viewer's browser test plays, as `scan_for_the_drive_seed` finds it:
    /// the first seed from 1 whose page team gets a substitution pick, the tick of the first
    /// advice that holds one, and that pick's code. The browser test reads the same file.
    const DRIVE: &str = include_str!("../../../e2e/match/advice-drive.json");

    #[derive(serde::Deserialize)]
    struct Drive {
        seed: u64,
        tick: u32,
        code: String,
    }

    fn drive() -> Drive {
        serde_json::from_str(DRIVE).unwrap()
    }

    fn loaded() -> crate::content::Loaded {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
        crate::content::load(Some(&dir), None, None, None).unwrap()
    }

    fn page_match(loaded: &crate::content::Loaded, seed: u64, minutes: u32) -> Simulation {
        let [a, b] = &loaded.teams;
        let config = MatchConfig::new(seed, minutes, &loaded.content, [a, b])
            .unwrap()
            .with_manager(HOME, Manager::Human);
        Simulation::new(config).unwrap()
    }

    /// What one played match leaves: a hash over every tick's gate state bytes, and every
    /// advice message in order.
    struct Played {
        hash: u64,
        advice: Vec<Advice>,
    }

    /// Plays `seed` to full time. With `advise`, the advice drive runs after every step; with
    /// `plant`, the first substitution it picks is also queued into the match, the one write
    /// the advice must never make.
    fn play(loaded: &crate::content::Loaded, seed: u64, advise: bool, plant: bool) -> Played {
        let mut sim = page_match(loaded, seed, 90);
        let mut advisor = Advisor::default();
        let mut writer = StateWriter::new(false);
        let mut hasher = DefaultHasher::new();
        let mut advice = Vec::new();
        let mut planted = false;
        while !sim.is_over() {
            sim.step();
            let events = sim.take_events();
            hasher.write(writer.tick(&sim, &events));
            if advise && let Some(a) = advisor.after_step(&sim, &events) {
                if plant
                    && !planted
                    && let Some(p) = a.picks.iter().find(|p| p.kind == "substitution")
                {
                    planted = true;
                    sim.queue_change(
                        HOME,
                        Change::Substitution {
                            off: usize::from(p.off.unwrap()),
                            on: usize::from(p.on.unwrap()),
                        },
                    );
                }
                advice.push(a);
            }
        }
        Played {
            hash: hasher.finish(),
            advice,
        }
    }

    /// The first advice that holds a substitution pick, and that pick.
    fn first_sub_pick(advice: &[Advice]) -> Option<(&Advice, &AdvicePick)> {
        advice.iter().find_map(|a| {
            a.picks
                .iter()
                .find(|p| p.kind == "substitution" && p.code.starts_with("sub-"))
                .map(|p| (a, p))
        })
    }

    #[test]
    fn the_advice_changes_nothing_in_the_match_and_a_planted_queue_write_would() {
        let loaded = loaded();
        let seed = drive().seed;
        let without = play(&loaded, seed, false, false);
        let with = play(&loaded, seed, true, false);
        assert!(
            !with.advice.is_empty(),
            "the check found no pick to test with"
        );
        assert_eq!(
            with.hash, without.hash,
            "the advice moved a gate state byte"
        );
        // The control: the same run with the one write the advice must never make fails the
        // comparison, so the comparison can fail.
        let planted = play(&loaded, seed, true, true);
        assert_ne!(
            planted.hash, without.hash,
            "a planted queue write went unseen"
        );
    }

    #[test]
    fn the_drive_seed_gets_the_substitution_pick_the_scan_found() {
        let loaded = loaded();
        let drive = drive();
        let played = play(&loaded, drive.seed, true, false);
        let (advice, pick) = first_sub_pick(&played.advice).expect("no substitution pick");
        assert_eq!(
            (advice.tick, pick.code.as_str()),
            (drive.tick, drive.code.as_str())
        );
        assert!(pick.off.is_some() && pick.on.is_some() && pick.patch.is_none());
        // The assistant offers substitutions only.
        assert!(
            played
                .advice
                .iter()
                .all(|a| a.picks.iter().all(|p| p.kind == "substitution"))
        );
        let sim = page_match(&loaded, drive.seed, 90);
        let home = &sim.teams()[HOME];
        assert!(home.bench.contains(&usize::from(pick.on.unwrap())));
        assert!(home.lineup.contains(&usize::from(pick.off.unwrap())));
    }

    #[test]
    #[ignore = "a scan over 20 full matches; run it to write e2e/match/advice-drive.json again"]
    fn scan_for_the_drive_seed() {
        let loaded = loaded();
        for seed in 1..=20 {
            let played = play(&loaded, seed, true, false);
            if let Some((a, p)) = first_sub_pick(&played.advice) {
                // The file `e2e/match/advice-drive.json` holds this line.
                println!(
                    "{{ \"seed\": {seed}, \"tick\": {}, \"minute\": {}, \"code\": \"{}\" }}",
                    a.tick, a.minute, p.code
                );
                return;
            }
            println!("seed {seed}: no substitution pick");
        }
        panic!("no seed in 1 to 20 gives a substitution pick");
    }

    #[test]
    fn the_drive_offers_no_pick_before_a_check_is_due() {
        let loaded = loaded();
        let sim = page_match(&loaded, drive().seed, 90);
        assert!(Advisor::default().after_step(&sim, &[]).is_none());
    }

    #[test]
    fn no_advice_runs_for_a_team_the_computer_manages() {
        let loaded = loaded();
        let [a, b] = &loaded.teams;
        let config = MatchConfig::new(drive().seed, 90, &loaded.content, [a, b]).unwrap();
        let mut sim = Simulation::new(config).unwrap();
        let mut advisor = Advisor::default();
        while !sim.is_over() {
            sim.step();
            let events = sim.take_events();
            assert!(advisor.after_step(&sim, &events).is_none());
        }
    }

    #[test]
    fn a_tactics_change_as_a_pick_carries_its_patch_and_no_players() {
        // The check proposes substitutions only; the wire's tactics kind stays readable.
        let mut patch = TacticsPatch::mentality(3);
        patch.instructions[0] = Some(2);
        let p = pick(&Change::Tactics(patch), AiCode::SubFatigue);
        assert_eq!(p.kind, "tactics");
        assert_eq!((p.off, p.on), (None, None));
        let wire = p.patch.unwrap();
        assert_eq!(wire.mentality, Some(3));
        assert_eq!(
            wire.instructions,
            Some([Some(2), None, None, None, None, None])
        );
        let bare = patch_wire(&TacticsPatch::mentality(1));
        assert_eq!(
            bare.instructions, None,
            "no instruction level, no instruction list"
        );
    }
}
