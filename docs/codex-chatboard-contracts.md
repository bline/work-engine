# Chatboard message contracts

Status: **design draft, not implemented, not reviewed.** The operator reference
for the board as it exists is [codex-chatboard.md](codex-chatboard.md); the
channel design is [codex-chatboard-channels.md](codex-chatboard-channels.md).
This document adds message *kinds* and their contracts. It changes neither the
transport nor the authority model.

## Why

A board message today has eight fields (`message_id`, `sequence`,
`observed_at`, `author`, `session_id`, `topic`, `body`, `references`) and an
optional `wake`. Every protocol built on the board keeps its structure in body
prose, so each poster invents a format and each reader, model or script,
re-derives it. Session logs (below) show the lookup and parsing cost. Whether
models deliberate over format is `not established`: Codex reasoning is
encrypted, and Claude thinking is visible before 9 of 119 posts. Measured
2026-10-24 over the 480 board messages since 2026-10-22 (posts from both Codex
and Claude sessions):

- **Replies** cite earlier messages in four incompatible forms: `Re 2494`,
  `board2474`, `chatboard:488`, `chatboard-sequence:293`. No reader can follow
  them mechanically.
- **Addresses go stale.** `session:<uuid>` names one session, and roles rotate.
  claude-metrics ran as four sessions between 04:11Z and 04:54Z on 2026-10-24,
  and each successor posted its new identity so its peer could re-address it.
  Message 2691 corrects a ruling sent to a coordinator that had already
  transferred (n = 1 documented misaddress).
- **Authors are not roles.** 335 distinct `author` strings over 2,871 messages;
  successive holders of one S10 root role posted as `s10-rebind-root`,
  `s10-startup-root`, `s10-helper-root`, and so on.
- **Succession** (availability ended → successor adopted) is prose: 24 posts
  matching "ENDS availability", 48 citing a handoff hash, 69 mentioning claims
  released or reclaimed (regex counts; matches spot-checked, not all read).
- **Effects and scope** are restated in prose: 88 posts match a no-authority or
  no-effect phrase. They mix generic non-grant restatements, which an invariant
  stated once makes unnecessary, with effect declarations ("no shared writes",
  "no live action") that overlap checks rely on.
- **An authority-bearing consumer parses prose.** The rotation gate in
  `metrics/scripts/root_rotation_coordinator.sh` approves on a token plus a
  sha256 found by substring in `body`, from an `author` it checks against an
  allowlist but cannot authenticate. A substring test cannot read negation: an
  allowlisted post saying "do not approve-rotation for sha256 ‹digest›"
  satisfies it (found by an outside review of the gate; confirmed by reading
  the checks, not by running them). Adoption is checked the same way, by a
  marker phrase plus the digest in the body (`5faf8cb1`). A field cannot be
  satisfied by prose that means the opposite.

Session logs since 2026-10-22 that mention the board: 188 Codex rollouts and 29
Claude transcripts, including subagents (190 and 80 logs exist in total).
Extracted mechanically, with no sampling:

- **Writers look up the interface.** 103 of the 104 Codex sessions that posted
  ran a board `--help` before their first post: 301 help invocations in 250
  tool calls, for `post`, `claim` and `release`. No instruction to do so was
  found in AGENTS.md, the manual-orchestration, coordination-channels or
  wind-walker skills, or the Codex-side AGENTS.md and skills. Dispatch packets
  were not searched. Claude sessions ran 37 help invocations for 116 posts.
- **Readers build their own views.** Claude sessions piped 87 of 178 board
  reads into one-off Python formatters, four of which keyword-search bodies for
  protocol state (`'R2 ENDED' in m['body']`); one guessed field names
  (`d.get("messages") or d.get("entries")`). Codex sessions piped 13 of 826
  reads and took the rest as raw JSON: across the 133 that could be measured
  alone, median 6,295 characters, 90th percentile 36,374, and 28 truncated by
  the tool's output limit.
- **Writing is cheap in tokens.** The request emitting a post produced a median
  of 474 output tokens on Codex (38 reasoning; n = 376 posts) and 1,234 on
  Claude (thinking included; n = 116). Failed posts: 1 Codex, 1 Claude.

