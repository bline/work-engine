# Native repository facts

`repository-fact-v1` preserves reusable assertions about one trusted stable
repository identity. A producer assigns an assertion key and a bounded assertion
scope; successor revisions preserve that identity and decision scope. The
`claim-evidence` Rust owner supplies revision identities, admission validation,
exact historical resolution, and producer-attributed reliance.

This is a native profile with its own closed startup and projection shapes. It
does not expose the filesystem adapter's full discovery filters, typed lineage,
refresh or reliance-retirement operation set. A repository fact is neither
a revision-bound review finding nor workflow approval.

## Read capabilities

The trusted launcher can run
`work-engine-server claims-stdio /absolute/read-profile.json`. This mode exposes
only repository reads and requires no session-query profile, command grant,
HTTP listener or transport credential. Several processes may use the same
private read profile. Tool arguments cannot select another root or scope.

- `discover_repository_facts` searches subject/proposition text and returns
  bounded candidate heads, with applicability `not_assessed`.
- `resolve_repository_fact` selects an exact claim ID, revision ID and revision
  SHA-256, including historical revisions and their admission provenance.
- When evidence custody is configured, `read_repository_evidence` reads an
  exact CBM capture, and `check_repository_evidence_currency` compares its
  relative source anchors with the configured checkout.

Owner projections preserve their native canonical JSON in `canonical_projection`.
Large projections use `canonical_projection_page`, advancing byte offsets, and
the SHA-256 of the complete projection. A fragment cannot establish the full
owner record; reliance on that record needs complete, digest-verified content. Reissuing
the same operation with `projection_page: {offset, max_bytes, sha256}` continues
that exact projection. Changed canonical inputs refuse stale continuation, so
restarting assembly may be necessary. Evidence artifact pages similarly retain
the full immutable artifact digest.

Projection input identity includes state, exact admissions and root/bootstrap
binding. Artifact integrity verifies captured bytes; source currency reports
`bytes_match`, `bytes_changed` or `unavailable` for selected anchors. Neither
decides proposition truth, support sufficiency or consumer applicability. CBM
documentation references identify review candidates; a mention does not prove
that code implements a sentence's meaning.

## Publication and reliance

Publication uses the separately trusted `work-engine-claims-host`, with an
externally selected startup file and expected digest. Its fixed repository,
actor, grant, scope and action set cannot be supplied by an operation payload.
The operator's authority and OS custody authenticate this arrangement; matching
hashes or an `owner` string alone do not authenticate a launcher.

The host supports `create_claim`, `publish_revision` and `record_reliance`.
Revision publication names an exact predecessor. Reliance names an exact
revision ID and SHA-256, consumer identity/revision and decision scope. The
consumer owns the judgment to rely; the host records the admitted producer's
attributed assertion, without authenticating direct consumer attestation.

Operation IDs bind exact requests. `inspect` reconciles an entered or unknown
outcome without publishing another request; automatically retrying an unknown
operation can misstate whether an effect occurred. A later manual revision
preserves the original record rather than rewriting it. There is no MCP write,
automatic source refresh, semantic invalidation or downstream reopening here.

The repository's `rust/crates/work-engine-claims-host/README.md` and
`rust/crates/work-engine-server/README.md` describe deployment and closed input
examples. `docs/repository-claims-operator-guide.md` describes the local
environment and available operator controls. These are repository references;
installed native binaries and startup files come from the trusted deployment.
