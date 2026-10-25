# Chatboard message contracts — implementation plan

## Status and authority

- Status: **draft, revised after round-1 review, the tier-2 review (round 2),
  an external review with user decisions (round 3) and user decisions D9–D11
  with a tier-1 review (round 4), and a reviewer-2 confirmation review
  (round 5), and amended with the user's acceptance decisions (round 6).
  Accepted for implementation by the user, 2026-10-25, after the round-6
  amendments.** The acceptance is the user's (relayed by the coordinator in
  `planner-7-packet-v1.md`). This document itself grants no commit, push,
  board-post, claim, instruction-change or experiment authority beyond what
  that acceptance supplies (below).
- Author: planner-1 (Claude Code subagent `a3a6cea992ed00053`), packet
  `planner-1-packet-v1.md` sha256 `e836577950d5df1918c179cba3708a345c7c521ea0d5958ee75c99b3982b1e9e`.
  Round-1 revision: planner-2 (subagent `a31f4077025b6ed87`), packet
  `planner-2-packet-v1.md` sha256 `0f4af87fbf8b680960f81740eadd12ae237cc010aec491645e7fcfa65cda3a77`,
  from plan sha256 `feb72531…d14a` and `reviewer-1-findings.md` sha256
  `1832112c…c476`. Round-2 revision: planner-3 (subagent `a61f21cb5b1276ebe`),
  packet `planner-3-packet-v1.md`, from plan sha256 `8601b12b…6f53` and
  `fable-reviewer-1-findings.md` sha256 `94a0841e…12a3`. Round-3 revision: planner-4
  (Claude Code subagent; its ID is `not established` by the planner), packet
  `planner-4-packet-v1.md` sha256 `5150d01f…9eb61`, from plan sha256 `932002c0…23c5`
  and `root-handoff-v2.md` sha256 `61f7d3ac…5a5f` (external review X1–X5, user
  decisions of 2026-10-24), then X6 from `planner-4-packet-v1-addendum-1.md`
  sha256 `5d9a6419…705b`, applied to the intermediate plan sha256 `97bf2d3d…4e3b`
  (v4). Round-4 revision: planner-5 (Claude Code subagent), packet
  `planner-5-packet-v1.md`, from plan sha256 `75684397…8a71` (v5) and
  `reviewer-2-findings.md` sha256 `56496b86…4dfa` (user decisions D9–D11 of
  2026-10-25). Round-5 revision: planner-6 (Claude Code subagent
  `a192e0ecaed39c2a9`), packet `planner-6-packet-v1.md`, from plan sha256
  `1d437f97…e855` (v6) and `reviewer-2-confirmation.md` sha256
  `169eddfc…c2fe`. Round-6 revision: planner-7 (Claude Code subagent
  `a2b7b850629e05713`), packet `planner-7-packet-v1.md`, from plan sha256
  `37e987f7…3a2` (v7) and the user's acceptance decisions of 2026-10-25.
  Coordinator: Claude Code root
  `8ef14d91-27be-434a-8b47-607702704708` (formerly `8c4ceab7-…`, `60e24438-…`).
- Contract source: `docs/codex-chatboard-contracts.md` (the draft). It is
  tracked since `a1a1e269` (bytes unchanged) and self-described as "design
  draft, not implemented, not reviewed". This plan implements its envelope and tooling sections; it does
  not edit or ratify the draft.
- Binding constraints: settled facts 1–7 of the root handoff (not reopened
  here); DESIGN.md §1.5, §3.6, §5.

The user accepted this plan on 2026-10-25 (round 6). Acceptance supplies
implementation authority only for the accepted slices, their write scopes and
the decisions below as accepted. It does not answer user-open question 1, and
acceptance does not wait on Q1.

## Revision binding

Bound HEAD remains `a1a1e269`. The packet named HEAD `ea4e6144`; the coordinator
rebased the subject to `a1a1e269`, which commits the draft with unchanged
bytes. `ea227a9f` (between them) changes only the rotation gate. The round-1
review ran at `3b2d00ec`, which adds only
`metrics/probes/tmux-root-rotation-20261024.json`. This revision was written at
observed HEAD `35060584`, which changes only `metrics/scripts/root_rotation_coordinator.sh`
(+47, a window-ambiguity refusal); the adapter, draft and `skills/durable-state/`
are identical at `a1a1e269` and `35060584` (`git diff --stat` empty for those
paths). The gate's adoption comment moved from l.239–245 to l.286–292.
The tier-2 review ran at `4b80c2bb`; the round-2 revision was written at
observed HEAD `9af4e514`. Both commits after `35060584` change only
`metrics/probes/tmux-root-rotation-20261024.{json,md}`; `git diff --stat
a1a1e269 9af4e514` over the adapter, the draft, `skills/durable-state/` and
`context_probe.py` is empty. The round-3 revision was written at observed HEAD
`05783a8d`; `git diff --stat a1a1e269 05783a8d` over the same paths is empty,
and the adapter and draft digests below were re-checked at that HEAD. The
round-4, round-5 and round-6 revisions were written at the same observed HEAD
`05783a8d`, with both digests re-checked.

| subject | identity |
|---|---|
| HEAD | `a1a1e26963068c1be7e70d76b50eb03802ad6056` (`rescue-detached-20260911`) |
| `docs/codex-chatboard-contracts.md` | sha256 `0a70ee7704baa4aac240f893bfbaea9851c06db8134d0fd4a5b9c82265705f93` (tracked since `a1a1e269`, 259 lines) |
| `skills/durable-state/scripts/codex_chatboard.py` | sha256 `a740a0649141709ed3d9fde4036fdbcad362e4dc7d59cf92f8266e1eefe79648` (unchanged since `ea4e6144`) |
| chatboard tests at HEAD | `python3 -m unittest tests.test_codex_chatboard` from `skills/durable-state`: 24 tests, OK (run 2026-10-24) |

A slice that starts on a different adapter or draft digest re-reads the changed
parts before relying on this plan's line references.

## Code facts the slices rely on

Read from `codex_chatboard.py` at the digest above:

- `encode()` (l.69–70) writes `json.dumps(..., indent=2, sort_keys=True)`. Key
  order is therefore not a compatibility risk; byte compatibility of ordinary
  rows reduces to **no new key is present unless its flag was given**, the
  pattern `wake` uses (l.130–133).
- `mutate()` (l.73–88) calls `change(board)` on a freshly decoded snapshot on
  every CAS retry; a `store.publish` failure containing `revision conflict`
  (l.80–85) triggers the retry. Validation placed inside `change` therefore
  re-runs against each snapshot it would publish (settled fact 3). Per attempt
  the order is read → `change` → publish, and `change` (l.78) runs outside the
  `try`: a `ChatboardError` (a `ValueError`, l.32) raised by `change` propagates
  out of `mutate` with no publish and no retry.
- `post()` idempotency (l.119–136) compares the whole candidate row with any
  existing row of the same `message_id`. Envelope fields join that comparison:
  a replay must repeat them exactly. Cross-message checks must compare against
  the candidate's own `sequence` (the existing one on replay), not
  `next_sequence`, or a valid replay could fail.
- `read_view()` (l.192–205) slices the newest `limit` rows before any
  filtering. A `--kind`/`--role` filter must filter **before** limiting, as
  `wait_board` already does (l.381–388: `matches = …`, then
  `rows = matches[:limit]`; test
  `test_existing_messages_filter_before_limit_and_cursor_does_not_skip_matches`).
- `wait_board()` matches topics exactly and repeatably (l.387
  `row["topic"] in selected`; CLI `--topic` l.438–439), so `role:<name>`
  delivery needs no adapter change (draft "Proposed tooling" 4) **only when the
  row's `topic` is that address**. `to` is not read by `wait`; a row addressed
  `to` a role on another topic does not wake that role's waiter. D10 (a),
  decided 2026-10-25, adds delivery by `to` in S2 (X2).
- The post result shape is `{"revision", "message"}` (l.143). New keys appear
  only inside `message` and only when given.

## Decisions for plan acceptance

Every row is decided. D2 ownership was decided by the user on 2026-10-24 and
D9, D10 and D11 on 2026-10-25; on 2026-10-25 (round 6) the user accepted the
proposed defaults of D1, D2 location and D3–D8 as written.

