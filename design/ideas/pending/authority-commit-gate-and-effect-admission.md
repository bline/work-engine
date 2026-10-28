# Authority Commit Gate and Effect Admission

> **Question:** When authority to perform an effect can be contracted or revoked at any moment by owners in several independent domains, how does a host prove that a specific effect was still authorized at the instant it crossed its mutation boundary — rather than at some earlier moment of checking?

## Status and authority

Exploratory mechanism design, directly captured from operator discussion on
2026-10-01.

This document does not accept a design, amend canonical architecture, change
the roadmap, authorize implementation, authorize implementation planning,
establish that a new mechanism exists, or create any authority. It is design
work only. Two hard prerequisites (§12) block implementation planning, and
both belong to canonical owners who have not decided them. No canonical
architecture page was edited to produce this document, and none should be
edited on its basis before those prerequisites are settled.

```yaml
idea_status:
  architectural_supersession: none
  architectural_supersession_note: "Checked 2026-10-01 against the plausible canonical owners by reading their actual claims, not by term absence: mechanisms/resource-lease-and-fencing.md (owns single-resource, TTL-expiring lease+fence; §4.1 compares invariant shape and finds it different), mechanisms/transition-fencing-and-leases.md (owns a one-shot preparation interval; that page's own text already declines a third fence class), mechanisms/revision-cas-and-publication.md (owns succession safety; explicitly does not decide when supersession occurs, which is why §12.1 is open), authority-and-ownership.md (owns authority grant/ceiling, not cross-domain commit serialization), organizational-compilation.md (owns the admission that produces ExecutionEnvelope, not its enforcement at effect boundaries), role-and-contract-structure.md and runtime-realization.md (projection and capability respectively, neither an authority source at commit time). No canonical view states a multi-key, non-expiring serialization claim over a derived set of semantic invalidation domains. Corroborated mechanically: grep of app-server/src finds no ExecutionEnvelope store, and no use of update-ref --stdin or the reference-transaction hook."
  residue: present
  residue_ledger: "§12.1 and §12.2 (both blocking, both owned by canonical owners), §17 item 10, and the four OPEN fourth-review findings in §1.1 — see inline [KIND: RESIDUE] tags"
  backlog: present
  backlog_ledger: "§17 items 1-8 and 11 — see inline [KIND: BACKLOG] tags; items 3, 4, 8 are UNCHECKED"
  audit_scope:
    - open-question-ledger
  audit_scope_completeness: partial
  audit_scope_completeness_note: "§17 items 3 (lock-ordering inversion across mechanisms), 4 (reference-transaction hook coverage across every Git invocation path) and 8 (RoleRealization generation read path) are UNCHECKED — named but not examined. All other ledger items carry a checked disposition. The four fourth-review findings in §1.1 are OPEN: this revision proposes a restructure addressing them and does not claim resolution."
  status_as_of: 2026-10-02
```

```yaml
idea_provenance:
  origin: direct_capture
  captured_on: 2026-10-01
```

---

## 1. Provenance and review history

This document **supersedes**
[execution-envelope-live-effect-admission.md](execution-envelope-live-effect-admission.md)
(1953 lines, SHA-256 `b165f52b0f06fb1b71ff8434562906627a501b3ea3b52c88bfc285edfdb917bf`).
That file is left in place unmodified; disposition of it is a separate
decision.

