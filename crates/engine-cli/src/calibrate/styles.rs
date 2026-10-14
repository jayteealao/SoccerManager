//! The styles suite: every ordered pairing of two distinct named styles of `styles.json`,
//! each match with identical squads. A match takes the home club of the equal suite's
//! fixture at the same place and gives a copy of it to the away side, so both sides field
//! the same players while the clubs rotate across matches. Each side starts in its style's
//! tactics, in a formation drawn from the style's list by a draw seeded from the fixture
//! key, with the lineup its AI picks for that formation.
//!
//! The suite is report only: it feeds no realism band and no verdict. `--suite styles` plays
//! it alone; a change run plays one match per ordered pairing on the changed engine only and
//! adds the report beside its verdict. The old engine never plays it.
//!
//! `--style-pairing` plays only the named ordered pairings, the same matches as in a full
//! run; there a style may meet itself. A measures build adds each pairing's probe.

use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use engine::data::TeamFile;
use engine::data::styles::{IDENTITY_STATS, Styles};
use engine::observe::MatchStats;
use engine::{Commentary, Content, EngineError, Tactics};

use super::fixtures::{self, Fixture, FixtureKey, Leagues};
use super::worker::{MatchInput, play_match};
use crate::report::styles::{self as report, StyleMatch, StylesReport};

/// One match of the styles suite.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StyleFixture {
    pub key: FixtureKey,
    pub engine_seed: u64,
    /// The equal suite's fixture at the same place; its home club plays on both sides.
    pub fixture: Fixture,
    /// Each side's style, home first.
    pub styles: [usize; 2],
    /// Each side's formation, as an index into the tactics file, home first.
    pub formations: [u8; 2],
}

/// Every match of the suite: `matches` matches for each ordered pairing of distinct styles,
/// pairing by pairing (home style, then away style, in file order), or for each pairing of
/// `only` in its order. Match `j` of a pairing takes the home club of the equal suite's
/// fixture `j`; each side's formation is drawn from its style's list by a draw seeded from
/// the fixture's key and the side.
pub fn style_fixtures(
    seed: u64,
    matches: u32,
    styles: &Styles,
    formation_names: &[String],
    only: Option<&[[usize; 2]]>,
) -> Vec<StyleFixture> {
    let equal = fixtures::fixtures(matches);
    let n = styles.styles.len();
    let all: Vec<[usize; 2]> = (0..n)
        .flat_map(|h| (0..n).filter(move |&w| w != h).map(move |w| [h, w]))
        .collect();
    let mut out = Vec::new();
    for &[h, w] in only.unwrap_or(&all) {
        let names = [&styles.styles[h].name, &styles.styles[w].name];
        for (j, f) in equal.iter().enumerate() {
            let league = format!("league:{}", seed.wrapping_add(u64::from(f.league)));
            let club = f.clubs[0].to_string();
            let (key, engine_seed) = fixtures::key_of(
                "styles",
                &league,
                [&club, &club],
                [names[0], names[1]],
                j as u64,
            );
            // Each side draws its own formation, the away side with the second draw.
            let formations = [(0u64, h), (1u64, w)].map(|(side, s)| {
                let list = &styles.styles[s].formations;
                let draw = fixtures::reseeded(key.as_u64(), side + 1);
                list[(draw % list.len() as u64) as usize]
            });
            debug_assert!(
                formations
                    .iter()
                    .all(|&f| usize::from(f) < formation_names.len())
            );
            out.push(StyleFixture {
                key,
                engine_seed,
                fixture: Fixture {
                    clubs: [f.clubs[0], f.clubs[0]],
                    ..*f
                },
                styles: [h, w],
                formations,
            });
        }
    }
    out
}

/// The two sides of `f`: the home club, and a copy of it with its id marked as the away side.
fn sides(f: &StyleFixture, leagues: &mut Leagues<'_>) -> [TeamFile; 2] {
    let [home, _] = leagues.teams(&f.fixture);
    let mut away = home.clone();
    away.club.id = format!("{}-b", home.club.id);
    [home, away]
}

