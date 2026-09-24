//! The snapshot (named mechanism: snapshot at every stoppage). A snapshot holds the whole
//! state of a running match, so a match resumed from it continues tick for tick like the one
//! that never stopped, on the same build with the same content.
//!
//! Layout, little-endian:
//! - Header, 64 bytes: magic `SMSN`, version (u16), two reserved bytes, the build hash
//!   (16 bytes, zero-padded), the content hash (12 bytes), `owner.id` (16 bytes), and the
//!   match stamp (u64 milliseconds). Four bytes are reserved.
//! - Body: the seed, the length in minutes, the team-file digests, and every field of the
//!   running match except the scratch buffer and the events already handed out: the lineups,
//!   benches, tactics, substitutions used, and managers of both teams, each player's squad
//!   identity, energy, and effective values, the change queue in order with the tick of the
//!   latest stoppage that admitted each change kind, and the AI manager's memory. Every float is stored as its exact bits. Version 2 added the tactics fields. Version 4 added the
//!   knockout switch (the byte after the tick), the extra-time periods and their added
//!   time, the team that kicked off extra time, and the shoot-out: the kickers in order,
//!   the keepers, both order cursors, the team that kicks first, the end, the scores, the
//!   kicks taken, and the kick in progress. Version 5 added each player's foul cooldown (the
//!   first tick the player may tackle again), after the yellow cards.
//! - Trailer, 40 bytes: magic `SMSE`, the body length (u32), and the SHA-256 of the header
//!   and the body.
//!
//! The reader fails closed, in order: magic, version, length, checksum, build. The content
//! and the team files are checked when a match is rebuilt from the snapshot. A refusal names
//! the check that failed and never quotes the file's bytes.

use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::ai::{AiState, Manager};
use crate::data::rules::StoppageKind;
use crate::error::EngineError;
use crate::math::{DVec2, DVec3};
use crate::player::{Derived, Status};
use crate::record::TickSink;
use crate::rng::{EngineRng, RngState};
use crate::rules::clock::Tally;
use crate::rules::fouls::Card;
use crate::rules::{DeadBall, PendingCard, Phase, Shootout, Stoppage};
use crate::sim::{DecidedBy, MatchConfig, Simulation, Summary};
use crate::tactics::change::{Change, ChangeId, QueuedChange, SubLedger};
use crate::tactics::{RoleDuty, Tactics, TacticsPatch};
use crate::team::PLAYERS_PER_TEAM;

/// Layout version this build reads and writes.
pub const VERSION: u16 = 5;
/// The file name of a match's latest snapshot inside its match folder.
pub const FILE_NAME: &str = "snapshot.smsn";

const MAGIC: &[u8; 4] = b"SMSN";
const TRAILER_MAGIC: &[u8; 4] = b"SMSE";
const HEADER_BYTES: usize = 64;
const TRAILER_BYTES: usize = 40;
const BUILD_AT: usize = 8;
const BUILD_BYTES: usize = 16;
const CONTENT_AT: usize = 24;
const CONTENT_BYTES: usize = 12;
const OWNER_AT: usize = 36;
const MILLIS_AT: usize = 52;
/// An absent roster index.
const NONE: u8 = u8::MAX;

/// One snapshot: the match identity in the header and the encoded match state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    pub build_hash: String,
    pub content_hash: String,
    pub owner_id: [u8; 16],
    pub match_millis: u64,
    body: Vec<u8>,
}

impl Snapshot {
    /// Captures `sim` as it stands, under the given match identity.
    pub fn capture(sim: &Simulation, owner_id: [u8; 16], match_millis: u64) -> Self {
        let mut w = Writer::default();
        encode(sim, &mut w);
        Self {
            build_hash: crate::build_hash().to_string(),
            content_hash: sim.config.content_hash.clone(),
            owner_id,
            match_millis,
            body: w.0,
        }
    }

    /// The seed of the captured match.
    pub fn seed(&self) -> u64 {
        u64::from_le_bytes(self.body[0..8].try_into().expect("8 bytes"))
    }

    /// The length of the captured match in minutes.
    pub fn minutes(&self) -> u32 {
        u32::from_le_bytes(self.body[8..12].try_into().expect("4 bytes"))
    }

    /// The tick the snapshot was taken on.
    pub fn tick(&self) -> u32 {
        let at = 12 + 64;
        u32::from_le_bytes(self.body[at..at + 4].try_into().expect("4 bytes"))
    }

    /// `true` when the captured match is a knockout match. A match resumed from the snapshot
    /// must be configured the same way.
    pub fn knockout(&self) -> bool {
        self.body.get(12 + 64 + 4).is_some_and(|&b| b == 1)
    }

