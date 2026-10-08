//! The keyed stream registry. Extra draws for one key leave every other key's sequence
//! unchanged, and a substitute draws on his own key. A key outside the table fails and names
//! itself. The scheme's table, key derivation, and draw conversion match a committed digest,
//! and the golden file records the scheme this build plays. The draw counter equals the draws
//! taken, and no engine source draws around the registry.

mod common;

use std::path::Path;
use std::sync::OnceLock;

use engine::data::TeamFile;
use engine::gate::golden;
use engine::gate::{self, Fixture, Inputs};
use engine::rng::EngineRng;
use engine::streams::table::{self, PlayerKind, SCHEME_DIGESTS, TABLE};
use engine::streams::{
    Action, KEYED_SCHEME, Key, PlayerKey, STREAM_SCHEME, Scheme, StreamState, Streams,
};
use engine::{Change, Content, EngineEventKind, Manager, MatchConfig, Simulation};

fn loaded() -> &'static (Content, [TeamFile; 2]) {
    static LOADED: OnceLock<(Content, [TeamFile; 2])> = OnceLock::new();
    LOADED.get_or_init(|| {
        let content = common::content();
        let teams = common::default_teams(&content);
        (content, teams)
    })
}

fn inputs() -> Inputs<'static> {
    let (content, [a, b]) = loaded();
    Inputs {
        content,
        teams: [a, b],
        pack: None,
    }
}

/// Two player keys (team 0 squad 3, team 1 squad 7) for each player row, and the match key
/// for each match row, in table order.
fn sample_keys() -> Vec<Key> {
    let mut keys = Vec::new();
    for row in &TABLE {
        match row.kind {
            PlayerKind::Match => keys.push(Key::of_match(row.action)),
            PlayerKind::Actor => {
                for (team, squad) in [(0, 3), (1, 7)] {
                    keys.push(Key {
                        action: row.action,
                        player: PlayerKey::of_squad(team, squad),
                    });
                }
            }
        }
    }
    keys
}

const RUN: usize = 16;

/// `RUN` draws from every key of `keys` except `skip`, in order, after `extra` draws from
/// `skip`, with `draw` reading the key's value.
fn sequences_with(
    mut draw: impl FnMut(Key) -> f64,
    keys: &[Key],
    skip: Option<Key>,
    extra: usize,
) -> Vec<Vec<u64>> {
    if let Some(x) = skip {
        for _ in 0..extra {
            draw(x);
        }
    }
    keys.iter()
        .map(|&k| {
            if Some(k) == skip {
                Vec::new()
            } else {
                (0..RUN).map(|_| draw(k).to_bits()).collect()
            }
        })
        .collect()
}

/// [`sequences_with`] on a seed-42 registry.
fn sequences(keys: &[Key], skip: Option<Key>, extra: usize) -> Vec<Vec<u64>> {
    let mut s = Streams::keyed(42);
    sequences_with(|k| s.draw(k), keys, skip, extra)
}

#[test]
fn extra_draws_on_one_key_leave_every_other_key_unchanged() {
    let keys = sample_keys();
    // 5 match rows and 35 player rows.
    assert_eq!(keys.len(), 5 + 2 * 35);
    let control = sequences(&keys, None, 0);
    for (x, &key) in keys.iter().enumerate() {
        let disturbed = sequences(&keys, Some(key), 100);
        for (k, other) in keys.iter().enumerate().filter(|&(k, _)| k != x) {
            assert_eq!(
                disturbed[k],
                control[k],
                "100 extra draws on {} moved {}",
                key.name(),
                other.name()
            );
        }
    }
    // The same player in another subsystem, another player in the same subsystem, and the
    // match key are all among the keys compared above.
    let (shot, tackle_other, card) = (
        Key {
            action: Action::ShotScore,
            player: PlayerKey::of_squad(0, 3),
        },
        Key {
            action: Action::Tackle,
            player: PlayerKey::of_squad(1, 7),
        },
        Key::of_match(Action::AddedTime),
    );
    assert!(keys.contains(&shot) && keys.contains(&tackle_other) && keys.contains(&card));

    // Negative control: with one shared stream for every key, the same disturbance moves the
    // next key, so the comparison above can fail.
    let shared = |skip, extra| {
        let mut rng = EngineRng::from_seed(42);
        sequences_with(|_| rng.next_f64(), &keys, skip, extra)
    };
    assert_ne!(
        shared(Some(keys[0]), 100)[1],
        shared(None, 0)[1],
        "one shared stream moves every key"
    );
}

