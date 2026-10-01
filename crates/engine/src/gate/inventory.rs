//! State inventory version 1: every field of a running match that can affect later play,
//! in the order the gate hashes it. The header goes in once; groups G1 to G11 go in after
//! every tick.
//!
//! The encoder destructures `Simulation`, `Player`, and `Referee` with no `..`, so a new
//! field is a compile error until it is given a disposition here: hashed in a group, or
//! excluded with the reason beside it.
//!
//! A mark names each part of the bytes for the per-tick state writer (`w.mark`); a mark
//! writes no byte, so the hashed bytes do not depend on it.

use crate::canon::{self, Writer};
use crate::player::Player;
use crate::rules::{Phase, Referee};
use crate::sim::{EngineEvent, EventDetail, Simulation};
use crate::tactics::change::{Change, RejectReason};

use super::{FieldKind, Fixture, GATE_SCHEMA, INVENTORY_VERSION};

/// The bytes that open a fixture's running hash: the versions, the fixture, and the match
/// identity.
pub(crate) fn header(w: &mut Writer, fixture: &Fixture, sim: &Simulation) {
    w.u16(INVENTORY_VERSION);
    w.u16(GATE_SCHEMA);
    w.text(&fixture.id);
    let config = &sim.config;
    w.u64(config.seed);
    w.u32(config.minutes);
    w.bool(config.knockout);
    w.text(&config.content_hash);
    for digest in &config.team_digests {
        w.raw(digest);
    }
    w.text(sim.plugins.pack.as_deref().unwrap_or(""));
}

