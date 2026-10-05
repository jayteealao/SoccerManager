// The worker count of the browser suites, and their projects.
//
// Every test starts its own engine on a free port with its own data folder, so tests run in
// several workers. A test that measures frame rate or playback speed against the wall clock
// carries the tag @timing: other workers would load the machine and move its numbers, so it
// runs in the timing project, on one worker, after the others.
//
// The projects are the same whatever the count, so `--project=<name>` picks the same tests
// and a test keeps its id. SM_E2E_WORKERS sets the count; CI runs one worker. With more than
// one, the timing project is the teardown of the others, so it waits for them and still runs
// when one of them fails. Playwright runs a teardown project in full whatever the command line
// picks: `--project=viewer` would run every timing test, each shard of `--shard` would run
// every timing test, and `--last-failed` or `--only-changed` would run every timing test, or
// none when only timing tests are picked. A run with one of those flags therefore has one
// worker and no teardown: the timing project then runs last, as the last project of the
// config. A run narrowed to some files or tests (`-g`, a file name) still runs every timing
// test when the count is above one; set SM_E2E_WORKERS=1 for it.

/// The tag of a test that measures time.
export const TIMING = /@timing/;

/// Measured on the reference desktop (8 cores) with the match suite: 1 worker 624 s, 2 workers
/// 421 s with the same results; 3 and 4 workers save under a minute more, and the load fails
/// tests that wait on the background matchday or a paused page clock. With the four window-size
/// projects (455 tests, 2 workers) the match suite took 20.2 minutes, the shell suite 0.1 and
/// the engine suite (playwright.config.mjs) 19.1.
const DEFAULT_WORKERS = 2;

/// The command-line flags that pick tests in a way a teardown project ignores.
const PICKING_FLAGS = ['--project', '--last-failed', '--shard', '--only-changed'];

function picksTests(argv) {
  return argv.some((arg) => PICKING_FLAGS.some((flag) => arg === flag || arg.startsWith(`${flag}=`)));
}

export function workerCount() {
  if (picksTests(process.argv)) {
    // The worker processes load this config again with their own command line; the variable
    // carries the count to them, so they build the same projects.
    process.env.SM_E2E_WORKERS = '1';
    return 1;
  }
  const set = Number.parseInt(process.env.SM_E2E_WORKERS ?? '', 10);
  if (Number.isInteger(set) && set > 0) {
    return set;
  }
  return process.env.CI ? 1 : DEFAULT_WORKERS;
}

/// The window-size projects of the match suite: each runs only the tests tagged @sizes. The
/// 1920 project compares pixels against its own baselines; the layout projects run the layout
/// check. `metadata.mode` tells `snap()` which.
export const SIZES = [
  { name: 'chromium-1920', grep: /@sizes/, use: { viewport: { width: 1920, height: 1080 } }, metadata: { mode: 'pixels' } },
  { name: 'layout-768x1024', grep: /@sizes/, use: { viewport: { width: 768, height: 1024 } }, metadata: { mode: 'layout' } },
  { name: 'layout-1024x640', grep: /@sizes/, use: { viewport: { width: 1024, height: 640 } }, metadata: { mode: 'layout' } },
  { name: 'layout-2560x1440', grep: /@sizes/, use: { viewport: { width: 2560, height: 1440 } }, metadata: { mode: 'layout' } },
];

/// The projects of a suite: each of `own` without its @timing tests (a project that carries
/// its own grep, as the size projects do, keeps it), then `timing` with the @timing tests of
/// every file, on one worker. With more than one worker, `timing` is the teardown of the
/// others.
export function suiteProjects(own, workers) {
  const teardown = workers > 1 ? { teardown: 'timing' } : {};
  return [
    ...own.map((project) => ({ ...project, grepInvert: TIMING, ...teardown })),
    { name: 'timing', grep: TIMING, workers: 1 },
  ];
}