/// Each side's value of every identity statistic of a played match.
fn identity_stats(s: &MatchStats) -> [[f64; IDENTITY_STATS.len()]; 2] {
    let mut out = [[0.0; IDENTITY_STATS.len()]; 2];
    for (side, row) in out.iter_mut().enumerate() {
        let shots = f64::from(s.tactics.shots[side]);
        for (k, name) in IDENTITY_STATS.iter().enumerate() {
            row[k] = match *name {
                "possession_pct" => s.figures.possession_pct[side],
                "passes" => f64::from(s.figures.passes[side]),
                "pass_accuracy_pct" => s.figures.pass_accuracy_pct[side],
                "shots" => shots,
                "shots_on_target_share" if shots > 0.0 => {
                    f64::from(s.figures.shots_on_target[side]) / shots
                }
                "shots_on_target_share" => f64::NAN,
                "fouls" => f64::from(s.laws.fouls[side]),
                "corners" => f64::from(s.laws.corners[side]),
                "goals" => f64::from(s.goals[side]),
                "xg" => s.figures.xg[side],
                _ => f64::NAN,
            };
        }
    }
    out
}

/// Each side's probe of a played match from its statistics and its realism measures.
#[cfg(feature = "measures")]
fn probe(s: &MatchStats, m: &engine::measures::MatchMeasures) -> [report::ProbeSide; 2] {
    use crate::report::styles::PROBE;
    [0, 1].map(|side| {
        let f = f64::from;
        let shots = f(m.shot_bins[side].iter().sum::<u32>());
        let x = &m.transitions[side];
        let losses = f(x.losses);
        let ticks = m.third_ticks[side];
        let live = f(ticks.iter().sum::<u32>());
        let values: [(f64, f64); PROBE.len()] = [
            (f(s.tactics.shots[side]), 1.0),
            (f(s.goals[side]), 1.0),
            (s.figures.xg[side], 1.0),
            (s.figures.xg[side], shots),
            (m.shot_distance_m[side], shots),
            (f(m.possessions[side]), 1.0),
            (f(m.turnovers[side]), f(m.possessions[side])),
            (losses, 1.0),
            (f(x.losses_by_third[0]), losses),
            (f(x.losses_by_third[1]), losses),
            (f(x.losses_by_third[2]), losses),
            (f(x.regains), 1.0),
            (f(x.counter_attacks), 1.0),
            (x.regain_shot_s, f(x.regain_shots)),
            (f(x.pressures), 1.0),
            (f(x.counterpress_pressures), losses),
            (f(x.counterpress_regains), losses),
            (x.last_counterpress_s, f(x.last_counterpress)),
            (f(s.laws.offsides[side]), 1.0),
            (f(ticks[0]), live),
            (f(ticks[1]), live),
            (f(ticks[2]), live),
        ];
        values
    })
}

/// What one run of the suite needs.
pub struct SuiteInput<'a> {
    pub content: &'a Content,
    pub commentary: &'a Commentary,
    pub seed: u64,
    /// Matches per ordered pairing.
    pub matches: u32,
    pub minutes: u32,
    pub jobs: u32,
    pub owner_id: &'a str,
    /// Only these ordered pairings (home style, away style), when given.
    pub pairings: Option<&'a [[usize; 2]]>,
}

