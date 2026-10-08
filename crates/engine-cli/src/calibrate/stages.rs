//! The time and memory of each stage of a calibration run: play, checks, commentary,
//! writing, disk and judge, beside the run's total.
//!
//! A thread enters a stage with [`enter`]; the guard adds the time spent to the stage when
//! it drops. Time is thread time summed over the threads, so on many threads a stage can
//! take longer than the run. The rule checker runs inside the match, so its time is
//! estimated from every 16th call, timed and scaled, and moved from play to checks.
//!
//! Memory comes from [`CountingAlloc`], the program's allocator: each thread counts the heap
//! bytes it holds, and a stage's figure is the largest growth of one thread's heap while it
//! was in the stage. Nothing here feeds back into a result.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64, Ordering};
use std::time::{Duration, Instant};

/// A stage of a run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Stage {
    Play,
    Checks,
    Commentary,
    Writing,
    Disk,
    Judge,
}

impl Stage {
    pub const ALL: [Stage; 6] = [
        Stage::Play,
        Stage::Checks,
        Stage::Commentary,
        Stage::Writing,
        Stage::Disk,
        Stage::Judge,
    ];

    pub fn code(self) -> &'static str {
        match self {
            Stage::Play => "play",
            Stage::Checks => "checks",
            Stage::Commentary => "commentary",
            Stage::Writing => "writing",
            Stage::Disk => "disk",
            Stage::Judge => "judge",
        }
    }
}

const NONE: u8 = u8::MAX;

thread_local! {
    /// The heap bytes this thread holds: allocated minus freed on this thread.
    static LIVE: Cell<i64> = const { Cell::new(0) };
    /// The stage this thread is in, or [`NONE`].
    static TAG: Cell<u8> = const { Cell::new(NONE) };
    /// [`LIVE`] when the thread entered its stage, and the largest growth since.
    static BASE: Cell<i64> = const { Cell::new(0) };
    static PEAK: Cell<i64> = const { Cell::new(0) };
}

static NANOS: [AtomicU64; 6] = [const { AtomicU64::new(0) }; 6];
static PEAK_BYTES: [AtomicI64; 6] = [const { AtomicI64::new(0) }; 6];

/// The program's allocator: the system allocator, counting each thread's live heap bytes
/// once [`enable`] has run. Until then (every command but `calibrate`) an allocation pays
/// one relaxed load and counts nothing.
pub struct CountingAlloc;

/// Whether [`CountingAlloc`] counts: set by `calibrate`, the one command with stage costs.
static COUNTING: AtomicBool = AtomicBool::new(false);

/// Turns the heap counting on for the rest of the process.
pub fn enable() {
    COUNTING.store(true, Ordering::Relaxed);
}

fn counted(delta: i64) {
    if !COUNTING.load(Ordering::Relaxed) {
        return;
    }
    // The thread-locals hold plain numbers with no destructor, so they are always there;
    // `try_with` keeps an allocation during thread teardown from panicking all the same.
    let _ = LIVE.try_with(|live| {
        let now = live.get() + delta;
        live.set(now);
        if delta > 0 {
            let _ = BASE.try_with(|base| {
                let _ = PEAK.try_with(|peak| peak.set(peak.get().max(now - base.get())));
            });
        }
    });
}

// SAFETY: every call goes straight to the system allocator with the caller's layout and
// pointer; the counting touches only `Cell`s of plain numbers and never allocates.
// nosemgrep: rust.lang.security.unsafe-usage.unsafe-usage -- a counting wrapper over the system allocator; see SAFETY above
unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: the caller's contract for `alloc` is passed on unchanged.
        // nosemgrep: rust.lang.security.unsafe-usage.unsafe-usage -- forwards to the system allocator; see SAFETY above
        let p = unsafe { System.alloc(layout) };
        if !p.is_null() {
            counted(layout.size() as i64);
        }
        p
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: the caller's contract for `dealloc` is passed on unchanged.
        // nosemgrep: rust.lang.security.unsafe-usage.unsafe-usage -- forwards to the system allocator; see SAFETY above
        unsafe { System.dealloc(ptr, layout) };
        counted(-(layout.size() as i64));
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // SAFETY: the caller's contract for `alloc_zeroed` is passed on unchanged.
        // nosemgrep: rust.lang.security.unsafe-usage.unsafe-usage -- forwards to the system allocator; see SAFETY above
        let p = unsafe { System.alloc_zeroed(layout) };
        if !p.is_null() {
            counted(layout.size() as i64);
        }
        p
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // SAFETY: the caller's contract for `realloc` is passed on unchanged.
        // nosemgrep: rust.lang.security.unsafe-usage.unsafe-usage -- forwards to the system allocator; see SAFETY above
        let p = unsafe { System.realloc(ptr, layout, new_size) };
        if !p.is_null() {
            counted(new_size as i64 - layout.size() as i64);
        }
        p
    }
}

/// A thread in a stage; leaving it adds the time and the heap growth to the stage.
pub struct Span {
    stage: Stage,
    started: Instant,
    outer: (u8, i64, i64),
}

/// Enters `stage` on this thread until the guard drops. A stage entered inside another
/// pauses nothing: the outer stage's time includes it.
pub fn enter(stage: Stage) -> Span {
    let live = LIVE.with(Cell::get);
    let outer = (
        TAG.with(|t| t.replace(stage as u8)),
        BASE.with(|b| b.replace(live)),
        PEAK.with(|p| p.replace(0)),
    );
    Span {
        stage,
        started: Instant::now(),
        outer,
    }
}

