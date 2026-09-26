---
schema: sdlc/v1
type: ship-plan
slug: soccer-manager
plan-version: 1
created-at: "2026-09-25T23:46:20Z"
updated-at: "2026-09-25T23:46:20Z"
project-name: "SoccerManager"
template-hint: none

# === Required core — read by /wf ship ===

# Block A — what ship means
ship-meaning: publish
ship-environments:
  - name: "smoke"
    auto-promote: true
  - name: "prerelease"
    auto-promote: false
  - name: "release"
    auto-promote: false
ship-cadence: on-demand

# Block B — versioning contract
version-scheme: semver
version-source-of-truth:
  - { path: "Cargo.toml", field: "workspace.package.version" }
version-bump-rule: manual
version-bump-cmd: "cargo release version <major|minor|patch|x.y.z-beta.N|x.y.z-rc.N> --execute"
prerelease-suffix: "-beta.N | -rc.N"
post-release-version: next-dev
post-release-version-cmd: "cargo release version <next x.y.z>-dev --execute"

# Block C — CI/CD contract
ci-pipeline:
  pre-merge-checks:
    - fmt
    - clippy
    - test-fast
    - coverage
    - commit-convention
    - pr-title
    - cargo-deny
    - npm-audit-e2e
    - gitleaks
    - semgrep
    - windows-build
  release-trigger: tag-on-main
  release-workflow-file: ".github/workflows/release.yml"
  release-jobs: [version-gate, build-windows, build-linux, smoke-linux, draft-release, attest]
  publish-dry-run-cmd: "gh workflow run release.yml -f dry-run=true"
  publish-cmd: "git push origin v<version>  # the workflow uploads to a draft release; publish with: gh release edit v<version> --draft=false [--prerelease]"
  required-secrets:
    - { name: "GITHUB_TOKEN", purpose: "Built-in token; creates the draft release and uploads the assets (permissions: contents: write, id-token: write, attestations: write)." }
  secrets-staleness-threshold-days: 90
  ci-ergonomics:
    dep-cache: true
    matrix: { os: ["windows-latest", "ubuntu-22.04"], versions: ["rust-toolchain.toml"] }
    release-concurrency: true
    path-filters: true

# Block D — post-publish verification contract
post-publish-checks:
  - { kind: windows-clean-machine, cmd: "gh release download v<version> -p '*setup.exe*' -D dist && pwsh packaging/windows/run-sandbox.ps1", expect: "exit 0; results.json pass: true (runs on the DRAFT, before publishing)" }
  - { kind: github-release, cmd: "gh release view v<version> --json assets --jq '.assets[].name'", expect: "4 assets: windows-x64-setup.exe, linux-x86_64.tar.gz, and their two .sha256 files" }
  - { kind: hash-round-trip, cmd: "gh release download v<version> -D .scratch/verify && cd .scratch/verify && sha256sum -c *.sha256", expect: "every file OK" }
  - { kind: version-match, cmd: "tar -xzf SoccerManager-<version>-linux-x86_64.tar.gz && ./SoccerManager-<version>-linux-x86_64/engine-cli --version", expect: "prints <version>, equal to the tag without the v" }
  - { kind: tester-smoke, cmd: "human: install the prerelease and play one match", expect: "a tester records one full match with no failure before promotion to release" }
propagation-window-min-minutes: 0
propagation-window-max-minutes: 5
poll-interval-seconds: 30

# Block E — rollout + rollback contract
rollout-strategy: staged
rollout-stages: ["draft", "prerelease", "release"]
rollback-mechanism: gh-release-yank
rollback-time-estimate-min: 5
rollback-cmd: "gh release edit v<bad> --prerelease && gh release edit v<prior> --latest"
rollback-verify-cmd: "gh release view --json tagName --jq .tagName"
prior-artifact-retention: "all releases, never deleted"
irreversible-steps:
  - "A downloaded copy of a bad build stays on the player's machine; a yank cannot recall it."
  - "A pushed version tag is permanent; a fixed release takes a new version, never a re-used one."
