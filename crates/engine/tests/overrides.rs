//! Trait overrides in play: a strong trait that an instruction held for its player goes
//! against may break the instruction, with the player pressed or low in mood, by one draw on
//! his own key. The tests play each pair of the conflict table — the trait strong on a home
//! player in a fitting slot and the instruction held — and read the job probe's moments and
//! draws.
//!
//! The rate batch plays 40 matches a pair, so it is ignored and runs in release:
//!
//! ```text
//! cargo test --release -p engine --features sensitivity --test overrides -- --ignored --nocapture
//! ```
#![cfg(feature = "sensitivity")]

mod common;

use common::{content, default_teams, index};
use engine::data::tactics::{INSTRUCTIONS, TEMPO};
use engine::data::traits::TraitStrength;
use engine::math::DVec2;
use engine::record::RECORD_BYTES;
use engine::scenario::Scene;
use engine::traits::{OverrideCause, TraitKind};
use engine::{
    EngineError, EngineEvent, EngineEventKind, EventDetail, Manager, MatchConfig, Simulation,
    Snapshot, Stoppage, TickRecord, TickSink, VecSink,
};

/// One pair of the conflict table: the trait, the home formation slot that carries it, and
/// the instruction held against it.
struct Pair {
    kind: TraitKind,
    slot: usize,
    instruction: &'static str,
}

/// The pairs, each on a player whose place in the shipped 4-4-2 meets its moments: the keeper
/// for the keeper's traits, a wide player for the wide ones, a striker for the shot and the
/// hold-up play, a central midfielder for the rest.
fn pairs() -> Vec<Pair> {
    use TraitKind as T;
    let p = |kind, slot, instruction| Pair {
        kind,
        slot,
        instruction,
    };
    let all = vec![
        p(T::TriesKillerBalls, 6, "passing_directness:short"),
        p(T::TriesLobsAndChips, 6, "passing_directness:short"),
        p(T::CrossesEarly, 5, "passing_directness:short"),
        p(T::KicksLong, 0, "passing_directness:short"),
        p(T::PlaysShortSimplePasses, 6, "passing_directness:direct"),
        p(T::PlaysOneTwos, 9, "passing_directness:direct"),
        p(T::ThrowsShort, 0, "passing_directness:direct"),
        p(T::DwellsOnBall, 6, "tempo:faster"),
        p(T::RunsWithBallOften, 5, "tempo:faster"),
        p(T::HoldsUpBall, 9, "tempo:faster"),
        p(T::TriesTricks, 5, "tempo:faster"),
        p(T::ReleasesBallEarly, 6, "tempo:slower"),
        p(T::CutsInside, 5, "width:wide"),
        p(T::HugsTouchline, 8, "width:narrow"),
        p(T::SwitchesPlay, 6, "width:narrow"),
        p(T::PressesRelentlessly, 9, "pressing:low"),
        p(T::GetsForward, 1, "duty:defend"),
        p(T::ArrivesLateInBox, 6, "duty:defend"),
        p(T::StaysBack, 6, "duty:attack"),
        p(T::ShootsFromDistance, 9, "mentality:cautious"),
        p(T::RushesOut, 0, "line_height:deep"),
        p(T::StaysOnLine, 0, "line_height:high"),
    ];
    all.into_iter()
        .filter(|p| !p.kind.conflicts().is_empty())
        .collect()
}

/// The pair for `kind`.
fn pair(kind: TraitKind) -> Pair {
    pairs()
        .into_iter()
        .find(|p| p.kind == kind)
        .unwrap_or_else(|| panic!("{} conflicts with no instruction", kind.name()))
}

/// The seed's match on the shipped content and default teams, both sides managed by a person
/// so no in-match change moves the instruction: the pair's player carries the trait at
/// `strength`, his side holds the instruction, and the trait's chance is `chance` when given.
fn pair_config(
    seed: u64,
    minutes: u32,
    pair: &Pair,
    strength: Option<TraitStrength>,
    chance: Option<f64>,
) -> MatchConfig {
    let content = content();
    let [a, b] = default_teams(&content);
    let mut config = MatchConfig::new(seed, minutes, &content, [&a, &b]).unwrap();
    config.managers = [Manager::Human; 2];
    let i = index(0, pair.slot);
    let squad = config.players[i].squad;
    config.teams[0].squad[squad].set_trait(pair.kind, strength);
    hold(&mut config, pair.slot, pair.instruction);
    if let Some(c) = chance {
        let traits = config
            .tuning
            .contract
            .traits
            .as_mut()
            .expect("shipped block");
        *traits
            .override_chance
            .of_mut(pair.kind)
            .expect("a conflicting trait") = c;
    }
    config
}

