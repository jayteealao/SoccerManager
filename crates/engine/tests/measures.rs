//! The realism measures on full matches: attached, they change no tick and no event, and
//! their counts add up to the match summary's. Builds only with the `measures` feature:
//!
//! ```text
//! cargo test -p engine --features measures --test measures
//! ```

mod common;

use engine::Simulation;
use engine::gate::StateWriter;
use engine::measures::{
    CHAIN_BIN_S, CHAIN_PASS_BINS, CHAIN_SECOND_BINS, MatchMeasures, UNKNOWN_CELL,
};
use sha2::{Digest, Sha256};

/// One played match: every tick's state digest, its events, its summary, and its measures
/// when attached.
struct Played {
    digests: Vec<[u8; 32]>,
    events: Vec<engine::EngineEvent>,
    summary: engine::Summary,
    measures: Option<MatchMeasures>,
}

/// The seed's 90-minute match on the shipped content and default teams.
fn play(seed: u64, measured: bool) -> Played {
    let content = common::content();
    let [a, b] = common::default_teams(&content);
    let config = engine::MatchConfig::new(seed, 90, &content, [&a, &b]).unwrap();
    let mut sim = Simulation::new(config).unwrap();
    if measured {
        sim.with_measures();
    }
    let mut writer = StateWriter::new(false);
    let mut digests = Vec::new();
    let mut events = Vec::new();
    while !sim.is_over() {
        sim.step();
        if sim.is_over() {
            sim.finish();
        }
        let tick_events = sim.take_events();
        digests.push(Sha256::digest(writer.tick(&sim, &tick_events)).into());
        events.extend(tick_events);
    }
    Played {
        digests,
        events,
        summary: sim.summary(),
        measures: sim.measures().cloned(),
    }
}

/// The measures read the match and write nothing: attached, every tick's state digest, every
/// event and the summary equal those of the same seed without them; and the counts add up.
#[test]
fn the_measures_change_no_tick_and_add_up_to_the_summary() {
    let mut scored = 0;
    for seed in 1..=4 {
        let off = play(seed, false);
        let on = play(seed, true);
        assert!(off.measures.is_none());
        assert_eq!(
            off.digests.len(),
            on.digests.len(),
            "seed {seed}: tick count"
        );
        if let Some(t) = (0..off.digests.len()).find(|&t| off.digests[t] != on.digests[t]) {
            panic!("seed {seed}: tick {} differs with the measures on", t + 1);
        }
        assert_eq!(off.events, on.events, "seed {seed}: events");
        assert_eq!(off.summary, on.summary, "seed {seed}: summary");
        let (s, m) = (on.summary, on.measures.unwrap());
        scored += s.goals[0] + s.goals[1];
        for t in 0..2 {
            let ctx = format!("seed {seed} side {t}");
            assert_eq!(
                m.shot_bins[t].iter().sum::<u32>(),
                s.shots[t],
                "{ctx}: shots"
            );
            let attempts: u32 = m.pass_attempts[t].iter().sum();
            let completed: u32 = m.pass_completed[t].iter().sum();
            assert_eq!(attempts, s.passes[t], "{ctx}: passes");
            assert_eq!(completed, s.passes_completed[t], "{ctx}: completed passes");
            assert_eq!(m.pass_attempts[t][UNKNOWN_CELL], 0, "{ctx}: unknown passes");
            for cell in 0..m.pass_attempts[t].len() {
                assert!(
                    m.pass_completed[t][cell] <= m.pass_attempts[t][cell],
                    "{ctx}: cell {cell}"
                );
            }
            assert_eq!(
                m.goals_by_phase[t].iter().sum::<u32>(),
                s.goals[t],
                "{ctx}: goals"
            );
            assert_eq!(
                m.third_ticks[t].iter().sum::<u32>(),
                s.possession_ticks[t],
                "{ctx}: thirds"
            );
            assert!(m.chains[t] > 0, "{ctx}: no chain");
            assert_eq!(
                m.chain_pass_bins[t].iter().sum::<u32>(),
                m.chains[t],
                "{ctx}"
            );
            assert_eq!(
                m.chain_second_bins[t].iter().sum::<u32>(),
                m.chains[t],
                "{ctx}"
            );
            // The sums sit inside the bounds the bins give.
            let low_passes: u32 = (0..CHAIN_PASS_BINS)
                .map(|b| b as u32 * m.chain_pass_bins[t][b])
                .sum();
            assert!(m.chain_passes[t] >= low_passes, "{ctx}: chain passes");
            let bin_ticks = u64::from(CHAIN_BIN_S * engine::TICKS_PER_SECOND);
            let (low, high): (u64, u64) = (0..CHAIN_SECOND_BINS).fold((0, 0), |(lo, hi), b| {
                let n = u64::from(m.chain_second_bins[t][b]);
                // The overflow bin has no upper bound.
                let top = if b + 1 == CHAIN_SECOND_BINS && n > 0 {
                    u64::MAX / 4
                } else {
                    (b as u64 + 1) * bin_ticks * n
                };
                (lo + b as u64 * bin_ticks * n, hi.saturating_add(top))
            });
            assert!(
                (low..=high).contains(&m.chain_ticks[t]),
                "{ctx}: chain ticks {} outside {low}..={high}",
                m.chain_ticks[t]
            );
            // Every shot is less than a pitch length from goal.
            assert!(
                m.shot_distance_m[t] <= f64::from(s.shots[t]) * 120.0,
                "{ctx}"
            );
        }
        assert!(
            m.pass_attempts.iter().flatten().sum::<u32>() > 0,
            "seed {seed}: a real match passes"
        );
    }
    assert!(
        scored > 0,
        "seeds 1 to 4 score, so goal phases are exercised"
    );
}

