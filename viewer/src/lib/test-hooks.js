// The read-only test hook, `window.__touchline`. The browser tests and drives read the match
// through it; it exposes no setter, because a hook that can change the page is a hook that
// can hide a fault. The names follow the former page's hook where they mean the same thing.

import { COMPONENT_COUNT } from './decode.js';
import { signals } from './signal.js';
import { copyTactics } from './tactics-panel.js';

/// Installs the hook for `session` on `target` (the window) and returns it.
export function install(session, target = globalThis) {
  const hook = Object.freeze({
    screen: () => session.screen,
    lastRendered: () => Array.from(session.rendered),
    lastRenderedTick: () => session.renderedTick,
    signals,
    frame: () => session.scheduler.budget(),
    history: () => ({
      ticks_stored: session.history ? session.history.count : 0,
      newest_tick: session.history ? session.history.newestTick : 0,
    }),
    matchDay: () => session.snapshot(),
    lastRewind: () => session.lastRewind,
    stoppageAt: (tick) => session.stoppages.next(tick),
    tickAt: (tick) => {
      const out = new Int16Array(COMPONENT_COUNT);
      return session.history && session.history.tickAt(tick, out) ? Array.from(out) : null;
    },
    recovery: () => ({
      kind: session.panel ? session.panel.kind : null,
      title: session.panel ? session.panel.title : null,
      actions: session.panel ? [...session.panel.actions] : [],
      path: session.panel?.path ?? null,
      engine_state: session.status ? session.status['engine.state'] : null,
      engine_pid: session.status ? (session.status['engine.pid'] ?? null) : null,
      snapshot_tick: session.status ? (session.status['snapshot.tick'] ?? null) : null,
      reconnecting: session.screen === 'reconnecting',
      step: session.stepShown,
    }),
    /// The Resume a saved match screen: whether it shows, the save's version and why it
    /// cannot continue, and whether an engine was started for it.
    resume: () => {
      const info = session.resumeInfo;
      return {
        shown: session.view === 'resume',
        kind: info?.kind ?? null,
        saved_version: info?.['saved.version'] ?? null,
        saved_tick: info?.['saved.tick'] ?? null,
        reason: info?.reason ?? null,
        engine_pid: session.status ? (session.status['engine.pid'] ?? null) : null,
      };
    },
    /// The engine that plays the match, the launcher's own version, the version the hello
    /// named, and the loading steps' words.
    engineVersion: () => ({
      engine: session.status?.['engine.version'] ?? null,
      launcher: session.status?.['launcher.version'] ?? null,
      hello: session.engineVersion,
      resumed_from: session.status?.['match.resumed_from'] ?? null,
      steps: session.steps.map((s) => s.label),
    }),
    /// The ground the match is played on and where the pitch canvas draws it: the ground in
    /// metres, the box, and the drawn rectangle in CSS pixels; null before a canvas has a pitch.
    pitchGeometry: () => (session.pitch ? session.pitch.geometry() : null),
    events: () => session.match.events.map((e) => ({ ...e })),
    view: () => session.view,
    /// The report on show or last shown, as the former page's hook named it; `open` is true
    /// while the report view is up, and `state` is `loading` or `ready`.
    report: () => {
      const report = session.report;
      return {
        kind: report ? report.kind : null,
        open: session.view === 'report',
        state: report ? report.state : null,
        tick: report ? report.tick : null,
        score: report ? [...report.model.score] : null,
        counts: report ? Object.fromEntries(report.model.rows.map((r) => [r.id, [...r.counts]])) : null,
      };
    },
    replay: () => ({
      stored: session.stored,
      frames: session.frames.count,
      ticks: session.frames.tickFrames,
      last_saved_name: session.lastSaved ? session.lastSaved.name : null,
      last_saved_size: session.lastSaved ? session.lastSaved.bytes.length : null,
      last_saved_hash: session.lastSaved ? session.lastSaved.hash : null,
    }),
    lastSavedBytes: () => (session.lastSaved ? Array.from(session.lastSaved.bytes) : null),
    /// The notice on show: its kind, its word and its words, or null; and the playback speed
    /// asked for and the one in effect.
    notice: () => ({
      shown: session.notice !== null,
      kind: session.notice?.kind ?? null,
      word: session.notice?.word ?? null,
      message: session.notice?.message ?? null,
      speed: session.speed,
      effective_speed: session.effectiveSpeed,
    }),
    advice: () => ({
      tick: session.dugout.advice?.tick ?? null,
      picks: session.dugout.proposals().map((p) => ({ text: p.text, code: p.pick.code, accepted: p.accepted })),
    }),
    sheet: () => {
      const sheet = session.sheet();
      return sheet
        ? {
            shapes: [...sheet.shapes],
            home: sheet.home.eleven.map((r) => r.squad),
            bench: sheet.home.bench.map((r) => r.squad),
            away: sheet.away.eleven.map((r) => r.name),
            dots: sheet.dots.map((side) => side.length),
            rules: sheet.rules.live.map((r) => r.text),
            level: sheet.rules.level,
          }
        : null;
    },
    /// The skip to the result: its state and the tick it was skipped at, or null.
    skip: () => (session.skip ? { state: session.skip.state, from: session.skip.from } : null),
    canSkip: () => session.canSkip,
    /// The other grounds: the round's fixtures, every ground event that arrived per fixture,
    /// the tick each ground has reached, and what the list shows at the rendered tick.
    matchday: () => {
      const day = session.matchday;
      return {
        fixtures: day ? day.fixtures.map((f) => ({ fixture: f.fixture, home: f.home['team.name'], away: f.away['team.name'] })) : [],
        events: day ? day.events.map((list) => list.map((e) => ({ ...e }))) : [],
        reached: day ? [...day.reached] : [],
        grounds: structuredClone(session.grounds),
      };
    },
    /// `true` once every ground has reached `tick` or ended, so a screenshot shows settled rows.
    groundsReady: (tick) => {
      const day = session.matchday;
      if (!day) {
        return false;
      }
      return day.fixtures.every(
        (_, i) =>
          (day.reached[i] ?? 0) >= tick ||
          day.events[i].some((e) => e.kind === 'full-time' || e.kind === 'unavailable')
      );
    },
    /// SHA-256 in hex over every stored binary tick frame, in order. Text frames carry wall
    /// stamps and are left out; the tick frames are what a saved replay holds of the match.
    tickFrameDigest: async () => {
      const store = session.frames;
      const parts = [];
      let size = 0;
      for (let i = 0; i < store.count; i += 1) {
        const { text, payload } = store.frame(i);
        if (!text) {
          parts.push(payload);
          size += payload.length;
        }
      }
      const all = new Uint8Array(size);
      let at = 0;
      for (const part of parts) {
        all.set(part, at);
        at += part.length;
      }
      const digest = new Uint8Array(await globalThis.crypto.subtle.digest('SHA-256', all));
      return Array.from(digest, (b) => b.toString(16).padStart(2, '0')).join('');
    },
    /// The type of every command the page sent, in order.
    sentCommands: () => [...session.sent],
    pending: () => session.dugout.pending.all(session.renderedTick),
    lineup: () => session.dugout.lineupView(),
    dugout: () => ({
      phase: session.dugout.phase,
      lead_holding: session.lead.holding,
      lead_pauses: session.lead.pauses,
      tactics: session.dugout.tactics ? copyTactics(session.dugout.tactics) : null,
      subs_left: session.dugout.picker.left,
      editing: session.dugout.editing ? { ...session.dugout.editing } : null,
      cancel_refused: session.dugout.cancelRefused,
    }),
  });
  target.__touchline = hook;
  return hook;
}