The superseded document is the terminal state of eight architecture
reconciliation rounds (originally opening from a "Branch Plan ↔ Proposal
mapping" seam) followed by three adversarial review passes, each returning
**BLOCKING**. This document has since taken a fourth pass, also **BLOCKING**
(§1.1).

**Review independence, stated once and used verbatim wherever referenced:**
All review passes to date were retained same-model reviews. No pass claimed
independence (`independence_claimed: false`).

The reconciliation rounds are not restated here; their surviving conclusions
are the premises of §3's boundary. The review history matters only for what it
**withdrew**, which this document does not reintroduce:

| Withdrawn | Why |
|---|---|
| `.fenced` effect path | A guarantee contingent on an adapter choosing to call its own confirmation step is not a guarantee. Even a fully compliant adapter retains a confirmation-to-commit window. |
| `.sameStoreTransactional` as an admission path | Required ExecutionEnvelope, attempt admission, role realization, and the mutation to share one database transaction. Four different owners' artifacts; at least two separate databases (§4.2). Unreachable as specified. |
| "the admission record is recovered" | False as a description of existing code. `git-publisher.mjs` reports `published_unconfirmed` when the ref advanced but no admission row exists; it does not reconstruct the admission (§8.3). |
| Inferring "did not commit" from the target ref's current value | ABA-unsafe. parent → commit → parent is indistinguishable from parent-unchanged (§8.3). |
| Any claim that a zombie executor necessarily loses its CAS | If neither ref has advanced, a stale executor can still succeed. Addressed by an epoch, not by assumption (§9.2). |

Two findings from those passes remain **open and blocking**, carried forward
unchanged as §12.

### 1.1 Fourth review pass — findings OPEN, restructure proposed

`[KIND: RESIDUE] [OPEN]`

Reviewed subject: this document at SHA-256
`1fc29739bc11c6589e70962ca45f3cc2b61e1c7042a1ae7ba2d3fd70f0fdfb1e`
(682 lines). Verdict: **BLOCKING**. Four findings, all accepted as correct.

**None of the four is closed by this revision.** What follows is a *proposed
restructure* responding to them, to be judged by the next review pass. The
sections it touches are marked accordingly. Treating a proposed restructure as
a resolution is the specific error this document is trying not to repeat.

| # | Finding | Status | Restructure proposed |
|---|---|---|---|
| 1 | Ledger `create` on a stable ref succeeds only once; the second operation in the same scope fails | **OPEN** | Genesis-only `create`, then `update <ledger-ref> <new-tip> <expected-old-tip>` (§8.2; both forms verified) |
| 2 | The empirically tested Git failure was *validation-phase rejection*, not the dangerous *commit-phase* partial failure. Git's files backend applies a multi-ref transaction such that a concurrent reader may observe a subset; a ledger-advanced/target-not state would be misread as committed | **OPEN** | Move authoritative evidence into the target commit as trailers, reducing the transaction to one ref write (§8.2) |
| 3 | "Receipt absent ⇒ retry safe" is conditional on the old target transaction no longer being able to commit | **OPEN** | Negative inference requires the target transaction to be **terminal** (§9.1) |
| 4 | Hash-linking does not detect deletion of the only ledger tip, or rewind to a valid ancestor | **OPEN** | Ledger demoted to non-authoritative audit trail; audit claim narrowed (§9.4) |

Two factual corrections from the same pass, both applied:

- The review-independence characterization is now stated once, verbatim, above. This error regressed across three passes because each revision re-derived the wording from context instead of from the review metadata.
- §4.1's claim that Resource Lease and Fencing does **not** span a mutation boundary was **false** and is corrected: `withAdmission` runs `mutate()` inside its own `BEGIN IMMEDIATE`/`COMMIT`, so it does span it. That axis does not distinguish the two mechanisms, and §4.1 no longer argues from it.

---

## 2. The invariant

Stated once, unqualified. Every effect path below either mechanically enforces
it or is explicitly excluded from this mechanism. No path is "supported with
caveats."

> **No effect may commit if any authority or validity generation it depends on
> ceased to be authoritative before that effect crossed its mutation
> boundary.**

The previous document's central failure was preserving this sentence while
admitting an effect class that could not enforce it. Narrowing the supported
surface is the correction.

---

## 3. Purpose, and what this mechanism does not own

This is a **mechanism** candidate — a reusable serialization-and-commit
protocol — not a dimension. It owns no class of semantic truth.

**Owned:** cross-process serialization of semantic invalidation domains; the
derived gate scope; the operation state machine; the two commit primitives; the
Git operation epoch, commit-trailer evidence format, and the non-authoritative
effect ledger; recovery/reconciliation outcomes; the failure vocabulary;
durable receipts.

**Not owned,** and never decided here:

- what organizational authority *is*, or who may grant it — `authority-and-ownership.md`;
- when an ExecutionEnvelope should be superseded — the owning dimension's judgment, consistent with `mechanisms/revision-cas-and-publication.md`, which states it "does not decide when a revision should be superseded — that is a domain judgment";
- what a role contract requires — `role-and-contract-structure.md`;
- what a runtime realization is or when it is stale — `runtime-realization.md`;
- what a physical resource means or who may hold it — `mechanisms/resource-lease-and-fencing.md`;
- what a slice, attempt, or campaign means — the slice/campaign owner, which has no canonical architecture view (§15).

The gate serializes. Canonical owners retain semantic authority, without
exception.

---

## 4. Why a new sibling mechanism, not a new consumer of an existing one

`mechanisms/resource-lease-and-fencing.md` sets the precedent that this
question must be answered by **invariant shape**, not vocabulary overlap: it
resolved fenced active-binding as "a new resource type on this existing
mechanism, not a new mechanism," and explicitly declined to add "a third fence
class" to `mechanisms/transition-fencing-and-leases.md`. The same test is
applied here, and reaches the opposite answer.

### 4.1 Shape comparison

| | Resource Lease and Fencing | Transition Fencing and Leases | **AuthorityCommitGate** |
|---|---|---|---|
| Protected thing | one named physical resource | one preparation interval | a derived **set** of semantic invalidation domains |
| Keys per hold | single (`request.resource.key`) | single transition | **many, atomically acquired** |
| Expiry | TTL; standing holder relationship with renewal | one-shot interval | **none; does not expire, not stealable** |
| On holder death | lease expires, successor takes over | interval abandoned | **fails closed; no automatic steal** |

**One axis explicitly conceded as non-distinguishing.** An earlier revision of
this table claimed Resource Lease and Fencing does not span a mutation
boundary. That was **false**: `withAdmission` calls `mutate()` *inside* its own
transaction —

```js
#transaction(operation) {
  this.database.exec("BEGIN IMMEDIATE");
  try { const result = operation(); this.database.exec("COMMIT"); return result; }
  catch (error) { try { this.database.exec("ROLLBACK"); } catch {} throw error; }
}
```
(`workspace-coordination/sqlite-store.mjs:29-34`, with `const result = mutate()`
at line 68 inside it)

— so it genuinely spans the mutation boundary for a single-store mutation. The
mutation-boundary axis therefore does **not** support a new mechanism, and the
argument below does not use it.

The two axes that do remain, both verified against the implementation rather
than inferred from the page:

1. **Single-key vs. multi-key scope.** `acquire` operates on exactly one `request.resource.key`, with `fencingToken = current.generation + 1` (`sqlite-store.mjs:39-51`). The gate requires a canonically ordered set acquired atomically (§5).
2. **Expiring vs. non-expiring holds.** The existing lease carries an `expiresAt` and `withAdmission` **rejects an expired lease outright** — `if (Date.parse(current.lease.expiresAt) <= Date.parse(admittedAt)) throw new Error("resource lease is expired")` (`sqlite-store.mjs:67`). A hold that can expire while an external commit is in flight reintroduces precisely the window §2 forbids.

Whether those two axes are sufficient to justify a sibling mechanism rather
than an extension of the existing one remains the mechanism corpus owners'
judgment (§17 item 10), and this revision does not strengthen that claim.

So: a new sibling mechanism. Resource Lease and Fencing is unchanged and
continues to protect physical resources, contributing
`resource_fence_generation` to the validation vector (§6). The gate **may
reuse that store's transactional implementation pattern** — a private
`BEGIN IMMEDIATE` / `COMMIT` / `ROLLBACK` wrapper, as both existing stores use
(`workspace-coordination/sqlite-store.mjs:29-34`;
`slice-campaign/sqlite-store.mjs:8-13`) — but not its lease semantics.

### 4.2 The fact that makes a shared gate necessary

Verified: the two relevant stores are **separate SQLite databases**.

- `app-server/src/services/slice-campaign/capability-host-runtime.mjs:37` — `filePath: path.join(path.resolve(stateRoot), "slice-campaign.sqlite3")`
- `app-server/src/services/workspace-coordination/runtime.mjs:37` — `filePath: storePath ?? path.join(root, "coordination.sqlite")`

Two files, two `DatabaseSync` connections, each with its own `BEGIN IMMEDIATE`.
Neither serializes the other. ExecutionEnvelope compounds this: it has **no
store anywhere** — grep for `ExecutionEnvelope` across `app-server/src` returns
nothing.

Therefore: wrapping several reads in one SQLite transaction cannot serialize
these domains, and "same host process" is insufficient unless every writer is
provably routed through that process. A durable, cross-process gate is the only
shape that serializes them.

---

## 5. Gate scope

Scope is a **host-derived, canonically ordered, atomically acquired set** of
invalidation-domain keys. Callers cannot supply it — a caller-supplied scope
would let a caller narrow its own validation, which is the authority-claim-by-
presence failure this architecture rejects everywhere.

```text
execution-envelope/<envelope_identity>
attempt-admission/<workspace_identity>
role-realization/<realization_identity>
resource/<resource_type>/<resource_identity>
parent-authority/<authority_source_ref>   # blocked, pending §12.1
policy/<policy_ref>                       # blocked, pending §12.1
```

Rules:

1. Scope is derived host-side from canonical identities, never from caller input.
2. An effect acquires the **union** of every key affecting its validity.
3. A writer advancing generation *g* must acquire **the same key** that effects depending on *g* acquire. Non-intersecting keys are a correctness bug, not a performance choice.
4. Multi-key acquisition is atomic and uses a canonical total order (lexicographic over the key strings above), so concurrent acquisitions cannot deadlock.
5. A process-local mutex is insufficient unless every writer is provably routed through that process (§4.2).
6. The gate owns serialization only.

### 5.1 The attempt-admission grain, verified

The key is the **workspace admission domain**, not the attempt identity. This
is verified against the store, because the grain choice is a correctness
matter rather than a naming preference.

`slice_campaign_admission` is keyed by workspace:

```sql
CREATE TABLE IF NOT EXISTS slice_campaign_admission (
  workspace TEXT PRIMARY KEY, identity_key TEXT NOT NULL UNIQUE,
  FOREIGN KEY(identity_key) REFERENCES slice_campaign_state(identity_key)
) STRICT
```
(`slice-campaign/sqlite-store.mjs:148-151`)

and `supersede()` **transfers that row's holder** rather than replacing the row:

```sql
UPDATE slice_campaign_admission SET identity_key = ? WHERE workspace = ? AND identity_key = ?
```
(`slice-campaign/sqlite-store.mjs:65-67`, requiring `holder?.identity_key === key` first at lines 52-57)

The workspace row persists across supersession; only its holder changes.
Consequently:

- **Gate key:** `attempt-admission/<workspace_identity>` — the domain whose holder can change.
- **Generation value in the vector:** the exact current attempt identity, `runId:sliceNumber:attemptId:planVersion` (the real `identityKey` shape used by `slice-campaign/contract.mjs`).

Keying the gate by the *old attempt identity* would be wrong: the successor's
effects would acquire a different key, so the transferring writer and the
superseded attempt's in-flight effects would never intersect, and the transfer
would not be serialized against them. Keying by workspace makes the old
attempt's effects, the successor's effects, and the transferring writer all
contend on one key.

---

## 6. The validation vector

```text
EffectGenerationVector:
  ev_revision                   # ExecutionEnvelope current head
  attempt_admission_value       # current holder of the workspace admission domain
  role_realization_revision     # RoleRealization current/applicable
  resource_fence_generation?    # present only when a fenced resource is touched
  parent_authority_generation?  # BLOCKED, pending §12.1
  policy_generation?            # BLOCKED, pending §12.1
```

Each component is read **from its canonical owner**, never from a cache and
never from the caller. The gate rejects unless every observed component equals
the expected component.

The two blocked components are not optional-by-design; they are absent because
their invalidation semantics are undecided (§12.1). A vector missing a
component that *should* be in it does not fail safely — it fails silently.
This is why §12.1 blocks implementation planning rather than merely being
listed as future work.

---

## 7. The gate mechanism

### 7.1 Operation state machine

Persisted in the gate store:

```text
prepared → committing → committed
                      → aborted
                      → reconciliation_required
```

### 7.2 Hold semantics

- A hold spanning a mutation boundary **does not expire** and is **not automatically stealable**. There is no TTL and no steal path. This is the deliberate inversion of Resource Lease and Fencing (§4.1).
- **Heartbeats are observability only.** A missed heartbeat is a signal to a human or an audit, never an authorization to seize a hold.
- If a holder dies in `committing`, every overlapping effect **and every authority writer** contending on any of its keys fails closed with `gate_reconciliation_required`, until that operation's commit primitive is reconciled (§9).

The cost of this choice is explicit and accepted: a dead holder in
`committing` blocks its invalidation domains until reconciled. That is a
liveness cost paid to keep the invariant unqualified. A self-healing timeout
here would be exactly the expiring-lease semantics §4.1 rejects.

### 7.3 Sequence

```text
1. Host derives gate scope from canonical identities; callers cannot supply it.
2. Gate atomically acquires every scope key in canonical order.
3. Gate re-reads the complete generation vector from canonical owners.
4. Gate rejects unless every observed generation is current.
5. Selected commit primitive crosses the mutation boundary.
6. Commit primitive produces durable, operation-bound evidence.
7. Host records the admission outcome.
8. Gate releases the scope keys.
```

Step 4 before step 5, with the keys held across both, is the whole mechanism.

---

## 8. The two commit primitives

Two primitives, **one admission path**. The distinction is how the mutation
boundary is crossed, not a second set of validation rules.

### 8.1 `localSqlCommit`

For effects whose entire mutation is a write to one ACID store.

- The authority generations **need not live in that store** — the gate holds them stable across the commit. This is what makes the path reachable, and is the specific correction to the withdrawn `.sameStoreTransactional` (§1).
- The target store's transaction writes the mutation **and** an operation receipt keyed by a unique `operationId`, together.
- Recovery queries **that target-store receipt** (§9.1).

Existing precedent for the receipt-in-the-same-transaction shape:
`withAdmission` rejects a duplicate `operationId` before mutating
(`workspace-coordination/sqlite-store.mjs:59-61`), runs `mutate()` inside the
transaction (line 68), and records the admission row with the mutation's result
in that same transaction (lines 70-72).

One verified constraint on any reuse of that function: it **rejects
asynchronous callbacks** — `if (result && typeof result.then === "function")
throw new TypeError("mutation admission callback returned an asynchronous
result")` (line 69). A commit primitive composed inside it must be synchronous.
Git invocations already are (`execFileSync` / `spawnSync`,
`git-publisher.mjs:6-7`), so §8.2 is compatible; a primitive that is not
synchronous cannot use this shape.

### 8.2 `gitRefCommit`

For effects whose mutation is a Git ref advance.

Git's branch-parent CAS is **resource contention protection, not authority
fencing** — it proves the parent did not move, and says nothing about whether
authority was still current. Authority currency is the gate's job (§7.3 step 4,
keys held); the Git transaction additionally verifies an **operation epoch** so
that a revoked operation cannot commit even if it reaches Git.

