---
schema: sdlc/v1
type: verify
slug: task-realism-programme-design-docs
slice-slug: task
status: complete
tags: [task, design-document, realism]
metric-interactive-checks-run: 0
metric-interactive-checks-passed: 0
evidence-dir: "docs/design/realism/"
stage-number: 6
created-at: "2026-09-22T21:34:35Z"
updated-at: "2026-09-22T21:34:35Z"
result: pass
metric-checks-run: 5
metric-checks-passed: 5
metric-acceptance-met: 5
metric-acceptance-total: 5
metric-acceptance-user-observable: 5
metric-acceptance-code-only: 0
metric-acceptance-mock-rung: 0
metric-issues-found: 1
convergence: converged
refs:
  index: 00-index.md
  brief: 01-task.md
  implement: 05-implement.md
next-command: task
next-invocation: "/wf task task-realism-programme-design-docs"
---

# Verify: the three realism programme design documents

## The Verification

Each acceptance criterion was re-observed by reading the files back after they were written, not by trusting the writers' reports. A script compared every document with the board's frontmatter and with the files on disk. Every criterion closes at `live`. One issue was found during the checks, stale sibling-link notes, and it was fixed before the final read-back.

## Evidence

| AC | Observation | Result | evidence-rung |
|---|---|---|---|
| AC-1 | `ls docs/design/realism/` lists `01-engine-realism.md` (853 lines), `02-event-contract-redesign.md` (495), and `03-season-layer.md` (396). | pass | live |
| AC-2 | `grep "^## W-"` reads back W-01 to W-07 in order in document 1 and W-09 to W-12 in order in document 3. Document 2 holds W-08 alone, named in its header table at line 6. | pass | live |
| AC-3 | A script expanded each card's "Inherits" line (137, 14, and 54 ids) and found every id in its document. Missing: none. | pass | live |
| AC-4 | The same script checked every id cited (193, 32, and 93) against the board's claims, assumptions, and contradictions. Unknown: none. | pass | live |
| AC-5 | Each document cites the board, `FINDINGS.md`, `ACTIONS.md`, the `F16_` files, and the `gap_` outputs. A script tested 62, 50, and 24 listed repository paths: none is missing. Document 3 carries every source URL recorded by C-172 to C-193. | pass | live |

## Issue found

- Documents 2 and 3 said their sibling documents did not exist, because the writers ran in parallel. The notes were replaced with plain links, and a `grep` for the stale wording then returned nothing.