| id | question | options | decision (default as proposed) |
|---|---|---|---|
| D1 | **Placement** (settled fact 2) | (a) extend the Python adapter now, kind registry as a JSON contract file a later Rust port reads; (b) new Rust board component now; (c) Python with kinds hard-coded | **Decided (a) by the user, 2026-10-25 (default accepted).** **(a).** No Rust board exists and `docs/rust-development.md` does not authorize rewriting existing components. The registry file is the port seam: a Rust reader consumes the same file. |
| D2 | Registry location, and **who owns the kind contracts** | Location: (a) `skills/durable-state/contracts/codex-chatboard-kinds.v1.json`, beside the adapter; (b) under `docs/`. Ownership: (i) the draft owns the kind contracts (draft l.76–78); (ii) ownership moves to the registry; the draft's kinds table becomes its prose projection | **Ownership decided (ii) by the user, 2026-10-24.** The JSON registry is authoritative for kind contracts; the draft's Kinds table (draft l.145–155) and its field-note enumerations are its prose projection, and a kind change edits the registry first. Draft l.76–78 still say "this document owns the kind contracts"; revising them is the **draft owner's** change, a dependency this campaign records and does not perform (R2). The parity test runs docs → registry (S1 Checks). Location **(a), decided by the user, 2026-10-25 (default accepted)**: the adapter resolves it relative to its own file, so an installed skill copy carries it. A copy of the script alone does not (`experiments/agent-experiment-control/live/context_probe.py:19` copies only `codex_chatboard.py` and `durable_state.py`); S1's lazy-load rule covers that case wherever the file lives. |
| D3 | Open question 2: who names roles | (a) first adopter names it; grammar only; (b) a roles registry file | **Decided (a) by the user, 2026-10-25 (default accepted).** **(a)**, with grammar check at post and conflicts reported by the `roles` view. A registry file can be added later without changing stored rows. A first holder cannot post a valid `adoption` (no predecessor or handoff), so it does not appear as holder until the first succession: D9, R7. |
| D4 | Open question 3: `effects` required on `status`/`availability` | (a) optional everywhere; (b) required on those kinds | **Decided (a) by the user, 2026-10-25 (default accepted).** **(a)** for this campaign; the uptake script reports the share of those kinds carrying `effects`, which is the evidence the user's decision would need. Making it required later is a registry change, not a schema change. |
| D5 | Open question 4: unregistered kinds | (a) reject at post, naming the registered kinds and `note`; (b) accept and store as `note`; (c) accept and store as-is, unvalidated | **Decided (a) by the user, 2026-10-25 (default accepted).** **(a)**, the draft's proposal: a misspelling cannot create an unvalidated protocol, and the set stays open through `note` plus registry additions (DESIGN §3.6). Readers do **not** reject a stored unknown kind (a newer registry may have written it); they display it as recorded. |
| D6 | Open question 5: `control:rotation:<role>` → `role:` | (a) leave unchanged in this campaign; (b) move | **Decided (a) by the user, 2026-10-25 (default accepted).** **(a).** The topic belongs to the gate, owned by the tmux session (settled fact 5); any move is that session's change after the user answers. |
| D7 | Extra keys in `data` | (a) accepted, unvalidated; (b) rejected unless listed | **Decided (a) by the user, 2026-10-25 (default accepted).** **(a).** Only required fields and enums are checked (DESIGN §1.5: require only what a consumer needs to tell valid from invalid). `effects` stays closed to its two lists, because overlap checks read exactly those. |
| D8 | Recomputable baselines | (a) commit extraction and aggregation scripts under `metrics/scripts/` with a frozen input set before rollout; (b) leave them in the assignment directory | **Decided (a) by the user, 2026-10-25 (default accepted).** **(a)**, see slice M and Measurement: a post-rollout comparison is only meaningful if the same script reproduces the baseline from the same inputs. |
| D9 | **What `roles` reports, including a first holder** (X1 + R7, one decision) | (a) the view reports a *reported* holder, never an owner, from `adoption` and `ending` reports, with succession relations classified per role; (b) the draft owner adds a first-holder `adoption` form before S3 (draft change, outside this campaign) | **Decided (a), qualified, by the user, 2026-10-25.** An `adoption` row records that a session *said* it adopted; it can be posted before the transition it reports commits, and the gate's acceptance is not on the board in a form the view reads. The view therefore never prints "holder" or "owner": the field is `reported_holder`, and the output header states it is a projection of unauthenticated reports, not committed ownership (draft l.204–209, invariant 2). A role with no `adoption` reports `reported_holder: not established` and lists the sessions that posted `availability` or rows with that `role` as `reporting_sessions`; the first-holder form stays a question to the draft owner. **Every succession entry** (one per `adoption`: predecessor, successor, adoption sequence, predecessor `ending` sequence or `not reported`) carries `committed: not established`, including a clean A→B whose predecessor has posted `ending`: no board row shows that a rotation committed. **Overlap is not conflict, and fork is neither:** B's adoption naming A as `data.predecessor_session` while A has not reported `ending` is `overlap` (succession reported, not completed); two or more unended adoptions naming the same predecessor (B and C both from A) are a `fork`; two unended adoptions linked by no predecessor chain and sharing no predecessor are a `conflict`. S3 defines the role state for chains, mixtures and all-ended roles. Being addressed to a role (D10) is not evidence of reported holding either. This narrows the draft's "two unended adoptions … are a conflict" (draft l.205–207); the draft owner revises that sentence (R2). If (b) lands first, S3 adds the form's fixture; nothing stored changes. |
| D10 | **Binding `--to` addresses to delivery** (X2) | (a) `wait` delivers by `to` as well as `topic`: a waiter on topic `role:X` or `session:U` also wakes on rows whose `to` contains that address, whatever their `topic`; one row serves any number of addressees; (b) post-time rule: with `--to`, `topic` must be one of the `to` addresses; with several addresses only the topic's address is guaranteed a wake, and the post cannot see who else watches the topic; (c) no binding; `to` is advisory and delivery stays the poster's choice of topic | **Decided (a) by the user, 2026-10-25.** The wake preserves the original message byte-unchanged and states, outside the row, whether it matched by topic, by recipient (`to`), or both; a row matching both is delivered once (S2). **Invariant:** being addressed to a role does not establish that the recipient holds it; delivery is not authorization. The draft separates the two on purpose: `topic` is the channel `wait` matches, `to` names who must act, and "several addressees on one `aspect:` channel are common, and a topic cannot express them" (draft l.124–126); a role holder subscribes to `role:<name>` and its `session:` topic (draft l.192–193). No draft sentence binds `to` to `topic`: basis, `grep -n '`to`'` over the whole draft (259 lines) hits l.117, 124, 149, 217 and 228, and none binds `to` to `topic` (reviewer-2 P3-3). So under (c) a `request` `to role:X` on an `aspect:` topic reaches X only if X watches that channel. (a) is the only option that delivers every addressee of a multi-recipient row without per-recipient copies (which would split `in_reply_to` chains). It changes `wait_board`'s match (l.387) and contradicts draft l.193–194 ("Exact topic matching delivers it today with no code change") and l.232–233 ("`wait` needs no change"), which become draft-owner revisions (R2). Rows without `to` match exactly as at HEAD. The adapter cannot have written `to` (adapter `post()` builds the row from fixed keys, l.119–133); whether another writer has stored `to` is `not established` (S0 class 4). `request.data.reply_topic` (draft l.149) is unchanged. Enforced in S2 Checks. |
| D11 | **A dedicated `adopt` operation** (X6) | (a) typed `post --kind adoption` only, as now; (b) an `adopt` subcommand that writes the same row, and `post --kind adoption` is still accepted; (c) `adopt`, and `post` rejects `kind: adoption` | **Decided (c) by the user, 2026-10-25.** At the bound adapter the subcommands are `read`, `wait`, `topics`, `post`, `claim`, `release` (adapter l.431–472); no adopt operation exists in the draft or the adapter. `adopt` accepts only what succession needs and makes exactly one append, which gives a host permission rule a narrow command it can allow on its own without allowing the broad `post` (S5 note, X4 point 2), and it checks the poster's own handoff file against the digest. `post` refuses `kind: adoption` by registry lookup, not a hard-coded exception: the registry marks `adoption` as written only by `adopt` (a per-kind `writer` attribute), so the D5 check stays data-driven. `adopt` validates the `adoption` required fields and delegates to the same `mutate()` board path: no separate storage, no special trust in the caller, no authority to commit a rotation by posting. It authenticates no one and confers no transfer authority (Q1 stays open). **X6 closes only on a live run:** `adopt` succeeding through a real Claude successor under the intended permission configuration; a shell-stub test does not close it, and S1 alone cannot (S5). |

**User-open, not assumed by any slice:**

1. Can a board message carry approval authority? `approval` stays undefined;
   no slice defines it and the gate keeps its approval form. Explicitly
   unresolved (user, 2026-10-25): the board transports approval assertions but
   does not establish approval authority; plan acceptance does not wait on Q1.

**Resolved by the user (relayed by the coordinator, 2026-10-24):**

6. Rewrite the draft's "Why" to lead with the invariant argument? **No** —
   the "Why" is not rewritten. Closed.
7. Commit the draft? **Yes** — committed in `a1a1e269`. Closed.

**Decided by the user, 2026-10-24 (round 3):**

- D2 ownership → **(ii)**: the registry is authoritative; see D2.
- **Row contract version → yes, optional.** A typed row (one with `kind`)
  carries the registry version it was validated against (S1); rows without
  envelope fields stay byte-identical; `schema_version` stays 1.

**Decided by the user, 2026-10-25 (round 4; localized decisions, no redesign):**

- D9 → **(a), qualified**: `roles` reports a reported holder, never an owner;
  A→B is `overlap`, B and C both from A is a `fork` (a separate state), and
  every succession entry carries `committed: not established`; see D9, S3.
- D10 → **(a)**: `wait` delivers by `to` as well as `topic`, reports the match
  reason outside the row, delivers a row matching both once; delivery is not
  authorization; see D10, S2.
- D11 → **(c)**: dedicated `adopt`; `post` refuses `kind: adoption`; X6 closes
  only on a live Claude-successor run under the intended permission
  configuration; see D11, S1, S5.

**Decided by the user, 2026-10-25 (round 6: acceptance):**

- The plan is accepted for implementation after the round-6 amendments
  (Status).
- D1–D8 → the proposed defaults, as written in their rows; open questions
  2–5 are thereby answered by D3–D6.