    /// The whole file.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(HEADER_BYTES + self.body.len() + TRAILER_BYTES);
        let mut header = [0u8; HEADER_BYTES];
        header[0..4].copy_from_slice(MAGIC);
        header[4..6].copy_from_slice(&VERSION.to_le_bytes());
        put_text(
            &mut header[BUILD_AT..BUILD_AT + BUILD_BYTES],
            &self.build_hash,
        );
        put_text(
            &mut header[CONTENT_AT..CONTENT_AT + CONTENT_BYTES],
            &self.content_hash,
        );
        header[OWNER_AT..OWNER_AT + 16].copy_from_slice(&self.owner_id);
        header[MILLIS_AT..MILLIS_AT + 8].copy_from_slice(&self.match_millis.to_le_bytes());
        out.extend_from_slice(&header);
        out.extend_from_slice(&self.body);
        let digest: [u8; 32] = Sha256::digest(&out).into();
        out.extend_from_slice(TRAILER_MAGIC);
        // The body of a match state is a few kilobytes, far below 4 GiB.
        out.extend_from_slice(&(self.body.len() as u32).to_le_bytes());
        out.extend_from_slice(&digest);
        out
    }

    /// Parses a whole file. `shown` names the file in a refusal.
    pub fn from_bytes(bytes: &[u8], shown: &str) -> Result<Self, EngineError> {
        let refuse = |reason: String| {
            let data_dir =
                std::env::var_os(crate::observe::identity::DATA_DIR_ENV).map(PathBuf::from);
            tracing::error!(
                signal = "snapshot.refused",
                path = %shorten_for_log(shown, data_dir.as_deref()),
                reason = %reason
            );
            EngineError::Snapshot {
                path: shown.to_string(),
                reason,
            }
        };
        if bytes.len() < 4 || &bytes[0..4] != MAGIC {
            return Err(refuse("bad magic: not a snapshot file".into()));
        }
        if bytes.len() < 6 {
            return Err(refuse(format!("truncated: {} bytes", bytes.len())));
        }
        let version = u16::from_le_bytes([bytes[4], bytes[5]]);
        if version != VERSION {
            return Err(refuse(format!(
                "unknown version {version}; this build reads {VERSION}"
            )));
        }
        if bytes.len() < HEADER_BYTES + TRAILER_BYTES {
            return Err(refuse(format!("truncated: {} bytes", bytes.len())));
        }
        let trailer = &bytes[bytes.len() - TRAILER_BYTES..];
        if &trailer[0..4] != TRAILER_MAGIC {
            return Err(refuse("truncated: the trailer is missing".into()));
        }
        let body_len = u32::from_le_bytes(trailer[4..8].try_into().expect("4 bytes")) as usize;
        let held = bytes.len() - HEADER_BYTES - TRAILER_BYTES;
        if body_len != held {
            return Err(refuse(format!(
                "length mismatch: the trailer says {body_len} body bytes, the file holds {held}"
            )));
        }
        let digest: [u8; 32] = Sha256::digest(&bytes[..bytes.len() - TRAILER_BYTES]).into();
        if digest[..] != trailer[8..40] {
            return Err(refuse("checksum mismatch: the file is corrupt".into()));
        }
        let header = &bytes[..HEADER_BYTES];
        let build_hash = get_text(&header[BUILD_AT..BUILD_AT + BUILD_BYTES]);
        let ours = crate::build_hash();
        if build_hash != fit(ours, BUILD_BYTES) {
            return Err(refuse(format!(
                "build mismatch: written by build {build_hash}; this build is {ours}"
            )));
        }
        let body = bytes[HEADER_BYTES..HEADER_BYTES + body_len].to_vec();
        if body.len() < 12 + 64 + 4 {
            return Err(refuse("malformed body: too short".into()));
        }
        Ok(Self {
            build_hash,
            content_hash: get_text(&header[CONTENT_AT..CONTENT_AT + CONTENT_BYTES]),
            owner_id: header[OWNER_AT..OWNER_AT + 16]
                .try_into()
                .expect("16 bytes"),
            match_millis: u64::from_le_bytes(
                header[MILLIS_AT..MILLIS_AT + 8]
                    .try_into()
                    .expect("8 bytes"),
            ),
            body,
        })
    }

    /// Reads a snapshot file. `shown` names the file in a refusal.
    pub fn read(path: &Path, shown: &str) -> Result<Self, EngineError> {
        let bytes = std::fs::read(path).map_err(|source| EngineError::Read {
            path: shown.to_string(),
            source,
        })?;
        Self::from_bytes(&bytes, shown)
    }

    /// Writes the file at `path` so a reader never sees half of it: the bytes go to a
    /// temporary file in the same folder, which then replaces `path`. Returns the byte count.
    pub fn write_atomic(&self, path: &Path) -> Result<usize, EngineError> {
        let bytes = self.to_bytes();
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let temporary = path.with_extension("smsn.tmp");
        std::fs::write(&temporary, &bytes)?;
        // `rename` replaces an existing file on Windows 10 1607 and later
        // (source: https://doc.rust-lang.org/std/fs/fn.rename.html).
        std::fs::rename(&temporary, path)?;
        Ok(bytes.len())
    }
}

