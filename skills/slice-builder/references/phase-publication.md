# Publishing a phase consequence

`<skill-dir>` is the slice-builder skill's directory.

Use the stable identity supplied by the supervisor and recover its current
`durable_revision` with `<skill-dir>/../slice-supervisor/scripts/resume_active_slice.py`
immediately before publication. Then invoke the CAS boundary directly:

Active-slice recovery and publication are owned only by the slice-supervisor
scripts. Do not look for copies under slice-builder or another skill. Pass
`--identity-json` an object with exactly `run_id`, `slice_number`, `attempt_id`,
and `plan_version`; camelCase keys and inferred alternate shapes are invalid.

```bash
python3 <skill-dir>/../slice-supervisor/scripts/manage_active_slice.py \
  --repository <repository> publish-phase \
  --identity-json '<run/slice/attempt/plan identity>' \
  --expected-revision '<recovered durable_revision>' \
  --phase implementation \
  --consequence-json '<compact integrity-bound consequence>'
```

For the final gate use the same command with `--phase gate` and the then-current
recovered revision. A revision conflict means publication did not establish the
consequence; recover and reconcile instead of retrying against an assumed state
or returning the transient result as complete.
