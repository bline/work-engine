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

## Authored files

`authored_sorting.*` and `authored_example.*` were written for this crate. They are not copied from
any source. Their expected outputs were derived from RFC 8785 sections 3.2.2 and 3.2.3 and
recomputed independently; the sha256 values are in `provenance.json`.
