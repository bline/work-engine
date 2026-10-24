# Focused context-control remediation live check, 2026-10-24

Three bounded native Claude Code invocations exercised the remediated context
reader and board adapter in an isolated workspace. All three finished, with no
retry or production board injection. This is a cooperative mechanics check, not
a broad performance or isolation result.

The private evidence root
`/home/bline/.local/state/work-engine/context-control-remediation-20261024/live/remediation-01`
holds the frozen manifest (`3710d30d…` SHA-256), passing no-model preflight
(`cd7efdef…`), complete streams/native transcript copies, listener rows,
collection records, and [metrics](context-control-remediation-20261024.json).
The private runner/helper source hashes are `7fa66cad…`/`1a97067c…` and the
copied candidate context-status binary hash is `7a0cbee1…`. The frozen route
requested Claude Sonnet 5.5, medium, through Claude Code 2.1.310 with Bash/Read;
all native init/model records matched. The CLI executable's bytes were not
digest-pinned, although its version was observed before entry and in each init.

## Observed outcomes

| Check | Observed result |
| --- | --- |
| Real control receipt | The first subject's native tool result contained the full event from its own verified transcript: 20,681 input tokens, source line 56, `claude-sonnet-5-5`. The model identified it as an observation and finished the bounded arithmetic task (84). |
| Quiet listening and self posts | One owned `--follow` listener ran 75.22 seconds and ended with the documented timeout exit 2. Ordinary own post 1 was filtered; explicit `--wake` post 2 was delivered. After the wake, 64.91 seconds passed before finite end with zero native model responses in that interval. The model received the collected rows in one Bash result after the listener ended. |
| New-task capacity | A retained invocation measured a fresh valid 25,928-token reading and considered a separate hypothetical 160,000-token task plus 20,000 for collection and unfinished obligations. The 205,928 total exceeds the historical Claude band floor of 202,533 by 3,395; it chose transfer and did not execute the task. It did not extend its earlier bounded-task decision to the new scope. |
| Successor rebind | A fresh session verified its own native source. The predecessor cursor failed with exit 1 for a different binding; a new own baseline succeeded with exit 0 and a 6,616-token real observation. |

The 1,000/2,000-token control levels were explicitly lowered **test policy**.
`in_band` against those levels says nothing about provider exhaustion. Claude's
transcript supplies no context window; the historical 202,533 band floor is not
a provider reset deadline. The new-task size was an admission assumption, not
measured work. The helper chose post timing and collected the listener; no
automatic wake of an idle model or unassisted capacity performance was tested.

## Accounting and collection

Native transcripts contain 6, 3 and 4 unique model calls (13 total); CLI results
report 11, 3 and 6 turns (20 total), all below the frozen 36-turn cap. Native
usage deltas total 26 uncached input, 34,818 cache-creation input, 158,468
cache-read input and 7,222 output tokens. CLI list-cost deltas are $0.1389816,
$0.0434288 and $0.0449804, total **$0.2273908** against the frozen $1.50 cap.
The resumed result is cumulative, so its cost and usage were differenced once;
these are reported list estimates, not billed charges.

All CLI processes and the listener were collected. The copied Bash/Read tools
and workspace instructions were cooperative restrictions; they do not establish
an OS sandbox or whole-session noninterference.

## Remediation and final validation

The integrated change rejects reset/synthetic/invalid current measurements,
labels retained historical observations through direct and agent views, and
preserves valid smaller post-reset requests. Board own-post exclusion is opt-in;
explicit wake signals and local controls remain deliverable. The instructions
require a capacity judgment for the new scope and keep quiet rearming in the host.

The focused deterministic checks passed **112 tests**: 57 Rust reader/control
tests, 21 Python reader tests, 24 board tests and 10 board/control integration
tests against the replacement release binary. The 57 Rust tests also passed in
debug; this is a repeat configuration, not 57 additional unique tests. Both
skills passed their validators. Archived Codex and Claude failure prefixes now
produce unavailable current telemetry; the earlier Claude 91,579-token reading
is retained as historical, without substituting the later 99,981-token call.

A fresh read-only Claude Opus 5.5/high review, retained for one clarification
round, accepted snapshot `a64e9a7d…`. Its first pass disclosed incomplete doctrine
reading; the continuation completed that reading and re-evaluated its findings.
Four documentation files were clarified after the live freeze: shared-session
publisher suppression, single capacity records, replay scope and diagnostic
wording. The executable and test bytes are unchanged. The live run establishes
the earlier instruction exposure; the final wording has static review and
validator evidence, without another claimed live run. One redundant defensive
arithmetic guard remains as a non-blocking advisory observation.

The reviewer episode reports $1.9450886 cumulative list cost, separately from
the $0.2273908 subject check. These figures exclude Codex coordination,
implementation and harness preparation; they are not total task cost or billed
charges. Private review evidence and source manifests are siblings of the live
run under the remediation evidence root. The two original builders inherited
Astra/high before the root consulted the repository's Sol/medium selection; that
launch deviation is retained in the successor record. The later harness worker
explicitly requested and observed Sol/medium.
