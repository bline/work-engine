# Native Claude transport

Keep `claude-code` as the harness for every provider selected by this skill.
Changing the inference gateway must not silently substitute Codex, an
OpenRouter agent loop, or another generic harness for Claude Code's system
prompt, tools, compaction, permissions, or session machinery.

## Transport identities

- `anthropic`: the ordinary native Claude route with no gateway override.
- `openrouter`: the same native `claude` executable using OpenRouter's
  Anthropic-compatible endpoint.
- `auto`: try `anthropic` once, then consider `openrouter` only when the
  launcher recognizes a quota or rate-limit failure.

The logical review provider may remain `claude`, but every receipt must record
the Claude Code version, gateway, requested model, launcher digest, command
digest, attempt results, and whether the upstream provider was actually
observed. A requested OpenRouter model is not evidence of its upstream route.

## Launcher

Use the deterministic launcher rather than reconstructing environment changes
at each call:

```bash
python3 skills/claude-recon-implementation/scripts/claude_transport.py \
  --transport auto \
  --continuity disposable \
  --allow-paid-failover \
  --receipt '<transport-receipt.json>' \
  --openrouter-model 'anthropic/claude-sonnet-5-20260630' \
  -- \
  claude -p \
    --effort medium \
    --model sonnet \
    --no-session-persistence \
    --tools 'mcp__codebase-memory-mcp,Read,Glob,Grep' \
    --output-format json \
    --json-schema '<schema>' \
    --dangerously-skip-permissions \
    '<bounded prompt>'
```

`OPENROUTER_API_KEY` must be present only in the process environment. The
launcher supplies `ANTHROPIC_BASE_URL`, `ANTHROPIC_AUTH_TOKEN`, an empty
`ANTHROPIC_API_KEY`, and `ANTHROPIC_DEFAULT_SONNET_MODEL` to the OpenRouter
attempt. It never stores the credential in a receipt.

Possession of `OPENROUTER_API_KEY` is not spend authority. Auto mode requires
`--allow-paid-failover` from the governing invocation before a quota failure
may cause an OpenRouter call. Selecting `--transport openrouter` is itself an
explicit paid-route selection.

Use `--transport openrouter` when the route is selected before provider entry.
Use a `:batch` model only after the exact model and native Claude Code route
have passed a separately recorded compatibility test, then supply
`--allow-batch-route`. Standard and batch routes are distinct execution
configurations even when their model family is the same.

### Research-grade upstream pinning

An `anthropic/...` model slug identifies the model family; it does not by
itself restrict OpenRouter to Anthropic's first-party endpoint. For a research
run, use a dedicated API key assigned to a guardrail whose provider allowlist
is exactly `['anthropic']` and whose model allowlist contains only the exact
requested model. Immediately before provider entry, observe both the guardrail
and its key assignment through OpenRouter's management API and normalize them
to
[openrouter-routing-attestation.schema.json](openrouter-routing-attestation.schema.json).

Produce that artifact with the read-only collector:

```bash
OPENROUTER_MANAGEMENT_KEY='<management credential>' \
OPENROUTER_API_KEY_HASH='<non-secret inference-key hash>' \
python3 skills/claude-recon-implementation/scripts/openrouter_routing_attestation.py \
  --guardrail-id '<guardrail UUID>' \
  --model 'anthropic/claude-sonnet-5-20260630' \
  --output '<routing-attestation.json>'
```

The collector issues only `GET` requests: one exact guardrail lookup and a
complete paginated read of key assignments. It fails closed unless the
guardrail permits only `anthropic`, permits only the requested model, and the
selected key hash has exactly one direct assignment to that guardrail. Keep the
management credential separate from the inference key; neither credential is
written to the artifact. The key hash is an identifier, not a secret.

Pass that fresh artifact with `--require-anthropic-1p`,
`--routing-attestation`, and the non-secret `OPENROUTER_API_KEY_HASH`. The
launcher checks the provider/model restrictions, key assignment, and freshness
before either side of an auto route runs. It records the attestation digest but
still reports the upstream provider as requested rather than observed. Bind
the post-run OpenRouter generation record separately to confirm which provider
actually served the request.

Do not treat a dashboard preference that merely prioritizes Anthropic as a
research pin: priority plus enabled fallbacks can still select another
provider.

## Failover boundaries

Automatic failover is not a generic retry policy. Authentication, permission,
tool, malformed-request, model, and repository failures return without a paid
fallback. A disposable request is replayable only with
`--no-session-persistence`; the launcher buffers the failed attempt so partial
output is not mistaken for the result.

A retained review has a stronger identity contract:

- A fresh retained entry using `--session-id` does not automatically replay.
  After a quota failure, either select OpenRouter with a new authorized
  reviewer/session binding or reconcile the existing episode explicitly.
- A remediation call using `--resume` may cross gateways only after a smoke
  test has established that the installed Claude Code version reopens the same
  local transcript correctly. Until then, omit
  `--allow-retained-resume-failover` and stop truthfully.
- If a cross-gateway resume fails or its continuity cannot be trusted, mark the
  episode uncertain and use the `independent-review-state` successor-writer
  path. Describe it as reconstructed continuation, never same-session review.

The local session ID preserves a Claude Code transcript reference. It does not
prove identical inference service, upstream provider, cache behavior, or
gateway processing.
