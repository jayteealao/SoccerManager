// The open-source notices: the crate closure, the licence check and its planted failures, the
// clarifications, the npm packages of a bundle, and the check of a written file.

import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { test } from 'vitest';

import {
  assemble,
  crateClosure,
  crateNotice,
  fontNotices,
  licenceAllowed,
  packageDir,
  parseAllow,
  problems,
  readClarify,
  repoAllow,
  sourceImports,
  verify,
} from '../scripts/notices.mjs';
import { REPO_ROOT } from './helpers.js';

const DATA = path.join(REPO_ROOT, 'viewer/tests/data/notices');

function metadata() {
  const text = fs.readFileSync(path.join(DATA, 'metadata.json'), 'utf8');
  return JSON.parse(text.replaceAll('<dir>', DATA.replace(/\\/g, '/')));
}

const ALLOW = parseAllow(fs.readFileSync(path.join(REPO_ROOT, 'deny.toml'), 'utf8'));

test('the allow list is deny.toml [licenses].allow', () => {
  assert.ok(ALLOW.includes('MIT'));
  assert.ok(ALLOW.includes('Apache-2.0'));
  assert.ok(ALLOW.includes('Apache-2.0 WITH LLVM-exception'));
  assert.ok(!ALLOW.includes('GPL-3.0-only'));
  assert.deepEqual(repoAllow(), ALLOW);
});

test('an expression passes when one alternative is allowed, and AND needs both', () => {
  assert.equal(licenceAllowed('MIT OR Apache-2.0', ALLOW), true);
  assert.equal(licenceAllowed('GPL-3.0-only OR MIT', ALLOW), true);
  assert.equal(licenceAllowed('MIT AND GPL-3.0-only', ALLOW), false);
  assert.equal(licenceAllowed('(MIT OR Apache-2.0) AND Unicode-3.0', ALLOW), true);
  assert.equal(licenceAllowed('Apache-2.0 WITH LLVM-exception', ALLOW), true);
  assert.equal(licenceAllowed('MIT/Apache-2.0', ALLOW), true);
  assert.equal(licenceAllowed('GPL-3.0-only', ALLOW), false);
  assert.equal(licenceAllowed('', ALLOW), false);
  assert.equal(licenceAllowed('OFL-1.1', ALLOW), false, 'OFL is allowed for fonts only');
});

test('the closure follows normal dependencies only and leaves the workspace out', () => {
  const names = crateClosure(metadata()).map((p) => p.name);
  assert.deepEqual(names, ['bare', 'good', 'gpl']);
});

test('a crate with no licence text and a crate with a disallowed licence fail the build, by name', () => {
  const crates = crateClosure(metadata()).map((p) => crateNotice(p, new Map()));
  assert.throws(
    () => assemble({ version: '0', crates, allow: ALLOW }),
    (error) => {
      assert.match(error.message, /bare 2\.0\.0 \(crate\) has no licence text/);
      assert.match(error.message, /gpl 3\.1\.0 \(crate\) has the licence 'GPL-3\.0-only'/);
      assert.doesNotMatch(error.message, /good/);
      assert.equal(error.problems.length, 2);
      return true;
    }
  );
});

test('a reviewed clarification gives a crate the standard text with its authors', () => {
  const clarify = readClarify(path.join(DATA, 'clarify.json'));
  const bare = crateClosure(metadata()).find((p) => p.name === 'bare');
  const notice = crateNotice(bare, clarify);
  assert.equal(notice.licence, 'MIT');
  assert.match(notice.text, /^MIT License\n\nCopyright \(c\) Bare Author/);
  assert.ok(notice.clarified);
  assert.deepEqual(problems([notice], ALLOW), []);
});

test('a crate that ships its licence carries the file text', () => {
  const good = crateClosure(metadata()).find((p) => p.name === 'good');
  const notice = crateNotice(good);
  assert.match(notice.text, /Copyright \(c\) The good crate authors/);
  assert.equal(notice.description, 'A crate that ships its licence');
});

test('the repository clarifies exactly the three crates that ship no licence file', () => {
  const clarify = readClarify();
  assert.deepEqual([...clarify.keys()].sort(), ['garde@0.23.0', 'garde_derive@0.23.0', 'rhai_codegen@3.2.0']);
  for (const entry of clarify.values()) {
    assert.ok(entry.reason, `${entry.name} names why`);
    assert.equal(entry.text, 'MIT');
  }
});

test('a bundled module id names its package folder, scoped or not', () => {
  assert.equal(packageDir('/a/node_modules/svelte/src/internal/client/index.js'), '/a/node_modules/svelte');
  assert.equal(packageDir('C:\\a\\node_modules\\@scope\\pkg\\x.js'), 'C:/a/node_modules/@scope/pkg');
  assert.equal(packageDir('/a/node_modules/x/node_modules/y/i.js'), '/a/node_modules/x/node_modules/y');
  assert.equal(packageDir('/a/viewer/src/main.js'), null);
});

test('the font licences are listed as OFL-1.1, which passes for fonts only', () => {
  const fonts = fontNotices();
  assert.ok(fonts.some((f) => f.name === 'Saira'));
  assert.ok(fonts.every((f) => f.licence === 'OFL-1.1' && f.text.includes('SIL OPEN FONT LICENSE')));
  assert.deepEqual(problems(fonts, ALLOW), []);
});

test('verify names a shipped crate, an import and a font the file leaves out', () => {
  const file = {
    packages: [{ kind: 'crate', name: 'good', version: '1.0.0', licence: 'MIT', text: 'MIT License' }],
  };
  const found = verify(file, {
    metadata: metadata(),
    previous: [{ name: 'old', version: '0.1.0' }],
    imports: ['svelte'],
    fonts: [{ name: 'Saira', version: null }],
    allow: ALLOW,
  });
  assert.ok(found.some((f) => f.includes('bare 2.0.0 ships in the program')));
  assert.ok(found.some((f) => f.includes('gpl 3.1.0 ships in the program')));
  assert.ok(found.some((f) => f.includes('old 0.1.0 ships in the previous engine')));
  assert.ok(found.some((f) => f.includes('imports svelte')));
  assert.ok(found.some((f) => f.includes('Saira')));
});

test('the viewer source imports only packages the bundle carries', () => {
  assert.deepEqual(sourceImports(), ['svelte']);
});
