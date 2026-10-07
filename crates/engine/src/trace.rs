//! The debug trace (named mechanism: draw recorder). With debug mode on, the stream registry
//! records every random draw inside its only draw call, and explicit hooks record each
//! decision point and each rule outcome of the coverage list ([`Point`]). All records go to
//! one buffer, so they come out in the order the engine ran them. Nothing in the trace is
//! hashed or saved in a snapshot, and a hook never draws or writes match state, so turning
//! debug mode on changes no result.
//!
//! The recorder is compiled in with the default cargo feature `debug-trace` and switched on
//! per match at run time ([`crate::Simulation::new_traced`]). A build without the feature
//! has no recorder; [`COMPILED`] says which build is running.
//!
//! The trace file is JSON Lines: one header line ([`TraceHeader`]), then one object per
//! record, with `"t"` (the tick) and `"k"` (`draw`, `decision`, or `rule`).

use std::io::Write;

use serde_json::{Value, json};

#[cfg(feature = "scenario")]
use crate::sim::EngineEvent;
use crate::sim::EngineEventKind;
use crate::streams::{Action, KEY_COUNT, Key};

/// The version of the trace file format.
pub const TRACE_VERSION: u32 = 1;

/// `true` when this build has the draw recorder (the `debug-trace` feature).
pub const COMPILED: bool = cfg!(feature = "debug-trace");

/// The maths library every engine sine, cosine, exponent, and arctangent uses, as the
/// header names it. The version is the one `Cargo.lock` resolves, read by the build script.
pub const MATHS: &str = concat!("libm ", env!("ENGINE_LIBM_VERSION"));

/// One random draw.
#[derive(Debug, Clone, PartialEq)]
pub struct DrawRecord {
    /// The tick the step produces (`tick + 1` while it runs), as for events.
    pub tick: u32,
    /// The stream key: its table row (the subsystem and the kind of action) and the player.
    pub key: Key,
    /// The draw's number within its key since the trace was switched on, from 0.
    pub index: u64,
    /// The value in `[0, 1)` the registry returned.
    pub value: f64,
    /// The probabilities the draw is tested against, at a chance draw: one, or two
    /// cumulative thresholds for the tackle and the card. Empty for a value draw.
    pub thresholds: Vec<f64>,
    /// `true` when a test scene's scripted queue served the draw.
    pub scripted: bool,
}

/// One decision point or rule outcome.
#[derive(Debug, Clone, PartialEq)]
pub struct PointRecord {
    pub tick: u32,
    pub point: Point,
    /// The option scores of a decision point, or the facts of a rule outcome.
    pub detail: Value,
}

/// One trace record.
#[derive(Debug, Clone, PartialEq)]
pub enum TraceRecord {
    Draw(DrawRecord),
    Point(PointRecord),
}

impl TraceRecord {
    pub fn tick(&self) -> u32 {
        match self {
            TraceRecord::Draw(d) => d.tick,
            TraceRecord::Point(p) => p.tick,
        }
    }

    /// The record as one JSON Lines object.
    pub fn to_json(&self) -> Value {
        match self {
            TraceRecord::Draw(d) => {
                let row = d.key.action.row();
                let mut v = json!({
                    "t": d.tick,
                    "k": "draw",
                    "subsystem": row.subsystem.name(),
                    "stream_id": format!("{:#018x}", d.key.stream_id()),
                    "key": d.key.name(),
                    "index": d.index,
                    "value": d.value,
                    "scripted": d.scripted,
                });
                if !d.thresholds.is_empty() {
                    v["p"] = json!(d.thresholds);
                }
                v
            }
            TraceRecord::Point(p) => json!({
                "t": p.tick,
                "k": if p.point.is_decision() { "decision" } else { "rule" },
                "point": p.point.name(),
                "detail": p.detail,
            }),
        }
    }
}

/// The trace buffer of one match. It lives in the stream registry, so the draw recorder
/// and the hooks write to one buffer in execution order.
#[derive(Debug, Clone)]
pub struct Trace {
    tick: u32,
    records: Vec<TraceRecord>,
    /// Draws per key (by the key's dense index) since the trace was switched on.
    per_key: Vec<u64>,
    /// The pass candidates the carrier's options scored in this call, taken by the carrier
    /// record.
    candidates: Vec<(usize, f64)>,
}

impl Trace {
    /// An empty trace whose records carry `tick` until the next stamp.
    pub fn new(tick: u32) -> Self {
        Self {
            tick,
            records: Vec::new(),
            per_key: vec![0; KEY_COUNT],
            candidates: Vec::new(),
        }
    }

