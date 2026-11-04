# S10 Pi/Docker real coding qualification — 2026-11-04

Status: accepted bounded private qualification. The paid attempt is complete;
this record does not accept full S10, M6, production integration or release.

The deterministic Rust manual host launched Pi's `openai-codex` subscription
provider inside Docker. The actual `gpt-6-luna` session selected a
`workspace_exec` command that corrected the disposable Rust parser's minute
multiplier from 1,000 to 60,000. Only `src/lib.rs` changed. All three original
tests passed, including malformed input and overflow cases, and the host reran
them against the exported source snapshot in a separate read-only validator.

The successful foreground run took 38.9 seconds: two provider requests, one
model-selected tool call, 2,280 input tokens and 150 output tokens. Both raw
responses reported HTTP 200, the expected model, completed status and explicit
usage. Inference, tool and validator containers were removed; the projected
access file was removed and its private directory was empty. The host receipt
was acknowledged. Root separately accepts this disposable qualification; the
immutable receipt's `owner_pending` field and absence of production domain
settlement remain unchanged.

## Configuration and authentication

Each job controls these locations explicitly:

| Field | Meaning |
| --- | --- |
| `coding.provider.lane.pi_config.root` | Operator-selected public Pi configuration directory; this bounded profile supports `settings.json` and empty `models.json`. |
| `coding.provider.lane.native_auth_source` | Selected native Codex authentication file; trusted Rust projects only access, expiry and account into private staging. |

No host Pi installation or separate Pi login was required. Pi calls the
subscription provider directly; it does not launch Codex CLI or read its
`config.toml`. `CODEX_HOME` remains relevant to an actual native Codex CLI
backend. API-key fallback, refresh and full-cache mounting were disabled.

## Source, review and evidence

The private source is based on `b59a02159760e4acf074230d2691ab6afea66fec`.
The repaired 26-file manifest is
`44808fb00a8d7e45cfd43b3d3ab10b78e92cc54a4f8e59382f82624f7771d197`;
the executable is
`772217e03b8e5f6080d1c13aee851becdc7cced7801e7406a225356d20e59846`.

An isolated Astra/high/default reviewer with an observed read-only grant found
three P2 defects: recovery without final acceptance, normalized SDK defaults
concealing missing usage/status, and exported bytes differing from tested
bytes. The retained builder repaired them. A fresh read-only replacement
reconstructed the bounded review episode and accepted all three repairs; it
did not claim another complete independent review. Unchanged boundaries rely
on the preserved initial review. Twenty focused Rust tests, five Node tests,
two actual provider-disabled failure-path cases, strict Clippy and the offline
build passed. Recovery interruption evidence is a durable-cut fixture rather
than an observed OS crash.

The [Root result](/home/bline/.local/state/work-engine/s10-supervision-20261103/real-job-20261104/ROOT-RESULT.json)
binds the source, reviews, exact runtime admission, task diff and receipt.

## Budget and remaining work

The original $20 aggregate cap and first three-hour execution clock were
preserved. Recorded native cost was $18.0043678 before publication and closure;
the two runtime requests settled to a conservative catalog proxy of $0.000685.
These are cost proxies, not measured subscription quota or billing. Reporting
and closure tail was not yet settled at this observation. Earlier preparation
guard crossings and review-allocation variance remain in the assignment.

The first launch refused before state creation because Root omitted the
caller-created empty state directory. The pre-entry correction reused the
exact job and original deadline; no provider request was replayed.

This closes the real-coding-proof gap left by the
[host component result](rust-lifecycle-s10-pi-docker-host-component-result-20261103.md).
Private source integration, canonical Rust context replacement and the actual
MCP/Dot-to-Pi combined workflow remain separate work. The other owner's board
4354 result closed its MCP attempt partially; this coding proof does not
upgrade it. No further paid work is launched by this result.