impl Simulation {
    /// Rebuilds the match a snapshot captured. `config` must come from the same content and
    /// team files, with the snapshot's seed and length; anything else is refused by name.
    pub fn from_snapshot(config: MatchConfig, snapshot: &Snapshot) -> Result<Self, EngineError> {
        let refuse = |reason: String| EngineError::Snapshot {
            path: FILE_NAME.to_string(),
            reason,
        };
        if snapshot.content_hash != config.content_hash {
            return Err(refuse(format!(
                "content mismatch: written with content {}; loaded content {}",
                snapshot.content_hash, config.content_hash
            )));
        }
        let mut sim = Simulation::blank(config)?;
        let mut r = Reader::new(&snapshot.body);
        decode(&mut sim, &mut r).map_err(|reason| refuse(reason.to_string()))?;
        sim.timeline = vec![(sim.tick, sim.teams.clone())];
        Ok(sim)
    }
}

/// Writes the latest snapshot of one match at every stoppage, replacing the one before it
/// (`SM_DATA_DIR/matches/<match.id>/snapshot.smsn`). A failed write is logged and the match
/// continues.
pub struct SnapshotSink {
    path: PathBuf,
    shown: String,
    match_id: String,
    owner_id: [u8; 16],
    match_millis: u64,
    /// Snapshots written.
    pub writes: u32,
}

impl SnapshotSink {
    pub fn new(data_dir: &Path, match_id: &str, owner_id: [u8; 16], match_millis: u64) -> Self {
        Self {
            path: data_dir.join("matches").join(match_id).join(FILE_NAME),
            shown: format!("matches/{match_id}/{FILE_NAME}"),
            match_id: match_id.to_string(),
            owner_id,
            match_millis,
            writes: 0,
        }
    }

    /// Where the snapshot is written.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl TickSink for SnapshotSink {
    fn on_tick(&mut self, _record: &crate::record::TickRecord) -> Result<(), EngineError> {
        Ok(())
    }

    fn on_stoppage(&mut self, stoppage: &Stoppage, sim: &Simulation) -> Result<(), EngineError> {
        let snapshot = Snapshot::capture(sim, self.owner_id, self.match_millis);
        match snapshot.write_atomic(&self.path) {
            Ok(bytes) => {
                self.writes += 1;
                tracing::debug!(
                    signal = "snapshot.written",
                    match.id = %self.match_id,
                    snapshot.tick = stoppage.tick,
                    kind = stoppage.kind.code(),
                    bytes
                );
            }
            Err(err) => {
                tracing::error!(
                    signal = "snapshot.write_failed",
                    match.id = %self.match_id,
                    snapshot.tick = stoppage.tick,
                    path = %self.shown,
                    reason = %full_reason(&err)
                );
            }
        }
        Ok(())
    }
}

fn put_text(slot: &mut [u8], text: &str) {
    let fitted = fit(text, slot.len());
    slot[..fitted.len()].copy_from_slice(fitted.as_bytes());
}

fn get_text(slot: &[u8]) -> String {
    let end = slot.iter().position(|&b| b == 0).unwrap_or(slot.len());
    String::from_utf8_lossy(&slot[..end]).into_owned()
}

/// Shortens `shown` for a log signal: relative to `data_dir` (normally `SM_DATA_DIR`) or
/// the working directory when the path falls under one of them, otherwise just the file
/// name. A signal never carries an absolute path, so a user name in a home folder never
/// reaches a log; the error text shown to the user on stderr keeps the path as typed.
pub fn shorten_for_log(shown: &str, data_dir: Option<&Path>) -> String {
    let path = Path::new(shown);
    if !path.is_absolute() {
        return shown.to_string();
    }
    let cwd = std::env::current_dir().ok();
    for base in [data_dir, cwd.as_deref()].into_iter().flatten() {
        if let Ok(rel) = path.strip_prefix(base) {
            return rel.to_string_lossy().replace('\\', "/");
        }
    }
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| shown.to_string())
}

/// Formats `err` together with its source chain, one cause per `: `, the way the error
/// line shown to the user already renders it (`anyhow`'s alternate `{:#}`). Used so a
/// signal's `reason` carries the underlying OS error instead of just the top-level text.
fn full_reason(err: &EngineError) -> String {
    let mut out = err.to_string();
    let mut source = std::error::Error::source(err);
    while let Some(s) = source {
        out.push_str(": ");
        out.push_str(&s.to_string());
        source = s.source();
    }
    out
}

