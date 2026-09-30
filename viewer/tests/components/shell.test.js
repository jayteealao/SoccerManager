// @vitest-environment jsdom
// The shell and the shared components, in jsdom: what each renders, how the action block
// and the tabs respond, and that a stub takes no focus.

import assert from 'node:assert/strict';
import { fireEvent, render, screen } from '@testing-library/svelte';
import { createRawSnippet } from 'svelte';
import { test, vi } from 'vitest';

import AppShell from '../../src/components/AppShell.svelte';
import InfoStrip from '../../src/components/InfoStrip.svelte';
import KvRow from '../../src/components/KvRow.svelte';
import Notice from '../../src/components/Notice.svelte';
import PairedBar from '../../src/components/PairedBar.svelte';
import SectionLabel from '../../src/components/SectionLabel.svelte';
import StepList from '../../src/components/StepList.svelte';
import StubSection from '../../src/components/StubSection.svelte';
import SurfacePanel from '../../src/components/SurfacePanel.svelte';

const text = (html) => createRawSnippet(() => ({ render: () => html }));

const TABS = [
  { id: 'match', label: 'Match', active: true },
  { id: 'touchline', label: 'Touchline', stub: true },
  { id: 'tactics', label: 'Tactics', menu: true },
  { id: 'squad', label: 'Squad', menu: true, stub: true },
];

function shell(props = {}) {
  return render(AppShell, {
    title: 'Ashford Rovers 1–1 Port Varrow',
    subtitle: "Heritage Cup · Round 4 · Live 67'",
    date: 'TUE 3 NOVEMBER',
    dateSub: 'Live 20:52',
    action: 'Resume',
    tabs: TABS,
    children: text('<p data-testid="body">stub body</p>'),
    ...props,
  });
}

test('the shell renders the rail, the band, the date block, the action block, the tabs and its body', () => {
  const { container } = shell();
  assert.ok(container.querySelector('nav.rail'));
  const current = screen.getByRole('link', { name: 'Match' });
  assert.equal(current.getAttribute('aria-current'), 'page');
  assert.ok(screen.getByText('Ashford Rovers 1–1 Port Varrow'));
  assert.ok(screen.getByText("Heritage Cup · Round 4 · Live 67'"));
  assert.ok(screen.getByText('TUE 3 NOVEMBER'));
  const action = screen.getByRole('button', { name: 'Resume' });
  assert.equal(action.tagName, 'BUTTON');
  const match = screen.getByRole('button', { name: 'Match' });
  assert.equal(match.getAttribute('aria-current'), 'page');
  assert.ok(match.classList.contains('on'));
  assert.ok(screen.getByRole('button', { name: 'Tactics' }));
  assert.ok(screen.getByTestId('body'));
});

test('the action block fires on click and on Enter, and not while busy', async () => {
  const onaction = vi.fn();
  const { rerender } = shell({ onaction });
  const action = screen.getByRole('button', { name: 'Resume' });
  await fireEvent.click(action);
  assert.equal(onaction.mock.calls.length, 1);
  // A native button turns Enter into a click; jsdom does not, so the key press is followed
  // by the click the browser would send.
  action.focus();
  assert.equal(document.activeElement, action);
  await fireEvent.keyDown(action, { key: 'Enter' });
  await fireEvent.click(action);
  assert.equal(onaction.mock.calls.length, 2);

  await rerender({ busy: true });
  assert.equal(action.disabled, true);
  assert.equal(action.getAttribute('aria-busy'), 'true');
  await fireEvent.click(action);
  assert.equal(onaction.mock.calls.length, 2, 'a busy action block does nothing');
});

test('a live tab reports its id', async () => {
  const ontab = vi.fn();
  shell({ ontab });
  await fireEvent.click(screen.getByRole('button', { name: 'Tactics' }));
  assert.deepEqual(ontab.mock.calls, [['tactics']]);
});

test('every stub in the shell is inert and hidden, and none of its parts takes focus', () => {
  const { container } = shell();
  const stubs = [...container.querySelectorAll('[data-stub]')];
  assert.ok(stubs.length >= 20, `${stubs.length} stubs`);
  for (const stub of stubs) {
    assert.ok(stub.hasAttribute('inert'), stub.dataset.stub);
    assert.equal(stub.getAttribute('aria-hidden'), 'true', stub.dataset.stub);
    assert.equal(stub.querySelector('a, button, input, select, textarea, [tabindex]'), null);
  }
  assert.ok(container.querySelector('[data-stub="tab: touchline"]'));
  assert.ok(container.querySelector('[data-stub="rail section: home"]'));
  // Only the current section, the two live tabs and the action block are focus stops.
  const stops = [...container.querySelectorAll('a[href], button:not([disabled])')];
  assert.deepEqual(
    stops.map((el) => el.textContent.trim() || el.getAttribute('aria-label')),
    ['Match', 'Resume', 'Match', 'Tactics']
  );
});