#### Restructure responding to §1.1 findings 1 and 2 — OPEN, not closed

An earlier revision put authoritative operation evidence in a **second ref**
(an append-only ledger) advanced in the same transaction as the target ref.
Finding 2 is that this is unsafe on Git's files backend, and finding 1 is that
the `create` form breaks on the second operation in a scope. Both are correct.

The restructure: **authoritative operation evidence moves into the target
commit itself, as trailers**, reducing the Git transaction to a single ref
write.

```text
verify refs/work-engine/effect-epoch/<operationId>  <expectedEpoch>
update refs/heads/<target>                          <newCommit> <expectedParent>
```

where `<newCommit>`'s own message carries:

```text
Work-Engine-Operation-Id: <operationId>
Work-Engine-Generation-Vector: <generation_vector_digest>
```

The trailers are part of the commit object, so they are content-addressed and
immutable: a commit carrying `operationId` cannot exist without that operation
having produced it. `verify` writes nothing. So the transaction performs
**exactly one ref write**, and there is no inter-ref state for a partial commit
phase to expose.

**What the documentation establishes, quoted exactly** (`git-update-ref(1)`):

> If all `<ref>`s can be locked with matching `<old-oid>`s simultaneously, all
> modifications are performed. Otherwise, no modifications are performed. Note
> that while each individual `<ref>` is updated or deleted atomically, a
> concurrent reader may still see a subset of the modifications.

