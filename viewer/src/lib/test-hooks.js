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
