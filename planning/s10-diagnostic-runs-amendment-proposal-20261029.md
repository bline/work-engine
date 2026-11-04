# S10 diagnostic runs: adopted amendment (2026-10-29)

Status: **adopted and applied to the S10 implementation plan on 2026-10-29.**
The owner instructed this root to adopt the amendment and then execute within a
two-hour bounded window. Revision 2 applies review findings D1 to D3 on
revision 1 (`1d0a94f9…`). The reviewed revision 2 digest is
`8e2d12eb48cf78870c9ed6c7fc5e298ed769767c5a3182b89728fd2fac8b8be8`.

## Why

In the S10 session of 2026-10-28/29 (n=1), the last run of the candidate on this
host was the startup-only measurement at 21:53Z (readiness 7.56 s). In the
following 17.6 hours (22:00Z to 15:37Z) the work went into private source
copies. About 14 source or contract gaps were found (counted from the root's
progress messages, not audited one by one). Most went through an amendment, an
Astra review and a new supervisor, and none of the fixes was tried in a real
run. The failed qualification attempt at 19:49Z was the first run of the real
host under the observer; earlier runs used a synthetic host. That one run showed
both causes of the failure: the host took 91 s to open its socket under the
observer, and the controller's 10 s read timeout on the submit reply produced
`os error 11`.

The condition that kept runs out of that loop is recorded in the root's
continuation (`runtime_evidence_repair.target_limits`): "realstartup root-only
after review". It follows the sequence the owner approved on 2026-10-28: observer
comparisons, review, and then one bounded startup measurement before any fresh
qualification attempt. `IMPLEMENTATION_PLAN.md` already allows "normal focused
tests, bounded diagnosis and in-scope remediation" under an implementation grant,
but it does not say whether running the candidate on this host counts as bounded
diagnosis. This amendment says that it does, within limits, and keeps
qualification attempts as they are.

## Adopted text

Applied to `experiments/context-lifecycle-rust/IMPLEMENTATION_PLAN.md`, section
S10, after the paragraph ending "stops advancement for diagnosis.":

> **Diagnostic runs.** Between qualification attempts, the implementation owner
> may run the current candidate on this host to see how it behaves. This is a
> standing authorization. A run that meets the limits below needs no review,
> entry grant, qualification freeze or approval from the root. It informs
> engineering only: it satisfies no qualification check, consumes no
> qualification attempt, and is never cited as qualification evidence. The
> assignment names the operations its diagnostic runs may exercise. If it names
> none, a run may only start the host to readiness and stop it.
>
> - **Identity.** The run record identifies the candidate. It holds the digest
>   of each executable the run starts and the command, plus the digest of each
>   configuration file. It also holds the source identity: a commit, or for a
>   tree outside the repository the manifest digest from `custody.py freeze`,
>   taken once per source change. `custody.py run` records the command and,
>   with `--digest`, the files.
> - **State.** The run starts from a new private root made for that run: home,
>   Codex home, stores and work directory. No later qualification subject reuses
>   that root.
> - **Permitted inputs and effects.** Besides its own root, the run may read:
>   - system directories (`/usr`, `/lib*`, `/bin`, `/sbin`, and `/etc` except
>     `/etc/codex`), read-only;
>   - `/dev` and its own `/proc` entries;
>   - the candidate's executables and pinned inputs, read-only.
>
>   It writes only under its root, and its network is loopback only.
>
>   Everything else is prohibited. That includes:
>   - the rest of the user's home, including Codex, Claude and Work Engine
>     state;
>   - the repository working tree;
>   - other content of `/tmp`, `/var/tmp` and `/dev/shm`;
>   - `/run/user`, which holds the user's service manager and session bus;
>   - other sessions' processes;
>   - credentials, live providers, and any network beyond loopback;
>   - Codex system configuration under `/etc/codex`;
>   - any change to an installed tool or service, or to a host-wide privilege,
>     namespace or hardening setting.
> - **Exclusion before the run.** Each prohibited access is excluded before the
>   run starts. Either a mechanism enforces the exclusion, or the resource is
>   shown to be absent. Clearing the environment and capping process lifetime
>   do not enforce filesystem or network limits.
>
>   One mechanism on this host (as of 2026-10-29) is `bwrap`. It ran
>   unprivileged with a private network holding only loopback, a read-only root,
>   and a private home with chosen paths bound in, as user 1000 with no
>   capabilities. `/etc/codex` was absent.
>
>   If a prohibited access cannot be excluded, the run does not start. The
>   record names the mechanism used, because a sandbox can change what the run
>   observes.
> - **Containment.** The sandbox runs inside one transient user scope with a
>   wall-clock cap and a kill of the whole control group, as the 2026-10-28
>   startup grant did (`systemd-run --user --scope`, `RuntimeMaxSec`,
>   `KillMode=control-group`). The run is started through `custody.py run`, so
>   its process IDs, exit and leftover processes are recorded.
> - **Cleanup.** When the run ends, its scope is gone and no process of it
>   remains. The run stops only its own processes. The record and logs are kept
>   whatever the outcome. The root may be deleted after that.
> - **Limit.** At most 10 diagnostic runs per assignment. They come out of the
>   assignment's time and quota budget, and each stays within the qualification
>   attempt's own caps. The coordinator may approve more runs and record that
>   in the assignment's scope record. An extension does not raise the budget,
>   and a new assignment for the same work does not reset the count.
>
> **Failures.** Some failures are ordinary: a failed assertion, a nonzero exit,
> or a candidate stopped by the cap after missing a deadline. They are
> engineering information. Diagnosis continues, and unlike a failed
> qualification attempt they do not stop candidate advancement.
>
> A boundary failure is different. It is any of these:
> - a process that outlives the run or escapes its scope;
> - an observed access or effect outside the permitted ones;
> - a cleanup that leaves the scope, processes or files outside the root behind.
>
> After a boundary failure, the implementation owner tells its supervisor at
> once. No further diagnostic run starts in the assignment until the supervisor
> accepts a fix.
>
> So that repeated runs cannot hide a flaky result, the qualification review
> receives the list of diagnostic runs on the candidate's lineage, with their
> identities and outcomes. Qualification attempts do not change: a frozen and
> reviewed subject, a separate grant, one attempt, and no reuse.

## What changes in practice

- A builder can run the candidate on this host within minutes of a change. It
  doesn't wait for a review cycle or the root.
- The S10 root's condition "realstartup root-only after review" no longer
  applies to diagnostic runs. It still applies to qualification attempts. The
  root's continuation says that quoted recommendations do not waive that
  condition, so the owner's decision should reach the root as an instruction,
  not only as this document.
- No new tool is required. `systemd-run`, `bwrap` and `custody.py` (released in
  `471f6106`) exist.
- Whether the candidate and its ptrace observer run unchanged under `bwrap` is
  not established. The first startup-only run tests it.

## What it does not change

It does not change live-provider proof, qualification attempts and their
single-use rule, the provider-disabled gate, review of the frozen candidate,
or the authority needed for publication, cutover or shutdown.

## Adoption decision

The owner adopted revision 2 and instructed execution within a two-hour window.

1. **Adopted:** the diagnostic-run authorization above.
2. **Who:** the assigned implementation owner, under this standing
   authorization, with no root approval for each conforming run.
3. **Limit:** 10 runs per assignment, as a ceiling within the assignment's time
   and quota budget.
4. **First use:** one startup-only diagnostic run on the current candidate, which
   also shows whether it runs under the sandbox. Broader provider-disabled
   operations are named in the assignment that needs them.