That sentence is load-bearing in both directions. "Each individual `<ref>` is
updated or deleted atomically" is what makes a single-ref transaction safe.
"A concurrent reader may still see a subset of the modifications" is a direct
documentary confirmation of finding 2: a multi-ref transaction is **not**
observably atomic on this backend, so the previous two-ref ledger design could
not support the inference it was carrying.

**Verified empirically** (git 2.53.0, `files` ref backend confirmed via
`git rev-parse --show-ref-format`), with the covered failure mode stated for
each — this is the discipline the prior pass failed:

| Claim | Result | Failure mode covered |
|---|---|---|
| `create <ref>` succeeds once, then fails | Confirmed — second `create` gives `fatal: cannot lock ref '...': reference already exists` | **Finding 1 directly.** Steady-state sequence, not a race |
| `update <ref> <new> <old-tip>` succeeds with the correct tip and rejects a wrong one | Confirmed both ways (`...is at <x> but expected <y>`) | The corrected ledger-append form |
| `verify <ref> <old-oid>` exists and changes nothing | Confirmed; documented as "Verify `<ref>` against `<old-oid>` but do not change it" | Establishes the epoch check adds no second write |
| Trailers on the new commit are readable from the target ref after commit | Confirmed (`git log -1 --format=%B` shows `Work-Engine-Operation-Id`) | Positive-evidence readback |
| An advanced epoch rejects the whole transaction | Confirmed — `verify` failed, target ref unchanged | **Validation-phase rejection only.** See the limits below |
| `prepare` locks every queued ref, and `verify` participates in that locking | Confirmed — `.git/refs/work-engine/effect-epoch/op1.lock` and `.git/refs/heads/target.lock` both present while prepared | Concurrent-revocation exclusion during the transaction window |
| A concurrent revocation **cannot** land while the transaction is prepared | Confirmed — the concurrent `update-ref` failed with `Unable to create '...op1.lock': File exists` (exit 128) | Revocation-vs-commit interleaving, during the prepared window |

The lock finding is stronger than the prior pass claimed, and worth stating
precisely: because `prepare` takes and **holds** a lock on every queued ref
including the `verify` target, a revocation cannot interleave between the epoch
check and the target ref write. Within the prepared window the race is
*excluded*, not merely resolved after the fact.

**What these tests do NOT establish**, stated plainly because the prior pass's
error was over-generalizing exactly here:

- **No crash was induced.** Nothing above tests process death or I/O failure *during* the commit phase. The single-ref shape means there is no second ref to leave inconsistent, and the documentation states individual ref updates are atomic — but that is an argument from documented per-ref atomicity plus transaction shape, **not** a crash test. A crash-injection test belongs in §14 and has not been run.
- **Stale lock files block subsequent operations.** The concurrent-revocation failure message is Git's generic advisory: *"a git process may have crashed in this repository earlier: remove the file manually to continue."* A holder that dies while prepared therefore leaves a `.lock` file that blocks later operations until cleared. This is consistent with §7.2's deliberate fail-closed posture, but it is a real operational consequence, not a free property, and it is a second route by which a dead holder blocks a domain.
- **Nothing was tested on the `reftable` backend** (see below), nor across clones, nor with concurrent non-`update-ref` Git writers.

#### The effect ledger, demoted

The append-only ledger is **retained as a non-authoritative audit trail** and
is no longer the basis of any commit/not-committed decision (§9.3, §9.4). Its
entries, when written, record:

```yaml
operation_id:
target_ref:
expected_old:
committed_new:
generation_vector_digest:
committed_at:
previous_ledger_commit:
```

Corrected write form, per finding 1: **genesis-only `create`**, then
`update refs/work-engine/effect-ledger/<targetScope> <newTip> <expectedOldTip>`
for every subsequent entry. Both forms are verified above.

Because the ledger is a second ref, a ledger write must **not** share the
authoritative transaction — that is the very coupling finding 2 rejects. It is
written separately, best-effort, and its absence is explicitly **not** evidence
about whether the effect committed (§9.3).

#### `reftable` — documented alternative, not adopted

Git's `reftable` backend would make genuine multi-ref atomicity available, and
would permit the two-ref design the restructure abandons. It is **not adopted
here.** Adopting it would require verifying (a) the repository format
requirement and migration path for every repository this mechanism touches,
and (b) its actual crash guarantee, neither of which has been done. The
repository tested is `files` format, confirmed mechanically. Until both are
verified, the single-ref trailer shape is the specified design.

#### Pre-existing facts, unchanged

**Neither `update-ref --stdin` nor the `reference-transaction` hook is used
anywhere in `app-server/src` today.** The only existing `update-ref` calls are
single-ref (`git-publisher.mjs:163`, `git-worktree.mjs:51`,
`git-worktree.mjs:71`). Both are therefore **proposed additions**, not existing
capability.

**No operation binding exists in the commit object today.** The publication
commit is built as
`["commit-tree", preparation.tree, "-p", preparation.expectedParent]` with
input `subject\n\nbody\n` (`git-publisher.mjs:145-146`). A `requestDigest`
covering `operationId` is computed, but it lives in the preparation record
(`git-publisher.mjs:115`), not in Git. Adding the trailers above is what would
put operation-bound evidence **in Git**.

