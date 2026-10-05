// Where focus goes when an overlay closes. Pure apart from the DOM it is given.

/// Returns focus to `before`, the element that held it when an overlay opened, once the
/// overlay has closed. When that element has left the page, or focus already moved
/// somewhere other than the page body, the menu button takes it, or nothing does.
export function restoreFocus(before, doc = globalThis.document) {
  if (!doc) {
    return;
  }
  queueMicrotask(() => {
    const active = doc.activeElement;
    if (active && active !== doc.body && active.isConnected) {
      return;
    }
    const target = before?.isConnected ? before : doc.querySelector('[data-menu-button]');
    target?.focus?.();
  });
}
