# Contributing

This guide tells you how to set up a checkout, run the checks, write commits, and make a release.

## Set up a checkout

1. Install Rust with rustup. `rust-toolchain.toml` selects the toolchain (1.92.0, with rustfmt and clippy) on the first build.
2. Install Node 22 or later for the page tests and the browser suite.
3. Install the hook tools: [lefthook](https://github.com/evilmartians/lefthook), [committed](https://github.com/crate-ci/committed) (`cargo install committed`), and [gitleaks](https://github.com/gitleaks/gitleaks).
4. Install the hooks:

   ```bash
   lefthook install
   ```

5. Install the browser suite:

   ```bash
   cd e2e && npm ci
   ```

6. Build the match viewer, a Svelte 5 app built with Vite. The engine serves `viewer/dist`, and the browser suite needs it:

   ```bash
   npm --prefix viewer ci
   npm --prefix viewer run build
   ```

   Rebuild it after every change under `viewer/`. `npm --prefix viewer test` runs the viewer's own tests.

## Check your change

Each pull request runs these checks. Run the fast ones before you push:

| Check | Command |
|---|---|
| Format | `cargo fmt --all --check` |
| Lint | `cargo clippy --workspace --all-targets --locked -- -D warnings` |
| Engine tests | `cargo test --workspace --locked` |
| Viewer tests | `npm --prefix viewer test` |
| Coverage | `cargo llvm-cov --workspace --locked --fail-under-lines 78` |
| Licences and advisories | `cargo deny check` |

The pull request also checks the commit messages, the title, secrets (gitleaks), code patterns (semgrep), the browser suite's dependencies (`npm audit`), and a Windows build that must link the C runtime statically.

The slow tests and the calibration suites are not part of a pull request. Run them before a release:

```bash
cargo test --release --workspace -- --include-ignored
```

CAUTION: the slow tests use every core for a long time. On a machine that is not stable under a long full load, run them on a build server.

## Write commits

Commits follow [Conventional Commits](https://www.conventionalcommits.org/) with a lowercase subject, for example `fix(engine): take penalties as kicks`. The allowed types are `feat`, `fix`, `docs`, `test`, `chore`, `refactor`, `perf`, `style`, `build`, `ci`, and `revert`. The subject is at most 100 characters. `committed.toml` holds the rules, and the commit-msg hook checks each commit.

The pull request title follows the same format.

## Change the game's content

A change under `content/rules/`, `content/teams/`, `content/tuning.json`, `content/attributes.json`, or `content/tactics.json` needs an entry under **Content** in the Unreleased section of `CHANGELOG.md`. Do not change `schema_version` for a content edit. The engine accepts one schema version only.

## Make a release

1. Run the slow tests and the calibration suite (`engine-cli calibrate --seed 42 --suite equal --matches 1000`). Every realism band must pass.
2. Move the Unreleased entries in `CHANGELOG.md` to a new `## [x.y.z] - YYYY-MM-DD` section.
3. Set the version with cargo-release, for example `cargo release version 0.2.0-rc.1 --execute`, and commit.
4. Tag and push the tag: `git tag v0.2.0-rc.1 && git push origin v0.2.0-rc.1`. The release workflow builds both platforms and uploads a draft release.
5. Download the draft setup file and run the Windows clean-machine check: `pwsh packaging/windows/run-sandbox.ps1`. The check must exit 0.
6. Publish the draft: `gh release edit v0.2.0-rc.1 --draft=false`.
7. After the release, set main to the next development version, for example `cargo release version 0.2.1-dev --execute`.

`packaging/README.md` describes the build scripts and the clean-machine checks. `docs/runbooks/` holds the steps for known release failures.