### 8.3 What existing Git reconciliation actually does

Stated precisely, because the superseded document misstated it:

- `promote()` on failure: if the target ref already equals the intended commit, it consults `coordination.inspectAdmission(operationId)` and returns `published_observed` **if an admission row exists**, otherwise `unconfirmedPublication(...)` — status `published_unconfirmed` (`git-publisher.mjs:164-170`, `93-95`). It does **not** reconstruct a missing admission.
- `reconcile()`: on `observed === sealed.commit` it consults `inspectAdmission` (lines 176-178); on `observed !== sealed.expectedParent` it returns `ambiguous` (line 179); otherwise it returns **`eligible` without consulting `inspectAdmission` at all** (line 180).

That last branch is the ABA hole. Its exact exposure, stated without
overstatement: `withAdmission` rejects a reused `operationId`
(`sqlite-store.mjs:59-61`), so a committed-and-recorded operation cannot be
replayed. The live hazard is the narrower case where the ref advanced **and no
admission row was recorded** (a crash between `update-ref` returning and the
SQLite `COMMIT`), and the ref was subsequently moved back to
`expectedParent`. `reconcile()` then reports `eligible`, and a retry would
commit the effect a second time.

How far the restructure addresses this, stated precisely rather than as a
closure claim: a commit trailer is **positive evidence while its commit remains
reachable from the target ref**. In the rewind case above, the commit carrying
the trailer is no longer reachable from the target ref, so the trailer is not
observable and the negative inference is *still* unsafe. The restructure
therefore does **not** close the ABA hole by itself — it is closed only by
§9.3's additional preconditions (a proved no-rewind contract and executor
quiescence), and absent those the correct outcome is
`reconciliation_required`, not `eligible`.

---

## 9. Recovery and reconciliation

### 9.1 `localSqlCommit`

The mutation and its receipt share one transaction, so SQLite's own atomicity
decides: either both exist or neither does. Recovery reads the target-store
receipt for the `operationId`.

**Restructure responding to §1.1 finding 3 — OPEN, not closed.** An earlier
revision concluded "receipt absent ⇒ retry safe." That is conditional, and the
condition was missing. Receipt-absence only means *not yet committed*; it does
not mean *cannot still commit*. The corrected rule:

| Receipt | Additional condition | Meaning |
|---|---|---|
| present | — | committed; gate hold resolves `committed` |
| absent | **the target transaction is terminal** | did not commit; gate hold resolves `aborted`, retry safe |
| absent | terminality not established | **`reconciliation_required`** — no negative inference |

**"The target transaction is terminal"** means: it can no longer commit under
any circumstance. This wording is deliberate and replaces the earlier "the
holder is dead." Process death is *one route* to terminality, not its
definition — a live holder mid-transaction, a suspended process, or a
connection that has not yet been torn down are all cases where the holder is
not dead and the transaction is not terminal.

How terminality is established is **not settled by this document**
(§17 item 11). One commonly-assumed route is that SQLite rolls back an
uncommitted transaction when its connection is lost, so a confirmed
connection teardown would imply terminality for that transaction. **This has
not been verified** against `node:sqlite`'s `DatabaseSync` behaviour in this
repository, and it is recorded as unverified rather than relied upon. It is in
any case a route to terminality, not a general method for establishing it.

The gate store is a **different** database from the target store (§4.2), so a
crash can leave the gate in `committing` while the target store already shows
the receipt. The target-store receipt is authoritative for the effect; the gate
record is repaired from it. This is why §8.1 requires the receipt to be written
by the target transaction rather than by the gate.

### 9.2 `gitRefCommit`, and the zombie executor

A stale executor whose gate hold failed **may still succeed** if neither ref
has advanced. This document does not claim otherwise. Before a failed
operation's hold may be released as `aborted`, recovery must either:

1. establish that the executor cannot still commit, or
2. **advance/revoke the Git-visible operation epoch** that the ref transaction verifies.

For Git, (2) is preferred and is the specified mechanism. Revocation advances
`refs/work-engine/effect-epoch/<operationId>` **first**; only then may the hold
be released.

The race resolves as follows, with the two windows distinguished because they
have different strengths:

- the executor's transaction lands first → it commits, and the target commit's trailer records it (§8.2);
- revocation lands first → the executor's `verify` fails and its entire transaction is rejected (empirically confirmed, §8.2);
- **while the executor's transaction is already prepared**, revocation *cannot* land at all: `prepare` holds a lock on every queued ref including the `verify` target, and a concurrent revocation fails to acquire it (empirically confirmed, §8.2). Within that window the interleaving is **excluded**, not merely ordered.

Exactly one outcome occurs, and it is afterwards discoverable from the target
commit's trailer. That is the honest guarantee: the invariant holds because a
transaction that commits after revocation is rejected, not because the executor
is assumed dead.

Two limits, stated rather than left to a reviewer:

- Revocation ordering is only a **precondition for releasing the hold**, not a proof the executor has stopped. An executor that never reaches Git leaves the operation in `committing` until reconciled (§7.2).
- A holder that dies while its transaction is prepared leaves a Git `.lock` file that blocks subsequent operations on those refs until cleared (§8.2). This is a second route — alongside §7.2 — by which a dead holder blocks a domain, and it is a consequence of the non-expiring posture rather than a defect in it.

### 9.3 Git reconciliation outcomes

Rewritten for the §8.2 restructure. The authoritative evidence is the **commit
trailer**, not the ledger. The governing asymmetry, stated before the table
because every row depends on it:

> A trailer is **positive evidence while its commit remains reachable from the
> target ref** — it is **not** an independent history of every publication.

Positive and negative inference therefore have different costs.

**Positive inference** ("committed") requires only: a commit reachable from the
target ref carrying a matching `Work-Engine-Operation-Id` trailer. Reachability
is the condition; a trailer on an unreachable commit proves nothing about the
current state of the ref.

**Negative inference** ("did not commit") requires **all three**:

1. target ref `== expected_old`; **and**
2. a **proved no-rewind contract** for that ref over the relevant interval; **and**
3. **executor quiescence** — the executor can no longer commit (established by epoch revocation per §9.2, or by terminality in the §9.1 sense).

`target ref == expected_old` **alone does not prove "did not commit."** The ref
could have advanced and then been rewound, which is exactly the ABA case §8.3
describes. Absent either (2) or (3), the outcome is `reconciliation_required`.
There is no fallback to a negative inference.

