//! The running rule checker reports exactly what the old validator reports: on a fixed set
//! with one planted violation for every tick rule and every event rule, and on real
//! matches played through the checker as a tick sink. Each planted case is first checked
//! against the old validator (it must report the plant), then the running checker must
//! give the same report and contain the plant. The running checker keeps only the first
//! `KEPT_PER_RULE` violations of each rule, so it is compared with the old report cut the
//! same way, and its count of each rule with the old report's.

mod common;

use engine::EngineEventKind::{Card, Foul, FullTime, Goal, HalfTime, KickOff, Substitution as Sub};
use engine::math::DVec2;
use engine::record::{FanoutSink, TickRecord, VecSink};
use engine::team::Team;
use engine::{
    EngineEvent, EngineEventKind, MatchConfig, RunningCheck, Simulation, StreamRules, Tactics,
    Tuning, Validator, Violation,
};

/// Every rule the two checkers judge: seven tick rules, then the twelve event rules.
const RULES: [&str; 19] = [
    "in_bounds",
    "separation",
    "ball_speed",
    "player_speed",
    "restart_spot",
    "anchor_tolerance",
    "ball_rest",
    "event_order",
    "kick_off_first",
    "full_time_last",
    "goal_score",
    "score_kept",
    "period_end_team",
    "player_side",
    "card_kind",
    "sent_off_silent",
    "substitution_squad",
    "substitution_limits",
    "added_time",
];

/// One match's input to both checkers.
struct Input {
    tuning: Tuning,
    timeline: Vec<(u32, [Team; 2])>,
    events: Vec<EngineEvent>,
    records: Vec<TickRecord>,
    rules: StreamRules,
}

/// The old validator's report: the tick rules, then the event rules.
fn old(input: &Input) -> Vec<Violation> {
    let mut out = Validator::for_match(input.tuning.clone(), &input.timeline, &input.events)
        .check(&input.records);
    out.extend(Validator::check_events(&input.events, &input.rules));
    out
}

