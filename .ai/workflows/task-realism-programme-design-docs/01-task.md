---
schema: sdlc/v1
type: intake
slug: task-realism-programme-design-docs
workflow-type: task
status: complete
stage-number: 1
created-at: "2026-09-22T21:21:54Z"
updated-at: "2026-09-22T21:21:54Z"
blast-radius: repo-local
gate-decision: auto-proceeded-low-risk
origin-brainstorm: brainstorm-realism-additions-20260922
tags: [task, design-document, realism]
refs:
  index: 00-index.md
  next: 05-implement.md
sources:
  - .ai/workflows/brainstorm-realism-additions-20260922/01-brainstorm.md
  - .scratch/out/FINDINGS.md
  - .scratch/out/ACTIONS.md
  - .scratch/out/F16_head.md
  - .scratch/out/F16_part_a.md
  - .scratch/out/F16_part_b.md
  - .scratch/out/F16_part_c.md
  - .scratch/out/F16_tail.md
  - .ai/workflows/football-manager-match-engine/02-shape.md
  - .ai/workflows/football-manager-match-engine/03-slice.md
  - .ai/workflows/football-manager-match-engine/steer.md
next-command: task
next-invocation: "/wf task task-realism-programme-design-docs"
---

# Task: write the three realism programme design documents

## The Task

The realism brainstorm ended in session 4 with twelve work units in three programmes: engine realism, the event-contract redesign, and the season layer. You asked for one design document per programme, three in all, grounded in the board and in every findings document and source that the board's cards list. The board holds 213 claims, 62 assumptions, and 36 resolved contradictions, and the findings behind it sit in a gitignored folder on this machine. This task turns that material into three documents a later `/wf intake` can build from, in the order the board set.

## Restated request

Write three design documents under `docs/design/realism/`, one per programme. Each document states the programme's goal, its work units in sequence order, the design decisions with the board ids that carry them, the measured targets with their sources, the open assumptions, the risks, and the references. Every figure in a document cites a board claim, a findings section, or a source URL that the board records.

## Blast radius

`repo-local`. The task writes three new Markdown files inside the repository and this workflow's own artifacts. It changes no code and touches nothing outside the working tree. Gate: auto-proceeded-low-risk.

## Steps

1. Read the sources: the board, the findings documents, the engine workflow's shape, slice index, and steer file, and the code files each card lists. Outcome check: each document's reference list names only paths that exist.
2. Write `docs/design/realism/01-engine-realism.md` for units W-01 to W-07. Outcome check: read the file back and find all seven units in sequence order.
3. Write `docs/design/realism/02-event-contract-redesign.md` for unit W-08. Outcome check: read the file back and find the contract design and its timing.
4. Write `docs/design/realism/03-season-layer.md` for units W-09 to W-12. Outcome check: read the file back and find all four units in sequence order.
5. Check every document against the board. Outcome check: every board id a document cites exists on the board, and every claim id each card lists appears in its document.

## Acceptance criteria

- AC-1: three files exist at `docs/design/realism/01-engine-realism.md`, `02-event-contract-redesign.md`, and `03-season-layer.md`. Observed by `ls docs/design/realism/`.
- AC-2: each document covers its programme's units in the board's sequence order (W-01 to W-07, W-08, W-09 to W-12). Observed by reading the unit headings back from each file.
- AC-3: each document cites every claim id and assumption id its cards list on the board. Observed by a script that compares the ids in each card's "Inherits" line with the ids found in the document.
- AC-4: every board id cited in a document exists on the board. Observed by a script that checks each cited id against the board's frontmatter.
- AC-5: each document lists the findings documents and sources its cards name, and every listed repository path exists. Observed by a script that tests each listed path.

## Assumptions and open questions

- The documents go under `docs/design/realism/`, beside the existing `docs/reference/`. Moving them later is a file move.
- The findings documents stay in `.scratch/out/`, which git ignores. The design documents cite them by path; they do not copy them. The board's CAUTION on this risk stands.
- The README now describes throw-ins, offside, fouls, and cards as built. Board claim C-01 predates that work. The documents describe the current engine from the README and the code, not from C-01.
- The documents are design documents, not plans. The `/wf intake` for each programme owns the slices and the plans.
