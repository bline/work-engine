**Judgment: reopen the plan’s workspace/tool realization before admitting another FT01 repair.** Retain the host Rust/native Pi/Docker architecture and current acceptance predicates. FT02 has a credible bounded repair. Neither finding is closed by this review.

Reviewed packet: `95fd8c0a7cd3329ed774ba39f76597ace637601352a74fb5af8dc31b574400ca`. Reviewed source12 entries: `ab623a081d6c76c84a613b45ef8b07005f00b6cf3219cab33faa97fea5f6bc9d`.

1. **FT01 exposes an unestablished enforcement boundary, beyond another path-check omission.**

   The [host tool consumer](/home/bline/cbm-work/docker-pi-completion70-20261114/rust/crates/work-engine-manual-host/examples/pi_native_tools_probe.rs:212) permits `write` and `bash` after generic frame inspection. The [receiver](/home/bline/cbm-work/docker-pi-completion70-20261114/rust/crates/work-engine-manual-host/receiver/pi-native-tools.mjs:33) then invokes the actual SDK builtins. The [write implementation](/home/bline/cbm-work/pi-artifact-handoff-20261113/pi/node_modules/@earendil-works/pi-coding-agent/dist/core/tools/write.js:30) creates directories and writes the resolved destination. The [bash implementation](/home/bline/cbm-work/pi-artifact-handoff-20261113/pi/node_modules/@earendil-works/pi-coding-agent/dist/core/tools/bash.js:48) spawns a shell with descendant execution.

   Checking path components would reject the reported encoded basename. It would not establish exclusion for names or contents computed during execution. Individually clean inputs can be combined into forbidden bytes; subsequent writes can complete a forbidden representation in an existing file. A pre-execution command-string check cannot establish those resulting filesystem states.

   The [current Docker profile](/home/bline/cbm-work/docker-pi-completion70-20261114/rust/crates/work-engine-manual-host/src/pi_native.rs:235) provides writable `/scratch` directly to these processes. Host publication checks therefore do not own every application-retaining operation. `noexec` does not prevent the observed interpreter-based workflow or its filesystem writes.

   This invalidates the prior advice’s sufficiency premise: private host name constructors and an exact-call handshake do not cover native writers and their descendants. Original A staying outside the workload prevents direct credential exposure; it does not establish the required exclusion predicate.

   The trusted-laptop clarification remains intact. This concerns application-created workspace names and bytes, which the current assignment explicitly includes. Treating all scratch files as unchecked candidates until export would change that acceptance boundary.

2. **A narrower execution profile is a credible candidate, but its sufficiency needs an explicit disposition.**

   The smallest plausible alternative is:

   - Workspace mutations pass through a trusted operation that owns the exact destination, resulting bytes, original-A check, and actual write.
   - Shell/build/test execution receives checked inputs and cannot directly mutate any filesystem in scope.
   - Generated artifacts and diagnostics return through bounded volatile channels and reach the workspace or custody only through checked publication.

   This could support real development work where a compiler emits its artifact through a pipe and tests consume checked inputs. It preserves a writable workspace through mediated operations. It does **not** preserve unrestricted writable-shell behavior.

   The SDK offers `operations` seams for write and shell execution, but those seams alone are insufficient. Its [output accumulator](/home/bline/cbm-work/pi-artifact-handoff-20261113/pi/node_modules/@earendil-works/pi-coding-agent/dist/core/tools/output-accumulator.js:39) also creates and writes temporary output files. The candidate must cover SDK-generated files, descendants, inherited descriptors, and every writable filesystem surface. An exact command allowlist is insufficient when that command executes mutable workspace scripts.

   Root should determine whether an explicit restricted profile still satisfies the intended complete workflow. A passing fixture alone cannot decide product sufficiency. If it does, realization can change while preserving acceptance. If generic writable bash/build programs are required, the plan needs a substantially stronger filesystem mediation mechanism.

   For that broader route, the minimum property is complete mediation: every relevant mutation either reaches an owner-controlled check of immutable exact operands and resulting state before execution, or is denied. Descendants must not bypass it through alternate syscalls, writable mappings, inherited handles, or namespace aliases. Neither hooks nor posthoc scans supply that property.

   **No such mechanism is established under the present Docker/no-privilege realization.** I do not conclude that every unprivileged mechanism is impossible. I conclude that treating one as an available small repair is unsupported. Its deployment permissions, enforcement coverage, useful behavior, and cost require a separate bounded feasibility decision. This review authorizes no privilege or infrastructure change.