/// A report as the running checker keeps it: the first `KEPT_PER_RULE` violations of each
/// rule in report order, and each rule's total.
type Kept = (Vec<Violation>, Vec<(&'static str, usize)>);

/// The old report cut to what the running checker keeps.
fn capped(old: &[Violation]) -> Kept {
    let mut seen = [0usize; RunningCheck::RULES.len()];
    let place = |rule| RunningCheck::RULES.iter().position(|r| *r == rule).unwrap();
    let kept = old
        .iter()
        .filter(|v| {
            let n = &mut seen[place(v.rule)];
            *n += 1;
            *n <= RunningCheck::KEPT_PER_RULE
        })
        .cloned()
        .collect();
    let totals = RunningCheck::RULES
        .map(|rule| (rule, old.iter().filter(|v| v.rule == rule).count()))
        .to_vec();
    (kept, totals)
}

/// The running checker fed in tick order: before each record, every team shape and event
/// whose tick is not after the record's, each in its own order; the rest after the last.
/// It returns the kept violations and each rule's total, and checks that its whole count is
/// the old report's.
fn running(input: &Input) -> Kept {
    let (timeline, events) = (&input.timeline, &input.events);
    let mut check = RunningCheck::new(
        input.tuning.clone(),
        timeline[0].1.clone(),
        input.rules.clone(),
    );
    let (mut shape, mut event) = (1, 0);
    for r in &input.records {
        while shape < timeline.len() && timeline[shape].0 <= r.tick {
            check.shapes(timeline[shape].0, timeline[shape].1.clone());
            shape += 1;
        }
        while event < events.len() && events[event].tick <= r.tick {
            check.event(&events[event]);
            event += 1;
        }
        check.tick(r);
    }
    for (tick, teams) in &timeline[shape..] {
        check.shapes(*tick, teams.clone());
    }
    for e in &events[event..] {
        check.event(e);
    }
    assert_eq!(check.ticks() as usize, input.records.len());
    let found = check.finish();
    assert_eq!(found.len(), old(input).len(), "the whole count");
    (found.kept().to_vec(), found.totals().collect())
}

/// The seed-42 10-minute match: its records, recorded beside the running checker, its
/// events and its team shapes.
fn recorded_match() -> Input {
    let config = common::short_match(10);
    let rules = StreamRules::for_config(&config);
    let mut sim = Simulation::new(config.clone()).unwrap();
    let live = RunningCheck::for_match(&sim, rules.clone());
    let mut sink = FanoutSink::new(VecSink::default(), live);
    sim.run(&mut sink).unwrap();
    let (records, live) = sink.into_parts();
    let live = live.finish_match(&sim);
    let input = Input {
        tuning: config.tuning.clone(),
        timeline: sim.team_timeline().to_vec(),
        events: sim.take_events(),
        records: records.records,
        rules,
    };
    let old = old(&input);
    assert_eq!(live.len(), old.len());
    assert_eq!(
        (live.kept().to_vec(), live.totals().collect()),
        capped(&old),
        "the live face equals the old report"
    );
    input
}

/// One planted case: a name, the input, and the violation the plant must cause.
struct Plant {
    name: &'static str,
    input: Input,
    rule: &'static str,
    tick: u32,
    /// The event index for an event rule; `None` for a tick rule.
    index: Option<usize>,
    player: Option<usize>,
}

/// The tick-rule plants, each on the records of the recorded 10-minute match.
fn tick_plants() -> Vec<Plant> {
    let base = recorded_match();
    assert!(old(&base).is_empty(), "the base match is clean");
    let pitch = *base.timeline[0].1[0].pitch();
    let plant = |name, rule, k: usize, player, change: &dyn Fn(&mut Vec<TickRecord>)| {
        let mut records = base.records.clone();
        change(&mut records);
        Plant {
            name,
            input: Input {
                tuning: base.tuning.clone(),
                timeline: base.timeline.clone(),
                events: base.events.clone(),
                records,
                rules: base.rules.clone(),
            },
            rule,
            tick: base.records[k].tick,
            index: None,
            player,
        }
    };
    let mut plants = Vec::new();

    // An unparked player 5 m past the touchline on one tick.
    let k = 7000;
    plants.push(plant(
        "player off the pitch",
        "in_bounds",
        k,
        Some(5),
        &|r| r[k].players[5][1] = (pitch.half_width() + 5.0) as f32,
    ));

    // Player 1 set onto player 0.
    let k = 10;
    plants.push(plant(
        "two players on one spot",
        "separation",
        k,
        Some(0),
        &|r| r[k].players[1] = r[k].players[0],
    ));

    // The ball moved 3 m on one tick that is not a restart.
    let k = 5000;
    assert!(!base.records[k].restart && !base.records[k + 1].restart);
    plants.push(plant("ball too fast", "ball_speed", k, None, &|r| {
        r[k].ball[0] += 3.0
    }));

    // Player 3 moved 5 m on one tick inside a half: a jump no top speed allows.
    let k = 3000;
    plants.push(plant(
        "player jumps 5 m",
        "player_speed",
        k,
        Some(3),
        &|r| r[k].players[3][0] += 5.0,
    ));

    // The ball 1 m off the spot on a restart tick that names a spot.
    let k = base
        .records
        .iter()
        .position(|r| {
            r.restart
                && r.tick > 1
                && base
                    .events
                    .iter()
                    .any(|e| e.tick == r.tick && e.spot.is_some())
        })
        .expect("a restart with a named spot");
    plants.push(plant(
        "ball off the restart spot",
        "restart_spot",
        k,
        None,
        &|r| r[k].ball[0] += 1.0,
    ));

    // One outfield player held at the pitch corner farthest from the ball and its anchors
    // while the ball lies still, for the grace period and ten ticks more. No event falls in the
    // stretch: a stoppage that opens there moves the point the anchors read from the ball to
    // the restart spot, which moves the anchors under him (seed 42's first such stretch, after
    // the style presets, is a throw-in's fetch).
    let grace = base.tuning.anchor_grace_ticks as usize;
    let still = (1..base.records.len() - grace - 10)
        .find(|&s| {
            let r = &base.records;
            let window = &r[s..s + grace + 10];
            let outside = |t: u32| t <= r[s - 1].tick || t > window.last().unwrap().tick;
            window.iter().all(|x| x.ball == r[s].ball && !x.restart)
                && base.timeline.iter().all(|(t, _)| outside(*t))
                && base.events.iter().all(|e| outside(e.tick))
        })
        .expect("a stretch with the ball still");
    let (player, team, slot) = (16, 1, 5);
    let teams = &base
        .timeline
        .iter()
        .rev()
        .find(|(t, _)| *t <= base.records[still].tick)
        .unwrap()
        .1;
    let ball = base.records[still].ball;
    let ball = DVec2::new(f64::from(ball[0]), f64::from(ball[1]));
    let anchors = [true, false].map(|has| teams[team].anchor(slot, ball, has, &base.tuning));
    let (hx, hy) = (pitch.half_length() - 1.0, pitch.half_width() - 1.0);
    let corner = [(hx, hy), (hx, -hy), (-hx, hy), (-hx, -hy)]
        .map(|(x, y)| DVec2::new(x, y))
        .into_iter()
        .max_by(|a, b| {
            let far = |p: &DVec2| {
                anchors
                    .iter()
                    .fold((*p - ball).length(), |d, a| d.min((*p - *a).length()))
            };
            far(a).total_cmp(&far(b))
        })
        .unwrap();
    plants.push(plant(
        "player drifting from the anchor",
        "anchor_tolerance",
        still + grace - 1,
        Some(player),
        &|r| {
            for x in &mut r[still..still + grace + 10] {
                x.players[player] = [corner.x as f32, corner.y as f32];
            }
        },
    ));

    // The ball held still in open play for the rest limit and ten records more: from a record
    // after a restart was taken (the ball placed, then moving), with no stoppage and no restart
    // in the window. The rule fires on the record that reaches the limit.
    let limit = (engine::validate::MAX_BALL_REST_S / base.tuning.dt).round() as usize;
    let stops: Vec<u32> = base
        .events
        .iter()
        .filter(|e| e.spot.is_some())
        .map(|e| e.tick)
        .collect();
    let r = &base.records;
    // Record k is in play: the last stoppage at or before it was placed by a restart record
    // and the ball has moved since on a record that is not one.
    let mut taken = vec![false; r.len()];
    let (mut dead, mut placed, mut next) = (false, false, 0);
    for k in 0..r.len() {
        while next < stops.len() && stops[next] <= r[k].tick {
            (dead, placed) = (true, false);
            next += 1;
        }
        if dead && r[k].restart {
            placed = true;
        } else if dead && placed && k > 0 && r[k].ball != r[k - 1].ball {
            dead = false;
        }
        taken[k] = !dead;
    }
    let s = (1..r.len() - limit - 20)
        .find(|&s| {
            let window = &r[s..=s + limit + 10];
            taken[s]
                && r[s].ball != r[s - 1].ball
                && window.iter().all(|x| !x.restart)
                && !stops
                    .iter()
                    .any(|&t| t >= r[s].tick && t <= window.last().unwrap().tick)
        })
        .expect("an open-play stretch as long as the rest limit");
    plants.push(plant(
        "ball still in open play",
        "ball_rest",
        s + limit,
        None,
        &|r| {
            let at = r[s].ball;
            for x in &mut r[s..=s + limit + 10] {
                x.ball = at;
            }
        },
    ));
    plants
}

/// `base` with `records` and, when given, its timeline and events: one planted input.
fn planted(
    base: &Input,
    records: Vec<TickRecord>,
    timeline: Option<Vec<(u32, [Team; 2])>>,
    events: Option<Vec<EngineEvent>>,
) -> Input {
    Input {
        tuning: base.tuning.clone(),
        timeline: timeline.unwrap_or_else(|| base.timeline.clone()),
        events: events.unwrap_or_else(|| base.events.clone()),
        records,
        rules: base.rules.clone(),
    }
}

/// `p` as a record position.
fn at(p: DVec2) -> [f32; 2] {
    [p.x as f32, p.y as f32]
}

/// A record position as the checkers read it.
fn read(p: [f32; 2]) -> DVec2 {
    DVec2::new(f64::from(p[0]), f64::from(p[1]))
}

/// Separation plants on the records of `base`, each a pair that comes near or under the
/// minimum distance in a way a cheaper pair check could miss: closing from far at top
/// speed, held just outside the minimum for many records then just inside, a sent-off
/// player coming back from the parking spot onto a team-mate, a substitute entering onto an
/// opponent on his entry tick, and a long jump onto another player.
fn near_miss_plants(base: &Input) -> Vec<Plant> {
    let min = engine::validate::MIN_SEPARATION;
    let pitch = *base.timeline[0].1[0].pitch();
    let tick = |k: usize| base.records[k].tick;
    let plant = |name, k: usize, player, input: Input| Plant {
        name,
        input,
        rule: "separation",
        tick: tick(k),
        index: None,
        player: Some(player),
    };
    let mut plants = Vec::new();

    // Player 2 closes on player 9 from 20 m over 40 records, ending 0.05 m from him: no
    // more than the top speed's step on each record.
    let k = 4000;
    let mut records = base.records.clone();
    for (n, r) in records[k - 40..=k].iter_mut().enumerate() {
        let gap = 0.05 + (20.0 - 0.05) * (40 - n) as f64 / 40.0;
        r.players[2] = at(read(r.players[9]) + DVec2::new(gap, 0.0));
    }
    let d = (read(records[k].players[2]) - read(records[k].players[9])).length();
    assert!(d < min, "the closing pair ends under the minimum: {d}");
    plants.push(plant(
        "pair closing at top speed from far",
        k,
        2,
        planted(base, records, None, None),
    ));

    // Players 4 and 15 held on one spot just outside the minimum for 200 records, then just
    // inside it.
    let (k, hold) = (6000, 200);
    let mut records = base.records.clone();
    let spot = DVec2::new(0.5, 0.5);
    let outside = (min + 1e-6) as f32;
    let inside = (min - 1e-4) as f32;
    for (n, r) in records[k - hold..=k].iter_mut().enumerate() {
        r.players[4] = at(spot);
        let off = if n == hold { inside } else { outside };
        r.players[15] = [spot.x as f32 + off, spot.y as f32];
    }
    let gap = |r: &TickRecord| (read(r.players[4]) - read(r.players[15])).length();
    assert!(gap(&records[k - 1]) >= min && gap(&records[k - 1]) < min + 1e-5);
    assert!(gap(&records[k]) < min);
    plants.push(plant(
        "pair held at the minimum, then under it",
        k,
        4,
        planted(base, records, None, None),
    ));

    // Player 6 parked for 100 records, as a sent-off player is, then on player 7's spot.
    let k = 8000;
    let mut records = base.records.clone();
    for r in &mut records[k - 100..k] {
        r.players[6] = at(pitch.parking_spot(0, 6));
    }
    records[k].players[6] = records[k].players[7];
    plants.push(plant(
        "parked player back onto a team-mate",
        k,
        6,
        planted(base, records, None, None),
    ));

    // An away substitute enters slot 4 (player 15) on an opponent, player 1, on his entry
    // tick: parked before it, as a player off the pitch is.
    let k = 10_000;
    let mut records = base.records.clone();
    for r in &mut records[k - 50..k] {
        r.players[15] = at(pitch.parking_spot(1, 4));
    }
    records[k].players[15] = records[k].players[1];
    let mut events = base.events.clone();
    let mut entry = events
        .iter()
        .find(|e| e.team.is_some())
        .copied()
        .expect("an event with a team");
    entry.kind = Sub;
    entry.tick = tick(k);
    entry.team = Some(1);
    entry.player = Some(15);
    entry.secondary = None;
    entry.spot = None;
    entry.card = None;
    entry.advantage = None;
    entry.added_time_s = None;
    entry.detail = Some(engine::EventDetail::Substitution {
        off: base.rules.lineups[1][4],
        on: 20,
    });
    let place = events.partition_point(|e| e.tick <= tick(k));
    events.insert(place, entry);
    plants.push(plant(
        "substitute enters onto an opponent",
        k,
        1,
        planted(base, records, None, Some(events)),
    ));

    // Player 3 jumps onto the player farthest from him, at least 30 m away.
    let k = 12_000;
    let mut records = base.records.clone();
    let from = read(records[k - 1].players[3]);
    let far = (0..22)
        .filter(|&j| j != 3)
        .max_by(|&a, &b| {
            let d = |j: usize| (read(records[k].players[j]) - from).length();
            d(a).total_cmp(&d(b))
        })
        .unwrap();
    assert!((read(records[k].players[far]) - from).length() >= 30.0);
    records[k].players[3] = records[k].players[far];
    plants.push(plant(
        "30 m jump onto another player",
        k,
        3.min(far),
        planted(base, records, None, None),
    ));
    plants
}

/// The first stretch at or after record `from` in which the ball lies still for the anchor
/// grace and ten records more, with no team shape and no event inside it.
fn still_stretch(base: &Input, from: usize) -> usize {
    let grace = base.tuning.anchor_grace_ticks as usize;
    let r = &base.records;
    (from.max(1)..r.len() - grace - 10)
        .find(|&s| {
            let window = &r[s..s + grace + 10];
            let outside = |t: u32| t <= r[s - 1].tick || t > window.last().unwrap().tick;
            window.iter().all(|x| x.ball == r[s].ball && !x.restart)
                && base.timeline.iter().all(|(t, _)| outside(*t))
                && base.events.iter().all(|e| outside(e.tick))
        })
        .expect("a stretch with the ball still")
}

/// An anchor plant on the first still stretch at or after record `from`: the player in
/// `slot` of `team` held at the pitch corner farthest from the ball and his anchors for the
/// grace period and ten records more. When `change` is given, it is applied to the shape in
/// force and the changed shape comes into force on the record before the stretch, so the
/// drift is judged on it.
fn drift_plant(
    base: &Input,
    name: &'static str,
    from: usize,
    team: usize,
    slot: &dyn Fn(&Team) -> usize,
    change: Option<&dyn Fn(&mut Team)>,
) -> Plant {
    let grace = base.tuning.anchor_grace_ticks as usize;
    let still = still_stretch(base, from);
    let start = base.records[still - 1].tick;
    let mut teams = base
        .timeline
        .iter()
        .rev()
        .find(|(t, _)| *t <= start)
        .unwrap()
        .1
        .clone();
    let mut timeline = base.timeline.clone();
    if let Some(change) = change {
        change(&mut teams[team]);
        let place = timeline.partition_point(|(t, _)| *t <= start);
        timeline.insert(place, (start, teams.clone()));
    }
    let slot = slot(&teams[team]);
    let player = team * 11 + slot;
    let pitch = *teams[team].pitch();
    let ball = read([base.records[still].ball[0], base.records[still].ball[1]]);
    let anchors = [true, false].map(|has| teams[team].anchor(slot, ball, has, &base.tuning));
    let (hx, hy) = (pitch.half_length() - 1.0, pitch.half_width() - 1.0);
    let corner = [(hx, hy), (hx, -hy), (-hx, hy), (-hx, -hy)]
        .map(|(x, y)| DVec2::new(x, y))
        .into_iter()
        .max_by(|a, b| {
            let far = |p: &DVec2| {
                anchors
                    .iter()
                    .fold((*p - ball).length(), |d, a| d.min((*p - *a).length()))
            };
            far(a).total_cmp(&far(b))
        })
        .unwrap();
    let mut records = base.records.clone();
    for x in &mut records[still..still + grace + 10] {
        x.players[player] = at(corner);
    }
    Plant {
        name,
        input: planted(base, records, Some(timeline), None),
        rule: "anchor_tolerance",
        tick: base.records[still + grace - 1].tick,
        index: None,
        player: Some(player),
    }
}

/// Anchor plants on a shape other than the kick-off one: after a tactics change, after a
/// sending-off reshapes a line, in the second half after the ends switch, and on an acting
/// keeper.
fn shape_plants(base: &Input) -> Vec<Plant> {
    let content = common::content();
    let tuning = base.tuning.clone();
    let quarter = base.records.len() / 4;
    let second_half = base
        .events
        .iter()
        .find(|e| e.kind == HalfTime)
        .map(|e| base.records.partition_point(|r| r.tick <= e.tick))
        .expect("a half-time");
    let same = |s: usize| move |_: &Team| s;
    let tactics_change = |team: &mut Team| {
        let mut tactics = Tactics::defaults(&content.tactics);
        tactics.mentality = (content.tactics.mentalities.len() - 1) as u8;
        tactics.instructions[engine::data::tactics::LINE_HEIGHT] = 2;
        team.set_tactics(tactics, &content.tactics, &tuning);
    };
    let sent_off = |team: &mut Team| team.reshape(3);
    let keeper_off = |team: &mut Team| team.reshape(0);
    vec![
        drift_plant(
            base,
            "drift after a tactics change",
            quarter,
            0,
            &same(7),
            Some(&tactics_change),
        ),
        drift_plant(
            base,
            "drift after a sending-off reshapes the line",
            quarter * 3 / 2,
            1,
            &same(2),
            Some(&sent_off),
        ),
        drift_plant(
            base,
            "drift in the second half",
            second_half,
            0,
            &same(9),
            None,
        ),
        drift_plant(
            base,
            "drift of an acting keeper",
            quarter * 3,
            1,
            &|team: &Team| {
                assert_ne!(team.keeper_slot(), 0, "an outfield player keeps goal");
                team.keeper_slot()
            },
            Some(&keeper_off),
        ),
    ]
}

fn event(kind: EngineEventKind, tick: u32, team: Option<usize>, scores: [u32; 2]) -> EngineEvent {
    EngineEvent {
        tick,
        kind,
        team,
        scores,
        minute: tick / 3000,
        minute_added: None,
        player: None,
        secondary: None,
        card: None,
        advantage: None,
        added_time_s: None,
        spot: None,
        detail: None,
        period: None,
        shootout_round: None,
        shootout_scored: None,
        shootout_scores: None,
        decided_by: None,
    }
}

/// A legal stream with a goal, a kick-off after it, half-time and full time.
fn good() -> Vec<EngineEvent> {
    vec![
        event(KickOff, 1, Some(0), [0, 0]),
        event(Goal, 100, Some(1), [0, 1]),
        event(KickOff, 101, Some(0), [0, 1]),
        event(HalfTime, 200, None, [0, 1]),
        event(FullTime, 400, None, [0, 1]),
    ]
}

/// A legal stream with a foul with its fouled player, a booking, a substitution after it,
/// and added time on both period ends.
fn player_stream() -> Vec<EngineEvent> {
    let mut kick_off = event(KickOff, 0, Some(0), [0, 0]);
    kick_off.player = Some(10);
    let mut foul = event(Foul, 10, Some(0), [0, 0]);
    foul.player = Some(4);
    foul.secondary = Some(15);
    foul.advantage = Some(false);
    let mut card = event(Card, 10, Some(0), [0, 0]);
    card.player = Some(4);
    card.card = Some(engine::Card::Yellow);
    let mut sub = event(Sub, 20, Some(1), [0, 0]);
    sub.player = Some(13);
    sub.detail = Some(engine::EventDetail::Substitution { off: 2, on: 14 });
    let mut half_time = event(HalfTime, 30, None, [0, 0]);
    half_time.added_time_s = Some(75);
    let mut full_time = event(FullTime, 60, None, [0, 0]);
    full_time.added_time_s = Some(120);
    vec![kick_off, foul, card, sub, half_time, full_time]
}

/// The rules of a regulation match on the shipped rule pack: 5 substitutions in 3 windows,
/// half-time free, 60 to 900 added seconds, every squad index in its own slot.
fn planted_rules() -> StreamRules {
    StreamRules {
        substitutions: 5,
        windows: 3,
        half_time_exempt: true,
        added_s: Some((60, 900)),
        lineups: [std::array::from_fn(|s| s), std::array::from_fn(|s| s)],
    }
}

/// A substitution in `player_stream`'s shape: the away side's `slot` off, squad `on` on.
fn sub(tick: u32, slot: usize, on: usize) -> EngineEvent {
    let mut e = player_stream()[3];
    e.tick = tick;
    e.player = Some(11 + slot);
    e.detail = Some(engine::EventDetail::Substitution { off: slot, on });
    e
}

/// The event-rule plants, each on a legal stream with no records.
fn event_plants() -> Vec<Plant> {
    let config = common::short_match(1);
    let teams = Simulation::new(config.clone()).unwrap().teams();
    let rules = planted_rules();
    let plant = |name, rule, events: Vec<EngineEvent>, index: usize| Plant {
        name,
        rule,
        tick: events[index].tick,
        player: events[index].player,
        index: Some(index),
        input: Input {
            tuning: config.tuning.clone(),
            timeline: vec![(0, teams.clone())],
            events,
            records: Vec::new(),
            rules: rules.clone(),
        },
    };
    let mut plants = Vec::new();

    let mut s = good();
    s[2].tick = 50;
    plants.push(plant("event before the one before it", "event_order", s, 2));

    let mut s = good();
    s[0].kind = Goal;
    plants.push(plant("no kick-off first", "kick_off_first", s, 0));

    let mut s = good();
    s.push(event(KickOff, 401, Some(1), [0, 1]));
    plants.push(plant("event after full time", "full_time_last", s, 4));

    let mut s = good();
    for e in &mut s[1..] {
        e.scores = [1, 1];
    }
    plants.push(plant("goal adds two", "goal_score", s, 1));

    let mut s = good();
    s[3].scores = [0, 2];
    s[4].scores = [0, 2];
    plants.push(plant("score moves with no goal", "score_kept", s, 3));

    let mut s = good();
    s[3].team = Some(0);
    plants.push(plant("half-time names a team", "period_end_team", s, 3));

    let mut s = player_stream();
    s[1].player = Some(14);
    plants.push(plant("foul named on the wrong side", "player_side", s, 1));

    let mut s = player_stream();
    s[2].card = Some(engine::Card::SecondYellow);
    plants.push(plant("second yellow with no yellow", "card_kind", s, 2));

    let mut s = player_stream();
    s[2].card = Some(engine::Card::Red);
    let mut again = s[1];
    again.tick = 25;
    s.insert(4, again);
    plants.push(plant(
        "sent-off player fouls again",
        "sent_off_silent",
        s,
        4,
    ));

    let mut s = player_stream();
    s[3].detail = Some(engine::EventDetail::Substitution { off: 5, on: 14 });
    plants.push(plant(
        "substitution takes off a player not in the slot",
        "substitution_squad",
        s,
        3,
    ));

    // Windows at ticks 20, 21 and 22, then a fourth at tick 25.
    let mut s = player_stream();
    s.insert(4, sub(21, 5, 15));
    s.insert(5, sub(22, 6, 16));
    s.insert(6, sub(25, 7, 17));
    plants.push(plant("a fourth window", "substitution_limits", s, 6));

    let mut s = player_stream();
    s[4].added_time_s = Some(30);
    plants.push(plant("added time below the range", "added_time", s, 4));

    plants
}

/// Streams the rules pass: a substitution at the half-time tick is free whether it comes
/// after the half-time event or before it in the stream.
fn half_time_streams() -> Vec<Vec<EngineEvent>> {
    let mut after = player_stream();
    after.insert(4, sub(21, 5, 15));
    after.insert(5, sub(22, 6, 16));
    assert_eq!(after[6].kind, HalfTime);
    after.insert(7, sub(30, 7, 17));
    let mut before = player_stream();
    before.insert(4, sub(21, 5, 15));
    before.insert(5, sub(22, 6, 16));
    before.insert(6, sub(30, 7, 17));
    assert_eq!(before[7].kind, HalfTime);
    vec![after, before]
}

fn check_plant(p: &Plant) {
    let expected = |v: &Violation| {
        v.rule == p.rule
            && v.tick == p.tick
            && v.player == p.player
            && p.index.is_none_or(|i| v.value == i as f64)
    };
    let old = old(&p.input);
    assert!(
        old.iter().any(expected),
        "{}: the old validator does not report the plant (a fixture defect): {old:?}",
        p.name
    );
    let new = running(&p.input);
    assert_eq!(new, capped(&old), "{}: the reports differ", p.name);
    assert!(
        new.0.iter().any(expected),
        "{}: the plant is caught",
        p.name
    );
}

#[test]
fn every_planted_tick_rule_gives_the_same_report() {
    for p in &tick_plants() {
        check_plant(p);
    }
}

/// The near misses and shape changes a cheaper checker could get wrong give the same
/// report, each with its plant caught.
#[test]
fn every_near_miss_and_shape_change_plant_gives_the_same_report() {
    let base = recorded_match();
    assert!(old(&base).is_empty(), "the base match is clean");
    let near = near_miss_plants(&base);
    let shapes = shape_plants(&base);
    assert_eq!((near.len(), shapes.len()), (5, 4));
    for p in near.iter().chain(&shapes) {
        check_plant(p);
    }
}

#[test]
fn every_planted_event_rule_gives_the_same_report() {
    for p in &event_plants() {
        check_plant(p);
    }
    let config = common::short_match(1);
    let teams = Simulation::new(config.clone()).unwrap().teams();
    for events in half_time_streams() {
        let input = Input {
            tuning: config.tuning.clone(),
            timeline: vec![(0, teams.clone())],
            events,
            records: Vec::new(),
            rules: planted_rules(),
        };
        assert!(old(&input).is_empty(), "{:?}", old(&input));
        assert_eq!(running(&input), capped(&old(&input)));
    }
}

#[test]
fn the_fixed_set_plants_every_rule() {
    let mut planted: Vec<&str> = event_plants().iter().map(|p| p.rule).collect();
    planted.extend([
        "in_bounds",
        "separation",
        "ball_speed",
        "player_speed",
        "restart_spot",
        "anchor_tolerance",
        "ball_rest",
    ]);
    planted.sort_unstable();
    let mut rules = RULES.to_vec();
    rules.sort_unstable();
    assert_eq!(planted, rules);
    assert_eq!(RunningCheck::RULES.len(), RULES.len());
    for rule in RunningCheck::RULES {
        assert!(RULES.contains(&rule), "{rule} has no plant");
    }
    // Separation and the anchor rule also carry the near misses and the shape changes.
    let base = recorded_match();
    assert!(
        near_miss_plants(&base)
            .iter()
            .all(|p| p.rule == "separation")
    );
    assert!(
        shape_plants(&base)
            .iter()
            .all(|p| p.rule == "anchor_tolerance")
    );
}

/// Plays `config` through the running checker and a record list, with `prepare` run on
/// the match before kick-off, and returns both reports: the old one whole, the running one
/// as it keeps it. The running checker's whole count must be the old report's.
fn both(config: MatchConfig, prepare: &dyn Fn(&mut Simulation)) -> (Vec<Violation>, Kept) {
    let rules = StreamRules::for_config(&config);
    let mut sim = Simulation::new(config).unwrap();
    prepare(&mut sim);
    let live = RunningCheck::for_match(&sim, rules.clone());
    let mut sink = FanoutSink::new(VecSink::default(), live);
    sim.run(&mut sink).unwrap();
    let (records, live) = sink.into_parts();
    assert_eq!(live.ticks(), sim.tick());
    let new = live.finish_match(&sim);
    let input = Input {
        tuning: sim.tuning().clone(),
        timeline: sim.team_timeline().to_vec(),
        events: sim.take_events(),
        records: records.records,
        rules,
    };
    let old = old(&input);
    assert_eq!(new.len(), old.len(), "the whole count");
    (old, (new.kept().to_vec(), new.totals().collect()))
}

/// The real matches: seeds 1 to 5, 9, 11 and 38 over 90 minutes, a 10-minute match, a
/// knockout with a shoot-out, a match with a player sent off at kick-off and cards
/// otherwise off, and a match of two different formations. `tighten` narrows the anchor
/// rule so the reports are not empty.
fn real_matches(tighten: bool) -> Vec<(String, Vec<Violation>, Kept)> {
    let content = common::content();
    let [a, b] = common::default_teams(&content);
    let formations = content.tactics.formations.len();
    assert!(formations >= 2);
    let tune = |mut config: MatchConfig| {
        if tighten {
            config.tuning.anchor_tolerance = 2.0;
            config.tuning.anchor_ball_distance = 5.0;
            config.tuning.anchor_grace_ticks = 5;
        }
        config
    };
    let seeds = [1u64, 2, 3, 4, 5, 9, 11, 38];
    let cases = 12u64;
    common::run_many(0..=cases - 1, |i| {
        let at = |seed, minutes| MatchConfig::new(seed, minutes, &content, [&a, &b]).unwrap();
        let none: &dyn Fn(&mut Simulation) = &|_| {};
        let (name, (old, new)) = match i as usize {
            i if i < seeds.len() => (
                format!("seed {}", seeds[i]),
                both(tune(at(seeds[i], 90)), none),
            ),
            8 => ("10 minutes seed 1".into(), both(tune(at(1, 10)), none)),
            9 => (
                "knockout seed 1".into(),
                both(tune(at(1, 90).with_knockout()), none),
            ),
            10 => {
                let mut config = tune(at(3, 90));
                config.tuning.red_base = 0.0;
                config.tuning.yellow_base = 0.0;
                config.tuning.yellow_aggression_weight = 0.0;
                (
                    "centre-back sent off at kick-off".into(),
                    both(config, &|sim| sim.send_off_before_kickoff(13)),
                )
            }
            _ => {
                let mut config = at(4, 90);
                for (team, formation) in [0u8, formations as u8 - 1].into_iter().enumerate() {
                    let mut tactics = Tactics::defaults(&content.tactics);
                    tactics.set_formation(formation, &content.tactics);
                    config = config.with_tactics(team, tactics);
                }
                ("two formations".into(), both(tune(config), none))
            }
        };
        (name, old, new)
    })
}

#[test]
fn real_matches_give_the_same_empty_report() {
    for (name, old, new) in real_matches(false) {
        assert!(old.is_empty(), "{name}: {old:?}");
        assert_eq!(new, capped(&old), "{name}");
        assert!(new.0.is_empty(), "{name}");
    }
}

/// The tight anchor breaks its rule past the cap on a real match: the running checker
/// keeps the first `KEPT_PER_RULE` and still counts every one the old validator reports.
#[test]
fn real_matches_under_a_tight_anchor_give_the_same_report() {
    let mut anchors = 0;
    let mut past_cap = 0;
    for (name, old, new) in real_matches(true) {
        assert!(!old.is_empty(), "{name}: the tight anchor finds drift");
        assert_eq!(new, capped(&old), "{name}");
        anchors += old.iter().filter(|v| v.rule == "anchor_tolerance").count();
        past_cap += new
            .1
            .iter()
            .filter(|(_, n)| *n > RunningCheck::KEPT_PER_RULE)
            .count();
    }
    assert!(anchors > 0);
    assert!(past_cap > 0, "a rule counted past the cap");
}

/// A clean match through the sink alone: the running checker reports nothing.
#[test]
fn a_clean_full_match_reports_no_violation() {
    let config = common::full_match();
    let rules = StreamRules::for_config(&config);
    let mut sim = Simulation::new(config).unwrap();
    let mut check = RunningCheck::for_match(&sim, rules);
    sim.run(&mut check).unwrap();
    assert_eq!(check.ticks(), sim.tick());
    assert!(check.finish_match(&sim).is_empty());
    // The checker took no event: the match still holds them all.
    assert!(sim.take_events().iter().any(|e| e.kind == FullTime));
}
