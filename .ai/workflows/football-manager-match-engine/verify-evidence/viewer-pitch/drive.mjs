// Verify drive for viewer-pitch, headless Edge over CDP.
// Usage: node drive.mjs <scenario>   scenario = main | lag | nolag | live
import { spawn } from 'node:child_process';
import { writeFileSync, mkdirSync } from 'node:fs';
import { launch, waitFor, sleep } from './cdp.mjs';

const REPO = 'C:/Users/jayte/Documents/dev/SoccerManager';
const EV = `${REPO}/.ai/workflows/football-manager-match-engine/verify-evidence/viewer-pitch`;
const SCRATCH = 'C:/Users/jayte/AppData/Local/Temp/claude/C--Users-jayte-Documents-dev-SoccerManager/049d44ec-4e9f-4612-a5ae-6c0af975bcdd/scratchpad';
mkdirSync(EV, { recursive: true });
const scenario = process.argv[2];
const out = { scenario, started: new Date().toISOString(), steps: [] };
const log = (name, data) => { out.steps.push({ name, at: new Date().toISOString(), ...data }); console.log(name, JSON.stringify(data)); };

function startServer(args, logName) {
  return new Promise((res, rej) => {
    const p = spawn(`${REPO}/target/release/engine-cli.exe`, args, { cwd: REPO });
    let text = '';
    const onData = (d) => {
      text += d.toString();
      const m = text.match(/http:\/\/127\.0\.0\.1:\d+\//);
      if (m && !p.url) { p.url = m[0]; res(p); }
    };
    p.stdout.on('data', onData);
    p.stderr.on('data', onData);
    p.on('exit', (code) => { p.exitCode2 = code; writeFileSync(`${EV}/${logName}`, text + `\nexit ${code}\n`); if (!p.url) rej(new Error(text)); });
    p.log = () => text;
  });
}

const HOOK = `(() => {
  window.__skips = 0; window.__sig = [];
  const oi = console.info.bind(console);
  console.info = (m) => { if (typeof m === 'string' && m.includes('"record.kind":"viewer-event"') && !m.includes('"viewer.history"')) { __sig.push(m); if (m.includes('"viewer.tick_skipped"')) __skips += JSON.parse(m).skipped; } oi(m); };
  window.__clicks = [];
  for (const t of ['click','keydown']) document.addEventListener(t, e => __clicks.push({t, target: e.target.id || e.target.tagName, trusted: e.isTrusted, at: Math.round(performance.now())}), true);
  return true; })()`;

const READ = `({
  now: performance.now(),
  tick: __touchline.lastRenderedTick(),
  clock: document.getElementById('clock').textContent,
  speed: document.querySelector('[aria-pressed=true][id^=speed-]')?.id,
  effective: document.getElementById('speed-effective').textContent,
  notice: document.getElementById('notice').textContent.replace(/\\s+/g,' ').trim(),
  live: document.getElementById('live').textContent,
  gauge: document.getElementById('gauge').textContent,
  vis: document.visibilityState,
  skipped_total: window.__skips,
  frame: __touchline.frame(),
  history: __touchline.history(),
})`;

async function openPage(cdp, url) {
  await cdp.goto(url);
  await waitFor(cdp, 'globalThis.__touchline !== undefined', 10000);
  await cdp.eval(HOOK);
  await waitFor(cdp, '__touchline.lastRenderedTick() > 0', 20000);
}

async function rate(cdp, seconds) {
  const a = await cdp.eval(READ);
  await sleep(seconds * 1000);
  const b = await cdp.eval(READ);
  const wall = (b.now - a.now) / 1000;
  const match = (b.tick - a.tick) / 50;
  return { a: { tick: a.tick, clock: a.clock, now: a.now }, b: { tick: b.tick, clock: b.clock, now: b.now }, wall_s: wall, match_s: match, ratio: match / wall, speed: b.speed, effective: b.effective };
}

const cdp = await launch({ profile: `${SCRATCH}/edge-profile-${scenario}`, port: 9333 + ['main', 'lag', 'nolag', 'live', 'probe', 'instr', 'replayend'].indexOf(scenario) });
cdp.on((m) => { if (m.method === 'Runtime.exceptionThrown') log('page-exception', { text: m.params.exceptionDetails?.exception?.description }); });
let server;
try {
  if (scenario === 'probe') {
    server = await startServer(['replay', '--fixture', 'fixture.smfx', '--speed', '8', '--web', 'web'], 'server-probe.log');
    await openPage(cdp, server.url);
    await sleep(6000);
    log('probe', await cdp.eval(READ));
    log('ua', { ua: await cdp.eval('navigator.userAgent'), iso: await cdp.eval('crossOriginIsolated'), mem: await cdp.eval('typeof performance.measureUserAgentSpecificMemory') });
    try { log('mem', { r: await cdp.eval('performance.measureUserAgentSpecificMemory().then(r => r.bytes, e => String(e))') }); } catch (e) { log('mem-error', { e: String(e) }); }
    await cdp.shot(`${EV}/probe-headless.png`);
  }

  if (scenario === 'main') {
    server = await startServer(['replay', '--fixture', 'fixture.smfx', '--speed', '8', '--web', 'web'], 'server-main.log');
    await openPage(cdp, server.url);
    // AC-a: five minutes of match time at 1x.
    await cdp.shot(`${EV}/ac-a-t0.png`);
    const samples = [await cdp.eval(READ)];
    for (let i = 1; i <= 10; i++) {
      await sleep(30000);
      samples.push(await cdp.eval(READ));
      if (i === 5) await cdp.shot(`${EV}/ac-a-t150.png`);
    }
    await cdp.shot(`${EV}/ac-a-final.png`);
    const signalsA = await cdp.eval('__sig.slice()');
    log('ac-a', { samples, signals: signalsA.filter((s) => !s.includes('frame_budget')), frame_budget_rows: signalsA.filter((s) => s.includes('frame_budget')).map((s) => JSON.parse(s)), clicks: await cdp.eval('__clicks') });

    // AC-d: 4x, three windows of 20 s.
    await cdp.click('#speed-4');
    await sleep(2000);
    await cdp.shot(`${EV}/ac-d-t0.png`);
    const windows = [];
    for (let i = 0; i < 3; i++) windows.push(await rate(cdp, 20));
    await cdp.shot(`${EV}/ac-d-final.png`);
    log('ac-d', { windows });

    // AC-f: rewind by a click on the scrubber at three positions.
    const rewinds = [];
    for (const fx of [0.5, 0.25, 0.75]) {
      await cdp.click('#scrub', fx, 0.5);
      await sleep(400);
      const r = await cdp.eval(`(() => { const lr = __touchline.lastRewind(); const t = __touchline.lastRenderedTick(); const stored = __touchline.tickAt(t); const drawn = __touchline.lastRendered(); return { lastRewind: lr && { tick: lr.tick, exact: lr.exact, components: lr.drawn.length }, renderedTick: t, clock: document.getElementById('clock').textContent, hookEqual: stored !== null && stored.every((v, i) => v === drawn[i]), scrub: document.getElementById('scrub').value, playing: document.getElementById('play').textContent }; })()`);
      rewinds.push({ fx, ...r });
      await cdp.shot(`${EV}/ac-f-${Math.round(fx * 100)}.png`);
      await sleep(1500);
    }
    log('ac-f', { rewinds, rewindSignals: (await cdp.eval('__sig.slice()')).filter((s) => s.includes('viewer.rewind')) });

    // AC-g: the gauge and the byte count, after the whole fixture has arrived.
    await waitFor(cdp, '__touchline.history().ticks_stored >= 270000', 240000).catch(() => {});
    await sleep(3000);
    log('ac-g', { history: await cdp.eval('__touchline.history()'), gauge: await cdp.eval("document.getElementById('gauge').textContent"), perfMemory: await cdp.eval('performance.memory ? {used: performance.memory.usedJSHeapSize, total: performance.memory.totalJSHeapSize} : null') });
    let mem = null;
    try { mem = await cdp.eval('performance.measureUserAgentSpecificMemory().then(r => ({bytes: r.bytes, top: r.breakdown.filter(b=>b.bytes>1e6).map(b=>({bytes:b.bytes, types:b.types}))}), e => String(e))'); } catch (e) { mem = String(e); }
    log('ac-g-direct', { mem, pageBytesAfter: await cdp.eval('__touchline.history().page_bytes') });
    await cdp.shot(`${EV}/ac-g-final.png`);
  }

  if (scenario === 'lag' || scenario === 'nolag') {
    const args = ['replay', '--fixture', 'fixture.smfx', '--speed', '8', '--web', 'web'];
    if (scenario === 'lag') args.push('--sustain', '3');
    server = await startServer(args, `server-${scenario}.log`);
    await openPage(cdp, server.url);
    await sleep(2000);
    await cdp.click('#speed-8');
    await cdp.shot(`${EV}/ac-e-${scenario}-t0.png`);
    const reads = [];
    for (let i = 0; i < 3; i++) {
      await sleep(8000);
      const r = await cdp.eval(READ);
      reads.push({ notice: r.notice, live: r.live, speed: r.speed, effective: r.effective, tick: r.tick, newest: r.history.newest_tick });
    }
    const windows = [await rate(cdp, 10)];
    await cdp.shot(`${EV}/ac-e-${scenario}-final.png`);
    const sig = await cdp.eval('__sig.slice()');
    log(`ac-e-${scenario}`, { reads, windows, lagRows: sig.filter((s) => s.includes('viewer.lag')), skippedRows: sig.filter((s) => s.includes('tick_skipped')).length, clicks: await cdp.eval('__clicks') });
    const vitals = await cdp.eval(`new Promise((res) => {
      const v = { lcp: null, cls: 0, inp: null, events: 0 };
      new PerformanceObserver((l) => { for (const e of l.getEntries()) v.lcp = e.startTime; }).observe({ type: 'largest-contentful-paint', buffered: true });
      new PerformanceObserver((l) => { for (const e of l.getEntries()) if (!e.hadRecentInput) v.cls += e.value; }).observe({ type: 'layout-shift', buffered: true });
      new PerformanceObserver((l) => { for (const e of l.getEntries()) { v.events++; if (e.interactionId) v.inp = Math.max(v.inp ?? 0, e.duration); } }).observe({ type: 'event', buffered: true, durationThreshold: 16 });
      setTimeout(() => res(v), 1500);
    })`);
    log(`vitals-${scenario}`, vitals);
    // Keyboard: Tab to the 8x button and press Enter/Space, and read the focus ring.
    const focus = await cdp.eval(`(() => { const b = document.getElementById('speed-2'); b.focus({ focusVisible: true }); const s = getComputedStyle(b); return { outline: s.outlineStyle + ' ' + s.outlineWidth + ' ' + s.outlineColor, boxShadow: s.boxShadow, active: document.activeElement.id }; })()`);
    log(`focus-${scenario}`, focus);
  }

  if (scenario === 'instr') {
    await cdp.send('Page.addScriptToEvaluateOnNewDocument', { source: `window.__all = []; const _oi = console.info.bind(console); console.info = (m) => { if (typeof m === 'string' && m.includes('viewer-event') && !m.includes('"viewer.history"') && !m.includes('frame_budget')) __all.push(m); _oi(m); };` });
    server = await startServer(['serve', '--seed', '42', '--minutes', '10', '--web', 'web'], 'server-instr.log');
    await cdp.goto(server.url);
    await sleep(12000);
    log('instr', { rows: await cdp.eval('__all.slice(0, 20)'), history: await cdp.eval('__touchline.history()'), notice: await cdp.eval("document.getElementById('notice').textContent.replace(/\\s+/g,' ').trim()") });
    await cdp.shot(`${EV}/instr-stream-ended.png`);
  }

  if (scenario === 'replayend') {
    server = await startServer(['replay', '--fixture', 'fixture.smfx', '--speed', '8', '--web', 'web'], 'server-replayend.log');
    await openPage(cdp, server.url);
    await sleep(3000);
    await cdp.goto('about:blank');
    await sleep(2000);
    log('replayend', { serverExit: server.exitCode2 ?? 'still running', tail: server.log().slice(-400) });
  }

  if (scenario === 'live') {
    server = await startServer(['serve', '--seed', '42', '--web', 'web'], 'server-live.log');
    await openPage(cdp, server.url);
    // Step 1: the page names the engine from the hello.
    const step1 = await cdp.eval(`({ teams: document.getElementById('teams').textContent, engine: document.getElementById('engine-version').textContent, connected: __sig.find(s => s.includes('viewer.connected')) ?? null })`);
    await cdp.shot(`${EV}/ac-h-step1.png`);
    log('ac-h-step1', step1);
    // Step 4: the clock advances; 22 markers and the ball move every tick; no marker leaves the pitch.
    const frames = [];
    for (let i = 0; i < 40; i++) {
      frames.push(await cdp.eval('({tick: __touchline.lastRenderedTick(), clock: document.getElementById("clock").textContent, pos: __touchline.lastRendered()})'));
      await sleep(250);
    }
    let outOfBounds = 0; let maxAbsX = 0; let maxAbsY = 0; let movedPlayers = new Set(); let ballMoved = false;
    for (let f = 0; f < frames.length; f++) {
      const p = frames[f].pos;
      for (let k = 0; k < 22; k++) {
        const x = p[3 + k * 2]; const y = p[4 + k * 2];
        maxAbsX = Math.max(maxAbsX, Math.abs(x)); maxAbsY = Math.max(maxAbsY, Math.abs(y));
        if (Math.abs(x) > 5250 || Math.abs(y) > 3400) outOfBounds++;
        if (f > 0 && (x !== frames[f - 1].pos[3 + k * 2] || y !== frames[f - 1].pos[4 + k * 2])) movedPlayers.add(k);
      }
      if (f > 0 && (p[0] !== frames[f - 1].pos[0] || p[1] !== frames[f - 1].pos[1])) ballMoved = true;
    }
    await cdp.shot(`${EV}/ac-h-step4.png`);
    log('ac-h-step4', { first: { tick: frames[0].tick, clock: frames[0].clock }, last: { tick: frames.at(-1).tick, clock: frames.at(-1).clock }, playersMoved: movedPlayers.size, ballMoved, outOfBounds, maxAbsX_cm: maxAbsX, maxAbsY_cm: maxAbsY });
    // Step 5: 4x.
    await cdp.click('#speed-4');
    await sleep(2000);
    const w = [await rate(cdp, 15), await rate(cdp, 15)];
    await cdp.shot(`${EV}/ac-h-step5.png`);
    log('ac-h-step5', { windows: w, notice: await cdp.eval("document.getElementById('notice').textContent.replace(/\\s+/g,' ').trim()"), history: await cdp.eval('__touchline.history()') });
    // AC-f re-drives on the fully stored live match, and AC-g page bytes at 270,000 ticks.
    const playBefore = await cdp.eval(`(() => { const b = document.getElementById('play'); return { label: b.textContent.trim(), ariaPressed: b.getAttribute('aria-pressed'), dataPlaying: b.dataset.playing ?? null, bg: getComputedStyle(b).backgroundColor }; })()`);
    await cdp.click('#play');
    await sleep(500);
    const playAfter = await cdp.eval(`(() => { const b = document.getElementById('play'); return { label: b.textContent.trim(), ariaPressed: b.getAttribute('aria-pressed'), dataPlaying: b.dataset.playing ?? null, bg: getComputedStyle(b).backgroundColor, live: document.getElementById('live').textContent, t1: __touchline.lastRenderedTick() }; })()`);
    await sleep(1000);
    log('play-toggle', { playBefore, playAfter, t2: await cdp.eval('__touchline.lastRenderedTick()') });
    const rewinds = [];
    for (const fx of [0.2, 0.5, 0.8]) {
      await cdp.click('#scrub', fx, 0.5);
      await sleep(400);
      const r = await cdp.eval(`(() => { const lr = __touchline.lastRewind(); const t = __touchline.lastRenderedTick(); const stored = __touchline.tickAt(t); const drawn = __touchline.lastRendered(); return { lastRewind: lr && { tick: lr.tick, exact: lr.exact, components: lr.drawn.length }, renderedTick: t, clock: document.getElementById('clock').textContent, hookEqual: stored !== null && stored.every((v, i) => v === drawn[i]), playing: document.getElementById('play').textContent.trim() }; })()`);
      rewinds.push({ fx, ...r });
      await cdp.shot(`${EV}/ac-f-live-${Math.round(fx * 100)}.png`);
    }
    log('ac-f-live', { rewinds, rewindSignals: (await cdp.eval('__sig.slice()')).filter((s) => s.includes('viewer.rewind')) });
    let mem = null;
    try { mem = await cdp.eval('performance.measureUserAgentSpecificMemory().then(r => r.bytes, e => String(e))'); } catch (e) { mem = String(e); }
    log('ac-g-live', { history: await cdp.eval('__touchline.history()'), mem, gauge: await cdp.eval("document.getElementById('gauge').textContent") });
  }
} catch (e) {
  log('error', { e: String(e.stack || e) });
} finally {
  out.finished = new Date().toISOString();
  writeFileSync(`${EV}/drive-${scenario}.json`, JSON.stringify(out, null, 2));
  cdp.close();
  try { server?.kill(); } catch {}
  await sleep(500);
  if (server) writeFileSync(`${EV}/server-${scenario}.log`, server.log() + `\nexit ${server.exitCode2 ?? 'killed by driver'}\n`);
  process.exit(0);
}
