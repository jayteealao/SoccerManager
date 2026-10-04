// The layout check of what the page renders at one window size, run in the page. Playwright
// has no assertion for overflow, overlap or hit areas, so this report measures them with the
// page's own boxes. Each row names the rule, the element and its box:
//
//   1. fill: the page's root box equals the window (no empty band beside or below it);
//   2. sideways scroll: the page, or any box that scrolls sideways (overflow-x auto or scroll),
//      is wider than its window; the tab row at the compact step is the one drawn sideways
//      scroll and is allowed;
//   3. clipped text: a text run that does not fit its own box or is cut by a box that clips
//      (overflow hidden or clip). An ellipsis is allowed only where `data-may-truncate` marks
//      the text or a box around it (the header title, club and player names);
//   4. overlap: two visible text runs or controls whose boxes cross by more than 1 px on both
//      axes where neither holds the other, outside `[data-layout-layer]` (the report
//      timeline, whose markers for moments a minute apart touch, and the kick-off pitch,
//      where the two centre forwards' names touch); while a dialog is open only its own
//      parts are checked, and an open menu crosses nothing under it;
//   5. small controls: every visible control that takes focus answers `elementFromPoint` at
//      the four corners of a square of `minControl` px around its centre, so a hit area drawn
//      by a pseudo-element counts and a covered control does not;
//   6. cut off: every control lies inside the window across, and inside the window down or in
//      a box that scrolls down to it.
//
// It also returns the rendered contrast failures (support/contrast.mjs) under `contrast`. The
// report is data; the caller asserts it.
import { contrastReport } from './contrast.mjs';

