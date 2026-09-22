// The page: socket in, pitch out, controls between them.

import { COMPONENT_COUNT, decodeInto, newFrame } from './decode.mjs';
import { History } from './history.mjs';
import { between } from './interpolate.mjs';
import { colours as markColours, drawMark, setFavicon } from './mark.mjs';
import { Pitch } from './pitch.mjs';
import { Playback, SPEEDS } from './playback.mjs';
import { Scheduler, TICKS_PER_SECOND } from './schedule.mjs';
import { signal, signals } from './signal.mjs';
import { MatchSocket, socketAddress } from './socket.mjs';
import { Stoppages, stopsPlay } from './stoppages.mjs';

const el = (id) => document.getElementById(id);

const scheduler = new Scheduler();
const stoppages = new Stoppages();
const rendered = new Int16Array(COMPONENT_COUNT);
const earlier = new Int16Array(COMPONENT_COUNT);
const later = new Int16Array(COMPONENT_COUNT);

let history = null;
let pitch = null;
let socket = null;
let playback = null;
let renderedTick = 0;
let scrubbing = false;
let resumeAfterScrub = true;
let lastRewind = null;

/// The clock, as a manager reads it: minutes and seconds of match time.
function clockText(tick) {
  const seconds = Math.floor(tick / TICKS_PER_SECOND);
  return `${String(Math.floor(seconds / 60)).padStart(2, '0')}:${String(seconds % 60).padStart(2, '0')}`;
}

function announce(text) {
  el('live').textContent = text;
}

/// One notice element, two kinds, and the state word always says which. A colour on its
/// own never carries the difference between "the engine is slow" and "the stream ended".
function showNotice(text, kind = 'lag') {
  const notice = el('notice');
  const shown = Boolean(text);
  notice.dataset.shown = shown ? 'true' : 'false';
  notice.dataset.kind = kind;
  notice.setAttribute('aria-hidden', shown ? 'false' : 'true');
  el('notice-word').textContent = shown ? (kind === 'error' ? 'Stream ended' : 'Lag') : '';
  el('notice-text').textContent = text ?? '';
}

function setSpeedButtons(requested, effective) {
  for (const speed of SPEEDS) {
    el(`speed-${speed}`).setAttribute('aria-pressed', String(speed === requested));
  }
  el('speed-effective').textContent = `${effective}x`;
  announce(
    requested === effective
      ? `Speed ${requested} times real time.`
      : `Speed ${requested} times asked for. Playing at ${effective} times.`
  );
}

function start(hello) {
  history = new History(hello.ticks_expected);
  pitch = new Pitch(el('pitch'), [
    { primary: hello.teams[0]['team.kit.primary'], secondary: hello.teams[0]['team.kit.secondary'] },
    { primary: hello.teams[1]['team.kit.primary'], secondary: hello.teams[1]['team.kit.secondary'] },
  ]);
  el('teams').textContent = `${hello.teams[0]['team.name']} v ${hello.teams[1]['team.name']}`;
  el('engine-version').textContent = `engine ${hello['engine.version']}`;
  el('scrub').max = String(hello.ticks_expected);
  playback = new Playback({
    scheduler,
    onSpeed: setSpeedButtons,
    onNotice: showNotice,
  });
  playback.select(1);
}

let previous = newFrame();
let incoming = newFrame();

function onTick(buffer) {
  const result = decodeInto(buffer, previous, incoming);
  if (!result) {
    return;
  }
  history.append(incoming.tick, incoming.components);
  socket.noteTick(incoming.tick);
  playback.noteArrival(performance.now(), incoming.tick);
  if (result.kind !== 'delta') {
    history.report();
    history.measurePage();
    if (result.kind === 'restart') {
      stoppages.add(incoming.tick);
    }
  }
  if (!scrubbing) {
    el('scrub').value = String(incoming.tick);
  }
  [previous, incoming] = [incoming, previous];
  revealPitch();
}

/// Cross-fades the skeleton away on the first tick. `hidden` alone would snap, because
/// `display` is not animatable.
let revealed = false;
function revealPitch() {
  if (revealed) {
    return;
  }
  revealed = true;
  const skeleton = el('skeleton');
  skeleton.dataset.leaving = 'true';
  setTimeout(() => {
    skeleton.hidden = true;
  }, 250);
}

function onMessage(message) {
  if (stopsPlay(message)) {
    stoppages.add(message.tick);
  }
  // `ticks_expected` is the most ticks the match can last. Full time marks the real end,
  // so the scrubber stops at the last tick that arrived.
  if (message.type === 'event' && message['event.type'] === 'full-time' && history) {
    el('scrub').max = String(history.newestTick);
  }
}

