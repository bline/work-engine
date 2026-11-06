# Fresh job review admission

This additive source API separates reusable installed delegation from exact
per-job admission. Historical fixed deployment acceptance remains supported.
This change does not install policy or activate a provider.

`JobReviewPolicy::open_installed()` opens only the fixed protected path
`/etc/work-engine/review-host/job-policy.json`, using existing root-owned file
and ancestry checks. Version-1 policy fixes delegated UID, repository, private
jobs parent, run/plan/slice/obligation, accepted objective boundary, required claim
contracts, Claude profile/program/guard/model/catalog, credential metadata, prompt
template, resource policy and horizon. Required `startupAuthority` fixes the exact
claims source and configuration byte hashes plus the completion consumer. Those
bytes bind source identity, claims root, production/completion grants and their
decision scopes; private descriptors cannot select substitute bootstrap authority.
Its `executionTemplate` uses the existing
exact acceptance schema. Subject/session/attempt and candidate/index/profile
digests vary per job. Current active unexpired policy is required at admission,
reopen and entry checks. Policy replacement, removal, expiry or revocation fences
subsequent checks; it does not establish cancellation of already entered work.

`JobReviewRequest` has no approval, authority path, model, program, policy choice
or output directory. `directory_name()` derives one location from complete
campaign attempt identity. The delegated owner creates that private directory
under the policy jobs parent. Its `campaign` child is the only permitted campaign
root. A new caller label cannot relocate a spent dispatch.

`policy.admit(request)` exclusively publishes `review-admission.json` and returns
Committed, Replayed or OutcomeUnknown. Pending staging is retained as custody,
not an execution/retry token. Uncertain publication reconciles only by exact
final `policy.reopen(request)` readback. Reopen reads current policy and derives
the entire admission again: presence, plausible hash or same UID grants nothing.

The returned `InstalledAcceptancePin` feeds
`TrustedConfig::qualified_initial_v4` and
`Campaign::install_qualified_initial_authority`. Consumers check exact paths and
current derivation. Installation and entry compare the complete admitted
CandidateRef with the selected campaign and dispatched request, including receipt
and physical-profile artifact hashes. Existing campaign prepare/dispatch owns one-shot
`FreshInitialEntry`; recovery cannot recreate it.
`QualifiedWriter::execute_applied_initial` revalidates admission before consuming
entry and checks its actual store root against the admitted evidence child.
This execution-owner check also applies to public composition without workflow
startup. Credential, isolation, custody, response and claims contracts retain their
existing owners. Admission permits an attempt; it does not imply provider success,
independent-review establishment, review acceptance or publication.

`TrustedStartup::load_for_job(path, pin)` composes a canonical read-only private
descriptor with the independently obtained admission and otherwise preserves
qualified startup checks. Workflow revalidation reopens the same admission.
Wire commands cannot choose this constructor or provide policy paths/pins.
Historical `TrustedStartup::load` still requires deployment-owned qualified
startup. Policy pins execution/profile/prompt; a descriptor cannot widen them.

With `test-support`, `open_fixture(path)` supplies explicit fixture authority.
Admissions carry `standing: fixture`. Qualified provider execution refuses it.
`QualifiedWriter::check_fixture_job_entry` consumes its entry through the same
exact binding comparison against an offline fixture scope, without launching a
process, reading tokens or publishing execution evidence. Same-UID fixtures do
not establish production operator/agent isolation. Existing OfflineInitialEntry
cannot become installed authority.

Production use still needs independently authorized protected policy installation,
operator/agent boundary qualification, exact fresh job inputs, actual review and
checked restart/recovery. This source/fixture deliverable establishes none of
those operational results.

The installed `reviewSelectionTemplate` fixes claim propositions, covered state,
acceptance and realization/capability requirements. The selection owner derives
fresh subject/episode/consumer claim IDs with its existing codec, then validates
the complete selection. Job requests cannot supply a weaker selection or choose
claim IDs. This avoids re-installing policy merely because a candidate changed.

Job startup and all writable owner roots are confined to exact children of the
admitted attempt directory: startup.json, workspace, campaign, episode, claims
and evidence. A private descriptor cannot redirect writes into another job.