- Planner defaults accepted: `adopt` adds `to: ["role:<role>"]` automatically
  (S1, P2-A); an uppercase `--predecessor-session` is rejected, not normalised
  (S1); mixture precedence `conflict` > `fork` > `overlap` (S3); predecessor
  chains traverse ended adoptions (S3, P3-4).
- Q1 stays explicitly unresolved; acceptance does not wait on it.
- No Fable review of the plan; Fable reviews the S1 implementation diff and
  the workstream acceptance boundary before S4 (Review policy).
- Registry format: the plan's own format, not JSON Schema (S1 Registry
  content).
- S1 additions: a registry-derived `kinds` view and field-naming rejection
  messages, each with a check (S1).
- S2↔S5 integration condition: the gate's adoption recognition checks the
  row's actual `topic`, tested before the gate relies on the D10 `wait` (S2,
  S5).
- Typed adoption via `adopt` is authoritative for gate recognition; the X6
  live validation passes `--handoff-path` (S5).

## Slices

Dependency graph (an edge means "must land before"):

```
S0 → S1 → S2 → S3 → S4
M  ──────────────→ S4
S1 → S5
```

S2 and S3 share write scope (`codex_chatboard.py` `parser`, the same new test
file), so they are serialised, S2 first. M depends on nothing and may run at any
time before S4. S5 (gate) needs S1 and may run in parallel with S2–S4. Each
slice is one commit (or a short series) whose checks pass on its own.

Common check, run from `skills/durable-state` in every slice that touches the
adapter, with the pinned context-status binary (path and sha256 from the
coordinator's assignment record; at plan time
`/home/bline/.local/state/work-engine/tools/context-status/7a0cbee1b4f8bfbf3cb0a0c1f2c7e6b7a34454ff2ea63b14ca79f97c83345c18/work-engine-context-status`):

```
CONTEXT_STATUS_TEST_BINARY=<pinned binary> PYTHONDONTWRITEBYTECODE=1 \
  python3 -m unittest tests.test_codex_chatboard tests.test_context_control
```

Pass requires OK with `skipped=0`: without the variable all 10
`test_context_control` tests skip (reviewer-1 P3-1: 34 run, skipped=10).
`test_context_control.py` loads `scripts/codex_chatboard.py` directly (l.17–20).

### S0 — consumer census (read-only)

- Owner: this campaign.
- Write scope: `metrics/probes/chatboard-consumer-census-<date>.json` (new).
- Purpose: settled fact 4's consumer list came from a name grep and is not
  complete. Compatibility is not claimed until this census exists.
- Search domain and artifact classes, each with its command:
  1. direct invocation or import of the adapter:
     `git grep -n 'codex_chatboard'` over the repository, plus the same grep
     over `~/.codex/AGENTS.md`, `~/.codex/skills/`, `~/.claude/` skills and
     agents (installed copies are outside the repository). Script-only copies
     count here: `context_probe.py:19` copies the adapter without `contracts/`
     and is recorded `affected` unless S1's lazy registry rule holds;
  2. indirect invocation through a variable: `git grep -n 'BOARD_CMD\|CHATBOARD'`
     and every script that sets or exports such a variable;
  3. readers of board JSON output (`read`, `wait`, `topics`, `post` results):
     `git grep -nE 'next_since|\["messages"\]|get\("messages"\)|\.messages\b|\["message"\]|get\("message"\)|\.message\b|\["revision"\]|\.revision\b'`,
     each hit read to confirm it parses board output (the `message`/`revision`
     alternatives cover post-result readers). For each `wait` consumer the
     census also records a semantic criterion that no key check detects:
     **relies on returned rows' `topic` ∈ selected topics**. Under D10 (a) a
     consumer meeting it is `affected` even if it reads only existing keys. The
     gate's adoption wait is named explicitly (`root_rotation_coordinator.sh`
     l.216, l.272 wait on `session:$sid`; l.293–299 match only `session_id`,
     marker and digest, taking the topic filter as given), although S5 owns it;
  4. direct readers of the stored payload, bypassing the adapter:
     `git grep -n 'codex-chatboard/v1\|work_engine_codex_chatboard_v1\|codex-chatboard'`
     including `rust/` and `app-server/`;
  5. dispatch packets and handoffs that embed board commands (the draft notes
     these were not searched): the coordinator names the directories to scan;
     a partial scan is recorded as partial.
- Acceptance: each consumer listed with file:line, the fields it reads, and a
  disposition (`unaffected: reads only existing keys`, `affected`, or
  `not established`); `unaffected` is not available to a `wait` consumer that
  meets the class-3 topic criterion. The census states its domain and which classes were
  scanned completely. A class not scanned leaves compatibility for that class
  `not established`, stated in S1's acceptance.

### S1 — kind registry, envelope on `post`, and `adopt` (D11)

- Owner: this campaign. Depends on S0 and on D1, D2, D5, D7, D11. Records, does not
  perform, a draft-owner dependency: under D2 (ii) draft l.76–78 must stop
  claiming kind ownership, and the draft's Envelope table (l.113–121) gains the
  row contract-version field. S1 does not wait for it; until it lands the draft
  and the registry disagree about ownership, not about content (the parity test
  holds the content equal).
- Write scope: `skills/durable-state/contracts/codex-chatboard-kinds.v1.json`
  (new); `skills/durable-state/scripts/codex_chatboard.py` (`post`, `adopt` (D11), a registry
  loader, a pure `validate_envelope`, `parser` post flags, the `kinds` view);
  `skills/durable-state/tests/test_codex_chatboard_contracts.py` (new);
  `docs/codex-chatboard.md` (the operator reference, draft l.3–5: the new post
  flags, envelope checks and replay strictness, R4).
- Read set: the draft (Envelope, Kinds, Deterministic checks); adapter l.44–145,
  427–477; S0 census; `docs/codex-chatboard.md` post section.
- Registry content: `artifact_type`, `version: 1` (the value typed rows record,
  below), the envelope field set (`kind`, `role`, `to`, `in_reply_to`,
  `supersedes`, `effects`, `data`, the draft Envelope table l.113–121, plus
  `contract_version`, which the registry adds; reviewer-2 confirmation P3-1),
  the envelope grammar (`role`
  pattern `^[a-z0-9][a-z0-9._-]{0,63}$`; `to` address patterns `role:<role
  pattern>` and `session:<canonical uuid>`, as `canonical_uuid` normalises
  `session_id`), and per
  kind: required paths, enums (`reply.disposition`, `availability.state`),
  optional `data` names, which may be nested paths (`availability` lists
  `handoff`, `handoff.path` and `handoff.sha256`, draft l.166–167, as
  optional; reviewer-2 confirmation P2-B), consumer text, and an optional `writer` naming the
  only subcommand that may write the kind (D11 (c): `adoption` has
  `writer: adopt`; a kind without `writer` is written by `post`). `note` is
  registered with no required fields.
- Registry format (user decision, 2026-10-25): the plan's own format above,
  not JSON Schema; its types, required fields and enums stay a subset that
  maps one-to-one onto JSON Schema, should a second consumer appear.
