// @vitest-environment jsdom
// The page's fit to the window, in jsdom: below 768 px wide the page zooms down by the width
// alone; a short window keeps its size and scrolls, so browser zoom still grows the text.

import assert from 'node:assert/strict';
import { render } from '@testing-library/svelte';
import { afterEach, test } from 'vitest';

import Root from '../../src/Root.svelte';

const door = { view: 'none', overlay: null, session: null };
const start = { width: window.innerWidth, height: window.innerHeight };

afterEach(() => {
  window.innerWidth = start.width;
  window.innerHeight = start.height;
});

function zoomAt(width, height) {
  window.innerWidth = width;
  window.innerHeight = height;
  const { container, unmount } = render(Root, { door });
  const zoom = container.querySelector('.fit').style.zoom;
  unmount();
  return zoom;
}

test('below 768 px wide the page zooms by the width alone', () => {
  assert.equal(Number(zoomAt(384, 300)), 0.5);
  assert.equal(Number(zoomAt(683, 384)), 683 / 768);
});

test('a short window keeps its size and scrolls instead of shrinking', () => {
  assert.equal(zoomAt(1280, 560), '');
  assert.equal(zoomAt(1366, 384), '');
  assert.equal(zoomAt(1280, 800), '');
});
