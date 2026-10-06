//! The fixture list of a calibration run: generated leagues of 20 clubs, each playing a
//! double round-robin, the stronger club of the strength suite, and the formation pairing
//! of the formations suite.
//!
//! Every fixture has a key: a hash of what the fixture is (its scenario, its world, the two
//! clubs, both formations, and a repeat number), never of its place in the run. So a run
//! with more matches keeps every earlier fixture's key and engine seed, and a new formation
//! leaves the other pairings' matches alone.

use std::fmt;

use engine::Content;
use engine::data::TeamFile;
use engine::data::generator::generate_league;
use engine::observe::identity::MatchId;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest, Sha256};

use crate::report::{RED_CARD_ARMS, Suite};

/// The scheme fixture keys are made by. Reports and run folders carry it, so a report
/// from the earlier scheme (seeds from the fixture's place in the run) is told apart.
pub const FIXTURE_SCHEME: &str = "fixture-key-1";

/// A fixture's key: the first 8 bytes of the SHA-256 of what the fixture is, shown as 16
/// hex characters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FixtureKey(u64);

impl FixtureKey {
    pub fn as_u64(self) -> u64 {
        self.0
    }

    /// The key a 16-hex-character text names.
    pub fn parse(text: &str) -> Option<Self> {
        (text.len() == 16)
            .then(|| u64::from_str_radix(text, 16).ok())
            .flatten()
            .map(Self)
    }
}

impl fmt::Display for FixtureKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:016x}", self.0)
    }
}

impl Serialize for FixtureKey {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for FixtureKey {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let text = String::deserialize(d)?;
        Self::parse(&text)
            .ok_or_else(|| serde::de::Error::custom(format!("{text} is not a fixture key")))
    }
}

/// The key of a fixture, and the engine seed the digest gives it: SHA-256 over the
/// length-prefixed scheme, scenario, world, two clubs (home first), two formations (home
/// first), and repeat number. The key is the digest's first 8 bytes, the seed the next 8.
pub fn key_of(
    scenario: &str,
    world: &str,
    clubs: [&str; 2],
    formations: [&str; 2],
    repeat: u64,
) -> (FixtureKey, u64) {
    let mut hasher = Sha256::new();
    let repeat = repeat.to_string();
    for part in [
        FIXTURE_SCHEME,
        scenario,
        world,
        clubs[0],
        clubs[1],
        formations[0],
        formations[1],
        &repeat,
    ] {
        hasher.update((part.len() as u64).to_le_bytes());
        hasher.update(part.as_bytes());
    }
    let digest = hasher.finalize();
    let word = |at: usize| {
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&digest[at..at + 8]);
        u64::from_be_bytes(bytes)
    };
    (FixtureKey(word(0)), word(8))
}

/// One match of a run with its key: what the parent plans and a worker plays.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Keyed {
    pub key: FixtureKey,
    /// The seed the engine plays the match with, stored beside the key.
    pub engine_seed: u64,
    /// The league and the clubs. The red-card suite plays the default clubs: league 0,
    /// clubs `[0, 1]`, or `[1, 0]` in the swapped order.
    pub fixture: Fixture,
    /// The strength suite: the side of the boosted club.
    pub boosted: Option<usize>,
    /// The formations suite: the pairing's number in [`pairings`], and the side of its
    /// first formation.
    pub pairing: Option<(usize, usize)>,
    /// The red-card suite: the arm's number in [`RED_CARD_ARMS`].
    pub arm: Option<usize>,
}

