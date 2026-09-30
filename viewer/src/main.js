// The viewer's entry. Until the screens are ported the engine serves the page in `web/`;
// this entry already applies the skin the `viewer.skin` slot names.

import { applySkin, engineSkin } from './skins/index.js';

applySkin(await engineSkin());
