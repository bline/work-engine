# Independent review, Module A: financial semantics of the Pi attempt-accounting contract (single response, no tools)

You are an independent adversarial reviewer of a **design**: the financial part of a normative accounting contract,
written before any implementation. A pure Rust attempt reducer (slice S2a) may be built from its financial stages only
if they survive your review. Other reviewers will examine lifecycle, attribution and admission (Module B) and the corpus
validator (Module C). You review **financial semantics** and the financial interfaces to those stages.

Judge the documents, producer source and raw evidence alone. Comments, labels, notes and the decision register's
"applied as" and "status" columns are the author's claims, not evidence. You are not given the author's reports, any
earlier review, or any validator result. Passing tests would not be evidence of correctness in any case. No verdict is
expected of you in either direction.

You have **no tools** and this is the **only request**. Produce the complete review in this one response and keep
within the output contract below, so that it completes. If your examination of an area is materially incomplete, say
so for that area rather than implying full qualification.

## The review set

14 files follow this brief. Each sits between
`===== BEGIN FILE <path> | sha256 <digest> | <bytes> bytes | <lines> lines | encoding <numbered|verbatim> =====` and
`===== END FILE <path> =====`. The set ends with `===== END OF REVIEW SET: <n> files =====` (n = 14). If any END
line or the final line is missing, say so and return `INCOMPLETE`.

Numbered files show `NNNNN| ` before each line; cite them as `path:line`. Excerpt files carry the producer's original
line numbers; cite those. Cite a corpus case as `case:<case_id>`. Labels such as `R5-01` and `D4` are the author's
identifiers for rules and decisions; they assert nothing.

Files:

- `contract/CONTRACT.md`: NORMATIVE contract 1.4-s1 (scope: stages E-K and the financial interfaces of 2.3, 12.3, 12.4) (65859 bytes, 759 lines)
- `contract/TRANSITIONS.md`: NORMATIVE factorised transition tables (14970 bytes, 184 lines)
- `contract/INVARIANTS.md`: NORMATIVE executable invariants (17016 bytes, 70 lines)
- `contract/MONEY.md`: NORMATIVE money precision and rounding (4801 bytes, 70 lines)
- `contract/RESERVATION-BOUNDS.md`: NORMATIVE analysis: reservations and caps as bounds (4843 bytes, 50 lines)
- `contract/DECISION-REGISTER.md`: owner decisions (proposals to challenge) (19654 bytes, 139 lines)
- `contract/S2A-ENTRY-CRITERIA.md`: proposed S2a scope and gates (4754 bytes, 68 lines)
- `corpus/SCHEMA.md`: corpus schema (7969 bytes, 88 lines)
- `corpus/sample-cases.jsonl`: 56 corpus cases (author narrative and labels stripped), one compact JSON object per line (255195 bytes, 56 lines)
- `producer/pi_container.py.excerpts`: deployed proxy excerpts (release 92a03789), original line numbers (55157 bytes, 857 lines)
- `producer/role_host.py.excerpts`: deployed launcher excerpts (release 92a03789), original line numbers (25857 bytes, 370 lines)
- `producer/scope_budget.py.excerpts`: deployed objective record tool excerpt (release 92a03789) (968 bytes, 19 lines)
- `producer/openai_api_rates.json`: published OpenAI rates (736 bytes, 14 lines)
- `producer/anthropic_api_rates.json`: published Anthropic rates (1179 bytes, 57 lines)

**Authority of the files:**
- **Normative:** `contract/CONTRACT.md` (`1.4-s1`) together with `TRANSITIONS.md`, `INVARIANTS.md`, `MONEY.md` and
  `RESERVATION-BOUNDS.md`. Your scope is §1–§9 and §13 (stages E to K), plus how §2.3, §12.3 and §12.4 use money.
- **Owner decisions:** `contract/DECISION-REGISTER.md`. Treat them as proposals to challenge. Report any that are
  unsound, inconsistent with each other or with the producer, or whose consequence the contract states wrongly.
- **Producer facts:** the excerpts of `pi_container.py`, `role_host.py` and `scope_budget.py` (deployed release
  `92a03789`) and the rate files are authoritative. Where the contract and the producer disagree, say so.
- **Corpus:** `corpus/sample-cases.jsonl` holds 56 of the 173 cases, with author narrative and labels
  removed (`notes`, `rationale`, revision logs, `group`, `findings`).
  - Each case carries raw producer rows (`inputs`), the expected financial facts (`expected.attempts`) and
    per-evaluation expectations. Where present, a `historical` block gives the previous reader A1.3.4's outcome, for
    comparison only.
  - Cases have a `profile`: `current_producer` (what release `92a03789` can write) or `future_qualified_gate`
    (synthetic, for the unfinished Rust vertical, with explicit gate evidence).
  - Unsampled cases are not shown; do not claim to have verified them.
- **Schema:** `corpus/SCHEMA.md`. It still lists the removed fields.

## Judgment A: financial-contract verdict

Give ACCEPT, ACCEPT_WITH_RESERVATIONS or REJECT. Examine at least these areas, each against the contract text, the raw
rows and the producer source:

1. **Settlement validity and replacement eligibility.** Evidence and state types; correlation (key join versus
   auxiliary rows); the `Valid` conditions (§5); obligation and `amount_source`; replacement. Does a valid settlement
   always justify releasing or replacing the reservation?
2. **Reservation retention and release.** Can rejection, uncertainty, a diagnostic or a pricing failure ever cause a
   monetary fallback or lower an amount?
3. **Incomplete and unpriceable usage.** Challenge specifically whether **any** path lets an incomplete valuation
   reduce a retained reservation, or turns unknown additional exposure into a finite amount.
4. **Error-status settlements with supported usage.** Are proxy charge, supported usage and unknown billing kept
   apart? Is such a row producer-writable, and is any synthetic case labelled as synthetic?
5. **Conservative provisional exposure versus provider billing.** Is the measure of every bound stated (§2.1, §8,
   F3, X-2)? Are reservations, proxy charges, declared caps, cost proxies and enforced billing bounds kept apart (§2.2)?
6. **Exact nano-USD arithmetic and rounding** (§2, `MONEY.md`).
7. **Valuation totality.** Missing, null, zero, valid and malformed cache counts; mixed observations (V-4, V-8);
   model identity by explicit rule rather than matching (MC-1 to MC-3); service tiers.
8. **Conflicting reservations and attempt identities.**
9. **Status validity by settlement path and producer event type**, including the corpus expectations for malformed
   and missing statuses, and producer-reachability labels.
10. **Monetary independence from diagnostics, and from temporal attribution.** No diagnostic, lifecycle or time
    placement may change an attempt's money.
11. **The known total used for refusal** (§12.4 step 3). It must be defined over financial facts, without
    double-counting a reservation and its settlement.
12. **Profiles.** Does any `current_producer` case rely on a gate? Does any future case pass off synthetic evidence as
    producer evidence?

**Three architectural challenges. Answer each explicitly:**
- **Exposure versus verified billing.** A complete, priceable usage settlement may establish an exact amount in the
  provisional accounting measure. It must not thereby establish that the provider invoice is complete or final. Can the
  types, the `billing` fact or any downstream consumer (rollup, refusal, admission) conflate the two?
- **Authenticated billing bounds.** A numerical gate bound and a binding digest do not by themselves establish
  authenticated authority. Does the contract clearly require a qualified verification result from the responsible
  future admission authority before a gate can change commitment accounting? Name any synthetic positive case that
  would admit on self-asserted evidence alone.
- **Conflicting reservation evidence.** Is using the largest independently supported reservation amount, with
  blocking uncertainty, correct for correlated evidence? Can independently distinct attempts ever be silently merged?
  Can duplicate evidence be double-counted?

## Judgment B: raw-evidence derivation (required)

For each case below, **derive the settlement decision, obligation, replacement, valuation and exposure yourself**. Use
the raw producer rows (`inputs.sessions[].logs`), the rate files and the contract's rules. Then compare your result
with `expected.attempts`. Internal consistency of the expected record is not proof that it is correct.

- `VC01-openai-usage-settlement`
- `VC02-anthropic-usage-settlement`
- `P130-R02-rounded-up-charge-admitted`
- `VC04-upstream-error-keeps-reservation`
- `S02-control-error-status-after-reservation`
- `P133-A02-error-status-with-priceable-usage`
- `U08-missing-observed-tier`
- `P130-R01-usage-incomplete-terminal`
- `P133-A01-incomplete-usage-above-reservation`
- `P2-usage-settlement-unpriced-identity-keeps-charge`
- `N01-identity-refusal-dearer-served`
- `Ha-inconsistent-cache-split-keeps-supported-low`
- `N06-null-cache-counts`
- `P133-A04-cache-write-absent-legacy`
- `P133-A04-cache-counts-both-absent`
- `P133-A04-cached-malformed`
- `P133-A04-mixed-observations`
- `P133-A05-model-case-variant`
- `S06-string-http-status`
- `A06-reservation-usage-null-status-no-id`
- `P133-A06-usage-settlement-status-absent`
- `P133-A06-error-status-null`
- `R04-reused-reservation-id`
- `P133-A09-conflicting-reservations-with-settlement`

The list mixes legitimate successful settlements (OpenAI and Anthropic) with malformed and uncertain cases. Check:
- usage completeness and monetary replacement;
- error-status retention;
- missing OpenAI cache counts and mixed observations;
- Anthropic pricing;
- model-name canonicalisation;
- conflicting reservations;
- status validity by event type;
- usage-backed low bounds and conservative high bounds.

Report **every** disagreement with your derivation. Where the rate files or rules do not determine an outcome, say
`not determinable` rather than guessing.

## Judgment C: S2a financial readiness

Using `S2A-ENTRY-CRITERIA.md`: can the financial core of a pure Rust attempt reducer (stages C to K) be implemented
without making a new normative decision? List missing preconditions, undefined state combinations, unsupported
monetary assumptions and code-affecting ambiguities.

**Separate** two kinds of blocker:
- those that block S2a's immediate financial core;
- those that only affect later execution-gate, attribution and admission integration.

## Structural assessment

For each finding, say whether a local correction to one rule or table fixes it (`local`), or whether it shows that a
structural element is wrong (`structural`). Structural elements are the measures, the type split, stage boundaries or
the evidence model. Then give an overall `architecture_assessment`: `sound`, `local_corrections` or
`redesign_needed`, with the reason.

## Severity

| Severity | Meaning |
|---|---|
| **P1** | A financial design defect that would make a correct implementation lower conservative exposure, double-count, release without justification, turn unknown into a finite bound, let a diagnostic or a time placement change money, conflate provisional exposure with established billing, or be unable to proceed. **Blocks S2a.** |
| **P2** | A correctness or completeness defect without those effects, or a spurious fail-closed. |
| **P3** | Minor: wording, or an ambiguity that does not block. |

`verified` means traced with citations and a concrete input; anything else is `hypothetical`.

## Output contract (keep to it so the response completes)

**Part 1:** one fenced `json` block, in this order:

```json
{
  "review_set_complete": true,
  "financial_contract_verdict": "ACCEPT | ACCEPT_WITH_RESERVATIONS | REJECT | INCOMPLETE",
  "s2a_financial_readiness": "READY | NOT_READY | INCOMPLETE",
  "architecture_assessment": {"result": "sound | local_corrections | redesign_needed", "reason": "..."},
  "examination": [{"area": 1, "result": "sound | defect | not_established", "basis": "file:line, case:<id>"}],
  "architectural_challenges": [{"challenge": "exposure_vs_billing | authenticated_bounds | conflicting_reservations", "result": "sound | defect | not_established", "basis": "..."}],
  "findings": [{"id": "A-01", "severity": "P1|P2|P3", "status": "verified|hypothetical", "kind": "contract|corpus|decision|producer",
                "class": "local|structural", "location": "file:line", "title": "...",
                "counterexample": "raw rows -> rule -> wrong outcome -> correct outcome", "smallest_correction": "..."}],
  "derivations": [{"case": "<id>", "agrees": true, "derived": "decision; obligation; replacement; valuation low/high; exposure low/high (kind)", "disagreement": "..."}],
  "s2a_blockers": {"financial_core": ["..."], "later_integration": ["..."]},
  "minimum_required_revisions": ["..."],
  "inadequately_examined": ["..."]
}
```

- Give `examination` for all 12 areas.
- Order findings P1 first; at most 15 findings.
- Keep each derivation and each list item to one line.

**Part 2:** at most 400 words: the reasoning behind both verdicts and the architecture assessment, and what remains
uncertain.

# Review set

===== BEGIN FILE contract/CONTRACT.md | sha256 8c4f89ca102278ef64c009d215abf639682ef83b9f21999bf36af4675e2e3de3 | 65859 bytes | 759 lines | encoding numbered =====
    1| # Pi attempt-accounting contract, S1 (normative)
    2| 
    3| **Contract version:** `pi-accounting-contract/1.4-s1` (revision 5, P1.33: revision items R5-01 to R5-09,
    4| of which R5-01, R5-02, R5-03 and R5-06 are owner dispositions; see `DECISION-REGISTER.md`). Revision 4 `1.3-s1` (P1.32), revision 3
    5| `1.2-s1` (P1.30), revision 2 `1.1-s1` (P1.29) and revision 1 `1.0-s1` (P1.28) are preserved unchanged.
    6| **Status:** proposed normative contract, not reviewed in this revision and not implemented.
    7| 
    8| **Producer under contract:**
    9| - release `92a03789`: `pi_container.py` `78083734…`, `role_host.py` `cbca1bc9…`;
   10| - rate authority: `openai_api_rates.json` `c0bdfd66…` and `anthropic_api_rates.json` `8a286433…`.
   11| 
   12| **Owner decisions:** D1–D10 (`DECISION-REGISTER.md`) are binding inputs.
   13| 
   14| **Not normative:**
   15| - the deployed reader (`ab2a0f84`, development tooling, unchanged);
   16| - A1.3.4 (`086bdec1`, a historical comparison baseline only).
   17| 
   18| Key words: **MUST** and **MUST NOT** are requirements. "Blocking" means the finding makes every affected objective's
   19| admission facts `NotEstablished` (§12).
   20| 
   21| ---
   22| 
   23| ## 1. Scope and stages
   24| 
   25| | Stage | Where | Input | Output | May read |
   26| |---|---|---|---|---|
   27| | **E** evidence adaptation | outside the core (D3) | bytes of every log a turn record names, role_host ledger rows, objective records, admission journals, rate files | `EvidenceSet` (§3) | files only |
   28| | **C** correlation | core | `EvidenceSet` | `AttemptGroup`s plus `NonAttemptEvidence` (§4) | E |
   29| | **S** settlement | core | `AttemptGroup`, `LaunchFacts`, pricing primitives (§7.1) | `SettlementDecision` plus `Obligation` (§5, §6) | C |
   30| | **V** valuation | core | `AttemptGroup`, `LaunchFacts`, pricing primitives | `Valuation` (§7) | C |
   31| | **X** exposure | core | S plus V outputs, lane decision | `Exposure` (§8) | C, S, V |
   32| | **K** classification | core | the complete facts of C to X | `Vec<Uncertainty>`, `Vec<Diagnostic>` (§9) | C, S, V, X |
   33| | **L** lifecycle | core | lifecycle rows of one session | `SessionLifecycle` (§10) | E |
   34| | **T** attribution | core, separate module | X outputs (time facts, attempt keys), L, `AdmissionRecords`, `ObjectiveFacts` | `Ownership` plus `Placement` per attempt (§11) | X, L, K (read-only) |
   35| | **R** rollup | core | T, X, K, L, commitments (§12.3) | `ObjectiveAccounting` (§12) | all above, read-only |
   36| | admission decision | outside the core | closure state, R and the requested allocation (§12.4) | admit, refuse or not established | R |
   37| 
   38| **Dependency direction (MUST):** `E → C → S → V → X → K`.
   39| - **Pricing primitives** (§7.1) are leaf functions shared by S (the producer-consistency check) and V (bounds). They
   40|   read only evidence and the rate table.
   41| - **No stage reads a later stage's output.** K's findings are never an input to S, V, X or T's placement
   42|   computation.
   43| - **T and R** may copy blocking findings into their outputs but MUST NOT modify any amount.
   44| 
   45| **Pure and deterministic.** Stages C to R are pure functions. Output is canonical (§13), and equal inputs give
   46| byte-identical outputs. The core does no I/O, reads no clock (the reference time is an input) and never panics on
   47| producer input.
   48| 
   49| ## 2. Money (D10)
   50| 
   51| - **Canonical amount:** `Nano = i128` nano-USD, with checked arithmetic. Overflow is `Err(Overflow)`, which callers
   52|   MUST treat as blocking `ArithmeticFailure`. Values never wrap or saturate.
   53| - **Rates:** parsed from the rate files' decimal literals, never via binary floats, into nano-USD per token, an exact
   54|   integer for every current rate and multiplier (`MONEY.md`: 163 of 163). A rate or multiplier that is not an exact
   55|   integer in nano-USD per token is `Err(UnsupportedPrecision)`: rejected at load, with the rate table marked
   56|   unusable, so every valuation is `Unpriceable{RateTableUnusable}`.
   57| - **Producer amounts** (`charged_usd`, `reserved_usd`) are decimal literals with at most 6 fractional digits, converted
   58|   exactly. More digits, exponent forms whose value is not an exact nano amount, negative values, booleans, strings,
   59|   NaN or infinities are `Malformed`.
   60| - **Rounding (R-02).** The core never rounds a current amount: valuations are exact integers, and producer amounts
   61|   are taken exactly as logged. The producer's own rounding (`round(x, 6)` USD, `pi_container.py:1018-1019`) moves a
   62|   logged amount up or down from the producer's float value. It can never lower conservative exposure:
   63|   - a logged **charge** enters a high only as a term of `max(charged, usage bound)` (FB-1, §8), so rounding down
   64|     leaves the high at the usage bound or above, and rounding up only raises it;
   65|   - a logged **reservation** is a known amount, never a bound (FB-2), so its rounding creates no finite bound;
   66|   - the supported **low** is a usage valuation, never a producer amount (F11).
   67| 
   68|   If a future rate table needs rounding, every rational amount MUST be rounded once per bound, low by floor and high
   69|   by ceiling (`MONEY.md`).
   70| - **Producer-consistency tolerance.** `T_round = 501` nano. The producer's logged charge differs from its float value
   71|   by at most 500 nano (6-place rounding), and the float value differs from the exact price `p` by under 1.2 nano for
   72|   `p < 10^15` nano (10^6 USD). Both the logged charge and `p` are integers in nano, so their difference is at most 501
   73|   (`MONEY.md`). For `p ≥ 10^15` nano the bound is not proven, and the settlement is `Rejected[ToleranceRange]`.
   74| 
   75| ```text
   76| High  = Exact(Bound) | AtLeast(Bound)    // AtLeast: `amount` is supported; more is unresolved; never a finite maximum (D5)
   77| Bound = { amount: Nano, basis: AmountBasis, evidence: [EvidenceRef] }
   78| AmountBasis = ProxyCharge | ProxyReservation | UsageValuation{identities, tier_multiplier, split}
   79|             | ProviderBilling          // reserved; no evidence source today; never produced in S1
   80|             | None                     // amount 0 with no support (for `low`)
   81| Exposure = { low: Bound, high: High }  // invariant: low.amount ≤ high.amount
   82| ```
   83| 
   84| **Order on `High`:**
   85| - `Exact(a) ≤ Exact(b)` iff a ≤ b;
   86| - `Exact(a) ≤ AtLeast(b)` iff a ≤ b;
   87| - `AtLeast(a) ≤ AtLeast(b)` iff a ≤ b;
   88| - `AtLeast(a) ≤ Exact(b)` is never true.
   89| 
   90| **Sum:**
   91| - `Exact + Exact = Exact`;
   92| - any `AtLeast` term makes the sum `AtLeast` of the known sum.
   93| 
   94| **Bases are never merged:** a sum carries a basis histogram, not one basis.
   95| 
   96| **Evidentiary standard, stated rather than verified:** usage valuations price provider-reported usage at the published
   97| rates. Verified provider billing is unavailable, so every `Exact` high is conditional on that standard.
   98| 
   99| ### 2.1 Finite-bound rule (owner disposition Q2)
  100| 
  101| Four amounts are kept apart and never stand in for one another:
  102| 
  103| | Amount | Basis | Meaning |
  104| |---|---|---|
  105| | reserved | `ProxyReservation` | what the proxy set aside before forwarding |
  106| | conservatively charged | `ProxyCharge` | what the proxy kept against its own session limit |
  107| | provider bill | `ProviderBilling` | established only by bill evidence; **no source exists today, never produced** |
  108| | unknown additional billing | `High::AtLeast` | what remains unresolved |
  109| 
  110| **Rule FB-1 (usage bound; revised for R-01, R-02, R-03).** A high may be `Exact` only when the attempt has a **usage
  111| bound**: the high of a `UsageValuation` of **complete** provider-reported usage (V-5 `complete = true`), over an
  112| established identity set (one identity, or several identities each priced, §8) and an established tier set (V-3).
  113| The stated high is `Exact(max(usage bound, the known amounts §8 adds))`. A proxy charge above the usage bound raises
  114| the stated high and gives it its basis (X-0, for example `ProxyCharge` after upward rounding), but the high is finite
  115| because the usage bound exists, not because of the charge. **Without a usage bound, no known amount makes a high
  116| finite.**
  117| 
  118| **Rule FB-2.** Proxy reservations, proxy charges and declared session caps are known amounts, never upper bounds on
  119| provider billing. A high with no usage bound is `AtLeast(largest known amount)`.
  120| 
  121| **Monetary measures (R5-01, R5-02, owner disposition P1.33).** Six facts are kept apart for every attempt. None stands in
  122| for another:
  123| 
  124| | Fact | Where it is recorded | Measure |
  125| |---|---|---|
  126| | terminal execution | `Execution` (§6.2) | none (a time fact) |
  127| | structurally valid settlement | `SettlementDecision::Valid` (§5) | none (a validity fact) |
  128| | complete and priceable usage | `Valuation` with `complete` and established sets (§7, FB-1) | **usage valuation**: what the reported usage costs at published rates |
  129| | eligibility to replace the reservation | `Replacement` (below) | none (a decision fact) |
  130| | proxy charge and proxy reservation | `settlement.charged`, `R` | **proxy amounts**: what the proxy kept and set aside |
  131| | independently established provider billing | `AmountBasis::ProviderBilling` | never produced in S1 |
  132| 
  133| **Exposure is one measure: conservative provisional exposure**, the amount the admission accounting must hold for the
  134| attempt. Its `low` is supported usage valuation, never a proxy amount. Its `high` is either:
  135| - `Exact(…)`, when the reservation is **replaced** by an eligible usage settlement (FB-1); or
  136| - `AtLeast(known)`, where `known` is the largest of the known components the conservative policy must hold: the
  137|   retained reservation `R`, the proxy charge, and the usage valuation's high or `priced_high`. Each component is a
  138|   lower bound of what the policy holds, with its own basis (X-0). The maximum is taken within this one measure, not
  139|   across measures: provider billing is never claimed, and the usage valuation keeps its own record in `Valuation`.
  140| 
  141| ```text
  142| Replacement = Eligible                                   // Valid(Usage) with a usage bound (FB-1): the reservation is replaced
  143|             | Ineligible{ reason: NotUsageSettlement      // no valid usage settlement (absent, rejected, conflicting, other bases)
  144|                                 | UsageIncomplete          // valued, but not complete (V-5)
  145|                                 | UsageUnpriceable         // tier set or identity set not established, malformed counts
  146|                                 | NoReservation }          // nothing to replace (unbudgeted, orphan, subscription)
  147| ```
  148| 
  149| **A structurally valid settlement is not proof of complete billing.** Only `Eligible` replaces reservation-based
  150| exposure. An ineligible valid settlement closes the attempt in time and keeps the reservation in the conservative
  151| measure (`amount_source = ReservationBound`, §6), and its supported partial usage stays as the low.
  152| 
  153| **Why the reservation is not a bound.** It is the producer's worst case, but its conditions are not all
  154| producer-enforced (`RESERVATION-BOUNDS.md`):
  155| - the output cap is enforced;
  156| - the input bound is a byte estimate, and the producer marks it "assumed, not verified" for inline images;
  157| - the served tier is not enforced: the proxy itself charges a response at whatever tier was served;
  158| - no long-context rule is applied to Anthropic;
  159| - the provider's billing of errored or aborted requests is unknown.
  160| 
  161| **Consequence (Q10, P1.30; OQ-1, P1.32):** under D5, an objective is `NotEstablished` while it owns or may own (as a
  162| candidate) any attempt without a usage bound, or holds any commitment of kind `AtLeast` (§12.3). That covers in-flight,
  163| unsettled, error-status, upstream-error, reservation-basis and rejected attempts, and **every admitted session of the
  164| current producer, retired or not**, because a retired Pi session can be resumed without new objective admission
  165| (§2.3). Only a qualified `ExecutionGate` (§2.3) releases or bounds a session's commitment. No current producer evidence
  166| qualifies.
  167| 
  168| ### 2.2 Spending mechanisms are kept apart (R-04, P1.30; OQ-3, P1.32)
  169| 
  170| | # | Mechanism | Source today | Is it a provider-billing bound? |
  171| |---|---|---|---|
  172| | 1 | **request-count stop** | `--max-requests`: the proxy refuses requests beyond a count | **no**: it limits how many requests are forwarded, not what each costs |
  173| | 2 | **cost-proxy stop** | subscription `--max-usd`: an API-equivalent cost-proxy threshold checked before admission (`pi_container.py` subscription budget) | **no**: one admitted response can overshoot, and subscription billing is unknown |
  174| | 3 | **proxy reservation constraint** | the proxy refuses a new reservation when settled + outstanding + worst case > the session limit (`:984-988`) | **no**: it gates reservations, and a reservation is not a bound (B-2, B-4, B-5, B-6) |
  175| | 4 | **internal accounting commitment** | §12.3 `committed = max(0, cap − owned high)` | an allocation for admission arithmetic, not a bound |
  176| | 5 | **unenforced declared cap** | the admission `max_usd` | **no**: `settle()` charges the served tier without a cap (`:994-1019`), and a retired session can resume under it (§2.3) |
  177| | 6 | **enforced provider-billing bound** | an `ExecutionGate` mechanism `billing_bound` (§2.3) | **yes, only when qualified**: enforcement across forwarding, retries, failures, incomplete streams and settlement must be evidenced. No current producer evidence qualifies |
  178| 
  179| Unknown additional provider exposure stays `High::AtLeast` and commitment kind `AtLeast`, and blocks (D5). Renaming a
  180| mechanism changes no expectation.
  181| 
  182| ### 2.3 Execution gates and capability profiles (OQ-1, owner disposition P1.32)
  183| 
  184| **Producer fact (release `92a03789`).** Retirement does not stop a Pi session from spending. `role_host status`
  185| reports a retired Pi session as `stopped` (`role_host.py:1504`), `PiHost.deliver` refuses only a `running` or
  186| `absent` session (`:1543-1550`), and the new turn starts under the original session cap and prior spend (`:1256`,
  187| `:1305`), without reading the objective admission journal. So, for the current producer:
  188| - retirement and idleness are activity labels, not enforcement;
  189| - outstanding exposure and the ability to resume without admission are **separate facts**: the first is the owned
  190|   lines (§12.1), the second the commitment (§12.3);
  191| - settled past requests do not make a session safe to count as finished;
  192| - a declared cap is not a provider-billing bound (§2.2).
  193| 
  194| **`ExecutionGate` evidence (for the future Rust vertical; no current producer writes it).**
  195| 
  196| ```text
  197| ExecutionGate = { gate_id, authority: { id, kind }, session: SessionId, scope: Session | Request{request_id},
  198|                   effective_from: Stamp, effective_until: Stamp | Open, writer: Authority | SameUidFile | Declaration,
  199|                   mechanisms: { new_forwarding: Mechanism, retries: Mechanism, reopening: Mechanism, billing_bound: Mechanism } }
  200| Mechanism     = Enforced{ receipt: { ref, sha256 } } | Absent
  201| BillingBound  = Enforced{ receipt, bound: { amount_nano, currency, scope, covers: [Activity], binding_sha256 } } | Absent   // mechanism billing_bound
  202| Activity      = forwarding | retries | partial_streams | failures | settlement
  203| ```
  204| 
  205| A gate **qualifies** only if every rule holds (otherwise it is ignored for commitments and its failures are recorded
  206| as `gate.reasons`):
  207| 
  208| | Rule | Statement |
  209| |---|---|
  210| | G-1 Authority | `authority.kind = vertical_admission_authority`, its `id` equals the objective's declared admission authority, and `writer = Authority`. A same-UID-writable file, a declaration, or a plain `gated: true` field is not evidence (`GateWriterNotAuthority`, `GateAuthorityMismatch`) |
  211| | G-2 Identity | `session` equals the session whose commitment is being decided (`GateSessionMismatch`) |
  212| | G-3 Scope | scope `Session`, which covers every request of the session. A `Request` scope covers one request and cannot release or bound a session commitment (`GateScopeInsufficient`) |
  213| | G-4 Period | `effective_from` is no later than the session's admission (`launched` row), and `effective_until` is open or no earlier than the reference time (`GatePeriodInsufficient`) |
  214| | G-5 Mechanisms | **to release a retired session's commitment:** `new_forwarding`, `retries` and `reopening` all `Enforced` (refusing delivery after retirement alone does not prove that an active session cannot create further exposure). **To make an open or pending commitment finite:** `billing_bound` and `retries` `Enforced` (OQ-3), with a bound satisfying G-8. Otherwise `GateMechanismMissing` |
  215| | G-6 Receipts | every `Enforced` mechanism used carries a receipt with a reference and a sha256 (`GateReceiptMissing`) |
  216| | G-8 Billing bound (R5-03, owner disposition P1.33) | to make an open or pending commitment finite, `billing_bound` must carry a **numerical enforced bound**: `amount_nano` a non-negative integer no greater than the admission cap (a bound above, or not comparable with, the authorized cap does not qualify); `currency = USD`; `scope = session` for this session; `covers` all of `forwarding`, `retries`, `partial_streams`, `failures` and `settlement`; and `binding_sha256` equal to the sha256 of the canonical JSON (sorted keys, no spaces) of `{amount_nano, authority_id, covers, currency, effective_from, effective_until, gate_id, session}`. A declared cap, a mechanism flag without a bound, or a same-UID-writable assertion does not qualify (`GateBoundMissing`, `GateBoundExceedsCap`, `GateBoundCurrency`, `GateBoundCoverage`, `GateBoundBinding`). **Authentication limit:** the digest binds the fields to one another and to the receipt. Verifying the authority's signature over them needs key material that S1 does not have, so it is the future vertical's evidence-adapter obligation (stage E) |
  217| | G-7 Consistency | no evidence contradicts the gate: a session whose lifecycle is `reopened` after `effective_from` while `reopening` is claimed `Enforced` makes the gate **contradicted**, blocking `GateContradicted` for the admitting objectives |
  218| 
  219| A qualified gate never changes an attempt's money (stages C to X). It decides only whether a commitment is released
  220| or finite (§12.3), so outstanding exposure still blocks through the owned lines.
  221| 
  222| **Capability profiles (corpus).** Every corpus case declares one:
  223| - `current_producer`: what release `92a03789` can establish. No gate evidence is attached, and objectives use the
  224|   legacy journal and closure records.
  225| - `future_qualified_gate`: synthetic normative cases for the unfinished Rust vertical, with explicit gate and closure
  226|   evidence from a `vertical_admission_authority`. They are `producer_unreachable` today, carry no A1.3.4 historical
  227|   comparison, and are counted separately.
  228| 
  229| ## 3. Evidence (stage E output, the core's input)
  230| 
  231| ```text
  232| EvidenceRef  = { source: SourceId, line: u32, sha256: Digest }          // SourceId = logical path + file sha256
  233| Field<T>     = Present(T) | Null | Absent | Malformed(JsonType)
  234| EvidenceItem = { ref, epoch: ProducerFeatures, kind: EvidenceKind }
  235| EvidenceKind =
  236|     Reservation  { n: Field<u64>, rid: Field<Rid>, reserved: Field<Nano>, at: Field<Stamp> }
  237|   | Settlement   { event: Request|UpstreamError, rid: Field<Rid>, n: Field<u64>, basis: Field<Basis>,
  238|                    charged: Field<Nano>, reserved: Field<Nano>, status: Field<u16>, usage: Field<Usage>,
  239|                    model: Field<String>, response_model: Field<String>, response_id: Field<String>,
  240|                    observed_tier: Field<String>, terminal: Field<String>, model_mismatch: Field<bool>,
  241|                    labels: Labels, at: Field<Stamp> }
  242|   | Request      { same fields as Settlement, no `usd` }                 // unbudgeted API, or subscription
  243|   | UpstreamError{ n, labels, at, detail }                               // no `usd`
  244|   | Admission    { rid: Field<u64>, n: Field<u64>, model: Field<String>, labels, at }
  245|   | Measurement  { attached to its row: rid: Field<u64>, status, cost_proxy }
  246|   | StreamError  { n: Field<u64>, shape: ReadFailure|HeaderWrite, labels, measurement: Option<Measurement>, at }
  247|   | IdentityRefusal { n, reason, authorized: Field<String>, served: Field<String>, response_id, labels, at }
  248|   | Refused      { status, reason, labels, at }
  249|   | Unreadable   { reason: NotJson | NotObject | PartialFinalLine | UnknownEvent(String) }
  250| Labels = { provider: Field<String>, auth_lane: Field<String> }
  251| Basis  = Usage | Reservation | ErrorStatus | Unknown(String)
  252| Usage  = OpenAi{ input, output, cached: Field<u64>, cache_write: Field<u64>, reasoning }
  253|        | Anthropic{ input, cache_read, cache_write, output, w5m: Field<u64>, w1h: Field<u64> }
  254| ```
  255| 
  256| **Adapter obligations (stage E, specified here because the core relies on them):**
  257| - **E-1 Totality.** Every non-blank line of every proxy log named by a turn record is exactly one `EvidenceItem`. A
  258|   line that is not JSON, not an object, or the final line without a newline is `Unreadable`. Nothing is skipped.
  259| - **E-2 Fidelity.** Fields are recorded as written (`Absent`, `Null`, `Malformed`). No defaults are applied in E.
  260| - **E-3 Shape.** `StreamError.shape = ReadFailure` iff the row has a `detail` key, else `HeaderWrite`
  261|   (`pi_container.py:1295`, `:1513`, `:1546`).
  262| - **E-4 Producer features.** `ProducerFeatures` are feature flags detected per session, not dates:
  263|   `usd_accounting`, `cache_write_field`, `anthropic_route`, `subscription_lane`, `cost_proxy`, `attempt_id`.
  264| - **E-5 Inventory facts.** E also emits inventory evidence:
  265|   - turn numbers recorded;
  266|   - logs found on disk but not referenced, copied (byte-identical) or misplaced;
  267|   - missing logs;
  268|   - logs shared by two turns.
  269| 
  270|   These become `InventoryEvidence` with the session scopes that could own them (§9; R-03, A1-03, S-05, V-07).
  271| 
  272| `LaunchFacts` per session: `pinned_model` (`provider/model` on the launch row), `auth_lane`, `parent`, `turns`.
  273| 
  274| **Model canonicalisation (R5-05; rule version `mc-1`).** Pricing identity is never established by fuzzy matching.
  275| - **MC-1 Launch model.** A launch model is `<provider>/<model_id>`, with `<provider>` one of `openai`, `openai-codex` or
  276|   `anthropic`, split at the **first** `/`. That is exactly how the producer derives the proxy's `model_id`
  277|   (`role_host.py:1280`: `opts["model"].split("/", 1)[1]`). The canonical pinned identity is `(provider family,
  278|   model_id)`, where `openai` and `openai-codex` are the OpenAI family. A launch model with no `/`, or with another
  279|   provider, has **no canonical identity**: `ModelUnresolved`.
  280| - **MC-2 Row models.** A proxy row's `model`, `response_model` or refusal `served` value is a bare model id in the
  281|   row's provider family (the `provider` label, or OpenAI when absent). It is used exactly as written. No case folding,
  282|   no prefix stripping, no version or date normalisation.
  283| - **MC-3 Equality and rates.** Two identities are equal iff their family and model id are byte-identical. Rates are
  284|   looked up by exact model id in the family's rate file. A missing rate is `IdentityWithoutRate` (V-6). A
  285|   `ModelUnresolved` pinned identity fails §5 condition 8 (`ModelMismatch`) and is an unpriceable identity (V-1).
  286| 
  287| ## 4. Correlation (stage C)
  288| 
  289| ```text
  290| AttemptKey  = ProducerAttempt{incarnation, seq}      // proposed producer field (D7); none exist today
  291|             | Reservation{source, rid}               // budgeted
  292|             | Admission{source, rid}                 // subscription cost-proxy
  293|             | Row{source, line}                      // unbudgeted request, or an orphan
  294| Correlation = { key_join: Exact | Ambiguous{candidates: [AttemptKey]} | Unattributed,
  295|                 auxiliary: [{ evidence: EvidenceRef, rule: ShapeRule }] }      // RS-1..RS-3 joins (C-5 to C-7)
  296| AttemptGroup = { key, correlation, reservations: [EvidenceItem], settlements: [EvidenceItem],
  297|                  requests, errors, refusals, admissions, measurements, launch: LaunchFacts }
  298| ```
  299| 
  300| **Key join versus auxiliary rows (R-06).** `key_join` is the confidence of the attempt's own key: the
  301| reservation–settlement join by `rid`, or the row itself for `Row` and `Admission` keys. Error and refusal rows joined
  302| by a shape rule are listed in `auxiliary` with their rule. They add evidence (identities, diagnostics, forwarding) but
  303| never close an obligation and never change `key_join`. F1 reads `key_join`.
  304| 
  305| | Rule | Statement |
  306| |---|---|
  307| | C-1 | A `Reservation` with a usable `rid` keys `Reservation{source, rid}`. Byte-identical repeats are one item (diagnostic `RepeatedIdenticalRow`). **Different rows with one `rid` are conflicting reservation evidence (R5-09):** all stay visible in the group, blocking `ReservationConflict` is raised, and the decision follows Table S1 (no settlement: `Absent`; one settlement: `Rejected[ReservationConflict…]`; two or more: `Conflicting`). Their independence is not established, so `R` is the largest of them (X-0), never their sum and never the smaller. **A reservation whose `rid` is absent or malformed** keys `Row{source, line}` with `key_join = Unattributed`. Its decision is `Absent` (no settlement can join it) and its obligation `Outstanding`, with blocking `MalformedEvidence(rid)` and `UnaccountedAttempt`. A malformed `reserved` contributes 0 to R and adds `MalformedEvidence(reserved)` (R-06) |
  308| | C-2 | A `Settlement` joins `Reservation{source, usd.rid}` (`Exact`). An unusable `rid` keys `Row{source, line}` with the settlement only. A settlement with no reservation of that `rid` in its source keys `Row{source, line}` (`Exact`): decision `Rejected[NoReservation…]`, obligation `None{NoReservation}` (R-06) |
  309| | C-3 | An unbudgeted `Request` (API lane, no `usd`) keys `Row{source, line}`, `Exact` (one row, one attempt) |
  310| | C-4 | A subscription `Admission` keys `Admission{source, rid}`. A measured row joins the admission whose `rid` its measurement names, if that admission is earlier in the source (`Exact`). Otherwise it keys `Row` with `Unattributed` |
  311| | C-5 | **Read failure (shape rule RS-1).** A `StreamError` with shape `ReadFailure`, unmeasured, joins the first later `Request` or `UpstreamError` in its source with the same `n` that no other error has claimed, **only if** `n` names exactly one forwarded attempt in the source: at most one `Request` or `UpstreamError` row and at most one distinct reservation `rid` with that `n` |
  312| | C-6 | **Header-write failure (RS-2).** A `StreamError` not joined by C-5 joins a reservation with its `n` that no valid settlement closes and no other error has claimed, under the same uniqueness condition |
  313| | C-7 | **Refusal pairing (RS-3).** An `IdentityRefusal` joins the first later `Request` with the same `n` and the same served model, one to one. Otherwise it joins a unique unsettled reservation with its `n` (RS-2's condition). Otherwise `Row`, `Unattributed` |
  314| | C-8 | **Ambiguity.** An error whose `n` fails the uniqueness condition is its own `Row` attempt with `Ambiguous{candidates}`. An error with a missing or malformed `n` is a `Row` attempt, `Unattributed`. Ambiguity is never resolved by order, closeness or amounts |
  315| | C-9 | `Refused` rows are `NonAttemptEvidence`; they never key, close or value an attempt and carry lane evidence only. A refused row with a **preflight** reason (every reason except `upstream unreachable` and an identity-refusal reason) records a request the proxy did not forward. A refused row with `upstream unreachable` or an identity-refusal reason is the proxy's client-side report **after** an attempt (`pi_container.py:1218`, `:1326`, `:1502`) and proves nothing about forwarding (R-13) |
  316| | C-10 | **Measurement subtypes.** A measurement row is a `Request`, `UpstreamError` or `StreamError` with an attached `Measurement`; the attachment does not change the row's kind |
  317| | C-11 | **Correlation never crosses sources.** A source is one turn's log; `n` and `rid` restart per proxy incarnation |
  318| 
  319| ## 5. Settlement decision (stage S)
  320| 
  321| ```text
  322| SettlementDecision = Absent
  323|                    | Valid{ basis: Usage|Reservation|UpstreamError|ErrorStatus, charged: Nano, settlement: EvidenceRef }
  324|                    | Rejected{ reasons: [RejectReason], claimed: [Nano] }
  325|                    | Conflicting{ settlements: [EvidenceRef], claimed: [Nano] }
  326|                    | NotApplicable          // unbudgeted, subscription, or orphan attempt
  327| ```
  328| 
  329| **`Valid` iff every condition holds, otherwise `Rejected` with every failing reason** (codes in brackets):
  330| 1. Exactly one reservation and exactly one distinct settlement in the group [`ReservationConflict`, `NoReservation`].
  331| 2. The settlement is appended after the reservation [`Order`].
  332| 3. `n` is a positive integer on both (never boolean, string, float or zero) and equal [`AttemptNumber`].
  333| 4. `charged` and `reserved` are well-formed amounts [`AmountType`], and `reserved` equals the reservation's amount
  334|    exactly [`ReservedMismatch`].
  335| 5. `basis` is present and known [`BasisMissing`, `BasisUnknown`]. An `UpstreamError` requires `Reservation`.
  336| 6. `Reservation` or `UpstreamError`: charged equals the reservation exactly [`KeepAll`].
  337| 7. **Status, by settlement path (R5-06, owner disposition P1.33).** A missing status is neither an automatic failure nor
  338|    a universal exemption. It is accepted only where the producer writes no status:
  339| 
  340|    | Path | Settlement row | Status required | Absent or null | Producer evidence |
  341|    |---|---|---|---|---|
  342|    | `Usage` | `request` | integer 200 | `StatusType` | every request row carries `http_status: resp.status` (`pi_container.py:1549`, unchanged since `385f3174`; 1,588 of 1,588 real request rows are integers) |
  343|    | `ErrorStatus` | `request` | integer, not 200 | `StatusType` | as above |
  344|    | `Reservation` | `request` | integer | `StatusType` | as above |
  345|    | `Reservation` (`UpstreamError`) | `upstream_error` | none | **accepted** | the producer writes no status on `upstream_error` rows (`:1497-1502`, `:1215-1217`) |
  346| 
  347|    A status that is present but not an integer (string, float, boolean, object) is `StatusType` on every path. An
  348|    integer that contradicts the basis (non-200 with `Usage`, 200 with `ErrorStatus`) is `ErrorCharge`. `ErrorStatus`
  349|    also needs charged = 0 [`ErrorCharge`].
  350| 8. `Usage`:
  351|    - the usage is priceable under the producer's own rule (§7.1) [`UsageUnpriceable`];
  352|    - the request `model` equals the canonical pinned `model_id` (MC-1 to MC-3) [`ModelMismatch`];
  353|    - `producer_price < 10^15` nano [`ToleranceRange`], and `charged ≥ producer_price − T_round` [`Undercharge`].
  354| 
  355| `Conflicting` replaces `Valid` or `Rejected` when two distinct settlements name one reservation.
  356| 
  357| **Permissible transitions as evidence is added (the decision is a function of the evidence set; this lists how it may
  358| change):**
  359| 
  360| | From | Adding | To |
  361| |---|---|---|
  362| | Absent | a well-formed valid settlement | Valid |
  363| | Absent | an invalid settlement | Rejected |
  364| | Valid or Rejected | a second, distinct settlement | Conflicting |
  365| | Valid | a second, different reservation with the same `rid` | Rejected(`ReservationConflict`) |
  366| | any | a byte-identical repeat | unchanged |
  367| | Conflicting | anything | Conflicting |
  368| 
  369| **No other transition exists.** In particular there is no `Conflicting → Valid`, no `Rejected → Valid` without
  370| removing evidence, and nothing makes a decision `Valid` because a numeric amount is present (`Valid` requires all eight
  371| conditions).
  372| 
  373| ## 6. Obligation: releasing versus replacing (stage S)
  374| 
  375| ```text
  376| Obligation = Outstanding                 // the reservation is an open obligation in time (§11.2)
  377|            | Closed{ by: EvidenceRef, amount_source: SettlementAmount | ReservationConfirmed | ReservationBound }
  378|            | None{ cause: Unbudgeted | NoReservation | Orphan | Subscription }   // no reservation exists (R-06)
  379| ```
  380| 
  381| `Obligation` is an **execution-time** fact: whether a reservation is still open in time. It is not a financial
  382| disposition (§6.2).
  383| 
  384| **`amount_source` (R5-08).** It records where the conservative measure's amount comes from. It is constructed by stage
  385| S from the decision and the replacement fact, and nothing else sets it:
  386| 
  387| | Variant | Constructed when | Meaning |
  388| |---|---|---|
  389| | `SettlementAmount` | `Valid(Usage)` with `Replacement::Eligible` | the reservation is replaced by the settlement and usage valuation |
  390| | `ReservationConfirmed` | `Valid(Reservation)` or `Valid(UpstreamError)` (KeepAll: charged = R) | the proxy kept the whole reservation as its charge; the reservation stays as a known amount, `AtLeast` |
  391| | `ReservationBound` | `Valid(ErrorStatus)`; `Valid(Usage)` with `Replacement::Ineligible`; `Absent`, `Rejected`, `Conflicting` with a reservation | the reservation is retained as provisional exposure without being the proxy's charge |
  392| 
  393| `amount_source` is absent only for `Obligation::None` (no reservation exists). Its JSON spellings are
  394| `settlement_amount`, `reservation_confirmed` and `reservation_bound`.
  395| 
  396| The five distinctions the owner asked for:
  397| 
  398| | # | Concept | Contract term |
  399| |---|---|---|
  400| | 1 | Releasing a reservation | `Obligation::Closed`: the attempt has ended in time (its interval ends at the settlement). Only `Valid` closes. Closing never establishes billing (§6.2, R-07) |
  401| | 2 | Replacing provisional exposure with a validated settlement | only `Valid(Usage)` with `Replacement::Eligible` (a usage bound, FB-1) replaces the reservation (`SettlementAmount`). An ineligible usage settlement, `Valid(ErrorStatus)` and the invalid decisions keep it (`ReservationBound`, R5-01, D4). `Valid(Reservation)` and `Valid(UpstreamError)` confirm it (`ReservationConfirmed`) |
  402| | 3 | Retaining independent usage evidence | `Valuation` (§7) is computed from usage evidence whatever the settlement decision, and supplies the supported low bound wherever §8 says so |
  403| | 4 | Classifying unknown additional exposure | `High::AtLeast` (§8), with a blocking `UnresolvedExposure` |
  404| | 5 | Deciding whether admission is established | stage R (§12), outside S to X |
  405| 
  406| `Absent`, `Rejected` and `Conflicting` leave the obligation `Outstanding`.
  407| 
  408| ### 6.2 Execution versus billing (R-07, owner disposition P1.30)
  409| 
  410| ```text
  411| Execution = Terminal | NotTerminal      // Terminal: a settlement, request, upstream_error, stream_error or response_model_refused row is in the attempt's evidence
  412| Replacement = Eligible | Ineligible{reason}   // §2.1 (R5-01): decides whether the reservation may be replaced
  413| Billing   = Established | Unresolved | NotApplicable   // Established iff the high is Exact (FB-1); NotApplicable for the Codex pool
  414| ```
  415| 
  416| Five facts are kept apart and never stand in for one another:
  417| 
  418| | Fact | Contract term |
  419| |---|---|
  420| | execution completion | `Execution::Terminal` |
  421| | valid proxy settlement | `SettlementDecision::Valid` (and `Obligation::Closed`) |
  422| | usage valuation | `Valuation` (§7); a usage bound when complete over established sets (FB-1) |
  423| | verified provider billing | `AmountBasis::ProviderBilling`: no source exists; never produced in S1 |
  424| | unresolved additional exposure | `High::AtLeast`, so `Billing::Unresolved` |
  425| 
  426| `Billing::Established` means established under the usage-valuation evidentiary standard (§2), not verified billing.
  427| An attempt that is `Terminal` and `Closed` can still be `Unresolved` (an upstream error, an error status, a
  428| reservation-basis settlement, incomplete usage). Rollup, predecessor and commitment rules read `Billing`, never
  429| `Execution` or `Obligation`, to decide whether money is resolved.
  430| 
  431| **Error-status attempts (Q2).**
  432| - The settlement's charge of 0 is the proxy's own accounting (`ProxyCharge` 0), never the provider's bill.
  433| - The reserved amount R is kept as a known amount, and the additional billing is unknown: exposure
  434|   `low 0, high AtLeast(R [ProxyReservation])`, diagnostic `NonBillingUnverified`.
  435| - `Exact(0)` is permitted only for an error class with authoritative evidence of non-billing (D4). That class is empty.
  436| 
  437| ### 6.1 Forwarding fact (Q6)
  438| 
  439| ```text
  440| Forwarding = Forwarded{evidence}        // a provider response exists: a request row with an upstream HTTP status, a response id or model,
  441|                                         // or usage; a response_model_refused row; any stream_error row (every producer stream_error
  442|                                         // writer runs after the upstream response arrived: pi_container.py:1295, :1513, :1546) (R-13)
  443|            | Unknown                     // a reservation or admission with no response row; an upstream_error row (the request may or
  444|                                         // may not have reached the provider)
  445| NotForwarded                             // only for requests that are not attempts: a refused row with a preflight reason (C-9)
  446| ```
  447| 
  448| Forwarding is derived only from the attempt's own rows (its key rows and `auxiliary` rows), never from a `refused`
  449| row.
  450| 
  451| Forwarding is recorded on every attempt. It is a fact, not a finding, and it decides nothing about money or identity:
  452| - an identity refusal is `Forwarded` (the provider answered with another model);
  453| - an unresolved identity contradiction still blocks, whatever the forwarding fact.
  454| 
  455| A request proven never to have reached the provider (a `refused` row with a preflight reason) has no exposure and no
  456| identity finding; it is not an attempt.
  457| 
  458| ## 7. Valuation (stage V)
  459| 
  460| ### 7.1 Pricing primitives (leaf functions)
  461| 
  462| - **`openai_price(usage, model, tier_mult)`** = `(input − cached − write) × r_in + cached × r_cached + write × r_write
  463|   + output × r_out`. The tier multiplier is applied to all rates. When `input > 272,000`, input-side rates × 2 and
  464|   output × 1.5. All in exact nano per token.
  465| - **`anthropic_price(usage, model, residual)`** = `input × r_in + cache_read × r_read + w5m × r_5m + w1h × r_1h
  466|   + (cache_write − w5m − w1h) × residual(r_5m, r_1h) + output × r_out`, with `residual ∈ {min, max}`. The producer
  467|   consistency price uses `max` (the producer's rule). The valuation range uses `min` for the low and `max` for the
  468|   high (V-4, R-15). A null or absent `w5m`/`w1h` counts as 0, as in the producer.
  469| - **`producer_price(settlement, pinned)`.** The proxy's own rule:
  470|   - OpenAI: tier = the `observed_tier` multiplier if recognised, else the **highest** multiplier in the table; a null
  471|     cache count counts as 0 (`pi_container.py:1001`).
  472|   - Anthropic: defined only for a `standard`, null or absent tier (`service_tier in (None, "standard")`,
  473|     `pi_container.py:1052-1054`), an integer status, integer counts, and `w5m + w1h ≤ cache_write`.
  474|   - Otherwise undefined, which gives `UsageUnpriceable`.
  475| - **Unknown model.** A model absent from the rate files has no rate. `claude-haiku-5-5` is absent: producer-unpriceable
  476|   (N-08).
  477| 
  478| ### 7.2 Valuation result
  479| 
  480| ```text
  481| Valuation = NotReported                                   // no usage on the attempt's rows
  482|           | Valued{ identities: [Model], low: Bound, high: Bound, splits: [Split], complete: bool }
  483|           | Unpriceable{ reasons: [UnpriceableReason], supported: Option<Bound /*low*/>, priced_high: Option<Bound> }
  484| ```
  485| 
  486| | Rule | Statement |
  487| |---|---|
  488| | V-1 Identities | the distinct canonical identities (MC-1 to MC-3) among the launch pinned model, request `model`, `response_model` and any joined `IdentityRefusal.served`. A malformed name or a `ModelUnresolved` pinned model is an unpriceable identity |
  489| | V-2 Range | `low` = minimum and `high` = maximum of the price over every identity × every allowed split × residual rate × the tier set. The range is a usage bound only when the tier set and the identity set are established and the usage is complete (FB-1) |
  490| | V-3 Tier set (OpenAI) | **recognised** `observed_tier` (a key of the rate table's `tier_multiplier`): that one multiplier; the tier set is established by the response. **Missing** (`null` or absent) **or unrecognised**: the tier set is **not established**. The rate table is not evidence that every servable tier is known (owner disposition R-03/Q5, P1.30), and the producer charging such a tier at the table's highest multiplier fixes its charge, not the set of tiers. Valuation `Unpriceable{TierUnestablished(Missing \| Unrecognised), supported: None, priced_high}`: the known valuation at the dearest table multiplier is kept as `priced_high`, no minimum is supported (an unknown tier could be cheaper), and §8 gives `AtLeast` |
  491| | V-3 Tier set (Anthropic) | `standard`, `null` or absent gives standard, as the producer accepts (R-14). Any other string gives `Unpriceable{NonStandardTier}`. Any other non-string gives `Unpriceable{MalformedTier}` (repairs N-02) |
  492| | V-4 Splits (OpenAI) (R5-04) | each of `cached_tokens` and `cache_write_tokens` is in one state. **Present integer ≥ 0** (including 0): that value. **Null**: unknown (the provider omitted it; the current producer writes `null`, `pi_container.py:1563-1564`, and prices it as 0 for its own charge only). **Absent key**: unknown (written by producers before `ab2a0f84`, which did not log `cache_write_tokens`; 59 real unbudgeted rows). It is **never assumed zero**. **Malformed** (present, not an integer ≥ 0): `Unpriceable{MalformedCount}`. Unknown counts are enumerated over the feasible set `{cached, write ≥ 0, cached + write ≤ input}`, with any known count fixed. The range is the minimum and maximum price over its vertices; all vertices are priceable, so the range is a usage bound when the usage is complete (FB-1). Known counts that violate `cached + write ≤ input` are `Unpriceable{MalformedCount}` |
  493| | V-4 Splits (Anthropic) | `w5m + w1h ≤ cache_write`: the reported split, with the residual writes `cache_write − w5m − w1h` at min(r_5m, r_1h) for the low and max for the high (R-15). Otherwise all writes at min for the low and max for the high, with diagnostic `InconsistentCacheSplit` (repairs probe H-a); the counts contradict each other, so the usage is not complete (V-5) |
  494| | V-5 Completeness | `complete = true` only for usage from a terminal success (`response.completed`, `message_stop`, status 200) whose counts are consistent (for Anthropic, `w5m + w1h ≤ cache_write`). Otherwise the usage may understate the bill and is not a usage bound (FB-1). Completeness is computed from the usage evidence, never from a diagnostic |
  495| | V-6 Partial pricing | if some identity has no rate: `Unpriceable{IdentityWithoutRate, supported: None, priced_high: high over priced identities}`. No minimum is supported, because the unpriced identity could be cheaper, so §8's low is 0 |
  496| | V-7 Excluded sources | usage not on a proxy row (e.g. a Pi session file's `cacheWrite`) is not evidence (A1-06) |
  497| | V-8 Several observations (R-05, R5-04) | an attempt with more than one usage observation (conflicting settlements; a settlement plus a joined refusal) has `Valued{low = min of lows, high = max of highs, complete = all complete, identities = union}`. If any observation is `Unpriceable`, the result is `Unpriceable{reasons = union}` with **`supported = None`** (the unpriceable observation could be the cheapest) and **`priced_high` = max of every valued high and every priced high** (absent when none exists). Adding an observation never lowers `high` |
  498| 
  499| ## 8. Exposure (stage X)
  500| 
  501| Pool: `PiApi` for API lanes and for any dollar evidence. `Codex` for subscription attempts on an established
  502| subscription lane; these carry no dollars and billing is `Unknown`. A contradicted lane is treated as `PiApi`.
  503| 
  504| | Decision \ evidence | `low` | `high` |
  505| |---|---|---|
  506| | `Valid(Usage)`, `Valued`, `complete`, one identity | valuation low | `Exact(max(charged [ProxyCharge], valuation high [UsageValuation]))` (FB-1) |
  507| | `Valid(Usage)`, `Valued`, `complete`, several identities | valuation low (cheapest identity) | `Exact(max(charged, dearest))`, plus blocking `IdentityContradiction` (D6) |
  508| | `Valid(Usage)`, `Valued`, not `complete` (`Ineligible{UsageIncomplete}`) | valuation low | `AtLeast(max(R [ProxyReservation], charged [ProxyCharge], valuation high [UsageValuation]))`, blocking. The reservation is retained (R5-01) |
  509| | `Valid(Usage)`, `Unpriceable` (`Ineligible{UsageUnpriceable}`: identity without a rate, tier set not established, malformed counts, …) | supported low or 0 | `AtLeast(max(R, charged, priced_high or 0))`, blocking. The reservation is retained (R5-01) |
  510| | `Valid(Reservation)`, any valuation | valuation low if `Valued`, else supported low or 0 | `AtLeast(max(R [ProxyReservation], valuation high or priced_high or 0))` (FB-2). The producer keeps the reservation only when its own usage rule fails (invalid counts, a contradictory Anthropic split, a non-standard Anthropic tier, an incomplete exchange), so there is no usage bound whatever V-5 says (R-01) |
  511| | `Valid(UpstreamError)` | 0 | `AtLeast(R)` (FB-2) |
  512| | `Valid(ErrorStatus)` | valuation low if `Valued`, else supported low or 0 | `AtLeast(max(R, valuation high or priced_high or 0))` (FB-2, Q2, R5-02; diagnostic `NonBillingUnverified`). The proxy's charge of 0 is a proxy amount, not billing; reported usage keeps its provenance as a usage valuation |
  513| | `Absent` (reservation, no settlement) | 0 | `AtLeast(R)` (FB-2) |
  514| | `Rejected` or `Conflicting`, with a reservation | 0 | `AtLeast(max(R, every well-formed claimed charge, every usage observation's valuation high or priced_high))`, blocking (FB-2, R-05). A claimed amount that is not well-formed is ignored for the maximum; the rejection itself blocks |
  515| | `NoReservation` (settlement only) | 0 | `AtLeast(max(well-formed charged, valuation high or priced_high))`, else `AtLeast(0)`; blocking |
  516| | `NotApplicable`, unbudgeted, `Valued`, `complete`, one identity | valuation low | `Exact(valuation high)` |
  517| | `NotApplicable`, unbudgeted, `Valued`, `complete`, several identities | valuation low (cheapest) | `Exact(dearest)`, blocking `IdentityContradiction` |
  518| | `NotApplicable`, unbudgeted, not `complete`, or `Unpriceable` | supported low | `AtLeast(valuation or priced high)`, blocking |
  519| | `NotApplicable`, unbudgeted non-200 status, no usage | 0 | `AtLeast(0)`, blocking `UnresolvedExposure{UnverifiedNonBilling}` (D4) |
  520| | Orphan or ambiguous error attempt (no reservation of its own) | 0 | `AtLeast(0)`, blocking |
  521| | Subscription attempt, lane established | — | pool `Codex`, billing `Unknown`; no dollar exposure; may carry a cost-proxy measurement |
  522| | Subscription attempt, lane contradicted | 0 | `PiApi`, `AtLeast(0)`, blocking `LaneContradiction` |
  523| 
  524| **Rules:**
  525| - **X-0. Notation.**
  526|   - `R` is the **largest of the group's distinct well-formed reservation amounts** (R5-09). Normally there is one. When
  527|     one `rid` names different reservations, their independence is not established, so they are neither summed nor
  528|     reduced to the smaller: all stay visible, `R` is the largest, and `ReservationConflict` blocks. A malformed amount
  529|     contributes 0 and adds `MalformedEvidence(reserved)` (R-06).
  530|   - `max(a [basis_a], b [basis_b])` takes the basis of the larger. **On a tie the basis is `UsageValuation`, falling
  531|     back to `ProxyReservation`, then `ProxyCharge`** (revision 5 corrects the order to match the KeepAll case, where the
  532|     charge is the reservation). An `Exact` high may carry basis `ProxyCharge` only when a usage bound exists below it
  533|     (FB-1, R-02).
  534| - **X-1.** `low.amount ≤ high.amount` always. If a valuation low would exceed the chosen high (impossible under §7,
  535|   stated for completeness), that is blocking `ArithmeticFailure`.
  536| - **X-2. Unknown never becomes finite.** No sum, projection or rollup turns `AtLeast` into `Exact`, and a known partial
  537|   amount never caps an unresolved one. The only `AtLeast` → `Exact` transition is replacement by an eligible usage
  538|   settlement (FB-1, F3 exception, R5-01), when evidence is added to an attempt.
  539| - **X-3. Findings never feed money.** Exposure is a function of (decision, obligation, valuation, lane, `R`) only. No
  540|   finding is an input.
  541| 
  542| ## 9. Classification (stage K)
  543| 
  544| `classify(facts) → (Vec<Uncertainty>, Vec<Diagnostic>)`.
  545| 
  546| **Blocking `Uncertainty`** (each carries evidence refs and the session scopes that could own them):
  547| 
  548| | Code | Raised when |
  549| |---|---|
  550| | `SettlementRejected(reason)` | decision `Rejected` |
  551| | `SettlementConflict` | decision `Conflicting` |
  552| | `ReservationConflict` | reservations with one `rid` differ |
  553| | `SettlementWithoutReservation` | `NoReservation` |
  554| | `IdentityContradiction` | ≥ 2 identities, or an unpaired refusal naming another model (D6) |
  555| | `LaneContradiction` | a present label contradicts the session lane or provider; any `IdentityRefusal` in a subscription session; dollar evidence in a subscription session; subscription labels in an API session |
  556| | `UnadmittedMeasurement` | a measurement with no earlier admission |
  557| | `UnaccountedAttempt{cause}` | orphan or ambiguous error; unbudgeted request with no response and no charge |
  558| | `UnresolvedExposure{cause}` | any `AtLeast` high (D5) |
  559| | `UnreadableEvidence` | any `Unreadable` item in an owned source |
  560| | `InventoryIncomplete{kind}` | turn gap, unrecorded turn file, unreferenced, misplaced or shared log, missing log |
  561| | `DuplicateResponseIdentity` | one provider response id in two attempts |
  562| | `AttributionContradiction` | an owned attempt is placed wholly outside its owner's window (an admission/time contradiction) |
  563| | `ReopenedWithoutAdmission` | a session whose lifecycle has `reopened` (a deliver after a retire, D2), for the objectives that admitted it; persists after a later retirement (R-10) |
  564| | `UnadmittedExecution` | a vertical-produced attempt that no admission covers (O-8); no current case, because no producer writes `attempt_id` yet |
  565| | `CommitmentInvalid` | §12.3 A-1 or A-2 |
  566| | `UnenforcedCommitment` | a commitment of kind `AtLeast` (§12.3): pending, launched-open, retired-resumable or reopened, without a qualified `ExecutionGate` (R-04, OQ-1) |
  567| | `GateContradicted` | gate evidence claims `reopening` enforced but the session was reopened after `effective_from` (§2.3 G-7) |
  568| | `ArithmeticFailure` | — |
  569| | `RateTableUnusable` | — |
  570| 
  571| `Unreadable` and inventory findings are scoped to every session that could own the source: the path's session id,
  572| the ledger's naming session, or both (repairs S-05; V-07 is scoped this way instead of globally).
  573| 
  574| **Additional blocking code:** `MalformedEvidence(field)`: a malformed field on evidence that no settlement rejection
  575| already covers (e.g. a reservation's `n`, an unbudgeted row's non-object `usage`).
  576| 
  577| **Non-blocking `Diagnostic`:** `RepeatedIdenticalRow`, `InconsistentCacheSplit`, `NonBillingUnverified`,
  578| `IdentityAbsent`, `ProducerLogCopy`, `ErrorStatusSettlement`, `ReadFailurePaired`, `PartialUsage`,
  579| `UnfinishedAdmission` (an admitted subscription request with no observation: an unknown Codex call),
  580| `UncertainCodexCall` (a measured, producer-labelled subscription error).
  581| 
  582| **Rule K-1.** Deleting any diagnostic from an output changes no other field. Classification is downstream only.
  583| 
  584| ## 10. Session lifecycle (stage L)
  585| 
  586| ```text
  587| SessionLifecycle = { turns: [Turn{n, start: Stamp, source}], retirements: [Stamp|Undated], state, reopened, clock }
  588| state    = Open | Retired{at} | Contradicted{reason}          // the terminal state, from the last lifecycle row
  589| reopened = None | Some{retired_at, reopened_turn}             // a deliver followed a retire at some point (R-10)
  590| clock = Trusted | Untrusted{ AheadOfReference | ProxyRowsOutOfOrder | LifecycleContradicted }
  591| ```
  592| 
  593| | Rule | Statement |
  594| |---|---|
  595| | L-1 (G1) | Dated lifecycle rows never go backwards in append order; otherwise `Contradicted` |
  596| | L-2 (G2) | A turn's proxy rows are not dated before its start row; otherwise `Contradicted` |
  597| | L-3 (G3) | A turn's proxy rows are not dated after any dated retirement that follows it; otherwise `Contradicted` |
  598| | L-4 | `Retired{at}` iff the last lifecycle row is a dated retire and L-1 to L-3 hold, whether or not the session was reopened earlier. An undated retirement bounds nothing |
  599| | L-5 | `reopened = Some` iff a `deliver` follows a retire anywhere in the rows (D2: no atomic re-admission mechanism exists, so blocking `ReopenedWithoutAdmission` for the admitting objectives). L-4 and L-5 are independent facts: retire, deliver, retire gives `Retired{at}` with `reopened = Some` (R-10) |
  600| | L-6 | `clock = Untrusted` if any proxy row is dated after the reference time, dated proxy rows go backwards in append order, or `Contradicted` |
  601| | L-7 (OQ-1) | `Retired{at}` is a fact about the lifecycle rows, not an execution gate. It releases a commitment only together with a qualified `ExecutionGate` (§2.3, §12.3) |
  602| 
  603| ## 11. Temporal attribution (stage T)
  604| 
  605| ### 11.1 Ownership (D1)
  606| 
  607| ```text
  608| Ownership = Owned{ objective, admission: EvidenceRef }
  609|           | Uncertain{ candidates: [Objective], cause }      // conflicting or several admissions; a reopened turn
  610|           | Unadmitted{ candidates: [Objective] }            // no admission record covers the session (O-6)
  611|           | Unowned                                          // no candidate at all
  612| ```
  613| 
  614| | Rule | Statement |
  615| |---|---|
  616| | O-1 | An attempt is owned by the objective whose admission record (journal `launched` row, or a legacy `objective_admission` row) admitted the attempt's session **for the turn that contains the attempt** |
  617| | O-2 | Turns started after a retirement (reopened) are covered by no admission until an atomic re-admission exists (D2), so their attempts are `Uncertain{cause: ReopenedWithoutAdmission}` |
  618| | O-3 | Several admitting objectives for one session, or none established, give `Uncertain`. Session ancestry may define additional **candidates** (which objectives must block) but never assigns ownership |
  619| | O-3a | **Authority evidence is never filtered away (R-08).** Every objective with an admission record naming the attempt's session is a candidate of an `Uncertain` attempt, whatever its tree or window. Ancestry and time can add candidates; they never remove an admitting objective. A contradiction between admission and time (an owned attempt wholly outside its owner's window) stays visible as `AttributionContradiction` (Table T) |
  620| | O-4 | An attempt in a session that no objective tree contains and no admission names is `Unowned`; it affects no objective |
  621| | O-5 | An attempt whose pool is `Codex` (an established subscription lane, no dollar evidence) affects only Codex reporting. Its ownership uncertainty does not block `pi_api` admission. A lane-contradicted attempt is `PiApi`, so O-3 applies |
  622| | O-6 | **Unadmitted historical evidence (Q8).** An attempt in a session that no admission record covers is `Unadmitted{candidates}`. This includes every unbudgeted API session of the current producer, which writes no admission. Its observed financial facts are kept unchanged (stages C to X). No admission authority is synthesised for it |
  623| | O-7 | **Candidates.** The candidates of an `Unadmitted` attempt are the objectives whose tree contains its session **and** whose window could contain its §11.2 interval. The candidates of an `Uncertain` attempt are its admitting objectives (O-3a) plus the same tree-and-time set. "Could contain" uses the obligation interval, never an isolated row stamp (R-12): an `Outstanding` interval runs to its bound (the following dated retirement, or open). A candidate is dropped only when the interval lies wholly `Before` or `After` the window on a trusted clock; an unplaceable interval (P-2: untrusted clock, missing stamps without consistent neighbours, inversion) keeps every tree candidate. Only candidates block, with `HistoricalUnadmitted` or `OwnershipUncertain`; unrelated objectives never do |
  624| | O-8 | **Vertical execution.** In the Work Engine vertical, metered execution requires explicit admission before execution. An attempt whose producer features mark it as vertical-produced (`attempt_id` present) and which no admission covers is blocking `UnadmittedExecution` for its candidates. Historical evidence never establishes that authority retroactively |
  625| 
  626| ### 11.2 Placement
  627| 
  628| ```text
  629| Interval  = Closed obligation: [reservation_at, settlement_at] | Outstanding: [reservation_at, bound]
  630|             | Unbudgeted: [row_at, row_at]
  631| bound     = the dated retirement following the attempt's turn (L) | Open
  632| Placement = Inside | Before | After | Straddles{Start|Closure} | Unplaceable{cause}
  633| ```
  634| 
  635| | Rule | Statement |
  636| |---|---|
  637| | P-1 | Placement is computed against the **owner's** window [start, closed_at or open], using `closed_at` only when the closure is established (§12.4). Otherwise the window is treated as open for placement |
  638| | P-2 | `Unplaceable` if the session clock is untrusted, a required stamp is missing with no consistent dated neighbours (A1.3.x neighbour rule: nearest dated rows before and after, in append order, must not invert), or the interval is inverted |
  639| | P-3 | Stamps after the reference time make the session's clock untrusted (§10), so every attempt in it is `Unplaceable` (repairs S-03, C1, C2) |
  640| | P-4 C1 rule | An attempt owned by O and reserved inside O's window **stays O's** even when it settles or remains outstanding after O's closure. O's totals are `provisional` until every owned attempt is closed or bounded by a dated retirement |
  641| 
  642| **Rule P-5. T modifies no amount.** It returns ownership and placement only.
  643| 
  644| ## 12. Objective rollup and admission-facing output (stage R)
  645| 
  646| ### 12.1 Output
  647| 
  648| ```text
  649| ObjectiveAccounting = {
  650|   objective, window, status: Established | NotEstablished{ reasons: [Reason] }, provisional: bool,
  651|   owned_closed: ExposureSum, owned_outstanding: ExposureSum,     // owned attempts placed Inside (or C1-retained)
  652|   predecessor_obligations: ExposureSum,                          // §12.2
  653|   commitments: [Commitment],                                     // §12.3
  654|   uncertain: [AttemptKey], blocking: [Uncertainty], diagnostics: [Diagnostic] }
  655| ```
  656| 
  657| **`reasons`** uses the §9 blocking codes plus:
  658| - `OwnershipUncertain`: an uncertain `PiApi` attempt names the objective as a candidate;
  659| - `HistoricalUnadmitted`: an unadmitted `PiApi` attempt names the objective as a candidate (O-6, O-7);
  660| - `PlacementUnplaceable`, `PlacementStraddles`: an owned attempt is unplaceable or straddles the window;
  661| - `UnenforcedCommitment`: a commitment of kind `AtLeast` (§12.3);
  662| - `GateContradicted`: a gate for an admitted session is contradicted (§2.3 G-7);
  663| - `ClosureNotEstablished`: the objective's record claims a closure (`closed_at` or a close entry) that is not
  664|   established (§12.4).
  665| 
  666| **Rollup lines:**
  667| - `owned_closed` sums owned attempts whose obligation is `Closed` or none (unbudgeted and orphan attempts);
  668| - `owned_outstanding` sums owned `Outstanding` attempts;
  669| - owned attempts are summed whatever their placement, including `Before` and `After` (money is never moved by
  670|   placement, P-5), and placement problems add reasons.
  671| 
  672| `Established` iff **all** of:
  673| - no blocking finding scoped to the objective's owned or candidate attempts and sessions;
  674| - no `AtLeast` in any summed line;
  675| - no owned attempt `Straddles` or `Unplaceable`;
  676| - no `Uncertain` or `Unadmitted` attempt naming the objective as a candidate;
  677| - every commitment well-formed and of kind `Exact` (§12.3);
  678| - no claimed closure that is not established (§12.4).
  679| 
  680| Otherwise `NotEstablished` with every reason.
  681| 
  682| ### 12.2 Predecessor obligations (D1; revised for R-07, owner disposition P1.30)
  683| 
  684| - An objective record MAY name its predecessor (`predecessor` field).
  685| - The successor's `predecessor_obligations` = the high exposure of every predecessor-owned attempt whose **billing is
  686|   `Unresolved`** (§6.2: `Outstanding`, or `Closed` with an `AtLeast` high), plus the predecessor's open commitments
  687|   (pending, launched-open, retired-resumable, reopened; §12.3). The line is `AtLeast` if any term is.
  688| - **Settled predecessor spend**, meaning owned attempts whose billing is `Established`, is never included.
  689| - Execution completion does not discharge an obligation. An attempt that closes on an upstream error, an error status,
  690|   a reservation basis or incomplete usage stays in the successor's line until its billing is established.
  691| - Established closure of the predecessor (§12.4) refuses new admission to the predecessor. It does not discharge the
  692|   predecessor's obligations; they keep constraining the successor.
  693| - An attempt appears in exactly one objective's owned lines and at most once in one successor's predecessor line, so
  694|   nothing is counted twice within an objective.
  695| - Shared-root objectives without a `predecessor` link: no constraint is inferred (XO-1 remains a known open issue).
  696| 
  697| ### 12.3 Commitments (computed for the admission authority; enforcement stays outside the core)
  698| 
  699| ```text
  700| Commitment = { admission_id, session: Option<SessionId>, cap: Nano,
  701|                state: pending | launched_open | retired | retired_resumable | released | reopened | invalid,
  702|                committed: Nano, kind: Exact | AtLeast, gate: { qualified: bool, reasons: [GateReason] } }
  703| ```
  704| 
  705| | Rule | Statement |
  706| |---|---|
  707| | A-1 (S-08) | Caps are well-formed amounts > 0, and owner limits ≥ 0. Malformed, negative, zero or non-finite values give `Invalid` (committed 0, `Exact`), so the objective is `NotEstablished{CommitmentInvalid}` |
  708| | A-2 | One `pending` per admission. `released` with `launched` or `runner_starting` is `Invalid`. Pending without `launched` or `released` is committed in full (`committed = ceiling`: the cap, or a qualified G-8 bound, R5-03). A valid `released` row releases it: no session was launched, so nothing can resume (committed 0, `Exact`) |
  709| | A-3 (OQ-1) | **Launched.** `committed = max(0, ceiling − the session's owned high)`, where `ceiling` is the admission cap, or the qualified G-8 `amount_nano` (never above the cap) when a qualified billing-bound gate exists (R5-03). The state follows the lifecycle: `launched_open` (not retired, not reopened), `reopened` (reopened, not retired), or, when the lifecycle state is `Retired`: `retired` if a qualified gate establishes release (G-5), otherwise **`retired_resumable`**, which keeps the commitment (L-7). Only `retired` (committed 0, `Exact`) releases. Owned spend above the cap is `NotEstablished{CommitmentInvalid}` |
  710| | A-4 | A session with `reopened = Some`: whatever the state, the objective is `NotEstablished{ReopenedWithoutAdmission}` until a qualified admission covers the reopening (D2). A reopening never releases a commitment and never establishes safety. If a gate claims `reopening` enforced, the reopening contradicts it (G-7) |
  711| | A-5 (R-04, Q10, OQ-1, OQ-3, R5-03) | **Kind.** `pending`, `launched_open`, `retired_resumable` and `reopened` commitments are `AtLeast(committed)` with blocking `UnenforcedCommitment`, unless a qualified gate with an enforced, numerically bound `billing_bound` (G-5, G-8) makes a `pending` or `launched_open` commitment `Exact(committed)`. `retired`, `released` and `invalid` are `Exact(0)`. Every commitment records `gate.qualified` and the failed rules (§2.3). For the current producer: `qualified = false`, reason `NoGateEvidence` |
  712| 
  713| ### 12.4 Admission (outside the core, for corpus expectations; revised for R-09, Q11, OQ-2, OQ-4)
  714| 
  715| The decision for a request `new` against objective O is taken in this order:
  716| 
  717| 1. **Closure (Q11, OQ-4).** O's closure evidence:
  718|    - **no claim** (`closed_at` null and no close entry): open;
  719|    - **established**, by profile:
  720|      - `current_producer` (**legacy cooperative trust**): a `scope_budget close` entry `{at, disposition, reason,
  721|        set_by}` with `at` equal to `closed_at`; `closed_at` a well-formed RFC 3339 UTC stamp no earlier than
  722|        `started_at` and no later than the reference time; and no contradictory evidence (no admission row for O dated
  723|        after `closed_at`, no second close entry with another time). `set_by` is not evidence of owner identity. A
  724|        same-UID agent can write this record, and S1 cannot prove otherwise. This is operational closure under
  725|        cooperative tooling, not a tamper-resistant authorization;
  726|      - `future_qualified_gate`: a closure record from O's declared `vertical_admission_authority` with a receipt
  727|        reference and sha256, and the same time and coherence conditions. The legacy file format alone does not
  728|        establish authority;
  729|    - **claimed but not established** (anything else, including a `closed_at` with no close entry): the decision is
  730|      `not_established`, and O carries reason `ClosureNotEstablished`.
  731| 
  732|    An established closure gives **`refused`** before any monetary evaluation. This is an authorization result, not
  733|    evidence that O's accounting is complete: unresolved obligations remain recorded, O's totals stay provisional while
  734|    any owned attempt is unresolved, and its obligations keep constraining successors (§12.2).
  735| 2. **Limit.** If O's limit is not a well-formed amount ≥ 0, the decision is `not_established`.
  736| 3. **Known over-limit (OQ-2).** `known = owned_closed.high.amount + owned_outstanding.high.amount +
  737|    predecessor_obligations.high.amount + Σ committed (non-invalid commitments) + new`, counting the known amount of
  738|    every `AtLeast` term. Each attempt contributes once (its reservation or its settlement-derived amount, F2), and
  739|    only **owned** attempts count: `Uncertain` and `Unadmitted` attempts are never attributed speculatively. If
  740|    `known > limit`, the decision is **`refused`**, even while billing uncertainty remains. This is refusal under the
  741|    conservative budget policy, not a statement that the provider invoice is known, and O's reasons are unchanged.
  742| 4. **Accounting.** Otherwise, if O is `NotEstablished`, the decision is `not_established` (the known portion is within
  743|    the limit, but unknown exposure may exceed it).
  744| 5. **Headroom.** Otherwise every term is `Exact`: `admit` iff `known ≤ limit`. (Step 3 has already refused `known >
  745|    limit`.)
  746| 
  747| `refused` and `not_established` are equally non-permissive (A5); they differ only in meaning.
  748| 
  749| ## 13. Canonical output and determinism
  750| 
  751| - Outputs are canonical JSON: sorted keys, amounts as decimal strings of nano-USD integers, lists ordered by
  752|   (source, line) or by `AttemptKey` order.
  753| - Equal inputs give byte-identical output.
  754| - Session and source presentation order MUST NOT change any output beyond declared append-order rules within a source.
  755| 
  756| ## 14. Transition specification
  757| 
  758| `TRANSITIONS.md` gives the factorised complete tables (per stage, plus composition), and `INVARIANTS.md` the
  759| executable invariants.
===== END FILE contract/CONTRACT.md =====

===== BEGIN FILE contract/TRANSITIONS.md | sha256 5986fab710155381df802d39ac7b3f1166aeb0b920568247bb91958aad328e33 | 14970 bytes | 184 lines | encoding numbered =====
    1| # Transition specification (normative, `pi-accounting-contract/1.4-s1`)
    2| 
    3| ## Why factorised tables are complete
    4| 
    5| The owner asked for every combination of seven dimensions:
    6| - reservation (3 states);
    7| - settlement (4);
    8| - usage (3);
    9| - correlation (3);
   10| - lane (2);
   11| - lifecycle (3 terminal states × reopened yes/no);
   12| - attribution (4).
   13| 
   14| That is 3,456 combinations.
   15| 
   16| The contract makes each stage a function of **only its declared inputs** (CONTRACT §1). So the combined result is the
   17| composition of the stage tables below. A dimension a stage does not read cannot change that stage's output.
   18| 
   19| Completeness is shown per stage: each table below covers the full product of its own inputs. Composition rules state
   20| which outputs flow where. The invariants INV-F7 (window independence) and INV-F4 (finding inertness) are the
   21| executable check that no undeclared input leaks in.
   22| 
   23| | Stage | Reads | Does not read |
   24| |---|---|---|
   25| | S (decision, obligation) | reservation, settlement, correlation | usage valuation range, lane, lifecycle, attribution, findings |
   26| | V (valuation) | usage, identities, tier, completeness | settlement decision, lane, lifecycle, attribution, findings |
   27| | X (exposure) | S output, V output, lane | lifecycle, attribution, findings |
   28| | K (findings) | C to X outputs | — (it is a sink) |
   29| | L (lifecycle) | lifecycle rows, proxy-row stamps, reference time | money |
   30| | T (ownership, placement) | X time facts, L, admission records, windows | amounts (copied, never changed) |
   31| | R (rollup, admission facts) | T, X, K, L, commitments | — |
   32| 
   33| ## Table S1: settlement decision and obligation
   34| 
   35| Reservation: `0` absent, `1` present, `2+` conflicting. Settlement: `0` absent, `V` valid (all §5 conditions),
   36| `X` rejected, `2+` conflicting. Correlation is the settlement's join to the reservation.
   37| 
   38| | Reservation | Settlement | Correlation | Decision | Obligation | Amount source |
   39| |---|---|---|---|---|---|
   40| | 0 | 0 | — | `NotApplicable` (unbudgeted or orphan) | `None{Unbudgeted \| Orphan \| Subscription}` | valuation or unresolved (Table X) |
   41| | 0 | V or X | `Row` key (C-2), no reservation of that `rid` | `Rejected[NoReservation…]` | `None{NoReservation}` | claimed charge or valuation, `AtLeast` |
   42| | 0 | 2+ | — | `Conflicting` | `None{NoReservation}` | claimed or valuation, `AtLeast` |
   43| | 1 | 0 | — | `Absent` | `Outstanding` | reservation bound |
   44| | 1 | V (Usage), `Replacement::Eligible` (usage bound) | Exact | `Valid(Usage)` | `Closed{SettlementAmount}` | **settlement and valuation replace the reservation; `Exact`** (FB-1) |
   45| | 1 | V (Usage), `Replacement::Ineligible` (incomplete or unpriceable usage) | Exact | `Valid(Usage)` | `Closed{ReservationBound}` | **reservation retained**; known = max(R, charged, usage high or priced high), `AtLeast`; supported usage low kept (R5-01) |
   46| | 1 | V (Reservation, UpstreamError) | Exact | `Valid(..)` | `Closed{ReservationConfirmed}` | reservation as a known amount (the proxy kept it as its charge), `AtLeast` (billing unresolved) |
   47| | 1 | V (ErrorStatus) | Exact | `Valid(ErrorStatus)` | `Closed{ReservationBound}` | **reservation retained, not replaced** (D4); reported usage kept: known = max(R, usage high or priced high) (R5-02) |
   48| | 1 | X | Exact | `Rejected[reasons]` | `Outstanding` | reservation, raised to claimed and to any valuation high (R-05) |
   49| | 1 | 2+ | Exact | `Conflicting` | `Outstanding` | reservation, raised to claimed and to any valuation high (R-05) |
   50| | 2+ | 0 | — | `Absent`, plus blocking `ReservationConflict` | `Outstanding{ReservationBound}` | R = largest distinct reservation (R5-09); all stay visible |
   51| | 2+ | V or X | Exact | `Rejected[ReservationConflict…]` | `Outstanding{ReservationBound}` | max(largest distinct reservation, claimed, valuation high) |
   52| | 2+ | 2+ | Exact | `Conflicting` | `Outstanding{ReservationBound}` | max(largest distinct reservation, claimed, valuation high) |
   53| | `rid` unusable | — | `Row`, `Unattributed` (C-1) | `Absent` | `Outstanding` | well-formed R or 0, `AtLeast`; `MalformedEvidence`, `UnaccountedAttempt` |
   54| 
   55| **Correlation note.** Settlements join only by an `Exact` `rid` key (C-2): that is the attempt's `key_join`. Shape
   56| rules RS-1 to RS-3 join error and refusal rows as `auxiliary` rows (C-5 to C-7); `Ambiguous` and `Unattributed` keys
   57| apply only to error rows that join nothing (C-8). Auxiliary rows never close an obligation: they add evidence
   58| (diagnostics, identity, forwarding) but cannot produce `Valid` or change `key_join` (R-06).
   59| 
   60| ## Table V: valuation
   61| 
   62| | Usage evidence | Identities | Tier | Valuation | Complete |
   63| |---|---|---|---|---|
   64| | none | — | — | `NotReported` | — |
   65| | well-formed, every identity priced | 1 | OpenAI recognised; Anthropic standard, null or absent | `Valued{low, high}` | terminal success with consistent counts: true; else false |
   66| | well-formed, every identity priced | ≥ 2 | as above | `Valued{low = cheapest, high = dearest}` (contradictory) | as above |
   67| | well-formed, some identity has no rate | any | any | `Unpriceable{IdentityWithoutRate, supported, priced_high}` | — |
   68| | well-formed | any | OpenAI missing or unrecognised | `Unpriceable{TierUnestablished, supported: None, priced_high}` (R-03) | — |
   69| | malformed count, malformed tier, non-standard Anthropic tier | any | any | `Unpriceable{reason}` | — |
   70| | OpenAI cache count present (including 0) | any | recognised | `Valued` with that count | as above |
   71| | OpenAI cache count null or absent key (R5-04) | any | recognised | `Valued` over the vertices of `cached + write ≤ input`, known counts fixed; never assumed 0 | as above |
   72| | OpenAI cache count malformed, or known counts above input | any | any | `Unpriceable{MalformedCount}` | — |
   73| | Anthropic residual writes (split ≤ writes) | any | standard | `Valued`, residual at min (low) and max (high) (R-15) | as above |
   74| | Anthropic split > writes | any | standard | `Valued` with writes at min and max rate; diagnostic `InconsistentCacheSplit` | false (counts contradict) |
   75| | several usage observations, all valued | union | any | V-8: min of lows, max of highs | all complete |
   76| | several usage observations, any unpriceable (R5-04) | union | any | `Unpriceable{union of reasons, supported: None, priced_high: max of valued and priced highs}` | — |
   77| 
   78| ## Table X: exposure (decision × valuation × identities × lane)
   79| 
   80| "1 id" / "≥2 id" is the identity count. `R` is the reservation amount. Bases are in brackets.
   81| 
   82| | Decision | Valuation | Ids | low | high | Blocking added by K |
   83| |---|---|---|---|---|---|
   84| | Valid(Usage) | Valued, complete | 1 | val.low [UsageValuation] | Exact(max(charged [ProxyCharge], val.high)) | — |
   85| | Valid(Usage) | Valued, complete | ≥2 | val.low | Exact(max(charged, val.high)) | IdentityContradiction |
   86| | Valid(Usage), Ineligible | Valued, not complete | any | val.low | AtLeast(max(R [ProxyReservation], charged [ProxyCharge], val.high [UsageValuation])) | UnresolvedExposure (≥2 ids: IdentityContradiction) |
   87| | Valid(Usage), Ineligible | Unpriceable (any reason) | any | supported or 0 | AtLeast(max(R, charged, priced_high or 0)) | UnresolvedExposure |
   88| | Valid(Reservation) | any | any | val.low or supported or 0 | AtLeast(max(R [ProxyReservation], val.high or priced_high or 0)) | UnresolvedExposure (≥2 ids: IdentityContradiction) |
   89| | Valid(UpstreamError) | any | any | 0 | AtLeast(R) | UnresolvedExposure |
   90| | Valid(ErrorStatus) | any | any | val.low if Valued, else supported or 0 | AtLeast(max(R [ProxyReservation], val.high or priced_high or 0)) | UnresolvedExposure (diagnostic NonBillingUnverified) |
   91| | Absent | any | any | 0 | AtLeast(R) | UnresolvedExposure (2+ reservations: ReservationConflict; joined refusal: IdentityContradiction; unusable `rid`: MalformedEvidence, UnaccountedAttempt) |
   92| | Rejected or Conflicting, with reservation | any | any | 0 | AtLeast(max(R, well-formed claimed, val.high or priced_high)) | SettlementRejected or SettlementConflict, UnresolvedExposure |
   93| | Rejected or Conflicting, no reservation | any | any | 0 | AtLeast(max(well-formed claimed, val.high or priced_high) or 0) | SettlementWithoutReservation, UnresolvedExposure |
   94| | NotApplicable unbudgeted | Valued, complete | 1 | val.low | Exact(val.high) | — |
   95| | NotApplicable unbudgeted | Valued, complete | ≥2 | val.low | Exact(val.high) | IdentityContradiction |
   96| | NotApplicable unbudgeted | Valued, not complete, or Unpriceable | any | supported or 0 | AtLeast(val.high or priced_high or 0) | UnresolvedExposure |
   97| | NotApplicable unbudgeted, non-200, no usage | — | — | 0 | AtLeast(0) | UnresolvedExposure{UnverifiedNonBilling} |
   98| | NotApplicable orphan or ambiguous error | — | — | 0 | AtLeast(0) | UnaccountedAttempt, UnresolvedExposure |
   99| 
  100| **Lane overlay** (applied after the rows above):
  101| 
  102| | Lane evidence | Pool | Effect |
  103| |---|---|---|
  104| | API session, labels consistent or absent | PiApi | as above |
  105| | API session, subscription labels or measurement, or another provider | PiApi | as above, plus LaneContradiction |
  106| | Subscription session, labels consistent or absent where the producer writes none | Codex | no dollar exposure; billing Unknown; an unfinished admission is an unknown Codex call |
  107| | Subscription session, contradicting label on any event, dollar evidence, `IdentityRefusal`, unadmitted measurement | PiApi | low 0, high AtLeast(0) (or the dollar rows above), plus LaneContradiction (and UnaccountedAttempt for admissions and errors) |
  108| 
  109| ## Table L: lifecycle
  110| 
  111| | Lifecycle rows (append order) | Proxy-row stamps | State | Reopened | Clock |
  112| |---|---|---|---|---|
  113| | launch (+ delivers), no retire | consistent | Open | no | Trusted |
  114| | … ending in a dated retire, no deliver after any retire | consistent with L-2 and L-3 | Retired{at} (a fact, not a gate: L-7) | no | Trusted |
  115| | … ending in an undated retire | consistent | Open (bounds nothing) | no | Trusted |
  116| | retire followed by deliver, last row a deliver | consistent | Open | **yes** (D2: blocks admitting objectives) | Trusted |
  117| | retire, deliver, …, ending in a dated retire | consistent | Retired{at} (releases nothing without a qualified gate, L-7) | **yes** (still blocks, R-10) | Trusted |
  118| | backwards stamps (L-1) | any | Contradicted | as rows show | Untrusted |
  119| | turn rows before their start (L-2), or after a following retirement (L-3) | — | Contradicted | as rows show | Untrusted |
  120| | any | a row after the reference time, or proxy rows backwards | (state as above) | as above | Untrusted |
  121| 
  122| ## Table T: ownership × placement → attribution
  123| 
  124| | Ownership | Clock | Interval vs owner window | Attribution |
  125| |---|---|---|---|
  126| | Owned(O) | Trusted | inside [start, closure or open] | Inside(O) |
  127| | Owned(O) | Trusted | reserved inside, ends after O's closure | Inside(O), O `provisional` (C1) |
  128| | Owned(O) | Trusted | begins before O's start | Straddles{Start}: blocks O |
  129| | Owned(O) | Trusted | wholly before start or after closure | Before or After: still O's money (P-5; §12.1 sums owned attempts whatever their placement), plus blocking `AttributionContradiction` for O (an admission/time contradiction) |
  130| | Owned(O) | Untrusted | — | Unplaceable: blocks O |
  131| | Uncertain{cands} | any | — | Uncertain: blocks every candidate (OwnershipUncertain). Candidates always include every admitting objective (O-3a, R-08) |
  132| | Unadmitted{cands} (O-6) | any | — | Unadmitted: blocks every candidate (HistoricalUnadmitted). Candidates = tree ∩ temporally possible on the obligation interval; unplaceable keeps (O-7, R-12) |
  133| | Unowned | any | — | affects no objective |
  134| 
  135| ## Table R: admission facts
  136| 
  137| | Condition | Status |
  138| |---|---|
  139| | no blocking finding, no AtLeast line, no `AtLeast` commitment, no Straddles, Unplaceable, Uncertain or Unadmitted affecting O, commitments valid, no claimed closure that is not established | Established |
  140| | any of the above | NotEstablished{every reason} |
  141| 
  142| ## Table A: admission decision (§12.4)
  143| 
  144| | Closure | Limit | Known total vs limit (OQ-2) | Accounting | Decision |
  145| |---|---|---|---|---|
  146| | established (legacy cooperative or vertical authority) | any | any | any | **refused** (authorization; obligations remain) |
  147| | claimed, not established | any | any | O is `NotEstablished{ClosureNotEstablished}` | **not_established** |
  148| | none | malformed | any | any | not_established |
  149| | none | valid | known > limit (owned amounts only, known parts of `AtLeast` included) | any | **refused** (conservative budget policy; invoice not known) |
  150| | none | valid | known ≤ limit | NotEstablished | not_established |
  151| | none | valid | known ≤ limit | Established | admitted |
  152| 
  153| ## Table G: session commitments (§12.3, §2.3)
  154| 
  155| | Session | Gate (qualified per G-1..G-7) | State | Committed / kind |
  156| |---|---|---|---|
  157| | pending (no launch, no release) | none, or no `billing_bound` | pending | cap, `AtLeast` |
  158| | pending | `billing_bound` (with a G-8 numerical bound ≤ cap) and `retries` enforced | pending | bound amount, `Exact` |
  159| | launched, open | none, or no `billing_bound` | launched_open | max(0, cap − owned high), `AtLeast` |
  160| | launched, open | `billing_bound` (with a G-8 numerical bound ≤ cap) and `retries` enforced | launched_open | max(0, bound − owned high), `Exact` |
  161| | launched, `Retired` | none, or not all of `new_forwarding`, `retries`, `reopening` | **retired_resumable** | max(0, cap − owned high), `AtLeast` |
  162| | launched, `Retired` | `new_forwarding`, `retries`, `reopening` enforced, not contradicted | retired | 0, `Exact` |
  163| | launched, reopened, not retired | any (a claimed `reopening` gate is contradicted, G-7) | reopened | max(0, cap − owned high), `AtLeast` |
  164| | released before launch | — | released | 0, `Exact` |
  165| | malformed cap | — | invalid | 0, `Exact` |
  166| 
  167| ## Table P: predecessor line (§12.2)
  168| 
  169| | Predecessor-owned attempt | Billing | In the successor's line |
  170| |---|---|---|
  171| | `Outstanding` | Unresolved | yes, its high (`AtLeast`) |
  172| | `Closed`, high `AtLeast` (upstream error, error status, reservation basis, incomplete usage) | Unresolved | **yes** (R-07) |
  173| | `Closed`, high `Exact` | Established | no (settled spend) |
  174| | open commitment (pending, launched-open, retired-resumable, reopened) | — | yes, `AtLeast(committed)` unless a qualified `billing_bound` makes it `Exact` (A-5) |
  175| 
  176| **Rule FB (rev 3; rev 5 adds replacement eligibility).** A high is `Exact` only with a usage bound: complete usage over established identity and tier
  177| sets, which also makes the usage settlement eligible to replace the reservation. Reservations, proxy charges and declared caps are known amounts, never bounds (CONTRACT §2.1, §2.2).
  178| 
  179| ## No second fallback
  180| 
  181| Table X is total. Every cell names its amount source, and no cell is chosen by the presence or absence of a finding.
  182| The previous mechanism (choose a reservation-only representation when "something went wrong while pricing") has no
  183| counterpart: a pricing failure is the `Unpriceable` valuation state, an input to a fixed cell. INV-F4 and the
  184| validator's `suppress-diagnostics` self-test check this.
===== END FILE contract/TRANSITIONS.md =====

===== BEGIN FILE contract/INVARIANTS.md | sha256 749041340c6621066908a0a837a473de769152364dcbb8c9871e8853a469c6a2 | 17016 bytes | 70 lines | encoding numbered =====
    1| # Executable invariants (`pi-accounting-contract/1.4-s1`)
    2| 
    3| There are three families, kept apart: **financial** (stages C to X: money facts), **attribution** (stage T) and
    4| **admission** (stage R and the admission facts). Each entry gives:
    5| - the statement, and its preconditions where they matter;
    6| - the observable result and its corpus coverage (case ids under `corpus/cases`; the validator checks every invariant
    7|   has at least one case);
    8| - the generator property for S2 (proptest), and the smallest implementation mutation that must violate the invariant.
    9| 
   10| Corpus cases cite the invariants they exercise. P1.27's INV-1 to INV-11 are refined here into F1 to F11, with T1 to T5
   11| and A1 to A6 added. Revision 3 adds F12, A7 and A8 and restates F1, F3, F5, A1, A3, A4, A5 and A6. Revision 4 adds A9 and A10 and restates A1, A3, A5, A7 and A8. Revision 5 adds F13 and restates F3, F5 and A9.
   12| 
   13| ## Financial invariants (stages C to X)
   14| 
   15| | ID | Statement | Preconditions | Passing example | Violating example | Observable | Corpus | Property (S2) | Smallest violating mutation |
   16| |---|---|---|---|---|---|---|---|---|
   17| | **F1 Release** | `Obligation::Closed` ⇒ `SettlementDecision::Valid` with an `Exact` **key join** to the attempt's reservation (auxiliary RS-rule rows do not count, R-06). A numeric amount alone never releases | a reservation exists | VC01 (usage settlement closes); N01 (closed with `key_join` Exact and an RS-3 auxiliary row) | S01 (charge 0 against 0.8 under `reservation`) | rejected: `Outstanding`, high `AtLeast(R)`, blocking | 20 cases incl. S01×3, S02×2, U01, R02, C4, S06, V01, V02, U05, N02, R06 | for random settlements failing any §5 condition, `obligation = Outstanding` | drop one §5 condition (e.g. skip the `KeepAll` check) |
   18| | **F2 No double count** | Per attempt, exposure counts the reservation or the settlement-derived amount, never both. Totals are sums over distinct `AttemptKey`s; byte-identical rows are one item | — | VC01 (high 0.015, not 0.515) | summing the reservation and the settlement charge | Σ exposure = Σ per-attempt | VC01, VC06, VC12, R04, C3, DUP | Σ over attempts = rollup; adding an identical copy changes nothing | add the settlement charge on top of the reservation |
   19| | **F3 Monotone high** | Adding evidence never lowers `high` (order of §2) in an attempt or any total, **except** a valid usage settlement with `Replacement::Eligible` (a usage bound), which may replace `AtLeast(R)` with the validated `Exact` amount. A valid but ineligible usage settlement, an error-status settlement, and conflicting or rejected settlements keep `R` and only add terms to the maximum (R5-01, R5-02, R-05) | evidence added to an existing set | S04 (refused row added: high stays `AtLeast(R)`); U08 (incomplete usage: high stays `AtLeast(500M)`) | a refused row closing the interval (deac8099 S-04); incomplete usage lowering a 500M reservation to 30M | `high(E ∪ e) ≥ high(E)` unless the added row is an eligible usage settlement | 24+ cases incl. N01, VC03, S01, U08, Q5, P2, P130-R01, P133-A01, P133-A02 | random base plus random row other than an eligible usage settlement: high non-decreasing | let incomplete usage replace the reservation |
   20| | **F4 Finding inertness** | No monetary field depends on the presence, absence or order of any finding. Removing all findings from an output leaves every amount unchanged, and deleting an input row whose only effect is a diagnostic changes no amount | — | Ha (inconsistent split: low 0.018 kept, diagnostic only) | A1.3.3 N-01 (diagnostic deletes the low bound and lowers high); A1.3.4 H-a (low → 0) | amounts equal with and without the diagnostic | N01×3, Ha, V01-other-model, DUP | compute money, strip findings, recompute: equal (structural in Rust: classification is downstream) | choose the representation by `findings.is_empty()` (the A1.3.2 guard) |
   21| | **F5 Unknown stays unknown** | `AtLeast` survives every sum and projection; the only `AtLeast` → `Exact` transition is replacement by an eligible usage settlement (X-2). Reservations, proxy charges and declared caps are never a finite maximum (FB-2); unknown cache counts, tiers and identities are enumerated or left unpriceable, never assumed (V-3, V-4) | `AtLeast` present | N01-variant-reservation-above-both; U08; P133-A04 absent cache counts (bounded over vertices) | collapsing to `Exact(max(R, dearest))`; pricing a missing tier or an absent cache count as zero | `high.kind = at_least`, plus `UnresolvedExposure` | N01×4, N08, D4, U08, Q5, P130-R01 | sum or project any set containing `AtLeast`: result `AtLeast` | replace `AtLeast(x)` by `Exact(x)` when x ≥ R |
   22| | **F6 Totality** | Every evidence line lands in exactly one attempt, non-attempt or unreadable record; nothing is silently dropped or double-assigned | E-1 adapter | R05 (null and [] rows unreadable and blocking) | skipping non-object rows (A1.1 R-05) | validator totality check over all lines | 15 cases incl. R05, U02, N04×4, A1-01, C3 | line count in = Σ records out | `continue` on unparseable lines |
   23| | **F7 Window independence** | Stages C to X and L take no objective window; their outputs are identical for every objective evaluation | — | Hb (attempt facts identical for windows from 06:00 and 06:20) | A1.3.4 probe H-b (identity finding raised only when counted) | grader: per-evaluation facts equal | Hb, C1, S03, R01, N05, U03, A04 | evaluate under random windows: stage outputs byte-equal | early-return on window placement before validation (A1.3.x `if not keep: return`) |
   24| | **F8 Order independence** | Presenting sessions or logs in a different order changes nothing; append order matters only within a source | — | ORD twins | dedupe by first-seen across sessions | twin cases equal | ORD×2 | permute sessions: equal | a global `seen` set across sessions |
   25| | **F9 Pool integrity** | Dollar evidence never leaves `PiApi`; a contradicted lane is never a Codex call; Codex attempts carry no money | — | VC08 (Codex, no money) | N03 (contradicted admission as an unfinished Codex call) | pool and blocking | 10 cases (N03×6, V03, U06, VC08) | any lane-contradicting label gives `PiApi` plus `LaneContradiction` | trust the launch lane over row labels |
   26| | **F10 Determinism** | Equal inputs and rate table give byte-identical canonical output | — | ORD twins run twice | hash-map iteration order leaking into output | two runs compared byte for byte (harness requirement) | ORD×2 | run twice, compare bytes | an unordered collection serialised directly |
   27| | **F12 Execution is not billing** | `Execution::Terminal` and `Obligation::Closed` never imply `Billing::Established`; billing is established iff the high is `Exact` (§6.2, R-07) | — | VC01 (terminal, closed, established) | VC04 (terminal, closed by upstream error, unresolved) treated as settled spend | `billing` field equals `high.kind == exact` | every `pi_api` attempt | closed attempts with `AtLeast` high stay unresolved | derive billing from the obligation state |
   28| | **F13 Replacement eligibility** | `amount_source = SettlementAmount` iff `Valid(Usage)` with a usage bound; an ineligible usage settlement is `ReservationBound` and keeps `R` in the conservative measure; `ReservationConfirmed` only for `Valid(Reservation)` and `Valid(UpstreamError)` (R5-01, R5-08) | — | VC01 (eligible) | U08 replacing its reservation | `replacement` and `amount_source` follow the decision and valuation | every budgeted case | — | mark every valid usage settlement eligible |
   29| | **F11 Supported low** | `low ≤ high`; a non-zero `low` is backed by a usage valuation (basis `usage_valuation`) of reported usage over plausible identities | — | VC03 (low 0.015 from usage under a reservation settlement) | low from a charge or a reservation | basis check | 6 cases incl. Ha×2, N06, A06, U05, VC03 | `low.basis = usage_valuation` whenever `low > 0` | set `low = charged` |
   30| 
   31| ## Attribution invariants (stage T)
   32| 
   33| | ID | Statement | Passing | Violating | Corpus | Property | Mutation |
   34| |---|---|---|---|---|---|---|
   35| | **T1 No money in attribution** | T outputs ownership and placement only; amounts in T's output equal X's | R01 (unplaceable, still priced 0.015) | pricing requiring a placement timestamp (A1.3.x before P1.26 P10) | R01, Hb | T(X) amounts equal X amounts | price only placed rows |
   36| | **T2 Ownership from admission, never ancestry** | An attempt is owned only through an admission record covering its turn. Ancestry, intersected with temporal possibility on the obligation interval (O-7), only adds candidates; every admitting objective of an `Uncertain` attempt is a candidate (O-3a); uncertainty (`Uncertain`, `Unadmitted`) blocks every candidate; unrelated scopes, including tree members whose window cannot contain the attempt's interval, are unaffected | OWN-budgeted-session-without-admission; P130-R08; P130-R12 | assigning tree members (A1.3.4); dropping admitting objectives without a tree (P1.29 R-08); excluding by a row stamp an interval still open (R-12) | OWN×2, R06, S05, V07, U07, VC06, Q8, P130-R08, P130-R12×2 | random trees without admissions: never `Owned` | infer the owner from `parent_session_id` |
   37| | **T3 C1 retention** | An owned attempt reserved inside its owner's window stays the owner's after closure; the owner is `provisional` while any owned attempt is outstanding | U04, D1-closed-objective-provisional | dropping a closure-straddling charge (U-04) | 8 cases | closure moved earlier: ownership unchanged, provisional set | move the attempt to the window containing its settlement |
   38| | **T4 Untrusted time never drops** | Untrusted clocks, contradicted lifecycles and future stamps make attempts `Unplaceable` (blocking), never excluded and never charged elsewhere | C1 (settlement at reference + 2 min: unplaceable) | excluding a future-stamped row (S-03, C1, C2) | 11 cases | random clock faults: placement unplaceable, amounts unchanged | treat a stamp after the reference time as outside the window |
   39| | **T5 Codex scope** | Ownership uncertainty of a `Codex` attempt does not block `pi_api` admission (O-5) | VC08 | blocking every objective holding a subscription session | VC08 | — | apply O-3 to Codex attempts |
   40| 
   41| ## Admission invariants (stage R)
   42| 
   43| | ID | Statement | Passing | Violating | Corpus | Property | Mutation |
   44| |---|---|---|---|---|---|---|
   45| | **A1 Valid controls admit** | Evidence with an established owner, settled billing, no `AtLeast` commitment and headroom is admitted, counted **per capability profile**. Under `current_producer` only sessionless or Codex cases can meet it (OQ-1); `future_qualified_gate` cases with qualified gates are the metered positive controls. A reject-everything implementation fails both | current: VC08, N03 controls, V03-control; future: P132-FG-* positives | an implementation that blocks everything | per-profile minimums (validator) | — | return `not_established` unconditionally (self-test `always_reject`) |
   46| | **A2 Established excludes unresolved exposure** | Any `AtLeast` in an objective's summed lines or commitments, any blocking finding in its scope, or an uncertain closure gives `NotEstablished` | N01, D4, N03 | admitting on a known partial amount | 6 cases | — | ignore `AtLeast` in the rollup |
   47| | **A3 Commitments never increase headroom** | Malformed, negative, conflicting or reopened commitment records give `NotEstablished` or keep the commitment. Open, pending, retired-resumable and reopened commitments are `AtLeast` unless a qualified gate decides otherwise (A-5). Retirement alone releases nothing (L-7), and a reopening never releases (A-4) | S08, N07×2, P130-R10, P132-FG-retired-released | a -1 cap (S-08); any retire releasing (N-07); a retired current-producer session releasing (OQ-1) | 10 cases | random journal corruption never lowers Σ committed below the valid reading | release the commitment when any retire row exists |
   48| | **A4 Predecessor obligations, no double count** | A successor's predecessor line = the high of every predecessor attempt whose billing is unresolved (outstanding, or closed with `AtLeast`) + open commitments; settled (billing-established) predecessor spend is excluded; predecessor closure discharges nothing; no attempt is in two owned lines | VC15, D1-successor, P130-R07 | charging the successor with settled spend; dropping a closed-but-unresolved attempt (P1.29 R-07) | 3 cases | settle a predecessor attempt with a usage bound: it leaves the successor line; close it on an upstream error: it stays | include `owned_closed` in the line; select by obligation state instead of billing |
   49| | **A5 Monotone admission** | Adding non-settlement evidence never turns a denial (`refused` or `not_established`, equally non-permissive) into `admitted`, **except** an enumerated permitted release: a valid dated retirement **together with a qualified gate** (A-3), or a valid `released` admission row (A-2). A retirement under the current producer is not a permitted release (OQ-1). `refused` and `not_established` may change into each other (OQ-2: added evidence can raise the known total) | V06, R05, A1-03, N02, N05, N07; permitted: P132-FG pair | dropping unreadable evidence; treating a current-producer retirement as a release | 13 cases | random base plus random non-settlement evidence other than a permitted release: never denial → admitted | treat unreadable lines as absent |
   50| | **A6 Producer-supported omissions keep the settlement valid** | A producer-supported omission does not reject the settlement. Null OpenAI cache counts are valued over the split vertices and can admit. A missing OpenAI tier keeps a `Valid` settlement but has no usage bound, so it is `AtLeast` (R-03). A null Anthropic tier is standard (R-14) | N06, P130-R14 (admitted); U08 (valid, `AtLeast`) | rejecting the settlement (spurious fail-closed); pricing a missing tier as finite | 3 cases | — | require cached tokens present; reject a null Anthropic tier |
   51| | **A7 Closure precedence** | An established closure gives `refused` before any monetary or accounting evaluation; a claimed but unestablished closure gives `not_established`. Current producer: a consistent `scope_budget close` entry matching `closed_at` (legacy cooperative trust; `set_by` proves nothing). Future vertical: an authority closure record. Closure never discharges predecessor obligations (§12.4, Q11, OQ-4) | P132-CP-closed-valid-close-entry (refused); P132-FG-closure-authority (refused) | timestamp-only closure (D1-closed, U04×2: not established); a close entry contradicting `closed_at`; a legacy close entry in the vertical profile | 8 cases | random closure evidence: established → refused whatever the accounting | apply the limit test before the closure test |
   52| | **A8 Unenforced commitments block** | A commitment without a qualified gate deciding release or a billing bound is `AtLeast` and blocks (A-5); current-producer retired sessions stay `retired_resumable` (OQ-1) | P132-FG-retired-released and P132-FG-open-billing-bound (admitted) | VC01 and the other current-producer retired controls (`retired_resumable`); a self-asserted gate | 20+ cases | any commitment without a qualified gate: `AtLeast` | `kind = exact` for retired or launched-open commitments |
   53| | **A9 Gate qualification** | A gate counts only if authority, identity, scope, period, mechanisms, receipts and consistency all hold (G-1..G-7), and a billing-bound gate also binds a numerical USD amount no greater than the cap, covering forwarding, retries, partial streams, failures and settlement, with a matching binding digest (G-8, R5-03); it changes no attempt money | P132-FG-retired gated twins; P133-FG-VC14-authenticated-bound | self-asserted, wrong authority, wrong session, request scope, expired, missing mechanisms or receipts, contradicted; bound missing, above the cap, wrong currency, incomplete coverage, binding mismatch | 15 cases | random gate field corruption never qualifies a gate | trust `gated: true`; trust a bound flag without an amount |
   54| | **A10 Known over-limit refusal** | When the known total of owned amounts, valid commitments, attributable predecessor obligations and the new request exceeds the limit, the decision is `refused` even with unresolved billing; reasons are unchanged; candidate (uncertain or unadmitted) amounts are never attributed (OQ-2) | D1-successor, VC14 limit-0.55, P132-CP-known-over-limit (refused) | P132-CP-unknown-below-limit (known portion below the limit: `not_established`) | 20+ cases | raising a known amount never turns refused into admitted | return `not_established` whenever any term is `AtLeast` |
   55| 
   56| ## Mapping from P1.27
   57| 
   58| | P1.27 | 1.0-s1 |
   59| |---|---|
   60| | INV-1 Release | F1 |
   61| | INV-2 No double count | F2 |
   62| | INV-3 Monotone high | F3 |
   63| | INV-4 Finding-inert low | F4 (all money) and F11 |
   64| | INV-5 Unknown stays unknown | F5 |
   65| | INV-6 Totality | F6 |
   66| | INV-7 Window independence | F7 and T1 |
   67| | INV-8 Order independence | F8 |
   68| | INV-9 Pool integrity | F9 |
   69| | INV-10 Admission monotonicity | A5 |
   70| | INV-11 Determinism | F10 |
===== END FILE contract/INVARIANTS.md =====

===== BEGIN FILE contract/MONEY.md | sha256 83a6b5f8159264b73587facdad46eafa69a8f172cef98231256aba56a10fc6c4 | 4801 bytes | 70 lines | encoding numbered =====
    1| # Money precision and representability (D10 findings)
    2| 
    3| **Script:** `evidence/money/representability.py`; output `evidence/money/representability.txt`. Run against the
    4| release rate files:
    5| - `openai_api_rates.json` `c0bdfd66ee69f1644eaf188f87f6af5e47beacc0739d20aac5846c923d66fd58`;
    6| - `anthropic_api_rates.json` `8a286433d9e08b37533f2f908f2db60c4357e1a566018c9ca8364937652bdef5`.
    7| 
    8| Rates are parsed from the JSON text as exact decimals, never as binary floats.
    9| 
   10| ## Findings
   11| 
   12| 1. **Every rate is exact.** All 163 rate × multiplier combinations the producer can apply are exact integers in
   13|    nano-USD per token, with largest denominator 1:
   14|    - OpenAI: 4 models × 4 rate fields × 4 tiers × long/short = 128;
   15|    - Anthropic: 7 models × 5 fields = 35.
   16| 2. **The literals involved:** rates `0.05 … 50` USD per million tokens (two decimal places at most); multipliers 0.5,
   17|    1, 2, 1.5.
   18|    - The finest product is 0.05 × 0.5 (flex cached) = 0.025 USD per million = **25 nano-USD per token**.
   19|    - The finest long-output product is 5 × 0.5 × 1.5 = 3.75 USD per million = 3,750 nano-USD per token.
   20| 3. **Valuations are therefore exact integer arithmetic**, with no rounding: tokens × integer.
   21| 4. **Producer amounts** (`charged_usd`, `reserved_usd`) are logged with at most 6 decimals, so they are exact in nano
   22|    (× 1000).
   23| 5. **Overflow headroom.**
   24|    - The largest rate is 150,000 nano per token (50 USD per million × 2 priority × 1.5 long output).
   25|    - Even 2^40 tokens (about 10^12) give 1.6 × 10^17 nano, against an `i128` maximum of about 1.7 × 10^38.
   26|    - Overflow can only come from malformed counts (e.g. 10^30 tokens), which checked arithmetic reports as
   27|      `ArithmeticFailure` (blocking).
   28| 6. **Conclusion:** nano-USD `i128` is sufficient for the current rate set. No alternative is needed.
   29| 
   30| ## Rules for rates that are not exact (future-proofing; none today)
   31| 
   32| - A rate file is loaded as exact rationals. If any rate × multiplier is not an integer in nano per token, the table is
   33|   `UnsupportedPrecision` and loading fails, so all valuations are `Unpriceable{RateTableUnusable}` (blocking). This is
   34|   the S1 rule, so nothing is ever silently approximated.
   35| - **If the owner later wants to admit such rates,** the specified rounding is: compute the whole valuation as an exact
   36|   rational, then round once, `low = floor`, `high = ceil`.
   37| - **Proof (corrected in revision 3).** For a rational `p`: `floor(p) ≤ p ≤ ceil(p)`, `p − floor(p) < 1` and
   38|   `ceil(p) − p < 1`. (`ceil(p) − floor(p)` is 0 for an integer `p` and exactly 1 otherwise; revision 2 wrongly stated
   39|   `< 1`.) So each bound errs by under 1 nano-USD, toward the conservative side. A sum of k bounds rounded separately
   40|   errs by under k nano, still directional. Rounding once per valuation keeps it under 1.
   41| 
   42| ## Producer-consistency tolerance (`T_round`)
   43| 
   44| The proxy computes a usage charge in binary floating point and logs `round(charge, 6)`.
   45| 
   46| **Bound.**
   47| - `|logged − p_float| ≤ 0.5 × 10^-6 USD` = 500 nano (6-place rounding).
   48| - `p_float` comes from at most about 10 float operations on exact inputs, so `|p_float − p| ≤ 10 × 2^-53 × |p|`. For
   49|   `p < 10^15` nano (10^6 USD) that is under 1.2 nano.
   50| - So `|logged − p| < 501.2` nano. Both `logged` (a multiple of 1,000 nano) and `p` (exact rates × integer counts) are
   51|   integers in nano, so their difference is an integer, and `|logged − p| ≤ 501`.
   52| 
   53| **Rule.** `T_round = 501` nano, applied only when `p < 10^15` nano. For `p ≥ 10^15` nano the proof above does not
   54| hold, so the settlement is `Rejected[ToleranceRange]` (CONTRACT §5), never excused and never checked with an
   55| unproven tolerance. Revision 2's scaling formula `T_round(p) = 500 + ceil(1.2e-15 × p)` stated no unit for `p`; it is
   56| removed.
   57| 
   58| **Direction (R-02).** The logged charge can sit up to 500 nano above or below the producer's float value, and rounding
   59| up can move a sub-micro-USD charge from below 500 nano to 1,000 nano (for example gpt-6.1-terra flex, 21 cached
   60| tokens: exact price 525 nano, logged 1,000). This never lowers conservative exposure, because the charge enters a high
   61| only as a term of `max(charged, usage bound)`, and a high is finite only through its usage bound (CONTRACT §2.1 FB-1).
   62| 
   63| **Direction.** The tolerance can only accept a charge up to `T_round` below the exact price. The high bound always uses
   64| `max(charged, exact valuation)`, so the tolerance never lowers exposure.
   65| 
   66| ## Bases on ties (revision 5)
   67| 
   68| When two terms of a maximum are equal, the high's basis is `UsageValuation`, then `ProxyReservation`, then
   69| `ProxyCharge` (CONTRACT X-0). Revision 4's text put `ProxyCharge` before `ProxyReservation`, contradicting the 9 KeepAll
   70| cases (charged = R), whose basis is the reservation. The amount is unchanged; only the provenance label was in question.
===== END FILE contract/MONEY.md =====

===== BEGIN FILE contract/RESERVATION-BOUNDS.md | sha256 f0282f1e09214e74d0cab651757aedaf3330229b8f2b470b06cea7cdf2b1ab5c | 4843 bytes | 50 lines | encoding numbered =====
    1| # Is a proxy reservation an upper bound on provider billing? (Q2 analysis, contract 1.1-s1; commitments added in 1.2-s1; mechanisms in 1.3-s1; numerical bound in 1.4-s1)
    2| 
    3| **Producer:** release `92a03789`, `pi_container.py` `78083734`.
    4| 
    5| **The reservation formula.**
    6| - **OpenAI:** `reserve_usd` (`:689-694`) = `(input_bound × max(r_input, r_cache_write) × m_in + max_output × r_output ×
    7|   m_out) / 10^6 × multiplier`.
    8| - **Anthropic:** `worst_case` (`:1038-1041`) = `input_bound × max(r_input, r_5m, r_1h) + max_output × r_output`.
    9| 
   10| For R to bound what the provider bills, every condition below must hold. The table records which ones the producer
   11| **enforces**.
   12| 
   13| | # | Condition | Enforced by the producer? | Evidence |
   14| |---|---|---|---|
   15| | B-1 | output tokens ≤ `max_output_tokens` | **yes**: the proxy overwrites the request's `max_output_tokens` (OpenAI) or `max_tokens` (Anthropic) when absent or not below the cap, and refuses an Anthropic thinking budget not below it | `:1162-1170` (Anthropic); `:1404-1408` (OpenAI `max_output_tokens` rewrite) |
   16| | B-2 | input tokens ≤ `input_bound` | **no**: a byte-count estimate (`rest_bytes + item bytes + overhead × items`, or a provider-reported prefix plus new bytes). The producer's own docstring says an inline image's token count staying below its base64 length is "assumed, not verified" | `input_bound` `:725-741`; `unpriceable_input` docstring `:743-748` |
   17| | B-3 | the provider bills the pinned model's rates | partly: the request model is pinned (`model_not_pinned`), and the Anthropic path refuses an unverified response model. But billing follows the model the provider ran, and the OpenAI path never checks the response model | `check_request` `:526-545`; `anthropic_identity_refusal` `:625-639` |
   18| | B-4 | the provider bills at no more than the reserved tier multiplier | **no**: the proxy requests a tier, and the provider may serve another. `UsdBudget` charges "a response served at another tier at that tier", and a missing tier at the highest multiplier | `UsdBudget` docstring `:948-960`, `settle` `:994-1021` |
   19| | B-5 | no surcharge outside the rate table (e.g. long context) | **no**: Anthropic: "No long-context rule is applied" | `AnthropicUsdBudget` docstring `:1023-1033` |
   20| | B-6 | an errored or aborted request costs no more than its worst case | **not established**: the producer assumes a non-200 status is unbilled ("assumed, not verified") and keeps the reservation for broken streams. It has no evidence of the provider's billing either way | `UsdBudget.settle` `:1003-1004`; `AnthropicUsdBudget.settle` `:1050-1051` |
   21| 
   22| **Conclusion.** Only B-1 is producer-enforced. B-2, B-4, B-5 and B-6 are assumptions; B-3 is partial. So the
   23| reservation is a known amount the proxy set aside, not a proven upper bound (rule FB-2). The same holds for a proxy
   24| charge that is not a usage price.
   25| 
   26| **What would establish a finite bound, as future producer evidence (not in S1):**
   27| - the provider's own usage or bill for the attempt;
   28| - a producer that refuses unbounded inputs (B-2), pins and verifies the served tier (B-4), and refuses any route with
   29|   surcharges outside the rate table (B-5);
   30| - provider documentation, recorded as authoritative evidence, for the billing of errored requests (B-6; this is D4's
   31|   non-billable class).
   32| 
   33| **Not affected:** an exposure backed by complete provider-reported usage (FB-1) remains `Exact`, conditional on the
   34| stated evidentiary standard (CONTRACT §2).
   35| 
   36| ## Session caps and commitments (revision 3, R-04)
   37| 
   38| The same conditions decide whether an **admission cap** (`max_usd`) bounds a session's provider billing:
   39| - `reserve()` refuses a new reservation when `settled + outstanding + worst case > max_usd` (`:984-988`). That is an
   40|   enforced constraint on **reservations**, and a reservation is not a bound (B-2, B-4, B-5, B-6).
   41| - `settle()` charges the served tier and usage with no cap (`:994-1019`). Spend can therefore exceed the cap. For
   42|   example (P1.29 R-04, reproduced): gpt-6-astra, input bound 2,400 and output cap 9,000 reserve 0.48 at the default
   43|   tier, inside a 0.5 cap; the same tokens served at priority cost 0.948.
   44| 
   45| So a declared cap is an unenforced declared cap (CONTRACT §2.2), and `cap − spend` is an internal accounting
   46| commitment, not a bound. Revision 4 adds that a retired Pi session can resume under the same cap without objective
   47| admission (CONTRACT §2.3), so retirement does not end the session's capacity to spend either. Every commitment
   48| without a qualified `ExecutionGate` is `AtLeast` (CONTRACT §12.3 A-5). An enforced `billing_bound` needs the conditions
   49| above enforced across forwarding, retries, failures, incomplete streams and settlement (OQ-3). It must also bind a numerical
   50| USD amount no greater than the cap, with a binding digest (CONTRACT §2.3 G-8, R5-03). The current producer provides none.
===== END FILE contract/RESERVATION-BOUNDS.md =====

===== BEGIN FILE contract/DECISION-REGISTER.md | sha256 2ceeec56b07c34a8fc78ae1da8323581d5897890f8348864052595c7665a6355 | 19654 bytes | 139 lines | encoding numbered =====
    1| # Owner decision register (P1.28 to P1.33, contract `1.4-s1`)
    2| 
    3| | # | Owner decision (P1.28) | Where applied | Residual question for the owner |
    4| |---|---|---|---|
    5| | D1 | C1 reservation-time ownership. A closed objective with unresolved obligations keeps provisional totals. Ownership is never inferred from ancestry; an uncertain owner blocks the affected objectives. Predecessor obligations constrain successors without transferring or double-counting settled spend | CONTRACT §11.1 O-1 to O-4, §11.2 P-4, §12.2; TRANSITIONS Table T | **Q1:** successors are defined by an explicit `predecessor` field on the objective record, which does not exist today. Shared-root objectives without the link keep XO-1 open |
    6| | D2 | R1 reacquire before execution. Reopening is refused until an atomic admission mechanism exists; retrospective reinstatement is not enforcement | CONTRACT §10 L-5, §11.1 O-2, §12.3 A-4 | none for S1. The enforcement point (refusing delivery to a retired session) is in the admission authority, outside the core |
    7| | D3 | One Rust crate with an I/O-free core; adaptation and integration outside it. No crate in S1 | CONTRACT §1 (stage E outside the core) | none |
    8| | D4 | Unknown provider billing is `AtLeast(0)`. `Exact(0)` only for an authoritatively nonbillable error class; a proxy assumption alone is insufficient | CONTRACT §6 (ErrorStatus closes but does not replace), §8 (unbudgeted non-200: `AtLeast(0)` blocking) | **Q2 (P1.28 text, superseded by the P1.29 disposition: `AtLeast(R)`):** with a reservation present, an error-status attempt keeps `Exact(R)` (the producer's worst-case bound) rather than `AtLeast(0)`, so it does not block. Confirm, or require `AtLeast(R)`. **Q3:** the nonbillable class is empty until authoritative provider evidence is supplied. Real records contain 0 error-status settlements and 0 non-200 request rows (89 logs), so neither choice changes a historical result |
    9| | D5 | `AtLeast(x)` is not a finite maximum; admission cannot use it as headroom; blocking | CONTRACT §2, §8 X-2, §9 `UnresolvedExposure`, §12.1 | none |
   10| | D6 | Every unresolved identity contradiction blocks; diagnostics never modify amounts | CONTRACT §7.2 V-1, §8, §9, X-3, K-1 | none |
   11| | D7 | Additive `attempt_id = (incarnation, sequence)`; `n` stays separate; no producer change | CONTRACT §4 (`ProducerAttempt` key, `attempt_id` feature flag) | none for S1 (S6 unauthorised) |
   12| | D8 | A sanitised, local-only corpus derived from the real logs; digests, provenance and mappings kept; no raw logs committed | corpus `real/` (derived aggregates and sanitised per-session cases; raw logs stay in place) | none |
   13| | D9 | Plan IR recovery stays independent; no resumption in S1 | — | none |
   14| | D10 | Checked `i128` nano-USD; exact rates; directional rounding where needed; overflow and precision fail explicitly | CONTRACT §2; MONEY.md | none: all 163 current rate × multiplier combinations are exact integers in nano-USD per token |
   15| 
   16| ## P1.29 owner dispositions (revision 2)
   17| 
   18| | Question | Owner disposition (P1.29) | Applied as | Status |
   19| |---|---|---|---|
   20| | Q2 | Keep reservation-based conservative exposure for error-status attempts. Distinguish reserved, proxy-charged, an evidenced bill, and unknown additional billing. A reservation is not automatically a proven bound; a finite bound needs producer-enforced conditions; never `Exact(0)` because of an error | CONTRACT §2.1 (FB-1, FB-2, the four amounts), §6 (error-status), §8 Table X; `RESERVATION-BOUNDS.md` (only the output cap B-1 is producer-enforced) | **closed.** Consequence: Q10 |
   21| | Q5 | Missing OpenAI tier: an interval over all supported plausible tiers with exact rates; high ≥ the dearest rate and ≥ the proxy charge; no unsupported low; if the tier set cannot be established, unresolved exposure | V-3: missing gives every table tier (low cheapest, high dearest); an unrecognised string gives `Unpriceable{UnknownTier}`, so `AtLeast` with no supported low | closed in P1.29; **superseded by the P1.30 R-03/Q5 disposition** (a missing tier no longer establishes the tier set) |
   22| | Q6 | An identity contradiction blocks; verified financial evidence is kept independently; non-forwarding is distinguished; diagnostics never change money | §6.1 forwarding fact (identity refusals are `Forwarded`; `refused` rows are non-attempts); X-3 and K-1 unchanged; refusal exposure follows FB-2 (`AtLeast`) | closed |
   23| | Q8 | No synthesised authority for historical unadmitted requests. They are historical, unadmitted evidence with financial data kept; only possible owners block; future vertical execution needs admission | O-6 (`Unadmitted`), O-7 (candidates = tree ∩ temporally possible), O-8 (`UnadmittedExecution` for vertical-produced attempts); reason `HistoricalUnadmitted` | closed |
   24| 
   25| **New owner question from revision 2:**
   26| - **Q10.** FB-2 with D5 means any owned attempt whose exposure rests on a reservation alone makes its objective
   27|   `NotEstablished`. That covers in-flight requests, unsettled reservations, error statuses, upstream errors,
   28|   reservation-basis settlements and rejected settlements.
   29|   - **Operational effect:** a new session cannot be admitted while another session of the same objective has a request
   30|     in flight or an unsettled reservation.
   31|   - **Historical effect:** the frozen Step D objective (two reservation-basis charges) would be `NotEstablished` under
   32|     this contract.
   33|   - **Options:**
   34|     - accept this (strict, consistent with Q2 and D5);
   35|     - define producer evidence that makes a reservation a bound (B-2, B-4 and B-5 enforced; a future producer slice);
   36|     - allow admission against a stated assumption class, as an explicit owner-authorised exception, contrary to the
   37|       current Q2 wording.
   38| 
   39|   Revision 2 encodes the strict option.
   40| - **Q11. Refusal versus `not_established` when refusal is already certain.** §12 decides `not_established` for any
   41|   objective that is not established, before the limit test. Under revision 2, two cases that revision 1 refused now read
   42|   `not_established`. Both decisions deny admission.
   43|   - `D1-closed-objective-provisional`: the objective is closed. The revision-1 validator refuses admission to a closed
   44|     objective, a rule the §12 text does not state.
   45|   - `D1-successor-constrained-by-predecessor-obligations`, evaluation `successor`: the known amounts alone (predecessor
   46|     line `AtLeast(1.0)` plus the new 0.1) already exceed the 1.0 limit.
   47|   - **Options:** keep the current order (any unresolved exposure gives `not_established`); or refuse first when the
   48|     objective is closed or when the known amounts already exceed the limit. Either way, state the closed-objective rule
   49|     in §12 or drop it from the validator.
   50|   - Revision 2 leaves the §12 text and the validator unchanged.
   51| 
   52| **Remaining open after P1.29:** superseded by the P1.30 section below.
   53| 
   54| ## P1.30 owner dispositions (revision 3)
   55| 
   56| | Item | Owner disposition (P1.30) | Applied as | Status |
   57| |---|---|---|---|
   58| | R-03 / Q5 | The rate table is not evidence that every service tier is known. A finite interval needs evidence of the complete applicable tier set; otherwise keep known valuation and represent unresolved additional exposure. Never manufacture an upper bound from an incomplete table | V-3: only a recognised `observed_tier` establishes the tier set. Missing or unrecognised gives `Unpriceable{TierUnestablished}`, with `priced_high` kept and no supported low; §8 gives `AtLeast`. Supersedes P1.29's missing-tier treatment | closed |
   59| | R-04 / Q10 | A pending request whose exposure is not provably bounded prevents established admission for objectives that may own it. An unenforced session cap is not a billing bound. Distinguish an enforced constraint, an internal commitment, an unenforced declared cap and unknown additional exposure. Bounded concurrency may be qualified later; for S1, unbounded or unverifiable in-flight exposure blocks. Keep positive controls | §2.2 (the four amounts); FB-2 extended to declared caps; §12.3 A-5: open and pending commitments are `AtLeast` (`UnenforcedCommitment`) unless a `QualifiedBound` exists; no S1 evidence qualifies. Controls: 17 admitted (minimum 15 unchanged) | closed; interpretation OQ-1; **amended by P1.32** (OQ-1: retired current-producer sessions also keep their commitment; QualifiedBound replaced by ExecutionGate) |
   60| | Q11 | Authoritative closure gives REFUSED before monetary headroom; it is an authorization result, not complete accounting. Unresolved predecessor obligations stay recorded and constrain successors. Uncertain closure gives NOT_ESTABLISHED | §12.4 step 1 and Table A; `ClosureUncertain`; §12.2 (closure discharges nothing); A7 | closed; authority definition NEW-1; **amended by P1.32** (OQ-4: closure needs a consistent close entry (legacy) or authority record (vertical); ClosureUncertain renamed ClosureNotEstablished) |
   61| | R-07 | Terminal execution does not settle billing. Under C1, predecessor obligations persist until their financial disposition is established. Keep execution completion, valid proxy settlement, usage valuation, verified billing and unresolved exposure distinct. A successor gains no headroom because a predecessor attempt closed | §6.2 (`Execution`, `Billing`); §12.2 and Table P (line selects by billing); F12; A4 | closed |
   62| | R-08 | No filtering or precedence rule may discard conflicting authority evidence before classification. Contradictions stay visible and block | O-3a; O-7 for `Uncertain`; T2; validator check | closed |
   63| | R-02 | Resolve FB-1 versus upward-rounded charges; exact i128 nano and rates; directional rounding and tolerance including the 6-decimal producer representation; rounding never lowers conservative exposure without supporting settlement evidence | FB-1 reworded around the usage bound; §2 rounding direction; `T_round` integer argument and `ToleranceRange`; MONEY.md | closed |
   64| 
   65| The other P1.29 findings are dispositioned in `FINDINGS-REGISTER.md`.
   66| 
   67| **Open owner questions after P1.30:** OQ-1 to OQ-4 are decided in P1.32 (below). The text that follows is historical.
   68| 
   69| **Open owner questions after P1.30:**
   70| - **OQ-1 (R-04 scope).** Revision 3 treats every open-session or pending commitment as unbounded, not only sessions with
   71|   a request in flight, because R-04's overrun happens after admission on an idle open session. Cost: VC14,
   72|   S08-control-pending-commitment and C1-control-before-reference no longer admit. The alternative, counting an idle
   73|   session's commitment as finite and blocking only on outstanding reservations, admits R-04's counterexample. Confirm
   74|   or choose.
   75| - **OQ-2 (known amounts over the limit).** When an objective is `NotEstablished` but its known amounts already exceed
   76|   the limit (`D1-successor-constrained-by-predecessor-obligations`), revision 3 decides `not_established`, not
   77|   `refused`. Both deny admission. Confirm, or make a certain refusal take precedence.
   78| - **OQ-3 (`QualifiedBound` evidence).** What producer evidence should qualify a bounded commitment (B-1 to B-6
   79|   enforced, served tier verified)? It is needed before any concurrent admission can be established. No S1 evidence
   80|   qualifies.
   81| - **OQ-4 (closure authority).** Revision 3 takes a well-formed `closed_at` no later than the reference time as
   82|   authoritative closure, and a malformed or later value as uncertain (NEW-1). Confirm, or name another authority
   83|   (for example a closure journal row).
   84| - Still open from P1.28: **Q1** (predecessor links and XO-1), **Q3** (nonbillable error class, empty), **Q4**
   85|   (unbudgeted incomplete usage, now also covered by R-01's rule), **Q7** (final line without newline), **Q9**
   86|   (session with no launch row).
   87| 
   88| 
   89| **Further owner questions raised by the contract** (P1.28 report §10, historical text; Q2, Q5, Q6 and Q8 are superseded by the tables
   90| above, and Q5 again by P1.30):
   91| - **Q4: unbudgeted incomplete usage** (`terminal` null or failed, no `usd`) is `AtLeast`, blocking. Real records hold 2
   92|   such rows, already flagged by every reader.
   93| - **Q5: missing OpenAI tier** (U-08) is priced over {cheapest, dearest} multipliers instead of being rejected. This is a
   94|   normative repair of a known open defect; confirm it belongs in S2 scope.
   95| - **Q6: identity-refusal reservations** are `AtLeast(max(R, dearest))` (unknown additional exposure) rather than a
   96|   finite range, because the reservation bounds only the pinned model. This is stricter than A1.3.4 (which reported a
   97|   finite high). Both block.
   98| - **Q7: a final line without its newline** (a complete JSON object included) is unreadable evidence, blocking, on every
   99|   lane. Previously only the subscription lane did this; the known U-02 decision is taken this way. Real logs: 0 of 90
  100|   lack a final newline.
  101| - **Q8: unbudgeted API sessions** have no admission record in the current producer, so under D1 their attempts have
  102|   uncertain ownership and block every candidate objective (contract O-6). The alternative is to admit unbudgeted
  103|   sessions through an explicit record (a producer or launcher change). Real records: 134 completed and 2 incomplete
  104|   unbudgeted API request rows (89 logs).
  105| - **Q9: a session with no launch row but an admission record** (R-06). Can stage E treat a log found only by the
  106|   host-directory inventory as evidence, or only as an inventory finding? The corpus encodes this as an ambiguity
  107|   (`R06-launch-row-missing`).
  108| 
  109| ## P1.32 owner dispositions (revision 4)
  110| 
  111| | Item | Owner disposition (P1.32) | Applied as | Status |
  112| |---|---|---|---|
  113| | OQ-1 (option C) | Strict current producer, qualified future gate. Under release `92a03789` retirement does not establish that a Pi session cannot incur future exposure (`role_host deliver` resumes a retired session without objective admission). Session history is not gated because a session is idle or retired. Outstanding exposure and the ability to resume are separate facts. A declared cap is not a billing bound. A typed `ExecutionGate` for the future vertical must establish authority, identity, scope, effective period and enforcement, separately for new forwarding, retries and reopening. A plain flag, a declaration or a same-UID claim is not evidence | §2.3 (producer fact, `ExecutionGate`, G-1..G-7, capability profiles); L-7; §12.3 A-3..A-5 (`retired_resumable`, gate-dependent release and bound); Table G; A3, A8, A9; corpus profiles `current_producer` / `future_qualified_gate` | closed |
  114| | OQ-2 | Refuse when the supported known components of the conservative measure exceed the limit (owned exposure, known parts of `AtLeast`, valid commitments, attributable unresolved predecessor obligations, the new commitment), without double-counting a reservation and its settlement. Refusal is budget policy, not invoice knowledge. Known ≤ limit with possible unknown excess gives NOT_ESTABLISHED. Ownership or limit contradictions stay uncertain, never speculatively attributed. REFUSED and NOT_ESTABLISHED are equally non-permissive | §12.4 steps 2-4; Table A; A5 (denials equal); A10 | closed |
  115| | OQ-3 | Request-count stops, cost-proxy stops, proxy reservations, commitments, declared caps and actual billing bounds are different mechanisms. A finite billing bound needs evidenced enforcement across forwarding, failures, retries and incomplete settlements. No current declared cap or cost proxy qualifies. No expectation change from renaming | §2.2 (six mechanisms); G-5 (`billing_bound` with `retries`) | closed; no expectation changed by it |
  116| | OQ-4 | Current tooling: a consistent `scope_budget close` entry with matching `closed_at`, valid timestamps, coherent lifecycle and no contradictory closure evidence is operationally established closure under **legacy cooperative trust** (not tamper-resistant; same-UID writable; `set_by: user` proves nothing). Future vertical: closure evidence from its admission authority, never inferred from the legacy format. An established closure refuses new admission; obligations remain and totals may stay provisional. Conflicting or incomplete closure evidence gives NOT_ESTABLISHED | §12.4 step 1 by profile; `ClosureNotEstablished`; A7; corpus closure cases | closed |
  117| 
  118| **Unchanged:** D1–D10; Q1, Q3, Q4, Q7, Q9 (open); the Q2, Q5, Q6 and Q8 dispositions (P1.29), with Q5 as amended by
  119| R-03/Q5 (P1.30).
  120| 
  121| **Recorded consequence (OQ-1).** Under the current producer no objective that has admitted a metered session can
  122| establish a new admission. Every such session keeps an `AtLeast` commitment until a qualified gate exists, so the
  123| current-producer admitted controls are only sessionless or Codex cases. Metered positive controls exist only in the
  124| `future_qualified_gate` profile (`corpus/REVISION-4.md`).
  125| 
  126| ## P1.33 owner dispositions (revision 5)
  127| 
  128| | Item | Owner disposition (P1.33) | Applied as | Status |
  129| |---|---|---|---|
  130| | R5-01 | A settlement with incomplete or unbounded usage valuation must not reduce or replace the retained reservation's conservative exposure. Keep terminal execution, structural validity, complete priceable usage, replacement eligibility, proxy amounts and independently established billing apart. Preserve supported partial usage and unresolved additional exposure; never make an open-ended amount finite. State which measure a bound belongs to; do not apply `max(R, usage)` across measures | §2.1 monetary measures (six facts; exposure is one measure, conservative provisional exposure); `Replacement` (Eligible / Ineligible{reason}); §8 ineligible usage rows keep R; `ReservationBound`; X-2 restated; F3, F5, F13 | closed |
  131| | R5-02 | An error-status settlement may close the attempt while keeping reservation-based exposure (D4), but must not discard supported usage. Keep proxy charge, supported usage and unknown billing apart, with provenance. Determine producer reachability; mark synthetic cases as synthetic | §8 `Valid(ErrorStatus)`: low = usage valuation low, high = AtLeast(max(R, usage high)); the error-status-with-usage case is synthetic, reachability not established (the OpenAI writer takes usage only from a terminal SSE event, and a non-200 status settles as error status) | closed |
  132| | R5-03 | A qualified billing-bound gate must establish an authenticated, enforced numerical limit, bound to cap, identity, authority, period, currency and covered activity (retries, partial streams, failures). A declared cap or same-UID assertion does not qualify. A bound above, or inconsistent with, the cap, or without established coverage, does not establish finite exposure. Keep this distinct from S2a | §2.3 `BillingBound` type and G-8; A-2, A-3, A-5 use the bound as ceiling; signature verification is a stage-E obligation of the future vertical (S1 checks the binding digest only) | closed; S1 authentication limit recorded |
  133| | R5-06 | Absent or null HTTP status is permitted only for producer-supported shapes that otherwise satisfy the settlement conditions; it is neither an automatic failure nor a universal exemption. Malformed and contradictory status stays invalid. Decide per path | §5 condition 7 table: usage, error-status and reservation request rows need an integer (the producer always writes one); only `upstream_error` rows may lack a status; A06 (null status on a request row) becomes `Rejected[StatusType]` and producer-unreachable | closed |
  134| 
  135| Revision items R5-04, R5-05, R5-07, R5-08 and R5-09 are contract changes made without owner decisions. The author's
  136| change log is not part of this review set.
  137| 
  138| **Unchanged:** D1–D10; Q1, Q3, Q4, Q7, Q9 (open); Q2, Q5, Q6, Q8 (as amended in P1.30); OQ-1 to OQ-4 (P1.32). R5-02
  139| refines Q2's error-status row: it adds supported usage to the conservative measure and keeps `AtLeast(R)` as the floor.
===== END FILE contract/DECISION-REGISTER.md =====

===== BEGIN FILE contract/S2A-ENTRY-CRITERIA.md | sha256 651dabff59c575bf2f2c0f7d37838950e3c1daca07dd12a43dd0e1b7dfc6e60f | 4754 bytes | 68 lines | encoding numbered =====
    1| # S2a entry criteria: budgeted API-lane attempt reducer only (not authorised)
    2| 
    3| **Contract:** `pi-accounting-contract/1.4-s1` (revision 5). Revision 5 changes S2a stages C to K: replacement eligibility
    4| and the conservative-exposure measure (R5-01), error-status usage (R5-02), valuation totality (R5-04), model
    5| canonicalisation `mc-1` (R5-05), status rules per path (R5-06), the typed `amount_source` (R5-08) and conflicting
    6| reservations (R5-09). R5-03 (gate billing bound) is stage R and the future gate, outside S2a. Revision 2's version of this page wrongly said Q2 keeps
    7| `Exact(R)`, and excluded RS-1 to RS-3 while its grading subset needed them (P1.29 R-16 premise). Both are corrected
    8| here.
    9| 
   10| **Scope.** Stages C (budgeted correlation, **including** the shape rules RS-1 to RS-3 for error and refusal rows
   11| joined to budgeted attempts), S, V, X and K for API-lane sessions with dollar reservations. That includes the
   12| attempt-level facts `forwarding`, `execution` and `billing` (§6.1, §6.2).
   13| 
   14| **Excluded:**
   15| - subscription lanes;
   16| - unbudgeted requests;
   17| - lifecycle L, attribution T, rollup R, commitments and admission;
   18| - the evidence adapter E. S2a consumes typed evidence from a test fixture loader inside its own test suite.
   19| 
   20| ## Entry gates (all required before S2a starts)
   21| 
   22| 1. **Independent design review** of the current revision completed under the modular plan (`review-plan/` of P1.30): module A
   23|    (financial semantics) and module C (corpus and validator) each return a complete verdict with no unresolved P1.
   24|    Module B (lifecycle, attribution, admission) must have no unresolved P1 that touches stages C to K. The final
   25|    integration review is required before S5, not before S2a.
   26| 2. **Owner answers** are recorded for every question that touches stages C to K. Q2, Q5, Q6 and Q10 are decided and
   27|    applied (revisions 2 and 3). OQ-1 to OQ-4 (P1.32) touch only commitments, closure and admission, which S2a excludes.
   28| 3. **The contract is frozen** at a reviewed version, with every review correction applied and the validator passing
   29|    on the corpus (check, revisions, self-test, sabotage).
   30| 4. **A crate decision is recorded** (D3: one crate, I/O-free core) with its name and workspace location, and an S2a
   31|    budget authorised.
   32| 
   33| ## Contract sections S2a must satisfy
   34| 
   35| | Section | Content | S2a obligation |
   36| |---|---|---|
   37| | §2, §2.1 | money, `High`, bases, rounding direction, `T_round` and `ToleranceRange`, FB-1 usage bound, FB-2 | full (checked `i128`, no floats, basis tie rule X-0) |
   38| | §3 | evidence types for `Reservation`, `Settlement`, `StreamError`, `IdentityRefusal`, `Refused`; `Field<T>`; model canonicalisation MC-1 to MC-3 | consume as typed input; E-1 to E-5 are not S2a's |
   39| | §4 | C-1 (including unusable `rid`), C-2 (including `Row` keys for no-reservation settlements), C-5 to C-9, C-11; `key_join` and `auxiliary` | full |
   40| | §5 | settlement decision, the `Valid` conditions, reject reasons, transitions | full |
   41| | §6, §6.1, §6.2 | obligation (including `None`), typed `amount_source`, release versus replacement, `Replacement`, forwarding, execution versus billing | full |
   42| | §7.1, §7.2 | pricing primitives; V-1 to V-8 for OpenAI and Anthropic | full (V-3 tier set per R-03/Q5) |
   43| | §8 | Table X rows for budgeted decisions | full |
   44| | §9 | classification codes produced by S to X | the subset for budgeted attempts |
   45| | §13 | canonical output and determinism | full |
   46| | TRANSITIONS | Table S1, Table V, Table X (budgeted rows), "no second fallback" | full |
   47| | INVARIANTS | F1 to F6 (F6 within a source), F10, F11, F12 | full, as properties |
   48| 
   49| ## Corpus subset S2a must pass (attempt-level fields only)
   50| 
   51| **Selection rule** (applied by `validate_corpus.py results --scope s2a`): every `current_producer` case with at least
   52| one attempt keyed `res:` (a reservation) in a session whose launch lane is `api`. `future_qualified_gate` cases are
   53| excluded: their attempt facts duplicate current-producer cases. The validator prints the selected case ids. The P1.30
   54| selection is recorded in `validator/s2a-subset.txt`.
   55| 
   56| **Attempt fields graded:** key, correlation (`kind`, `auxiliary`), evidence, settlement, obligation, valuation,
   57| exposure, blocking, diagnostics, forwarding, execution, billing, replacement.
   58| 
   59| **Excluded from S2a grading:** attribution, objectives, commitments, admission and lifecycle. Cases outside the
   60| selection rule stay in the corpus for S5 and S7.
   61| 
   62| ## Exit criteria
   63| 
   64| - 100% agreement on the subset's attempt-level fields (`validate_corpus.py results --scope s2a`).
   65| - Proptest properties for F1 to F6 and F10 to F13.
   66| - Mutation testing: semantic equivalents of reader mutants K01 to K04, K15 to K21 and the F-table mutations, all
   67|   caught.
   68| - No I/O in the core. Code review against the contract.
===== END FILE contract/S2A-ENTRY-CRITERIA.md =====

===== BEGIN FILE corpus/SCHEMA.md | sha256 909f97a071cb59b69b140f71d7c37cc79d049d7847889ae74e49bf9e51fcf4a3 | 7969 bytes | 88 lines | encoding numbered =====
    1| # Corpus schema `pi-accounting-corpus/1`
    2| 
    3| One JSON file per case: `cases/<case_id>.json`, with canonical key order. The executable definition is
    4| `validator/validate_corpus.py`; this page describes it.
    5| 
    6| **Money:** nano-USD integer strings. **Times:** RFC 3339 UTC. **Evidence references:** `<alias>/turn-<n>:<line>`.
    7| **Attempt keys:** `res:<alias>/turn-<n>:rid=<r>`, `adm:<alias>/turn-<n>:rid=<r>`, `row:<alias>/turn-<n>:line=<l>`.
    8| 
    9| ```text
   10| {
   11|   schema: "pi-accounting-corpus/1", contract: "pi-accounting-contract/1.4-s1",      // revision 5 (P1.33); 1.3-s1 P1.32, 1.2-s1 P1.30, 1.1-s1 P1.29, 1.0-s1 P1.28
   12|   profile: current_producer | future_qualified_gate,      // OQ-1 capability profile (CONTRACT 2.3)
   13|   case_id, group, role: finding | control | valid_control,
   14|   class: producer_valid | historical_legacy | malformed | adversarial_synthetic | attribution_uncertain | future_vertical,
   15|   reachability: producer_reachable | producer_unreachable | legacy | not_established,
   16|   findings: [finding ids], provenance: { origin: "<script>::<test>", variant?, also? }, invariants: ["INV-…"],
   17|   controls: [case ids],                                   // mandatory for role=finding
   18|   twin_of?: case id,                                      // order-independence twins (INV-F8)
   19|   revisions: [{rule, field, from, to}],                   // revision 2: every normative field changed from revision 1, with its contract rule
   20|   revisions_r3: [{rule, field, from, to}],                // revision 3: every field changed from revision 2 ("__absent__" = the field did not exist)
   21|   revisions_r4: [{rule, field, from, to}],                // revision 4: every field changed from revision 3 (top-level paths allowed: profile, evidence_pair.*)
   22|   revisions_r5: [{rule, field, from, to}],                // revision 5: every field changed from revision 4 (top-level paths: class, reachability, role, findings, controls)
   23|   added_in?: "1.1-s1" | "1.2-s1" | "1.3-s1" | "1.4-s1",   // a case with no counterpart in the previous revision
   24|   evidence_pair?: {other, relation: subset|superset, permitted_release: bool},   // INV-A5 pairs (R-11)
   25|   inputs: { reference_time, rates: {file: sha256},
   26|             gates?: [{gate_id, authority: {id, kind}, session, scope: {kind: session|request, request_id?}, effective_from, effective_until|null,
   27|                       writer: authority|same_uid_file|declaration, mechanisms: {new_forwarding|retries|reopening|billing_bound: {enforced, receipt?: {ref, sha256},
   28|                       bound?: {amount_nano, currency, scope, covers: [failures|forwarding|partial_streams|retries|settlement], binding_sha256}}}}],   // bound: billing_bound only (G-8)
   29|                                                           // future_qualified_gate only (CONTRACT 2.3)
   30|             sessions: [{ session_id, alias, launch: {model, auth_lane, parent} | null,
   31|                          lifecycle: [{event: launch|deliver|retire, at|null, turn?}],
   32|                          logs: {"turn-N": {lines: [raw line strings, exact bytes], final_newline}},
   33|                          extra_files?: {name: {lines, final_newline}},       // on disk, not named by a turn record
   34|                          ignored_sources?: {...} }] },                         // present but not evidence (V-7)
   35|   evaluations: [{ name,
   36|                   objectives: [{objective, started_at, closed_at|null, limit, tree_sessions, predecessor|null,
   37|                                 close?: {at, disposition, reason, set_by},                 // scope_budget close entry (current_producer, legacy cooperative trust)
   38|                                 admission_authority?: {id, kind}, closure_record?: {authority_id, at, receipt}, // future_qualified_gate only
   39|                                 admissions: [{admission_id, session, source: journal|role_host_objective_admission, rows, gate_id?}]}],
   40|                   admission_request: {objective, new},
   41|                   expected: { attribution: [{attempt, objective|null, ownership: owned|uncertain|unadmitted|unowned, placement, cause?,   // uncertain: candidates include every admitting objective (O-3a)
   42|                                               candidates?, provisional_retained?}],            // unadmitted: objective null, candidates per O-7
   43|                               objectives: [{objective, status, reasons, provisional, owned_closed, owned_outstanding,
   44|                                             predecessor_obligations, commitments: [{admission_id, state, committed, kind: exact|at_least, gate: {qualified, reasons}}]}],   // state adds retired_resumable
   45|                               admission: {objective, new, limit, decision} } }],
   46|   expected: {                                             // shared by every evaluation: window-independent facts
   47|     attempts: [{ key, correlation: {kind: exact|ambiguous|unattributed, candidates?, auxiliary?: [{evidence, rule: RS-1|RS-2|RS-3}]},
   48|                  forwarding: forwarded|unknown, execution: terminal|not_terminal, billing: established|unresolved|not_applicable,
   49|                  replacement: {eligible, reason: UsageBound|NotUsageSettlement|UsageIncomplete|UsageUnpriceable|NoReservation},   // 2.1 (R5-01)
   50|                  evidence: [refs], settlement: {decision, basis?, reasons, reserved?, charged?},
   51|                  obligation: {state, amount_source: settlement_amount|reservation_confirmed|reservation_bound|valuation_or_unresolved},   // 6 (R5-08)
   52|                  valuation: {state: valued|unpriceable|not_reported|not_applicable, identities?, low?, high?, complete?, derivations?, reasons?, supported_low?, priced_high?},
   53|                  exposure: {pool: pi_api, low: {nano, basis}, high: {kind: exact|at_least, nano, basis}} | {pool: codex, billing: unknown},
   54|                  blocking: [codes], diagnostics: [codes] }],
   55|     non_attempt: [{evidence, kind, blocking}], unreadable: [{evidence, reason, blocking}],
   56|     inventory: [{session, kind, file?, blocking}], lifecycle: {alias: {state: open|retired|contradicted, reopened: bool, clock, reason?}} },
   57|   ambiguity: null | {fields, note},                        // where one answer cannot be established
   58|   notes, rationale: null | text,                           // rationale is required when historical differs
   59|   historical: { a134_086bdec1: {decision, spent_high_nano, spent_low_nano, blocking_kinds, ...} }   // comparison only
   60| }
   61| ```
   62| 
   63| **Derivation records** (`valuation.derivations[]`) are what the validator re-prices from the rate files:
   64| 
   65| ```text
   66| { provider: openai|anthropic, model, usage, tier_multiplier, nano, for: low|high|both|none,
   67|   remainder_rate?: min|max, split?: reported|ignored }
   68| ```
   69| 
   70| **Revision 3 log** (`revisions_r3`). Same field paths as revision 2, plus `lifecycle[<alias>].<field>` and
   71| `<evaluation>/<objective>.commitments[<admission_id>].<field>`. `validate_corpus.py revisions <corpus> <revision-2 corpus>`
   72| undoes it and requires the revision-2 case back exactly, apart from `contract`, `revisions_r3` and an appended rationale.
   73| The revision-2 `revisions` list is carried unchanged.
   74| 
   75| **Revision log.** A `revisions` entry's `field` is `attempts[<key>].<path>` or `<evaluation>/attribution[<key>].<field>`,
   76| `<evaluation>/admission.<field>` or `<evaluation>/<objective>.<path>`. `validate_corpus.py revisions <corpus> <previous>`
   77| undoes every entry, newest first, and requires the result to equal the revision-1 case, apart from `contract`,
   78| `revisions`, the attempts' `forwarding` and a rationale that must begin with the revision-1 rationale. It also requires
   79| historical blocks to be unchanged and every revision-1 case and finding tag to be present.
   80| 
   81| **Result sets** (an implementation's output, graded by `validate_corpus.py results`):
   82| 
   83| ```text
   84| { case_id: { evaluations: { name: { attempts, non_attempt, unreadable, inventory, lifecycle, attribution, objectives, admission } } } }
   85| ```
   86| 
   87| These are compared field by field after canonical sorting. Attempt facts must be identical across a case's
   88| evaluations (INV-F7).
===== END FILE corpus/SCHEMA.md =====

===== BEGIN FILE corpus/sample-cases.jsonl | sha256 e7b5c4f2c2089a076889ca69218fce78913d538ee322a331e47c2dfd3276db04 | 255195 bytes | 56 lines | encoding numbered =====
    1| {"ambiguity":null,"case_id":"VC01-openai-usage-settlement","class":"producer_valid","contract":"pi-accounting-contract/1.4-s1","controls":[],"evaluations":[{"admission_request":{"new":"100000000","objective":"o"},"expected":{"admission":{"decision":"refused","limit":"1000000000","new":"100000000","objective":"o"},"attribution":[{"attempt":"res:S1/turn-1:rid=1","objective":"o","ownership":"owned","placement":"inside"}],"objectives":[{"commitments":[{"admission_id":"a1","committed":"985000000","gate":{"qualified":false,"reasons":["NoGateEvidence"]},"kind":"at_least","state":"retired_resumable"}],"objective":"o","owned_closed":{"high":{"kind":"exact","nano":"15000000"},"low":"15000000"},"owned_outstanding":{"high":{"kind":"exact","nano":"0"},"low":"0"},"predecessor_obligations":{"high":{"kind":"exact","nano":"0"},"low":"0"},"provisional":false,"reasons":["UnenforcedCommitment"],"status":"not_established"}]},"name":"default","objectives":[{"admissions":[{"admission_id":"a1","rows":[{"admission_id":"a1","at":"2026-11-10T06:00:05Z","event":"pending","max_usd":1.0},{"admission_id":"a1","at":"2026-11-10T06:00:10Z","event":"launched","session_id":"00000000-0000-4000-8000-0000000000c1"}],"session":"00000000-0000-4000-8000-0000000000c1","source":"journal"}],"closed_at":null,"limit":"1000000000","objective":"o","predecessor":null,"started_at":"2026-11-10T06:00:00Z","tree_sessions":["00000000-0000-4000-8000-0000000000c1"]}]}],"expected":{"attempts":[{"billing":"established","blocking":[],"correlation":{"kind":"exact"},"diagnostics":[],"evidence":["S1/turn-1:1","S1/turn-1:2"],"execution":"terminal","exposure":{"high":{"basis":"usage_valuation","kind":"exact","nano":"15000000"},"low":{"basis":"usage_valuation","nano":"15000000"},"pool":"pi_api"},"forwarding":"forwarded","key":"res:S1/turn-1:rid=1","obligation":{"amount_source":"settlement_amount","state":"closed"},"replacement":{"eligible":true,"reason":"UsageBound"},"settlement":{"basis":"usage","charged":"15000000","decision":"valid","reasons":[],"reserved":"500000000"},"valuation":{"complete":true,"derivations":[{"for":"both","model":"gpt-6-astra","nano":"15000000","provider":"openai","tier_multiplier":"1","usage":{"cache_write_tokens":0,"cached_tokens":0,"input_tokens":1000,"output_tokens":100}}],"high":"15000000","identities":["gpt-6-astra"],"low":"15000000","state":"valued"}}],"inventory":[],"lifecycle":{"S1":{"clock":"trusted","reopened":false,"state":"retired"}},"non_attempt":[],"unreadable":[]},"historical":{"a134_086bdec1":{"blocking_kinds":[],"decision":"admitted","evaluation":"default","note":"comparison only; A1.3.4 has no ownership (D1) concept: it attributes by session tree and time window","reader_sha256":"086bdec1d443064f6847821858f34bbf227f8804a9a5fc8a362416f62b29d621","reasons":["within the objective pi_api limit"],"spent_high_nano":"15000000","spent_low_nano":"15000000"}},"inputs":{"rates":{"anthropic_api_rates.json":"8a286433d9e08b37533f2f908f2db60c4357e1a566018c9ca8364937652bdef5","openai_api_rates.json":"c0bdfd66ee69f1644eaf188f87f6af5e47beacc0739d20aac5846c923d66fd58"},"reference_time":"2026-11-10T07:00:00Z","sessions":[{"alias":"S1","launch":{"auth_lane":"api","model":"openai/gpt-6-astra","parent":"root"},"lifecycle":[{"at":"2026-11-10T06:00:10Z","event":"launch","turn":1},{"at":"2026-11-10T06:50:00Z","event":"retire"}],"logs":{"turn-1":{"final_newline":true,"lines":["{\"at\": \"2026-11-10T06:30:00Z\", \"event\": \"usd_reserved\", \"n\": 1, \"rid\": 1, \"reserved_usd\": 0.5, \"input_bound\": 6000, \"bound_basis\": \"bytes\"}","{\"at\": \"2026-11-10T06:30:02Z\", \"event\": \"request\", \"n\": 1, \"http_status\": 200, \"model\": \"gpt-6-astra\", \"requested_tier\": null, \"observed_tier\": \"default\", \"terminal\": \"response.completed\", \"response_id\": \"resp_1\", \"response_model\": \"gpt-6-astra\", \"usage\": {\"input_tokens\": 1000, \"output_tokens\": 100, \"cached_tokens\": 0, \"cache_write_tokens\": 0, \"reasoning_tokens\": 0}, \"seconds\": 1.0, \"usd\": {\"rid\": 1, \"reserved_usd\": 0.5, \"charged_usd\": 0.015, \"charge_basis\": \"usage\", \"session_spent_usd\": 0.015, \"multiplier_floor\": 0.0}}"]}},"session_id":"00000000-0000-4000-8000-0000000000c1"}]},"invariants":["INV-A1","INV-F1","INV-F2"],"profile":"current_producer","provenance":{"origin":"contract::producer pi_container.py:1554-1571 (UsdBudget.settle usage)"},"reachability":"producer_reachable","role":"valid_control","schema":"pi-accounting-corpus/1"}
    2| {"ambiguity":null,"case_id":"VC02-anthropic-usage-settlement","class":"producer_valid","contract":"pi-accounting-contract/1.4-s1","controls":[],"evaluations":[{"admission_request":{"new":"100000000","objective":"o"},"expected":{"admission":{"decision":"refused","limit":"1000000000","new":"100000000","objective":"o"},"attribution":[{"attempt":"res:S1/turn-1:rid=1","objective":"o","ownership":"owned","placement":"inside"}],"objectives":[{"commitments":[{"admission_id":"a1","committed":"972000000","gate":{"qualified":false,"reasons":["NoGateEvidence"]},"kind":"at_least","state":"retired_resumable"}],"objective":"o","owned_closed":{"high":{"kind":"exact","nano":"28000000"},"low":"28000000"},"owned_outstanding":{"high":{"kind":"exact","nano":"0"},"low":"0"},"predecessor_obligations":{"high":{"kind":"exact","nano":"0"},"low":"0"},"provisional":false,"reasons":["UnenforcedCommitment"],"status":"not_established"}]},"name":"default","objectives":[{"admissions":[{"admission_id":"a1","rows":[{"admission_id":"a1","at":"2026-11-10T06:00:05Z","event":"pending","max_usd":1.0},{"admission_id":"a1","at":"2026-11-10T06:00:10Z","event":"launched","session_id":"00000000-0000-4000-8000-0000000000c1"}],"session":"00000000-0000-4000-8000-0000000000c1","source":"journal"}],"closed_at":null,"limit":"1000000000","objective":"o","predecessor":null,"started_at":"2026-11-10T06:00:00Z","tree_sessions":["00000000-0000-4000-8000-0000000000c1"]}]}],"expected":{"attempts":[{"billing":"established","blocking":[],"correlation":{"kind":"exact"},"diagnostics":[],"evidence":["S1/turn-1:1","S1/turn-1:2"],"execution":"terminal","exposure":{"high":{"basis":"usage_valuation","kind":"exact","nano":"28000000"},"low":{"basis":"usage_valuation","nano":"28000000"},"pool":"pi_api"},"forwarding":"forwarded","key":"res:S1/turn-1:rid=1","obligation":{"amount_source":"settlement_amount","state":"closed"},"replacement":{"eligible":true,"reason":"UsageBound"},"settlement":{"basis":"usage","charged":"28000000","decision":"valid","reasons":[],"reserved":"300000000"},"valuation":{"complete":true,"derivations":[{"for":"both","model":"claude-fable-5-2","nano":"28000000","provider":"anthropic","remainder_rate":"max","split":"reported","tier_multiplier":"1","usage":{"cache_creation_input_tokens":0,"cache_read_input_tokens":0,"input_tokens":1000,"output_tokens":500}}],"high":"28000000","identities":["claude-fable-5-2"],"low":"28000000","state":"valued"}}],"inventory":[],"lifecycle":{"S1":{"clock":"trusted","reopened":false,"state":"retired"}},"non_attempt":[],"unreadable":[]},"historical":{"a134_086bdec1":{"blocking_kinds":[],"decision":"admitted","evaluation":"default","note":"comparison only; A1.3.4 has no ownership (D1) concept: it attributes by session tree and time window","reader_sha256":"086bdec1d443064f6847821858f34bbf227f8804a9a5fc8a362416f62b29d621","reasons":["within the objective pi_api limit"],"spent_high_nano":"28000000","spent_low_nano":"28000000"}},"inputs":{"rates":{"anthropic_api_rates.json":"8a286433d9e08b37533f2f908f2db60c4357e1a566018c9ca8364937652bdef5","openai_api_rates.json":"c0bdfd66ee69f1644eaf188f87f6af5e47beacc0739d20aac5846c923d66fd58"},"reference_time":"2026-11-10T07:00:00Z","sessions":[{"alias":"S1","launch":{"auth_lane":"api","model":"anthropic/claude-fable-5-2","parent":"root"},"lifecycle":[{"at":"2026-11-10T06:00:10Z","event":"launch","turn":1},{"at":"2026-11-10T06:50:00Z","event":"retire"}],"logs":{"turn-1":{"final_newline":true,"lines":["{\"at\": \"2026-11-10T06:30:00Z\", \"event\": \"usd_reserved\", \"n\": 1, \"rid\": 1, \"reserved_usd\": 0.3, \"input_bound\": 6000, \"bound_basis\": \"bytes\"}","{\"at\": \"2026-11-10T06:30:02Z\", \"event\": \"request\", \"provider\": \"anthropic\", \"n\": 1, \"http_status\": 200, \"model\": \"claude-fable-5-2\", \"requested_tier\": null, \"observed_tier\": \"standard\", \"terminal\": \"message_stop\", \"response_id\": \"msg_1\", \"response_model\": \"claude-fable-5-2\", \"response_model_check\": \"verified\", \"usage\": {\"input_tokens\": 1000, \"cache_read_input_tokens\": 0, \"cache_creation_input_tokens\": 0, \"output_tokens\": 500}, \"seconds\": 1.0, \"usd\": {\"rid\": 1, \"reserved_usd\": 0.3, \"charged_usd\": 0.028, \"charge_basis\": \"usage\", \"session_spent_usd\": 0.028, \"multiplier_floor\": 0.0}}"]}},"session_id":"00000000-0000-4000-8000-0000000000c1"}]},"invariants":["INV-A1","INV-F1"],"profile":"current_producer","provenance":{"origin":"contract::producer pi_container.py:1043-1072 (AnthropicUsdBudget.settle usage)"},"reachability":"producer_reachable","role":"valid_control","schema":"pi-accounting-corpus/1"}
    3| {"added_in":"1.2-s1","ambiguity":null,"case_id":"P130-R02-rounded-up-charge-admitted","class":"producer_valid","contract":"pi-accounting-contract/1.4-s1","controls":[],"evaluations":[{"admission_request":{"new":"100000000","objective":"o"},"expected":{"admission":{"decision":"refused","limit":"1000000000","new":"100000000","objective":"o"},"attribution":[{"attempt":"res:S1/turn-1:rid=1","objective":"o","ownership":"owned","placement":"inside"}],"objectives":[{"commitments":[{"admission_id":"a1","committed":"999999000","gate":{"qualified":false,"reasons":["NoGateEvidence"]},"kind":"at_least","state":"retired_resumable"}],"objective":"o","owned_closed":{"high":{"kind":"exact","nano":"1000"},"low":"525"},"owned_outstanding":{"high":{"kind":"exact","nano":"0"},"low":"0"},"predecessor_obligations":{"high":{"kind":"exact","nano":"0"},"low":"0"},"provisional":false,"reasons":["UnenforcedCommitment"],"status":"not_established"}]},"name":"default","objectives":[{"admissions":[{"admission_id":"a1","rows":[{"admission_id":"a1","at":"2026-11-10T06:00:05Z","event":"pending","max_usd":1.0},{"admission_id":"a1","at":"2026-11-10T06:00:10Z","event":"launched","session_id":"00000000-0000-4000-8000-0000000000c1"}],"session":"00000000-0000-4000-8000-0000000000c1","source":"journal"}],"closed_at":null,"limit":"1000000000","objective":"o","predecessor":null,"started_at":"2026-11-10T06:00:00Z","tree_sessions":["00000000-0000-4000-8000-0000000000c1"]}]}],"expected":{"attempts":[{"billing":"established","blocking":[],"correlation":{"kind":"exact"},"diagnostics":[],"evidence":["S1/turn-1:1","S1/turn-1:2"],"execution":"terminal","exposure":{"high":{"basis":"proxy_charge","kind":"exact","nano":"1000"},"low":{"basis":"usage_valuation","nano":"525"},"pool":"pi_api"},"forwarding":"forwarded","key":"res:S1/turn-1:rid=1","obligation":{"amount_source":"settlement_amount","state":"closed"},"replacement":{"eligible":true,"reason":"UsageBound"},"settlement":{"basis":"usage","charged":"1000","decision":"valid","reasons":[],"reserved":"500000000"},"valuation":{"complete":true,"derivations":[{"for":"both","model":"gpt-6.1-terra","nano":"525","provider":"openai","tier_multiplier":"0.5","usage":{"cache_write_tokens":0,"cached_tokens":21,"input_tokens":21,"output_tokens":0}}],"high":"525","identities":["gpt-6.1-terra"],"low":"525","state":"valued"}}],"inventory":[],"lifecycle":{"S1":{"clock":"trusted","reopened":false,"state":"retired"}},"non_attempt":[],"unreadable":[]},"historical":{"a134_086bdec1":{"blocking_kinds":[],"decision":"admitted","evaluation":"default","note":"comparison only; A1.3.4 has no ownership (D1) concept: it attributes by session tree and time window","reader_sha256":"086bdec1d443064f6847821858f34bbf227f8804a9a5fc8a362416f62b29d621","reasons":["within the objective pi_api limit"],"spent_high_nano":"1000","spent_low_nano":"525"}},"inputs":{"rates":{"anthropic_api_rates.json":"8a286433d9e08b37533f2f908f2db60c4357e1a566018c9ca8364937652bdef5","openai_api_rates.json":"c0bdfd66ee69f1644eaf188f87f6af5e47beacc0739d20aac5846c923d66fd58"},"reference_time":"2026-11-10T07:00:00Z","sessions":[{"alias":"S1","launch":{"auth_lane":"api","model":"openai/gpt-6.1-terra","parent":"root"},"lifecycle":[{"at":"2026-11-10T06:00:10Z","event":"launch","turn":1},{"at":"2026-11-10T06:50:00Z","event":"retire"}],"logs":{"turn-1":{"final_newline":true,"lines":["{\"at\": \"2026-11-10T06:30:00Z\", \"event\": \"usd_reserved\", \"n\": 1, \"rid\": 1, \"reserved_usd\": 0.5, \"input_bound\": 6000, \"bound_basis\": \"bytes\"}","{\"at\": \"2026-11-10T06:30:02Z\", \"event\": \"request\", \"n\": 1, \"http_status\": 200, \"model\": \"gpt-6.1-terra\", \"requested_tier\": null, \"observed_tier\": \"flex\", \"terminal\": \"response.completed\", \"response_id\": \"resp_1\", \"response_model\": \"gpt-6.1-terra\", \"usage\": {\"input_tokens\": 21, \"output_tokens\": 0, \"cached_tokens\": 21, \"cache_write_tokens\": 0, \"reasoning_tokens\": 0}, \"seconds\": 1.0, \"usd\": {\"rid\": 1, \"reserved_usd\": 0.5, \"charged_usd\": 1e-06, \"charge_basis\": \"usage\", \"session_spent_usd\": 1e-06, \"multiplier_floor\": 0.0}}"]}},"session_id":"00000000-0000-4000-8000-0000000000c1"}]},"invariants":["INV-F5","INV-F11","INV-A1"],"profile":"current_producer","provenance":{"origin":"contract::FB-1, 2 rounding, X-0 (P1.29 R-02)"},"reachability":"producer_reachable","role":"valid_control","schema":"pi-accounting-corpus/1"}
    4| {"ambiguity":null,"case_id":"VC03-reservation-settlement-with-usage","class":"producer_valid","contract":"pi-accounting-contract/1.4-s1","controls":[],"evaluations":[{"admission_request":{"new":"100000000","objective":"o"},"expected":{"admission":{"decision":"refused","limit":"1000000000","new":"100000000","objective":"o"},"attribution":[{"attempt":"res:S1/turn-1:rid=1","objective":"o","ownership":"owned","placement":"inside"}],"objectives":[{"commitments":[{"admission_id":"a1","committed":"700000000","gate":{"qualified":false,"reasons":["NoGateEvidence"]},"kind":"at_least","state":"retired_resumable"}],"objective":"o","owned_closed":{"high":{"kind":"at_least","nano":"300000000"},"low":"15000000"},"owned_outstanding":{"high":{"kind":"exact","nano":"0"},"low":"0"},"predecessor_obligations":{"high":{"kind":"exact","nano":"0"},"low":"0"},"provisional":false,"reasons":["UnenforcedCommitment","UnresolvedExposure"],"status":"not_established"}]},"name":"default","objectives":[{"admissions":[{"admission_id":"a1","rows":[{"admission_id":"a1","at":"2026-11-10T06:00:05Z","event":"pending","max_usd":1.0},{"admission_id":"a1","at":"2026-11-10T06:00:10Z","event":"launched","session_id":"00000000-0000-4000-8000-0000000000c1"}],"session":"00000000-0000-4000-8000-0000000000c1","source":"journal"}],"closed_at":null,"limit":"1000000000","objective":"o","predecessor":null,"started_at":"2026-11-10T06:00:00Z","tree_sessions":["00000000-0000-4000-8000-0000000000c1"]}]}],"expected":{"attempts":[{"billing":"unresolved","blocking":["UnresolvedExposure"],"correlation":{"kind":"exact"},"diagnostics":["PartialUsage"],"evidence":["S1/turn-1:1","S1/turn-1:2"],"execution":"terminal","exposure":{"high":{"basis":"proxy_reservation","kind":"at_least","nano":"300000000"},"low":{"basis":"usage_valuation","nano":"15000000"},"pool":"pi_api"},"forwarding":"forwarded","key":"res:S1/turn-1:rid=1","obligation":{"amount_source":"reservation_confirmed","state":"closed"},"replacement":{"eligible":false,"reason":"NotUsageSettlement"},"settlement":{"basis":"reservation","charged":"300000000","decision":"valid","reasons":[],"reserved":"300000000"},"valuation":{"complete":false,"derivations":[{"for":"both","model":"gpt-6-astra","nano":"15000000","provider":"openai","tier_multiplier":"1","usage":{"cache_write_tokens":0,"cached_tokens":0,"input_tokens":1000,"output_tokens":100}}],"high":"15000000","identities":["gpt-6-astra"],"low":"15000000","state":"valued"}}],"inventory":[],"lifecycle":{"S1":{"clock":"trusted","reopened":false,"state":"retired"}},"non_attempt":[],"unreadable":[]},"historical":{"a134_086bdec1":{"blocking_kinds":[],"decision":"admitted","evaluation":"default","note":"comparison only; A1.3.4 has no ownership (D1) concept: it attributes by session tree and time window","reader_sha256":"086bdec1d443064f6847821858f34bbf227f8804a9a5fc8a362416f62b29d621","reasons":["within the objective pi_api limit"],"spent_high_nano":"300000000","spent_low_nano":"15000000"}},"inputs":{"rates":{"anthropic_api_rates.json":"8a286433d9e08b37533f2f908f2db60c4357e1a566018c9ca8364937652bdef5","openai_api_rates.json":"c0bdfd66ee69f1644eaf188f87f6af5e47beacc0739d20aac5846c923d66fd58"},"reference_time":"2026-11-10T07:00:00Z","sessions":[{"alias":"S1","launch":{"auth_lane":"api","model":"openai/gpt-6-astra","parent":"root"},"lifecycle":[{"at":"2026-11-10T06:00:10Z","event":"launch","turn":1},{"at":"2026-11-10T06:50:00Z","event":"retire"}],"logs":{"turn-1":{"final_newline":true,"lines":["{\"at\": \"2026-11-10T06:30:00Z\", \"event\": \"usd_reserved\", \"n\": 1, \"rid\": 1, \"reserved_usd\": 0.3, \"input_bound\": 6000, \"bound_basis\": \"bytes\"}","{\"at\": \"2026-11-10T06:30:02Z\", \"event\": \"request\", \"n\": 1, \"http_status\": 200, \"model\": \"gpt-6-astra\", \"requested_tier\": null, \"observed_tier\": \"default\", \"terminal\": null, \"response_id\": \"resp_1\", \"response_model\": \"gpt-6-astra\", \"usage\": {\"input_tokens\": 1000, \"output_tokens\": 100, \"cached_tokens\": 0, \"cache_write_tokens\": 0, \"reasoning_tokens\": 0}, \"seconds\": 1.0, \"usd\": {\"rid\": 1, \"reserved_usd\": 0.3, \"charged_usd\": 0.3, \"charge_basis\": \"reservation\", \"session_spent_usd\": 0.3, \"multiplier_floor\": 0.0}}"]}},"session_id":"00000000-0000-4000-8000-0000000000c1"}]},"invariants":["INV-F11","INV-F3"],"profile":"current_producer","provenance":{"origin":"contract::producer: terminal null -> settle(None) keeps the reservation (pi_container.py:1571, :1014)"},"reachability":"producer_reachable","role":"valid_control","schema":"pi-accounting-corpus/1"}
    5| {"ambiguity":null,"case_id":"VC04-upstream-error-keeps-reservation","class":"producer_valid","contract":"pi-accounting-contract/1.4-s1","controls":[],"evaluations":[{"admission_request":{"new":"100000000","objective":"o"},"expected":{"admission":{"decision":"refused","limit":"1000000000","new":"100000000","objective":"o"},"attribution":[{"attempt":"res:S1/turn-1:rid=1","objective":"o","ownership":"owned","placement":"inside"}],"objectives":[{"commitments":[{"admission_id":"a1","committed":"700000000","gate":{"qualified":false,"reasons":["NoGateEvidence"]},"kind":"at_least","state":"retired_resumable"}],"objective":"o","owned_closed":{"high":{"kind":"at_least","nano":"300000000"},"low":"0"},"owned_outstanding":{"high":{"kind":"exact","nano":"0"},"low":"0"},"predecessor_obligations":{"high":{"kind":"exact","nano":"0"},"low":"0"},"provisional":false,"reasons":["UnenforcedCommitment","UnresolvedExposure"],"status":"not_established"}]},"name":"default","objectives":[{"admissions":[{"admission_id":"a1","rows":[{"admission_id":"a1","at":"2026-11-10T06:00:05Z","event":"pending","max_usd":1.0},{"admission_id":"a1","at":"2026-11-10T06:00:10Z","event":"launched","session_id":"00000000-0000-4000-8000-0000000000c1"}],"session":"00000000-0000-4000-8000-0000000000c1","source":"journal"}],"closed_at":null,"limit":"1000000000","objective":"o","predecessor":null,"started_at":"2026-11-10T06:00:00Z","tree_sessions":["00000000-0000-4000-8000-0000000000c1"]}]}],"expected":{"attempts":[{"billing":"unresolved","blocking":["UnresolvedExposure"],"correlation":{"kind":"exact"},"diagnostics":[],"evidence":["S1/turn-1:1","S1/turn-1:2"],"execution":"terminal","exposure":{"high":{"basis":"proxy_reservation","kind":"at_least","nano":"300000000"},"low":{"basis":"none","nano":"0"},"pool":"pi_api"},"forwarding":"unknown","key":"res:S1/turn-1:rid=1","obligation":{"amount_source":"reservation_confirmed","state":"closed"},"replacement":{"eligible":false,"reason":"NotUsageSettlement"},"settlement":{"basis":"upstream_error","charged":"300000000","decision":"valid","reasons":[],"reserved":"300000000"},"valuation":{"state":"not_reported"}}],"inventory":[],"lifecycle":{"S1":{"clock":"trusted","reopened":false,"state":"retired"}},"non_attempt":[],"unreadable":[]},"historical":{"a134_086bdec1":{"blocking_kinds":[],"decision":"admitted","evaluation":"default","note":"comparison only; A1.3.4 has no ownership (D1) concept: it attributes by session tree and time window","reader_sha256":"086bdec1d443064f6847821858f34bbf227f8804a9a5fc8a362416f62b29d621","reasons":["within the objective pi_api limit"],"spent_high_nano":"300000000","spent_low_nano":"0"}},"inputs":{"rates":{"anthropic_api_rates.json":"8a286433d9e08b37533f2f908f2db60c4357e1a566018c9ca8364937652bdef5","openai_api_rates.json":"c0bdfd66ee69f1644eaf188f87f6af5e47beacc0739d20aac5846c923d66fd58"},"reference_time":"2026-11-10T07:00:00Z","sessions":[{"alias":"S1","launch":{"auth_lane":"api","model":"openai/gpt-6-astra","parent":"root"},"lifecycle":[{"at":"2026-11-10T06:00:10Z","event":"launch","turn":1},{"at":"2026-11-10T06:50:00Z","event":"retire"}],"logs":{"turn-1":{"final_newline":true,"lines":["{\"at\": \"2026-11-10T06:30:00Z\", \"event\": \"usd_reserved\", \"n\": 1, \"rid\": 1, \"reserved_usd\": 0.3, \"input_bound\": 6000, \"bound_basis\": \"bytes\"}","{\"at\": \"2026-11-10T06:30:02Z\", \"event\": \"upstream_error\", \"n\": 1, \"detail\": \"connection reset\", \"usd\": {\"rid\": 1, \"reserved_usd\": 0.3, \"charged_usd\": 0.3, \"charge_basis\": \"reservation\", \"session_spent_usd\": 0.3, \"multiplier_floor\": 0.0}}"]}},"session_id":"00000000-0000-4000-8000-0000000000c1"}]},"invariants":["INV-F1","INV-F3"],"profile":"current_producer","provenance":{"origin":"repro_p111.py::A1-01","variant":"control"},"reachability":"producer_reachable","role":"valid_control","schema":"pi-accounting-corpus/1"}
    6| {"ambiguity":null,"case_id":"VC05-unsettled-reservation","class":"producer_valid","contract":"pi-accounting-contract/1.4-s1","controls":[],"evaluations":[{"admission_request":{"new":"100000000","objective":"o"},"expected":{"admission":{"decision":"refused","limit":"1000000000","new":"100000000","objective":"o"},"attribution":[{"attempt":"res:S1/turn-1:rid=1","objective":"o","ownership":"owned","placement":"inside"}],"objectives":[{"commitments":[{"admission_id":"a1","committed":"700000000","gate":{"qualified":false,"reasons":["NoGateEvidence"]},"kind":"at_least","state":"retired_resumable"}],"objective":"o","owned_closed":{"high":{"kind":"exact","nano":"0"},"low":"0"},"owned_outstanding":{"high":{"kind":"at_least","nano":"300000000"},"low":"0"},"predecessor_obligations":{"high":{"kind":"exact","nano":"0"},"low":"0"},"provisional":false,"reasons":["UnenforcedCommitment","UnresolvedExposure"],"status":"not_established"}]},"name":"default","objectives":[{"admissions":[{"admission_id":"a1","rows":[{"admission_id":"a1","at":"2026-11-10T06:00:05Z","event":"pending","max_usd":1.0},{"admission_id":"a1","at":"2026-11-10T06:00:10Z","event":"launched","session_id":"00000000-0000-4000-8000-0000000000c1"}],"session":"00000000-0000-4000-8000-0000000000c1","source":"journal"}],"closed_at":null,"limit":"1000000000","objective":"o","predecessor":null,"started_at":"2026-11-10T06:00:00Z","tree_sessions":["00000000-0000-4000-8000-0000000000c1"]}]}],"expected":{"attempts":[{"billing":"unresolved","blocking":["UnresolvedExposure"],"correlation":{"kind":"exact"},"diagnostics":[],"evidence":["S1/turn-1:1"],"execution":"not_terminal","exposure":{"high":{"basis":"proxy_reservation","kind":"at_least","nano":"300000000"},"low":{"basis":"none","nano":"0"},"pool":"pi_api"},"forwarding":"unknown","key":"res:S1/turn-1:rid=1","obligation":{"amount_source":"reservation_bound","state":"outstanding"},"replacement":{"eligible":false,"reason":"NotUsageSettlement"},"settlement":{"decision":"absent","reasons":[],"reserved":"300000000"},"valuation":{"state":"not_reported"}}],"inventory":[],"lifecycle":{"S1":{"clock":"trusted","reopened":false,"state":"retired"}},"non_attempt":[],"unreadable":[]},"historical":{"a134_086bdec1":{"blocking_kinds":[],"decision":"admitted","evaluation":"default","note":"comparison only; A1.3.4 has no ownership (D1) concept: it attributes by session tree and time window","reader_sha256":"086bdec1d443064f6847821858f34bbf227f8804a9a5fc8a362416f62b29d621","reasons":["within the objective pi_api limit"],"spent_high_nano":"300000000","spent_low_nano":"0"}},"inputs":{"rates":{"anthropic_api_rates.json":"8a286433d9e08b37533f2f908f2db60c4357e1a566018c9ca8364937652bdef5","openai_api_rates.json":"c0bdfd66ee69f1644eaf188f87f6af5e47beacc0739d20aac5846c923d66fd58"},"reference_time":"2026-11-10T07:00:00Z","sessions":[{"alias":"S1","launch":{"auth_lane":"api","model":"openai/gpt-6-astra","parent":"root"},"lifecycle":[{"at":"2026-11-10T06:00:10Z","event":"launch","turn":1},{"at":"2026-11-10T06:50:00Z","event":"retire"}],"logs":{"turn-1":{"final_newline":true,"lines":["{\"at\": \"2026-11-10T06:30:00Z\", \"event\": \"usd_reserved\", \"n\": 1, \"rid\": 1, \"reserved_usd\": 0.3, \"input_bound\": 6000, \"bound_basis\": \"bytes\"}"]}},"session_id":"00000000-0000-4000-8000-0000000000c1"}]},"invariants":["INV-F3","INV-T3"],"profile":"current_producer","provenance":{"origin":"contract::killed stream: reservation written, handler never settled"},"reachability":"producer_reachable","role":"valid_control","schema":"pi-accounting-corpus/1"}
    7| {"ambiguity":null,"case_id":"C3-identical-reservation-copy","class":"malformed","contract":"pi-accounting-contract/1.4-s1","controls":["S02-control-error-status-after-reservation"],"evaluations":[{"admission_request":{"new":"100000000","objective":"o"},"expected":{"admission":{"decision":"refused","limit":"1000000000","new":"100000000","objective":"o"},"attribution":[{"attempt":"res:S1/turn-1:rid=1","objective":"o","ownership":"owned","placement":"inside"}],"objectives":[{"commitments":[{"admission_id":"a1","committed":"200000000","gate":{"qualified":false,"reasons":["NoGateEvidence"]},"kind":"at_least","state":"retired_resumable"}],"objective":"o","owned_closed":{"high":{"kind":"at_least","nano":"800000000"},"low":"0"},"owned_outstanding":{"high":{"kind":"exact","nano":"0"},"low":"0"},"predecessor_obligations":{"high":{"kind":"exact","nano":"0"},"low":"0"},"provisional":false,"reasons":["UnenforcedCommitment","UnresolvedExposure"],"status":"not_established"}]},"name":"default","objectives":[{"admissions":[{"admission_id":"a1","rows":[{"admission_id":"a1","at":"2026-11-10T06:00:05Z","event":"pending","max_usd":1.0},{"admission_id":"a1","at":"2026-11-10T06:00:10Z","event":"launched","session_id":"00000000-0000-4000-8000-0000000000c1"}],"session":"00000000-0000-4000-8000-0000000000c1","source":"journal"}],"closed_at":null,"limit":"1000000000","objective":"o","predecessor":null,"started_at":"2026-11-10T06:00:00Z","tree_sessions":["00000000-0000-4000-8000-0000000000c1"]}]}],"expected":{"attempts":[{"billing":"unresolved","blocking":["UnresolvedExposure"],"correlation":{"kind":"exact"},"diagnostics":["ErrorStatusSettlement","NonBillingUnverified","RepeatedIdenticalRow"],"evidence":["S1/turn-1:1","S1/turn-1:2","S1/turn-1:3"],"execution":"terminal","exposure":{"high":{"basis":"proxy_reservation","kind":"at_least","nano":"800000000"},"low":{"basis":"none","nano":"0"},"pool":"pi_api"},"forwarding":"forwarded","key":"res:S1/turn-1:rid=1","obligation":{"amount_source":"reservation_bound","state":"closed"},"replacement":{"eligible":false,"reason":"NotUsageSettlement"},"settlement":{"basis":"error_status","charged":"0","decision":"valid","reasons":[],"reserved":"800000000"},"valuation":{"state":"not_reported"}}],"inventory":[],"lifecycle":{"S1":{"clock":"trusted","reopened":false,"state":"retired"}},"non_attempt":[],"unreadable":[]},"historical":{"a134_086bdec1":{"blocking_kinds":[],"decision":"admitted","evaluation":"default","note":"comparison only; A1.3.4 has no ownership (D1) concept: it attributes by session tree and time window","reader_sha256":"086bdec1d443064f6847821858f34bbf227f8804a9a5fc8a362416f62b29d621","reasons":["within the objective pi_api limit"],"spent_high_nano":"0","spent_low_nano":"0"}},"inputs":{"rates":{"anthropic_api_rates.json":"8a286433d9e08b37533f2f908f2db60c4357e1a566018c9ca8364937652bdef5","openai_api_rates.json":"c0bdfd66ee69f1644eaf188f87f6af5e47beacc0739d20aac5846c923d66fd58"},"reference_time":"2026-11-10T07:00:00Z","sessions":[{"alias":"S1","launch":{"auth_lane":"api","model":"openai/gpt-6-astra","parent":"root"},"lifecycle":[{"at":"2026-11-10T06:00:10Z","event":"launch","turn":1},{"at":"2026-11-10T06:50:00Z","event":"retire"}],"logs":{"turn-1":{"final_newline":true,"lines":["{\"at\": \"2026-11-10T06:30:00Z\", \"event\": \"usd_reserved\", \"n\": 1, \"rid\": 1, \"reserved_usd\": 0.8, \"input_bound\": 6000, \"bound_basis\": \"bytes\"}","{\"at\": \"2026-11-10T06:30:00Z\", \"event\": \"request\", \"n\": 1, \"http_status\": 429, \"model\": \"gpt-6-astra\", \"requested_tier\": null, \"observed_tier\": \"default\", \"terminal\": null, \"response_id\": null, \"response_model\": \"gpt-6-astra\", \"usage\": {\"input_tokens\": null, \"output_tokens\": null, \"cached_tokens\": null, \"cache_write_tokens\": null, \"reasoning_tokens\": 0}, \"seconds\": 1.0, \"usd\": {\"rid\": 1, \"reserved_usd\": 0.8, \"charged_usd\": 0.0, \"charge_basis\": \"error_status\", \"session_spent_usd\": 0.0, \"multiplier_floor\": 0.0}}","{\"at\": \"2026-11-10T06:30:00Z\", \"event\": \"usd_reserved\", \"n\": 1, \"rid\": 1, \"reserved_usd\": 0.8, \"input_bound\": 6000, \"bound_basis\": \"bytes\"}"]}},"session_id":"00000000-0000-4000-8000-0000000000c1"}]},"invariants":["INV-F2","INV-F6"],"profile":"current_producer","provenance":{"origin":"P1.19 report::C3: exact reservation replay after settlement"},"reachability":"producer_unreachable","role":"finding","schema":"pi-accounting-corpus/1"}
    8| {"ambiguity":null,"case_id":"R04-reused-reservation-id","class":"malformed","contract":"pi-accounting-contract/1.4-s1","controls":["VC05-unsettled-reservation"],"evaluations":[{"admission_request":{"new":"100000000","objective":"o"},"expected":{"admission":{"decision":"refused","limit":"1000000000","new":"100000000","objective":"o"},"attribution":[{"attempt":"res:S1/turn-1:rid=1","objective":"o","ownership":"owned","placement":"inside"}],"objectives":[{"commitments":[{"admission_id":"a1","committed":"200000000","gate":{"qualified":false,"reasons":["NoGateEvidence"]},"kind":"at_least","state":"retired_resumable"}],"objective":"o","owned_closed":{"high":{"kind":"exact","nano":"0"},"low":"0"},"owned_outstanding":{"high":{"kind":"at_least","nano":"800000000"},"low":"0"},"predecessor_obligations":{"high":{"kind":"exact","nano":"0"},"low":"0"},"provisional":false,"reasons":["ReservationConflict","UnenforcedCommitment","UnresolvedExposure"],"status":"not_established"}]},"name":"default","objectives":[{"admissions":[{"admission_id":"a1","rows":[{"admission_id":"a1","at":"2026-11-10T06:00:05Z","event":"pending","max_usd":1.0},{"admission_id":"a1","at":"2026-11-10T06:00:10Z","event":"launched","session_id":"00000000-0000-4000-8000-0000000000c1"}],"session":"00000000-0000-4000-8000-0000000000c1","source":"journal"}],"closed_at":null,"limit":"1000000000","objective":"o","predecessor":null,"started_at":"2026-11-10T06:00:00Z","tree_sessions":["00000000-0000-4000-8000-0000000000c1"]}]}],"expected":{"attempts":[{"billing":"unresolved","blocking":["ReservationConflict","UnresolvedExposure"],"correlation":{"kind":"exact"},"diagnostics":[],"evidence":["S1/turn-1:1","S1/turn-1:2"],"execution":"not_terminal","exposure":{"high":{"basis":"proxy_reservation","kind":"at_least","nano":"800000000"},"low":{"basis":"none","nano":"0"},"pool":"pi_api"},"forwarding":"unknown","key":"res:S1/turn-1:rid=1","obligation":{"amount_source":"reservation_bound","state":"outstanding"},"replacement":{"eligible":false,"reason":"NotUsageSettlement"},"settlement":{"decision":"absent","reasons":[]},"valuation":{"state":"not_reported"}}],"inventory":[],"lifecycle":{"S1":{"clock":"trusted","reopened":false,"state":"retired"}},"non_attempt":[],"unreadable":[]},"historical":{"a134_086bdec1":{"blocking_kinds":["pi_reservation_conflict"],"decision":"not_established","evaluation":"default","note":"comparison only; A1.3.4 has no ownership (D1) concept: it attributes by session tree and time window","reader_sha256":"086bdec1d443064f6847821858f34bbf227f8804a9a5fc8a362416f62b29d621","reasons":["objective accounting unavailable, unpriced or inconsistent"],"spent_high_nano":null,"spent_low_nano":null}},"inputs":{"rates":{"anthropic_api_rates.json":"8a286433d9e08b37533f2f908f2db60c4357e1a566018c9ca8364937652bdef5","openai_api_rates.json":"c0bdfd66ee69f1644eaf188f87f6af5e47beacc0739d20aac5846c923d66fd58"},"reference_time":"2026-11-10T07:00:00Z","sessions":[{"alias":"S1","launch":{"auth_lane":"api","model":"openai/gpt-6-astra","parent":"root"},"lifecycle":[{"at":"2026-11-10T06:00:10Z","event":"launch","turn":1},{"at":"2026-11-10T06:50:00Z","event":"retire"}],"logs":{"turn-1":{"final_newline":true,"lines":["{\"at\": \"2026-11-10T06:30:00Z\", \"event\": \"usd_reserved\", \"n\": 1, \"rid\": 1, \"reserved_usd\": 0.8, \"input_bound\": 6000, \"bound_basis\": \"bytes\"}","{\"at\": \"2026-11-10T06:30:01Z\", \"event\": \"usd_reserved\", \"n\": 2, \"rid\": 1, \"reserved_usd\": 0.1, \"input_bound\": 6000, \"bound_basis\": \"bytes\"}"]}},"session_id":"00000000-0000-4000-8000-0000000000c1"}]},"invariants":["INV-F2","INV-F3"],"profile":"current_producer","provenance":{"origin":"repro_p113.py::test_R04"},"reachability":"producer_unreachable","role":"finding","schema":"pi-accounting-corpus/1"}
    9| {"added_in":"1.2-s1","ambiguity":null,"case_id":"P130-R06-reservation-rid-missing","class":"malformed","contract":"pi-accounting-contract/1.4-s1","controls":["VC05-unsettled-reservation"],"evaluations":[{"admission_request":{"new":"100000000","objective":"o"},"expected":{"admission":{"decision":"refused","limit":"1000000000","new":"100000000","objective":"o"},"attribution":[{"attempt":"row:S1/turn-1:line=1","objective":"o","ownership":"owned","placement":"inside"}],"objectives":[{"commitments":[{"admission_id":"a1","committed":"700000000","gate":{"qualified":false,"reasons":["NoGateEvidence"]},"kind":"at_least","state":"retired_resumable"}],"objective":"o","owned_closed":{"high":{"kind":"exact","nano":"0"},"low":"0"},"owned_outstanding":{"high":{"kind":"at_least","nano":"300000000"},"low":"0"},"predecessor_obligations":{"high":{"kind":"exact","nano":"0"},"low":"0"},"provisional":false,"reasons":["MalformedEvidence","UnaccountedAttempt","UnenforcedCommitment","UnresolvedExposure"],"status":"not_established"}]},"name":"default","objectives":[{"admissions":[{"admission_id":"a1","rows":[{"admission_id":"a1","at":"2026-11-10T06:00:05Z","event":"pending","max_usd":1.0},{"admission_id":"a1","at":"2026-11-10T06:00:10Z","event":"launched","session_id":"00000000-0000-4000-8000-0000000000c1"}],"session":"00000000-0000-4000-8000-0000000000c1","source":"journal"}],"closed_at":null,"limit":"1000000000","objective":"o","predecessor":null,"started_at":"2026-11-10T06:00:00Z","tree_sessions":["00000000-0000-4000-8000-0000000000c1"]}]}],"expected":{"attempts":[{"billing":"unresolved","blocking":["MalformedEvidence","UnaccountedAttempt","UnresolvedExposure"],"correlation":{"kind":"unattributed"},"diagnostics":[],"evidence":["S1/turn-1:1"],"execution":"not_terminal","exposure":{"high":{"basis":"proxy_reservation","kind":"at_least","nano":"300000000"},"low":{"basis":"none","nano":"0"},"pool":"pi_api"},"forwarding":"unknown","key":"row:S1/turn-1:line=1","obligation":{"amount_source":"reservation_bound","state":"outstanding"},"replacement":{"eligible":false,"reason":"NotUsageSettlement"},"settlement":{"decision":"absent","reasons":[],"reserved":"300000000"},"valuation":{"state":"not_reported"}}],"inventory":[],"lifecycle":{"S1":{"clock":"trusted","reopened":false,"state":"retired"}},"non_attempt":[],"unreadable":[]},"historical":{"a134_086bdec1":{"blocking_kinds":["malformed_pi_reservation"],"decision":"not_established","evaluation":"default","note":"comparison only; A1.3.4 has no ownership (D1) concept: it attributes by session tree and time window","reader_sha256":"086bdec1d443064f6847821858f34bbf227f8804a9a5fc8a362416f62b29d621","reasons":["objective accounting unavailable, unpriced or inconsistent"],"spent_high_nano":null,"spent_low_nano":null}},"inputs":{"rates":{"anthropic_api_rates.json":"8a286433d9e08b37533f2f908f2db60c4357e1a566018c9ca8364937652bdef5","openai_api_rates.json":"c0bdfd66ee69f1644eaf188f87f6af5e47beacc0739d20aac5846c923d66fd58"},"reference_time":"2026-11-10T07:00:00Z","sessions":[{"alias":"S1","launch":{"auth_lane":"api","model":"openai/gpt-6-astra","parent":"root"},"lifecycle":[{"at":"2026-11-10T06:00:10Z","event":"launch","turn":1},{"at":"2026-11-10T06:50:00Z","event":"retire"}],"logs":{"turn-1":{"final_newline":true,"lines":["{\"at\": \"2026-11-10T06:30:00Z\", \"event\": \"usd_reserved\", \"n\": 1, \"reserved_usd\": 0.3, \"input_bound\": 6000, \"bound_basis\": \"bytes\"}"]}},"session_id":"00000000-0000-4000-8000-0000000000c1"}]},"invariants":["INV-F6"],"profile":"current_producer","provenance":{"origin":"contract::C-1 unusable rid (P1.29 R-06)"},"reachability":"producer_unreachable","role":"finding","schema":"pi-accounting-corpus/1"}
   10| {"added_in":"1.2-s1","ambiguity":null,"case_id":"P130-R06-reservation-amount-malformed","class":"malformed","contract":"pi-accounting-contract/1.4-s1","controls":["VC05-unsettled-reservation"],"evaluations":[{"admission_request":{"new":"100000000","objective":"o"},"expected":{"admission":{"decision":"refused","limit":"1000000000","new":"100000000","objective":"o"},"attribution":[{"attempt":"res:S1/turn-1:rid=1","objective":"o","ownership":"owned","placement":"inside"}],"objectives":[{"commitments":[{"admission_id":"a1","committed":"1000000000","gate":{"qualified":false,"reasons":["NoGateEvidence"]},"kind":"at_least","state":"retired_resumable"}],"objective":"o","owned_closed":{"high":{"kind":"exact","nano":"0"},"low":"0"},"owned_outstanding":{"high":{"kind":"at_least","nano":"0"},"low":"0"},"predecessor_obligations":{"high":{"kind":"exact","nano":"0"},"low":"0"},"provisional":false,"reasons":["MalformedEvidence","UnenforcedCommitment","UnresolvedExposure"],"status":"not_established"}]},"name":"default","objectives":[{"admissions":[{"admission_id":"a1","rows":[{"admission_id":"a1","at":"2026-11-10T06:00:05Z","event":"pending","max_usd":1.0},{"admission_id":"a1","at":"2026-11-10T06:00:10Z","event":"launched","session_id":"00000000-0000-4000-8000-0000000000c1"}],"session":"00000000-0000-4000-8000-0000000000c1","source":"journal"}],"closed_at":null,"limit":"1000000000","objective":"o","predecessor":null,"started_at":"2026-11-10T06:00:00Z","tree_sessions":["00000000-0000-4000-8000-0000000000c1"]}]}],"expected":{"attempts":[{"billing":"unresolved","blocking":["MalformedEvidence","UnresolvedExposure"],"correlation":{"kind":"exact"},"diagnostics":[],"evidence":["S1/turn-1:1"],"execution":"not_terminal","exposure":{"high":{"basis":"none","kind":"at_least","nano":"0"},"low":{"basis":"none","nano":"0"},"pool":"pi_api"},"forwarding":"unknown","key":"res:S1/turn-1:rid=1","obligation":{"amount_source":"reservation_bound","state":"outstanding"},"replacement":{"eligible":false,"reason":"NotUsageSettlement"},"settlement":{"decision":"absent","reasons":[]},"valuation":{"state":"not_reported"}}],"inventory":[],"lifecycle":{"S1":{"clock":"trusted","reopened":false,"state":"retired"}},"non_attempt":[],"unreadable":[]},"historical":{"a134_086bdec1":{"blocking_kinds":["malformed_pi_reservation"],"decision":"not_established","evaluation":"default","note":"comparison only; A1.3.4 has no ownership (D1) concept: it attributes by session tree and time window","reader_sha256":"086bdec1d443064f6847821858f34bbf227f8804a9a5fc8a362416f62b29d621","reasons":["objective accounting unavailable, unpriced or inconsistent"],"spent_high_nano":null,"spent_low_nano":null}},"inputs":{"rates":{"anthropic_api_rates.json":"8a286433d9e08b37533f2f908f2db60c4357e1a566018c9ca8364937652bdef5","openai_api_rates.json":"c0bdfd66ee69f1644eaf188f87f6af5e47beacc0739d20aac5846c923d66fd58"},"reference_time":"2026-11-10T07:00:00Z","sessions":[{"alias":"S1","launch":{"auth_lane":"api","model":"openai/gpt-6-astra","parent":"root"},"lifecycle":[{"at":"2026-11-10T06:00:10Z","event":"launch","turn":1},{"at":"2026-11-10T06:50:00Z","event":"retire"}],"logs":{"turn-1":{"final_newline":true,"lines":["{\"at\": \"2026-11-10T06:30:00Z\", \"event\": \"usd_reserved\", \"n\": 1, \"rid\": 1, \"reserved_usd\": \"0.3\", \"input_bound\": 6000, \"bound_basis\": \"bytes\"}"]}},"session_id":"00000000-0000-4000-8000-0000000000c1"}]},"invariants":["INV-F5"],"profile":"current_producer","provenance":{"origin":"contract::C-1, X-0 malformed reserved (P1.29 R-06)"},"reachability":"producer_unreachable","role":"finding","schema":"pi-accounting-corpus/1"}
   11| {"ambiguity":null,"case_id":"S02-settlement-before-reservation","class":"malformed","contract":"pi-accounting-contract/1.4-s1","controls":["S02-control-error-status-after-reservation"],"evaluations":[{"admission_request":{"new":"100000000","objective":"o"},"expected":{"admission":{"decision":"refused","limit":"1000000000","new":"100000000","objective":"o"},"attribution":[{"attempt":"res:S1/turn-1:rid=1","objective":"o","ownership":"owned","placement":"inside"}],"objectives":[{"commitments":[{"admission_id":"a1","committed":"200000000","gate":{"qualified":false,"reasons":["NoGateEvidence"]},"kind":"at_least","state":"retired_resumable"}],"objective":"o","owned_closed":{"high":{"kind":"exact","nano":"0"},"low":"0"},"owned_outstanding":{"high":{"kind":"at_least","nano":"800000000"},"low":"0"},"predecessor_obligations":{"high":{"kind":"exact","nano":"0"},"low":"0"},"provisional":false,"reasons":["SettlementRejected","UnenforcedCommitment","UnresolvedExposure"],"status":"not_established"}]},"name":"default","objectives":[{"admissions":[{"admission_id":"a1","rows":[{"admission_id":"a1","at":"2026-11-10T06:00:05Z","event":"pending","max_usd":1.0},{"admission_id":"a1","at":"2026-11-10T06:00:10Z","event":"launched","session_id":"00000000-0000-4000-8000-0000000000c1"}],"session":"00000000-0000-4000-8000-0000000000c1","source":"journal"}],"closed_at":null,"limit":"1000000000","objective":"o","predecessor":null,"started_at":"2026-11-10T06:00:00Z","tree_sessions":["00000000-0000-4000-8000-0000000000c1"]}]}],"expected":{"attempts":[{"billing":"unresolved","blocking":["SettlementRejected","UnresolvedExposure"],"correlation":{"kind":"exact"},"diagnostics":[],"evidence":["S1/turn-1:1","S1/turn-1:2"],"execution":"terminal","exposure":{"high":{"basis":"proxy_reservation","kind":"at_least","nano":"800000000"},"low":{"basis":"none","nano":"0"},"pool":"pi_api"},"forwarding":"forwarded","key":"res:S1/turn-1:rid=1","obligation":{"amount_source":"reservation_bound","state":"outstanding"},"replacement":{"eligible":false,"reason":"NotUsageSettlement"},"settlement":{"charged":"0","decision":"rejected","reasons":["Order"],"reserved":"800000000"},"valuation":{"state":"not_reported"}}],"inventory":[],"lifecycle":{"S1":{"clock":"trusted","reopened":false,"state":"retired"}},"non_attempt":[],"unreadable":[]},"historical":{"a134_086bdec1":{"blocking_kinds":["pi_settlement_invalid"],"decision":"not_established","evaluation":"default","note":"comparison only; A1.3.4 has no ownership (D1) concept: it attributes by session tree and time window","reader_sha256":"086bdec1d443064f6847821858f34bbf227f8804a9a5fc8a362416f62b29d621","reasons":["objective accounting unavailable, unpriced or inconsistent"],"spent_high_nano":null,"spent_low_nano":null}},"inputs":{"rates":{"anthropic_api_rates.json":"8a286433d9e08b37533f2f908f2db60c4357e1a566018c9ca8364937652bdef5","openai_api_rates.json":"c0bdfd66ee69f1644eaf188f87f6af5e47beacc0739d20aac5846c923d66fd58"},"reference_time":"2026-11-10T07:00:00Z","sessions":[{"alias":"S1","launch":{"auth_lane":"api","model":"openai/gpt-6-astra","parent":"root"},"lifecycle":[{"at":"2026-11-10T06:00:10Z","event":"launch","turn":1},{"at":"2026-11-10T06:50:00Z","event":"retire"}],"logs":{"turn-1":{"final_newline":true,"lines":["{\"at\": \"2026-11-10T06:30:00Z\", \"event\": \"request\", \"n\": 1, \"http_status\": 429, \"model\": \"gpt-6-astra\", \"requested_tier\": null, \"observed_tier\": \"default\", \"terminal\": null, \"response_id\": null, \"response_model\": \"gpt-6-astra\", \"usage\": {\"input_tokens\": null, \"output_tokens\": null, \"cached_tokens\": null, \"cache_write_tokens\": null, \"reasoning_tokens\": 0}, \"seconds\": 1.0, \"usd\": {\"rid\": 1, \"reserved_usd\": 0.8, \"charged_usd\": 0.0, \"charge_basis\": \"error_status\", \"session_spent_usd\": 0.0, \"multiplier_floor\": 0.0}}","{\"at\": \"2026-11-10T06:30:00Z\", \"event\": \"usd_reserved\", \"n\": 1, \"rid\": 1, \"reserved_usd\": 0.8, \"input_bound\": 6000, \"bound_basis\": \"bytes\"}"]}},"session_id":"00000000-0000-4000-8000-0000000000c1"}]},"invariants":["INV-F1","INV-F3"],"profile":"current_producer","provenance":{"origin":"repro_p115.py::test_S02"},"reachability":"producer_unreachable","role":"finding","schema":"pi-accounting-corpus/1"}
   12| {"ambiguity":null,"case_id":"R02-string-charge","class":"malformed","contract":"pi-accounting-contract/1.4-s1","controls":["S02-control-error-status-after-reservation"],"evaluations":[{"admission_request":{"new":"100000000","objective":"o"},"expected":{"admission":{"decision":"refused","limit":"1000000000","new":"100000000","objective":"o"},"attribution":[{"attempt":"res:S1/turn-1:rid=1","objective":"o","ownership":"owned","placement":"inside"}],"objectives":[{"commitments":[{"admission_id":"a1","committed":"200000000","gate":{"qualified":false,"reasons":["NoGateEvidence"]},"kind":"at_least","state":"retired_resumable"}],"objective":"o","owned_closed":{"high":{"kind":"exact","nano":"0"},"low":"0"},"owned_outstanding":{"high":{"kind":"at_least","nano":"800000000"},"low":"0"},"predecessor_obligations":{"high":{"kind":"exact","nano":"0"},"low":"0"},"provisional":false,"reasons":["SettlementRejected","UnenforcedCommitment","UnresolvedExposure"],"status":"not_established"}]},"name":"default","objectives":[{"admissions":[{"admission_id":"a1","rows":[{"admission_id":"a1","at":"2026-11-10T06:00:05Z","event":"pending","max_usd":1.0},{"admission_id":"a1","at":"2026-11-10T06:00:10Z","event":"launched","session_id":"00000000-0000-4000-8000-0000000000c1"}],"session":"00000000-0000-4000-8000-0000000000c1","source":"journal"}],"closed_at":null,"limit":"1000000000","objective":"o","predecessor":null,"started_at":"2026-11-10T06:00:00Z","tree_sessions":["00000000-0000-4000-8000-0000000000c1"]}]}],"expected":{"attempts":[{"billing":"unresolved","blocking":["SettlementRejected","UnresolvedExposure"],"correlation":{"kind":"exact"},"diagnostics":[],"evidence":["S1/turn-1:1","S1/turn-1:2"],"execution":"terminal","exposure":{"high":{"basis":"proxy_reservation","kind":"at_least","nano":"800000000"},"low":{"basis":"none","nano":"0"},"pool":"pi_api"},"forwarding":"forwarded","key":"res:S1/turn-1:rid=1","obligation":{"amount_source":"reservation_bound","state":"outstanding"},"replacement":{"eligible":false,"reason":"NotUsageSettlement"},"settlement":{"decision":"rejected","reasons":["AmountType"],"reserved":"800000000"},"valuation":{"state":"not_reported"}}],"inventory":[],"lifecycle":{"S1":{"clock":"trusted","reopened":false,"state":"retired"}},"non_attempt":[],"unreadable":[]},"historical":{"a134_086bdec1":{"blocking_kinds":["pi_settlement_invalid"],"decision":"not_established","evaluation":"default","note":"comparison only; A1.3.4 has no ownership (D1) concept: it attributes by session tree and time window","reader_sha256":"086bdec1d443064f6847821858f34bbf227f8804a9a5fc8a362416f62b29d621","reasons":["objective accounting unavailable, unpriced or inconsistent"],"spent_high_nano":null,"spent_low_nano":null}},"inputs":{"rates":{"anthropic_api_rates.json":"8a286433d9e08b37533f2f908f2db60c4357e1a566018c9ca8364937652bdef5","openai_api_rates.json":"c0bdfd66ee69f1644eaf188f87f6af5e47beacc0739d20aac5846c923d66fd58"},"reference_time":"2026-11-10T07:00:00Z","sessions":[{"alias":"S1","launch":{"auth_lane":"api","model":"openai/gpt-6-astra","parent":"root"},"lifecycle":[{"at":"2026-11-10T06:00:10Z","event":"launch","turn":1},{"at":"2026-11-10T06:50:00Z","event":"retire"}],"logs":{"turn-1":{"final_newline":true,"lines":["{\"at\": \"2026-11-10T06:30:00Z\", \"event\": \"usd_reserved\", \"n\": 1, \"rid\": 1, \"reserved_usd\": 0.8, \"input_bound\": 6000, \"bound_basis\": \"bytes\"}","{\"at\": \"2026-11-10T06:30:02Z\", \"event\": \"request\", \"n\": 1, \"http_status\": 429, \"model\": \"gpt-6-astra\", \"requested_tier\": null, \"observed_tier\": \"default\", \"terminal\": null, \"response_id\": null, \"response_model\": \"gpt-6-astra\", \"usage\": {\"input_tokens\": null, \"output_tokens\": null, \"cached_tokens\": null, \"cache_write_tokens\": null, \"reasoning_tokens\": 0}, \"seconds\": 1.0, \"usd\": {\"rid\": 1, \"reserved_usd\": 0.8, \"charged_usd\": \"0.8\", \"charge_basis\": \"error_status\", \"session_spent_usd\": \"0.8\", \"multiplier_floor\": 0.0}}"]}},"session_id":"00000000-0000-4000-8000-0000000000c1"}]},"invariants":["INV-F1","INV-F3"],"profile":"current_producer","provenance":{"origin":"repro_p113.py::test_R02_malformed_charge_cancels_a_reservation"},"reachability":"producer_unreachable","role":"finding","schema":"pi-accounting-corpus/1"}
   13| {"ambiguity":null,"case_id":"C4-missing-charge-basis","class":"malformed","contract":"pi-accounting-contract/1.4-s1","controls":["S02-control-error-status-after-reservation"],"evaluations":[{"admission_request":{"new":"100000000","objective":"o"},"expected":{"admission":{"decision":"refused","limit":"1000000000","new":"100000000","objective":"o"},"attribution":[{"attempt":"res:S1/turn-1:rid=1","objective":"o","ownership":"owned","placement":"inside"}],"objectives":[{"commitments":[{"admission_id":"a1","committed":"200000000","gate":{"qualified":false,"reasons":["NoGateEvidence"]},"kind":"at_least","state":"retired_resumable"}],"objective":"o","owned_closed":{"high":{"kind":"exact","nano":"0"},"low":"0"},"owned_outstanding":{"high":{"kind":"at_least","nano":"800000000"},"low":"0"},"predecessor_obligations":{"high":{"kind":"exact","nano":"0"},"low":"0"},"provisional":false,"reasons":["SettlementRejected","UnenforcedCommitment","UnresolvedExposure"],"status":"not_established"}]},"name":"default","objectives":[{"admissions":[{"admission_id":"a1","rows":[{"admission_id":"a1","at":"2026-11-10T06:00:05Z","event":"pending","max_usd":1.0},{"admission_id":"a1","at":"2026-11-10T06:00:10Z","event":"launched","session_id":"00000000-0000-4000-8000-0000000000c1"}],"session":"00000000-0000-4000-8000-0000000000c1","source":"journal"}],"closed_at":null,"limit":"1000000000","objective":"o","predecessor":null,"started_at":"2026-11-10T06:00:00Z","tree_sessions":["00000000-0000-4000-8000-0000000000c1"]}]}],"expected":{"attempts":[{"billing":"unresolved","blocking":["SettlementRejected","UnresolvedExposure"],"correlation":{"kind":"exact"},"diagnostics":[],"evidence":["S1/turn-1:1","S1/turn-1:2"],"execution":"terminal","exposure":{"high":{"basis":"proxy_reservation","kind":"at_least","nano":"800000000"},"low":{"basis":"none","nano":"0"},"pool":"pi_api"},"forwarding":"forwarded","key":"res:S1/turn-1:rid=1","obligation":{"amount_source":"reservation_bound","state":"outstanding"},"replacement":{"eligible":false,"reason":"NotUsageSettlement"},"settlement":{"charged":"0","decision":"rejected","reasons":["BasisMissing"],"reserved":"800000000"},"valuation":{"state":"not_reported"}}],"inventory":[],"lifecycle":{"S1":{"clock":"trusted","reopened":false,"state":"retired"}},"non_attempt":[],"unreadable":[]},"historical":{"a134_086bdec1":{"blocking_kinds":["pi_settlement_invalid"],"decision":"not_established","evaluation":"default","note":"comparison only; A1.3.4 has no ownership (D1) concept: it attributes by session tree and time window","reader_sha256":"086bdec1d443064f6847821858f34bbf227f8804a9a5fc8a362416f62b29d621","reasons":["objective accounting unavailable, unpriced or inconsistent"],"spent_high_nano":null,"spent_low_nano":null}},"inputs":{"rates":{"anthropic_api_rates.json":"8a286433d9e08b37533f2f908f2db60c4357e1a566018c9ca8364937652bdef5","openai_api_rates.json":"c0bdfd66ee69f1644eaf188f87f6af5e47beacc0739d20aac5846c923d66fd58"},"reference_time":"2026-11-10T07:00:00Z","sessions":[{"alias":"S1","launch":{"auth_lane":"api","model":"openai/gpt-6-astra","parent":"root"},"lifecycle":[{"at":"2026-11-10T06:00:10Z","event":"launch","turn":1},{"at":"2026-11-10T06:50:00Z","event":"retire"}],"logs":{"turn-1":{"final_newline":true,"lines":["{\"at\": \"2026-11-10T06:30:00Z\", \"event\": \"usd_reserved\", \"n\": 1, \"rid\": 1, \"reserved_usd\": 0.8, \"input_bound\": 6000, \"bound_basis\": \"bytes\"}","{\"at\": \"2026-11-10T06:30:02Z\", \"event\": \"request\", \"n\": 1, \"http_status\": 429, \"model\": \"gpt-6-astra\", \"requested_tier\": null, \"observed_tier\": \"default\", \"terminal\": null, \"response_id\": null, \"response_model\": \"gpt-6-astra\", \"usage\": {\"input_tokens\": null, \"output_tokens\": null, \"cached_tokens\": null, \"cache_write_tokens\": null, \"reasoning_tokens\": 0}, \"seconds\": 1.0, \"usd\": {\"rid\": 1, \"reserved_usd\": 0.8, \"charged_usd\": 0.0, \"session_spent_usd\": 0.0, \"multiplier_floor\": 0.0}}"]}},"session_id":"00000000-0000-4000-8000-0000000000c1"}]},"invariants":["INV-F1","INV-F3"],"profile":"current_producer","provenance":{"origin":"P1.19 report::C4: 429 settlement without a basis"},"reachability":"producer_unreachable","role":"finding","schema":"pi-accounting-corpus/1"}
   14| {"ambiguity":null,"case_id":"S01-reservation-basis-charged-less","class":"malformed","contract":"pi-accounting-contract/1.4-s1","controls":["VC04-upstream-error-keeps-reservation"],"evaluations":[{"admission_request":{"new":"100000000","objective":"o"},"expected":{"admission":{"decision":"refused","limit":"1000000000","new":"100000000","objective":"o"},"attribution":[{"attempt":"res:S1/turn-1:rid=1","objective":"o","ownership":"owned","placement":"inside"}],"objectives":[{"commitments":[{"admission_id":"a1","committed":"200000000","gate":{"qualified":false,"reasons":["NoGateEvidence"]},"kind":"at_least","state":"retired_resumable"}],"objective":"o","owned_closed":{"high":{"kind":"exact","nano":"0"},"low":"0"},"owned_outstanding":{"high":{"kind":"at_least","nano":"800000000"},"low":"0"},"predecessor_obligations":{"high":{"kind":"exact","nano":"0"},"low":"0"},"provisional":false,"reasons":["SettlementRejected","UnenforcedCommitment","UnresolvedExposure"],"status":"not_established"}]},"name":"default","objectives":[{"admissions":[{"admission_id":"a1","rows":[{"admission_id":"a1","at":"2026-11-10T06:00:05Z","event":"pending","max_usd":1.0},{"admission_id":"a1","at":"2026-11-10T06:00:10Z","event":"launched","session_id":"00000000-0000-4000-8000-0000000000c1"}],"session":"00000000-0000-4000-8000-0000000000c1","source":"journal"}],"closed_at":null,"limit":"1000000000","objective":"o","predecessor":null,"started_at":"2026-11-10T06:00:00Z","tree_sessions":["00000000-0000-4000-8000-0000000000c1"]}]}],"expected":{"attempts":[{"billing":"unresolved","blocking":["SettlementRejected","UnresolvedExposure"],"correlation":{"kind":"exact"},"diagnostics":[],"evidence":["S1/turn-1:1","S1/turn-1:2"],"execution":"terminal","exposure":{"high":{"basis":"proxy_reservation","kind":"at_least","nano":"800000000"},"low":{"basis":"none","nano":"0"},"pool":"pi_api"},"forwarding":"forwarded","key":"res:S1/turn-1:rid=1","obligation":{"amount_source":"reservation_bound","state":"outstanding"},"replacement":{"eligible":false,"reason":"NotUsageSettlement"},"settlement":{"charged":"250000000","decision":"rejected","reasons":["KeepAll"],"reserved":"800000000"},"valuation":{"state":"not_reported"}}],"inventory":[],"lifecycle":{"S1":{"clock":"trusted","reopened":false,"state":"retired"}},"non_attempt":[],"unreadable":[]},"historical":{"a134_086bdec1":{"blocking_kinds":["pi_settlement_invalid"],"decision":"not_established","evaluation":"default","note":"comparison only; A1.3.4 has no ownership (D1) concept: it attributes by session tree and time window","reader_sha256":"086bdec1d443064f6847821858f34bbf227f8804a9a5fc8a362416f62b29d621","reasons":["objective accounting unavailable, unpriced or inconsistent"],"spent_high_nano":null,"spent_low_nano":null}},"inputs":{"rates":{"anthropic_api_rates.json":"8a286433d9e08b37533f2f908f2db60c4357e1a566018c9ca8364937652bdef5","openai_api_rates.json":"c0bdfd66ee69f1644eaf188f87f6af5e47beacc0739d20aac5846c923d66fd58"},"reference_time":"2026-11-10T07:00:00Z","sessions":[{"alias":"S1","launch":{"auth_lane":"api","model":"openai/gpt-6-astra","parent":"root"},"lifecycle":[{"at":"2026-11-10T06:00:10Z","event":"launch","turn":1},{"at":"2026-11-10T06:50:00Z","event":"retire"}],"logs":{"turn-1":{"final_newline":true,"lines":["{\"at\": \"2026-11-10T06:30:00Z\", \"event\": \"usd_reserved\", \"n\": 1, \"rid\": 1, \"reserved_usd\": 0.8, \"input_bound\": 6000, \"bound_basis\": \"bytes\"}","{\"at\": \"2026-11-10T06:30:02Z\", \"event\": \"request\", \"n\": 1, \"http_status\": 200, \"model\": \"gpt-6-astra\", \"requested_tier\": null, \"observed_tier\": \"default\", \"terminal\": null, \"response_id\": null, \"response_model\": \"gpt-6-astra\", \"usage\": {\"input_tokens\": null, \"output_tokens\": null, \"cached_tokens\": null, \"cache_write_tokens\": null, \"reasoning_tokens\": 0}, \"seconds\": 1.0, \"usd\": {\"rid\": 1, \"reserved_usd\": 0.8, \"charged_usd\": 0.25, \"charge_basis\": \"reservation\", \"session_spent_usd\": 0.25, \"multiplier_floor\": 0.0}}"]}},"session_id":"00000000-0000-4000-8000-0000000000c1"}]},"invariants":["INV-F1","INV-F3"],"profile":"current_producer","provenance":{"origin":"repro_p115.py::test_S01","variant":"reservation basis charged 0.25"},"reachability":"producer_unreachable","role":"finding","schema":"pi-accounting-corpus/1"}
   15| {"ambiguity":null,"case_id":"S01-upstream-error-charged-zero","class":"malformed","contract":"pi-accounting-contract/1.4-s1","controls":["VC04-upstream-error-keeps-reservation"],"evaluations":[{"admission_request":{"new":"100000000","objective":"o"},"expected":{"admission":{"decision":"refused","limit":"1000000000","new":"100000000","objective":"o"},"attribution":[{"attempt":"res:S1/turn-1:rid=1","objective":"o","ownership":"owned","placement":"inside"}],"objectives":[{"commitments":[{"admission_id":"a1","committed":"200000000","gate":{"qualified":false,"reasons":["NoGateEvidence"]},"kind":"at_least","state":"retired_resumable"}],"objective":"o","owned_closed":{"high":{"kind":"exact","nano":"0"},"low":"0"},"owned_outstanding":{"high":{"kind":"at_least","nano":"800000000"},"low":"0"},"predecessor_obligations":{"high":{"kind":"exact","nano":"0"},"low":"0"},"provisional":false,"reasons":["SettlementRejected","UnenforcedCommitment","UnresolvedExposure"],"status":"not_established"}]},"name":"default","objectives":[{"admissions":[{"admission_id":"a1","rows":[{"admission_id":"a1","at":"2026-11-10T06:00:05Z","event":"pending","max_usd":1.0},{"admission_id":"a1","at":"2026-11-10T06:00:10Z","event":"launched","session_id":"00000000-0000-4000-8000-0000000000c1"}],"session":"00000000-0000-4000-8000-0000000000c1","source":"journal"}],"closed_at":null,"limit":"1000000000","objective":"o","predecessor":null,"started_at":"2026-11-10T06:00:00Z","tree_sessions":["00000000-0000-4000-8000-0000000000c1"]}]}],"expected":{"attempts":[{"billing":"unresolved","blocking":["SettlementRejected","UnresolvedExposure"],"correlation":{"kind":"exact"},"diagnostics":[],"evidence":["S1/turn-1:1","S1/turn-1:2"],"execution":"terminal","exposure":{"high":{"basis":"proxy_reservation","kind":"at_least","nano":"800000000"},"low":{"basis":"none","nano":"0"},"pool":"pi_api"},"forwarding":"unknown","key":"res:S1/turn-1:rid=1","obligation":{"amount_source":"reservation_bound","state":"outstanding"},"replacement":{"eligible":false,"reason":"NotUsageSettlement"},"settlement":{"charged":"0","decision":"rejected","reasons":["KeepAll"],"reserved":"800000000"},"valuation":{"state":"not_reported"}}],"inventory":[],"lifecycle":{"S1":{"clock":"trusted","reopened":false,"state":"retired"}},"non_attempt":[],"unreadable":[]},"historical":{"a134_086bdec1":{"blocking_kinds":["pi_settlement_invalid"],"decision":"not_established","evaluation":"default","note":"comparison only; A1.3.4 has no ownership (D1) concept: it attributes by session tree and time window","reader_sha256":"086bdec1d443064f6847821858f34bbf227f8804a9a5fc8a362416f62b29d621","reasons":["objective accounting unavailable, unpriced or inconsistent"],"spent_high_nano":null,"spent_low_nano":null}},"inputs":{"rates":{"anthropic_api_rates.json":"8a286433d9e08b37533f2f908f2db60c4357e1a566018c9ca8364937652bdef5","openai_api_rates.json":"c0bdfd66ee69f1644eaf188f87f6af5e47beacc0739d20aac5846c923d66fd58"},"reference_time":"2026-11-10T07:00:00Z","sessions":[{"alias":"S1","launch":{"auth_lane":"api","model":"openai/gpt-6-astra","parent":"root"},"lifecycle":[{"at":"2026-11-10T06:00:10Z","event":"launch","turn":1},{"at":"2026-11-10T06:50:00Z","event":"retire"}],"logs":{"turn-1":{"final_newline":true,"lines":["{\"at\": \"2026-11-10T06:30:00Z\", \"event\": \"usd_reserved\", \"n\": 1, \"rid\": 1, \"reserved_usd\": 0.8, \"input_bound\": 6000, \"bound_basis\": \"bytes\"}","{\"at\": \"2026-11-10T06:30:02Z\", \"event\": \"upstream_error\", \"n\": 1, \"detail\": \"connection reset\", \"usd\": {\"rid\": 1, \"reserved_usd\": 0.8, \"charged_usd\": 0.0, \"charge_basis\": \"reservation\", \"session_spent_usd\": 0.0, \"multiplier_floor\": 0.0}}"]}},"session_id":"00000000-0000-4000-8000-0000000000c1"}]},"invariants":["INV-F1","INV-F3"],"profile":"current_producer","provenance":{"origin":"repro_p115.py::test_S01","variant":"upstream_error charged 0"},"reachability":"producer_unreachable","role":"finding","schema":"pi-accounting-corpus/1"}
   16| {"ambiguity":null,"case_id":"S06-string-http-status","class":"malformed","contract":"pi-accounting-contract/1.4-s1","controls":["VC01-openai-usage-settlement"],"evaluations":[{"admission_request":{"new":"100000000","objective":"o"},"expected":{"admission":{"decision":"refused","limit":"1000000000","new":"100000000","objective":"o"},"attribution":[{"attempt":"res:S1/turn-1:rid=1","objective":"o","ownership":"owned","placement":"inside"}],"objectives":[{"commitments":[{"admission_id":"a1","committed":"200000000","gate":{"qualified":false,"reasons":["NoGateEvidence"]},"kind":"at_least","state":"retired_resumable"}],"objective":"o","owned_closed":{"high":{"kind":"exact","nano":"0"},"low":"0"},"owned_outstanding":{"high":{"kind":"at_least","nano":"800000000"},"low":"0"},"predecessor_obligations":{"high":{"kind":"exact","nano":"0"},"low":"0"},"provisional":false,"reasons":["SettlementRejected","UnenforcedCommitment","UnresolvedExposure"],"status":"not_established"}]},"name":"default","objectives":[{"admissions":[{"admission_id":"a1","rows":[{"admission_id":"a1","at":"2026-11-10T06:00:05Z","event":"pending","max_usd":1.0},{"admission_id":"a1","at":"2026-11-10T06:00:10Z","event":"launched","session_id":"00000000-0000-4000-8000-0000000000c1"}],"session":"00000000-0000-4000-8000-0000000000c1","source":"journal"}],"closed_at":null,"limit":"1000000000","objective":"o","predecessor":null,"started_at":"2026-11-10T06:00:00Z","tree_sessions":["00000000-0000-4000-8000-0000000000c1"]}]}],"expected":{"attempts":[{"billing":"unresolved","blocking":["SettlementRejected","UnresolvedExposure"],"correlation":{"kind":"exact"},"diagnostics":[],"evidence":["S1/turn-1:1","S1/turn-1:2"],"execution":"terminal","exposure":{"high":{"basis":"proxy_reservation","kind":"at_least","nano":"800000000"},"low":{"basis":"none","nano":"0"},"pool":"pi_api"},"forwarding":"forwarded","key":"res:S1/turn-1:rid=1","obligation":{"amount_source":"reservation_bound","state":"outstanding"},"replacement":{"eligible":false,"reason":"NotUsageSettlement"},"settlement":{"charged":"0","decision":"rejected","reasons":["StatusType","Undercharge"],"reserved":"800000000"},"valuation":{"complete":false,"derivations":[{"for":"both","model":"gpt-6-astra","nano":"15000000","provider":"openai","tier_multiplier":"1","usage":{"cache_write_tokens":0,"cached_tokens":0,"input_tokens":1000,"output_tokens":100}}],"high":"15000000","identities":["gpt-6-astra"],"low":"15000000","state":"valued"}}],"inventory":[],"lifecycle":{"S1":{"clock":"trusted","reopened":false,"state":"retired"}},"non_attempt":[],"unreadable":[]},"historical":{"a134_086bdec1":{"blocking_kinds":["pi_http_status_invalid","pi_settlement_invalid"],"decision":"not_established","evaluation":"default","note":"comparison only; A1.3.4 has no ownership (D1) concept: it attributes by session tree and time window","reader_sha256":"086bdec1d443064f6847821858f34bbf227f8804a9a5fc8a362416f62b29d621","reasons":["objective accounting unavailable, unpriced or inconsistent"],"spent_high_nano":null,"spent_low_nano":null}},"inputs":{"rates":{"anthropic_api_rates.json":"8a286433d9e08b37533f2f908f2db60c4357e1a566018c9ca8364937652bdef5","openai_api_rates.json":"c0bdfd66ee69f1644eaf188f87f6af5e47beacc0739d20aac5846c923d66fd58"},"reference_time":"2026-11-10T07:00:00Z","sessions":[{"alias":"S1","launch":{"auth_lane":"api","model":"openai/gpt-6-astra","parent":"root"},"lifecycle":[{"at":"2026-11-10T06:00:10Z","event":"launch","turn":1},{"at":"2026-11-10T06:50:00Z","event":"retire"}],"logs":{"turn-1":{"final_newline":true,"lines":["{\"at\": \"2026-11-10T06:30:00Z\", \"event\": \"usd_reserved\", \"n\": 1, \"rid\": 1, \"reserved_usd\": 0.8, \"input_bound\": 6000, \"bound_basis\": \"bytes\"}","{\"at\": \"2026-11-10T06:30:02Z\", \"event\": \"request\", \"n\": 1, \"http_status\": \"200\", \"model\": \"gpt-6-astra\", \"requested_tier\": null, \"observed_tier\": \"default\", \"terminal\": \"response.completed\", \"response_id\": null, \"response_model\": \"gpt-6-astra\", \"usage\": {\"input_tokens\": 1000, \"output_tokens\": 100, \"cached_tokens\": 0, \"cache_write_tokens\": 0, \"reasoning_tokens\": 0}, \"seconds\": 1.0, \"usd\": {\"rid\": 1, \"reserved_usd\": 0.8, \"charged_usd\": 0.0, \"charge_basis\": \"usage\", \"session_spent_usd\": 0.0, \"multiplier_floor\": 0.0}}"]}},"session_id":"00000000-0000-4000-8000-0000000000c1"}]},"invariants":["INV-F1","INV-F3"],"profile":"current_producer","provenance":{"origin":"repro_p115.py::test_S06"},"reachability":"producer_unreachable","role":"finding","schema":"pi-accounting-corpus/1"}
   17| {"ambiguity":null,"case_id":"V01-cheaper-response-model-undercharge","class":"adversarial_synthetic","contract":"pi-accounting-contract/1.4-s1","controls":["V01-control-producer-price"],"evaluations":[{"admission_request":{"new":"100000000","objective":"o"},"expected":{"admission":{"decision":"refused","limit":"1000000000","new":"100000000","objective":"o"},"attribution":[{"attempt":"res:S1/turn-1:rid=1","objective":"o","ownership":"owned","placement":"inside"}],"objectives":[{"commitments":[{"admission_id":"a1","committed":"200000000","gate":{"qualified":false,"reasons":["NoGateEvidence"]},"kind":"at_least","state":"retired_resumable"}],"objective":"o","owned_closed":{"high":{"kind":"exact","nano":"0"},"low":"0"},"owned_outstanding":{"high":{"kind":"at_least","nano":"800000000"},"low":"0"},"predecessor_obligations":{"high":{"kind":"exact","nano":"0"},"low":"0"},"provisional":false,"reasons":["IdentityContradiction","SettlementRejected","UnenforcedCommitment","UnresolvedExposure"],"status":"not_established"}]},"name":"default","objectives":[{"admissions":[{"admission_id":"a1","rows":[{"admission_id":"a1","at":"2026-11-10T06:00:05Z","event":"pending","max_usd":1.0},{"admission_id":"a1","at":"2026-11-10T06:00:10Z","event":"launched","session_id":"00000000-0000-4000-8000-0000000000c1"}],"session":"00000000-0000-4000-8000-0000000000c1","source":"journal"}],"closed_at":null,"limit":"1000000000","objective":"o","predecessor":null,"started_at":"2026-11-10T06:00:00Z","tree_sessions":["00000000-0000-4000-8000-0000000000c1"]}]}],"expected":{"attempts":[{"billing":"unresolved","blocking":["IdentityContradiction","SettlementRejected","UnresolvedExposure"],"correlation":{"kind":"exact"},"diagnostics":[],"evidence":["S1/turn-1:1","S1/turn-1:2"],"execution":"terminal","exposure":{"high":{"basis":"proxy_reservation","kind":"at_least","nano":"800000000"},"low":{"basis":"none","nano":"0"},"pool":"pi_api"},"forwarding":"forwarded","key":"res:S1/turn-1:rid=1","obligation":{"amount_source":"reservation_bound","state":"outstanding"},"replacement":{"eligible":false,"reason":"NotUsageSettlement"},"settlement":{"charged":"60000000","decision":"rejected","reasons":["Undercharge"],"reserved":"800000000"},"valuation":{"complete":true,"derivations":[{"for":"low","model":"gpt-6.1-terra","nano":"60000000","provider":"openai","tier_multiplier":"1","usage":{"cache_write_tokens":0,"cached_tokens":0,"input_tokens":0,"output_tokens":12000}},{"for":"high","model":"gpt-6-astra","nano":"600000000","provider":"openai","tier_multiplier":"1","usage":{"cache_write_tokens":0,"cached_tokens":0,"input_tokens":0,"output_tokens":12000}}],"high":"600000000","identities":["gpt-6-astra","gpt-6.1-terra"],"low":"60000000","state":"valued"}}],"inventory":[],"lifecycle":{"S1":{"clock":"trusted","reopened":false,"state":"retired"}},"non_attempt":[],"unreadable":[]},"historical":{"a134_086bdec1":{"blocking_kinds":["pi_settlement_invalid"],"decision":"not_established","evaluation":"default","note":"comparison only; A1.3.4 has no ownership (D1) concept: it attributes by session tree and time window","reader_sha256":"086bdec1d443064f6847821858f34bbf227f8804a9a5fc8a362416f62b29d621","reasons":["objective accounting unavailable, unpriced or inconsistent"],"spent_high_nano":null,"spent_low_nano":null}},"inputs":{"rates":{"anthropic_api_rates.json":"8a286433d9e08b37533f2f908f2db60c4357e1a566018c9ca8364937652bdef5","openai_api_rates.json":"c0bdfd66ee69f1644eaf188f87f6af5e47beacc0739d20aac5846c923d66fd58"},"reference_time":"2026-11-10T07:00:00Z","sessions":[{"alias":"S1","launch":{"auth_lane":"api","model":"openai/gpt-6-astra","parent":"root"},"lifecycle":[{"at":"2026-11-10T06:00:10Z","event":"launch","turn":1},{"at":"2026-11-10T06:50:00Z","event":"retire"}],"logs":{"turn-1":{"final_newline":true,"lines":["{\"at\": \"2026-11-10T06:30:00Z\", \"event\": \"usd_reserved\", \"n\": 1, \"rid\": 1, \"reserved_usd\": 0.8, \"input_bound\": 6000, \"bound_basis\": \"bytes\"}","{\"at\": \"2026-11-10T06:30:02Z\", \"event\": \"request\", \"n\": 1, \"http_status\": 200, \"model\": \"gpt-6-astra\", \"requested_tier\": null, \"observed_tier\": \"default\", \"terminal\": \"response.completed\", \"response_id\": \"resp_1\", \"response_model\": \"gpt-6.1-terra\", \"usage\": {\"input_tokens\": 0, \"output_tokens\": 12000, \"cached_tokens\": 0, \"cache_write_tokens\": 0, \"reasoning_tokens\": 0}, \"seconds\": 1.0, \"usd\": {\"rid\": 1, \"reserved_usd\": 0.8, \"charged_usd\": 0.06, \"charge_basis\": \"usage\", \"session_spent_usd\": 0.06, \"multiplier_floor\": 0.0}}"]}},"session_id":"00000000-0000-4000-8000-0000000000c1"}]},"invariants":["INV-F1","INV-F3"],"profile":"current_producer","provenance":{"origin":"repro_p123.py::test_V01","variant":"claimed: charge 0.06 for usage priced 0.6"},"reachability":"producer_unreachable","role":"finding","schema":"pi-accounting-corpus/1"}
   18| {"ambiguity":null,"case_id":"P9-claimed-charge-above-reservation","class":"malformed","contract":"pi-accounting-contract/1.4-s1","controls":["VC01-openai-usage-settlement"],"evaluations":[{"admission_request":{"new":"100000000","objective":"o"},"expected":{"admission":{"decision":"refused","limit":"1000000000","new":"100000000","objective":"o"},"attribution":[{"attempt":"res:S1/turn-1:rid=1","objective":"o","ownership":"owned","placement":"inside"}],"objectives":[{"commitments":[{"admission_id":"a1","committed":"300000000","gate":{"qualified":false,"reasons":["NoGateEvidence"]},"kind":"at_least","state":"retired_resumable"}],"objective":"o","owned_closed":{"high":{"kind":"exact","nano":"0"},"low":"0"},"owned_outstanding":{"high":{"kind":"at_least","nano":"700000000"},"low":"0"},"predecessor_obligations":{"high":{"kind":"exact","nano":"0"},"low":"0"},"provisional":false,"reasons":["SettlementRejected","UnenforcedCommitment","UnresolvedExposure"],"status":"not_established"}]},"name":"default","objectives":[{"admissions":[{"admission_id":"a1","rows":[{"admission_id":"a1","at":"2026-11-10T06:00:05Z","event":"pending","max_usd":1.0},{"admission_id":"a1","at":"2026-11-10T06:00:10Z","event":"launched","session_id":"00000000-0000-4000-8000-0000000000c1"}],"session":"00000000-0000-4000-8000-0000000000c1","source":"journal"}],"closed_at":null,"limit":"1000000000","objective":"o","predecessor":null,"started_at":"2026-11-10T06:00:00Z","tree_sessions":["00000000-0000-4000-8000-0000000000c1"]}]}],"expected":{"attempts":[{"billing":"unresolved","blocking":["SettlementRejected","UnresolvedExposure"],"correlation":{"kind":"exact"},"diagnostics":[],"evidence":["S1/turn-1:1","S1/turn-1:2"],"execution":"terminal","exposure":{"high":{"basis":"proxy_charge","kind":"at_least","nano":"700000000"},"low":{"basis":"none","nano":"0"},"pool":"pi_api"},"forwarding":"forwarded","key":"res:S1/turn-1:rid=1","obligation":{"amount_source":"reservation_bound","state":"outstanding"},"replacement":{"eligible":false,"reason":"NotUsageSettlement"},"settlement":{"charged":"700000000","decision":"rejected","reasons":["ReservedMismatch"],"reserved":"500000000"},"valuation":{"complete":true,"derivations":[{"for":"both","model":"gpt-6-astra","nano":"15000000","provider":"openai","tier_multiplier":"1","usage":{"cache_write_tokens":0,"cached_tokens":0,"input_tokens":1000,"output_tokens":100}}],"high":"15000000","identities":["gpt-6-astra"],"low":"15000000","state":"valued"}}],"inventory":[],"lifecycle":{"S1":{"clock":"trusted","reopened":false,"state":"retired"}},"non_attempt":[],"unreadable":[]},"historical":{"a134_086bdec1":{"blocking_kinds":["pi_settlement_invalid"],"decision":"not_established","evaluation":"default","note":"comparison only; A1.3.4 has no ownership (D1) concept: it attributes by session tree and time window","reader_sha256":"086bdec1d443064f6847821858f34bbf227f8804a9a5fc8a362416f62b29d621","reasons":["objective accounting unavailable, unpriced or inconsistent"],"spent_high_nano":null,"spent_low_nano":null}},"inputs":{"rates":{"anthropic_api_rates.json":"8a286433d9e08b37533f2f908f2db60c4357e1a566018c9ca8364937652bdef5","openai_api_rates.json":"c0bdfd66ee69f1644eaf188f87f6af5e47beacc0739d20aac5846c923d66fd58"},"reference_time":"2026-11-10T07:00:00Z","sessions":[{"alias":"S1","launch":{"auth_lane":"api","model":"openai/gpt-6-astra","parent":"root"},"lifecycle":[{"at":"2026-11-10T06:00:10Z","event":"launch","turn":1},{"at":"2026-11-10T06:50:00Z","event":"retire"}],"logs":{"turn-1":{"final_newline":true,"lines":["{\"at\": \"2026-11-10T06:30:00Z\", \"event\": \"usd_reserved\", \"n\": 1, \"rid\": 1, \"reserved_usd\": 0.4, \"input_bound\": 6000, \"bound_basis\": \"bytes\"}","{\"at\": \"2026-11-10T06:30:02Z\", \"event\": \"request\", \"n\": 1, \"http_status\": 200, \"model\": \"gpt-6-astra\", \"requested_tier\": null, \"observed_tier\": \"default\", \"terminal\": \"response.completed\", \"response_id\": \"resp_1\", \"response_model\": \"gpt-6-astra\", \"usage\": {\"input_tokens\": 1000, \"output_tokens\": 100, \"cached_tokens\": 0, \"cache_write_tokens\": 0, \"reasoning_tokens\": 0}, \"seconds\": 1.0, \"usd\": {\"rid\": 1, \"reserved_usd\": 0.5, \"charged_usd\": 0.7, \"charge_basis\": \"usage\", \"session_spent_usd\": 0.7, \"multiplier_floor\": 0.0}}"]}},"session_id":"00000000-0000-4000-8000-0000000000c1"}]},"invariants":["INV-F3"],"profile":"current_producer","provenance":{"origin":"test_usage_ledger_pi_a134.py::test_contradicted_settlement_never_lowers_the_retained_reservation"},"reachability":"producer_unreachable","role":"finding","schema":"pi-accounting-corpus/1"}
   19| {"ambiguity":null,"case_id":"S02-control-error-status-after-reservation","class":"producer_valid","contract":"pi-accounting-contract/1.4-s1","controls":[],"evaluations":[{"admission_request":{"new":"100000000","objective":"o"},"expected":{"admission":{"decision":"refused","limit":"1000000000","new":"100000000","objective":"o"},"attribution":[{"attempt":"res:S1/turn-1:rid=1","objective":"o","ownership":"owned","placement":"inside"}],"objectives":[{"commitments":[{"admission_id":"a1","committed":"200000000","gate":{"qualified":false,"reasons":["NoGateEvidence"]},"kind":"at_least","state":"retired_resumable"}],"objective":"o","owned_closed":{"high":{"kind":"at_least","nano":"800000000"},"low":"0"},"owned_outstanding":{"high":{"kind":"exact","nano":"0"},"low":"0"},"predecessor_obligations":{"high":{"kind":"exact","nano":"0"},"low":"0"},"provisional":false,"reasons":["UnenforcedCommitment","UnresolvedExposure"],"status":"not_established"}]},"name":"default","objectives":[{"admissions":[{"admission_id":"a1","rows":[{"admission_id":"a1","at":"2026-11-10T06:00:05Z","event":"pending","max_usd":1.0},{"admission_id":"a1","at":"2026-11-10T06:00:10Z","event":"launched","session_id":"00000000-0000-4000-8000-0000000000c1"}],"session":"00000000-0000-4000-8000-0000000000c1","source":"journal"}],"closed_at":null,"limit":"1000000000","objective":"o","predecessor":null,"started_at":"2026-11-10T06:00:00Z","tree_sessions":["00000000-0000-4000-8000-0000000000c1"]}]}],"expected":{"attempts":[{"billing":"unresolved","blocking":["UnresolvedExposure"],"correlation":{"kind":"exact"},"diagnostics":["ErrorStatusSettlement","NonBillingUnverified"],"evidence":["S1/turn-1:1","S1/turn-1:2"],"execution":"terminal","exposure":{"high":{"basis":"proxy_reservation","kind":"at_least","nano":"800000000"},"low":{"basis":"none","nano":"0"},"pool":"pi_api"},"forwarding":"forwarded","key":"res:S1/turn-1:rid=1","obligation":{"amount_source":"reservation_bound","state":"closed"},"replacement":{"eligible":false,"reason":"NotUsageSettlement"},"settlement":{"basis":"error_status","charged":"0","decision":"valid","reasons":[],"reserved":"800000000"},"valuation":{"state":"not_reported"}}],"inventory":[],"lifecycle":{"S1":{"clock":"trusted","reopened":false,"state":"retired"}},"non_attempt":[],"unreadable":[]},"historical":{"a134_086bdec1":{"blocking_kinds":[],"decision":"admitted","evaluation":"default","note":"comparison only; A1.3.4 has no ownership (D1) concept: it attributes by session tree and time window","reader_sha256":"086bdec1d443064f6847821858f34bbf227f8804a9a5fc8a362416f62b29d621","reasons":["within the objective pi_api limit"],"spent_high_nano":"0","spent_low_nano":"0"}},"inputs":{"rates":{"anthropic_api_rates.json":"8a286433d9e08b37533f2f908f2db60c4357e1a566018c9ca8364937652bdef5","openai_api_rates.json":"c0bdfd66ee69f1644eaf188f87f6af5e47beacc0739d20aac5846c923d66fd58"},"reference_time":"2026-11-10T07:00:00Z","sessions":[{"alias":"S1","launch":{"auth_lane":"api","model":"openai/gpt-6-astra","parent":"root"},"lifecycle":[{"at":"2026-11-10T06:00:10Z","event":"launch","turn":1},{"at":"2026-11-10T06:50:00Z","event":"retire"}],"logs":{"turn-1":{"final_newline":true,"lines":["{\"at\": \"2026-11-10T06:30:00Z\", \"event\": \"usd_reserved\", \"n\": 1, \"rid\": 1, \"reserved_usd\": 0.8, \"input_bound\": 6000, \"bound_basis\": \"bytes\"}","{\"at\": \"2026-11-10T06:30:02Z\", \"event\": \"request\", \"n\": 1, \"http_status\": 429, \"model\": \"gpt-6-astra\", \"requested_tier\": null, \"observed_tier\": \"default\", \"terminal\": null, \"response_id\": null, \"response_model\": \"gpt-6-astra\", \"usage\": {\"input_tokens\": null, \"output_tokens\": null, \"cached_tokens\": null, \"cache_write_tokens\": null, \"reasoning_tokens\": 0}, \"seconds\": 1.0, \"usd\": {\"rid\": 1, \"reserved_usd\": 0.8, \"charged_usd\": 0.0, \"charge_basis\": \"error_status\", \"session_spent_usd\": 0.0, \"multiplier_floor\": 0.0}}"]}},"session_id":"00000000-0000-4000-8000-0000000000c1"}]},"invariants":["INV-A1","INV-F1"],"profile":"current_producer","provenance":{"origin":"repro_p115.py::test_S02","variant":"control: reservation then 429"},"reachability":"producer_reachable","role":"control","schema":"pi-accounting-corpus/1"}
   20| {"ambiguity":null,"case_id":"D4-unbudgeted-error-status","class":"producer_valid","contract":"pi-accounting-contract/1.4-s1","controls":["VC06-unbudgeted-completed-request"],"evaluations":[{"admission_request":{"new":"100000000","objective":"o"},"expected":{"admission":{"decision":"not_established","limit":"1000000000","new":"100000000","objective":"o"},"attribution":[{"attempt":"row:S1/turn-1:line=1","candidates":["o"],"objective":null,"ownership":"unadmitted","placement":"uncertain"}],"objectives":[{"commitments":[],"objective":"o","owned_closed":{"high":{"kind":"exact","nano":"0"},"low":"0"},"owned_outstanding":{"high":{"kind":"exact","nano":"0"},"low":"0"},"predecessor_obligations":{"high":{"kind":"exact","nano":"0"},"low":"0"},"provisional":false,"reasons":["HistoricalUnadmitted","UnresolvedExposure"],"status":"not_established"}]},"name":"default","objectives":[{"admissions":[],"closed_at":null,"limit":"1000000000","objective":"o","predecessor":null,"started_at":"2026-11-10T06:00:00Z","tree_sessions":["00000000-0000-4000-8000-0000000000c1"]}]}],"expected":{"attempts":[{"billing":"unresolved","blocking":["UnresolvedExposure"],"correlation":{"kind":"exact"},"diagnostics":[],"evidence":["S1/turn-1:1"],"execution":"terminal","exposure":{"high":{"basis":"none","kind":"at_least","nano":"0"},"low":{"basis":"none","nano":"0"},"pool":"pi_api"},"forwarding":"forwarded","key":"row:S1/turn-1:line=1","obligation":{"amount_source":"valuation_or_unresolved","state":"none"},"replacement":{"eligible":false,"reason":"NoReservation"},"settlement":{"decision":"not_applicable","reasons":[]},"valuation":{"state":"not_reported"}}],"inventory":[],"lifecycle":{"S1":{"clock":"trusted","reopened":false,"state":"retired"}},"non_attempt":[],"unreadable":[]},"historical":{"a134_086bdec1":{"blocking_kinds":[],"decision":"admitted","evaluation":"default","note":"comparison only; A1.3.4 has no ownership (D1) concept: it attributes by session tree and time window","reader_sha256":"086bdec1d443064f6847821858f34bbf227f8804a9a5fc8a362416f62b29d621","reasons":["within the objective pi_api limit"],"spent_high_nano":"0","spent_low_nano":"0"}},"inputs":{"rates":{"anthropic_api_rates.json":"8a286433d9e08b37533f2f908f2db60c4357e1a566018c9ca8364937652bdef5","openai_api_rates.json":"c0bdfd66ee69f1644eaf188f87f6af5e47beacc0739d20aac5846c923d66fd58"},"reference_time":"2026-11-10T07:00:00Z","sessions":[{"alias":"S1","launch":{"auth_lane":"api","model":"openai/gpt-6-astra","parent":"root"},"lifecycle":[{"at":"2026-11-10T06:00:10Z","event":"launch","turn":1},{"at":"2026-11-10T06:50:00Z","event":"retire"}],"logs":{"turn-1":{"final_newline":true,"lines":["{\"at\": \"2026-11-10T06:30:02Z\", \"event\": \"request\", \"n\": 1, \"http_status\": 529, \"model\": \"gpt-6-astra\", \"requested_tier\": null, \"observed_tier\": \"default\", \"terminal\": null, \"response_id\": null, \"response_model\": \"gpt-6-astra\", \"usage\": {\"input_tokens\": null, \"output_tokens\": null, \"cached_tokens\": null, \"cache_write_tokens\": null, \"reasoning_tokens\": 0}, \"seconds\": 1.0}"]}},"session_id":"00000000-0000-4000-8000-0000000000c1"}]},"invariants":["INV-A2","INV-F5"],"profile":"current_producer","provenance":{"origin":"contract::D4: unbudgeted non-200 with no usage"},"reachability":"producer_reachable","role":"finding","schema":"pi-accounting-corpus/1"}
   21| {"ambiguity":null,"case_id":"A06-reservation-usage-null-status-no-id","class":"adversarial_synthetic","contract":"pi-accounting-contract/1.4-s1","controls":["VC03-reservation-settlement-with-usage"],"evaluations":[{"admission_request":{"new":"100000000","objective":"o"},"expected":{"admission":{"decision":"refused","limit":"1000000000","new":"100000000","objective":"o"},"attribution":[{"attempt":"res:S1/turn-1:rid=1","objective":"o","ownership":"owned","placement":"inside"}],"objectives":[{"commitments":[{"admission_id":"a1","committed":"750000000","gate":{"qualified":false,"reasons":["NoGateEvidence"]},"kind":"at_least","state":"retired_resumable"}],"objective":"o","owned_closed":{"high":{"kind":"exact","nano":"0"},"low":"0"},"owned_outstanding":{"high":{"kind":"at_least","nano":"250000000"},"low":"0"},"predecessor_obligations":{"high":{"kind":"exact","nano":"0"},"low":"0"},"provisional":false,"reasons":["SettlementRejected","UnenforcedCommitment","UnresolvedExposure"],"status":"not_established"}]},"name":"default","objectives":[{"admissions":[{"admission_id":"a1","rows":[{"admission_id":"a1","at":"2026-11-10T06:00:05Z","event":"pending","max_usd":1.0},{"admission_id":"a1","at":"2026-11-10T06:00:10Z","event":"launched","session_id":"00000000-0000-4000-8000-0000000000c1"}],"session":"00000000-0000-4000-8000-0000000000c1","source":"journal"}],"closed_at":null,"limit":"1000000000","objective":"o","predecessor":null,"started_at":"2026-11-10T06:00:00Z","tree_sessions":["00000000-0000-4000-8000-0000000000c1"]}]}],"expected":{"attempts":[{"billing":"unresolved","blocking":["SettlementRejected","UnresolvedExposure"],"correlation":{"kind":"exact"},"diagnostics":["IdentityAbsent","PartialUsage"],"evidence":["S1/turn-1:1","S1/turn-1:2"],"execution":"terminal","exposure":{"high":{"basis":"proxy_reservation","kind":"at_least","nano":"250000000"},"low":{"basis":"none","nano":"0"},"pool":"pi_api"},"forwarding":"forwarded","key":"res:S1/turn-1:rid=1","obligation":{"amount_source":"reservation_bound","state":"outstanding"},"replacement":{"eligible":false,"reason":"NotUsageSettlement"},"settlement":{"charged":"250000000","decision":"rejected","reasons":["StatusType"],"reserved":"250000000"},"valuation":{"complete":false,"derivations":[{"for":"both","model":"gpt-6-astra","nano":"15000000","provider":"openai","tier_multiplier":"1","usage":{"cache_write_tokens":0,"cached_tokens":0,"input_tokens":1000,"output_tokens":100}}],"high":"15000000","identities":["gpt-6-astra"],"low":"15000000","state":"valued"}}],"inventory":[],"lifecycle":{"S1":{"clock":"trusted","reopened":false,"state":"retired"}},"non_attempt":[],"unreadable":[]},"historical":{"a134_086bdec1":{"blocking_kinds":[],"decision":"admitted","evaluation":"default","note":"comparison only; A1.3.4 has no ownership (D1) concept: it attributes by session tree and time window","reader_sha256":"086bdec1d443064f6847821858f34bbf227f8804a9a5fc8a362416f62b29d621","reasons":["within the objective pi_api limit"],"spent_high_nano":"250000000","spent_low_nano":"15000000"}},"inputs":{"rates":{"anthropic_api_rates.json":"8a286433d9e08b37533f2f908f2db60c4357e1a566018c9ca8364937652bdef5","openai_api_rates.json":"c0bdfd66ee69f1644eaf188f87f6af5e47beacc0739d20aac5846c923d66fd58"},"reference_time":"2026-11-10T07:00:00Z","sessions":[{"alias":"S1","launch":{"auth_lane":"api","model":"openai/gpt-6-astra","parent":"root"},"lifecycle":[{"at":"2026-11-10T06:00:10Z","event":"launch","turn":1},{"at":"2026-11-10T06:50:00Z","event":"retire"}],"logs":{"turn-1":{"final_newline":true,"lines":["{\"at\": \"2026-11-10T06:30:00Z\", \"event\": \"usd_reserved\", \"n\": 1, \"rid\": 1, \"reserved_usd\": 0.25, \"input_bound\": 6000, \"bound_basis\": \"bytes\"}","{\"at\": \"2026-11-10T06:30:02Z\", \"event\": \"request\", \"n\": 1, \"http_status\": null, \"model\": \"gpt-6-astra\", \"requested_tier\": null, \"observed_tier\": \"default\", \"terminal\": null, \"response_id\": null, \"response_model\": \"gpt-6-astra\", \"usage\": {\"input_tokens\": 1000, \"output_tokens\": 100, \"cached_tokens\": 0, \"cache_write_tokens\": 0, \"reasoning_tokens\": 0}, \"seconds\": 1.0, \"usd\": {\"rid\": 1, \"reserved_usd\": 0.25, \"charged_usd\": 0.25, \"charge_basis\": \"reservation\", \"session_spent_usd\": 0.25, \"multiplier_floor\": 0.0}}"]}},"session_id":"00000000-0000-4000-8000-0000000000c1"}]},"invariants":["INV-F11","INV-F3"],"profile":"current_producer","provenance":{"also":["repro_p111.py::A1-05","repro_p123.py::V05"],"origin":"repro_unit_a.py::A-06"},"reachability":"producer_unreachable","role":"finding","schema":"pi-accounting-corpus/1"}
   22| {"ambiguity":null,"case_id":"V02-control-producer-price","class":"producer_valid","contract":"pi-accounting-contract/1.4-s1","controls":[],"evaluations":[{"admission_request":{"new":"100000000","objective":"o"},"expected":{"admission":{"decision":"refused","limit":"1000000000","new":"100000000","objective":"o"},"attribution":[{"attempt":"res:S1/turn-1:rid=1","objective":"o","ownership":"owned","placement":"inside"}],"objectives":[{"commitments":[{"admission_id":"a1","committed":"984000000","gate":{"qualified":false,"reasons":["NoGateEvidence"]},"kind":"at_least","state":"retired_resumable"}],"objective":"o","owned_closed":{"high":{"kind":"exact","nano":"16000000"},"low":"10000000"},"owned_outstanding":{"high":{"kind":"exact","nano":"0"},"low":"0"},"predecessor_obligations":{"high":{"kind":"exact","nano":"0"},"low":"0"},"provisional":false,"reasons":["UnenforcedCommitment"],"status":"not_established"}]},"name":"default","objectives":[{"admissions":[{"admission_id":"a1","rows":[{"admission_id":"a1","at":"2026-11-10T06:00:05Z","event":"pending","max_usd":1.0},{"admission_id":"a1","at":"2026-11-10T06:00:10Z","event":"launched","session_id":"00000000-0000-4000-8000-0000000000c1"}],"session":"00000000-0000-4000-8000-0000000000c1","source":"journal"}],"closed_at":null,"limit":"1000000000","objective":"o","predecessor":null,"started_at":"2026-11-10T06:00:00Z","tree_sessions":["00000000-0000-4000-8000-0000000000c1"]}]}],"expected":{"attempts":[{"billing":"established","blocking":[],"correlation":{"kind":"exact"},"diagnostics":[],"evidence":["S1/turn-1:1","S1/turn-1:2"],"execution":"terminal","exposure":{"high":{"basis":"usage_valuation","kind":"exact","nano":"16000000"},"low":{"basis":"usage_valuation","nano":"10000000"},"pool":"pi_api"},"forwarding":"forwarded","key":"res:S1/turn-1:rid=1","obligation":{"amount_source":"settlement_amount","state":"closed"},"replacement":{"eligible":true,"reason":"UsageBound"},"settlement":{"basis":"usage","charged":"16000000","decision":"valid","reasons":[],"reserved":"300000000"},"valuation":{"complete":true,"derivations":[{"for":"low","model":"claude-fable-5-2","nano":"10000000","provider":"anthropic","remainder_rate":"min","split":"reported","tier_multiplier":"1","usage":{"cache_creation_input_tokens":1000,"cache_read_input_tokens":0,"input_tokens":0,"output_tokens":0}},{"for":"high","model":"claude-fable-5-2","nano":"16000000","provider":"anthropic","remainder_rate":"max","split":"reported","tier_multiplier":"1","usage":{"cache_creation_input_tokens":1000,"cache_read_input_tokens":0,"input_tokens":0,"output_tokens":0}}],"high":"16000000","identities":["claude-fable-5-2"],"low":"10000000","state":"valued"}}],"inventory":[],"lifecycle":{"S1":{"clock":"trusted","reopened":false,"state":"retired"}},"non_attempt":[],"unreadable":[]},"historical":{"a134_086bdec1":{"blocking_kinds":[],"decision":"admitted","evaluation":"default","note":"comparison only; A1.3.4 has no ownership (D1) concept: it attributes by session tree and time window","reader_sha256":"086bdec1d443064f6847821858f34bbf227f8804a9a5fc8a362416f62b29d621","reasons":["within the objective pi_api limit"],"spent_high_nano":"16000000","spent_low_nano":"10000000"}},"inputs":{"rates":{"anthropic_api_rates.json":"8a286433d9e08b37533f2f908f2db60c4357e1a566018c9ca8364937652bdef5","openai_api_rates.json":"c0bdfd66ee69f1644eaf188f87f6af5e47beacc0739d20aac5846c923d66fd58"},"reference_time":"2026-11-10T07:00:00Z","sessions":[{"alias":"S1","launch":{"auth_lane":"api","model":"anthropic/claude-fable-5-2","parent":"root"},"lifecycle":[{"at":"2026-11-10T06:00:10Z","event":"launch","turn":1},{"at":"2026-11-10T06:50:00Z","event":"retire"}],"logs":{"turn-1":{"final_newline":true,"lines":["{\"at\": \"2026-11-10T06:30:00Z\", \"event\": \"usd_reserved\", \"n\": 1, \"rid\": 1, \"reserved_usd\": 0.3, \"input_bound\": 6000, \"bound_basis\": \"bytes\"}","{\"at\": \"2026-11-10T06:30:02Z\", \"event\": \"request\", \"provider\": \"anthropic\", \"n\": 1, \"http_status\": 200, \"model\": \"claude-fable-5-2\", \"requested_tier\": null, \"observed_tier\": \"standard\", \"terminal\": \"message_stop\", \"response_id\": \"msg_1\", \"response_model\": \"claude-fable-5-2\", \"response_model_check\": \"verified\", \"usage\": {\"input_tokens\": 0, \"cache_read_input_tokens\": 0, \"cache_creation_input_tokens\": 1000, \"output_tokens\": 0}, \"seconds\": 1.0, \"usd\": {\"rid\": 1, \"reserved_usd\": 0.3, \"charged_usd\": 0.016, \"charge_basis\": \"usage\", \"session_spent_usd\": 0.016, \"multiplier_floor\": 0.0}}"]}},"session_id":"00000000-0000-4000-8000-0000000000c1"}]},"invariants":["INV-F1"],"profile":"current_producer","provenance":{"origin":"repro_p123.py::test_V02","variant":"control: charge 0.016"},"reachability":"producer_reachable","role":"control","schema":"pi-accounting-corpus/1"}
   23| {"ambiguity":null,"case_id":"V02-unsplit-writes-undercharged","class":"adversarial_synthetic","contract":"pi-accounting-contract/1.4-s1","controls":["V02-control-producer-price"],"evaluations":[{"admission_request":{"new":"100000000","objective":"o"},"expected":{"admission":{"decision":"refused","limit":"1000000000","new":"100000000","objective":"o"},"attribution":[{"attempt":"res:S1/turn-1:rid=1","objective":"o","ownership":"owned","placement":"inside"}],"objectives":[{"commitments":[{"admission_id":"a1","committed":"700000000","gate":{"qualified":false,"reasons":["NoGateEvidence"]},"kind":"at_least","state":"retired_resumable"}],"objective":"o","owned_closed":{"high":{"kind":"exact","nano":"0"},"low":"0"},"owned_outstanding":{"high":{"kind":"at_least","nano":"300000000"},"low":"0"},"predecessor_obligations":{"high":{"kind":"exact","nano":"0"},"low":"0"},"provisional":false,"reasons":["SettlementRejected","UnenforcedCommitment","UnresolvedExposure"],"status":"not_established"}]},"name":"default","objectives":[{"admissions":[{"admission_id":"a1","rows":[{"admission_id":"a1","at":"2026-11-10T06:00:05Z","event":"pending","max_usd":1.0},{"admission_id":"a1","at":"2026-11-10T06:00:10Z","event":"launched","session_id":"00000000-0000-4000-8000-0000000000c1"}],"session":"00000000-0000-4000-8000-0000000000c1","source":"journal"}],"closed_at":null,"limit":"1000000000","objective":"o","predecessor":null,"started_at":"2026-11-10T06:00:00Z","tree_sessions":["00000000-0000-4000-8000-0000000000c1"]}]}],"expected":{"attempts":[{"billing":"unresolved","blocking":["SettlementRejected","UnresolvedExposure"],"correlation":{"kind":"exact"},"diagnostics":[],"evidence":["S1/turn-1:1","S1/turn-1:2"],"execution":"terminal","exposure":{"high":{"basis":"proxy_reservation","kind":"at_least","nano":"300000000"},"low":{"basis":"none","nano":"0"},"pool":"pi_api"},"forwarding":"forwarded","key":"res:S1/turn-1:rid=1","obligation":{"amount_source":"reservation_bound","state":"outstanding"},"replacement":{"eligible":false,"reason":"NotUsageSettlement"},"settlement":{"charged":"10000000","decision":"rejected","reasons":["Undercharge"],"reserved":"300000000"},"valuation":{"complete":true,"derivations":[{"for":"low","model":"claude-fable-5-2","nano":"10000000","provider":"anthropic","remainder_rate":"min","split":"reported","tier_multiplier":"1","usage":{"cache_creation_input_tokens":1000,"cache_read_input_tokens":0,"input_tokens":0,"output_tokens":0}},{"for":"high","model":"claude-fable-5-2","nano":"16000000","provider":"anthropic","remainder_rate":"max","split":"reported","tier_multiplier":"1","usage":{"cache_creation_input_tokens":1000,"cache_read_input_tokens":0,"input_tokens":0,"output_tokens":0}}],"high":"16000000","identities":["claude-fable-5-2"],"low":"10000000","state":"valued"}}],"inventory":[],"lifecycle":{"S1":{"clock":"trusted","reopened":false,"state":"retired"}},"non_attempt":[],"unreadable":[]},"historical":{"a134_086bdec1":{"blocking_kinds":["pi_settlement_invalid"],"decision":"not_established","evaluation":"default","note":"comparison only; A1.3.4 has no ownership (D1) concept: it attributes by session tree and time window","reader_sha256":"086bdec1d443064f6847821858f34bbf227f8804a9a5fc8a362416f62b29d621","reasons":["objective accounting unavailable, unpriced or inconsistent"],"spent_high_nano":null,"spent_low_nano":null}},"inputs":{"rates":{"anthropic_api_rates.json":"8a286433d9e08b37533f2f908f2db60c4357e1a566018c9ca8364937652bdef5","openai_api_rates.json":"c0bdfd66ee69f1644eaf188f87f6af5e47beacc0739d20aac5846c923d66fd58"},"reference_time":"2026-11-10T07:00:00Z","sessions":[{"alias":"S1","launch":{"auth_lane":"api","model":"anthropic/claude-fable-5-2","parent":"root"},"lifecycle":[{"at":"2026-11-10T06:00:10Z","event":"launch","turn":1},{"at":"2026-11-10T06:50:00Z","event":"retire"}],"logs":{"turn-1":{"final_newline":true,"lines":["{\"at\": \"2026-11-10T06:30:00Z\", \"event\": \"usd_reserved\", \"n\": 1, \"rid\": 1, \"reserved_usd\": 0.3, \"input_bound\": 6000, \"bound_basis\": \"bytes\"}","{\"at\": \"2026-11-10T06:30:02Z\", \"event\": \"request\", \"provider\": \"anthropic\", \"n\": 1, \"http_status\": 200, \"model\": \"claude-fable-5-2\", \"requested_tier\": null, \"observed_tier\": \"standard\", \"terminal\": \"message_stop\", \"response_id\": \"msg_1\", \"response_model\": \"claude-fable-5-2\", \"response_model_check\": \"verified\", \"usage\": {\"input_tokens\": 0, \"cache_read_input_tokens\": 0, \"cache_creation_input_tokens\": 1000, \"output_tokens\": 0}, \"seconds\": 1.0, \"usd\": {\"rid\": 1, \"reserved_usd\": 0.3, \"charged_usd\": 0.01, \"charge_basis\": \"usage\", \"session_spent_usd\": 0.01, \"multiplier_floor\": 0.0}}"]}},"session_id":"00000000-0000-4000-8000-0000000000c1"}]},"invariants":["INV-F1"],"profile":"current_producer","provenance":{"origin":"repro_p123.py::test_V02","variant":"claimed: charge 0.01 for unsplit writes"},"reachability":"producer_unreachable","role":"finding","schema":"pi-accounting-corpus/1"}
   24| {"ambiguity":null,"case_id":"N01-variant-reservation-above-both","class":"producer_valid","contract":"pi-accounting-contract/1.4-s1","controls":["N01-control-served-pinned"],"evaluations":[{"admission_request":{"new":"40000000","objective":"o"},"expected":{"admission":{"decision":"refused","limit":"100000000","new":"40000000","objective":"o"},"attribution":[{"attempt":"res:S1/turn-1:rid=1","objective":"o","ownership":"owned","placement":"inside"}],"objectives":[{"commitments":[{"admission_id":"a1","committed":"900000000","gate":{"qualified":false,"reasons":["NoGateEvidence"]},"kind":"at_least","state":"retired_resumable"}],"objective":"o","owned_closed":{"high":{"kind":"at_least","nano":"100000000"},"low":"20000000"},"owned_outstanding":{"high":{"kind":"exact","nano":"0"},"low":"0"},"predecessor_obligations":{"high":{"kind":"exact","nano":"0"},"low":"0"},"provisional":false,"reasons":["IdentityContradiction","UnenforcedCommitment","UnresolvedExposure"],"status":"not_established"}]},"name":"default","objectives":[{"admissions":[{"admission_id":"a1","rows":[{"admission_id":"a1","at":"2026-11-10T06:00:05Z","event":"pending","max_usd":1.0},{"admission_id":"a1","at":"2026-11-10T06:00:10Z","event":"launched","session_id":"00000000-0000-4000-8000-0000000000c1"}],"session":"00000000-0000-4000-8000-0000000000c1","source":"journal"}],"closed_at":null,"limit":"100000000","objective":"o","predecessor":null,"started_at":"2026-11-10T06:00:00Z","tree_sessions":["00000000-0000-4000-8000-0000000000c1"]}]}],"expected":{"attempts":[{"billing":"unresolved","blocking":["IdentityContradiction","UnresolvedExposure"],"correlation":{"auxiliary":[{"evidence":"S1/turn-1:2","rule":"RS-3"}],"kind":"exact"},"diagnostics":["PartialUsage"],"evidence":["S1/turn-1:1","S1/turn-1:2","S1/turn-1:3"],"execution":"terminal","exposure":{"high":{"basis":"proxy_reservation","kind":"at_least","nano":"100000000"},"low":{"basis":"usage_valuation","nano":"20000000"},"pool":"pi_api"},"forwarding":"forwarded","key":"res:S1/turn-1:rid=1","obligation":{"amount_source":"reservation_confirmed","state":"closed"},"replacement":{"eligible":false,"reason":"NotUsageSettlement"},"settlement":{"basis":"reservation","charged":"100000000","decision":"valid","reasons":[],"reserved":"100000000"},"valuation":{"complete":false,"derivations":[{"for":"low","model":"claude-sonnet-5","nano":"20000000","provider":"anthropic","remainder_rate":"max","split":"reported","tier_multiplier":"1","usage":{"cache_creation_input_tokens":0,"cache_read_input_tokens":0,"input_tokens":10000,"output_tokens":0}},{"for":"high","model":"claude-fable-5-2","nano":"80000000","provider":"anthropic","remainder_rate":"max","split":"reported","tier_multiplier":"1","usage":{"cache_creation_input_tokens":0,"cache_read_input_tokens":0,"input_tokens":10000,"output_tokens":0}}],"high":"80000000","identities":["claude-sonnet-5","claude-fable-5-2"],"low":"20000000","state":"valued"}}],"inventory":[],"lifecycle":{"S1":{"clock":"trusted","reopened":false,"state":"retired"}},"non_attempt":[],"unreadable":[]},"historical":{"a134_086bdec1":{"blocking_kinds":["pi_model_identity_conflict"],"decision":"not_established","evaluation":"default","note":"comparison only; A1.3.4 has no ownership (D1) concept: it attributes by session tree and time window","reader_sha256":"086bdec1d443064f6847821858f34bbf227f8804a9a5fc8a362416f62b29d621","reasons":["objective accounting unavailable, unpriced or inconsistent"],"spent_high_nano":null,"spent_low_nano":null}},"inputs":{"rates":{"anthropic_api_rates.json":"8a286433d9e08b37533f2f908f2db60c4357e1a566018c9ca8364937652bdef5","openai_api_rates.json":"c0bdfd66ee69f1644eaf188f87f6af5e47beacc0739d20aac5846c923d66fd58"},"reference_time":"2026-11-10T07:00:00Z","sessions":[{"alias":"S1","launch":{"auth_lane":"api","model":"anthropic/claude-sonnet-5","parent":"root"},"lifecycle":[{"at":"2026-11-10T06:00:10Z","event":"launch","turn":1},{"at":"2026-11-10T06:50:00Z","event":"retire"}],"logs":{"turn-1":{"final_newline":true,"lines":["{\"at\": \"2026-11-10T06:30:00Z\", \"event\": \"usd_reserved\", \"n\": 1, \"rid\": 1, \"reserved_usd\": 0.1, \"input_bound\": 6000, \"bound_basis\": \"bytes\"}","{\"at\": \"2026-11-10T06:30:01Z\", \"event\": \"response_model_refused\", \"provider\": \"anthropic\", \"n\": 1, \"reason\": \"response_model_mismatch\", \"authorized_model\": \"claude-sonnet-5\", \"response_model\": \"claude-fable-5-2\", \"response_id\": \"msg_x\", \"verified_bytes_forwarded\": false, \"detail\": null}","{\"at\": \"2026-11-10T06:30:01Z\", \"event\": \"request\", \"provider\": \"anthropic\", \"n\": 1, \"http_status\": 200, \"model\": \"claude-sonnet-5\", \"requested_tier\": null, \"observed_tier\": \"standard\", \"terminal\": null, \"response_id\": \"msg_x\", \"response_model\": \"claude-fable-5-2\", \"response_model_check\": \"response_model_mismatch\", \"usage\": {\"input_tokens\": 10000, \"cache_read_input_tokens\": 0, \"cache_creation_input_tokens\": 0, \"output_tokens\": 0}, \"seconds\": 1.0, \"model_mismatch\": true, \"usd\": {\"rid\": 1, \"reserved_usd\": 0.1, \"charged_usd\": 0.1, \"charge_basis\": \"reservation\", \"session_spent_usd\": 0.1, \"multiplier_floor\": 0.0}}"]}},"session_id":"00000000-0000-4000-8000-0000000000c1"}]},"invariants":["INV-F5"],"profile":"current_producer","provenance":{"origin":"repro_p125.py::test_N01_identity_refusal_reservation","variant":"control: reservation 0.10"},"reachability":"producer_reachable","role":"finding","schema":"pi-accounting-corpus/1"}
   25| {"added_in":"1.2-s1","ambiguity":null,"case_id":"P130-R05-conflicting-keeps-valuation-high","class":"adversarial_synthetic","contract":"pi-accounting-contract/1.4-s1","controls":["N01-identity-refusal-dearer-served"],"evaluations":[{"admission_request":{"new":"40000000","objective":"o"},"expected":{"admission":{"decision":"refused","limit":"100000000","new":"40000000","objective":"o"},"attribution":[{"attempt":"res:S1/turn-1:rid=1","objective":"o","ownership":"owned","placement":"inside"}],"objectives":[{"commitments":[{"admission_id":"a1","committed":"920000000","gate":{"qualified":false,"reasons":["NoGateEvidence"]},"kind":"at_least","state":"retired_resumable"}],"objective":"o","owned_closed":{"high":{"kind":"exact","nano":"0"},"low":"0"},"owned_outstanding":{"high":{"kind":"at_least","nano":"80000000"},"low":"0"},"predecessor_obligations":{"high":{"kind":"exact","nano":"0"},"low":"0"},"provisional":false,"reasons":["IdentityContradiction","SettlementConflict","UnenforcedCommitment","UnresolvedExposure"],"status":"not_established"}]},"name":"default","objectives":[{"admissions":[{"admission_id":"a1","rows":[{"admission_id":"a1","at":"2026-11-10T06:00:05Z","event":"pending","max_usd":1.0},{"admission_id":"a1","at":"2026-11-10T06:00:10Z","event":"launched","session_id":"00000000-0000-4000-8000-0000000000c1"}],"session":"00000000-0000-4000-8000-0000000000c1","source":"journal"}],"closed_at":null,"limit":"100000000","objective":"o","predecessor":null,"started_at":"2026-11-10T06:00:00Z","tree_sessions":["00000000-0000-4000-8000-0000000000c1"]}]}],"evidence_pair":{"other":"N01-identity-refusal-dearer-served","permitted_release":false,"relation":"superset"},"expected":{"attempts":[{"billing":"unresolved","blocking":["IdentityContradiction","SettlementConflict","UnresolvedExposure"],"correlation":{"auxiliary":[{"evidence":"S1/turn-1:2","rule":"RS-3"}],"kind":"exact"},"diagnostics":["PartialUsage"],"evidence":["S1/turn-1:1","S1/turn-1:2","S1/turn-1:3","S1/turn-1:4"],"execution":"terminal","exposure":{"high":{"basis":"usage_valuation","kind":"at_least","nano":"80000000"},"low":{"basis":"none","nano":"0"},"pool":"pi_api"},"forwarding":"forwarded","key":"res:S1/turn-1:rid=1","obligation":{"amount_source":"reservation_bound","state":"outstanding"},"replacement":{"eligible":false,"reason":"NotUsageSettlement"},"settlement":{"charged":"50000000","decision":"conflicting","reasons":[],"reserved":"50000000"},"valuation":{"complete":false,"derivations":[{"for":"low","model":"claude-sonnet-5","nano":"20000000","provider":"anthropic","remainder_rate":"max","split":"reported","tier_multiplier":"1","usage":{"cache_creation_input_tokens":0,"cache_read_input_tokens":0,"input_tokens":10000,"output_tokens":0}},{"for":"high","model":"claude-fable-5-2","nano":"80000000","provider":"anthropic","remainder_rate":"max","split":"reported","tier_multiplier":"1","usage":{"cache_creation_input_tokens":0,"cache_read_input_tokens":0,"input_tokens":10000,"output_tokens":0}}],"high":"80000000","identities":["claude-sonnet-5","claude-fable-5-2"],"low":"20000000","state":"valued"}}],"inventory":[],"lifecycle":{"S1":{"clock":"trusted","reopened":false,"state":"retired"}},"non_attempt":[],"unreadable":[]},"historical":{"a134_086bdec1":{"blocking_kinds":["pi_settlement_conflict"],"decision":"not_established","evaluation":"default","note":"comparison only; A1.3.4 has no ownership (D1) concept: it attributes by session tree and time window","reader_sha256":"086bdec1d443064f6847821858f34bbf227f8804a9a5fc8a362416f62b29d621","reasons":["objective accounting unavailable, unpriced or inconsistent"],"spent_high_nano":null,"spent_low_nano":null}},"inputs":{"rates":{"anthropic_api_rates.json":"8a286433d9e08b37533f2f908f2db60c4357e1a566018c9ca8364937652bdef5","openai_api_rates.json":"c0bdfd66ee69f1644eaf188f87f6af5e47beacc0739d20aac5846c923d66fd58"},"reference_time":"2026-11-10T07:00:00Z","sessions":[{"alias":"S1","launch":{"auth_lane":"api","model":"anthropic/claude-sonnet-5","parent":"root"},"lifecycle":[{"at":"2026-11-10T06:00:10Z","event":"launch","turn":1},{"at":"2026-11-10T06:50:00Z","event":"retire"}],"logs":{"turn-1":{"final_newline":true,"lines":["{\"at\": \"2026-11-10T06:30:00Z\", \"event\": \"usd_reserved\", \"n\": 1, \"rid\": 1, \"reserved_usd\": 0.05, \"input_bound\": 6000, \"bound_basis\": \"bytes\"}","{\"at\": \"2026-11-10T06:30:01Z\", \"event\": \"response_model_refused\", \"provider\": \"anthropic\", \"n\": 1, \"reason\": \"response_model_mismatch\", \"authorized_model\": \"claude-sonnet-5\", \"response_model\": \"claude-fable-5-2\", \"response_id\": \"msg_x\", \"verified_bytes_forwarded\": false, \"detail\": null}","{\"at\": \"2026-11-10T06:30:01Z\", \"event\": \"request\", \"provider\": \"anthropic\", \"n\": 1, \"http_status\": 200, \"model\": \"claude-sonnet-5\", \"requested_tier\": null, \"observed_tier\": \"standard\", \"terminal\": null, \"response_id\": \"msg_x\", \"response_model\": \"claude-fable-5-2\", \"response_model_check\": \"response_model_mismatch\", \"usage\": {\"input_tokens\": 10000, \"cache_read_input_tokens\": 0, \"cache_creation_input_tokens\": 0, \"output_tokens\": 0}, \"seconds\": 1.0, \"model_mismatch\": true, \"usd\": {\"rid\": 1, \"reserved_usd\": 0.05, \"charged_usd\": 0.05, \"charge_basis\": \"reservation\", \"session_spent_usd\": 0.05, \"multiplier_floor\": 0.0}}","{\"at\": \"2026-11-10T06:30:05Z\", \"event\": \"request\", \"provider\": \"anthropic\", \"n\": 1, \"http_status\": 200, \"model\": \"claude-sonnet-5\", \"requested_tier\": null, \"observed_tier\": \"standard\", \"terminal\": null, \"response_id\": \"msg_x\", \"response_model\": \"claude-fable-5-2\", \"response_model_check\": \"response_model_mismatch\", \"usage\": {\"input_tokens\": 10000, \"cache_read_input_tokens\": 0, \"cache_creation_input_tokens\": 0, \"output_tokens\": 0}, \"seconds\": 1.0, \"model_mismatch\": true, \"usd\": {\"rid\": 1, \"reserved_usd\": 0.05, \"charged_usd\": 0.05, \"charge_basis\": \"reservation\", \"session_spent_usd\": 0.05, \"multiplier_floor\": 0.0}}"]}},"session_id":"00000000-0000-4000-8000-0000000000c1"}]},"invariants":["INV-F3","INV-A5"],"profile":"current_producer","provenance":{"origin":"contract::8 rejected/conflicting row, V-8 (P1.29 R-05)"},"reachability":"producer_unreachable","role":"finding","schema":"pi-accounting-corpus/1"}
   26| {"added_in":"1.2-s1","ambiguity":null,"case_id":"P130-R01-usage-incomplete-terminal","class":"producer_valid","contract":"pi-accounting-contract/1.4-s1","controls":["VC01-openai-usage-settlement"],"evaluations":[{"admission_request":{"new":"100000000","objective":"o"},"expected":{"admission":{"decision":"refused","limit":"1000000000","new":"100000000","objective":"o"},"attribution":[{"attempt":"res:S1/turn-1:rid=1","objective":"o","ownership":"owned","placement":"inside"}],"objectives":[{"commitments":[{"admission_id":"a1","committed":"500000000","gate":{"qualified":false,"reasons":["NoGateEvidence"]},"kind":"at_least","state":"retired_resumable"}],"objective":"o","owned_closed":{"high":{"kind":"at_least","nano":"500000000"},"low":"15000000"},"owned_outstanding":{"high":{"kind":"exact","nano":"0"},"low":"0"},"predecessor_obligations":{"high":{"kind":"exact","nano":"0"},"low":"0"},"provisional":false,"reasons":["UnenforcedCommitment","UnresolvedExposure"],"status":"not_established"}]},"name":"default","objectives":[{"admissions":[{"admission_id":"a1","rows":[{"admission_id":"a1","at":"2026-11-10T06:00:05Z","event":"pending","max_usd":1.0},{"admission_id":"a1","at":"2026-11-10T06:00:10Z","event":"launched","session_id":"00000000-0000-4000-8000-0000000000c1"}],"session":"00000000-0000-4000-8000-0000000000c1","source":"journal"}],"closed_at":null,"limit":"1000000000","objective":"o","predecessor":null,"started_at":"2026-11-10T06:00:00Z","tree_sessions":["00000000-0000-4000-8000-0000000000c1"]}]}],"expected":{"attempts":[{"billing":"unresolved","blocking":["UnresolvedExposure"],"correlation":{"kind":"exact"},"diagnostics":[],"evidence":["S1/turn-1:1","S1/turn-1:2"],"execution":"terminal","exposure":{"high":{"basis":"proxy_reservation","kind":"at_least","nano":"500000000"},"low":{"basis":"usage_valuation","nano":"15000000"},"pool":"pi_api"},"forwarding":"forwarded","key":"res:S1/turn-1:rid=1","obligation":{"amount_source":"reservation_bound","state":"closed"},"replacement":{"eligible":false,"reason":"UsageIncomplete"},"settlement":{"basis":"usage","charged":"15000000","decision":"valid","reasons":[],"reserved":"500000000"},"valuation":{"complete":false,"derivations":[{"for":"both","model":"gpt-6-astra","nano":"15000000","provider":"openai","tier_multiplier":"1","usage":{"cache_write_tokens":0,"cached_tokens":0,"input_tokens":1000,"output_tokens":100}}],"high":"15000000","identities":["gpt-6-astra"],"low":"15000000","state":"valued"}}],"inventory":[],"lifecycle":{"S1":{"clock":"trusted","reopened":false,"state":"retired"}},"non_attempt":[],"unreadable":[]},"historical":{"a134_086bdec1":{"blocking_kinds":[],"decision":"admitted","evaluation":"default","note":"comparison only; A1.3.4 has no ownership (D1) concept: it attributes by session tree and time window","reader_sha256":"086bdec1d443064f6847821858f34bbf227f8804a9a5fc8a362416f62b29d621","reasons":["within the objective pi_api limit"],"spent_high_nano":"15000000","spent_low_nano":"15000000"}},"inputs":{"rates":{"anthropic_api_rates.json":"8a286433d9e08b37533f2f908f2db60c4357e1a566018c9ca8364937652bdef5","openai_api_rates.json":"c0bdfd66ee69f1644eaf188f87f6af5e47beacc0739d20aac5846c923d66fd58"},"reference_time":"2026-11-10T07:00:00Z","sessions":[{"alias":"S1","launch":{"auth_lane":"api","model":"openai/gpt-6-astra","parent":"root"},"lifecycle":[{"at":"2026-11-10T06:00:10Z","event":"launch","turn":1},{"at":"2026-11-10T06:50:00Z","event":"retire"}],"logs":{"turn-1":{"final_newline":true,"lines":["{\"at\": \"2026-11-10T06:30:00Z\", \"event\": \"usd_reserved\", \"n\": 1, \"rid\": 1, \"reserved_usd\": 0.5, \"input_bound\": 6000, \"bound_basis\": \"bytes\"}","{\"at\": \"2026-11-10T06:30:02Z\", \"event\": \"request\", \"n\": 1, \"http_status\": 200, \"model\": \"gpt-6-astra\", \"requested_tier\": null, \"observed_tier\": \"default\", \"terminal\": \"response.incomplete\", \"response_id\": \"resp_1\", \"response_model\": \"gpt-6-astra\", \"usage\": {\"input_tokens\": 1000, \"output_tokens\": 100, \"cached_tokens\": 0, \"cache_write_tokens\": 0, \"reasoning_tokens\": 0}, \"seconds\": 1.0, \"usd\": {\"rid\": 1, \"reserved_usd\": 0.5, \"charged_usd\": 0.015, \"charge_basis\": \"usage\", \"session_spent_usd\": 0.015, \"multiplier_floor\": 0.0}}"]}},"session_id":"00000000-0000-4000-8000-0000000000c1"}]},"invariants":["INV-F5","INV-F12"],"profile":"current_producer","provenance":{"origin":"contract::FB-1, V-5, 8 row 3 (P1.29 R-01)"},"reachability":"producer_reachable","role":"finding","schema":"pi-accounting-corpus/1"}
   27| {"ambiguity":null,"case_id":"N08-model-without-producer-rate","class":"producer_valid","contract":"pi-accounting-contract/1.4-s1","controls":["N08-control-priced-model"],"evaluations":[{"admission_request":{"new":"100000000","objective":"o"},"expected":{"admission":{"decision":"not_established","limit":"1000000000","new":"100000000","objective":"o"},"attribution":[{"attempt":"row:S1/turn-1:line=1","candidates":["o"],"objective":null,"ownership":"unadmitted","placement":"uncertain"}],"objectives":[{"commitments":[],"objective":"o","owned_closed":{"high":{"kind":"exact","nano":"0"},"low":"0"},"owned_outstanding":{"high":{"kind":"exact","nano":"0"},"low":"0"},"predecessor_obligations":{"high":{"kind":"exact","nano":"0"},"low":"0"},"provisional":false,"reasons":["HistoricalUnadmitted","UnresolvedExposure"],"status":"not_established"}]},"name":"default","objectives":[{"admissions":[],"closed_at":null,"limit":"1000000000","objective":"o","predecessor":null,"started_at":"2026-11-10T06:00:00Z","tree_sessions":["00000000-0000-4000-8000-0000000000c1"]}]}],"expected":{"attempts":[{"billing":"unresolved","blocking":["UnresolvedExposure"],"correlation":{"kind":"exact"},"diagnostics":[],"evidence":["S1/turn-1:1"],"execution":"terminal","exposure":{"high":{"basis":"none","kind":"at_least","nano":"0"},"low":{"basis":"none","nano":"0"},"pool":"pi_api"},"forwarding":"forwarded","key":"row:S1/turn-1:line=1","obligation":{"amount_source":"valuation_or_unresolved","state":"none"},"replacement":{"eligible":false,"reason":"NoReservation"},"settlement":{"decision":"not_applicable","reasons":[]},"valuation":{"derivations":[],"priced_high":null,"reasons":["IdentityWithoutRate"],"state":"unpriceable","supported_low":null}}],"inventory":[],"lifecycle":{"S1":{"clock":"trusted","reopened":false,"state":"retired"}},"non_attempt":[],"unreadable":[]},"historical":{"a134_086bdec1":{"blocking_kinds":[],"decision":"admitted","evaluation":"default","note":"comparison only; A1.3.4 has no ownership (D1) concept: it attributes by session tree and time window","reader_sha256":"086bdec1d443064f6847821858f34bbf227f8804a9a5fc8a362416f62b29d621","reasons":["within the objective pi_api limit"],"spent_high_nano":"1000000","spent_low_nano":"1000000"}},"inputs":{"rates":{"anthropic_api_rates.json":"8a286433d9e08b37533f2f908f2db60c4357e1a566018c9ca8364937652bdef5","openai_api_rates.json":"c0bdfd66ee69f1644eaf188f87f6af5e47beacc0739d20aac5846c923d66fd58"},"reference_time":"2026-11-10T07:00:00Z","sessions":[{"alias":"S1","launch":{"auth_lane":"api","model":"anthropic/claude-haiku-5-5","parent":"root"},"lifecycle":[{"at":"2026-11-10T06:00:10Z","event":"launch","turn":1},{"at":"2026-11-10T06:50:00Z","event":"retire"}],"logs":{"turn-1":{"final_newline":true,"lines":["{\"at\": \"2026-11-10T06:30:02Z\", \"event\": \"request\", \"provider\": \"anthropic\", \"n\": 1, \"http_status\": 200, \"model\": \"claude-haiku-5-5\", \"requested_tier\": null, \"observed_tier\": \"standard\", \"terminal\": \"message_stop\", \"response_id\": \"msg_1\", \"response_model\": \"claude-haiku-5-5\", \"response_model_check\": \"verified\", \"usage\": {\"input_tokens\": 10000, \"cache_read_input_tokens\": 0, \"cache_creation_input_tokens\": 0, \"output_tokens\": 0}, \"seconds\": 1.0}"]}},"session_id":"00000000-0000-4000-8000-0000000000c1"}]},"invariants":["INV-F5"],"profile":"current_producer","provenance":{"origin":"repro_p125.py::test_N08_haiku_long_context"},"reachability":"producer_reachable","role":"finding","schema":"pi-accounting-corpus/1"}
   28| {"ambiguity":null,"case_id":"U08-missing-observed-tier","class":"producer_valid","contract":"pi-accounting-contract/1.4-s1","controls":["VC01-openai-usage-settlement"],"evaluations":[{"admission_request":{"new":"100000000","objective":"o"},"expected":{"admission":{"decision":"refused","limit":"1000000000","new":"100000000","objective":"o"},"attribution":[{"attempt":"res:S1/turn-1:rid=1","objective":"o","ownership":"owned","placement":"inside"}],"objectives":[{"commitments":[{"admission_id":"a1","committed":"500000000","gate":{"qualified":false,"reasons":["NoGateEvidence"]},"kind":"at_least","state":"retired_resumable"}],"objective":"o","owned_closed":{"high":{"kind":"at_least","nano":"500000000"},"low":"0"},"owned_outstanding":{"high":{"kind":"exact","nano":"0"},"low":"0"},"predecessor_obligations":{"high":{"kind":"exact","nano":"0"},"low":"0"},"provisional":false,"reasons":["UnenforcedCommitment","UnresolvedExposure"],"status":"not_established"}]},"name":"default","objectives":[{"admissions":[{"admission_id":"a1","rows":[{"admission_id":"a1","at":"2026-11-10T06:00:05Z","event":"pending","max_usd":1.0},{"admission_id":"a1","at":"2026-11-10T06:00:10Z","event":"launched","session_id":"00000000-0000-4000-8000-0000000000c1"}],"session":"00000000-0000-4000-8000-0000000000c1","source":"journal"}],"closed_at":null,"limit":"1000000000","objective":"o","predecessor":null,"started_at":"2026-11-10T06:00:00Z","tree_sessions":["00000000-0000-4000-8000-0000000000c1"]}]}],"expected":{"attempts":[{"billing":"unresolved","blocking":["UnresolvedExposure"],"correlation":{"kind":"exact"},"diagnostics":[],"evidence":["S1/turn-1:1","S1/turn-1:2"],"execution":"terminal","exposure":{"high":{"basis":"proxy_reservation","kind":"at_least","nano":"500000000"},"low":{"basis":"none","nano":"0"},"pool":"pi_api"},"forwarding":"forwarded","key":"res:S1/turn-1:rid=1","obligation":{"amount_source":"reservation_bound","state":"closed"},"replacement":{"eligible":false,"reason":"UsageUnpriceable"},"settlement":{"basis":"usage","charged":"30000000","decision":"valid","reasons":[],"reserved":"500000000"},"valuation":{"priced_high":"30000000","reasons":["TierUnestablished"],"state":"unpriceable","supported_low":null}}],"inventory":[],"lifecycle":{"S1":{"clock":"trusted","reopened":false,"state":"retired"}},"non_attempt":[],"unreadable":[]},"historical":{"a134_086bdec1":{"blocking_kinds":["pi_settlement_invalid"],"decision":"not_established","evaluation":"default","note":"comparison only; A1.3.4 has no ownership (D1) concept: it attributes by session tree and time window","reader_sha256":"086bdec1d443064f6847821858f34bbf227f8804a9a5fc8a362416f62b29d621","reasons":["objective accounting unavailable, unpriced or inconsistent"],"spent_high_nano":null,"spent_low_nano":null}},"inputs":{"rates":{"anthropic_api_rates.json":"8a286433d9e08b37533f2f908f2db60c4357e1a566018c9ca8364937652bdef5","openai_api_rates.json":"c0bdfd66ee69f1644eaf188f87f6af5e47beacc0739d20aac5846c923d66fd58"},"reference_time":"2026-11-10T07:00:00Z","sessions":[{"alias":"S1","launch":{"auth_lane":"api","model":"openai/gpt-6-astra","parent":"root"},"lifecycle":[{"at":"2026-11-10T06:00:10Z","event":"launch","turn":1},{"at":"2026-11-10T06:50:00Z","event":"retire"}],"logs":{"turn-1":{"final_newline":true,"lines":["{\"at\": \"2026-11-10T06:30:00Z\", \"event\": \"usd_reserved\", \"n\": 1, \"rid\": 1, \"reserved_usd\": 0.5, \"input_bound\": 6000, \"bound_basis\": \"bytes\"}","{\"at\": \"2026-11-10T06:30:02Z\", \"event\": \"request\", \"n\": 1, \"http_status\": 200, \"model\": \"gpt-6-astra\", \"requested_tier\": null, \"observed_tier\": null, \"terminal\": \"response.completed\", \"response_id\": \"resp_1\", \"response_model\": \"gpt-6-astra\", \"usage\": {\"input_tokens\": 1000, \"output_tokens\": 100, \"cached_tokens\": 0, \"cache_write_tokens\": 0, \"reasoning_tokens\": 0}, \"seconds\": 1.0, \"usd\": {\"rid\": 1, \"reserved_usd\": 0.5, \"charged_usd\": 0.03, \"charge_basis\": \"usage\", \"session_spent_usd\": 0.03, \"multiplier_floor\": 0.0}}"]}},"session_id":"00000000-0000-4000-8000-0000000000c1"}]},"invariants":["INV-A6"],"profile":"current_producer","provenance":{"origin":"repro_p121.py::test_U08_missing_tier_usage_settlement_rejected","variant":"claimed"},"reachability":"producer_reachable","role":"finding","schema":"pi-accounting-corpus/1"}
   29| {"added_in":"1.1-s1","ambiguity":null,"case_id":"Q5-unrecognised-openai-tier","class":"producer_valid","contract":"pi-accounting-contract/1.4-s1","controls":["U08-missing-observed-tier","VC01-openai-usage-settlement"],"evaluations":[{"admission_request":{"new":"100000000","objective":"o"},"expected":{"admission":{"decision":"refused","limit":"1000000000","new":"100000000","objective":"o"},"attribution":[{"attempt":"res:S1/turn-1:rid=1","objective":"o","ownership":"owned","placement":"inside"}],"objectives":[{"commitments":[{"admission_id":"a1","committed":"500000000","gate":{"qualified":false,"reasons":["NoGateEvidence"]},"kind":"at_least","state":"retired_resumable"}],"objective":"o","owned_closed":{"high":{"kind":"at_least","nano":"500000000"},"low":"0"},"owned_outstanding":{"high":{"kind":"exact","nano":"0"},"low":"0"},"predecessor_obligations":{"high":{"kind":"exact","nano":"0"},"low":"0"},"provisional":false,"reasons":["UnenforcedCommitment","UnresolvedExposure"],"status":"not_established"}]},"name":"default","objectives":[{"admissions":[{"admission_id":"a1","rows":[{"admission_id":"a1","at":"2026-11-10T06:00:05Z","event":"pending","max_usd":1.0},{"admission_id":"a1","at":"2026-11-10T06:00:10Z","event":"launched","session_id":"00000000-0000-4000-8000-0000000000c1"}],"session":"00000000-0000-4000-8000-0000000000c1","source":"journal"}],"closed_at":null,"limit":"1000000000","objective":"o","predecessor":null,"started_at":"2026-11-10T06:00:00Z","tree_sessions":["00000000-0000-4000-8000-0000000000c1"]}]}],"expected":{"attempts":[{"billing":"unresolved","blocking":["UnresolvedExposure"],"correlation":{"kind":"exact"},"diagnostics":[],"evidence":["S1/turn-1:1","S1/turn-1:2"],"execution":"terminal","exposure":{"high":{"basis":"proxy_reservation","kind":"at_least","nano":"500000000"},"low":{"basis":"none","nano":"0"},"pool":"pi_api"},"forwarding":"forwarded","key":"res:S1/turn-1:rid=1","obligation":{"amount_source":"reservation_bound","state":"closed"},"replacement":{"eligible":false,"reason":"UsageUnpriceable"},"settlement":{"basis":"usage","charged":"30000000","decision":"valid","reasons":[],"reserved":"500000000"},"valuation":{"priced_high":"30000000","reasons":["TierUnestablished"],"state":"unpriceable","supported_low":null}}],"inventory":[],"lifecycle":{"S1":{"clock":"trusted","reopened":false,"state":"retired"}},"non_attempt":[],"unreadable":[]},"historical":{"a134_086bdec1":{"blocking_kinds":["pi_settlement_invalid"],"decision":"not_established","evaluation":"default","note":"comparison only; A1.3.4 has no ownership (D1) concept: it attributes by session tree and time window","reader_sha256":"086bdec1d443064f6847821858f34bbf227f8804a9a5fc8a362416f62b29d621","reasons":["objective accounting unavailable, unpriced or inconsistent"],"spent_high_nano":null,"spent_low_nano":null}},"inputs":{"rates":{"anthropic_api_rates.json":"8a286433d9e08b37533f2f908f2db60c4357e1a566018c9ca8364937652bdef5","openai_api_rates.json":"c0bdfd66ee69f1644eaf188f87f6af5e47beacc0739d20aac5846c923d66fd58"},"reference_time":"2026-11-10T07:00:00Z","sessions":[{"alias":"S1","launch":{"auth_lane":"api","model":"openai/gpt-6-astra","parent":"root"},"lifecycle":[{"at":"2026-11-10T06:00:10Z","event":"launch","turn":1},{"at":"2026-11-10T06:50:00Z","event":"retire"}],"logs":{"turn-1":{"final_newline":true,"lines":["{\"at\": \"2026-11-10T06:30:00Z\", \"event\": \"usd_reserved\", \"n\": 1, \"rid\": 1, \"reserved_usd\": 0.5, \"input_bound\": 6000, \"bound_basis\": \"bytes\"}","{\"at\": \"2026-11-10T06:30:02Z\", \"event\": \"request\", \"n\": 1, \"http_status\": 200, \"model\": \"gpt-6-astra\", \"requested_tier\": null, \"observed_tier\": \"scale\", \"terminal\": \"response.completed\", \"response_id\": \"resp_1\", \"response_model\": \"gpt-6-astra\", \"usage\": {\"input_tokens\": 1000, \"output_tokens\": 100, \"cached_tokens\": 0, \"cache_write_tokens\": 0, \"reasoning_tokens\": 0}, \"seconds\": 1.0, \"usd\": {\"rid\": 1, \"reserved_usd\": 0.5, \"charged_usd\": 0.03, \"charge_basis\": \"usage\", \"session_spent_usd\": 0.03, \"multiplier_floor\": 0.0}}"]}},"session_id":"00000000-0000-4000-8000-0000000000c1"}]},"invariants":["INV-F5"],"profile":"current_producer","provenance":{"origin":"contract::V-3 unrecognised observed_tier string (owner disposition Q5, P1.29)"},"reachability":"producer_reachable","role":"control","schema":"pi-accounting-corpus/1"}
   30| {"ambiguity":null,"case_id":"P2-usage-settlement-unpriced-identity-keeps-charge","class":"adversarial_synthetic","contract":"pi-accounting-contract/1.4-s1","controls":["V01-control-producer-price"],"evaluations":[{"admission_request":{"new":"100000000","objective":"o"},"expected":{"admission":{"decision":"refused","limit":"1000000000","new":"100000000","objective":"o"},"attribution":[{"attempt":"res:S1/turn-1:rid=1","objective":"o","ownership":"owned","placement":"inside"}],"objectives":[{"commitments":[{"admission_id":"a1","committed":"200000000","gate":{"qualified":false,"reasons":["NoGateEvidence"]},"kind":"at_least","state":"retired_resumable"}],"objective":"o","owned_closed":{"high":{"kind":"at_least","nano":"800000000"},"low":"0"},"owned_outstanding":{"high":{"kind":"exact","nano":"0"},"low":"0"},"predecessor_obligations":{"high":{"kind":"exact","nano":"0"},"low":"0"},"provisional":false,"reasons":["IdentityContradiction","UnenforcedCommitment","UnresolvedExposure"],"status":"not_established"}]},"name":"default","objectives":[{"admissions":[{"admission_id":"a1","rows":[{"admission_id":"a1","at":"2026-11-10T06:00:05Z","event":"pending","max_usd":1.0},{"admission_id":"a1","at":"2026-11-10T06:00:10Z","event":"launched","session_id":"00000000-0000-4000-8000-0000000000c1"}],"session":"00000000-0000-4000-8000-0000000000c1","source":"journal"}],"closed_at":null,"limit":"1000000000","objective":"o","predecessor":null,"started_at":"2026-11-10T06:00:00Z","tree_sessions":["00000000-0000-4000-8000-0000000000c1"]}]}],"expected":{"attempts":[{"billing":"unresolved","blocking":["IdentityContradiction","UnresolvedExposure"],"correlation":{"kind":"exact"},"diagnostics":[],"evidence":["S1/turn-1:1","S1/turn-1:2"],"execution":"terminal","exposure":{"high":{"basis":"proxy_reservation","kind":"at_least","nano":"800000000"},"low":{"basis":"none","nano":"0"},"pool":"pi_api"},"forwarding":"forwarded","key":"res:S1/turn-1:rid=1","obligation":{"amount_source":"reservation_bound","state":"closed"},"replacement":{"eligible":false,"reason":"UsageUnpriceable"},"settlement":{"basis":"usage","charged":"600000000","decision":"valid","reasons":[],"reserved":"800000000"},"valuation":{"derivations":[{"for":"high","model":"gpt-6-astra","nano":"600000000","provider":"openai","tier_multiplier":"1","usage":{"cache_write_tokens":0,"cached_tokens":0,"input_tokens":0,"output_tokens":12000}}],"priced_high":"600000000","reasons":["IdentityWithoutRate"],"state":"unpriceable","supported_low":null}}],"inventory":[],"lifecycle":{"S1":{"clock":"trusted","reopened":false,"state":"retired"}},"non_attempt":[],"unreadable":[]},"historical":{"a134_086bdec1":{"blocking_kinds":["pi_model_identity_conflict"],"decision":"not_established","evaluation":"default","note":"comparison only; A1.3.4 has no ownership (D1) concept: it attributes by session tree and time window","reader_sha256":"086bdec1d443064f6847821858f34bbf227f8804a9a5fc8a362416f62b29d621","reasons":["objective accounting unavailable, unpriced or inconsistent"],"spent_high_nano":null,"spent_low_nano":null}},"inputs":{"rates":{"anthropic_api_rates.json":"8a286433d9e08b37533f2f908f2db60c4357e1a566018c9ca8364937652bdef5","openai_api_rates.json":"c0bdfd66ee69f1644eaf188f87f6af5e47beacc0739d20aac5846c923d66fd58"},"reference_time":"2026-11-10T07:00:00Z","sessions":[{"alias":"S1","launch":{"auth_lane":"api","model":"openai/gpt-6-astra","parent":"root"},"lifecycle":[{"at":"2026-11-10T06:00:10Z","event":"launch","turn":1},{"at":"2026-11-10T06:50:00Z","event":"retire"}],"logs":{"turn-1":{"final_newline":true,"lines":["{\"at\": \"2026-11-10T06:30:00Z\", \"event\": \"usd_reserved\", \"n\": 1, \"rid\": 1, \"reserved_usd\": 0.8, \"input_bound\": 6000, \"bound_basis\": \"bytes\"}","{\"at\": \"2026-11-10T06:30:02Z\", \"event\": \"request\", \"n\": 1, \"http_status\": 200, \"model\": \"gpt-6-astra\", \"requested_tier\": null, \"observed_tier\": \"default\", \"terminal\": \"response.completed\", \"response_id\": \"resp_1\", \"response_model\": \"gpt-9-unpriced\", \"usage\": {\"input_tokens\": 0, \"output_tokens\": 12000, \"cached_tokens\": 0, \"cache_write_tokens\": 0, \"reasoning_tokens\": 0}, \"seconds\": 1.0, \"usd\": {\"rid\": 1, \"reserved_usd\": 0.8, \"charged_usd\": 0.6, \"charge_basis\": \"usage\", \"session_spent_usd\": 0.6, \"multiplier_floor\": 0.0}}"]}},"session_id":"00000000-0000-4000-8000-0000000000c1"}]},"invariants":["INV-F3","INV-F5"],"profile":"current_producer","provenance":{"origin":"test_usage_ledger_pi_a134.py::test_N01_unpriced_identity_on_a_usage_settlement_keeps_the_charge"},"reachability":"producer_reachable","role":"finding","schema":"pi-accounting-corpus/1"}
   31| {"ambiguity":null,"case_id":"V01-usage-settlement-other-response-model","class":"adversarial_synthetic","contract":"pi-accounting-contract/1.4-s1","controls":["VC02-anthropic-usage-settlement"],"evaluations":[{"admission_request":{"new":"100000000","objective":"o"},"expected":{"admission":{"decision":"refused","limit":"1000000000","new":"100000000","objective":"o"},"attribution":[{"attempt":"res:S1/turn-1:rid=1","objective":"o","ownership":"owned","placement":"inside"}],"objectives":[{"commitments":[{"admission_id":"a1","committed":"920000000","gate":{"qualified":false,"reasons":["NoGateEvidence"]},"kind":"at_least","state":"retired_resumable"}],"objective":"o","owned_closed":{"high":{"kind":"exact","nano":"80000000"},"low":"20000000"},"owned_outstanding":{"high":{"kind":"exact","nano":"0"},"low":"0"},"predecessor_obligations":{"high":{"kind":"exact","nano":"0"},"low":"0"},"provisional":false,"reasons":["IdentityContradiction","UnenforcedCommitment"],"status":"not_established"}]},"name":"default","objectives":[{"admissions":[{"admission_id":"a1","rows":[{"admission_id":"a1","at":"2026-11-10T06:00:05Z","event":"pending","max_usd":1.0},{"admission_id":"a1","at":"2026-11-10T06:00:10Z","event":"launched","session_id":"00000000-0000-4000-8000-0000000000c1"}],"session":"00000000-0000-4000-8000-0000000000c1","source":"journal"}],"closed_at":null,"limit":"1000000000","objective":"o","predecessor":null,"started_at":"2026-11-10T06:00:00Z","tree_sessions":["00000000-0000-4000-8000-0000000000c1"]}]}],"expected":{"attempts":[{"billing":"established","blocking":["IdentityContradiction"],"correlation":{"kind":"exact"},"diagnostics":[],"evidence":["S1/turn-1:1","S1/turn-1:2"],"execution":"terminal","exposure":{"high":{"basis":"usage_valuation","kind":"exact","nano":"80000000"},"low":{"basis":"usage_valuation","nano":"20000000"},"pool":"pi_api"},"forwarding":"forwarded","key":"res:S1/turn-1:rid=1","obligation":{"amount_source":"settlement_amount","state":"closed"},"replacement":{"eligible":true,"reason":"UsageBound"},"settlement":{"basis":"usage","charged":"80000000","decision":"valid","reasons":[],"reserved":"300000000"},"valuation":{"complete":true,"derivations":[{"for":"low","model":"claude-sonnet-5","nano":"20000000","provider":"anthropic","remainder_rate":"max","split":"reported","tier_multiplier":"1","usage":{"cache_creation_input_tokens":0,"cache_read_input_tokens":0,"input_tokens":10000,"output_tokens":0}},{"for":"high","model":"claude-fable-5-2","nano":"80000000","provider":"anthropic","remainder_rate":"max","split":"reported","tier_multiplier":"1","usage":{"cache_creation_input_tokens":0,"cache_read_input_tokens":0,"input_tokens":10000,"output_tokens":0}}],"high":"80000000","identities":["claude-fable-5-2","claude-sonnet-5"],"low":"20000000","state":"valued"}}],"inventory":[],"lifecycle":{"S1":{"clock":"trusted","reopened":false,"state":"retired"}},"non_attempt":[],"unreadable":[]},"historical":{"a134_086bdec1":{"blocking_kinds":["pi_model_identity_conflict"],"decision":"not_established","evaluation":"default","note":"comparison only; A1.3.4 has no ownership (D1) concept: it attributes by session tree and time window","reader_sha256":"086bdec1d443064f6847821858f34bbf227f8804a9a5fc8a362416f62b29d621","reasons":["objective accounting unavailable, unpriced or inconsistent"],"spent_high_nano":null,"spent_low_nano":null}},"inputs":{"rates":{"anthropic_api_rates.json":"8a286433d9e08b37533f2f908f2db60c4357e1a566018c9ca8364937652bdef5","openai_api_rates.json":"c0bdfd66ee69f1644eaf188f87f6af5e47beacc0739d20aac5846c923d66fd58"},"reference_time":"2026-11-10T07:00:00Z","sessions":[{"alias":"S1","launch":{"auth_lane":"api","model":"anthropic/claude-fable-5-2","parent":"root"},"lifecycle":[{"at":"2026-11-10T06:00:10Z","event":"launch","turn":1},{"at":"2026-11-10T06:50:00Z","event":"retire"}],"logs":{"turn-1":{"final_newline":true,"lines":["{\"at\": \"2026-11-10T06:30:00Z\", \"event\": \"usd_reserved\", \"n\": 1, \"rid\": 1, \"reserved_usd\": 0.3, \"input_bound\": 6000, \"bound_basis\": \"bytes\"}","{\"at\": \"2026-11-10T06:30:01Z\", \"event\": \"request\", \"provider\": \"anthropic\", \"n\": 1, \"http_status\": 200, \"model\": \"claude-fable-5-2\", \"requested_tier\": null, \"observed_tier\": \"standard\", \"terminal\": \"message_stop\", \"response_id\": \"msg_1\", \"response_model\": \"claude-sonnet-5\", \"response_model_check\": \"verified\", \"usage\": {\"input_tokens\": 10000, \"cache_read_input_tokens\": 0, \"cache_creation_input_tokens\": 0, \"output_tokens\": 0}, \"seconds\": 1.0, \"model_mismatch\": true, \"usd\": {\"rid\": 1, \"reserved_usd\": 0.3, \"charged_usd\": 0.08, \"charge_basis\": \"usage\", \"session_spent_usd\": 0.08, \"multiplier_floor\": 0.0}}"]}},"session_id":"00000000-0000-4000-8000-0000000000c1"}]},"invariants":["INV-A2","INV-F4"],"profile":"current_producer","provenance":{"origin":"test_usage_ledger_pi_a134.py::test_N01_usage_settlement_with_another_response_model"},"reachability":"producer_unreachable","role":"finding","schema":"pi-accounting-corpus/1"}
   32| {"ambiguity":{"fields":["attempts.valuation"],"note":"Which usage schema applies when the row names another provider is undetermined; the contract prices by the session launch provider, and the lane contradiction blocks regardless."},"case_id":"N03-api-row-other-provider","class":"adversarial_synthetic","contract":"pi-accounting-contract/1.4-s1","controls":["VC02-anthropic-usage-settlement"],"evaluations":[{"admission_request":{"new":"100000000","objective":"o"},"expected":{"admission":{"decision":"refused","limit":"1000000000","new":"100000000","objective":"o"},"attribution":[{"attempt":"res:S1/turn-1:rid=1","objective":"o","ownership":"owned","placement":"inside"}],"objectives":[{"commitments":[{"admission_id":"a1","committed":"972000000","gate":{"qualified":false,"reasons":["NoGateEvidence"]},"kind":"at_least","state":"retired_resumable"}],"objective":"o","owned_closed":{"high":{"kind":"exact","nano":"28000000"},"low":"28000000"},"owned_outstanding":{"high":{"kind":"exact","nano":"0"},"low":"0"},"predecessor_obligations":{"high":{"kind":"exact","nano":"0"},"low":"0"},"provisional":false,"reasons":["LaneContradiction","UnenforcedCommitment"],"status":"not_established"}]},"name":"default","objectives":[{"admissions":[{"admission_id":"a1","rows":[{"admission_id":"a1","at":"2026-11-10T06:00:05Z","event":"pending","max_usd":1.0},{"admission_id":"a1","at":"2026-11-10T06:00:10Z","event":"launched","session_id":"00000000-0000-4000-8000-0000000000c1"}],"session":"00000000-0000-4000-8000-0000000000c1","source":"journal"}],"closed_at":null,"limit":"1000000000","objective":"o","predecessor":null,"started_at":"2026-11-10T06:00:00Z","tree_sessions":["00000000-0000-4000-8000-0000000000c1"]}]}],"expected":{"attempts":[{"billing":"established","blocking":["LaneContradiction"],"correlation":{"kind":"exact"},"diagnostics":[],"evidence":["S1/turn-1:1","S1/turn-1:2"],"execution":"terminal","exposure":{"high":{"basis":"usage_valuation","kind":"exact","nano":"28000000"},"low":{"basis":"usage_valuation","nano":"28000000"},"pool":"pi_api"},"forwarding":"forwarded","key":"res:S1/turn-1:rid=1","obligation":{"amount_source":"settlement_amount","state":"closed"},"replacement":{"eligible":true,"reason":"UsageBound"},"settlement":{"basis":"usage","charged":"28000000","decision":"valid","reasons":[],"reserved":"300000000"},"valuation":{"complete":true,"derivations":[{"for":"both","model":"claude-fable-5-2","nano":"28000000","provider":"anthropic","remainder_rate":"max","split":"reported","tier_multiplier":"1","usage":{"cache_creation_input_tokens":0,"cache_read_input_tokens":0,"input_tokens":1000,"output_tokens":500}}],"high":"28000000","identities":["claude-fable-5-2"],"low":"28000000","state":"valued"}}],"inventory":[],"lifecycle":{"S1":{"clock":"trusted","reopened":false,"state":"retired"}},"non_attempt":[],"unreadable":[]},"historical":{"a134_086bdec1":{"blocking_kinds":["pi_lane_conflict","pi_settlement_invalid"],"decision":"not_established","evaluation":"default","note":"comparison only; A1.3.4 has no ownership (D1) concept: it attributes by session tree and time window","reader_sha256":"086bdec1d443064f6847821858f34bbf227f8804a9a5fc8a362416f62b29d621","reasons":["objective accounting unavailable, unpriced or inconsistent"],"spent_high_nano":null,"spent_low_nano":null}},"inputs":{"rates":{"anthropic_api_rates.json":"8a286433d9e08b37533f2f908f2db60c4357e1a566018c9ca8364937652bdef5","openai_api_rates.json":"c0bdfd66ee69f1644eaf188f87f6af5e47beacc0739d20aac5846c923d66fd58"},"reference_time":"2026-11-10T07:00:00Z","sessions":[{"alias":"S1","launch":{"auth_lane":"api","model":"anthropic/claude-fable-5-2","parent":"root"},"lifecycle":[{"at":"2026-11-10T06:00:10Z","event":"launch","turn":1},{"at":"2026-11-10T06:50:00Z","event":"retire"}],"logs":{"turn-1":{"final_newline":true,"lines":["{\"at\": \"2026-11-10T06:30:00Z\", \"event\": \"usd_reserved\", \"n\": 1, \"rid\": 1, \"reserved_usd\": 0.3, \"input_bound\": 6000, \"bound_basis\": \"bytes\"}","{\"at\": \"2026-11-10T06:30:02Z\", \"event\": \"request\", \"provider\": \"openai\", \"n\": 1, \"http_status\": 200, \"model\": \"claude-fable-5-2\", \"requested_tier\": null, \"observed_tier\": \"standard\", \"terminal\": \"message_stop\", \"response_id\": \"msg_1\", \"response_model\": \"claude-fable-5-2\", \"response_model_check\": \"verified\", \"usage\": {\"input_tokens\": 1000, \"cache_read_input_tokens\": 0, \"cache_creation_input_tokens\": 0, \"output_tokens\": 500}, \"seconds\": 1.0, \"usd\": {\"rid\": 1, \"reserved_usd\": 0.3, \"charged_usd\": 0.028, \"charge_basis\": \"usage\", \"session_spent_usd\": 0.028, \"multiplier_floor\": 0.0}}"]}},"session_id":"00000000-0000-4000-8000-0000000000c1"}]},"invariants":["INV-F9"],"profile":"current_producer","provenance":{"origin":"test_usage_ledger_pi_a134.py::test_N03_api_session_provider_evidence"},"reachability":"producer_unreachable","role":"finding","schema":"pi-accounting-corpus/1"}
   33| {"ambiguity":null,"case_id":"N04-budgeted-header-failure-own-reservation","class":"producer_valid","contract":"pi-accounting-contract/1.4-s1","controls":[],"evaluations":[{"admission_request":{"new":"100000000","objective":"o"},"expected":{"admission":{"decision":"refused","limit":"1000000000","new":"100000000","objective":"o"},"attribution":[{"attempt":"res:S1/turn-1:rid=1","objective":"o","ownership":"owned","placement":"inside"},{"attempt":"res:S1/turn-1:rid=2","objective":"o","ownership":"owned","placement":"inside"}],"objectives":[{"commitments":[{"admission_id":"a1","committed":"585000000","gate":{"qualified":false,"reasons":["NoGateEvidence"]},"kind":"at_least","state":"retired_resumable"}],"objective":"o","owned_closed":{"high":{"kind":"exact","nano":"15000000"},"low":"15000000"},"owned_outstanding":{"high":{"kind":"at_least","nano":"400000000"},"low":"0"},"predecessor_obligations":{"high":{"kind":"exact","nano":"0"},"low":"0"},"provisional":false,"reasons":["UnenforcedCommitment","UnresolvedExposure"],"status":"not_established"}]},"name":"default","objectives":[{"admissions":[{"admission_id":"a1","rows":[{"admission_id":"a1","at":"2026-11-10T06:00:05Z","event":"pending","max_usd":1.0},{"admission_id":"a1","at":"2026-11-10T06:00:10Z","event":"launched","session_id":"00000000-0000-4000-8000-0000000000c1"}],"session":"00000000-0000-4000-8000-0000000000c1","source":"journal"}],"closed_at":null,"limit":"1000000000","objective":"o","predecessor":null,"started_at":"2026-11-10T06:00:00Z","tree_sessions":["00000000-0000-4000-8000-0000000000c1"]}]}],"expected":{"attempts":[{"billing":"established","blocking":[],"correlation":{"kind":"exact"},"diagnostics":[],"evidence":["S1/turn-1:1","S1/turn-1:2"],"execution":"terminal","exposure":{"high":{"basis":"usage_valuation","kind":"exact","nano":"15000000"},"low":{"basis":"usage_valuation","nano":"15000000"},"pool":"pi_api"},"forwarding":"forwarded","key":"res:S1/turn-1:rid=1","obligation":{"amount_source":"settlement_amount","state":"closed"},"replacement":{"eligible":true,"reason":"UsageBound"},"settlement":{"basis":"usage","charged":"15000000","decision":"valid","reasons":[],"reserved":"500000000"},"valuation":{"complete":true,"derivations":[{"for":"both","model":"gpt-6-astra","nano":"15000000","provider":"openai","tier_multiplier":"1","usage":{"cache_write_tokens":0,"cached_tokens":0,"input_tokens":1000,"output_tokens":100}}],"high":"15000000","identities":["gpt-6-astra"],"low":"15000000","state":"valued"}},{"billing":"unresolved","blocking":["UnresolvedExposure"],"correlation":{"auxiliary":[{"evidence":"S1/turn-1:4","rule":"RS-2"}],"kind":"exact"},"diagnostics":[],"evidence":["S1/turn-1:3","S1/turn-1:4"],"execution":"terminal","exposure":{"high":{"basis":"proxy_reservation","kind":"at_least","nano":"400000000"},"low":{"basis":"none","nano":"0"},"pool":"pi_api"},"forwarding":"forwarded","key":"res:S1/turn-1:rid=2","obligation":{"amount_source":"reservation_bound","state":"outstanding"},"replacement":{"eligible":false,"reason":"NotUsageSettlement"},"settlement":{"decision":"absent","reasons":[],"reserved":"400000000"},"valuation":{"state":"not_reported"}}],"inventory":[],"lifecycle":{"S1":{"clock":"trusted","reopened":false,"state":"retired"}},"non_attempt":[],"unreadable":[]},"historical":{"a134_086bdec1":{"blocking_kinds":[],"decision":"admitted","evaluation":"default","note":"comparison only; A1.3.4 has no ownership (D1) concept: it attributes by session tree and time window","reader_sha256":"086bdec1d443064f6847821858f34bbf227f8804a9a5fc8a362416f62b29d621","reasons":["within the objective pi_api limit"],"spent_high_nano":"415000000","spent_low_nano":"15000000"}},"inputs":{"rates":{"anthropic_api_rates.json":"8a286433d9e08b37533f2f908f2db60c4357e1a566018c9ca8364937652bdef5","openai_api_rates.json":"c0bdfd66ee69f1644eaf188f87f6af5e47beacc0739d20aac5846c923d66fd58"},"reference_time":"2026-11-10T07:00:00Z","sessions":[{"alias":"S1","launch":{"auth_lane":"api","model":"openai/gpt-6-astra","parent":"root"},"lifecycle":[{"at":"2026-11-10T06:00:10Z","event":"launch","turn":1},{"at":"2026-11-10T06:50:00Z","event":"retire"}],"logs":{"turn-1":{"final_newline":true,"lines":["{\"at\": \"2026-11-10T06:30:00Z\", \"event\": \"usd_reserved\", \"n\": 1, \"rid\": 1, \"reserved_usd\": 0.5, \"input_bound\": 6000, \"bound_basis\": \"bytes\"}","{\"at\": \"2026-11-10T06:30:02Z\", \"event\": \"request\", \"n\": 1, \"http_status\": 200, \"model\": \"gpt-6-astra\", \"requested_tier\": null, \"observed_tier\": \"default\", \"terminal\": \"response.completed\", \"response_id\": \"resp_1\", \"response_model\": \"gpt-6-astra\", \"usage\": {\"input_tokens\": 1000, \"output_tokens\": 100, \"cached_tokens\": 0, \"cache_write_tokens\": 0, \"reasoning_tokens\": 0}, \"seconds\": 1.0, \"usd\": {\"rid\": 1, \"reserved_usd\": 0.5, \"charged_usd\": 0.015, \"charge_basis\": \"usage\", \"session_spent_usd\": 0.015, \"multiplier_floor\": 0.0}}","{\"at\": \"2026-11-10T06:30:04Z\", \"event\": \"usd_reserved\", \"n\": 2, \"rid\": 2, \"reserved_usd\": 0.4, \"input_bound\": 6000, \"bound_basis\": \"bytes\"}","{\"at\": \"2026-11-10T06:30:05Z\", \"event\": \"stream_error\", \"n\": 2}"]}},"session_id":"00000000-0000-4000-8000-0000000000c1"}]},"invariants":["INV-A1","INV-F6"],"profile":"current_producer","provenance":{"origin":"test_usage_ledger_pi_a134.py::test_N04_budgeted_header_failure_is_covered_by_its_own_open_reservation"},"reachability":"producer_reachable","role":"control","schema":"pi-accounting-corpus/1"}
   34| {"ambiguity":null,"case_id":"Ha-inconsistent-cache-split-keeps-supported-low","class":"producer_valid","contract":"pi-accounting-contract/1.4-s1","controls":["Ha-control-consistent-split"],"evaluations":[{"admission_request":{"new":"100000000","objective":"o"},"expected":{"admission":{"decision":"refused","limit":"1000000000","new":"100000000","objective":"o"},"attribution":[{"attempt":"res:S1/turn-1:rid=1","objective":"o","ownership":"owned","placement":"inside"}],"objectives":[{"commitments":[{"admission_id":"a1","committed":"700000000","gate":{"qualified":false,"reasons":["NoGateEvidence"]},"kind":"at_least","state":"retired_resumable"}],"objective":"o","owned_closed":{"high":{"kind":"at_least","nano":"300000000"},"low":"18000000"},"owned_outstanding":{"high":{"kind":"exact","nano":"0"},"low":"0"},"predecessor_obligations":{"high":{"kind":"exact","nano":"0"},"low":"0"},"provisional":false,"reasons":["UnenforcedCommitment","UnresolvedExposure"],"status":"not_established"}]},"name":"default","objectives":[{"admissions":[{"admission_id":"a1","rows":[{"admission_id":"a1","at":"2026-11-10T06:00:05Z","event":"pending","max_usd":1.0},{"admission_id":"a1","at":"2026-11-10T06:00:10Z","event":"launched","session_id":"00000000-0000-4000-8000-0000000000c1"}],"session":"00000000-0000-4000-8000-0000000000c1","source":"journal"}],"closed_at":null,"limit":"1000000000","objective":"o","predecessor":null,"started_at":"2026-11-10T06:00:00Z","tree_sessions":["00000000-0000-4000-8000-0000000000c1"]}]}],"expected":{"attempts":[{"billing":"unresolved","blocking":["UnresolvedExposure"],"correlation":{"kind":"exact"},"diagnostics":["InconsistentCacheSplit","PartialUsage"],"evidence":["S1/turn-1:1","S1/turn-1:2"],"execution":"terminal","exposure":{"high":{"basis":"proxy_reservation","kind":"at_least","nano":"300000000"},"low":{"basis":"usage_valuation","nano":"18000000"},"pool":"pi_api"},"forwarding":"forwarded","key":"res:S1/turn-1:rid=1","obligation":{"amount_source":"reservation_confirmed","state":"closed"},"replacement":{"eligible":false,"reason":"NotUsageSettlement"},"settlement":{"basis":"reservation","charged":"300000000","decision":"valid","reasons":[],"reserved":"300000000"},"valuation":{"complete":false,"derivations":[{"for":"low","model":"claude-fable-5-2","nano":"18000000","provider":"anthropic","remainder_rate":"min","split":"ignored","tier_multiplier":"1","usage":{"cache_creation_input_tokens":1000,"cache_read_input_tokens":0,"ephemeral_1h_input_tokens":500,"ephemeral_5m_input_tokens":1000,"input_tokens":1000,"output_tokens":0}},{"for":"high","model":"claude-fable-5-2","nano":"24000000","provider":"anthropic","remainder_rate":"max","split":"ignored","tier_multiplier":"1","usage":{"cache_creation_input_tokens":1000,"cache_read_input_tokens":0,"ephemeral_1h_input_tokens":500,"ephemeral_5m_input_tokens":1000,"input_tokens":1000,"output_tokens":0}}],"high":"24000000","identities":["claude-fable-5-2"],"low":"18000000","state":"valued"}}],"inventory":[],"lifecycle":{"S1":{"clock":"trusted","reopened":false,"state":"retired"}},"non_attempt":[],"unreadable":[]},"historical":{"a134_086bdec1":{"blocking_kinds":[],"decision":"admitted","evaluation":"default","note":"comparison only; A1.3.4 has no ownership (D1) concept: it attributes by session tree and time window","reader_sha256":"086bdec1d443064f6847821858f34bbf227f8804a9a5fc8a362416f62b29d621","reasons":["within the objective pi_api limit"],"spent_high_nano":"300000000","spent_low_nano":"0"}},"inputs":{"rates":{"anthropic_api_rates.json":"8a286433d9e08b37533f2f908f2db60c4357e1a566018c9ca8364937652bdef5","openai_api_rates.json":"c0bdfd66ee69f1644eaf188f87f6af5e47beacc0739d20aac5846c923d66fd58"},"reference_time":"2026-11-10T07:00:00Z","sessions":[{"alias":"S1","launch":{"auth_lane":"api","model":"anthropic/claude-fable-5-2","parent":"root"},"lifecycle":[{"at":"2026-11-10T06:00:10Z","event":"launch","turn":1},{"at":"2026-11-10T06:50:00Z","event":"retire"}],"logs":{"turn-1":{"final_newline":true,"lines":["{\"at\": \"2026-11-10T06:30:00Z\", \"event\": \"usd_reserved\", \"n\": 1, \"rid\": 1, \"reserved_usd\": 0.3, \"input_bound\": 6000, \"bound_basis\": \"bytes\"}","{\"at\": \"2026-11-10T06:30:02Z\", \"event\": \"request\", \"provider\": \"anthropic\", \"n\": 1, \"http_status\": 200, \"model\": \"claude-fable-5-2\", \"requested_tier\": null, \"observed_tier\": \"standard\", \"terminal\": null, \"response_id\": \"msg_1\", \"response_model\": \"claude-fable-5-2\", \"response_model_check\": \"verified\", \"usage\": {\"input_tokens\": 1000, \"cache_read_input_tokens\": 0, \"cache_creation_input_tokens\": 1000, \"output_tokens\": 0, \"ephemeral_5m_input_tokens\": 1000, \"ephemeral_1h_input_tokens\": 500}, \"seconds\": 1.0, \"usd\": {\"rid\": 1, \"reserved_usd\": 0.3, \"charged_usd\": 0.3, \"charge_basis\": \"reservation\", \"session_spent_usd\": 0.3, \"multiplier_floor\": 0.0}}"]}},"session_id":"00000000-0000-4000-8000-0000000000c1"}]},"invariants":["INV-F11","INV-F4"],"profile":"current_producer","provenance":{"origin":"probe_p127.py::test_Ha","variant":"inconsistent split"},"reachability":"producer_reachable","role":"finding","schema":"pi-accounting-corpus/1"}
   35| {"ambiguity":null,"case_id":"Ha-control-consistent-split","class":"producer_valid","contract":"pi-accounting-contract/1.4-s1","controls":[],"evaluations":[{"admission_request":{"new":"100000000","objective":"o"},"expected":{"admission":{"decision":"refused","limit":"1000000000","new":"100000000","objective":"o"},"attribution":[{"attempt":"res:S1/turn-1:rid=1","objective":"o","ownership":"owned","placement":"inside"}],"objectives":[{"commitments":[{"admission_id":"a1","committed":"700000000","gate":{"qualified":false,"reasons":["NoGateEvidence"]},"kind":"at_least","state":"retired_resumable"}],"objective":"o","owned_closed":{"high":{"kind":"at_least","nano":"300000000"},"low":"18000000"},"owned_outstanding":{"high":{"kind":"exact","nano":"0"},"low":"0"},"predecessor_obligations":{"high":{"kind":"exact","nano":"0"},"low":"0"},"provisional":false,"reasons":["UnenforcedCommitment","UnresolvedExposure"],"status":"not_established"}]},"name":"default","objectives":[{"admissions":[{"admission_id":"a1","rows":[{"admission_id":"a1","at":"2026-11-10T06:00:05Z","event":"pending","max_usd":1.0},{"admission_id":"a1","at":"2026-11-10T06:00:10Z","event":"launched","session_id":"00000000-0000-4000-8000-0000000000c1"}],"session":"00000000-0000-4000-8000-0000000000c1","source":"journal"}],"closed_at":null,"limit":"1000000000","objective":"o","predecessor":null,"started_at":"2026-11-10T06:00:00Z","tree_sessions":["00000000-0000-4000-8000-0000000000c1"]}]}],"expected":{"attempts":[{"billing":"unresolved","blocking":["UnresolvedExposure"],"correlation":{"kind":"exact"},"diagnostics":["PartialUsage"],"evidence":["S1/turn-1:1","S1/turn-1:2"],"execution":"terminal","exposure":{"high":{"basis":"proxy_reservation","kind":"at_least","nano":"300000000"},"low":{"basis":"usage_valuation","nano":"18000000"},"pool":"pi_api"},"forwarding":"forwarded","key":"res:S1/turn-1:rid=1","obligation":{"amount_source":"reservation_confirmed","state":"closed"},"replacement":{"eligible":false,"reason":"NotUsageSettlement"},"settlement":{"basis":"reservation","charged":"300000000","decision":"valid","reasons":[],"reserved":"300000000"},"valuation":{"complete":false,"derivations":[{"for":"both","model":"claude-fable-5-2","nano":"18000000","provider":"anthropic","remainder_rate":"max","split":"reported","tier_multiplier":"1","usage":{"cache_creation_input_tokens":1000,"cache_read_input_tokens":0,"ephemeral_1h_input_tokens":0,"ephemeral_5m_input_tokens":1000,"input_tokens":1000,"output_tokens":0}}],"high":"18000000","identities":["claude-fable-5-2"],"low":"18000000","state":"valued"}}],"inventory":[],"lifecycle":{"S1":{"clock":"trusted","reopened":false,"state":"retired"}},"non_attempt":[],"unreadable":[]},"historical":{"a134_086bdec1":{"blocking_kinds":[],"decision":"admitted","evaluation":"default","note":"comparison only; A1.3.4 has no ownership (D1) concept: it attributes by session tree and time window","reader_sha256":"086bdec1d443064f6847821858f34bbf227f8804a9a5fc8a362416f62b29d621","reasons":["within the objective pi_api limit"],"spent_high_nano":"300000000","spent_low_nano":"18000000"}},"inputs":{"rates":{"anthropic_api_rates.json":"8a286433d9e08b37533f2f908f2db60c4357e1a566018c9ca8364937652bdef5","openai_api_rates.json":"c0bdfd66ee69f1644eaf188f87f6af5e47beacc0739d20aac5846c923d66fd58"},"reference_time":"2026-11-10T07:00:00Z","sessions":[{"alias":"S1","launch":{"auth_lane":"api","model":"anthropic/claude-fable-5-2","parent":"root"},"lifecycle":[{"at":"2026-11-10T06:00:10Z","event":"launch","turn":1},{"at":"2026-11-10T06:50:00Z","event":"retire"}],"logs":{"turn-1":{"final_newline":true,"lines":["{\"at\": \"2026-11-10T06:30:00Z\", \"event\": \"usd_reserved\", \"n\": 1, \"rid\": 1, \"reserved_usd\": 0.3, \"input_bound\": 6000, \"bound_basis\": \"bytes\"}","{\"at\": \"2026-11-10T06:30:02Z\", \"event\": \"request\", \"provider\": \"anthropic\", \"n\": 1, \"http_status\": 200, \"model\": \"claude-fable-5-2\", \"requested_tier\": null, \"observed_tier\": \"standard\", \"terminal\": null, \"response_id\": \"msg_1\", \"response_model\": \"claude-fable-5-2\", \"response_model_check\": \"verified\", \"usage\": {\"input_tokens\": 1000, \"cache_read_input_tokens\": 0, \"cache_creation_input_tokens\": 1000, \"output_tokens\": 0, \"ephemeral_5m_input_tokens\": 1000, \"ephemeral_1h_input_tokens\": 0}, \"seconds\": 1.0, \"usd\": {\"rid\": 1, \"reserved_usd\": 0.3, \"charged_usd\": 0.3, \"charge_basis\": \"reservation\", \"session_spent_usd\": 0.3, \"multiplier_floor\": 0.0}}"]}},"session_id":"00000000-0000-4000-8000-0000000000c1"}]},"invariants":["INV-F11"],"profile":"current_producer","provenance":{"origin":"probe_p127.py::test_Ha","variant":"consistent split"},"reachability":"producer_reachable","role":"control","schema":"pi-accounting-corpus/1"}
   36| {"ambiguity":null,"case_id":"N06-null-cache-counts","class":"producer_valid","contract":"pi-accounting-contract/1.4-s1","controls":["VC01-openai-usage-settlement"],"evaluations":[{"admission_request":{"new":"100000000","objective":"o"},"expected":{"admission":{"decision":"refused","limit":"1000000000","new":"100000000","objective":"o"},"attribution":[{"attempt":"res:S1/turn-1:rid=1","objective":"o","ownership":"owned","placement":"inside"}],"objectives":[{"commitments":[{"admission_id":"a1","committed":"982500000","gate":{"qualified":false,"reasons":["NoGateEvidence"]},"kind":"at_least","state":"retired_resumable"}],"objective":"o","owned_closed":{"high":{"kind":"exact","nano":"17500000"},"low":"6000000"},"owned_outstanding":{"high":{"kind":"exact","nano":"0"},"low":"0"},"predecessor_obligations":{"high":{"kind":"exact","nano":"0"},"low":"0"},"provisional":false,"reasons":["UnenforcedCommitment"],"status":"not_established"}]},"name":"default","objectives":[{"admissions":[{"admission_id":"a1","rows":[{"admission_id":"a1","at":"2026-11-10T06:00:05Z","event":"pending","max_usd":1.0},{"admission_id":"a1","at":"2026-11-10T06:00:10Z","event":"launched","session_id":"00000000-0000-4000-8000-0000000000c1"}],"session":"00000000-0000-4000-8000-0000000000c1","source":"journal"}],"closed_at":null,"limit":"1000000000","objective":"o","predecessor":null,"started_at":"2026-11-10T06:00:00Z","tree_sessions":["00000000-0000-4000-8000-0000000000c1"]}]}],"expected":{"attempts":[{"billing":"established","blocking":[],"correlation":{"kind":"exact"},"diagnostics":[],"evidence":["S1/turn-1:1","S1/turn-1:2"],"execution":"terminal","exposure":{"high":{"basis":"usage_valuation","kind":"exact","nano":"17500000"},"low":{"basis":"usage_valuation","nano":"6000000"},"pool":"pi_api"},"forwarding":"forwarded","key":"res:S1/turn-1:rid=1","obligation":{"amount_source":"settlement_amount","state":"closed"},"replacement":{"eligible":true,"reason":"UsageBound"},"settlement":{"basis":"usage","charged":"15000000","decision":"valid","reasons":[],"reserved":"500000000"},"valuation":{"complete":true,"derivations":[{"for":"low","model":"gpt-6-astra","nano":"6000000","provider":"openai","tier_multiplier":"1","usage":{"cache_write_tokens":0,"cached_tokens":1000,"input_tokens":1000,"output_tokens":100}},{"for":"high","model":"gpt-6-astra","nano":"17500000","provider":"openai","tier_multiplier":"1","usage":{"cache_write_tokens":1000,"cached_tokens":0,"input_tokens":1000,"output_tokens":100}},{"for":"none","model":"gpt-6-astra","nano":"15000000","provider":"openai","tier_multiplier":"1","usage":{"cache_write_tokens":0,"cached_tokens":0,"input_tokens":1000,"output_tokens":100}}],"high":"17500000","identities":["gpt-6-astra"],"low":"6000000","state":"valued"}}],"inventory":[],"lifecycle":{"S1":{"clock":"trusted","reopened":false,"state":"retired"}},"non_attempt":[],"unreadable":[]},"historical":{"a134_086bdec1":{"blocking_kinds":[],"decision":"admitted","evaluation":"default","note":"comparison only; A1.3.4 has no ownership (D1) concept: it attributes by session tree and time window","reader_sha256":"086bdec1d443064f6847821858f34bbf227f8804a9a5fc8a362416f62b29d621","reasons":["within the objective pi_api limit"],"spent_high_nano":"17500000","spent_low_nano":"6000000"}},"inputs":{"rates":{"anthropic_api_rates.json":"8a286433d9e08b37533f2f908f2db60c4357e1a566018c9ca8364937652bdef5","openai_api_rates.json":"c0bdfd66ee69f1644eaf188f87f6af5e47beacc0739d20aac5846c923d66fd58"},"reference_time":"2026-11-10T07:00:00Z","sessions":[{"alias":"S1","launch":{"auth_lane":"api","model":"openai/gpt-6-astra","parent":"root"},"lifecycle":[{"at":"2026-11-10T06:00:10Z","event":"launch","turn":1},{"at":"2026-11-10T06:50:00Z","event":"retire"}],"logs":{"turn-1":{"final_newline":true,"lines":["{\"at\": \"2026-11-10T06:30:00Z\", \"event\": \"usd_reserved\", \"n\": 1, \"rid\": 1, \"reserved_usd\": 0.5, \"input_bound\": 6000, \"bound_basis\": \"bytes\"}","{\"at\": \"2026-11-10T06:30:02Z\", \"event\": \"request\", \"n\": 1, \"http_status\": 200, \"model\": \"gpt-6-astra\", \"requested_tier\": null, \"observed_tier\": \"default\", \"terminal\": \"response.completed\", \"response_id\": \"resp_1\", \"response_model\": \"gpt-6-astra\", \"usage\": {\"input_tokens\": 1000, \"output_tokens\": 100, \"cached_tokens\": null, \"cache_write_tokens\": null, \"reasoning_tokens\": 0}, \"seconds\": 1.0, \"usd\": {\"rid\": 1, \"reserved_usd\": 0.5, \"charged_usd\": 0.015, \"charge_basis\": \"usage\", \"session_spent_usd\": 0.015, \"multiplier_floor\": 0.0}}"]}},"session_id":"00000000-0000-4000-8000-0000000000c1"}]},"invariants":["INV-A6","INV-F11"],"profile":"current_producer","provenance":{"origin":"repro_p125.py::test_N06_absent_cache_counts","variant":"claimed"},"reachability":"producer_reachable","role":"finding","schema":"pi-accounting-corpus/1"}
   37| {"ambiguity":null,"case_id":"N01-identity-refusal-dearer-served","class":"producer_valid","contract":"pi-accounting-contract/1.4-s1","controls":["N01-control-served-pinned"],"evaluations":[{"admission_request":{"new":"40000000","objective":"o"},"expected":{"admission":{"decision":"refused","limit":"100000000","new":"40000000","objective":"o"},"attribution":[{"attempt":"res:S1/turn-1:rid=1","objective":"o","ownership":"owned","placement":"inside"}],"objectives":[{"commitments":[{"admission_id":"a1","committed":"920000000","gate":{"qualified":false,"reasons":["NoGateEvidence"]},"kind":"at_least","state":"retired_resumable"}],"objective":"o","owned_closed":{"high":{"kind":"at_least","nano":"80000000"},"low":"20000000"},"owned_outstanding":{"high":{"kind":"exact","nano":"0"},"low":"0"},"predecessor_obligations":{"high":{"kind":"exact","nano":"0"},"low":"0"},"provisional":false,"reasons":["IdentityContradiction","UnenforcedCommitment","UnresolvedExposure"],"status":"not_established"}]},"name":"default","objectives":[{"admissions":[{"admission_id":"a1","rows":[{"admission_id":"a1","at":"2026-11-10T06:00:05Z","event":"pending","max_usd":1.0},{"admission_id":"a1","at":"2026-11-10T06:00:10Z","event":"launched","session_id":"00000000-0000-4000-8000-0000000000c1"}],"session":"00000000-0000-4000-8000-0000000000c1","source":"journal"}],"closed_at":null,"limit":"100000000","objective":"o","predecessor":null,"started_at":"2026-11-10T06:00:00Z","tree_sessions":["00000000-0000-4000-8000-0000000000c1"]}]}],"expected":{"attempts":[{"billing":"unresolved","blocking":["IdentityContradiction","UnresolvedExposure"],"correlation":{"auxiliary":[{"evidence":"S1/turn-1:2","rule":"RS-3"}],"kind":"exact"},"diagnostics":["PartialUsage"],"evidence":["S1/turn-1:1","S1/turn-1:2","S1/turn-1:3"],"execution":"terminal","exposure":{"high":{"basis":"usage_valuation","kind":"at_least","nano":"80000000"},"low":{"basis":"usage_valuation","nano":"20000000"},"pool":"pi_api"},"forwarding":"forwarded","key":"res:S1/turn-1:rid=1","obligation":{"amount_source":"reservation_confirmed","state":"closed"},"replacement":{"eligible":false,"reason":"NotUsageSettlement"},"settlement":{"basis":"reservation","charged":"50000000","decision":"valid","reasons":[],"reserved":"50000000"},"valuation":{"complete":false,"derivations":[{"for":"low","model":"claude-sonnet-5","nano":"20000000","provider":"anthropic","remainder_rate":"max","split":"reported","tier_multiplier":"1","usage":{"cache_creation_input_tokens":0,"cache_read_input_tokens":0,"input_tokens":10000,"output_tokens":0}},{"for":"high","model":"claude-fable-5-2","nano":"80000000","provider":"anthropic","remainder_rate":"max","split":"reported","tier_multiplier":"1","usage":{"cache_creation_input_tokens":0,"cache_read_input_tokens":0,"input_tokens":10000,"output_tokens":0}}],"high":"80000000","identities":["claude-sonnet-5","claude-fable-5-2"],"low":"20000000","state":"valued"}}],"inventory":[],"lifecycle":{"S1":{"clock":"trusted","reopened":false,"state":"retired"}},"non_attempt":[],"unreadable":[]},"historical":{"a134_086bdec1":{"blocking_kinds":["pi_model_identity_conflict"],"decision":"not_established","evaluation":"default","note":"comparison only; A1.3.4 has no ownership (D1) concept: it attributes by session tree and time window","reader_sha256":"086bdec1d443064f6847821858f34bbf227f8804a9a5fc8a362416f62b29d621","reasons":["objective accounting unavailable, unpriced or inconsistent"],"spent_high_nano":null,"spent_low_nano":null}},"inputs":{"rates":{"anthropic_api_rates.json":"8a286433d9e08b37533f2f908f2db60c4357e1a566018c9ca8364937652bdef5","openai_api_rates.json":"c0bdfd66ee69f1644eaf188f87f6af5e47beacc0739d20aac5846c923d66fd58"},"reference_time":"2026-11-10T07:00:00Z","sessions":[{"alias":"S1","launch":{"auth_lane":"api","model":"anthropic/claude-sonnet-5","parent":"root"},"lifecycle":[{"at":"2026-11-10T06:00:10Z","event":"launch","turn":1},{"at":"2026-11-10T06:50:00Z","event":"retire"}],"logs":{"turn-1":{"final_newline":true,"lines":["{\"at\": \"2026-11-10T06:30:00Z\", \"event\": \"usd_reserved\", \"n\": 1, \"rid\": 1, \"reserved_usd\": 0.05, \"input_bound\": 6000, \"bound_basis\": \"bytes\"}","{\"at\": \"2026-11-10T06:30:01Z\", \"event\": \"response_model_refused\", \"provider\": \"anthropic\", \"n\": 1, \"reason\": \"response_model_mismatch\", \"authorized_model\": \"claude-sonnet-5\", \"response_model\": \"claude-fable-5-2\", \"response_id\": \"msg_x\", \"verified_bytes_forwarded\": false, \"detail\": null}","{\"at\": \"2026-11-10T06:30:01Z\", \"event\": \"request\", \"provider\": \"anthropic\", \"n\": 1, \"http_status\": 200, \"model\": \"claude-sonnet-5\", \"requested_tier\": null, \"observed_tier\": \"standard\", \"terminal\": null, \"response_id\": \"msg_x\", \"response_model\": \"claude-fable-5-2\", \"response_model_check\": \"response_model_mismatch\", \"usage\": {\"input_tokens\": 10000, \"cache_read_input_tokens\": 0, \"cache_creation_input_tokens\": 0, \"output_tokens\": 0}, \"seconds\": 1.0, \"model_mismatch\": true, \"usd\": {\"rid\": 1, \"reserved_usd\": 0.05, \"charged_usd\": 0.05, \"charge_basis\": \"reservation\", \"session_spent_usd\": 0.05, \"multiplier_floor\": 0.0}}"]}},"session_id":"00000000-0000-4000-8000-0000000000c1"}]},"invariants":["INV-A2","INV-F3","INV-F4","INV-F5"],"profile":"current_producer","provenance":{"origin":"repro_p125.py::test_N01_identity_refusal_reservation","variant":"claimed: producer refusal pair, response fable"},"reachability":"producer_reachable","role":"finding","schema":"pi-accounting-corpus/1"}
   38| {"ambiguity":null,"case_id":"N01-control-served-pinned","class":"producer_valid","contract":"pi-accounting-contract/1.4-s1","controls":[],"evaluations":[{"admission_request":{"new":"40000000","objective":"o"},"expected":{"admission":{"decision":"refused","limit":"100000000","new":"40000000","objective":"o"},"attribution":[{"attempt":"res:S1/turn-1:rid=1","objective":"o","ownership":"owned","placement":"inside"}],"objectives":[{"commitments":[{"admission_id":"a1","committed":"950000000","gate":{"qualified":false,"reasons":["NoGateEvidence"]},"kind":"at_least","state":"retired_resumable"}],"objective":"o","owned_closed":{"high":{"kind":"at_least","nano":"50000000"},"low":"20000000"},"owned_outstanding":{"high":{"kind":"exact","nano":"0"},"low":"0"},"predecessor_obligations":{"high":{"kind":"exact","nano":"0"},"low":"0"},"provisional":false,"reasons":["UnenforcedCommitment","UnresolvedExposure"],"status":"not_established"}]},"name":"default","objectives":[{"admissions":[{"admission_id":"a1","rows":[{"admission_id":"a1","at":"2026-11-10T06:00:05Z","event":"pending","max_usd":1.0},{"admission_id":"a1","at":"2026-11-10T06:00:10Z","event":"launched","session_id":"00000000-0000-4000-8000-0000000000c1"}],"session":"00000000-0000-4000-8000-0000000000c1","source":"journal"}],"closed_at":null,"limit":"100000000","objective":"o","predecessor":null,"started_at":"2026-11-10T06:00:00Z","tree_sessions":["00000000-0000-4000-8000-0000000000c1"]}]}],"expected":{"attempts":[{"billing":"unresolved","blocking":["UnresolvedExposure"],"correlation":{"kind":"exact"},"diagnostics":["PartialUsage"],"evidence":["S1/turn-1:1","S1/turn-1:2"],"execution":"terminal","exposure":{"high":{"basis":"proxy_reservation","kind":"at_least","nano":"50000000"},"low":{"basis":"usage_valuation","nano":"20000000"},"pool":"pi_api"},"forwarding":"forwarded","key":"res:S1/turn-1:rid=1","obligation":{"amount_source":"reservation_confirmed","state":"closed"},"replacement":{"eligible":false,"reason":"NotUsageSettlement"},"settlement":{"basis":"reservation","charged":"50000000","decision":"valid","reasons":[],"reserved":"50000000"},"valuation":{"complete":false,"derivations":[{"for":"both","model":"claude-sonnet-5","nano":"20000000","provider":"anthropic","remainder_rate":"max","split":"reported","tier_multiplier":"1","usage":{"cache_creation_input_tokens":0,"cache_read_input_tokens":0,"input_tokens":10000,"output_tokens":0}}],"high":"20000000","identities":["claude-sonnet-5"],"low":"20000000","state":"valued"}}],"inventory":[],"lifecycle":{"S1":{"clock":"trusted","reopened":false,"state":"retired"}},"non_attempt":[],"unreadable":[]},"historical":{"a134_086bdec1":{"blocking_kinds":[],"decision":"admitted","evaluation":"default","note":"comparison only; A1.3.4 has no ownership (D1) concept: it attributes by session tree and time window","reader_sha256":"086bdec1d443064f6847821858f34bbf227f8804a9a5fc8a362416f62b29d621","reasons":["within the objective pi_api limit"],"spent_high_nano":"50000000","spent_low_nano":"20000000"}},"inputs":{"rates":{"anthropic_api_rates.json":"8a286433d9e08b37533f2f908f2db60c4357e1a566018c9ca8364937652bdef5","openai_api_rates.json":"c0bdfd66ee69f1644eaf188f87f6af5e47beacc0739d20aac5846c923d66fd58"},"reference_time":"2026-11-10T07:00:00Z","sessions":[{"alias":"S1","launch":{"auth_lane":"api","model":"anthropic/claude-sonnet-5","parent":"root"},"lifecycle":[{"at":"2026-11-10T06:00:10Z","event":"launch","turn":1},{"at":"2026-11-10T06:50:00Z","event":"retire"}],"logs":{"turn-1":{"final_newline":true,"lines":["{\"at\": \"2026-11-10T06:30:00Z\", \"event\": \"usd_reserved\", \"n\": 1, \"rid\": 1, \"reserved_usd\": 0.05, \"input_bound\": 6000, \"bound_basis\": \"bytes\"}","{\"at\": \"2026-11-10T06:30:01Z\", \"event\": \"request\", \"provider\": \"anthropic\", \"n\": 1, \"http_status\": 200, \"model\": \"claude-sonnet-5\", \"requested_tier\": null, \"observed_tier\": \"standard\", \"terminal\": null, \"response_id\": \"msg_x\", \"response_model\": \"claude-sonnet-5\", \"response_model_check\": \"verified\", \"usage\": {\"input_tokens\": 10000, \"cache_read_input_tokens\": 0, \"cache_creation_input_tokens\": 0, \"output_tokens\": 0}, \"seconds\": 1.0, \"usd\": {\"rid\": 1, \"reserved_usd\": 0.05, \"charged_usd\": 0.05, \"charge_basis\": \"reservation\", \"session_spent_usd\": 0.05, \"multiplier_floor\": 0.0}}"]}},"session_id":"00000000-0000-4000-8000-0000000000c1"}]},"invariants":["INV-A1","INV-F4"],"profile":"current_producer","provenance":{"origin":"repro_p125.py::test_N01_identity_refusal_reservation","variant":"control: response model = pinned sonnet"},"reachability":"producer_reachable","role":"control","schema":"pi-accounting-corpus/1"}
   39| {"ambiguity":null,"case_id":"VC06-unbudgeted-completed-request","class":"producer_valid","contract":"pi-accounting-contract/1.4-s1","controls":[],"evaluations":[{"admission_request":{"new":"100000000","objective":"o"},"expected":{"admission":{"decision":"not_established","limit":"1000000000","new":"100000000","objective":"o"},"attribution":[{"attempt":"row:S1/turn-1:line=1","candidates":["o"],"objective":null,"ownership":"unadmitted","placement":"uncertain"}],"objectives":[{"commitments":[],"objective":"o","owned_closed":{"high":{"kind":"exact","nano":"0"},"low":"0"},"owned_outstanding":{"high":{"kind":"exact","nano":"0"},"low":"0"},"predecessor_obligations":{"high":{"kind":"exact","nano":"0"},"low":"0"},"provisional":false,"reasons":["HistoricalUnadmitted"],"status":"not_established"}]},"name":"default","objectives":[{"admissions":[],"closed_at":null,"limit":"1000000000","objective":"o","predecessor":null,"started_at":"2026-11-10T06:00:00Z","tree_sessions":["00000000-0000-4000-8000-0000000000c1"]}]}],"expected":{"attempts":[{"billing":"established","blocking":[],"correlation":{"kind":"exact"},"diagnostics":[],"evidence":["S1/turn-1:1"],"execution":"terminal","exposure":{"high":{"basis":"usage_valuation","kind":"exact","nano":"15000000"},"low":{"basis":"usage_valuation","nano":"15000000"},"pool":"pi_api"},"forwarding":"forwarded","key":"row:S1/turn-1:line=1","obligation":{"amount_source":"valuation_or_unresolved","state":"none"},"replacement":{"eligible":false,"reason":"NoReservation"},"settlement":{"decision":"not_applicable","reasons":[]},"valuation":{"complete":true,"derivations":[{"for":"both","model":"gpt-6-astra","nano":"15000000","provider":"openai","tier_multiplier":"1","usage":{"cache_write_tokens":0,"cached_tokens":0,"input_tokens":1000,"output_tokens":100}}],"high":"15000000","identities":["gpt-6-astra"],"low":"15000000","state":"valued"}}],"inventory":[],"lifecycle":{"S1":{"clock":"trusted","reopened":false,"state":"retired"}},"non_attempt":[],"unreadable":[]},"historical":{"a134_086bdec1":{"blocking_kinds":[],"decision":"admitted","evaluation":"default","note":"comparison only; A1.3.4 has no ownership (D1) concept: it attributes by session tree and time window","reader_sha256":"086bdec1d443064f6847821858f34bbf227f8804a9a5fc8a362416f62b29d621","reasons":["within the objective pi_api limit"],"spent_high_nano":"15000000","spent_low_nano":"15000000"}},"inputs":{"rates":{"anthropic_api_rates.json":"8a286433d9e08b37533f2f908f2db60c4357e1a566018c9ca8364937652bdef5","openai_api_rates.json":"c0bdfd66ee69f1644eaf188f87f6af5e47beacc0739d20aac5846c923d66fd58"},"reference_time":"2026-11-10T07:00:00Z","sessions":[{"alias":"S1","launch":{"auth_lane":"api","model":"openai/gpt-6-astra","parent":"root"},"lifecycle":[{"at":"2026-11-10T06:00:10Z","event":"launch","turn":1},{"at":"2026-11-10T06:50:00Z","event":"retire"}],"logs":{"turn-1":{"final_newline":true,"lines":["{\"at\": \"2026-11-10T06:30:02Z\", \"event\": \"request\", \"n\": 1, \"http_status\": 200, \"model\": \"gpt-6-astra\", \"requested_tier\": null, \"observed_tier\": \"default\", \"terminal\": \"response.completed\", \"response_id\": \"resp_1\", \"response_model\": \"gpt-6-astra\", \"usage\": {\"input_tokens\": 1000, \"output_tokens\": 100, \"cached_tokens\": 0, \"cache_write_tokens\": 0, \"reasoning_tokens\": 0}, \"seconds\": 1.0}"]}},"session_id":"00000000-0000-4000-8000-0000000000c1"}]},"invariants":["INV-F2","INV-T2"],"profile":"current_producer","provenance":{"origin":"contract::API session without --max-usd: request row, no usd"},"reachability":"producer_reachable","role":"valid_control","schema":"pi-accounting-corpus/1"}
   40| {"added_in":"1.4-s1","ambiguity":null,"case_id":"P133-A01-incomplete-usage-above-reservation","class":"producer_valid","contract":"pi-accounting-contract/1.4-s1","controls":["P130-R01-usage-incomplete-terminal"],"evaluations":[{"admission_request":{"new":"100000000","objective":"o"},"expected":{"admission":{"decision":"refused","limit":"1000000000","new":"100000000","objective":"o"},"attribution":[{"attempt":"res:S1/turn-1:rid=1","objective":"o","ownership":"owned","placement":"inside"}],"objectives":[{"commitments":[{"admission_id":"a1","committed":"985000000","gate":{"qualified":false,"reasons":["NoGateEvidence"]},"kind":"at_least","state":"retired_resumable"}],"objective":"o","owned_closed":{"high":{"kind":"at_least","nano":"15000000"},"low":"15000000"},"owned_outstanding":{"high":{"kind":"exact","nano":"0"},"low":"0"},"predecessor_obligations":{"high":{"kind":"exact","nano":"0"},"low":"0"},"provisional":false,"reasons":["UnenforcedCommitment","UnresolvedExposure"],"status":"not_established"}]},"name":"default","objectives":[{"admissions":[{"admission_id":"a1","rows":[{"admission_id":"a1","at":"2026-11-10T06:00:05Z","event":"pending","max_usd":1.0},{"admission_id":"a1","at":"2026-11-10T06:00:10Z","event":"launched","session_id":"00000000-0000-4000-8000-0000000000c1"}],"session":"00000000-0000-4000-8000-0000000000c1","source":"journal"}],"closed_at":null,"limit":"1000000000","objective":"o","predecessor":null,"started_at":"2026-11-10T06:00:00Z","tree_sessions":["00000000-0000-4000-8000-0000000000c1"]}]}],"expected":{"attempts":[{"billing":"unresolved","blocking":["UnresolvedExposure"],"correlation":{"kind":"exact"},"diagnostics":[],"evidence":["S1/turn-1:1","S1/turn-1:2"],"execution":"terminal","exposure":{"high":{"basis":"usage_valuation","kind":"at_least","nano":"15000000"},"low":{"basis":"usage_valuation","nano":"15000000"},"pool":"pi_api"},"forwarding":"forwarded","key":"res:S1/turn-1:rid=1","obligation":{"amount_source":"reservation_bound","state":"closed"},"replacement":{"eligible":false,"reason":"UsageIncomplete"},"settlement":{"basis":"usage","charged":"15000000","decision":"valid","reasons":[],"reserved":"10000000"},"valuation":{"complete":false,"derivations":[{"for":"both","model":"gpt-6-astra","nano":"15000000","provider":"openai","tier_multiplier":"1","usage":{"cache_write_tokens":0,"cached_tokens":0,"input_tokens":1000,"output_tokens":100}}],"high":"15000000","identities":["gpt-6-astra"],"low":"15000000","state":"valued"}}],"inventory":[],"lifecycle":{"S1":{"clock":"trusted","reopened":false,"state":"retired"}},"non_attempt":[],"unreadable":[]},"historical":{"a134_086bdec1":{"blocking_kinds":[],"decision":"admitted","evaluation":"default","note":"comparison only; A1.3.4 has no ownership (D1) concept: it attributes by session tree and time window","reader_sha256":"086bdec1d443064f6847821858f34bbf227f8804a9a5fc8a362416f62b29d621","reasons":["within the objective pi_api limit"],"spent_high_nano":"15000000","spent_low_nano":"15000000"}},"inputs":{"rates":{"anthropic_api_rates.json":"8a286433d9e08b37533f2f908f2db60c4357e1a566018c9ca8364937652bdef5","openai_api_rates.json":"c0bdfd66ee69f1644eaf188f87f6af5e47beacc0739d20aac5846c923d66fd58"},"reference_time":"2026-11-10T07:00:00Z","sessions":[{"alias":"S1","launch":{"auth_lane":"api","model":"openai/gpt-6-astra","parent":"root"},"lifecycle":[{"at":"2026-11-10T06:00:10Z","event":"launch","turn":1},{"at":"2026-11-10T06:50:00Z","event":"retire"}],"logs":{"turn-1":{"final_newline":true,"lines":["{\"at\": \"2026-11-10T06:30:00Z\", \"event\": \"usd_reserved\", \"n\": 1, \"rid\": 1, \"reserved_usd\": 0.01, \"input_bound\": 6000, \"bound_basis\": \"bytes\"}","{\"at\": \"2026-11-10T06:30:02Z\", \"event\": \"request\", \"n\": 1, \"http_status\": 200, \"model\": \"gpt-6-astra\", \"requested_tier\": null, \"observed_tier\": \"default\", \"terminal\": \"response.incomplete\", \"response_id\": \"resp_1\", \"response_model\": \"gpt-6-astra\", \"usage\": {\"input_tokens\": 1000, \"output_tokens\": 100, \"cached_tokens\": 0, \"cache_write_tokens\": 0, \"reasoning_tokens\": 0}, \"seconds\": 1.0, \"usd\": {\"rid\": 1, \"reserved_usd\": 0.01, \"charged_usd\": 0.015, \"charge_basis\": \"usage\", \"session_spent_usd\": 0.015, \"multiplier_floor\": 0.0}}"]}},"session_id":"00000000-0000-4000-8000-0000000000c1"}]},"invariants":["INV-F3","INV-F13"],"profile":"current_producer","provenance":{"origin":"contract::targeted case (from P130-R01-usage-incomplete-terminal)"},"reachability":"producer_reachable","role":"finding","schema":"pi-accounting-corpus/1"}
   41| {"added_in":"1.4-s1","ambiguity":null,"case_id":"P133-A02-error-status-with-priceable-usage","class":"adversarial_synthetic","contract":"pi-accounting-contract/1.4-s1","controls":["S02-control-error-status-after-reservation"],"evaluations":[{"admission_request":{"new":"100000000","objective":"o"},"expected":{"admission":{"decision":"refused","limit":"1000000000","new":"100000000","objective":"o"},"attribution":[{"attempt":"res:S1/turn-1:rid=1","objective":"o","ownership":"owned","placement":"inside"}],"objectives":[{"commitments":[{"admission_id":"a1","committed":"0","gate":{"qualified":false,"reasons":["NoGateEvidence"]},"kind":"at_least","state":"retired_resumable"}],"objective":"o","owned_closed":{"high":{"kind":"at_least","nano":"1000000000"},"low":"1000000000"},"owned_outstanding":{"high":{"kind":"exact","nano":"0"},"low":"0"},"predecessor_obligations":{"high":{"kind":"exact","nano":"0"},"low":"0"},"provisional":false,"reasons":["UnenforcedCommitment","UnresolvedExposure"],"status":"not_established"}]},"name":"default","objectives":[{"admissions":[{"admission_id":"a1","rows":[{"admission_id":"a1","at":"2026-11-10T06:00:05Z","event":"pending","max_usd":1.0},{"admission_id":"a1","at":"2026-11-10T06:00:10Z","event":"launched","session_id":"00000000-0000-4000-8000-0000000000c1"}],"session":"00000000-0000-4000-8000-0000000000c1","source":"journal"}],"closed_at":null,"limit":"1000000000","objective":"o","predecessor":null,"started_at":"2026-11-10T06:00:00Z","tree_sessions":["00000000-0000-4000-8000-0000000000c1"]}]}],"expected":{"attempts":[{"billing":"unresolved","blocking":["UnresolvedExposure"],"correlation":{"kind":"exact"},"diagnostics":["ErrorStatusSettlement","NonBillingUnverified"],"evidence":["S1/turn-1:1","S1/turn-1:2"],"execution":"terminal","exposure":{"high":{"basis":"usage_valuation","kind":"at_least","nano":"1000000000"},"low":{"basis":"usage_valuation","nano":"1000000000"},"pool":"pi_api"},"forwarding":"forwarded","key":"res:S1/turn-1:rid=1","obligation":{"amount_source":"reservation_bound","state":"closed"},"replacement":{"eligible":false,"reason":"NotUsageSettlement"},"settlement":{"basis":"error_status","charged":"0","decision":"valid","reasons":[],"reserved":"800000000"},"valuation":{"complete":false,"derivations":[{"for":"both","model":"gpt-6-astra","nano":"1000000000","provider":"openai","tier_multiplier":"1","usage":{"cache_write_tokens":0,"cached_tokens":0,"input_tokens":0,"output_tokens":20000}}],"high":"1000000000","identities":["gpt-6-astra"],"low":"1000000000","state":"valued"}}],"inventory":[],"lifecycle":{"S1":{"clock":"trusted","reopened":false,"state":"retired"}},"non_attempt":[],"unreadable":[]},"historical":{"a134_086bdec1":{"blocking_kinds":[],"decision":"admitted","evaluation":"default","note":"comparison only; A1.3.4 has no ownership (D1) concept: it attributes by session tree and time window","reader_sha256":"086bdec1d443064f6847821858f34bbf227f8804a9a5fc8a362416f62b29d621","reasons":["within the objective pi_api limit"],"spent_high_nano":"0","spent_low_nano":"0"}},"inputs":{"rates":{"anthropic_api_rates.json":"8a286433d9e08b37533f2f908f2db60c4357e1a566018c9ca8364937652bdef5","openai_api_rates.json":"c0bdfd66ee69f1644eaf188f87f6af5e47beacc0739d20aac5846c923d66fd58"},"reference_time":"2026-11-10T07:00:00Z","sessions":[{"alias":"S1","launch":{"auth_lane":"api","model":"openai/gpt-6-astra","parent":"root"},"lifecycle":[{"at":"2026-11-10T06:00:10Z","event":"launch","turn":1},{"at":"2026-11-10T06:50:00Z","event":"retire"}],"logs":{"turn-1":{"final_newline":true,"lines":["{\"at\": \"2026-11-10T06:30:00Z\", \"event\": \"usd_reserved\", \"n\": 1, \"rid\": 1, \"reserved_usd\": 0.8, \"input_bound\": 6000, \"bound_basis\": \"bytes\"}","{\"at\": \"2026-11-10T06:30:02Z\", \"event\": \"request\", \"n\": 1, \"http_status\": 429, \"model\": \"gpt-6-astra\", \"requested_tier\": null, \"observed_tier\": \"default\", \"terminal\": null, \"response_id\": null, \"response_model\": \"gpt-6-astra\", \"usage\": {\"input_tokens\": 0, \"output_tokens\": 20000, \"cached_tokens\": 0, \"cache_write_tokens\": 0, \"reasoning_tokens\": 0}, \"seconds\": 1.0, \"usd\": {\"rid\": 1, \"reserved_usd\": 0.8, \"charged_usd\": 0.0, \"charge_basis\": \"error_status\", \"session_spent_usd\": 0.0, \"multiplier_floor\": 0.0}}"]}},"session_id":"00000000-0000-4000-8000-0000000000c1"}]},"invariants":["INV-F3","INV-F4"],"profile":"current_producer","provenance":{"origin":"contract::targeted case (from S02-control-error-status-after-reservation)"},"reachability":"not_established","role":"finding","schema":"pi-accounting-corpus/1"}
   42| {"added_in":"1.4-s1","ambiguity":null,"case_id":"P133-A04-cache-write-absent-legacy","class":"historical_legacy","contract":"pi-accounting-contract/1.4-s1","controls":["VC06-unbudgeted-completed-request"],"evaluations":[{"admission_request":{"new":"100000000","objective":"o"},"expected":{"admission":{"decision":"not_established","limit":"1000000000","new":"100000000","objective":"o"},"attribution":[{"attempt":"row:S1/turn-1:line=1","candidates":["o"],"objective":null,"ownership":"unadmitted","placement":"uncertain"}],"objectives":[{"commitments":[],"objective":"o","owned_closed":{"high":{"kind":"exact","nano":"0"},"low":"0"},"owned_outstanding":{"high":{"kind":"exact","nano":"0"},"low":"0"},"predecessor_obligations":{"high":{"kind":"exact","nano":"0"},"low":"0"},"provisional":false,"reasons":["HistoricalUnadmitted"],"status":"not_established"}]},"name":"default","objectives":[{"admissions":[],"closed_at":null,"limit":"1000000000","objective":"o","predecessor":null,"started_at":"2026-11-10T06:00:00Z","tree_sessions":["00000000-0000-4000-8000-0000000000c1"]}]}],"expected":{"attempts":[{"billing":"established","blocking":[],"correlation":{"kind":"exact"},"diagnostics":[],"evidence":["S1/turn-1:1"],"execution":"terminal","exposure":{"high":{"basis":"usage_valuation","kind":"exact","nano":"17500000"},"low":{"basis":"usage_valuation","nano":"15000000"},"pool":"pi_api"},"forwarding":"forwarded","key":"row:S1/turn-1:line=1","obligation":{"amount_source":"valuation_or_unresolved","state":"none"},"replacement":{"eligible":false,"reason":"NoReservation"},"settlement":{"decision":"not_applicable","reasons":[]},"valuation":{"complete":true,"derivations":[{"for":"low","model":"gpt-6-astra","nano":"15000000","provider":"openai","tier_multiplier":"1","usage":{"cache_write_tokens":0,"cached_tokens":0,"input_tokens":1000,"output_tokens":100}},{"for":"high","model":"gpt-6-astra","nano":"17500000","provider":"openai","tier_multiplier":"1","usage":{"cache_write_tokens":1000,"cached_tokens":0,"input_tokens":1000,"output_tokens":100}}],"high":"17500000","identities":["gpt-6-astra"],"low":"15000000","state":"valued"}}],"inventory":[],"lifecycle":{"S1":{"clock":"trusted","reopened":false,"state":"retired"}},"non_attempt":[],"unreadable":[]},"historical":{"a134_086bdec1":{"blocking_kinds":[],"decision":"admitted","evaluation":"default","note":"comparison only; A1.3.4 has no ownership (D1) concept: it attributes by session tree and time window","reader_sha256":"086bdec1d443064f6847821858f34bbf227f8804a9a5fc8a362416f62b29d621","reasons":["within the objective pi_api limit"],"spent_high_nano":"17500000","spent_low_nano":"15000000"}},"inputs":{"rates":{"anthropic_api_rates.json":"8a286433d9e08b37533f2f908f2db60c4357e1a566018c9ca8364937652bdef5","openai_api_rates.json":"c0bdfd66ee69f1644eaf188f87f6af5e47beacc0739d20aac5846c923d66fd58"},"reference_time":"2026-11-10T07:00:00Z","sessions":[{"alias":"S1","launch":{"auth_lane":"api","model":"openai/gpt-6-astra","parent":"root"},"lifecycle":[{"at":"2026-11-10T06:00:10Z","event":"launch","turn":1},{"at":"2026-11-10T06:50:00Z","event":"retire"}],"logs":{"turn-1":{"final_newline":true,"lines":["{\"at\": \"2026-11-10T06:30:02Z\", \"event\": \"request\", \"n\": 1, \"http_status\": 200, \"model\": \"gpt-6-astra\", \"requested_tier\": null, \"observed_tier\": \"default\", \"terminal\": \"response.completed\", \"response_id\": \"resp_1\", \"response_model\": \"gpt-6-astra\", \"usage\": {\"input_tokens\": 1000, \"output_tokens\": 100, \"cached_tokens\": 0, \"reasoning_tokens\": 0}, \"seconds\": 1.0}"]}},"session_id":"00000000-0000-4000-8000-0000000000c1"}]},"invariants":["INV-F5"],"profile":"current_producer","provenance":{"origin":"contract::targeted case (from VC06-unbudgeted-completed-request)"},"reachability":"legacy","role":"finding","schema":"pi-accounting-corpus/1"}
   43| {"added_in":"1.4-s1","ambiguity":null,"case_id":"P133-A04-cache-counts-both-absent","class":"adversarial_synthetic","contract":"pi-accounting-contract/1.4-s1","controls":["VC06-unbudgeted-completed-request"],"evaluations":[{"admission_request":{"new":"100000000","objective":"o"},"expected":{"admission":{"decision":"not_established","limit":"1000000000","new":"100000000","objective":"o"},"attribution":[{"attempt":"row:S1/turn-1:line=1","candidates":["o"],"objective":null,"ownership":"unadmitted","placement":"uncertain"}],"objectives":[{"commitments":[],"objective":"o","owned_closed":{"high":{"kind":"exact","nano":"0"},"low":"0"},"owned_outstanding":{"high":{"kind":"exact","nano":"0"},"low":"0"},"predecessor_obligations":{"high":{"kind":"exact","nano":"0"},"low":"0"},"provisional":false,"reasons":["HistoricalUnadmitted"],"status":"not_established"}]},"name":"default","objectives":[{"admissions":[],"closed_at":null,"limit":"1000000000","objective":"o","predecessor":null,"started_at":"2026-11-10T06:00:00Z","tree_sessions":["00000000-0000-4000-8000-0000000000c1"]}]}],"expected":{"attempts":[{"billing":"established","blocking":[],"correlation":{"kind":"exact"},"diagnostics":[],"evidence":["S1/turn-1:1"],"execution":"terminal","exposure":{"high":{"basis":"usage_valuation","kind":"exact","nano":"17500000"},"low":{"basis":"usage_valuation","nano":"6000000"},"pool":"pi_api"},"forwarding":"forwarded","key":"row:S1/turn-1:line=1","obligation":{"amount_source":"valuation_or_unresolved","state":"none"},"replacement":{"eligible":false,"reason":"NoReservation"},"settlement":{"decision":"not_applicable","reasons":[]},"valuation":{"complete":true,"derivations":[{"for":"low","model":"gpt-6-astra","nano":"6000000","provider":"openai","tier_multiplier":"1","usage":{"cache_write_tokens":0,"cached_tokens":1000,"input_tokens":1000,"output_tokens":100}},{"for":"high","model":"gpt-6-astra","nano":"17500000","provider":"openai","tier_multiplier":"1","usage":{"cache_write_tokens":1000,"cached_tokens":0,"input_tokens":1000,"output_tokens":100}}],"high":"17500000","identities":["gpt-6-astra"],"low":"6000000","state":"valued"}}],"inventory":[],"lifecycle":{"S1":{"clock":"trusted","reopened":false,"state":"retired"}},"non_attempt":[],"unreadable":[]},"historical":{"a134_086bdec1":{"blocking_kinds":[],"decision":"admitted","evaluation":"default","note":"comparison only; A1.3.4 has no ownership (D1) concept: it attributes by session tree and time window","reader_sha256":"086bdec1d443064f6847821858f34bbf227f8804a9a5fc8a362416f62b29d621","reasons":["within the objective pi_api limit"],"spent_high_nano":"17500000","spent_low_nano":"6000000"}},"inputs":{"rates":{"anthropic_api_rates.json":"8a286433d9e08b37533f2f908f2db60c4357e1a566018c9ca8364937652bdef5","openai_api_rates.json":"c0bdfd66ee69f1644eaf188f87f6af5e47beacc0739d20aac5846c923d66fd58"},"reference_time":"2026-11-10T07:00:00Z","sessions":[{"alias":"S1","launch":{"auth_lane":"api","model":"openai/gpt-6-astra","parent":"root"},"lifecycle":[{"at":"2026-11-10T06:00:10Z","event":"launch","turn":1},{"at":"2026-11-10T06:50:00Z","event":"retire"}],"logs":{"turn-1":{"final_newline":true,"lines":["{\"at\": \"2026-11-10T06:30:02Z\", \"event\": \"request\", \"n\": 1, \"http_status\": 200, \"model\": \"gpt-6-astra\", \"requested_tier\": null, \"observed_tier\": \"default\", \"terminal\": \"response.completed\", \"response_id\": \"resp_1\", \"response_model\": \"gpt-6-astra\", \"usage\": {\"input_tokens\": 1000, \"output_tokens\": 100, \"reasoning_tokens\": 0}, \"seconds\": 1.0}"]}},"session_id":"00000000-0000-4000-8000-0000000000c1"}]},"invariants":["INV-F5"],"profile":"current_producer","provenance":{"origin":"contract::targeted case (from VC06-unbudgeted-completed-request)"},"reachability":"not_established","role":"finding","schema":"pi-accounting-corpus/1"}
   44| {"added_in":"1.4-s1","ambiguity":null,"case_id":"P133-A04-cached-nonzero","class":"producer_valid","contract":"pi-accounting-contract/1.4-s1","controls":[],"evaluations":[{"admission_request":{"new":"100000000","objective":"o"},"expected":{"admission":{"decision":"refused","limit":"1000000000","new":"100000000","objective":"o"},"attribution":[{"attempt":"res:S1/turn-1:rid=1","objective":"o","ownership":"owned","placement":"inside"}],"objectives":[{"commitments":[{"admission_id":"a1","committed":"988600000","gate":{"qualified":false,"reasons":["NoGateEvidence"]},"kind":"at_least","state":"retired_resumable"}],"objective":"o","owned_closed":{"high":{"kind":"exact","nano":"11400000"},"low":"11400000"},"owned_outstanding":{"high":{"kind":"exact","nano":"0"},"low":"0"},"predecessor_obligations":{"high":{"kind":"exact","nano":"0"},"low":"0"},"provisional":false,"reasons":["UnenforcedCommitment"],"status":"not_established"}]},"name":"default","objectives":[{"admissions":[{"admission_id":"a1","rows":[{"admission_id":"a1","at":"2026-11-10T06:00:05Z","event":"pending","max_usd":1.0},{"admission_id":"a1","at":"2026-11-10T06:00:10Z","event":"launched","session_id":"00000000-0000-4000-8000-0000000000c1"}],"session":"00000000-0000-4000-8000-0000000000c1","source":"journal"}],"closed_at":null,"limit":"1000000000","objective":"o","predecessor":null,"started_at":"2026-11-10T06:00:00Z","tree_sessions":["00000000-0000-4000-8000-0000000000c1"]}]}],"expected":{"attempts":[{"billing":"established","blocking":[],"correlation":{"kind":"exact"},"diagnostics":[],"evidence":["S1/turn-1:1","S1/turn-1:2"],"execution":"terminal","exposure":{"high":{"basis":"usage_valuation","kind":"exact","nano":"11400000"},"low":{"basis":"usage_valuation","nano":"11400000"},"pool":"pi_api"},"forwarding":"forwarded","key":"res:S1/turn-1:rid=1","obligation":{"amount_source":"settlement_amount","state":"closed"},"replacement":{"eligible":true,"reason":"UsageBound"},"settlement":{"basis":"usage","charged":"11400000","decision":"valid","reasons":[],"reserved":"500000000"},"valuation":{"complete":true,"derivations":[{"for":"both","model":"gpt-6-astra","nano":"11400000","provider":"openai","tier_multiplier":"1","usage":{"cache_write_tokens":0,"cached_tokens":400,"input_tokens":1000,"output_tokens":100}}],"high":"11400000","identities":["gpt-6-astra"],"low":"11400000","state":"valued"}}],"inventory":[],"lifecycle":{"S1":{"clock":"trusted","reopened":false,"state":"retired"}},"non_attempt":[],"unreadable":[]},"historical":{"a134_086bdec1":{"blocking_kinds":[],"decision":"admitted","evaluation":"default","note":"comparison only; A1.3.4 has no ownership (D1) concept: it attributes by session tree and time window","reader_sha256":"086bdec1d443064f6847821858f34bbf227f8804a9a5fc8a362416f62b29d621","reasons":["within the objective pi_api limit"],"spent_high_nano":"11400000","spent_low_nano":"11400000"}},"inputs":{"rates":{"anthropic_api_rates.json":"8a286433d9e08b37533f2f908f2db60c4357e1a566018c9ca8364937652bdef5","openai_api_rates.json":"c0bdfd66ee69f1644eaf188f87f6af5e47beacc0739d20aac5846c923d66fd58"},"reference_time":"2026-11-10T07:00:00Z","sessions":[{"alias":"S1","launch":{"auth_lane":"api","model":"openai/gpt-6-astra","parent":"root"},"lifecycle":[{"at":"2026-11-10T06:00:10Z","event":"launch","turn":1},{"at":"2026-11-10T06:50:00Z","event":"retire"}],"logs":{"turn-1":{"final_newline":true,"lines":["{\"at\": \"2026-11-10T06:30:00Z\", \"event\": \"usd_reserved\", \"n\": 1, \"rid\": 1, \"reserved_usd\": 0.5, \"input_bound\": 6000, \"bound_basis\": \"bytes\"}","{\"at\": \"2026-11-10T06:30:02Z\", \"event\": \"request\", \"n\": 1, \"http_status\": 200, \"model\": \"gpt-6-astra\", \"requested_tier\": null, \"observed_tier\": \"default\", \"terminal\": \"response.completed\", \"response_id\": \"resp_1\", \"response_model\": \"gpt-6-astra\", \"usage\": {\"input_tokens\": 1000, \"output_tokens\": 100, \"cached_tokens\": 400, \"cache_write_tokens\": 0, \"reasoning_tokens\": 0}, \"seconds\": 1.0, \"usd\": {\"rid\": 1, \"reserved_usd\": 0.5, \"charged_usd\": 0.0114, \"charge_basis\": \"usage\", \"session_spent_usd\": 0.0114, \"multiplier_floor\": 0.0}}"]}},"session_id":"00000000-0000-4000-8000-0000000000c1"}]},"invariants":["INV-F11"],"profile":"current_producer","provenance":{"origin":"contract::targeted case (from VC01-openai-usage-settlement)"},"reachability":"producer_reachable","role":"valid_control","schema":"pi-accounting-corpus/1"}
   45| {"added_in":"1.4-s1","ambiguity":null,"case_id":"P133-A04-cached-malformed","class":"malformed","contract":"pi-accounting-contract/1.4-s1","controls":["VC01-openai-usage-settlement"],"evaluations":[{"admission_request":{"new":"100000000","objective":"o"},"expected":{"admission":{"decision":"refused","limit":"1000000000","new":"100000000","objective":"o"},"attribution":[{"attempt":"res:S1/turn-1:rid=1","objective":"o","ownership":"owned","placement":"inside"}],"objectives":[{"commitments":[{"admission_id":"a1","committed":"500000000","gate":{"qualified":false,"reasons":["NoGateEvidence"]},"kind":"at_least","state":"retired_resumable"}],"objective":"o","owned_closed":{"high":{"kind":"at_least","nano":"500000000"},"low":"0"},"owned_outstanding":{"high":{"kind":"exact","nano":"0"},"low":"0"},"predecessor_obligations":{"high":{"kind":"exact","nano":"0"},"low":"0"},"provisional":false,"reasons":["UnenforcedCommitment","UnresolvedExposure"],"status":"not_established"}]},"name":"default","objectives":[{"admissions":[{"admission_id":"a1","rows":[{"admission_id":"a1","at":"2026-11-10T06:00:05Z","event":"pending","max_usd":1.0},{"admission_id":"a1","at":"2026-11-10T06:00:10Z","event":"launched","session_id":"00000000-0000-4000-8000-0000000000c1"}],"session":"00000000-0000-4000-8000-0000000000c1","source":"journal"}],"closed_at":null,"limit":"1000000000","objective":"o","predecessor":null,"started_at":"2026-11-10T06:00:00Z","tree_sessions":["00000000-0000-4000-8000-0000000000c1"]}]}],"expected":{"attempts":[{"billing":"unresolved","blocking":["UnresolvedExposure"],"correlation":{"kind":"exact"},"diagnostics":[],"evidence":["S1/turn-1:1","S1/turn-1:2"],"execution":"terminal","exposure":{"high":{"basis":"proxy_reservation","kind":"at_least","nano":"500000000"},"low":{"basis":"none","nano":"0"},"pool":"pi_api"},"forwarding":"forwarded","key":"res:S1/turn-1:rid=1","obligation":{"amount_source":"reservation_confirmed","state":"closed"},"replacement":{"eligible":false,"reason":"NotUsageSettlement"},"settlement":{"basis":"reservation","charged":"500000000","decision":"valid","reasons":[],"reserved":"500000000"},"valuation":{"priced_high":null,"reasons":["MalformedCount"],"state":"unpriceable","supported_low":null}}],"inventory":[],"lifecycle":{"S1":{"clock":"trusted","reopened":false,"state":"retired"}},"non_attempt":[],"unreadable":[]},"historical":{"a134_086bdec1":{"blocking_kinds":[],"decision":"admitted","evaluation":"default","note":"comparison only; A1.3.4 has no ownership (D1) concept: it attributes by session tree and time window","reader_sha256":"086bdec1d443064f6847821858f34bbf227f8804a9a5fc8a362416f62b29d621","reasons":["within the objective pi_api limit"],"spent_high_nano":"500000000","spent_low_nano":"0"}},"inputs":{"rates":{"anthropic_api_rates.json":"8a286433d9e08b37533f2f908f2db60c4357e1a566018c9ca8364937652bdef5","openai_api_rates.json":"c0bdfd66ee69f1644eaf188f87f6af5e47beacc0739d20aac5846c923d66fd58"},"reference_time":"2026-11-10T07:00:00Z","sessions":[{"alias":"S1","launch":{"auth_lane":"api","model":"openai/gpt-6-astra","parent":"root"},"lifecycle":[{"at":"2026-11-10T06:00:10Z","event":"launch","turn":1},{"at":"2026-11-10T06:50:00Z","event":"retire"}],"logs":{"turn-1":{"final_newline":true,"lines":["{\"at\": \"2026-11-10T06:30:00Z\", \"event\": \"usd_reserved\", \"n\": 1, \"rid\": 1, \"reserved_usd\": 0.5, \"input_bound\": 6000, \"bound_basis\": \"bytes\"}","{\"at\": \"2026-11-10T06:30:02Z\", \"event\": \"request\", \"n\": 1, \"http_status\": 200, \"model\": \"gpt-6-astra\", \"requested_tier\": null, \"observed_tier\": \"default\", \"terminal\": \"response.completed\", \"response_id\": \"resp_1\", \"response_model\": \"gpt-6-astra\", \"usage\": {\"input_tokens\": 1000, \"output_tokens\": 100, \"cached_tokens\": \"400\", \"cache_write_tokens\": 0, \"reasoning_tokens\": 0}, \"seconds\": 1.0, \"usd\": {\"rid\": 1, \"reserved_usd\": 0.5, \"charged_usd\": 0.5, \"charge_basis\": \"reservation\", \"session_spent_usd\": 0.5, \"multiplier_floor\": 0.0}}"]}},"session_id":"00000000-0000-4000-8000-0000000000c1"}]},"invariants":["INV-F5"],"profile":"current_producer","provenance":{"origin":"contract::targeted case (from VC01-openai-usage-settlement)"},"reachability":"not_established","role":"finding","schema":"pi-accounting-corpus/1"}
   46| {"added_in":"1.4-s1","ambiguity":null,"case_id":"P133-A04-mixed-observations","class":"adversarial_synthetic","contract":"pi-accounting-contract/1.4-s1","controls":["VC01-openai-usage-settlement"],"evaluations":[{"admission_request":{"new":"100000000","objective":"o"},"expected":{"admission":{"decision":"refused","limit":"1000000000","new":"100000000","objective":"o"},"attribution":[{"attempt":"res:S1/turn-1:rid=1","objective":"o","ownership":"owned","placement":"inside"}],"objectives":[{"commitments":[{"admission_id":"a1","committed":"500000000","gate":{"qualified":false,"reasons":["NoGateEvidence"]},"kind":"at_least","state":"retired_resumable"}],"objective":"o","owned_closed":{"high":{"kind":"exact","nano":"0"},"low":"0"},"owned_outstanding":{"high":{"kind":"at_least","nano":"500000000"},"low":"0"},"predecessor_obligations":{"high":{"kind":"exact","nano":"0"},"low":"0"},"provisional":false,"reasons":["SettlementConflict","UnenforcedCommitment","UnresolvedExposure"],"status":"not_established"}]},"name":"default","objectives":[{"admissions":[{"admission_id":"a1","rows":[{"admission_id":"a1","at":"2026-11-10T06:00:05Z","event":"pending","max_usd":1.0},{"admission_id":"a1","at":"2026-11-10T06:00:10Z","event":"launched","session_id":"00000000-0000-4000-8000-0000000000c1"}],"session":"00000000-0000-4000-8000-0000000000c1","source":"journal"}],"closed_at":null,"limit":"1000000000","objective":"o","predecessor":null,"started_at":"2026-11-10T06:00:00Z","tree_sessions":["00000000-0000-4000-8000-0000000000c1"]}]}],"expected":{"attempts":[{"billing":"unresolved","blocking":["SettlementConflict","UnresolvedExposure"],"correlation":{"kind":"exact"},"diagnostics":[],"evidence":["S1/turn-1:1","S1/turn-1:2","S1/turn-1:3"],"execution":"terminal","exposure":{"high":{"basis":"proxy_reservation","kind":"at_least","nano":"500000000"},"low":{"basis":"none","nano":"0"},"pool":"pi_api"},"forwarding":"forwarded","key":"res:S1/turn-1:rid=1","obligation":{"amount_source":"reservation_bound","state":"outstanding"},"replacement":{"eligible":false,"reason":"NotUsageSettlement"},"settlement":{"charged":"15000000","decision":"conflicting","reasons":[],"reserved":"500000000"},"valuation":{"priced_high":"15000000","reasons":["MalformedCount"],"state":"unpriceable","supported_low":null}}],"inventory":[],"lifecycle":{"S1":{"clock":"trusted","reopened":false,"state":"retired"}},"non_attempt":[],"unreadable":[]},"historical":{"a134_086bdec1":{"blocking_kinds":["pi_settlement_conflict"],"decision":"not_established","evaluation":"default","note":"comparison only; A1.3.4 has no ownership (D1) concept: it attributes by session tree and time window","reader_sha256":"086bdec1d443064f6847821858f34bbf227f8804a9a5fc8a362416f62b29d621","reasons":["objective accounting unavailable, unpriced or inconsistent"],"spent_high_nano":null,"spent_low_nano":null}},"inputs":{"rates":{"anthropic_api_rates.json":"8a286433d9e08b37533f2f908f2db60c4357e1a566018c9ca8364937652bdef5","openai_api_rates.json":"c0bdfd66ee69f1644eaf188f87f6af5e47beacc0739d20aac5846c923d66fd58"},"reference_time":"2026-11-10T07:00:00Z","sessions":[{"alias":"S1","launch":{"auth_lane":"api","model":"openai/gpt-6-astra","parent":"root"},"lifecycle":[{"at":"2026-11-10T06:00:10Z","event":"launch","turn":1},{"at":"2026-11-10T06:50:00Z","event":"retire"}],"logs":{"turn-1":{"final_newline":true,"lines":["{\"at\": \"2026-11-10T06:30:00Z\", \"event\": \"usd_reserved\", \"n\": 1, \"rid\": 1, \"reserved_usd\": 0.5, \"input_bound\": 6000, \"bound_basis\": \"bytes\"}","{\"at\": \"2026-11-10T06:30:02Z\", \"event\": \"request\", \"n\": 1, \"http_status\": 200, \"model\": \"gpt-6-astra\", \"requested_tier\": null, \"observed_tier\": \"default\", \"terminal\": \"response.completed\", \"response_id\": \"resp_1\", \"response_model\": \"gpt-6-astra\", \"usage\": {\"input_tokens\": 1000, \"output_tokens\": 100, \"cached_tokens\": 0, \"cache_write_tokens\": 0, \"reasoning_tokens\": 0}, \"seconds\": 1.0, \"usd\": {\"rid\": 1, \"reserved_usd\": 0.5, \"charged_usd\": 0.015, \"charge_basis\": \"usage\", \"session_spent_usd\": 0.015, \"multiplier_floor\": 0.0}}","{\"at\": \"2026-11-10T06:30:05Z\", \"event\": \"request\", \"n\": 1, \"http_status\": 200, \"model\": \"gpt-6-astra\", \"requested_tier\": null, \"observed_tier\": \"default\", \"terminal\": \"response.completed\", \"response_id\": \"resp_1\", \"response_model\": \"gpt-6-astra\", \"usage\": {\"input_tokens\": 1000, \"output_tokens\": 100, \"cached_tokens\": \"400\", \"cache_write_tokens\": 0, \"reasoning_tokens\": 0}, \"seconds\": 1.0, \"usd\": {\"rid\": 1, \"reserved_usd\": 0.5, \"charged_usd\": 0.015, \"charge_basis\": \"usage\", \"session_spent_usd\": 0.015, \"multiplier_floor\": 0.0}}"]}},"session_id":"00000000-0000-4000-8000-0000000000c1"}]},"invariants":["INV-F3","INV-F5"],"profile":"current_producer","provenance":{"origin":"contract::targeted case (from VC01-openai-usage-settlement)"},"reachability":"producer_unreachable","role":"finding","schema":"pi-accounting-corpus/1"}
   47| {"added_in":"1.4-s1","ambiguity":null,"case_id":"P133-A05-model-case-variant","class":"adversarial_synthetic","contract":"pi-accounting-contract/1.4-s1","controls":["VC01-openai-usage-settlement"],"evaluations":[{"admission_request":{"new":"100000000","objective":"o"},"expected":{"admission":{"decision":"refused","limit":"1000000000","new":"100000000","objective":"o"},"attribution":[{"attempt":"res:S1/turn-1:rid=1","objective":"o","ownership":"owned","placement":"inside"}],"objectives":[{"commitments":[{"admission_id":"a1","committed":"500000000","gate":{"qualified":false,"reasons":["NoGateEvidence"]},"kind":"at_least","state":"retired_resumable"}],"objective":"o","owned_closed":{"high":{"kind":"exact","nano":"0"},"low":"0"},"owned_outstanding":{"high":{"kind":"at_least","nano":"500000000"},"low":"0"},"predecessor_obligations":{"high":{"kind":"exact","nano":"0"},"low":"0"},"provisional":false,"reasons":["IdentityContradiction","SettlementRejected","UnenforcedCommitment","UnresolvedExposure"],"status":"not_established"}]},"name":"default","objectives":[{"admissions":[{"admission_id":"a1","rows":[{"admission_id":"a1","at":"2026-11-10T06:00:05Z","event":"pending","max_usd":1.0},{"admission_id":"a1","at":"2026-11-10T06:00:10Z","event":"launched","session_id":"00000000-0000-4000-8000-0000000000c1"}],"session":"00000000-0000-4000-8000-0000000000c1","source":"journal"}],"closed_at":null,"limit":"1000000000","objective":"o","predecessor":null,"started_at":"2026-11-10T06:00:00Z","tree_sessions":["00000000-0000-4000-8000-0000000000c1"]}]}],"expected":{"attempts":[{"billing":"unresolved","blocking":["IdentityContradiction","SettlementRejected","UnresolvedExposure"],"correlation":{"kind":"exact"},"diagnostics":[],"evidence":["S1/turn-1:1","S1/turn-1:2"],"execution":"terminal","exposure":{"high":{"basis":"proxy_reservation","kind":"at_least","nano":"500000000"},"low":{"basis":"none","nano":"0"},"pool":"pi_api"},"forwarding":"forwarded","key":"res:S1/turn-1:rid=1","obligation":{"amount_source":"reservation_bound","state":"outstanding"},"replacement":{"eligible":false,"reason":"NotUsageSettlement"},"settlement":{"charged":"15000000","decision":"rejected","reasons":["ModelMismatch"],"reserved":"500000000"},"valuation":{"priced_high":"15000000","reasons":["IdentityWithoutRate"],"state":"unpriceable","supported_low":null}}],"inventory":[],"lifecycle":{"S1":{"clock":"trusted","reopened":false,"state":"retired"}},"non_attempt":[],"unreadable":[]},"historical":{"a134_086bdec1":{"blocking_kinds":["pi_settlement_invalid"],"decision":"not_established","evaluation":"default","note":"comparison only; A1.3.4 has no ownership (D1) concept: it attributes by session tree and time window","reader_sha256":"086bdec1d443064f6847821858f34bbf227f8804a9a5fc8a362416f62b29d621","reasons":["objective accounting unavailable, unpriced or inconsistent"],"spent_high_nano":null,"spent_low_nano":null}},"inputs":{"rates":{"anthropic_api_rates.json":"8a286433d9e08b37533f2f908f2db60c4357e1a566018c9ca8364937652bdef5","openai_api_rates.json":"c0bdfd66ee69f1644eaf188f87f6af5e47beacc0739d20aac5846c923d66fd58"},"reference_time":"2026-11-10T07:00:00Z","sessions":[{"alias":"S1","launch":{"auth_lane":"api","model":"openai/gpt-6-astra","parent":"root"},"lifecycle":[{"at":"2026-11-10T06:00:10Z","event":"launch","turn":1},{"at":"2026-11-10T06:50:00Z","event":"retire"}],"logs":{"turn-1":{"final_newline":true,"lines":["{\"at\": \"2026-11-10T06:30:00Z\", \"event\": \"usd_reserved\", \"n\": 1, \"rid\": 1, \"reserved_usd\": 0.5, \"input_bound\": 6000, \"bound_basis\": \"bytes\"}","{\"at\": \"2026-11-10T06:30:02Z\", \"event\": \"request\", \"n\": 1, \"http_status\": 200, \"model\": \"GPT-6-Astra\", \"requested_tier\": null, \"observed_tier\": \"default\", \"terminal\": \"response.completed\", \"response_id\": \"resp_1\", \"response_model\": \"gpt-6-astra\", \"usage\": {\"input_tokens\": 1000, \"output_tokens\": 100, \"cached_tokens\": 0, \"cache_write_tokens\": 0, \"reasoning_tokens\": 0}, \"seconds\": 1.0, \"usd\": {\"rid\": 1, \"reserved_usd\": 0.5, \"charged_usd\": 0.015, \"charge_basis\": \"usage\", \"session_spent_usd\": 0.015, \"multiplier_floor\": 0.0}}"]}},"session_id":"00000000-0000-4000-8000-0000000000c1"}]},"invariants":["INV-F5"],"profile":"current_producer","provenance":{"origin":"contract::targeted case (from VC01-openai-usage-settlement)"},"reachability":"not_established","role":"finding","schema":"pi-accounting-corpus/1"}
   48| {"added_in":"1.4-s1","ambiguity":null,"case_id":"P133-A06-usage-settlement-status-absent","class":"adversarial_synthetic","contract":"pi-accounting-contract/1.4-s1","controls":["VC01-openai-usage-settlement"],"evaluations":[{"admission_request":{"new":"100000000","objective":"o"},"expected":{"admission":{"decision":"refused","limit":"1000000000","new":"100000000","objective":"o"},"attribution":[{"attempt":"res:S1/turn-1:rid=1","objective":"o","ownership":"owned","placement":"inside"}],"objectives":[{"commitments":[{"admission_id":"a1","committed":"500000000","gate":{"qualified":false,"reasons":["NoGateEvidence"]},"kind":"at_least","state":"retired_resumable"}],"objective":"o","owned_closed":{"high":{"kind":"exact","nano":"0"},"low":"0"},"owned_outstanding":{"high":{"kind":"at_least","nano":"500000000"},"low":"0"},"predecessor_obligations":{"high":{"kind":"exact","nano":"0"},"low":"0"},"provisional":false,"reasons":["SettlementRejected","UnenforcedCommitment","UnresolvedExposure"],"status":"not_established"}]},"name":"default","objectives":[{"admissions":[{"admission_id":"a1","rows":[{"admission_id":"a1","at":"2026-11-10T06:00:05Z","event":"pending","max_usd":1.0},{"admission_id":"a1","at":"2026-11-10T06:00:10Z","event":"launched","session_id":"00000000-0000-4000-8000-0000000000c1"}],"session":"00000000-0000-4000-8000-0000000000c1","source":"journal"}],"closed_at":null,"limit":"1000000000","objective":"o","predecessor":null,"started_at":"2026-11-10T06:00:00Z","tree_sessions":["00000000-0000-4000-8000-0000000000c1"]}]}],"expected":{"attempts":[{"billing":"unresolved","blocking":["SettlementRejected","UnresolvedExposure"],"correlation":{"kind":"exact"},"diagnostics":[],"evidence":["S1/turn-1:1","S1/turn-1:2"],"execution":"terminal","exposure":{"high":{"basis":"proxy_reservation","kind":"at_least","nano":"500000000"},"low":{"basis":"none","nano":"0"},"pool":"pi_api"},"forwarding":"forwarded","key":"res:S1/turn-1:rid=1","obligation":{"amount_source":"reservation_bound","state":"outstanding"},"replacement":{"eligible":false,"reason":"NotUsageSettlement"},"settlement":{"charged":"15000000","decision":"rejected","reasons":["StatusType"],"reserved":"500000000"},"valuation":{"complete":false,"derivations":[{"for":"both","model":"gpt-6-astra","nano":"15000000","provider":"openai","tier_multiplier":"1","usage":{"cache_write_tokens":0,"cached_tokens":0,"input_tokens":1000,"output_tokens":100}}],"high":"15000000","identities":["gpt-6-astra"],"low":"15000000","state":"valued"}}],"inventory":[],"lifecycle":{"S1":{"clock":"trusted","reopened":false,"state":"retired"}},"non_attempt":[],"unreadable":[]},"historical":{"a134_086bdec1":{"blocking_kinds":[],"decision":"admitted","evaluation":"default","note":"comparison only; A1.3.4 has no ownership (D1) concept: it attributes by session tree and time window","reader_sha256":"086bdec1d443064f6847821858f34bbf227f8804a9a5fc8a362416f62b29d621","reasons":["within the objective pi_api limit"],"spent_high_nano":"15000000","spent_low_nano":"15000000"}},"inputs":{"rates":{"anthropic_api_rates.json":"8a286433d9e08b37533f2f908f2db60c4357e1a566018c9ca8364937652bdef5","openai_api_rates.json":"c0bdfd66ee69f1644eaf188f87f6af5e47beacc0739d20aac5846c923d66fd58"},"reference_time":"2026-11-10T07:00:00Z","sessions":[{"alias":"S1","launch":{"auth_lane":"api","model":"openai/gpt-6-astra","parent":"root"},"lifecycle":[{"at":"2026-11-10T06:00:10Z","event":"launch","turn":1},{"at":"2026-11-10T06:50:00Z","event":"retire"}],"logs":{"turn-1":{"final_newline":true,"lines":["{\"at\": \"2026-11-10T06:30:00Z\", \"event\": \"usd_reserved\", \"n\": 1, \"rid\": 1, \"reserved_usd\": 0.5, \"input_bound\": 6000, \"bound_basis\": \"bytes\"}","{\"at\": \"2026-11-10T06:30:02Z\", \"event\": \"request\", \"n\": 1, \"model\": \"gpt-6-astra\", \"requested_tier\": null, \"observed_tier\": \"default\", \"terminal\": \"response.completed\", \"response_id\": \"resp_1\", \"response_model\": \"gpt-6-astra\", \"usage\": {\"input_tokens\": 1000, \"output_tokens\": 100, \"cached_tokens\": 0, \"cache_write_tokens\": 0, \"reasoning_tokens\": 0}, \"seconds\": 1.0, \"usd\": {\"rid\": 1, \"reserved_usd\": 0.5, \"charged_usd\": 0.015, \"charge_basis\": \"usage\", \"session_spent_usd\": 0.015, \"multiplier_floor\": 0.0}}"]}},"session_id":"00000000-0000-4000-8000-0000000000c1"}]},"invariants":["INV-F1"],"profile":"current_producer","provenance":{"origin":"contract::targeted case (from VC01-openai-usage-settlement)"},"reachability":"producer_unreachable","role":"finding","schema":"pi-accounting-corpus/1"}
   49| {"added_in":"1.4-s1","ambiguity":null,"case_id":"P133-A06-error-status-null","class":"adversarial_synthetic","contract":"pi-accounting-contract/1.4-s1","controls":["S02-control-error-status-after-reservation"],"evaluations":[{"admission_request":{"new":"100000000","objective":"o"},"expected":{"admission":{"decision":"refused","limit":"1000000000","new":"100000000","objective":"o"},"attribution":[{"attempt":"res:S1/turn-1:rid=1","objective":"o","ownership":"owned","placement":"inside"}],"objectives":[{"commitments":[{"admission_id":"a1","committed":"200000000","gate":{"qualified":false,"reasons":["NoGateEvidence"]},"kind":"at_least","state":"retired_resumable"}],"objective":"o","owned_closed":{"high":{"kind":"exact","nano":"0"},"low":"0"},"owned_outstanding":{"high":{"kind":"at_least","nano":"800000000"},"low":"0"},"predecessor_obligations":{"high":{"kind":"exact","nano":"0"},"low":"0"},"provisional":false,"reasons":["SettlementRejected","UnenforcedCommitment","UnresolvedExposure"],"status":"not_established"}]},"name":"default","objectives":[{"admissions":[{"admission_id":"a1","rows":[{"admission_id":"a1","at":"2026-11-10T06:00:05Z","event":"pending","max_usd":1.0},{"admission_id":"a1","at":"2026-11-10T06:00:10Z","event":"launched","session_id":"00000000-0000-4000-8000-0000000000c1"}],"session":"00000000-0000-4000-8000-0000000000c1","source":"journal"}],"closed_at":null,"limit":"1000000000","objective":"o","predecessor":null,"started_at":"2026-11-10T06:00:00Z","tree_sessions":["00000000-0000-4000-8000-0000000000c1"]}]}],"expected":{"attempts":[{"billing":"unresolved","blocking":["SettlementRejected","UnresolvedExposure"],"correlation":{"kind":"exact"},"diagnostics":[],"evidence":["S1/turn-1:1","S1/turn-1:2"],"execution":"terminal","exposure":{"high":{"basis":"proxy_reservation","kind":"at_least","nano":"800000000"},"low":{"basis":"none","nano":"0"},"pool":"pi_api"},"forwarding":"forwarded","key":"res:S1/turn-1:rid=1","obligation":{"amount_source":"reservation_bound","state":"outstanding"},"replacement":{"eligible":false,"reason":"NotUsageSettlement"},"settlement":{"charged":"0","decision":"rejected","reasons":["StatusType"],"reserved":"800000000"},"valuation":{"state":"not_reported"}}],"inventory":[],"lifecycle":{"S1":{"clock":"trusted","reopened":false,"state":"retired"}},"non_attempt":[],"unreadable":[]},"historical":{"a134_086bdec1":{"blocking_kinds":["pi_settlement_invalid"],"decision":"not_established","evaluation":"default","note":"comparison only; A1.3.4 has no ownership (D1) concept: it attributes by session tree and time window","reader_sha256":"086bdec1d443064f6847821858f34bbf227f8804a9a5fc8a362416f62b29d621","reasons":["objective accounting unavailable, unpriced or inconsistent"],"spent_high_nano":null,"spent_low_nano":null}},"inputs":{"rates":{"anthropic_api_rates.json":"8a286433d9e08b37533f2f908f2db60c4357e1a566018c9ca8364937652bdef5","openai_api_rates.json":"c0bdfd66ee69f1644eaf188f87f6af5e47beacc0739d20aac5846c923d66fd58"},"reference_time":"2026-11-10T07:00:00Z","sessions":[{"alias":"S1","launch":{"auth_lane":"api","model":"openai/gpt-6-astra","parent":"root"},"lifecycle":[{"at":"2026-11-10T06:00:10Z","event":"launch","turn":1},{"at":"2026-11-10T06:50:00Z","event":"retire"}],"logs":{"turn-1":{"final_newline":true,"lines":["{\"at\": \"2026-11-10T06:30:00Z\", \"event\": \"usd_reserved\", \"n\": 1, \"rid\": 1, \"reserved_usd\": 0.8, \"input_bound\": 6000, \"bound_basis\": \"bytes\"}","{\"at\": \"2026-11-10T06:30:02Z\", \"event\": \"request\", \"n\": 1, \"http_status\": null, \"model\": \"gpt-6-astra\", \"requested_tier\": null, \"observed_tier\": \"default\", \"terminal\": null, \"response_id\": null, \"response_model\": \"gpt-6-astra\", \"usage\": {\"input_tokens\": null, \"output_tokens\": null, \"cached_tokens\": null, \"cache_write_tokens\": null, \"reasoning_tokens\": 0}, \"seconds\": 1.0, \"usd\": {\"rid\": 1, \"reserved_usd\": 0.8, \"charged_usd\": 0.0, \"charge_basis\": \"error_status\", \"session_spent_usd\": 0.0, \"multiplier_floor\": 0.0}}"]}},"session_id":"00000000-0000-4000-8000-0000000000c1"}]},"invariants":["INV-F1"],"profile":"current_producer","provenance":{"origin":"contract::targeted case (from S02-control-error-status-after-reservation)"},"reachability":"producer_unreachable","role":"finding","schema":"pi-accounting-corpus/1"}
   50| {"added_in":"1.4-s1","ambiguity":null,"case_id":"P133-A09-conflicting-reservations-with-settlement","class":"adversarial_synthetic","contract":"pi-accounting-contract/1.4-s1","controls":["VC01-openai-usage-settlement","R04-reused-reservation-id","C3-identical-reservation-copy"],"evaluations":[{"admission_request":{"new":"100000000","objective":"o"},"expected":{"admission":{"decision":"refused","limit":"1000000000","new":"100000000","objective":"o"},"attribution":[{"attempt":"res:S1/turn-1:rid=1","objective":"o","ownership":"owned","placement":"inside"}],"objectives":[{"commitments":[{"admission_id":"a1","committed":"500000000","gate":{"qualified":false,"reasons":["NoGateEvidence"]},"kind":"at_least","state":"retired_resumable"}],"objective":"o","owned_closed":{"high":{"kind":"exact","nano":"0"},"low":"0"},"owned_outstanding":{"high":{"kind":"at_least","nano":"500000000"},"low":"0"},"predecessor_obligations":{"high":{"kind":"exact","nano":"0"},"low":"0"},"provisional":false,"reasons":["ReservationConflict","SettlementRejected","UnenforcedCommitment","UnresolvedExposure"],"status":"not_established"}]},"name":"default","objectives":[{"admissions":[{"admission_id":"a1","rows":[{"admission_id":"a1","at":"2026-11-10T06:00:05Z","event":"pending","max_usd":1.0},{"admission_id":"a1","at":"2026-11-10T06:00:10Z","event":"launched","session_id":"00000000-0000-4000-8000-0000000000c1"}],"session":"00000000-0000-4000-8000-0000000000c1","source":"journal"}],"closed_at":null,"limit":"1000000000","objective":"o","predecessor":null,"started_at":"2026-11-10T06:00:00Z","tree_sessions":["00000000-0000-4000-8000-0000000000c1"]}]}],"expected":{"attempts":[{"billing":"unresolved","blocking":["ReservationConflict","SettlementRejected","UnresolvedExposure"],"correlation":{"kind":"exact"},"diagnostics":[],"evidence":["S1/turn-1:1","S1/turn-1:2","S1/turn-1:3"],"execution":"terminal","exposure":{"high":{"basis":"proxy_reservation","kind":"at_least","nano":"500000000"},"low":{"basis":"none","nano":"0"},"pool":"pi_api"},"forwarding":"forwarded","key":"res:S1/turn-1:rid=1","obligation":{"amount_source":"reservation_bound","state":"outstanding"},"replacement":{"eligible":false,"reason":"NotUsageSettlement"},"settlement":{"charged":"15000000","decision":"rejected","reasons":["ReservationConflict"],"reserved":"500000000"},"valuation":{"complete":true,"derivations":[{"for":"both","model":"gpt-6-astra","nano":"15000000","provider":"openai","tier_multiplier":"1","usage":{"cache_write_tokens":0,"cached_tokens":0,"input_tokens":1000,"output_tokens":100}}],"high":"15000000","identities":["gpt-6-astra"],"low":"15000000","state":"valued"}}],"inventory":[],"lifecycle":{"S1":{"clock":"trusted","reopened":false,"state":"retired"}},"non_attempt":[],"unreadable":[]},"historical":{"a134_086bdec1":{"blocking_kinds":["pi_reservation_conflict"],"decision":"not_established","evaluation":"default","note":"comparison only; A1.3.4 has no ownership (D1) concept: it attributes by session tree and time window","reader_sha256":"086bdec1d443064f6847821858f34bbf227f8804a9a5fc8a362416f62b29d621","reasons":["objective accounting unavailable, unpriced or inconsistent"],"spent_high_nano":null,"spent_low_nano":null}},"inputs":{"rates":{"anthropic_api_rates.json":"8a286433d9e08b37533f2f908f2db60c4357e1a566018c9ca8364937652bdef5","openai_api_rates.json":"c0bdfd66ee69f1644eaf188f87f6af5e47beacc0739d20aac5846c923d66fd58"},"reference_time":"2026-11-10T07:00:00Z","sessions":[{"alias":"S1","launch":{"auth_lane":"api","model":"openai/gpt-6-astra","parent":"root"},"lifecycle":[{"at":"2026-11-10T06:00:10Z","event":"launch","turn":1},{"at":"2026-11-10T06:50:00Z","event":"retire"}],"logs":{"turn-1":{"final_newline":true,"lines":["{\"at\": \"2026-11-10T06:30:00Z\", \"event\": \"usd_reserved\", \"n\": 1, \"rid\": 1, \"reserved_usd\": 0.5, \"input_bound\": 6000, \"bound_basis\": \"bytes\"}","{\"at\": \"2026-11-10T06:30:01Z\", \"event\": \"usd_reserved\", \"n\": 1, \"rid\": 1, \"reserved_usd\": 0.3, \"input_bound\": 6000, \"bound_basis\": \"bytes\"}","{\"at\": \"2026-11-10T06:30:02Z\", \"event\": \"request\", \"n\": 1, \"http_status\": 200, \"model\": \"gpt-6-astra\", \"requested_tier\": null, \"observed_tier\": \"default\", \"terminal\": \"response.completed\", \"response_id\": \"resp_1\", \"response_model\": \"gpt-6-astra\", \"usage\": {\"input_tokens\": 1000, \"output_tokens\": 100, \"cached_tokens\": 0, \"cache_write_tokens\": 0, \"reasoning_tokens\": 0}, \"seconds\": 1.0, \"usd\": {\"rid\": 1, \"reserved_usd\": 0.5, \"charged_usd\": 0.015, \"charge_basis\": \"usage\", \"session_spent_usd\": 0.015, \"multiplier_floor\": 0.0}}"]}},"session_id":"00000000-0000-4000-8000-0000000000c1"}]},"invariants":["INV-F2"],"profile":"current_producer","provenance":{"origin":"contract::targeted case (from VC01-openai-usage-settlement)"},"reachability":"producer_unreachable","role":"finding","schema":"pi-accounting-corpus/1"}
   51| {"added_in":"1.4-s1","ambiguity":null,"case_id":"P133-FG-VC14-authenticated-bound","class":"future_vertical","contract":"pi-accounting-contract/1.4-s1","controls":[],"evaluations":[{"admission_request":{"new":"100000000","objective":"o"},"expected":{"admission":{"decision":"admitted","limit":"1000000000","new":"100000000","objective":"o"},"attribution":[{"attempt":"res:S1/turn-1:rid=1","objective":"o","ownership":"owned","placement":"inside"}],"objectives":[{"commitments":[{"admission_id":"a1","committed":"485000000","gate":{"qualified":true,"reasons":[]},"kind":"exact","state":"launched_open"}],"objective":"o","owned_closed":{"high":{"kind":"exact","nano":"15000000"},"low":"15000000"},"owned_outstanding":{"high":{"kind":"exact","nano":"0"},"low":"0"},"predecessor_obligations":{"high":{"kind":"exact","nano":"0"},"low":"0"},"provisional":false,"reasons":[],"status":"established"}]},"name":"limit-1.0","objectives":[{"admission_authority":{"id":"wev-admission-authority-1","kind":"vertical_admission_authority"},"admissions":[{"admission_id":"a1","gate_id":"g-S1","rows":[{"admission_id":"a1","at":"2026-11-10T06:00:05Z","event":"pending","max_usd":0.5},{"admission_id":"a1","at":"2026-11-10T06:00:10Z","event":"launched","session_id":"00000000-0000-4000-8000-0000000000c1"}],"session":"00000000-0000-4000-8000-0000000000c1","source":"journal"}],"closed_at":null,"limit":"1000000000","objective":"o","predecessor":null,"started_at":"2026-11-10T06:00:00Z","tree_sessions":["00000000-0000-4000-8000-0000000000c1"]}]}],"expected":{"attempts":[{"billing":"established","blocking":[],"correlation":{"kind":"exact"},"diagnostics":[],"evidence":["S1/turn-1:1","S1/turn-1:2"],"execution":"terminal","exposure":{"high":{"basis":"usage_valuation","kind":"exact","nano":"15000000"},"low":{"basis":"usage_valuation","nano":"15000000"},"pool":"pi_api"},"forwarding":"forwarded","key":"res:S1/turn-1:rid=1","obligation":{"amount_source":"settlement_amount","state":"closed"},"replacement":{"eligible":true,"reason":"UsageBound"},"settlement":{"basis":"usage","charged":"15000000","decision":"valid","reasons":[],"reserved":"500000000"},"valuation":{"complete":true,"derivations":[{"for":"both","model":"gpt-6-astra","nano":"15000000","provider":"openai","tier_multiplier":"1","usage":{"cache_write_tokens":0,"cached_tokens":0,"input_tokens":1000,"output_tokens":100}}],"high":"15000000","identities":["gpt-6-astra"],"low":"15000000","state":"valued"}}],"inventory":[],"lifecycle":{"S1":{"clock":"trusted","reopened":false,"state":"open"}},"non_attempt":[],"unreadable":[]},"historical":{},"inputs":{"gates":[{"authority":{"id":"wev-admission-authority-1","kind":"vertical_admission_authority"},"effective_from":"2026-11-10T06:00:05Z","effective_until":null,"gate_id":"g-S1","mechanisms":{"billing_bound":{"bound":{"amount_nano":"500000000","binding_sha256":"68dda7d580ea94b0c65214f1938d8a3fe07683bf56bac5a8f11cf0255c0160f3","covers":["failures","forwarding","partial_streams","retries","settlement"],"currency":"USD","scope":"session"},"enforced":true,"receipt":{"ref":"authority-receipt:S1-billing_bound","sha256":"eb21af7cd5c8a234faedb9ec0f77080cc75bf11904a5ca6a3744849f12fbac0a"}},"new_forwarding":{"enforced":true,"receipt":{"ref":"authority-receipt:S1-new_forwarding","sha256":"48627919de37a2276f5dcb1de5efce5082508a99f2a0479b1c2923e093152f69"}},"reopening":{"enforced":true,"receipt":{"ref":"authority-receipt:S1-reopening","sha256":"6a59c035f1ded753aa69c6cee1f5626faf2a2bf2d93366d378a07bf12c4cdb08"}},"retries":{"enforced":true,"receipt":{"ref":"authority-receipt:S1-retries","sha256":"ad74cf4e263dd56b37d9cef747ce3273809e06de4eefc5f3050c919c71cb2d39"}}},"scope":{"kind":"session"},"session":"00000000-0000-4000-8000-0000000000c1","writer":"authority"}],"rates":{"anthropic_api_rates.json":"8a286433d9e08b37533f2f908f2db60c4357e1a566018c9ca8364937652bdef5","openai_api_rates.json":"c0bdfd66ee69f1644eaf188f87f6af5e47beacc0739d20aac5846c923d66fd58"},"reference_time":"2026-11-10T07:00:00Z","sessions":[{"alias":"S1","launch":{"auth_lane":"api","model":"openai/gpt-6-astra","parent":"root"},"lifecycle":[{"at":"2026-11-10T06:00:10Z","event":"launch","turn":1}],"logs":{"turn-1":{"final_newline":true,"lines":["{\"at\": \"2026-11-10T06:30:00Z\", \"event\": \"usd_reserved\", \"n\": 1, \"rid\": 1, \"reserved_usd\": 0.5, \"input_bound\": 6000, \"bound_basis\": \"bytes\"}","{\"at\": \"2026-11-10T06:30:02Z\", \"event\": \"request\", \"n\": 1, \"http_status\": 200, \"model\": \"gpt-6-astra\", \"requested_tier\": null, \"observed_tier\": \"default\", \"terminal\": \"response.completed\", \"response_id\": \"resp_1\", \"response_model\": \"gpt-6-astra\", \"usage\": {\"input_tokens\": 1000, \"output_tokens\": 100, \"cached_tokens\": 0, \"cache_write_tokens\": 0, \"reasoning_tokens\": 0}, \"seconds\": 1.0, \"usd\": {\"rid\": 1, \"reserved_usd\": 0.5, \"charged_usd\": 0.015, \"charge_basis\": \"usage\", \"session_spent_usd\": 0.015, \"multiplier_floor\": 0.0}}"]}},"session_id":"00000000-0000-4000-8000-0000000000c1"}]},"invariants":["INV-A9","INV-A8"],"profile":"future_qualified_gate","provenance":{"origin":"contract::targeted case (from P132-FG-VC14-open-session-billing-bound)"},"reachability":"producer_unreachable","role":"valid_control","schema":"pi-accounting-corpus/1"}
   52| {"added_in":"1.4-s1","ambiguity":null,"case_id":"P133-FG-bound-exceeds-cap","class":"future_vertical","contract":"pi-accounting-contract/1.4-s1","controls":["P133-FG-VC14-authenticated-bound"],"evaluations":[{"admission_request":{"new":"100000000","objective":"o"},"expected":{"admission":{"decision":"not_established","limit":"1000000000","new":"100000000","objective":"o"},"attribution":[{"attempt":"res:S1/turn-1:rid=1","objective":"o","ownership":"owned","placement":"inside"}],"objectives":[{"commitments":[{"admission_id":"a1","committed":"485000000","gate":{"qualified":false,"reasons":["GateBoundExceedsCap"]},"kind":"at_least","state":"launched_open"}],"objective":"o","owned_closed":{"high":{"kind":"exact","nano":"15000000"},"low":"15000000"},"owned_outstanding":{"high":{"kind":"exact","nano":"0"},"low":"0"},"predecessor_obligations":{"high":{"kind":"exact","nano":"0"},"low":"0"},"provisional":false,"reasons":["UnenforcedCommitment"],"status":"not_established"}]},"name":"limit-1.0","objectives":[{"admission_authority":{"id":"wev-admission-authority-1","kind":"vertical_admission_authority"},"admissions":[{"admission_id":"a1","gate_id":"g-S1","rows":[{"admission_id":"a1","at":"2026-11-10T06:00:05Z","event":"pending","max_usd":0.5},{"admission_id":"a1","at":"2026-11-10T06:00:10Z","event":"launched","session_id":"00000000-0000-4000-8000-0000000000c1"}],"session":"00000000-0000-4000-8000-0000000000c1","source":"journal"}],"closed_at":null,"limit":"1000000000","objective":"o","predecessor":null,"started_at":"2026-11-10T06:00:00Z","tree_sessions":["00000000-0000-4000-8000-0000000000c1"]}]}],"expected":{"attempts":[{"billing":"established","blocking":[],"correlation":{"kind":"exact"},"diagnostics":[],"evidence":["S1/turn-1:1","S1/turn-1:2"],"execution":"terminal","exposure":{"high":{"basis":"usage_valuation","kind":"exact","nano":"15000000"},"low":{"basis":"usage_valuation","nano":"15000000"},"pool":"pi_api"},"forwarding":"forwarded","key":"res:S1/turn-1:rid=1","obligation":{"amount_source":"settlement_amount","state":"closed"},"replacement":{"eligible":true,"reason":"UsageBound"},"settlement":{"basis":"usage","charged":"15000000","decision":"valid","reasons":[],"reserved":"500000000"},"valuation":{"complete":true,"derivations":[{"for":"both","model":"gpt-6-astra","nano":"15000000","provider":"openai","tier_multiplier":"1","usage":{"cache_write_tokens":0,"cached_tokens":0,"input_tokens":1000,"output_tokens":100}}],"high":"15000000","identities":["gpt-6-astra"],"low":"15000000","state":"valued"}}],"inventory":[],"lifecycle":{"S1":{"clock":"trusted","reopened":false,"state":"open"}},"non_attempt":[],"unreadable":[]},"historical":{},"inputs":{"gates":[{"authority":{"id":"wev-admission-authority-1","kind":"vertical_admission_authority"},"effective_from":"2026-11-10T06:00:05Z","effective_until":null,"gate_id":"g-S1","mechanisms":{"billing_bound":{"bound":{"amount_nano":"600000000","binding_sha256":"beffd21a002785f1800fe8a4992cd6c6a27bcdca81aa9e43b71efa6bdb64fc63","covers":["failures","forwarding","partial_streams","retries","settlement"],"currency":"USD","scope":"session"},"enforced":true,"receipt":{"ref":"authority-receipt:S1-billing_bound","sha256":"eb21af7cd5c8a234faedb9ec0f77080cc75bf11904a5ca6a3744849f12fbac0a"}},"new_forwarding":{"enforced":true,"receipt":{"ref":"authority-receipt:S1-new_forwarding","sha256":"48627919de37a2276f5dcb1de5efce5082508a99f2a0479b1c2923e093152f69"}},"reopening":{"enforced":true,"receipt":{"ref":"authority-receipt:S1-reopening","sha256":"6a59c035f1ded753aa69c6cee1f5626faf2a2bf2d93366d378a07bf12c4cdb08"}},"retries":{"enforced":true,"receipt":{"ref":"authority-receipt:S1-retries","sha256":"ad74cf4e263dd56b37d9cef747ce3273809e06de4eefc5f3050c919c71cb2d39"}}},"scope":{"kind":"session"},"session":"00000000-0000-4000-8000-0000000000c1","writer":"authority"}],"rates":{"anthropic_api_rates.json":"8a286433d9e08b37533f2f908f2db60c4357e1a566018c9ca8364937652bdef5","openai_api_rates.json":"c0bdfd66ee69f1644eaf188f87f6af5e47beacc0739d20aac5846c923d66fd58"},"reference_time":"2026-11-10T07:00:00Z","sessions":[{"alias":"S1","launch":{"auth_lane":"api","model":"openai/gpt-6-astra","parent":"root"},"lifecycle":[{"at":"2026-11-10T06:00:10Z","event":"launch","turn":1}],"logs":{"turn-1":{"final_newline":true,"lines":["{\"at\": \"2026-11-10T06:30:00Z\", \"event\": \"usd_reserved\", \"n\": 1, \"rid\": 1, \"reserved_usd\": 0.5, \"input_bound\": 6000, \"bound_basis\": \"bytes\"}","{\"at\": \"2026-11-10T06:30:02Z\", \"event\": \"request\", \"n\": 1, \"http_status\": 200, \"model\": \"gpt-6-astra\", \"requested_tier\": null, \"observed_tier\": \"default\", \"terminal\": \"response.completed\", \"response_id\": \"resp_1\", \"response_model\": \"gpt-6-astra\", \"usage\": {\"input_tokens\": 1000, \"output_tokens\": 100, \"cached_tokens\": 0, \"cache_write_tokens\": 0, \"reasoning_tokens\": 0}, \"seconds\": 1.0, \"usd\": {\"rid\": 1, \"reserved_usd\": 0.5, \"charged_usd\": 0.015, \"charge_basis\": \"usage\", \"session_spent_usd\": 0.015, \"multiplier_floor\": 0.0}}"]}},"session_id":"00000000-0000-4000-8000-0000000000c1"}]},"invariants":["INV-A9","INV-A8"],"profile":"future_qualified_gate","provenance":{"origin":"contract::targeted case (from P132-FG-VC14-open-session-billing-bound)"},"reachability":"producer_unreachable","role":"finding","schema":"pi-accounting-corpus/1"}
   53| {"added_in":"1.4-s1","ambiguity":null,"case_id":"P133-FG-bound-binding-mismatch","class":"future_vertical","contract":"pi-accounting-contract/1.4-s1","controls":["P133-FG-VC14-authenticated-bound"],"evaluations":[{"admission_request":{"new":"100000000","objective":"o"},"expected":{"admission":{"decision":"not_established","limit":"1000000000","new":"100000000","objective":"o"},"attribution":[{"attempt":"res:S1/turn-1:rid=1","objective":"o","ownership":"owned","placement":"inside"}],"objectives":[{"commitments":[{"admission_id":"a1","committed":"485000000","gate":{"qualified":false,"reasons":["GateBoundBinding"]},"kind":"at_least","state":"launched_open"}],"objective":"o","owned_closed":{"high":{"kind":"exact","nano":"15000000"},"low":"15000000"},"owned_outstanding":{"high":{"kind":"exact","nano":"0"},"low":"0"},"predecessor_obligations":{"high":{"kind":"exact","nano":"0"},"low":"0"},"provisional":false,"reasons":["UnenforcedCommitment"],"status":"not_established"}]},"name":"limit-1.0","objectives":[{"admission_authority":{"id":"wev-admission-authority-1","kind":"vertical_admission_authority"},"admissions":[{"admission_id":"a1","gate_id":"g-S1","rows":[{"admission_id":"a1","at":"2026-11-10T06:00:05Z","event":"pending","max_usd":0.5},{"admission_id":"a1","at":"2026-11-10T06:00:10Z","event":"launched","session_id":"00000000-0000-4000-8000-0000000000c1"}],"session":"00000000-0000-4000-8000-0000000000c1","source":"journal"}],"closed_at":null,"limit":"1000000000","objective":"o","predecessor":null,"started_at":"2026-11-10T06:00:00Z","tree_sessions":["00000000-0000-4000-8000-0000000000c1"]}]}],"expected":{"attempts":[{"billing":"established","blocking":[],"correlation":{"kind":"exact"},"diagnostics":[],"evidence":["S1/turn-1:1","S1/turn-1:2"],"execution":"terminal","exposure":{"high":{"basis":"usage_valuation","kind":"exact","nano":"15000000"},"low":{"basis":"usage_valuation","nano":"15000000"},"pool":"pi_api"},"forwarding":"forwarded","key":"res:S1/turn-1:rid=1","obligation":{"amount_source":"settlement_amount","state":"closed"},"replacement":{"eligible":true,"reason":"UsageBound"},"settlement":{"basis":"usage","charged":"15000000","decision":"valid","reasons":[],"reserved":"500000000"},"valuation":{"complete":true,"derivations":[{"for":"both","model":"gpt-6-astra","nano":"15000000","provider":"openai","tier_multiplier":"1","usage":{"cache_write_tokens":0,"cached_tokens":0,"input_tokens":1000,"output_tokens":100}}],"high":"15000000","identities":["gpt-6-astra"],"low":"15000000","state":"valued"}}],"inventory":[],"lifecycle":{"S1":{"clock":"trusted","reopened":false,"state":"open"}},"non_attempt":[],"unreadable":[]},"historical":{},"inputs":{"gates":[{"authority":{"id":"wev-admission-authority-1","kind":"vertical_admission_authority"},"effective_from":"2026-11-10T06:00:05Z","effective_until":null,"gate_id":"g-S1","mechanisms":{"billing_bound":{"bound":{"amount_nano":"499999000","binding_sha256":"68dda7d580ea94b0c65214f1938d8a3fe07683bf56bac5a8f11cf0255c0160f3","covers":["failures","forwarding","partial_streams","retries","settlement"],"currency":"USD","scope":"session"},"enforced":true,"receipt":{"ref":"authority-receipt:S1-billing_bound","sha256":"eb21af7cd5c8a234faedb9ec0f77080cc75bf11904a5ca6a3744849f12fbac0a"}},"new_forwarding":{"enforced":true,"receipt":{"ref":"authority-receipt:S1-new_forwarding","sha256":"48627919de37a2276f5dcb1de5efce5082508a99f2a0479b1c2923e093152f69"}},"reopening":{"enforced":true,"receipt":{"ref":"authority-receipt:S1-reopening","sha256":"6a59c035f1ded753aa69c6cee1f5626faf2a2bf2d93366d378a07bf12c4cdb08"}},"retries":{"enforced":true,"receipt":{"ref":"authority-receipt:S1-retries","sha256":"ad74cf4e263dd56b37d9cef747ce3273809e06de4eefc5f3050c919c71cb2d39"}}},"scope":{"kind":"session"},"session":"00000000-0000-4000-8000-0000000000c1","writer":"authority"}],"rates":{"anthropic_api_rates.json":"8a286433d9e08b37533f2f908f2db60c4357e1a566018c9ca8364937652bdef5","openai_api_rates.json":"c0bdfd66ee69f1644eaf188f87f6af5e47beacc0739d20aac5846c923d66fd58"},"reference_time":"2026-11-10T07:00:00Z","sessions":[{"alias":"S1","launch":{"auth_lane":"api","model":"openai/gpt-6-astra","parent":"root"},"lifecycle":[{"at":"2026-11-10T06:00:10Z","event":"launch","turn":1}],"logs":{"turn-1":{"final_newline":true,"lines":["{\"at\": \"2026-11-10T06:30:00Z\", \"event\": \"usd_reserved\", \"n\": 1, \"rid\": 1, \"reserved_usd\": 0.5, \"input_bound\": 6000, \"bound_basis\": \"bytes\"}","{\"at\": \"2026-11-10T06:30:02Z\", \"event\": \"request\", \"n\": 1, \"http_status\": 200, \"model\": \"gpt-6-astra\", \"requested_tier\": null, \"observed_tier\": \"default\", \"terminal\": \"response.completed\", \"response_id\": \"resp_1\", \"response_model\": \"gpt-6-astra\", \"usage\": {\"input_tokens\": 1000, \"output_tokens\": 100, \"cached_tokens\": 0, \"cache_write_tokens\": 0, \"reasoning_tokens\": 0}, \"seconds\": 1.0, \"usd\": {\"rid\": 1, \"reserved_usd\": 0.5, \"charged_usd\": 0.015, \"charge_basis\": \"usage\", \"session_spent_usd\": 0.015, \"multiplier_floor\": 0.0}}"]}},"session_id":"00000000-0000-4000-8000-0000000000c1"}]},"invariants":["INV-A9","INV-A8"],"profile":"future_qualified_gate","provenance":{"origin":"contract::targeted case (from P132-FG-VC14-open-session-billing-bound)"},"reachability":"producer_unreachable","role":"finding","schema":"pi-accounting-corpus/1"}
   54| {"added_in":"1.3-s1","ambiguity":null,"case_id":"P132-FG-VC14-open-session-billing-bound","class":"future_vertical","contract":"pi-accounting-contract/1.4-s1","controls":["P133-FG-VC14-authenticated-bound"],"evaluations":[{"admission_request":{"new":"100000000","objective":"o"},"expected":{"admission":{"decision":"not_established","limit":"1000000000","new":"100000000","objective":"o"},"attribution":[{"attempt":"res:S1/turn-1:rid=1","objective":"o","ownership":"owned","placement":"inside"}],"objectives":[{"commitments":[{"admission_id":"a1","committed":"485000000","gate":{"qualified":false,"reasons":["GateBoundMissing"]},"kind":"at_least","state":"launched_open"}],"objective":"o","owned_closed":{"high":{"kind":"exact","nano":"15000000"},"low":"15000000"},"owned_outstanding":{"high":{"kind":"exact","nano":"0"},"low":"0"},"predecessor_obligations":{"high":{"kind":"exact","nano":"0"},"low":"0"},"provisional":false,"reasons":["UnenforcedCommitment"],"status":"not_established"}]},"name":"limit-1.0","objectives":[{"admission_authority":{"id":"wev-admission-authority-1","kind":"vertical_admission_authority"},"admissions":[{"admission_id":"a1","gate_id":"g-S1","rows":[{"admission_id":"a1","at":"2026-11-10T06:00:05Z","event":"pending","max_usd":0.5},{"admission_id":"a1","at":"2026-11-10T06:00:10Z","event":"launched","session_id":"00000000-0000-4000-8000-0000000000c1"}],"session":"00000000-0000-4000-8000-0000000000c1","source":"journal"}],"closed_at":null,"limit":"1000000000","objective":"o","predecessor":null,"started_at":"2026-11-10T06:00:00Z","tree_sessions":["00000000-0000-4000-8000-0000000000c1"]}]}],"expected":{"attempts":[{"billing":"established","blocking":[],"correlation":{"kind":"exact"},"diagnostics":[],"evidence":["S1/turn-1:1","S1/turn-1:2"],"execution":"terminal","exposure":{"high":{"basis":"usage_valuation","kind":"exact","nano":"15000000"},"low":{"basis":"usage_valuation","nano":"15000000"},"pool":"pi_api"},"forwarding":"forwarded","key":"res:S1/turn-1:rid=1","obligation":{"amount_source":"settlement_amount","state":"closed"},"replacement":{"eligible":true,"reason":"UsageBound"},"settlement":{"basis":"usage","charged":"15000000","decision":"valid","reasons":[],"reserved":"500000000"},"valuation":{"complete":true,"derivations":[{"for":"both","model":"gpt-6-astra","nano":"15000000","provider":"openai","tier_multiplier":"1","usage":{"cache_write_tokens":0,"cached_tokens":0,"input_tokens":1000,"output_tokens":100}}],"high":"15000000","identities":["gpt-6-astra"],"low":"15000000","state":"valued"}}],"inventory":[],"lifecycle":{"S1":{"clock":"trusted","reopened":false,"state":"open"}},"non_attempt":[],"unreadable":[]},"historical":{},"inputs":{"gates":[{"authority":{"id":"wev-admission-authority-1","kind":"vertical_admission_authority"},"effective_from":"2026-11-10T06:00:05Z","effective_until":null,"gate_id":"g-S1","mechanisms":{"billing_bound":{"enforced":true,"receipt":{"ref":"authority-receipt:S1-billing_bound","sha256":"eb21af7cd5c8a234faedb9ec0f77080cc75bf11904a5ca6a3744849f12fbac0a"}},"new_forwarding":{"enforced":true,"receipt":{"ref":"authority-receipt:S1-new_forwarding","sha256":"48627919de37a2276f5dcb1de5efce5082508a99f2a0479b1c2923e093152f69"}},"reopening":{"enforced":true,"receipt":{"ref":"authority-receipt:S1-reopening","sha256":"6a59c035f1ded753aa69c6cee1f5626faf2a2bf2d93366d378a07bf12c4cdb08"}},"retries":{"enforced":true,"receipt":{"ref":"authority-receipt:S1-retries","sha256":"ad74cf4e263dd56b37d9cef747ce3273809e06de4eefc5f3050c919c71cb2d39"}}},"scope":{"kind":"session"},"session":"00000000-0000-4000-8000-0000000000c1","writer":"authority"}],"rates":{"anthropic_api_rates.json":"8a286433d9e08b37533f2f908f2db60c4357e1a566018c9ca8364937652bdef5","openai_api_rates.json":"c0bdfd66ee69f1644eaf188f87f6af5e47beacc0739d20aac5846c923d66fd58"},"reference_time":"2026-11-10T07:00:00Z","sessions":[{"alias":"S1","launch":{"auth_lane":"api","model":"openai/gpt-6-astra","parent":"root"},"lifecycle":[{"at":"2026-11-10T06:00:10Z","event":"launch","turn":1}],"logs":{"turn-1":{"final_newline":true,"lines":["{\"at\": \"2026-11-10T06:30:00Z\", \"event\": \"usd_reserved\", \"n\": 1, \"rid\": 1, \"reserved_usd\": 0.5, \"input_bound\": 6000, \"bound_basis\": \"bytes\"}","{\"at\": \"2026-11-10T06:30:02Z\", \"event\": \"request\", \"n\": 1, \"http_status\": 200, \"model\": \"gpt-6-astra\", \"requested_tier\": null, \"observed_tier\": \"default\", \"terminal\": \"response.completed\", \"response_id\": \"resp_1\", \"response_model\": \"gpt-6-astra\", \"usage\": {\"input_tokens\": 1000, \"output_tokens\": 100, \"cached_tokens\": 0, \"cache_write_tokens\": 0, \"reasoning_tokens\": 0}, \"seconds\": 1.0, \"usd\": {\"rid\": 1, \"reserved_usd\": 0.5, \"charged_usd\": 0.015, \"charge_basis\": \"usage\", \"session_spent_usd\": 0.015, \"multiplier_floor\": 0.0}}"]}},"session_id":"00000000-0000-4000-8000-0000000000c1"}]},"invariants":["INV-A9","INV-A8"],"profile":"future_qualified_gate","provenance":{"origin":"contract::2.3 ExecutionGate (future vertical twin of VC14-open-session-commitment)"},"reachability":"producer_unreachable","role":"finding","schema":"pi-accounting-corpus/1"}
   55| {"added_in":"1.3-s1","ambiguity":null,"case_id":"P132-FG-VC01-gated","class":"future_vertical","contract":"pi-accounting-contract/1.4-s1","controls":[],"evaluations":[{"admission_request":{"new":"100000000","objective":"o"},"expected":{"admission":{"decision":"admitted","limit":"1000000000","new":"100000000","objective":"o"},"attribution":[{"attempt":"res:S1/turn-1:rid=1","objective":"o","ownership":"owned","placement":"inside"}],"objectives":[{"commitments":[{"admission_id":"a1","committed":"0","gate":{"qualified":true,"reasons":[]},"kind":"exact","state":"retired"}],"objective":"o","owned_closed":{"high":{"kind":"exact","nano":"15000000"},"low":"15000000"},"owned_outstanding":{"high":{"kind":"exact","nano":"0"},"low":"0"},"predecessor_obligations":{"high":{"kind":"exact","nano":"0"},"low":"0"},"provisional":false,"reasons":[],"status":"established"}]},"name":"default","objectives":[{"admission_authority":{"id":"wev-admission-authority-1","kind":"vertical_admission_authority"},"admissions":[{"admission_id":"a1","gate_id":"g-S1","rows":[{"admission_id":"a1","at":"2026-11-10T06:00:05Z","event":"pending","max_usd":1.0},{"admission_id":"a1","at":"2026-11-10T06:00:10Z","event":"launched","session_id":"00000000-0000-4000-8000-0000000000c1"}],"session":"00000000-0000-4000-8000-0000000000c1","source":"journal"}],"closed_at":null,"limit":"1000000000","objective":"o","predecessor":null,"started_at":"2026-11-10T06:00:00Z","tree_sessions":["00000000-0000-4000-8000-0000000000c1"]}]}],"expected":{"attempts":[{"billing":"established","blocking":[],"correlation":{"kind":"exact"},"diagnostics":[],"evidence":["S1/turn-1:1","S1/turn-1:2"],"execution":"terminal","exposure":{"high":{"basis":"usage_valuation","kind":"exact","nano":"15000000"},"low":{"basis":"usage_valuation","nano":"15000000"},"pool":"pi_api"},"forwarding":"forwarded","key":"res:S1/turn-1:rid=1","obligation":{"amount_source":"settlement_amount","state":"closed"},"replacement":{"eligible":true,"reason":"UsageBound"},"settlement":{"basis":"usage","charged":"15000000","decision":"valid","reasons":[],"reserved":"500000000"},"valuation":{"complete":true,"derivations":[{"for":"both","model":"gpt-6-astra","nano":"15000000","provider":"openai","tier_multiplier":"1","usage":{"cache_write_tokens":0,"cached_tokens":0,"input_tokens":1000,"output_tokens":100}}],"high":"15000000","identities":["gpt-6-astra"],"low":"15000000","state":"valued"}}],"inventory":[],"lifecycle":{"S1":{"clock":"trusted","reopened":false,"state":"retired"}},"non_attempt":[],"unreadable":[]},"historical":{},"inputs":{"gates":[{"authority":{"id":"wev-admission-authority-1","kind":"vertical_admission_authority"},"effective_from":"2026-11-10T06:00:05Z","effective_until":null,"gate_id":"g-S1","mechanisms":{"billing_bound":{"enforced":true,"receipt":{"ref":"authority-receipt:S1-billing_bound","sha256":"eb21af7cd5c8a234faedb9ec0f77080cc75bf11904a5ca6a3744849f12fbac0a"}},"new_forwarding":{"enforced":true,"receipt":{"ref":"authority-receipt:S1-new_forwarding","sha256":"48627919de37a2276f5dcb1de5efce5082508a99f2a0479b1c2923e093152f69"}},"reopening":{"enforced":true,"receipt":{"ref":"authority-receipt:S1-reopening","sha256":"6a59c035f1ded753aa69c6cee1f5626faf2a2bf2d93366d378a07bf12c4cdb08"}},"retries":{"enforced":true,"receipt":{"ref":"authority-receipt:S1-retries","sha256":"ad74cf4e263dd56b37d9cef747ce3273809e06de4eefc5f3050c919c71cb2d39"}}},"scope":{"kind":"session"},"session":"00000000-0000-4000-8000-0000000000c1","writer":"authority"}],"rates":{"anthropic_api_rates.json":"8a286433d9e08b37533f2f908f2db60c4357e1a566018c9ca8364937652bdef5","openai_api_rates.json":"c0bdfd66ee69f1644eaf188f87f6af5e47beacc0739d20aac5846c923d66fd58"},"reference_time":"2026-11-10T07:00:00Z","sessions":[{"alias":"S1","launch":{"auth_lane":"api","model":"openai/gpt-6-astra","parent":"root"},"lifecycle":[{"at":"2026-11-10T06:00:10Z","event":"launch","turn":1},{"at":"2026-11-10T06:50:00Z","event":"retire"}],"logs":{"turn-1":{"final_newline":true,"lines":["{\"at\": \"2026-11-10T06:30:00Z\", \"event\": \"usd_reserved\", \"n\": 1, \"rid\": 1, \"reserved_usd\": 0.5, \"input_bound\": 6000, \"bound_basis\": \"bytes\"}","{\"at\": \"2026-11-10T06:30:02Z\", \"event\": \"request\", \"n\": 1, \"http_status\": 200, \"model\": \"gpt-6-astra\", \"requested_tier\": null, \"observed_tier\": \"default\", \"terminal\": \"response.completed\", \"response_id\": \"resp_1\", \"response_model\": \"gpt-6-astra\", \"usage\": {\"input_tokens\": 1000, \"output_tokens\": 100, \"cached_tokens\": 0, \"cache_write_tokens\": 0, \"reasoning_tokens\": 0}, \"seconds\": 1.0, \"usd\": {\"rid\": 1, \"reserved_usd\": 0.5, \"charged_usd\": 0.015, \"charge_basis\": \"usage\", \"session_spent_usd\": 0.015, \"multiplier_floor\": 0.0}}"]}},"session_id":"00000000-0000-4000-8000-0000000000c1"}]},"invariants":["INV-A9","INV-A8"],"profile":"future_qualified_gate","provenance":{"origin":"contract::2.3 ExecutionGate (future vertical twin of VC01-openai-usage-settlement)"},"reachability":"producer_unreachable","role":"valid_control","schema":"pi-accounting-corpus/1"}
   56| {"added_in":"1.3-s1","ambiguity":null,"case_id":"P132-FG-VC01-no-gate","class":"future_vertical","contract":"pi-accounting-contract/1.4-s1","controls":["P132-FG-VC01-gated"],"evaluations":[{"admission_request":{"new":"100000000","objective":"o"},"expected":{"admission":{"decision":"refused","limit":"1000000000","new":"100000000","objective":"o"},"attribution":[{"attempt":"res:S1/turn-1:rid=1","objective":"o","ownership":"owned","placement":"inside"}],"objectives":[{"commitments":[{"admission_id":"a1","committed":"985000000","gate":{"qualified":false,"reasons":["NoGateEvidence"]},"kind":"at_least","state":"retired_resumable"}],"objective":"o","owned_closed":{"high":{"kind":"exact","nano":"15000000"},"low":"15000000"},"owned_outstanding":{"high":{"kind":"exact","nano":"0"},"low":"0"},"predecessor_obligations":{"high":{"kind":"exact","nano":"0"},"low":"0"},"provisional":false,"reasons":["UnenforcedCommitment"],"status":"not_established"}]},"name":"default","objectives":[{"admission_authority":{"id":"wev-admission-authority-1","kind":"vertical_admission_authority"},"admissions":[{"admission_id":"a1","rows":[{"admission_id":"a1","at":"2026-11-10T06:00:05Z","event":"pending","max_usd":1.0},{"admission_id":"a1","at":"2026-11-10T06:00:10Z","event":"launched","session_id":"00000000-0000-4000-8000-0000000000c1"}],"session":"00000000-0000-4000-8000-0000000000c1","source":"journal"}],"closed_at":null,"limit":"1000000000","objective":"o","predecessor":null,"started_at":"2026-11-10T06:00:00Z","tree_sessions":["00000000-0000-4000-8000-0000000000c1"]}]}],"expected":{"attempts":[{"billing":"established","blocking":[],"correlation":{"kind":"exact"},"diagnostics":[],"evidence":["S1/turn-1:1","S1/turn-1:2"],"execution":"terminal","exposure":{"high":{"basis":"usage_valuation","kind":"exact","nano":"15000000"},"low":{"basis":"usage_valuation","nano":"15000000"},"pool":"pi_api"},"forwarding":"forwarded","key":"res:S1/turn-1:rid=1","obligation":{"amount_source":"settlement_amount","state":"closed"},"replacement":{"eligible":true,"reason":"UsageBound"},"settlement":{"basis":"usage","charged":"15000000","decision":"valid","reasons":[],"reserved":"500000000"},"valuation":{"complete":true,"derivations":[{"for":"both","model":"gpt-6-astra","nano":"15000000","provider":"openai","tier_multiplier":"1","usage":{"cache_write_tokens":0,"cached_tokens":0,"input_tokens":1000,"output_tokens":100}}],"high":"15000000","identities":["gpt-6-astra"],"low":"15000000","state":"valued"}}],"inventory":[],"lifecycle":{"S1":{"clock":"trusted","reopened":false,"state":"retired"}},"non_attempt":[],"unreadable":[]},"historical":{},"inputs":{"gates":[],"rates":{"anthropic_api_rates.json":"8a286433d9e08b37533f2f908f2db60c4357e1a566018c9ca8364937652bdef5","openai_api_rates.json":"c0bdfd66ee69f1644eaf188f87f6af5e47beacc0739d20aac5846c923d66fd58"},"reference_time":"2026-11-10T07:00:00Z","sessions":[{"alias":"S1","launch":{"auth_lane":"api","model":"openai/gpt-6-astra","parent":"root"},"lifecycle":[{"at":"2026-11-10T06:00:10Z","event":"launch","turn":1},{"at":"2026-11-10T06:50:00Z","event":"retire"}],"logs":{"turn-1":{"final_newline":true,"lines":["{\"at\": \"2026-11-10T06:30:00Z\", \"event\": \"usd_reserved\", \"n\": 1, \"rid\": 1, \"reserved_usd\": 0.5, \"input_bound\": 6000, \"bound_basis\": \"bytes\"}","{\"at\": \"2026-11-10T06:30:02Z\", \"event\": \"request\", \"n\": 1, \"http_status\": 200, \"model\": \"gpt-6-astra\", \"requested_tier\": null, \"observed_tier\": \"default\", \"terminal\": \"response.completed\", \"response_id\": \"resp_1\", \"response_model\": \"gpt-6-astra\", \"usage\": {\"input_tokens\": 1000, \"output_tokens\": 100, \"cached_tokens\": 0, \"cache_write_tokens\": 0, \"reasoning_tokens\": 0}, \"seconds\": 1.0, \"usd\": {\"rid\": 1, \"reserved_usd\": 0.5, \"charged_usd\": 0.015, \"charge_basis\": \"usage\", \"session_spent_usd\": 0.015, \"multiplier_floor\": 0.0}}"]}},"session_id":"00000000-0000-4000-8000-0000000000c1"}]},"invariants":["INV-A9","INV-A8"],"profile":"future_qualified_gate","provenance":{"origin":"contract::2.3 ExecutionGate (future vertical twin of VC01-openai-usage-settlement)"},"reachability":"producer_unreachable","role":"finding","schema":"pi-accounting-corpus/1"}
===== END FILE corpus/sample-cases.jsonl =====

===== BEGIN FILE producer/pi_container.py.excerpts | sha256 bfd226df9fc1fb8b5395a74490e3c1696273fab08e15d0e4da19b951320bb4b8 | 55157 bytes | 857 lines | encoding verbatim =====
# pi_container.py (sha256 of whole file 780837341790780f84163ee6151b8ce66cff56f5a3f6ddf8c15f939dba50f470)
# Line-numbered excerpts; everything else omitted.

# ---- check_request: lines 526-545 ----
  526| def check_request(body, *, model_id: str) -> str | None:
  527|     """A refusal reason, or None when the request may go upstream."""
  528|     if not isinstance(body, dict):
  529|         return "body_not_object"
  530|     unexpected = sorted(set(body) - ALLOWED_FIELDS)
  531|     if unexpected:
  532|         return "unexpected_field:" + ",".join(unexpected)
  533|     if body.get("model") != model_id:
  534|         return "model_not_pinned"
  535|     if body.get("store") is True:
  536|         return "store_refused"
  537|     choice = body.get("tool_choice")
  538|     if choice is not None and not (
  539|             (isinstance(choice, str) and choice in TOOL_CHOICE_STRINGS)
  540|             or (isinstance(choice, dict) and choice.get("type") in TOOL_CHOICE_TYPES)):
  541|         return "tool_choice_refused"
  542|     hosted = declared_tool_types(body) - ALLOWED_TOOL_TYPES
  543|     if hosted:
  544|         return "hosted_tool_refused:" + ",".join(sorted(str(t) for t in hosted))
  545|     return None

# ---- anthropic_identity_refusal: lines 625-639 ----
  625| def anthropic_identity_refusal(event, model_id: str) -> str | None:
  626|     """None when a stream's first event is a message_start naming exactly the
  627|     authorized model; otherwise why the stream is refused. The response's own
  628|     message.model is checked, not the request's."""
  629|     if not isinstance(event, dict) or event.get("type") != "message_start":
  630|         return "response_model_not_first_event"
  631|     message = event.get("message")
  632|     if not isinstance(message, dict):
  633|         return "response_model_malformed"
  634|     if "model" not in message or message["model"] is None:
  635|         return "response_model_missing"
  636|     if not isinstance(message["model"], str) or not message["model"]:
  637|         return "response_model_malformed"
  638|     return None if message["model"] == model_id else "response_model_mismatch"
  639| 

# ---- usage_usd, reserve_usd, input_bound, unpriceable_input: lines 680-760 ----
  680| def usage_usd(rates: dict, *, input_tokens, cached, cache_write, output_tokens, multiplier) -> float:
  681|     """Same formula as usage_ledger.codex_usd, times the tier multiplier."""
  682|     long = input_tokens > rates["long_input_threshold"]
  683|     m_in, m_out = (rates["long_input_multiplier"], rates["long_output_multiplier"]) if long else (1, 1)
  684|     base = ((input_tokens - cached - cache_write) * rates["input"] + cached * rates["cached_input"]
  685|             + cache_write * rates["cache_write"]) * m_in + output_tokens * rates["output"] * m_out
  686|     return base / 1e6 * multiplier
  687| 
  688| 
  689| def reserve_usd(rates: dict, *, input_bound: int, max_output_tokens: int, multiplier: float) -> float:
  690|     """Worst case: every input token at the dearest input rate (a cache can always miss)."""
  691|     long = input_bound > rates["long_input_threshold"]
  692|     m_in, m_out = (rates["long_input_multiplier"], rates["long_output_multiplier"]) if long else (1, 1)
  693|     dearest = max(rates["input"], rates["cache_write"])
  694|     return (input_bound * dearest * m_in + max_output_tokens * rates["output"] * m_out) / 1e6 * multiplier
  695| 
  696| 
  697| def _canonical(value) -> bytes:
  698|     return json.dumps(value, sort_keys=True, separators=(",", ":")).encode()
  699| 
  700| 
  701| def input_basis(body: dict, items_key: str = "input") -> dict:
  702|     """Byte sizes and a hash chain over the request's input items
  703|     (`input` for Responses, `messages` for Anthropic Messages).
  704| 
  705|     chains[k] identifies the first k items, so a later request can be checked
  706|     for starting with an earlier request's whole input. A token is at least
  707|     one byte of its text and JSON escaping only adds bytes, so a canonical
  708|     item's byte length bounds its text tokens.
  709|     """
  710|     items = body.get(items_key, [])
  711|     if not isinstance(items, list):
  712|         items = [items]
  713|     rest = _canonical({k: v for k, v in body.items() if k != items_key})
  714|     h = hashlib.sha256(b"").digest()
  715|     chains, sizes = [h.hex()], []
  716|     for item in items:
  717|         raw = _canonical(item)
  718|         sizes.append(len(raw))
  719|         h = hashlib.sha256(h + hashlib.sha256(raw).digest()).digest()
  720|         chains.append(h.hex())
  721|     return {"items": len(items), "chains": chains, "item_bytes": sizes,
  722|             "rest_sha256": hashlib.sha256(rest).hexdigest(), "rest_bytes": len(rest)}
  723| 
  724| 
  725| def input_bound(basis: dict, prefix: dict | None) -> tuple[int, str]:
  726|     """Upper bound on the request's input tokens, and how it was found.
  727| 
  728|     prefix: the provider's input_tokens for an earlier request whose input
  729|     this request starts with. Those tokens already cover its instructions and
  730|     tools; changed ones are counted again in full.
  731|     """
  732|     n, sizes = basis["items"], basis["item_bytes"]
  733|     if prefix and isinstance(prefix.get("input_tokens"), int) and prefix["items"] <= n \
  734|             and basis["chains"][prefix["items"]] == prefix["chain_sha256"]:
  735|         k = prefix["items"]
  736|         bound = prefix["input_tokens"] + sum(sizes[k:]) + ITEM_OVERHEAD_TOKENS * (n - k)
  737|         if basis["rest_sha256"] != prefix["rest_sha256"]:
  738|             bound += basis["rest_bytes"] + ITEM_OVERHEAD_TOKENS
  739|         return bound, "prefix"
  740|     return basis["rest_bytes"] + sum(sizes) + ITEM_OVERHEAD_TOKENS * (n + 1), "bytes"
  741| 
  742| 
  743| def unpriceable_input(value) -> str | None:
  744|     """Inputs whose tokens the body's bytes do not bound: files, and images by URL or file id.
  745| 
  746|     An inline (data:) image is bounded by its base64 length; that an image's
  747|     token count stays below it is assumed, not verified for every detail level.
  748|     """
  749|     if isinstance(value, dict):
  750|         kind = value.get("type")
  751|         if kind == "input_file":
  752|             return "usd_limit_unpriceable_input:input_file"
  753|         if kind == "input_image" and not str(value.get("image_url") or "").startswith("data:"):
  754|             return "usd_limit_unpriceable_input:input_image"
  755|         for item in value.values():
  756|             found = unpriceable_input(item)
  757|             if found:
  758|                 return found
  759|     elif isinstance(value, list):
  760|         for item in value:

# ---- budgets: SubscriptionProxyBudget, UsdBudget, AnthropicUsdBudget: lines 868-1076 ----
  868| class SubscriptionProxyBudget:
  869|     """Serialized stopping threshold, with no reservation or hard output cap.
  870| 
  871|     The proxy owns locking and durably writes admission before any upstream effect.
  872|     An unknown exchange keeps its admission unresolved and permanently stops this session.
  873|     """
  874|     max_output_tokens = None
  875| 
  876|     def __init__(self, *, max_usd, rates, spent_before_usd=0.0):
  877|         self.max_usd, self.rates = max_usd, rates
  878|         self.settled = spent_before_usd
  879|         self.inflight = None
  880|         self.exhausted = spent_before_usd >= max_usd
  881|         self.unknown = False
  882|         self.serial = 0
  883| 
  884|     def admit(self):
  885|         if self.exhausted:
  886|             return None, "usd_limit: subscription proxy threshold reached or accounting unknown"
  887|         if self.inflight is not None:
  888|             return None, "subscription_proxy_request_in_flight"
  889|         self.serial += 1
  890|         self.inflight = self.serial
  891|         return {"rid": self.serial}, None
  892| 
  893|     def settle(self, ticket, *, http_status, response, terminal, stream_error):
  894|         usage = response.get("usage")
  895|         reason = None
  896|         if (http_status != 200 or stream_error or terminal not in ("response.completed", "response.done")):
  897|             reason = "exchange_not_completed"
  898|         elif (response.get("model") != self.rates["model"] or not isinstance(response.get("id"), str)
  899|               or not response["id"].strip()):
  900|             reason = "response_identity_not_established"
  901|         elif response.get("status") not in (None, "completed") or response.get("error"):
  902|             reason = "exchange_not_completed"
  903|         elif not isinstance(usage, dict):
  904|             reason = "usage_not_established"
  905|         else:
  906|             counts = [usage.get(k) for k in ("input_tokens", "output_tokens")]
  907|             if not all(type(v) is int and v >= 0 for v in counts):
  908|                 reason = "usage_not_established"
  909|         amount, basis = None, None
  910|         if reason is None:
  911|             i, o = counts
  912|             details = usage.get("input_tokens_details")
  913|             if details is not None and not isinstance(details, dict):
  914|                 reason = "invalid_cache_usage"
  915|             else:
  916|                 details = details or {}
  917|                 c, w = details.get("cached_tokens"), details.get("cache_write_tokens")
  918|                 if any(v is not None and (type(v) is not int or v < 0 or v > i) for v in (c, w)) or (c or 0) + (w or 0) > i:
  919|                     reason = "invalid_cache_usage"
  920|                 else:
  921|                     if c is None or w is None:
  922|                         # Missing optional cache counts: value all input at the dearer rate.
  923|                         c, w = 0, i if self.rates["cache_write"] > self.rates["input"] else 0
  924|                         basis = "conservative_all_input_at_dearest_rate"
  925|                     else:
  926|                         basis = "reported_cache_usage"
  927|                     try:
  928|                         amount = usage_usd(self.rates, input_tokens=i, cached=c, cache_write=w,
  929|                                            output_tokens=o, multiplier=self.rates["tier_multiplier"]["default"])
  930|                         if not math.isfinite(amount) or amount < 0:
  931|                             reason = "valuation_not_established"
  932|                     except (ArithmeticError, ValueError):
  933|                         reason = "valuation_not_established"
  934|         if reason:
  935|             amount = None
  936|             self.unknown = self.exhausted = True
  937|         else:
  938|             self.settled = math.fsum((self.settled, amount))
  939|             self.exhausted = self.settled >= self.max_usd
  940|         return {"rid": ticket["rid"], "status": "unknown" if reason else "valued",
  941|                 "cost_proxy_usd": amount, "valuation_basis": basis, "reason": reason,
  942|                 "session_cost_proxy_usd": self.settled, "limit_reached": self.exhausted,
  943|                 "measurement": SUBSCRIPTION_PROXY_BASIS, "valuation_tier": "default",
  944|                 "model": self.rates["model"], "rates_sha256": hashlib.sha256(_canonical(self.rates)).hexdigest(),
  945|                 "billing": "unknown", "quota": "unknown", "limitation": SUBSCRIPTION_PROXY_LIMITATION}
  946| 
  947| 
  948| class UsdBudget:
  949|     """A session dollar limit, enforced per request. Callers hold the proxy lock.
  950| 
  951|     reserve() sets aside the request's worst-case cost or refuses; settle()
  952|     replaces it with the charge from the provider's final usage. Reservations
  953|     use the requested tier's multiplier (the dearer of it and default with
  954|     strict_tier). A response served at another tier is charged at that tier,
  955|     and later reservations use at least the dearer of it and default; a
  956|     response reporting no tier is charged at the dearest multiplier, which
  957|     then becomes that floor. After the first refusal every later request is
  958|     refused.
  959|     """
  960| 
  961|     def __init__(self, *, max_usd: float, rates: dict, tier: str | None, max_output_tokens: int,
  962|                  spent_before_usd: float = 0.0, prefix: dict | None = None,
  963|                  multiplier_floor: float = 0.0, strict_tier: bool = False):
  964|         if (tier or "default") not in rates["tier_multiplier"]:
  965|             raise ContainerError("usage", f"a dollar limit needs a priced tier, not {tier!r}")
  966|         self.max_usd, self.rates, self.tier = max_usd, rates, tier or "default"
  967|         self.max_output_tokens, self.strict_tier = max_output_tokens, strict_tier
  968|         self.settled, self.outstanding = spent_before_usd, 0.0
  969|         self.prefix, self.floor = prefix, multiplier_floor
  970|         self.exhausted, self.seq = False, 0
  971| 
  972|     def multiplier(self) -> float:
  973|         table = self.rates["tier_multiplier"]
  974|         m = max(table[self.tier], table["default"]) if self.strict_tier else table[self.tier]
  975|         return max(m, self.floor)
  976| 
  977|     def worst_case(self, bound: int) -> float:
  978|         return reserve_usd(self.rates, input_bound=bound, max_output_tokens=self.max_output_tokens,
  979|                            multiplier=self.multiplier())
  980| 
  981|     def reserve(self, basis: dict):
  982|         """(ticket, None) when the request may go, or (None, refusal detail)."""
  983|         bound, how = input_bound(basis, self.prefix)
  984|         amount = self.worst_case(bound)
  985|         if self.exhausted or self.settled + self.outstanding + amount > self.max_usd:
  986|             self.exhausted = True
  987|             return None, (f"usd_limit: spent {self.settled + self.outstanding:.4f} + worst case {amount:.4f} "
  988|                           f"> limit {self.max_usd:.4f}")
  989|         self.seq += 1
  990|         self.outstanding += amount
  991|         return {"rid": self.seq, "reserved_usd": amount, "input_bound": bound, "bound_basis": how,
  992|                 "basis": basis}, None
  993| 
  994|     def settle(self, ticket: dict, *, http_status, response: dict) -> dict:
  995|         """Record the charge. http_status None: the upstream exchange failed part way."""
  996|         self.outstanding -= ticket["reserved_usd"]
  997|         table = self.rates["tier_multiplier"]
  998|         usage = response.get("usage") or {}
  999|         details = usage.get("input_tokens_details") or {}
 1000|         i, o = usage.get("input_tokens"), usage.get("output_tokens")
 1001|         c, w = details.get("cached_tokens") or 0, details.get("cache_write_tokens") or 0
 1002|         if http_status is not None and http_status != 200:
 1003|             # An error status before any stream: treated as unbilled (assumed, not verified).
 1004|             charged, how = 0.0, "error_status"
 1005|         elif all(isinstance(x, int) and x >= 0 for x in (i, o, c, w)) and c + w <= i:
 1006|             observed = response.get("service_tier")
 1007|             m = table[observed] if observed in table else max(table.values())
 1008|             if observed != self.tier:
 1009|                 self.floor = max(self.floor, table["default"], m)
 1010|             charged, how = usage_usd(self.rates, input_tokens=i, cached=c, cache_write=w, output_tokens=o,
 1011|                                      multiplier=m), "usage"
 1012|             basis = ticket["basis"]
 1013|             self.prefix = {"items": basis["items"], "chain_sha256": basis["chains"][-1],
 1014|                            "rest_sha256": basis["rest_sha256"], "input_tokens": i}
 1015|         else:
 1016|             charged, how = ticket["reserved_usd"], "reservation"
 1017|         self.settled += charged
 1018|         return {"rid": ticket["rid"], "reserved_usd": round(ticket["reserved_usd"], 6),
 1019|                 "charged_usd": round(charged, 6), "charge_basis": how,
 1020|                 "session_spent_usd": round(self.settled + self.outstanding, 6), "multiplier_floor": self.floor}
 1021| 
 1022| 
 1023| class AnthropicUsdBudget(UsdBudget):
 1024|     """UsdBudget for Anthropic Messages, with the usage ledger's rates (native_usd).
 1025| 
 1026|     No tiers: the proxy refuses service_tier and speed, so a request runs at
 1027|     standard rates; a response reporting another tier keeps its reservation.
 1028|     Worst case: every input token at the dearest of the input and both
 1029|     cache-write rates, plus the output cap (thinking counts as output). Cache
 1030|     writes are charged by lifetime; writes the provider did not split by
 1031|     lifetime are charged at the dearer rate. No long-context rule is applied,
 1032|     as in the ledger.
 1033|     """
 1034| 
 1035|     def multiplier(self) -> float:
 1036|         return 1.0
 1037| 
 1038|     def worst_case(self, bound: int) -> float:
 1039|         r = self.rates
 1040|         dearest = max(r["input"], r["cache_write_5m"], r["cache_write_1h"])
 1041|         return (bound * dearest + self.max_output_tokens * r["output"]) / 1e6
 1042| 
 1043|     def settle(self, ticket: dict, *, http_status, response: dict) -> dict:
 1044|         self.outstanding -= ticket["reserved_usd"]
 1045|         u, r = response.get("usage") or {}, self.rates
 1046|         i, cr, cw, o = (u.get(k) for k in ("input_tokens", "cache_read_input_tokens",
 1047|                                              "cache_creation_input_tokens", "output_tokens"))
 1048|         w5, w1 = u.get("ephemeral_5m_input_tokens") or 0, u.get("ephemeral_1h_input_tokens") or 0
 1049|         total = None
 1050|         if http_status is not None and http_status != 200:
 1051|             charged, how = 0.0, "error_status"  # assumed unbilled, not verified
 1052|         elif http_status is not None \
 1053|                 and all(isinstance(x, int) and not isinstance(x, bool) and x >= 0 for x in (i, cr, cw, o, w5, w1)) \
 1054|                 and w5 + w1 <= cw and u.get("service_tier") in (None, "standard"):
 1055|             # http_status None: the stream ended before message_stop, so the
 1056|             # usage seen so far (message_start) understates output; keep the reservation.
 1057|             charged = (i * r["input"] + cr * r["cache_read"] + w5 * r["cache_write_5m"] + w1 * r["cache_write_1h"]
 1058|                        + (cw - w5 - w1) * max(r["cache_write_5m"], r["cache_write_1h"]) + o * r["output"]) / 1e6
 1059|             how, total = "usage", i + cr + cw
 1060|             basis = ticket["basis"]
 1061|             self.prefix = {"items": basis["items"], "chain_sha256": basis["chains"][-1],
 1062|                            "rest_sha256": basis["rest_sha256"], "input_tokens": total}
 1063|         else:
 1064|             charged, how = ticket["reserved_usd"], "reservation"
 1065|         self.settled += charged
 1066|         out = {"rid": ticket["rid"], "reserved_usd": round(ticket["reserved_usd"], 6),
 1067|                "charged_usd": round(charged, 6), "charge_basis": how,
 1068|                "session_spent_usd": round(self.settled + self.outstanding, 6), "multiplier_floor": self.floor}
 1069|         if total is not None:
 1070|             out["input_total"] = total
 1071|         return out
 1072| 
 1073| 
 1074| BUDGETS = {"openai": UsdBudget, "anthropic": AnthropicUsdBudget}
 1075| 
 1076| 

# ---- EgressProxy init, log, refuse, forward_anthropic (reserve, stream, stream_error, identity refusal, request row): lines 1078-1330 ----
 1078|     """HTTP/1.1 on a Unix socket; one upstream request per accepted request."""
 1079| 
 1080|     def __init__(self, socket_path: str, *, upstream: str, key: str, model_id: str,
 1081|                  tier: str | None, log_path: str, max_requests: int | None = None,
 1082|                  max_output_tokens: int | None = None, budget: UsdBudget | None = None,
 1083|                  on_usd_exhausted=None,
 1084|                  max_connections: int = MAX_CONNECTIONS, max_body: int = MAX_BODY_BYTES,
 1085|                  socket_timeout: float = SOCKET_TIMEOUT, provider: str = "openai",
 1086|                  admission_wait: float = ADMISSION_WAIT_SECONDS):
 1087|         if provider == "openai-codex" and (tier or max_output_tokens is not None):
 1088|             raise ValueError("Codex subscription takes no service tier, API budget or output cap")
 1089|         if budget is not None and max_output_tokens != budget.max_output_tokens:
 1090|             raise ValueError("a dollar limit needs the proxy's output cap to be the budget's")
 1091|         if provider not in PROVIDERS:
 1092|             raise ValueError(f"no egress route for provider {provider!r}")
 1093|         if budget is not None and type(budget) is not ({**BUDGETS, "openai-codex": SubscriptionProxyBudget})[provider]:
 1094|             raise ValueError(f"wrong budget type for {provider}")
 1095|         if provider == "openai-codex":
 1096|             self.account = codex_account(key)
 1097|         if provider == "anthropic" and tier:
 1098|             raise ValueError("the anthropic route sets no service tier")
 1099|         self.provider, self.route = provider, PROVIDERS[provider]["route"]
 1100|         self.socket_path, self.key, self.model_id, self.tier = socket_path, key, model_id, tier
 1101|         self.upstream = urllib.parse.urlsplit(upstream)
 1102|         self.log_path, self.max_requests = log_path, max_requests
 1103|         self.max_output_tokens, self.budget, self.on_usd_exhausted = max_output_tokens, budget, on_usd_exhausted
 1104|         self.max_connections, self.max_body, self.socket_timeout = max_connections, max_body, socket_timeout
 1105|         self.accepted = 0
 1106|         self.dropped_connections = 0
 1107|         self.lock = threading.Lock()
 1108|         # Signalled under self.lock whenever a subscription admission is released.
 1109|         self.settled = threading.Condition(self.lock)
 1110|         self.admission_wait = admission_wait
 1111|         self.server = None
 1112| 
 1113|     def log(self, row: dict):
 1114|         with self.lock:
 1115|             self._log_locked(row)
 1116| 
 1117|     def _log_locked(self, row, *, durable=False):
 1118|         with open(self.log_path, "a") as fh:
 1119|             fh.write(json.dumps({"at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()), **row}) + "\n")
 1120|             if durable:
 1121|                 fh.flush()
 1122|                 os.fsync(fh.fileno())
 1123| 
 1124|     def start(self):
 1125|         proxy = self
 1126|         # Exists from the start, so a turn with no requests still leaves a
 1127|         # readable (empty) log for the next turn's spend total.
 1128|         open(self.log_path, "a").close()
 1129|         if isinstance(self.budget, SubscriptionProxyBudget):
 1130|             # Make the log's directory entry durable before an admitted request can leave.
 1131|             fd = os.open(str(Path(self.log_path).parent), os.O_RDONLY | os.O_DIRECTORY)
 1132|             try:
 1133|                 os.fsync(fd)
 1134|             finally:
 1135|                 os.close(fd)
 1136| 
 1137|         class Handler(BaseHTTPRequestHandler):
 1138|             # HTTP/1.0 framing: the streamed body ends when the connection closes.
 1139|             timeout = proxy.socket_timeout  # bounds slow headers and bodies
 1140| 
 1141|             def log_message(self, *args):
 1142|                 pass
 1143| 
 1144|             def refuse(self, code: int, reason: str):
 1145|                 proxy.log({"event": "refused", "method": self.command, "path": self.path, "http_status": code, "reason": reason})
 1146|                 payload = json.dumps({"error": {"message": f"role_host egress proxy: {reason}", "type": "egress_refused"}}).encode()
 1147|                 self.send_response(code)
 1148|                 self.send_header("content-type", "application/json")
 1149|                 self.send_header("content-length", str(len(payload)))
 1150|                 self.send_header("connection", "close")
 1151|                 self.end_headers()
 1152|                 self.wfile.write(payload)
 1153|                 self.close_connection = True
 1154| 
 1155|             def forward_anthropic(self, body, n, release):
 1156|                 """Anthropic Messages: check, cap, reserve, forward with the host key, stream back, log usage."""
 1157|                 query = urllib.parse.urlsplit(self.path).query
 1158|                 betas = [b.strip() for b in (self.headers.get("anthropic-beta") or "").split(",") if b.strip()]
 1159|                 reason = "query_refused" if query not in ("", "beta=true") else \
 1160|                     check_anthropic_request(body, model_id=proxy.model_id, betas=betas)
 1161|                 if reason is None and proxy.max_output_tokens is not None:
 1162|                     asked = body.get("max_tokens")
 1163|                     if not (isinstance(asked, int) and not isinstance(asked, bool)
 1164|                             and 0 < asked < proxy.max_output_tokens):
 1165|                         body["max_tokens"] = proxy.max_output_tokens
 1166|                     thinking = body.get("thinking") or {}
 1167|                     budget_tokens = thinking.get("budget_tokens")
 1168|                     if thinking.get("type") == "enabled" and not (
 1169|                             isinstance(budget_tokens, int) and budget_tokens < body["max_tokens"]):
 1170|                         reason = "thinking_budget_not_below_output_cap"
 1171|                 if reason is None and proxy.budget is not None:
 1172|                     reason = anthropic_unpriceable(body.get("system")) or anthropic_unpriceable(body.get("messages"))
 1173|                 if reason:
 1174|                     release()
 1175|                     return self.refuse(403, reason)
 1176|                 ticket = None
 1177|                 if proxy.budget is not None:
 1178|                     basis = input_basis(body, items_key="messages")
 1179|                     with proxy.lock:
 1180|                         ticket, refusal = proxy.budget.reserve(basis)
 1181|                     if ticket is None:
 1182|                         release()
 1183|                         self.refuse(429, refusal)
 1184|                         if proxy.on_usd_exhausted:
 1185|                             proxy.on_usd_exhausted(refusal)
 1186|                         return
 1187|                     proxy.log({"event": "usd_reserved", "n": n, "rid": ticket["rid"],
 1188|                                "reserved_usd": round(ticket["reserved_usd"], 6),
 1189|                                "input_bound": ticket["input_bound"], "bound_basis": ticket["bound_basis"]})
 1190| 
 1191|                 def settle(http_status, response):
 1192|                     if ticket is None:
 1193|                         return {}
 1194|                     with proxy.lock:
 1195|                         usd = proxy.budget.settle(ticket, http_status=http_status, response=response)
 1196|                     b = ticket["basis"]
 1197|                     return {"usd": usd, "input_basis": {"items": b["items"], "chain_sha256": b["chains"][-1],
 1198|                                                         "rest_sha256": b["rest_sha256"]}}
 1199| 
 1200|                 data = json.dumps(body).encode()
 1201|                 # Headers are rebuilt, not copied: the container's x-api-key and
 1202|                 # authorization never go upstream, and only allowed betas do.
 1203|                 headers = {"content-type": "application/json", "anthropic-version": ANTHROPIC_VERSION,
 1204|                            "x-api-key": proxy.key, "content-length": str(len(data))}
 1205|                 if betas:
 1206|                     headers["anthropic-beta"] = ",".join(betas)
 1207|                 up = proxy.upstream
 1208|                 conn_cls = http.client.HTTPSConnection if up.scheme == "https" else http.client.HTTPConnection
 1209|                 started = time.time()
 1210|                 try:
 1211|                     conn = conn_cls(up.hostname, up.port, timeout=900)
 1212|                     conn.request("POST", up.path.rstrip("/") + proxy.route + (f"?{query}" if query else ""),
 1213|                                  data, headers)
 1214|                     resp = conn.getresponse()
 1215|                 except OSError as e:
 1216|                     proxy.log({"event": "upstream_error", "provider": "anthropic", "n": n, "detail": str(e)[:300],
 1217|                                **settle(None, {})})
 1218|                     return self.refuse(502, "upstream unreachable")
 1219|                 replied = []
 1220| 
 1221|                 def reply_headers():
 1222|                     replied.append(True)
 1223|                     self.send_response(resp.status)
 1224|                     for k, v in resp.getheaders():
 1225|                         if k.lower() not in ("transfer-encoding", "connection", "content-length", "keep-alive"):
 1226|                             self.send_header(k, v)
 1227|                     self.send_header("connection", "close")
 1228|                     self.end_headers()
 1229| 
 1230|                 # A 200 stream is held, line by line, until its first event (pings
 1231|                 # aside) is a message_start naming exactly the authorized model;
 1232|                 # nothing from an unverified model reaches the container. Error
 1233|                 # statuses carry no model output and pass through as before.
 1234|                 streaming = resp.status == 200
 1235|                 if not streaming:
 1236|                     reply_headers()
 1237|                 usage, buf, head = {}, b"", b""
 1238|                 message, terminal, stop_reason, error_type, stream_error = {}, None, None, None, None
 1239|                 verified, identity_refusal, held, held_bytes = not streaming, None, [], 0
 1240|                 try:
 1241|                     while identity_refusal is None:
 1242|                         chunk = resp.read1(65536)
 1243|                         if not chunk:
 1244|                             break
 1245|                         if not streaming:
 1246|                             self.wfile.write(chunk)
 1247|                             self.wfile.flush()
 1248|                             if len(head) < 2000:
 1249|                                 head += chunk[: 2000 - len(head)]
 1250|                         buf += chunk
 1251|                         *lines, buf = buf.split(b"\n")
 1252|                         for line in lines:
 1253|                             event, kind = None, None
 1254|                             if line.startswith(b"data:"):
 1255|                                 try:
 1256|                                     event = json.loads(line[5:].strip())
 1257|                                 except ValueError:
 1258|                                     event = None
 1259|                                 kind = event.get("type") if isinstance(event, dict) else None
 1260|                                 if streaming and not verified and kind != "ping":
 1261|                                     identity_refusal = anthropic_identity_refusal(event, proxy.model_id)
 1262|                                     verified = identity_refusal is None
 1263|                                 elif streaming and kind == "message_start":
 1264|                                     identity_refusal = "response_model_restated"
 1265|                             if streaming and identity_refusal is None:
 1266|                                 held.append(line + b"\n")
 1267|                                 held_bytes += len(line) + 1
 1268|                             if isinstance(event, dict):
 1269|                                 if kind == "message_start" and isinstance(event.get("message"), dict):
 1270|                                     message = event["message"]
 1271|                                     merge_anthropic_usage(usage, message.get("usage"))
 1272|                                 elif kind == "message_delta":
 1273|                                     merge_anthropic_usage(usage, event.get("usage"))
 1274|                                     stop_reason = (event.get("delta") or {}).get("stop_reason") or stop_reason
 1275|                                 elif kind == "message_stop":
 1276|                                     terminal = kind
 1277|                                 elif kind == "error":
 1278|                                     error_type = (event.get("error") or {}).get("type") or "error"
 1279|                             if identity_refusal:
 1280|                                 break
 1281|                         if streaming and verified and identity_refusal is None and held:
 1282|                             if not replied:
 1283|                                 reply_headers()
 1284|                             self.wfile.write(b"".join(held))
 1285|                             self.wfile.flush()
 1286|                             held, held_bytes = [], 0
 1287|                         elif streaming and not verified and held_bytes + len(buf) > ANTHROPIC_IDENTITY_HOLD_BYTES:
 1288|                             identity_refusal = "response_model_not_established"
 1289|                     if streaming and verified and identity_refusal is None and buf:
 1290|                         self.wfile.write(buf)
 1291|                         self.wfile.flush()
 1292|                 except OSError as e:
 1293|                     # The container went away (cancellation) or the upstream broke.
 1294|                     stream_error = str(e)[:300]
 1295|                     proxy.log({"event": "stream_error", "provider": "anthropic", "n": n, "detail": stream_error})
 1296|                 finally:
 1297|                     conn.close()
 1298|                 if streaming and not verified and identity_refusal is None:
 1299|                     identity_refusal = "response_model_not_established"
 1300|                 response_model = loggable_model(message.get("model"))
 1301|                 if identity_refusal:
 1302|                     # Fail closed: no retry, no fallback; the reservation stays charged below.
 1303|                     proxy.log({"event": "response_model_refused", "provider": "anthropic", "n": n,
 1304|                                "reason": identity_refusal, "authorized_model": proxy.model_id,
 1305|                                "response_model": response_model, "response_id": message.get("id"),
 1306|                                "verified_bytes_forwarded": bool(replied), "detail": stream_error})
 1307|                 proxy.log({
 1308|                     "event": "request", "provider": "anthropic", "n": n, "http_status": resp.status,
 1309|                     "model": body.get("model"), "requested_tier": None, "observed_tier": usage.get("service_tier"),
 1310|                     "terminal": terminal, "response_id": message.get("id"), "response_model": response_model,
 1311|                     **({"model_mismatch": True} if response_model and response_model != proxy.model_id else {}),
 1312|                     **({"response_model_check": identity_refusal or "verified"} if streaming else {}),
 1313|                     "stop_reason": stop_reason, "thinking": (body.get("thinking") or {}).get("type"),
 1314|                     "effort": (body.get("output_config") or {}).get("effort"), "betas": betas,
 1315|                     "usage": {k: usage.get(k) for k in ANTHROPIC_USAGE_KEYS},
 1316|                     **({"server_tool_use": usage["server_tool_use"]} if usage.get("server_tool_use") else {}),
 1317|                     **({"stream_error_type": error_type} if error_type else {}),
 1318|                     "seconds": round(time.time() - started, 3),
 1319|                     **({"error_head": head.decode("utf-8", "replace")} if resp.status != 200 else {}),
 1320|                     **settle(None if identity_refusal else
 1321|                              resp.status if terminal is not None or resp.status != 200 else None, {"usage": usage}),
 1322|                 })
 1323|                 self.close_connection = True
 1324|                 if identity_refusal and not replied:
 1325|                     try:
 1326|                         self.refuse(502, identity_refusal)
 1327|                     except OSError:
 1328|                         pass
 1329| 
 1330|             def do_GET(self):

# ---- do_POST: accept counter, release, subscription admission, reservation, settle, finish, OpenAI writers, request row: lines 1330-1575 ----
 1330|             def do_GET(self):
 1331|                 self.refuse(403, f"only POST {proxy.route} is allowed")
 1332| 
 1333|             do_PUT = do_DELETE = do_PATCH = do_GET
 1334| 
 1335|             def do_POST(self):
 1336|                 if self.path.split("?", 1)[0] != proxy.route:
 1337|                     return self.refuse(403, f"only POST {proxy.route} is allowed")
 1338|                 if self.headers.get("transfer-encoding"):
 1339|                     return self.refuse(411, "chunked or encoded request bodies are not accepted")
 1340|                 length = self.headers.get("content-length", "")
 1341|                 if not length.isdigit() or not 0 < int(length) <= proxy.max_body:
 1342|                     return self.refuse(413 if length.isdigit() else 411,
 1343|                                        f"content-length must be between 1 and {proxy.max_body}")
 1344|                 # Reserve a slot under the cap before any body is buffered; a
 1345|                 # request refused below gives its slot back.
 1346|                 with proxy.lock:
 1347|                     spent_out = proxy.budget is not None and proxy.budget.exhausted
 1348|                     if spent_out or (proxy.max_requests is not None and proxy.accepted >= proxy.max_requests):
 1349|                         n = None
 1350|                     else:
 1351|                         proxy.accepted += 1
 1352|                         n = proxy.accepted
 1353|                 if spent_out:
 1354|                     return self.refuse(429, "usd_limit: reached earlier in this turn")
 1355|                 if n is None:
 1356|                     return self.refuse(429, f"request cap {proxy.max_requests} reached")
 1357| 
 1358|                 def release():
 1359|                     with proxy.lock:
 1360|                         proxy.accepted -= 1
 1361| 
 1362|                 try:
 1363|                     raw = self.rfile.read(int(length))
 1364|                 except OSError:
 1365|                     release()
 1366|                     return proxy.log({"event": "refused", "reason": "body not received in time"})
 1367|                 if len(raw) != int(length):
 1368|                     release()
 1369|                     return self.refuse(400, "body shorter than content-length")
 1370|                 try:
 1371|                     encoding = self.headers.get("content-encoding", "identity")
 1372|                     if proxy.provider == "openai-codex" and encoding == "zstd":
 1373|                         import zstandard
 1374|                         frame = zstandard.get_frame_parameters(raw)
 1375|                         if (frame.window_size > proxy.max_body or
 1376|                                 frame.content_size not in (zstandard.CONTENTSIZE_UNKNOWN, zstandard.CONTENTSIZE_ERROR)
 1377|                                 and frame.content_size > proxy.max_body):
 1378|                             raise ValueError()
 1379|                         # One native frame only. Reject concatenated/skippable trailing frames;
 1380|                         # never let an unrestricted decoder process a later unvalidated window.
 1381|                         raw = zstandard.ZstdDecompressor().decompress(
 1382|                             raw, max_output_size=proxy.max_body, allow_extra_data=False)
 1383|                     elif encoding != "identity":
 1384|                         raise ValueError()
 1385|                     body = json.loads(raw)
 1386|                 except Exception:
 1387|                     # Do not retain compressed body/decoder errors: they may contain private data.
 1388|                     release()
 1389|                     return self.refuse(400, "body is not JSON")
 1390|                 if proxy.provider == "anthropic":
 1391|                     return self.forward_anthropic(body, n, release)
 1392|                 reason = check_request(body, model_id=proxy.model_id)
 1393|                 if reason:
 1394|                     release()
 1395|                     return self.refuse(403, reason)
 1396|                 if proxy.provider == "openai-codex" and (body.get("stream") is not True or body.get("store") is not False
 1397|                                                           or "service_tier" in body or "max_output_tokens" in body):
 1398|                     release()
 1399|                     return self.refuse(403, "subscription_request_fields_refused")
 1400|                 if proxy.tier:
 1401|                     body["service_tier"] = proxy.tier
 1402|                 else:
 1403|                     body.pop("service_tier", None)
 1404|                 if proxy.max_output_tokens is not None:
 1405|                     asked = body.get("max_output_tokens")
 1406|                     if not (isinstance(asked, int) and not isinstance(asked, bool)
 1407|                             and 0 < asked < proxy.max_output_tokens):
 1408|                         body["max_output_tokens"] = proxy.max_output_tokens
 1409|                 ticket = None
 1410|                 subscription_budget = isinstance(proxy.budget, SubscriptionProxyBudget)
 1411|                 if subscription_budget:
 1412|                     with proxy.lock:
 1413|                         # Serialize, do not refuse: wait for the previous exchange to settle.
 1414|                         proxy.settled.wait_for(lambda: proxy.budget.inflight is None or proxy.budget.exhausted,
 1415|                                                timeout=proxy.admission_wait)
 1416|                         ticket, refusal = proxy.budget.admit()
 1417|                         if ticket is not None:
 1418|                             try:
 1419|                                 proxy._log_locked({"event": "subscription_proxy_admitted", "n": n,
 1420|                                                    "rid": ticket["rid"], "model": proxy.model_id}, durable=True)
 1421|                             except OSError:
 1422|                                 proxy.budget.unknown = proxy.budget.exhausted = True
 1423|                                 ticket, refusal = None, "usd_limit: admission persistence failed"
 1424|                                 if proxy.on_usd_exhausted:
 1425|                                     proxy.on_usd_exhausted(refusal)
 1426|                     if ticket is None:
 1427|                         release()
 1428|                         self.refuse(429, refusal)
 1429|                         if proxy.budget.exhausted and proxy.on_usd_exhausted:
 1430|                             proxy.on_usd_exhausted(refusal)
 1431|                         return
 1432|                 elif proxy.budget is not None:
 1433|                     reason = unpriceable_input(body.get("input"))
 1434|                     if reason:
 1435|                         release()
 1436|                         return self.refuse(403, reason)
 1437|                     basis = input_basis(body)
 1438|                     with proxy.lock:
 1439|                         ticket, refusal = proxy.budget.reserve(basis)
 1440|                     if ticket is None:
 1441|                         release()
 1442|                         self.refuse(429, refusal)
 1443|                         if proxy.on_usd_exhausted:
 1444|                             proxy.on_usd_exhausted(refusal)
 1445|                         return
 1446|                     proxy.log({"event": "usd_reserved", "n": n, "rid": ticket["rid"],
 1447|                                "reserved_usd": round(ticket["reserved_usd"], 6),
 1448|                                "input_bound": ticket["input_bound"], "bound_basis": ticket["bound_basis"]})
 1449| 
 1450|                 def settle(http_status, response):
 1451|                     if ticket is None or subscription_budget:
 1452|                         return {}
 1453|                     with proxy.lock:
 1454|                         usd = proxy.budget.settle(ticket, http_status=http_status, response=response)
 1455|                     b = ticket["basis"]
 1456|                     return {"usd": usd, "input_basis": {"items": b["items"], "chain_sha256": b["chains"][-1],
 1457|                                                         "rest_sha256": b["rest_sha256"]}}
 1458| 
 1459|                 def finish(row, *, http_status=None, response=None, terminal=None, stream_error=False):
 1460|                     if not subscription_budget:
 1461|                         proxy.log(row)
 1462|                         return
 1463|                     with proxy.lock:
 1464|                         try:
 1465|                             measurement = proxy.budget.settle(ticket, http_status=http_status, response=response or {},
 1466|                                                               terminal=terminal, stream_error=stream_error)
 1467|                             try:
 1468|                                 proxy._log_locked({**row, "subscription_proxy": measurement}, durable=True)
 1469|                             except OSError:
 1470|                                 proxy.budget.unknown = proxy.budget.exhausted = True
 1471|                                 raise
 1472|                             finally:
 1473|                                 if proxy.budget.exhausted and proxy.on_usd_exhausted:
 1474|                                     proxy.on_usd_exhausted("subscription proxy threshold reached or accounting unknown")
 1475|                             proxy.budget.inflight = None
 1476|                         finally:
 1477|                             # Wake waiters even on failure: they re-check exhaustion and refuse.
 1478|                             proxy.settled.notify_all()
 1479| 
 1480|                 data = json.dumps(body).encode()
 1481|                 headers = {k: v for k, v in self.headers.items()
 1482|                            if k.lower() not in ("host", "authorization", "content-length", "connection",
 1483|                                                 "transfer-encoding", "keep-alive")}
 1484|                 headers.update({"authorization": f"Bearer {proxy.key}", "content-length": str(len(data))})
 1485|                 if proxy.provider == "openai-codex":
 1486|                     headers = {"authorization": f"Bearer {proxy.key}", "chatgpt-account-id": proxy.account,
 1487|                                "originator": "pi", "OpenAI-Beta": "responses=experimental",
 1488|                                "accept": "text/event-stream", "content-type": "application/json",
 1489|                                "content-length": str(len(data))}
 1490|                 up = proxy.upstream
 1491|                 conn_cls = http.client.HTTPSConnection if up.scheme == "https" else http.client.HTTPConnection
 1492|                 started = time.time()
 1493|                 try:
 1494|                     conn = conn_cls(up.hostname, up.port, timeout=900)
 1495|                     conn.request("POST", up.path.rstrip("/") + "/responses", data, headers)
 1496|                     resp = conn.getresponse()
 1497|                 except (OSError, http.client.HTTPException) as e:
 1498|                     # The request may have reached the provider, so a reservation stays charged.
 1499|                     finish({"event": "upstream_error", "n": n, "detail": str(e)[:300],
 1500|                                **({"provider": "openai-codex", "auth_lane": "subscription", "model": proxy.model_id}
 1501|                                   if proxy.provider == "openai-codex" else {}), **settle(None, {})})
 1502|                     return self.refuse(502, "upstream unreachable")
 1503|                 stream_error = False
 1504|                 try:
 1505|                     self.send_response(resp.status)
 1506|                     for k, v in resp.getheaders():
 1507|                         if k.lower() not in ("transfer-encoding", "connection", "content-length", "keep-alive"):
 1508|                             self.send_header(k, v)
 1509|                     self.send_header("connection", "close")
 1510|                     self.end_headers()
 1511|                 except OSError:
 1512|                     conn.close()
 1513|                     finish({"event": "stream_error", "n": n}, stream_error=True)
 1514|                     return
 1515|                 terminal, buf, head = None, b"", b""
 1516|                 try:
 1517|                     while True:
 1518|                         chunk = resp.read1(65536)
 1519|                         if not chunk:
 1520|                             # read1() can silently return EOF with declared bytes still missing.
 1521|                             # Chunked framing raises IncompleteRead itself; close-delimited SSE
 1522|                             # has no remaining length and relies on its terminal event.
 1523|                             if subscription_budget and resp.length not in (None, 0):
 1524|                                 stream_error = True
 1525|                             break
 1526|                         self.wfile.write(chunk)
 1527|                         self.wfile.flush()
 1528|                         if resp.status != 200 and len(head) < 2000:
 1529|                             head += chunk[: 2000 - len(head)]
 1530|                         buf += chunk
 1531|                         *lines, buf = buf.split(b"\n")
 1532|                         for line in lines:
 1533|                             if line.startswith(b"data: "):
 1534|                                 try:
 1535|                                     event = json.loads(line[6:])
 1536|                                 except ValueError:
 1537|                                     continue
 1538|                                 if isinstance(event, dict):
 1539|                                     if subscription_budget and (event.get("type") == "error" or
 1540|                                             event.get("type") in TERMINAL_EVENTS and terminal is not None):
 1541|                                         stream_error = True
 1542|                                     if event.get("type") in TERMINAL_EVENTS:
 1543|                                         terminal = event
 1544|                 except (OSError, http.client.HTTPException) as e:
 1545|                     stream_error = True
 1546|                     proxy.log({"event": "stream_error", "n": n, "detail": str(e)[:300]})
 1547|                 finally:
 1548|                     conn.close()
 1549|                 response = (terminal or {}).get("response") or {}
 1550|                 response = response if isinstance(response, dict) else {}
 1551|                 usage = response.get("usage") or {}
 1552|                 usage = usage if isinstance(usage, dict) else {}
 1553|                 input_details = usage.get("input_tokens_details")
 1554|                 input_details = input_details if isinstance(input_details, dict) else {}
 1555|                 output_details = usage.get("output_tokens_details")
 1556|                 output_details = output_details if isinstance(output_details, dict) else {}
 1557|                 finish({
 1558|                     "event": "request", "n": n, "http_status": resp.status, "model": body.get("model"),
 1559|                     **({"provider": "openai-codex", "auth_lane": "subscription", "billing": "unknown", "quota": "unknown"}
 1560|                        if proxy.provider == "openai-codex" else {}),
 1561|                     "requested_tier": body.get("service_tier"), "observed_tier": response.get("service_tier"),
 1562|                     "terminal": (terminal or {}).get("type"), "response_id": response.get("id"),
 1563|                     "response_model": response.get("model"),
 1564|                     "usage": {"input_tokens": usage.get("input_tokens"), "output_tokens": usage.get("output_tokens"),
 1565|                               "cached_tokens": input_details.get("cached_tokens"),
 1566|                               "cache_write_tokens": input_details.get("cache_write_tokens"),
 1567|                               "reasoning_tokens": output_details.get("reasoning_tokens")},
 1568|                     "seconds": round(time.time() - started, 3),
 1569|                     **({"error_head": head.decode("utf-8", "replace")} if resp.status != 200 and proxy.provider != "openai-codex" else {}),
 1570|                     **settle(resp.status if terminal is not None or resp.status != 200 else None, response),
 1571|                 }, http_status=resp.status, response=response, terminal=(terminal or {}).get("type"), stream_error=stream_error)
 1572|                 self.close_connection = True
 1573| 
 1574|         slots = threading.BoundedSemaphore(self.max_connections)
 1575| 

# ---- Server: threaded handlers: lines 1576-1592 ----
 1576|         class Server(socketserver.ThreadingMixIn, socketserver.UnixStreamServer):
 1577|             daemon_threads = True
 1578| 
 1579|             def process_request(self, request, client_address):
 1580|                 # Refuse beyond the connection bound instead of spawning a thread.
 1581|                 if not slots.acquire(blocking=False):
 1582|                     with proxy.lock:
 1583|                         proxy.dropped_connections += 1
 1584|                     self.shutdown_request(request)
 1585|                     return
 1586|                 super().process_request(request, client_address)
 1587| 
 1588|             def process_request_thread(self, request, client_address):
 1589|                 try:
 1590|                     super().process_request_thread(request, client_address)
 1591|                 finally:
 1592|                     slots.release()
===== END FILE producer/pi_container.py.excerpts =====

===== BEGIN FILE producer/role_host.py.excerpts | sha256 e71ef49afad8d30068c646e49a81e1c27a8b28e39cb8c507bded3f375d418bbf | 25857 bytes | 370 lines | encoding verbatim =====
# role_host.py (sha256 of whole file cbca1bc9656ed6de2c1cd43e1e357d7ed27dbaa08b178a3131cc329ed3433475)
# Line-numbered excerpts; everything else omitted.

# ---- Ledger.append (flock, at): lines 242-247 ----
  242|     def append(self, row: dict) -> None:
  243|         row = {"at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()), **row}
  244|         with open(self.path, "a") as fh:
  245|             fcntl.flock(fh, fcntl.LOCK_EX)
  246|             fh.write(json.dumps(row, sort_keys=True) + "\n")
  247| 

# ---- Pi._spawn and turn rows: lines 1169-1312 ----
 1169|     def _spawn(self, sid, opts, *, cwd, prompt, ledger, event, name=None, parent=None, env=None):
 1170|         args = dict(cwd=cwd, prompt=prompt, ledger=ledger, event=event, name=name, parent=parent, env=env)
 1171|         if not env or (env.get("usd_limit") or {}).get("measurement") != "api_equivalent_cost_proxy":
 1172|             return self._spawn_unlocked(sid, opts, **args)
 1173|         # Session exclusion must span fresh busy/accounting checks, ordinal selection,
 1174|         # runner startup and ledger publication, not just one proxy's request threads.
 1175|         with open(Path(env["host_dir"]) / "turn-admission.lock", "a") as lock:
 1176|             try:
 1177|                 fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
 1178|             except BlockingIOError:
 1179|                 raise HostError(BUSY, "refused_busy", f"{sid} has a turn admission in progress") from None
 1180|             try:
 1181|                 if event == "deliver":
 1182|                     st = self.status(sid, ledger)
 1183|                     if st["state"] == "running":
 1184|                         raise HostError(BUSY, "refused_busy", f"{sid} has turn {st['turn']} in progress")
 1185|                     if st["state"] == "absent":
 1186|                         raise HostError(NOT_ESTABLISHED, "deliver_failed", f"pi session {sid} is unknown to this host")
 1187|                     turns = [r for r in self._turns(sid, ledger) if r.get("turn")]
 1188|                     if turns and turns[-1].get("pid_start") is None:
 1189|                         raise HostError(NOT_ESTABLISHED, "usd_spend_not_established",
 1190|                                         "previous runner identity is unknown; no turn started")
 1191|                 return self._spawn_unlocked(sid, opts, **args)
 1192|             finally:
 1193|                 fcntl.flock(lock, fcntl.LOCK_UN)
 1194| 
 1195|     def _spawn_unlocked(self, sid, opts, *, cwd, prompt, ledger, event, name=None, parent=None, env=None):
 1196|         turn = sum(1 for r in self._turns(sid, ledger) if r.get("turn")) + 1
 1197|         if env is not None:
 1198|             return self._spawn_container(sid, opts, cwd=cwd, prompt=prompt, ledger=ledger, event=event,
 1199|                                          turn=turn, env=env, name=name, parent=parent)
 1200|         folder = ledger.path.parent / "pi" / sid
 1201|         folder.mkdir(parents=True, exist_ok=True)
 1202|         files = {k: folder / f"turn-{turn}.{suffix}" for k, suffix in self.TURN_FILES.items()}
 1203|         self._write_prompt(files["prompt"], prompt)
 1204|         argv = [self.exe, "--mode", "json", "--session-id", sid, "-e", str(self.EXTENSION), *opts["argv"]]
 1205|         if name:
 1206|             argv += ["--name", name]
 1207|         env = {**child_env(), "WORK_ENGINE_PI_SERVICE_TIER": opts["tier"] or "",
 1208|                "WORK_ENGINE_PI_SERVICE_TIER_MODEL": opts["model"],
 1209|                "WORK_ENGINE_PI_SERVICE_TIER_RECORD": str(files["tier_record"])}
 1210|         try:
 1211|             proc = subprocess.Popen(["/bin/sh", "-c", self.WRAPPER, "sh",
 1212|                                      *(str(files[k]) for k in ("prompt", "events", "stderr", "exit")), *argv],
 1213|                                     cwd=cwd, env=env, stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL,
 1214|                                     stderr=subprocess.DEVNULL, start_new_session=True)
 1215|         except OSError as e:
 1216|             raise HostError(NOT_ESTABLISHED, "fail_launch", f"pi turn: {e}")
 1217|         row = self._turn_row(sid, opts, event=event, turn=turn, proc=proc, files=files)
 1218|         if event == "launch":
 1219|             row.update(name=name, cwd=str(Path(cwd).resolve()), parent_session_id=parent)
 1220|         ledger.append(row)
 1221|         self._verify_header(proc, row, files, ledger)
 1222|         return row
 1223| 
 1224|     @staticmethod
 1225|     def _write_prompt(path, prompt, *, durable=False):
 1226|         try:
 1227|             with open(path, "x") as fh:
 1228|                 fh.write(prompt)
 1229|                 if durable:
 1230|                     fh.flush()
 1231|                     os.fsync(fh.fileno())
 1232|             if durable:
 1233|                 fd = os.open(str(path.parent), os.O_RDONLY | os.O_DIRECTORY)
 1234|                 try:
 1235|                     os.fsync(fd)
 1236|                 finally:
 1237|                     os.close(fd)
 1238|         except FileExistsError:
 1239|             raise HostError(NOT_ESTABLISHED, "turn_not_established", f"{path} already exists")
 1240| 
 1241|     def _turn_row(self, sid, opts, *, event, turn, proc, files):
 1242|         return {"host": self.name, "event": event, "session_id": sid, "turn": turn, "pid": proc.pid,
 1243|                 "pid_start": self._start_time(proc.pid), "at_epoch": time.time(),
 1244|                 "files": {k: str(v) for k, v in files.items()}, "model": opts["model"],
 1245|                 "thinking": opts["thinking"], "auth_lane": opts["lane"], "service_tier": opts["tier"],
 1246|                 "host_argv": opts["argv"]}
 1247| 
 1248|     before_runner_start = None  # set by an objective-admitted launch: writes the runner_starting marker
 1249| 
 1250|     def _spawn_container(self, sid, opts, *, cwd, prompt, ledger, event, turn, env, name=None, parent=None):
 1251|         cdir, hdir = Path(env["container_dir"]), Path(env["host_dir"])
 1252|         files = {k: cdir / f"turn-{turn}.{suffix}" for k, suffix in self.TURN_FILES.items()}
 1253|         files.update({k: hdir / f"turn-{turn}.{suffix}" for k, suffix in (
 1254|             ("proxy_log", "proxy.jsonl"), ("runner_record", "runner.jsonl"),
 1255|             ("runner_log", "runner.log"), ("spec", "spec.json"))})
 1256|         prior = self._prior_usd(sid, ledger, env) if env.get("usd_limit") else None
 1257|         opts = {**opts, **({"container_subscription": True} if env["selection"].get("lane") == "subscription" else {})}
 1258|         key = self.provider_key(opts, cwd=cwd)
 1259|         if (env.get("usd_limit") or {}).get("measurement") == "api_equivalent_cost_proxy":
 1260|             # Reserve the ordinal in host-only storage before any runner effect.
 1261|             # Agent-writable prompt deletion must not hide an unrecorded orphan turn.
 1262|             admission = {"session_id": sid, "turn": turn, "proxy_log": str(files["proxy_log"])}
 1263|             self._write_prompt(hdir / f"turn-{turn}.admission.json", json.dumps(admission) + "\n", durable=True)
 1264|         self._write_prompt(files["prompt"], prompt)
 1265|         self.socket_root().mkdir(mode=0o700, parents=True, exist_ok=True)
 1266|         socket_dir = self.socket_root() / f"{sid}-{turn}"
 1267|         container_name = f"we-pi-{sid}-{turn}"
 1268|         pi_argv = ["--mode", "json", "--session-id", sid, "--session-dir", env["session_dir"],
 1269|                    "-e", str(self.EXTENSION), *opts["argv"]]
 1270|         if name:
 1271|             pi_argv += ["--name", name]
 1272|         container_env = _pi_container().container_env(env, str(files["tier_record"]))
 1273|         route = _pi_container().PROVIDERS[opts["provider"]]
 1274|         spec = {"session_id": sid, "turn": turn, "cwd": str(Path(cwd).resolve()), "container_name": container_name,
 1275|                 "docker": self.docker(), "socket_dir": str(socket_dir), "socket_path": str(socket_dir / "egress.sock"),
 1276|                 "container_dir": env["container_dir"], "environment": env, "container_env": container_env,
 1277|                 "pi_argv": pi_argv, "files": {k: str(v) for k, v in files.items()},
 1278|                 "upstream": os.environ.get(route["upstream_env"], route["upstream"]),
 1279|                 **({"provider": opts["provider"]} if opts["provider"] != "openai" else {}),
 1280|                 "model_id": opts["model"].split("/", 1)[1], "service_tier": opts["tier"],
 1281|                 "max_requests": env.get("max_requests"), "max_output_tokens": env.get("max_output_tokens"),
 1282|                 "usd_limit": {**env["usd_limit"], **prior} if prior is not None else None}
 1283|         files["spec"].write_text(json.dumps(spec, indent=1) + "\n")
 1284|         runner = Path(__file__).resolve().parent / "pi_container.py"
 1285|         if self.before_runner_start is not None:
 1286|             # Budget-admitted launch: the durable runner_starting marker is written and verified before any
 1287|             # provider-capable process exists; if it cannot be, nothing is started.
 1288|             self.before_runner_start(session_id=sid, turn=turn, spec=str(files["spec"]), container=container_name)
 1289|         with open(files["runner_log"], "ab") as log:
 1290|             try:
 1291|                 proc = subprocess.Popen([sys.executable, "-I", str(runner), "run-turn", str(files["spec"])],
 1292|                                         stdin=subprocess.PIPE, stdout=log, stderr=subprocess.STDOUT,
 1293|                                         env=child_env(), cwd=str(hdir), start_new_session=True)
 1294|             except OSError as e:
 1295|                 raise HostError(NOT_ESTABLISHED, "fail_launch", f"pi container runner: {e}")
 1296|         try:
 1297|             proc.stdin.write((key + "\n").encode())
 1298|             proc.stdin.close()
 1299|         except OSError:
 1300|             pass  # the runner died; the header wait reports it
 1301|         del key
 1302|         row = self._turn_row(sid, opts, event=event, turn=turn, proc=proc, files=files)
 1303|         row["container"] = {"name": container_name, "fingerprint_sha256": env["fingerprint"]["sha256"]}
 1304|         if prior is not None:
 1305|             row["usd_limit"] = {**env["usd_limit"], "spent_before_usd": prior["spent_before_usd"],
 1306|                                 "max_output_tokens": env["max_output_tokens"]}
 1307|         if event == "launch":
 1308|             row.update(name=name, cwd=str(Path(cwd).resolve()), parent_session_id=parent, container_environment=env)
 1309|         ledger.append(row)
 1310|         self._verify_header(proc, row, files, ledger)
 1311|         return row
 1312| 

# ---- status, launch, deliver, retire: lines 1486-1606 ----
 1486|     def status(self, sid, ledger=None) -> dict:
 1487|         rows = self._turns(sid, ledger) if ledger else []
 1488|         turns = [r for r in rows if r.get("turn")]
 1489|         session_dir = self._session_dir_for(sid, ledger)
 1490|         if not turns:
 1491|             if self.session_file(sid) is None:
 1492|                 return {"state": "absent", "kind": None}
 1493|             raise HostError(NOT_ESTABLISHED, "status_not_established",
 1494|                             f"pi session {sid} exists but role_host started none of its turns; its holder is not observable here")
 1495|         last = turns[-1]
 1496|         kind = "pi_container_turn" if last.get("container") else "pi_turn"
 1497|         if self._alive(last):
 1498|             return {"state": "running", "kind": kind, "turn": last["turn"], "pid": last["pid"]}
 1499|         if last.get("container") and self._container_state(last["container"]["name"]):
 1500|             # The runner (and with it the egress proxy) is gone; the container is not.
 1501|             return {"state": "running", "kind": "container_without_runner", "turn": last["turn"],
 1502|                     "container": last["container"]["name"]}
 1503|         lifecycle = [r for r in rows if r.get("event") in ("launch", "deliver", "retire")]
 1504|         state = "stopped" if lifecycle[-1].get("event") == "retire" else "idle"
 1505|         if state == "idle" and self.session_file(sid, session_dir) is None:
 1506|             state = "absent"
 1507|         out = {"state": state, "kind": kind, "last_turn": self.observe_turn(last)}
 1508|         leftovers = self._leftovers(last)
 1509|         if leftovers:
 1510|             out["leftovers"] = leftovers  # the runner died before cleanup; retire collects them
 1511|         return out
 1512| 
 1513|     def launch(self, *, cwd, name, prompt, host_args, ledger, parent=None, lane=None, tier=None, container=None) -> dict:
 1514|         opts = self.options(host_args, lane, tier)
 1515|         if container is not None:
 1516|             if lane == "subscription":
 1517|                 if opts["provider"] != "openai-codex":
 1518|                     raise HostError(USAGE, "usage", "container subscription uses the openai-codex provider and existing host Codex ChatGPT auth")
 1519|                 try:
 1520|                     __import__("zstandard")
 1521|                 except ImportError:
 1522|                     raise HostError(NOT_ESTABLISHED, "transport_not_ready", "Codex subscription requires the host zstandard decoder") from None
 1523|                 opts["container_subscription"] = True
 1524|             elif opts["provider"] == "openai-codex":
 1525|                 raise HostError(USAGE, "usage", "openai-codex is a subscription transport, not an API lane")
 1526|             routes = _pi_container().PROVIDERS
 1527|             if opts["provider"] not in routes:
 1528|                 raise HostError(USAGE, "usage", f"container mode routes providers {', '.join(sorted(routes))} "
 1529|                                 f"through its egress proxy, not {opts['provider']!r}")
 1530|             if container.get("codebase_memory") and "--no-mcp" in opts["argv"]:
 1531|                 raise HostError(USAGE, "usage", "--codebase-memory needs MCP; drop --no-mcp")
 1532|         auth = self.check_lane(opts, cwd=cwd, env=child_env())
 1533|         sid = str(uuid.uuid4())
 1534|         env = (self.container_environment(sid, ledger, cwd=cwd, container=container, opts=opts)
 1535|                if container is not None else None)
 1536|         row = self._spawn(sid, opts, cwd=cwd, prompt=prompt, ledger=ledger, event="launch", name=name,
 1537|                           parent=parent, env=env)
 1538|         out = {**row, "credential": auth, "session_header_verified": True}
 1539|         if env is not None:
 1540|             out["environment_fingerprint"] = env["fingerprint"]["sha256"]
 1541|         return out
 1542| 
 1543|     def deliver(self, sid, *, message, cwd, ledger, host_args=()) -> dict:
 1544|         if host_args:
 1545|             raise HostError(USAGE, "usage", "pi delivery reuses the launch settings; launch a new session to change them")
 1546|         st = self.status(sid, ledger)
 1547|         if st["state"] == "running":
 1548|             raise HostError(BUSY, "refused_busy", f"{sid} has turn {st['turn']} in progress")
 1549|         if st["state"] == "absent":
 1550|             raise HostError(NOT_ESTABLISHED, "deliver_failed", f"pi session {sid} is unknown to this host")
 1551|         launch = self._launch_row(sid, ledger)
 1552|         # Pi resolves --session-id within the working directory's project; from
 1553|         # another directory it would silently create a new session.
 1554|         if Path(cwd).resolve() != Path(launch["cwd"]):
 1555|             raise HostError(USAGE, "usage", f"pi sessions belong to their working directory; deliver with --cwd {launch['cwd']}")
 1556|         opts = {"argv": launch["host_argv"], "model": launch["model"], "provider": launch["model"].split("/", 1)[0],
 1557|                 "thinking": launch["thinking"], "lane": launch["auth_lane"], "tier": launch["service_tier"]}
 1558|         env = launch.get("container_environment")
 1559|         if env is not None:
 1560|             env = self._check_environment(env, cwd=launch["cwd"], ledger=ledger)
 1561|         if env is not None and env["selection"].get("lane") == "subscription":
 1562|             opts["container_subscription"] = True
 1563|         auth = self.check_lane(opts, cwd=cwd, env=child_env())
 1564|         row = self._spawn(sid, opts, cwd=cwd, prompt=message, ledger=ledger, event="deliver", env=env)
 1565|         return {"session_id": sid, "delivered": True, "turn": row["turn"], "credential": auth}
 1566| 
 1567|     def retire(self, sid, *, ledger, force=False) -> dict:
 1568|         st = self.status(sid, ledger)
 1569|         turns = [r for r in self._turns(sid, ledger) if r.get("turn")]
 1570|         if st["state"] in ("absent", "stopped"):
 1571|             collected = self._collect_leftovers(turns[-1]) if turns else {}
 1572|             if collected:
 1573|                 ledger.append({"host": self.name, "event": "collect", "session_id": sid,
 1574|                                "turn": turns[-1]["turn"], "collected": collected})
 1575|             return {"session_id": sid, "retired": "already_not_loaded", "state": st["state"],
 1576|                     **({"collected": collected} if collected else {})}
 1577|         if st["state"] == "running":
 1578|             if not force:
 1579|                 raise HostError(BUSY, "refused_busy", f"{sid} has turn {st['turn']} in progress")
 1580|             last = [r for r in self._turns(sid, ledger) if r.get("turn")][-1]
 1581|             name = (last.get("container") or {}).get("name")
 1582|             # SIGTERM only: on the host Pi kills the tool process trees it started
 1583|             # detached (utils/shell.js); in a container the runner stops the
 1584|             # container, and the kernel ends everything in it with its PID 1.
 1585|             if self._alive(last):
 1586|                 try:
 1587|                     os.killpg(last["pid"], signal.SIGTERM)
 1588|                 except ProcessLookupError:
 1589|                     pass
 1590|                 wait = self.STOP_WAIT + (20 if name else 0)
 1591|                 deadline = time.time() + wait
 1592|                 while self._group_alive(last["pid"]):
 1593|                     if time.time() > deadline:
 1594|                         raise HostError(NOT_ESTABLISHED, "retire_not_established",
 1595|                                         f"turn {last['turn']} process group {last['pid']} still alive {wait}s after SIGTERM")
 1596|                     time.sleep(0.2)
 1597|             collected = self._collect_leftovers(last)
 1598|             ledger.append({"host": self.name, "event": "retire", "session_id": sid, "terminated_turn": last["turn"],
 1599|                            **({"collected": collected} if collected else {})})
 1600|             return {"session_id": sid, "retired": "turn_terminated", "turn": last["turn"],
 1601|                     **({"collected": collected} if collected else {})}
 1602|         collected = self._collect_leftovers(turns[-1]) if turns else {}
 1603|         ledger.append({"host": self.name, "event": "retire", "session_id": sid,
 1604|                        **({"collected": collected} if collected else {})})
 1605|         return {"session_id": sid, "retired": "idle_released", **({"collected": collected} if collected else {})}
 1606| 

# ---- objective_admission and admission journal: lines 1607-1695 ----
 1607| def objective_admission(record, max_usd, parent, ledger_path, journal=None) -> dict:
 1608|     """--objective-record: the usage ledger admits this Pi session's --max-usd against the objective's
 1609|     owner-set pi_api limit, counting spend and every open session's unspent limit. Anything but an
 1610|     admission refuses the launch; the per-session proxy limit is unchanged."""
 1611|     if max_usd is None:
 1612|         raise HostError(USAGE, "usage", "--objective-record needs --max-usd")
 1613|     if not parent:
 1614|         raise HostError(USAGE, "usage", "--objective-record needs --parent: the objective session that owns this Pi session")
 1615|     reader = os.environ.get("ROLE_HOST_USAGE_LEDGER")
 1616|     candidates = [Path(reader)] if reader else [Path(__file__).resolve().parents[3] / "metrics/scripts/usage_ledger.py"]
 1617|     if not reader:
 1618|         top = subprocess.run(["git", "rev-parse", "--show-toplevel"], capture_output=True, text=True)
 1619|         if top.returncode == 0 and top.stdout.strip():
 1620|             candidates.append(Path(top.stdout.strip()) / "metrics/scripts/usage_ledger.py")
 1621|     found = next((c for c in candidates if c.is_file()), None)
 1622|     if found is None:
 1623|         raise HostError(NOT_ESTABLISHED, "objective_accounting_unavailable", "usage ledger not found")
 1624|     r = subprocess.run([sys.executable, "-I", str(found), "pi-admit", "--record", str(Path(record).resolve()),
 1625|                         "--max-usd", repr(float(max_usd)), "--parent", parent, "--role-host-ledger", str(ledger_path),
 1626|                         *(["--admission-journal", str(journal)] if journal else [])],
 1627|                        capture_output=True, text=True, timeout=600)
 1628|     try:
 1629|         out = json.loads(r.stdout)
 1630|     except ValueError:
 1631|         raise HostError(NOT_ESTABLISHED, "objective_accounting_unavailable", (r.stderr or r.stdout)[-800:])
 1632|     if r.returncode != 0 or out.get("decision") != "admitted":
 1633|         status = "objective_pi_limit_refused" if out.get("decision") == "refused" else "objective_accounting_unavailable"
 1634|         raise HostError(NOT_ESTABLISHED, status, json.dumps(out, sort_keys=True))
 1635|     out["reader"] = str(found)
 1636|     out["reader_sha256"] = hashlib.sha256(found.read_bytes()).hexdigest()
 1637|     return out
 1638| 
 1639| 
 1640| def admission_paths(record) -> tuple[Path, Path]:
 1641|     """Per-objective lock and admission journal at one host-wide place, whatever --state a launcher uses."""
 1642|     base = Path(os.environ.get("ROLE_HOST_OBJECTIVE_LOCK_DIR") or
 1643|                 Path(os.environ.get("XDG_STATE_HOME", Path.home() / ".local/state")) / "work-engine" / "pi-objective-admissions")
 1644|     base.mkdir(parents=True, exist_ok=True)
 1645|     key = hashlib.sha256(str(Path(record).resolve()).encode()).hexdigest()[:32]
 1646|     return base / f"{key}.lock", base / f"{key}.admissions.jsonl"
 1647| 
 1648| 
 1649| def journal_append(journal: Path, row: dict) -> None:
 1650|     row = {"at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()), "pid": os.getpid(), **row}
 1651|     with open(journal, "a") as fh:
 1652|         fh.write(json.dumps(row, sort_keys=True) + "\n")
 1653|         fh.flush()
 1654|         os.fsync(fh.fileno())
 1655| 
 1656| 
 1657| def journal_has(journal: Path, admission_id: str, event: str) -> bool:
 1658|     """Whether the journal holds `event` for this admission; an unreadable or corrupted journal raises."""
 1659|     rows = [json.loads(line) for line in journal.read_text().splitlines()]
 1660|     return any(isinstance(r, dict) and r.get("admission_id") == admission_id and r.get("event") == event for r in rows)
 1661| 
 1662| 
 1663| def mark_runner_starting(journal: Path, admission_id: str, **detail) -> None:
 1664|     journal_append(journal, {"event": "runner_starting", "admission_id": admission_id, **detail})
 1665|     try:
 1666|         verified = journal_has(journal, admission_id, "runner_starting")
 1667|     except (OSError, ValueError):
 1668|         verified = False
 1669|     if not verified:
 1670|         raise HostError(NOT_ESTABLISHED, "runner_marker_unverified", f"runner_starting for {admission_id} not readable in {journal}")
 1671| 
 1672| 
 1673| def admit_transaction(record, max_usd, parent, ledger_path, admission_id) -> tuple[dict, Path]:
 1674|     """Under the objective lock: refuse a reused admission id, recompute capacity with the journal, and write
 1675|     the pending allocation before the lock is released."""
 1676|     lock, journal = admission_paths(record)
 1677|     with open(lock, "a") as lf:
 1678|         fcntl.flock(lf, fcntl.LOCK_EX)
 1679|         try:
 1680|             if journal.exists():
 1681|                 for line in journal.read_text().splitlines():
 1682|                     try:
 1683|                         row = json.loads(line)
 1684|                     except ValueError:
 1685|                         raise HostError(NOT_ESTABLISHED, "objective_accounting_unavailable", f"corrupted admission journal {journal}")
 1686|                     if isinstance(row, dict) and row.get("admission_id") == admission_id:
 1687|                         raise HostError(NOT_ESTABLISHED, "admission_duplicate", f"admission {admission_id} already exists in {journal}")
 1688|             out = objective_admission(record, max_usd, parent, ledger_path, journal)
 1689|             journal_append(journal, {"event": "pending", "admission_id": admission_id, "objective_record": out["record"],
 1690|                                      "max_usd": float(max_usd), "parent": parent, "state_ledger": str(ledger_path),
 1691|                                      "admission": {k: out.get(k) for k in ("limit_usd", "spent_high_usd", "committed_usd",
 1692|                                                                            "exposure_usd", "reader_sha256")}})
 1693|         finally:
 1694|             fcntl.flock(lf, fcntl.LOCK_UN)
 1695|     return out, journal
===== END FILE producer/role_host.py.excerpts =====

===== BEGIN FILE producer/scope_budget.py.excerpts | sha256 48b3868f4f1274815d6a601f95cce99498faf1af844052afaf885d35d1fb0a6e | 968 bytes | 19 lines | encoding verbatim =====
# scope_budget.py (sha256 of whole file 5c546edb48ee71efe6db826dfd557d82e307d7e29910c562907f656124dd4e83)
# Line-numbered excerpts; everything else omitted.

# ---- scope_budget close: lines 338-352 ----
  338| def cmd_close(args: argparse.Namespace) -> int:
  339|     path = Path(args.record)
  340|     doc = load(path)
  341|     sb = section(doc, path)
  342|     require_open(sb)
  343|     at = parse(args.at) if args.at else now()
  344|     if at.tzinfo is None:
  345|         raise UsageError("--at must include a timezone")
  346|     if at < parse(sb["started_at"]) or at > now():
  347|         raise UsageError("--at must be between started_at and now")
  348|     closed_at = stamp(at.astimezone(dt.timezone.utc))
  349|     entry = {"at": closed_at, "disposition": args.disposition, "reason": args.reason, "set_by": args.set_by}
  350|     sb["closed_at"], sb["close"] = closed_at, entry
  351|     save(path, doc)
  352|     print(json.dumps(entry))
===== END FILE producer/scope_budget.py.excerpts =====

===== BEGIN FILE producer/openai_api_rates.json | sha256 c0bdfd66ee69f1644eaf188f87f6af5e47beacc0739d20aac5846c923d66fd58 | 736 bytes | 14 lines | encoding numbered =====
    1| {
    2|  "source": "metrics/scripts/usage_ledger.py CODEX_RATES, API_TIER_MULTIPLIER and PRICING_RULES (codex_long_*); tests/test_pi_container.py fails if these differ",
    3|  "unit": "USD per million tokens",
    4|  "models": {
    5|   "gpt-6.1-terra": {"input": 1, "cached_input": 0.05, "cache_write": 1.25, "output": 5},
    6|   "gpt-6-sol": {"input": 2, "cached_input": 0.2, "cache_write": 2.5, "output": 10},
    7|   "gpt-6.1-sol": {"input": 2, "cached_input": 0.1, "cache_write": 2.5, "output": 10},
    8|   "gpt-6-astra": {"input": 10, "cached_input": 1, "cache_write": 12.5, "output": 50}
    9|  },
   10|  "tier_multiplier": {"default": 1.0, "flex": 0.5, "priority": 2.0, "fast": 2.0},
   11|  "long_input_threshold": 272000,
   12|  "long_input_multiplier": 2,
   13|  "long_output_multiplier": 1.5
   14| }
===== END FILE producer/openai_api_rates.json =====

===== BEGIN FILE producer/anthropic_api_rates.json | sha256 8a286433d9e08b37533f2f908f2db60c4357e1a566018c9ca8364937652bdef5 | 1179 bytes | 57 lines | encoding numbered =====
    1| {
    2|  "source": "metrics/scripts/usage_ledger.py NATIVE_RATES (USD per million tokens); claude-haiku-5-5 is left out because the ledger applies a long-input rule to it. A test keeps these equal.",
    3|  "models": {
    4|   "claude-sonnet-5-5": {
    5|    "input": 2,
    6|    "output": 10,
    7|    "cache_write_5m": 2.5,
    8|    "cache_write_1h": 4,
    9|    "cache_read": 0.1
   10|   },
   11|   "claude-opus-5-5": {
   12|    "input": 4,
   13|    "output": 20,
   14|    "cache_write_5m": 5,
   15|    "cache_write_1h": 8,
   16|    "cache_read": 0.2
   17|   },
   18|   "claude-fable-5-2": {
   19|    "input": 8,
   20|    "output": 40,
   21|    "cache_write_5m": 10,
   22|    "cache_write_1h": 16,
   23|    "cache_read": 0.2
   24|   },
   25|   "claude-fable-5-1": {
   26|    "input": 10,
   27|    "output": 50,
   28|    "cache_write_5m": 12.5,
   29|    "cache_write_1h": 20,
   30|    "cache_read": 0.25
   31|   },
   32|   "claude-sonnet-5": {
   33|    "input": 2,
   34|    "output": 10,
   35|    "cache_write_5m": 2.5,
   36|    "cache_write_1h": 4,
   37|    "cache_read": 0.2
   38|   },
   39|   "claude-opus-5": {
   40|    "input": 5,
   41|    "output": 25,
   42|    "cache_write_5m": 6.25,
   43|    "cache_write_1h": 10,
   44|    "cache_read": 0.5
   45|   },
   46|   "claude-opus-4-8": {
   47|    "input": 5,
   48|    "output": 25,
   49|    "cache_write_5m": 6.25,
   50|    "cache_write_1h": 10,
   51|    "cache_read": 0.5
   52|   }
   53|  },
   54|  "tier_multiplier": {
   55|   "default": 1.0
   56|  }
   57| }
===== END FILE producer/anthropic_api_rates.json =====

===== END OF REVIEW SET: 14 files =====

Reminder: the review set is complete only if every file has its END line and the END OF REVIEW SET line is present. Answer now, in this single response, following the output contract (Part 1 JSON, then Part 2 prose). No tool calls are available.
