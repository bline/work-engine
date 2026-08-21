# Semantic judgment and authority

The deterministic CLI can locate candidate statements and verify explicit
identifiers and relations. It cannot decide whether a statement is an
invariant or route/default, identify its true causal parent, assign ownership or
authority, determine conditionality, or declare two concepts equivalent.

For those classifications, inspect the owning doctrine and role contracts,
apply the route-invariance and causal-failure tests in `DESIGN.md`, and record a
small judgment artifact. Each judgment needs a stable ID, one supported semantic
category, a subject, a conclusion, direct evidence references, producer
attribution, and whether accepting it changes a contract.

The supported categories are `invariant_status`, `causal_parent`, `ownership`,
`authority`, `conditionality`, and `equivalence`. Evidence references must point
to canonical repository sources; a rendered graph is not sufficient evidence.

If `changes_contract` is false, the judgment may guide analysis or rendering
without rewriting doctrine. If it is true, `human_approval` must contain an
approver and approval reference before the CLI will accept it. Approval of an
engineering plan is not automatically approval to change a product contract.

Example shape:

```json
{
  "schema_version": 1,
  "producer": "agent or human identity",
  "judgments": [
    {
      "id": "judgment-ownership-example",
      "category": "ownership",
      "subject": "artifact.example",
      "conclusion": "role.example owns its lifecycle",
      "evidence": ["docs/example.md#owning-contract"],
      "changes_contract": false,
      "human_approval": null
    }
  ]
}
```

Judgment artifacts are run evidence. Do not merge them into a new canonical
graph store or apply their conclusions to canonical inputs automatically.