db-migrations-reversible: n/a

# Block F — recovery playbooks
recovery-playbooks:
  - id: nsis-missing
    triggers: ["makensis.*not (found|recognized)", "cannot find makensis", "NSIS.*not installed"]
    steps:
      - "Add a step before build.ps1 in the Windows job: choco install nsis -y."
      - "Re-run the failed job: gh run rerun <run-id> --failed."
  - id: version-mismatch
    triggers: ["release_version", "tag .* does not match", "version mismatch"]
    steps:
      - "Do not re-tag. Delete the draft release if one exists: gh release delete v<version> --yes."
      - "Delete the wrong remote tag only when no release was published from it: git push origin :refs/tags/v<version>."
      - "Run cargo release version <x.y.z> --execute, commit, and tag again."
  - id: smartscreen-glibc
    triggers: ["SmartScreen", "Windows protected your PC", "GLIBC_[0-9.]+' not found"]
    steps:
      - "Windows: tell the player to choose More info, then Run anyway. The setup file is unsigned."
      - "Linux: confirm the player's glibc is 2.35 or later (ldd --version). The archive is built on ubuntu-22.04."
  - id: sandbox-unavailable
    triggers: ["exit code 3", "Windows Sandbox is not enabled", "exit code 2", "no results arrive"]
    steps:
      - "Exit 3: enable Windows Sandbox (Optional Features > Windows Sandbox), restart, and run run-sandbox.ps1 again."
      - "Exit 2: close the sandbox window and run run-sandbox.ps1 again."
      - "When the sandbox stays unavailable, use the Hyper-V fallback in packaging/README.md with smoke.ps1."

# Block G — stakeholder + announcement contract
announcement:
  channels: ["GitHub release notes", "Discord", "social post", "itch.io devlog"]
  template-path: ".ai/release-announcement-template.md"

# === Inbound half — read by /wf ship-plan build (and the local gate in /wf handoff) ===

# Block H — code-quality gates
code-quality:
  format-check: { tool: "rustfmt", cmd: "cargo fmt --all --check" }
  lint:         { tool: "clippy", cmd: "cargo clippy --workspace --all-targets --locked -- -D warnings" }
  type-check:   { tool: "n/a", cmd: "" }
  test-coverage: { min-percent: 78, cmd: "cargo llvm-cov --workspace --locked --fail-under-lines 78" }
  commit-convention:   { spec: conventional, config-path: "committed.toml", enforce: [local, ci] }
  pr-title-convention: { spec: conventional, enforce: [ci] }

# Block I — local developer experience
local-dx:
  git-hooks:
    framework: lefthook
    hooks:
      pre-commit: ["cargo fmt --all --check", "gitleaks protect --staged"]
      commit-msg: ["committed --commit-file {1}"]
      pre-push:   []
  editorconfig: true
  runtime-version-files: ["rust-toolchain.toml"]
  task-runner: { kind: none, targets: {} }
  bootstrap-cmd: "lefthook install && (cd e2e && npm ci)"
  contributing-doc: true

# Block J — repo governance
governance:
  branch-protection:
    base-branch: "main"
    mechanism: branch-protection
    required-checks: ["fmt", "clippy", "test-fast", "coverage", "commit-convention", "pr-title", "cargo-deny", "npm-audit-e2e", "gitleaks", "semgrep", "windows-build"]
    required-approvals: 0
    dismiss-stale-reviews: false
    require-up-to-date: true
    enforce-admins: false
    require-code-owner-reviews: false
    require-conversation-resolution: true
    require-linear-history: false
    allow-force-pushes: false
    allow-deletions: false
    apply-via: gh-api
  codeowners:
    - { path: "*", owners: ["@jayteealao"] }
  pr-template: true
  issue-templates: true
  dependency-automation: { tool: none, ecosystems: [], schedule: "" }
  merge: { method: merge, auto-merge: false, merge-queue: false }

