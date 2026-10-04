// The token scan. Every colour and font value lives in a skin; a component or a module reads
// it through `var(--…)`. This scan fails, naming the file and the line, on any of these in a
// `.svelte`, `.js` or `.css` file outside `src/skins/`:
//
//   - a hex colour, or an rgb, rgba, hsl, hsla, oklch, oklab or color-mix call;
//   - a named CSS colour in a colour property;
//   - a font family name (in `font-family`, in the `font` shorthand, or as an SVG attribute);
//   - `var(--ink-4)` anywhere but StubSection.svelte: that grey is for inactive stub text only.
//
// It also holds the window-step layout rules:
//
//   - a fluid font size: clamp(), min(), max(), a viewport unit or a container unit in
//     `font-size` or in the `font` shorthand. Text steps only with the page's scale;
//   - layout read in script (getBoundingClientRect, offsetWidth and offsetHeight, clientWidth
//     and clientHeight, ResizeObserver, innerWidth and innerHeight, or a width or height read
//     from getComputedStyle). Only the pitch measures its box; the exceptions are listed in
//     LAYOUT_READS_ALLOWED;
//   - an `@media` width or height other than the window steps: min-width 1024px, 1600px,
//     1920px or 2560px, max-width 1023px or 1919px (the top of wide) and max-height 599px;
//   - a compact width (max-width 1023px) without the compact height (max-height 599px) in the
//     same prelude: the compact step is both halves.
//
// Comments are not scanned: they describe values, they do not set them.
//
// Usage: node scripts/token-scan.mjs [file or folder ...]   (default: src)

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const ROOT = path.join(HERE, '..');
const SKINS = path.join(ROOT, 'src', 'skins');
const EXTENSIONS = new Set(['.svelte', '.js', '.css']);

const NAMED = [
  'aliceblue', 'antiquewhite', 'aqua', 'aquamarine', 'azure', 'beige', 'bisque', 'black',
  'blanchedalmond', 'blue', 'blueviolet', 'brown', 'burlywood', 'cadetblue', 'chartreuse',
  'chocolate', 'coral', 'cornflowerblue', 'cornsilk', 'crimson', 'cyan', 'darkblue',
  'darkcyan', 'darkgoldenrod', 'darkgray', 'darkgreen', 'darkgrey', 'darkkhaki',
  'darkmagenta', 'darkolivegreen', 'darkorange', 'darkorchid', 'darkred', 'darksalmon',
  'darkseagreen', 'darkslateblue', 'darkslategray', 'darkslategrey', 'darkturquoise',
  'darkviolet', 'deeppink', 'deepskyblue', 'dimgray', 'dimgrey', 'dodgerblue', 'firebrick',
  'floralwhite', 'forestgreen', 'fuchsia', 'gainsboro', 'ghostwhite', 'gold', 'goldenrod',
  'gray', 'green', 'greenyellow', 'grey', 'honeydew', 'hotpink', 'indianred', 'indigo',
  'ivory', 'khaki', 'lavender', 'lavenderblush', 'lawngreen', 'lemonchiffon', 'lightblue',
  'lightcoral', 'lightcyan', 'lightgoldenrodyellow', 'lightgray', 'lightgreen', 'lightgrey',
  'lightpink', 'lightsalmon', 'lightseagreen', 'lightskyblue', 'lightslategray',
  'lightslategrey', 'lightsteelblue', 'lightyellow', 'lime', 'limegreen', 'linen', 'magenta',
  'maroon', 'mediumaquamarine', 'mediumblue', 'mediumorchid', 'mediumpurple',
  'mediumseagreen', 'mediumslateblue', 'mediumspringgreen', 'mediumturquoise',
  'mediumvioletred', 'midnightblue', 'mintcream', 'mistyrose', 'moccasin', 'navajowhite',
  'navy', 'oldlace', 'olive', 'olivedrab', 'orange', 'orangered', 'orchid', 'palegoldenrod',
  'palegreen', 'paleturquoise', 'palevioletred', 'papayawhip', 'peachpuff', 'peru', 'pink',
  'plum', 'powderblue', 'purple', 'rebeccapurple', 'red', 'rosybrown', 'royalblue',
  'saddlebrown', 'salmon', 'sandybrown', 'seagreen', 'seashell', 'sienna', 'silver',
  'skyblue', 'slateblue', 'slategray', 'slategrey', 'snow', 'springgreen', 'steelblue', 'tan',
  'teal', 'thistle', 'tomato', 'turquoise', 'violet', 'wheat', 'white', 'whitesmoke',
  'yellow', 'yellowgreen',
];

