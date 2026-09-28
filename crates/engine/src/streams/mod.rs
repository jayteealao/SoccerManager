//! The keyed stream registry: every random draw of match play goes through here, under a key
//! from the stream-id table ([`table`]).
//!
//! Two schemes exist. Scheme 0 (legacy, the one matches play) reads every key from one
//! shared `ChaCha8Rng` stream in draw order, so extra draws for one key shift every later
//! draw. Scheme 1 (keyed, built only in test builds until it is switched on) gives each key
//! its own stream: the match's one seed with the key's stream id set through `set_stream`
//! (source: `rand_chacha-0.10.0/src/chacha.rs:179-186` in the local cargo registry), so extra
//! draws for one key leave every other key's sequence unchanged. Both schemes convert a draw
//! the same way: `random::<f64>()`, one 64-bit output (two 32-bit words) per draw.
//!
//! A test scene may script draws: each row names the queue (referee or injury) a draw reads
//! first, before the stream.

pub mod table;

use rand::{RngExt, SeedableRng};
use rand_chacha::ChaCha8Rng;

use crate::rng::StreamState;

pub use table::{Action, Class, KEY_COUNT, KEYED_SCHEME, Key, PlayerKey};

/// A word position at or past this is outside a ChaCha stream (a 68-bit number; source:
/// `rand_chacha-0.10.0/src/chacha.rs:145-165`).
pub const WORD_POS_END: u128 = 1 << 68;

/// How keys map to streams.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scheme {
    /// Scheme 0: every key reads the one shared stream in draw order.
    Legacy,
    /// Scheme 1: every key reads its own stream.
    Keyed,
}

impl Scheme {
    /// The scheme's wire id.
    pub fn id(self) -> u8 {
        match self {
            Scheme::Legacy => 0,
            Scheme::Keyed => KEYED_SCHEME,
        }
    }

    /// The scheme a wire id names.
    pub fn from_id(id: u8) -> Option<Self> {
        match id {
            0 => Some(Scheme::Legacy),
            KEYED_SCHEME => Some(Scheme::Keyed),
            _ => None,
        }
    }
}

/// Unused slot in the dense key index.
const UNUSED: u16 = u16::MAX;

/// The streams of one match.
#[derive(Debug, Clone)]
pub struct Streams {
    scheme: Scheme,
    /// The generator seed every stream starts from.
    seed: [u8; 32],
    /// Scheme 0's one stream (stream id 0).
    shared: ChaCha8Rng,
    /// Scheme 1's streams used so far, in first-use order, with their ids.
    used: Vec<(u64, ChaCha8Rng)>,
    /// Scheme 1: per key (by [`Key::index`]), its place in `used`, or `UNUSED`. Empty in
    /// scheme 0.
    index: Vec<u16>,
    /// Draws served, scripted or not. Not match state: never hashed, never in a snapshot.
    draws: u64,
    /// Draws a test scripted for referee-class keys, served before the stream.
    #[cfg(feature = "scenario")]
    referee: std::collections::VecDeque<f64>,
    /// Draws a test scripted for injury-class keys, served before the stream. They are kept
    /// apart from the referee's so a law test's scripted draws never feed an injury roll.
    #[cfg(feature = "scenario")]
    injury: std::collections::VecDeque<f64>,
}

impl Streams {
    /// Scheme 0 for a match on `seed`: the one stream matches play today.
    pub fn legacy(seed: u64) -> Self {
        let shared = ChaCha8Rng::seed_from_u64(seed);
        Self::with(shared.get_seed(), shared, Scheme::Legacy)
    }

    /// Scheme 1 for a match on `seed` (test builds and `keyed-streams` builds only until the
    /// scheme is switched on).
    #[cfg(any(test, feature = "scenario", feature = "keyed-streams"))]
    pub fn keyed(seed: u64) -> Self {
        Self::new(seed, Scheme::Keyed)
    }