#[test]
fn a_substitute_draws_on_his_own_key() {
    let (content, [a, b]) = loaded();
    let config = MatchConfig::new(42, 90, content, [a, b])
        .unwrap()
        .with_manager(0, Manager::Human)
        .with_manager(1, Manager::Human);
    let mut sim = Simulation::new(config).unwrap();
    let team = &sim.teams()[0];
    let (off, on) = (team.lineup[9], team.bench[0]);
    let mut queued = false;
    let mut at_sub: Option<(StreamState, Vec<PlayerKey>)> = None;
    while !sim.is_over() {
        if !queued && sim.tick() >= 60_000 {
            sim.queue_change(0, Change::Substitution { off, on });
            queued = true;
        }
        let before: Vec<PlayerKey> = sim.players().iter().map(PlayerKey::of).collect();
        sim.step();
        let events = sim.take_events();
        if at_sub.is_none()
            && events
                .iter()
                .any(|e| e.kind == EngineEventKind::Substitution)
        {
            at_sub = Some((sim.stream_state(), before));
            let after: Vec<PlayerKey> = sim.players().iter().map(PlayerKey::of).collect();
            let at = at_sub.as_ref().unwrap();
            let changed: Vec<usize> = (0..after.len()).filter(|&i| after[i] != at.1[i]).collect();
            assert_eq!(
                changed.len(),
                1,
                "only the substituted place changes its key"
            );
            let i = changed[0];
            assert_eq!(at.1[i], PlayerKey::of_squad(0, off));
            assert_eq!(
                after[i],
                PlayerKey::of_squad(0, on),
                "the substitute takes the roster place but brings his own key"
            );
            assert!(
                after.iter().filter(|&&k| k == after[i]).count() == 1,
                "no other player shares the substitute's key"
            );
        }
    }
    sim.finish();
    let (at_sub, _) = at_sub.expect("the substitution was applied");
    let end = sim.stream_state();
    let of = |state: &StreamState, squad: usize| -> Vec<(u64, u128)> {
        state
            .entries
            .iter()
            .copied()
            .filter(|&(id, _)| {
                Key::from_stream_id(id).unwrap().player == PlayerKey::of_squad(0, squad)
            })
            .collect()
    };
    // Coming on, he drew only the match part of his form: two draws on its key.
    let form = Key::player(Action::FormMatch, PlayerKey::of_squad(0, on)).stream_id();
    assert_eq!(
        of(&at_sub, on),
        vec![(form, 4)],
        "the substitute had drawn only his form on entry"
    );
    assert!(
        !of(&end, on).is_empty(),
        "the substitute drew on his own keys"
    );
    assert!(!of(&at_sub, off).is_empty());
    assert_eq!(
        of(&end, off),
        of(&at_sub, off),
        "the replaced player's keys stop where he left"
    );
}

#[test]
#[should_panic(expected = "unregistered stream key laws.added_time team 0 squad 3")]
fn a_key_not_in_the_table_fails_and_names_it() {
    Streams::keyed(42).draw(Key {
        action: Action::AddedTime,
        player: PlayerKey::of_squad(0, 3),
    });
}

#[test]
#[should_panic(expected = "unregistered stream key decision.shot_score team 1 squad 40")]
fn a_squad_index_past_the_table_fails_and_names_it() {
    Streams::keyed(42).draw(Key {
        action: Action::ShotScore,
        player: PlayerKey::of_squad(1, 40),
    });
}

#[test]
fn the_scheme_matches_its_committed_digest() {
    for (scheme, committed) in SCHEME_DIGESTS {
        let scheme = Scheme::from_id(scheme).expect("a committed scheme exists");
        let now = table::digest(scheme);
        assert_eq!(
            now,
            committed,
            "scheme {}: the table, the key derivation, or the draw conversion changed: bump \
             the scheme and add its digest",
            scheme.id()
        );
    }
    assert_eq!(SCHEME_DIGESTS.map(|(id, _)| id), [KEYED_SCHEME]);
    assert_eq!(STREAM_SCHEME, KEYED_SCHEME);
}

