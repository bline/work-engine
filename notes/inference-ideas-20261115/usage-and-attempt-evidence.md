# Usage and attempt evidence

Status: idea extraction, 2026-11-15. Source: [scratch notes](../scratch.md),
lines 2–42, 140–173, 207–215, 283–287, 434–479, and 796–808. Code and provider
behavior are reported observations from the source, not a fresh audit. See the
[collection index](README.md).

## Idea

Observe inference at the host dispatch boundary and preserve both raw provider
evidence and useful normalized usage. Cache metrics should be attributable to
the actual attempt, model, account, configuration, and billing route. Missing
measurements remain unknown rather than becoming zero.

The notes propose capturing provider input counts, cache reads, and cache
writes directly from response events. They distinguish data present inside an
SDK snapshot from data explicitly extracted, persisted, tested, and exposed by
Work Engine.

## Reported telemetry and instrumentation gaps

| Path or source | What the scratch conversation reports |
| --- | --- |
| Pi SDK | Normalized `usage.cacheRead` and `usage.cacheWrite` can carry provider cache counts |
| New `pi-native.mjs` | Snapshots export session messages that may contain normalized cache usage |
| Rust `pi_native.rs` | Raw cache counts are not separately extracted or recorded in the described revision |
| Synthetic native response fixture | Input, output, and total counts appear, but cache fields are absent |
| Older Pi coding runtime | `pi-coding.mjs` imports `pi-native-access.mjs`; `rawTerminalEvidence()` is identified as an observation point |
| New native SDK runtime | Uses a separate credential store and intercepted fetch; does not import that older access helper |

The practical consequence is route-specific instrumentation: modifying the
older helper would not establish telemetry for the newer credential-free
transport. The proposed host gateway needs observation on its actual response
path, with fixtures that exercise cache fields and their absence.

## Provider usage semantics

The source names OpenAI's `input_tokens_details.cached_tokens` and possible
cache-write fields, and Anthropic's `cache_read_input_tokens` and
`cache_creation_input_tokens`. It cautions that input totals differ: in the
reported Anthropic shape, `input_tokens` is the uncached portion and cache
categories are additional; the OpenAI input count is described differently.

These are qualification targets, not a final schema. Raw event shape, field
location, availability, and total-calculation semantics need verification for
each transport before normalization or cross-provider comparison.

## Attempts, failover, and uncertainty

A route switch is another attempt. Quota exhaustion or stream interruption
may occur after resources have been consumed. The proposed accounting retains
the failed attempt, observations, unresolved reservation, and switch reason;
a later successful attempt does not erase them or authorize blindly replaying
uncertain work.

The proposed evidence includes the actual provider and model, payer account,
billing route, request settings, outcome, and usage basis. Routing decisions
also retain the policy revision and observation snapshot, including timestamps,
rate-table inputs where used, authorization, and unavailable information.
Requested identity alone does not establish actual execution identity.

Actual API billing, subscription quota consumption or availability, and an
estimated API-equivalent cost stay separate. Existing accounting and settlement
owners remain the proposed authority for their meanings.

Related: [resource management](inference-resource-management.md),
[research uses](research-and-compiled-policy.md), and
[measurement experiments](qualification-experiments.md).
