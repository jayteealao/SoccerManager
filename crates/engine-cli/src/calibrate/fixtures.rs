//! The fixture list of a calibration run: generated leagues of 20 clubs, each playing a
//! double round-robin, with a seed per match, the stronger club of the strength suite, and
//! the formation pairing of the formations suite.

use engine::Content;
use engine::data::TeamFile;
use engine::data::generator::generate_league;
use engine::observe::identity::MatchId;

use crate::report::Suite;

/// Clubs per generated league.
pub const CLUBS: u32 = 20;
/// Fixtures of one league's double round-robin.
pub const FIXTURES_PER_LEAGUE: u32 = CLUBS * (CLUBS - 1);

/// One match of the run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fixture {
    /// The fixture's number in the run, from 0.
    pub index: u32,
    /// The league it belongs to: league `k` is generated from `seed + k`.
    pub league: u32,
    /// The two clubs' numbers inside the league, home first.
    pub clubs: [usize; 2],
}

impl Fixture {
    /// The side of the boosted club in the strength suite: home on even fixtures, away on
    /// odd ones, so any advantage of playing at home cancels out.
    pub fn boosted_side(&self) -> usize {
        (self.index % 2) as usize
    }
}

/// The first `matches` fixtures of the run.
pub fn fixtures(matches: u32) -> Vec<Fixture> {
    let one = double_round_robin(CLUBS as usize);
    (0..matches)
        .map(|index| Fixture {
            index,
            league: index / FIXTURES_PER_LEAGUE,
            clubs: one[(index % FIXTURES_PER_LEAGUE) as usize],
        })
        .collect()
}

/// Every pairing of `n` formations, a formation against itself included: each `[a, b]` with
/// `a <= b`, in order. Ten formations give 55 pairings.
pub fn pairings(n: usize) -> Vec<[usize; 2]> {
    (0..n).flat_map(|a| (a..n).map(move |b| [a, b])).collect()
}

/// One match of the formations suite.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FormationFixture {
    /// The match, with the clubs of the equal suite's fixture at the same place in its
    /// pairing.
    pub fixture: Fixture,
    /// The pairing's number in [`pairings`].
    pub pairing: usize,
    /// The side of the pairing's first formation: home on even places, away on odd ones.
    pub first_side: usize,
}

/// The fixtures of the formations suite: `matches` matches for each of `pairings`
/// pairings. Match `i` belongs to pairing `i / matches`, at place `i % matches`.
pub fn formation_fixtures(matches: u32, pairings: usize) -> Vec<FormationFixture> {
    let equal = fixtures(matches);
    // At most 16 formations give 136 pairings.
    (0..matches * pairings as u32)
        .map(|index| {
            let local = index % matches;
            FormationFixture {
                fixture: Fixture {
                    index,
                    ..equal[local as usize]
                },
                pairing: (index / matches) as usize,
                first_side: (local % 2) as usize,
            }
        })
        .collect()
}

/// The fixtures of the selected pairings only, in order, each with the index, the match
/// seed, the clubs, and the home side it has in the full list of [`formation_fixtures`]. So
/// a targeted run plays the same matches as the same pairings of a full run.
pub fn formation_fixtures_for(
    matches: u32,
    pairings: usize,
    selected: &[usize],
) -> Vec<FormationFixture> {
    formation_fixtures(matches, pairings)
        .into_iter()
        .filter(|f| selected.contains(&f.pairing))
        .collect()
}

/// One match of the red-card suite.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RedCardFixture {
    /// The fixture's number in the suite, from 0: arm `a` plays `a x matches` onward.
    pub index: u32,
    /// The arm's number in [`crate::report::RED_CARD_ARMS`].
    pub arm: usize,
    /// The engine seed, used directly: each seed from `seed` onward plays twice in a row.
    pub engine_seed: u64,
    /// `true` when the default clubs play in the other order: the second club at home.
    pub swapped: bool,
}