    /// Stamps the tick the next records carry.
    pub fn stamp(&mut self, tick: u32) {
        self.tick = tick;
    }

    /// Records one draw. In match play the registry's draw call is its only caller.
    pub fn push_draw(&mut self, key: Key, value: f64, thresholds: &[f64], scripted: bool) {
        let index = match key.index() {
            Some(slot) => {
                let n = self.per_key[slot];
                self.per_key[slot] += 1;
                n
            }
            // A release build draws only table keys; a debug build has already refused one.
            None => 0,
        };
        self.records.push(TraceRecord::Draw(DrawRecord {
            tick: self.tick,
            key,
            index,
            value,
            thresholds: thresholds.to_vec(),
            scripted,
        }));
    }

    /// Records one decision point or rule outcome.
    pub fn push_point(&mut self, point: Point, detail: Value) {
        self.records.push(TraceRecord::Point(PointRecord {
            tick: self.tick,
            point,
            detail,
        }));
    }

    /// Notes one scored pass candidate for the next carrier record.
    pub fn push_candidate(&mut self, mate: usize, score: f64) {
        self.candidates.push((mate, score));
    }

    /// Takes the pass candidates noted since the last call.
    pub fn take_candidates(&mut self) -> Vec<(usize, f64)> {
        std::mem::take(&mut self.candidates)
    }

    /// Takes every record since the last call.
    pub fn take(&mut self) -> Vec<TraceRecord> {
        std::mem::take(&mut self.records)
    }
}

/// `true` for the 20 actions whose draw is tested against a probability. The other 18 are
/// value draws: score noise, aims, angles, lofts, the keeper's dive, and added time. The
/// list is outside the stream table, so the scheme digest does not change.
pub fn is_chance(action: Action) -> bool {
    match action {
        Action::ClearWide
        | Action::ShotSide
        | Action::Block
        | Action::Save
        | Action::SaveHold
        | Action::ParrySide
        | Action::CrossClear
        | Action::CrossWide
        | Action::KeeperCatch
        | Action::Tackle
        | Action::FoulCard
        | Action::ExtraKickOff
        | Action::ShootoutFirstTeam
        | Action::ShootoutEnd
        | Action::ShootoutSave
        | Action::ShootoutSaveHold
        | Action::InjuryMinute
        | Action::InjuryTackle
        | Action::Lapse
        | Action::Header => true,
        Action::ShotScore
        | Action::PassScore
        | Action::DribbleScore
        | Action::HoldScore
        | Action::ClearScore
        | Action::PassAim
        | Action::ClearAim
        | Action::ShotAim
        | Action::ShotSpread
        | Action::ShotLoft
        | Action::BlockDeflect
        | Action::ParryAngle
        | Action::ParryLoft
        | Action::CrossAngle
        | Action::CrossLoft
        | Action::ExtraTimeAdded
        | Action::AddedTime
        | Action::KeeperDive => false,
    }
}

/// The coverage list: every decision point (the engine picks one of several alternatives
/// by score, distance, rank, or a hook's answer) and every rule outcome (a law, a roll, or
/// a match-control rule settles what happens). Formation anchors and the lone forward's
/// line hold are formulas with no alternative, so they are not points.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Point {
    // Decision points.
    Carrier,
    RestartPass,
    Press,
    Cover,
    Chase,
    LooseBall,
    RestartTaker,
    AiManager,
    ScriptDecision,
    ShootoutOrder,
    // Rule outcomes.
    KickOff,
    Goal,
    BallOut,
    Tackle,
    Foul,
    Card,
    Offside,
    Injury,
    DeadBall,
    RestartTaken,
    AddedTime,
    ExtraTimeAdded,
    HalfTime,
    ExtraTimeKickOff,
    FullTime,
    ShotBlock,
    ShotSave,
    CrossClear,
    KeeperCatch,
    ShootoutStart,
    ShootoutSave,
    ShootoutKick,
    ShootoutDecided,
    ChangeApplied,
    ChangeRejected,
    SendOff,
    Abandoned,
}

impl Point {
    /// Every point: 10 decision points, then 27 rule outcomes.
    pub const ALL: [Point; 37] = [
        Point::Carrier,
        Point::RestartPass,
        Point::Press,
        Point::Cover,
        Point::Chase,
        Point::LooseBall,
        Point::RestartTaker,
        Point::AiManager,
        Point::ScriptDecision,
        Point::ShootoutOrder,
        Point::KickOff,
        Point::Goal,
        Point::BallOut,
        Point::Tackle,
        Point::Foul,
        Point::Card,
        Point::Offside,
        Point::Injury,
        Point::DeadBall,
        Point::RestartTaken,
        Point::AddedTime,
        Point::ExtraTimeAdded,
        Point::HalfTime,
        Point::ExtraTimeKickOff,
        Point::FullTime,
        Point::ShotBlock,
        Point::ShotSave,
        Point::CrossClear,
        Point::KeeperCatch,
        Point::ShootoutStart,
        Point::ShootoutSave,
        Point::ShootoutKick,
        Point::ShootoutDecided,
        Point::ChangeApplied,
        Point::ChangeRejected,
        Point::SendOff,
        Point::Abandoned,
    ];

