# Qualification experiments and open questions

Status: extracted experiment ideas, 2026-11-15. Source:
[scratch notes](../scratch.md), lines 183–189, 215–217, 297, 405–428,
757, 848–866, and 901–918. These are proposed investigations; none was run for
this extraction. See the [collection index](README.md).

## Offline gateway foundation

The source proposes using native Pi in a network-disabled container with
placeholder credentials and two synthetic host-side billing routes. Intended
observations include host-only authentication, valid streamed responses,
configured quota-failure fallback, preservation of uncertain attempts, and
receipts identifying the actual route and accounting basis.

The development-model refinement separates establishing the credential-free
request/authorization/stream/settlement chain from adding authorized billing
failover. Real streaming, cancellation, backpressure, and accounting remain
distinct from the reported buffered synthetic probe.

## Real authentication and route compatibility

The notes propose separately qualifying actual subscription and API routes,
including registration and permissions, supported model and effort settings,
endpoints, request fields, and continuation behavior. Existing Codex tokens
are not assumed interchangeable with a newer sign-in registration.

These tests would also establish whether the source's reported plan-usage
restrictions and hosted/commercial eligibility apply to the intended deployment.
The conversation's pause and authorization boundaries remain intact.

## Cache experiment matrix

| Comparison | Question |
| --- | --- |
| Same model, same effort, same route, repeated prefix | Is there a measurable warm-cache control? |
| New Pi session, same model, credential, and rendered prefix | Is reuse independent of the client session or container in this configuration? |
| Same model and route, changed effort through the supported update mechanism | Does the earlier prefix remain reusable? |
| Same model, subscription to API | Does cache reuse survive a funding/authentication switch? |
| Return to the original route | Does its previous cache remain usable? |
| Equivalent Claude work through OAuth and API-key paths | Are misses attributable to prefix changes, isolation, or both? |

Run applicable controls separately for OpenAI and Anthropic. Provider, model,
account scope, adapter version, retention, and timing matter. The source
specifically deprioritizes paid tests of cross-model Claude reuse given its
reported model-specific cache; that is a prioritization proposal, not a result.

For Claude route comparisons, inspect effective system instructions and tool
rendering. The source reports authentication-dependent prompt transformations
in Pi. A switch that changes the prefix does not isolate the cache-sharing
question. It also suggests provider cache diagnostics, if actually available,
to distinguish those causes.

## Evidence to collect

The proposed measurements are actual cache-read tokens, cache writes where
reported, uncached and total input under the provider's semantics, output,
latency, and costs. Bind them to the actual model, effort, request configuration,
account and funding route, effective-prefix comparison, retention/timing, and
attempt outcome. Missing fields remain unknown. Request success alone is not
evidence of cache reuse.

Use source-specific pricing and separate observed billing from estimates or
subscription availability. Prefix comparison must account for adapter-added
instructions and tool transformations, not just Work Engine's stored messages.

## Decisions the evidence could inform

The experiments could determine which route transitions preserve useful cache,
whether supported effort changes are a cheaper adaptation than model changes,
which cache settings to use for each transport, and when a switch is worth its
expected locality loss. They could also establish whether the gateway has
sufficient streaming and attempt evidence for bounded failover.

Resource failure policies can be tested with synthetic quota and credit states;
optimization needs measured quality and efficiency outcomes. Neither kind of
test alone establishes the other's correctness or authorizes implementation.