/// The home side holds the instruction `code` for the player in `slot`.
fn hold(config: &mut MatchConfig, slot: usize, code: &str) {
    let (name, level) = code.split_once(':').expect("an instruction code");
    let schema = config.tactics.clone();
    let mut tactics = config.teams[0].tactics;
    match name {
        "duty" => tactics.roles[slot].duty = schema.duty_index(level).unwrap() as u8,
        "mentality" => tactics.mentality = schema.mentality_index(level).unwrap() as u8,
        _ => {
            let at = INSTRUCTIONS.iter().position(|n| *n == name).unwrap();
            let l = schema.instructions.level_names(at);
            tactics.instructions[at] = l.iter().position(|n| *n == level).unwrap() as u8;
        }
    }
    let tuning = config.tuning.clone();
    config.teams[0].set_tactics(tactics, &schema, &tuning);
}

/// One played match: its events, and the pair's player's moments met and overrides taken.
struct Played {
    events: Vec<EngineEvent>,
    moments: u32,
    overrides: u32,
}

fn play(config: MatchConfig, i: usize) -> Played {
    let mut sim = Simulation::new(config).unwrap();
    sim.with_probe();
    while !sim.is_over() {
        sim.step();
    }
    sim.finish();
    let probe = sim.probe().expect("probe attached");
    let (moments, overrides) = (probe.trait_moments[i], probe.trait_overrides[i]);
    Played {
        events: sim.take_events(),
        moments,
        overrides,
    }
}

/// The override events of the match.
fn overrides(events: &[EngineEvent]) -> Vec<&EngineEvent> {
    events
        .iter()
        .filter(|e| e.kind == EngineEventKind::TraitOverride)
        .collect()
}

/// The trait and cause an override event names.
fn named(e: &EngineEvent) -> (TraitKind, OverrideCause) {
    match e.detail {
        Some(EventDetail::Override {
            trait_kind, cause, ..
        }) => (trait_kind, cause),
        ref other => panic!("an override without its detail: {other:?}"),
    }
}

/// The override rate: each pair's player, the trait strong and the instruction held, takes
/// 2 to 4 overrides a match on average over 40 seeds at the shipped chance; every override
/// names him, the trait, the instruction and a cause.
#[test]
#[ignore]
fn each_conflicting_trait_overrides_two_to_four_times_a_match() {
    const SEEDS: u64 = 40;
    let mut failed = Vec::new();
    for pair in pairs() {
        let i = index(0, pair.slot);
        let (mut moments, mut taken, mut events, mut low) = (0u32, 0u32, 0usize, 0usize);
        for seed in 0..SEEDS {
            let played = play(
                pair_config(seed, 90, &pair, Some(TraitStrength::Strong), None),
                i,
            );
            moments += played.moments;
            taken += played.overrides;
            let found = overrides(&played.events);
            events += found.len();
            for e in found {
                assert_eq!(e.player, Some(i), "{}", pair.kind.name());
                assert_eq!(e.team, Some(0));
                let (kind, cause) = named(e);
                assert_eq!(kind, pair.kind);
                if cause == OverrideCause::LowMood {
                    low += 1;
                }
                let Some(EventDetail::Override { instruction, .. }) = e.detail else {
                    unreachable!()
                };
                assert_eq!(instruction.code(), pair.instruction);
            }
        }
        assert_eq!(
            events as u32,
            taken,
            "{}: every override is an event",
            pair.kind.name()
        );
        let (m, o) = (
            f64::from(moments) / SEEDS as f64,
            f64::from(taken) / SEEDS as f64,
        );
        println!(
            "{:<26} {:<28} slot {:>2}  moments {:>6.2}  overrides {:>5.2}  low mood {:>3}",
            pair.kind.name(),
            pair.instruction,
            pair.slot,
            m,
            o,
            low
        );
        if !(2.0..=4.0).contains(&o) {
            failed.push(format!("{}: {o:.2}", pair.kind.name()));
        }
    }
    assert!(
        failed.is_empty(),
        "pairs outside 2 to 4 a match: {failed:?}"
    );
}

/// The moments each pair meets a match at chance 1 over `seeds` seeds: the measurement that
/// sets the chances and decides which pairs can reach the target at all.
#[test]
#[ignore]
fn the_moments_each_pair_meets_at_chance_one() {
    const SEEDS: u64 = 20;
    for pair in pairs() {
        let i = index(0, pair.slot);
        let mut moments = 0u32;
        for seed in 0..SEEDS {
            moments += play(
                pair_config(seed, 90, &pair, Some(TraitStrength::Strong), Some(0.02)),
                i,
            )
            .moments;
        }
        println!(
            "{:<26} {:<28} moments a match at chance 1: {:>6.2}",
            pair.kind.name(),
            pair.instruction,
            f64::from(moments) / SEEDS as f64
        );
    }
}