/// The team-shape figures read the match and write nothing: driven beside a match, every
/// tick's state digest and every event equal those of the same seed without them; and their
/// counts add up.
#[test]
fn the_shape_figures_change_no_tick_and_add_up() {
    use engine::TickSink;
    use engine::measures::shape::{GROUPS, ShapeSink, field_names};
    let names = field_names();
    let at = |name: &str| names.iter().position(|n| n == name).unwrap();
    for seed in [1, 2] {
        let off = play(seed, false);
        let content = common::content();
        let [a, b] = common::default_teams(&content);
        let config = engine::MatchConfig::new(seed, 90, &content, [&a, &b]).unwrap();
        let mut sim = Simulation::new(config).unwrap();
        let mut shape = ShapeSink::default();
        let mut writer = StateWriter::new(false);
        let (mut digests, mut events) = (Vec::<[u8; 32]>::new(), Vec::new());
        let mut stoppages = 0.0;
        while !sim.is_over() {
            sim.step();
            shape.on_step(&sim).unwrap();
            if let Some(stoppage) = sim.stoppage() {
                stoppages += 1.0;
                shape.on_stoppage(&stoppage, &sim).unwrap();
            }
            if sim.is_over() {
                sim.finish();
            }
            let tick_events = sim.take_events();
            digests.push(Sha256::digest(writer.tick(&sim, &tick_events)).into());
            events.extend(tick_events);
        }
        assert!(
            off.digests == digests,
            "seed {seed}: a tick differs with the shape figures on"
        );
        assert_eq!(off.events, events, "seed {seed}: events");
        let summary = sim.summary();
        let v = shape.finish();
        let mut stops = 0.0;
        for (t, v) in v.iter().enumerate() {
            let ctx = format!("seed {seed} side {t}");
            let mut live = 0.0;
            for g in GROUPS {
                let ticks = v[at(&format!("{g}.live_ticks"))];
                let paced: f64 = ["stand", "walk", "jog", "run", "hs", "sprint"]
                    .iter()
                    .map(|p| v[at(&format!("{g}.pace_{p}"))])
                    .sum();
                assert_eq!(paced, ticks, "{ctx} {g}: every live sample has one pace");
                live += ticks;
            }
            // Ten outfield players at most, on the live ticks.
            assert!(live > 0.0, "{ctx}: no live sample");
            assert!(
                live <= 10.0 * f64::from(summary.live_ticks),
                "{ctx}: {live} samples"
            );
            assert!(v[at("calm_ticks")] <= v[at("dead_ticks")], "{ctx}: calm");
            assert!(v[at("dead_ticks")] > 0.0, "{ctx}: no dead sample");
            let total = v[at("links_total")];
            assert!(total > 0.0, "{ctx}: no link");
            assert!(v[at("links_top2")] <= total, "{ctx}: top two");
            assert!(v[at("links_back")] <= total, "{ctx}: back line");
            for k in [
                "throw_in",
                "free_kick",
                "goal_kick",
                "corner",
                "kick_off",
                "penalty",
            ] {
                stops += v[at(&format!("stops_{k}"))];
            }
            // The restarts the summary counts, by kind.
            for (k, n) in [
                ("throw_in", summary.throw_ins[t]),
                ("goal_kick", summary.goal_kicks[t]),
                ("corner", summary.corners[t]),
                ("free_kick", summary.free_kicks[t]),
            ] {
                assert_eq!(v[at(&format!("stops_{k}"))], f64::from(n), "{ctx}: {k}");
            }
        }
        assert!(
            stops > 0.0 && stops <= stoppages,
            "seed {seed}: {stops} of {stoppages} stops"
        );
    }
}

