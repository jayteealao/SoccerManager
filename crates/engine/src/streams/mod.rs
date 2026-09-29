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

pub use table::{Action, Class, KEY_COUNT, KEYED_SCHEME, Key, PlayerKey};

/// A word position at or past this is outside a ChaCha stream (a 68-bit number; source:
/// `rand_chacha-0.10.0/src/chacha.rs:145-165`).
pub const WORD_POS_END: u128 = 1 << 68;

/// The random-stream scheme this build plays ([`Scheme`]): scheme 1, one
/// keyed stream per key. The golden file's ledger records it with every entry.
pub const STREAM_SCHEME: u8 = KEYED_SCHEME;

/// The position of every random stream a match draws from, as the replay gate hashes it:
/// the stream scheme, then each stream's id and word position in ascending stream id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamState {
    pub scheme: u8,
    /// `(stream id, word position)` per stream, in stream order.
    pub entries: Vec<(u64, u128)>,
}

impl StreamState {
    /// The canonical bytes: the scheme id (u8), the entry count (u32), then per entry the
    /// stream id (u64) and the word position (u128), all little-endian.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(5 + 24 * self.entries.len());
        out.push(self.scheme);
        // A match has a handful of streams, far below 4 billion.
        out.extend_from_slice(&(self.entries.len() as u32).to_le_bytes());
        for (stream, word_pos) in &self.entries {
            out.extend_from_slice(&stream.to_le_bytes());
            out.extend_from_slice(&word_pos.to_le_bytes());
        }
        out
    }
}

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
    /// Places in `used`, kept in ascending stream id, so the state is read without sorting.
    order: Vec<u16>,
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
    /// The debug trace, when debug mode is on. Not match state: never hashed, never in a
    /// snapshot.
    #[cfg(feature = "debug-trace")]
    trace: Option<Box<crate::trace::Trace>>,
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
            order: Vec::new(),
            index: vec![UNUSED; KEY_COUNT],
            draws: 0,
            #[cfg(feature = "scenario")]
            referee: std::collections::VecDeque::new(),
            #[cfg(feature = "scenario")]
            injury: std::collections::VecDeque::new(),
            #[cfg(feature = "debug-trace")]
            trace: None,
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
        self.draw_with(key, &[])
    }

    /// A value in `[0, 1)` for `key` that the caller tests against `thresholds`: one
    /// probability, or two cumulative thresholds for a three-way split. The caller keeps its
    /// own comparison; the thresholds only reach the debug trace.
    pub fn tested(&mut self, key: Key, thresholds: &[f64]) -> f64 {
        self.draw_with(key, thresholds)
    }

    /// The registry's only draw call: every draw, scripted or not, is counted here and, in
    /// debug mode, recorded here before it is returned.
    #[inline]
    fn draw_with(&mut self, key: Key, thresholds: &[f64]) -> f64 {
        #[cfg(any(debug_assertions, feature = "scenario"))]
        if !key.registered() {
            panic!("unregistered stream key {}", key.name());
        }
        self.draws += 1;
        #[cfg(feature = "scenario")]
        let scripted = self.scripted(key);
        #[cfg(not(feature = "scenario"))]
        let scripted: Option<f64> = None;
        let value = match scripted {
            Some(draw) => draw,
            None => self.stream(key).random::<f64>(),
        };
        #[cfg(feature = "debug-trace")]
        if let Some(trace) = self.trace.as_deref_mut() {
            record_draw(trace, key, value, thresholds, scripted.is_some());
        }
        #[cfg(not(feature = "debug-trace"))]
        let _ = thresholds;
        value
    }

    /// A value in `[lo, hi)` for `key`.
    pub fn range(&mut self, key: Key, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.draw(key)
    }

    /// `true` with probability `p`, for `key`.
    pub fn chance(&mut self, key: Key, p: f64) -> bool {
        self.tested(key, &[p]) < p
    }

    /// Switches the debug trace on; its records carry `tick` until the next stamp. Does
    /// nothing in a build without the recorder.
    pub fn enable_trace(&mut self, tick: u32) {
        #[cfg(feature = "debug-trace")]
        {
            self.trace = Some(Box::new(crate::trace::Trace::new(tick)));
        }
        #[cfg(not(feature = "debug-trace"))]
        let _ = tick;
    }

    /// `true` while the debug trace is on; always `false` in a build without the recorder.
    #[inline(always)]
    pub fn trace_on(&self) -> bool {
        #[cfg(feature = "debug-trace")]
        {
            self.trace.is_some()
        }
        #[cfg(not(feature = "debug-trace"))]
        {
            false
        }
    }

    /// The debug trace, while it is on.
    pub(crate) fn trace_mut(&mut self) -> Option<&mut crate::trace::Trace> {
        #[cfg(feature = "debug-trace")]
        {
            self.trace.as_deref_mut()
        }
        #[cfg(not(feature = "debug-trace"))]
        {
            None
        }
    }

    /// Stamps the tick the next trace records carry.
    #[inline]
    pub(crate) fn begin_tick(&mut self, tick: u32) {
        #[cfg(feature = "debug-trace")]
        if let Some(trace) = self.trace.as_deref_mut() {
            trace.stamp(tick);
        }
        #[cfg(not(feature = "debug-trace"))]
        let _ = tick;
    }

    /// Takes every trace record since the last call; empty while the trace is off.
    pub fn take_trace(&mut self) -> Vec<crate::trace::TraceRecord> {
        self.trace_mut().map(|t| t.take()).unwrap_or_default()
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
                let used = &self.used;
                let place = self.order.partition_point(|&o| used[usize::from(o)].0 < id);
                self.order.insert(place, at);
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
        let entries: Vec<(u64, u128)> = self
            .order
            .iter()
            .map(|&o| {
                let (id, rng) = &self.used[usize::from(o)];
                (*id, rng.get_word_pos())
            })
            .collect();
        StreamState {
            scheme: KEYED_SCHEME,
            entries,
        }
    }

    /// Writes the bytes of [`Streams::stream_state`]`().to_bytes()` straight into `w`, with
    /// no list built or sorted: the scheme id (u8), the stream count (u32), then each stream's
    /// id (u64) and word position (u128), in ascending stream id.
    pub(crate) fn write_state(&self, w: &mut crate::canon::Writer) {
        w.u8(KEYED_SCHEME);
        w.count(self.order.len());
        for &o in &self.order {
            let (id, rng) = &self.used[usize::from(o)];
            w.u64(*id);
            w.raw(&rng.get_word_pos().to_le_bytes());
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
            // The entries arrive in ascending id, so the order is the fill order.
            s.order.push((s.used.len() - 1) as u16);
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

/// Pushes one draw record, out of line so the draw path stays one branch while the trace is
/// off.
#[cfg(feature = "debug-trace")]
#[inline(never)]
#[cold]
fn record_draw(
    trace: &mut crate::trace::Trace,
    key: Key,
    value: f64,
    thresholds: &[f64],
    scripted: bool,
) {
    trace.push_draw(key, value, thresholds, scripted);
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
    fn write_state_writes_the_bytes_of_the_stream_state() {
        let mut s = Streams::keyed(9);
        let mut w = crate::canon::Writer::default();
        s.write_state(&mut w);
        assert_eq!(w.bytes(), s.stream_state().to_bytes());
        let keys = mixed_keys();
        for i in 0..40 {
            s.draw(keys[(i * 3) % keys.len()]);
        }
        let mut w = crate::canon::Writer::default();
        s.write_state(&mut w);
        assert_eq!(w.bytes(), s.stream_state().to_bytes());
        let restored = Streams::restore(s.seed(), &s.stream_state().entries).unwrap();
        let mut w = crate::canon::Writer::default();
        restored.write_state(&mut w);
        assert_eq!(w.bytes(), s.stream_state().to_bytes());
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

    #[cfg(feature = "debug-trace")]
    mod recorder {
        use super::*;
        use crate::trace::TraceRecord;

        fn draws(records: &[TraceRecord]) -> Vec<&crate::trace::DrawRecord> {
            records
                .iter()
                .filter_map(|r| match r {
                    TraceRecord::Draw(d) => Some(d),
                    TraceRecord::Point(_) => None,
                })
                .collect()
        }

        #[test]
        fn the_trace_is_off_by_default_and_records_nothing() {
            let mut s = Streams::keyed(3);
            assert!(!s.trace_on());
            for key in mixed_keys() {
                s.draw(key);
            }
            assert!(s.take_trace().is_empty());
        }

        #[test]
        fn every_draw_is_recorded_with_its_key_index_and_value() {
            let mut s = Streams::keyed(3);
            let mut plain = Streams::keyed(3);
            s.enable_trace(7);
            let keys = mixed_keys();
            let mut values = Vec::new();
            for i in 0..40 {
                let key = keys[i % keys.len()];
                let v = s.draw(key);
                assert_eq!(
                    v.to_bits(),
                    plain.draw(key).to_bits(),
                    "the trace moves no value"
                );
                values.push((key, v));
                if i == 20 {
                    s.begin_tick(8);
                }
            }
            s.range(keys[0], -1.0, 1.0);
            let records = s.take_trace();
            let d = draws(&records);
            assert_eq!(d.len() as u64, s.draws(), "one record per counted draw");
            for (k, (key, v)) in values.iter().enumerate() {
                assert_eq!(d[k].key, *key);
                assert_eq!(d[k].value.to_bits(), v.to_bits());
                assert_eq!(d[k].index, (k / keys.len()) as u64, "index counts per key");
                assert_eq!(d[k].tick, if k <= 20 { 7 } else { 8 });
                assert!(d[k].thresholds.is_empty());
                assert!(!d[k].scripted);
            }
            assert_eq!(d[40].index, 8);
            assert!(s.take_trace().is_empty(), "take drains the buffer");
        }

        #[test]
        fn a_chance_draw_records_its_probability() {
            let mut s = Streams::keyed(5);
            s.enable_trace(1);
            let key = p(Action::KeeperCatch, 0, 0);
            let hit = s.chance(key, 0.25);
            let split = p(Action::Tackle, 1, 3);
            let v = s.tested(split, &[0.1, 0.3]);
            let records = s.take_trace();
            let d = draws(&records);
            assert_eq!(d[0].thresholds, vec![0.25]);
            assert_eq!(hit, d[0].value < 0.25);
            assert_eq!(d[1].thresholds, vec![0.1, 0.3]);
            assert_eq!(d[1].value.to_bits(), v.to_bits());
        }

        #[test]
        #[cfg(feature = "scenario")]
        fn a_scripted_draw_is_recorded_and_marked() {
            let mut s = Streams::keyed(5);
            s.enable_trace(1);
            s.script_referee(&[0.125]);
            let v = s.draw(p(Action::Tackle, 0, 2));
            assert_eq!(v, 0.125);
            s.draw(p(Action::Tackle, 0, 2));
            let records = s.take_trace();
            let d = draws(&records);
            assert_eq!(d.len() as u64, s.draws());
            assert!(d[0].scripted);
            assert!(!d[1].scripted);
            assert_eq!(d[1].index, 1);
        }

        #[test]
        fn a_clone_carries_the_buffer_and_a_restore_keeps_the_count() {
            let mut s = Streams::keyed(9);
            s.enable_trace(1);
            s.draw(p(Action::ShotAim, 0, 5));
            let mut copy = s.clone();
            copy.draw(p(Action::ShotAim, 0, 5));
            assert_eq!(copy.take_trace().len(), 2, "the clone has its own buffer");
            assert_eq!(s.take_trace().len(), 1, "the original keeps its own");
            // A restored clone: the restored registry's count and buffer agree.
            let real = s.clone();
            s.draw(p(Action::PassScore, 1, 1));
            s = real;
            s.draw(p(Action::PassScore, 1, 1));
            assert_eq!(s.take_trace().len() as u64, s.draws() - 1);
        }
    }
}
