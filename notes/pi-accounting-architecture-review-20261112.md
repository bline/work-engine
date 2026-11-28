# Pi accounting architecture review

Date: 2026-11-12, America/Denver. Author: Codex. Requested by the Work Engine owner.

**Recommendation:** give accounting state transitions a focused redesign before
another cycle of narrowly scoped fixes. The evidence already justifies it;
another P1 regression is not needed to establish the value of reassessment.

This is a dated architectural assessment of Claude session
`7c0cdefa-513d-4f30-9bd4-7575bebdeb52`, centered on the
[P1.25 report](/home/bline/cbm-work/p125-review/P1.25-REPORT.md) and frozen
A1.3.3. It is not implementation approval, an independent acceptance review,
or deployment qualification. At the analysis cutoff, 2026-11-13 03:02 UTC,
the session was already implementing P1.26 / A1.3.4. Its stronger fallback
invariants and interaction tests are a useful foundation for this proposal.
The assessment does not evaluate the completed A1.3.4 result or change that
work's authorized scope.

The session made real progress, but its original Plan IR experiment became
blocked on increasingly broad accounting qualification. Many review findings
are inherited defects or incomplete repairs, rather than new regressions.
Nevertheless, the same boundaries keep producing failures: request identity,
settlement validity, pricing uncertainty, time attribution, and retirement.

**The strongest architectural evidence is N-01.** In frozen A1.3.3, pricing
appends an identity-conflict diagnostic to global `ISSUES`. The caller
interprets any new diagnostic as failed pricing, deletes those diagnostics,
and substitutes a reservation-only result. That discards supported usage
evidence and the larger exposure estimate. The archived reproduction changes
admission from refused at 0.08 on A1.3.2 and the deployed reader to admitted
at 0.05 on A1.3.3. The objective limit is 0.10 and the proposed new allocation
is 0.04. This is a synthetic reproduction; occurrence in actual billing was
not established. Whether a realistic reservation would be that small was
also unestablished. The loss of usage evidence occurs even with the larger
reservation control.

The causal branch is in the
[frozen reader](/home/bline/cbm-work/p125-review/trees/a133/metrics/scripts/usage_ledger.py),
lines 1327–1342. The
[P1.25 dispositions](/home/bline/cbm-work/p125-review/docs/REPRODUCTION-DISPOSITIONS.md)
and [archived output](/home/bline/cbm-work/p125-review/repro/out/repro-on-a133.txt)
record the outcomes and controls. Adding a diagnostic changes money
calculations through an unrelated control path; that makes correctness
fragile.

**1. Give reservation transitions one explicit owner.**

Extract a pure function that takes an attempt's evidence and returns its
accounting state. Its result should separately represent:

- Reservation and settlement evidence.
- Supported usage valuation and conservative exposure.
- Whether settlement is sufficiently established to release the reservation.
- Blocking uncertainty and explanatory diagnostics.
- Evidence references.

Diagnostics should not act as implicit return values. Only an explicit,
validated settlement decision should release or replace a reservation.
For one attempt, reservation and usage evidence need to be reconciled without
double counting. Distinct attempts remain separate obligations. Unknown
exposure remains unknown, even when a known partial amount is available.
Proxy-accounted amounts, usage valuations, and verified provider billing
retain their different meanings.

This is the highest-value immediate refactor. Moving existing branches into
different files would not accomplish it. The useful structural change is to
make evidence, validity, and exposure explicit outputs, so a diagnostic cannot
accidentally select a weaker representation.

**2. Separate accounting facts from time-window projection.**

Establish what happened and what can be priced before deciding which
objective window contains it:

```mermaid
flowchart LR
    A[Producer records] --> B[Validated evidence]
    B --> C[Attempt accounting]
    C --> D[Objective attribution]
    D --> E[Admission decision]
```

An uncertain timestamp should affect attribution without destroying otherwise
valid usage evidence. An interval crossing objective closure should remain
visible as uncertain attribution. A reporting window does not establish
settlement or release an obligation.

A1.3.3 interleaves pricing, placement, deduplication, diagnostics, and reservation
release inside a 454-line `build_pi`, compared with 136 lines in the deployed
reader. Size alone is not the defect; the interacting responsibilities explain
why a local repair can change an unrelated accounting consequence. See
`build_pi`, lines 1097–1550 in the
[frozen reader](/home/bline/cbm-work/p125-review/trees/a133/metrics/scripts/usage_ledger.py),
and the closed-window finding U-04 in the
[P1.21 report](/home/bline/cbm-work/p121-oneshot-review/P1.21-REPORT.md).

**3. Make request identity explicit at the producer.**

The producer uses `accepted` both as a capacity counter and as request number
`n`, then decrements it on refusal. That permits reuse. The reader subsequently
treats matching `n` values as evidence that another row accounts for a stream
error. N-04 demonstrates why those meanings should be separated. The relevant
producer code is `EgressProxy.start.Handler.do_POST`, lines 1344–1360 in
[pi_container.py](../skills/manual-orchestration/scripts/pi_container.py).
The matching reader branch starts at line 1502 in the frozen A1.3.3 reader.

