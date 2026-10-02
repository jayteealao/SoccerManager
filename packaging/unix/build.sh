#!/bin/sh
# Builds the Linux (or macOS) release archive: the engine, the content, the built viewer, the
# licences, the `soccermanager` start script, and the previous release's engine in
# `previous/`, in one versioned folder. It needs Rust, Node 22.12 or later with npm, and git
# with the previous release's tag fetched (the previous engine is built from it).
#
# Run from the repository folder:  sh packaging/unix/build.sh
# On Windows, inside WSL:          wsl -d Ubuntu-24.04 -- sh packaging/unix/build.sh
# Writes:  dist/SoccerManager-<version>-<platform>.tar.gz and its .sha256
#
# The version is read from the built program's --version, so the archive name, the program,
# and the engine's hello message always name the same version.
set -eu

# A non-login shell (such as `wsl -- sh ...`) may not have rustup's folder on PATH.
if ! command -v cargo >/dev/null 2>&1 && [ -x "$HOME/.cargo/bin/cargo" ]; then
    PATH="$HOME/.cargo/bin:$PATH"
    export PATH
fi

repo=$(cd "$(dirname "$0")/../.." && pwd)
dist="$repo/dist"
# Linux objects stay out of the Windows target folder, and off a mounted Windows drive.
CARGO_TARGET_DIR=${CARGO_TARGET_DIR:-"$HOME/.cache/soccermanager-target"}
export CARGO_TARGET_DIR

# The viewer build needs Node; say so before the long engine build.
if ! command -v node >/dev/null 2>&1 || ! command -v npm >/dev/null 2>&1; then
    echo "error: node or npm is not on PATH; Node 22.12 or later is needed to build the viewer" >&2
    exit 1
fi

(cd "$repo" && cargo build --release --locked -p engine-cli)

# The previous release's engine, built from its tag (or reused from the cache), ships beside
# this one, so a match that release saved finishes on the engine that started it.
previous=$(sh "$repo/packaging/unix/previous-engine.sh" | tail -n 1)
if [ ! -x "$previous/engine-cli" ]; then
    echo "error: the previous release's engine was not built (the script printed '$previous')" >&2
    exit 1
fi

# The page the release carries is the viewer's release build: the viewer and the handshake
# page, with the font licence texts, the open-source notices (this program's crates, the
# previous engine's crates, the viewer's packages and the fonts), and no test page. The build
# fails when a shipped package has no licence text or no allowed licence; the check then reads
# the written file again from the sources.
SM_PREVIOUS_NOTICES="$previous/notices-crates.json"
export SM_PREVIOUS_NOTICES
(cd "$repo/viewer" && npm ci --no-audit --no-fund && npm run build:release \
    && npm run notices:verify -- dist-release/notices.json)

program="$CARGO_TARGET_DIR/release/engine-cli"
version=$("$program" --version | awk '{ print $2 }')
if [ -z "$version" ]; then
    echo "error: cannot read a version from '$program --version'" >&2
    exit 1
fi

case "$(uname -s)" in
    Linux) os=linux ;;
    Darwin) os=macos ;;
    *) os=$(uname -s | tr '[:upper:]' '[:lower:]') ;;
esac
platform="$os-$(uname -m)"
name="SoccerManager-$version-$platform"

stage="$dist/stage-$platform"
rm -rf "$stage"
mkdir -p "$stage/$name"
cp "$program" "$stage/$name/engine-cli"
cp -R "$repo/content" "$stage/$name/content"
cp -R "$repo/viewer/dist-release" "$stage/$name/web"
cp -R "$previous" "$stage/$name/previous"
cp "$repo/LICENSE-MIT" "$repo/LICENSE-APACHE" "$stage/$name/"
cp "$repo/packaging/unix/soccermanager" "$stage/$name/soccermanager"
chmod 755 "$stage/$name/engine-cli" "$stage/$name/soccermanager" "$stage/$name/previous/engine-cli"

archive="$dist/$name.tar.gz"
tar -czf "$archive" -C "$stage" "$name"
if command -v sha256sum >/dev/null 2>&1; then
    (cd "$dist" && sha256sum "$name.tar.gz" > "$name.tar.gz.sha256")
else
    (cd "$dist" && shasum -a 256 "$name.tar.gz" > "$name.tar.gz.sha256")
fi

echo "$archive"