3. **FT02 needs attributable acquisition, with explicit uncertainty before acquisition is known.**

   [CreatedContainer](/home/bline/cbm-work/docker-pi-completion70-20261114/rust/crates/work-engine-manual-host/src/pi_native.rs:204) removes its identifier while active. [Workload::create](/home/bline/cbm-work/docker-pi-completion70-20261114/rust/crates/work-engine-manual-host/src/pi_native.rs:282) arms it with the requested name before Docker returns. A name conflict can consequently remove a foreign container.

   A bounded correction should distinguish:

   - **Intent:** record a checked, attempt-specific operation identity before create. Intent grants reconciliation responsibility, not ownership of the requested name.
   - **Acquisition:** bind the returned container ID to that intent and verified creation metadata. Establish cleanup ownership before subsequent fallible publication or currentness checks. Remove the acquired ID rather than resolving the requested name again.
   - **Uncertain creation:** after lost ACK, timeout, malformed response, or publication denial, reconcile against daemon-observed identity and attempt-specific metadata. Matching a static label or name is insufficient. Preserve foreign resources; remove only attributable acquisitions. Ambiguity remains UNKNOWN, with no productive start or automatic create replay.

   An absent container at one observation does not settle an outstanding create. Cleanup failure also needs an explicit disposition; an ignored destructor error cannot close the interval. Required controls include foreign-name conflict, actual creation with lost ACK, denial after acquisition, name replacement, and failed removal.

The **next bounded scope** should establish one explicit workspace execution profile and its enforcement boundary before another full repair cycle. Prefer testing the restricted execution candidate first if root finds its proposed developer workflow sufficient. Its decisive evidence should combine an actual native write/build/test/export workflow with attempts to create encoded or dynamically derived names, assemble forbidden file content, and reach SDK/descendant output sinks. Refusal must precede the retaining operation; final artifact scans remain supplementary.

Invalidate that candidate if any writer bypasses mediation, enforcement requires authority outside the grant, or required development work cannot run within the profile. Root then owns the next design disposition; any reduction of protected scope or required workflow belongs to the acceptance owner. FT02 can be addressed independently without implying FT01 feasibility.

The source12 successful workflow remains useful revision-bound evidence. **FT01/FT02 remain OPEN; the full qualification matrix and complete source/proof review remain unfinished.** Earlier findings and historical limits retain their recorded dispositions.

I read the complete current rejection and prior design advice, and verified the four material detached-source files against source12 manifest `3b778ef412b6be7fd22b205f4e8f69076b7626b18152280d9fdf7912c0537f77`. The three inspected SDK files matched indexed SDK manifest `fcac09b8a577dba8b39f5ea32a280747d0294f7b5b39b0fb148d11d63ad45c0b`. Tier 2 graph generation `2026-11-14T17:58:28Z` lacks the four native paths; SDK paths are outside that project. Exact-source reads supplied the material evidence. This was static review, not runtime qualification or an exhaustive platform survey.

Session `01a1dc28-cda8-7431-9c5b-cf2b44f9029d` independently confirmed Astra/xhigh/default and read-only/restricted-network/never settings, including latest applied record **53**, SHA `f2295af11c2f0c9d40933a2ced2475ce61845cae9eda673cf479c7877667a4d3`. Listener **30012** used the verified rollout, pinned binary, and **185000/220000, step 1** policy. It remained quiet; explicit bound checks supplied pressure observations, latest **115,824 tokens, epoch 0**. The listener was interrupted and terminal **exit 1 fully collected**.

**No owned process, child, or uncollected tool result remains.** No source/evidence mutation, builds/tests, containers/workloads, provider/credential/network operations, or children occurred. Root remains sole collector, persistence, admission, and acceptance owner. The **2.5/mark 2** allocation and unchanged human **70** remain in force; spend reconciliation belongs to root. This completes the bounded design assignment and grants no implementation acceptance.