| Evidence | Target ref | Conditions | Outcome |
|---|---|---|---|
| matching trailer on a reachable commit | any value | — | **committed** |
| no matching reachable trailer | `expected_old` | no-rewind contract proved **and** executor quiescent | **not committed** |
| no matching reachable trailer | `expected_old` | either condition unproved | **`reconciliation_required`** |
| no matching reachable trailer | any other value | — | **conflict / ambiguous** |
| trailer with same `operation_id`, different `generation_vector_digest` | any value | — | **operation identity conflict** |
| target ref unreadable | — | — | **`authority_state_unavailable`** |

#### 9.3.1 The unanchored window

The gate-store ledger-tip anchor (§9.4) detects rewinds **only for tips
successfully recorded there.** A crash between Git publication and the
gate-store anchor update leaves the newest effect **unanchored** — and that is
precisely the case recovery most needs to resolve.

That window must resolve to **`reconciliation_required`**. It must not resolve
to a negative inference, because the anchor that would have detected a rewind
of that specific commit is the one that was never written.

#### 9.3.2 Consequence for the no-rewind contract, stated plainly

The no-rewind contract of condition (2) depends on the `reference-transaction`
guard (§9.4) being in place **and verified to cover every Git write path this
repository uses.** §17 item 4 records that coverage as **UNCHECKED**.

Therefore, as specified today: **the Git path's negative inference is
unavailable, and recovery of a non-committed Git effect returns
`reconciliation_required`.** This is a stated limitation of the current design,
not an assumption that the guard is present. Making the negative inference
available requires closing §17 item 4 first.

### 9.4 Ledger integrity — narrowed audit claim

**Restructure responding to §1.1 finding 4 — OPEN, not closed.** The earlier
revision claimed hash-linking made deletion or rewind "detectable," and rested
marker-absence inference on that. Finding 4 is correct that it does not:

- **Deleting the only ledger tip** leaves no chain to check. Absence then looks indistinguishable from "never existed."
- **Rewinding to a valid ancestor** preserves hash-link validity while discarding later entries. Every remaining link verifies.

Hash-linking gives **tamper-evidence within a chain you can already see** — not
an existence proof, and not retention. The narrowed claim:

> The ledger is a **non-authoritative audit trail.** It is not the basis of any
> commit/not-committed decision (§9.3). Its integrity properties are
> tamper-evidence *within observable history*, bounded by the enforcement below
> and by the gaps stated after it.

Enforcement, with present-day status marked:

- Only the publisher identity may write either namespace (`effect-ledger/*`, `effect-epoch/*`). *(Proposed.)*
- A `reference-transaction` hook rejects ledger deletion, rewind, or a ledger commit whose parent is not the current ledger tip. *(Proposed; not present today — §8.2. Coverage across all Git write paths is **UNCHECKED**, §17 item 4.)*
- Repository permissions prevent other processes from rewriting ref storage directly. *(Proposed.)*
- Ledger history is retained indefinitely, or archived into hash-linked immutable segments. *(Undesigned — §17 item 6.)*
- The gate store records the **expected ledger tip** after each append, giving partial rewind detection: a tip behind the recorded anchor indicates a rewind. **Gap:** this covers only tips successfully recorded, and specifically not the unanchored window of §9.3.1.
- A rebuildable index may accelerate `operationId` lookup. Neither index nor ledger is authoritative for the commit decision; the target commit's trailer is (§8.2).

Direct administrative database or Git tampering remains **out-of-contract
corruption — detectable by audit where observable history permits, not
prevented.** A `reference-transaction` hook is repository-local and does not
travel with a clone; this is a real limit, stated rather than papered over.

---

## 10. Unsupported effects

Explicitly fail-closed, with outcome `effect_class_not_safely_admissible`:

- opaque network effects;
- human-facing irreversible actions;
- **any** effect whose commit primitive cannot produce durable, operation-bound evidence.

These are not "supported with caveats" and not admitted with a weaker promise.
They are outside the mechanism.

Introducing weaker drain-or-reconcile semantics for them later would be a
**deliberate canonical change to the invariant in §2**, made by its owners — not
an exception inserted inside this design. That distinction is the single most
important lesson of the three review passes.

---

## 11. Residual trust boundary

Named plainly, because a reviewer will otherwise find it unstated:

> **Safety depends on every canonical authority writer and every governed
> effect route being reachable only through the gate.**

This is materially stronger than the withdrawn adapter-compliance dependency
(§1) because the writer set is **bounded, internal, and enumerable**, whereas
adapters are open-ended. It remains a dependency, not a proof.

Made mechanically auditable:

1. Canonical stores expose **no ungated mutation capability** to adapters — the gate is in the type, not in the documentation.
2. Each writer route is **registered** with its gate-scope derivation.
3. Gate receipts record writer identity, acquired keys, and before/after generations.
4. Tests **enumerate every generation writer** and prove it enters the gate (§14).
5. Adding a new writer **fails closed** until it declares and tests its gate keys.

Item 5 is what keeps the invariant from eroding as the system grows, and is a
hard requirement rather than a convention.

---

## 12. Two hard prerequisites

Both block implementation planning. Neither is this document's to decide. They
are of **equal weight**: either one unsettled leaves gate scope non-derivable.

### 12.1 Upstream authority and policy invalidation (carried forward, unresolved)

`[KIND: RESIDUE] [OPEN]`

> Canonical owners must decide how parent-authority contraction and policy
> contraction invalidate or supersede dependent ExecutionEnvelopes.

A current envelope head does not prove its upstream authority remains current.
`mechanisms/revision-cas-and-publication.md` explicitly does not supply this —
it "does not decide when a revision should be superseded." Until settled,
`parent-authority/*` and `policy/*` cannot enter the gate scope or the
validation vector, and the vector is therefore known-incomplete.

**Owner:** `organizational-compilation.md` and/or
`authority-and-ownership.md`. This document takes no position and adopts no
placeholder default.

### 12.2 The closed validity-dependency-to-invalidation-key registry

`[KIND: RESIDUE] [OPEN]`

> Each canonical owner must declare, for every validity dependency it owns:
> its invalidation-domain key derivation; its current generation/value read;
> every writer capable of advancing it; the gate keys those writers acquire;
> and tests proving that effects and invalidating writers intersect on the
> same key.

Until this registry is complete and closed, **gate scope is not mechanically
derivable** — §5 rule 2 ("the union of every key affecting its validity")
cannot be computed from an open set. A partial registry yields a scope that is
silently too narrow, which fails open.

