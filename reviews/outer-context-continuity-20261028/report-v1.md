I've finished reading everything in scope and am writing up the findings now.

# Review: outer-context-continuity subject-v1

**Subject:** `subject-v1.json` (SHA256 `c74464a1…446fe`), 8 files: 6 prose files, `role_host.py`, and `test_codex_settings.py`. I read the frozen copies directly and did not recompute their SHA-256s, because I have no Bash. No graph queries were needed.

**Applicability:** applicable. All six prose files are normative text that agents load, and `role_host.py settings` puts a new tool behind those instructions.

**Verdict: `changes_required`.** Two findings need small rewordings (OCT-OPUS-1, -2). One claim about how the code behaves can't be checked inside my reading limits (OCT-OPUS-3). Everything else is low severity or optional.

## Requested changes that check out
- **Pressure no longer implies root transfer.** All six prose files now agree: coordinator.md:69, context-boundary.md:16, 18–23 and 157–159, context-control.md:5–7 and 96–98, same-session-context.md:9–14.
- **The Rust runtime layer is kept separate** (same-session-context.md:3–5). A search of the frozen skills directory finds no Rust host contract cited as binding.
- **These obligations are kept:**
  - Authority: same-session-context.md:23–27 and context-boundary.md:25–29, consistent with DESIGN §11:651.
  - Wakeup provenance: context-boundary.md:35–37 and same-session-context.md:59–62.
  - Collection: context-boundary.md:41–44 and same-session-context.md:71–74.
  - Listener rebinding: coordinator.md:7–16, context-control.md:84–88 and same-session-context.md:76–80.
  - No overclaimed automation: same-session-context.md:31–42 and 93–96.
- **The settings reader matches its description.** It checks the session identity (role_host.py:95–96), filters events by `thread_id` (100–101), never falls back to the shared config, leaves an unknown tier as `not_established` (113–121), and reviewers keep their own tier (codex-host.md:72–73).

## Changes required

**OCT-OPUS-1 (medium): the manual transfer route never tells the user which settings to launch the successor with.**
- **Where:** context-boundary.md:202–209 (manual route), compared with codex-host.md:59–63 and context-boundary.md:177–182.
- **What it protects:** settings the successor was asked to start with, versus whatever the shared defaults give it.
- **Evidence (observed):** the launch arguments are offered only for `role_host.py launch` and the rotation gate. On the manual route the user launches the successor, and the instruction they're given names only the handoff, its digest, the skill and the first actions. The handoff records the settings, but the successor reads it only after it has launched. Until then the shared `config.toml` decides, and per codex-host.md:35–36 Fast persisted into that file, so any match is luck.
- **Consequence:** the user's goal of keeping Fast for the successor coordinator isn't met on one of the two documented routes. A mismatch only surfaces at the post-adoption check (codex-host.md:69–71), which costs the user a second step.
- **Advice (restate):** put the observed model, effort and tier, with the matching launch flags or a `/fast` instruction, into the successor instruction given to the user. Add the settings check to the successor's first actions.
- **Confidence:** high on the gap.

**OCT-OPUS-2 (medium): the post-recovery settings check can just re-read the pre-recovery record.**
- **Where:** same-session-context.md:82–85, codex-host.md:53–57 and 69–71, role_host.py:97–104.
- **What it protects:** settings observed after recovery, versus no recorded change at all.
- **Evidence:** the reader returns the last matching event in the file. Same-session compaction keeps the same rollout file. If no new `thread_settings_applied` record follows the replacement, the reader returns the same line and digest as the saved observation, the comparison "matches", and nothing after recovery was actually observed.
- **Two related gaps:**
  - The "saved root observation" (same-session-context.md:83) has no step that records it. The preparation list (46–51) leaves it out, and codex-host.md:45–46 asks for the reading only "before preparing a handoff".
  - "Do not let shared defaults silently change…during compaction" (84–85) assumes a control the session doesn't have. All it can do is detect a change and raise it.
- **Advice (restate):**
  - Say that a record at or before the replacement point means "no recorded change", not "preservation observed", and should be reported that way.
  - Record the reading in the continuation before an intentional replacement.
  - Replace "do not let" with "detect and surface".
- **Confidence:** medium, inferred. I can't tell from the subject whether Codex writes a record per turn or only on change.

**OCT-OPUS-3 (unresolved): I can't confirm the rotation gate accepts `--host-arg`.**
- **Where:** codex-host.md:61–63, compared with the gate commands at context-boundary.md:185–201.
- **Evidence:** the prose cites "the rotation gate's repeated `--host-arg` options", but `root_rotation.py` is outside my reading limits. The `prepare` and `commit --detach` commands shown don't include any settings arguments.
- **Consequence:** an agent copying the shown commands would launch the successor on shared defaults.
- **Advice:** check which `root_rotation.py` subcommand at this revision passes `--host-arg` to the launch, and show it in step 4 (Gate). If none does, remove the claim.

