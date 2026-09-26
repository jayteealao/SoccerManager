// What the page says when the match is not playing: the loading steps while the engine
// starts, the first-run panel when there is no engine, and the error panel when the engine
// or the connection stops. Pure: no DOM, so every state is tested without a browser.

import { TICKS_PER_SECOND } from './schedule.mjs';

/// The loading steps, in order. Each shows a state word, never a spinner.
export const STEPS = Object.freeze([
  { id: 'engine', label: 'Starting the engine' },
  { id: 'connect', label: 'Connecting to the match' },
  { id: 'kickoff', label: 'Waiting for kick-off' },
]);

/// The state words a step can carry.
export const STEP_WORDS = Object.freeze({ done: 'Done', active: 'In progress', waiting: 'Waiting' });

/// The steps with their state words, for the step that is in progress (0 to 2), or 3 once
/// every step is done.
export function loadingSteps(current) {
  return STEPS.map((step, i) => {
    const state = i < current ? 'done' : i === current ? 'active' : 'waiting';
    return { ...step, state, word: STEP_WORDS[state] };
  });
}

/// `mm:ss` of match time at `tick`.
export function clockAt(tick) {
  const seconds = Math.floor(Math.max(0, tick) / TICKS_PER_SECOND);
  return `${String(Math.floor(seconds / 60)).padStart(2, '0')}:${String(seconds % 60).padStart(2, '0')}`;
}

/// The engine's own refusal of a snapshot reads `snapshot refused: <path>: <reason>`
/// (`crates/engine/src/error.rs`). The manager needs the reason, not the path.
export function refusalReason(text) {
  const message = String(text ?? '').trim();
  const prefix = 'snapshot refused: ';
  if (!message.startsWith(prefix)) {
    return message;
  }
  const rest = message.slice(prefix.length);
  const split = rest.indexOf(': ');
  return split < 0 ? rest : rest.slice(split + 2);
}

/// The instruction the first-run panel gives.
export const BUILD_INSTRUCTION =
  'Build it with "cargo build --release -p engine-cli", then start the launcher again, or point it at the program with --engine.';

/// The line an error panel adds when no launcher is running to restart the engine.
export const NO_LAUNCHER_HINT = 'Start with engine-cli launch to restart from the last stoppage.';

/// The panel for an engine status (`engine.json`), or null when play can go on. `launcher`
/// is false when the page was served by the engine itself, or when nothing answered.
export function panelModel(status) {
  if (!status) {
    return {
      kind: 'crashed',
      word: 'Error',
      title: 'The engine stopped',
      body: 'The connection closed and nothing answered for the engine.',
      hint: NO_LAUNCHER_HINT,
      actions: ['abandon'],
    };
  }
  const state = status['engine.state'];
  const launcher = status.launcher === true;
  switch (state) {
    case 'not-found':
      return {
        kind: 'first-run',
        word: 'Setup',
        title: 'Touchline could not find the match engine',
        body: 'The launcher looked for the engine program here:',
        path: status['engine.path'] ?? '',
        instruction: BUILD_INSTRUCTION,
        actions: ['open-replay'],
      };
    case 'crashed': {
      const code = status['engine.code'];
      const tick = status['snapshot.tick'];
      return {
        kind: 'crashed',
        word: 'Error',
        title: code === null || code === undefined
          ? 'The engine stopped'
          : `The engine stopped (exit code ${code})`,
        body:
          tick === null || tick === undefined
            ? 'No stoppage was saved before it stopped.'
            : `The match was saved at the stoppage at ${clockAt(tick)}.`,
        restartLabel:
          tick === null || tick === undefined ? 'Restart' : `Restart from ${clockAt(tick)}`,
        hint: launcher ? null : NO_LAUNCHER_HINT,
        actions: launcher ? ['restart', 'abandon'] : ['abandon'],
      };
    }
    case 'refused':
      return {
        kind: 'refused',
        word: 'Error',
        title: `The saved match could not be read: ${refusalReason(status['engine.reason'])}`,
        body: 'The match cannot continue. Abandon it to keep what was played.',
        actions: ['abandon'],
      };
    case 'abandoned':
      return {
        kind: 'abandoned',
        word: 'Abandoned',
        title: 'The match was abandoned',
        body: 'What was played stays on the pitch and plays back.',
        actions: ['save-replay', 'open-replay'],
      };
    default:
      return null;
  }
}

/// Milliseconds before reconnect attempt `attempt` (0 first): 250 ms, doubling, capped at
/// four seconds.
export function backoff(attempt) {
  return Math.min(4000, 250 * 2 ** Math.max(0, attempt));
}
