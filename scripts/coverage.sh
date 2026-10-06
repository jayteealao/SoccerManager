#!/bin/sh
# The coverage command and the pre-push check (lefthook.yml).
#
#   sh scripts/coverage.sh run            Run cargo llvm-cov with the line floor and,
#                                         when it passes, record a pass for the Rust
#                                         content it measured. Run it as
#                                         `lefthook run coverage`.
#   sh scripts/coverage.sh check <remote> The pre-push step. Reads git's pushed refs on
#                                         stdin and refuses a ref that changes Rust code
#                                         unless a pass is recorded for its Rust content.
#
# A full run takes about 30 minutes, so the push only checks a recorded pass.
# COVERAGE_FLOOR sets the line floor of a run (default 78); the push accepts only a
# pass recorded at 78 or more.
set -u

# The Rust code, the build settings that change it, and the files compiled into it
# (include_bytes! and include_str! targets outside crates/): the push trigger and the
# key. Test data that the tests read at run time (content/, gate/ and others) is not
# part of it. It is split on spaces into pathspecs, so no entry may contain a space.
RUST_PATHS="crates Cargo.toml Cargo.lock rust-toolchain.toml .cargo content/rules/default.json packaging/previous-engine.json"
MIN_FLOOR=78

cd "$(git rev-parse --show-toplevel)" || exit 1

# Passes live in the git directory that every worktree of this clone shares.
store=$(git rev-parse --path-format=absolute --git-common-dir)/coverage-passes

# The key of a commit's Rust content: the hash of the listing (mode, blob id, path)
# of every file under RUST_PATHS. A reworded message keeps it; any content change
# under those paths changes it.
key() {
  git ls-tree -r "$1" -- $RUST_PATHS | git hash-object --stdin
}

rust_status() {
  git status --porcelain --untracked-files=all -- $RUST_PATHS
}

is_zero() {
  case $1 in
    *[!0]*) return 1 ;;
    *) return 0 ;;
  esac
}

run() {
  floor=${COVERAGE_FLOOR:-$MIN_FLOOR}
  case $floor in
    [0-9] | [1-9][0-9] | 100 | 101) ;;
    *)
      echo "COVERAGE_FLOOR must be a whole number from 0 to 101, not '$floor'." >&2
      exit 2
      ;;
  esac

  if ! command -v cargo-llvm-cov >/dev/null 2>&1; then
    echo "cargo-llvm-cov is not installed. Install it with:" >&2
    echo "  cargo install cargo-llvm-cov --locked" >&2
    echo "  rustup component add llvm-tools-preview" >&2
    exit 1
  fi

  if [ -n "$(rust_status)" ]; then
    echo "The Rust code has uncommitted changes. Commit them first: a coverage pass is" >&2
    echo "recorded for committed Rust content only." >&2
    rust_status >&2
    exit 1
  fi

  head=$(git rev-parse HEAD) || exit 1
  k=$(key HEAD) || exit 1

  echo "Running coverage with a floor of $floor% lines. This takes about 30 minutes."
  if [ "$floor" -lt "$MIN_FLOOR" ]; then
    echo "The floor is below $MIN_FLOOR%: the pass will not satisfy the pre-push check."
  fi

  cargo llvm-cov --workspace --locked --fail-under-lines "$floor"
  status=$?
  if [ "$status" -ne 0 ]; then
    echo "Coverage did not pass (exit $status). No pass was recorded." >&2
    exit "$status"
  fi

  if [ -n "$(rust_status)" ] || [ "$(key HEAD)" != "$k" ]; then
    echo "The Rust code changed during the run. No pass was recorded." >&2
    exit 1
  fi

  mkdir -p "$store" || exit 1
  tmp="$store/.$k.$$"
  printf 'floor=%s\nhead=%s\nat=%s\n' "$floor" "$head" "$(date -u +%Y-%m-%dT%H:%M:%SZ)" >"$tmp" &&
    mv -f "$tmp" "$store/$k" || exit 1
  echo "Recorded a coverage pass for this Rust content."
}

check() {
  remote=${1:-}
  refused=0
  while read -r local_ref local_sha remote_ref remote_sha; do
    [ -n "${local_sha:-}" ] || continue
    # A deleted ref pushes no code.
    is_zero "$local_sha" && continue

    # Compare with the remote's commit for this ref when this clone has it. Otherwise
    # leave out every commit already known on that remote. For a URL, or a remote with
    # no fetched refs, this leaves out nothing, so the whole history counts.
    same_ref=
    if ! is_zero "${remote_sha:-0}" && git cat-file -e "$remote_sha^{commit}" 2>/dev/null; then
      base=$remote_sha
      same_ref=$remote_sha
    elif [ -n "$remote" ]; then
      base="--remotes=$remote"
    else
      base=
    fi

    if ! changed=$(git log -1 --format=%H "$local_sha" --not $base -- $RUST_PATHS); then
      changed=unknown
    fi
    [ -n "$changed" ] || continue

    # The pushed Rust content equals what the remote already holds for this ref (a
    # change and its revert): nothing new to measure.
    if [ -n "$same_ref" ] && [ "$(key "$local_sha")" = "$(key "$same_ref")" ]; then
      continue
    fi

    pass_floor=$(sed -n 's/^floor=//p' "$store/$(key "$local_sha")" 2>/dev/null)
    case $pass_floor in
      [0-9] | [1-9][0-9] | 100 | 101) ;;
      *) pass_floor=-1 ;;
    esac
    if [ "$pass_floor" -lt "$MIN_FLOOR" ]; then
      echo "Push refused: ${remote_ref:-$local_ref} changes Rust code, and no coverage pass is recorded for its Rust content. Run 'lefthook run coverage' (about 30 minutes) and push again." >&2
      refused=1
    fi
  done
  exit "$refused"
}

case ${1:-} in
  run) run ;;
  check) check "${2:-}" ;;
  *)
    echo "usage: sh scripts/coverage.sh run | check <remote>" >&2
    exit 2
    ;;
esac