# Block K — security & supply-chain gates
security:
  sast:             { tool: semgrep, cmd: "semgrep scan --config p/rust --config p/javascript --error", schedule: "every PR" }
  dependency-audit: { tool: "cargo-deny + npm-audit", cmd: "cargo deny check && (cd e2e && npm audit --audit-level=high)", fail-on: high }
  secret-scanning:  { tool: gitleaks, cmd: "gitleaks detect --redact", pre-commit: true }
  sbom:             { tool: none, format: cyclonedx, publish-with-release: false }
  license-check:    { tool: "cargo-deny", allow: ["MIT", "Apache-2.0", "Apache-2.0 WITH LLVM-exception", "BSD-2-Clause", "BSD-3-Clause", "ISC", "Zlib", "Unicode-3.0", "CC0-1.0"], deny: ["GPL-2.0", "GPL-3.0", "AGPL-3.0", "LGPL-2.1", "LGPL-3.0", "SSPL-1.0"] }

# === Extensions — open schema, not read by /wf ship unless a consumer opts in by id ===

additional-contracts:
  - id: windows-pr-build
    purpose: "Windows-only code and the static C runtime are checked on every PR, not first at release."
    fields:
      runner: "windows-latest"
      cmds: ["cargo build --release --locked -p engine-cli", "cargo clippy --workspace --all-targets --locked -- -D warnings", "cargo test --workspace --locked"]
      static-crt-check: "dumpbin /dependents target/release/engine-cli.exe must not list VCRUNTIME*.dll; RUSTFLAGS must be unset in the job"
    enforced-by: "the windows-build PR check"
  - id: release-hygiene
    purpose: "The release workflow is safe to re-run and proves where its files came from."
    fields:
      tag-version-gate: "first job fails unless the tag without its v equals Cargo.toml workspace.package.version"
      publish-order: "upload to a DRAFT release; Windows Sandbox check on the draft setup.exe; publish the same files as prerelease; promote the same files to release; never rebuild between stages"
      triggers: ["push tags v*", "workflow_dispatch (dry-run input)"]
      concurrency: "one release run at a time"
      permissions: "contents: write, id-token: write, attestations: write; nothing else"
      actions: "third-party actions pinned to a commit SHA"
      provenance: "actions/attest-build-provenance on every asset"
      assets: "glob dist/SoccerManager-*; never hardcode a version in the workflow"
    enforced-by: "/wf ship-plan build (writes the workflow) and /wf ship (checks the draft before publishing)"
  - id: nsis-prerelease-version
    purpose: "A prerelease or -dev version builds a Windows setup file."
    fields:
      rule: "VIProductVersion takes the numeric x.y.z plus .0 with the suffix stripped (0.2.0-rc.1 -> 0.2.0.0); ProductVersion keeps the full version"
      file: "packaging/windows/installer.nsi"
    enforced-by: "/wf ship-plan build (patches installer.nsi) and the Windows release job"
  - id: replay-save-compat
    purpose: "Every release reads every replay and save written by any earlier release."
    fields:
      policy: "all old replays must load, including older protocol versions"
      fixtures: "web/tests/data/ holds one .smfx per released version (git-tracked by the .gitignore exception)"
      test: "a fast test decodes every fixture; a fixture that fails blocks the PR"
      consequence: "a protocol bump ships a reader for each earlier protocol version"
    enforced-by: "the test-fast PR check"
  - id: content-changelog
    purpose: "Players learn when tuning, rules, attributes, or teams change."
    fields:
      paths: ["content/rules/", "content/tuning.json", "content/attributes.json", "content/teams/", "content/tactics.json"]
      rule: "a change under these paths needs an entry under 'Content' in CHANGELOG.md for the next version"
      not: "schema_version is a format version the engine enforces; content edits never bump it"
    enforced-by: "a PR check that fails when these paths change and CHANGELOG.md does not"
  - id: realism-gate
    purpose: "A release candidate plays inside the realism bands before it is tagged."
    fields:
      runner: "the Contabo server through .scratch/remote/vps.sh"
      cmds: ["cargo build --release -q -p engine-cli && ./target/release/engine-cli --content-dir content calibrate --seed 42 --suite equal --matches 1000 --out ../out/release-<version>", "cargo test --workspace --locked -- --include-ignored"]
      pass: "report.json inside every band in content/realism-bands.json; every slow test passes"
      evidence: "report.json attached to the ship run record"
    enforced-by: "/wf ship pre-flight (human attaches the evidence)"
