# Keep the replay gate honest

This guide shows how to run the replay gate, add a machine's hash set, and regenerate the golden hashes when a change is meant to change the matches. It also lists what the pull-request check rejects.

The gate plays 22 fixed matches and compares a hash of the full match state after every tick with the hashes in `gate/golden.json`. A change that leaves the engine's results exactly as they were keeps every hash. Every flag is in [the command-line reference](../reference/cli.md#gate), and every rule of the check is in [the guard reference](../reference/cli.md#guard).

The commands use the release build. Build it first, from the repository root:

```bash
cargo build --release --locked -p engine-cli
```

## Run the gate

```bash
target/release/engine-cli gate
```

Each match prints `match` or `differs`. A match that differs names the window of ticks in which its state first changed. The command exits 0 when every match matches and 2 when one differs.

When the gate reports `differs` and the change was not meant to change any result, the change has a bug. Find it; do not regenerate.

## Decide whether a regeneration is allowed

Regenerate only when the change is meant to change match results, for example a new maths library or a new random-stream scheme, and the reviewers agree to it. A regeneration makes the change visible in review; it does not make it correct.

Also regenerate when the toolchain in `rust-toolchain.toml` changes, even when the hashes stay the same. The check requires a `regenerate` entry for a toolchain change.

If the change also changes what the gate measures (the fixture list, the state inventory version, or the checkpoint spacing), first raise `GATE_SCHEMA` in `crates/engine/src/gate/mod.rs`. The check rejects such a change without a higher gate schema.

## Regenerate the hashes

1. Commit the code change first, on a clean tree. The ledger entry records the build commit as its `candidate`, and the check rejects a candidate with a `-dirty` mark.
2. Rebuild the release binary, then run the regeneration with a reason that a reviewer can check:

   ```bash
   target/release/engine-cli gate --regenerate --reason "libm replaces the platform sin, cos, exp, and atan2"
   ```

   The command refuses to run without a reason, and leaves the golden file byte-identical. On success it writes this machine's new hash set, appends one `regenerate` entry, and names the machines whose sets it dropped.
3. Commit `gate/golden.json` in its own commit.
4. On each dropped machine, add its set again (next section).

The entry's `band_result` path, `gate/bands/ledger-<index>.json`, is where the realism band run for this regeneration is saved later. Never edit the entry to fill it in.

## Add a machine's hash set

Each machine key (`<os>-<arch>`, for example `linux-x86_64`) has its own hash set until the engine's maths give the same results everywhere. To add the set of a machine that has none, on that machine and with unchanged code:

```bash
target/release/engine-cli gate --add-machine-set --reason "linux-x86_64 set from the pinned ubuntu-24.04 image"
```

The command refuses when the machine already has a set, and when the golden file does not fit the build. It appends one `add-machine-set` entry and updates `set_differences`, which lists the matches that differ between each pair of machines.

For the Linux set that CI checks, use the set that CI made. When the gate job's Linux leg fails because the set is missing, it uploads a candidate file as the run artifact `gate-candidate-linux`. Download it, copy it over `gate/golden.json`, and commit it:

```bash
gh run download <run-id> -n gate-candidate-linux
```

## Check the history before you push

```bash
target/release/engine-cli guard --base main
```

The check reads every commit after `main` that changes the golden file and compares it with its parent. It prints `<commit> ok` for each good commit, and exits 2 when one breaks a rule. It rejects:

- a hash change, a removed hash set, or a toolchain change with no new `regenerate` entry;
- an edited or removed ledger entry, or more than one new entry in one commit;
- a fixture-list, state-inventory, or checkpoint-spacing change without a higher gate schema;
- an `add-machine-set` entry that adds any other set, or changes an existing set or the header;
- a new hash set with no entry, or an entry with no change;
- a `regenerate` candidate that is dirty, unknown, or not an ancestor of its commit;
- a deleted golden file.

Each commit is checked on its own, so a later commit cannot repair an earlier one. Fix the history before the pull request, for example by folding the golden-file change into a commit that also adds its entry.
