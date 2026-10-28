# Active slice recovery commands and recovery rules

`<skill-dir>` is the slice-supervisor skill's directory.

Command lines (generated from `--help`):

<!-- cli-synopsis script=skills/slice-supervisor/scripts/manage_active_slice.py display="python3 <skill-dir>/scripts/manage_active_slice.py" -->
```text
# global options, given before the subcommand: --repository REPOSITORY
python3 <skill-dir>/scripts/manage_active_slice.py begin --identity-json IDENTITY_JSON --actor-binding-json ACTOR_BINDING_JSON --phase PHASE --obligation-json OBLIGATION_JSON [--authoritative-refs-json AUTHORITATIVE_REFS_JSON] [--accepted-boundary-json ACCEPTED_BOUNDARY_JSON]
python3 <skill-dir>/scripts/manage_active_slice.py publish-phase --identity-json IDENTITY_JSON --expected-revision EXPECTED_REVISION --phase PHASE --consequence-json CONSEQUENCE_JSON
python3 <skill-dir>/scripts/manage_active_slice.py bind-actor --identity-json IDENTITY_JSON --expected-revision EXPECTED_REVISION --event-id EVENT_ID --actor-binding-json ACTOR_BINDING_JSON
python3 <skill-dir>/scripts/manage_active_slice.py wait --identity-json IDENTITY_JSON --expected-revision EXPECTED_REVISION --event-id EVENT_ID --capability CAPABILITY [--provider PROVIDER] --reason REASON
python3 <skill-dir>/scripts/manage_active_slice.py retire --identity-json IDENTITY_JSON --expected-revision EXPECTED_REVISION --event-id EVENT_ID --outcome OUTCOME --reason REASON
```
<!-- /cli-synopsis -->

<!-- cli-synopsis script=skills/slice-supervisor/scripts/resume_active_slice.py display="python3 <skill-dir>/scripts/resume_active_slice.py" -->
```text
python3 <skill-dir>/scripts/resume_active_slice.py --repository REPOSITORY --identity-json IDENTITY_JSON [--capability CAPABILITY] [--recovery-event-id RECOVERY_EVENT_ID] [--revision REVISION] [--history-limit HISTORY_LIMIT] [--history-cursor HISTORY_CURSOR]
```
<!-- /cli-synopsis -->

When recovered evidence establishes that an admitted attempt cannot continue
but its accepted work must be replanned in the same workspace, use the
host-owned lifecycle supersession control. Supersession preserves the stopped
attempt's candidate, review, finding, and authority evidence in its
attempt-local receipt while atomically transferring mutable-workspace admission
to a distinct successor at a new accepted boundary. It is not a backward phase
transition, named-campaign terminal-history append, review acceptance, finding
evaluation, provider retry, or publication authority. An already-reported
zero-finding episode may reconcile a stranded empty binding during that atomic
transition without synthesizing a finding or reliance; other review obligations
retain their exact state. Before continuing the successor, establish the exact
predecessor/successor supersession relationship from the atomic response or
recovery evidence. If the response is uncertain or lost, recover both attempts
and reconcile their relationship before continuing.

When the pending obligation is conclusively discharged, or the attempt reaches
an authoritative terminal outcome already decided by its owner, record
lifecycle closure with `<skill-dir>/scripts/manage_active_slice.py retire`. Terminal
outcomes include, without being limited to, accepted, stopped, and failed. The
CLI records decisions already made by the model-driven supervisor; it does not
decide discharge, acceptance, stopping, failure, provider choice, retry,
scheduling, or authority. The slice workflow alone authorizes and interprets
transitions; the shared durable-state primitive only publishes opaque revisions
atomically.

Preserve stable run, slice, attempt, and plan identity, handled consequence IDs,
reference-only links to stronger artifacts, and the logical actor binding.
Provider session identity is optional provenance. Capability waiting is
nonterminal, while retirement prevents stale input from resurrecting the
attempt. This path is distinct from `resume_campaign.py`; terminal-history
semantics remain unchanged.

Implementation and gate completion can cross a builder return or mailbox
boundary before the enclosing slice is terminal. Before that transient channel
can become the only copy, require the builder-side workflow to publish the
compact phase consequence with `<skill-dir>/scripts/manage_active_slice.py publish-phase`.
Bind established consequences to integrity-identified implementation, artifact,
or gate evidence. If interruption prevents completion from being established,
publish an explicit uncertain consequence when recovery can do so safely; never
infer completion from runtime liveness or remembered delivery.

`<skill-dir>/scripts/resume_active_slice.py` returns the supervisor's compact recovery
projection: stable attempt identity, current phase, accepted boundary, pending
obligation, latest phase consequence, authoritative references, lifecycle
state, and durable provenance. Its `runtime_binding` is explicitly nonsemantic.
The projection is not a general interface to model context, transcripts, hidden
reasoning, or worker control-plane state. Continue only from the recovered
semantic consequence and its integrity-bound references.

For a review provider that supports resumable sessions, assign the reviewer
runtime session ID before the initial provider call and publish it in
`actor_binding.runtime_session_id` through `<skill-dir>/scripts/manage_active_slice.py
bind-actor`, with the current durable revision as `--expected-revision` and a
stable unique `--event-id`. The compare-and-swap and replay identity prevent a
stale transition from replacing the intended reviewer binding; skipping this
route can make recovery lose the session or resume the wrong reviewer
continuation. Require the builder to use that exact
binding for the fresh-entry review and every ordinary remediation continuation.
After supervisor-context replacement, recover the binding with
`<skill-dir>/scripts/resume_active_slice.py` and resume the provider session instead of recreating
the review research. The reviewer remains a runtime binding, not the durable
owner: if it is unavailable or model judgment requires a fresh perspective,
record replacement provenance and the reset reason while preserving the same
pending obligation and applicable findings.

Active-slice recovery and publication are owned only by these slice-supervisor
scripts; do not infer a copy under another skill. Every `--identity-json` value
must be an object with exactly `run_id`, `slice_number`, `attempt_id`, and
`plan_version`; camelCase keys and inferred alternate shapes are invalid.