impl Drop for Span {
    fn drop(&mut self) {
        add(self.stage, self.started.elapsed());
        let peak = PEAK.with(Cell::get);
        PEAK_BYTES[self.stage as usize].fetch_max(peak, Ordering::Relaxed);
        let (tag, base, outer_peak) = self.outer;
        TAG.with(|t| t.set(tag));
        // The outer stage's growth includes this one's, measured from its own start.
        let inner_base = BASE.with(|b| b.replace(base));
        PEAK.with(|p| p.set(outer_peak.max(peak + inner_base - base)));
    }
}

/// Adds `d` to `stage`'s time.
pub fn add(stage: Stage, d: Duration) {
    let nanos = u64::try_from(d.as_nanos()).unwrap_or(u64::MAX);
    NANOS[stage as usize].fetch_add(nanos, Ordering::Relaxed);
}

/// Moves `d` of time from `from` to `to`: the checker's estimated time out of play.
pub fn shift(from: Stage, to: Stage, d: Duration) {
    let nanos = u64::try_from(d.as_nanos()).unwrap_or(u64::MAX);
    let _ = NANOS[from as usize].fetch_update(Ordering::Relaxed, Ordering::Relaxed, |v| {
        Some(v.saturating_sub(nanos))
    });
    NANOS[to as usize].fetch_add(nanos, Ordering::Relaxed);
}

/// One stage's cost in a report.
pub use crate::report::StageCost as Cost;

/// Every stage's cost so far, by stage code, with the run's total.
pub fn snapshot(total: Duration) -> BTreeMap<String, Cost> {
    let mb = |bytes: i64| (bytes.max(0) as f64 / (1024.0 * 1024.0) * 1000.0).round() / 1000.0;
    let mut out: BTreeMap<String, Cost> = Stage::ALL
        .iter()
        .map(|&s| {
            let i = s as usize;
            (
                s.code().to_string(),
                Cost {
                    ms: NANOS[i].load(Ordering::Relaxed) / 1_000_000,
                    peak_mb: Some(mb(PEAK_BYTES[i].load(Ordering::Relaxed))),
                },
            )
        })
        .collect();
    out.insert(
        "total".to_string(),
        Cost {
            ms: u64::try_from(total.as_millis()).unwrap_or(u64::MAX),
            peak_mb: engine::observe::process::peak_memory_mb()
                .map(|m| (m * 1000.0).round() / 1000.0),
        },
    );
    out
}

/// The console table of a run's stage costs, in stage order, then the total.
pub fn render_table(costs: &BTreeMap<String, Cost>) -> String {
    let mut out = String::from("stage            time ms    peak MB\n");
    let order = Stage::ALL.iter().map(|s| s.code()).chain(["total"]);
    for code in order {
        if let Some(c) = costs.get(code) {
            let mb = c
                .peak_mb
                .map_or_else(|| "-".to_string(), |m| format!("{m:.3}"));
            out.push_str(&format!("{code:<12} {:>11} {mb:>10}\n", c.ms));
        }
    }
    out.push_str(
        "(thread time summed over threads; checks estimated from every 16th checker call)\n",
    );
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    // The counters are process-wide and the tests share a process, so each test checks
    // only that its own stage grew.
    #[test]
    fn a_stage_counts_its_time_and_the_heap_it_grows_and_restores_the_outer_stage() {
        enable();
        let before = snapshot(Duration::ZERO);
        {
            let _span = enter(Stage::Commentary);
            let held: Vec<u8> = vec![7; 4 << 20];
            std::hint::black_box(&held);
            {
                let _inner = enter(Stage::Disk);
                let more: Vec<u8> = vec![1; 1 << 20];
                std::hint::black_box(&more);
                assert_eq!(TAG.with(Cell::get), Stage::Disk as u8);
            }
            assert_eq!(TAG.with(Cell::get), Stage::Commentary as u8);
            // The outer stage holds its own 4 MiB and the inner 1 MiB.
            assert!(PEAK.with(Cell::get) >= 5 << 20, "{}", PEAK.with(Cell::get));
            std::thread::sleep(Duration::from_millis(2));
        }
        assert_eq!(TAG.with(Cell::get), NONE);
        let after = snapshot(Duration::from_millis(5));
        assert!(after["commentary"].peak_mb.unwrap() >= 5.0);
        assert!(after["disk"].peak_mb.unwrap() >= 1.0);
        assert!(after["commentary"].ms >= before["commentary"].ms + 2);
        assert_eq!(after["total"].ms, 5);
    }

    #[test]
    fn a_shift_moves_time_between_stages() {
        add(Stage::Play, Duration::from_millis(50));
        let before = snapshot(Duration::ZERO);
        shift(Stage::Play, Stage::Checks, Duration::from_millis(20));
        let after = snapshot(Duration::ZERO);
        // Other tests of this process may add to either stage meanwhile.
        assert!(after["checks"].ms >= before["checks"].ms + 20);
        let table = render_table(&after);
        for code in [
            "play",
            "checks",
            "commentary",
            "writing",
            "disk",
            "judge",
            "total",
        ] {
            assert!(table.contains(code), "{table}");
        }
    }
}
