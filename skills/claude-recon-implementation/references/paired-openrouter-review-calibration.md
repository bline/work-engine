# Paired OpenRouter review calibration

Use this mode to compare two deployable native-Claude configurations without
using Anthropic subscription capacity:

| arm | gateway | upstream | experimental betas | workflow authority |
| --- | --- | --- | --- | --- |
| realtime | OpenRouter realtime | Anthropic 1P | enabled | caller-facing review |
| batch | OpenRouter Batch | Anthropic 1P | disabled | shadow measurement only |

This is intentionally a practical configuration comparison. Transport and the
experimental-beta setting are bundled; do not describe a difference as a pure
batch effect. Defer aggregate interpretation until the registered campaign is
complete and findings have been adjudicated. Neither arm's benchmark result
approves the reviewed production slice.

## Evidence boundary

Before the realtime provider call:

1. bind one immutable review subject and one exact review packet;
2. create separate, byte-identical isolated `CLAUDE_CONFIG_DIR` clones for the
   two arms so realtime mutations cannot change the batch starting state;
3. retain a content-safe manifest describing the frozen config;
4. create a fresh Anthropic-only routing attestation; and
5. register the pair durably.

The batch command must be byte-for-byte the same native Claude command as the
realtime command. Only the transport environment, isolated config path, and
`CLAUDE_CODE_DISABLE_EXPERIMENTAL_BETAS=1` differ. Never put the realtime
result, its findings, or remediation into the batch packet.

Use the same `--output-format json` and `--json-schema` arguments for both arms.
The semantic payload must use the review-bench v2 classification fields:
`verdict`, `findings`, `verified_claims`, and `observations`. This preserves
defects separately from acceptance evidence and non-defect notes.

Initialize the ten-pair campaign once:

```bash
python3 skills/claude-recon-implementation/scripts/paired_review_evidence.py init \
  --campaign-root '<campaign-root>' \
  --campaign-id '<campaign-id>' \
  --target-pairs 10 \
  --model 'anthropic/claude-sonnet-5-20260630'
```

Run each review through the paired front controller:

```bash
python3 skills/claude-recon-implementation/scripts/claude_paired_review.py run \
  --campaign-root '<campaign-root>' \
  --pair-id 'pair-01' \
  --ordinal 1 \
  --subject-identity '<immutable-checkpoint-identity>' \
  --subject-artifact '<subject-binding.json>' \
  --review-packet '<review-prompt.md>' \
  --config-manifest '<content-safe-config-manifest.json>' \
  --realtime-config-dir '<realtime-config-clone>' \
  --batch-config-dir '<batch-config-clone>' \
  --routing-attestation '<routing-attestation.json>' \
  --working-directory '<immutable-review-workspace>' \
  -- claude -p \
    --effort medium \
    --model sonnet \
    --no-session-persistence \
    --tools 'mcp__codebase-memory-mcp,Read,Glob,Grep' \
    --output-format json \
    --json-schema '<review-bench-v2-schema>' \
    --dangerously-skip-permissions
```

Do not put the review packet in the trailing Claude command. The controller
copies the packet into the registered pair and appends that exact UTF-8 content
to both Claude argument vectors. This makes packet identity executable rather
than merely documentary.

The controller registers the pair before provider entry, launches the detached
batch worker and detached finalizer, then runs realtime synchronously and
returns its exact stdout and exit status to the caller. This ordering prevents
a realtime-only attempt from becoming invisible in the denominator if the
caller exits or the shadow later fails. The batch arm never changes the review
returned to the workflow.

Runtime evidence is written under
`<campaign-root>/pairs/<pair-id>/runtime/`. The background worker records
infrastructure failure, Claude failure, or complete batch lineage atomically;
the finalizer converts terminal arm evidence into `pair-receipt.json`. If
automatic finalizer launch fails, the controller receipt explicitly marks
`manual_finalization_required`. A missing or failed shadow remains a registered
pair in the campaign denominator.

The lower-level `paired_review_evidence.py`, `claude_transport.py`, and
`claude_batch_review.py` commands are recovery and diagnostic interfaces, not
the normal calibration entry point. The current proxy cannot reattach after a
process crash to a paid batch already submitted. Reconcile every recorded batch
ID manually; never replay merely because the local worker stopped.

Automatic finalization fails closed unless both arms share the subject, packet, command,
initial config digest, model, Claude version, and routing attestation; use their
expected beta settings; and retain successful output and complete batch
request-to-terminal lineage. It copies the raw evidence into the pair artifact
directory before producing `pair-receipt.json`.