---

# Ship Plan — SoccerManager

## What "ship" means here
A release publishes the downloadable game: a Windows 11 x64 setup file (NSIS) and a Linux x86_64 archive, each with a `.sha256` file. The files go to a GitHub release on this repository. The repository is private, so only invited people can download a release until the download location changes. Discovery found the build scripts in `packaging/windows/build.ps1` and `packaging/unix/build.sh`, and built 0.1.0 files in `dist/`, but no earlier tag or release. A release moves through three stages: smoke, prerelease, and release. Releases happen on demand, when a finished piece of work is worth a build. The self-updating Tauri app from the packaging brainstorm replaces the NSIS setup later; that change is a `/wf ship-plan edit`.

## Versioning
The version is semver, and it lives in one place: `[workspace.package].version` in `Cargo.toml`. The test `crates/engine-cli/tests/release_version.rs` keeps `--version`, the engine's `hello` message, and every release file name equal to it. cargo-release sets the version; the owner picks the level by hand. `CHANGELOG.md` is written by hand. Prereleases carry `-beta.N` or `-rc.N`, as each release needs. After a release, main moves to the next `-dev` version, so a development build is never mistaken for a release.

## CI/CD pipeline
Releases run on GitHub Actions. This reverses the earlier "hosted CI later" decision from the packaging brainstorm. A pushed `v*` tag starts `.github/workflows/release.yml`:

1. The version gate fails unless the tag equals the `Cargo.toml` version.
2. `windows-latest` builds the setup file with `build.ps1`.
3. `ubuntu-22.04` builds the archive with `build.sh` (glibc 2.35 floor) and runs `smoke.sh`.
4. The workflow uploads all four files to a draft release and attests their provenance.

The workflow also runs by hand (`workflow_dispatch`), one run at a time, with `contents: write` as its only write permission, and with third-party actions pinned to a commit.

Every PR runs the light checks on Ubuntu (format, clippy, fast tests, coverage, commit and PR-title conventions, cargo-deny, npm audit, gitleaks, semgrep) and a Windows build job. PRs that change only `.ai/` or `docs/` skip the checks. Heavy runs (slow tests, calibrate) stay on this PC or the Contabo server, and the owner is asked first. The only secret is the built-in `GITHUB_TOKEN`. Private-repository Actions minutes are metered, and Windows minutes count double.

## Post-publish verification
The release is checked while it is still a draft:
1. The owner downloads the draft setup file and runs `run-sandbox.ps1`. The check must exit 0. It is the only clean-machine check for Windows.
2. The release lists four assets.
3. Every downloaded file matches its `.sha256` file.
4. The Linux program prints the tag's version.

The draft is then published as a prerelease. A tester installs it and plays one full match before the same files are promoted to a release. No stage rebuilds a file.

## Rollout strategy
Draft, then prerelease, then release, with the same files at each stage. Promotion is by hand.

## Rollback playbook
The signal is a failed post-publish check or a player report. Mark the bad release as a prerelease, and mark the prior release as latest (about 5 minutes). Then fix forward with a new patch version. Every release is kept, so any prior version can become latest again. A rollback cannot recall a copy that a player already downloaded, and it never re-uses a pushed tag.

