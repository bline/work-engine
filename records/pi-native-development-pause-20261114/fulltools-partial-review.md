**Verdict: changes_required on source12. This is a partial review; complete qualification review remains unfinished.** Two static defects are established below. Neither invalidates the recorded successful workflow, but the first prevents accepting its complete first-retention boundary.

Reviewed subject: entries `ab623a081d6c76c84a613b45ef8b07005f00b6cf3219cab33faa97fea5f6bc9d`, source manifest `3b778ef4…37f77`, RESULT `121124ed…fb53`, evidence index `fcb1de86…2559`. I independently verified **all 1,810 indexed artifact hashes and all 1,148 current source-file hashes**, with no mismatch.

1. **FT-01 — High: native tool destinations bypass component-level exclusion before creation.**

   The host checks and retains the complete tool frame, then authorizes `write` or `bash` after checking only the tool name and that parameters are an object. See [receive_checked](/home/bline/cbm-work/docker-pi-completion70-20261114/rust/crates/work-engine-manual-host/examples/pi_native_tools_probe.rs:56) and [tool authorization](/home/bline/cbm-work/docker-pi-completion70-20261114/rust/crates/work-engine-manual-host/examples/pi_native_tools_probe.rs:212). The generic [string inspector](/home/bline/cbm-work/docker-pi-completion70-20261114/rust/crates/review-execution-evidence/src/pi.rs:228) interprets complete strings; it does not decompose tool paths.

   Static counterexample: original A is `abcdefghijklmnop`; a native write call supplies:

   ```json
   {"path":"/scratch/YWJjZGVmZ2hpamtsbW5vcA==","content":"clean"}
   ```

   The complete path contains neither literal A nor a complete canonical encoding of A. It passes that inspection. The [receiver](/home/bline/cbm-work/docker-pi-completion70-20261114/rust/crates/work-engine-manual-host/receiver/pi-native-tools.mjs:34) consumes the grant and calls the real SDK tool. The pinned SDK [write implementation](/home/bline/cbm-work/pi-artifact-handoff-20261113/pi/node_modules/@earendil-works/pi-coding-agent/dist/core/tools/write.js:30) resolves the path, creates parent directories, and writes the file. Its basename already retains the supported base64 representation of A.

   A later scratch-name observation may reject it, but that occurs after creation. Volatile workspace isolation does not establish the packet’s application-controlled name-exclusion predicate.

   **This is a recurrence of RR-01/NR-01 in the newly added consumer.** The corrected host directory constructor covers its own destinations; the operation inventory does not establish equivalent coverage for native tool destinations and descendants. Root owns the required design escalation before ordinary repair. The next bounded outcome must establish exclusion at the actual workspace retaining operations, including derived names, while preserving the required useful tools workflow.

2. **FT-02 — Medium: failed Docker creation can remove a pre-existing container that this attempt never acquired.**

   [Workload::create](/home/bline/cbm-work/docker-pi-completion70-20261114/rust/crates/work-engine-manual-host/src/pi_native.rs:282) arms `CreatedContainer` with the requested name **before** executing Docker create. [Its destructor](/home/bline/cbm-work/docker-pi-completion70-20261114/rust/crates/work-engine-manual-host/src/pi_native.rs:208) unconditionally attempts removal while active.

   Static counterexample: a container already occupies `we-pi-native-<binding.session>`. Docker create rejects the name conflict; `cmd_effect` returns an error; the armed guard invokes `docker rm -f` on that existing name. This invocation never obtained its returned container ID or established ownership.

   **Consequence:** cleanup can affect another attempt’s resource. Random UUID generation lowers the normal-run collision probability; it does not establish the ownership property of this public consumer. Cleanup needs an attributable acquisition or reconciliation boundary that distinguishes a conflicting existing resource from this attempt’s possibly unacknowledged creation.

The prior findings retain these dispositions:

