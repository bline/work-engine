# OpenRouter Bridge Launch

Status: proposed runtime shape, not charter doctrine

Canonical runtime configuration:
[`genesis/runtime-manifest.json`](genesis/runtime-manifest.json)  
Canonical opening question:
[`genesis/opening-question.txt`](genesis/opening-question.txt)  
Canonical system seed: [`bridge-agent-seed.md`](bridge-agent-seed.md)

## Purpose

Run [`bridge-agent-seed.md`](bridge-agent-seed.md) as the actual system message
without the Codex coding-agent harness. This removes one important source of
role pressure, but it does not make the model unconditioned or independent of
its training, upstream provider, or router.

## Founding provenance

The identity of the bridge contribution depends on more than the displayed
model name. Record, for every request:

- the SHA-256 of the seed file;
- the requested canonical model slug;
- the allowed provider endpoint or provider slug;
- sampling and reasoning parameters;
- the raw request messages and raw response;
- the returned model, system fingerprint when present, usage, and routing
  metadata;
- any retry, fallback, context compression, guardrail, or other pipeline event.

The intended bridge model is OpenAI GPT-5.6 Sol, requested on OpenRouter as
`openai/gpt-5.6-sol`. Use one explicit model rather than a model fallback list.
Restrict `provider.only` to `openai` and set `provider.allow_fallbacks` to
`false` so the founding participant is not silently moved to an Azure or Amazon
Bedrock endpoint. A failed request is better founding evidence than an
unrecorded participant substitution.

Enable OpenRouter router metadata with the `X-OpenRouter-Metadata: enabled`
header. Its pipeline record can expose routing attempts and some material
transformations. Absence of metadata is not proof that no transformation
occurred; cached responses, provider internals, and model training remain
outside that record.

Choose the provider data policy deliberately. `provider.data_collection:
"deny"` filters out endpoints reported as collecting prompts, and
`provider.zdr: true` restricts routing to zero-data-retention endpoints. These
settings can reduce endpoint availability and do not replace preserving the
founding record under community control.

## Opening sequence

For the first request, send only:

1. the exact bytes of [`bridge-agent-seed.md`](bridge-agent-seed.md), including
   its final newline, as the `system` message; and
2. the exact bytes of
   [`genesis/opening-question.txt`](genesis/opening-question.txt), including its
   final newline, as the `user` message.

The question is intentionally stored separately so its bytes do not depend on
an example embedded in this document.

Do not include proposed charter language or the root context in that first
request. Preserve the response, then disclose
[`root-context.md`](root-context.md) and begin the reciprocal dialogue. This
does not remove the seed's framing, but it gives the agent's initial problem map
a chance to exist before it becomes a reaction to a substantive human draft.

## Request shape

The canonical request settings are fields of
[`genesis/runtime-manifest.json`](genesis/runtime-manifest.json), not duplicated
here. The client must construct the API body from that manifest, substitute the
two exact message byte sequences named above, and record the complete
authorization-redacted request bytes as a `model_request` event before sending.
The manifest currently records `paid_execution_authorized: false`; creating it
does not authorize the request.

The bridge dialogue should remain one ordered message history. If the client
uses the Chat Completions endpoint, send the retained history on each turn and
append each returned assistant message exactly. Store the runtime record
separately from later summaries so a polished charter cannot overwrite its
founding evidence.
