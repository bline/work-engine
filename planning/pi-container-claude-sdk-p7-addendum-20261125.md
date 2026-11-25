# Addendum: P7 needs on the Claude subscription container lane

**Status:** This addendum records what the operator-approved implementation
covers for P7's needs. It leaves the reviewed
[core plan](pi-container-claude-sdk-implementation-plan-20261124.md) unchanged and authorizes no live
request, smoke or P7 launch.

P7 run 1 ([perspective-intent research](../experiments/perspective-intent-research/p7-run1/))
stopped at its smoke. The reason was the pi-subagents route: it injected a
supervisor bridge and an acceptance contract, and no tool manifest from outside
the model was available. In the container lane, Pi runs directly under
role_host and every model request crosses the host proxy. Four needs were
identified.

| Need | Implemented offline | Evidence | Still open |
| --- | --- | --- | --- |
| Pin `claude-opus-5-5` at high effort | Yes. `CLAUDE_SDK_MODELS` contains `claude-haiku-5-5` and `claude-opus-5-5`, and the proxy refuses any other request model and any other first `message_start` model. | S0 `run-p7-notools`: `model: claude-opus-5-5`, `output_config.effort: high`, `thinking: adaptive` | The live response model string is unproven. |
| Fresh, tool-less session with a replaced system prompt | Yes. role_host now passes Pi's `--no-tools`/`-nt` and `--system-prompt` through. Each launch is a new Pi session and a new container with a fresh tmpfs home. | S0 `run-p7-notools`: `tools: []` (no deferred placeholder), the replaced prompt in the third system block, Claude Code's billing header and SDK identity before it | Billing (below) |
| Receipt evidence from the host | Yes. Each `claude_sdk_admitted` row has `system_sha256` (system blocks after the per-request billing header), `tools_sha256`, `tool_names`, `effort`, `thinking`, `betas` and `body_sha256`. Each disposition has `response_model`, `billing_verdict` and usage. `status` shows them under `egress.claude_sdk.requests`. | Proxy unit tests; real-container join test | — |
| A live Opus smoke under the P7 contract | Not done | — | Needs separate approval after the core plan's S3 |

**Billing risk.** The extension's README reports a 2026-10-04 observation:
Anthropic answered `400 Third-party apps now draw from your extra usage` when
requests carried Pi's default prompt, and accepted them within the plan only
with the extension's Claude prompt. A P7 run replaces the system prompt, so its
requests might be classified the same way. The lane fails closed. A 400, or a
response whose `anthropic-ratelimit-unified-status` is not
`allowed`/`allowed_warning`, releases no output, halts the turn and never falls
back to extra usage. But only a live request can show which classification
applies. Until then, P7 on this lane is **not established**. The first live
Opus request is the test; stop at the first refusal.

**Receipt fit.** These are digests, so they meet the core plan's rule against
raw bodies. A P7 receipt can compare `system_sha256` with the digest of the
intended prompt only if it computes that digest over the same canonical JSON
blocks: `pi_container._canonical(system[1:])`. Those blocks include Claude
Code's SDK identity block and Pi's environment suffix. So the receipt should pin
the observed digest from a calibration run rather than predict it.
