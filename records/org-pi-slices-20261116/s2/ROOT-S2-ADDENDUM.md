# S2 root addendum: R2 admission proposal revised per independent review

Author: Pi root `01a1e56e-be12-7424-977b-c7c7983887d0`, 2026-11-16. Applies the revisions requested in
`s2/review/REVIEW.md` §8 (Fable xhigh, `revise_before_owner`, no P1, 8 P2) to
`s2/design/PROPOSAL.md` (sha256 `4f0c9de0…fa1f9`). The reviewer judged that a root addendum applying §8 needs no
second independent review. This is a proposal for owner decision; it grants no implementation authority.

## Retained from the proposal

Pure candidate derivation (`work-engine-organization`, unchanged); one organizational-authority decision owner
(new pure crate as its home); per-effect admission at the consuming host; owner-attested principal /
vantage-actor / component-binding records with actor-anchor separation (Option A, Stage 0 rev6 §18 with
Amendment 6.1 F4/F5); storage/publication mechanism kept out of R2; production admission keeps refusing.

## Corrections (review §8)

1. **S1 path facts (F-1).** The S1 native path supplies no realization claim (`request(None)`), so no
   `KnownIncompatibility` blocker exists there. Under D3 the frozen world would refuse because the unconditional
   `realization-admission:*` blockers have no discharge. R2 scope therefore includes: the delivery host derives
   the realization description from what it will run and feeds it to preparation.
2. **Second world is mandatory under D3 (F-1).** The v1 vocabulary admits one provider and one confinement class,
   so the only admitting world the unchanged compiler can produce is the frozen requirements without
   `realizationConstraints`. It is a labelled assumption world (the SDK child is not confined; `mutation: none`
   is assumed, not true). The native matrix is rebased onto it; the frozen world becomes the negative (spawn 0,
   consumer invocations 0).
3. **Delivery is an effect boundary (F-2).** Spawning a session and giving it the contract is admitted
   separately from consumption. The host owns the boundary (6.1 F3: reserve → observe-for-reservation → gate →
   regate at effect commit); callers cannot reuse an observation for a later boundary. `run_native` stops being a
   public spawn route without admission. Same-observation/later-epoch is a required negative case.
4. **Oracle (F-3).** Replace the universal paired-control rule with Amendment 6.1 §6's oracle classes (no
   observation → no check outcomes; legitimately co-failing checks; production all-refuse).
5. **Observation-source obligations (F-4).** The contract states what any future source must guarantee: an
   issuance is reported committed only once durable; reported revisions are never retracted and are totally
   ordered; an uncertain revocation keeps fencing; one observation is one state. "No observation while a write
   is in progress" is removed as a mechanism. D6 then approves these as acceptance criteria for the later
   storage decision.
6. **Discharge (F-5).** Preparation exports a closed `BlockerKind`. The admission contract owns a total function
   kind → discharging check(s). The host derives discharges from gate outcomes and refuses unknown kinds. The
   discharge map is an output, never a caller input.
7. **Missing owner decisions (F-6)** added below as O7–O10.
8. **Epoch ledger removed from R2 (F-7).** Consumption single-use and staleness stay with the episode owner
   (`review-episode-core` already refuses a moved revision and a handled transition id). Delivery-boundary
   single-use is recorded as a gap until its lifecycle owner is decided (D7 deferred to R3). The relation of the
   §18 `Principal` to `lifecycle-core`'s `CommandAdmission.principal` is recorded as open.
9. **Gate scope (F-8).** R2 implements O3, O4, O5, O8, O9 and R1–R4 plus the five states; O1, O2, O6, O7 wait for
   the publication mechanism that defines their records.
10. **Slicing (review §7).** Seam first. **R2-S** (refusal seam with assumed discharges): closed `BlockerKind`;
    host-derived realization description; refuse before spawn on `KnownIncompatibility` or undischarged kind,
    re-evaluate before consumption; only discharges are a labelled fixture assumption set, total over kinds;
    second world; native matrix rebased; S1 semantic limit closed with a consumer consequence. Estimate
    $24–33 (unverified). Does not meet R2's exit alone. **R2-A'** (admission core, trimmed per item 9, host-owned
    boundary, replaces assumed discharges for separation/component-binding/authority/realization kinds) follows;
    not estimated; exceeds this objective's remaining budget.

P3 items F-9..F-11 (drop uninhabited production instantiation; property-defined identity types; retire or
decide `Store::admit` delegation; name the three meanings of "admission" apart) are adopted as builder guidance.

## Owner decisions requested

| # | Decision | Root recommendation |
|---|---|---|
| O1 | Organizational authority is the single owner of standing, records and Gate O (home: new pure crate); Gate R and effect admission belong to the consuming host | Yes |
| O2 | Separation mechanism: owner-attested principal / vantage-actor / component-binding records, actor-anchor separation. Consequence: two vantages under one account are never separate (no production separation for same-account service vantages until distinct accounts or a key-anchored kind) | Yes, accepting the consequence |
| O3 | R1's fixture path must refuse delivery and consumption on undischarged blockers / known realization incompatibility (closes S1's semantic limit); native matrix rebased onto a labelled constraint-free second world; frozen world becomes the negative | Yes |
| O4 | Delivery (session spawn + contract exposure) is its own admission boundary, host-owned per 6.1 F3 | Yes |
| O5 | Defer a mechanism-table revision (new `Profile`) | Defer |
| O6 | Keep storage/publication out of R2; approve the four observation-source obligations as acceptance criteria for the later storage decision | Yes |
| O7 | The three unowned mapping rules (`scope-digest-to-subject`, `problem-scope-to-episode-identity`, `consumption-evidence-mapping`): accept that production admission stays unreachable until owners are named | Accept, name owners later |
| O8 | Actual-actor evidence: production R3 compares host observation of the spawned process with the attested binding; fixture uses declared values, labelled | Yes |
| O9 | Accept knowingly the adopted parts of the unaccepted Amendment 6.1 (F3 boundary, F4, F5, §6 oracle classes) | Yes |
| O10 | Epoch ledger out of R2; delivery-boundary single-use recorded as a gap; production home decided in R3 after reading lifecycle owners | Yes |
| O11 | Dispatch R2-S now within this objective (~$36 remaining; estimate $24–33), with Fable xhigh review | Owner's call |
