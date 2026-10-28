# Retained reviewer and active-slice binding

`<skill-dir>` is the slice-builder skill's directory.

Provision the same authority-bound review-episode state for either retained
review provider. Before the initial provider boundary, bind the exact subject,
writer generation, authority provenance, and resumable runtime reference. For
Codex, predeclare the canonical reviewer task target, bind it as provider
`codex`, and verify the target returned by `spawn_agent`; give that reviewer the
authority file and repository-local review-state adapter. For Claude, bind and
verify the assigned persistent session UUID and expose the same profile through
the configured MCP adapter. In either case the reviewer begins the episode,
publishes its attributed result, and reads it back before returning. Ordinary
remediation uses that same identity and episode. State availability or
persistence never upgrades Codex adversarial review into independent-review
evidence.

Give either reviewer the same bounded observational state needed by the review:
exact active-slice current or retained history and validated claim-lineage
projections when material. Provider transport may differ, but subject identity,
freshness, limits, and authority may not. Never substitute raw private-state
enumeration or unvalidated claim files for the bounded projection.

When active-slice recovery is configured and this builder launches the
reviewer, assign the provider's resumable runtime reference before the initial
review call and publish it through the active-slice mechanism as
`actor_binding.runtime_session_id` with `<skill-dir>/../slice-supervisor/scripts/manage_active_slice.py
bind-actor` (`<skill-dir>` is this skill's directory), passing the exact current durable revision as `--expected-revision`
and a stable unique `--event-id`. This CAS route prevents a stale writer or a
conflicting replay from silently binding the wrong reviewer session; omitting
it can make later recovery resume a lost or different reviewer continuation.
After builder or supervisor context
replacement, recover the binding with `<skill-dir>/../slice-supervisor/scripts/resume_active_slice.py` before issuing
the next reviewer call; resume that exact session rather than reconstructing
the review. If the binding cannot be recovered or the provider session is no
longer available, preserve the pending obligation and applicable findings,
record the replacement reason and provenance, and label the replacement as a
new reviewer rather than a continuation.

Command lines (generated from `--help`):

<!-- cli-synopsis script=skills/slice-supervisor/scripts/manage_active_slice.py subs="bind-actor publish-phase" display="python3 <skill-dir>/../slice-supervisor/scripts/manage_active_slice.py" -->
```text
# global options, given before the subcommand: --repository REPOSITORY
python3 <skill-dir>/../slice-supervisor/scripts/manage_active_slice.py bind-actor --identity-json IDENTITY_JSON --expected-revision EXPECTED_REVISION --event-id EVENT_ID --actor-binding-json ACTOR_BINDING_JSON
python3 <skill-dir>/../slice-supervisor/scripts/manage_active_slice.py publish-phase --identity-json IDENTITY_JSON --expected-revision EXPECTED_REVISION --phase PHASE --consequence-json CONSEQUENCE_JSON
# other subcommands (put --help after one for its usage): begin, wait, retire
```
<!-- /cli-synopsis -->

<!-- cli-synopsis script=skills/slice-supervisor/scripts/resume_active_slice.py display="python3 <skill-dir>/../slice-supervisor/scripts/resume_active_slice.py" -->
```text
python3 <skill-dir>/../slice-supervisor/scripts/resume_active_slice.py --repository REPOSITORY --identity-json IDENTITY_JSON [--capability CAPABILITY] [--recovery-event-id RECOVERY_EVENT_ID] [--revision REVISION] [--history-limit HISTORY_LIMIT] [--history-cursor HISTORY_CURSOR]
```
<!-- /cli-synopsis -->
