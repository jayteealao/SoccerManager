#!/bin/sh
# Builds the previous release's engine from its git tag, so a saved match from that release
# can finish on the engine that started it. The release ships the result as `previous/`:
# `previous/engine-cli` and the tag's own `previous/content/`.
#
# Run from the repository folder:  sh packaging/unix/previous-engine.sh
# The pin (tag, commit, version) is packaging/previous-engine.json. The cache folder is
# $SM_PREVIOUS_CACHE, or ~/.cache/soccermanager-previous; the script prints the path of the
# finished `previous/` folder and reuses it while its program prints the pinned version.
#
# The tag is built in a detached git worktree with `cargo build --locked`, and the worktree
# is removed in every case; the checkout this script runs from is not touched. Before it goes,
# the open-source notices of the crates that program is built from are written to
# `previous/notices-crates.json` (Node is needed), which the viewer build adds to the notices
# the release ships.
set -eu

if ! command -v cargo >/dev/null 2>&1 && [ -x "$HOME/.cargo/bin/cargo" ]; then
    PATH="$HOME/.cargo/bin:$PATH"
    export PATH
fi

repo=$(cd "$(dirname "$0")/../.." && pwd)
pin="$repo/packaging/previous-engine.json"
field() {
    sed -n "s/^[[:space:]]*\"$1\"[[:space:]]*:[[:space:]]*\"\([^\"]*\)\".*/\1/p" "$pin"
}
tag=$(field tag)
commit=$(field commit)
version=$(field version)
if [ -z "$tag" ] || [ -z "$commit" ] || [ -z "$version" ]; then
    echo "error: $pin must name a tag, a commit and a version" >&2
    exit 1
fi

cache=${SM_PREVIOUS_CACHE:-"$HOME/.cache/soccermanager-previous"}
out="$cache/previous"
program="$out/engine-cli"
crate_list="$out/notices-crates.json"

printed_version() {
    "$1" --version 2>/dev/null | awk '{ print $2 }'
}

if [ -x "$program" ] && [ -d "$out/content" ] && [ -f "$crate_list" ] && [ "$(printed_version "$program")" = "$version" ]; then
    echo "$out"
    exit 0
fi

found=$(git -C "$repo" rev-parse --verify --quiet "refs/tags/$tag^{commit}" || true)
if [ -z "$found" ]; then
    echo "error: the tag $tag is not in this clone; fetch it with: git fetch --no-tags origin tag $tag" >&2
    exit 1
fi
if [ "$found" != "$commit" ]; then
    echo "error: the tag $tag points at $found, but $pin pins $commit" >&2
    exit 1
fi

mkdir -p "$cache"
checkout="$cache/checkout"
remove_worktree() {
    if [ -e "$checkout" ]; then
        git -C "$repo" worktree remove --force "$checkout" >/dev/null 2>&1 || true
        rm -rf "$checkout"
    fi
    git -C "$repo" worktree prune >/dev/null 2>&1 || true
}
remove_worktree
trap remove_worktree EXIT INT TERM

git -C "$repo" worktree add --detach "$checkout" "$commit" >/dev/null
(cd "$checkout" && cargo build --release --locked -p engine-cli --target-dir "$cache/target")

# A build from a changed tree carries a `-dirty` build hash, and its saves would match no
# released build.
if [ -n "$(git -C "$checkout" status --porcelain --untracked-files=no)" ]; then
    echo "error: the worktree of $tag changed during the build; refusing a dirty build" >&2
    exit 1
fi

rm -rf "$out"
mkdir -p "$out"
cp "$cache/target/release/engine-cli" "$program"
cp -R "$checkout/content" "$out/content"
chmod 755 "$program"

# The crate list of the tag's program, with each licence text, read while the worktree (and
# its Cargo.lock) is still here; the viewer build adds it to the notices the release ships.
node "$repo/viewer/scripts/notices.mjs" crates --manifest-path "$checkout/Cargo.toml" --out "$crate_list" >/dev/null

got=$(printed_version "$program")
if [ "$got" != "$version" ]; then
    echo "error: the program built from $tag prints version '$got', not $version" >&2
    exit 1
fi

echo "$out"
