{
  "verdict": "REVISE_PLAN",
  "primary_boundary": "C refinement and F co-reference coverage versus conserved highs and independently supported additive lows",
  "ordinary_repair_recommended": false,
  "implementation_under_plan_recommended": false,
  "required_design_changes": [
    "P1: Define a discovery-independent refinement normal form that cannot merge previously additive children without eligible F coverage.",
    "P1: Separate conservative hold ownership from supported-low ownership when distinct evidence origins may describe the same usage.",
    "P2: Specify affirmative co-reference and financial-candidate predicates; observed RS uniqueness cannot alone establish exact release coverage."
  ],
  "owner_decisions": [
    "Adopt explicit refinement and supported-low multiplicity semantics before implementation.",
    "Identify the evidence standard that permits auxiliary anchors to be discharged, including its applicability to partial source discovery.",
    "If full executable raw-to-output conformance is required in S1, resolve that acceptance requirement against D3 explicitly; static agreement cannot establish it."
  ],
  "evidence_limits": [
    "All 51 file boundaries and the final review-set sentinel are present; hashes and byte counts were not independently recomputed.",
    "No tools, commands or tests were executed. Census and reproduction results remain attributed evidence.",
    "The complete corpus, raw audit FACTS, individual inventory entries, full production implementation and live authority are unavailable or unestablished.",
    "This review neither clears Module A/B/C nor establishes full raw-to-output conformance or live enforcement."
  ]
}

## Diagnosis and amended policy

The reviewed candidate is **`candidate/PLAN-R5.md`**, not the superseded R4 plan.

R7’s failure is a monetary-partition change: separately summed contributions become one maximum. Its origin remains unestablished—not a demonstrated repair-caused regression. Recurring I1 independently triggers architectural review; infrastructure, quota failures and timeouts count zero implementation rejections.

`owner/ROOT-POLICY-R5.md` genuinely resolves the previous arrival-row-kind conflict. Late discovery of a source-earlier reservation may now establish eligibility. **AtLeast15M → Exact15M is an authorized release only when the complete witness covers the affected obligation.** Repeating the old blanket Rejected→Valid objection would be incorrect.

The amendment’s principle is coherent: authority comes from complete affirmative coverage, not whichever row arrived last. Its safety depends on proving coverage and preserving everything outside it. R5 improves that boundary substantially, but its refinement and low aggregation rules still admit incompatible implementations.

## Three material issues

### 1. P1 — Refinement can merge siblings, or become discovery-dependent

**Location:** `candidate/PLAN-R5.md:42–52,90,108,141`.

Consider one initially scoped TicketClaim containing:

- reservation origin **r**, 500M;
- settlement origin **s**, charge and supported usage 15M, but incomplete usage—therefore no eligible replacement.

Now add definite producer-identity evidence in stages:

1. Only **r** is proved to belong to producer attempt P. Line 48 places r in `(parent,P)` and s in `(parent,residual)`. Their retained exposures sum to **AtLeast515M**.
2. Add proof that **s also belongs to P**.

Recomputing line 48’s deterministic children places both origins in `(parent,P)` and removes the empty residual. The maximum becomes **AtLeast500M**, without eligible F coverage. All components survived, but their additive ownership did not.

Alternatively, interpreting “never merge two C units” strictly preserves the two children. Then supplying both identity proofs all at once can produce one producer-attempt unit under line 42, or one P child, yielding500M rather than515M.

Thus the plan needs more than its asserted nonmerging invariant: **flat regrouping violates conservation; history-preserving subdivision conflicts with final-set normalization.**

**Required change:** define immutable base-assignment inputs separately from later identity evidence, and an explicit refinement normal form monotone under evidence inclusion. Later co-reference must not reunite additive siblings outside F. A bounded atomization strategy is a credible alternative; another unspecified “refinement” operation is not.

**Acceptance witness:** the partial-P-proof sequence above, reversed discovery, and all-at-once input must produce identical final ownership and totals, with no unauthorized515→500 decrease.

### 2. P1 — Additive evidence origins do not establish additive supported usage

**Location:** `candidate/PLAN-R5.md:43–46,100–104`.

Take two different origin positions in **Unestablished** scope containing identical dollar-settlement bytes: same response identity, rid, and usage worth15M; no reservation is supplied.

The plan deliberately does not deduplicate them. Each becomes an OriginAtom, with supported low15M and unresolved high. R sums the lows to **30M**.

But an admitted possibility is one request whose record was copied twice. That possibility supports15M, not a30M aggregate lower bound. The conservative high may intentionally retain both policy holds; that does not make the usage observations independently additive.

