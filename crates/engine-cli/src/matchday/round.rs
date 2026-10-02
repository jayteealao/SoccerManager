//! The round of the matchday: every club in the content folder's `teams/` except the two the
//! player's match plays, paired into fixtures from a seed derived from the player's seed. The
//! same served match always meets the same round, so a reconnect, a resume and a test all see
//! one matchday.

use std::collections::BTreeMap;
use std::path::PathBuf;

use engine::data::TeamFile;
use engine::snapshot::{MarkFixture, MatchdayMark};
use engine::{Content, ContentDir};

use crate::calibrate::fixtures::splitmix64;

/// The folder inside the content folder that holds the club files.
const TEAMS_DIR: &str = "teams";

/// Mixed into the player's seed for the round seed ("matchday" in ASCII), so the round never
/// shares a seed with the player's match.
const ROUND_SALT: u64 = 0x6d61_7463_6864_6179;

/// One club of the round, as its file loaded.
#[derive(Debug, Clone)]
pub struct Club {
    pub file: TeamFile,
    /// The SHA-256 of the file's bytes, saved in a snapshot's matchday mark.
    pub digest: [u8; 32],
}

/// One background match.
#[derive(Debug, Clone)]
pub struct Fixture {
    /// The fixture's place in the round, from 0.
    pub index: u32,
    pub seed: u64,
    /// Home first. `None` when the club's file is missing or changed since a saved match
    /// recorded it: the fixture is unavailable from kick-off.
    pub clubs: [Option<Club>; 2],
    /// The two club identifiers, home first, as the round recorded them.
    pub club_ids: [String; 2],
    /// Why the fixture cannot play, when a club file is missing or changed.
    pub refused: Option<String>,
}

/// The fixtures of one matchday, and how every one of them is played.
#[derive(Debug, Clone)]
pub struct Round {
    pub seed: u64,
    pub fixtures: Vec<Fixture>,
}

impl Round {
    /// The round of a served match: the clubs in `teams/` other than `player_clubs`, ordered by
    /// club id, shuffled with the round seed and paired neighbour with neighbour, home first.
    /// An odd club out has no fixture. A file that does not load is left out of the round.
    pub fn for_match(
        dir: &ContentDir,
        content: &Content,
        player_clubs: [&str; 2],
        player_seed: u64,
    ) -> Round {
        let seed = splitmix64(player_seed ^ ROUND_SALT);
        let mut clubs: Vec<Club> = load_clubs(dir, content)
            .into_values()
            .filter(|c| !player_clubs.contains(&c.file.club.id.as_str()))
            .collect();
        // `load_clubs` keys by club id, so the clubs are already in club-id order.
        shuffle(&mut clubs, seed);
        let fixtures = clubs
            .chunks_exact(2)
            .enumerate()
            .map(|(i, pair)| {
                let index = u32::try_from(i).expect("a round holds few fixtures");
                Fixture {
                    index,
                    seed: fixture_seed(seed, index),
                    club_ids: [pair[0].file.club.id.clone(), pair[1].file.club.id.clone()],
                    clubs: [Some(pair[0].clone()), Some(pair[1].clone())],
                    refused: None,
                }
            })
            .collect();
        Round { seed, fixtures }
    }

    /// The round a snapshot's mark recorded. A club whose file is missing, or whose bytes
    /// differ from the ones the mark recorded, makes its fixture unavailable from kick-off.
    pub fn from_mark(mark: &MatchdayMark, dir: &ContentDir, content: &Content) -> Round {
        let mut clubs = load_clubs(dir, content);
        let fixtures = mark
            .fixtures
            .iter()
            .enumerate()
            .map(|(i, recorded)| {
                let index = u32::try_from(i).expect("a round holds few fixtures");
                let mut refused = None;
                let mut found = [None, None];
                for (side, id) in recorded.clubs.iter().enumerate() {
                    match clubs.get(id) {
                        Some(club) if club.digest == recorded.digests[side] => {
                            found[side] = Some(club.clone());
                        }
                        Some(_) => {
                            refused.get_or_insert(format!(
                                "the club file of {id} changed since the match was saved"
                            ));
                        }
                        None => {
                            refused.get_or_insert(format!("no club file names {id}"));
                        }
                    }
                }
                Fixture {
                    index,
                    seed: fixture_seed(mark.round_seed, index),
                    clubs: found,
                    club_ids: recorded.clubs.clone(),
                    refused,
                }
            })
            .collect();
        clubs.clear();
        Round {
            seed: mark.round_seed,
            fixtures,
        }
    }

