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
