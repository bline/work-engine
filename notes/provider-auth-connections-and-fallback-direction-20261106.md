# Named provider connections and configurable auth fallback

Recorded 2026-11-06 UTC from the user's direction in session
`01a1a7ed-45f5-7041-9db5-a6c9aa1cd3f3`, following the bounded Sol preparation
round at `1a4888453a2de9c24cc4c75329a54fe88c2d3c80`.

Status: attributed future product direction and candidate design implications.
The Web interfaces described below are future work. This note grants no
implementation, provider execution, credential access, paid fallback or rollout
authority. The preceding $45 qualification round is closed.

## User's source statement

> With Auth, I think we should have several options. I think we should be able to support changing the auth option from api key to oauth. I think we might be able to supply an interface on the web side that allows us to connect different auth methods, give them a name, and select them later (these interfaces are not built).
> Those are my future plans for auth. Maybe even be able to fallback to a secondary auth method, like using quota and fallback to api key or the reverse.

Support for several auth methods, named reusable connections and later selection
is the user's stated direction. The Web interface is anticipated. Automatic
fallback remains a tentative capability to design; its defaults, triggers and
limits have not been selected.

## Product shape to carry into planning

The subject is authentication to inference providers. Work Engine application
sign-in and authorization govern who may manage or use those connections.

A named connection could carry a stable identifier, editable display name,
provider and route, authentication method, account or organization identity,
credential reference, permitted capabilities, funding arrangement and observed
health. OAuth and API key are authentication methods. Subscription quota,
prepaid API credits and additional paid spend describe funding arrangements;
an auth-method label alone cannot establish the funding source or available
balance. Secret material would remain with a backend credential owner, with
public connection metadata and status available to authorized Web consumers.

The future Web surface could let an operator connect an account or supply a key,
name the connection, inspect its status, disconnect it and select it for work.
Connection names would be labels; jobs and receipts would bind stable identities
and the applicable connection or policy revision. Renaming a connection or
changing a default should preserve the identity and attribution of existing work.
Credential rotation and revocation need their own observable generation/status.

A job or execution profile could select one connection, or a primary connection
with an explicitly configured fallback policy. A policy could express
subscription-first with a bounded API backup, or API-first with an OAuth backup.
The operator could authorize that policy once for a scope, allowing the Host to
switch automatically within its limits. Requiring a fresh human response for
every permitted switch would defeat that intended automation.

Candidate policy distinctions include eligible secondary connections, permitted
failure or exhaustion reasons, finite attempts, per-funding-source limits and the
overall task budget. Compatibility checks would cover the requested model,
reasoning level, endpoint, request parameters and provider capabilities. Switching
auth can require a different provider route or adapter; it cannot be assumed to
be replacement of one credential string.

The Host would resolve the selected connection through its credential owner and
enforce the admitted policy at provider-call boundaries. Fallback would preserve
task, workspace, operation and effect identities. A new inference attempt would
retain its predecessor, switch reason, connection, payer and usage evidence;
switching connections would not authorize repeating an already performed tool
effect. Unknown prior effects or charges would remain visible during recovery.

## Relationship to the qualified implementation

At the recorded commit, `ProviderPlan::validate` binds the selected model,
runtime admission, native credential-source reference, request count and budget.
The Sol tuple also binds high reasoning and a payer account. The current
`project_native_access` lane requires ChatGPT-mode native credentials and refuses
an API key. These are bounded source observations about the qualified lane.

Later API-key and fallback support would need an explicit versioned contract
through the existing credential, admission, provider, lifecycle and accounting
owners. Backend connection management would expose public operations to the Web
UI; the UI would consume those operations. Exact ownership and storage placement
remain implementation-planning questions. This note does not create a new
general-purpose auth service or settle its schema.

The existing no-silent-fallback requirement continues to apply: unconfigured
failure cannot silently select another payer. A future explicitly authorized
fallback policy would make the switch an intended, bounded and recorded action.
The current OAuth qualification and its historical receipts retain their original
scope; this direction does not qualify API-key execution retroactively.

## Tests to add when this capability is scoped

- Connect, select, rotate, expire and revoke each supported credential method;
  verify connection ownership, secret handling and actionable status.
- Create multiple named connections, select the intended stable identity and
  preserve running-job attribution across renames, default changes and rotation.
- Exercise subscription-to-API and API-to-OAuth fallback with compatible model
  and reasoning settings, including a successful secondary and both unavailable.
- Deny an unconfigured secondary, unauthorized account, incompatible route,
  exhausted per-source budget or expired policy before new provider entry.
- Distinguish quota exhaustion, throttling, network uncertainty and revoked
  credentials according to the selected policy; bound attempts and preserve all
  usage, switch reasons and uncertain charges without fabricating quota data.
- Interrupt and restart during fallback; verify one task/effect history, no
  repeated tool effect and durable attribution of every inference attempt.
- Once built, exercise Web management and selection through the real backend
  and credential owner into a bounded execution, with independently checked
  receipts. UI success alone would not establish provider access or task success.

These tests extend the future auth qualification scope. They are not new gates
retroactively attached to the completed supervised pilot. The wider open tests
remain recorded in the [bounded risk map](../records/repeatable-development-sol-20261106/RISK-EVIDENCE.md).

## Evidence and limits

Repository evidence: Tier 2 bounded verification, graph project
`home-bline-code-work-engine`, generation `2026-11-06T04:29:31Z`. Relevant lookup
pages were complete. Coverage for `pi_coding_provider.rs` and `pi_coding_auth.rs`
was content-verified with unresolved-call limitations. The exact provider
validation snippet and auth projection source at lines 90–155 govern the source
observations; no exhaustive architecture or Web-absence claim is made. The Web
interface's future status is attributed to the user. No native credential
contents or external provider capabilities were inspected for this note.