/// The state after one tick, groups G1 to G11. `events` are the events this tick recorded,
/// drained from the match.
pub(crate) fn state(w: &mut Writer, sim: &Simulation, events: &[EngineEvent]) {
    let Simulation {
        // Constant for the match; the header holds its identity.
        config: _,
        teams,
        players,
        ball,
        streams,
        tick,
        carrier,
        control_since,
        last_touch,
        last_kicker,
        summary,
        referee,
        restart,
        // Drained by the runner every tick and hashed as G11 from `events`.
        events: _,
        stoppage,
        timeline,
        managers,
        queue,
        // A record of the verdicts G11 already hashes as events; not state.
        applied: _,
        ledgers,
        ai,
        keeper_beaten,
        pass_in_flight,
        shot_in_flight,
        shot_on_target,
        shot_quality,
        blockers_tried,
        restart_taker,
        clearers_tried,
        // Test seams under the `scenario` feature; no release build has them.
        #[cfg(feature = "scenario")]
            census: _,
        #[cfg(feature = "scenario")]
            forced_kicks: _,
        #[cfg(feature = "scenario")]
            phase_log: _,
        plugins,
        script_cache,
        // End-of-match idempotency only; the phase in G6 says the match is over.
        finished: _,
        // Rewritten before every use.
        scratch: _,
    } = sim;

    use FieldKind::{Bytes, Floats};
    // G1 tick.
    w.mark(Bytes, || "tick".into());
    w.u32(*tick);
    w.mark(Bytes, || "restart".into());
    w.bool(*restart);

    // G2 ball.
    w.mark(Floats, || "ball.pos".into());
    w.v3_at(&|| "ball.pos".into(), ball.pos);
    w.mark(Floats, || "ball.vel".into());
    w.v3_at(&|| "ball.vel".into(), ball.vel);

    // G3 players, roster order.
    for (i, p) in players.iter().enumerate() {
        player(w, i, p);
    }

    // G4 possession and in-play.
    w.mark(Bytes, || "carrier".into());
    w.index(*carrier);
    w.mark(Bytes, || "control_since".into());
    w.u32(*control_since);
    w.mark(Bytes, || "last_touch".into());
    w.index(*last_touch);
    w.mark(Bytes, || "last_kicker".into());
    w.index(*last_kicker);
    w.mark(Bytes, || "keeper_beaten".into());
    w.bool(*keeper_beaten);
    w.mark(Bytes, || "pass_in_flight".into());
    w.index(*pass_in_flight);
    w.mark(Bytes, || "shot_in_flight".into());
    w.index(*shot_in_flight);
    w.mark(Bytes, || "shot_on_target".into());
    w.bool(*shot_on_target);
    w.mark(Floats, || "shot_quality".into());
    w.f64_at(|| "shot_quality".into(), *shot_quality);
    w.mark(Bytes, || "blockers_tried".into());
    w.u32(*blockers_tried);
    w.mark(Bytes, || "clearers_tried".into());
    w.u32(*clearers_tried);
    w.mark(Bytes, || "restart_taker".into());
    w.index(*restart_taker);

    // G5 score and discipline.
    w.mark(Bytes, || "summary".into());
    w.summary(summary);
    let Referee {
        phase,
        // Derived state: it equals `phases::derive(phase, shootout)` at every tick boundary,
        // and both are hashed.
        named: _,
        offside,
        pending,
        tally,
        clock,
        abandoned,
        extra_kick_off,
        shootout,
    } = referee;
    w.mark(Bytes, || "referee.tally".into());
    let crate::rules::clock::Tally {
        kinds,
        cards,
        // Not hashed while no event counts a review; hashing it is a result change.
        reviews,
    } = tally;
    debug_assert_eq!(
        *reviews, 0,
        "a video review was counted; hash it with its own result change"
    );
    for count in kinds {
        w.u32(*count);
    }
    w.u32(*cards);
    w.mark(Bytes, || "referee.pending".into());
    w.count(pending.len());
    for card in pending {
        w.index(Some(card.player));
        w.u8(card.card as u8);
    }

    // G6 rules, restart, clock.
    w.mark(Bytes, || "referee.phase".into());
    match phase {
        Phase::Live => w.u8(0),
        Phase::DeadBall(d) => {
            w.u8(1);
            w.u8(d.kind.index() as u8);
            w.index(Some(d.team));
            w.v2_at(&|| "referee.phase.spot".into(), d.spot);
            w.bool(d.direct);
            w.u32(d.since);
            w.u32(d.ready_at);
            w.index(Some(d.taker));
        }
        Phase::FullTime => w.u8(2),
    }
    w.mark(Bytes, || "referee.offside".into());
    w.u32(*offside);
    w.mark(Bytes, || "referee.clock".into());
    w.u32(clock.halves);
    w.u32(clock.half);
    w.u32(clock.half_start);
    w.u32(clock.half_ticks);
    w.u32(clock.extra_periods);
    w.u32(clock.extra_ticks);
    w.bool(clock.plays_added);
    for added in clock.added_ticks {
        w.opt_u32(added);
    }
    w.mark(Bytes, || "referee.abandoned".into());
    w.bool(*abandoned);
    w.mark(Bytes, || "referee.extra_kick_off".into());
    w.index(*extra_kick_off);
    w.mark(Bytes, || "referee.shootout".into());
    match shootout {
        None => w.u8(0),
        Some(s) => {
            w.u8(1);
            for order in &s.order {
                w.count(order.len());
                for &i in order {
                    w.index(Some(i));
                }
            }
            for keeper in s.keepers {
                w.index(Some(keeper));
            }
            for cursor in s.cursor {
                w.u64(cursor as u64);
            }
            w.index(Some(s.first));
            w.f64_at(|| "referee.shootout.end".into(), s.end);
            w.pair(s.scores);
            w.pair(s.taken);
            w.index(s.kicker);
            w.opt_u32(s.live_since);
        }
    }
    w.mark(Bytes, || "stoppage".into());
    match stoppage {
        None => w.u8(0),
        Some(s) => {
            w.u8(1);
            w.u32(s.tick);
            w.u8(s.kind.index() as u8);
            w.index(s.team);
            w.v2_at(&|| "stoppage.spot".into(), s.spot);
        }
    }

    // G7 teams and managers.
    for (t, team) in teams.iter().enumerate() {
        w.mark(Bytes, || format!("teams[{t}]"));
        w.team(&|| format!("teams[{t}]"), team);
    }
    w.mark(Bytes, || "timeline".into());
    w.count(timeline.len());
    for (from, shapes) in timeline {
        w.u32(*from);
        for (t, team) in shapes.iter().enumerate() {
            w.team(&|| format!("timeline@{from}.teams[{t}]"), team);
        }
    }
    w.mark(Bytes, || "managers".into());
    for m in managers {
        w.u8(canon::manager_code(*m));
    }
    w.mark(Bytes, || "ai".into());
    for state in ai {
        w.opt_pair(state.trailing_acted);
        w.opt_pair(state.leading_acted);
        w.bool(state.due);
    }
    w.mark(Bytes, || "ledgers".into());
    for ledger in ledgers {
        w.u8(ledger.used);
        w.u8(ledger.windows);
        w.opt_u32(ledger.window_at);
    }

    // G8 pending changes.
    w.mark(Bytes, || "queue.pending".into());
    w.count(queue.pending.len());
    for q in &queue.pending {
        w.index(Some(q.team));
        w.u32(q.id.tick);
        w.u32(q.id.n);
        change(w, &q.change);
    }
    w.mark(Bytes, || "queue.next".into());
    w.u32(queue.next);
    w.mark(Bytes, || "queue.admitted".into());
    for at in queue.admitted {
        w.opt_u32(at);
    }

    // G9 plugin state. The boxed hooks show through their presence and through play; the
    // pack identity is in the header; the text of `details` is a log signal and is excluded.
    // The watchdog mark (`slow_calls`) depends on the machine's speed and is excluded, so a
    // slow machine hashes the same match.
    w.mark(Bytes, || "plugins.hooks".into());
    for present in plugins.hooks_present() {
        w.bool(present);
    }
    w.mark(Bytes, || "plugins.failures".into());
    for failures in plugins.failures() {
        w.u32(failures);
    }
    w.mark(Bytes, || "plugins.stats".into());
    w.u32(plugins.stats.calls);
    w.u32(plugins.stats.aborts);
    w.u32(plugins.stats.denials);
    w.u32(plugins.stats.disabled);
    w.mark(Bytes, || "plugins.refresh_ticks".into());
    w.u32(plugins.refresh_ticks);
    w.mark(Bytes, || "script_cache".into());
    match script_cache {
        None => w.u8(0),
        Some(c) => {
            w.u8(1);
            w.index(Some(c.carrier));
            w.u32(c.until);
            for (name, v) in [
                ("pass", c.offsets.pass),
                ("dribble", c.offsets.dribble),
                ("shoot", c.offsets.shoot),
                ("clear", c.offsets.clear),
                ("hold", c.offsets.hold),
            ] {
                w.f64_at(|| format!("script_cache.offsets.{name}"), v);
            }
        }
    }

    // G10 streams.
    // The registry's draw counter and its scripted-draw queues are not match state: the
    // stream positions already hold every draw taken.
    w.mark(FieldKind::Streams, || "streams".into());
    streams.write_state(w);

    // G11 match events of this tick. A `script` event is excluded: its note points at log
    // text that holds wall-clock outcomes under the real clock; G9 counts the failure.
    let kept: Vec<&EngineEvent> = events
        .iter()
        .filter(|e| !matches!(e.detail, Some(EventDetail::Script(_))))
        .collect();
    w.mark(Bytes, || "events".into());
    w.count(kept.len());
    for e in kept {
        event(w, e);
    }
}

