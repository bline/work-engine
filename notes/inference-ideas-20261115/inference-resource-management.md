# Inference resource management

Status: idea extraction, 2026-11-15. Source: [scratch notes](../scratch.md),
lines 121–161, 207–215, and 595–761. The operator explicitly separates funding
resources and their failure policies from efficiency and quality. See the
[collection index](README.md) for provenance.

## Idea

Give inference funding and availability their own responsibility. Preferences
describe normal selection; failure policies describe what to do when a resource
cannot execute. Preferring subscription usage does not itself authorize paid
API fallback.

The conversation identifies four initial funding streams:

| Stream | Resource described in the notes |
| --- | --- |
| OpenAI Codex subscription | ChatGPT/Codex plan quota |
| OpenAI API | Credits, billing limits, and request budgets |
| Anthropic Claude subscription | Claude plan quota |
| Anthropic API | Credits, spending limits, and request budgets |

Each has distinct credentials, limits, observations, and accounting. Multiple
accounts per stream are a possible extension. Actual API charges, estimated
API-equivalent cost, and subscription availability remain different evidence;
one cannot stand in for another.

## Preferences, constraints, and failure policies

A preference can rank otherwise eligible resources. A hard constraint can
forbid paid use or cap it. A failure policy can stop, select an authorized
alternative, or return the lack of a feasible route to the workflow owner.

The source illustrates both subscription-first with no paid fallback and
subscription-first with a bounded API allowance. Its dollar amounts are
examples, not selected defaults.

Relevant states include temporary throttling, confirmed quota exhaustion,
authentication failure, unavailable account information, provider outage, and
uncertain request outcomes. Unknown availability is neither proof of available
capacity nor proof of exhaustion. OAuth expiry may call for host refresh;
an interrupted stream may require uncertainty handling rather than replay.

## Relationship to optimization

The proposed optimizer produces eligible ranked candidates. Resource management
finds an authorized, available funding route for those candidates or returns
a structured failure. It does not silently replace an unqualified model merely
to obtain funding.

| Situation | Distinction preserved in the notes |
| --- | --- |
| Another route looks cheaper while the current cache is warm | Optimization weighs the switch; resource management checks permission and funding |
| Subscription quota is exhausted | Apply the explicit exhaustion policy, including any authorized API limit |
| The model is unavailable on every permitted route | Return the constraint to the workflow; another model requires contract-permitted selection |

The source proposes deterministic enforcement for resource admission and
configured failure behavior. This is distinct from deciding what quality is
acceptable for a task. Route changes remain attributable new attempts with
prior usage and unresolved reservations preserved.

Open design questions include the representation and freshness of quota
observations, account selection, and exact integration with existing authority
and accounting owners. This extraction does not assign new storage or ledger
ownership.

Related: [optimization](inference-optimization-and-routing.md) and
[usage and attempt evidence](usage-and-attempt-evidence.md).