    /// The point's name in the trace file.
    pub fn name(self) -> &'static str {
        match self {
            Point::Carrier => "carrier",
            Point::RestartPass => "restart_pass",
            Point::Press => "press",
            Point::Cover => "cover",
            Point::Chase => "chase",
            Point::LooseBall => "loose_ball",
            Point::RestartTaker => "restart_taker",
            Point::AiManager => "ai_manager",
            Point::ScriptDecision => "script_decision",
            Point::ShootoutOrder => "shootout_order",
            Point::KickOff => "kick_off",
            Point::Goal => "goal",
            Point::BallOut => "ball_out",
            Point::Tackle => "tackle",
            Point::Foul => "foul",
            Point::Card => "card",
            Point::Offside => "offside",
            Point::Injury => "injury",
            Point::DeadBall => "dead_ball",
            Point::RestartTaken => "restart_taken",
            Point::AddedTime => "added_time",
            Point::ExtraTimeAdded => "extra_time_added",
            Point::HalfTime => "half_time",
            Point::ExtraTimeKickOff => "extra_time_kick_off",
            Point::FullTime => "full_time",
            Point::ShotBlock => "shot_block",
            Point::ShotSave => "shot_save",
            Point::CrossClear => "cross_clear",
            Point::KeeperCatch => "keeper_catch",
            Point::ShootoutStart => "shootout_start",
            Point::ShootoutSave => "shootout_save",
            Point::ShootoutKick => "shootout_kick",
            Point::ShootoutDecided => "shootout_decided",
            Point::ChangeApplied => "change_applied",
            Point::ChangeRejected => "change_rejected",
            Point::SendOff => "send_off",
            Point::Abandoned => "abandoned",
        }
    }

    /// `true` for a decision point, `false` for a rule outcome.
    pub fn is_decision(self) -> bool {
        matches!(
            self,
            Point::Carrier
                | Point::RestartPass
                | Point::Press
                | Point::Cover
                | Point::Chase
                | Point::LooseBall
                | Point::RestartTaker
                | Point::AiManager
                | Point::ScriptDecision
                | Point::ShootoutOrder
        )
    }
}

/// The points at which a draw of `action` is taken: a draw's tick holds at least one of
/// them. Exhaustive, so a new table row does not compile until it is mapped.
pub fn points_of(action: Action) -> &'static [Point] {
    match action {
        Action::ShotScore
        | Action::PassScore
        | Action::DribbleScore
        | Action::HoldScore
        | Action::ClearScore
        | Action::PassAim
        | Action::ClearWide
        | Action::ClearAim => &[Point::Carrier],
        // Open-play shots, and penalty and shoot-out kicks (`shot_kick`).
        Action::ShotSide | Action::ShotAim | Action::ShotSpread | Action::ShotLoft => {
            &[Point::Carrier, Point::RestartTaken]
        }
        Action::Block | Action::BlockDeflect => &[Point::ShotBlock],
        Action::Save | Action::SaveHold => &[Point::ShotSave],
        Action::ParrySide | Action::ParryAngle | Action::ParryLoft => {
            &[Point::ShotSave, Point::ShootoutSave]
        }
        Action::CrossClear | Action::CrossWide | Action::CrossAngle | Action::CrossLoft => {
            &[Point::CrossClear]
        }
        Action::KeeperCatch => &[Point::KeeperCatch],
        Action::Tackle => &[Point::Tackle],
        Action::FoulCard => &[Point::Foul],
        Action::ExtraTimeAdded => &[Point::ExtraTimeAdded],
        Action::AddedTime => &[Point::AddedTime],
        Action::ExtraKickOff => &[Point::ExtraTimeKickOff],
        Action::ShootoutFirstTeam | Action::ShootoutEnd => &[Point::ShootoutStart],
        Action::KeeperDive => &[Point::RestartTaken],
        Action::ShootoutSave | Action::ShootoutSaveHold => &[Point::ShootoutSave],
        Action::InjuryMinute | Action::InjuryTackle => &[Point::Injury],
        // A lapse is a defender losing his place in the shape.
        Action::Lapse => &[Point::Cover],
        Action::Header => &[Point::LooseBall],
    }
}

