# Hosted review: offline exact-scope conformance only

## Current build: no selectable Hosted route

The public commands `review-job-inputs` and `review-result-check` refuse the
following **historical-only** pairs. The names still identify the same wire
profiles; they do not select successors. Refusal writes no bundle or envelope.

| Profile | Requested model | Wire shape | Historical revision |
|---|---|---|---|
| `openai-codex` | `gpt-6-luna` | OpenAI Responses | `hosted-route-qualification-r1` |
| `claude-sdk` | `claude-haiku-5-5` | Anthropic Messages | `hosted-route-qualification-r2` |

To interpret archived results or replay when you have retained inputs, use the
**exact source/checker pin of the particular invocation** and its corresponding
Host pin, not this build. See the [R1 derivation](fixtures/hosted-route-qualification-r1/DERIVATION.md),
[R2 derivation](fixtures/hosted-route-qualification-r2/DERIVATION.md), and the
[dated qualification record](../../../records/org-hosted-review-20261117/README.md).
The committed remediation-6 crate is at `533b9cc0dd0843372b84f35901d3c54c7dd904d8`;
use the remediation-5 source hashes in the live receipt for the original live
check, and the Host pin in `PINS.txt` (`5af34cbf7c177d216c5b19dc6ca8c96e258052f8`).
The r6 envelopes report live `checks_passed` and offline `strict_json` failed
(with dependent checks not run). Neither result is consumable or exact-scope
evidence. These are historical observations and predicates, not claims of the
current build. Some replay inputs (including snapshots, tasks, bodies and raw
state) were not retained in the record. Availability of pinned source does not
establish reproducibility of the historical run.

## Unselected Messages artifact library

`milestone-a-messages-artifacts-v1` is a library comparison revision, not a
route recognized by either command above. It reads Git objects to materialize
an independently supplied baseline, subject and scope, renders the supplied
contract into a bounded task, and computes exact manifest bytes. Its comparison
checks those bytes, retained Messages bodies (one recorded attempt per request),
and the strict answer contract for that exact subject. This compares inputs and
outputs; it does not establish assignment or Host custody, process identity,
protected exposure, provider entry or settlement, an organizational admission,
or a result usable by either campaign consumer. The library does not publish
or start a job. Archival and synthetic conformance behavior below is unchanged.

## What the private conformance exercise checks

The two pinned test worlds under `fixtures/hosted-offline-exact-v1` are derived
individually from R1 and R2, with only run identity, obligation scope and both
condition propositions changed. Neither world is a public revision or accepts
caller-supplied requirements. They compare synthetic review artifacts at the
original builder projection and campaign terminalization boundaries, without
establishing either condition.

- The writer verifies commit→tree, the single parent, and SHA-256 (**P**) of
  the actual Git diff bytes. Those bytes are written to `.review/patch.diff`.
  The selected prepared obligation's `scope.digest` must equal **S**, the
  review episode codec's digest of precisely `commit`, `tree`, `patchIdentity`
  for the verified tuple. S is not P and does not hash `.review/subject.json`.
  Snapshot member, secret, size, prompt and read-budget checks still apply.
- The checker prepares and freezes again from the expected fixture revision
  and compares S of the record's tuple to that obligation scope. Separately it
  checks that the job pins the raw manifest, the manifest matches the record,
  and its patch entry equals P. This is artifact comparison, not a second Git
  audit or an observation that a Host served those bytes.
- It separately compares job/authority, limits, provider/model, blockers,
  contract and transport, task, wire body plus exact suffix, and each attempt's
  body digest. The answer stays a string; its own SHA-256, strict JSON shape,
  result contract, parsed subject digest and three fields are checked.
  Grounding counts are informational only. Messages-wire insertions are
  digested, not interpreted; thinking is neither answer nor evidence.
- The unchanged frozen requirements are prepared as a counterfactual, not
  materialized. Responses: `refused`, two known provider-family
  incompatibilities. Messages: `prepared_conditions_unmet`, zero known
  incompatibilities and two unestablished confinement blockers. Neither
  produces job inputs. The two chosen worlds keep every preparation blocker,
  including scope-to-subject; artifact equality does not discharge it or
  enact the organization authority's S1 admission decision.

Offline envelopes have a distinct schema and proposition. On both pass and
failure: `standing: owner_pending`, `org_admission: none_requested`, and
`consumable: false`. Body/request/attempt/settlement fields, when present, are
synthetic, not Host or provider observations. The exercise establishes no
confinement, mutation or custody claim, provider receipt, serving route,
payer, grants, review quality, acceptance, or production effect.
