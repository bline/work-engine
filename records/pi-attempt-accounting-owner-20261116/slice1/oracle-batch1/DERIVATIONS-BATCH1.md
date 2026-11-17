# Batch 1 authored derivations (8 historical cases, 26 scopes)

Author: the independent oracle worker for slice1 oracle-batch1 (owner/root session `01a1e569…`). Standing: an authored raw-policy argument, not consumer qualification. Root admits, joins and accepts.

Exact SourceRefs (byte spans, token and line digests, lexemes) are in `dossier/<case>.json`. They were extracted by `tools/sourceref.py`, a navigation scanner that computes nothing. All 13 raw line spans and digests, and every case and canonical-input digest, independently matched the derivation-stage RAW-ORIGINS/LEDGER navigation (0 mismatches).

Supplied outputs were read only after every derivation was written (`AGREEMENT.json`). One exception was accidental and is disclosed in RESULT.md.

## Shared premises and arguments

**Policy.** I used:
- base plan `43d0e7c8` (BASE);
- amendments `c8376f7a` (AMD), `90b6308e` (R3), `3f79f910` (EPR) and `727ad397` (LOWT);
- ROOT-POLICY-R5 `57303fdb`;
- ROOT-ADOPTION-R6 and ROOT-POLICY-R6 `3dbe5356`, both cited by canonical POLICY-R6 and by the declared grants;
- subject `spec/ck.json` (C–K canonical: MONEY, EVIDENCE, SETTLEMENT, VALUATION, USAGE-PRESENCE, X-DECISION, X-COMPONENTS, BASIS-TIE, LOGICAL-PRIMARY, WHOLE-COVERAGE, F-PROJECTION, SUPPORTED-LOW, R-FINANCIAL-PROVISIONAL, CONTINUITY, ATOMIZATION);
- subject `contract/CONTRACT.md` (§2.2, §2.3, §10, §11, §12, Revision6 gate trust boundary).

Hash drift: `/home/bline/cbm-work/p134-module-a/ARCHITECTURE-GUARD.md` now hashes to `507637b9…`, not the bound `2c4063d3…`. It is append-only and no claim here depends on it.

**Money.** Producer lexemes convert exactly as decimal × 10^9 nano. For example `0.4` → 400000000, `0.015` → 15000000, `1.0` → 1000000000, `0.8` → 800000000. Pinned rates are in USD per million tokens, so 1 USD/M = 1000 nano per token:
- gpt-6-astra: input 10, output 50;
- claude-fable-5-2: input 8;
- claude-sonnet-5: input 2.

`inputs.rates` digests equal the pinned files `c0bdfd66…` and `8a286433…`. Arithmetic is single products and sums. No reducer was used.

**PREM-SCOPE: the decisive unmet premise (root).** BASE §3 maps a usable dollar rid to `TicketClaim(scope,rid)` only "in an expressly admitted immutable initial reference scope". Otherwise each origin is `OriginBase(origin)`. Without an admitted key, a reservation and a settlement join a logical primary group only through an admitted exact link (BASE §3; LOGICAL-PRIMARY).

Every dollar case has exactly one declared scope, in its bundle InputProof: conditional_fixture, dollar_ticket, same_origin_only, covering every origin of `S1/turn-1`. Its authority_ref is `ROOT-ADOPTION-R6.json` (P9, A06, V01, VC08) or `ROOT-POLICY-R6.md@3dbe5356` (P130-R07, P132-FG, P132-CP). I read both documents. Neither admits per-case scope declarations. BASE §2 says "Current corpus scope declarations remain conditional as root selected", but the admission channel is external to the payload. I therefore record PREM-SCOPE as **RequiredUnmet(root)** in each case.

I separately checked each declared payload against the raw bytes (NS2):
- namespace: dollar_ticket matches raw `rid`/`usd.rid` = 1 on API-lane origins;
- extent: the covered origins equal the complete source inventory;
- duplicate rule: same_origin_only is adequate because no two origins share bytes.

So one root decision, admitting these raw-consistent conditional declarations, converts every `consequence_if_admitted` in this batch into a nominal claim. That decision is reusable across most of the corpus.

**Invariance method.** A claim is DerivedNominal when its consequence is the same under both PREM-SCOPE branches. The branches are: admitted, giving one TicketClaim hold unit; and not admitted, giving OriginBase units with an orphan NoReservation settlement and a reservation-only unit. Origin-level facts are invariant by construction: raw amounts, per-observation valuation, retained components by owning origin, lifecycle, ownership and placement. Unit- and scope-level money is invariant only in some cases, as stated per case.