    /// The mark a snapshot taken at `reveal_tick` saves for this round.
    pub fn mark(&self, reveal_tick: u32) -> MatchdayMark {
        MatchdayMark {
            round_seed: self.seed,
            reveal_tick,
            fixtures: self
                .fixtures
                .iter()
                .map(|f| MarkFixture {
                    clubs: f.club_ids.clone(),
                    digests: [0, 1]
                        .map(|side| f.clubs[side].as_ref().map_or([0; 32], |c| c.digest)),
                })
                .collect(),
        }
    }
}

/// The seed of fixture `index` of a round.
fn fixture_seed(round_seed: u64, index: u32) -> u64 {
    splitmix64(round_seed.wrapping_add(u64::from(index)))
}

/// Every club file in `teams/` that loads, keyed by club id. A file that does not load is
/// left out with a warning naming it by its path inside the content folder.
fn load_clubs(dir: &ContentDir, content: &Content) -> BTreeMap<String, Club> {
    let mut paths: Vec<PathBuf> = match std::fs::read_dir(dir.path(TEAMS_DIR)) {
        Ok(entries) => entries
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.is_file() && p.extension().is_some_and(|e| e == "json"))
            .collect(),
        Err(_) => Vec::new(),
    };
    paths.sort();
    let mut clubs = BTreeMap::new();
    for path in paths {
        match content.load_team(dir, &path) {
            Ok(loaded) => {
                clubs.entry(loaded.value.club.id.clone()).or_insert(Club {
                    file: loaded.value,
                    digest: loaded.digest,
                });
            }
            Err(err) => {
                tracing::warn!(
                    signal = "matchday.team_refused",
                    path = %dir.relative(&path),
                    reason = %err
                );
            }
        }
    }
    clubs
}