- Behaviour:
  - New flags `--kind --role --to (repeatable) --reply-to --supersedes
    --effects-file --data-file`; each adds its key to the row only when given.
  - Row contract version (user decision 2026-10-24): when `--kind` is given, the
    adapter adds `contract_version` (an integer, the registry's `version`) to
    the row; there is no flag for it. A row without `kind`, including one with
    other envelope fields, carries none, and a row without envelope fields is
    byte-identical to HEAD. `schema_version` stays 1. Readers display a stored
    `contract_version` as recorded and do not reject one they do not know (as
    the D5 reader rule does for kinds). It is part of the row, so replay
    compares it (R4).
  - The `role` pattern accepts the draft's examples (`s10-root`,
    `claude-metrics`, draft l.116) and the gate's one observed `$ROLE` value
    (`claude-root`, `launch_claude_successor.sh` l.22; n = 1, the gate's value
    set is a tmux window name and is not enumerated). A gate role outside the
    pattern is S5's to check against the compatibility note.
  - The registry is opened only when a flag that needs it (`--kind`, or a
    kind-dependent check) is given. A post with no new flags never opens it, so a
    script-only copy keeps working for ordinary posts. A missing registry is an
    error only then, and the message names the expected path.
  - `post --kind K` where the registry gives `K` a `writer` other than `post`
    (today only `adoption`) is refused before `mutate()`, with no board
    change, and the message names the writing subcommand (`adopt`).
  - Single-message checks run before `mutate()`: registered kind (D5), required
    fields and enums, `role`/`to` grammar, `effects` shape (when present, both
    `shared_writes` and `external_actions` are required, since an absent list
    would be ambiguous between none and `not established`; each is a list of
    nonempty strings, empty allowed; other keys rejected),
    `adoption.data.handoff_sha256` matching `^[0-9a-f]{64}$`.
  - Cross-message checks run **inside** the `change` function (settled fact 3):
    `in_reply_to` and `supersedes` name a sequence that exists in the snapshot
    and is lower than the candidate's own sequence. A **closing** `reply`
    (`data.disposition` `answered` or `declined`) must also name, by
    `in_reply_to`, a row whose `kind` is `request`, because S3's `requests`
    view closes a request from exactly those replies (X3; DESIGN §1.5). A
    `deferred` or `partial` reply, and a `receipt`, need only an existing
    earlier row.
  - `adopt` (D11 (c)). It shares the registry loader and `validate_envelope`,
    validates the registry's `adoption` required fields, and delegates to the
    same `mutate()` path and replay comparison as `post`: no separate storage,
    no special trust in the caller, no authority to commit a rotation by
    posting. It is the only writer of `kind: adoption` (registry `writer`).
    Arguments: author, session id, body, `--role`, `--predecessor-session`,
    `--handoff-sha256` and/or `--handoff-path`, and optional
    `--claims-reclaimed` and `--channels`. Other envelope flags (`--kind`,
    `--to`, `--reply-to`, `--supersedes`, `--effects-file`, `--data-file`)
    are not accepted. **Topic is not free** (reviewer-2 P3-6): `adopt` takes
    no `--topic` and writes on `session:<its own session id>`, the topic the
    gate waits on (`root_rotation_coordinator.sh` l.216; launcher l.36–37), so
    a permission rule naming `adopt` cannot reach non-`session:` topics. It
    can still reach any `session:<uuid>` topic under any session identity,
    because `--session-id` is caller-supplied and only its syntax is checked
    (adapter l.109); this is not authentication (reviewer-2 confirmation
    P3-2). **Role addressing (plan default, accepted by the user
    2026-10-25; reviewer-2 confirmation P2-A):** `adopt` adds `to: ["role:<role>"]` from
    its own `--role` (not a caller-supplied `--to`), so that under D10 a peer
    waiting on `role:<role>` is woken by the adoption: the draft names "peers
    re-addressing" as an `adoption` consumer (draft l.154), and role traffic
    travels on `role:<name>` (draft l.192–193). The topic stays fixed. The
    wake depends on S2; until S2 lands, peers learn of the adoption only by
    reading the board. Being addressed to the role is not holding (D10
    invariant); the row is a holding report because of its kind, as for any
    `adoption`. The rejected alternative was to record "peers re-addressing" as
    served by `read`/`roles` only and leave `to` unset. `body` stays
    free prose: a board row requires a nonempty body (adapter l.111), and
    prose carries no checked field. It writes `kind: adoption` and the
    registry's `contract_version`, and makes no other board mutation.
    Checks: the digest is 64 lowercase hex; when a path is given, the sha256
    of the file's bytes must equal the given digest, or becomes the digest
    when none is given; `predecessor_session` is a canonical UUID **already in
    lowercase form** (uppercase is rejected, not normalised, unlike the
    adapter's `canonical_uuid` return value, which `post` discards for
    `session_id`, adapter l.96–104, l.109) and different from the poster's
    session, compared case-insensitively (`canonical_uuid` accepts an
    uppercase `session_id`, adapter l.96–104, so an uppercase poster session
    paired with the same lowercase predecessor must still count as self;
    reviewer-2 confirmation P3-2); it is stored as given, which is therefore the canonical form the
    gate compares with `PRED_SESSION` by string equality (S5). These rules
    live in `adopt` only, because only `adopt` writes adoptions. The file
    check is byte identity of the poster's file at post time. It is not a
    check of the handoff's contents or truth, and a consumer still verifies
    the digest against its own copy. It does depart from draft l.220–222
    ("`post` does not and cannot check … whether a handoff hash matches its
    file"), and that is a draft-owner revision (R2).
  - A row with envelope fields but no `kind` is accepted and reads as `note`.
  - **Compact kind view** (user decision, 2026-10-25): a `kinds` subcommand
    prints one line per registered kind (name, writer, required fields, enums
    with allowed values, optional `data` names), or only the selected kind's
    line with `kinds K`; `post --kind K --help` prints K's line above the
    ordinary help where the parser allows it. At least the `kinds`
    subcommand is built; the S1 result records whether the `post --help`
    form was. The view is rendered from the loaded registry, never from text
    in the adapter; it reads no board. `kinds` is a new subcommand, so S0's
    class-3 census covers it like the other subcommands.
  - **Rejection messages** (user decision, 2026-10-25): every single-message
    and cross-message rejection names the field path that is missing or
    invalid and, where the registry or grammar defines them, the allowed
    values (the enum's members, the registered kinds plus `note`, the
    address patterns, the `effects` keys).
  - No check of truth, role holding or handoff-file contents (draft
    "Deterministic checks"; invariant 2).
- Checks (new tests, names indicative):
  - byte compatibility: a post with no new flags produces the same encoded
    board bytes as a fixture captured from the HEAD adapter for the same
    inputs, run once with the registry present and once with the registry file
    absent (a script-only copy);
  - each registered kind accepted with its required fields and rejected
    without each one; enum violations rejected; unregistered kind rejected
    with a message naming `note`;
  - dangling and forward `in_reply_to`/`supersedes` rejected;
  - closing reply (X3): an `answered` and a `declined` reply whose `in_reply_to`
    names a `note`, a `status` or a legacy row without `kind` are rejected
    with a message naming `request`; the same replies naming a `request` are
    accepted; a `deferred` reply naming a `note` is accepted. The check runs
    inside `change`: a variant of CAS case (c) below whose rewrite replaces the
    target `request` with a non-`request` row rejects on retry;
  - row contract version: a typed post's row has `contract_version` equal to
    the registry's `version`; an envelope-only post without `--kind` has no
    `contract_version`; the byte-compatibility fixture (no new flags) has none;
    a stored row with an unknown `contract_version` is read back unchanged;
  - CAS retry, deterministic (no existing test exercises the `revision
    conflict` retry; the existing `ThreadPoolExecutor` tests drive waiters):
    wrap the store's `publish` so its first call changes the stored board and
    then raises the revision conflict. Count `change` and `publish` calls.
    (a) A forward reference equal to the stale `next_sequence` is rejected at
    the first `change`: no `publish` call, no retry (the row did not exist when
    the poster acted). (b) With an append injection and a reference to a row
    that already existed, the post is published once with `sequence` equal to
    the fresh `next_sequence`, and `change` ran twice. (c) A second injection
    rewrites the stored board without the referenced row before raising the
    conflict (it simulates a recovered or reset board, not ordinary
    append-only traffic): the retry rejects, nothing is published, and `change`
    ran twice. Under append-only traffic, placing the check inside `change`
    changes no accept/reject outcome. It matters for a correct `sequence` on
    retry and for non-append changes;
  - idempotent replay with identical envelope succeeds; replay with a changed
    envelope raises the existing "different content" error;
  - docs → registry parity (D2 (ii); supersedes X5's names-and-required-fields
    test): the registry is the expected value and each document is checked
    against it, over every attribute that document projects. For the draft:
    every row of the Kinds table (draft l.145–155) — kind name, consumer text,
    required fields, optional `data` names — and the enumerations stated in its
    field notes (`reply.disposition`, draft l.159; `availability.state`,
    l.166). For `docs/codex-chatboard.md`: whatever kind attributes S1 writes
    there. A kind in the registry missing from a document, a kind in a document
    missing from the registry, and any differing attribute each fail with the
    registry value named as the source. The test reads the committed draft
    (tracked since `a1a1e269`). A projected attribute the parser cannot
    extract fails the test rather than being skipped. **Domain, stated as a
    closed list** (reviewer-2 P2-3): included are the Kinds-table attributes
    and the two enumerations above, and the draft's Envelope table (l.113–121):
    its field names must equal the registry's envelope field set and its `to`
    forms the registry's address patterns. `contract_version` is a pending
    projection (reviewer-2 confirmation P3-1): absent from the draft's
    Envelope table → pass, with the pending R2 item named in the output;
    present → its row must equal the registry's. `availability` `ending` →
    `handoff` (`path`, `sha256`) (draft l.166–167) is **included**, as the
    registry's optional nested `data` paths (reviewer-2 confirmation P2-B;
    the round-4 exclusion is withdrawn); whether `ending` should *require*
    them stays a question the S1 result puts to the draft owner.
    Explicitly **excluded**, each with its reason, recorded in the test:
    `notice.action` "or `none`" (l.164), a sentinel
    inside free text that no check reads; `live_children` empty-versus-absent
    (l.168–172), a reading rule, not a validation attribute; the request-open
    rule (l.160), owned by S3's `requests` view and tested there; and the
    registry's `writer` attribute (D11), which the draft does not project and
    `docs/codex-chatboard.md` does, so it is checked for that document only.
  - `adopt` (D11 (c)): (i) `post --kind adoption` is refused with no board
    change (revision unchanged) and the message names `adopt`; (ii) the
    `adopt` row equals a fixture of the adoption envelope, including
    `contract_version`, topic `session:<own session id>` and
    `to: ["role:<role>"]` (P2-A, accepted); each non-adoption
    flag listed above, and `--topic`, is refused by the parser with no board
    change; (iii) `--handoff-path` naming a file whose sha256 differs from
    `--handoff-sha256` is rejected with no publish, a matching file is
    accepted, and a path alone records the file's digest; a non-hex or short
    digest, an uppercase, non-canonical or self-referencing
    `--predecessor-session` are rejected, including self-reference where the
    poster's `--session-id` is the uppercase form of the predecessor
    (reviewer-2 confirmation P3-2); replay of an identical `adopt`
    succeeds. These tests call the adapter directly (a shell-level test); they
    do not close X6 (S5);
  - `kinds` view derived from the registry: its output equals a rendering
    of every registry kind (one line each), `kinds K` prints exactly K's
    line, and run against a temporary registry with one kind added and one
    enum value changed the output changes accordingly (so no kind text is
    hard-coded); an unknown `K` is rejected naming the registered kinds; if
    `post --kind K --help` is built, its kind line equals `kinds K`'s;
  - rejection messages: for each registered kind, omitting each required
    field yields a message containing that field's path; each enum violation
    yields a message containing the field path and every allowed value; a
    `role`/`to` grammar violation names the field and the pattern; an
    `effects` shape violation names the offending key and the two allowed
    keys;
  - the common check: the 24 existing tests still pass unchanged, `skipped=0`.