Common facts:
- No raw identity binding, distinctness fact or scope/duplicate contradiction exists in any case (no `attempt_id`; `response_id` is not a binding), so Atomize = BreakCopies = false.
- `response_model_check`, `model_mismatch`, `seconds`, `session_spent_usd`, `multiplier_floor`, `input_bound` and `bound_basis` are retained raw fields that no cited clause uses to set an amount.

**Commitment and admission arithmetic (CONTRACT §12.3–12.4).**
- Cap = admission `max_usd`.
- A launched current-producer session gives `committed = max(0, cap − session owned high)`, using the known amount of an AtLeast term.
- If the lifecycle is Retired and no gate is qualified, the state is retired_resumable and the kind is AtLeast (A-3, A-5, L-7, §2.3 producer fact).
- `known = owned_closed.high + owned_outstanding.high + predecessor.high + Σcommitted + new`.

**K codes.** Attempt finding codes such as `SettlementRejected` are not one of the seven families. The canonical ck.json does not define the §9 catalogue; it lives in the validator. Claims use `*_must_include` only with canonically named codes: UnresolvedExposure, UnenforcedCommitment, ClosureNotEstablished, PlacementStraddles, NoGateEvidence and IdentityContradiction.

## <a id="p9"></a>P9-claimed-charge-above-reservation

Raw:
- `:1` usd_reserved, rid 1, n 1, `reserved_usd 0.4`, at 06:30:00.
- `:2` request, n 1, status 200, model and response_model gpt-6-astra, tier `default`, terminal `response.completed`, usage 1000/100/0/0, `usd{rid 1, reserved_usd 0.5, charged_usd 0.7, basis usage}`.
- Launch is openai/gpt-6-astra on the api lane at 06:00:10. Retire is at 06:50. Reference time is 07:00.

Valuation (V-1..V-5): one identity, tier ×1, splits known 0/0, complete. 1000×10000 + 100×50000 = 15,000,000.

Origin-local checks all pass: producer price 15M, charge 700M ≥ 15M − 501. The reserved/charged mismatch needs a unique-reservation witness, so it sits in the grouped claim.

Retained components (BASE §3, §6; AMD P9-RETAINED-CHARGE):
- reservation 400M at `:1`;
- charge 700M at `:2`;
- usage high 15M at `:2`;
- claimed-reserved 500M at `:2` is a validity premise only.

In every branch the 700M charge stays with `:2`, and the scope high is AtLeast and ≥ 700M (R8-FIN-01, direct).

If PREM-SCOPE is admitted:
- C-validity: Rejected[ReservedMismatch], because condition 4 compares 500000000 with 400000000.
- Outstanding obligation, reservation_bound, NotUsageSettlement, terminal, forwarded.
- X-BRANCH-025 gives AtLeast(max(400, 700, 15)) = AtLeast700M with basis proxy_charge, and low 15M usage_valuation.
- Commitment a1: retired_resumable, AtLeast(1000M − 700M) = 300M.
- Scope lows: owned_outstanding has U = Seed = {`:2`}, single_floor 15M, selected 15M, single_ambiguous. owned_closed is empty.

If not admitted (sketch): there are two units. The reservation-only unit is Outstanding at AtLeast400M. The orphan settlement is NoReservation with a none obligation (closed line) at AtLeast700M, low 15M. The low moves to owned_closed, and owned spend of 1100M exceeds the cap (CommitmentInvalid).

Invariant in both branches:
- Lifecycle is retired 06:50, clock trusted.
- Owned by o via the a1 launched row.
- Inside the open window: closed_at is null and there is no close entry.
- Status not_established, provisional true.
- Admission refused: known is 1100M admitted and ≥ 1200M unadmitted, both above 1000M.
- Continuity does not apply. The result is a function of the final facts, so no discovery route lowers 700M.

## <a id="a06"></a>A06-reservation-usage-null-status-no-id

Raw:
- `:1` reserved 0.25.
- `:2` request with explicit null `http_status`, `terminal` and `response_id`; usage 1000/100/0/0; `usd{reserved 0.25, charged 0.25, basis reservation}`.

