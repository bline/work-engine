# Review host job admission runtime entry — 2026-11-06

The Rust review host accepts a private `--job-request` alongside `--startup`.
Initialization derives admission through the fixed installed `JobReviewPolicy`;
serving reopens the exact existing admission and revalidates the current policy
before reconstructing `TrustedStartup::load_for_job`. The request cannot select
policy paths, grants, approved flags, or a serialized admission capability.
Without a request the historical startup route remains in use.

Uncertain admission publication returns `admission_unknown` with the exact
admission path for reconciliation. Serving cannot create missing authority.
Only a `test-support` build accepts the explicit fixture-policy selector; that
standing refuses qualified provider execution. Same-UID fixtures demonstrate
composition and refusal, not production operator/agent isolation.

The candidate boundary recognizes the current CBM `unresolved_calls` status as
incomplete coverage with an exact source hash and `direct_source_required`
disposition. It retains gap metadata and keeps `coverage_complete=false`.
Unrecognized statuses remain refused. Historical complete-coverage fixture
expectations are unchanged.

## Validation and operational status

Actual executable initialization and its idempotent replay succeeded with the
complete fixture. Complete serve/inspect/cold-reopen qualification remains
pending in private custody. The first focused run exposed the existing CBM
status incompatibility, now covered by a passing owner test for exact fallback
and unknown-status refusal. A subsequent run exposed a synthetic jobsRoot mode
mismatch, corrected to0700. The next run initialized twice but reached the
generic fixture's30-second serving deadline; the job fixture now uses the
existing qualified-fixture readiness bound. These failed runs are preserved.

Production compilation and both production parser tests passed. Scoped lint
passed with the coverage repair. The older controlled-host suite exceeded its
180-second custody bound in its first case; no passing full-suite claim is
made. The final focused run and any remaining limitations are recorded outside
this immutable source candidate in the private custody/result packet.

No protected installation, production activation, provider call, or real
credential read is authorized by this source change. A fixture policy or config
must never become production installation material. The prior selected Claude
executable is absent and the CBM digest changed; current executable presence
and metadata do not establish qualification. Genuine program/profile/catalog,
current native boundary, credential eligibility, exact fresh job inputs, and
operator installation decisions remain with their existing owners.

Private custody, complete failed runs, required production-input manifest and
confined installation/readback skeleton are under
`/home/bline/.local/state/work-engine/rust-loop-runtime-entry-20261106T1521/runtime-builder/`.
Source review, source acceptance, operational admission, review acceptance and
publication remain separate decisions owned by Root and the operator.
