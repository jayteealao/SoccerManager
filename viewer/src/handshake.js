// The handshake page's entry: applies the skin the `viewer.skin` slot names, mounts the
// page and runs the check once. RUN AGAIN runs it again. The page opens the engine's socket,
// prints the hello and a sample of the tick frames, and renders no pitch.

import { mount } from 'svelte';

import { HandshakeRun } from './lib/handshake-run.svelte.js';
import HandshakeScreen from './screens/HandshakeScreen.svelte';
import { applySkin, engineSkin } from './skins/index.js';

const skin = await applySkin(await engineSkin());
const run = new HandshakeRun();
mount(HandshakeScreen, { target: document.getElementById('app'), props: { run } });
document.documentElement.dataset.ready = skin;
run.run();
