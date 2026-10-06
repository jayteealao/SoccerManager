#!/usr/bin/env sh
# Proves that moving ratings to tenths of 1 to 20 changes no match: the build of a base
# commit and the build of the working tree play every gate match and the committed replay
# with the same full state after every tick, a planted change of one tenth is caught, and
# calibration runs give the same figures.
#
# Usage: sh scripts/rescale-proof.sh <base-commit>
#
# Run it from the repository root. It builds the base commit in a detached worktree under
# .scratch/rescale-proof/base and keeps a copy of its executable, then this tree's release
# build. Both builds use this tree's target folder: a deeper one can pass the 260-character
# path limit of the Windows linker. Every other file it writes stays under
# .scratch/rescale-proof/. It prints one line per check and exits non-zero when any check
# fails.

set -u

base_rev=${1:?usage: sh scripts/rescale-proof.sh <base-commit>}
root=$(pwd)
work="$root/.scratch/rescale-proof"
mkdir -p "$work/fixtures" "$work/control" "$work/calibrate"
failures=0

pass() { echo "pass: $*"; }
fail() {
  echo "FAIL: $*"
  failures=$((failures + 1))
}

exe=""
case "$(uname -s 2>/dev/null)" in
  MINGW* | MSYS* | CYGWIN*) exe=".exe" ;;
esac

# (a) The base build, in its own worktree and target folder.
base_dir="$work/base"
if [ ! -d "$base_dir" ]; then
  git worktree add --detach "$base_dir" "$base_rev" >/dev/null 2>&1 || {
    echo "FAIL: cannot add a worktree for $base_rev"
    exit 1
  }
fi
git -C "$base_dir" checkout --detach --quiet "$base_rev" || exit 1
(cd "$base_dir" && CARGO_TARGET_DIR="$root/target" cargo build --release --locked -p engine-cli --quiet) || {
  echo "FAIL: the base build failed"
  exit 1
}
base_bin="$work/engine-cli-base$exe"
cp "$root/target/release/engine-cli$exe" "$base_bin" || exit 1
cargo build --release --locked -p engine-cli --quiet || {
  echo "FAIL: this tree's build failed"
  exit 1
}
this_bin="$root/target/release/engine-cli$exe"
base_content="$base_dir/content"
this_content="$root/content"
echo "base: $base_rev ($(git rev-parse --short "$base_rev"))"
echo "this: $(git rev-parse --short HEAD)$(git diff --quiet || echo ' with changes')"

# (b) The 22 gate matches, recorded by the base build with the base content. A replay file
# holds every input, so both builds re-simulate it from the file alone.
seeds="42 1 7 99 2026 0 18446744073709551615 3 11 23 57 123 314 777 1000 4242 9001 31337 65535 1000003"
for s in $seeds; do
  "$base_bin" record --content-dir "$base_content" --seed "$s" --minutes 90 \
    --out "$work/fixtures/seed-$s.smfx" >/dev/null 2>&1 || fail "record seed-$s"
done
cat >"$work/fixtures/change.json" <<'JSON'
[
  { "tick": 60000, "team": 0, "change": { "substitution": { "slot": 9, "bench": 0 } } },
  { "tick": 90000, "team": 1, "change": { "mentality": 4 } }
]
JSON
"$base_bin" record --content-dir "$base_content" --seed 42 --minutes 90 \
  --changes "$work/fixtures/change.json" --out "$work/fixtures/change.smfx" >/dev/null 2>&1 ||
  fail "record change"
"$base_bin" record --content-dir "$base_content" --seed 2 --minutes 90 --knockout \
  --script-pack "$base_content/scripts/sample" --out "$work/fixtures/knockout.smfx" \
  >/dev/null 2>&1 || fail "record knockout"

