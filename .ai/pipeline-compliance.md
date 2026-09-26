---
schema: sdlc/v1
type: pipeline-compliance
created-at: "2026-09-26T00:40:00Z"
updated-at: "2026-09-26T00:40:00Z"
plan-version-at-run: 1
ship-meaning: publish
ecosystem: rust
files-created:
  - .github/workflows/pr-checks.yml
  - .github/workflows/pr-title.yml
  - .github/workflows/release.yml
  - .github/workflows/rollback.yml
  - .github/CODEOWNERS
  - .github/PULL_REQUEST_TEMPLATE.md
  - .github/ISSUE_TEMPLATE/bug_report.md
  - .github/ISSUE_TEMPLATE/feature_request.md
  - .github/ISSUE_TEMPLATE/config.yml
  - docs/runbooks/nsis-missing.md
  - docs/runbooks/version-mismatch.md
  - docs/runbooks/smartscreen-glibc.md
  - docs/runbooks/sandbox-unavailable.md
  - committed.toml
  - deny.toml
  - lefthook.yml
  - rust-toolchain.toml
  - .editorconfig
  - CONTRIBUTING.md
  - CHANGELOG.md
files-patched:
  - packaging/windows/installer.nsi
  - packaging/windows/build.ps1
  - packaging/unix/smoke.sh
  - .gitignore
files-compliant: []
audits:
  A-pre-merge-checks: fixed
  B-release-trigger: fixed
  C-release-jobs: fixed
  D-dry-run-cmd: fixed
  E-publish-cmd: fixed
  F-required-secrets: fixed
  G-version-bump: fixed
  H-post-publish: fixed
  I-rollback: fixed
  J-runbooks: fixed
  K-quality-gates: fixed
  L-commit-pr-title: fixed
  M-git-hooks: fixed
  N-dx-files: fixed
  O-governance: fixed
  P-security: fixed
  Q-env-protection: skipped
  R-merge-controls: fixed
  S-ci-ergonomics: fixed
branch-protection-applied: failed
environment-protection-applied: n/a
merge-settings-applied: yes
secrets-to-set-manually: []
deps-to-install:
  - { name: "lefthook", reason: "git hooks (lefthook.yml)", command: "winget install evilmartians.lefthook  (or: go install github.com/evilmartians/lefthook@latest), then: lefthook install" }
  - { name: "committed", reason: "commit-msg hook", command: "cargo install committed --locked  (installed on this PC during the build)" }
  - { name: "gitleaks", reason: "pre-commit secret scan", command: "winget install gitleaks.gitleaks" }
  - { name: "cargo-deny", reason: "local licence and advisory check", command: "cargo install cargo-deny --locked  (installed on this PC during the build)" }
  - { name: "cargo-llvm-cov", reason: "local coverage check", command: "cargo install cargo-llvm-cov --locked  (installed on this PC during the build)" }
gates-to-activate:
  - { gate: "SDLC_GATE_PROVENANCE", blocked-on: "artifact attestations need a public repository or GitHub Enterprise Cloud", activation: "gh variable set SDLC_GATE_PROVENANCE --body true (after the repository is public)" }
  - { gate: "branch-protection", blocked-on: "private repository on the free plan (HTTP 403)", activation: "gh api -X PUT repos/jayteealao/SoccerManager/branches/main/protection --input <payload below> (after the repository is public or on Pro)" }
plan-amendments-needed:
  - "security.license-check.allow: add MPL-2.0 (smartstring 1.0.1 through rhai; the owner chose a global allow) and MIT-0, BSL-1.0, Unlicense (present as options in the tree)."
  - "ci-pipeline.release-jobs: add test (unit tests on the release path) and verify-draft."
  - "ci-pipeline.pre-merge-checks and governance.branch-protection.required-checks: add content-changelog."
  - "ci-pipeline.ci-ergonomics.path-filters: only .ai/ is ignored; docs/ is checked because crates/engine-cli/tests/docs.rs reads it."
  - "PRODUCT.md licence rule: record the MPL-2.0 allowance (a product file; edit by hand)."
not-built:
  - "replay-save-compat: readers for older protocol versions are engine and page work; route to a workflow."
routing: committed   # 6f66ee6 (amended), trailer sdlc-unreviewed: true
uncommitted-outputs: []
validation:
  yaml-syntax: pass
  actionlint: pass
  config-syntax: pass
  version-consistency: pass
  graph-integrity: pass
  repo-gates: pass
  provisioning: scaffolded-inert
---

# Pipeline Compliance — SoccerManager

The pipeline did not exist before this run. Every audit except environment protection (the plan has none) was missing, and each one is now closed.

## Files created

- `.github/workflows/pr-checks.yml`: the pull-request checks. Ubuntu runs fmt, clippy, the fast tests, coverage (78% line floor), `committed`, cargo-deny, npm audit (e2e), gitleaks, semgrep, and the content-changelog check. Windows runs a build, a static C runtime check, clippy, and the fast tests. PRs that change only `.ai/` skip the checks. Every job has a timeout.
- `.github/workflows/pr-title.yml`: the conventional PR-title check.
- `.github/workflows/release.yml`: runs on a `v*` tag and on a manual dry run. The jobs run in this order:
  1. version-gate: the tag must equal the `Cargo.toml` version, and `CHANGELOG.md` must have that version's section. Both checks run before any build.
  2. test.
  3. build-windows: NSIS, with the static C runtime check.
  4. build-linux: ubuntu-22.04, glibc 2.35.
  5. smoke-linux.
  6. draft-release.
  7. verify-draft: assets, hash round-trip, and version.
  8. attest: inert.
