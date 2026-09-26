//! The tick codec: positions in signed centimetres, one 98-byte keyframe every
//! `keyframe_interval` ticks and a 47-byte delta between. Encoding writes into a caller-owned
//! buffer, so the hot path allocates nothing (plan risk R1).

use crate::ProtocolError;

/// Players carried by one tick.
pub const PLAYER_COUNT: usize = 22;
/// Payload bytes of a keyframe: tick (4) + ball (6) + 22 players x 4.
pub const KEYFRAME_BYTES: usize = 4 + 6 + PLAYER_COUNT * 4;
/// Payload bytes of a delta: ball (3) + 22 players x 2. The tick is implied.
pub const DELTA_BYTES: usize = 3 + PLAYER_COUNT * 2;
/// Ticks between keyframes when the tuning file names no other value.
pub const DEFAULT_KEYFRAME_INTERVAL: u32 = 50;

/// One tick with every position quantised to a signed centimetre.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Quantised {
    pub tick: u32,
    pub ball: [i16; 3],
    pub players: [[i16; 2]; PLAYER_COUNT],
}

/// Metres to centimetres, saturating at the `i16` range.
fn cm(metres: f32) -> i16 {
    let v = (f64::from(metres) * 100.0).round();
    v.clamp(f64::from(i16::MIN), f64::from(i16::MAX)) as i16
}

/// Centimetres back to metres.
fn metres(centimetres: i16) -> f32 {
    f32::from(centimetres) / 100.0
}

impl Quantised {
    /// Quantises one tick of engine positions.
    pub fn from_metres(tick: u32, ball: [f32; 3], players: &[[f32; 2]; PLAYER_COUNT]) -> Self {
        let mut out = [[0i16; 2]; PLAYER_COUNT];
        for (slot, p) in out.iter_mut().zip(players.iter()) {
            *slot = [cm(p[0]), cm(p[1])];
        }
        Self {
            tick,
            ball: [cm(ball[0]), cm(ball[1]), cm(ball[2])],
            players: out,
        }
    }

    /// The ball in metres.
    pub fn ball_metres(&self) -> [f32; 3] {
        [
            metres(self.ball[0]),
            metres(self.ball[1]),
            metres(self.ball[2]),
        ]
    }

    /// Every player in metres, in roster order.
    pub fn players_metres(&self) -> [[f32; 2]; PLAYER_COUNT] {
        let mut out = [[0.0f32; 2]; PLAYER_COUNT];
        for (slot, p) in out.iter_mut().zip(self.players.iter()) {
            *slot = [metres(p[0]), metres(p[1])];
        }
        out
    }

    /// Every component in wire order: three ball values, then two per player.
    fn components(&self) -> [i16; 3 + PLAYER_COUNT * 2] {
        let mut out = [0i16; 3 + PLAYER_COUNT * 2];
        out[0..3].copy_from_slice(&self.ball);
        for (i, p) in self.players.iter().enumerate() {
            out[3 + i * 2] = p[0];
            out[4 + i * 2] = p[1];
        }
        out
    }
}

/// Writes a keyframe payload: the tick, then every position as little-endian centimetres.
pub fn encode_keyframe_into(q: &Quantised, buf: &mut [u8; KEYFRAME_BYTES]) {
    buf[0..4].copy_from_slice(&q.tick.to_le_bytes());
    let mut at = 4;
    for v in q.components() {
        buf[at..at + 2].copy_from_slice(&v.to_le_bytes());
        at += 2;
    }
}

/// Reads a keyframe payload.
pub fn decode_keyframe(buf: &[u8; KEYFRAME_BYTES]) -> Quantised {
    let at = |i: usize| i16::from_le_bytes([buf[4 + i * 2], buf[5 + i * 2]]);
    let mut players = [[0i16; 2]; PLAYER_COUNT];
    for (i, p) in players.iter_mut().enumerate() {
        *p = [at(3 + i * 2), at(4 + i * 2)];
    }
    Quantised {
        tick: u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]),
        ball: [at(0), at(1), at(2)],
        players,
    }
}

