**ACCEPT_FOR_BOUNDED_IMPLEMENTATION — P1: 0 · P2: 0 · P3: 0.**

Subject: [PLAN.md](/home/bline/cbm-work/pi-accounting-continuation-20261114/design/PLAN.md), SHA256 `43d0e7c8df270eb77d9aff32c87e03f5000bc89163da676c8a50daf7affdbe61`, with SOURCE-BINDINGS SHA256 `64ad366edc751a374a0029095c91ba405855a31be82908455daccd0e3e0c2fa6`. This is architectural acceptance of the bounded candidate, subject to root adoption—not acceptance of R7 or completed qualification.

I evaluated root’s explicitly selected semantics: immutable initial TicketClaims; whole-base atomization; separate aggregate supported lows; affirmative exact release coverage regardless of which prerequisite arrives last; and unchanged VC07 remaining aggregate `AtLeast15M` because its auxiliary anchor lacks linkage. I did not apply the superseded blanket Rejected→Valid prohibition.

**R5’s three findings are closed for this design candidate:**

- **Refinement conservation:** [lines 88–114](/home/bline/cbm-work/pi-accounting-continuation-20261114/design/PLAN.md:88) define fixed bases and monotone predicates. The r500/s15 witness becomes 515 on either first identity binding and stays 515 after the second; all-at-once produces the same atoms. Exact-copy subdivision cannot remerge siblings. For retained nonnegative components, `sum(child maxima) ≥ parent maximum`.
- **Unsupported duplicate lows:** [lines 141–171](/home/bline/cbm-work/pi-accounting-continuation-20261114/design/PLAN.md:141) separate physical independence from conservative holds. Uncertain 15M copies support 15M; independently admitted slots support 30M. P15/Q20 plus an ambiguous 15M alternative gives `min(15,15)+min(20,15)=30M`. An unpriceable possible alternative contributes zero support rather than being silently omitted. These conclusions depend on the stated slot guarantees and complete admitted-or-candidate scope inventory.
- **Weak auxiliary linkage:** [lines 128–135](/home/bline/cbm-work/pi-accounting-continuation-20261114/design/PLAN.md:128) prohibit observed `n` uniqueness, RS attachment and completeness labels from establishing release authority. The pinned producer confirms increment/decrement reuse of `accepted` and threaded handlers. Its auxiliary records do not establish the stronger live linkage.

The composed boundaries also hold at the proposed declaration level:

- **Eligibility invalidation preserves known exposure.** Under [lines 175–187](/home/bline/cbm-work/pi-accounting-continuation-20261114/design/PLAN.md:175), restoring original/refined holds retains the previously covered charges and priced context components. Their summed maxima cannot fall below the former resolved maximum; invalid eligibility restores unresolvedness. This requires the specified retention, including V8’s maximum over valued and partially priced highs.
- **Finite positives and unresolved anchors remain distinct.** The pinned multiidentity primary example supports low20M/Exact80M. It has no auxiliary anchor requiring discharge. VC07’s separate AtLeast0 anchor changes the aggregate tag without changing its numeric 15M.
- **Logical validity does not follow hold multiplicity.** Distinct reservation/settlement evidence determines validity; atomization determines conservative ownership. S forwards C’s witness and cannot construct a competing decision.
- **Supported lows are scope-specific.** Candidate ownership supplies alternatives, not positive owned credits. Recomputing within each rollup avoids summing uncertain child lows. Exclusion invalidation and contradictory-frame handling are necessary parts of this contract.

All nine inherited obligations remain implementation and qualification requirements: correlation conservation; local lows15/15/20/60; lawful orphan/refusal applicability; actual external wire; removal of writable S constructors; semantic dependency invalidation; malformed model7; evidence-based cardinality; and Draft2020-12/Rust agreement.

I directly confirmed the material consumer defects. In particular, [Python lines 438–473](/home/bline/cbm-work/p137-s1r7/s1/validator/validate_corpus.py:438) normalize reservation JSON and force rejected/conflicting lows to zero; its evaluation path also sums attempt lows. The proposed actual-entry-point qualification must cover these consumers. Rust-only success cannot discharge them. The schema’s Stage/Boundary mismatch, projection restrictions, dependency paths and independent-axis gaps likewise remain unresolved in immutable R7.

**Admissible implementation boundary:** a new isolated subject containing declarations, exact serialization/schema/rendering, structural checks, immutable witness forwarding, and independently authored bounded raw assertions, with actual Python consumer repair or explicitly registered equivalent replacement. Preserve independent expected arithmetic, raw inputs, historical failures and required qualification/reviews. No reducer, interpreter, solver, production adapter, workspace integration or S2a entry is admitted.

Finite assertions can establish their named bounded claims within D3. The candidate truthfully disclaims full raw-to-output conformance and live provenance; I found no requirement here that makes forbidden broader implementation necessary merely to validate those bounded claims.

Evidence scope: complete PLAN, bindings, RESULT, required doctrine, owner policies and prior reviews; focused pinned producer, raw cases, schema, declarations, Rust and Python sections. Graph coverage reported external subjects unavailable; conclusions use direct source fallback. No tests or reducer execution occurred, and no whole-corpus or live qualification is claimed. R5’s host-accounting gap remains preserved and is not an implementation rejection.

Verified reviewer session `01a1d811-93de-7ef0-9cc7-76d34f3a90cd`, observed `gpt-6-astra/xhigh/default`. Read-only; no edits, subagents, other provider calls or outstanding processes. Root remains sole sealing and acceptance owner.
