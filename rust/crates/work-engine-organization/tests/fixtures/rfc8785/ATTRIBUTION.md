# Attribution for the RFC 8785 test vectors

## Upstream files (input/, output/, outhex/)

The 18 files in `input/`, `output/` and `outhex/` are the `testdata/` files of
<https://github.com/cyberphone/json-canonicalization> at commit
`dc406ceaf94b5fa554fcabb92c091089c2357e83`.

- Copyright 2018 Anders Rundgren.
- Licence: Apache License, Version 2.0. The full text is in `LICENSE-APACHE-2.0` in this directory.
- The files are **byte-identical to upstream at that commit** (LF line endings). Each file's source URL
  (`raw.githubusercontent.com` at the commit) and sha256 are in `provenance.json`, and a test checks the
  sha256 values.
- An earlier copy of these files came through an intermediate: the `tests/resources/testdata` directory of
  the `serde_json_canonicalizer` 0.3.2 crate (crates.io, MIT). That copy carried CRLF line endings in the
  12 `input/*.json` and `outhex/*.txt` files, so it was not unmodified; the 6 `output/*.json` files were
  identical. The upstream bytes are now restored and no file passes through the intermediate.

### Licence evidence summary

From the upstream tree and files at the pinned commit (recorded in the repository-external provenance
record `upstream-licence/PROVENANCE-RECORD.md`):

- The root `LICENSE` is the Apache-2.0 application notice, "Copyright 2018 Anders Rundgren" (sha256
  `6821faaddedf2d78c95bb6d98b127e9e616097afd2f6bcc34389f000d13ab12d`).
- The full recursive tree listing at the commit (149 entries, not truncated) contains no `NOTICE`,
  `COPYING` or `PATENTS` file. The only other licence path is `python3/src/org/webpki/json/LICENSE`, in a
  subtree that does not contain `testdata/`.
- `testdata/README.md` contains no licence, copyright or terms text.
- Not established: that the copyright holder intended the root notice to cover the test data beyond the
  repository convention, and any terms outside the repository.
- This summary is evidence, not a legal conclusion.

### Upstream licence fetch provenance

`UPSTREAM-LICENSE` is the upstream repository's own `LICENSE`, copied byte for byte. It was fetched
on 2026-11-11 (22:38:50Z) with curl from raw.githubusercontent.com:

- URL: <https://raw.githubusercontent.com/cyberphone/json-canonicalization/dc406ceaf94b5fa554fcabb92c091089c2357e83/LICENSE>
- HTTP status: 200.
- sha256: `6821faaddedf2d78c95bb6d98b127e9e616097afd2f6bcc34389f000d13ab12d`.
- It is the Apache-2.0 notice with "Copyright 2018 Anders Rundgren".
- The `NOTICE` path at the same commit returned HTTP 404, and the full tree listing above has no NOTICE
  file.
- The `testdata/` directory holds no licence file of its own; the repository `LICENSE` is the only
  licence statement covering it.

`LICENSE-APACHE-2.0` (the full Apache-2.0 text) is kept alongside it.

## Authored files

`authored_sorting.*` and `authored_example.*` were written for this crate. They are not copied from
any source. Their expected outputs were derived from RFC 8785 sections 3.2.2 and 3.2.3 and
recomputed independently; the sha256 values are in `provenance.json`.