---

## 13. Failure vocabulary

Bounded outcomes. No generic `denied`.

| Outcome | Class |
|---|---|
| `admitted` | success |
| `gate_scope_unavailable` | contention — another holder holds an overlapping key |
| `gate_reconciliation_required` | stale state — a holder died in `committing` (§7.2) |
| `attempt_not_current` | semantic refusal — workspace admission holder changed |
| `authority_superseded` | semantic refusal — `ev_revision` moved |
| `effect_not_authorized` | semantic refusal — effect outside the admitted boundary |
| `subject_out_of_scope` | semantic refusal — subject outside the admitted scope |
| `role_not_authorized` | semantic refusal — role not admitted for this effect |
| `role_realization_stale` | capability staleness, not an authority refusal |
| `resource_fence_invalid` | physical fencing failure (Resource Lease and Fencing) |
| `operation_epoch_revoked` | the Git epoch was advanced before this transaction |
| `operation_replay` | identical `operationId` already admitted — rejected outright, per Resource Lease and Fencing Key Invariant 4 |
| `operation_identity_conflict` | `operationId` reused with different content |
| `authority_state_unavailable` | a canonical owner's state could not be read, or the target ref was unreadable |
| `malformed_authority_state` | a generation was read but is uninterpretable |
| `reconciliation_required` | **no safe inference is available** — negative-inference preconditions unproved (§9.3), the unanchored window (§9.3.1), or a holder died in `committing` |
| `ledger_lineage_corrupt` | ledger audit-trail integrity check failed — an **audit** signal (§9.4), never the basis of a commit/not-committed decision |
| `effect_class_not_safely_admissible` | outside the mechanism (§10) |

`reconciliation_required` is deliberately a first-class outcome rather than a
variant of refusal. It is the correct answer whenever evidence is insufficient
for either inference, and §9.3.2 means it is currently the expected outcome for
a non-committed Git effect.

Semantic refusal, capability staleness, unavailability, physical fencing,
malformation, and replay are deliberately distinct: collapsing them loses the
information a caller needs to decide between retry, reconciliation, and
escalation.

---

## 14. Test matrix

Negative tests (each must fail closed, with the named outcome):

| Case | Expected |
|---|---|
| caller supplies a forged/narrowed gate scope | ignored; host derives scope (§5 rule 1) |
| caller supplies a forged EV revision | ignored; gate re-reads from owner |
| caller requests a broader effect class than admitted | `effect_not_authorized` |
| EV supersedes between scope acquisition and commit | impossible — writer must acquire the same key (§5 rule 3) |
| EV supersedes before scope acquisition | `authority_superseded` |
| workspace admission transferred to a successor attempt | `attempt_not_current` |
| role realization stale | `role_realization_stale` |
| physical fence stale | `resource_fence_invalid` |
| valid EV + invalid fence | `resource_fence_invalid` |
| invalid EV + valid fence | `authority_superseded` |
| repeated identical `operationId` | `operation_replay` |
| repeated conflicting `operationId` | `operation_identity_conflict` |
| epoch advanced, then stale executor attempts Git commit | `operation_epoch_revoked`; target ref unchanged |
| revocation attempted while executor's transaction is prepared | revocation fails to lock; executor commits (§9.2) |
| stale executor commits before revocation lands | committed; trailer records it; discoverable (§9.2) |
| crash after Git commit, before SQL receipt | reachable trailer present → **committed** (not `published_unconfirmed` guesswork) |
| crash after SQL mutation+receipt, before gate update | receipt authoritative; gate repaired (§9.1) |
| receipt absent, target transaction **not** proved terminal | `reconciliation_required` — no retry (§9.1) |
| receipt absent, target transaction proved terminal | `aborted`; retry safe (§9.1) |
| holder dies in `committing` | overlapping effects **and writers** get `gate_reconciliation_required` |
| holder dies while Git transaction prepared | stale `.lock` blocks those refs until cleared (§9.2) |
| **ABA: ref parent → commit → parent** | trailer unreachable; no-rewind contract unproved → **`reconciliation_required`** (§9.3) — *not* "not committed" |
| ABA, with no-rewind contract proved and executor quiescent | **not committed**; retry safe (§9.3) |
| Git publication succeeded, crash before gate-store tip anchor | **`reconciliation_required`** — unanchored window (§9.3.1) |
| ledger ref deleted (only tip) | `ledger_lineage_corrupt` — audit signal only; commit decision still from trailer (§9.4) |
| ledger rewound to a valid ancestor | links verify; detected only if behind the gate-store anchor (§9.4) |
| second operation in the same ledger scope | append via `update <ref> <new> <old-tip>` succeeds (§8.2 finding 1) |
| EV store unavailable | `authority_state_unavailable` |
| target ref unreadable | `authority_state_unavailable` |
| opaque network effect requested | `effect_class_not_safely_admissible` |
| historical attempt replay | `attempt_not_current` |
| successor attempt under same slice | admitted on its own vector |

**Not yet run, and required before any acceptance claim:** a crash-injection
test for the commit phase. §8.2 establishes per-ref atomicity from
documentation and transaction shape, **not** from an induced crash. Killing the
process between `prepare` and `commit`, and during `commit`, is the test that
would substantiate it.

**Writer-route exhaustiveness** (§11 items 2, 4, 5) — a distinct class, and
the one that keeps the invariant true over time:

- enumerate every writer capable of advancing any vector component, and prove each enters the gate;
- prove every effect and every invalidating writer for a given component **intersect on the same key**;
- prove a newly added writer without a declared gate-scope derivation **fails closed**;
- prove no ungated mutation path to any canonical store is reachable from an adapter.

---

## 15. Implementation decomposition

**Nothing below may enter implementation planning while §12.1 or §12.2 is
open.** Marked accordingly.

