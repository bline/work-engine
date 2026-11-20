# Cache affinity and effort adaptation

Status: idea extraction, 2026-11-15. Source: [scratch notes](../scratch.md),
lines 305–430 and 765–918. The operator raises cache loss as a routing cost and
asks about reuse across model, effort, and billing boundaries. Provider rules
and SDK support below are source-reported claims requiring qualification.
See the [collection index](README.md).

## Idea

Choose inference routes with accumulated cache opportunity in view. A nominally
cheaper fresh-request route may be more expensive for a conversation with a
large reusable prefix. A successful failover also need not imply immediately
switching back when the original route recovers; repeated switching could
repeatedly sacrifice locality.

The notes distinguish client session identity from provider cache identity.
Preserving a session may preserve instructions, tools, and conversation prefix,
but does not prove a hit. Replacing a Pi container need not inherently lose
reuse if the relevant provider conditions survive.

The economic example in the source compares a cache write at 125% of an
uncached input baseline with a read at 5%, producing a 25:1 ratio. These are
unverified, model-specific pricing assertions from the conversation, not a
current rate table. The durable idea is to compare actual cache economics,
including write cost, rather than route labels or nominal input prices alone.

## Boundaries to distinguish

| Boundary | Source-reported finding or unresolved question |
| --- | --- |
| Prompt construction | Matching rendered prefixes, including tools and instructions, matters; an identical client conversation may render differently |
| New client session or worker | Cache is described as independent of the client session object; measure whether reuse survives recreation |
| Model change | Do not assume reuse; the source describes Claude entries as model-specific and warns about OpenAI model changes |
| Effort change | Later passages describe append-only or per-message mechanisms that may preserve earlier cache on supported configurations |
| API versus subscription | Sharing remains experimentally unestablished for both providers |
| Account, organization, workspace, or region | The source reports provider-specific isolation scopes; qualify the actual route and account boundary |
| Retention and timing | Eligibility depends on provider retention rules and elapsed time, not just a stable prefix |

For OpenAI, the notes report prefix matching, machine locality, organization
and region isolation, and cache-preserving in-conversation effort updates.
For Anthropic, they report exact prefixes and breakpoints, workspace scope,
model-specific entries, and short or extended retention with different write
costs. These details remain verification targets rather than portable rules.

## Effort adaptation within a model

The later conversation identifies changing effort while keeping the same model
and warm prefix as a promising optimization. It describes supported OpenAI
in-conversation updates and Anthropic per-message effort updates that leave the
top-level setting stable. It also reports relevant Pi 1.5.0 compatibility support
and a beta feature for certain Claude models.

This qualifies the earlier warning that thinking or effort changes can
invalidate cached messages: the update mechanism and model configuration
matter. The extraction does not treat every effort change as cache-preserving,
or model switching as equivalent to effort switching.

## Adapter-specific risks

The source reports `cacheRetention: "none"` in Work Engine's Pi paths. It says
this can suppress a cache key in the legacy Codex transport and can affect
explicit caching differently in the public Responses transport. The idea is to
review the actual cache policy during transport changes rather than copying a
setting across adapters by name.

It also reports that Pi's Claude OAuth path adds an identifying system
instruction and transforms tool names differently from the API-key path.
Thus a billing switch may change the effective prefix even if Work Engine's
conversation is unchanged. A miss under those conditions cannot by itself
establish provider-level cache isolation.

Related: [usage evidence](usage-and-attempt-evidence.md) and the
[qualification experiment matrix](qualification-experiments.md).
