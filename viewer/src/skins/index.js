// The skin loader. A skin is a folder of token, style and font files; the `viewer.skin`
// slot names the folder, and the engine reports the name in `/engine.json`. The loader sets
// `data-theme` on <html> and loads that skin's styles. Components read only `var(--…)`, so
// a skin swap changes values, never code.

import './base.css';

/// The skins this build ships, in the slot registry's order. `crates/engine/tests/viewer_skin.rs`
/// holds the engine's skin names equal to these folders.
export const SKINS = ['broadcast-blue', 'interim-light'];

/// The look shown when no skin is named, or when the name is not one this build ships.
export const DEFAULT_SKIN = 'broadcast-blue';

const loaders = {
  'broadcast-blue': () =>
    Promise.all([import('./broadcast-blue/tokens.css'), import('./broadcast-blue/skin.css')]),
  'interim-light': () =>
    Promise.all([import('./interim-light/tokens.css'), import('./interim-light/skin.css')]),
};

/// `name` when this build ships it, otherwise the default look.
export function pick(name) {
  return SKINS.includes(name) ? name : DEFAULT_SKIN;
}

/// The skin `/engine.json` names, or the default look when the key is absent or the page
/// cannot reach the engine.
export async function engineSkin(fetchImpl = globalThis.fetch) {
  try {
    const status = await (await fetchImpl('/engine.json', { cache: 'no-store' })).json();
    return pick(status['viewer.skin']);
  } catch {
    return DEFAULT_SKIN;
  }
}

/// Applies `name`: loads its styles and sets `data-theme` on the root element. Resolves once
/// the styles are in and the skin's fonts have loaded, so the first frame is drawn in the
/// skin's own faces.
export async function applySkin(name, doc = globalThis.document) {
  const skin = pick(name);
  await loaders[skin]();
  doc.documentElement.dataset.theme = skin;
  await doc.fonts?.ready;
  return skin;
}