fn player(w: &mut Writer, i: usize, p: &Player) {
    let Player {
        // Fixed by the roster index.
        id: _,
        team: _,
        slot,
        squad,
        shirt,
        // Functions of the squad index and the team file, whose digest is in the header.
        attributes: _,
        base: _,
        derived,
        energy,
        pos,
        vel,
        target,
        facing,
        status,
        yellow,
        foul_ready,
    } = p;
    use FieldKind::{Bytes, Floats};
    w.mark(Floats, || format!("players[{i}].pos"));
    w.v2_at(&|| format!("players[{i}].pos"), *pos);
    w.mark(Floats, || format!("players[{i}].vel"));
    w.v2_at(&|| format!("players[{i}].vel"), *vel);
    w.mark(Floats, || format!("players[{i}].target"));
    w.v2_at(&|| format!("players[{i}].target"), *target);
    w.mark(Floats, || format!("players[{i}].facing"));
    w.v2_at(&|| format!("players[{i}].facing"), *facing);
    w.mark(Bytes, || format!("players[{i}].status"));
    w.u8(canon::status_code(*status));
    w.mark(Bytes, || format!("players[{i}].yellow"));
    w.u8(*yellow);
    w.mark(Bytes, || format!("players[{i}].foul_ready"));
    w.u32(*foul_ready);
    w.mark(Bytes, || format!("players[{i}].slot"));
    w.u8(*slot as u8);
    w.mark(Bytes, || format!("players[{i}].squad"));
    w.u8(*squad as u8);
    w.mark(Bytes, || format!("players[{i}].shirt"));
    w.u8(*shirt);
    w.mark(Floats, || format!("players[{i}].energy"));
    w.f64_at(|| format!("players[{i}].energy"), *energy);
    w.mark(Floats, || format!("players[{i}].derived"));
    w.derived(&|| format!("players[{i}].derived"), derived);
}

