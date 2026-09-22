---
schema: sdlc/v1
type: verify-index
slug: football-manager-match-engine
status: in-progress
stage-number: 6
created-at: "2026-09-21T22:40:58Z"
updated-at: "2026-09-22T13:35:00Z"
slices-verified: 3
slices-total: 16
slices:
  - {slug: engine-core, result: pass, convergence: not-needed, artifact: 06-verify-engine-core.md, verified-at: "2026-09-21T22:40:58Z"}
  - {slug: data-schemas-generator, result: pass, convergence: converged, fix-commit: "ae31329", artifact: 06-verify-data-schemas-generator.md, verified-at: "2026-09-22T10:01:47Z"}
  - {slug: stream-protocol, result: pass, convergence: converged, fix-commit: "c7f7357", artifact: 06-verify-stream-protocol.md, verified-at: "2026-09-22T13:35:00Z"}
tags: [engine, rust]
refs:
  index: 00-index.md
  implement-index: 05-implement.md
next-command: wf-review
next-invocation: "/wf review football-manager-match-engine stream-protocol"
---

# Verify Index

| Slice | Result | Convergence | Checks | Acceptance | Interactive | Artifact |
|---|---|---|---|---|---|---|
| `engine-core` | pass | not-needed | 12 / 12 | 6 / 6 (1 user-observable, live) | 3 / 3 | [06-verify-engine-core.md](06-verify-engine-core.md) |
| `data-schemas-generator` | pass | converged (1 round, fix `ae31329`) | 13 / 13 | 6 / 6 (0 user-observable) | not applicable | [06-verify-data-schemas-generator.md](06-verify-data-schemas-generator.md) |
| `stream-protocol` | pass | converged (1 round, fix `c7f7357`) | 15 / 15 | 6 / 6 (0 user-observable) | not applicable | [06-verify-stream-protocol.md](06-verify-stream-protocol.md) |

Evidence quality across verified slices: live 1 / n-a 17. Runtime-evidence deferrals: none.

## Recommended Next Stage

- `/wf review football-manager-match-engine stream-protocol` (slug-wide ledger; the engine-core and data-schemas-generator reviews are still pending on the same ledger)
