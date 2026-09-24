#!/usr/bin/env bash
# Fetches the MakeHuman base mesh and the targets this PoC uses.
# Source: https://github.com/makehumancommunity/makehuman (assets are CC0 1.0,
# see LICENSE.ASSETS.md in that repository). Pinned to one commit so the
# genome -> mesh mapping is reproducible.
set -euo pipefail
HERE="$(cd "$(dirname "$0")/.." && pwd)"
DEST="$HERE/downloads/makehuman"
COMMIT="a8bc2d54ff0ac92e78ff71431b1023eda42bf482"
TMP="$HERE/downloads/.mh-clone"

if [ -f "$DEST/3dobjs/base.obj" ]; then
  echo "MakeHuman assets already present in $DEST"
  exit 0
fi

rm -rf "$TMP"
git clone --filter=blob:none --no-checkout https://github.com/makehumancommunity/makehuman.git "$TMP"
git -C "$TMP" sparse-checkout set --no-cone \
  /LICENSE.md /LICENSE.ASSETS.md \
  /makehuman/data/3dobjs/base.obj \
  /makehuman/data/targets/macrodetails/ \
  /makehuman/data/targets/head/ /makehuman/data/targets/chin/ /makehuman/data/targets/nose/ \
  /makehuman/data/targets/mouth/ /makehuman/data/targets/cheek/ /makehuman/data/targets/eyebrows/ \
  /makehuman/data/targets/forehead/ /makehuman/data/targets/ears/ /makehuman/data/targets/neck/ \
  /makehuman/data/targets/eyes/
git -C "$TMP" checkout "$COMMIT"

mkdir -p "$DEST"
cp "$TMP/LICENSE.md" "$TMP/LICENSE.ASSETS.md" "$DEST/"
cp -r "$TMP/makehuman/data/3dobjs" "$DEST/"
cp -r "$TMP/makehuman/data/targets" "$DEST/"
find "$DEST/targets" -name images -type d -prune -exec rm -rf {} +
rm -rf "$TMP"
echo "MakeHuman CC0 assets written to $DEST"
