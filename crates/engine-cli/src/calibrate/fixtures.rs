//! The fixture list of a calibration run: generated leagues of 20 clubs, each playing a
//! double round-robin, with a seed per match and the stronger club of the strength suite.

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
    fn the_boosted_side_alternates() {
        let f = fixtures(4);
        let sides: Vec<usize> = f.iter().map(Fixture::boosted_side).collect();
        assert_eq!(sides, [0, 1, 0, 1]);
    }
}
