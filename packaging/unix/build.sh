#!/bin/sh
# Builds the Linux (or macOS) release archive: the engine, the content, the page, the
# licences, and the `soccermanager` start script, in one versioned folder.
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

(cd "$repo" && cargo build --release --locked -p engine-cli)

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
cp -R "$repo/web" "$stage/$name/web"
# The page's own tests are not part of the game.
rm -rf "$stage/$name/web/tests"
cp "$repo/LICENSE-MIT" "$repo/LICENSE-APACHE" "$stage/$name/"
cp "$repo/packaging/unix/soccermanager" "$stage/$name/soccermanager"
chmod 755 "$stage/$name/engine-cli" "$stage/$name/soccermanager"

archive="$dist/$name.tar.gz"
tar -czf "$archive" -C "$stage" "$name"
if command -v sha256sum >/dev/null 2>&1; then
    (cd "$dist" && sha256sum "$name.tar.gz" > "$name.tar.gz.sha256")
else
    (cd "$dist" && shasum -a 256 "$name.tar.gz" > "$name.tar.gz.sha256")
fi

echo "$archive"