/// The points that give an event of `kind`: an event's tick holds at least one of them.
/// Exhaustive, so a new event kind does not compile until it is mapped.
pub fn points_of_event(kind: EngineEventKind) -> &'static [Point] {
    match kind {
        // The opening kick-off and each half's are placed at once; after a goal the
        // kick-off is a dead ball.
        EngineEventKind::KickOff => &[Point::KickOff, Point::DeadBall],
        EngineEventKind::Goal => &[Point::Goal],
        EngineEventKind::HalfTime => &[Point::HalfTime],
        EngineEventKind::FullTime => &[Point::FullTime],
        EngineEventKind::Offside => &[Point::Offside],
        EngineEventKind::Foul => &[Point::Foul],
        EngineEventKind::Card => &[Point::Card],
        EngineEventKind::ThrowIn
        | EngineEventKind::Corner
        | EngineEventKind::GoalKick
        | EngineEventKind::FreeKick => &[Point::DeadBall],
        // A penalty in play, and each shoot-out kick set up and decided.
        EngineEventKind::Penalty => &[Point::DeadBall, Point::ShootoutStart, Point::ShootoutKick],
        EngineEventKind::Injury => &[Point::Injury],
        EngineEventKind::Substitution => &[Point::ChangeApplied],
        EngineEventKind::AiDecision => &[Point::AiManager],
        EngineEventKind::ChangeApplied => &[Point::ChangeApplied],
        EngineEventKind::ChangeRejected => &[Point::ChangeRejected],
        // The hooks that can fail inside a step: the decision hook and the rule hook (a
        // foul's card). The commentary hook runs outside the step.
        EngineEventKind::Script => &[Point::ScriptDecision, Point::Foul],
    }
}

/// The header line of a trace file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceHeader {
    pub seed: u64,
    /// The stream scheme id.
    pub scheme: u8,
}

impl TraceHeader {
    pub fn to_json(&self) -> Value {
        json!({
            "trace_version": TRACE_VERSION,
            "seed": self.seed,
            "scheme": self.scheme,
            "engine": crate::build_hash(),
            "crate_version": crate::version(),
            "maths": MATHS,
        })
    }
}

/// Writes the header line.
pub fn write_header(w: &mut impl Write, header: &TraceHeader) -> std::io::Result<()> {
    serde_json::to_writer(&mut *w, &header.to_json())?;
    w.write_all(b"\n")
}

/// Writes one line per record.
pub fn write_records(w: &mut impl Write, records: &[TraceRecord]) -> std::io::Result<()> {
    for r in records {
        serde_json::to_writer(&mut *w, &r.to_json())?;
        w.write_all(b"\n")?;
    }
    Ok(())
}

/// Record counts by kind.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Counts {
    pub draws: u64,
    pub decisions: u64,
    pub rules: u64,
}

impl Counts {
    pub fn add(&mut self, records: &[TraceRecord]) {
        for r in records {
            match r {
                TraceRecord::Draw(_) => self.draws += 1,
                TraceRecord::Point(p) if p.point.is_decision() => self.decisions += 1,
                TraceRecord::Point(_) => self.rules += 1,
            }
        }
    }
}

/// Checks the draw records of a whole traced match against the registry's draw counter
/// `registry`: one record per draw served, each on a table key with a value in
/// `[0, 1)`, with thresholds exactly at the chance actions ([`is_chance`]), and each key's
/// indexes consecutive from 0. Returns the first failure in words.
#[cfg(feature = "scenario")]
pub fn check_draws<'a>(
    records: impl IntoIterator<Item = &'a TraceRecord>,
    registry: u64,
) -> Result<(), String> {
    let mut check = DrawCheck::default();
    for r in records {
        check.one(r)?;
    }
    check.finish(registry)
}

/// The draw check of [`check_draws`], fed one tick at a time.
#[cfg(feature = "scenario")]
#[derive(Debug, Clone)]
pub struct DrawCheck {
    next: Vec<u64>,
    n: u64,
}

#[cfg(feature = "scenario")]
impl Default for DrawCheck {
    fn default() -> Self {
        Self {
            next: vec![0; KEY_COUNT],
            n: 0,
        }
    }
}

#[cfg(feature = "scenario")]
impl DrawCheck {
    /// Checks the draws among `records`.
    pub fn feed(&mut self, records: &[TraceRecord]) -> Result<(), String> {
        records.iter().try_for_each(|r| self.one(r))
    }

