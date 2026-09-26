---
schema: sdlc/v1
type: implement
slug: task-realism-programme-design-docs
slice-slug: task
status: complete
stage-number: 5
created-at: "2026-09-22T21:34:35Z"
updated-at: "2026-09-22T21:34:35Z"
metric-files-changed: 3
metric-lines-added: 1743
metric-lines-removed: 0
metric-deviations-from-plan: 2
metric-review-fixes-applied: 0
commit-sha: ""
tags: [task, design-document, realism]
refs:
  index: 00-index.md
  brief: 01-task.md
  verify: 06-verify.md
next-command: task
next-invocation: "/wf task task-realism-programme-design-docs"
---

# Implement: the three realism programme design documents

## The Implementation

The brief asked for three design documents, one per programme, grounded in the board and every findings document its cards list. Three writer sub-agents ran in parallel, one per document, and each read the board, the findings in `.scratch/out/`, the engine workflow, and the code its cards name. The documents total 1743 lines. Reading the code, the writers found that the engine has moved since the board was written, so two board claims are now partly out of date, and each document says so where it matters.

## What happened per step

| Step | Result | Note |
|---|---|---|
| 1. Read the sources | done | Every source the brief lists was read. `gap_idsse.txt` does not exist; the writers used `gap_idsse.json` and `gap_idsse_an.txt`. |
| 2. Write `01-engine-realism.md` | done | 853 lines, W-01 to W-07 in order, each with goal, design, measured targets, code touchpoints, open assumptions, risks, and exit checks. |
| 3. Write `02-event-contract-redesign.md` | done | 495 lines. W-08 is the whole document, so it has no per-unit heading; the header table names W-08 and board card B-16. |
| 4. Write `03-season-layer.md` | done | 396 lines, W-09 to W-12 in order, with the same sub-headings as document 1. |
| 5. Check against the board | done | See `06-verify.md`. |

## Deviations

1. Two documents said that their sibling documents did not exist, because the three writers ran at the same time. After all three landed, the coordinator replaced those notes with plain links in `02-event-contract-redesign.md` and `03-season-layer.md`.
2. The three new files show as staged in git (`A`). This task ran no `git add`, and no writer ran a state-changing git command. Something outside this task staged them. They are not committed.

## Findings that argue with the board

- **C-44 is out of date.** The event model now has 13 kinds and an optional `player.id` (`crates/protocol/src/event.rs:15` to `:33`, `:122`, `:126`), filled only on offside, foul, and card. The match-rules work moved the protocol to version 2. `02-event-contract-redesign.md` states the current contract.
- **C-01 is superseded.** The laws of the game are built (`README.md`). `01-engine-realism.md` describes the current engine.
- **C-162 overstates what exists.** The change queue holds a change and answers; the code does not yet apply it at a stoppage (`crates/protocol/src/command.rs:1` to `:4`).
- **C-02 and C-17 count 37 attributes.** `content/attributes.json` holds 36, so 14 slots are free, not 13.
- **`docs/reference/protocol.md:18` says `v` must equal 1.** The protocol is version 2. The document records this as a risk; this task did not edit the reference.
- The documents record 43 open questions between them.
