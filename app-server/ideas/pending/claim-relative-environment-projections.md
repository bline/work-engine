# Claim-relative environment projections

## Status

Operator-originated architectural idea, captured on 2026-09-09 while diagnosing
subscription authentication for an isolated native Claude reviewer.

This document is exploratory. It does not accept a design, amend `DESIGN.md`,
change a runtime manifest, authorize provider use, define a credential format,
or make environment variables authoritative workflow state. The associated
operational incident remains documented in
`app-server/docs/native-claude-isolated-review-authentication.md`.

## Recognition event

The native-review host supplied a durable Claude setup token to its transport,
but the process receipt originally did not establish whether that
authentication input reached the launched process. A boolean-only receipt
field was added during diagnosis. It established that
`CLAUDE_CODE_OAUTH_TOKEN` was present at the transport boundary while Claude
still returned `Not logged in · Please run /login` for the retained session.

This exposed a broader architectural requirement:

> When environment state can change a required execution or production-path
> claim, the realization should declare a claim-relative environment
> projection and the launched process should produce non-secret evidence of
> whether the realized environment satisfied it.

Environment is part of the execution coordinate. An unfiltered environment
dump is not an acceptable coordinate: it leaks secrets, captures irrelevant
ambient state, and makes reconstruction depend on mutable machine details.

## Placement hypothesis

The expected projection belongs in the realization or runtime manifest, beside
but distinct from capabilities.

- Capabilities state what the realization is allowed to do.
- The environment projection states which process-environment facts are
  required, forbidden, exact, redacted, or deliberately irrelevant for a
  particular claim profile.
- Runtime admission checks whether the selected realization can produce the
  projection and its required evidence.
- A process receipt records the admissible observation of what was realized
  and binds it to the projection revision.

The existing runtime manifest already binds roles to capabilities and a
`runtimeEnvironmentRevision`, while runtime satisfaction binds that revision
to observed context. Those are likely integration points, not evidence that
the new projection should be encoded as a capability.

## Candidate projection shape

The exact schema remains unaccepted. A candidate needs to express at least:

- a stable projection identity and revision;
- the claim or admission profile for which the projection is adequate;
- required variables and how each may be evidenced;
- forbidden variables whose presence would invalidate routing or custody;
- exact non-secret values when behavior depends on them;
- secret references by owner and mechanism, never secret value or a
  correlatable secret digest;
- variables whose values must be redacted but whose presence may be attested;
- an explicit policy for undeclared variables that are later found relevant;
- the realization responsible for constructing the child environment; and
- the independently owned observer or harness responsible for attesting it.

Illustrative, non-normative form:

```yaml
environmentProjection:
  schemaVersion: 1
  profile: native-claude-subscription-review-v1
  required:
    CLAUDE_CODE_OAUTH_TOKEN:
      evidence: presence
      secretSourceOwner: operator-credential-store
  exact:
    CLAUDE_CONFIG_DIR:
      ownership: reviewer-runtime
      evidence: owned-path-identity
  forbidden:
    - ANTHROPIC_API_KEY
    - ANTHROPIC_AUTH_TOKEN
    - ANTHROPIC_BASE_URL
    - OPENROUTER_API_KEY
    - OPENROUTER_MANAGEMENT_KEY
  undeclaredRelevantVariables: admission_failure
```

The corresponding receipt should record the projection identity, satisfied or
failed checks, explicit unavailable evidence, and the process/attempt identity.
It must not record secret contents or secret-derived fingerprints.

## Current Claude authentication flow

The current compatibility flow is the **Claude `/login` access-only projection
for an isolated retained reviewer**:

```text
operator runs `claude auth login --claudeai`
        |
        v
durable owner-only canonical `.credentials.json` outside Git
        |
        v
App Server proxy receives `--claude-login-credentials PATH`
        |
        v
native-review host reads only `.claudeAiOauth.accessToken` and `expiresAt`
        |
        +-- removes stale isolated `.credentials.json`
        +-- constructs direct-Anthropic child environment
        |
        v
`CLAUDE_CODE_OAUTH_TOKEN` -> `claude_transport.py` -> Claude Code child
        |
        v
transport receipt + isolated retained-session artifacts
```

Useful names for external research include:

- `claude setup-token`;
- `CLAUDE_CODE_OAUTH_TOKEN`;
- Claude Code long-lived authentication token;
- Claude Code headless or print-mode authentication;
- Claude Agent SDK subscription authentication;
- `CLAUDE_CONFIG_DIR` isolated sessions;
- Claude Code `--resume` authentication with a setup token; and
- authentication precedence between setup tokens, `.credentials.json`, and
  Anthropic API-key variables.

