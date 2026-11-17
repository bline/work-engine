revise_before_owner

# Focused review of the R2 reconciliation

Reviewer: Claude Fable 5.2, fresh adversarial design reviewer, read-only. Subject:
`design/RECONCILIATION.md`. Source: `/home/bline/code/work-engine` at `9b289171` (HEAD taken from the
session's git snapshot; I could not run git, shell was denied). No builds, tests or runtime observation:
every statement below comes from reading source, n = 1 reading per cited site.

Verdict: **revise_before_owner**. H1 and the K-c source claim hold. S-A and S-B are sound to put to the
owner after two wording fixes. S-C, N3(a) and N4 are not ready: the issuer and revocation mechanisms they
name cannot work against the lifecycle store's single-writer lock as written, and N4 omits the operator's
standing K-c decision.

## What I checked and what held

| Check | Result | Evidence read |
|---|---|---|
| 1. Transaction held across the send | Holds | `lifecycle-store/src/pi_coding.rs:209-221`: `BEGIN IMMEDIATE`, `parent_binding`, `send(...)`, then commit. Same shape for the tool effect, `:88-138` |
| 1. Currentness predicate | Holds as described | `pi_coding.rs:93`, `:173` (revision, revoked, issuer, scope, subject/context/build/proof-run, expiry, admin ownership); session currentness `:102`, `:181` |
| 1. Consumption before entry | Holds, with a qualifier | `coding.rs:320-321` consumes before the transaction, but in process memory (`hosted_attempts`). The durable record is the Host journal: `gate_entered` saved at `pi_coding_hosted.rs:869-872` before `transport.send` at `:876`, both inside the lifecycle transaction |
| 1. Settlement owners | Exist | `pi_coding_hosted.rs:914` (`settle_attempt`), `pi_coding.rs:285` (`settle_pi_coding`); bodies not read |
| 1. review-delivery is outside those owners | Holds | `work-engine-review-delivery/Cargo.toml:19-28`: no manual-host or lifecycle dependency |
| 3. Grants carry no independent authority | Holds | `sqlite.rs:626` compares the grant's issuer only with the opener's string; `coding-qualification.rs:168-169` opens as `"fixture-root"`; `:178-205` installs four grants; `pi_coding.rs:93`/`:173` do not test `principal` |

H1 is correct: a spawn-boundary host in review-delivery would be a second mechanism with the same
production meaning. The anti-pattern quote is at `docs/contracts/system-design.md:226-227` (section number
not checked). Qualifier for the owner: the existing owners accept only three coding profile pairs
(`pi_coding.rs:42-56`), so they own the mechanism class, not a review delivery effect.

## Findings

### P1-1. The external issuer and the revocation fence cannot reach the store as designed (§2.2 a, §4, S-C, N3, N4)

`SqliteLifecycleStore::open` takes `writer.lock` with `try_lock` and holds it "for this value's lifetime"
(`sqlite.rs:196`, `:246-260`). The store is per job, under the job's state directory, and the producing
binary creates it and registers the subject (`coding-qualification.rs:165-177`).

Consequences:

- "A separate process owns the write" (§4) fails: a second process cannot open the store while the job
  holds the lock.
- "The issuer calls `revoke_trusted_grant`" (§2.2 a) fails for the same reason, and the document names no
  way for an issuer to find each running job's store.
- Issuance has an unaddressed ordering: grants bind a subject row that the producer creates at start, so
  an external issuer needs a handshake or must create the store itself.
- "Signed grants verified at install" does not make issuance "unforgeable by the producing binary": the
  verifying code and the database file both belong to that binary. It gives decision independence for an
  honest producer only. The threat model must be stated.

Completeness basis: I read `open`/`open_inner` to `:269` and `coding-qualification.rs:150-219`. Whether
another open path admits a second process, or the server pre-creates the store, is not established.

Required revision: name the channel by which an issuer installs and revokes in a per-job single-writer
store (for example, the Host applies issuer-signed install/revoke messages), state the threat model, and
re-estimate S-C.

### P2-1. N4 omits the operator's standing K-c decision

`docs/dev-vertical-open-items.md:33-38` records: "acceptable for single-operator local use; revisit
(independent grant issuer) before jobs can be submitted by anyone other than the funder". S-C builds the
issuer with the funder as the only principal, which is the case already accepted, and does not deliver
non-funder submission. Its only consumer is then the open item itself: the S1 lesson. N4 must quote the
decision and its trigger, and the recommendation should be "hold until non-funder submission is wanted".

### P2-2. S-C has no defined decision content

Gate O and Gate R are review-shaped (separation, component binding, realization). The document does not
say what organizational authority decides for a coding job with one principal, and its observation source
does not exist (O6). Missing owner question: "what does an org admission mean for a coding job?"

### P2-3. "Held in one serialized step" overstates §2.2(b)

The `BEGIN IMMEDIATE` lock serializes writers of the lifecycle database only. An organizational
observation read inside the closure is a point-in-time read; a change after the read and before dispatch
is not fenced. (b) shortens the window that the document uses to reject (a) alone; it does not close it.
The judgment that (a) alone is insufficient is correct: (a) cannot fail closed on an unknown observation.

### P2-4. Unstated consequences of the per-effect regate

The regate is workable: the closures already do journal and upstream I/O under the lock
(`pi_coding_hosted.rs:872`, `:876`), and I found no re-entrancy hazard provided the handle never calls
back into `CodingContinuity` (both `RefCell`s are mutably borrowed, `coding.rs:327`, `:1092`). The
document should add:

- Lock order. The Host holds the lifecycle lock and then reads org state. An issuer must commit org state
  and release before touching lifecycle, never nested. `busy_timeout` is 250 ms (`sqlite.rs:178`).
- Liveness. The attempt is consumed before the regate (`coding.rs:321`) and the tool effect is marked
  attempted before it (`coding.rs:104-109`). A transient observation error therefore ends the attempt, and
  with one tool call per job, the job. O6 must require a local, bounded-latency source.
- Coverage. Three closure sites exist (`pi_coding.rs:74`, `:202`, `:227`); the document names two.
  `with_pi_coding_reset` is unlisted.
- Placement. For the tool effect the wrapper belongs in `CodingLifecycleGate::execute`
  (`coding.rs:120-129`), which passes the Host's actuator straight through.

### P2-5. "S-A + S-B meet R2's exit as the approved design states it" contradicts §6

Approved O4 and addendum item 3 make delivery a host-owned boundary (reserve, observe, gate, regate).
S-B builds none at the spawn, and no review runs on the hosted path (N7 deferred). After S-A and S-B no
path enforces reservation-sealed delivery admission for a review. The sentence should read "as amended by
N1 and the O4 change", and N1 should be split: N1a, do not build the spawn-boundary host (supported);
N1b, the production home is Host plus lifecycle (conditional on N7, whose feasibility §3 calls not
established).

### P3

- §2.1 projection lists two scopes; four grants are installed, including `EnqueueInput`
  (`coding-qualification.rs:186-190`).
- `coding.rs:503` and `:507` hard-code `grant_revision: 1` and `"operator:fixture"`; N6 needs both changed.
- §2.3 says a stale refusal is "released"; the code path calls `hosted_uncertain`
  (`pi_coding_hosted.rs:891`), and a refusal placed before `dispatch` is labelled "gate refused", not
  "refused inside gate admission". The released outcome is not established.
- S-A adds `realization_digest` and `GrantProjection`, whose only consumer is S-C. If S-C is held, move
  both out of S-A.

## Slices and sequencing

- **S-A**: correctly bounded as a pure crate, minus the two S-C-only items above.
- **S-B**: correctly bounded. It is the slice with real consumers (discharges and the consumption regate).
- **S-C**: not ready. It depends on P1-1, P2-1, P2-2 and on O6.
- Order S-A, S-B, then an S-C design round only if the operator wants non-funder submission.

## Not read

Amendment 6.1, `PROPOSAL.md`, `s2/review/REVIEW.md`, `native.rs` (`claim_native_turn`),
`review-episode-core`, `job.rs`, `coding-job-prepare.rs`, `lifecycle-service/src/actor.rs`,
`session-task-admission`, the qualification README, and `AGENTS.md`/`core.md` beyond their first 110 and
90 lines. Cost figures were not checked. The board read was not performed (shell denied).