/// A seed's match on the shipped content: its open-play passes, the passes their own passer
/// gained back first, those the keeper took back, and those he caught still on their way
/// (faster than a pass arrives).
fn self_regains(seed: u64) -> [u32; 4] {
    let content = common::content();
    let [a, b] = common::default_teams(&content);
    let config = engine::MatchConfig::new(seed, 90, &content, [&a, &b]).unwrap();
    let mut sim = Simulation::new(config).unwrap();
    sim.with_measures();
    while !sim.is_over() {
        sim.step();
        if sim.is_over() {
            sim.finish();
        }
        sim.take_events();
    }
    let m = sim.measures().cloned().unwrap();
    [
        m.pass_attempts.iter().flatten().sum(),
        m.self_regains.iter().sum(),
        m.keeper_self_regains.iter().sum(),
        m.keeper_self_catches.iter().sum(),
    ]
}

/// A kicker does not take back his own pass while it runs on along the line he sent it: over
/// eight seeded matches the keepers catch fewer than 5 of their own passes a match still on
/// their way, and passers gain back first fewer than 5 percent of open-play passes. A pass
/// that has slowed to a pass's arrival speed with nobody else on it is his to collect.
#[test]
fn a_kicker_does_not_take_back_his_own_pass_on_its_way() {
    let mut total = [0u32; 4];
    let seeds = 1..=8u64;
    let n = seeds.clone().count() as f64;
    for seed in seeds {
        for (t, x) in total.iter_mut().zip(self_regains(seed)) {
            *t += x;
        }
    }
    let share = f64::from(total[1]) / f64::from(total[0]);
    let catches = f64::from(total[3]) / n;
    assert!(catches < 5.0, "keeper own catches {catches:.2} a match");
    assert!(share < 0.05, "self-regained share {share:.4}");
}

/// The same figures over many matches, for the record: `SM_SELF_REGAIN_MATCHES` matches
/// (default 1000) from seed `SM_SELF_REGAIN_SEED` (default 9) times a million on, on all
/// cores.
///
/// ```text
/// cargo test --release -p engine --features measures --test measures -- --ignored self_regains_over_many --nocapture
/// ```
#[test]
#[ignore = "a long run for the record"]
fn self_regains_over_many_matches() {
    let var = |name: &str, default: u64| {
        std::env::var(name)
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(default)
    };
    let matches = var("SM_SELF_REGAIN_MATCHES", 1000);
    let base = var("SM_SELF_REGAIN_SEED", 9) * 1_000_000;
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get()) as u64;
    let next = std::sync::atomic::AtomicU64::new(0);
    let totals = std::sync::Mutex::new([0u64; 4]);
    std::thread::scope(|s| {
        for _ in 0..threads {
            s.spawn(|| {
                loop {
                    let k = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    if k >= matches {
                        break;
                    }
                    let one = self_regains(base + k);
                    let mut t = totals.lock().unwrap();
                    for (t, x) in t.iter_mut().zip(one) {
                        *t += u64::from(x);
                    }
                }
            });
        }
    });
    let [passes, own, keeper, catches] = totals.into_inner().unwrap();
    let per = |x: u64| x as f64 / matches as f64;
    println!(
        "matches {matches} seed base {base}: passes {:.1} a match, self-regained {:.2} a match \
         (share {:.4}), keeper own regains {:.2} a match, keeper own catches on their way {:.2} \
         a match",
        per(passes),
        per(own),
        own as f64 / passes as f64,
        per(keeper),
        per(catches)
    );
}