/// The longest prefix of `text` that fits `len` bytes.
fn fit(text: &str, len: usize) -> &str {
    let mut end = text.len().min(len);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

#[derive(Default)]
struct Writer(Vec<u8>);

impl Writer {
    fn u8(&mut self, v: u8) {
        self.0.push(v);
    }
    fn u32(&mut self, v: u32) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn u64(&mut self, v: u64) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn u128(&mut self, v: u128) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn f64(&mut self, v: f64) {
        self.0.extend_from_slice(&v.to_bits().to_le_bytes());
    }
    fn v2(&mut self, v: DVec2) {
        self.f64(v.x);
        self.f64(v.y);
    }
    fn v3(&mut self, v: DVec3) {
        self.f64(v.x);
        self.f64(v.y);
        self.f64(v.z);
    }
    fn index(&mut self, v: Option<usize>) {
        // Roster indices are below 22 and team indices below 2.
        self.u8(v.map_or(NONE, |i| i as u8));
    }
    fn pair(&mut self, v: [u32; 2]) {
        self.u32(v[0]);
        self.u32(v[1]);
    }
    fn opt_pair(&mut self, v: Option<[u32; 2]>) {
        self.u8(u8::from(v.is_some()));
        self.pair(v.unwrap_or([0, 0]));
    }
    fn opt_u8(&mut self, v: Option<u8>) {
        self.u8(u8::from(v.is_some()));
        self.u8(v.unwrap_or(0));
    }
    fn tactics(&mut self, t: &Tactics) {
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
    fn derived(&mut self, d: &Derived) {
        for v in [
            d.max_speed,
            d.max_accel,
            d.passing,
            d.dribbling,
            d.tackling,
            d.positioning,
            d.aggression,
            d.finishing,
            d.vision,
            d.decisions,
            d.composure,
            d.stamina,
            d.natural_fitness,
            d.injury_resistance,
        ] {
            self.f64(v);
        }
    }
}

struct Reader<'a> {
    buf: &'a [u8],
    at: usize,
}

type Decoded<T> = Result<T, &'static str>;

impl<'a> Reader<'a> {
    fn new(buf: &'a [u8]) -> Self {
        Self { buf, at: 0 }
    }
    fn take(&mut self, n: usize) -> Decoded<&'a [u8]> {
        let end = self.at + n;
        let slice = self
            .buf
            .get(self.at..end)
            .ok_or("malformed body: it ends early")?;
        self.at = end;
        Ok(slice)
    }
    fn u8(&mut self) -> Decoded<u8> {
        Ok(self.take(1)?[0])
    }
    fn u32(&mut self) -> Decoded<u32> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().expect("4")))
    }
    fn u64(&mut self) -> Decoded<u64> {
        Ok(u64::from_le_bytes(self.take(8)?.try_into().expect("8")))
    }
    fn u128(&mut self) -> Decoded<u128> {
        Ok(u128::from_le_bytes(self.take(16)?.try_into().expect("16")))
    }
    fn f64(&mut self) -> Decoded<f64> {
        Ok(f64::from_bits(self.u64()?))
    }
    fn v2(&mut self) -> Decoded<DVec2> {
        Ok(DVec2::new(self.f64()?, self.f64()?))
    }
    fn v3(&mut self) -> Decoded<DVec3> {
        Ok(DVec3::new(self.f64()?, self.f64()?, self.f64()?))
    }
    fn bool(&mut self) -> Decoded<bool> {
        match self.u8()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err("malformed body: a flag is neither 0 nor 1"),
        }
    }
    /// An index below `limit`, or `None`.
    fn index(&mut self, limit: usize) -> Decoded<Option<usize>> {
        match self.u8()? {
            NONE => Ok(None),
            i if usize::from(i) < limit => Ok(Some(usize::from(i))),
            _ => Err("malformed body: an index is out of range"),
        }
    }
    fn some_index(&mut self, limit: usize) -> Decoded<usize> {
        self.index(limit)?
            .ok_or("malformed body: a required index is absent")
    }
    fn pair(&mut self) -> Decoded<[u32; 2]> {
        Ok([self.u32()?, self.u32()?])
    }
    fn opt_pair(&mut self) -> Decoded<Option<[u32; 2]>> {
        let known = self.bool()?;
        let pair = self.pair()?;
        Ok(known.then_some(pair))
    }
    fn opt_u8(&mut self) -> Decoded<Option<u8>> {
        let known = self.bool()?;
        let v = self.u8()?;
        Ok(known.then_some(v))
    }
    fn tactics(&mut self) -> Decoded<Tactics> {
        let formation = self.u8()?;
        let mentality = self.u8()?;
        let mut instructions = [0u8; 6];
        for level in &mut instructions {
            *level = self.u8()?;
        }
        let mut roles = [RoleDuty::default(); PLAYERS_PER_TEAM];
        for rd in &mut roles {
            *rd = RoleDuty {
                role: self.u8()?,
                duty: self.u8()?,
            };
        }
        Ok(Tactics {
            formation,
            mentality,
            instructions,
            roles,
        })
    }
    fn derived(&mut self) -> Decoded<Derived> {
        Ok(Derived {
            max_speed: self.f64()?,
            max_accel: self.f64()?,
            passing: self.f64()?,
            dribbling: self.f64()?,
            tackling: self.f64()?,
            positioning: self.f64()?,
            aggression: self.f64()?,
            finishing: self.f64()?,
            vision: self.f64()?,
            decisions: self.f64()?,
            composure: self.f64()?,
            stamina: self.f64()?,
            natural_fitness: self.f64()?,
            injury_resistance: self.f64()?,
        })
    }
}

fn status_code(s: Status) -> u8 {
    match s {
        Status::OnPitch => 0,
        Status::SentOff => 1,
        Status::Injured => 2,
    }
}

fn manager_code(m: Manager) -> u8 {
    match m {
        Manager::Ai => 0,
        Manager::Human => 1,
    }
}