function frame(timestamp) {
  requestAnimationFrame(frame);
  if (!history || history.count === 0) {
    return;
  }
  const step = scheduler.advance(timestamp, history.newestTick, history.firstTick);
  if (!history.tickAt(step.from, earlier)) {
    return;
  }
  if (!history.tickAt(step.to, later)) {
    later.set(earlier);
  }
  between(earlier, later, step.fraction, rendered);
  pitch.draw(rendered);
  renderedTick = step.from;
  el('clock').textContent = clockText(step.from);
  el('gauge').textContent = gaugeText();
}

function gaugeText() {
  const budget = scheduler.budget();
  const megabytes = Math.round(history.bytes() / 1_000_000);
  // `refresh_hz` rides beside the frame rate everywhere the frame rate appears. A panel
  // running at 144 or at 32 makes a bare frame count meaningless on its own.
  return `${budget.fps_median} fps · ${budget.refresh_hz} Hz · ${megabytes} MB`;
}

/// A rewind draws the stored tick itself, not an interpolation towards it.
///
/// `exact` is measured, not assumed: the drawn frame is compared against the stored tick
/// component by component. A rewind that quietly drew an interpolation would satisfy an
/// `exact` flag that only asked whether the tick was in the history.
function rewind(tick) {
  const from = renderedTick;
  scheduler.seek(tick);
  pitch.clearTrail();
  const stored = history.tickAt(tick, earlier);
  let exact = false;
  if (stored) {
    between(earlier, earlier, 0, rendered);
    pitch.draw(rendered);
    renderedTick = tick;
    el('clock').textContent = clockText(tick);
    exact = rendered.every((value, i) => value === earlier[i]);
    lastRewind = { tick, drawn: Array.from(rendered), stored: Array.from(earlier), exact };
  }
  signal('viewer.rewind', { from_tick: from, to_tick: tick, stored, exact });
  announce(`Rewound to ${clockText(tick)}.`);
}

function wire() {
  el('play').addEventListener('click', () => {
    const playing = el('play').dataset.playing !== 'true';
    scheduler.setPlaying(playing);
    el('play').dataset.playing = String(playing);
    el('play').textContent = playing ? 'Pause' : 'Play';
    announce(playing ? 'Playing.' : 'Paused.');
  });

  for (const speed of SPEEDS) {
    el(`speed-${speed}`).addEventListener('click', () => playback.select(speed));
  }

  el('skip').addEventListener('click', () => {
    const next = stoppages.next(renderedTick);
    if (next === null) {
      announce('No later stoppage.');
      return;
    }
    rewind(next);
  });

  // A scrub holds the frame it lands on. Playback resumes when the drag ends, so the
  // pitch shows the stored tick for as long as the manager is looking at it.
  const scrub = el('scrub');
  scrub.addEventListener('input', () => {
    if (!scrubbing) {
      scrubbing = true;
      resumeAfterScrub = scheduler.playing;
      scheduler.setPlaying(false);
    }
    rewind(Number(scrub.value));
  });
  scrub.addEventListener('change', () => {
    scrubbing = false;
    scheduler.setPlaying(resumeAfterScrub);
  });
}

/// The read-only test hook. Three acceptance criteria read it, and it exposes no setter:
/// a hook that can change the page is a hook that can hide a fault.
function hook() {
  globalThis.__touchline = {
    lastRendered: () => Array.from(rendered),
    lastRenderedTick: () => renderedTick,
    signals,
    history: () => ({
      ticks_stored: history ? history.count : 0,
      history_bytes: history ? history.bytes() : 0,
      page_bytes: history ? history.pageBytes : null,
      page_bytes_reason: history ? history.pageBytesReason : 'no history yet',
      newest_tick: history ? history.newestTick : 0,
    }),
    frame: () => scheduler.budget(),
    lastRewind: () => lastRewind,
    stoppageAt: (tick) => stoppages.next(tick),
    tickAt: (tick) => {
      const out = new Int16Array(COMPONENT_COUNT);
      return history && history.tickAt(tick, out) ? Array.from(out) : null;
    },
  };
}

async function main() {
  // The mark is drawn from the live token values, never loaded from an image file.
  const mark = markColours(document);
  setFavicon(document, 32, mark);
  drawMark(el('mark').getContext('2d'), 0, 0, 28, mark);

  wire();
  hook();
  requestAnimationFrame(frame);

  const engine = await fetch('engine.json').then((r) => r.json());
  socket = new MatchSocket(socketAddress(engine['socket.port'], engine['protocol.version']), {
    onHello: start,
    onTick,
    onMessage,
  });
  socket.onClose = () => {
    // The canvas error state, stubbed here and completed in a later version: the panel and
    // the text, with no recovery action yet.
    showNotice('The match is no longer live. Start the engine again to watch another.', 'error');
    announce('The stream ended. The match is no longer live.');
  };
}

main();
