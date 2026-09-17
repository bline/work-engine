# Fenced OAuth credential-tip broker

## Status

Operator-originated raw architectural idea, captured on 2026-09-09 from a Web
Sol research round during recovery of PPCE Slice 2 native Claude review
authentication.

This document preserves the idea for later intake. It does not accept a design,
authorize implementation or provider use, change the migration roadmap, define
a production credential service, or endorse dependence on undocumented Claude
credential internals. Formal Idea Intake is deferred until this raw source has
an immutable Git revision under separate commit authority.

```yaml
idea_status:
  architectural_supersession: none
  architectural_supersession_note: "The 2026-09-16 canonical-view reconciliation below found strong future correspondences (Transition Fencing, Runtime Realization capability-observation vocabulary, Authority & Ownership) but nothing built yet -- broker itself confirmed unbuilt this session (only access-only extraction is real, in native-claude-code-adapter.mjs)."
  residue: present
  residue_ledger: "'Unresolved refresh hinge' plus 'Questions for later intake' (9 items) tagged by kind: 6 RESIDUE (budget-boundary policy, provider-neutral placement, subsystem lifecycle, containment boundary, lineage-classification authority, plus the hinge itself), 4 BACKLOG (empirical/implementation questions). All confirmed OPEN, none re-verified against current canonical-view text beyond this session's own broader reconciliation."
  backlog: present
  backlog_ledger: "4 of 9 'Questions for later intake' items are KIND: BACKLOG and confirmed OPEN; no dedicated staged-plan section exists."
  audit_scope:
    - open-question-ledger
  audit_scope_completeness: complete
  status_as_of: 2026-09-16
```

```yaml
idea_provenance:
  origin: direct_capture
```

## Recognition event

Claude's setup-token path was reproduced failing outside Work Engine on both
Claude Code 2.1.261 and 2.1.267. In each fresh, no-resume test, Claude recognized
`CLAUDE_CODE_OAUTH_TOKEN` as first-party OAuth but returned HTTP 401 before
inference. The temporary compatibility route is therefore ordinary `/login`
credentials, whose access token is short-lived and whose refresh token rotates.

Allowing each isolated reviewer to receive and refresh a copied credential
creates an unrecoverable race. Anthropic can consume and invalidate refresh
token N before Work Engine observes or rejects the reviewer's local file write.
Fencing publication after that mutation is too late.

The stronger candidate invariant is:

> The broker owns the sole refresh-capable OAuth tip. Consumers receive
> immutable, access-only projections of a numbered generation within a lineage
> epoch. Tip advancement requires exclusive fenced refresh or staged operator
> login, compare-and-swap publication, and generation drain.

## Candidate architecture

```text
operator /login
      |
      v
staged credential candidate
      |
      v
canonical credential tip (lineage L, generation N)
{ access token, refresh token, expiresAt, scopes }
      |
      v
fenced credential broker
      |
      +-- access-only projection N --> reviewer A
      +-- access-only projection N --> reviewer B
      +-- access-only projection N --> reviewer C

reviewers receive no refresh token and cannot advance credential lineage
```

Refresh is a separate broker-owned transition:

```text
STABLE(N)
   |
   +-- issue bounded access-only leases for N
   |
   +-- admission horizon reached
           v
       DRAINING(N)
           |
           +-- close new admission
           +-- wait for active readers of N to drain
           v
       REFRESHING(N, fence F)
           |
           +-- refresh through the sole credential worker
           +-- CAS expected tip N under fence F
           v
       STABLE(N+1)
```

This moves enforcement from rejecting unauthorized credential mutations to
withholding the capability to make them.

## Access-only realization result

The compatibility mechanism extracts only the current `/login`
`accessToken` from the broker-owned canonical credential and supplies that value
to a reviewer as `CLAUDE_CODE_OAUTH_TOKEN`. The reviewer receives no
`.credentials.json` and therefore no refresh token.

This mechanism was tested on 2026-09-09 with Claude Code 2.1.267 on the admitted
Linux host. A fresh isolated invocation returned `AUTH_OK`, and a separate
isolated session resumed successfully with the same session ID. Neither runtime
materialized `.credentials.json`; both received only the access token through
the environment. Durable non-secret receipts are retained under
`~/.local/state/work-engine/app-server-migration-934d50f/diagnostics/claude-login-access-projection-2.1.267/`.