const PLAYERS: usize = 2 * PLAYERS_PER_TEAM;

fn encode(sim: &Simulation, w: &mut Writer) {
    w.u64(sim.config.seed);
    w.u32(sim.config.minutes);
    for digest in &sim.config.team_digests {
        w.0.extend_from_slice(digest);
    }
    w.u32(sim.tick);
    w.u8(u8::from(sim.config.knockout));
    w.u8(u8::from(sim.restart));
    let rng = sim.rng.state();
    w.0.extend_from_slice(&rng.seed);
    w.u64(rng.stream);
    w.u128(rng.word_pos);
    w.v3(sim.ball.pos);
    w.v3(sim.ball.vel);
    w.index(sim.carrier);
    w.u32(sim.control_since);
    w.index(sim.last_touch);
    w.u8(u8::from(sim.keeper_beaten));
    encode_summary(&sim.summary, w);
    for (t, team) in sim.teams.iter().enumerate() {
        w.f64(team.attack_x);
        for (active, (x, y)) in team.active.iter().zip(team.formation.iter()) {
            w.u8(u8::from(*active));
            w.f64(*x);
            w.f64(*y);
        }
        w.tactics(&team.tactics);
        // Squad indices are below 40, the team file's limit.
        for s in team.lineup {
            w.u8(s as u8);
        }
        w.u8(team.bench.len() as u8);
        for s in &team.bench {
            w.u8(*s as u8);
        }
        let ledger = &sim.ledgers[t];
        w.u8(ledger.used);
        w.u8(ledger.windows);
        w.u8(u8::from(ledger.window_at.is_some()));
        w.u32(ledger.window_at.unwrap_or(0));
        w.u8(manager_code(sim.managers[t]));
        let ai = &sim.ai[t];
        w.opt_pair(ai.trailing_acted);
        w.opt_pair(ai.leading_acted);
        w.u8(u8::from(ai.due));
    }
    for p in &sim.players {
        w.v2(p.pos);
        w.v2(p.vel);
        w.v2(p.target);
        w.v2(p.facing);
        w.u8(status_code(p.status));
        w.u8(p.yellow);
        w.u32(p.foul_ready);
        w.u8(p.squad as u8);
        w.f64(p.energy);
        w.derived(&p.derived);
    }
    w.u32(sim.queue.next);
    // A queue holds far fewer than 4 billion changes.
    w.u32(sim.queue.pending.len() as u32);
    for q in &sim.queue.pending {
        w.u8(q.team as u8);
        w.u32(q.id.tick);
        w.u32(q.id.n);
        match &q.change {
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
                w.u8(patch.roles.len() as u8);
                for (squad, rd) in &patch.roles {
                    w.u8(*squad as u8);
                    w.u8(rd.role);
                    w.u8(rd.duty);
                }
            }
        }
    }
    for at in sim.queue.admitted {
        w.u8(u8::from(at.is_some()));
        w.u32(at.unwrap_or(0));
    }
    let referee = &sim.referee;
    match referee.phase {
        Phase::Live => w.u8(0),
        Phase::DeadBall(d) => {
            w.u8(1);
            // Kind and card indices are below 9.
            w.u8(d.kind.index() as u8);
            w.index(Some(d.team));
            w.v2(d.spot);
            w.u8(u8::from(d.direct));
            w.u32(d.since);
            w.u32(d.ready_at);
            w.index(Some(d.taker));
        }
        Phase::FullTime => w.u8(2),
    }
    w.u32(referee.offside);
    // At most one pending card per foul between two stoppages; far below 255.
    w.u8(referee.pending.len() as u8);
    for pending in &referee.pending {
        w.index(Some(pending.player));
        w.u8(Card::ALL
            .iter()
            .position(|c| *c == pending.card)
            .unwrap_or(0) as u8);
    }
    for count in referee.tally.kinds {
        w.u32(count);
    }
    w.u32(referee.tally.cards);
    w.u32(referee.clock.half);
    w.u32(referee.clock.half_start);
    for added in referee.clock.added_ticks {
        w.u8(u8::from(added.is_some()));
        w.u32(added.unwrap_or(0));
    }
    w.u8(u8::from(referee.abandoned));
    w.index(referee.extra_kick_off);
    match &referee.shootout {
        None => w.u8(0),
        Some(s) => {
            w.u8(1);
            for order in &s.order {
                // At most 11 kickers a team.
                w.u8(order.len() as u8);
                for &i in order {
                    w.index(Some(i));
                }
            }
            for keeper in s.keepers {
                w.index(Some(keeper));
            }
            for cursor in s.cursor {
                // A cursor grows by one a kick, far below 4 billion.
                w.u32(cursor as u32);
            }
            w.index(Some(s.first));
            w.u8(u8::from(s.end > 0.0));
            w.pair(s.scores);
            w.pair(s.taken);
            w.index(s.kicker);
            w.u8(u8::from(s.live_since.is_some()));
            w.u32(s.live_since.unwrap_or(0));
        }
    }
}

