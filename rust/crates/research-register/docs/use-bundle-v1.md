# Research-use companion v1

`use_bundle::validate_use_bundle(&PinnedSnapshot, &ValidRegister, BlobPin)` reads
an upstream companion from the source repository's verified Git objects. It
returns a privately constructed `ValidUseBundle`. No decoder, DTO, owner name,
qualification label or proof from this module is an execution permit. This
crate authenticates neither the publisher nor a purported research acceptance,
independent review, owner approval or capability grant. A later trusted owner
must establish those claims for its consumption boundary. Structural validity
alone is insufficient to admit a task or claim scientific qualification.

The source snapshot and register proof must agree in commit, register path and
register tree. The manifest names that same source. The manifest itself is a
separate `BlobPin` supplied by the reader: full lowercase 40-hex commit and blob,
relative repository path and lowercase 64-hex SHA-256. It never embeds its own
commit or digest. Input paths are relative to its containing directory, all
read at its commit. Every input has an exact blob and SHA-256. The manifest must
be published after all inputs exist. A missing input yields no partial proof.
No file is read from the working tree and links/gitlinks/directories are not
followed as files. Repository/object-store failures remain `Unreadable`; an
absent path or link target is `Missing`; invalid data/pins are `Invalid`;
unknown schemas and noncurrent decision requests are `Unsupported`.

Every JSON document has `"schema":"research_use_v1"`. Revisions are SHA-256 of
exact stored UTF-8 JSON bytes, including any newline. Field order and whitespace
changes create different revisions. Validation never reserializes input to
compute its identity. All nested duplicate keys and unknown fields are rejected.
This codec does not apply to legacy register/claim/coordination identities.

A manifest has this shape (angle-bracket values below are explanatory, not
accepted hashes):

```json
{
  "schema": "research_use_v1",
  "source": {"commit": "<source commit>", "register_path": "metrics/register", "register_tree": "<register tree>"},
  "inputs": [{"path": "subject.json", "blob": "<blob>", "sha256": "<record revision>"}],
  "request": "<request revision>",
  "applicability": ["<applicability revision>"]
}
```

The complete executable synthetic fixture is `tests/use_bundle.rs`; its
publication builds full actual hashes from a temporary repository. The
abbreviated illustration above needs all required records before it can pass.
Every input is a `RecordDocument`, `{"schema":"research_use_v1","record":...}`.
`record.kind` selects the following closed variants; exact field types are the
public Rust DTOs in `src/use_bundle.rs`:

| Kind | Meaning and required bindings |
| --- | --- |
| `subject` | Registered full finding ID and exact source study `BlobPin`, plus evidence revisions. A study edit changes this subject even if its finding ID stays constant. |
| `evidence` | `source.kind=git` with a verified `pin`, or `assertion` with `locator`, optional `asserted_sha256` and mandatory `limitation`. An off-repository assertion is never fetched or verified. |
| `assessment` | Subject revision, assessor/as-of context/scope, limitations, evidence, eight facets, predecessor/correction/competition revisions and qualification references. Multiple conflicting assessments remain visible. |
| `qualification` | Attributed assessor, purpose and disposition; asserted reviewer/exposure boundary; capability, authorization and other evidence revisions. These are locators/assertions, not authenticated acceptance. |
| `request` | Request ID, registered decision ID and exact revision spelling, target source `BlobPin` and selector, purpose, alternatives, conditions, operating context, owner and acceptance requirements. Historical/different decision revisions are unsupported. |
| `applicability` | Exact subject and request-byte revision, matching decision/revision, relation, bounds, owner and authorization evidence revisions. No inheritance to another subject/request/revision. |

For example, an assertion evidence document is:

```json
{"schema":"research_use_v1","record":{"kind":"evidence","source":{"kind":"assertion","locator":"synthetic://owner-decision","asserted_sha256":null,"limitation":"Synthetic unauthenticated publisher assertion; establishes no grant."}}}
```

The eight separate facets are `provenance`, `method_reliability`,
`reproducibility`, `completeness`, `consistency`, `temporal_relevance`,
`context_transfer` and `inferential_strength`. Each has `state`, `reason` and
`evidence` revisions. `not_assessed` permits an empty evidence list; assessed
`unknown` requires evidence just like `supported`, `limited`, `contested` and
`unsupported`. There is no score, age penalty, currentness clock or winner
selection. Applicability relations are `supports`, `limited_support`,
`contradicts` and `not_established`. An asserted owner authorization does not
turn `not_established` into support.

Required links must exist with the right kind. Assessment history links stay
within the exact subject; required predecessor/correction edges must be acyclic.
Competition edges can be reciprocal. Every selected applicability has an
assessment, and all manifest applicability records bind its selected request.
The proof retains original manifest and record bytes, input pins, and
`source_files` with exact path/pin metadata and `retained_git_bytes` keyed by
exact byte digest for source register files,
study/target files and verified Git evidence. A caller can reconstruct those
inputs without a newer URL or worktree version. Missing historical bytes make
the read unavailable. This module introduces no archival/garbage-collection
policy and makes no claim about completeness of evidence outside the manifest.

`read_passive(objects, bundle, pin)` reads later `PassiveDocument` records:

- `prepared_basis` refers backward to the exact bundle pin, request, selected
  assessments/applicability and existing card, delivery, recommendation,
  context-check and policy pins. Policy equals the request target; selected
  assessments cover the applicability subjects.
- `adoption` refers to an existing prepared basis, preparation revision,
  source execution ID, admission and grant pins.
- `feedback` refers to an existing adoption, the same execution ID,
  verification pin, costs, failures, unknowns and candidate status.

These are passive assertions, including all references to admission/grants.
Opaque consumer artifact pins establish exact byte availability only. They do
not establish actual delivery, currentness, admission, independent verification,
settlement or success. `ReadPassive` retains all bytes it reads; keep the
associated `ValidUseBundle` for the original upstream inputs. Neither passive
records nor their digests are upstream prerequisites, so publishing them does
not alter the earlier manifest. An upstream manifest rejects downstream kinds.
Two feedback records with the same execution ID retain that one source identity;
no independent replication or deduplication publication is inferred. Actual
admission serialization, trusted currentness and feedback import are later
owner work, outside this API.
