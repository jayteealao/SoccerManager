//! Canonical bytes of the match state, shared by the snapshot writer and the replay-gate
//! state hasher. Every number is little-endian, every float is its exact bits, every
//! collection is written by its caller with its length before its items, and every enum is a
//! fixed numeric tag. No number is ever written as text.
//!
//! A float that is NaN or infinite is still written, and the writer keeps the field path of
//! the first such value. The snapshot ignores it; the gate fails the match with it.
//!
//! A writer can also keep field marks: the name, kind, and byte offset of each named part of
//! the bytes. A mark writes no byte, so the bytes are the same with marks on or off. Only
//! the per-tick state writer of the gate switches them on.

use crate::ai::Manager;
use crate::gate::FieldKind;
use crate::math::{DVec2, DVec3};
use crate::player::{Derived, Status};
use crate::sim::{DecidedBy, Summary};
use crate::tactics::Tactics;
use crate::team::Team;

/// An absent roster or team index.
pub(crate) const NONE: u8 = u8::MAX;

/// A growing buffer of canonical bytes.
#[derive(Default)]
pub(crate) struct Writer {
    buf: Vec<u8>,
    /// The field path of the first NaN or infinite float written.
    fault: Option<String>,
    /// The field marks, when they are on: each name, kind, and the byte offset it starts at.
    fields: Option<Vec<(String, FieldKind, usize)>>,
}

impl Writer {
    /// A writer that keeps a mark for each named part of its bytes.
    pub(crate) fn with_fields() -> Self {
        Self {
            fields: Some(Vec::new()),
            ..Self::default()
        }
    }

    /// Starts the field `name()` of `kind` at the current offset. Writes no byte; when the
    /// marks are off, it does nothing and the name is never built.
    pub(crate) fn mark(&mut self, kind: FieldKind, name: impl FnOnce() -> String) {
        if let Some(fields) = &mut self.fields {
            fields.push((name(), kind, self.buf.len()));
        }
    }

    /// The field marks in byte order, when they are on.
    pub(crate) fn marks(&self) -> Option<&[(String, FieldKind, usize)]> {
        self.fields.as_deref()
    }

    /// The bytes written so far.
    pub(crate) fn bytes(&self) -> &[u8] {
        &self.buf
    }

    pub(crate) fn into_bytes(self) -> Vec<u8> {
        self.buf
    }

    /// Empties the buffer and the field marks and keeps their allocations. The fault stays.
    pub(crate) fn clear(&mut self) {
        self.buf.clear();
        if let Some(fields) = &mut self.fields {
            fields.clear();
        }
    }

    /// The field path of the first NaN or infinite float written, if any.
    pub(crate) fn fault(&self) -> Option<&str> {
        self.fault.as_deref()
    }

