# Trusted repository-fact host

`work-engine-claims-host initialize|execute|inspect|read STARTUP EXPECTED_STARTUP_SHA256`

This is an out-of-band local producer, not an MCP write tool. Authenticate the
launcher and protect startup/config/source custody with deployment and OS controls.
A matching digest or an `owner` string alone does not authenticate authority.
The operator provisions the files and selects the hashes before process entry.
`initialize` is explicit and consumes no request; the other commands consume one
closed JSON value on stdin. No request may select a root, actor, grant, scope,
authority file or action set.

The startup v1 shape is `Startup` in `src/lib.rs`: absolute `root`, `config`,
`source`, expected config/source SHA-256, trusted `repository_id`,
`decision_scope`, `actor`, `grant_id`, and `allowed_actions`. Bootstrap config
and source use schema 3 and `repository-claims-v1`; their exact closed fields
are demonstrated by `Fixture::new` in `tests/repository_host.rs`. Grants use
`repository-fact-v1` and select create/publish/reliance permissions. Bootstrap
versions 1 and 2 remain restricted to native review profiles.

The v1 bootstrap grant set is immutable for this adapter. There is no hot grant
admission or revocation endpoint. Quiesce/replace the deployment to change its
trusted authority. Startup/source changes refuse later operations and are checked
again before commit and after commit while the producer lease remains held. The
lease serializes producers with `flock` on the root's writer-only
`repository-admission.lock`; reads never create or acquire it. File-system
custody remains required: the host does not attest an adversary's transient edits
that were restored between observations.

`execute` and `inspect` accept the same canonical operation shape:

```json
{
  "schema_version": 1,
  "operation_id": "producer-assigned-operation",
  "action": "create_claim",
  "profile": "repository-fact-v1",
  "expected_state": null,
  "payload": {
    "subject": {
      "namespace": "repository-fact:trusted-repository-id",
      "subject_kind": "repository_fact",
      "stable_subject_id": "producer-assigned-assertion-key",
      "evidence_baseline": {
        "owner": "capture-owner", "reference": "immutable-capture-ref",
        "revision": "immutable-capture-revision",
        "integrity_sha256": "0000000000000000000000000000000000000000000000000000000000000000",
        "freshness": "exact captured bytes", "status": "unavailable"
      },
      "content_set": ["src/example.rs"]
    },
    "statement_identity": "producer-assigned-assertion-key",
    "initial_revision": {
      "proposition": "Attributed bounded assertion",
      "support_qualification": "Producer judgment",
      "assumptions": [], "limitations": ["Unobserved dependencies remain unknown"],
      "confidence": {"label": "bounded"},
      "evidence_references": [], "sensitivity_references": [],
      "evidence_mode": "capture", "judgment_kind": "repository_fact",
      "decision_scope": "startup-selected-scope",
      "profile_payload": {
        "repository_id": "trusted-repository-id",
        "assertion_key": "producer-assigned-assertion-key",
        "assertion_scope": "Immutable bounded assertion scope"
      },
      "reopening_conditions": ["New evidence"], "tombstone": false
    }
  }
}
```

`publish_revision` uses an exact revision ID in `expected_state` and payload
`{"claim_id": "...", "revision": {...}}`. Repository/key/assertion scope are
immutable. One concurrent successor wins. The same operation and admitted payload
replay even after later revisions; changed payload, predecessor or authority
conflicts. The existing claims codec owns all identities and hashes. Repository requests
are bounded to 1 MiB and reserve eight codec nesting levels for canonical and
projection envelopes before a write can enter.

`record_reliance` has null `expected_state` and payload fields `consumer`,
`consumer_revision`, `decision_scope`, `claim_revision_id`,
`claim_revision_sha256`, `state: "active"`, `predecessor_reliance: null`.
Historical exact revisions are valid reliance targets. The grant actor is the
attributed producer of this record, not a direct consumer attestation.

Successful execution returns status `applied` or `replayed`, an exact locator,
and `result: {record, sha256}`. Once an operation enters, its locator and status
are persisted in the canonical owner's SQLite database, including conflict events.
An entered/unknown operation is inspected/reconciled without publishing again.
`inspect` returns committed readback or reconciled no-effect canonical absence;
no effect is not permission to invent a new payload under the same operation ID.
Do not automatically retry `outcome_unknown`. Busy admission can explicitly fail.
The producer may recover a rollback journal on reopen; the reader refuses recovery.

# Read composition

The shared Rust server composes only `claim_evidence::RepositoryReadStartup` and
`RepositoryReader`. `OpenFiles` binds root and bootstrap hashes, while startup
selects repository and read decision scope. It requires neither a publication
grant selection nor a producer/campaign admission port.

* `RepositoryReader::open(startup)` checks pinned bootstrap/profile/root marker,
  database header, unsupported sidecars and root binding. Only repository schema 3
  with the versioned DELETE profile is supported. Review/WAL roots are rejected.
* `resolve(claim_id, revision_id, revision_sha256)` returns the exact historical
  record and authority, current heads, evidence-reference states, producer-attributed
  reliances and canonical projection provenance. It never follows a newer head.
* `discover(query, limit, cursor)` accepts 1–100 results per page, uses a bounded
  literal query over subject/proposition, and returns all matching heads as
  candidates. Cursor JSON binds query/scope/limit and canonical state digest;
  changed state rejects continuation rather than silently mixing publications.

The DTO has schema 1, kind `repository_fact_projection_v1`, `items`,
`next_cursor`, `repository_id`, `decision_scope`, and projection
`canonical_input_sha256` (state plus exact admissions and bootstrap/root binding),
`canonical_state_sha256`, bootstrap digests, root ID, `store_revision`, `request_sha256`, `build_version`,
`freshness` and `completeness`. Applicability and source currency are
`not_assessed`. Canonical projection freshness does not establish proposition
truth, source currency or support sufficiency. Immutable capture artifact
verification is distinct from validating its proposition. Cross-repository
subjects/coherent captures, automatic refresh/lineage and equivalence judgments
are outside this profile; generic evidence references preserve their own owners.

The reader uses a read-only coherent SQLite transaction and performs no
initialization, migrations, chmod, sidecar/lock creation or persistent projection
repair. Read failures, busy states and recovery refusal are explicit. The actual
process tests preserve names, bytes, inode identities and modes on successful,
missing, invalid, WAL/recovery and readable-permission paths, and cover concurrent
publication and crash reconciliation. They use synthetic temporary roots only.