const COLOUR_PROPERTY =
  /\b(?:color|background(?:-color)?|border(?:-(?:top|right|bottom|left))?(?:-color)?|outline(?:-color)?|fill|stroke|box-shadow|text-shadow|text-decoration-color|caret-color|accent-color|column-rule-color|stop-color)\s*[:=]\s*([^;{}]*)/gi;

const RULES = [
  { what: 'a hex colour', pattern: /#[0-9a-fA-F]{3,8}\b/ },
  {
    what: 'a colour function',
    pattern: /\b(?:rgba?|hsla?|oklch|oklab|lab|lch|hwb|color-mix|color)\(/i,
  },
  { what: 'a font family', pattern: /font-family\s*[:=]/i },
  {
    what: 'a font family',
    pattern:
      /\bfont\s*:[^;{}]*(?:['"][^'"]+['"]|\b(?:sans-serif|serif|monospace|system-ui|cursive|fantasy)\b)/i,
  },
];

const FLUID_FONT =
  /\bfont(?:-size)?\s*[:=][^;{}]*(?:\b(?:clamp|min|max)\(|\d(?:d|s|l)?v(?:w|h|i|b|min|max)\b|\dcq(?:w|h|i|b|min|max)\b)/i;

const LAYOUT_READ =
  /\b(getBoundingClientRect|offsetWidth|offsetHeight|clientWidth|clientHeight|ResizeObserver|innerWidth|innerHeight)\b|getComputedStyle\([^)]*\)\s*\.\s*(width|height)\b/g;

/// The layout reads each file may make, by file name. The pitch sizes its canvas to its box;
/// Root reads the window's width for the scale below 768 px wide; the test hooks report the
/// pitch's box to the browser tests; the commentary reads its scroll position to follow it.
const LAYOUT_READS_ALLOWED = {
  'PitchCanvas.svelte': 'all',
  'pitch.js': 'all',
  'Root.svelte': ['innerWidth'],
  'test-hooks.js': ['getBoundingClientRect'],
  'feed.js': ['clientHeight'],
};

const STEP_WIDTHS = { 'min-width': [1024, 1600, 1920, 2560], 'max-width': [1023, 1919], 'max-height': [599] };

/// The size features in one `@media` prelude that are not window steps.
function offStepFeatures(prelude) {
  const off = [];
  for (const [, feature, value] of prelude.matchAll(/\(\s*((?:min-|max-)?(?:width|height))\s*:\s*([^)]*)\)/gi)) {
    const px = /^(\d+)px$/.exec(value.trim());
    if (!px || !(STEP_WIDTHS[feature.toLowerCase()] ?? []).includes(Number(px[1]))) {
      off.push(`${feature}: ${value.trim()}`);
    }
  }
  if (/\b(?:width|height)\s*[<>=]/i.test(prelude)) {
    off.push('a range condition');
  }
  return off;
}

/// Whether a prelude has the compact width but not the compact height.
function halfCompact(prelude) {
  const has = (feature, px) => new RegExp(`\\(\\s*${feature}\\s*:\\s*${px}px\\s*\\)`, 'i').test(prelude);
  return has('max-width', 1023) && !has('max-height', 599);
}

/// The text with every comment blanked out, line breaks kept so line numbers hold.
export function stripComments(text) {
  const blank = (m) => m.replace(/[^\n]/g, ' ');
  return text
    .replace(/<!--[\s\S]*?-->/g, blank)
    .replace(/\/\*[\s\S]*?\*\//g, blank)
    .replace(/(^|[^:'"`\\])\/\/[^\n]*/g, (m, lead) => lead + blank(m.slice(lead.length)));
}

/// The findings in one file's text: `{ line, what, text }`.
export function scanText(text, file) {
  const findings = [];
  const lines = stripComments(text).split('\n');
  const stubOnly = path.basename(file) !== 'StubSection.svelte';
  const allowed = LAYOUT_READS_ALLOWED[path.basename(file)] ?? [];
  lines.forEach((line, i) => {
    const at = (what) => findings.push({ line: i + 1, what, text: line.trim() });
    for (const rule of RULES) {
      if (rule.pattern.test(line)) {
        at(rule.what);
      }
    }
    for (const [, value] of line.matchAll(COLOUR_PROPERTY)) {
      const word = value
        .toLowerCase()
        .match(/[a-z]+/g)
        ?.find((w) => NAMED.includes(w));
      if (word && !/var\(--[a-z0-9-]*$/.test(value.toLowerCase().split(word)[0])) {
        at(`the named colour ${word}`);
      }
    }
    if (stubOnly && /var\(\s*--ink-4\s*\)/.test(line)) {
      at('--ink-4 outside StubSection (it is for inactive stub text only)');
    }
    if (FLUID_FONT.test(line)) {
      at('a fluid font size (text steps only with the scale)');
    }
    if (allowed !== 'all') {
      for (const [m, name] of line.matchAll(LAYOUT_READ)) {
        if (!allowed.includes(name ?? m)) {
          at(`layout read in script (${name ?? 'getComputedStyle size'}); only the pitch measures its box`);
        }
      }
    }
  });
  // A prelude may wrap over lines, so it is read whole, up to its brace.
  const stripped = lines.join('\n');
  for (const media of stripped.matchAll(/@media\b([^{]*)/gi)) {
    const line = stripped.slice(0, media.index).split('\n').length;
    const prelude = media[1].replace(/\s+/g, ' ').trim();
    const at = (what) => findings.push({ line, what, text: `@media ${prelude}` });
    for (const feature of offStepFeatures(prelude)) {
      at(`an @media size that is not a window step (${feature})`);
    }
    if (halfCompact(prelude)) {
      at('a compact width without the compact height (max-height: 599px)');
    }
  }
  return findings.sort((a, b) => a.line - b.line);
}

function* files(target) {
  const stat = fs.statSync(target);
  if (stat.isFile()) {
    yield target;
    return;
  }
  for (const entry of fs.readdirSync(target, { withFileTypes: true })) {
    const full = path.join(target, entry.name);
    if (entry.isDirectory()) {
      if (path.resolve(full) !== path.resolve(SKINS)) {
        yield* files(full);
      }
    } else if (EXTENSIONS.has(path.extname(entry.name))) {
      yield full;
    }
  }
}

/// Every finding under `targets`, the skins folder excepted.
export function scan(targets) {
  const all = [];
  for (const target of targets) {
    for (const file of files(target)) {
      for (const finding of scanText(fs.readFileSync(file, 'utf8'), file)) {
        all.push({ file: path.relative(ROOT, file), ...finding });
      }
    }
  }
  return all;
}

if (process.argv[1] && fileURLToPath(import.meta.url) === path.resolve(process.argv[1])) {
  const targets = process.argv.slice(2);
  const findings = scan(targets.length ? targets : [path.join(ROOT, 'src')]);
  for (const f of findings) {
    console.error(`${f.file}:${f.line}: ${f.what}: ${f.text}`);
  }
  console.log(`${findings.length} literal colour or font values or layout rule breaks outside the skins`);
  process.exit(findings.length === 0 ? 0 : 1);
}