fn change(w: &mut Writer, change: &Change) {
    match change {
        Change::Substitution { off, on } => {
            w.u8(0);
            w.u8(*off as u8);
            w.u8(*on as u8);
        }
        Change::Tactics(patch) => {
            w.u8(1);
            w.opt_u8(patch.formation);
            w.opt_u8(patch.mentality);
            for level in patch.instructions {
                w.opt_u8(level);
            }
            w.count(patch.roles.len());
            for (squad, rd) in &patch.roles {
                w.u8(*squad as u8);
                w.u8(rd.role);
                w.u8(rd.duty);
            }
        }
    }
}

/// An optional flag as one byte: 0 absent, 1 false, 2 true.
fn opt_bool(w: &mut Writer, v: Option<bool>) {
    w.u8(v.map_or(0, |b| 1 + u8::from(b)));
}

fn event(w: &mut Writer, e: &EngineEvent) {
    let EngineEvent {
        tick,
        kind,
        team,
        scores,
        minute,
        minute_added,
        player,
        secondary,
        card,
        advantage,
        added_time_s,
        spot,
        detail,
        period,
        shootout_round,
        shootout_scored,
        shootout_scores,
        decided_by,
    } = e;
    w.u8(*kind as u8);
    w.u32(*tick);
    w.index(*team);
    w.pair(*scores);
    w.u32(*minute);
    w.opt_u32(*minute_added);
    w.index(*player);
    w.index(*secondary);
    w.u8(card.map_or(0, |c| c as u8 + 1));
    opt_bool(w, *advantage);
    w.opt_u32(*added_time_s);
    match spot {
        None => w.u8(0),
        Some(s) => {
            w.u8(1);
            w.v2_at(&|| "event.spot".into(), *s);
        }
    }
    match detail {
        None => w.u8(0),
        Some(EventDetail::Change { id, kind, reason }) => {
            w.u8(1);
            w.u32(id.tick);
            w.u32(id.n);
            w.u8(*kind as u8);
            reject_reason(w, reason);
        }
        Some(EventDetail::Substitution { off, on }) => {
            w.u8(2);
            w.u8(*off as u8);
            w.u8(*on as u8);
        }
        Some(EventDetail::Injury { source }) => {
            w.u8(3);
            w.u8(*source as u8);
        }
        Some(EventDetail::Ai { code }) => {
            w.u8(4);
            w.u8(*code as u8);
        }
        // Filtered out by the caller.
        Some(EventDetail::Script(_)) => w.u8(5),
    }
    w.opt_u32(*period);
    w.opt_u32(*shootout_round);
    opt_bool(w, *shootout_scored);
    w.opt_pair(*shootout_scores);
    w.u8(decided_by.map_or(0, |d| d as u8 + 1));
}

fn reject_reason(w: &mut Writer, reason: &Option<RejectReason>) {
    match reason {
        None => w.u8(0),
        Some(RejectReason::LimitReached { limit }) => {
            w.u8(1);
            w.u8(*limit);
        }
        Some(RejectReason::NoWindowLeft { windows }) => {
            w.u8(2);
            w.u8(*windows);
        }
        Some(RejectReason::NotOnPitch { squad }) => {
            w.u8(3);
            w.u8(*squad as u8);
        }
        Some(RejectReason::SentOff { squad }) => {
            w.u8(4);
            w.u8(*squad as u8);
        }
        Some(RejectReason::NotOnBench { squad }) => {
            w.u8(5);
            w.u8(*squad as u8);
        }
        Some(RejectReason::LeftThePitch { squad }) => {
            w.u8(6);
            w.u8(*squad as u8);
        }
        Some(RejectReason::OutOfRange) => w.u8(7),
    }
}