That experiment established the absence of refresh authority in the isolated
Claude configuration. It did not establish access-token containment within the
top-level reviewer process. Because the token is supplied through
`CLAUDE_CODE_OAUTH_TOKEN`, a separate admission experiment must determine
whether Claude-launched tools, shells, hooks, or subagents inherit the variable.
If descendants inherit it, the projection remains access-only but its bearer
credential exposure surface includes those descendants. The realization and
its threat model must record that surface rather than equating absence of a
refresh token with containment of the access token.

The bounded experiment was:

1. Perform ordinary `/login` in a broker-owned durable configuration.
2. Read its access token only inside the restricted credential boundary.
3. Launch one fresh isolated `claude -p` with that value supplied through
   `CLAUDE_CODE_OAUTH_TOKEN`.
4. Remove higher-priority API, gateway, and cloud-routing variables.
5. Supply no `.credentials.json` to the consumer.
6. Record only success or failure, access-only status, credential generation,
   and the admitted environment projection.
7. Never print or publish the token or a token-derived fingerprint.

This success supports the compatibility mechanism. It does not establish the
refresh protocol, safe concurrency, general provider portability, or permanent
product direction.

## Generation lifetime and drain

The credential's actual `expiresAt` should govern admission. Do not hard-code a
presumed OAuth lifetime.

```text
remaining_lifetime = expiresAt - now

required_lifetime =
    admitted_execution_budget
  + drain_margin
  + refresh_safety_margin

admit only when remaining_lifetime > required_lifetime
```

Each access-only projection carries a bounded generation lease and `notAfter`.
When a generation approaches its admission horizon, the broker closes
admission and drains its consumers before refresh unless exact experiments
establish that issued access tokens remain valid after rotation.

Crossing `notAfter` does not authorize the credential service to choose a
workflow response. The workflow owner decides whether to fence, replace,
resume, fail truthfully, or escalate.

## Suspect-generation handling

An unexpected authentication failure should not trigger an uncoordinated
refresh. A consumer does not request or perform credential refresh. It reports
the authentication failure against the exact generation it received; the host
routes that evidence to the broker, and the broker owns its interpretation:

```text
consumer reports auth_failure(generation=N, status=401)
        v
host authentication boundary
        |
        v
credential broker
        |
        +-- N is older than the current tip
        |       +-- stale report; do not transition the current generation
        |       +-- return the current access-only projection when retry is allowed
        |
        +-- N is current and healthy
        |       +-- do not rotate gratuitously
        |       +-- return retry refusal or explicit failure classification
        |
        +-- N is already SUSPECT or REFRESHING
        |       +-- join the existing singleflight transition
        |       +-- wait for its terminal result
        |
        +-- N is current and the report wins the suspect/refresh fence
                +-- close admission
                +-- validate the canonical tip through a broker-owned check
                        |
                        +-- valid: consumer-local or provider failure
                        +-- invalid or expired: drain and fenced refresh
                        +-- unavailable: truthful failure or escalation
```

The validation mechanism and whether it enters provider infrastructure remain
part of the eventual authority contract.

The reviewer-facing contract should remain provider-neutral and deliberately
ignorant of credential lifecycle. Its meaningful statement is:

```text
I received credential generation N.
Generation N produced this classified authentication failure.
Resolve that evidence for the originating operation.
```

It must not request OAuth refresh, select a credential source, choose a
provider, or advance credential lineage.

## Successor discovery and singleflight

The current compatibility implementation rereads the canonical credential
source on an authorized authentication retry. That is successor discovery, not
credential recovery. The eventual broker should make the distinction explicit:

```text
auth_failure(N)
        |
        v
reacquire access-only projection
        |
        +-- current generation > N
        |       +-- retry or resume against the successor when authorized
        |
        +-- current generation == N
                +-- return auth_stale until broker recovery advances the tip
```

When several consumers report failure for the same current generation, exactly
one broker-owned transition may validate or refresh it. Other callers join that
singleflight result rather than starting refresh work:

```text
first auth_failure(27): STABLE(27) -> SUSPECT(27) -> REFRESHING(27)
later auth_failure(27): join transition for 27 and wait
publication:             27 -> 28
eligible waiters:        receive access-only projection 28
late auth_failure(26):   stale report; never makes 28 suspect
```

An authentication failure may cause at most one automatic credential-generation
transition for its originating operation. After receiving one successor, that
operation must either succeed or surface a classified error. It must not rotate
again recursively, because a provider outage or unrelated authorization defect
must not consume credential lineage through an unbounded refresh loop.

