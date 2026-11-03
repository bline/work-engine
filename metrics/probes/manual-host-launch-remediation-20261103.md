# Native launch remediation checkpoint, 2026-11-03

**The complete launch path remains unqualified. The attempt stopped at the
feasibility checkpoint; no trial allocation or episode ran.** A small adapter
patch did not close the required orchestration boundary. There is no efficiency
estimate from this work.

The user [approved an additional $25](../../records/manual-host-launch-remediation-approval-20261103.json)
at 20:34:37.833Z, raising the total authorization ceiling to $225 while retaining
earlier costs and the original trial allocations. The slice allowed $5 for native
feasibility, $10 for conditional implementation, $5 for review/qualification and
$5 protected completion, with a first checkpoint at $5 or 45 minutes and a
three-active-hour cap. It did not authorize a new control broker or isolation
platform. Preparation under the preceding “Proposal?” request is included from
20:30:00.555Z; approval did not reset it.

The [result JSON](manual-host-launch-remediation-20261103.json),
[evidence projection](manual-host-launch-remediation-20261103-evidence.json),
[fresh review](../../reviews/manual-host-native-launch-20261103/feasibility-result.json)
and [review provenance](../../reviews/manual-host-native-launch-20261103/provenance.json)
preserve the observations and their limits.

| Property | Evidence and outcome |
| --- | --- |
| Named native permission selection | Established with an experimental client handshake and explicit selector |
| Controlled filesystem restriction | Allowed sentinel read succeeded; excluded reads/writes failed in the tested command executor; a model command also verified excluded-read denial |
| App/connector restriction | Explicit app controls removed callable apps in metadata; disabling configured servers/plugins alone left 305 app tools |
| Usable model execution | Established after removing an empty environment selection that disabled execution |
| Root orchestration transport | Failed in the tested model route: AF_UNIX socket creation returned EPERM |
| Complete frozen startup inputs | Unestablished; global AGENTS remained a reported source |
| End-to-end common root/child qualification | Unestablished |

The decisive model probe read its allowed public sentinel successfully. Its
other command attempted only a handshake with an owned app-server socket; it
requested no excluded file. The raw tool output records exit 1 at
`socket.socket(AF_UNIX, SOCK_STREAM)` with `Operation not permitted`, despite
the requested socket allowlist and proxy configuration. The failure happened
before pathname connection or any application RPC. The native history omitted
this second commandExecution item, so the evidence projection preserves its raw
custom-tool call/output pair rather than treating the final prose as proof.

The released `codex_app.Connection` opens precisely this kind of socket.
`role_host` uses it to create children, and the Rust manual host invokes that
adapter. Profile selection alone therefore does not supply a functioning
isolated orchestration route. If connectivity becomes available, the server-side
file, thread and child-grant boundaries will still need evidence: the client
forwards arbitrary RPC methods. **No control-plane bypass was demonstrated.**

The separate Astra/xhigh/default reviewer returned **unestablished**. It found no
supported small correction closing the remaining gap in the bounded evidence.
The root accepts that review as support for holding trial entry, not as a
universal impossibility claim or source approval. The reviewer used exact source
ranges because graph tools were unavailable. Native filesystem read-only,
network-off and zero MCP/callable-app inventories were observed before its first
inference; its actual selected-turn settings matched the request.

Nine controlled process attempts and four non-scored model preparations are
retained. The first model turn was interrupted after its collector encountered
an unsupported history method; exact native terminal readback was subsequently
collected. Another attempt had no execution environment. A third declined a
socket-plus-file-read request; that refusal was not treated as OS evidence.
The final harmless-handshake attempt supplied the decisive raw output. These
were setup attempts, not extra nominal episodes or trial retries. Wrapper exit
0 means collection completed, not that an inner qualification command passed.

A partial adapter candidate added optional profile selection and startup gates.
Its preliminary Python checks passed, but no Rust build, shipping review or
complete native qualification established it as acceptable. Its exact patch is
archived in private study state with SHA-256
`5c7123b5ce2aea0f344ae57790c7f4cdf5e24c58cbe4e2c431afa186af622455`.
The isolated source files were restored to their baseline. No runtime source
change, tool activation or production configuration change was accepted.

At **21:12:25Z**, the canonical reader observed **$4.5172616** across 95 priced
calls: controller $2.6281528, three probe actors $0.1355948 combined, and reviewer
$1.7535140. No calls were unpriced in that snapshot. The interrupted preparation
had no priced call recorded; that is not a settled-zero billing claim. Later
root reporting and closure calls are an unsettled tail. These are modeled,
speed-adjusted API-equivalent dollars, not subscription quota or an invoice.
The $0.075872 pre-initialization charge is already included; runtime ceilings
were tightened by that offset instead of resetting the allowance.

The selected turns are terminal and collected. All nine probe process groups
and the review collector recorded no remaining processes. The original
registration is unchanged, no prospective arm amendment was activated, and
trial allocations stay closed. The next implementation needs a concrete
supported scoped orchestration route; a new broker or isolation platform would
be a separately scoped decision. No follow-on work is scheduled by this record.

After this native checkpoint, the user requested Docker/exec exploration and
retained builder/reviewer sessions, culminating in “perfect. perhaps we should
explore this” at 21:20:47.528Z. That later bounded route investigation remains
inside the original $25 ceiling and clock, with separate Docker artifacts; it
does not qualify or erase the native attempt recorded here.
