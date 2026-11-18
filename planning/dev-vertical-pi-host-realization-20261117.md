# Pi Host realization with per-run providers: revised plan and sizing delta

2026-11-17, revision 1. Dev vertical. Replaces the Claude Host direction
(`planning/research-lifecycle-h2-claude-realization-20261117.md`, branch
`claude-realization-20261117`) after the operator's correction: Pi in its
Docker container stays the single native realization, and the provider is
chosen per run from Pi's four (`claude-sdk` subscription, `anthropic` API key,
`openai-codex` subscription, OpenAI API). Basis: the research-lifecycle
owner's direction (board 5874), the P0 sizing
(`planning/research-lifecycle-h2-pi-realization-sizing-20261117.md`, placement
A) and the in-container probe
(`records/dev-vertical-h2-route-20261117/pi-claude-probe/README.md`). No code
in research-lifecycle crates until the owner has read this.

## What the probe settled

Pi with `claude-sdk` runs inside the Pi Host's pinned container (`--network
none`, read-only, uid 1000, the Pi Host's own limits). The Claude CLI runs in
the container; its `ANTHROPIC_BASE_URL` is an in-container loopback forwarder
that frames each HTTP request onto the container's stdio; the Host injects
the real OAuth bearer (the container holds a dummy), retains the exact body
before dispatch, and forwards. Billing stays in plan; the requests carry no
account identifier.

## Target shape

- **One container process tree**: the receiver (Pi via its SDK) plus, for
  `claude-sdk`, the Claude CLI it spawns. All inference leaves through the
  Host over the framed stdio bridge; tools are executed outside the
  container (Host-served today; relayed to the lifecycle actor under the task
  host).
- **Two wire families in the Host**, selected by the run's provider:
  Responses (`openai-codex`, OpenAI API; exists) and Anthropic Messages
  (`claude-sdk`, `anthropic` API; new). Each family has its body policy,
  stream reader, terminal and usage settlement; the attempt ledger, funding
  failover, retention and screening stay single and shared.
- **Credential custody** stays Host-only on every route: `claude-sdk` bearer
  injected over a dummy; `anthropic` and OpenAI API keys and the ChatGPT
  access as today.
- **Under the task host**: the Pi Host speaks the lifecycle protocol subset to
  `lifecycle-service` (P0 placement A: H1 controller mode, H2 relayed tools,
  H3 per-turn capture, H4 bounds, H5 Docker observation). The realization is
  `pi-host`; the session protocol digest names the Pi Host protocol; the wire
  format is recorded per request and keys the verifier's projection (S1).

## Work items and sizing

| # | Item | Owner | Lines (est.) | Reuse |
| --- | --- | --- | --- | --- |
| W1 | Anthropic Messages family in the Host: endpoints (`api.anthropic.com/v1/messages`; bearer for `claude-sdk`, `x-api-key` for `anthropic`), header allowlist, body policy (model, stream, exact tools set with `mcp__pi__` names and the placeholder, key allowlist, no `mcp_servers`/`container`, `tool_choice` auto, canonical prefix-extension), Messages SSE relay, terminal and usage (incl. cache tokens), catalog rates for the chosen Claude models, rate-limit headers retained | manual-host | 600-1 000 | Claude Host proxy checks and SSE parser; `native-realization` canonical rule |
| W2 | Receiver: loopback forwarder for the `claude-sdk` provider (CLI HTTP to framed fetch), dummy credential in tmpfs `HOME`, runtime manifest pins the extension, the Agent SDK and its `claude` binary; `anthropic` API route uses the existing fetch interception | receiver, manual-host | 200-350 | probe `forwarder.mjs` |
| W3 | Attempt mapping for the CLI: each Pi provider call is one logical request; Claude Code's own retries become Host attempts under the existing failover/refusal rules (or are disabled if a switch exists) | manual-host | 100-200 | ledger |
| W4 | Standalone qualification on the hosted analysis profile: offline Messages fixture upstream, refusal cases, one live `claude-sdk` run (H2 treatment snapshot, non-H2 task) | dev vertical | harness 200-400 | Claude Host fixture, a4 harness |
| H1-H3 | Pi Host controller mode, relayed tools, per-turn capture | manual-host | 1 000-1 600 | Claude Host `serve`/capture logic as reference |
| H4-H5 | Bounds inside consumer limits; Docker observation and guard | manual-host | 200-350 | P0 |
| L | `lifecycle-service` runner for the Pi Host (from the branch's `claude_host.rs`), realization identity, `CaptureHostInput` | lifecycle-service | 200-400 | branch |
| S | Store and constants: allow-list `{Codex, Pi Host}`, scope literal by protocol digest (Codex byte pin kept), moved constants | lifecycle-store, native-realization | 50-100 | branch |
| T | Task host: projection keyed by the recorded wire format (Messages or Responses), tool-name prefix `mcp__pi__` for Pi's Claude routes, C1 checks | task host | 80-150 | branch |
| Q | Composed offline qualification (task host, lifecycle-service, Pi Host, fixture upstreams for both families) | all | 500-900 | branch composed test |

Total about 3 100-5 450 lines, of which W1-W4 (about 1 100-1 950) give a working
hosted Pi-on-Claude path on their own, before the lifecycle work. Against P0
(3 300-5 800) the totals are similar: the Anthropic family is new, while much
of H3, L, S, T and Q is reusable from the branch.

## Sequence

1. W1-W4 on the hosted analysis profile (dev vertical only; no
   research-lifecycle crate). Fable review. This is also the only route on
   which other lanes can run live Pi jobs this week (OpenAI exhausted until
   2026-11-21).
2. H1-H5 and L, S, T, Q on a new branch cut from the base, porting the
   reusable commits of `claude-realization-20261117`; Codex bytes and
   existing tests unchanged except the listed protocol literals. Fable review,
   then the owner's acceptance.

## Decisions needed

| Who | Decision |
| --- | --- |
| Operator | H2 model and effort among the `claude-sdk` models (R6 names Claude; the model is open). R9′ is now expected to be the Docker profile. |
| Research-lifecycle owner | Confirm the sequence and that the Claude Host branch's reusable commits may be ported to a new branch. |
| Dev vertical | W3: whether Claude Code's internal retries can be disabled, or are counted as attempts. |