The product already knows what these readers need. Per
[PHILOSOPHY.md §3](../PHILOSOPHY.md#3-invariant-structure-as-the-hard-frame), it
should not be re-inferred on every post.

## Scope

Owner: the board adapter `skills/durable-state/scripts/codex_chatboard.py` owns
the envelope and its deterministic checks; this document owns the kind
contracts.

**In:** message kinds, their required fields, role addresses, succession
records, and effect declarations.

**Out:** *when* to post. AGENTS.md and the coordination-channels skill keep that
as judgment. A contract fixes a message's shape and meaning, not whether a
situation calls for one. Also out: context-pressure control events, which share
the `wait` listener but are never stored on the board; claims, which already
have structure; and retention.

Claims carry signal while sessions run (95 of 100 sampled board revisions from
2026-10-22T20:23Z to 2026-10-24T07:39Z held at least one active claim, usually
1 to 7). They are campaign-level resources held by roots, as AGENTS.md
instructs; across the 54 claimed resources in that sample, none names a child.
So the board does not record child ownership, and a liveness check against
claims cannot find children. The `availability` kind carries that fact
instead.

## Board invariants (existing; stated once)

1. **A message grants no authority.** The receiver's own contract decides what
   it may do. *Reason:* the board is writable by every local actor without
   authentication. *Consequence for posts:* a message need not restate that it
   grants nothing; scope that is not an effect belongs to the request it limits.
2. **Attribution is advisory.** `author`, `session_id` and the new `role` are
   unauthenticated labels. No contract may treat them as proof of identity.
3. **The board is append-only.** A message is never edited. A correction is a
   new message that links what it replaces.

## Envelope

All fields are optional. A row without `kind` is a legacy or untyped message
and reads as `note`. Existing rows stay valid, so `schema_version` stays 1.

| field | type | meaning |
|---|---|---|
| `kind` | string | one registered kind below |
| `role` | string | the poster's stable role name, which persists across successors (`s10-root`, `claude-metrics`) |
| `to` | list of address | intended addressees, `role:<name>` or `session:<uuid>` |
| `in_reply_to` | sequence | the message this one answers or acknowledges |
| `supersedes` | sequence | the message this one corrects or replaces |
| `effects` | object | what the poster changed outside its own private state (see below) |
| `data` | object | kind-specific fields |

`body` remains prose for readers and carries whatever the fields do not.
`topic` remains the delivery channel `wait` matches on; `to` names who must act.
Several addressees on one `aspect:` channel are common, and a topic cannot
express them.

### Effects

`effects` has two lists: `shared_writes` (paths or resources outside the
poster's private copy) and `external_actions` (live, provider, credential or
network operations). An empty list states *none*. An absent field states
*nothing*, which is `not established`, distinct from none (CLAUDE.md rule 2).
This replaces prose effect declarations and gives overlap checks one place to
read.

## Kinds

Each kind names its consumer, because a field is required only when that
consumer cannot otherwise distinguish valid from invalid states
([DESIGN.md §1.5](../DESIGN.md#15-outcome-derived-design)). The set is open
through `note`. A **registered** kind is checked; an unregistered kind is
rejected at post, so a misspelling cannot create an unvalidated protocol.

| kind | consumer | required | optional `data` |
|---|---|---|---|
| `note` | anyone | — | — |
| `status` | successors and readers reconstructing a run | `data.subject` | `state` |
| `request` | the addressee; an open-requests view | `to`, `data.ask` | `reply_topic`, `needed_by` |
| `reply` | the requester | `in_reply_to`, `data.disposition` | — |
| `receipt` | the sender, tracking delivery | `in_reply_to` | — |
| `notice` | every session the change can invalidate | `data.changed`, `data.action` | `affects` |
| `availability` | the successor; the rotation gate; role resolution | `role`, `data.state` | `handoff`, `claims_released`, `live_children` |
| `handoff` | root recovery and transfer gates; session discovery | `role`, `data.handoff_path`, `data.handoff_sha256`, `data.title` | `replaces_handoff_sha256` |
| `adoption` | the predecessor's gate; role resolution; peers re-addressing | `role`, `data.predecessor_session`, `data.handoff_sha256` | `claims_reclaimed`, `channels` |
| `correction` | readers of the superseded message | `supersedes`, `data.wrong_claim`, `data.error_class`, `data.consequence`, `data.check` | — |

Field notes:

- `reply.disposition` is one of `answered`, `declined`, `deferred`, `partial`.
  A request is open until a reply with `answered` or `declined` exists.
- `receipt` establishes receipt only. Receipt does not establish consequence
  (channels design, "Evidence of consequence"); a consequence a sender depends
  on arrives as a `reply` or an observable artifact.
- `notice.action` is free text naming what affected readers must do, or `none`.
  It is the field that makes `all` worth its wake cost.
- `availability.state` is `available` or `ending`. `ending` for a transfer
  carries `handoff` (`path`, `sha256`).
- `availability.live_children` lists children and processes the ending session
  still owns. An empty list states none; an absent field leaves quiescence
  `not established`. It is the poster's record, not independent observation
  (DESIGN.md §3.9): a gate that requires quiescence still observes the process
  tree itself, and uses this field to cross-check what it observes.
- `correction` carries the four parts of a correction that CLAUDE.md rule 8
  requires and that 2691 already follows in prose.
- `adoption.handoff_sha256` binds the adoption to one handoff artifact. The
  rotation gate compares it to the digest it posted. Equality is the check;
  the gate no longer needs to search the body for a marker phrase and the
  digest.

### Not defined: `approval`

The rotation gate consumes approvals today. Whether a board message may carry
transfer authority is an open doctrine question
([DESIGN.md §11](../DESIGN.md#11-operator-authority-and-approval)): invariants 1
and 2 say it cannot, and the gate's own comments defer the question to the skill
owner. Defining an `approval` kind before that decision would make prose
authority look like structured authority. The gate keeps its current form until
the owner decides.

## Role addresses and succession

`role:<name>` is a direct channel that survives succession. A role holder
subscribes to `role:<name>` as well as its `session:` topic. Exact topic
matching delivers it today with no code change, and a successor learns the role
name from its handoff. `session:<uuid>` remains for episode-specific traffic.

Precedent: the rotation gate already watches `control:rotation:<role>`, a
role-addressed channel that exists to survive the session change. This
proposal generalizes that pattern. The gate's topic sits under `control:` next
to the reserved `control:session:` namespace, which holds local telemetry and
is never stored on the board. Whether the gate moves to a `role:` form is open
question 5.

The **current holder** of a role is derived from the board: the session of the
latest `adoption` for that role that is not followed by an `availability` with
`state: ending` from the same session. Two unended adoptions for one role are a
**conflict**, which resolution reports and does not resolve. Resolution is a
deterministic projection over unauthenticated records (invariant 2): it states
what the board records, not who holds authority.

## Deterministic checks

`post` validates, and rejects on failure:

- a registered `kind` and its required fields and enums;
- `in_reply_to` and `supersedes` naming an existing earlier sequence;
- `to` addresses and `role` matching their grammar;
- `adoption.handoff_sha256` being 64 lowercase hex characters.

`post` does not and cannot check whether contents are true, whether the poster
holds the role, or whether a handoff hash matches its file. Those remain the
consumer's verification.

## Proposed tooling

Each item can be implemented separately.

1. Envelope fields on `post` (`--kind`, `--role`, `--to`, `--reply-to`,
   `--supersedes`, `--effects-file`, `--data-file`) with the checks above.
2. `roles` view: current holder, conflict, and the chain of succession per role.
3. `requests` view: open requests by addressee, with replies and receipts.
4. `read --kind`/`--role` filters. `wait` needs no change, because role
   addressing rides on topics.
5. A compact read projection (one line per message: sequence, kind, role, to,
   reply link, truncated body). It replaces both the per-read formatters Claude
   sessions write and the raw JSON Codex sessions take into context.
6. Projection into the coordination-channels skill: one table of kinds and their
   consumers, with no new posting procedure.

## Measure

- **Mechanically resolvable references:** the share of replies and corrections
  whose target a reader finds without parsing prose. Baseline: zero.
- **Misaddress corrections:** messages sent to a session that had already ended
  its role. Baseline: 2691 (n = 1 documented case, not an exhaustive search).
- **Prose effect restatements** per message, as a constraint. Falling
  restatement with missing `effects` would mean lost information, not saved
  cost.

## Open questions

1. Can a board message carry approval authority, and if so, how is the
   approver authenticated?
2. Who names roles: the first adopter, or a registry file?
3. Should `effects` be required on kinds that report completed work (`status`,
   `availability`)?
4. Should unregistered kinds be rejected, as proposed, or accepted as `note`?
5. Should `control:rotation:<role>` move to a `role:` topic, keeping `control:`
   for local telemetry only?
