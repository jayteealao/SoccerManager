// Measures web/decode.mjs decodeInto over 10,000 recorded tick frames from fixture.smfx.
// Frames are handed over as standalone ArrayBuffers, as a WebSocket binaryType='arraybuffer'
// message delivers them. A second pass hands Uint8Arrays (as the unit test does) for contrast.
import { pathToFileURL } from 'node:url';
import { performance } from 'node:perf_hooks';
const WEB = 'C:/Users/jayte/Documents/dev/SoccerManager/web/';
const { decodeInto, newFrame, KIND_KEYFRAME, KIND_RESTART } = await import(pathToFileURL(WEB + 'decode.mjs').href);
const { readFixture, quiet } = await import(pathToFileURL(WEB + 'tests/helpers.mjs').href);

const N = 10000;
const { header, entries } = readFixture(N * 2);
let frames = entries.filter((e) => e.bytes);
const first = frames.findIndex((e) => e.bytes[0] === KIND_KEYFRAME || e.bytes[0] === KIND_RESTART);
frames = frames.slice(first, first + N);
if (frames.length < N) throw new Error(`only ${frames.length} frames`);
const buffers = frames.map((e) => e.bytes.slice().buffer);
const arrays = frames.map((e) => e.bytes);
const kinds = {};
for (const e of frames) kinds[e.bytes[0]] = (kinds[e.bytes[0]] ?? 0) + 1;
console.log(`fixture protocol=${header.protocolVersion} seed=${header.seed} frames=${frames.length} kinds=${JSON.stringify(kinds)} node=${process.version}`);

function run(input) {
  let prev = newFrame(), out = newFrame(), failed = 0;
  const t0 = performance.now();
  for (let i = 0; i < input.length; i += 1) {
    if (!decodeInto(input[i], prev, out)) failed += 1;
    const s = prev; prev = out; out = s;
  }
  const us = ((performance.now() - t0) * 1000) / input.length;
  return { us, failed, lastTick: prev.tick };
}
quiet(() => { run(buffers); run(arrays); }); // warm-up, not reported
for (const [label, input] of [['ArrayBuffer (wire shape)', buffers], ['Uint8Array (test shape)', arrays]]) {
  const reps = [];
  for (let r = 0; r < 5; r += 1) {
    const res = quiet(() => run(input));
    reps.push(res.us);
    console.log(`${label} rep ${r + 1}: mean ${res.us.toFixed(3)} us/frame, failed=${res.failed}, lastTick=${res.lastTick}`);
  }
  const mean = reps.reduce((a, b) => a + b, 0) / reps.length;
  console.log(`${label} overall mean ${mean.toFixed(3)} us/frame; budget < 25 us: ${mean < 25 ? 'PASS' : 'FAIL'}`);
}