    /// A fresh registry for a match on `seed` in `scheme` (test builds only).
    #[cfg(any(test, feature = "scenario", feature = "keyed-streams"))]
    pub fn new(seed: u64, scheme: Scheme) -> Self {
        let mut s = Self::legacy(seed);
        s.set_scheme(scheme);
        s
    }

    /// Switches a registry that has served no draw to `scheme`; scripted draws stay queued
    /// (test builds only).
    #[cfg(any(test, feature = "scenario", feature = "keyed-streams"))]
    pub fn set_scheme(&mut self, scheme: Scheme) {
        assert_eq!(self.draws, 0, "the scheme is chosen before the first draw");
        self.scheme = scheme;
        self.index = match scheme {
            Scheme::Legacy => Vec::new(),
            Scheme::Keyed => vec![UNUSED; KEY_COUNT],
        };
    }

    fn with(seed: [u8; 32], shared: ChaCha8Rng, scheme: Scheme) -> Self {
        Self {
            scheme,
            seed,
            shared,
            used: Vec::new(),
            index: Vec::new(),
            draws: 0,
            #[cfg(feature = "scenario")]
            referee: std::collections::VecDeque::new(),
            #[cfg(feature = "scenario")]
            injury: std::collections::VecDeque::new(),
        }
    }

    pub fn scheme(&self) -> Scheme {
        self.scheme
    }

    /// The generator seed every stream starts from.
    pub fn seed(&self) -> [u8; 32] {
        self.seed
    }

    /// Draws served so far (scripted draws included).
    pub fn draws(&self) -> u64 {
        self.draws
    }

    /// A value in `[0, 1)` for `key`.
    ///
    /// In a debug or test build, a key the table does not hold (a player key on a match row,
    /// the match key on a player row, or a squad index of 40 or more) panics and names it.
    pub fn draw(&mut self, key: Key) -> f64 {
        #[cfg(any(debug_assertions, feature = "scenario"))]
        if !key.registered() {
            panic!("unregistered stream key {}", key.name());
        }
        self.draws += 1;
        #[cfg(feature = "scenario")]
        if let Some(draw) = self.scripted(key) {
            return draw;
        }
        match self.scheme {
            Scheme::Legacy => self.shared.random::<f64>(),
            Scheme::Keyed => self.stream(key).random::<f64>(),
        }
    }

    /// A value in `[lo, hi)` for `key`.
    pub fn range(&mut self, key: Key, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.draw(key)
    }

    /// `true` with probability `p`, for `key`.
    pub fn chance(&mut self, key: Key, p: f64) -> bool {
        self.draw(key) < p
    }

    /// Scheme 1: the stream of `key`, created at word position 0 on first use.
    fn stream(&mut self, key: Key) -> &mut ChaCha8Rng {
        // `draw` has already refused an unregistered key in every build that can pick
        // scheme 1.
        let slot = key.index().expect("a registered key");
        let at = match self.index[slot] {
            UNUSED => {
                let id = key.stream_id();
                self.used.push((id, self.fresh(id)));
                // At most `KEY_COUNT` (2,916) streams, below `u16::MAX`.
                let at = (self.used.len() - 1) as u16;
                self.index[slot] = at;
                at
            }
            at => at,
        };
        &mut self.used[usize::from(at)].1
    }

    fn fresh(&self, id: u64) -> ChaCha8Rng {
        let mut rng = ChaCha8Rng::from_seed(self.seed);
        rng.set_stream(id);
        rng
    }

    /// The position of every stream, as the replay gate hashes it and a snapshot stores it:
    /// scheme 0 with its one shared stream, or scheme 1 with every used stream in ascending
    /// stream id.
    pub fn stream_state(&self) -> StreamState {
        let entries = match self.scheme {
            Scheme::Legacy => vec![(self.shared.get_stream(), self.shared.get_word_pos())],
            Scheme::Keyed => {
                let mut e: Vec<(u64, u128)> = self
                    .used
                    .iter()
                    .map(|(id, rng)| (*id, rng.get_word_pos()))
                    .collect();
                e.sort_unstable_by_key(|&(id, _)| id);
                e
            }
        };
        StreamState {
            scheme: self.scheme.id(),
            entries,
        }
    }