/// Every match `suite` plays with `matches` matches per suite, pairing, or arm, in fixture
/// order, each with its key and engine seed. `formations` names the formations of the
/// tactics file in order; `selected` holds the numbers of the pairings the formations suite
/// plays.
pub fn keyed(
    suite: Suite,
    seed: u64,
    matches: u32,
    formations: &[String],
    selected: &[usize],
) -> Vec<Keyed> {
    const DEFAULT: [&str; 2] = ["default", "default"];
    let league = |f: &Fixture| format!("league:{}", seed.wrapping_add(u64::from(f.league)));
    let clubs = |f: &Fixture| f.clubs.map(|c| c.to_string());
    match suite {
        Suite::Equal | Suite::Strength => fixtures(matches)
            .into_iter()
            .map(|f| {
                let boosted = (suite == Suite::Strength).then(|| f.boosted_side());
                let scenario = match boosted {
                    None => "equal",
                    Some(0) => "strength/boost-home",
                    Some(_) => "strength/boost-away",
                };
                let [home, away] = clubs(&f);
                let (key, engine_seed) = key_of(scenario, &league(&f), [&home, &away], DEFAULT, 0);
                Keyed {
                    key,
                    engine_seed,
                    fixture: f,
                    boosted,
                    pairing: None,
                    arm: None,
                }
            })
            .collect(),
        Suite::Formations => {
            let all = pairings(formations.len());
            formation_fixtures_for(matches, all.len(), selected)
                .into_iter()
                .map(|f| {
                    let [first, second] = all[f.pairing].map(|i| formations[i].as_str());
                    let sides = if f.first_side == 0 {
                        [first, second]
                    } else {
                        [second, first]
                    };
                    let [home, away] = clubs(&f.fixture);
                    let (key, engine_seed) =
                        key_of("formations", &league(&f.fixture), [&home, &away], sides, 0);
                    Keyed {
                        key,
                        engine_seed,
                        fixture: f.fixture,
                        boosted: None,
                        pairing: Some((f.pairing, f.first_side)),
                        arm: None,
                    }
                })
                .collect()
        }
        Suite::RedCard => {
            let world = format!("default-clubs:{seed}");
            red_card_fixtures(seed, matches)
                .into_iter()
                .map(|f| {
                    let k = f.index - f.arm as u32 * matches;
                    let clubs: [usize; 2] = if f.swapped { [1, 0] } else { [0, 1] };
                    let [home, away] = clubs.map(|c| c.to_string());
                    let scenario = format!("red-card/{}", RED_CARD_ARMS[f.arm].0);
                    let (key, _) =
                        key_of(&scenario, &world, [&home, &away], DEFAULT, u64::from(k / 2));
                    Keyed {
                        key,
                        // The slow test's experiment plays these seeds; they stay.
                        engine_seed: f.engine_seed,
                        fixture: Fixture {
                            index: f.index,
                            league: 0,
                            clubs,
                        },
                        boosted: None,
                        pairing: None,
                        arm: Some(f.arm),
                    }
                })
                .collect()
        }
    }
}

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