fn encode_summary(s: &Summary, w: &mut Writer) {
    w.u32(s.possession_changes);
    w.f64(s.ball_max_speed);
    w.u32(s.ball_idle_ticks);
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
        w.pair(pair);
    }
    w.u32(s.stoppages);
    w.u32(s.dead_ball_ticks);
    w.u32(s.offside_checks);
    w.pair(s.shots);
    w.pair(s.substitutions);
    w.pair(s.injuries);
    w.u32(s.changes_queued);
    w.u32(s.changes_applied);
    w.u32(s.changes_rejected);
    w.u32(s.ai_decisions);
    w.pair(s.shots_on_target);
    w.f64(s.xg[0]);
    w.f64(s.xg[1]);
    w.pair(s.passes);
    w.pair(s.passes_completed);
    w.pair(s.possession_ticks);
    w.pair(s.extra_added_s);
    w.u8(u8::from(s.extra_time));
    w.opt_pair(s.shootout);
    w.u32(s.shootout_kicks);
    w.u8(s
        .decided_by
        .and_then(|d| DecidedBy::ALL.iter().position(|x| *x == d))
        .map_or(0, |k| k as u8 + 1));
}

fn decode(sim: &mut Simulation, r: &mut Reader<'_>) -> Decoded<()> {
    let seed = r.u64()?;
    let minutes = r.u32()?;
    if seed != sim.config.seed || minutes != sim.config.minutes {
        return Err("configuration mismatch: another seed or match length");
    }
    for digest in sim.config.team_digests {
        if r.take(32)? != digest {
            return Err("team file mismatch: the team files differ from the ones the match used");
        }
    }
    sim.tick = r.u32()?;
    if r.bool()? != sim.config.knockout {
        return Err("configuration mismatch: a knockout match and a regular match");
    }
    sim.restart = r.bool()?;
    let rng_seed: [u8; 32] = r.take(32)?.try_into().expect("32 bytes");
    sim.rng = EngineRng::from_state(RngState {
        seed: rng_seed,
        stream: r.u64()?,
        word_pos: r.u128()?,
    });
    sim.ball.pos = r.v3()?;
    sim.ball.vel = r.v3()?;
    sim.carrier = r.index(PLAYERS)?;
    sim.control_since = r.u32()?;
    sim.last_touch = r.index(2)?;
    sim.keeper_beaten = r.bool()?;
    sim.summary = decode_summary(r)?;
    let schema = sim.config.tactics.clone();
    let tuning = sim.config.tuning.clone();
    for t in 0..2 {
        let team = &mut sim.teams[t];
        team.attack_x = r.f64()?;
        let mut formation = team.formation;
        for (slot, place) in formation.iter_mut().enumerate() {
            team.active[slot] = r.bool()?;
            *place = (r.f64()?, r.f64()?);
        }
        let tactics = r.tactics()?;
        if !tactics_in_range(&tactics, &schema) {
            return Err("malformed body: the tactics name an index the tactics file lacks");
        }
        team.set_tactics(tactics, &schema, &tuning);
        team.formation = formation;
        let squad = team.squad.len();
        for place in &mut team.lineup {
            *place = r.some_index(squad)?;
        }
        let bench = r.u8()?;
        team.bench = (0..bench)
            .map(|_| r.some_index(squad))
            .collect::<Decoded<Vec<_>>>()?;
        sim.ledgers[t] = SubLedger {
            used: r.u8()?,
            windows: r.u8()?,
            window_at: {
                let known = r.bool()?;
                let at = r.u32()?;
                known.then_some(at)
            },
        };
        sim.managers[t] = match r.u8()? {
            0 => Manager::Ai,
            1 => Manager::Human,
            _ => return Err("malformed body: an unknown manager"),
        };
        sim.ai[t] = AiState {
            trailing_acted: r.opt_pair()?,
            leading_acted: r.opt_pair()?,
            due: r.bool()?,
        };
    }
    for p in &mut sim.players {
        p.pos = r.v2()?;
        p.vel = r.v2()?;
        p.target = r.v2()?;
        p.facing = r.v2()?;
        p.status = match r.u8()? {
            0 => Status::OnPitch,
            1 => Status::SentOff,
            2 => Status::Injured,
            _ => return Err("malformed body: an unknown player status"),
        };
        p.yellow = r.u8()?;
        p.foul_ready = r.u32()?;
        let team = &sim.teams[p.team];
        let squad = r.some_index(team.squad.len())?;
        let entry = &team.squad[squad];
        p.squad = squad;
        p.shirt = entry.shirt;
        p.attributes = entry.attributes;
        p.base = entry.derived;
        p.energy = r.f64()?;
        p.derived = r.derived()?;
    }
    sim.queue.next = r.u32()?;
    let pending = r.u32()?;
    sim.queue.pending = Vec::new();
    for _ in 0..pending {
        let team = r.some_index(2)?;
        let id = ChangeId {
            tick: r.u32()?,
            n: r.u32()?,
        };
        let squad = sim.teams[team].squad.len();
        let change = match r.u8()? {
            0 => Change::Substitution {
                off: r.some_index(squad)?,
                on: r.some_index(squad)?,
            },
            1 => {
                let mut patch = TacticsPatch {
                    formation: r.opt_u8()?,
                    mentality: r.opt_u8()?,
                    ..TacticsPatch::default()
                };
                for level in &mut patch.instructions {
                    *level = r.opt_u8()?;
                }
                let roles = r.u8()?;
                for _ in 0..roles {
                    let s = r.some_index(squad)?;
                    patch.roles.push((
                        s,
                        RoleDuty {
                            role: r.u8()?,
                            duty: r.u8()?,
                        },
                    ));
                }
                Change::Tactics(patch)
            }
            _ => return Err("malformed body: an unknown change kind"),
        };
        sim.queue.pending.push(QueuedChange { id, team, change });
    }
    for at in &mut sim.queue.admitted {
        let known = r.bool()?;
        let tick = r.u32()?;
        *at = known.then_some(tick);
    }
    let referee = &mut sim.referee;
    referee.phase = match r.u8()? {
        0 => Phase::Live,
        1 => {
            let kind = StoppageKind::from_index(usize::from(r.u8()?))
                .ok_or("malformed body: an unknown restart kind")?;
            Phase::DeadBall(DeadBall {
                kind,
                team: r.some_index(2)?,
                spot: r.v2()?,
                direct: r.bool()?,
                since: r.u32()?,
                ready_at: r.u32()?,
                taker: r.some_index(PLAYERS)?,
            })
        }
        2 => Phase::FullTime,
        _ => return Err("malformed body: an unknown phase"),
    };
    referee.offside = r.u32()?;
    let pending = r.u8()?;
    referee.pending = (0..pending)
        .map(|_| {
            Ok(PendingCard {
                player: r.some_index(PLAYERS)?,
                card: *Card::ALL
                    .get(usize::from(r.u8()?))
                    .ok_or("malformed body: an unknown card")?,
            })
        })
        .collect::<Decoded<Vec<_>>>()?;
    let mut tally = Tally::default();
    for count in &mut tally.kinds {
        *count = r.u32()?;
    }
    tally.cards = r.u32()?;
    referee.tally = tally;
    referee.clock.half = r.u32()?;
    referee.clock.half_start = r.u32()?;
    for added in &mut referee.clock.added_ticks {
        let known = r.bool()?;
        let ticks = r.u32()?;
        *added = known.then_some(ticks);
    }
    referee.abandoned = r.bool()?;
    referee.extra_kick_off = r.index(2)?;
    referee.shootout = match r.u8()? {
        0 => None,
        1 => {
            let mut order: [Vec<usize>; 2] = [Vec::new(), Vec::new()];
            for side in &mut order {
                let len = r.u8()?;
                for _ in 0..len {
                    side.push(r.some_index(PLAYERS)?);
                }
            }
            let keepers = [r.some_index(PLAYERS)?, r.some_index(PLAYERS)?];
            let cursor = [r.u32()? as usize, r.u32()? as usize];
            let first = r.some_index(2)?;
            let end = if r.bool()? { 1.0 } else { -1.0 };
            let scores = r.pair()?;
            let taken = r.pair()?;
            let kicker = r.index(PLAYERS)?;
            let live = r.bool()?;
            let live_at = r.u32()?;
            Some(Shootout {
                order,
                keepers,
                cursor,
                first,
                end,
                scores,
                taken,
                kicker,
                live_since: live.then_some(live_at),
            })
        }
        _ => return Err("malformed body: an unknown shoot-out state"),
    };
    if r.at != r.buf.len() {
        return Err("malformed body: bytes left over");
    }
    Ok(())
}