    pub(crate) fn raw(&mut self, v: &[u8]) {
        self.buf.extend_from_slice(v);
    }
    pub(crate) fn u8(&mut self, v: u8) {
        self.buf.push(v);
    }
    pub(crate) fn u16(&mut self, v: u16) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }
    pub(crate) fn u32(&mut self, v: u32) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }
    pub(crate) fn u64(&mut self, v: u64) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }
    pub(crate) fn bool(&mut self, v: bool) {
        self.u8(u8::from(v));
    }
    /// A collection length. Match-state collections are far below 4 billion items.
    pub(crate) fn count(&mut self, n: usize) {
        self.u32(n as u32);
    }
    /// Length-prefixed UTF-8 bytes.
    pub(crate) fn text(&mut self, s: &str) {
        self.count(s.len());
        self.raw(s.as_bytes());
    }

    /// The exact bits of `v`. When `v` is NaN or infinite and no earlier float was, the
    /// writer keeps `path()` as the fault.
    pub(crate) fn f64_at(&mut self, path: impl FnOnce() -> String, v: f64) {
        if !v.is_finite() && self.fault.is_none() {
            self.fault = Some(path());
        }
        self.buf.extend_from_slice(&v.to_bits().to_le_bytes());
    }
    /// A vector under `path()`; the path is built only when a component is not finite.
    pub(crate) fn v2_at(&mut self, path: &dyn Fn() -> String, v: DVec2) {
        self.f64_at(|| format!("{}.x", path()), v.x);
        self.f64_at(|| format!("{}.y", path()), v.y);
    }
    pub(crate) fn v3_at(&mut self, path: &dyn Fn() -> String, v: DVec3) {
        self.f64_at(|| format!("{}.x", path()), v.x);
        self.f64_at(|| format!("{}.y", path()), v.y);
        self.f64_at(|| format!("{}.z", path()), v.z);
    }

    pub(crate) fn index(&mut self, v: Option<usize>) {
        // Roster indices are below 22 and team indices below 2.
        self.u8(v.map_or(NONE, |i| i as u8));
    }
    pub(crate) fn pair(&mut self, v: [u32; 2]) {
        self.u32(v[0]);
        self.u32(v[1]);
    }
    pub(crate) fn opt_pair(&mut self, v: Option<[u32; 2]>) {
        self.u8(u8::from(v.is_some()));
        self.pair(v.unwrap_or([0, 0]));
    }
    pub(crate) fn opt_u8(&mut self, v: Option<u8>) {
        self.u8(u8::from(v.is_some()));
        self.u8(v.unwrap_or(0));
    }
    pub(crate) fn opt_u32(&mut self, v: Option<u32>) {
        self.u8(u8::from(v.is_some()));
        self.u32(v.unwrap_or(0));
    }

    pub(crate) fn tactics(&mut self, t: &Tactics) {
        self.u8(t.formation);
        self.u8(t.mentality);
        for level in t.instructions {
            self.u8(level);
        }
        for rd in &t.roles {
            self.u8(rd.role);
            self.u8(rd.duty);
        }
    }

    /// A player's state deltas, one byte per attribute group, and the effective values they
    /// move most, each under `<path()>.<name>`: top speed, acceleration, turning, and the reach
    /// in the air. The rest of the derived values, and the stage values, follow from the
    /// deltas, the squad entry and the team file.
    pub(crate) fn derived(
        &mut self,
        path: &dyn Fn() -> String,
        d: &Derived,
        deltas: &[i8; crate::contract::states::GROUP_COUNT],
    ) {
        for &g in deltas {
            self.u8(g as u8);
        }
        self.f64_at(|| format!("{}.max_speed", path()), d.max_speed);
        self.f64_at(|| format!("{}.max_accel", path()), d.max_accel);
        self.f64_at(|| format!("{}.turn", path()), d.turn);
        self.f64_at(|| format!("{}.reach_m", path()), d.reach_m);
    }

    /// A team's shape as the snapshot stores it: the attack direction, each slot's activity
    /// and place, the tactics, the lineup, and the bench.
    pub(crate) fn team(&mut self, path: &dyn Fn() -> String, team: &Team) {
        self.f64_at(|| format!("{}.attack_x", path()), team.attack_x);
        for (slot, (active, (x, y))) in team.active.iter().zip(team.formation.iter()).enumerate() {
            self.u8(u8::from(*active));
            self.f64_at(|| format!("{}.formation[{slot}].x", path()), *x);
            self.f64_at(|| format!("{}.formation[{slot}].y", path()), *y);
        }
        self.tactics(&team.tactics);
        // Squad indices are below 40, the team file's limit.
        for s in team.lineup {
            self.u8(s as u8);
        }
        self.u8(team.bench.len() as u8);
        for s in &team.bench {
            self.u8(*s as u8);
        }
    }

    /// Every counter of the match summary, in declaration order.
    pub(crate) fn summary(&mut self, s: &Summary) {
        self.u32(s.possession_changes);
        self.f64_at(|| "summary.ball_max_speed".into(), s.ball_max_speed);
        self.u32(s.ball_idle_ticks);
        for pair in [
            s.goals,
            s.fouls,
            s.offsides,
            s.corners,
            s.throw_ins,
            s.goal_kicks,
            s.free_kicks,
            s.penalties,
            s.yellow,
            s.red,
            s.added_s,
        ] {
            self.pair(pair);
        }
        self.u32(s.stoppages);
        self.u32(s.dead_ball_ticks);
        self.u32(s.offside_checks);
        self.pair(s.shots);
        self.pair(s.substitutions);
        self.pair(s.injuries);
        self.u32(s.changes_queued);
        self.u32(s.changes_applied);
        self.u32(s.changes_rejected);
        self.u32(s.ai_decisions);
        self.pair(s.shots_on_target);
        self.f64_at(|| "summary.xg[0]".into(), s.xg[0]);
        self.f64_at(|| "summary.xg[1]".into(), s.xg[1]);
        self.pair(s.passes);
        self.pair(s.passes_completed);
        self.pair(s.clearances);
        self.pair(s.restart_kicks);
        self.u32(s.live_ticks);
        self.pair(s.possession_ticks);
        self.pair(s.extra_added_s);
        self.u8(u8::from(s.extra_time));
        self.opt_pair(s.shootout);
        self.u32(s.shootout_kicks);
        self.u8(s
            .decided_by
            .and_then(|d| DecidedBy::ALL.iter().position(|x| *x == d))
            .map_or(0, |k| k as u8 + 1));
    }
}

pub(crate) fn status_code(s: Status) -> u8 {
    match s {
        Status::OnPitch => 0,
        Status::SentOff => 1,
        Status::Injured => 2,
    }
}

pub(crate) fn manager_code(m: Manager) -> u8 {
    match m {
        Manager::Ai => 0,
        Manager::Human => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_nan_or_an_infinity_names_the_first_field_and_is_still_written() {
        let mut w = Writer::default();
        w.f64_at(|| "a".into(), 1.0);
        assert_eq!(w.fault(), None);
        w.v2_at(&|| "players[3].vel".into(), DVec2::new(0.0, f64::NAN));
        w.f64_at(|| "later".into(), f64::INFINITY);
        assert_eq!(w.fault(), Some("players[3].vel.y"));
        assert_eq!(w.bytes().len(), 32);
        let mut w = Writer::default();
        w.f64_at(|| "ball.pos.z".into(), f64::NEG_INFINITY);
        assert_eq!(w.fault(), Some("ball.pos.z"));
    }

    #[test]
    fn a_mark_writes_no_byte_and_is_kept_only_when_marks_are_on() {
        let mut off = Writer::default();
        let mut on = Writer::with_fields();
        for w in [&mut off, &mut on] {
            w.mark(FieldKind::Bytes, || "tick".into());
            w.u32(7);
            w.mark(FieldKind::Floats, || "ball.pos".into());
            w.f64_at(|| "ball.pos.x".into(), 1.5);
        }
        assert_eq!(off.bytes(), on.bytes());
        assert!(off.marks().is_none());
        let marks = on.marks().expect("marks are on");
        assert_eq!(marks.len(), 2);
        assert_eq!(marks[1], ("ball.pos".to_string(), FieldKind::Floats, 4));
        on.clear();
        assert_eq!(on.marks().map(<[_]>::len), Some(0));
    }

    #[test]
    fn a_float_is_its_exact_little_endian_bits() {
        let mut w = Writer::default();
        w.f64_at(|| unreachable!(), -0.0);
        assert_eq!(w.bytes(), (-0.0f64).to_bits().to_le_bytes());
    }
}