| Finding | Source12 disposition |
|---|---|
| RR-01 | Original host root/registry basename counterexamples repaired; the invariant recurs as FT-01. |
| RR-02 | Separate currentness checks now precede create and start. Cleanup adds FT-02. |
| RR-03 | The specific `{}` usage defect is repaired. Required usage/cost fields and inspected optional fields now match the pinned SDK. |
| NR-01 | **Open**, including FT-01. |
| NR-02 | Complete bounded grammar acceptance remains unestablished by this partial review; no additional malformed-message counterexample is asserted here. |
| NR-03 | Preserve the specific shared-accounting resolution. Complete failure/closure qualification remains separate. |
| NR-04 | Preserve the bounded successful post-disposal resolution. |
| NR-05 | Preserve the prior revision-bound resolution. Source12 has verified byte bindings and useful final-run records; this interruption leaves complete semantic proof review unfinished. |

The qualification classes currently stand as follows. These are consequence-level dispositions, not a replacement acceptance matrix:

| Required class | Assessment |
|---|---|
| Useful native sessions | Actual 9→11→13-entry evolution is recorded; complete representation qualification remains open. |
| Retained original A | Live ownership and finite rotation/removal/expiry controls are present; full consumer coverage is not established. |
| Before first retention | **Blocked by FT-01.** |
| Capability and seals | Inspected private issuance, root identity, exact publication and registry joins support bounded authenticity; remaining qualification is not closed. |
| Crash and reconcile | Init, snapshot and closure cuts provide bounded evidence. Capture-failure/journal-loss cuts and repeated authenticated crash reconciliation remain unmet. |
| Whole interval | Lost-ack evidence preserves UNKNOWN correctly. Real native partial-frame and observer/journal-loss intervals remain unmet; FT-02 adds an ownership defect. |
| Independent effects | Root-owned final observer records six receipts/five deduplicated effects outside the actor. Fault observers remain actor-recorded. |
| Artifact scan | Recorded posthoc audits have declared bounds; they do not establish first-retention exclusion. |
| C4 and regression | Focused C4/native-consumer controls are recorded passing. The broad gate remains unmet. |
| Runtime profile | Inspected Docker isolation and transport limits support the bounded route. Trusted-host assumptions remain unchanged; FT-01 concerns application retention. |
| Independent review | This retained review supplies two findings; the complete review obligation remains unfinished. |
| Historical and live limits | Historical failures remain open; no live-provider world-history or retroactive acceptance claim is supported. |
| Complete development workflow | The exact run records real native tools, compilation, three tests, checked export and fresh sealed reads. Its full boundary acceptance is blocked. |

The four Claude `ProgramDrift` failures have a credible direct external-pin attribution: expected and observed CBM hashes differ, and the test is byte-identical to main. This is not unchanged-baseline execution. The isolated owner/host reruns support bounded recovery from their broad-run failures without erasing those failures. Minimum remaining work is a root disposition of the attributed external prerequisite plus qualification of the affected predicates—not unrelated Claude repair solely to produce green output.

I inspected retention, registry, grammar, owner leases, host effect consumers, receiver, probe, observer, sealed-reader implementation, relevant tests and pinned SDK declarations/write implementation. For the **21 Rust spillover paths**, prior archived and current bytes normalized identically using read-only `rustfmt --emit stdout`; no semantic spillover was identified by that comparison. It is not a behavioral-equivalence proof. Main graph coverage was checked and remains inadequate for the detached native subject; exact-source fallback supplied the material findings.

**Ownership and continuation:** root remains sole persistence, whole-final and acceptance owner. No source/artifact mutation, build/test, workload/provider execution or children occurred. Applied settings record **808**, SHA `04e2d540c20019c42cdea0fe294de30c74267f0826e92f569acf8e1dc58de622`, confirms Astra/xhigh/default; the read-only/restricted-network/never grant remains in force. Listener **42418** used the verified binding and **185000/220000, step 1** policy; it delivered epoch-3 pressure at **192,121 tokens**, then was interrupted and its terminal **exit 130 fully collected**. No owned process remains.

`handoff_ready` for root persistence and retained-review recovery. The remaining review includes complete build/run/artifact semantic joins, outstanding qualification evidence and final grammar/currentness coverage. **No new acceptance or independence claim is made.**

The required refresh interrupted ordinary work. [Wind Walker](/home/bline/.codex/skills/wind-walker/SKILL.md) requires continuation-critical meaning to have durable ownership before replacement. This reviewer’s read-only grant prevents publishing the required recovery file/chatboard pointer, so I have returned the complete partial result for root persistence and have **not invoked an unsafe context replacement**.