    /// The registry a stream state continues: every listed stream at its word position. A
    /// key that is not listed starts at position 0 on first use, as in an uninterrupted
    /// match. Refuses a malformed entry by name.
    pub fn restore(
        seed: [u8; 32],
        scheme: Scheme,
        entries: &[(u64, u128)],
    ) -> Result<Self, String> {
        let malformed = |k: usize, why: String| format!("malformed stream entry {k}: {why}");
        for (k, &(_, word_pos)) in entries.iter().enumerate() {
            if word_pos >= WORD_POS_END {
                return Err(malformed(
                    k,
                    format!("word position {word_pos} is past the end of a stream"),
                ));
            }
        }
        match scheme {
            Scheme::Legacy => {
                let [(stream, word_pos)] = entries else {
                    return Err(format!(
                        "malformed stream entry count: scheme 0 holds one stream, the snapshot lists {}",
                        entries.len()
                    ));
                };
                if *stream != 0 {
                    return Err(malformed(
                        0,
                        format!("stream {stream} is not the shared stream 0"),
                    ));
                }
                let mut shared = ChaCha8Rng::from_seed(seed);
                shared.set_word_pos(*word_pos);
                Ok(Self::with(seed, shared, Scheme::Legacy))
            }
            Scheme::Keyed => {
                if entries.len() > KEY_COUNT {
                    return Err(format!(
                        "malformed stream entry count: {} entries, more than the {KEY_COUNT} keys",
                        entries.len()
                    ));
                }
                let mut s = Self::with(seed, ChaCha8Rng::from_seed(seed), Scheme::Keyed);
                s.index = vec![UNUSED; KEY_COUNT];
                let mut last: Option<u64> = None;
                for (k, &(id, word_pos)) in entries.iter().enumerate() {
                    if last.is_some_and(|l| id <= l) {
                        return Err(malformed(
                            k,
                            "the stream ids are not in ascending order".into(),
                        ));
                    }
                    last = Some(id);
                    let Some(key) = Key::from_stream_id(id) else {
                        return Err(malformed(
                            k,
                            format!("no table key derives stream id {id:#x}"),
                        ));
                    };
                    let mut rng = s.fresh(id);
                    rng.set_word_pos(word_pos);
                    s.used.push((id, rng));
                    // At most `KEY_COUNT` entries, checked above.
                    s.index[key.index().expect("a derived key is registered")] =
                        (s.used.len() - 1) as u16;
                }
                Ok(s)
            }
        }
    }

    /// Queues draws that referee-class keys return before they read the stream.
    #[cfg(feature = "scenario")]
    pub fn script_referee(&mut self, draws: &[f64]) {
        self.referee.extend(draws.iter().copied());
    }

    /// Queues draws that injury-class keys return before they read the stream.
    #[cfg(feature = "scenario")]
    pub fn script_injuries(&mut self, draws: &[f64]) {
        self.injury.extend(draws.iter().copied());
    }