# (c) Every tick's full state on both builds: bisect exits 0 when no tick differs.
for f in "$work"/fixtures/*.smfx "$root/viewer/tests/data/one-minute-v4.smfx"; do
  name=$(basename "$f")
  [ -f "$f" ] || continue
  out=$("$this_bin" bisect --fixture "$f" \
    --a-binary "$base_bin" --b-binary "$this_bin" 2>&1)
  code=$?
  if [ "$code" -eq 0 ]; then
    pass "bisect $name: no difference"
  else
    fail "bisect $name: exit $code: $(echo "$out" | head -n 3 | tr '\n' ' ')"
  fi
done
echo "skip: viewer/tests/data/one-minute.smfx is a version-3 file of frames only and cannot be re-simulated"

# (d) The planted control. A version 1 copy of the home team with every value under 5
# raised to 5 has an exact version 2 copy (2v tenths, all at least 1.0). The base build
# plays the version 1 copy; this build plays the exact copy and a copy with one starter's
# pace one tenth higher: the right back's (player index 4), whose top speed binds from the
# kick-off of this match. The exact copy's tick digests must equal the base's, and the
# planted copy's must not.
python_bin=$(command -v python3 || command -v python)
"$python_bin" - "$base_content/teams/default-a.json" "$work/control" <<'PY'
import json, sys
src, out = sys.argv[1], sys.argv[2]
v1 = json.load(open(src, encoding="utf-8"))
for p in v1["players"]:
    for k, v in p["attributes"].items():
        p["attributes"][k] = max(5, v)
json.dump(v1, open(f"{out}/home-v1.json", "w", encoding="utf-8"), indent=2)

def v2(planted):
    doc = json.loads(json.dumps(v1))
    doc["schema_version"] = 2
    for i, p in enumerate(doc["players"]):
        tenths = {k: 2 * v for k, v in p["attributes"].items()}
        if planted and i == 4:
            tenths["pace"] += 1
        p["attributes"] = {k: f"@{t // 10}.{t % 10}@" for k, t in tenths.items()}
        p["height"], p["age"], p["nationality"] = 180, 25, "ENG"
    text = json.dumps(doc, indent=2)
    return text.replace('"@', "").replace('@"', "")

open(f"{out}/home-v2-exact.json", "w", encoding="utf-8").write(v2(False))
open(f"{out}/home-v2-planted.json", "w", encoding="utf-8").write(v2(True))
PY
digests() {
  # $1 binary, $2 content folder, $3 team file, $4 name: record 5 minutes of seed 42, then
  # its tick digests.
  set -- "$1" "$3" "$4" "$2"
  "$1" record --content-dir "$4" --seed 42 --minutes 5 --team-a "$2" \
    --out "$work/control/$3.smfx" >/dev/null 2>&1 || return 1
  "$1" resimulate --compare --fixture "$work/control/$3.smfx" \
    --state-digests "$work/control/$3.digests" >/dev/null 2>&1
  grep -E '^[0-9]+ ' "$work/control/$3.digests" >"$work/control/$3.ticks"
}
digests "$base_bin" "$base_content" "$work/control/home-v1.json" base || fail "control: the base run"
digests "$this_bin" "$this_content" "$work/control/home-v2-exact.json" exact || fail "control: the exact run"
digests "$this_bin" "$this_content" "$work/control/home-v2-planted.json" planted || fail "control: the planted run"
ticks=$(wc -l <"$work/control/base.ticks" 2>/dev/null | tr -d ' ')
ticks=${ticks:-0}
if [ "$ticks" -gt 0 ] && cmp -s "$work/control/base.ticks" "$work/control/exact.ticks"; then
  pass "control: the exact version 2 copy gives the base digests on all $ticks ticks"
else
  fail "control: the exact version 2 copy differs from the base"
fi
if [ ! -s "$work/control/planted.ticks" ] || [ "$ticks" -eq 0 ]; then
  fail "control: the planted run gave no digests"
elif cmp -s "$work/control/base.ticks" "$work/control/planted.ticks"; then
  fail "control: one tenth more pace was not caught"
else
  first=$(diff "$work/control/base.ticks" "$work/control/planted.ticks" | grep -m1 -E '^[<>] ' | cut -c3- | cut -d' ' -f1)
  pass "control: one tenth more pace differs from tick $first"
fi

# (e) Calibration on both builds: the generator, the strength boost, and every figure.
for side in base this; do
  bin=$base_bin
  content=$base_content
  if [ "$side" = this ]; then
    bin=$this_bin
    content=$this_content
  fi
  for suite in equal strength; do
    rm -rf "$work/calibrate/$side-$suite"
    SM_DATA_DIR="$work/calibrate" "$bin" calibrate --content-dir "$content" --seed 42 \
      --matches 200 --suite "$suite" --out "$work/calibrate/$side-$suite" >/dev/null 2>&1
  done
done
for suite in equal strength; do
  if "$python_bin" - "$work/calibrate/base-$suite/report.json" "$work/calibrate/this-$suite/report.json" <<'PY'
import json, sys
def figures(path):
    r = json.load(open(path, encoding="utf-8"))
    bands = {(b["suite"], b["band"]): b["value"] for b in r["calib.bands"] if b["band"] != "wall_ms"}
    suites = {s: {k: v for k, v in f.items() if "wall" not in k and not k.endswith("_ms")}
              for s, f in r["calib.suites"].items()}
    return bands, suites
a, b = figures(sys.argv[1]), figures(sys.argv[2])
sys.exit(0 if a == b and a[0] else 1)
PY
  then
    pass "calibrate $suite: every band and suite figure is equal on both builds"
  else
    fail "calibrate $suite: the figures differ"
  fi
done

if [ "$failures" -eq 0 ]; then
  echo "rescale proof: all checks pass"
  exit 0
fi
echo "rescale proof: $failures checks fail"
exit 1