/// No override without a strong trait: with the trait absent or only moderate, a planted
/// chance of 1 never overrides, and the player meets no moment.
#[test]
fn no_trait_and_a_moderate_trait_never_override() {
    for pair in pairs() {
        let i = index(0, pair.slot);
        for seed in 0..6 {
            for strength in [None, Some(TraitStrength::Moderate)] {
                let played = play(pair_config(seed, 15, &pair, strength, Some(1.0)), i);
                assert_eq!(
                    (played.moments, overrides(&played.events).len()),
                    (0, 0),
                    "{} at {strength:?}, seed {seed}",
                    pair.kind.name()
                );
            }
        }
    }
}

/// The frequent pair the chance-1 checks play: a striker who holds the ball up against a
/// faster tempo.
const FREQUENT: TraitKind = TraitKind::HoldsUpBall;

/// No count limit: at chance 1 every moment met overrides, the counts differ between
/// matches, and a frequent pair takes more than 10 in a match.
#[test]
fn at_chance_one_every_moment_met_overrides() {
    let pair = pair(FREQUENT);
    let i = index(0, pair.slot);
    let mut counts = Vec::new();
    for seed in 0..4 {
        let played = play(
            pair_config(seed, 90, &pair, Some(TraitStrength::Strong), Some(1.0)),
            i,
        );
        assert_eq!(played.moments, played.overrides, "seed {seed}");
        assert_eq!(overrides(&played.events).len() as u32, played.overrides);
        counts.push(played.overrides);
    }
    println!("overrides a match at chance 1: {counts:?}");
    assert!(counts.iter().any(|&n| n != counts[0]), "{counts:?}");
    assert!(counts.iter().any(|&n| n > 10), "{counts:?}");
}

/// No cooldown and no phase exclusion: at chance 1, where every moment met overrides, the
/// frequent pair's player overrides again as soon as his override's carry ends and he meets
/// the next release (within 3 s: the 1.5 s carry and the next decision), under pressure,
/// and in added time.
#[test]
fn overrides_come_back_to_back_and_in_added_time() {
    let pair = pair(FREQUENT);
    let i = index(0, pair.slot);
    let mut back_to_back = false;
    let mut added = false;
    let mut pressure = false;
    for seed in 0..4 {
        let played = play(
            pair_config(seed, 90, &pair, Some(TraitStrength::Strong), Some(1.0)),
            i,
        );
        let found = overrides(&played.events);
        back_to_back |= found
            .windows(2)
            .any(|w| w[1].tick - w[0].tick <= 3 * engine::TICKS_PER_SECOND);
        added |= found.iter().any(|e| e.minute_added.is_some());
        pressure |= found.iter().any(|e| named(e).1 == OverrideCause::Pressure);
    }
    assert!(back_to_back, "no two overrides within 3 s");
    assert!(added, "no override in added time");
    assert!(pressure, "no override under pressure");
}

/// An anchor override lasts one spell of his side on the ball. Whenever a player's anchor
/// comes into force as his side gains the ball, he won an override on that tick: an anchor
/// left from an earlier spell, with a restart or a kick-off to the other side in between,
/// never comes back with no draw and no event.
#[test]
fn an_anchor_from_an_earlier_spell_never_comes_back_without_a_draw() {
    let pair = pair(TraitKind::GetsForward);
    let i = index(0, pair.slot);
    let mut starts = 0;
    // Seeds 0 to 4: seed 4 is the lowest whose match brings an anchor into force once the
    // player as a person joined the team shape (seeds 0 to 2 were enough before).
    for seed in 0..5 {
        let config = pair_config(seed, 90, &pair, Some(TraitStrength::Strong), Some(1.0));
        let mut sim = Simulation::new(config).unwrap();
        let (mut was_live, mut read) = (false, 0);
        while !sim.is_over() {
            sim.step();
            let events = sim.events();
            let won = events[read..]
                .iter()
                .any(|e| e.kind == EngineEventKind::TraitOverride && e.player == Some(i));
            read = events.len();
            let live = sim.trait_act(i).is_some();
            let gained = sim.carrier().is_some_and(|c| sim.players()[c].team == 0);
            if live && !was_live && gained {
                starts += 1;
                assert!(
                    won,
                    "seed {seed}, tick {}: his anchor came into force with no override",
                    sim.tick()
                );
            }
            was_live = live;
        }
    }
    assert!(starts > 0, "no anchor came into force");
}

