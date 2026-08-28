# Structural Core Ownership

**Status:** S3 implementation boundary; canonical ownership has not migrated

The skill compiler and Agent Environment Graph share one structural vocabulary
and closure implementation. The Python Agent Environment Graph implementation
owns role/environment validation and lossless role closure. The App Server
compiler consumes that result through a bounded machine projection; it does not
reimplement graph semantics.

## Concept owners

| Concept | Owner | Non-owner projections |
|---|---|---|
| Skill sections, section kinds, source spans, frontmatter, and closed source bindings that distinguish canonical authority from generated evidence | Skill compiler schema v1 | Generated `SKILL.md` and ephemeral skill IR |
| Interface boundaries and enforcement primitives | Skill compiler schema v1 | Ephemeral skill IR |
| Role identity, label, objective, context lifetime, relation names, and relation entry shapes | Agent Environment Graph validator | Skill-local experimental role profile and generated views |
| Entity namespaces and records | `docs/agent-environments.yaml`, validated by the Agent Environment Graph | Role-scoped projections |
| Invariant and mechanism records | `docs/workflow-invariants.md`, parsed and validated by the Agent Environment Graph | Role-scoped projections |
| Reference validity, direct invariant membership, transitive invariant closure, referenced entities and roles, and mechanism closure | Agent Environment Graph structural core | App Server compiler IR and rendered graph views |
| Projection source hashes, complete projection shape, and deterministic ordering | Agent Environment Graph structural core | Committed YAML comparison oracle |
| Complete skill IR composition | Skill compiler | Renderer inputs; the IR is generated and non-canonical |
| Semantic classification and contract-change approval | Canonical doctrine and the owning human authority | Judgment artifacts are evidence only |

The machine boundary accepts one complete candidate role record. It first loads
and validates both canonical inputs, then requires the candidate to equal the
named canonical role exactly. Only after equality is established does it place
the candidate in an isolated in-memory copy and compute the existing closure.
It cannot write canonical inputs or generated views and does not call advisory
analysis.

The returned versioned envelope binds the backend identity and script digest,
records canonical equality, and carries the complete projection. The App Server
adapter bounds execution time and output size, rejects stderr or malformed and
provenance-mismatched results, and fails compilation when the complete projection
differs from the pinned generated oracle.

## Canonical transition

Before an authorized ownership migration, these remain canonical:

- `skills/slice-builder/SKILL.md` for the legacy skill and role contract;
- `docs/workflow-invariants.md` for shared invariant and mechanism truth; and
- `docs/agent-environments.yaml` for the role environment.

The decomposed slice-builder sources remain experimental. Their complete role
profile must equal the central canonical role; successful compilation and parity
do not transfer authority.

After a separately accepted migration, a skill-local role profile may become its
semantic owner and the central environment may become a generated aggregate.
That transition requires an explicit migration record naming the old and new
owners, exact revisions, accepting authority, consumers, regeneration behavior,
and rollback expectations. S3 does not perform or authorize that transition.

Generated Markdown, generated YAML, compiler IR, analysis output, and judgment
artifacts remain subordinate projections or evidence. None is canonical doctrine.