/// The fixtures of the red-card suite: every arm plays each engine seed from `seed` onward
/// twice on the default clubs, first in their usual order and then with home and away
/// swapped. So each club is the reduced side in half the matches, and a difference in club
/// strength cancels out. `--seed 1 --matches 240` is the slow test's experiment (seeds 1 to
/// 120 in both orders). An odd `--matches` plays the last seed in the usual order only.
pub fn red_card_fixtures(seed: u64, matches: u32) -> Vec<RedCardFixture> {
    let arms = crate::report::RED_CARD_ARMS.len();
    (0..arms)
        .flat_map(|arm| {
            (0..matches).map(move |k| RedCardFixture {
                // Four arms of at most 100 000 matches each.
                index: arm as u32 * matches + k,
                arm,
                engine_seed: seed.wrapping_add(u64::from(k / 2)),
                swapped: !k.is_multiple_of(2),
            })
        })
        .collect()
}

/// A double round-robin of `n` clubs (`n` even) by the circle method: `n - 1` rounds in
/// which every club plays once, then the same rounds with home and away swapped. The first
/// fixtures already spread over every club, so a run shorter than a league stays balanced.
pub fn double_round_robin(n: usize) -> Vec<[usize; 2]> {
    let mut first = Vec::with_capacity(n * (n - 1) / 2);
    for round in 0..n - 1 {
        for i in 0..n / 2 {
            let a = if i == 0 { n - 1 } else { (round + i) % (n - 1) };
            let b = (round + n - 1 - i) % (n - 1);
            // Alternate who is at home so no club hosts every round.
            first.push(if (round + i) % 2 == 0 { [a, b] } else { [b, a] });
        }
    }
    let second: Vec<[usize; 2]> = first.iter().map(|&[h, a]| [a, h]).collect();
    first.extend(second);
    first
}

/// SplitMix64: a well-mixed 64-bit value from `x`.
pub fn splitmix64(x: u64) -> u64 {
    let mut z = x.wrapping_add(0x9e37_79b9_7f4a_7c15);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

/// The engine seed of fixture `index` in `suite`.
pub fn match_seed(run_seed: u64, suite: Suite, index: u32) -> u64 {
    splitmix64(run_seed ^ (suite.number() << 32) ^ u64::from(index))
}

/// The match identifier of fixture `index` in `suite`; `millis` is the run's start.
pub fn match_id(run_seed: u64, suite: Suite, index: u32, millis: u64) -> String {
    MatchId {
        seed: match_seed(run_seed, suite, index),
        millis,
    }
    .to_string()
}

/// A copy of `file` with every attribute times `boost`, rounded and clamped to 1 to 100.
pub fn boosted(file: &TeamFile, boost: f64) -> TeamFile {
    let mut out = file.clone();
    for p in &mut out.players {
        for v in p.attributes.values_mut() {
            // Clamped to 1 to 100 first, so the cast cannot truncate.
            *v = (f64::from(*v) * boost).round().clamp(1.0, 100.0) as u8;
        }
    }
    out
}

/// The generated leagues a worker needs, generated once each.
pub struct Leagues<'a> {
    seed: u64,
    content: &'a Content,
    cache: Vec<Option<Vec<TeamFile>>>,
}

impl<'a> Leagues<'a> {
    pub fn new(seed: u64, content: &'a Content) -> Self {
        Self {
            seed,
            content,
            cache: Vec::new(),
        }
    }

