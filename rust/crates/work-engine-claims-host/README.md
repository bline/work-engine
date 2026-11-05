# Trusted repository-fact host

`work-engine-claims-host initialize|execute|inspect|read STARTUP EXPECTED_STARTUP_SHA256`

`work-engine-claims-host reassess STARTUP EXPECTED_STARTUP_SHA256 --binding CAPTURE_BINDING --artifacts ARTIFACT_ROOT < REQUEST_JSON`

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
are demonstrated by the [test fixture](tests/repository_host.rs). Grants use
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

# Reassessment worklist

`reassess` is an out-of-band observation command, not a producer: it opens the
`RepositoryReader` only, takes no admission lock and writes nothing. The startup
selects root, repository and scope exactly as for `read`. `--binding` is the
`repository-evidence-cbm` `Binding` file; only its `repository_id` (which must
equal the startup's) and checkout path are used, its SHA-256 is echoed in the
output, and nothing is executed. `--artifacts` is an existing canonical artifact
root, read through `read_artifact` under the stable repository ID, so a renamed
CBM project does not change what a historical capture may be read for.

The request is closed, duplicate-key-free and at most 1 MiB: `schema_version: 1`
and 1–16 `selections`, each an exact `claim_id`, `revision_id` and
`revision_sha256`, with an optional exact `new_capture_sha256` naming a
separately captured artifact. Each selection is resolved with
`RepositoryReader::resolve`; no newest head is chosen, and the exact revision's
current-head flag, head revision IDs, decision scope, subject and projection
identity are returned. A malformed request, startup, binding or artifact root
rejects the whole batch with exit status 1. Anything else produces one canonical
JSON worklist (`repository_reassessment_worklist_v1`, exit 0) in which an
unresolvable selection or evidence reference is a `blocked` item and
`completeness.complete` is false. `completeness` counts the explicit selections
only; it says nothing about other claims. The output is the reviewable record;
there is no queue or journal.

Every material reference location of the exact revision is classified and
assessed, none narrowed to the first. The same three locations the canonical
projection checks are covered, each with its own role, and every finding, view and
accounting entry carries its `location` (`field`, `index`, `role`):

| field | role | meaning |
| --- | --- | --- |
| `revision.evidence_references[i]` | `selected_evidence` | ordinary current support |
| `revision.sensitivity_references[i]` | `current_sensitivity` | current support that the judgment is sensitive to |
| `subject.evidence_baseline` | `historical_baseline` | the claim subject's immutable baseline, inherited unchanged by every successor |

Current support (the first two roles) nominates as described below. The subject
baseline is history: a valid baseline whose anchored bytes later changed or went
missing is reported in `historical_baseline` and `historical_observations` and
does not nominate, so a successor whose current evidence was refreshed is not
re-nominated for ever by an anchor change in the original baseline. A baseline
that is not a verified, readable, same-repository capture still blocks the item,
exactly as for current support, so an unresolved baseline cannot yield a complete
assessment. Nothing here decides that the baseline is or is not still relevant.

A `repository-evidence-cbm` reference at any location must be well formed and
`verified`; another owner, a malformed or non-object reference, a reference whose
status is not `verified`, or a missing, foreign (`repository_id` mismatch),
digest-corrupt or unreadable artifact blocks the item with a specific code at that
location. An unreadable `evidence_references` or `sensitivity_references` field, or
an absent baseline, is itself a blocked entry. `reference_accounting` lists every
location with `outcome` (`assessed` or `blocked`) and `blocked_codes`, plus the
declared counts, so an omitted location would show as a count mismatch; the
top-level `completeness.reference_locations` sums them. A capture that appears at
several locations is read from the artifact root once but classified, assessed and
reported at each location with that location's role. `new_capture` is compared only
with `selected_evidence` captures. For each readable capture the host reports:

* anchor currency from `source_currency` against the binding checkout:
  `anchor_bytes_changed` or `anchor_bytes_unavailable` nominates review for current
  support. This fires on a code-only change even when the capture has zero
  documentation rows;
* documentation coverage (`pagination`, rows) and the rows whose `document` is
  one of that capture's anchored paths, with capture, observation and page-cursor
  provenance. An associated row with status `missing`, `unresolved`, `unavailable`
  or `ambiguous` nominates (`documentation_references_unresolved`); `resolved`
  does not; any other status is an uncertainty. Rows for other documents are only
  counted, so an unrelated broken document does not nominate every claim, and rows
  with no document path stay visible as an unresolved association;
* partial or not-requested documentation coverage as an uncertainty, never as
  clear.

For the historical baseline the same observations are made but listed under
`historical_observations` (with the same codes and `location.role`
`historical_baseline`) instead of `reasons` or `uncertainties`.

A supplied new capture is read the same way and reported with its own source
currency. `new_capture_differs` (with changed/added/removed anchor paths),
`new_capture_documentation_incomplete` and
`new_capture_unresolved_documentation_references` nominate; a new capture whose
anchors no longer match the checkout adds `new_capture_not_current`, so a stale
capture is not shown as current proof.

`disposition` is `blocked`, `review_nominated`, `uncertain` or
`no_nomination_observed`. The last means no listed reason applied to the bounded
observations; it is not a semantic all-clear. `bytes_match` and resolved links
are not semantic truth, missing references are navigation evidence rather than
proof of absence, indexed documentation rows are not a complete reference graph,
and applicability stays `not_assessed`. Source currency compares working-tree
bytes, not a commit. The adapter exposes documentation status but not its rows,
so the host reads rows back out of the verified artifact's observations; an
unparseable observation is reported as uncertainty.