/// A scene: home midfielder 6 carries `releases_ball_early` strong at mood `mood`, his side
/// plays at a slower tempo, its chance is 1, and he has just gained the ball at (-15, 0) with
/// no opponent near him: the override events of his first two seconds on the ball.
fn first_touch_scene(mood: f64) -> Vec<EngineEvent> {
    let me = index(0, 6);
    let mut config = common::short_match(10);
    config.managers = [Manager::Human; 2];
    *config
        .tuning
        .contract
        .traits
        .as_mut()
        .unwrap()
        .override_chance
        .of_mut(TraitKind::ReleasesBallEarly)
        .unwrap() = 1.0;
    let at = DVec2::new(-15.0, 0.0);
    let mut sim = common::spread(Scene::new(config), -30.0, 30.0)
        .tick(1_001)
        .place(me, at)
        .ball(engine::math::DVec3::new(at.x, at.y, 0.0))
        .carrier(Some(me))
        .control_since(1_000)
        .with_trait(me, "releases_ball_early", Some(TraitStrength::Strong))
        .instruction(0, TEMPO, "slower")
        .mood(me, mood)
        .build();
    for _ in 0..2 * engine::TICKS_PER_SECOND {
        sim.step();
    }
    sim.take_events()
        .into_iter()
        .filter(|e| e.kind == EngineEventKind::TraitOverride)
        .collect()
}

/// Both causes: unpressed at mood -50 he overrides on his first decision and the event names
/// low mood; at a neutral mood, unpressed, he meets no moment.
#[test]
fn an_unpressed_player_overrides_on_low_mood_and_not_at_a_neutral_mood() {
    let low = first_touch_scene(-50.0);
    assert_eq!(low.len(), 1, "{low:?}");
    assert_eq!(low[0].player, Some(index(0, 6)));
    assert_eq!(
        named(&low[0]),
        (TraitKind::ReleasesBallEarly, OverrideCause::LowMood)
    );
    assert!(first_touch_scene(0.0).is_empty());
}

/// Keeps every record and captures a snapshot at the first stoppage after the player's first
/// override.
struct Capture {
    player: usize,
    records: Vec<TickRecord>,
    snapshot: Option<Snapshot>,
}

impl TickSink for Capture {
    fn on_tick(&mut self, record: &TickRecord) -> Result<(), EngineError> {
        self.records.push(*record);
        Ok(())
    }

    fn on_stoppage(&mut self, _stoppage: &Stoppage, sim: &Simulation) -> Result<(), EngineError> {
        let overridden = sim
            .events()
            .iter()
            .any(|e| e.kind == EngineEventKind::TraitOverride && e.player == Some(self.player));
        if self.snapshot.is_none() && overridden {
            self.snapshot = Some(Snapshot::capture(sim, [3; 16], 1_700_000_000_000));
        }
        Ok(())
    }
}

fn bytes(records: &[TickRecord]) -> Vec<u8> {
    let mut out = Vec::with_capacity(records.len() * RECORD_BYTES);
    let mut buf = [0u8; RECORD_BYTES];
    for record in records {
        record.write_to(&mut buf);
        out.extend_from_slice(&buf);
    }
    out
}

/// A match resumed from a snapshot taken while an anchor override is in force plays on tick
/// for tick as the unbroken match: the override's state comes back from the snapshot.
#[test]
fn a_match_resumed_during_an_override_continues_it() {
    let pair = pair(TraitKind::ArrivesLateInBox);
    let i = index(0, pair.slot);
    let config = || pair_config(3, 90, &pair, Some(TraitStrength::Strong), Some(1.0));
    let mut capture = Capture {
        player: i,
        records: Vec::new(),
        snapshot: None,
    };
    let mut whole = Simulation::new(config()).unwrap();
    whole.run(&mut capture).unwrap();
    let snapshot = capture.snapshot.expect("a stoppage after an override");
    let from = snapshot.tick();
    let read = Snapshot::from_bytes(&snapshot.to_bytes(), "snapshot.smsn").unwrap();
    let again = Snapshot::from_bytes(&read.to_bytes(), "snapshot.smsn").unwrap();
    assert!(
        read.to_bytes() == again.to_bytes(),
        "the round trip changed the bytes"
    );
    let mut resumed = Simulation::from_snapshot(config(), &read).unwrap();
    let mut tail = VecSink::default();
    resumed.run(&mut tail).unwrap();
    let expected: Vec<TickRecord> = capture
        .records
        .iter()
        .copied()
        .filter(|r| r.tick > from)
        .collect();
    assert!(!expected.is_empty());
    assert!(
        bytes(&tail.records) == bytes(&expected),
        "the resumed match differs from the unbroken one"
    );
    let later = |events: &[EngineEvent]| {
        events
            .iter()
            .filter(|e| e.tick > from)
            .cloned()
            .collect::<Vec<_>>()
    };
    let whole_events = whole.take_events();
    assert!(
        later(&whole_events)
            .iter()
            .any(|e| e.kind == EngineEventKind::TraitOverride),
        "no override after the snapshot"
    );
    assert_eq!(later(resumed.events()), later(&whole_events));
    assert_eq!(resumed.summary(), whole.summary());
}