fn decode_summary(r: &mut Reader<'_>) -> Decoded<Summary> {
    Ok(Summary {
        possession_changes: r.u32()?,
        ball_max_speed: r.f64()?,
        ball_idle_ticks: r.u32()?,
        goals: r.pair()?,
        fouls: r.pair()?,
        offsides: r.pair()?,
        corners: r.pair()?,
        throw_ins: r.pair()?,
        goal_kicks: r.pair()?,
        free_kicks: r.pair()?,
        penalties: r.pair()?,
        yellow: r.pair()?,
        red: r.pair()?,
        added_s: r.pair()?,
        stoppages: r.u32()?,
        dead_ball_ticks: r.u32()?,
        offside_checks: r.u32()?,
        shots: r.pair()?,
        substitutions: r.pair()?,
        injuries: r.pair()?,
        changes_queued: r.u32()?,
        changes_applied: r.u32()?,
        changes_rejected: r.u32()?,
        ai_decisions: r.u32()?,
        shots_on_target: r.pair()?,
        xg: [r.f64()?, r.f64()?],
        passes: r.pair()?,
        passes_completed: r.pair()?,
        possession_ticks: r.pair()?,
        extra_added_s: r.pair()?,
        extra_time: r.bool()?,
        shootout: r.opt_pair()?,
        shootout_kicks: r.u32()?,
        decided_by: match r.u8()? {
            0 => None,
            k => Some(
                *DecidedBy::ALL
                    .get(usize::from(k) - 1)
                    .ok_or("malformed body: an unknown decision")?,
            ),
        },
    })
}

