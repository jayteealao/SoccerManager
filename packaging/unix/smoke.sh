#!/bin/sh
# The clean-user check for the Linux (or macOS) archive. It unpacks the archive into a fresh
# home folder, starts the game with a cleared environment from an unrelated folder, and
# checks that the page and the engine are served with no configuration.
#
# Run from the repository folder:  sh packaging/unix/smoke.sh <archive> <evidence-dir>
#
# Writes <evidence-dir>/results.json (one entry per check, and an overall pass) and the
# launcher's output. Exits 0 when every check passes and 1 otherwise.
set -u

if [ $# -ne 2 ]; then
    echo "usage: sh packaging/unix/smoke.sh <archive> <evidence-dir>" >&2
    exit 2
fi
repo=$(cd "$(dirname "$0")/../.." && pwd)
archive=$(cd "$(dirname "$1")" && pwd)/$(basename "$1")
mkdir -p "$2"
evidence=$(cd "$2" && pwd)
checks="$evidence/checks.jsonl"
# The test build in step (f) shares the build script's target folder.
CARGO_TARGET_DIR=${CARGO_TARGET_DIR:-"$HOME/.cache/soccermanager-target"}
export CARGO_TARGET_DIR
: > "$checks"

# One check per line; `fact` entries record the machine and never fail the run.
json_text() { printf '%s' "$1" | sed 's/\\/\\\\/g; s/"/\\"/g' | tr '\n\t' '  '; }
check() {
    printf '{"name":"%s","pass":%s,"detail":"%s"}\n' "$1" "$2" "$(json_text "$3")" >> "$checks"
    if [ "$2" = true ]; then word=PASS; else word=FAIL; fi
    echo "[$word] $1 - $3"
}
fact() {
    printf '{"name":"%s","pass":true,"fact":true,"detail":"%s"}\n' "$1" "$(json_text "$2")" >> "$checks"
    echo "[FACT] $1 - $2"
}

name=$(basename "$archive" .tar.gz)
archive_version=$(echo "$name" | sed -n 's/^SoccerManager-\([^-]*\)-.*$/\1/p')

# (a) A fresh home, and the archive unpacked into it.
home=$(mktemp -d)
elsewhere=$(mktemp -d)
mkdir -p "$home/opt"
if tar -xzf "$archive" -C "$home/opt"; then
    check extract true "unpacked $name into a fresh home"
else
    check extract false "tar could not unpack $archive"
fi
installed="$home/opt/$name"

# (b) What the program links against, as facts.
if command -v ldd >/dev/null 2>&1; then
    ldd "$installed/engine-cli" > "$evidence/ldd.txt" 2>&1
    fact ldd "$(tr '\n' ';' < "$evidence/ldd.txt")"
    fact glibc "$(ldd --version 2>&1 | head -n 1)"
fi

# (c) Start the game the way a player does, with nothing set but a home and a path.
(cd "$elsewhere" && env -i HOME="$home" PATH=/usr/bin:/bin "$installed/soccermanager" --minutes 1 \
    > "$evidence/launch.out.txt" 2> "$evidence/launch.err.txt") &
launcher=$!
address=""
i=0
while [ $i -lt 150 ] && [ -z "$address" ]; do
    address=$(grep -m 1 -E '^http://127\.0\.0\.1:[0-9]+/$' "$evidence/launch.out.txt" 2>/dev/null || true)
    [ -z "$address" ] && sleep 0.2
    i=$((i + 1))
done
if [ -n "$address" ]; then
    check address true "the start script printed $address"
else
    check address false "no page address within 30 seconds; see launch.err.txt"
fi

# (d) The page, the engine, and the data folder in the fresh home.
if [ -n "$address" ]; then
    if curl -sf "$address" -o "$evidence/index.html" && cmp -s "$evidence/index.html" "$installed/web/index.html"; then
        check page true "GET / returned the packaged index.html"
    else
        check page false "GET / did not return the packaged index.html"
    fi
    state=""
    i=0
    while [ $i -lt 240 ]; do
        curl -sf "${address}engine.json" -o "$evidence/engine.json" 2>/dev/null || true
        if grep -q '"engine.state":"running"' "$evidence/engine.json" 2>/dev/null &&
            grep -q '"socket.port":[0-9]' "$evidence/engine.json" 2>/dev/null; then
            state=running
            break
        fi
        sleep 0.25
        i=$((i + 1))
    done
    if [ "$state" = running ]; then
        check engine-running true "$(cat "$evidence/engine.json")"
    else
        check engine-running false "engine.json never reported running: $(cat "$evidence/engine.json" 2>/dev/null)"
    fi
    if [ -f "$home/.local/share/SoccerManager/engine.port" ]; then
        check data-folder true "engine.port written to ~/.local/share/SoccerManager in the fresh home"
    else
        check data-folder false "no engine.port in ~/.local/share/SoccerManager in the fresh home"
    fi
fi
if grep -q 'launch.open_failed' "$evidence/launch.err.txt" 2>/dev/null; then
    fact browser "no browser opened (launch.open_failed logged); the game kept running"
else
    fact browser "no launch.open_failed logged"
fi

# (e) The program's version is the archive's version.
program_version=$("$installed/engine-cli" --version | awk '{ print $2 }')
if [ -n "$program_version" ] && [ "$program_version" = "$archive_version" ]; then
    check version true "engine-cli --version $program_version matches the archive name"
else
    check version false "engine-cli --version '$program_version', archive name '$archive_version'"
fi

# (f) Stop the game, then read the engine's hello from the packaged program.
kill "$launcher" 2>/dev/null
pkill -f "$installed/engine-cli" 2>/dev/null
wait "$launcher" 2>/dev/null
if (cd "$repo" && SM_INSTALL_UNDER_TEST="$installed" cargo test --release --locked -p engine-cli \
    --test install_layout > "$evidence/install_layout.txt" 2>&1); then
    check hello-version true "the install-layout test passed against the packaged folder"
else
    check hello-version false "the install-layout test failed against the packaged folder; see install_layout.txt"
fi
pkill -f "$installed/engine-cli" 2>/dev/null

if grep -q '"pass":false' "$checks"; then pass=false; else pass=true; fi
{
    printf '{"platform":"%s","archive":"%s","at":"%s","pass":%s,"checks":[' \
        "$(uname -s)-$(uname -m)" "$(basename "$archive")" "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$pass"
    paste -sd, "$checks"
    printf ']}\n'
} > "$evidence/results.json"
rm -f "$checks"
rm -rf "$home" "$elsewhere"

echo "overall: $([ "$pass" = true ] && echo PASS || echo FAIL) ($evidence/results.json)"
[ "$pass" = true ]