The host, not the reviewer or broker, owns whether the originating workflow may
retry or resume after the broker resolves the report. Broker resolution supplies
credential state; it does not grant execution authority.

## Operator login and lineage fencing

Interactive operator login is an authoritative source of new credentials, but
it must not mutate the canonical tip outside broker serialization. Otherwise an
older credential worker refreshing `(lineage=L, generation=N)` could publish
after the operator logs in and overwrite the newer credential with a successor
from the displaced lineage.

The candidate boundary is:

```text
operator /login
        |
        v
private staged credential candidate
        |
        v
broker validates source, custody, and admissible metadata
        |
        +-- ordinary refresh lineage remains valid
        |       +-- fenced CAS advances generation N -> N+1
        |
        +-- login establishes a replacement credential lineage
                +-- fenced adoption advances lineage L -> L+1
                +-- generation restarts or advances under the new lineage
                +-- every in-flight CAS bound to lineage L becomes stale
```

The staged candidate remains secret, owner-only state and is never a workflow
artifact. Adoption produces a non-secret transition receipt. The exact rule for
distinguishing same-lineage reauthentication from lineage replacement remains
an intake question; safety requires only that an older-lineage transition
cannot overwrite an adopted operator credential.

## Ownership boundaries

- The credential broker owns the secret tip, generation lineage, access-only
  materialization, credential-state classification, singleflight refresh lease,
  staged-login adoption, and CAS transition.
- A dedicated credential worker is the only realization that may possess the
  refresh token or invoke refresh.
- The realization manifest references the authentication mechanism and
  environment-projection revision.
- The environment projection owns what authentication capability a process
  should receive, including `refreshCapable: false`.
- The process launcher constructs the environment.
- The process receipt attests admitted non-secret facts it received.
- Workflow owners decide whether credential unavailability permits waiting,
  retry, replacement, failure, or another route.
- The operator retains authority over interactive login and provider or
  spending-route changes.

The broker does not acquire review selection, workflow acceptance, provider
fallback, or roadmap authority by supplying credentials.

## Candidate receipts

Credential transition receipt:

```yaml
credentialLineage: 7
credentialGeneration: 43
predecessorGeneration: 42
refreshFence: refresh-lease-reference
transition: accepted
source: broker-owned-credential-worker
reason: scheduled_refresh
publishedAt: timestamp
```

Environment and process receipts can bind:

```yaml
authMechanism: claude-oauth-access-projection
credentialLineage: 7
credentialGeneration: 43
refreshCapable: false
environmentProjectionRevision: revision
transportAttested: true
```

Receipts must not include access tokens, refresh tokens, raw credential
digests, or globally correlatable token hashes. If the restricted credential
subsystem needs equality checking, a broker-keyed HMAC may be considered as
private internal state; it should not enter workflow, telemetry, or research
artifacts.

## Unresolved refresh hinge

