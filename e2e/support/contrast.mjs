// A WCAG 2.2 AA contrast check of what the page renders, run in the page. Every visible text
// run is measured against the background it sits on (the nearest painted ancestor, composited
// over the ones below it): 4.5:1 for body text, 3:1 for large text (24 px, or 18.66 px bold).
// A gradient ground is measured at each colour stop, and the worst stop is reported.
// Controls are measured too: each icon button's glyph and edge, the rewind playhead, and the
// focus ring against the ground, at 3:1. Text inside a stub is inert and exempt, so it is
// reported but never counted as a failure.
//
// Colours are read through a canvas, so a colour the skin writes in any notation (hex, rgb or
// oklch) comes back as sRGB bytes.

/// Runs the check on `page` and returns { checked, failures, exempt, rows }.
export async function contrastReport(page) {
  // A control that just changed state is still fading between its two colours; measure the
  // settled page, not a frame of a transition. Endless animations (the LIVE pulse) are left.
  await page.evaluate(() =>
    Promise.all(
      document
        .getAnimations()
        .filter((a) => a.effect?.getComputedTiming().iterations !== Infinity)
        .map((a) => a.finished.catch(() => null))
    )
  );
  return page.evaluate(() => {
    const probe = document.createElement('canvas');
    probe.width = 1;
    probe.height = 1;
    const ctx = probe.getContext('2d', { willReadFrequently: true });

    /// [r, g, b, a] in 0..255 (a in 0..1) for any CSS colour, or null for none.
    const rgba = (css) => {
      if (!css || css === 'transparent' || css === 'none') {
        return null;
      }
      ctx.clearRect(0, 0, 1, 1);
      ctx.fillStyle = 'rgba(0, 0, 0, 0)';
      ctx.fillStyle = css;
      ctx.fillRect(0, 0, 1, 1);
      const [r, g, b, a] = ctx.getImageData(0, 0, 1, 1).data;
      return a === 0 ? null : [r, g, b, a / 255];
    };
    const over = (top, bottom) => {
      const a = top[3];
      return [0, 1, 2].map((i) => Math.round(top[i] * a + bottom[i] * (1 - a))).concat(1);
    };
    const luminance = ([r, g, b]) => {
      const lin = (v) => {
        const c = v / 255;
        return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
      };
      return 0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b);
    };
    const ratio = (a, b) => {
      const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x);
      return (hi + 0.05) / (lo + 0.05);
    };
    const hex = (c) => `#${c.slice(0, 3).map((v) => v.toString(16).padStart(2, '0')).join('')}`;

    /// The colour stops of a gradient background image, or [] when there is none. A stop
    /// that paints nothing (transparent) is null.
    const stops = (image) => {
      if (!image || !image.includes('gradient(')) {
        return [];
      }
      const colours =
        /(?:rgba?|hsla?|hwb|lab|lch|oklab|oklch|color)\([^()]*\)|#[0-9a-f]{3,8}\b|transparent/gi;
      return (image.match(colours) ?? []).map((stop) => rgba(stop));
    };

    /// The grounds behind `el`: its own background and its ancestors', composited, over the
    /// page ground. A solid ground gives one colour; a gradient gives one per colour stop.
    const behind = (el) => {
      const layers = [];
      for (let node = el; node && node.nodeType === 1; node = node.parentElement) {
        const style = getComputedStyle(node);
        // The image paints over the colour, so it is the upper layer.
        const image = stops(style.backgroundImage);
        if (image.some(Boolean)) {
          layers.push(image);
          if (image.every((c) => c && c[3] === 1)) {
            break;
          }
        }
        const c = rgba(style.backgroundColor);
        if (c) {
          layers.push([c]);
          if (c[3] === 1) {
            break;
          }
        }
      }
      let grounds = [rgba(getComputedStyle(document.body).backgroundColor) ?? [255, 255, 255, 1]];
      for (const layer of layers.reverse()) {
        const next = new Map();
        for (const ground of grounds) {
          for (const stop of layer) {
            // A transparent stop leaves the ground below it.
            const colour = stop ? over(stop, ground) : ground;
            next.set(hex(colour), colour);
          }
        }
        grounds = [...next.values()];
      }
      return grounds;
    };

    /// The ground `fg` measures worst against, and that ratio.
    const worst = (fg, grounds) =>
      grounds
        .map((bg) => ({ bg, ratio: ratio(over(fg, bg), bg) }))
        .reduce((a, b) => (b.ratio < a.ratio ? b : a));
    const visible = (el) => {
      const box = el.getBoundingClientRect();
      if (box.width === 0 || box.height === 0 || box.bottom < 0 || box.top > innerHeight) {
        return false;
      }
      const style = getComputedStyle(el);
      return style.visibility !== 'hidden' && style.display !== 'none' && Number(style.opacity) > 0;
    };
    const describe = (el) =>
      `${el.tagName.toLowerCase()}${el.className && typeof el.className === 'string' ? `.${el.className.trim().split(/\s+/).join('.')}` : ''}`;

    const rows = [];
    const add = (row) => rows.push({ ...row, ratio: Math.round(row.ratio * 100) / 100 });

    // Text runs.
    const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT);
    const seen = new Set();
    for (let node = walker.nextNode(); node; node = walker.nextNode()) {
      const el = node.parentElement;
      if (!node.textContent.trim() || !el || seen.has(el) || el.closest('script, style, svg')) {
        continue;
      }
      seen.add(el);
      if (!visible(el)) {
        continue;
      }
      const style = getComputedStyle(el);
      const fg = rgba(style.color);
      if (!fg) {
        continue;
      }
      const { bg, ratio: measured } = worst(fg, behind(el));
      const size = parseFloat(style.fontSize);
      const bold = Number(style.fontWeight) >= 700;
      const large = size >= 24 || (bold && size >= 18.66);
      // Opacity on an ancestor fades the text and its own ground together; the stub is the
      // only faded part, and it is exempt.
      const stub = el.closest('[data-stub]');
      add({
        kind: 'text',
        text: node.textContent.trim().slice(0, 40),
        element: describe(el),
        fg: hex(over(fg, bg)),
        bg: hex(bg),
        size,
        floor: large ? 3 : 4.5,
        ratio: measured,
        exempt: stub ? `stub: ${stub.dataset.stub}` : null,
      });
    }

    // Icon buttons: glyph and edge against what the button sits on.
    for (const button of document.querySelectorAll('button.ib')) {
      if (!visible(button) || button.closest('[data-stub]')) {
        continue;
      }
      const style = getComputedStyle(button);
      const own = rgba(style.backgroundColor);
      const grounds = behind(button.parentElement);
      const faces = own ? grounds.map((ground) => over(own, ground)) : grounds;
      const glyph = rgba(style.color);
      const face = worst(glyph, faces);
      add({
        kind: 'glyph',
        text: button.getAttribute('aria-label'),
        element: describe(button),
        fg: hex(glyph),
        bg: hex(face.bg),
        floor: 3,
        ratio: face.ratio,
        exempt: null,
      });
      const edge = rgba(style.borderTopColor);
      if (edge && !own) {
        const ground = worst(edge, grounds);
        add({
          kind: 'boundary',
          text: `${button.getAttribute('aria-label')} edge`,
          element: describe(button),
          fg: hex(edge),
          bg: hex(ground.bg),
          floor: 3,
          ratio: ground.ratio,
          exempt: null,
        });
      }
    }

    // The rewind playhead against its track.
    const head = document.querySelector('.timeline .head');
    if (head && visible(head)) {
      const fg = rgba(getComputedStyle(head).backgroundColor);
      const { bg, ratio: measured } = worst(fg, behind(head.parentElement));
      add({
        kind: 'boundary',
        text: 'rewind playhead',
        element: describe(head),
        fg: hex(fg),
        bg: hex(bg),
        floor: 3,
        ratio: measured,
        exempt: null,
      });
    }

    // The focus ring against the page ground and the panel ground.
    const ring = rgba(getComputedStyle(document.documentElement).getPropertyValue('--cyan').trim());
    for (const token of ['--ground', '--ground-2']) {
      const bg = rgba(getComputedStyle(document.documentElement).getPropertyValue(token).trim());
      if (ring && bg) {
        add({
          kind: 'focus ring',
          text: `focus ring on ${token}`,
          element: ':focus-visible',
          fg: hex(ring),
          bg: hex(bg),
          floor: 3,
          ratio: ratio(ring, bg),
          exempt: null,
        });
      }
    }

    const failures = rows.filter((r) => !r.exempt && r.ratio < r.floor);
    const exempt = rows.filter((r) => r.exempt && r.ratio < r.floor);
    return { checked: rows.length, failures, exempt, rows };
  });
}
