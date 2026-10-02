// The open-source notices the game ships: every crate the engine program is built from, every
// npm package inside the viewer's bundle, and the fonts' licences, each with its licence text.
// The viewer build writes them to `notices.json` beside the page (the `notices()` plugin in
// vite.config.js); the licences screen reads that file, and the release carries it in `web/`.
//
// The build fails, naming each package, when a package has no licence text or no licence the
// project allows. The allow list is `deny.toml` `[licenses].allow` (crates and npm packages);
// OFL-1.1 is allowed for fonts only. A crate that ships no licence file needs a reviewed entry
// in `packaging/notices/clarify.json`, which names its licence, the reason, and the standard
// text it takes.
//
// Usage:
//   node scripts/notices.mjs crates --manifest-path <Cargo.toml> --out <file>
//       writes the crate list (with texts) of that workspace's engine-cli, for the previous
//       release's program, whose worktree is gone by the time the viewer is built;
//   node scripts/notices.mjs verify <notices.json>
//       checks a written file again from the sources: every crate of today's `cargo metadata`
//       closure (and of SM_PREVIOUS_NOTICES, when set), every package the viewer's source
//       imports, and every font licence is listed with an allowed licence and its text.

import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = path.dirname(fileURLToPath(import.meta.url));
export const VIEWER = path.resolve(HERE, '..');
export const REPO = path.resolve(VIEWER, '..');

/// The licence files a package ships, by name.
const LICENCE_FILE = /^(licen[cs]e|copying|notice|unlicense|copyright)([-._].*)?$/i;

/// The standard texts a clarified crate takes, with `<authors>` for its copyright holders.
export const STANDARD_TEXTS = {
  MIT: `MIT License

Copyright (c) <authors>

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
`,
};

// ---- Licences --------------------------------------------------------------------------------

