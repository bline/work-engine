# Cooperative live calibration

This harness exercises the committed experiment-control CLI with real Claude
Code sessions. It is test orchestration glue, not a production launcher,
authentication layer or sandbox. See
[the test design](../../../docs/agent-experiment-live-tests.md).

`prepare.py` materializes commit `9bf8885f`, copies a provisioned binary, creates
synthetic private fixtures and writes the run manifest. It rejects an existing
run directory. `checks.py preflight` calibrates the exposure detector and
records both managed refusals and known bypasses in a separate synthetic store.
`calibration.py run` launches the measured coordinator; its host operations can
launch only the two named workers and their bounded continuations.
`checks.py audit` produces a mechanical report for outside evaluation.

The manifest pins inputs and the native invocation budget. The harness retains
launch inputs, raw native streams, available native transcript copies, actual
session IDs, usage, process collection records, managed requests/responses and
Git state. Native transcript observation remains distinct from host-supplied
bytes. Context is checked before a retained worker continuation. Deadline
handling terminates owned process groups; any uncertain collection remains
explicit. No background listener or production chatboard is used by the trial.

Participant `client.py` removes request-envelope boilerplate but leaves policy
and decisions with the Rust implementation and measured model. It does not
prevent another command from bypassing it. The full shell remains cooperative;
actual command/output traces require audit for bypasses. This first package
covers L1–L3, not handoff, scoring or the separate context-enrollment rollout case.

Run roots contain unblinded private information. Do not give workers the run
manifest, store, host code, other workspaces, marker map or audit reports.
Permission to access them is not granted by sharing the filesystem. Public
reports should use role aliases and evidence references without private marker
values. The detector recognizes exact markers, four nested base64 layers and
bounded fixture paraphrases; outside semantic evaluation is still required.

Example preparation (no model call):

```sh
python3 experiments/agent-experiment-control/live/prepare.py \
  --root /absolute/new-private-run \
  --repository /absolute/work-engine \
  --binary /absolute/verified/experiment-control
python3 /absolute/new-private-run/checks.py \
  --root /absolute/new-private-run preflight
```

Verify the binary against its build/source record and freeze the manifest before
launching. A repaired preflight or harness gets retained provenance; failed
trials are not rewritten. Calling `calibration.py --root RUN run` is a paid live
operation, not a dry run. Native CLI dollar limits are admission thresholds;
an in-flight request may cross one. Keep raw provider counters and distinguish
reported usage-based cost from actual billed charges.

The first collected run and its limitations are recorded in
[the result report](../../../metrics/probes/experiment-control-live-20261023.md).
`context_probe.py` is a separate, one-run root-observation fixture with a fixed
private output root and candidate telemetry build. It refuses to reuse an
existing workspace. It is not part of the L1–L3 package or a portable installer;
prepare new pinned paths and a new manifest before another paid invocation.