- `.github/workflows/rollback.yml`: marks the bad release as a prerelease and makes the prior release latest. It deletes nothing, and it works only after the file is on `main`.
- `docs/runbooks/*.md`: the four recovery playbooks.
- `committed.toml`, `lefthook.yml`, `deny.toml`, `rust-toolchain.toml` (1.92.0), `.editorconfig`, `CONTRIBUTING.md`, `CHANGELOG.md` (Unreleased only), `.github/CODEOWNERS`, and the PR and issue templates.

## Files patched

- `packaging/windows/installer.nsi` and `build.ps1`: `VIProductVersion` takes `NUMERIC_VERSION`, the version without its prerelease suffix. The test compile of `0.2.0-rc.1` passed and gave ProductVersion `0.2.0-rc.1`. Without the patch, the same compile failed with "invalid VIProductVersion format".
- `packaging/unix/smoke.sh`: the archive version keeps its prerelease suffix. The old parse returned `0.2.0` for a `0.2.0-rc.1` archive, so every prerelease smoke failed.
- `.gitignore`: `.ai/*` is local-only, except the five project files. Commit `cc7fcac` holds the last tracked copy of the notes. The index holds 1,607 staged removals, and the files stay on disk.

## Secrets requiring manual configuration

None. The workflows use the built-in `GITHUB_TOKEN` with job-level permissions.

## Dev dependencies to install

| Package | For | Command |
|---|---|---|
| lefthook | git hooks | `winget install evilmartians.lefthook`, then `lefthook install` |
| committed | commit-msg hook | `cargo install committed --locked` (already on this PC) |
| gitleaks | pre-commit secret scan | `winget install gitleaks.gitleaks` |

The git hooks do nothing until lefthook, committed, and gitleaks are installed and `lefthook install` has run.

## Remote settings (gated)

- **Branch protection:** `failed`. The API returned HTTP 403: "Upgrade to GitHub Pro or make this repository public". When one of those is true, apply it:

  ```
  gh api -X PUT repos/jayteealao/SoccerManager/branches/main/protection --input payload.json
  ```

  ```json
  {
    "required_status_checks": { "strict": true, "checks": [
      {"context": "fmt"}, {"context": "clippy"}, {"context": "test-fast"}, {"context": "coverage"},
      {"context": "commit-convention"}, {"context": "pr-title"}, {"context": "cargo-deny"},
      {"context": "npm-audit-e2e"}, {"context": "gitleaks"}, {"context": "semgrep"}, {"context": "windows-build"} ] },
    "enforce_admins": false,
    "required_pull_request_reviews": { "required_approving_review_count": 0, "dismiss_stale_reviews": false, "require_code_owner_reviews": false },
    "required_conversation_resolution": true,
    "required_linear_history": false,
    "allow_force_pushes": false,
    "allow_deletions": false,
    "restrictions": null
  }
  ```

  A PR that changes only `.ai/` starts no check, so required checks would never report for it.
- **Environment protection:** `n/a`.
- **Merge settings:** `yes`. The PATCH ran. Merge commits are on; squash, rebase, and auto-merge are off.

## Validation

- YAML syntax passes for the four workflows, `lefthook.yml`, and the issue-template config. TOML syntax passes for `deny.toml`, `committed.toml`, and `rust-toolchain.toml`. JSON syntax passes for `sdlc-config.json`.
- actionlint 1.7.12 reports no problems in the four workflows.
- Version consistency: Rust comes from `rust-toolchain.toml` (the action reads it), and Node comes from `e2e/package.json`. There is no duplicate literal.
- Graph integrity: every `needs:` target exists, the only secret is `GITHUB_TOKEN`, and the only variable gate is `SDLC_GATE_PROVENANCE`.
- Repo gates: `committed main..HEAD` passes for every commit on the branch. `cargo deny check` passes: advisories, bans, licences, and sources are ok.
- Action pins: every action is pinned to the commit SHA of its latest release. Tag `v2.0.0` of setup-rust-toolchain resolves to the pinned SHA, and its `rustflags` input defaults to empty, so the static C runtime flags in `.cargo/config.toml` hold.

## Gates scaffolded inert

- `SDLC_GATE_PROVENANCE`: build provenance on the release assets. It is blocked because artifact attestations need a public repository or GitHub Enterprise Cloud. To activate it: `gh variable set SDLC_GATE_PROVENANCE --body true`.
- Branch protection: blocked on the plan tier. Use the command above.

## Known risks

- The coverage floor (78%) was measured on Windows (80.30%). CI measures on Linux, which instruments different platform branches. If the first PR's Linux figure is under 78, lower the floor with `/wf ship-plan edit`.
- The pinned toolchain (1.92.0) may raise clippy lints that the older server toolchain did not raise. The first PR run shows it.
- `Cargo.toml` is at 0.1.0 and `CHANGELOG.md` has no `## [0.1.0]` section, so a `v0.1.0` tag fails at version-gate by design. Write the section first.
- Run the release workflow by hand once (a dry run) before the first tag. GitHub offers `workflow_dispatch` only after the file is on `main`.

## Re-run compliance check

```
/wf ship-plan build --dry-run
```
