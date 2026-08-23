# Lifecycle and Evidence Review

Role: fresh read-only lifecycle and evidence specialist. Diagnostic and
advisory only; no proposal decision or implementation authority.

- Subject: `30fa4295c53714590b93c68b9666134ffea294e7`
- Claude session: `abda4c6b-f964-4c66-b8f6-6a25564e3e67`
- Readiness consequence: `revise_before_review_closure`
- Durable episode revision:
  `9427f4f276b22dd8450e0fae06c2eaff8ec3d153`
- Durable episode consequence: `remediation / await_remediation`

## Findings

### RIA-LE-001 — High — the formation revision is hash-bound but not explained

The subject binds baseline `faa633a` and review subject `30fa429` with an opaque
delta hash. It does not narrate that the latter revision added the
publication-before-review outcome to `proposal.md` and
`implementation-plan.md` in response to user judgment before review began.

A fresh consumer must reconstruct that distinction from Git history. Add an
attributed formation-revision note so initial formed meaning and the later
pre-review revision remain distinguishable without replaying conversation.

### RIA-LE-002 — Medium — canonical `packet.json` lacks a per-file subject binding

The bounded-content table individually hash-binds the four narrative files but
does not list `packet.json`, even though it owns closed lifecycle, placement,
uncertainty, relationship, and authority metadata. Bind it at the same
granularity in the next subject record and distinguish its hash from any
aggregate packet digest.

### RIA-LE-003 — Low — session verification is assigned to the wrong observer

The subject tells initial calls to verify their returned session identity, but
the reviewer toolset does not expose its own Claude runtime identity. The CLI
coordinator can and did verify that the returned JSON session ID matched the
precommitted binding. Assign this observation to the coordinator instead of
creating an impossible reviewer self-check.

## Durable-state dogfood consequence

The same retained Claude session later exercised its episode-bound MCP state.
The first attempt exposed an opaque transition-payload schema: the episode begin
persisted, invalid finding writes failed closed, and no false finding history
was created. After commit `05f23b3` made the nested payload self-describing, the
same session recorded all three findings on its first attempt, read them back,
and observed two ordered history revisions. This operational history is not new
proposal evidence or semantic acceptance.

The reviewer did not independently recompute Git hashes. Exact session identity
was verified by the coordinator from Claude's returned JSON, not by the reviewer
from inside its role.

