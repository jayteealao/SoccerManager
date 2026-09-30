// The viewer's entry: applies the skin the `viewer.skin` slot names, then mounts the views
// and starts their session. Until the served page switches to this viewer, the engine
// serves the page in `web/` to players; the browser tests serve this build with `--web`.

import { mount } from 'svelte';

import App from './App.svelte';
import { watchMotion } from './lib/goal-moment.js';
import { MatchSession } from './lib/match-session.svelte.js';
import { install } from './lib/test-hooks.js';
import { applySkin, engineSkin } from './skins/index.js';

const skin = await applySkin(await engineSkin());
watchMotion(document, globalThis);

const session = new MatchSession();
install(session);
mount(App, { target: document.getElementById('app'), props: { session } });
document.documentElement.dataset.ready = skin;
session.start();