- Acceptance: test output; the byte-compatibility fixture diff (empty); S0
  dispositions updated for every `affected` consumer; the
  `docs/codex-chatboard.md` diff. The draft's status line ("not implemented",
  l.3) becomes stale here; its update belongs to the draft owner (as in R2),
  flagged in the S1 result.

### S2 — read tooling: filters and compact projection

- Owner: this campaign. Depends on S1.
- Write scope: `codex_chatboard.py` (`read_view`, `wait_board`'s match
  predicate and result for D10, `parser` read flags); `tests/test_codex_chatboard_contracts.py`;
  `docs/codex-chatboard.md` (read flags, `--format compact`, the D5 reader
  rule, D10 delivery by `to` with its match reason, and the D10 invariant).
- Behaviour: `read --kind K --role R` (repeatable, filter before limit;
  `--kind note` also matches rows without `kind`, which read as `note`, draft
  l.110–111);
  `read --format compact` emitting one line per message: sequence, kind
  (`note` when absent), role, `to`, reply/supersedes link, author, topic,
  body truncated to a stated width. Default `read` output is unchanged.
  `wait` gets the same `--format compact` only if S0 shows a consumer that
  would use it; otherwise it is deferred.
  Delivery by `to` (D10 (a), decided 2026-10-25): `wait_board`'s predicate
  (l.387) also matches a row when any of its `to` addresses is in the selected
  topics. With no topic selected `wait` matches all rows, as at HEAD. Rows
  without `to` match by `topic` only. A row matching by both is returned
  once. The returned row is byte-unchanged; the **match reason** sits outside
  it, in a parallel result key `matches: [{"sequence", "matched_by"}]`, where
  `matched_by` is a list drawn from `topic` and `to` (both when both matched).
  `matches` is present only when at least one returned row matched by `to`,
  and then lists every returned row; otherwise the `wait` result is
  byte-identical to HEAD, since default `wait` output is an S0 class-3
  surface. A waiter may therefore receive a row whose `topic` is not one it
  selected; S0 class 3 records any consumer that relies on the topic filter
  as `affected`, and `matched_by` lets such a consumer (the gate's adoption
  wait, S5) keep a topic-only test.
  **Invariant (D10):** being addressed to a role does not establish that the
  recipient holds it; delivery is not authorization. `to` routes a wake and
  nothing else: it is not a holding report for `roles` (D9, S3) and not
  approval (Q1).
- Checks: filter-before-limit test with matches older than the newest
  `limit` rows; delivery by `to` (D10): a row on topic `aspect:x` with
  `to: [role:a, role:b]` wakes waiters on `role:a` and on `role:b`, one row
  each, and does not wake a waiter on `role:c`; a `session:` address behaves
  the same; the match reason per path: a topic-only match has no `matches`
  key, a `to`-only match reports `matched_by: ["to"]`, and in a result
  holding both kinds each row's `matched_by` is correct; a row on topic
  `role:a` with `to: [role:a]` is delivered once with `matched_by: ["topic",
  "to"]`; S2↔S5 integration fixture (user decision, 2026-10-25): a board
  holding a `kind: adoption` row whose `to` contains `session:<successor>`
  and whose `data.handoff_sha256` and `data.predecessor_session` match, but
  whose `topic` is not `session:<successor>`, is returned to a waiter on
  `session:<successor>` with `matched_by: ["to"]`; the fixture is committed
  with the tests and named in the reissued compatibility note (S5); filter-before-limit and the cursor hold for `to` matches; the 24
  existing tests pass unchanged (legacy rows have no `to`, so no `matches`
  key); compact output golden test including a legacy row and a
  stored unknown kind (D5 reader rule); the default `read` JSON is
  byte-identical to HEAD for the same board.
- Acceptance: tests; a sample compact read of the live board stored as a probe
  with its character count beside the raw read of the same range.

### S3 — `roles` and `requests` views

- Owner: this campaign. Depends on S2 (serialised: shared write scope).
- Write scope: `codex_chatboard.py` (two pure view functions, `parser`);
  `tests/test_codex_chatboard_contracts.py`; `docs/codex-chatboard.md` (the
  two subcommands).
- Behaviour (draft "Role addresses and succession", tooling 2–3):
  - `roles` (D9 (a), decided 2026-10-25): never labelled holder or owner. An
    `adoption` is *unended* until an `availability` with `state: ending` from
    the same session follows it. Per role the view emits:
    - `successions`: one entry per `adoption`, in sequence order:
      predecessor session, successor session, adoption sequence, the
      predecessor's `ending` sequence or `not reported`, and
      `committed: not established` on **every** entry, including a clean
      A→B whose predecessor has posted `ending`;
    - `relations`: each pair of unended adoptions (and each unended adoption
      whose predecessor has not reported `ending`) classed as `overlap` (the
      later names the earlier's session, or a session that has not reported
      `ending`, as `predecessor_session`), `fork` (two or more name the same
      predecessor), or `conflict` (linked by no predecessor chain and sharing
      no predecessor). **Chains traverse ended adoptions** (plan default,
      accepted by the user 2026-10-25; reviewer-2 confirmation P3-4): an ended
      adoption leaves the unended set but stays a link, so two unended
      adoptions whose predecessor chains meet at a common session, neither
      descending from the other, are a `fork` at that session (A→B with B
      ended, B→C and A→D unended: `fork` at A, the relation listing the path
      through B), and `conflict` is kept for chains that share no session;
    - `state`, one of: `no_adoption`; `ended` (adoptions exist, all ended);
      `single` (one unended adoption, its predecessor has reported `ending`);
      `overlap` (the unended adoptions form one chain, A→B→C included, with
      at least one predecessor `ending` not reported); `fork`; `conflict`. In
      a mixture the state is the first that applies of `conflict`, `fork`,
      `overlap`, and `relations` lists every pair, so no relation is hidden
      by the summary (A→B with B→C and B→D is `fork`, with the B→C/B→D fork
      and the A→B overlap both listed);
    - `reported_holder`: under `single` or `overlap`, the latest session of
      the chain with its adoption's sequence; under `fork` or `conflict`,
      `not established`, with the candidate sessions listed, never resolved;
      under `no_adoption` and `ended`, `not established` with
      `reporting_sessions` (sessions that posted `availability` or rows with
      that `role`), and for `ended` the last `ending` sequence.
    Rows merely addressed `to` the role are not holding reports (D10
    invariant). The output header states it is a projection of
    unauthenticated reports, not committed ownership or the gate's
    acceptance.
  - `requests`: requests by addressee; open until a `reply` with `answered` or
    `declined` names the request by `in_reply_to`; replies and receipts
    attached by `in_reply_to`. It relies on S1's closing-reply check (X3) for
    rows the S1 adapter wrote, and does not trust it for others: a closing
    reply whose target is not a `request` (written before S1 or by another
    writer) closes nothing and is listed as unattached.
- Checks: fixture boards for single reported holder; clean succession (A→B,
  A has posted `ending`: `single`, and its succession entry carries
  `committed: not established`); `overlap` (B's adoption names A, A has not
  posted `ending`: not a conflict); a three-adoption chain A→B→C, all
  unended (`overlap`, `reported_holder` C); `fork` (B and C both name A:
  neither `overlap` nor `conflict`, `reported_holder: not established`);
  mixed overlap and fork (A→B, B→C, B→D: `fork`, with every relation
  listed); `fork` through an ended adoption (A→B, B ended, then B→C and
  A→D unended: `fork` at A, P3-4); `conflict` (two unended adoptions whose
  chains share no session). Which case the gate's unbuilt generation guard
  targets ("two supervisors racing the same role", probe
  `metrics/probes/tmux-root-rotation-20261024.md` l.366–367) is `not
  established`; two supervisors rotating one predecessor would produce a
  `fork`, and the S1 result asks the gate owner (reviewer-2 confirmation
  P3-5); ending without successor
  and all adoptions ended (`ended`, `reported_holder: not established`, the
  last `ending` sequence); a role with `availability: available` and no
  `adoption`, whose output is `reported_holder: not established` with the
  session in `reporting_sessions` (D9, R7); a row addressed `to` the role
  from a non-adopting session does not appear as a reported holder (D10
  invariant); no output key named `holder` or
  `owner`; request open/answered/declined/deferred/partial;
  receipt does not close a request; a stored `answered` reply naming a
  non-`request` row closes nothing (X3).
- Acceptance: tests; the views are read-only (board revision unchanged before
  and after, as `test_topics_listing_counts_filters_and_pages_without_changing_board`
  checks for `topics`).

### M — recomputable baselines (D8)

- Owner: this campaign. Depends on nothing; must land **before S4**.
- Why the copy alone is insufficient (reviewer-1 P2-1): `extract_board_use.py`
  (`cc602321…`) writes one event per invocation with no read-output length and
  no truncation flag, and has no aggregation step; the aggregation that produced
  103/104, 301, 6,295 and 28/133 is not among the `evidence/` files (4 data
  files + 2 scripts). Neither script filters by time, and the listed transcripts
  are still growing: rerunning `reader_parsers.py` on 2026-10-24 over the same
  listings gave Codex 13/826 (matches) but Claude **88/179** against the
  baseline 87/178.
