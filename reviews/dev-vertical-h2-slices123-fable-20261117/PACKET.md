# Review packet: H2 Pi-route slices 1–3

Subject: `1051e660` against the closed dev-vertical milestone `4d5b0b34`
([subject.diff](subject.diff): `rust/` and the plan). Reviewer: read-only; write only `REVIEW.md`
in this directory. Use `git show 1051e660:<path>` for exact subject bytes (the working tree may
hold later changes). Do not build into `rust/crates/pi-pressure-context-host/target` or run Docker.

A first review of slices 1–2 alone (Opus, Claude quota) did not complete: the subscription's
5-hour limit ended it. This review replaces it and covers all three slices.

## Authority and scope

[Plan](../../planning/dev-vertical-h2-pi-route-20261117.md) (revision 2 design for slice 3). The
operator directed building what the research-lifecycle H2 run needs on the Pi route. This work is
the Pi Host side only; the task-host connection (slice 4) is not in the subject. Records:
[qualification record](../../records/dev-vertical-h2-route-20261117/README.md) (slices 1–2:
baseline D4 failure, run set s12 20/20 on `da0a54f8`). Slice 3 runs (offline analysis set and the
coding regression set on `1051e660`) are in progress; their results will be added to the record.

## What changed

1. **D4.** The receiver stops forwarding `message_update` on the live bridge; the live protocol
   digest says so. Host stdout is read incrementally from `coding_out_offset`.
2. **Analysis profile** `work-engine.pi-analysis-hosted.v1` (`pi_coding_analysis.rs`, hooks in
   `pi_coding.rs`, `pi_docker.rs`, `pi_coding_hosted.rs`, `pi_coding_provider.rs`; receiver mode in
   `receiver/pi-coding.mjs`): no shell, tool container or validator; the job workspace is the
   manifest-pinned snapshot, never mounted; Host-served `snapshot_list` / `snapshot_read`
   (create-only retained results, native completion must equal the delivery, SDK refusals never
   reach the Host); larger per-profile bounds (context 400 000 B, body 600 000 B, frame 1 MiB, job
   output 64 MiB, 32 requests, 64 reads); `Profile.toolchain`/`acceptance` became optional.
3. **Tools policy.** Request tools are collected from the top-level `tools` array and from Codex
   `additional_tools` input items (the earlier policy checked only the former; Pi declares tools
   in the latter). Analysis requests must declare exactly the two tools with a pinned canonical
   digest (`TOOLS_SHA256`, taken from the first offline run).
4. **Request body retention** (both hosted profiles): `coding-hosted-{request}.body.json`
   create-only before the first attempt; `request_body_sha256` on every attempt record, persisted
   before dispatch.
5. **Relay coalescing** (`pi_stream.rs`): the upstream reader thread appends body bytes that are
   already buffered (`Upstream::next_buffered`, never blocks) to the chunk it relays, up to
   16 KiB. Before this, one relayed chunk per HTTP chunk meant one container round trip and one
   fsync per provider event (8 000 deltas took ~260 s; after: ~38 s). The Host loop also skips its
   20 ms sleep after a tick that applied output or wrote frames.
6. **Lifecycle.** `lifecycle-store`: `PiCodingKind::Analysis`, `pi_analysis_proof.rs` grammar
   (any number of rounds; each executed tool result equals a `pi.analysis_read` Host delivery
   recorded between its start and end; refusals have none; final clean stop), settlement through
   `settle_pi_coding`, no effect entry, 32 KiB analysis prompt cap. `pi-pressure-context-host`:
   `analysis.rs` gate and `analysis-qualification` producer.
7. `work-engine-server`'s live class now names the coding profile explicitly (the analysis
   profile has no server class).

## Questions (complex review: evidence and admission design)

- Can any path let the model affect anything beyond reading pinned snapshot bytes: a different
  tool surface reaching the provider, a Host read outside the manifest, a mount, a shell?
- Is every claim of the record supportable: body retention and its timing, delivery/native
  matching, the grammar's binding of tool results to Host deliveries, settlement?
- Relay coalescing: does it preserve the settlement rule (terminal-last, tail rule, drain,
  screening, held-back bytes, truncation and interruption reasons) when a relayed chunk now spans
  HTTP chunks and may contain bytes after `response.completed`? Any blocking or loss in
  `next_buffered` / the held result?
- Bounds: are the new limits consistent end to end (receiver frame and body, Host frame and output,
  store ingress 1 MiB, settlement event count, prompt cap)? Anything that fails open?
- The coding profile: any regression from the shared changes (optional toolchain/acceptance,
  `is_hosted` now covering two schemas, the receipt pass condition, the loop timing)?
- Anything else wrong.

Severity: P0 (unsafe or false evidence), P1 (must fix), P2 (should fix), P3 (note). Cite
file:line at `1051e660`.