    #[cfg(feature = "scenario")]
    fn scripted(&mut self, key: Key) -> Option<f64> {
        match key.action.row().class {
            Class::Referee => self.referee.pop_front(),
            Class::Injury => self.injury.pop_front(),
            Class::Unscripted => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(action: Action, team: usize, squad: usize) -> Key {
        Key {
            action,
            player: PlayerKey::of_squad(team, squad),
        }
    }

    fn mixed_keys() -> [Key; 5] {
        [
            p(Action::PassScore, 0, 4),
            p(Action::Tackle, 1, 2),
            Key::of_match(Action::AddedTime),
            p(Action::InjuryMinute, 0, 9),
            p(Action::ParryAngle, 1, 0),
        ]
    }

    #[test]
    #[cfg(not(feature = "keyed-streams"))]
    fn legacy_draws_equal_todays_single_stream_bit_for_bit() {
        let mut oracle = crate::rng::EngineRng::from_seed(42);
        let mut s = Streams::legacy(42);
        let keys = mixed_keys();
        for i in 0..1_000 {
            let key = keys[i % keys.len()];
            match i % 3 {
                0 => assert_eq!(s.draw(key).to_bits(), oracle.next_f64().to_bits()),
                1 => assert_eq!(
                    s.range(key, -0.3, 0.7).to_bits(),
                    oracle.range_f64(-0.3, 0.7).to_bits()
                ),
                _ => assert_eq!(s.chance(key, 0.4), oracle.chance(0.4)),
            }
        }
        assert_eq!(s.draws(), 1_000);
        assert_eq!(s.stream_state(), oracle.stream_state());
    }

    #[test]
    fn a_keyed_stream_starts_at_word_zero_and_each_draw_reads_two_words() {
        let mut s = Streams::keyed(42);
        let key = p(Action::ShotAim, 0, 5);
        assert!(s.stream_state().entries.is_empty());
        s.draw(key);
        assert_eq!(s.stream_state().entries, vec![(key.stream_id(), 2)]);
        s.draw(key);
        s.draw(Key::of_match(Action::ShootoutEnd));
        let state = s.stream_state();
        assert_eq!(state.scheme, KEYED_SCHEME);
        assert_eq!(state.entries.len(), 2);
        assert!(state.entries.windows(2).all(|w| w[0].0 < w[1].0));
        assert!(state.entries.contains(&(key.stream_id(), 4)));
        assert_eq!(s.draws(), 3);
    }

    #[test]
    fn restore_continues_both_schemes_draw_for_draw() {
        for scheme in [Scheme::Legacy, Scheme::Keyed] {
            let mut a = Streams::new(7, scheme);
            let keys = mixed_keys();
            for i in 0..50 {
                a.draw(keys[i % keys.len()]);
            }
            let state = a.stream_state();
            let mut b = Streams::restore(a.seed(), scheme, &state.entries).unwrap();
            assert_eq!(b.stream_state(), state);
            // A key first used after the restore starts where it would have.
            let late = p(Action::CrossLoft, 1, 30);
            for i in 0..50 {
                let key = if i % 7 == 0 {
                    late
                } else {
                    keys[i % keys.len()]
                };
                assert_eq!(a.draw(key).to_bits(), b.draw(key).to_bits(), "{scheme:?}");
            }
            assert_eq!(a.stream_state(), b.stream_state());
        }
    }

    #[test]
    fn restore_refuses_a_malformed_entry_by_name() {
        let seed = Streams::legacy(1).seed();
        let refusal = |scheme, entries: &[(u64, u128)]| {
            Streams::restore(seed, scheme, entries).err().unwrap()
        };
        assert!(
            refusal(Scheme::Legacy, &[(0, 2), (0, 4)]).contains("malformed stream entry count")
        );
        assert!(refusal(Scheme::Legacy, &[(5, 2)]).contains("malformed stream entry 0: stream 5"));
        assert!(
            refusal(Scheme::Legacy, &[(0, WORD_POS_END)])
                .contains("malformed stream entry 0: word position")
        );
        let a = p(Action::ShotScore, 0, 1).stream_id();
        let b = p(Action::Tackle, 1, 1).stream_id();
        assert!(
            refusal(Scheme::Keyed, &[(b, 2), (a, 2)])
                .contains("malformed stream entry 1: the stream ids are not in ascending order")
        );
        assert!(
            refusal(Scheme::Keyed, &[(a, 2), (a | 0xFF, 2)])
                .contains("malformed stream entry 1: no table key")
        );
    }

    #[test]
    #[should_panic(expected = "unregistered stream key laws.added_time team 0 squad 3")]
    fn a_draw_on_a_key_outside_the_table_panics_and_names_it() {
        Streams::keyed(1).draw(p(Action::AddedTime, 0, 3));
    }

    #[test]
    #[should_panic(expected = "unregistered stream key laws.tackle team 1 squad 40")]
    fn a_draw_for_squad_40_panics_and_names_it() {
        Streams::legacy(1).draw(p(Action::Tackle, 1, 40));
    }
}