/// The match identifier of the fixture `key`; `millis` is the run's start. Red-card arms
/// that share an engine seed still get distinct identifiers.
pub fn match_id(key: FixtureKey, millis: u64) -> String {
    MatchId {
        seed: key.as_u64(),
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
        let names = formation_names(10);
        for suite in [
            Suite::Equal,
            Suite::Strength,
            Suite::Formations,
            Suite::RedCard,
        ] {
            assert_eq!(
                keyed(suite, 7, 50, &names, &all(10)),
                keyed(suite, 7, 50, &names, &all(10))
            );
        }
        assert_ne!(
            keyed(Suite::Equal, 7, 1, &names, &[])[0].key,
            keyed(Suite::Strength, 7, 1, &names, &[])[0].key
        );
        assert_ne!(
            keyed(Suite::Equal, 7, 1, &names, &[])[0].key,
            keyed(Suite::Equal, 8, 1, &names, &[])[0].key
        );
    }

    fn formation_names(n: usize) -> Vec<String> {
        (0..n).map(|i| format!("f{i}")).collect()
    }

    fn all(n: usize) -> Vec<usize> {
        (0..pairings(n).len()).collect()
    }

    #[test]
    fn a_bigger_run_keeps_every_earlier_key_and_seed() {
        let names = formation_names(10);
        for suite in [
            Suite::Equal,
            Suite::Strength,
            Suite::Formations,
            Suite::RedCard,
        ] {
            let small = keyed(suite, 3, 7, &names, &all(10));
            let big = keyed(suite, 3, 14, &names, &all(10));
            let seeds: std::collections::BTreeMap<_, _> =
                big.iter().map(|k| (k.key, k.engine_seed)).collect();
            for k in &small {
                assert_eq!(seeds.get(&k.key), Some(&k.engine_seed), "{suite:?} {k:?}");
            }
            assert_eq!(big.len(), small.len() * 2, "{suite:?}");
        }
        // In the equal and strength suites the bigger list starts with the smaller one.
        let small = keyed(Suite::Strength, 3, 7, &names, &[]);
        assert_eq!(small[..], keyed(Suite::Strength, 3, 14, &names, &[])[..7]);
    }

    #[test]
    fn a_new_formation_leaves_the_other_pairings_keys_alone() {
        let ten = formation_names(10);
        let mut eleven = ten.clone();
        eleven.push("new".into());
        let keys = |names: &[String], n: usize| -> std::collections::BTreeSet<(FixtureKey, u64)> {
            keyed(Suite::Formations, 3, 4, names, &all(n))
                .iter()
                .map(|k| (k.key, k.engine_seed))
                .collect()
        };
        let before = keys(&ten, 10);
        let after = keys(&eleven, 11);
        assert!(before.is_subset(&after));
        assert_eq!(after.len() - before.len(), 11 * 4);
    }

    #[test]
    fn keys_are_unique_over_every_suite_of_a_thousand_matches() {
        let names = formation_names(10);
        let mut seen = std::collections::BTreeSet::new();
        for suite in [
            Suite::Equal,
            Suite::Strength,
            Suite::Formations,
            Suite::RedCard,
        ] {
            for k in keyed(suite, 42, 1000, &names, &all(10)) {
                assert!(seen.insert(k.key), "{suite:?}: {} twice", k.key);
            }
        }
        // Equal and strength, 55 pairings, and four red-card arms.
        assert_eq!(seen.len(), 1000 * (2 + 55 + 4));
    }

    #[test]
    fn the_red_card_suite_keeps_its_engine_seeds_and_every_list_keeps_its_balance() {
        let names = formation_names(10);
        let red = keyed(Suite::RedCard, 1, 240, &names, &[]);
        let plain = red_card_fixtures(1, 240);
        for (k, f) in red.iter().zip(&plain) {
            assert_eq!(k.engine_seed, f.engine_seed);
            assert_eq!(k.arm, Some(f.arm));
            assert_eq!(k.fixture.clubs, if f.swapped { [1, 0] } else { [0, 1] });
        }
        let strength = keyed(Suite::Strength, 1, 4, &names, &[]);
        let sides: Vec<Option<usize>> = strength.iter().map(|k| k.boosted).collect();
        assert_eq!(sides, [Some(0), Some(1), Some(0), Some(1)]);
        let formations = keyed(Suite::Formations, 1, 4, &names, &[2]);
        let firsts: Vec<Option<(usize, usize)>> = formations.iter().map(|k| k.pairing).collect();
        assert_eq!(
            firsts,
            [Some((2, 0)), Some((2, 1)), Some((2, 0)), Some((2, 1))]
        );
    }

    #[test]
    fn a_key_reads_back_from_its_text() {
        let (key, _) = key_of("equal", "league:1", ["0", "1"], ["a", "b"], 0);
        let text = key.to_string();
        assert_eq!(text.len(), 16);
        assert_eq!(FixtureKey::parse(&text), Some(key));
        assert_eq!(FixtureKey::parse("xyz"), None);
        let json = serde_json::to_string(&key).unwrap();
        assert_eq!(serde_json::from_str::<FixtureKey>(&json).unwrap(), key);
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