## Low-severity findings

**OCT-OPUS-4 (low–medium, inferred): recovery instructions may not survive the replacement.**
- Preparation (same-session-context.md:46–51) doesn't name the skill and role references the recovered context needs to reload. The manual transfer route does name the skill (context-boundary.md:205–207).
- The rebinding, settings and collection obligations live in references read into the context that gets replaced. Whether they're reloaded depends on Wind Walker or the host, which I didn't review.
- **Advice:** add one clause to the preparation list naming those references.

**OCT-OPUS-5 (low, observed in code, not run): malformed lines escape the documented exit codes.**
- A line that is valid JSON but not an object, or a `payload` or `thread_settings` that isn't an object, raises `AttributeError` at role_host.py:95, 99 and 110.
- Neither handler catches it (105, 658), so the script prints a traceback and exits 1, outside the documented 0/21/32/40/64 (5–7). It still fails safe: no launch arguments are emitted.
- **Advice:** check for objects (`isinstance(..., dict)`) and return exit 32 with `settings_not_established`. Optionally tolerate a partial trailing line in a live rollout.

**OCT-OPUS-6 (low): the Codex version the tier mapping depends on isn't shown.**
- codex-host.md:64 says "the qualified Codex version" without naming it.
- The code applies the mapping without checking the version (111–113), and the output says `launch_arguments_available` without saying what version that rests on.
- The post-launch check is the backstop.
- **Advice:** name the version in the prose, or report the rollout's recorded version (if it has one) in the output.

**OCT-OPUS-7 (low, inferred): the text doesn't say which wins when the saved setting and the owner's standing choice disagree.**
- The owner selects Fast for supervisors and builders, the root uses `/fast`, and children inherit the root's tier (codex-host.md:3–4, 10, 34).
- If a predecessor is observed at `default`, keeping it (60–61) gives the successor's builders `default`, against the owner's standing choice.
- **Advice:** when the two disagree, ask the user rather than picking one silently.

**OCT-OPUS-8 (low): the same rules are worded differently in two files loaded together.**
- context-boundary.md:35–37 calls a saved wakeup "runtime-authored". same-session-context.md:59–60 more carefully says to keep its author and delivering actor; the author isn't always the runtime.
- "Current human direction governs the next admission to productive work" (same-session-context.md:61–62) can be read as requiring fresh human input even when the existing assignment already covers continuing (line 23).
- The copy in context-boundary.md is needed, because it's what reaches an agent after provider-initiated compaction.
- **Advice:** keep that short copy, make same-session-context.md the detailed source, and don't restate it differently.

**OCT-OPUS-9 (low): same-session-context.md is written for roots but loaded by any role.**
- SKILL.md:19 loads it for every role, but lines 71–74 and 82–85 say "this root" and "coordinator".
- **Advice:** limit the loading condition to roots, or say "this session" where the rule is general.

## Optional suggestions
- **S1:** "Use a supported route" (same-session-context.md:40–41) doesn't say which routes are supported. Name them: keep going on bounded work, the operator running native compaction by hand, or root transfer. Leave self-invoked compaction unqualified until the fixtures report. (DESIGN §1.3 and §2.3 ask instructions to describe available capabilities.)
- **S2:** Move the cost comparison in same-session-context.md:98–101 to `measurement.md`, and keep the warning against overclaiming qualification (91–96).
- **S3:** "Not executable shell text" (codex-host.md:63) needs an example. Show the `--host-arg=<element>` form; argparse treats a separate `-m` value as an option.
- **S4:** `event_sha256` covers the raw line including its newline (role_host.py:104). Document that, or hash the stripped line, so someone re-checking it gets the same digest.
- **S5:** Add tests for:
  - the CLI path, including exit 64 for a non-Codex host or a non-UUID session ID;
  - a missing model or effort producing no launch arguments;
  - lines that aren't JSON objects.

## Not established
- **Real rollout format:** I haven't checked the real Codex 0.164 format for `thread_settings_applied` or `session_meta`. The 4 tests use made-up records, and I didn't read the service-tier probe.
- **Fast on launch:** I don't know whether `thread/start` actually honours `config.service_tier="fast"`. Test 1 only checks that the arguments parse.
- **Standard-tier recording:** I don't know whether a standard-tier root with no explicit setting records `default` or nothing. If nothing, preservation comes back `not_established` in a common case. That fails safe, but it's a gap.
- **Unqualified routes:** compaction controls, automatic self-resumption and the ordering of concurrent human input aren't qualified, and the subject correctly doesn't claim they are. Live fixtures are still running, and I haven't inferred their results.
- **Files not reviewed:** `anthropic-host.md`, `supervisor.md`, `worker.md`, `call-cost.md`, `root_rotation.py` and Wind Walker. I confirmed the pressure/transfer contradiction is gone only within the six prose files.

This is advisory only: I'm not certifying or accepting anything, and deployment isn't my decision. This session is kept for the bounded remediation.