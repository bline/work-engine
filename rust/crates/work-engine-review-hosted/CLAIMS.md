# What a hosted review run may claim

Scope: one job on the hosted Pi analysis profile (`work-engine.pi-analysis-hosted.v1`) whose inputs
`review-job-inputs` wrote and whose answer `review-result-check` checked into an envelope with
`outcome: checks_passed`. There are exactly two provider profiles, chosen with `--provider`, named in
the job-inputs record and the envelope (`profile`), and nothing else is selectable:

| profile | requested provider id | requested model | wire | requirements revision |
|---|---|---|---|---|
| `openai-codex` | `openai-codex` | `gpt-6-luna` | OpenAI Responses | `hosted-route-qualification-r1` |
| `claude-sdk` | `claude-sdk` | `claude-haiku-5-5` | Anthropic Messages (Claude Code in the container) | `hosted-route-qualification-r2` |

Authority: operator decisions D2/OQ-1 (D) and D3/OQ-2, 2026-11-17 (r1; quoted in
`fixtures/hosted-route-qualification-r1/DERIVATION.md`), and decision D, 2026-11-18 (r2: keeps
`provider_family: anthropic`, removes only `execution_confinement`; quoted in
`fixtures/hosted-route-qualification-r2/DERIVATION.md`). Every claim is route qualification only.

"Requested" means what the body and the profile name: the provider id is the one the Host requires
in request frames and the model is the one in the retained body's `model` field. Neither is a claim
about the serving endpoint.

## May claim

1. **Contract delivered in the body the Host retained as dispatched.** The implementation-review/v1
   contract compiled from the profile's requirements revision, prepared and frozen
   (`contract_sha256`, recomputed by the checker from the retained inputs), is contained in the task
   whose bytes, plus exactly the Host's suffix length (202 bytes), are the one user text of every
   request body the Host retained create-only. Each body's SHA-256 equals every attempt's recorded
   `request_body_sha256`, and:
   - `openai-codex`: `model` is `gpt-6-luna`; the declared tools are exactly `snapshot_list` and
     `snapshot_read`; the body holds exactly one user text item.
   - `claude-sdk`: `model` is `claude-haiku-5-5`; the declared tools are exactly
     `mcp__pi__snapshot_list`, `mcp__pi__snapshot_read` and `DeferredToolPlaceholder`;
     `messages[0]` is a user message whose only content is one text block (or a string) holding
     the task plus the Host suffix; `messages[1]` may be one role `system` message; every later
     user message holds only `tool_result` blocks. Assistant turns, including `thinking` blocks,
     are not read.
2. **A live model answer that parses.** The final answer the Host recorded parses strictly (no fence
   stripping, no repair) as one implementation-review/v1 object whose subject equals the request
   subject (canonical digest and all three fields). On `claude-sdk` the answer is the Host's
   recorded answer only; `thinking` blocks are never answer or evidence.
3. **The vetted snapshot was the one served, and the run is the recorded one.** The job's pinned
   snapshot manifest equals the manifest this crate vetted (secret rules, bounds); the served
   `.review/patch.diff` digest equals the subject's `patchIdentity` and the revision's
   `scope.digest`; the job id, packet limits and authority task digest equal the record.
4. **Grounding as counted.** k of n evidence items name a member of the served manifest and that
   member's SHA-256; decisive evidence that does not is flagged `evidence_ungrounded`.
5. **Requested provider and model per profile, nothing about the route.** The envelope states the
   profile's requested provider and model (table above) and that they equal the record and every
   retained body. The serving route, the payer of each attempt and, on `claude-sdk`, the
   subscription billing guard's outcome are **Host-reported** and carried verbatim in
   `host_reported.hosted_attempts` (and `settlement`, `provider_requests`,
   `snapshot_deliveries`); the checker does not check them and no claim rests on them. On
   `claude-sdk`, the digests of Claude Code's `system[]` blocks and of the environment message are
   recorded per request in `host_reported.request_insertions`, by digest only, not interpreted.
   Role `system` messages in `messages[]` are tolerated only at `messages[1]` or directly after a
   user message holding only `tool_result` blocks (observed `<total_tokens>` notices), with string
   or text-block content; each is recorded by index and digest only, not interpreted; any other
   position or content is refused.
6. **Host-recorded boundaries, as the root's run record shows them:** two Host-served read-only
   tools (`snapshot_list`, `snapshot_read`); zero container tool-execution requests (the Host's
   `tool_requests` set must be empty at completion, `work-engine-manual-host/src/pi_coding_analysis.rs:296`;
   snapshot reads are Host deliveries, not tool requests); containment as the Host verified it
   at run time (from the per-run inspect artifact the root names); the listing of the only writable
   bind (`/export`).
7. **The frozen world stays unmet on both profiles.** `openai-codex`: the frozen candidate prepared
   with the hosted claim has a known realization incompatibility (provider family), is refused and
   produced no job inputs. `claude-sdk`: the provider family is claimed compatible, so the frozen
   candidate is prepared, but its `execution_confinement` constraints are unestablished, its
   conditions stay unmet (`prepared_conditions_unmet`) and it produced no job inputs.

## May not claim

- **Execution confinement is unestablished** on both profiles: no owner maps the hosted container to
  a confinement class, and r2 removes the constraint from the revision rather than establishing it.
- `mutation: none` is **unestablished**. The claims above cover only the boundaries actually
  observed and verified.
- The serving route, the payer, the subscription billing guard and any receipt by the provider:
  Host-reported (item 5), not checked, not independently observed.
- Claude Code's `system[]` blocks and environment message: recorded by digest, not interpreted. The
  Host's own pre-dispatch checks (billing-header block shape, pinned tools digest, retry header) are
  the Host's, not this crate's.
- No acceptance condition of either world is established; the frozen candidate's conditions stay
  unmet. Running a qualification does not establish a review condition or make anything consumable.
- Not consumable: every envelope has `consumable: false` and `standing: owner_pending`.
- Not admitted: `org_admission: none_requested`; no production admission, integration or push.
- Grants are self-issued by the operator's own process.
- Nothing self-authored in runtime admission: the job, authority, packet, budget and routes come
  from `records/dev-vertical-h2-route-20261117/analyze.py`, not from this crate.
- The review's quality and correctness are unassessed; a schema-valid answer is not a correct one.
- Preparation blockers are carried verbatim and none is discharged; the revision's normative values
  (execution-evidence custody, independent observer) are requirements, not shown properties.
- With one or two runs, nothing generalizes.
- Shape of a real Claude Code body is observed in n = 2 runs, one offline
  (`e2e/org-step3-offline-1`) and one live (`e2e/org-step3-live-1`), 3 requests each: `messages[0]`
  is a single user text block; role-`system` messages sit at `messages[1]` and directly after
  each tool-result-only user message (string content, except the last message of each body, which
  is a system message with a text-block list); assistant turns hold `tool_use` blocks (up to
  three in one turn, live) answered by a user message of `tool_result` blocks. The checker tolerates exactly those positions and
  content types and refuses extra content blocks in `messages[0]` and any other user content. Body
  shapes of other live runs, other prompts or other Claude Code behaviour are `not established`.