## Environment variables in the current path

### Required or injected

- `CLAUDE_CODE_OAUTH_TOKEN`: the short-lived access-only projection. The host
  reads it from the operator-owned canonical login credential and injects it
  only into the review subprocess. Receipts may attest presence, expiry, and
  `refreshCapable: false`, never value or fingerprint.
- `CLAUDE_CONFIG_DIR`: selects the reviewer-owned isolated Claude configuration
  and retained-session directory. Its owned path identity and pre-attempt
  content projection can affect continuity and authentication claims.

### Removed to prevent route or billing drift

- `ANTHROPIC_API_KEY`
- `ANTHROPIC_AUTH_TOKEN`
- `ANTHROPIC_BASE_URL`
- `OPENROUTER_API_KEY`
- `OPENROUTER_MANAGEMENT_KEY`

### Rejected when inherited for this direct-Anthropic realization

- `CLAUDE_CODE_USE_GATEWAY`
- `CLAUDE_CODE_USE_BEDROCK`
- `CLAUDE_CODE_USE_VERTEX`
- `CLAUDE_CODE_USE_FOUNDRY`
- `ANTHROPIC_BEDROCK_BASE_URL`
- `ANTHROPIC_VERTEX_BASE_URL`
- `ANTHROPIC_FOUNDRY_BASE_URL`

### Currently observed by the transport receipt

- `CLAUDE_CODE_DISABLE_EXPERIMENTAL_BETAS`
- `CLAUDE_CODE_DISABLE_TERMINAL_TITLE`

These last two are existing compatibility observations, not yet evidence that
they belong in every production-path environment profile.

Other ordinary inherited variables such as `PATH`, locale, home-directory
selection, proxy variables, certificate locations, and tool-specific settings
may also change behavior. This idea deliberately does not classify them all in
advance. Their inclusion must be justified relative to a claim and a concrete
realization, with newly discovered relevance producing an explicit projection
revision or an admission failure rather than historical reinterpretation.

## Ownership and evidence

The manifest owns the expected environment contract. The launcher owns
constructing the child environment. The runtime or harness owns observing and
attesting the resulting projection. The credential store owns secret material.
The service consuming the production-path claim owns whether the evidence is
adequate.

Self-recording remains distinguishable from independent observation. A
launcher saying that it intended to remove a routing variable does not alone
establish what the child received. Strong claims may require an independently
owned process-launch observation or operating-system/harness attestation.
Where that is unavailable, the claim remains unestablished rather than being
inferred from configuration.

## Relationship to existing architecture

This idea appears to extend:

- the revisioned research architecture's claim-relative coordinates,
  realization binding, receipts, and reconstruction coverage;
- runtime-manifest capability and `runtimeEnvironmentRevision` binding;
- runtime-satisfaction evidence used by manifest role projections;
- production-path claims, especially realization, capability, and custody
  facts; and
- scoped workflow events, which could expose a redacted environment-admission
  result without treating telemetry as the semantic owner.

It also follows the proposal-decision-gated implementation architecture: an
event discovered during execution is captured as attributed intake rather than
silently promoted into a general runtime contract.

## Questions for later intake

1. Does the projection belong directly in each realization manifest, in a
   separately revisioned environment profile referenced by it, or both?
2. Which environment facts affect realization admission, production-path
   claims, deterministic reproduction, or only diagnosis?
3. What observer can establish the final child environment independently of
   the launcher without exposing secrets?
4. How should secret-source ownership and rotation revision be identified
   without creating a correlatable credential fingerprint?
5. How should `PATH`, locale, proxy, certificate, and filesystem-location
   dependencies be normalized across providers and operating systems?
6. Is an allowlist feasible, or should admission combine required, forbidden,
   and explicitly ignored classes?
7. How does the projection distinguish reproducible starting conditions from
   an impossible promise of deterministic model output?
8. Which redacted environment-admission facts belong on the scoped event
   surface and which remain receipt-only evidence?

## Non-goals

This raw idea does not propose:

- storing complete process environments;
- storing, hashing, logging, or publishing credential values;
- treating environment equality as trajectory reproducibility;
- classifying every environment variable before a claim requires it;
- making capabilities carry configuration values or secret references;
- making telemetry authoritative for manifest satisfaction; or
- changing the authorization of the current PPCE reviewer episode.
