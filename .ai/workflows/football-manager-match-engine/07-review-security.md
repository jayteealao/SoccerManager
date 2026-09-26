---
schema: sdlc/v1
type: review-command
slug: football-manager-match-engine
dimension: security
parent: 07-review.md
review-scope: slug-wide
status: complete
created-at: "2026-09-23T20:34:36Z"
updated-at: "2026-09-23T20:34:36Z"
verdict: ship
metric-findings-total: 1
metric-findings-blocker: 0
metric-findings-high: 0
---

# Review: security

5 findings in this dimension. 4 are fixed and 1 are deferred. Nothing is open at BLOCKER or HIGH.

| ID | Sev | Conf | Status | Surfaced | File:Line | Issue | Fix / reason |
|---|---|---|---|---|---|---|---|
| SEC-1 | MED | high | fixed | 2026-09-23T20:34:36Z | crates/stream/src/server.rs:223-231 | WebSocket origin allowlist accepts the null origin and file:// | null and file:// dropped; tests and docs/reference/protocol.md updated. |
| SEC-2 | LOW | medium | fixed | 2026-09-23T20:34:36Z | crates/engine-cli/src/web.rs:171-255 | Page server never checks Host (DNS rebinding) | Only 127.0.0.1 / localhost with no port or the own port are answered (421 otherwise). Unit test. |
| SEC-4 | LOW | medium | fixed | 2026-09-23T20:34:36Z | crates/engine-cli/src/web.rs:152-163 | Page server: unbounded threads, no timeouts | 10 s read/write timeouts and a 64-connection cap. |
| SEC-3 | LOW | high | fixed | 2026-09-23T20:34:36Z | crates/stream/src/record.rs:244 | Replay reader trusts the header frame count for allocation | Capacity capped at body.len() / 9. Unit test. |
| SEC-5 | NIT | high | deferred | 2026-09-23T20:34:36Z | .ai/workflows/football-manager-match-engine/:n/a | Workflow evidence contains absolute local paths | Evidence files are generated workflow records; redacting them is outside this code diff and changes recorded evidence. Route to a redaction pass before publishing. |