| # | Unit | Status |
|---|---|---|
| 0 | Settle §12.1 (upstream invalidation semantics) | **BLOCKED — canonical owners** |
| 0 | Settle §12.2 (closed key registry) | **BLOCKED — all canonical owners** |
| 1 | ExecutionEnvelope store and revision participation (no store exists today, §4.2) | blocked on 0 |
| 2 | Gate store: multi-key atomic claim, operation state machine, non-expiring holds | blocked on 0 |
| 3 | Host-side scope derivation from the §12.2 registry | blocked on 0 |
| 4 | `localSqlCommit` primitive | blocked on 2, 3 |
| 5 | `gitRefCommit`: epoch refs, commit trailers, single-ref `update-ref --stdin` transaction | blocked on 2, 3 |
| 5a | Ledger writer as a **separate, non-authoritative** append (genesis `create`, then expected-tip `update`) | blocked on 5 |
| 6 | `reference-transaction` guard + coverage audit across every Git write path (§17 item 4) — **prerequisite for the Git negative inference** (§9.3.2) | blocked on 5 |
| 7 | Writer-route registration + gate entry for every existing writer | blocked on 2, 3 |
| 8 | Receipts, failure vocabulary, reconciliation entry points | blocked on 4, 5 |
| 9 | Test matrix (§14), including writer-route exhaustiveness **and commit-phase crash injection** | blocked on 7, 8 |

Unit 7 is the one most likely to be underestimated: it modifies existing
writers across several services and is where the §11 residual is either
genuinely closed or quietly left open.

An orthogonal, **already-actionable** item, independent of this entire design:
`reconcile()`'s `eligible` branch does not consult `inspectAdmission`, while
its sibling branch does (`git-publisher.mjs:176-180`). That asymmetry is a
narrow but real double-commit exposure in current code (§8.3) and can be filed
and fixed on its own, without waiting on §12.

---

## 16. Canonical documents requiring later amendment

Named only. **None edited.** None should be edited before §12 is settled.

- `authority-and-ownership.md` — §12.1 decision; gate as the serialization point at which authority currency is enforced.
- `organizational-compilation.md` — §12.1 decision; ExecutionEnvelope's store and revision participation.
- `mechanisms/resource-lease-and-fencing.md` — cross-reference only: the gate is a sibling, this mechanism is unchanged and contributes `resource_fence_generation`.
- `mechanisms/revision-cas-and-publication.md` — cross-reference only: ExecutionEnvelope revisions as an instance.
- `role-and-contract-structure.md` — confirm the role contract is a projection boundary, not a live authority source.
- `runtime-realization.md` — `role_realization_revision` as a vector component; relationship to the gate.
- A **canonical slice/campaign view**, which does not exist. `slice_campaign_admission` is the authoritative attempt-currentness record (§5.1) and must be documented and queryable as such. This gap has now been hit by every round of this work.
- A new mechanism page for the gate itself, if §4's shape argument is accepted.

---

## 17. Unverified assumptions and open questions

Explicitly unverified. Not explained, not narrated.

1. **Gate store substrate choice.** `[KIND: BACKLOG] [OPEN]` §4.1 specifies the *semantics*; whether the gate store is a third SQLite database, a table in an existing one, or another substrate is undecided. A third database adds a contention point; sharing one couples the gate to that store's availability. Not resolved here.
2. **Gate hold duration under load.** `[KIND: BACKLOG] [OPEN]` An exclusive multi-key hold spanning a Git subprocess is a real throughput cost. Unmeasured. No benchmark exists.
3. **Canonical key ordering sufficiency.** `[KIND: BACKLOG] [UNCHECKED]` Lexicographic ordering prevents deadlock among gate participants; whether any writer acquires a non-gate lock while holding gate keys (a lock-ordering inversion across mechanisms) has **not** been audited.
4. **`reference-transaction` hook coverage.** `[KIND: BACKLOG] [UNCHECKED]` Verified to exist in git 2.53.0's documentation; **not** verified against every Git invocation path this repository uses, nor against non-local operations. Hooks do not travel with a clone. **This item now gates a functional property, not just an audit property:** §9.3.2 makes the Git path's negative inference unavailable until this is closed.
5. **Ledger scope granularity.** `[KIND: BACKLOG] [OPEN]` `<targetScope>` is unspecified. Per-branch, per-repository, and per-workstream all have different contention and growth profiles. Undecided. Lower-stakes than in the prior revision, since the ledger is no longer authoritative (§9.4).
6. **Ledger growth and archival.** `[KIND: BACKLOG] [OPEN]` Hash-linked immutable segments are asserted as an option, not designed. No retention policy exists.
7. **Epoch ref lifecycle.** `[KIND: BACKLOG] [OPEN]` When an epoch ref may be deleted after an operation terminalizes, without weakening §9.3's "epoch reconciled" precondition, is undecided.
8. **`role_realization_revision` read path.** `[KIND: BACKLOG] [UNCHECKED]` Whether a current/applicable RoleRealization revision is cheaply readable as a generation has not been verified against `runtime-realization.md`'s implementation status.
9. **Status-block grammar.** `[KIND: BACKLOG] [ANSWERED]` Resolved during authoring, empirically rather than by reading the grammar's prose: `app-server/scripts/validate-idea-status.mjs` treats a document carrying `idea_provenance` **without** `idea_status` as an **error**, not as `UNAUDITED` — `UNAUDITED` requires *neither* block. An initial reading of [Idea Status Grammar](idea-status-grammar.md) §7 ("absence means not yet run through this process") got this backwards. Both blocks are now declared. The operator's intended `design` / `authorization` / `implementation` axes are the *canonical* axes, which that grammar states do not fit this corpus, so the design-only authorization is stated in prose in the Status section instead of in those axes.
10. **Whether §4's new-sibling-mechanism argument survives review.** `[KIND: RESIDUE] [OPEN]` `mechanisms/resource-lease-and-fencing.md` set a precedent of resolving a similar question *against* a new mechanism. §4.1 argues shape difference on verified grounds, but the judgment belongs to the mechanism corpus's owners. **Weakened this revision:** the mutation-boundary axis was false and has been withdrawn, leaving only two axes (multi-key scope, non-expiring holds) to carry the argument.
11. **How transaction terminality is established.** `[KIND: BACKLOG] [OPEN]` §9.1 requires "the target transaction is terminal" before any negative inference. No general method for establishing it is specified. The commonly-assumed route — SQLite rolling back on connection loss — is **unverified** against `node:sqlite`'s `DatabaseSync` in this repository, and in any case is one route rather than a method. Until specified, the SQL path's negative inference has the same status as the Git path's (§9.3.2): unavailable, outcome `reconciliation_required`.
12. **Commit-phase crash behaviour, untested.** `[KIND: BACKLOG] [UNCHECKED]` §8.2's single-ref safety argument rests on `git-update-ref(1)`'s documented per-ref atomicity plus the transaction having only one write. **No crash was induced.** A kill between `prepare` and `commit`, and during `commit`, is required to substantiate it (§14). The prior revision's error was over-generalizing a validation-phase test into a crash-atomicity claim; this item exists so that is not repeated silently.
