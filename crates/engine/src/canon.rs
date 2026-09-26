//! Canonical bytes of the match state, shared by the snapshot writer and the replay-gate
//! state hasher. Every number is little-endian, every float is its exact bits, every
//! collection is written by its caller with its length before its items, and every enum is a
//! fixed numeric tag. No number is ever written as text.
//!
//! A float that is NaN or infinite is still written, and the writer keeps the field path of
//! the first such value. The snapshot ignores it; the gate fails the match with it.

use crate::ai::Manager;
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
}

impl Writer {
    pub(crate) fn into_bytes(self) -> Vec<u8> {
        self.buf
    }

    pub(crate) fn raw(&mut self, v: &[u8]) {
        self.buf.extend_from_slice(v);
    }
    pub(crate) fn u8(&mut self, v: u8) {
        self.buf.push(v);
    }
    pub(crate) fn u32(&mut self, v: u32) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }
    pub(crate) fn u64(&mut self, v: u64) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }
    pub(crate) fn u128(&mut self, v: u128) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    /// The exact bits of `v`. When `v` is NaN or infinite and no earlier float was, the
    /// writer keeps `path()` as the fault.
    pub(crate) fn f64_at(&mut self, path: impl FnOnce() -> String, v: f64) {
        if !v.is_finite() && self.fault.is_none() {
            self.fault = Some(path());
        }
        self.buf.extend_from_slice(&v.to_bits().to_le_bytes());
    }
    pub(crate) fn v2_at(&mut self, path: &str, v: DVec2) {
        self.f64_at(|| format!("{path}.x"), v.x);
        self.f64_at(|| format!("{path}.y"), v.y);
    }
    pub(crate) fn v3_at(&mut self, path: &str, v: DVec3) {
        self.f64_at(|| format!("{path}.x"), v.x);
        self.f64_at(|| format!("{path}.y"), v.y);
        self.f64_at(|| format!("{path}.z"), v.z);
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

    /// The 14 derived values, each under `<path>.<name>`.
    pub(crate) fn derived(&mut self, path: &str, d: &Derived) {
        for (name, v) in [
            ("max_speed", d.max_speed),
            ("max_accel", d.max_accel),
            ("passing", d.passing),
            ("dribbling", d.dribbling),
            ("tackling", d.tackling),
            ("positioning", d.positioning),
            ("aggression", d.aggression),
            ("finishing", d.finishing),
            ("vision", d.vision),
            ("decisions", d.decisions),
            ("composure", d.composure),
            ("stamina", d.stamina),
            ("natural_fitness", d.natural_fitness),
            ("injury_resistance", d.injury_resistance),
        ] {
            self.f64_at(|| format!("{path}.{name}"), v);
        }
    }

    /// A team's shape as the snapshot stores it: the attack direction, each slot's activity
    /// and place, the tactics, the lineup, and the bench.
    pub(crate) fn team(&mut self, path: &str, team: &Team) {
        self.f64_at(|| format!("{path}.attack_x"), team.attack_x);
        for (slot, (active, (x, y))) in team.active.iter().zip(team.formation.iter()).enumerate() {
            self.u8(u8::from(*active));
            self.f64_at(|| format!("{path}.formation[{slot}].x"), *x);
            self.f64_at(|| format!("{path}.formation[{slot}].y"), *y);
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