    /// The draws checked so far.
    pub fn count(&self) -> u64 {
        self.n
    }

    /// Checks the count against the registry's draw counter.
    pub fn finish(&self, registry: u64) -> Result<(), String> {
        if self.n != registry {
            return Err(format!("{} draws recorded, registry {registry}", self.n));
        }
        Ok(())
    }

    fn one(&mut self, r: &TraceRecord) -> Result<(), String> {
        let TraceRecord::Draw(d) = r else {
            return Ok(());
        };
        self.n += 1;
        let Some(slot) = d.key.index() else {
            return Err(format!("tick {}: draw on a key outside the table", d.tick));
        };
        let name = d.key.name();
        if !(0.0..1.0).contains(&d.value) {
            return Err(format!(
                "tick {}: {name} value {} outside [0, 1)",
                d.tick, d.value
            ));
        }
        let chance = is_chance(d.key.action);
        if chance && d.thresholds.is_empty() {
            return Err(format!(
                "tick {}: chance draw {name} has no probability",
                d.tick
            ));
        }
        if !chance && !d.thresholds.is_empty() {
            return Err(format!(
                "tick {}: value draw {name} has a probability",
                d.tick
            ));
        }
        if d.index != self.next[slot] {
            return Err(format!(
                "tick {}: {name} index {}, expected {}",
                d.tick, d.index, self.next[slot]
            ));
        }
        self.next[slot] += 1;
        Ok(())
    }
}

/// Checks one tick's records against the coverage anchors: every draw has at least
/// one of its action's points ([`points_of`]) on its tick, and every event has at least one
/// of its kind's points ([`points_of_event`]) on its tick. `records` and `events` are what
/// a traced run hands over for a tick. Returns the first failure in words.
#[cfg(feature = "scenario")]
pub fn check_tick(records: &[TraceRecord], events: &[EngineEvent]) -> Result<(), String> {
    use std::collections::{BTreeMap, BTreeSet};
    let mut seen: BTreeMap<u32, BTreeSet<Point>> = BTreeMap::new();
    for r in records {
        if let TraceRecord::Point(p) = r {
            seen.entry(p.tick).or_default().insert(p.point);
        }
    }
    let has = |tick: u32, points: &[Point]| {
        seen.get(&tick)
            .is_some_and(|s| points.iter().any(|p| s.contains(p)))
    };
    for r in records {
        if let TraceRecord::Draw(d) = r
            && !has(d.tick, points_of(d.key.action))
        {
            return Err(format!(
                "tick {}: draw {} has none of its points {:?}",
                d.tick,
                d.key.name(),
                names(points_of(d.key.action))
            ));
        }
    }
    for e in events {
        if !has(e.tick, points_of_event(e.kind)) {
            return Err(format!(
                "tick {}: event {} has none of its points {:?}",
                e.tick,
                e.kind.code(),
                names(points_of_event(e.kind))
            ));
        }
    }
    Ok(())
}

#[cfg(feature = "scenario")]
fn names(points: &[Point]) -> Vec<&'static str> {
    points.iter().map(|p| p.name()).collect()
}

/// Adds the points `records` hold to `seen`.
#[cfg(feature = "scenario")]
pub fn note_points(seen: &mut std::collections::BTreeSet<Point>, records: &[TraceRecord]) {
    for r in records {
        if let TraceRecord::Point(p) = r {
            seen.insert(p.point);
        }
    }
}

/// The points the 22 gate matches never reach. Each has a scene test in
/// `crates/engine/tests/trace.rs` that forces it and checks its record.
#[cfg(feature = "scenario")]
pub const SCENE_ONLY: &[Point] = &[Point::CrossClear, Point::Abandoned];

/// Checks the partition: the points the gate matches reached plus [`SCENE_ONLY`] are
/// exactly [`Point::ALL`]. Returns the missing points by name, or the scene-only points the
/// matches reached after all.
#[cfg(feature = "scenario")]
pub fn check_partition(seen: &std::collections::BTreeSet<Point>) -> Result<(), String> {
    let missing: Vec<_> = Point::ALL
        .iter()
        .filter(|p| !seen.contains(p) && !SCENE_ONLY.contains(p))
        .map(|p| p.name())
        .collect();
    if !missing.is_empty() {
        return Err(format!("no record of {}", missing.join(", ")));
    }
    let both: Vec<_> = SCENE_ONLY
        .iter()
        .filter(|p| seen.contains(p))
        .map(|p| p.name())
        .collect();
    if !both.is_empty() {
        return Err(format!(
            "the gate matches reach the scene-only points {}",
            both.join(", ")
        ));
    }
    Ok(())
}
