# Final re-review: outer-context-continuity subject-v3

**Subject:** `subject-v3.json` (SHA256 `29e2a5cf…d71c`) and `delta-v3.patch`. This is a continuation by the same retained reviewer, so it makes no new freshness or independence claim. I didn't recompute any digests.

**Verdict, within the context and tier scope only: `acceptable_as_is`.** This makes no claim about the shared skill package as a whole, any release, or deployment.

## Applicability
- **Applicable:** the context-continuity and root-settings hunks in `context-boundary.md`, `same-session-context.md`, `codex-host.md`, `role_host.py` and `test_codex_settings.py`.
- **Omitted:**
  - **The two excluded hunks:** the load trigger for `organization-formation.md` at SKILL.md:14 and its paragraph at coordinator.md:20–21.
  - **Who owns them:** session `01a18339`, its supervisor and its Astra review own them and keep acceptance (`scope-exclusions-v3.json`).
  - **What the omission means:** this review makes no judgment about them. It neither certifies them nor recommends removing them.
  - **Evidence not checked:** I didn't inspect the exclusion's own evidence (board sequences 3468/3469, worker result). The omission doesn't depend on it.

## Status of findings 10–12

| ID | Status | Basis |
|---|---|---|
| OCT-OPUS-10 | **Resolved**: the hunks are omitted as owned by someone else | Exclusions recorded with owner, identities and disposition. The SKILL.md and coordinator.md digests are the same as in v2. |
| OCT-OPUS-11 | **Resolved** | `settings-reader-v3-fast-live.json` is bound to the code SHA `790bbe2a…`, which matches the subject-v3 `role_host.py`. On a real rollout reporting runtime 0.165.0, it exits 0, reads `priority` at line 114, and returns the `service_tier="fast"` launch arguments (n=1 observed). The 8-rollout v2 validation is as you reported; I didn't inspect it. The delta leaves the parsing path unchanged from v2 to v3; only the tier guard and the basis string changed. |
| OCT-OPUS-12 | **Resolved** | context-boundary.md:202 and 210–216 (v3). The gate and manual settings clauses now apply only to Codex. Other hosts carry known settings with their source, or mark them unestablished. The applied-settings check in the successor's first actions is Codex-only. |

**Version wording:** codex-host.md:68–73 (v3) now treats the installed CLI version (0.164.0) and the runtime version a daemon records (0.165.0) as separate observations, and says these cases don't qualify every version or ingress path. The live probe records the same thing, including that the daemon binary wasn't bound (probe JSON:5–9).

**Fast evidence:** I read probe lines 259–339. Both route B (a successor from a different predecessor) and route A (same session) record `priority` after the boundary, with `configuration_preserved` and `requested_tier_observed` both true (n=1 per route). That supports the citation for the mapping. The linked probe path exists in the live repository; I didn't check that it's byte-identical to the frozen copy.

## Correction to my v2 report
- **Wrong claim:** I marked OCT-OPUS-5 resolved and said v2 "checks every record's shape and returns exit 32".
- **What was missed:** a list or dict `service_tier` still raised an uncaught `TypeError` at the dictionary lookup (role_host.py:122 in v2). The script would exit 1, outside its documented exit codes. It still produced no launch arguments.
- **Error class:** an incomplete trace. I checked that whole records were validated but didn't follow each field to the operations that need a particular type.
- **Consequence:** v2 would have gone forward with one remaining malformed-input path outside the exit contract. v3 fixes it (line 122), and the extended test covers `[]`, `{}` and `17`.
- **Check that would have caught it:** trace every value read from the matched record to each use that assumes a type. I've now done that for v3, and no remaining path escapes.

## New findings
None material.

One optional nit: the comment at role_host.py:120 still says only "Native Codex 0.164", although the basis string at line 130 cites 0.165.0 too. My earlier optional point also still stands: a record from an unqualified version still yields `launch_arguments_available`. The prose disclaims that case, and checking settings after launch remains the backstop.

## Not qualified, and what I read
The subject claims none of these, and neither do I:
- autonomous self-compaction or resumption;
- ordering when other input arrives during a replacement;
- whether replay is safe;
- net billed savings;
- broad superiority;
- deployment.

I didn't inspect the authorship controls. As you describe them, their receipts are observations at a boundary, not proof of continuous absence.

I read only:
- `subject-v3.json` and `delta-v3.patch`;
- the scope exclusions;
- the v3 Fast live result;
- the live probe, via a search plus lines 250–339.

I did not read the context decision, the v2 validation, `assessment-v2.json`, `authorship-terminal.json`, the full probe, or any native transcripts.

This is advisory only. Acceptance and mutation stay with you as coordinator and with the decision owner.