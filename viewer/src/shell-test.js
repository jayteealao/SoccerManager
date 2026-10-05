// The shell test page: the shell with a stub body, in the skin `?skin=` names (the default
// look otherwise). The screenshots and the keyboard walk open this page; players never see it.

import { mount } from 'svelte';

import ShellTest from './ShellTest.svelte';
import { applySkin } from './skins/index.js';

const skin = await applySkin(new URLSearchParams(location.search).get('skin'));
mount(ShellTest, { target: document.getElementById('app') });
document.documentElement.dataset.ready = skin;
