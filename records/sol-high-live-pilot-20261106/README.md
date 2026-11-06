# Sol/high pilot: admission failure and preparation repair

Recorded 2026-11-06 UTC. **Live qualification not established. Preparation
repair accepted after fresh Astra/high review.**

The user approved the [one-job $45/two-hour proposal](../../planning/sol-high-live-development-pilot-budget-20261106.md)
at 04:55:20.877Z. The previous provider-free round remains closed. This round
submitted one operation and did not enter a provider or project native OAuth
credentials. It produced no model-authored patch. No second live job is included.

## Original operation and evidence

The exact operation is
`83ed5f9dfc10c0a76fbc923cf46251613e2775527c61d93298408238d9cc6051`;
the original receipt is
`fb21afebaed0e946b44de48ab54f1749fe105bb50771578f0a1538a7514bd56a`.
Its result is `not_established`, with native `owner_pending` unchanged. Exact
repeat and actual server restart/read/repeat recovered the same operation and
raw receipt. Provider requests and workspace tool requests were both zero.
The actual transport exercised here was command/server submission and reads;
live MCP submission was not exercised.

The submitted canonical budget used sequence `2`. The existing continuity
owner requires sequence `1` and refused it before tool preparation, native
projection or provider entry. The generic stale/exhausted/misbound error does
not establish an expiry failure: this run's concrete mismatch was sequence.
The request sequence was incorrectly used as a rebinding counter by Root.

Two earlier input failures also remain part of this round. An absolute deadline
chosen 600 seconds ahead failed the server's independent live admission window
of more than 5 and at most 240 seconds. Root rebound the never-submitted job to
230 seconds with fresh paths and budget evidence, preserving the old artifacts.
Then a non-UUID request ID failed command validation before connection. Root
corrected that client input; job identity, authorization and credential stayed
the same. Neither failure submitted an additional operation.

The exact producing binary was `coding-qualification`, SHA-256
`b4553a43d090009d410c64c0b24dbc03a230a6c8817e3719a204332b7260256b`.
Its source was the accepted `1a488845` code, with 22 material paths checked
against `e2dc3c6f`. Producing binaries and job artifacts remain immutable.
Later preparation repairs do not retroactively change that failed run.

## Independent acceptance boundary

Preflight confirmed that the current Host mounts `profile.acceptance` at
`/acceptance.rs` in both tool and validation containers. A verifier there is
visible to the implementing agent. This round therefore supplied a disclosed
non-authoritative smoke file and froze the true verifier privately for use
after export, outside all product mounts and later model calls. Host smoke
validation alone could not establish independent acceptance.

Private controls passed for a valid appended Rust test module and rejected
unchanged, empty and damaged original source. The active verifier accepts one
or more meaningful new tests; isolated one-test and three-test controls both
killed the midnight-fraction and fractional-rounding mutants using only the
new module's tests. Those controls ran in a restricted, network-disabled
Docker validation container. They are local control evidence, not acceptance
of model output: this operation never produced a candidate.

The Node inference image could not run the initial host-built verifier because
of a glibc mismatch and lacked the C linker for in-container compilation. Those
failed attempts are retained. The final control route used the existing fixed
tool/validation image and compiled inside it. No image pull or Pi upgrade was
needed. The Host's visible smoke file remains a limitation to account for in
future acceptance designs.

## Bounded repair and remaining work

Private repair `893a3d62` makes the preparation boundary reject impossible Sol
budget and deadline inputs before producing a bundle, while preserving existing
execution, authority and offline contracts. Original sequence-2 and overlong
deadline controls fail on the original source; the corrected checks, focused
tests, formatting and affected lint pass. One pre-existing dependency
`clippy::needless_borrow` allowance remains explicit. Fresh Astra/high/default
[review](REVIEW.json) accepted that exact repair and this bounded failure record;
[Root acceptance](ROOT-ACCEPTANCE.json) preserves the held live requirements.
Its one P3 suggestion to assert exact test diagnostics is deferred; successful
refusal, no output directories and source inspection establish the current
claim. Root's
client UUID correction remains an input repair; no new command schema is needed.

Sol/account eligibility, actual high-reasoning responses, one workspace effect,
SDK reset/recovery and independent acceptance of the requested timestamp tests
remain untested. The original timestamp module is unchanged. The
[Pi risk register](../../docs/pi-risks.md) retains its other open requirements.
This outcome does not establish unattended-development readiness.

## Custody and accounting

The builder collected the original server, reconciliation, restart and readback
processes. A first 30-second restart custody bound was too short and its failures
are retained; a separately bounded restart read recovered the original result.
Exact selected inference/tool/validator container absence checks passed.
Root independently matched 25 indexed original-result artifacts, exact
read/repeat bytes and six recorded container names plus the exact job label.
The separate reviewer completed and was retired. Root's focused composition
checks passed against shared baseline `d824a35f` plus the exact two-file repair.
The [validation record](VALIDATION.json) retains those checks.

The $17.680004 product hold was reserved conservatively and released as a
planning hold after terminal zero-entry evidence and collection. This does not
claim account settlement. [Accounting](ACCOUNTING.json) records the observed
pre-publication proxy and closing reserve, including preparation from
04:43:55.259Z, supervisor and builder turns, review, failed attempts and repairs.
Closing tails remain unasserted as settled; shared account quota is separate.

Private authority, exact inputs, controls, receipts and continuation are under
`/home/bline/.local/state/work-engine/sol-high-live-pilot-budget-20261106T0444/`.
They contain private operator/account bindings and are not copied wholesale
into this repository record. `assignment.json` owns current unfinished work;
the exact review and acceptance are copied here. The private `TERMINAL.json`
records final publication, budget disposition and release of all owned claims.
