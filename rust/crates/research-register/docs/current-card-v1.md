# Current-use research cards

The M3b current-use interface composes the existing commit-pinned snapshot,
whole-register validator, M1 research-use join and bounded reader with in-process
anchors and a private admission verifier. A card carries attributed structural,
scientific and applicability records. It grants no execution permission.

`work-engine-research-card CGROUP_ROOT TRUSTED_PROFILE_JSON` reads one bounded
`CardRequest` on stdin. The request contains `source` (the existing ReadRequest),
`current` (decision ID, exact revision string and exact M1 request-byte SHA-256),
and `codec` (`research_a4_json_v1`). Duplicate/unknown fields refuse. The public
startup has **no production authority installation**. It therefore cannot deliver
a qualified card. There is no environment variable, request field, key, verifier
or repository file that enables production admission.

The `synthetic-card-host` Cargo feature enables an explicitly synthetic startup
adapter, `Installation::synthetic`, for qualification code. Its caller separately
supplies installed issuers, exact delegation/purpose, disclosure scope and two
saved receipt copies/pins. Request JSON cannot install these values. The private
verifier compares pinned receipt bytes to exact saved copies, issuer/delegation,
role, purpose, decision/revision, request/bundle digest, the complete record
revision set and disclosure scope. Both qualification and applicability owner
roles are required. Accepted purpose-specific qualification and manifest-selected
applicability remain distinct checks; M1 publisher labels cannot construct the
private `AdmittedUse` proof. Exact record revisions bind subject/study pins,
assessment facets/competition, qualification reviewer/exposure/capabilities and
applicability relation/bounds. All competing assessment bytes remain visible.

This adapter assumes its in-process startup caller and installed values are
trusted. Synthetic owner/delegation receipts are **not signatures or production
authentication**. The feature-enabled private worker accepts its synthetic startup
state over the existing parent-owned pipe, under the shared sealed-image Linux
host; running that worker directly can emit synthetic bytes and establishes no
production authority. A normal build refuses even supplied private-worker
installation data. No production protection/owner installation evidence has been
established. Installing and accepting that boundary is separate work.

The shared Linux enclosure retains M3a's CPU rlimit, cgroup-v2 memory/swap/pids,
independent wall deadline, cancellation, bounded control pipes and combined output,
sealed executable and complete-frame checks. It opens Git only after controls.
The parent checks producer, source/bundle/profile pins and decoded card outcome;
no card escapes crash, OOM, truncated/oversized output or cancellation. Diagnostics
and reconstruction consume the same aggregate output envelope. CPU limits have
whole-second kernel granularity; the cgroup bounds memory, not address space.
Unsupported/unavailable controls refuse. Control input is at most 16 KiB; profile
input at most 8 KiB. Deadline and cancellation cover public request acquisition.

Invalid register dominates projection (D-6). Complete anchor computation then
precedes M1 validation for refusal classification, under the root's narrow M3b
engineering disposition. A requested current null/missing/unestablished anchor
returns `anchor_gap`. M1 requires its request target to be the source decision's
blob, so a missing requested document cannot independently have a valid joined
bundle; this precedence preserves that distinction without changing M1. Another
decision's null does not suppress a requested established anchor. All anchors and
reads must finish; a global read/resource failure still refuses the batch. Every
successful card requires both the M1 proof and exact admission. Historical and
unknown revision requests never substitute current content.

Selection includes decision-listed findings, exact current recorded authorized
applicability and explicitly admitted companion subjects. Only correction edges
expand selection to a fixed point. Objections/events and every judgment retain
source order and attribution without importing the objection finding's content.
Disclosures partition revision before authorization; cross-decision records are
metadata only for delivered findings. No siblings or cross-decision traversal.
Insufficient mandatory disclosure scope refuses. Historical markers are copied
without inferring successors. Native claims are never consulted: only delivered
findings' materialized rows determine `empty` or `unavailable`.

`card_wire` owns the exact `research_a4_current_v1` shape and codec. Every field is
mandatory, null unions are explicit, unknown/duplicate fields refuse, object keys
sort by Unicode scalar value and output ends with one LF. The lossless legacy
parser preserves number spellings and copied array order. Named outer arrays use
the packet's ordering. Digests remain external. Read-scope identity hashes the
portable read profile, excluding the host repository pathname. No timestamps,
PIDs or absolute host paths occur in card bytes.

The parent returns a separate `Reconstruction` containing the source/request,
profile/producer, exact saved installation and every verified object payload used
by the bounded batch. Retain its external digest with the delivery. `reconstruct`
checks that digest and every Git object identity, rebuilds snapshot, validation,
M1, anchors, admission and projection in an isolated in-memory object reader, then
compares exact card bytes. It never opens a repository, URL or newer source.
Missing/substituted bytes refuse. The original bounded read's prefix candidates
were all read and retained, so replay uses the frozen observed object universe;
it does not claim present-day repository prefix uniqueness. The saved basis
contains whole-register evidence needed for validation and must be retained only
within the trusted read scope. Digest equality establishes reconstruction, not
scientific acceptance, authentication, currentness or permission.

Tests include the packet's unchanged three synthetic byte vectors, independent
frozen source inputs for all eleven current cases plus historical/unknown controls,
correction/objection/competition, exact notices, synthetic authority refusals,
requested versus unrelated anchor gaps, codec strictness, public input deadlines
and actual Linux enclosure/reconstruction. Generated semantic goldens and bases
are qualification artifacts bound to the exercised executable, rather than legacy
A4 byte-conformance fixtures. No live H2, native claims, task adoption or production
admission is exercised.

Default builds also refuse reconstruction of serialized synthetic installation state.
Only an explicit synthetic-feature build can reproduce these synthetic cards. A saved
basis digest cannot install production authority or authorize a new use; default-build
production reconstruction readiness is not established by this qualification.

The root's `planning/research-lifecycle-m3b-wire-disposition-20261112.md`
(SHA-256 ebba987931965ba0ca43bfe02d5fffa91043f7242294a92fd28ced0434795ad4)
corrects the prepublication E shape: mandatory `legacy_applicability` contains
canonical exact current authorized legacy rows for each delivered finding, sorted
by applicability ID. Membership `via` remains that stable ID. These rows retain
relation, rationale and recorded authorization as attributed historical register
data, never authenticated new-use permission. Pending/declined/other-revision rows
remain separately grouped. Original three vectors qualify generic encoding and
history only; corrected-shape examples and actual semantic goldens qualify the
current shape. No legacy byte-conformance claim is made.