/// Writes a delta payload against `prev`. Returns `None` when any step is wider than one
/// signed byte (1.27 m in a tick), which is the caller's signal to send a keyframe instead.
pub fn encode_delta_into(
    cur: &Quantised,
    prev: &Quantised,
    buf: &mut [u8; DELTA_BYTES],
) -> Option<()> {
    let a = cur.components();
    let b = prev.components();
    for (slot, (x, y)) in buf.iter_mut().zip(a.iter().zip(b.iter())) {
        let step = i32::from(*x) - i32::from(*y);
        *slot = i8::try_from(step).ok()?.to_le_bytes()[0];
    }
    Some(())
}

/// Reads a delta payload against `prev`. The tick is `prev.tick + 1`.
pub fn decode_delta(prev: &Quantised, buf: &[u8; DELTA_BYTES]) -> Result<Quantised, ProtocolError> {
    let base = prev.components();
    let mut out = [0i16; 3 + PLAYER_COUNT * 2];
    for (i, slot) in out.iter_mut().enumerate() {
        let step = i16::from(buf[i] as i8);
        *slot = base[i]
            .checked_add(step)
            .ok_or(ProtocolError::DeltaOutOfRange {
                tick: prev.tick + 1,
            })?;
    }
    let mut players = [[0i16; 2]; PLAYER_COUNT];
    for (i, p) in players.iter_mut().enumerate() {
        *p = [out[3 + i * 2], out[4 + i * 2]];
    }
    Ok(Quantised {
        tick: prev.tick + 1,
        ball: [out[0], out[1], out[2]],
        players,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(tick: u32, shift: f32) -> Quantised {
        let mut players = [[0.0f32; 2]; PLAYER_COUNT];
        for (i, p) in players.iter_mut().enumerate() {
            *p = [i as f32 + shift, -(i as f32) - shift];
        }
        Quantised::from_metres(tick, [1.5 + shift, -2.5, 0.25], &players)
    }

    #[test]
    fn a_keyframe_round_trips() {
        let q = sample(7, 0.0);
        let mut buf = [0u8; KEYFRAME_BYTES];
        encode_keyframe_into(&q, &mut buf);
        assert_eq!(decode_keyframe(&buf), q);
        assert_eq!(KEYFRAME_BYTES, 98);
        assert_eq!(DELTA_BYTES, 47);
    }

    #[test]
    fn a_step_at_the_ball_speed_cap_encodes_as_a_delta() {
        // 40 m/s at 50 ticks per second is 0.80 m, which is 80 cm, inside the signed byte.
        let prev = sample(1, 0.0);
        let cur = sample(2, 0.80);
        let mut buf = [0u8; DELTA_BYTES];
        assert!(encode_delta_into(&cur, &prev, &mut buf).is_some());
        assert_eq!(decode_delta(&prev, &buf).unwrap(), cur);
    }

    #[test]
    fn a_two_metre_jump_forces_a_keyframe() {
        let prev = sample(1, 0.0);
        let cur = sample(2, 2.0);
        let mut buf = [0u8; DELTA_BYTES];
        assert!(encode_delta_into(&cur, &prev, &mut buf).is_none());
    }

    #[test]
    fn a_full_keyframe_cycle_reconstructs_every_position() {
        let mut key = [0u8; KEYFRAME_BYTES];
        let first = sample(100, 0.0);
        encode_keyframe_into(&first, &mut key);
        let mut decoded = decode_keyframe(&key);
        assert_eq!(decoded, first);
        for step in 1..DEFAULT_KEYFRAME_INTERVAL {
            let cur = sample(100 + step, step as f32 * 0.1);
            let mut delta = [0u8; DELTA_BYTES];
            encode_delta_into(&cur, &decoded, &mut delta).expect("0.1 m fits in a delta");
            decoded = decode_delta(&decoded, &delta).unwrap();
            assert_eq!(decoded, cur, "tick {}", 100 + step);
            for (a, b) in decoded.players_metres().iter().zip(cur.players_metres()) {
                assert!((a[0] - b[0]).abs() <= 0.01 && (a[1] - b[1]).abs() <= 0.01);
            }
        }
    }
}