**[KIND: RESIDUE] [OPEN — a mechanism-choice gap that determines whether unattended refresh is possible at all, not merely an implementation detail; the document's own text: "this hinge must be resolved before the broker can claim unattended operation."]**

The broker still needs a supported way to advance the canonical credential.
Claude appears to refresh `/login` credentials lazily during authenticated
operation. A design cannot assume `claude auth status` performs refresh or that
an undocumented token endpoint is stable.

Candidate approaches needing evidence include:

- a broker-owned Claude credential worker whose controlled authenticated
  operation permits refresh;
- a documented refresh interface, if Anthropic provides one;
- operator-mediated `/login` when automatic refresh cannot be established; or
- treating expiry as a visible availability boundary rather than fabricating
  unattended refresh.

This hinge must be resolved before the broker can claim unattended operation.

## External evidence noted at capture

These public Anthropic repository reports are observations, not Work Engine
authority:

- [#91708](https://github.com/anthropics/claude-code/issues/91708) reports a
  file-backed refresh race on Claude Code 2.1.258 and process-local refresh
  deduplication.
- [#80585](https://github.com/anthropics/claude-code/issues/80585) reports
  concurrency failures around rotating subscription credentials.
- [#37678](https://github.com/anthropics/claude-code/issues/37678) discusses a
  token-broker mitigation.
- [#52203](https://github.com/anthropics/claude-code/issues/52203) records the
  documented access-token role of `CLAUDE_CODE_OAUTH_TOKEN`.
- [#81350](https://github.com/anthropics/claude-code/issues/81350) and
  [#37512](https://github.com/anthropics/claude-code/issues/37512) report the
  access-only internal shape of environment-supplied OAuth credentials.

Some reports concern other platforms or versions. They justify exact testing,
not generalized behavior claims.

## Real-code findings (2026-09-16 investigation)

The access-only extraction this document tested manually on 2026-09-09 is now
real, shipped code — confirmed directly, not assumed. `readClaudeLoginAccessProjection`
(`app-server/src/services/reviewer-runtime/native-claude-code-adapter.mjs:54-83`)
opens the owner-only `.credentials.json`, refuses it unless the file mode and
uid match the current owner, extracts only `claudeAiOauth.accessToken` and
`expiresAt`, refuses an already-expired token, and returns exactly
`{mechanism: "claude_login_access_projection", token, expiresAt,
refreshCapable: false}` — the literal `refreshCapable: false` field this
document's own "Access-only realization result" and "Candidate receipts"
sections propose. `directAnthropicEnvironment` (lines 41-52) strips exactly
the cloud-routing variables this document's "Removed to prevent route or
billing drift" section names, and separately deletes any inherited
`CLAUDE_CODE_OAUTH_TOKEN` before re-injecting the extracted one.

**The broker itself remains entirely unbuilt, exactly as this document's own
status claims.** A repository-wide search for credential/OAuth handling
(`grep -rli "credential.*broker\|CLAUDE_CODE_OAUTH_TOKEN\|refreshCapable"
app-server/src`) returns only this one file. There is no generation counter,
no lineage, no CAS, no drain sequence, no singleflight, and no multi-reader
tracking anywhere — every call to `readClaudeLoginAccessProjection` is an
independent, stateless read with no coordination between concurrent
reviewers, which is exactly the unrecoverable race this document's own
"Recognition event" describes. This confirms the compatibility mechanism
without establishing the refresh protocol, matching the document's own
"Access-only realization result" closing caveat precisely.

## Relationship to the architecture views (2026-09-16)

None of this is implemented, so nothing here is added to any settled view's
confirmed-instances list — these are candidate correspondences for a future
proposal, not citations of real code, and are recorded here rather than in
any of the 20 views.

**`mechanisms/transition-fencing-and-leases.md` — the refresh transition is
this mechanism's shape, precisely, and would be a strong future candidate
instance.** `STABLE(N) → DRAINING(N) → REFRESHING(N, fence F) → STABLE(N+1)`
is bind (fence F on generation N) → close new admission → drain active
readers → prepare (refresh through the sole credential worker) → revalidate
(CAS expected tip N) → publish (N+1) → release — the identical sequence this
session confirmed today for the executable-generation reload, which this
mechanism's own page now names as a real, implemented second instance
alongside Context Lifecycle. This document's own singleflight suspect/refresh
fence ("exactly one broker-owned transition may validate or refresh it")
is the same "may this transition proceed concurrently with other
transitions?" question that mechanism exists to answer. Like the
executable-generation instance, it protects neither an unresolved semantic
judgment nor an organizational topology change — a third kind of thing
credential-tip authority — reinforcing, not deciding, today's open question
about whether the "two fence classes" framing needs a third named class or
should read "at least two, not exhaustive."

**`mechanisms/resource-lease-and-fencing.md` — deliberately ruled out,
despite surface vocabulary overlap.** Reviewer "access-only projection N"
is held concurrently by multiple readers (reviewer A, B, C simultaneously)
against one generation — a shared-read broadcast, not the single-current-
holder mutual exclusion that mechanism's own `acquire`/`admitMutation` shape
defines. "Fenced," "generation," and "lease" all appear in both, but the
underlying invariant differs, the same discipline used elsewhere this
session to keep this mechanism distinct from Transition Fencing.

**`runtime-realization.md` §2/§3 — the credential tip's own state is already
this dimension's own vocabulary, just with a different concrete subject.**
"`identity: claude-oauth-access/reviewer-boundary, generation: 43, state:
available`" is structurally identical to that section's own worked example
(`identity: codex/openai/account-A/provider-access, state: available,
generation: 184, ...`). If ever built, the credential tip's generation state
is naturally domain detail under this dimension's existing capability-
observation pattern, not new architecture — the same "one more concrete
subject of an already-owned pattern" shape found repeatedly this session,
not yet a citable instance because nothing is built.

**`authority-and-ownership.md` — two independent, already-correct
instances, uncited by name.** §12 (Invalidation Never Mints Authority):
"Crossing `notAfter` does not authorize the credential service to choose a
workflow response. The workflow owner decides..." is this document's own,
independently-arrived-at restatement of §12, joining the session's growing
confirmed-instance list (claim refresh, runtime-realization invalidation,
plan failure, context unfitness, resource-lease supersession, intent
projection). §7 (Participation Modes): reviewers only **Observe** (receive
an access-only projection); the dedicated credential worker alone may
**Execute** refresh; workflow owners **Decide**/**Admit** the consequence of
unavailability — this document's own "Ownership boundaries" section already
enforces this separation without using the settled vocabulary.

**`claim-relative-environment-projections.md` — this document is a concrete,
detailed elaboration of that idea, reconciled earlier in this same session.**
That document's own illustrative `environmentProjection` YAML names
`CLAUDE_CODE_OAUTH_TOKEN` with `evidence: presence` and
`secretSourceOwner: operator-credential-store` almost exactly as a smaller
version of what this document works out in full (generation lineage,
receipts, drain, singleflight). The existing cross-reference (above, under
"Relationship to other pending ideas") is accurate; strengthened here now
that both documents have had their own architecture-view reconciliation
pass.

**`mechanisms/revision-cas-and-publication.md` — the generation lineage
itself is a clean, additional proposed instance, nested under Runtime
Realization, not an independent new dimension-level row.** Predecessor/
successor generation numbers, CAS-published transitions, and lineage-epoch
replacement making "every in-flight CAS bound to lineage L... stale" all
match this mechanism's shape — the same nested-instance-under-an-existing-
dimension pattern used today for the executable-generation substrate, not a
reason to add an eighth top-level instance to that mechanism's confirmed
count.

## Relationship to other pending ideas

This idea complements:

- `claim-relative-environment-projections.md`, which can carry credential
  generation and `refreshCapable: false`;
- `scoped-workflow-event-surface.md`, which can expose redacted admission,
  drain, suspect, and refresh transitions;
- `revisioned-research-and-execution-architecture.md`, whose realization,
  receipt, and recovery-frontier model applies; and
- `proposal-decision-gated-implementation-compilation.md`, because an incident
  discovery remains raw input until separately formed and accepted.

## Questions for later intake

1. [KIND: BACKLOG] [OPEN — an empirical investigation, no evidence this was checked] What scopes and account metadata are required for native review beyond the
   tested login realization?
2. [KIND: BACKLOG] [OPEN — a provider-behavior empirical question] Does refresh invalidate already-issued access tokens?
3. [KIND: RESIDUE] [OPEN — an authority/budget-boundary policy question] What execution budget is admissible against `expiresAt`?
4. [KIND: BACKLOG] [OPEN — an implementation-mechanism detail] How are crashed consumers removed from active-reader counts safely?
5. [KIND: BACKLOG] [OPEN — an implementation-mechanism design question] What supported operation lets the broker refresh without unrelated work?
6. [KIND: RESIDUE] [OPEN — bears directly on ProviderTurnPort/HarnessRuntimePort placement, an architecture-boundary question] Can the broker remain provider-specific behind a provider-neutral
   credential capability contract?
7. [KIND: RESIDUE] [OPEN — a scope/lifecycle-ownership decision for the whole subsystem] If setup-token authentication is repaired, should this subsystem retire,
   remain fallback, or generalize?
8. [KIND: RESIDUE] [OPEN — an authority/evidence-boundary containment question] Which Claude-launched descendants inherit the access-token environment
   projection, and what containment or admission boundary is required?
9. [KIND: RESIDUE] [OPEN — a lineage-classification/authority question] What evidence distinguishes an operator login that continues a credential
   lineage from one that must advance the lineage epoch?

## Non-goals

This raw idea does not propose:

- giving reviewers refresh tokens;
- accepting reviewer-written credentials after server-side rotation;
- sharing mutable `.credentials.json` across concurrent reviewers;
- storing secrets or ordinary token hashes in receipts;
- treating a generation number as proof authentication succeeded;
- silently switching provider or billing route;
- making the credential broker a workflow or research authority; or
- claiming the validated access-only compatibility path establishes the still
  unresolved refresh and concurrency design.