/// Plays the suite on `jobs` threads and builds its report; `None` when the content has no
/// styles file.
pub fn play(input: &SuiteInput<'_>) -> anyhow::Result<Option<StylesReport>> {
    let Some(styles) = input.content.styles.as_ref() else {
        return Ok(None);
    };
    let names: Vec<String> = input
        .content
        .tactics
        .formations
        .iter()
        .map(|f| f.name.clone())
        .collect();
    let list = style_fixtures(input.seed, input.matches, styles, &names, input.pairings);
    let started = Instant::now();
    let next = AtomicUsize::new(0);
    let results: Mutex<Vec<(usize, StyleMatch, usize)>> = Mutex::new(Vec::new());
    let first_error: Mutex<Option<EngineError>> = Mutex::new(None);
    let threads = usize::try_from(input.jobs.max(1))
        .unwrap_or(1)
        .min(list.len().max(1));
    std::thread::scope(|scope| {
        for _ in 0..threads {
            let (list, next, results, first_error, names) =
                (&list, &next, &results, &first_error, &names);
            scope.spawn(move || {
                let mut leagues = Leagues::new(input.seed, input.content);
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some(f) = list.get(i) else { break };
                    let teams = sides(f, &mut leagues);
                    let tactics: [Tactics; 2] = [0, 1].map(|side| {
                        styles.styles[f.styles[side]]
                            .tactics(f.formations[side], &input.content.tactics)
                    });
                    let match_id = format!("styles-{}", f.key);
                    let played = play_match(
                        &MatchInput {
                            content: input.content,
                            commentary: input.commentary,
                            teams: &teams,
                            formations: None,
                            tactics: Some(tactics),
                            red_card: None,
                            key: f.key,
                            seed: f.engine_seed,
                            minutes: input.minutes,
                            owner_id: input.owner_id,
                            match_id: &match_id,
                        },
                        &mut |_| None,
                    );
                    match played {
                        Ok(p) => {
                            let m = StyleMatch {
                                styles: f.styles,
                                formations: f.formations.map(|x| names[usize::from(x)].clone()),
                                goals: p.stats.goals,
                                stats: identity_stats(&p.stats),
                                #[cfg(feature = "measures")]
                                probe: p.measures.as_deref().map(|m| probe(&p.stats, m)),
                                #[cfg(not(feature = "measures"))]
                                probe: None,
                            };
                            results.lock().unwrap_or_else(|e| e.into_inner()).push((
                                i,
                                m,
                                p.stats.validate_violations,
                            ));
                        }
                        Err(e) => {
                            first_error
                                .lock()
                                .unwrap_or_else(|e| e.into_inner())
                                .get_or_insert(e);
                        }
                    }
                }
            });
        }
    });
    if let Some(e) = first_error.into_inner().unwrap_or_else(|e| e.into_inner()) {
        return Err(e.into());
    }
    let mut done = results.into_inner().unwrap_or_else(|e| e.into_inner());
    done.sort_by_key(|(i, _, _)| *i);
    let violations = done.iter().map(|(_, _, v)| v).sum();
    let matches: Vec<StyleMatch> = done.into_iter().map(|(_, m, _)| m).collect();
    let style_names: Vec<String> = styles.styles.iter().map(|s| s.name.clone()).collect();
    let identities: Vec<_> = styles.styles.iter().map(|s| s.identity.clone()).collect();
    Ok(Some(report::build(
        &style_names,
        &identities,
        input.matches,
        u32::try_from(list.len()).unwrap_or(u32::MAX),
        &matches,
        violations,
        u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn content() -> Content {
        crate::content::load(
            Some(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")),
            None,
            None,
            None,
        )
        .unwrap()
        .content
    }

    /// Every ordered pairing of distinct styles, each with `matches` matches; identical
    /// clubs on both sides; each formation from its style's list; keys unique and stable.
    #[test]
    fn the_fixtures_cover_every_ordered_pairing_with_one_club() {
        let content = content();
        let styles = content.styles.as_ref().unwrap();
        let names: Vec<String> = content
            .tactics
            .formations
            .iter()
            .map(|f| f.name.clone())
            .collect();
        let n = styles.styles.len();
        let list = style_fixtures(42, 3, styles, &names, None);
        assert_eq!(list.len(), n * (n - 1) * 3);
        let mut pairs = std::collections::BTreeMap::new();
        for f in &list {
            assert_ne!(f.styles[0], f.styles[1]);
            assert_eq!(f.fixture.clubs[0], f.fixture.clubs[1]);
            for side in 0..2 {
                assert!(
                    styles.styles[f.styles[side]]
                        .formations
                        .contains(&f.formations[side])
                );
            }
            *pairs.entry(f.styles).or_insert(0) += 1;
        }
        assert_eq!(pairs.len(), n * (n - 1));
        assert!(pairs.values().all(|&c| c == 3));
        let keys: std::collections::BTreeSet<_> = list.iter().map(|f| f.key).collect();
        assert_eq!(keys.len(), list.len());
        assert_eq!(list, style_fixtures(42, 3, styles, &names, None));
        // More matches keep the earlier ones.
        let more = style_fixtures(42, 4, styles, &names, None);
        for f in &list {
            assert!(more.contains(f));
        }
    }

    /// `--style-pairing`: only the named ordered pairings, the same matches as in a full run,
    /// and a style may meet itself, each side drawing its own formation from its list.
    #[test]
    fn a_pairing_filter_plays_only_its_pairings_and_a_style_may_meet_itself() {
        let content = content();
        let styles = content.styles.as_ref().unwrap();
        let names: Vec<String> = content
            .tactics
            .formations
            .iter()
            .map(|f| f.name.clone())
            .collect();
        let only = [[0, 1], [2, 2]];
        let list = style_fixtures(42, 3, styles, &names, Some(&only));
        assert_eq!(list.len(), 6);
        let full = style_fixtures(42, 3, styles, &names, None);
        for f in list.iter().filter(|f| f.styles == [0, 1]) {
            assert!(full.contains(f));
        }
        let mirror: Vec<_> = list.iter().filter(|f| f.styles == [2, 2]).collect();
        assert_eq!(mirror.len(), 3);
        for f in mirror {
            assert!(
                f.formations
                    .iter()
                    .all(|x| styles.styles[2].formations.contains(x))
            );
        }
    }
}
