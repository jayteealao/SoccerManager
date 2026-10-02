// The viewer's entry. The splash is mounted at once in the default look; when the launcher
// answers `/engine.json`, the skin the `viewer.skin` slot names is applied, the player's
// settings take effect, and the page goes on: to the start screen when the launcher opened
// on it (`front-door: true`), or straight into the match session as before. `data-ready`
// names the skin once the answer is in. The handshake page is a separate entry
// (`handshake.js`) with no session.

import { mount } from 'svelte';

import Root from './Root.svelte';
import { FrontDoor } from './lib/front-door.svelte.js';
import { watchMotion } from './lib/goal-moment.js';
import { fetchStatus } from './lib/launcher.js';
import { install } from './lib/test-hooks.js';
import { DEFAULT_SKIN, applySkin, pick } from './skins/index.js';

await applySkin(DEFAULT_SKIN);
watchMotion(document, globalThis);

const query = globalThis.matchMedia?.('(prefers-reduced-motion: reduce)');
let door = null;
door = new FrontDoor({ onSession: (session) => install(session, globalThis, () => door) });
mount(Root, {
  target: document.getElementById('app'),
  props: { door, systemReduces: () => Boolean(query?.matches) },
});

const status = await fetchStatus();
const skin = await applySkin(pick(status?.['viewer.skin']));
door.answer(status);
document.documentElement.dataset.ready = skin;
