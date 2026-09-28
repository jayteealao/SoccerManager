//! The keyed stream registry: every random draw of match play goes through here, under a key
//! from the stream-id table ([`table`]).
//!
//! Matches play scheme 1 (keyed): each key has its own stream, the match's one seed with the
//! key's stream id set through `set_stream` (source: `rand_chacha-0.10.0/src/chacha.rs:179-186`
//! in the local cargo registry), so extra draws for one key leave every other key's sequence
//! unchanged. A draw is `random::<f64>()`, one 64-bit output (two 32-bit words). Scheme 0, one
//! shared stream for every key, was played before the one recorded result change and is gone;
//! its id is refused.
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
    /// Scheme 1: every key reads its own stream.
    Keyed,
}

impl Scheme {
    /// The scheme's wire id.
    pub fn id(self) -> u8 {
        match self {
            Scheme::Keyed => KEYED_SCHEME,
        }
    }

    /// The scheme a wire id names.
    pub fn from_id(id: u8) -> Option<Self> {
        match id {
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
    /// The generator seed every stream starts from.
    seed: [u8; 32],
    /// The streams used so far, in first-use order, with their ids.
    used: Vec<(u64, ChaCha8Rng)>,
    /// Per key (by [`Key::index`]), its place in `used`, or `UNUSED`.
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
    /// The streams of a match on `seed`, the generator seed of `ChaCha8Rng::seed_from_u64`.
    pub fn keyed(seed: u64) -> Self {
        Self::with(ChaCha8Rng::seed_from_u64(seed).get_seed())
    }

    fn with(seed: [u8; 32]) -> Self {
        Self {
            seed,
            used: Vec::new(),
            index: vec![UNUSED; KEY_COUNT],
            draws: 0,
            #[cfg(feature = "scenario")]
            referee: std::collections::VecDeque::new(),
            #[cfg(feature = "scenario")]
            injury: std::collections::VecDeque::new(),
        }
    }

    pub fn scheme(&self) -> Scheme {
        Scheme::Keyed
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
        self.stream(key).random::<f64>()
    }

    /// A value in `[lo, hi)` for `key`.
    pub fn range(&mut self, key: Key, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.draw(key)
    }

    /// `true` with probability `p`, for `key`.
    pub fn chance(&mut self, key: Key, p: f64) -> bool {
        self.draw(key) < p
    }

    /// The stream of `key`, created at word position 0 on first use.
    fn stream(&mut self, key: Key) -> &mut ChaCha8Rng {
        // A debug or test build has already refused an unregistered key in `draw`; a release
        // build draws only table keys.
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
    /// scheme 1 with every used stream in ascending stream id.
    pub fn stream_state(&self) -> StreamState {
        let mut entries: Vec<(u64, u128)> = self
            .used
            .iter()
            .map(|(id, rng)| (*id, rng.get_word_pos()))
            .collect();
        entries.sort_unstable_by_key(|&(id, _)| id);
        StreamState {
            scheme: KEYED_SCHEME,
            entries,
        }
    }

    /// The registry a stream state continues: every listed stream at its word position. A
    /// key that is not listed starts at position 0 on first use, as in an uninterrupted
    /// match. Refuses a malformed entry by name.
    pub fn restore(seed: [u8; 32], entries: &[(u64, u128)]) -> Result<Self, String> {
        let malformed = |k: usize, why: String| format!("malformed stream entry {k}: {why}");
        for (k, &(_, word_pos)) in entries.iter().enumerate() {
            if word_pos >= WORD_POS_END {
                return Err(malformed(
                    k,
                    format!("word position {word_pos} is past the end of a stream"),
                ));
            }
        }
        if entries.len() > KEY_COUNT {
            return Err(format!(
                "malformed stream entry count: {} entries, more than the {KEY_COUNT} keys",
                entries.len()
            ));
        }
        let mut s = Self::with(seed);
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
            s.index[key.index().expect("a derived key is registered")] = (s.used.len() - 1) as u16;
        }
        Ok(s)
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
    fn restore_continues_every_stream_draw_for_draw() {
        let mut a = Streams::keyed(7);
        let keys = mixed_keys();
        for i in 0..50 {
            a.draw(keys[i % keys.len()]);
        }
        let state = a.stream_state();
        let mut b = Streams::restore(a.seed(), &state.entries).unwrap();
        assert_eq!(b.stream_state(), state);
        // A key first used after the restore starts where it would have.
        let late = p(Action::CrossLoft, 1, 30);
        for i in 0..50 {
            let key = if i % 7 == 0 {
                late
            } else {
                keys[i % keys.len()]
            };
            assert_eq!(a.draw(key).to_bits(), b.draw(key).to_bits());
        }
        assert_eq!(a.stream_state(), b.stream_state());
    }

    #[test]
    fn restore_refuses_a_malformed_entry_by_name() {
        let seed = Streams::keyed(1).seed();
        let refusal = |entries: &[(u64, u128)]| Streams::restore(seed, entries).err().unwrap();
        let a = p(Action::ShotScore, 0, 1).stream_id();
        let b = p(Action::Tackle, 1, 1).stream_id();
        assert!(refusal(&[(a, WORD_POS_END)]).contains("malformed stream entry 0: word position"));
        assert!(
            refusal(&[(b, 2), (a, 2)])
                .contains("malformed stream entry 1: the stream ids are not in ascending order")
        );
        assert!(
            refusal(&[(a, 2), (a | 0xFF, 2)]).contains("malformed stream entry 1: no table key")
        );
        assert!(refusal(&[(0, 2)]).contains("malformed stream entry 0: no table key"));
    }

    #[test]
    #[should_panic(expected = "unregistered stream key laws.added_time team 0 squad 3")]
    fn a_draw_on_a_key_outside_the_table_panics_and_names_it() {
        Streams::keyed(1).draw(p(Action::AddedTime, 0, 3));
    }

    #[test]
    #[should_panic(expected = "unregistered stream key laws.tackle team 1 squad 40")]
    fn a_draw_for_squad_40_panics_and_names_it() {
        Streams::keyed(1).draw(p(Action::Tackle, 1, 40));
    }
}
