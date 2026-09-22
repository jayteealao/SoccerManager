---
schema: sdlc/v1
type: verify-index
slug: football-manager-match-engine
status: in-progress
stage-number: 6
created-at: "2026-09-21T22:40:58Z"
updated-at: "2026-09-22T10:01:47Z"
slices-verified: 2
slices-total: 16
slices:
  - {slug: engine-core, result: pass, convergence: not-needed, artifact: 06-verify-engine-core.md, verified-at: "2026-09-21T22:40:58Z"}
  - {slug: data-schemas-generator, result: pass, convergence: converged, fix-commit: "ae31329", artifact: 06-verify-data-schemas-generator.md, verified-at: "2026-09-22T10:01:47Z"}
tags: [engine, rust]
refs:
  index: 00-index.md
  implement-index: 05-implement.md
next-command: wf-review
next-invocation: "/wf review football-manager-match-engine data-schemas-generator"
---

# Verify Index

| Slice | Result | Convergence | Checks | Acceptance | Interactive | Artifact |
|---|---|---|---|---|---|---|
| `engine-core` | pass | not-needed | 12 / 12 | 6 / 6 (1 user-observable, live) | 3 / 3 | [06-verify-engine-core.md](06-verify-engine-core.md) |
| `data-schemas-generator` | pass | converged (1 round, fix `ae31329`) | 13 / 13 | 6 / 6 (0 user-observable) | not applicable | [06-verify-data-schemas-generator.md](06-verify-data-schemas-generator.md) |

Evidence quality across verified slices: live 1 / n-a 11. Runtime-evidence deferrals: none.

## Recommended Next Stage

- `/wf review football-manager-match-engine data-schemas-generator` (slug-wide ledger; engine-core review is still pending on the same ledger)