This is precisely where **evidence identity must remain distinct from obligation independence**. Equal amounts are not proof of duplication, but distinct evidence positions are not proof of independent usage either.

**Required change:** distinguish conservative hold units from supported-low credits. Define how unresolved co-reference affects aggregate lows while preserving each observation’s valuation and provenance. Do not silently redefine “supported low” as a sum of policy holds. Any changed external low semantics requires root disposition.

**Acceptance witnesses:**

- uncertain copies retain conservative highs without doubling supported usage;
- independently established identical-byte incarnations remain additive;
- equal-amount, byte-distinct reservations remain conflicting rather than UniqueReservation;
- the established scoped15M/15M/20M/60M lows remain preserved.

### 3. P2 — Auxiliary release coverage lacks an affirmative admission predicate

**Location:** `candidate/PLAN-R5.md:56–68,78–80,143`.

The plan permits a “declared applicable unique RS co-reference witness” to absorb an auxiliary anchor. It does not define what establishes that witness under arbitrary subset discovery.

The supplied producer makes observed n-uniqueness insufficient. Threaded handlers increment `accepted` and preflight failures decrement it (`producer/pi_container.py.excerpts`, original lines1351–1360). Consequently, overlapping requests can reuse n.

A partial observation can contain:

- R1;
- a stream error from another forwarded request sharing n;
- S1, a complete settlement for R1;

while the second request’s reservation is not yet discovered. The observed RS-1 uniqueness conditions can pair that error with S1. Absorbing its anchor would produce Exact15M, although the additional evidenced attempt remains potentially unbilled: the aggregate must retain unresolvedness.

If R5’s “unique” means stronger evidence than the supplied RS rules, that stronger predicate is missing. Merely naming the result “definite” does not establish it.

**Required change:** specify positive ticket/producer linkage, or independently established scope completeness and sufficient writer guarantees, for release-bearing auxiliary co-reference. Otherwise retain the anchor. Also make “compatible” and “no narrower relation” precise enough that new evidence cannot shrink financial candidates.

**Acceptance witness:** the partial-source reused-n case must not discharge the unrelated anchor; a genuinely qualified co-reference positive must still discharge it.

## Alternatives and retained improvements

**Stable initial TicketClaims** prevent the original rekeying decrease by starting same-reference orphan charges at500M. Root explicitly selected that changed policy. It is coherent only as an unresolved conditional hold—not proof of one physical attempt or permission to use500M as finite headroom. Collision and multiplicity evidence must remain explicit.

**Explicit merge accounting** preserves constituent holds but needs exact discharge ownership. R5’s F coverage is a useful bounded form of that approach.

The **smallest credible revision** retains stable base units, monotone pricing context, component provenance and exact F coverage, while fixing refinement and lower-bound independence. A general interpretation envelope or replacement financial state model is not yet justified.

The retained High order is appropriate: numeric preservation alone cannot justify AtLeast→Exact. Likewise, summing conservative per-unit highs160M instead of a joint-envelope100M is an explicit policy choice; it does not justify summing unsupported lows.

## Qualification and ownership

R5 correctly targets the actual remaining consumers:

- **S:** removing writable decision constructors establishes one validity constructor owner; independently challenge the owning C result as well.
- **Serialization:** use actual `pool`, `kind`, decimal `nano`, basis strings, valuation amount strings and `derivations`, obligation `amount_source/by`, and qualified reason strings—not internal tagged-enum round trips.
- **Python:** `validator/validate_corpus.py:473` forces the wrong zero low; lines438–449 normalize duplicate JSON. The actual entry points must reject independently constructed, internally consistent wrong expectations and matching rollups.
- **Dependencies:** predicate changes must invalidate V/F/X/R, generated views, validator qualification and relevant acknowledgements—not merely alter generated bytes.
- **Coverage:** require lawful raw orphan/refusal/subscription-dollar and missing-launch witnesses, malformed model7 negatives, real reservation multiplicity, and preserved established priced multiidentity finite bounds.

These are credible plan obligations, not completed qualification. Independent authorship and custody must remain established separately from generator agreement. Monetary resolution must remain distinct from temporal closure. Trusted fixture gates must remain exact-payload-bound `test_mock` inputs, never live verification.

D3 can accommodate revised declarations, static mapping/ownership checks and separately authored bounded witnesses. It cannot supply a general coverage solver or full financial evaluator disguised as qualification.

This recommendation grants **no mutation, production, S2a or contract-change authority**. Root owns adoption; the architecture gate, unresolved R7 findings and review history remain.