# Review packet: Pi Host W1-W4 (Anthropic Messages family, `claude-sdk` provider)

Subject: `c4a8ecfc` (code), `814e2436` and `8876b435` (record and its correction), against `e527c36e`
([subject.diff](subject.diff): `work-engine-manual-host`, its receiver and tests, one line in
`work-engine-server`, the harness and the record). The reviewer is read-only and writes only
`REVIEW.md` in this directory. Use `git show 8876b435:<path>` for exact subject bytes, since the
working tree may hold later changes. Do not build into `rust/crates/pi-pressure-context-host/target`,
run Docker, or make provider calls.

## Authority and scope

[Plan](../../planning/dev-vertical-pi-host-realization-20261117.md) items W1-W4, authorized by the
operator 2026-11-18 (model `claude-haiku-5-5`). Pi in Docker stays the single native realization;
the provider is chosen per run. The research-lifecycle owner confirmed the sequence (board 5879),
with condition 4: disable Claude Code's internal retries if possible, otherwise every CLI retry
must be a Host attempt, and no hidden retry may reach upstream. The lifecycle crates are out of
scope. Evidence is the [qualification record](../../records/dev-vertical-h2-route-20261117/README.md),
section "W1-W4". The design rationale is in the probe
[README](../../records/dev-vertical-h2-route-20261117/pi-claude-probe/README.md).

## What changed

1. `pi_stream.rs`: `Wire` (Responses or Messages); Messages usage (`message_start` plus the last
   `message_delta`), `message_stop` under the existing tail rule, and `error` events counted as
   failures. Response heads keep the rate-limit, overage and `request-id` headers, which
   `relay.json` retains.
2. `pi_coding_provider.rs`: `ProviderPlan.provider`, bound by the admission (`claude-sdk` only on
   the analysis profile); Pi's Anthropic catalog; `messages_cost` per token class at the tier of
   all input tokens; Messages settlement, where the raw stop reason must agree with the container.
3. `pi_coding_hosted.rs`: endpoints `https:anthropic-oauth` and `https:anthropic-api`; the
   Messages header allowlist; the Messages URL (`?beta=true` kept); strict JSON; the key
   allowlist; `tool_choice`; custom tools only; the pinned analysis tools digest; canonical prefix
   continuity (copied from `native-realization` on another branch, to be unified later); client
   retry refusal; and the subscription billing guard.
4. `pi_coding_https.rs`: credential kinds `claude-code-auth` (re-read per connection, all tokens
   screened), `anthropic-api-key-env` and `anthropic-api-key-file`.
5. Receiver `pi-coding.mjs`: the `claude-sdk` mode (extension loading; model resolution after the
   services load; a loopback forwarder with one request per grant; a dummy credential; Claude
   Code's env switches; completion only after the relay settles; forced exit after close).
6. Harness: `claude_runtime.py`, the `pi_hosted_messages_observer` example, and `analyze.py`.

## Questions (complex review: transport containment, money, evidence)

- Can the container, Claude Code or the model cause any upstream request that is not exactly one
  gate-admitted, ledger-reserved Host attempt: a retry, a second request per grant, a request
  outside a grant, another path or host, or a body the policy did not check (duplicate keys,
  encodings)?
- Can access material (the subscription token, an earlier token after a refresh, an API key)
  reach the container, the journal or a retained file? Is the dummy credential harmless?
- Messages settlement: are usage extraction, tier choice and stop-reason agreement right and
  conservative? Can a stream settle as completed with usage that understates the cost? Is the
  945 001 µUSD reservation (and the 32 M analysis ceiling) a sound worst case?
- Billing guard: does it fail closed for every case where a request might draw extra usage, given
  the headers we have? Is any case falsely refused?
- Canonical continuity: is the copy faithful to the accepted two-rule rule, and is applying it
  per job (including across failover attempts) correct?
- Receiver: races (completion vs relay end; forwarder vs grant lifetime; abort and cancel),
  hangs, and anything that could leave a request unaccounted.
- Is every claim in the record's W1-W4 section supported, including the live run's finding that
  the analysis grammar refuses `thinking` blocks and that thinking cannot be turned off for this
  model?

Severity: P0 (containment, money or credential loss), P1 (a wrong claim or a broken guarantee),
P2 (a defect with a bounded effect), P3 (clarity). Give a file and line for each finding, and a
verdict (accept, or changes required).
