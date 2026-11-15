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

{NFILES} files follow this brief. Each sits between
`===== BEGIN FILE <path> | sha256 <digest> | <bytes> bytes | <lines> lines | encoding <numbered|verbatim> =====` and
`===== END FILE <path> =====`. The set ends with `===== END OF REVIEW SET: <n> files =====` (n = {NFILES}). If any END
line or the final line is missing, say so and return `INCOMPLETE`.

Numbered files show `NNNNN| ` before each line; cite them as `path:line`. Excerpt files carry the producer's original
line numbers; cite those. Cite a corpus case as `case:<case_id>`. Labels such as `R5-01` and `D4` are the author's
identifiers for rules and decisions; they assert nothing.

Files:

{INDEX}

**Authority of the files:**
- **Normative:** `contract/CONTRACT.md` (`1.4-s1`) together with `TRANSITIONS.md`, `INVARIANTS.md`, `MONEY.md` and
  `RESERVATION-BOUNDS.md`. Your scope is §1–§9 and §13 (stages E to K), plus how §2.3, §12.3 and §12.4 use money.
- **Owner decisions:** `contract/DECISION-REGISTER.md`. Treat them as proposals to challenge. Report any that are
  unsound, inconsistent with each other or with the producer, or whose consequence the contract states wrongly.
- **Producer facts:** the excerpts of `pi_container.py`, `role_host.py` and `scope_budget.py` (deployed release
  `92a03789`) and the rate files are authoritative. Where the contract and the producer disagree, say so.
- **Corpus:** `corpus/sample-cases.jsonl` holds {NSAMPLE} of the {NCASES} cases, with author narrative and labels
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

{DERIVE}

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