- Write scope (all new, under `metrics/scripts/` unless stated):
  1. `chatboard_board_use.py`: `extract_board_use.py` plus, per `read`/`wait`
     event, the tool-result output length in characters and a truncation flag
     (the host's truncation marker in that result), with the marker patterns
     stated in the script;
  2. `chatboard_reader_parsers.py`: `reader_parsers.py` (`f6ee651e…`) with the
     input freeze below;
  3. `chatboard_baselines.py`: aggregation over the event files emitting each
     baseline number with its definition — sessions that posted, of which ran
     `--help` before their first post (103/104); total help calls (301); median
     raw read characters and truncated count (6,295; 28/133); the draft's
     regex counts (88 no-effect posts, draft l.35; 24/48/69 succession counts,
     l.32–34) with the regexes the script uses, stated in it. The original
     regexes are not recorded, so these counts are *recomputed*; resolvable references
     (zero, draft l.243);
  4. `chatboard_contract_uptake.py`: board-only uptake (envelope presence,
     `effects` presence, resolvable references);
  5. input freeze: `metrics/probes/chatboard-baseline-inputs-<date>.json`,
     listing each input transcript with sha256, size and line count at freeze;
     every script reads only the first `line count` lines of each listed file
     and fails if the prefix digest differs. Full digests of the two copied
     scripts are verified against the assignment directory at copy time.
- Checks: each baseline is labelled **reproduced** (the original script and
  aggregation, over byte-identical inputs) or **recomputed** (a new script
  over the frozen inputs). Since the original inputs were not frozen, every
  baseline is expected to be *recomputed*; a recomputed value differing from
  the draft's figure is reported beside it, not adjusted.
- Acceptance: the recomputation output and the freeze file stored under
  `metrics/probes/`. Until M lands, the baselines marked `not established` in
  Measurement are not used as comparison points.

### S4 — instruction projection (exposure)

- Owner: this campaign. Depends on S3 and M.
- Write scope: `skills/coordination-channels/SKILL.md` only: one table of kinds,
  consumers and the post/read flags; role-address subscription; no posting
  procedure and no "when to post" rule (DESIGN §5; draft Scope "Out").
- Precondition (settled fact 7): search for an active experiment that owns this
  skill's exposure — `git grep -n 'coordination-channels'` under `experiments/`
  and `metrics/`, plus the experiment-control state the coordinator names. If
  one exists, the slice waits for its owner's release or a recorded decision.
- Checks: the skill's kind table equals the registry (same parity approach as
  S1); `git diff --stat` shows only the one file.
- Acceptance: the exposure check result recorded with its search domain; the
  diff.

### S5 — rotation-gate adoption

- Owner: **the tmux root-handoff session** (settled fact 5). This campaign
  supplies S1 and a compatibility note; it does not edit the gate.
- This campaign's write scope: the compatibility note
  `metrics/probes/chatboard-s1-gate-compatibility-<date>.json` (registry
  version and the `contract_version` field typed rows carry, adapter digest
  after S1, the `adoption` required fields, the `role` and `to` grammars, and
  the `adopt` command surface (D11 (c)), including its fixed
  `session:<own id>` topic, its automatic `to: ["role:<role>"]` (P2-A,
  accepted) and lowercase-canonical `predecessor_session`),
  written at S1 acceptance. D10's delivery rule lands in S2, not S1: the S1
  note marks it **pending S2**, and the note is reissued at S2 acceptance with
  the adapter digest after S2 and the `matches`/`matched_by` result key
  (reviewer-2 P3-4). The reissued note states that under D10 the gate's
  `session:$sid` wait can return rows addressed `to` that session on other
  topics, and that `matched_by` lets the gate keep its adoption test
  topic-only. **Integration condition S2↔S5 (user decision, 2026-10-25;
  tested):** because D10 lets `wait --topic session:X` return a row addressed
  `to` `session:X` on another topic, the gate must not rely on the S2 `wait`
  until its adoption recognition checks that the row's actual `topic` is
  `session:<successor>`, not only a match by recipient plus digest. The
  reissued note states this condition and points at S2's integration
  fixture; the gate's test on that fixture must show the row is **not**
  accepted. The gate test is the gate owner's to write and run; until it
  passes, the condition is unmet and the gate keeps the HEAD `wait`
  behaviour it was tested against. Their write scope:
  `metrics/scripts/root_rotation_coordinator.sh`,
  `metrics/scripts/launch_claude_successor.sh`, their tests.
- The compatibility note keeps three questions separate and answers only the
  first (X4):
  1. **schema validity** — whether a row passes S1's checks; the note states
     these;
  2. **host permission to run the board write** — whether the successor's
     host lets it execute the post at all (a tool-permission or sandbox
     matter, outside the schema and the adapter); the note records it as the
     gate owner's to establish, not as implied by validity;
  3. **authority to transfer responsibility** — user-open Q1; a schema-valid
     `adoption` establishes neither 2 nor 3, and the schema does not resolve
     Q1.
- The note also states (X6, D11 (c)) that only `adopt` writes a typed
  `adoption`; the gate still recognises the row by its fields, not by the
  command. It names `adopt` as the narrow command a host permission rule
  *could* allow for point 2. This campaign writes no permission rule and
  does not claim one exists.
- **X6 closure (open until a live successor run).** S1 supplies `adopt` and
  its shell-level tests; that is S1's part and does not close X6. X6 closes
  only when `adopt` succeeds through a real Claude successor launched by the
  gate's launcher, under the intended permission configuration; a shell-stub
  test does not close it. The permission grant that configuration needs
  requires the user's authorization (pending), and the host's actual
  rule-matching behaviour on the `adopt` command line must be tested in that
  run, not inferred from the rule's text. The live run must pass
  `--handoff-path` (so `adopt` checks the handoff file's bytes against the
  digest); an `adopt` given only `--handoff-sha256` does not count for X6
  closure (user decision, 2026-10-25). The run, and recording its result,
  belong to the gate owner (S5); this campaign records the criterion.
- Change, as the gate's own comment anticipates (l.239–245 at `3b2d00ec`;
  l.286–292 at `35060584`, same text): accept adoption
  when a message from the successor session has `kind: adoption`, `topic`
  equal to `session:<successor>` (S2↔S5 integration condition),
  `data.handoff_sha256` equal to the posted digest, and
  `data.predecessor_session` equal to `PRED_SESSION` (string equality;
  `adopt` stores only the lowercase canonical form); the successor's launcher
  emits that row with `adopt` (D11 (c)). The approval
  path is untouched (user question 1).
- **Typed adoption is authoritative for gate recognition** (user decision,
  2026-10-25). A prose-compatibility path is permitted only if necessary and
  only temporarily; typed adoption via `adopt` must be the exclusive accepted
  form before the mechanism is declared established. Whether a temporary
  prose path is needed is the gate owner's to establish; how long it lasts is
  bounded by that criterion.
- Open on their side, not decided here: the generation guard for two unended
  adoptions (S3's `roles` view reports `fork` and `conflict` and can serve as their
  check).
- Acceptance: theirs; this campaign's record is the compatibility note above
  (S1 issue and S2 reissue). X6 stays open until the live successor run.

## Review policy

User decision, 2026-10-24 (not a default):

| slice | review | reviewer receives |
|---|---|---|
| every slice (S0–S4, M) | tier 1: Opus, high, fresh | the slice's packet, diff, check output and the plan section |
| S1 | tier 1, then tier 2: Fable, fresh context, small input | the S1 diff, the draft's Envelope/Kinds/Deterministic-checks sections, tier-1 findings, with paths |
| workstream acceptance, before S4 exposure | tier 2: Fable, fresh | S1–S3 and M diffs (or their summary with paths), tier-1 findings, the S4 precondition result |
| any P1 the builder and tier 1 cannot resolve | tier 2: Fable, fresh | the P1, both positions, the cited evidence |

Fable is never given a retained follow-up (`references/anthropic-host.md`).
This plan received its fresh Fable review (round 2, below) before user
acceptance; no retained follow-up is sent. User decision, 2026-10-25: no
further Fable review of the plan; Fable reviews the S1 implementation diff
and the workstream acceptance boundary before S4, as the table states. Round 3 is an external review
pasted by the user, checked by the coordinator, not a tier under this policy.

## Rollout and exposure

1. S0, M: no behaviour change.
2. S1: new optional flags; no live session uses them until told. Ordinary posts
   byte-identical.
3. S2, S3: new read options and views; default outputs unchanged.
4. S5 may start once S1 lands; it changes only the gate's own sessions.
5. S4 last, after the exposure check and tier-2 acceptance: this is the point
   where live sessions start reading the contract.

Rollback: S1–S3 are additive; reverting the adapter leaves typed rows stored and
readable by the old adapter, because `decode` checks only top-level keys
(l.54–66) and old readers ignore unknown row keys (S0 confirms per consumer).

## Measurement

"Status" is whether the baseline can be recomputed by a committed script from
frozen inputs today; `not established` holds until M lands.