A separate producer change should assign each attempt a unique identity,
scoped to its proxy incarnation, and carry it through reservation, forwarding,
errors, and completion. Provider/lane evidence and the pricing basis should
be consistently attributable to that attempt. Capacity counters should remain
capacity counters.

Historical records still need a conservative interpretation: ambiguous
correlation remains unresolved. A new schema cannot retroactively establish
missing identity. P1.25 did not establish the Pi concurrency needed for its
specific N-04 scenario; the severity assessment in its dispositions is
conditional on that uncertainty. This proposal does not treat subsequent
P1.26 investigation as a completed finding.

**4. Share lifecycle interpretation between spending and admission.**

The accounting reader recognizes that delivery can reopen a retired session.
Admission still uses the existence of a retirement to release commitment.
Those are incompatible interpretations of the same lifecycle, producing N-07.
See `pi_admission`, beginning at line 1515 in
[usage_ledger.py](../metrics/scripts/usage_ledger.py), especially the retirement
set at line 1556; and `Pi.deliver`, beginning at line 1543 in
[role_host.py](../skills/manual-orchestration/scripts/role_host.py).
P1.25 reproduced N-07 on the legacy allocation path; it did not test the
journal path for that finding.

One lifecycle reducer should interpret launch, delivery, and retirement history
for both consumers. Reopening should either preserve or reacquire the required
commitment, or be refused before execution. The admission owner should define
that policy explicitly. This remains the separate N-07/S-08 work already
identified; it should not silently expand A1.3.4.

**Strengthen the accumulated qualification evidence.**

Promote the reproduced defects into a stable regression corpus with
independently specified expected outcomes. The P1.25
[reproduction script](/home/bline/cbm-work/p125-review/repro/repro_p125.py)
explicitly says it prints outcomes and asserts nothing about whether they are
defects. That is appropriate for investigation, but its `OK` result does not
establish correctness or automatically protect a later implementation.

Permanent tests should assert exposure, retained evidence, attribution, and
admission consequences. Include generated event sequences covering duplicate
records, missing settlements, reused counters, retirement/reopening, clock
rollback, and contradictory identities. Keep expected amounts independent of
the reader's pricing helpers. Adding uncertainty without new valid settlement
evidence should not make admission more permissive. Duplicate evidence should
not suppress a distinct obligation. Tests also need valid controls so that
rejecting everything cannot satisfy the qualification.

The existing identity-refusal fixture illustrates the gap: it names a model
mismatch without actually giving the request and response different models.
See lines 183–191 of the frozen
[s2 tests](/home/bline/cbm-work/p125-review/trees/a133/metrics/scripts/tests/test_usage_ledger_pi_s2.py)
and the N-01 disposition. P1.26 already requests correction of this fixture
and metamorphic tests; preserve that work.

**Recommended next boundary.** Preserve A1.3.4 as the current repair checkpoint,
then make the next architectural review subject the extracted accounting
transition boundary and its regression corpus. Its new invariant is a good
foundation; the implementation should make violating it structurally
difficult. A new permanent component should follow the repository's
[Rust-first direction](../docs/rust-development.md). The immediate Python
repair can remain a bounded correction; a language migration alone would not
resolve the ownership problems.

Reader acceptance remains separate from deployment readiness. A correct reader
does not close the outstanding commitment, admission-guard, and retry work
needed to resume the pilot. The producer identity and admission lifecycle
changes above need their own bounded implementation and qualification scopes.
Preserve historical evidence and unresolved obligations through any replacement.

**Evidence basis and limits.** The assessment used the local
[Claude transcript](/home/bline/.claude/projects/-home-bline-code-work-engine/7c0cdefa-513d-4f30-9bd4-7575bebdeb52.jsonl),
frozen source, producer code, and archived reproduction results. The transcript
is appendable; the cutoff above bounds the observations used here. No suites
were rerun and the evolving A1.3.4 candidate was not qualified by this review.

- Frozen A1.3.3 reader SHA-256:
  `f15263e0447596feff15d74acec2b332673d4b46f79f9d2a657d0190e53d6a71`.
- Repository source references are bound to commit
  `697d66213b3dd89e57a4f28a20d13dff8dbe41a0`; relative links display the current
  checkout, so consult that commit if those files subsequently change.
- Structural discovery used Tier 2 verification against graph project
  `home-bline-code-work-engine`, generation `2026-11-13T02:51:22Z`, store
  generation `u7163423b6c9f1aa7g2`. Relevant searches and direct traces were
  consumed; exact snippets and direct source supported the material claims.
- Coverage reported unresolved operator calls in the three repository source
  files; relevant reported ranges were read directly. Frozen external trees,
  reports, and the transcript were outside the graph project and were read
  directly. The transcript connector required reauthentication, so transcript
  access used the local file. This is bounded architectural evidence, not an
  exhaustive audit or independent runtime qualification.

The original assessment was announced on the repository chatboard at sequence
5082. This note preserves that assessment for later work; it does not supersede
the original review findings or the owner's current execution instructions.
