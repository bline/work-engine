# Native Pi development paused, 2026-11-14

Status: unfinished implementation committed and merged at the operator's request.
This record does not accept the full development vertical or qualify production use.

## Operator decision

At 2026-11-14T23:03:16.889Z the operator answered the pending workspace choice:

> Yes; please commit and merge everything you have done so far. We are down to 1% weekly quota so pause once everything is commited.

The question offered temporary unchecked candidates in an isolated Docker scratch
workspace, with original credentials held on the host and every Work Engine export
and custody publication checked. The affirmative answer accepts that boundary.
The former before-every-scratch-write predicate is superseded for these temporary
candidates. It remains required at Work Engine retention/publication boundaries;
credential isolation and the trusted laptop assumptions remain in force.
This changes acceptance scope prospectively. It does not repair the old FT-01
counterexample or establish completeness of the revised boundary.

The same instruction authorizes committing and merging the unfinished work now,
then pausing. It does not authorize additional implementation, inference, provider
calls, deployment or push. Integration targets the operator's active
`rescue-detached-20260911` branch; unrelated uncommitted work is preserved.

## Preserved implementation and evidence

The commit preserves native Pi memory/FIFO prerequisites, the actual pinned Pi
SDK Docker transport, checked retention and sealed read path, native tools and
workspace build/test/export probes, the result-contract change and focused tests,
and FT-02 acquisition/immutable-ID cleanup. Formatting spillover from earlier
cargo fmt runs is included in the frozen source subject rather than discarded.
The existing main Pi Docker path is not replaced by this offline transport.

The successful pre-FT02 source12 workflow exercised real native tools, compilation,
three tests, checked export, three evolving snapshots and three fresh sealed
readers. A separately collected host observer recorded six receipts and five
deduplicated effects. Those results are bound to their exact source and synthetic
inputs. FT-02 subsequently changed one file; its focused acquisition, transport,
result, examples and actual Docker controls passed, and retained Astra review
accepted that bounded repair only. The complete source12 workflow was not rerun
after FT-02. No live provider or real credential qualification is claimed.

The evidence copies beside this record retain exact builder results, partial
review, separate design reassessment, bounded FT-02 review and acceptance,
and qualification state. `evidence-manifest.json` binds each copy and the private
durable indexes, which continue to own all detailed artifacts and failed runs.
The authority event is bound at rollout line13658, event SHA-256
`61a26e52022ea82eb65ce71df957cd99a49be4021165fa1dd69ad936473b2479`.

## Unfinished work at pause

The complete 13-class qualification matrix, fully green broad regression gate,
and complete exact-source/proof review remain unmet. Preserve all crash, EIO,
observer-loss, late-response and lost-ack observations and their explicit UNKNOWN
outcomes. Four Claude ProgramDrift failures and original intermittent broad-gate
failures remain recorded; isolated passing reruns do not erase them.

FT-01's original scratch-write requirement has changed by operator authority,
but the revised scratch-versus-publication profile still needs implementation and
qualification reconciliation. The existing historical matrix is not rewritten as
passing. No original-A retention, export or custody check is waived. FT-02 is
accepted only within its bounded delta and stated test limitations. Historical
Claude findings and the unfinished fulltools review are not closed by this commit.

All builders, reviewers, runtime jobs and their prior listeners were settled and
fully collected before integration. The root will close its temporary integration
monitors and release the coordination claim at pause. Further engineering requires
an explicit user resume; no automatic continuation or completion is implied.