Valuation: valued, but incomplete because status and terminal are null (V-5). Low = high = 15M. The L4 rule keeps that low despite rejection (BASE §5 table; R8-RAW-DERIVATIONS#A06, reused after re-derivation).

The origin-local failure is StatusType: a reservation-basis settlement on a request row needs an integer status, and the field is null (SETTLEMENT condition 7). This holds in both branches.

If PREM-SCOPE is admitted:
- Rejected[StatusType]; KeepAll and reserved-equality both pass (250M = 250M).
- Outstanding, reservation_bound, NotUsageSettlement.
- X-BRANCH-025 gives AtLeast250M. Reservation and charge tie, so BASIS-TIE picks proxy_reservation. Low 15M.
- Commitment AtLeast750M.
- owned_outstanding low 15M.

If not admitted: the low moves to owned_closed on the orphan unit.

Invariant: refused in both branches, with known 1100M each way.

## <a id="v01"></a>V01-usage-settlement-other-response-model

Raw:
- Launch anthropic/claude-fable-5-2.
- `:1` reserved 0.3.
- `:2` provider anthropic, status 200, `message_stop`, tier `standard`, model claude-fable-5-2, response_model claude-sonnet-5; usage input 10000 with cache_read, cache_creation and output all 0; ephemeral splits absent (zero); charged 0.08, basis usage.

Identity set (MC-1..MC-3) is {fable, sonnet}, both priced. Valuation is complete over established identity and tier sets: low = sonnet 10000×2000 = 20M, high = fable 10000×8000 = 80M (V-2).

Local checks: the request model equals pinned, so there is no ModelMismatch (condition 8 compares the request identity). Producer price is 80M and the charge is 80M, so it passes. Complete, fully priced multiple identities stay finite, with downstream IdentityContradiction blocking (USAGE-PRESENCE; BASE §8; R8-IDENTITY-01, direct).

If PREM-SCOPE is admitted:
- Valid(usage), UsageBound eligible.
- Accepted budgeted coverage: units `:1`+`:2`, valuation usage {`:2`}, no anchors.
- Closed by `:2`, settlement_amount.
- X-BRANCH-023 gives Exact(max(80, 80)) = Exact80M with basis usage_valuation (tie). Low 20M.
- Commitment AtLeast920M.

Invariant:
- The owned_closed low of 20M with U = Seed = {`:2`}, because the usage unit is on the closed line in both branches.
- The owned_outstanding low of 0.
- Commitment kind AtLeast, so provisional true and not_established.
- Refused, with known 1100M either way.

## <a id="vc08"></a>VC08-subscription-measured-request

Raw:
- Launch openai-codex/gpt-6-astra on the subscription lane.
- `:1` `subscription_proxy_admitted`, rid 1.
- `:2` subscription request: provider openai-codex, auth_lane subscription, billing and quota unknown, `subscription_proxy{rid 1, cost_proxy_usd 0.015, measurement api_equivalent_cost_proxy}`, no `usd`.

Both raw references are in the **admission namespace**. The only declared contract is dollar_ticket, whose payload text promises a usable dollar rid. It is inapplicable (BASE §3; AMD NAMESPACE-APPLICATION; R8-NAMESPACE-01, direct).

The admission-namespace grant is RequiredUnmet (root, Q-NAMESPACE). That premise only decides whether there are one or two codex units. In every branch the result is the same:
- Clean subscription lane, so C-KEY subscription and X-BRANCH-017: pool codex, billing unknown, no money.
- cost_proxy is CONTRACT §2.2 mechanism 2, not dollar evidence.
- Obligation none{subscription}.
- Replacement ineligible/NoReservation. This was a self-correction made after derivation, for a type reason: Replacement has only eligible and ineligible variants, and F-REPLACEMENT-1 applies.

Objective o has no admissions, so both origins are Unadmitted with candidate o (O-6, O-7), codex only (O-5). Codex uncertainty does not block pi_api admission (INVARIANTS T5). o is established, provisional false, with exact-0 lines and no commitments. Admit: known 100M ≤ 1000M.

Scope U is empty because codex usage is not a PiApi financial observation.

## <a id="p130-r07"></a>P130-R07-closed-unresolved-predecessor-constrains-successor

Raw:
- `:1` reserved 0.5.
- `:2` upstream_error, n 1, `usd{reserved 0.5, charged 0.5, basis reservation}`, with no status and no usage.
- Retire 06:38.
- Objective p: a1 launched for c1, closed_at 06:40 with **no close entry**, so ClosureNotEstablished (§12.4) and the window is treated as open (P-1).
- Objective s: predecessor p, empty tree, no admissions.

Local checks pass: an absent status is accepted on upstream_error, and the basis is reservation. Valuation is not_reported because there is no usage. No usage observation exists anywhere, so all four scope lows are 0 (invariant). s owns nothing.

If PREM-SCOPE is admitted:
- Valid(upstream_error). Closed by `:2`, reservation_confirmed, NotUsageSettlement, forwarding unknown.
- X-BRANCH-027 gives AtLeast500M with basis proxy_reservation (tie).
- p commitment a1 retired_resumable, AtLeast500M.
- s predecessor_obligations = AtLeast(500M unresolved attempt + 500M open commitment) = AtLeast1000M (§12.2).

Invariant:
- Placement inside: the window is open and all stamps are ≥ 06:00.
- p not_established with ClosureNotEstablished and UnenforcedCommitment.
- s not_established and provisional, because the predecessor line is AtLeast in both branches.
- Admission to s refused: known 1100M either way, above the 1000M limit.

## <a id="p132-fg"></a>P132-FG-VC15-gated (future_qualified_gate)

Raw:
- `:1` reserved 0.5.
- `:2` complete request, 1000/100, charged 0.015, basis usage.
- Retire 06:38.
- p: authority `wev-admission-authority-1`, a1 with gate g-S1, closed_at 06:40 with no `closure_record`, so ClosureNotEstablished under the future profile.
- s: predecessor p.

Gate g-S1 qualifies, conditionally, on test_mock:
- Identity triple: the objective, gate and witness authority IDs are each a present, nonempty string, and they are equal (R8-GATE-IDENTITY-01, direct).
- Kind is vertical_admission_authority and writer is authority (G-1).
- Session c1 matches (G-2). Scope is session (G-3).
- effective_from 06:00:05 is no later than the launched row at 06:00:10, and the end is open (G-4).
- new_forwarding, retries and reopening are enforced, which is what release needs (G-5). Receipts are present (G-6). No reopen (G-7).
- The trusted fixture `payload_sha256` `b38088ca…` equals SHA-256 of the canonical compact sorted JSON of `inputs.gates[0]` (Revision6; P5).

The commitment is `retired`, Exact(0). That holds in both branches, because gate release does not depend on owned money. No live qualification is claimed.

If PREM-SCOPE is admitted:
- Valid, eligible, accepted coverage, Exact15M usage_valuation, low 15M.
- p provisional false.
- s: predecessor exact 0, established, admit at known 100M, conditional on test_mock.

Invariant: the p owned_closed low of 15M, the other lows of 0, p not_established (ClosureNotEstablished), and placement inside.

## <a id="p132-cp"></a>P132-CP-closed-predecessor-outstanding

Raw: a single origin `:1`, reserved 0.5, at 06:30. No retire, so the lifecycle is open.

There are two evaluations, request-successor and request-predecessor, with identical objectives:
- p: a1 launched; `close{at 06:40, …}` with closed_at 06:40. That is an established closure under current_producer: at == closed_at, within [06:00, 07:00], no later admission row, a single close entry.
- s: predecessor p, limit 2000M.

The single origin gives the same hold under both PREM-SCOPE branches. So everything is DerivedNominal except the namespace admission itself, which does not decide anything here:
- Settlement absent, Outstanding, reservation_bound, not_terminal, forwarding unknown.
- X-BRANCH-009 gives AtLeast500M proxy_reservation, low 0.
- Placement: the interval is [06:30, Open) against the window [06:00, 06:40], so **straddles_closure**. p keeps ownership (P-4) and gets reason PlacementStraddles (R8-PLACEMENT-01, direct).
- Commitment launched_open, AtLeast500M.
- s predecessor_obligations AtLeast1000M; s not_established and provisional.
- request-successor: known 1100M ≤ 2000M, so the decision is not_established (step 4).
- request-predecessor: the established closure gives refused (step 1).
- The p owned_outstanding high is AtLeast500M with low 0. Every other scope is empty. Results are identical across both evaluations (INV-F7).

## <a id="n07"></a>N07-control-retired-legacy

The turn-1 log has zero lines and final_newline true, so there are no origins. Retire is at 06:20. The legacy `role_host_objective_admission` row admits session c1 for o with `max_usd 0.8`.

The namespace, primary and valuation families do not apply: there are no origins. That is a derived outcome.

Commitment legacy:c1:
- O-1 treats the legacy row as the session admission record. The MONEY consequence (Q10, OQ-1) and the §2.3 producer fact keep an admitted current-producer session resumable after retirement.
- So the state is retired_resumable, committed max(0, 800M − 0) = 800M, AtLeast, NoGateEvidence.
- An alternative pending-like reading (A-2: committed in full) gives the same 800M AtLeast. Only the state label depends on the reading, and that is disclosed as a residual risk.

o is not_established (UnenforcedCommitment) and provisional. Admission refused: 0 + 800M + 500M = 1300M > 1000M.