After all ten pairs, audit completeness before adjudication:

```bash
python3 skills/claude-recon-implementation/scripts/paired_review_evidence.py audit \
  --campaign-root '<campaign-root>'
```

The audit succeeds only for exactly ten unique ordinals with ten
comparison-ready pair receipts. Compare evidence-backed findings, verified
acceptance claims, observations, verdicts, tool behavior, latency, tokens, and
cost after human or executable adjudication. Do not compare prose by lexical
similarity and do not silently discard failed or missing shadows.

After the audit succeeds, produce the descriptive comparison:

```bash
python3 skills/claude-recon-implementation/scripts/paired_review_evidence.py compare \
  --campaign-root '<campaign-root>'
```

This records exact structured-payload matches, verdict matches, finding-count
deltas, and which pairs require adjudication. Exact equality and counts are not
semantic truth, precision, recall, or production approval.

## Successor calibration roadmap

Do not make permanent pairing a condition of ordinary Claude review. Use the
initial campaign to choose a production configuration, then reopen calibration
when a change can materially affect inference requests, context, tools, or
review-result semantics.

Keep each successor calibration in a separately versioned campaign. Preserve
the earlier artifacts under their original protocol; do not pool results merely
because they use the same model or review subjects.

### Initial transport campaign

Run ten prospective pairs through the two OpenRouter configurations documented
above. This establishes practical compatibility and exposes gross semantic,
tool-use, reliability, latency, or cost differences. It does not establish
statistical equivalence or isolate transport from the beta setting.

### Context-lifecycle integration

After Claude review is migrated to app-server context lifecycle, use:

- five representative frozen anchor subjects from the initial campaign through
  both transports; and
- approximately five new prospective subjects through both transports.

The anchors provide a longitudinal bridge across the lifecycle change. The new
subjects exercise the integrated system under realistic future work. Keep the
pre-migration and post-migration configurations distinct in provenance,
including context ownership, compaction behavior, tool exposure, session mode,
and recovery mechanics.

This integration directly changes context and compaction mechanics, so a full
ten-pair successor campaign is proportionate even when the initial transport
campaign showed close agreement.

### Claims-system integration

Claims integration asks a different primary question: whether supplied claims
change review quality. Test that question with five to ten frozen anchor
subjects reviewed with claims absent versus claims present while holding one
selected transport fixed.

Add three to five realtime/batch bridge pairs with claims present to detect an
interaction between claims delivery and transport. Expand that bridge to ten
pairs only when the preregistered material-divergence rule activates. Ten new
transport pairs alone would mostly retest transport and would not identify the
effect of claims.

Keep pre-claims development evidence, claims-aware replays, and naturally
occurring post-build claim evidence distinguishable. A claims-aware replay is
not historical production behavior.

## Adaptive expansion rule

Freeze the expansion rule before inspecting successor outcomes. Treat any of
the following as material divergence:

- a verdict difference;
- an adjudicated blocking finding reported by only one configuration;
- a false high-confidence verified claim;
- a systematic difference in usable repository evidence or tool behavior;
- a transport-specific malformed result, context failure, or incomplete run;
- a severity or blocking-label difference that could change workflow action.

If a bridge contains material divergence, expand it to ten pairs and diagnose
the affected boundary. If the bridge contains none, retain the smaller result
as bounded compatibility evidence and stop; do not describe it as proof of
equivalence.

Reopen a transport bridge after changes to:

- Claude Code or the selected model;
- OpenRouter request or Batch response handling;
- beta flags, context management, compaction, or session continuity;
- system prompts, output schemas, tool definitions, or tool-loading behavior;
- provider pinning, routing attestations, caching, or inference parameters.

Ordinary implementation changes that leave those mechanics intact do not
require paired review merely because Claude reviews the change.

## Evidence allocation

The intended sequence is:

```text
initial transport calibration
    10 realtime/batch prospective pairs

context-lifecycle calibration
    5 frozen anchor pairs
  + 5 prospective pairs

claims calibration
    5–10 claims-absent/claims-present anchor comparisons on one transport
  + 3–5 claims-present transport bridge pairs
  + expansion to 10 bridge pairs only after material divergence
```

This allocation supports an engineering deployment decision and helps locate
large interactions. The sample sizes are descriptive and too small for a claim
of statistical equivalence. Human or executable adjudication remains the owner
of finding truth, and no calibration result accepts production work.
