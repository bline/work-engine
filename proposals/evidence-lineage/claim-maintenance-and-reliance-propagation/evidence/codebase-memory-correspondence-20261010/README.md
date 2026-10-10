# Correspondence Experiment Harness (2026-10-10)

Disposable exploration harness behind
[`../codebase-memory-correspondence-20261010.md`](../codebase-memory-correspondence-20261010.md).
It is a shadow prototype of evidence-anchor's `CodeEvidenceAdapter` and
document adapter against explicit test anchors, not an implementation
candidate.

| File | sha256 |
|---|---|
| `harness/cbm.py` | `6a118fad75a0524284dfbe0ae129e2c07d5bd0408be9508c7f3941613bd10373` |
| `harness/observer.py` | `f0c54b02be8e8a8d9a072fe9ed4f47912d859152cabc7dfe3b0994ebc24b35c9` |
| `harness/experiments.py` | `7471c8ff6c6546908571af14acbb239004d289e0fe99ba9b462f75bb42dc323e` |
| `results.json` | `0e8f14e0335bb3d864eed31781cf1269ee4a5c65ca9ee466f765140dd1acaac6` |

`results.json` is the raw output of one complete run of exactly these files:
27 cases and 15 findings. Qualified names in it embed the original scratch
work-directory path because Codebase Memory derives project names from
repository paths.

## Rerunning

1. Build Codebase Memory at `ec4fbd3a` (`make -f Makefile.cbm cbm
   BUILD_DIR=<dir>`).
2. Create a work directory `$W` with `cache/` and `home/`, and a private
   runtime directory with a short path (the daemon socket path must fit the
   Unix socket limit), for example `mktemp -d /tmp/cbmrt.XXXX`, mode 0700.
3. Write `$W/cbm.sh`:

   ```bash
   #!/bin/bash
   export HOME=$W/home CBM_CACHE_DIR=$W/cache CBM_RUNTIME_DIR=<runtime dir>
   exec <build dir>/codebase-memory-mcp cli "$@"
   ```

   The separate `CBM_RUNTIME_DIR` matters. Without it, the build's
   exact-version admission barrier refuses to start while another Codebase
   Memory version is active on the account.
4. `CBM_EXPERIMENT_DIR=$W python3 harness/experiments.py [e1 ... e6 e4b]`

Fixture repositories are created under `$W/exp/`. The observer uses only
supported CLI tools. `cbm.oracle_*` reads the SQLite store directly and is
used only to evaluate the supported signals, never to produce an
observation.