/// The `[licenses] allow` list of a `deny.toml` text.
export function parseAllow(toml) {
  const section = toml.match(/^\[licenses\]\s*$([\s\S]*?)(?=^\[)/m)?.[1] ?? '';
  const list = section.match(/^allow\s*=\s*\[([\s\S]*?)\]/m)?.[1] ?? '';
  return [...list.matchAll(/"([^"]+)"/g)].map((m) => m[1]);
}

/// Whether an SPDX expression passes `allow`: an OR passes when one side passes, an AND when
/// both do; `A WITH B` passes when the whole `A WITH B` or `A` alone is allowed. The old `/`
/// separator reads as OR.
export function licenceAllowed(expression, allow) {
  const text = String(expression ?? '').trim();
  if (!text) {
    return false;
  }
  const tokens = text.replace(/\//g, ' OR ').match(/\(|\)|[^\s()]+/g) ?? [];
  let at = 0;
  const allowed = new Set(allow);
  function primary() {
    if (tokens[at] === '(') {
      at += 1;
      const value = or();
      at += 1; // ')'
      return value;
    }
    const id = tokens[at++] ?? '';
    if (tokens[at] === 'WITH') {
      const exception = tokens[at + 1] ?? '';
      at += 2;
      return allowed.has(`${id} WITH ${exception}`) || allowed.has(id);
    }
    return allowed.has(id) || allowed.has(id.replace(/\+$/, ''));
  }
  function and() {
    let value = primary();
    while (tokens[at] === 'AND') {
      at += 1;
      value = primary() && value;
    }
    return value;
  }
  function or() {
    let value = and();
    while (tokens[at] === 'OR') {
      at += 1;
      value = and() || value;
    }
    return value;
  }
  return or();
}

/// The licence files in `dir`, joined: each under its file name.
export function licenceText(dir) {
  let names = [];
  try {
    names = fs
      .readdirSync(dir, { withFileTypes: true })
      .filter((e) => e.isFile() && LICENCE_FILE.test(e.name))
      .map((e) => e.name)
      .sort();
  } catch {
    return '';
  }
  return names
    .map((name) => {
      const body = fs.readFileSync(path.join(dir, name), 'utf8').replace(/\r\n/g, '\n').trim();
      return names.length > 1 ? `${name}\n\n${body}` : body;
    })
    .join('\n\n');
}

// ---- Crates ----------------------------------------------------------------------------------

/// The host triple of the Rust toolchain, the platform the build's own program runs on.
export function hostTriple() {
  const out = execFileSync('rustc', ['-vV'], { encoding: 'utf8' });
  return out.match(/^host:\s*(\S+)/m)[1];
}

/// `cargo metadata` for `manifestPath`, filtered to `platform`.
export function cargoMetadata(manifestPath = path.join(REPO, 'Cargo.toml'), platform = hostTriple()) {
  const out = execFileSync(
    'cargo',
    ['metadata', '--format-version', '1', '--locked', '--manifest-path', manifestPath, '--filter-platform', platform],
    { encoding: 'utf8', maxBuffer: 256 * 1024 * 1024, stdio: ['ignore', 'pipe', 'inherit'] }
  );
  return JSON.parse(out);
}

/// The packages `root`'s program is built from: its normal-dependency closure, workspace
/// crates left out, ordered by name and version.
export function crateClosure(metadata, root = 'engine-cli') {
  const workspace = new Set(metadata.workspace_members);
  const byId = new Map(metadata.packages.map((p) => [p.id, p]));
  const nodes = new Map(metadata.resolve.nodes.map((n) => [n.id, n]));
  const start = metadata.packages.find((p) => p.name === root && workspace.has(p.id));
  if (!start) {
    throw new Error(`cargo metadata names no workspace package ${root}`);
  }
  const seen = new Set();
  const stack = [start.id];
  while (stack.length) {
    const id = stack.pop();
    if (seen.has(id)) {
      continue;
    }
    seen.add(id);
    for (const dep of nodes.get(id)?.deps ?? []) {
      if (dep.dep_kinds.some((k) => k.kind === null)) {
        stack.push(dep.pkg);
      }
    }
  }
  return [...seen]
    .filter((id) => !workspace.has(id))
    .map((id) => byId.get(id))
    .sort((a, b) => a.name.localeCompare(b.name) || a.version.localeCompare(b.version));
}

/// The clarifications of `packaging/notices/clarify.json`, keyed `name@version`.
export function readClarify(file = path.join(REPO, 'packaging', 'notices', 'clarify.json')) {
  const body = JSON.parse(fs.readFileSync(file, 'utf8'));
  return new Map((body.crates ?? []).map((c) => [`${c.name}@${c.version}`, c]));
}

/// One crate's notice: its licence text from its own files, or from its clarification.
export function crateNotice(pkg, clarify = new Map()) {
  const dir = path.dirname(pkg.manifest_path);
  let text = licenceText(dir);
  if (!text && pkg.license_file) {
    try {
      text = fs.readFileSync(path.resolve(dir, pkg.license_file), 'utf8').replace(/\r\n/g, '\n').trim();
    } catch {
      text = '';
    }
  }
  let licence = pkg.license ?? '';
  let clarified = null;
  if (!text) {
    clarified = clarify.get(`${pkg.name}@${pkg.version}`) ?? null;
    if (clarified) {
      licence = clarified.licence ?? licence;
      const standard = STANDARD_TEXTS[clarified.text];
      const authors = (pkg.authors ?? []).join(', ') || `the ${pkg.name} authors`;
      text = standard ? standard.replace('<authors>', authors).trim() : '';
    }
  }
  return {
    kind: 'crate',
    name: pkg.name,
    version: pkg.version,
    licence,
    description: oneLine(pkg.description),
    text,
    ...(clarified ? { clarified: clarified.reason } : {}),
  };
}

/// The crate notices of the workspace at `manifestPath`.
export function crateNotices({ manifestPath, platform, clarify = readClarify() } = {}) {
  return crateClosure(cargoMetadata(manifestPath, platform)).map((pkg) => crateNotice(pkg, clarify));
}

// ---- npm packages ----------------------------------------------------------------------------

/// The package a bundled module id belongs to: its folder under the last `node_modules`, or
/// null for the viewer's own source.
export function packageDir(moduleId) {
  const id = moduleId.replace(/\\/g, '/').replace(/^\0/, '').split('?')[0];
  const at = id.lastIndexOf('/node_modules/');
  if (at < 0) {
    return null;
  }
  const rest = id.slice(at + '/node_modules/'.length).split('/');
  const name = rest[0].startsWith('@') ? `${rest[0]}/${rest[1]}` : rest[0];
  return `${id.slice(0, at)}/node_modules/${name}`;
}

/// One npm package's notice, from its folder.
export function npmNotice(dir) {
  const manifest = JSON.parse(fs.readFileSync(path.join(dir, 'package.json'), 'utf8'));
  const licence = typeof manifest.license === 'string' ? manifest.license : (manifest.license?.type ?? '');
  return {
    kind: 'npm',
    name: manifest.name,
    version: manifest.version,
    licence,
    description: oneLine(manifest.description),
    text: licenceText(dir),
  };
}

/// The notices of the npm packages the bundled modules come from.
export function npmNotices(moduleIds) {
  const dirs = new Set();
  for (const id of moduleIds) {
    const dir = packageDir(id);
    if (dir) {
      dirs.add(dir);
    }
  }
  return [...dirs].map(npmNotice).sort((a, b) => a.name.localeCompare(b.name));
}

// ---- Fonts -----------------------------------------------------------------------------------

/// The fonts' notices: one per `OFL-*.txt` in a skin's `fonts/`, named from the file.
export function fontNotices(skins = path.join(VIEWER, 'src', 'skins')) {
  const found = new Map();
  for (const skin of fs.readdirSync(skins, { withFileTypes: true })) {
    const folder = path.join(skins, skin.name, 'fonts');
    if (!skin.isDirectory() || !fs.existsSync(folder)) {
      continue;
    }
    for (const name of fs.readdirSync(folder).sort()) {
      const match = name.match(/^OFL-(.*)\.txt$/);
      if (!match || found.has(name)) {
        continue;
      }
      found.set(name, {
        kind: 'font',
        name: match[1].replace(/-/g, ' '),
        version: null,
        licence: 'OFL-1.1',
        description: `Screen text (${skin.name} skin)`,
        text: fs.readFileSync(path.join(folder, name), 'utf8').replace(/\r\n/g, '\n').trim(),
      });
    }
  }
  return [...found.values()];
}

// ---- The file --------------------------------------------------------------------------------

/// Every problem in `packages`: a package with no licence text, or with no allowed licence.
/// Fonts may also take OFL-1.1.
export function problems(packages, allow) {
  const found = [];
  for (const p of packages) {
    const id = p.version ? `${p.name} ${p.version}` : p.name;
    if (!p.text || !p.text.trim()) {
      found.push(`${id} (${p.kind}) has no licence text; add a reviewed entry to packaging/notices/clarify.json`);
    }
    const list = p.kind === 'font' ? [...allow, 'OFL-1.1'] : allow;
    if (!licenceAllowed(p.licence, list)) {
      found.push(`${id} (${p.kind}) has the licence '${p.licence || 'none'}', which deny.toml does not allow`);
    }
  }
  return found;
}

/// The notices file: the fonts, the npm packages, this program's crates, then the previous
/// release's crates not already listed. Throws, naming every problem, when one has any.
export function assemble({ version, fonts = [], npm = [], crates = [], previous = [], allow }) {
  const listed = new Set();
  const packages = [];
  const add = (p, extra = {}) => {
    const key = `${p.kind}:${p.name}@${p.version}`;
    if (!listed.has(key)) {
      listed.add(key);
      packages.push({ ...p, ...extra });
    }
  };
  fonts.forEach((p) => add(p));
  npm.forEach((p) => add(p));
  crates.forEach((p) => add(p));
  previous.forEach((p) => add(p, { previous: true }));
  const found = problems(packages, allow);
  if (found.length) {
    const error = new Error(`the open-source notices are incomplete:\n  ${found.join('\n  ')}`);
    error.problems = found;
    throw error;
  }
  return { version, packages };
}

/// The allow list of the repository's `deny.toml`.
export function repoAllow() {
  return parseAllow(fs.readFileSync(path.join(REPO, 'deny.toml'), 'utf8'));
}

/// The previous release's crate list, when SM_PREVIOUS_NOTICES names one.
export function previousCrates(file = process.env.SM_PREVIOUS_NOTICES) {
  if (!file) {
    return [];
  }
  return JSON.parse(fs.readFileSync(file, 'utf8')).crates ?? [];
}

/// The bare package names the viewer's source imports.
export function sourceImports(dir = path.join(VIEWER, 'src')) {
  const names = new Set();
  const walk = (folder) => {
    for (const entry of fs.readdirSync(folder, { withFileTypes: true })) {
      const full = path.join(folder, entry.name);
      if (entry.isDirectory()) {
        walk(full);
      } else if (/\.(js|mjs|svelte)$/.test(entry.name)) {
        const text = fs.readFileSync(full, 'utf8');
        for (const m of text.matchAll(/\bfrom\s+['"]([^'"]+)['"]|\bimport\s*\(?\s*['"]([^'"]+)['"]/g)) {
          const spec = m[1] ?? m[2];
          if (spec.startsWith('.') || spec.startsWith('/')) {
            continue;
          }
          const parts = spec.split('/');
          names.add(spec.startsWith('@') ? `${parts[0]}/${parts[1]}` : parts[0]);
        }
      }
    }
  };
  walk(dir);
  return [...names].sort();
}

/// Checks a written notices file again from the sources; returns every problem found.
export function verify(file, { metadata = null, previous = previousCrates(), imports = sourceImports(), fonts = fontNotices(), allow = repoAllow() } = {}) {
  const found = [...problems(file.packages ?? [], allow)];
  const listed = new Set((file.packages ?? []).map((p) => `${p.kind}:${p.name}@${p.version}`));
  const names = new Set((file.packages ?? []).filter((p) => p.kind === 'npm').map((p) => p.name));
  const closure = crateClosure(metadata ?? cargoMetadata());
  for (const pkg of closure) {
    if (!listed.has(`crate:${pkg.name}@${pkg.version}`)) {
      found.push(`the crate ${pkg.name} ${pkg.version} ships in the program but is not listed`);
    }
  }
  for (const pkg of previous) {
    if (!listed.has(`crate:${pkg.name}@${pkg.version}`)) {
      found.push(`the crate ${pkg.name} ${pkg.version} ships in the previous engine but is not listed`);
    }
  }
  for (const name of imports) {
    if (!names.has(name)) {
      found.push(`the viewer imports ${name}, which is not listed`);
    }
  }
  for (const font of fonts) {
    if (!listed.has(`font:${font.name}@${font.version}`)) {
      found.push(`the font licence of ${font.name} ships but is not listed`);
    }
  }
  return found;
}

function oneLine(text) {
  return String(text ?? '')
    .replace(/\s+/g, ' ')
    .trim();
}

// ---- Command line ----------------------------------------------------------------------------

function option(args, name) {
  const at = args.indexOf(name);
  return at >= 0 ? args[at + 1] : undefined;
}

async function main(args) {
  const [command, ...rest] = args;
  if (command === 'crates') {
    const manifestPath = path.resolve(option(rest, '--manifest-path') ?? path.join(REPO, 'Cargo.toml'));
    const out = option(rest, '--out');
    const crates = crateNotices({ manifestPath });
    const found = problems(crates, repoAllow());
    if (found.length) {
      console.error(`the open-source notices are incomplete:\n  ${found.join('\n  ')}`);
      return 1;
    }
    const body = `${JSON.stringify({ crates }, null, 1)}\n`;
    if (out) {
      fs.writeFileSync(out, body);
      console.log(`${crates.length} crates written to ${out}`);
    } else {
      process.stdout.write(body);
    }
    return 0;
  }
  if (command === 'verify') {
    const target = rest.find((a) => !a.startsWith('-'));
    if (!target) {
      console.error('usage: notices.mjs verify <notices.json>');
      return 2;
    }
    const file = JSON.parse(fs.readFileSync(path.resolve(target), 'utf8'));
    const found = verify(file);
    if (found.length) {
      console.error(`${target}: ${found.length} problem(s):\n  ${found.join('\n  ')}`);
      return 1;
    }
    const counts = {};
    for (const p of file.packages) {
      counts[p.kind] = (counts[p.kind] ?? 0) + 1;
    }
    console.log(`${target}: ${file.packages.length} packages listed (${Object.entries(counts).map(([k, n]) => `${n} ${k}`).join(', ')}); every shipped package has an allowed licence and its text`);
    return 0;
  }
  console.error('usage: notices.mjs crates [--manifest-path <Cargo.toml>] [--out <file>] | verify <notices.json>');
  return 2;
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  process.exitCode = await main(process.argv.slice(2));
}