test('a StubSection fades its content, labels it LATER, and hides it from focus', () => {
  const { container } = render(StubSection, {
    note: 'playback row',
    later: true,
    children: text('<button type="button">Play</button>'),
  });
  const stub = container.querySelector('[data-stub="playback row"]');
  assert.ok(stub.classList.contains('fade'));
  assert.ok(stub.hasAttribute('inert'));
  assert.equal(stub.getAttribute('aria-hidden'), 'true');
  assert.ok(stub.textContent.includes('LATER'));
  const inside = stub.querySelector('button');
  inside.focus();
  // jsdom does not implement `inert`, so the browser walk in the shell spec is the proof
  // that focus skips it; here the attribute and the hidden state are what is held.
  assert.ok(inside.closest('[inert]'));
  assert.equal(screen.queryByRole('button', { name: 'Play' }), null, 'hidden from the tree');
});

test('a notice and a surface panel name their kind in words, never colour alone', () => {
  render(Notice, { kind: 'warn', message: 'The connection to the engine dropped. Reconnecting.' });
  const status = screen.getByRole('status');
  assert.ok(status.textContent.includes('RECONNECTING'));
  assert.ok(status.classList.contains('warn'));

  render(Notice, { kind: 'bad', message: 'The match is no longer live.' });
  assert.ok(screen.getByRole('alert').textContent.includes('STREAM ENDED'));

  const { container } = render(SurfacePanel, {
    kind: 'refusal',
    title: 'This saved match cannot resume',
    children: text('<p>It was saved by engine 0.1.0.</p>'),
  });
  const panel = container.querySelector('.surface');
  assert.equal(panel.getAttribute('role'), 'alert');
  assert.ok(panel.textContent.includes('Cannot resume'));
  assert.ok(panel.querySelector('h2').textContent.includes('cannot resume'));
});

test('a step list marks done, current and pending with a mark and a word', () => {
  const { container } = render(StepList, {
    steps: [
      { label: 'Starting the engine', state: 'done' },
      { label: 'Connecting to the match', state: 'current', progress: 62 },
      { label: 'Waiting for kick-off', state: 'pending' },
    ],
  });
  const items = [...container.querySelectorAll('li')];
  assert.deepEqual(
    items.map((li) => li.querySelector('.mark').textContent),
    ['✓', '●', '○']
  );
  assert.deepEqual(
    items.map((li) => li.querySelector('.word').textContent),
    ['Done', 'In progress', 'Waiting']
  );
  assert.equal(items[1].querySelector('.bar i').style.width, '62%');
  assert.equal(items[2].querySelector('.bar i').style.width, '0%');
});

test('the strip, the label, the key-value row and the paired bar render their values', () => {
  const { container } = render(InfoStrip, {
    facts: [
      { value: 'Ashford Rovers v Port Varrow', label: 'Heritage Cup' },
      { value: '1 – 1', label: 'Score', num: true },
    ],
  });
  const cells = container.querySelectorAll('.cell');
  assert.equal(cells.length, 2);
  assert.ok(cells[1].querySelector('b').classList.contains('num'));

  render(SectionLabel, { label: 'Match figures', note: 'ASH · VAR' });
  assert.ok(screen.getByRole('heading', { name: /Match figures/ }));

  render(KvRow, { key: 'Possession', value: '54%' });
  assert.ok(screen.getByText('54%').classList.contains('num'));

  const bar = render(PairedBar, { label: 'Corners', home: 8, away: 3 }).container;
  assert.equal(bar.querySelector('.home').style.flexGrow, '73');
  assert.equal(bar.querySelector('.away').style.flexGrow, '27');
  const even = render(PairedBar, { label: 'Penalties', home: 0, away: 0 }).container;
  assert.equal(even.querySelector('.home').style.flexGrow, '50');
});

test('a tall strip leads with its crest, and a LATER fact, a LATER label and an action note are inert stubs', () => {
  const { container } = render(InfoStrip, {
    tall: true,
    lead: text('<i data-testid="lead">crest</i>'),
    facts: [
      { value: 'Ashford Rovers v Port Varrow', label: 'Heritage Cup' },
      { value: '12°C · dry', label: 'Weather', stub: true },
    ],
  });
  const strip = container.querySelector('.strip');
  assert.ok(strip.classList.contains('tall'));
  assert.ok(container.querySelector('.cell:first-child [data-testid="lead"]'), 'the lead sits in the first cell');
  const later = container.querySelector('[data-stub="strip fact: Weather"]');
  assert.ok(later.hasAttribute('inert'));
  assert.ok(later.textContent.includes('LATER'));

  const label = render(SectionLabel, { label: 'Analyst reads', later: true }).container;
  const mark = label.querySelector('[data-stub="LATER mark: Analyst reads"]');
  assert.ok(mark.hasAttribute('inert'));
  assert.equal(mark.getAttribute('aria-hidden'), 'true');

  const { container: band } = shell({ actionNote: 'Team talk', navLabel: 'Matchday views' });
  const note = band.querySelector('[data-stub="action note: Team talk"]');
  assert.ok(note.hasAttribute('inert'));
  assert.ok(note.textContent.includes('LATER'));
  assert.ok(screen.getByRole('navigation', { name: 'Matchday views' }));
});