| baseline | status before M | expected mover | recomputed by (after M) |
|---|---|---|---|
| 103/104 Codex posters ran `--help` first (301 calls) | `not established` (no aggregation script) | S4 (flags in the skill table). S1 adds flags, which could raise lookups before S4 lands | `chatboard_baselines.py` |
| Claude piped 87/178 reads to one-off formatters; Codex 13/826 | Codex reproduced 2026-10-24; Claude `not established` (rerun gave 88/179 on grown inputs) | S2 compact projection and S3 views, exposed by S4 | `chatboard_reader_parsers.py` |
| Codex raw read median 6,295 chars, 28/133 truncated | `not established` (fields not extracted) | S2 compact projection, exposed by S4 | `chatboard_board_use.py` + `chatboard_baselines.py` |
| Mechanically resolvable references: 0 (draft l.243) | `not established` (no script) | S1 `in_reply_to`/`supersedes` | `chatboard_contract_uptake.py` |
| Misaddress corrections: message 2691 (n = 1, not exhaustive) | case, not a rate | role addresses (S4) and `roles` view (S3) | manual read; reported as cases |
| Prose effect restatements: 88 posts (draft l.35) | `not established` (regex not recorded) | S1 `effects` | uptake script: restatement rate **and** `effects` presence; falling restatement with absent `effects` is lost information |

All comparisons are observational over different windows with concurrent
changes; none supports a causal claim without a matched design. Expected
directions above are `unverified` predictions.

## Open decisions and risks

User-open (carried, not decided):

- Q1 approval authority, and if allowed, how the approver is authenticated.
  The board transports approval assertions but does not establish approval
  authority; plan acceptance does not wait on Q1 (user, 2026-10-25).

Resolved: Q6 (do not rewrite the "Why"), Q7 (draft committed in `a1a1e269`),
review policy, D2 ownership (ii) and row contract version (user decisions
2026-10-24); D9 (a) qualified, D10 (a) and D11 (c) (user decisions
2026-10-25); plan acceptance, D1, D2 location, D3–D8 and the planner
defaults (`adopt`'s automatic `to: ["role:<role>"]`, S1, P2-A; uppercase
predecessor rejected, S1; `conflict` > `fork` > `overlap`, S3; predecessor
chains traversing ended adoptions, S3, P3-4) (user decisions 2026-10-25,
round 6). Q2–Q5 are answered by D3–D6 as accepted.

Risks:

- **R1 — contract source untracked: closed** by `a1a1e269` (Q7). The gate
  comment and S1's parity test now point at a tracked file.
