//! Wire frames: the one place that decides ticks travel as binary frames and control
//! messages as JSON text frames. A tick frame is a fixed-size buffer, never a heap
//! allocation, so the producer copies rather than allocates once per tick.

use crate::ProtocolError;
use crate::codec::{
    DELTA_BYTES, KEYFRAME_BYTES, Quantised, decode_delta, decode_keyframe, encode_delta_into,
    encode_keyframe_into,
};

/// A keyframe: every position, absolute.
pub const KIND_KEYFRAME: u8 = 0x01;
/// A delta against the previous tick.
pub const KIND_DELTA: u8 = 0x02;
/// A keyframe at a restart tick (kick-off). The validator reads this instead of guessing.
pub const KIND_RESTART: u8 = 0x03;
/// The largest binary tick frame: the kind tag plus a keyframe payload.
pub const MAX_TICK_FRAME: usize = 1 + KEYFRAME_BYTES;

/// One binary tick frame, ready to send. `len` bytes of `bytes` are live.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TickFrame {
    bytes: [u8; MAX_TICK_FRAME],
    len: u8,
}

impl TickFrame {
    /// A keyframe carrying every absolute position.
    pub fn keyframe(q: &Quantised, restart: bool) -> Self {
        let mut bytes = [0u8; MAX_TICK_FRAME];
        bytes[0] = if restart { KIND_RESTART } else { KIND_KEYFRAME };
        let mut payload = [0u8; KEYFRAME_BYTES];
        encode_keyframe_into(q, &mut payload);
        bytes[1..1 + KEYFRAME_BYTES].copy_from_slice(&payload);
        Self {
            bytes,
            len: (1 + KEYFRAME_BYTES) as u8,
        }
    }

    /// A delta against `prev`, or `None` when a step is too wide for one signed byte.
    pub fn delta(cur: &Quantised, prev: &Quantised) -> Option<Self> {
        let mut payload = [0u8; DELTA_BYTES];
        encode_delta_into(cur, prev, &mut payload)?;
        let mut bytes = [0u8; MAX_TICK_FRAME];
        bytes[0] = KIND_DELTA;
        bytes[1..1 + DELTA_BYTES].copy_from_slice(&payload);
        Some(Self {
            bytes,
            len: (1 + DELTA_BYTES) as u8,
        })
    }

    /// Adopts bytes read from a socket or a fixture.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ProtocolError> {
        let expected = match bytes.first() {
            Some(&KIND_KEYFRAME | &KIND_RESTART) => 1 + KEYFRAME_BYTES,
            Some(&KIND_DELTA) => 1 + DELTA_BYTES,
            Some(&other) => return Err(ProtocolError::UnknownFrameKind(other)),
            None => return Err(ProtocolError::EmptyFrame),
        };
        if bytes.len() != expected {
            return Err(ProtocolError::FrameLength {
                kind: bytes[0],
                expected,
                found: bytes.len(),
            });
        }
        let mut buf = [0u8; MAX_TICK_FRAME];
        buf[..expected].copy_from_slice(bytes);
        Ok(Self {
            bytes: buf,
            len: expected as u8,
        })
    }

    /// The kind tag.
    pub fn kind(&self) -> u8 {
        self.bytes[0]
    }

    /// True when this frame carries a restart (kick-off) tick.
    pub fn is_restart(&self) -> bool {
        self.bytes[0] == KIND_RESTART
    }

    /// The bytes exactly as they travel on the wire.
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.len as usize]
    }

    /// The absolute positions this frame carries. A delta needs the previous tick.
    pub fn decode(&self, prev: Option<&Quantised>) -> Result<Quantised, ProtocolError> {
        match self.kind() {
            KIND_KEYFRAME | KIND_RESTART => {
                let payload: [u8; KEYFRAME_BYTES] = self.bytes[1..1 + KEYFRAME_BYTES]
                    .try_into()
                    .expect("keyframe payload");
                Ok(decode_keyframe(&payload))
            }
            KIND_DELTA => {
                let prev = prev.ok_or(ProtocolError::DeltaWithoutKeyframe)?;
                let payload: [u8; DELTA_BYTES] = self.bytes[1..1 + DELTA_BYTES]
                    .try_into()
                    .expect("delta payload");
                decode_delta(prev, &payload)
            }
            other => Err(ProtocolError::UnknownFrameKind(other)),
        }
    }
}

/// One message on the wire: a binary tick frame, or a JSON text frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Frame {
    Tick(TickFrame),
    Text(String),
}

impl Frame {
    /// The bytes this frame puts on the wire.
    pub fn payload(&self) -> &[u8] {
        match self {
            Frame::Tick(f) => f.as_bytes(),
            Frame::Text(s) => s.as_bytes(),
        }
    }

    /// True for a JSON text frame.
    pub fn is_text(&self) -> bool {
        matches!(self, Frame::Text(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::PLAYER_COUNT;

    fn sample(tick: u32, shift: f32) -> Quantised {
        let players = [[shift, -shift]; PLAYER_COUNT];
        Quantised::from_metres(tick, [shift, 0.0, 0.0], &players)
    }

    #[test]
    fn a_restart_keyframe_is_tagged_and_decodes() {
        let q = sample(5, 1.0);
        let frame = TickFrame::keyframe(&q, true);
        assert_eq!(frame.kind(), KIND_RESTART);
        assert!(frame.is_restart());
        assert_eq!(frame.as_bytes().len(), 99);
        assert_eq!(frame.decode(None).unwrap(), q);
    }

    #[test]
    fn a_delta_frame_is_forty_eight_bytes_and_needs_its_predecessor() {
        let prev = sample(5, 0.0);
        let cur = sample(6, 0.5);
        let frame = TickFrame::delta(&cur, &prev).unwrap();
        assert_eq!(frame.as_bytes().len(), 48);
        assert_eq!(frame.decode(Some(&prev)).unwrap(), cur);
        assert!(matches!(
            frame.decode(None),
            Err(ProtocolError::DeltaWithoutKeyframe)
        ));
    }

    #[test]
    fn bytes_round_trip_and_a_bad_length_is_refused() {
        let frame = TickFrame::keyframe(&sample(1, 0.0), false);
        assert_eq!(TickFrame::from_bytes(frame.as_bytes()).unwrap(), frame);
        let err = TickFrame::from_bytes(&[KIND_DELTA, 0, 0]).unwrap_err();
        assert!(err.to_string().contains("expected 48"), "{err}");
        assert!(matches!(
            TickFrame::from_bytes(&[0x09; 99]),
            Err(ProtocolError::UnknownFrameKind(0x09))
        ));
    }
}
