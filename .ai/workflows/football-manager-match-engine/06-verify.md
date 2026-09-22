---
schema: sdlc/v1
type: verify-index
slug: football-manager-match-engine
status: in-progress
stage-number: 6
created-at: "2026-09-21T22:40:58Z"
updated-at: "2026-09-22T18:49:11Z"
slices-verified: 4
slices-total: 16
slices:
  - {slug: engine-core, result: pass, convergence: not-needed, artifact: 06-verify-engine-core.md, verified-at: "2026-09-21T22:40:58Z"}
  - {slug: data-schemas-generator, result: pass, convergence: converged, fix-commit: "ae31329", artifact: 06-verify-data-schemas-generator.md, verified-at: "2026-09-22T10:01:47Z"}
  - {slug: stream-protocol, result: pass, convergence: converged, fix-commit: "c7f7357", artifact: 06-verify-stream-protocol.md, verified-at: "2026-09-22T13:35:00Z"}
  - {slug: viewer-pitch, result: pass, convergence: converged, fix-commit: "c7c54bd", artifact: 06-verify-viewer-pitch.md, verified-at: "2026-09-22T18:49:11Z"}
tags: [engine, rust, viewer]
refs:
  index: 00-index.md
  implement-index: 05-implement.md
next-command: wf-review
next-invocation: "/wf review football-manager-match-engine viewer-pitch"
---

# Verify Index

| Slice | Result | Convergence | Checks | Acceptance | Interactive | Artifact |
|---|---|---|---|---|---|---|
| `engine-core` | pass | not-needed | 12 / 12 | 6 / 6 (1 user-observable, live) | 3 / 3 | [06-verify-engine-core.md](06-verify-engine-core.md) |
| `data-schemas-generator` | pass | converged (1 round, fix `ae31329`) | 13 / 13 | 6 / 6 (0 user-observable) | not applicable | [06-verify-data-schemas-generator.md](06-verify-data-schemas-generator.md) |
| `stream-protocol` | pass | converged (1 round, fix `c7f7357`) | 15 / 15 | 6 / 6 (0 user-observable) | not applicable | [06-verify-stream-protocol.md](06-verify-stream-protocol.md) |
| `viewer-pitch` | pass | converged (2 verify runs; fixes `75a4ba6` and `c7c54bd`) | 12 / 13 | 8 / 8 (5 user-observable, headless) | 5 / 5 | [06-verify-viewer-pitch.md](06-verify-viewer-pitch.md) |

Evidence quality across verified slices: live 1 / headless 5 / n-a 20. Runtime-evidence deferrals: none.

## Recommended Next Stage

- `/wf review football-manager-match-engine viewer-pitch` (slug-wide ledger; the engine-core, data-schemas-generator, and stream-protocol reviews are pending on the same ledger)
