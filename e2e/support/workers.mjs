// The worker count of the browser suites, and the project that runs the timing tests alone.
//
// Every test starts its own engine on a free port with its own data folder, so tests run in
// several workers. A test that measures frame rate or playback speed against the wall clock
// carries the tag @timing: other workers would load the machine and move its numbers, so with
// more than one worker it runs in the timing project, one test at a time, after the others.
//
// SM_E2E_WORKERS sets the count; CI runs one worker. Playwright runs a project that follows
// another (its teardown) in full, so a run narrowed to some tests also runs every timing test
// when the count is above one. Set SM_E2E_WORKERS=1 for a narrowed run: the projects are then
// as they were, and the timing tests run in order with the rest.

/// The tag of a test that measures time.
export const TIMING = /@timing/;

/// Measured on the reference desktop (8 cores) with the match suite: 1 worker 624 s, 2 workers
/// 421 s with the same results; 3 and 4 workers save under a minute more, and the load fails
/// tests that wait on the background matchday or a paused page clock.
const DEFAULT_WORKERS = 2;

export function workerCount() {
  const set = Number.parseInt(process.env.SM_E2E_WORKERS ?? '', 10);
  if (Number.isInteger(set) && set > 0) {
    return set;
  }
  return process.env.CI ? 1 : DEFAULT_WORKERS;
}

/// The project of the @timing tests: one worker, after every other project of the run.
export function timingProject(extra = {}) {
  return { name: 'timing', grep: TIMING, workers: 1, ...extra };
}
