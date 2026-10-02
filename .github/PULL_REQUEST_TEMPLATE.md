## Summary

<!-- What changes for the player or the developer, and why. -->

## Test evidence

<!-- The commands you ran and what they showed. Name any slow test or calibration run. -->

## Release note

<!-- One line for CHANGELOG.md, or "none". Content changes need an entry under Content. -->

## Checklist

- [ ] `cargo fmt --all --check` and `cargo clippy --workspace --all-targets -- -D warnings` pass
- [ ] `cargo test --workspace` and `npm --prefix viewer test` pass
- [ ] `npm --prefix viewer run scan` and `npm --prefix viewer run contrast` pass
- [ ] `engine-cli gate`, `engine-cli guard --base main` and `engine-cli fast-model stale` pass on the release build
- [ ] Commits and the title follow Conventional Commits
- [ ] `CHANGELOG.md` has an entry when the change reaches players
