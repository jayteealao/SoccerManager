// Minimal Chrome DevTools Protocol driver for headless Edge. Verify-time tooling only.
import { spawn } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const EDGE = 'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe';
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

export async function launch({ port = 9333, profile, width = 1280, height = 800 }) {
  mkdirSync(profile, { recursive: true });
  const proc = spawn(EDGE, [
    '--headless=new',
    `--remote-debugging-port=${port}`,
    `--user-data-dir=${profile}`,
    '--no-first-run',
    '--no-default-browser-check',
    '--disable-extensions',
    '--disable-background-timer-throttling',
    '--disable-renderer-backgrounding',
    '--disable-backgrounding-occluded-windows',
    `--window-size=${width},${height}`,
    'about:blank',
  ], { stdio: 'ignore' });
  let list = null;
  for (let i = 0; i < 100 && !list; i++) {
    await sleep(100);
    try {
      list = await fetch(`http://127.0.0.1:${port}/json/list`).then((r) => r.json());
    } catch {}
  }
  const page = list.find((t) => t.type === 'page');
  const cdp = await connect(page.webSocketDebuggerUrl);
  await cdp.send('Page.enable');
  await cdp.send('Runtime.enable');
  await cdp.send('Emulation.setDeviceMetricsOverride', { width, height, deviceScaleFactor: 1, mobile: false });
  cdp.proc = proc;
  return cdp;
}

async function connect(url) {
  const ws = new WebSocket(url);
  await new Promise((res, rej) => { ws.onopen = res; ws.onerror = rej; });
  let id = 0;
  const pending = new Map();
  const listeners = [];
  ws.onmessage = (ev) => {
    const msg = JSON.parse(ev.data);
    if (msg.id && pending.has(msg.id)) {
      const { res, rej } = pending.get(msg.id);
      pending.delete(msg.id);
      msg.error ? rej(new Error(JSON.stringify(msg.error))) : res(msg.result);
    } else if (msg.method) {
      for (const l of listeners) l(msg);
    }
  };
  const cdp = {
    send: (method, params = {}) => new Promise((res, rej) => {
      const n = ++id;
      pending.set(n, { res, rej });
      ws.send(JSON.stringify({ id: n, method, params }));
    }),
    on: (fn) => listeners.push(fn),
    async eval(expr) {
      const r = await cdp.send('Runtime.evaluate', { expression: expr, awaitPromise: true, returnByValue: true });
      if (r.exceptionDetails) throw new Error(r.exceptionDetails.exception?.description ?? JSON.stringify(r.exceptionDetails));
      return r.result.value;
    },
    async goto(url) {
      await cdp.send('Page.navigate', { url });
      await sleep(500);
    },
    async click(selector, fx = 0.5, fy = 0.5) {
      const rect = await cdp.eval(`(() => { const r = document.querySelector(${JSON.stringify(selector)}).getBoundingClientRect(); return {x: r.x, y: r.y, w: r.width, h: r.height}; })()`);
      const x = rect.x + rect.w * fx;
      const y = rect.y + rect.h * fy;
      await cdp.send('Input.dispatchMouseEvent', { type: 'mouseMoved', x, y });
      await cdp.send('Input.dispatchMouseEvent', { type: 'mousePressed', x, y, button: 'left', clickCount: 1 });
      await cdp.send('Input.dispatchMouseEvent', { type: 'mouseReleased', x, y, button: 'left', clickCount: 1 });
      return { x, y };
    },
    async shot(path) {
      const r = await cdp.send('Page.captureScreenshot', { format: 'png' });
      writeFileSync(path, Buffer.from(r.data, 'base64'));
      return path;
    },
    close() {
      try { ws.close(); } catch {}
      try { cdp.proc?.kill(); } catch {}
    },
  };
  return cdp;
}

export async function waitFor(cdp, expr, timeoutMs = 20000) {
  const t0 = Date.now();
  while (Date.now() - t0 < timeoutMs) {
    try { if (await cdp.eval(expr)) return true; } catch {}
    await sleep(100);
  }
  throw new Error(`timed out waiting for ${expr}`);
}

export { sleep, join };
