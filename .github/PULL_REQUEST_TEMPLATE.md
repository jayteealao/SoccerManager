## Summary

<!-- What changes for the player or the developer, and why. -->

## Test evidence

<!-- The commands you ran and what they showed. Name any slow test or calibration run. -->

## Release note

<!-- One line for CHANGELOG.md, or "none". Content changes need an entry under Content. -->

## Checklist

- [ ] `cargo fmt --all --check` and `cargo clippy --workspace --all-targets -- -D warnings` pass
- [ ] `cargo test --workspace` and `node --test "web/tests/*.test.mjs"` pass
- [ ] Commits and the title follow Conventional Commits
- [ ] `CHANGELOG.md` has an entry when the change reaches players
