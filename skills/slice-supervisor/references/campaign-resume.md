# Resuming a named campaign run

`<skill-dir>` is the slice-supervisor skill's directory.

After an interruption between terminal slices, recover one named run with:

```bash
python3 <skill-dir>/scripts/resume_campaign.py \
  --path <configured-metrics-path> \
  --campaign-preflight-json '<fresh-successful-preflight-result>' \
  --run-id '<stable-run-id>'
```

Resume only when the command reports `resumable: true`. It binds both the
effective engine configuration and campaign-source identity, reconstructs the
compact handoff, and identifies the next sequential slice without writing or
reserving it. When the terminal receipt carries a slice checkpoint, recovery
also verifies the private ref and commit tree and returns it as
`baseline_checkpoint`; that immutable identity, not current branch HEAD, is the
next slice's repository baseline. Structured stopped, failed, accepted-complete, and historical
continuation-unavailable results are stable non-resumable states. Command
failure means malformed or incompatible state requiring intervention. This
path does not recover mid-slice or partial artifacts and does not authorize an
amendment.