/// `true` when every index of `t` lies inside the tactics file.
fn tactics_in_range(t: &Tactics, schema: &crate::data::tactics::TacticsSchema) -> bool {
    usize::from(t.formation) < schema.formations.len()
        && usize::from(t.mentality) < schema.mentalities.len()
        && t.instructions
            .iter()
            .enumerate()
            .all(|(i, l)| usize::from(*l) < schema.instructions.level_count(i))
        && t.roles.iter().all(|rd| {
            usize::from(rd.role) < schema.roles.len() && usize::from(rd.duty) < schema.duties.len()
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::test_support::shipped_config;

    fn played(ticks: u32) -> Simulation {
        let mut sim = Simulation::new(shipped_config(42, 90).unwrap()).unwrap();
        for _ in 0..ticks {
            sim.step();
        }
        sim
    }

    #[test]
    fn capture_write_read_capture_gives_identical_bytes() {
        let sim = played(20_000);
        let first = Snapshot::capture(&sim, [7; 16], 1_700_000_000_000);
        let bytes = first.to_bytes();
        let read = Snapshot::from_bytes(&bytes, "t.smsn").unwrap();
        assert_eq!(read, first);
        let rebuilt = Simulation::from_snapshot(shipped_config(42, 90).unwrap(), &read).unwrap();
        let again = Snapshot::capture(&rebuilt, [7; 16], 1_700_000_000_000);
        assert_eq!(again.to_bytes(), bytes);
        assert_eq!(read.seed(), 42);
        assert_eq!(read.minutes(), 90);
        assert_eq!(read.tick(), 20_000);
    }

    #[test]
    fn a_version_one_file_is_refused_naming_both_versions() {
        let mut bytes = Snapshot::capture(&played(10), [0; 16], 1).to_bytes();
        bytes[4..6].copy_from_slice(&1u16.to_le_bytes());
        let err = Snapshot::from_bytes(&bytes, "s.smsn").unwrap_err();
        assert!(
            err.to_string()
                .contains("unknown version 1; this build reads 5"),
            "{err}"
        );
    }

    #[test]
    fn another_seed_or_content_is_refused_by_name() {
        let snapshot = Snapshot::capture(&played(10), [0; 16], 1);
        let err = Simulation::from_snapshot(shipped_config(43, 90).unwrap(), &snapshot)
            .err()
            .unwrap();
        assert!(err.to_string().contains("configuration mismatch"), "{err}");
        let mut other = snapshot.clone();
        other.content_hash = "000000000000".into();
        let err = Simulation::from_snapshot(shipped_config(42, 90).unwrap(), &other)
            .err()
            .unwrap();
        assert!(err.to_string().contains("content mismatch"), "{err}");
    }

    #[test]
    fn shorten_for_log_hides_everything_outside_the_data_folder_or_cwd() {
        // Outside both the data folder and the working directory: only the file name
        // survives, so a user name in a home folder never reaches a log.
        let outside = if cfg!(windows) {
            r"C:\Users\alice\AppData\Local\Temp\x\empty.smsn"
        } else {
            "/home/alice/tmp/x/empty.smsn"
        };
        assert_eq!(shorten_for_log(outside, None), "empty.smsn");

        // Under the data folder: shortened to the path relative to it.
        let data_dir = if cfg!(windows) {
            r"C:\Users\alice\AppData\Local\SoccerManager"
        } else {
            "/home/alice/.local/share/SoccerManager"
        };
        let under = if cfg!(windows) {
            r"C:\Users\alice\AppData\Local\SoccerManager\matches\42\snapshot.smsn"
        } else {
            "/home/alice/.local/share/SoccerManager/matches/42/snapshot.smsn"
        };
        assert_eq!(
            shorten_for_log(under, Some(Path::new(data_dir))),
            "matches/42/snapshot.smsn"
        );

        // Already relative: passed through unchanged.
        assert_eq!(
            shorten_for_log("matches/42/snapshot.smsn", Some(Path::new(data_dir))),
            "matches/42/snapshot.smsn"
        );
    }

    #[test]
    fn write_failed_reason_carries_the_underlying_os_error() {
        // A destination that cannot be created (its parent is a plain file, not a folder)
        // fails with an io error; the signal's reason must keep the OS error text, not
        // just the top-level "io error" summary.
        let dir = std::env::temp_dir().join(format!(
            "engine-snapshot-write-failed-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let blocking_file = dir.join("blocked");
        std::fs::write(&blocking_file, b"x").unwrap();
        let path = blocking_file.join("snapshot.smsn");

        let snapshot = Snapshot::capture(&played(1), [0; 16], 1);
        let err = snapshot.write_atomic(&path).unwrap_err();
        let reason = full_reason(&err);
        let _ = std::fs::remove_dir_all(&dir);

        assert!(
            reason.len() > "io error".len() && reason.starts_with("io error"),
            "{reason}"
        );
    }
}
