# Claim-lineage backbone dogfood

This directory is an experimental, reversible realization of the bounded
two-fixture exercise selected in
`proposals/evidence-lineage/claim-lineage-backbone-dogfood/fixture-selection-plan.md`.
It is not a proposal packet, a production registry, or a permanent placement
decision. No file in this tree may be named `packet.json`.

`records/claim-lineage-records.json` is the canonical dogfood input. Git owns
its history. `generated/projection.json` and `evidence-report.md` are derived
outputs; they own no claim meaning, authority, causality, or completeness beyond
their declared inputs. Rebuild and verify them with:

```bash
python3 proposals/evidence-lineage/_dogfood/claim-lineage-backbone-dogfood/scripts/claim_lineage_dogfood.py verify --root proposals/evidence-lineage/_dogfood/claim-lineage-backbone-dogfood
```

The selection is purposive and its historical outcomes were known. A passing
run demonstrates bounded representability across two cases; it is not
outcome-independent falsification and does not accept the parent proposal.