## Recovery playbooks
- `nsis-missing`: install NSIS in the Windows job and re-run it.
- `version-mismatch`: never re-tag; bump again with cargo-release and tag the new version.
- `smartscreen-glibc`: player guidance for the unsigned setup file and for the glibc 2.35 floor. Seeded from `packaging/README.md`.
- `sandbox-unavailable`: exit codes 2 and 3 of `run-sandbox.ps1`, and the Hyper-V fallback. Seeded from `packaging/README.md`.

## Stakeholder + announcement
The GitHub release notes carry the version's `CHANGELOG.md` section. The owner posts by hand to Discord, a social post, and an itch.io devlog, from `.ai/release-announcement-template.md`.

## Code-quality gates
- Format: `cargo fmt --all --check` against `rustfmt.toml`.
- Lint: `cargo clippy --workspace --all-targets -- -D warnings`.
- Tests: the fast tests, meaning `cargo test --workspace` without the ignored tests, plus `node --test "web/tests/*.test.mjs"`.
- Coverage: cargo-llvm-cov fails a PR under 78% line coverage. The baseline measured on 2026-09-25 is 80.30% for lines, 82.15% for functions, and 78.89% for regions.
- Commits follow conventional commits, checked by `committed` in the commit-msg hook and in CI. Merge commits are left out of that check.
- PR titles follow conventional commits, checked in CI.

Every gate is a pre-merge check.

## Local developer experience
lefthook runs `cargo fmt --all --check` and a staged gitleaks scan before each commit, and the commit-message check at commit-msg. No pre-push tests run, because heavy runs stay off this PC. `rust-toolchain.toml` pins the toolchain with rustfmt and clippy, and `.editorconfig` sets the line endings (the Unix scripts stay LF). `CONTRIBUTING.md` says how to build, test, commit, and release. A new checkout runs `lefthook install`, then `npm ci` in `e2e/`. There is no task runner.

## Repo governance
The wanted rules for `main`: every pre-merge check passes, the branch is up to date, conversations are resolved, there are no force-pushes and no deletions, 0 approvals are needed, and code-owner review is off. `/wf ship-plan build` applies them with `gh api`. Until the repository is public or on GitHub Pro, the API returns 403 and no check blocks a merge. PRs merge with a merge commit, and auto-merge is off. `CODEOWNERS` names `@jayteealao` for every path. A PR template and issue templates (bug with version and platform, feature request) are added. There is no dependency automation.

## Security & supply-chain gates
Every PR runs these gates:
- cargo-deny: the licence allow-list follows the product's MIT- and Apache-compatible rule, and any RustSec advisory fails the check.
- `npm audit --audit-level=high` for `e2e/`.
- gitleaks, which also runs in the pre-commit hook.
- semgrep, with the Rust and JavaScript rule sets.

There is no SBOM. CodeQL and GitHub secret scanning are not free for a private repository.

## Additional contracts
- **windows-pr-build.** Every PR builds, lints, and tests on `windows-latest`. The job also checks that `engine-cli.exe` does not import the Visual C++ runtime.
- **release-hygiene.** The workflow runs a tag-to-version gate, uploads to a draft release first, never rebuilds between stages, uses least-privilege permissions, pins actions to a commit, attests provenance, and never hardcodes a version.
- **nsis-prerelease-version.** `installer.nsi` strips the prerelease suffix for `VIProductVersion` (0.2.0-rc.1 becomes 0.2.0.0) and keeps the full version in `ProductVersion`.
- **replay-save-compat.** Every old replay must load, including replays from older protocol versions. `web/tests/data/` holds one fixture per released version, and a fast test decodes each one.
- **content-changelog.** A change to `content/` needs a Content entry in `CHANGELOG.md`. `schema_version` is a format version that the engine enforces, so a content edit never bumps it.
- **realism-gate.** Before a tag, the Contabo server runs calibrate on the equal suite, 1000 matches, seed 42, and the report must sit inside every realism band. The server also runs the slow tests (`--include-ignored`). The report is attached to the ship run.