    /// The two team files of `fixture`, home first.
    pub fn teams(&mut self, fixture: &Fixture) -> [TeamFile; 2] {
        let k = fixture.league as usize;
        if self.cache.len() <= k {
            self.cache.resize(k + 1, None);
        }
        let league = self.cache[k].get_or_insert_with(|| {
            generate_league(
                self.seed.wrapping_add(u64::from(fixture.league)),
                CLUBS,
                self.content,
            )
        });
        [
            league[fixture.clubs[0]].clone(),
            league[fixture.clubs[1]].clone(),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_double_round_robin_plays_every_ordered_pair_once() {
        let list = double_round_robin(20);
        assert_eq!(list.len(), 380);
        let mut seen = std::collections::BTreeSet::new();
        for [h, a] in &list {
            assert_ne!(h, a);
            assert!(seen.insert((*h, *a)), "{h} v {a} twice");
        }
        // Every club plays once in each of the first 19 rounds.
        for round in list[..190].chunks(10) {
            let mut clubs: Vec<usize> = round.iter().flatten().copied().collect();
            clubs.sort_unstable();
            assert_eq!(clubs, (0..20).collect::<Vec<_>>());
        }
    }

    #[test]
    fn the_same_seed_gives_the_same_fixtures_and_seeds() {
        assert_eq!(fixtures(1000), fixtures(1000));
        let f = fixtures(1000);
        assert_eq!(f[379].league, 0);
        assert_eq!(f[380].league, 1);
        assert_eq!(f[999].league, 2);
        assert_eq!(
            match_seed(7, Suite::Equal, 3),
            match_seed(7, Suite::Equal, 3)
        );
        assert_ne!(
            match_seed(7, Suite::Equal, 3),
            match_seed(7, Suite::Strength, 3)
        );
        assert_ne!(
            match_seed(7, Suite::Equal, 3),
            match_seed(7, Suite::Equal, 4)
        );
    }

    #[test]
    fn the_boost_rounds_and_clamps_at_one_hundred() {
        let dir = engine::ContentDir::at(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content"),
        );
        let content = Content::load(&dir).unwrap();
        let mut file = generate_league(1, 2, &content).remove(0);
        let first = file.players[0].attributes.keys().next().unwrap().clone();
        file.players[0].attributes.insert(first.clone(), 95);
        file.players[1].attributes.insert(first.clone(), 40);
        let strong = boosted(&file, 1.15);
        assert_eq!(strong.players[0].attributes[&first], 100);
        assert_eq!(strong.players[1].attributes[&first], 46);
        assert_eq!(strong.club, file.club);
    }

    #[test]
    fn ten_formations_give_fifty_five_pairings_each_once() {
        let list = pairings(10);
        assert_eq!(list.len(), 55);
        assert_eq!(list[0], [0, 0]);
        assert_eq!(list[1], [0, 1]);
        assert_eq!(list[54], [9, 9]);
        assert!(list.iter().all(|[a, b]| a <= b));
        let unique: std::collections::BTreeSet<_> = list.iter().collect();
        assert_eq!(unique.len(), 55);
    }

    #[test]
    fn a_pairing_plays_the_equal_clubs_and_alternates_its_home_formation() {
        let equal = fixtures(4);
        let list = formation_fixtures(4, 3);
        assert_eq!(list.len(), 12);
        for (i, f) in list.iter().enumerate() {
            assert_eq!(f.fixture.index, i as u32);
            assert_eq!(f.pairing, i / 4);
            assert_eq!(f.fixture.clubs, equal[i % 4].clubs);
            assert_eq!(f.fixture.league, equal[i % 4].league);
            assert_eq!(f.first_side, i % 2);
        }
    }

    #[test]
    fn a_selected_pairing_keeps_its_place_in_the_full_list() {
        let full = formation_fixtures(4, 55);
        let picked = formation_fixtures_for(4, 55, &[3, 40]);
        assert_eq!(picked.len(), 8);
        assert_eq!(picked[..4], full[12..16]);
        assert_eq!(picked[4..], full[160..164]);
    }

    #[test]
    fn the_red_card_suite_plays_every_arm_on_the_same_engine_seeds_in_both_orders() {
        let list = red_card_fixtures(1, 240);
        assert_eq!(list.len(), 960);
        for (i, f) in list.iter().enumerate() {
            assert_eq!(f.index, i as u32);
            assert_eq!(f.arm, i / 240);
            assert_eq!(f.engine_seed, 1 + (i % 240 / 2) as u64);
            assert_eq!(f.swapped, i % 2 == 1);
        }
        assert_eq!(list.last().unwrap().engine_seed, 120);
    }

    #[test]
    fn the_boosted_side_alternates() {
        let f = fixtures(4);
        let sides: Vec<usize> = f.iter().map(Fixture::boosted_side).collect();
        assert_eq!(sides, [0, 1, 0, 1]);
    }
}
