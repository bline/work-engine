# Attribution for the RFC 8785 test vectors

## Upstream files (input/, output/, outhex/)

The 18 files in `input/`, `output/` and `outhex/` are copies of the `testdata/` directory of
<https://github.com/cyberphone/json-canonicalization> at commit
`dc406ceaf94b5fa554fcabb92c091089c2357e83`.

- Copyright 2018 Anders Rundgren.
- Licence: Apache License, Version 2.0. The full text is in `LICENSE-APACHE-2.0` in this directory.
- The files are unmodified. Their sha256 values are in `provenance.json` and are checked by a test.
- They reached this repository through an intermediate copy: the `tests/resources/testdata`
  directory of the `serde_json_canonicalizer` 0.3.2 crate (crates.io), which copied them unmodified.
- The upstream repository has no NOTICE file.

### Upstream licence fetch provenance

`UPSTREAM-LICENSE` is the upstream repository's own `LICENSE`, copied byte for byte. It was fetched
on 2026-11-11 (22:38:50Z) with curl from raw.githubusercontent.com:

- URL: <https://raw.githubusercontent.com/cyberphone/json-canonicalization/dc406ceaf94b5fa554fcabb92c091089c2357e83/LICENSE>
- HTTP status: 200.
- sha256: `6821faaddedf2d78c95bb6d98b127e9e616097afd2f6bcc34389f000d13ab12d`.
- It is the Apache-2.0 notice with "Copyright 2018 Anders Rundgren".
- The `NOTICE` path at the same commit returned HTTP 404, so the upstream repository has no NOTICE
  file.
- The `testdata/` tree page at that commit (fetched earlier the same day) shows no licence file of
  its own, so the repository `LICENSE` is the licence that applies to those files.

`LICENSE-APACHE-2.0` (the full Apache-2.0 text) is kept alongside it.

## Authored files

`authored_sorting.*` and `authored_example.*` were written for this crate. They are not copied from
any source. Their expected outputs were derived from RFC 8785 sections 3.2.2 and 3.2.3 and
recomputed independently; the sha256 values are in `provenance.json`.