/// Runs the check on `page` and returns { size, rows, contrast }.
export async function layoutReport(page, { minControl = 24 } = {}) {
  const rows = await page.evaluate((min) => {
    const out = [];
    const view = { width: window.innerWidth, height: window.innerHeight };
    const box = (r) => ({ x: Math.round(r.left), y: Math.round(r.top), w: Math.round(r.width), h: Math.round(r.height) });
    const name = (el) => {
      const id = el.id ? `#${el.id}` : '';
      const cls = typeof el.className === 'string' && el.className.trim() ? `.${el.className.trim().split(/\s+/).slice(0, 2).join('.')}` : '';
      const label = el.getAttribute('aria-label') ?? el.textContent?.trim().slice(0, 40) ?? '';
      return `${el.tagName.toLowerCase()}${id}${cls}${label ? ` "${label}"` : ''}`;
    };
    const add = (rule, el, rect, detail = '') => out.push({ rule, element: name(el), box: box(rect), detail });

    const shown = (el) => {
      const r = el.getBoundingClientRect();
      if (r.width < 2 || r.height < 2) {
        return false;
      }
      const cs = getComputedStyle(el);
      return cs.visibility !== 'hidden' && Number(cs.opacity) > 0;
    };
    const clips = (cs, axis) => ['hidden', 'clip'].includes(axis === 'x' ? cs.overflowX : cs.overflowY);
    const scrolls = (cs, axis) => ['auto', 'scroll'].includes(axis === 'x' ? cs.overflowX : cs.overflowY);
    const mayTruncate = (el) => Boolean(el.closest('[data-may-truncate]'));
    const tabRow = (el) => Boolean(el.closest('nav.subnav'));

    // While a dialog is open, the page under its scrim is out of reach: check the dialog only.
    const dialog = [...document.querySelectorAll('[role="dialog"], [role="alertdialog"]')].find(shown) ?? null;
    const inScope = (el) => (dialog ? dialog.contains(el) : true);
    const layer = (el) => Boolean(el.closest('[data-layout-layer]')) && !(dialog && dialog.contains(el));
    // An open menu lies over the page: a page part under it neither crosses it nor is
    // measured through it.
    const menu = [...document.querySelectorAll('[role="menu"]')].find(shown) ?? null;
    const underMenu = (el, hit) => menu !== null && !menu.contains(el) && hit !== null && menu.contains(hit);

    // 1. fill
    const root = document.getElementById('app');
    if (root) {
      const r = root.getBoundingClientRect();
      if (Math.abs(r.width - view.width) > 1 || Math.abs(r.height - view.height) > 1) {
        add('fill', root, r, `the page is ${Math.round(r.width)} by ${Math.round(r.height)} in a ${view.width} by ${view.height} window`);
      }
      const top = root.firstElementChild;
      const t = top?.getBoundingClientRect();
      if (t && (Math.abs(t.width - view.width) > 1 || Math.abs(t.height - view.height) > 1)) {
        add('fill', top, t, `the page's top box is ${Math.round(t.width)} by ${Math.round(t.height)}`);
      }
    }

    const all = [...document.body.querySelectorAll('*')].filter((el) => !['SCRIPT', 'STYLE', 'TITLE', 'OPTION'].includes(el.tagName));

    // 2. sideways scroll
    const doc = document.scrollingElement;
    if (doc.scrollWidth > view.width + 1) {
      add('sideways scroll', doc, doc.getBoundingClientRect(), `the page is ${doc.scrollWidth} px wide`);
    }
    for (const el of all) {
      const cs = getComputedStyle(el);
      if (scrolls(cs, 'x') && el.scrollWidth > el.clientWidth + 1 && shown(el) && !tabRow(el) && inScope(el)) {
        add('sideways scroll', el, el.getBoundingClientRect(), `${el.scrollWidth} px in ${el.clientWidth} px`);
      }
    }

    // The text runs: each element's own text, measured by a range over its text nodes. A
    // range's box is the font's whole content area, which is taller than the line when the
    // line height is tight; each line is cut to its line height, so two stacked lines do not
    // read as an overlap.
    const texts = [];
    for (const el of all) {
      const nodes = [...el.childNodes].filter((n) => n.nodeType === 3 && n.textContent.trim());
      if (!nodes.length || !shown(el) || !inScope(el)) {
        continue;
      }
      const cs = getComputedStyle(el);
      const size = parseFloat(cs.fontSize) || 11;
      const line = (cs.lineHeight === 'normal' ? size * 1.2 : parseFloat(cs.lineHeight)) * (el.currentCSSZoom ?? 1);
      let left = Infinity;
      let top = Infinity;
      let right = -Infinity;
      let bottom = -Infinity;
      const lineRects = [];
      for (const node of nodes) {
        const range = document.createRange();
        range.selectNodeContents(node);
        for (const r of range.getClientRects()) {
          if (r.width > 0 && r.height > 0) {
            const inset = Math.max(0, (r.height - line) / 2);
            left = Math.min(left, r.left);
            top = Math.min(top, r.top + inset);
            right = Math.max(right, r.right);
            bottom = Math.max(bottom, r.bottom - inset);
            lineRects.push({ left: r.left, top: r.top + inset, right: r.right, bottom: r.bottom - inset });
          }
        }
      }
      if (right > left) {
        texts.push({ el, rect: { left, top, right, bottom, width: right - left, height: bottom - top }, lines: lineRects });
      }
    }

    // 3. clipped text
    for (const { el, rect } of texts) {
      const cs = getComputedStyle(el);
      if (cs.display !== 'inline') {
        const wide = el.scrollWidth > el.clientWidth + 1 && clips(cs, 'x');
        const tall = el.scrollHeight > el.clientHeight + 1 && clips(cs, 'y');
        if (wide && cs.textOverflow === 'ellipsis' && !mayTruncate(el)) {
          add('clipped text', el, rect, 'ends in an ellipsis where no data-may-truncate allows it');
        } else if ((wide && cs.textOverflow !== 'ellipsis') || tall) {
          add('clipped text', el, rect, `${el.scrollWidth} by ${el.scrollHeight} in ${el.clientWidth} by ${el.clientHeight}`);
        }
      }
      for (let up = el.parentElement; up && up !== document.body; up = up.parentElement) {
        const ucs = getComputedStyle(up);
        const u = up.getBoundingClientRect();
        const cutX = clips(ucs, 'x') && (rect.left < u.left - 1 || rect.right > u.right + 1);
        const cutY = clips(ucs, 'y') && (rect.top < u.top - 1 || rect.bottom > u.bottom + 1);
        if ((cutX && !mayTruncate(el)) || cutY) {
          add('clipped text', el, rect, `cut by ${name(up)}`);
          break;
        }
        if (scrolls(ucs, 'x') || scrolls(ucs, 'y')) {
          break;
        }
      }
    }

    // The controls that take focus.
    const CONTROL = 'button, a[href], input:not([type="hidden"]), select, textarea, [role="button"], [role="tab"], [role="slider"], [tabindex]';
    const controls = [...document.querySelectorAll(CONTROL)].filter(
      (el) => el.tabIndex >= 0 && !el.disabled && !el.closest('[inert]') && shown(el) && inScope(el)
    );

    // 4. overlap. Each item is cut to the boxes around it that clip or scroll, so a row
    // scrolled out of its list is not on show and overlaps nothing.
    const onShow = (el, rect) => {
      let { left, top, right, bottom } = rect;
      // From the item itself: a run cut by its own ellipsis shows only inside its box.
      for (let up = el; up && up !== document.body; up = up.parentElement) {
        const ucs = getComputedStyle(up);
        if (ucs.overflowX !== 'visible' || ucs.overflowY !== 'visible') {
          const u = up.getBoundingClientRect();
          left = Math.max(left, u.left);
          top = Math.max(top, u.top);
          right = Math.min(right, u.right);
          bottom = Math.min(bottom, u.bottom);
        }
      }
      return right - left > 1 && bottom - top > 1 ? { left, top, right, bottom, width: right - left, height: bottom - top } : null;
    };
    // A text run is measured line by line, so a run that wraps and starts on the line another
    // run ends does not cross it.
    const items = [...texts, ...controls.map((el) => ({ el, rect: el.getBoundingClientRect() }))]
      .filter((i) => !layer(i.el))
      .map((i) => ({ el: i.el, rect: onShow(i.el, i.rect), rects: (i.lines ?? [i.rect]).map((r) => onShow(i.el, r)).filter(Boolean) }))
      .filter((i) => i.rect && i.rects.length);
    const cross = (p, q) => {
      let best = null;
      for (const r of p.rects) {
        for (const s of q.rects) {
          const x = Math.min(r.right, s.right) - Math.max(r.left, s.left);
          const y = Math.min(r.bottom, s.bottom) - Math.max(r.top, s.top);
          if (x > 1 && y > 1 && (!best || x * y > best.x * best.y)) {
            best = { x, y };
          }
        }
      }
      return best;
    };
    for (let a = 0; a < items.length; a += 1) {
      for (let b = a + 1; b < items.length; b += 1) {
        const p = items[a];
        const q = items[b];
        if (p.el === q.el || p.el.contains(q.el) || q.el.contains(p.el)) {
          continue;
        }
        if (menu !== null && menu.contains(p.el) !== menu.contains(q.el)) {
          continue;
        }
        const hit = cross(p, q);
        if (hit) {
          add('overlap', p.el, p.rect, `crosses ${name(q.el)} by ${Math.round(hit.x)} by ${Math.round(hit.y)} px`);
        }
      }
    }

    // 5. small controls and 6. cut off
    const half = min / 2 - 1;
    for (const el of controls) {
      const r = el.getBoundingClientRect();
      const cx = r.left + r.width / 2;
      const cy = r.top + r.height / 2;
      let scroller = null;
      for (let up = el.parentElement; up; up = up.parentElement) {
        const ucs = getComputedStyle(up);
        if ((scrolls(ucs, 'y') && up.scrollHeight > up.clientHeight) || (scrolls(ucs, 'x') && up.scrollWidth > up.clientWidth)) {
          scroller = up;
          break;
        }
      }
      const across = r.left >= -1 && r.right <= view.width + 1;
      const down = r.top >= -1 && r.bottom <= view.height + 1;
      // Shown whole: no box around it that clips or scrolls cuts it.
      const v = onShow(el, r);
      const inScroller =
        v !== null && v.left <= r.left + 1 && v.right >= r.right - 1 && v.top <= r.top + 1 && v.bottom >= r.bottom - 1;
      if (!across && !(scroller && tabRow(el))) {
        add('cut off', el, r, 'outside the window across');
      } else if (!down && !scroller) {
        add('cut off', el, r, 'outside the window down, with no box that scrolls to it');
      }
      // A control scrolled out of its box's view, or partly out, is reachable; its hit area
      // is measured where it shows whole.
      if (!down || !across || (scroller && !inScroller)) {
        continue;
      }
      // A point past the window's edge is taken at the edge: the edge stops the pointer, so a
      // control against it is as large as its hit area inside the window.
      const inside = ([px, py]) => [Math.min(Math.max(px, 0), view.width - 1), Math.min(Math.max(py, 0), view.height - 1)];
      const misses = [
        [cx - half, cy - half],
        [cx + half, cy - half],
        [cx - half, cy + half],
        [cx + half, cy + half],
      ]
        .map(inside)
        .map(([px, py]) => document.elementFromPoint(px, py))
        .filter((hit) => !hit || !(hit === el || el.contains(hit)))
        .filter((hit) => !underMenu(el, hit));
      if (misses.length) {
        const by = misses[0] ? ` (${name(misses[0])} takes the first)` : '';
        add('small control', el, r, `${misses.length} of 4 points of a ${min} px square miss it${by}`);
      }
    }
    return out;
  }, minControl);
  const { failures } = await contrastReport(page);
  return { size: page.viewportSize(), minControl, rows, contrast: failures };
}
