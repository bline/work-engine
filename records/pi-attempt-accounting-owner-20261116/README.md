# Pi attempt accounting: owner continuation, 2026-11-16

Standing: dated campaign record. Pi session `01a1e569-2805-72ff-b837-03d1577c405d` took
ownership of the S1 campaign of Codex session `01a1d6e2-32ba-7701-9115-740dcb41af11` by
operator direction (board `pi-accounting:s1`, seq 5773). The prior campaign record is
[pi-attempt-accounting-wip-20261115](../pi-attempt-accounting-wip-20261115/README.md).
Nothing here qualifies production use, S2a or integration beyond this repository copy.

## Operator decisions

- Budget ceiling 200 proxy dollars, including the predecessor's 68.53 (seq 5778;
  [BUDGET.json](BUDGET.json)).
- Fable at xhigh replaces Astra xhigh for required reviews; recorded as same-provider review
  (seq 5779, model corrected to `claude-sdk/claude-fable-5-1` in seq 5786).
- Campaign children use the `claude-sdk` provider only (the OAuth `anthropic` route bills
  outside the subscription quota).
- After measured costs showed full S1 cannot fit the ceiling, the operator chose to narrow
  the campaign to the eleven historical findings (option 3). Full S1 (175 cases, all
  applicable 374 scopes, complete exact-current A/B/C review) stays explicitly OPEN.

## Slice 1

- **1a, independent oracle, measured batch.** Eight whole cases and 26 scopes derived
  without supplied outputs: 128/153 claims nominal, 25 waiting on root, none unexamined, no
  D3 conflict. Four disagreements with supplied outputs, all under open findings.
  [Result](slice1/oracle-batch1/RESULT.md). Unreviewed derivation proposal.
- **Root decisions R1** ([decisions](slice1/ROOT-DECISIONS-R1.json),
  [admissions](slice1/ROOT-ADMISSIONS.json)): per-case declared initial reference scopes
  admitted as conditional fixtures only, when raw-consistent (six entries); VC08's
  admission-namespace scope not admitted; dossier record shape bound as
  `pi-accounting-oracle-dossier/1`.
- **1b, spec author.** R8-OWNER-01 and R8-SCHEMA-DOMAIN-01 fixed; accepted bounded after
  root-executed verification and a Fable xhigh review (ACCEPT, P3 only, same-provider,
  reviewer did not execute). [Acceptance](slice1/ROOT-ACCEPTANCE-SPEC-R1.json),
  [review](slice1/review-spec-r1/REVIEW.md). The change is applied to
  [experiments/pi-attempt-accounting/s1](../../experiments/pi-attempt-accounting/s1), which now
  equals private successor commit `2729107` (base: accepted manifest `5e594cac`).

## Slice 2 (eleven historical findings)

The oracle and spec authors fixed the remaining nine findings on separate branches; root joined
them, found and had remediated a join defect, then verified 18/18 gates with root admissions
bound from outside the subject and all 175 cases' raw inputs unchanged. Fable xhigh review
(same-provider, executed probes) returned ACCEPT_WITH_CHANGES (one P2 admission-authority gap,
four P3); after one bounded repair round the retained re-check returned ACCEPT (bounded).
[Acceptance](slice2/ROOT-ACCEPTANCE-SLICE2.json), [review](slice2/review-r2/REVIEW.md),
[root decisions](slice2/ROOT-DECISION-R3.json), [admissions](slice2/ROOT-ADMISSIONS.json).
The change is applied to `experiments/pi-attempt-accounting/s1`, which now equals private
successor commit `3883d04`.

All eleven historical findings are closed within that reviewed private subject. Explicit limits:
dollar_ticket scope admissions are not consumed by the actual check path; Rust `evidence` has
no admission input; 149 oracle bundles remain schema-stale; R8-COVERAGE-01's independent raw
membership derivation is not done. Full S1 (175 cases, all applicable 374 scopes, complete
exact-current A/B/C review) remains OPEN. Owner-tree spend 109.57 plus predecessor 68.53. The architecture guard as of this record is
[ARCHITECTURE-GUARD-20261116.md](ARCHITECTURE-GUARD-20261116.md); its history carries over
unchanged. Private working paths cited in these files are historical locations, not
repository paths.
