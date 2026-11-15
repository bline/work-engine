**Recommend adopting exact R10 as a local structural correction, with the prerequisites below recorded in root’s disposition.** Another comparison-only repair under R9 is insufficient. The evidence does not justify reopening the broader product architecture or transferring domain ownership. This completes the bounded design review; it authorizes no implementation or acceptance.

Reviewed identities:

| Subject | Verified identity |
|---|---|
| PLAN-R9 | `ab8cf7ad1d79565dbbce3b422ceb927999f13eed5e9d21f9041b343dbd21aa9d` |
| PLAN-R10 | `8b168cf1276bf0be761d311b414d15e929b7398eac333176ec4d70b729e9b9e9` |
| History revision 36 | `4e3a692fb90c41de9c553621deb62959aa87cde8e41d4928dfab3968fa4bb5ae` |
| Rejected candidate3 checkpoint | `2617a6c622c74f330e3bd7722dff2e59fb332c87` |

I read the complete admitted plans, history, R9 adoption, C/D reports and root dispositions, A’s report, and the R5 recheck. Targeted direct-source inspection independently supports the current failure mechanisms. The following IDs identify design conclusions and prerequisites, not additional implementation-rejection episodes.

**AR10-001 — Qualification must own applicability and completion together.**

R9 already required complete appraisal. Its realization still allows successful top-level accounting while omitting an applicable relationship:

- [control_semantics](/home/bline/cbm-work/org-pi-review-continuation-20261114/realization-r9/evidence-gate/src/outputs.rs:1112) checks diagnostic support only for nonempty UTF-8.
- [appraise_bundle](/home/bline/cbm-work/org-pi-review-continuation-20261114/realization-r9/evidence-gate/src/outputs.rs:1294) dispatches refusal through a path that omits the start-field comparisons in `native_protocol`.
- [spec.rs](/home/bline/cbm-work/org-pi-review-continuation-20261114/realization-r9/evidence-gate/src/spec.rs:215) derives a control-level check identity. Completion of that identity does not establish completion of every applicable predicate inside it.

R10’s explicit applicability/coverage boundary addresses this cause. Required predicates must derive from the approved scenario, physically observed entries and owner-interpreted facts. Missing correspondence, an unknown role, or an unperformed applicable predicate must prevent successful appraisal. A stage may explain legitimate absence; it cannot erase an observed earlier fact.

A small finite representation is sufficient. The builder may choose its form. Adding another list of known controls or a coverage count would not establish this consequence.

**AR10-002 — Delivery should own reusable request and diagnostic meaning.**

The proposed owner is appropriate. [NativeDriver](/home/bline/cbm-work/wt-p1-vertical-20261113/rust/crates/work-engine-review-delivery/src/native.rs:982) constructs the request from the binding; [run_native](/home/bline/cbm-work/wt-p1-vertical-20261113/rust/crates/work-engine-review-delivery/src/native.rs:1067) selects the diagnostic; [capture_returned](/home/bline/cbm-work/wt-p1-vertical-20261113/rust/crates/work-engine-review-delivery/tests/common/mod.rs:581) persists its exact returned bytes. The qualifier should consume that owned meaning, without acquiring a second protocol definition.

The implementation must preserve these distinctions:

- Retained intended frame bytes, bytes accepted by writes, and peer processing are different facts.
- A refusal can lack an observation while its existing request still requires correspondence.
- Approved wrong-session and duplicate-start controls require their specified deviations. Applying successful-path rules unchanged would falsely reject valid controls.
- Diagnostic bytes require the producer’s actual selection rule and applicable source relationship. Comparing them indiscriminately with `capture.error` or the last raw error frame would be wrong.

I inspected the actual delivery-refusal, duplicate-start and wrong-session bundles. They support these distinctions: delivery-refusal retains a completed matching request and observation despite exit 5; duplicate-start retains two matching starts and its diagnostic; wrong-session changes the session while retaining matching contract/result bytes. The retained examples themselves are coherent. CR9-001/002 remain supported static counterexamples, not executed reproductions.

**AR10-003 — A conservative package-source inventory is credible, but dep-info cannot establish its completeness.**

Both selected dependency build scripts compile probe sources and delete their temporary dep-info. See [thiserror](/home/bline/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/thiserror-2.0.21/build.rs:103) and [proc-macro2](/home/bline/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/proc-macro2-1.0.107/build.rs:165). I independently confirmed the two reported probe paths are absent from the prospective inventory, cold-build record, common map and source manifest.

The existing [bootstrap](/home/bline/cbm-work/org-pi-review-continuation-20261114/implementation-r6/preparation-03/verifier_bootstrap.py:15) therefore cannot promote `uncovered: []` over surviving dep-info into complete material-input closure.

For this bounded dependency set, I favor R10’s conservative package-source superset over maintaining only a hand-selected probe list. Its establishment still requires:

- prospective binding of the selected unpacked source set and archive correspondence;
- source-backed treatment of material build-script reads, compiler/wrapper selection and relevant environment;
- separate derivation evidence for generated inputs;
- actual cold/common-build reconciliation and retained executable joins.

Package contents alone do not cover inputs outside those packages. Post-build equality cannot supply a missing prelaunch observation. These requirements remain compatible with actor-attributed custody and source-bound declarations; they do not require kernel tracing, hermeticity or hostile-writer protection.

**AR10-004 — Current bootstrap reuse has an unresolved authority prerequisite.**

The existing bootstrap explicitly includes and later reads protected configuration, and the current map names `/home/bline/.rustup/settings.toml`. I inspected the declaration only, not that file.

Consequently, “reuse the bootstrap and add package sources” is not an admissible executable route under R10’s unchanged protected-configuration exclusion. Before launch, root must establish an authorized route that avoids those reads, or obtain the owning authority’s disposition and admissible evidence. If neither is available, complete input establishment remains unmet. Omitting the input from the claim would silently narrow acceptance.

This does not require changing R10’s ownership or obtaining greater privileges as the default solution. It is a concrete prerequisite to realization.

D’s native-node concern also remains open. Current source performs parent PATH selection in [native_vertical.rs](/home/bline/cbm-work/wt-p1-vertical-20261113/rust/crates/work-engine-review-delivery/tests/native_vertical.rs:42), then launches `cfg.node` with a fixture cwd and cleared environment in `native.rs`. The supplied `native_node_selection` evidence is relevant. The next declaration must clearly distinguish lookup context from child invocation context and establish their correspondence. I do not close D’s concern through endpoint equality alone.

**Alternatives and retained decisions.**

I reject merely adding the two comparisons and two probe paths: that repairs the witnesses while leaving the recurring applicability and discovery boundaries unchanged. Running the successful validator indiscriminately on refusals would reject legitimate partial and negative cases. Dropping diagnostics, shrinking the declared suite, or redefining complete inputs around available evidence would change required claims.

A general qualification engine, new authority service, owner reassignment or stronger containment has no demonstrated causal necessity here. Existing delivery interpretation, canonical binding/consumer APIs, capture ownership and the finite qualifier provide suitable boundaries.

The R9 adoption remains attributable historical authority. Its ordinary-repair permission cannot carry forward past the current escalation without root’s new disposition. Existing passing executions, retained bytes and bounded corrective support survive; claims of complete appraisal and complete material source closure do not follow from them. No historical finding closes through this design judgment.

**Recommended next scope, after root disposition and capacity assessment.**

One coherent assignment may own the delivery interpretation changes, private qualifier applicability/completion, and prospective preparation/map correction. Preserve the existing capture lifecycle and domain owners. New first-party executable components follow the Rust direction.

Necessary validation consequences include:

1. Reject hash-consistent diagnostic substitutions and refusal request/result mismatches while accepting legitimate refusal variants.
2. Demonstrate that stage changes, later failures, unknown support roles and surviving remnants cannot suppress applicable predicates.
3. Preserve full intended-frame evidence alongside partial/failed write accounting, without claiming peer consumption.
4. Prospectively cover deleted probe inputs; reject package/archive mismatches and materially omitted inputs even when compilation produces the same outcome.
5. Retain capture-before-assertion, failed-finalization preservation, causal/timing/wait-error controls and all applicable earlier counterexamples.
6. Use fresh qualification paths and a new immutable subject; preserve the actual build/direct-execution joins. No retroactive qualification.

Root must separately reconcile the permitted input route, exact prospective subject and budget before implementation admission.

**Unfinished implementation obligations remain unchanged.** All **21 findings** remain open and all **20 whole-review obligations** remain unmet. A’s transitive SDK/preparation/owner applicability, B’s entirely unlaunched attempt/protocol/capture review, C’s complete actual domain/bundle appraisal, D’s remaining runtime/source/custody closure, and root’s collected shared-interface disposition remain owed. The precise unfinished lists in the admitted A/C/D reports remain operative. This review does not replace them.

For provenance, nine inspected product/qualifier/preparation source files matched the admitted source manifest. `finalize-preparation.py` was inspected as supplied preparation evidence but has no direct entry in that manifest; I make no completed common-input attribution claim for it. Tier-2 graph generation `2026-11-14T07:46:39Z` was confirmed; private/dependency paths were outside the project and received direct-source fallback. No graph completeness claim is made.

Reviewer identity is the acknowledged `01a1d962-3fe8-7452-b00e-e45f5bbfcdba`, with root-verified Astra/xhigh/default/read-only/never settings. Final bound telemetry was **158,303 tokens, epoch 0**; the governing preservation thresholds remain **185,000/220,000**. Latest delivered local cost was **2.247758/4** at 09:34:04Z; final tail cost is unknown here.

No product execution, mutation, replay, children, session enumeration or transcript reconstruction occurred. No reviewer-owned process remains. Root retains listener 03, sole collection, persistence and design disposition.