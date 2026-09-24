#!/usr/bin/env bash
# The tuning-loop measurements on the reference machine: three targeted runs against their
# baselines, then the same pairing with each build candidate.
T=C:/Users/jayte/AppData/Local/Temp/claude/C--Users-jayte-Documents-dev-SoccerManager/a7ca6102-5eb6-4026-b025-47fc91bca540/scratchpad/tl
cd C:/Users/jayte/Documents/dev/SoccerManager
export SM_DATA_DIR=$T/data SM_CONTENT_DIR=$PWD/content
run() { # name exe args...
  local name=$1 exe=$2; shift 2
  rm -rf "$T/$name"
  local s=$(date +%s.%N)
  "$T/engine-cli-$exe.exe" calibrate "$@" --out "$T/$name" > "$T/$name.out" 2> "$T/$name.err"
  local code=$?
  local e=$(date +%s.%N)
  echo "$name exe=$exe exit=$code wall_s=$(python -c "print(round($e-$s,1))") stats=$(ls $T/$name/stats | wc -l)"
}
run pair-a a --suite formations --pairing "4-4-1-1 v 4-4-2" --seed 42 --matches 1000 --baseline $T/full-42/report.json
run equal-a a --suite equal --seed 42 --matches 1000 --baseline $T/full-42/report.json
run red-a a --suite red-card --seed 1 --matches 120 --baseline $T/red-base/report.json
run pair-b b --suite formations --pairing "4-4-1-1 v 4-4-2" --seed 42 --matches 1000 --baseline $T/pair-a/report.json
run pair-c c --suite formations --pairing "4-4-1-1 v 4-4-2" --seed 42 --matches 1000 --baseline $T/pair-a/report.json
run pair-a2 a --suite formations --pairing "4-4-1-1 v 4-4-2" --seed 42 --matches 1000 --baseline $T/pair-a/report.json
run red-b b --suite red-card --seed 1 --matches 120 --baseline $T/red-a/report.json
run red-c c --suite red-card --seed 1 --matches 120 --baseline $T/red-a/report.json
echo MEASURE_DONE