/// A Fisher-Yates shuffle driven by SplitMix64 draws from `seed`.
fn shuffle<T>(items: &mut [T], seed: u64) {
    let mut state = seed;
    for i in (1..items.len()).rev() {
        state = splitmix64(state);
        let j = usize::try_from(state % (i as u64 + 1)).expect("an index fits usize");
        items.swap(i, j);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine::data::{TEAM_A_FILE, TEAM_B_FILE};
    use std::path::Path;

    fn shipped() -> ContentDir {
        ContentDir::at(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content"))
    }

    fn player_clubs(dir: &ContentDir, content: &Content) -> [String; 2] {
        [TEAM_A_FILE, TEAM_B_FILE]
            .map(|f| content.load_team(dir, &dir.path(f)).unwrap().value.club.id)
    }

    /// A copy of the content folder whose `teams/` holds only the files `keep` names.
    fn copy_with_teams(name: &str, keep: &[&str]) -> ContentDir {
        let to =
            std::env::temp_dir().join(format!("engine-cli-round-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&to);
        copy_dir(shipped().root(), &to, keep);
        ContentDir::at(to)
    }

    fn copy_dir(from: &Path, to: &Path, keep: &[&str]) {
        std::fs::create_dir_all(to).unwrap();
        for entry in std::fs::read_dir(from).unwrap() {
            let path = entry.unwrap().path();
            let target = to.join(path.file_name().unwrap());
            if path.is_dir() {
                copy_dir(&path, &target, keep);
            } else {
                let in_teams = path.parent().is_some_and(|p| p.ends_with(TEAMS_DIR));
                let name = path.file_name().unwrap().to_string_lossy().into_owned();
                if !in_teams || keep.contains(&name.as_str()) {
                    std::fs::copy(&path, &target).unwrap();
                }
            }
        }
    }

    #[test]
    fn the_same_match_meets_the_same_round_and_the_player_clubs_never_play_in_it() {
        let dir = shipped();
        let content = Content::load(&dir).unwrap();
        let ids = player_clubs(&dir, &content);
        let players = [ids[0].as_str(), ids[1].as_str()];
        let a = Round::for_match(&dir, &content, players, 42);
        let b = Round::for_match(&dir, &content, players, 42);
        assert_eq!(a.seed, b.seed);
        assert_eq!(a.fixtures.len(), 4, "ten clubs give four fixtures");
        for (x, y) in a.fixtures.iter().zip(&b.fixtures) {
            assert_eq!((x.seed, &x.club_ids), (y.seed, &y.club_ids));
        }
        let mut seen: Vec<&String> = a.fixtures.iter().flat_map(|f| &f.club_ids).collect();
        assert!(seen.iter().all(|id| !ids.contains(id)), "{seen:?}");
        seen.sort();
        seen.dedup();
        assert_eq!(seen.len(), 8, "every other club plays once");
        let seeds: std::collections::BTreeSet<u64> = a.fixtures.iter().map(|f| f.seed).collect();
        assert_eq!(seeds.len(), 4, "each fixture has its own seed");
        let other = Round::for_match(&dir, &content, players, 43);
        assert_ne!(
            a.fixtures.iter().map(|f| f.seed).collect::<Vec<_>>(),
            other.fixtures.iter().map(|f| f.seed).collect::<Vec<_>>()
        );
    }

    #[test]
    fn two_clubs_give_an_empty_round() {
        let dir = copy_with_teams("two", &[TEAM_A_FILE_NAME, TEAM_B_FILE_NAME]);
        let content = Content::load(&dir).unwrap();
        let ids = player_clubs(&dir, &content);
        let round = Round::for_match(&dir, &content, [&ids[0], &ids[1]], 42);
        assert!(round.fixtures.is_empty());
        let _ = std::fs::remove_dir_all(dir.root());
    }

    #[test]
    fn an_unreadable_club_file_is_left_out_of_the_round() {
        let dir = copy_with_teams("broken", &[]);
        let shipped_dir = shipped();
        let mut names: Vec<String> = std::fs::read_dir(shipped_dir.path(TEAMS_DIR))
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        for name in &names {
            std::fs::copy(
                shipped_dir.path(TEAMS_DIR).join(name),
                dir.path(TEAMS_DIR).join(name),
            )
            .unwrap();
        }
        std::fs::write(dir.path(TEAMS_DIR).join("zz-broken.json"), "{ not json").unwrap();
        let content = Content::load(&dir).unwrap();
        let ids = player_clubs(&dir, &content);
        let round = Round::for_match(&dir, &content, [&ids[0], &ids[1]], 42);
        assert_eq!(round.fixtures.len(), 4);
        let _ = std::fs::remove_dir_all(dir.root());
    }

    #[test]
    fn a_round_rebuilt_from_its_mark_is_the_same_round() {
        let dir = shipped();
        let content = Content::load(&dir).unwrap();
        let ids = player_clubs(&dir, &content);
        let round = Round::for_match(&dir, &content, [&ids[0], &ids[1]], 7);
        let mark = round.mark(12_000);
        assert_eq!(mark.reveal_tick, 12_000);
        let back = Round::from_mark(&mark, &dir, &content);
        assert_eq!(back.seed, round.seed);
        for (x, y) in round.fixtures.iter().zip(&back.fixtures) {
            assert_eq!((x.seed, &x.club_ids), (y.seed, &y.club_ids));
            assert!(y.refused.is_none());
        }
    }

    #[test]
    fn a_changed_club_file_makes_its_fixture_unavailable_after_a_resume() {
        let dir = shipped();
        let content = Content::load(&dir).unwrap();
        let ids = player_clubs(&dir, &content);
        let round = Round::for_match(&dir, &content, [&ids[0], &ids[1]], 7);
        let mut mark = round.mark(0);
        mark.fixtures[1].digests[0][0] ^= 1;
        mark.fixtures[2].clubs[1] = "club-nowhere".into();
        let back = Round::from_mark(&mark, &dir, &content);
        assert!(back.fixtures[0].refused.is_none());
        assert!(
            back.fixtures[1]
                .refused
                .as_deref()
                .unwrap()
                .contains("changed"),
            "{:?}",
            back.fixtures[1].refused
        );
        assert!(
            back.fixtures[2]
                .refused
                .as_deref()
                .unwrap()
                .contains("club-nowhere")
        );
    }

    const TEAM_A_FILE_NAME: &str = "default-a.json";
    const TEAM_B_FILE_NAME: &str = "default-b.json";
}