- **R2 — draft states D5 as decided.** Draft l.142–143 says unregistered kinds
  are rejected while its l.257 lists that as open. D5 was decided (a)
  (2026-10-25), so l.142–143 stand and the draft owner revises l.257; this
  plan does not edit the draft.
  The same owner revises l.76–78 (D2 (ii), decided 2026-10-24), the Envelope
  table l.113–121 (row `contract_version`), l.205–207 (`overlap`, `fork` and
  `conflict`, D9, decided 2026-10-25) and tooling item 2, l.230 ("current
  holder" becomes a reported holder, D9), l.193–194 ("Exact topic matching
  delivers it today with no code change") and l.232–233 (`wait` changes,
  D10 (a), decided 2026-10-25), l.213–218 ("`post` validates …
  `adoption.handoff_sha256`": under D11 (c) `post` no longer writes
  adoptions; reviewer-2 confirmation P3-3), l.220–222 (`adopt` checks the hash against
  the poster's file, D11, decided 2026-10-25) and tooling item 1, l.228–229
  (`adoption` is written by `adopt`, not `post`, D11 (c)), and the status
  line l.3 after S1.
- **R3 — incomplete consumer census.** Class 5 (packets, handoffs) and
  installed skill copies outside the repository may be partially scanned;
  compatibility for an unscanned class remains `not established`.
- **R4 — replay strictness.** Envelope fields enter idempotency comparison; a
  caller that retries with a regenerated `--data-file` (e.g. different key
  order is harmless under `sort_keys`, but a different value is not) gets the
  existing conflict error. A replay after a registry `version` change also
  differs (its `contract_version`) and gets the same error. Documented in
  `docs/codex-chatboard.md` (S1) and S4's table.
- **R5 — registry drift after a Rust port.** Mitigated by D2 (one
  authoritative file both implementations read) and by the docs → registry
  parity tests in S1 and S4.
- **R6 — exposure collision.** S4 changes what live sessions read; the
  settled-fact-7 check is a precondition, not a formality.
- **R7 — first holder not representable (draft tension, not edited here).**
  `adoption` requires `data.predecessor_session` and `data.handoff_sha256`
  (draft l.154), and the holder rule counts only `adoption` (draft l.204–206).
  A role's first holder has neither, so `roles` reports no holder until the
  first succession, which conflicts with D3's "first adopter names it". Folded
  with X1 into D9: S3 reports `reported_holder: not established` with
  `reporting_sessions`; the S1 result asks the draft owner whether a
  first-holder form is wanted (D9 (b)).

## Review disposition (round 1)

Findings: `reviewer-1-findings.md` sha256 `1832112c…c476` (0 P1, 5 P2, 5 P3).

- **P2-1** fixed — slice M (extraction fields, `chatboard_baselines.py`, input
  freeze, reproduced/recomputed labels); Measurement "status" column.
- **P2-2** fixed — S1 Checks, deterministic CAS-retry injection.
- **P2-3** fixed — `docs/codex-chatboard.md` added to S1, S2, S3 write scopes; R4.
- **P2-4** fixed — Review policy (user decision, not a default).
- **P2-5** fixed — D2 ownership options (i) default / (ii) alternative; R2.
- **P3-1** fixed — common check sets `CONTEXT_STATUS_TEST_BINARY`, requires
  `skipped=0`; `unverified` replaced with l.17–20.
- **P3-2** fixed — Code facts citations (l.381–388, l.387, l.438–439).
- **P3-3** fixed — Slices dependency graph; S2 → S3 serialised.
- **P3-4** fixed — S5 compatibility-note path; draft status line assigned to the
  draft owner (S1 acceptance, R2).
- **P3-5** fixed — S0 class 3 pattern adds `message`/`revision` alternatives.

## Review disposition (round 2, tier 2)

Findings: `fable-reviewer-1-findings.md` sha256 `94a0841e…12a3` (0 P1, 2 P2, 4 P3;
GO with fixes).

- **P2-A** fixed — S1 Checks restate the CAS test as (a) forward reference rejected at
  the first `change` with no publish or retry, (b) append injection gives a fresh
  `sequence` with `change` run twice, (c) a rewrite injection rejects on retry; Code
  facts record that `change` runs outside the `try`.
- **P2-B** fixed — S1 Behaviour adds the lazy registry rule and the missing-file error;
  S1 Checks run the byte fixture with the registry absent; S0 class 1 records
  `context_probe.py`; D2 notes the script-only copy.
- **P3-A** recorded, draft unchanged — R7; S3 fixture `holder: not established`; D3
  note; the question goes to the draft owner in the S1 result.
- **P3-B** fixed — `role` pattern in the S1 registry content with its basis (n = 1
  gate value); `effects` both-lists rule; `--kind note` matches absent `kind` (S2);
  `role`/`to` grammars in S5's compatibility note.
- **P3-C** fixed — M.3 says the script states its regexes and the counts are recomputed.
- **P3-D** fixed — `wake` citation l.130–133; revision binding updated to observed HEAD
  `9af4e514` (probe-only commits after `35060584`).

## Review disposition (round 3, external)

Input: `root-handoff-v2.md` sha256 `61f7d3ac…5a5f`, external review (pasted by
the user, coordinator-verified, X1–X5), the user decisions of 2026-10-24, and
X6 (`planner-4-packet-v1-addendum-1.md` sha256 `5d9a6419…705b`; source: the
rotation-gate session, via the user). Line numbers are of plan v5 (sha256
`75684397…8a71`), not of the current revision; status
and binding lines 5–6, 15–20 and 47–49 record the revision itself.

- **X1** fixed, folded with R7 as one decision — D9 (l.110, new); D3 (l.104);
  S3 `roles` Behaviour l.372–381 and Checks l.388–397 (`reported_holder`,
  `overlap` vs `conflict`, no `holder`/`owner` key); R7 l.585–592; R2 l.563–570.
- **X2** fixed — the draft does not bind `to` to `topic` (draft l.124–126:
  `topic` is the delivery channel, `to` names who must act, a topic cannot
  express several addressees; l.192–193: a holder subscribes to `role:<name>`
  and its `session:` topic). D10 (l.111, new); Code facts l.88–91; S2 write
  scope l.336–339, Behaviour l.348–353, Checks l.354–359; R2 l.563–570 (draft
  l.232–233).
- **X3** fixed — S1 cross-message check l.244–249; S1 Checks l.279–284; S3
  `requests` l.382–387 and Checks l.388–397.
- **X4** fixed — S5 compatibility note l.464–481 separates schema validity,
  host permission and transfer authority; Q1 not resolved. The review's
  premise of a failed live rotation is `not established` from this campaign's
  records and is not stated.
- **X5** superseded by D2 (ii) — S1 docs → registry parity l.306–317; R5
  l.580–582.
- **X6** (rotation-gate session via the user) addressed as a decision, not
  decided — D11 (l.112, new, default (b)); S1 heading l.192, Depends l.194,
  write scope l.201, `adopt` Behaviour l.250–266, Checks l.318–325; S5 note
  l.467–468 and l.482–486 (command-independent recognition, no permission
  rule claimed), Change l.492–493; R2 l.569–570 (draft l.220–222); Open
  decisions l.556.
- **User decision D2 → (ii)** — Decisions header l.98; D2 row l.103;
  "Decided by the user" l.125–131; S1 Depends l.194–199 (draft-owner
  dependency on draft l.76–78); R2 l.563–570; Open decisions l.554–556.
- **User decision: row contract version** — "Decided by the user" l.125–131;
  S1 Registry content l.208–209, Behaviour l.219–226, Checks l.285–288; S5
  note l.465–466; R4 l.574–579; R2 l.563–570 (draft Envelope table).
- Review policy l.512–514 records that round 3 is not a tier under it.

## Review disposition (round 4: user decisions D9–D11 and tier-1 reviewer-2)

Input: user decisions D9–D11 of 2026-10-25 (relayed by the coordinator in
`planner-5-packet-v1.md`) and `reviewer-2-findings.md` sha256 `56496b86…4dfa`
(0 P1, 3 P2, 8 P3; GO with fixes), against plan v5 sha256 `75684397…8a71`.
Line numbers are of plan v6 (sha256 `1d437f97…e855`), not of the current
revision (the D10 and D11 decided-list ranges corrected in round 5,
reviewer-2 confirmation P3-6). Status and binding lines 5–7, 15–25 and
55–57 record the revision itself. No permission rule is written; Q1 and the
acceptance of D1, D2 location and D3–D8 stay open.

| item | disposition | changed lines |
|---|---|---|
| D9 → (a) qualified | decided; reported holder only; `overlap`, separate `fork`, `conflict`; `committed: not established` on every succession entry | header l.105–107; D9 row l.119; decided list l.141–144; S3 `roles` l.455–486, Checks l.493–507; R2 l.695–706; Open decisions l.685–688; S5 open item l.626 |
| D10 → (a) | decided; match reason outside the row (`matches`/`matched_by`, present only on a `to` match), single delivery, invariant "delivery is not authorization" | Code facts l.98–99; D10 row l.120; decided list l.146–148; S2 write scope l.402–405, Behaviour l.414–431, Checks l.432–444; S3 l.483–484 and Checks l.505–507 (cross-reference); S5 note l.583–588; R2; Open decisions |
| D11 → (c) | decided; `post` refuses `adoption` by registry `writer`; `adopt` delegates to `mutate()`, no special trust; byte-equality checks removed; X6 open until a live successor run, grant user-pending | D11 row l.121; decided list l.149–151; S1 registry l.242–244, `post` refusal l.266–268, `adopt` l.283–312, Checks l.380–391; S5 note l.602–606, X6 closure l.607–615 (S1's part vs S5's part), Change l.616–623, Acceptance l.628–629; R2 |
| P2-1 | fixed: S0 class-3 semantic criterion; gate wait named | S0 l.200–206, Acceptance l.213–216; S2 l.425–427 |
| P2-2 | validation-set half moot (only `adopt` writes adoptions; rules in `adopt` only); storage half fixed: lowercase canonical required, stored as given | S1 `adopt` l.301–307; Checks l.388–389; S5 Change l.620–621 |
| P2-3 | fixed: parity domain a closed list; four attributes and `writer` excluded with reasons; Envelope table included, `contract_version` pending (R2) | S1 Checks l.363–379 |
| P3-1 | fixed: draft l.193–194 added (plus l.228–229 and l.230 under D11/D9) | D10 row l.120; R2 l.695–706 |
| P3-2 | fixed: "D2 ownership"; "D1, D2 location, D3–D8" | l.105–107; l.685–688 |
| P3-3 | fixed: grep basis for the `to` absence claim; adapter-only basis for "no stored `to`", other writers `not established` | D10 row l.120 |
| P3-4 | fixed: D10 marked pending S2; note reissued at S2 acceptance | S5 l.582–588, l.628–629 |
| P3-5 | fixed: chains of three or more, mixtures, all-ended and ending-without-successor outputs defined and fixtured | S3 l.463–482, Checks l.493–504 |
| P3-6 | fixed: `adopt` has no `--topic`, writes `session:<own id>`; `body` left free with reason | S1 l.292–297; Checks l.382–384 |
| P3-7 | fixed: addendum and v4 `97bf2d3d` named | l.20–22 |
| P3-8 | moot under D11 (c): the rationale was removed with the (b) default | D11 row l.121 |
| "Against user decisions" | applied as listed in the D9, D10 and D11 rows above; round-3 disposition (l.769–776) now states that its line numbers refer to v5 | l.774–775 |

## Review disposition (round 5: reviewer-2 confirmation)

Input: `reviewer-2-confirmation.md` sha256 `169eddfc…c2fe` (GO with fixes;
0 P1, 2 P2, 6 P3), against plan v6 sha256 `1d437f97…e855`, with
`planner-6-packet-v1.md`. Line numbers are of this revision. Status and
binding lines 5–8, 26–29 and 60–61 record the revision itself. No permission
rule is written; Q1 and the acceptance of D1, D2 location and D3–D8 stay
open; D9–D11 are not reopened.

| item | disposition | changed lines |
|---|---|---|
| P2-A | fixed as the reviewer's option (i), a **plan default** (accepted by the user in round 6): `adopt` keeps the fixed `session:<own id>` topic and adds `to: ["role:<role>"]` from `--role`, so D10 wakes role peers (draft l.154 "peers re-addressing"); wake pending S2; option (ii) recorded as the alternative | S1 `adopt` l.308–318; Checks l.411–412; S5 note l.624–625; Open decisions l.732–734 |
| P2-B | fixed: `handoff.path` and `handoff.sha256` are optional nested `data` paths in the registry and inside the parity domain; the round-4 exclusion is withdrawn; "should `ending` require them" stays with the draft owner | S1 Registry content l.248–251; Checks l.396–400 |
| P3-1 | fixed: the registry carries the envelope field set (draft l.113–121 plus `contract_version`); pending rule restated as absent → pass with the pending item named, present → must equal | S1 Registry content l.241–243; Checks l.393–396 |
| P3-2 | fixed: "cannot reach non-`session:` topics", with the caller-supplied `--session-id` limit stated; not-self comparison case-insensitive and tested | S1 `adopt` l.304–308, l.328–331; Checks l.418–420 |
| P3-3 | fixed: draft l.213–218 added to R2 | R2 l.749–751 |
| P3-4 | fixed as a **plan default** (accepted by the user in round 6): chains traverse ended adoptions; branching chains meeting at a common session are a `fork` there; `conflict` only for chains sharing no session; fixture added | S3 `relations` l.499–505; Checks l.537–538; Open decisions l.734 |
| P3-5 | fixed: generation-guard label dropped from `conflict`; which case the guard targets is `not established`; a two-supervisors race on one predecessor is a `fork`, and the S1 result asks the gate owner | S3 Checks l.538–544 |
| P3-6 | fixed: round-4 decided-list ranges now l.146–148 (D10) and l.149–151 (D11), stated as v6 line numbers | round-4 header l.862–864; table l.871–872 |

## Review disposition (round 6: user acceptance decisions)

Input: the user's acceptance decisions of 2026-10-25, relayed by the
coordinator in `planner-7-packet-v1.md`, against plan v7 sha256
`37e987f7…3a2`. Targeted edits only; no restructuring. Line numbers are of
this revision. Revision lines 33–36 and 68 record the revision itself. No
permission rule is written; no Fable review of the plan is sent.

| item | disposition | changed lines |
|---|---|---|
| 1. plan accepted | recorded as the user's acceptance, 2026-10-25; authority paragraph restated as accepted | Status l.8–13; authority l.45–48; decided list l.167–171 |
| 2. D1–D8 defaults | decided as proposed; D2 location decided; Q2–Q5 answered by D3–D6 | header l.117–119, l.121; D1–D8 rows l.123–130; decided list l.172–173; Open decisions l.821–825; R2 (D5) l.832–834 |
| 3. planner defaults | accepted: automatic `to: role:<role>`, uppercase predecessor rejected, `conflict` > `fork` > `overlap`, forks traverse ended adoptions; "awaiting acceptance" removed (round-5 rows annotated) | decided list l.174–177; S1 l.344–345, l.353–354, Checks l.462; S3 l.567; S5 l.691–692; Open decisions l.821–825; round-5 table l.988, l.993 |
| 4. Q1 unresolved | stated: the board transports approval assertions, does not establish approval authority; acceptance does not wait on Q1 | authority l.47–48; user-open l.138–140; decided list l.178; Open decisions l.815–816 |
| 5. no plan Fable review | recorded; Fable reviews the S1 diff and the workstream boundary before S4 | Review policy l.773–775; decided list l.179–180 |
| 6. registry format | one sentence: plan's format, not JSON Schema, a one-to-one-mappable subset | S1 l.288–290; decided list l.181–182 |
| 7. S1 additions | `kinds` view (and `post --kind K --help` where the parser allows) derived from the registry; rejection messages name field and allowed values; a check for each | S1 write scope l.267; Behaviour l.376–389; Checks l.473–484; decided list l.183–184 |
| 8. S2↔S5 integration | tested condition: gate checks the row's actual `topic`; S2 fixture (matching `to` and digest on another topic) committed; the gate's test must reject it before relying on D10 `wait` | S2 Checks l.533–538; S5 note l.699–708; S5 Change l.748–750; decided list l.185–187 |
| 9. typed adoption authoritative | prose path only if necessary and temporary; `adopt` exclusive before "established"; X6 live run must pass `--handoff-path`, digest-only does not count | S5 X6 closure l.734–737; S5 l.751–754 (typed authoritative, replaces the prose-transition open item); decided list l.188–189 |