#[test]
fn the_golden_file_records_the_scheme_this_build_plays() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../gate/golden.json");
    let file = golden::load_lenient(&path).unwrap();
    // The hashes are those of the last bootstrap or regenerate entry; the add-machine-set
    // entries after it record the same scheme.
    let base = file
        .ledger
        .iter()
        .rposition(|e| e.kind != golden::EntryKind::AddMachineSet)
        .expect("the ledger starts with a bootstrap");
    for entry in &file.ledger[base..] {
        assert_eq!(entry.scheme, STREAM_SCHEME);
    }
}

/// The fixtures that reach every subsystem: seed 42, and a knockout match without a script
/// pack that goes to extra time and a shoot-out (seed 11).
fn counter_fixtures() -> [Fixture; 2] {
    [
        Fixture::seed(42),
        Fixture {
            pack: None,
            ..Fixture::knockout(11)
        },
    ]
}

#[test]
fn the_draw_counter_equals_the_draws_taken() {
    for fixture in counter_fixtures() {
        let (played, draws, state) = gate::play_streams(&fixture, &inputs()).unwrap();
        let words: u128 = state.entries.iter().map(|&(_, w)| w).sum();
        assert_eq!(u128::from(draws) * 2, words, "{}", fixture.id);
        assert!(draws > 0);
        assert_eq!(state.scheme, KEYED_SCHEME);
        assert!(state.entries.len() >= 20, "{} keys", state.entries.len());
        if fixture.knockout {
            assert!(
                played.facts.shootout,
                "the knockout fixture reaches a shoot-out"
            );
        }
    }
}

#[test]
fn no_draw_bypasses_the_registry() {
    const PATTERNS: [&str; 9] = [
        "EngineRng",
        "ChaCha",
        "rand::",
        "rand_chacha",
        ".random(",
        ".random::<",
        "random_range",
        "next_u32",
        "next_u64",
    ];
    // Offline generation, a maths test's input sampler, the fast model and its events, which
    // play no match stream, and the sensitivity rules' design and bootstrap, which choose
    // matches and resample results outside any match, keep their own generator.
    const ALLOWED: [&str; 8] = [
        "streams/",
        "sensitivity/",
        "rng.rs",
        "data/generator.rs",
        "data/names.rs",
        "math.rs",
        "modules/fast_model.rs",
        "modules/fast_events.rs",
    ];
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let files = common::sources(&root);
    let mut offenders = Vec::new();
    let mut control = false;
    for file in files {
        let rel = file
            .strip_prefix(&root)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        let text = std::fs::read_to_string(&file).unwrap();
        let hits = PATTERNS.iter().any(|p| text.contains(p));
        if rel == "rng.rs" {
            control = hits;
        }
        if hits && !ALLOWED.iter().any(|a| rel.starts_with(a) || rel == *a) {
            offenders.push(rel);
        }
    }
    assert!(control, "the search finds the generator in rng.rs");
    assert!(
        offenders.is_empty(),
        "these files use a generator outside the registry: {offenders:?}"
    );
}

#[test]
fn scripted_draws_feed_only_their_own_class() {
    let tackle = Key {
        action: Action::Tackle,
        player: PlayerKey::of_squad(0, 1),
    };
    let injury = Key {
        action: Action::InjuryTackle,
        player: PlayerKey::of_squad(0, 1),
    };
    let shot = Key {
        action: Action::ShotScore,
        player: PlayerKey::of_squad(0, 1),
    };
    let mut s = Streams::keyed(42);
    s.script_injuries(&[0.75]);
    s.script_referee(&[0.25]);
    // The same registry with nothing scripted.
    let mut oracle = Streams::keyed(42);
    assert_eq!(
        s.draw(shot).to_bits(),
        oracle.draw(shot).to_bits(),
        "unscripted"
    );
    assert_eq!(s.draw(tackle), 0.25);
    assert_eq!(
        s.draw(tackle).to_bits(),
        oracle.draw(tackle).to_bits(),
        "an empty referee queue reads the stream, not the injury queue"
    );
    assert_eq!(s.draw(injury), 0.75);
    assert_eq!(s.draws(), 4);
}
