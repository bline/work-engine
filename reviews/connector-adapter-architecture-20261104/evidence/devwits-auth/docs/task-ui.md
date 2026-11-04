# Typed task review and history

This is the phase 1 source candidate for one present-mode Codex start in a
disposable repository. It adds presentation and observation transport. Public
write activation and a live task require the separately qualified release,
profile, endpoint and exact signed task. The browser never grants execution.

## Review the exact task

The existing board request contains `data.intent` using
`dot.task-intent/v1-proposal`. The UI validates and displays objective and
revision, exact proposal/input, provider/profile/model/effort/tier, workspace
and source, allowed write paths/network/concurrency/children, all-actor allowance
and reserve, deadlines/first-funded rule, submitting actor and reserved decisions.

The first supported profile is `disposable-codex-write-v1`, start only, Codex,
present, one task, no task network and no children. Its name alone does not
establish native confinement or profile qualification. Supported literals are
`budget.coverage: "all_objective_actors"` and
`clock.first_funded_rule: "first_funded_setup_or_readiness"`.

All referenced material is in the **same signed request row**:

```json
{"data":{"intent_artifacts":[{"ref":{"id":"…","revision":"…","sha256":"…"},"content_json":"{…}"}]}}
```

The browser verifies each required input/proposal/configuration/prerequisite/mode
reference against the SHA-256 of its original UTF-8 `content_json`. It displays
the complete escaped JSON content, not a replacement agent summary or hash alone.
Missing, ambiguous, changed, unsupported or unrenderable terms hold Approve.
The mode reference must contain the supported full mode view, with its matching
revision, explicit present mode and no pending transition. A valid digest over
an unknown mode version does not make that version reviewable.
Every nonempty task-approval comment, including whitespace, also holds; use Ask
a question to reconcile a condition into new exact terms. Reject/defer/ask remain
available. Admission deadlines are checked before and after the passkey prompt.

Ordinary board questions retain their existing flow. A malformed or unsupported
`data.intent` cannot fall back to generic approval. The answer-v1 field order,
exact signed bytes, request digest and independent pinned-key checks remain.
The acting Rust consumer owns current task/proposal/authority checks, trusted
proof, one-answer admission, accounting and qualified native effects.

## Keep task observations after the queue closes

The existing laptop-token `PUT /api/requests` and owner-session
`GET /api/requests` carry optional `task_projection`:

```json
{
  "schema":"dot.task-projection/v1-proposal",
  "observed_at":"2026-11-02T21:00:00Z",
  "source_revision":"observation-r1",
  "tasks":[{"task":{"id":"…","revision":"…","sha256":"…"},"request_json":"{…}","admission":null,"result":null}]
}
```

`task.sha256` is SHA-256 of the exact original board `request_json`; ID/revision
match its `data.intent`. Terms and reference contents are derived from that row,
not a second mutable intent copy. Admission and result views must name the same
task reference, and a result must name the exact admission. Unsupported,
duplicate, changed or cross-task observations are visibly unavailable. History
remains visible with zero open requests. Worker outcome, operational status,
usage coverage and native/child/process disposition remain separate. Missing
worker contracts and unknown spend/closure never become successful completion
or zero cost. Snapshot/source observation timestamps remain visible.

The Rust task owner produces the projection. To transport it, configure an
absolute `task_projection_file` in `~/.config/devwits/approval-sync.json`. The
source path/producer and protection from the worker's actual grant belong to
the qualified activation packet; file permissions alone do not prove isolation.
No production configuration is changed by this source candidate. The sync
includes the projection in its cache digest, so a result update uploads even
when open requests are unchanged. A configured missing/corrupt file fails the
push and remains retryable. Removing the configuration clears the transported
view on the next push. Redis/mailbox/sync are projections, never task authority.

## Checks and visual references

```sh
npm test
python3 -m unittest discover -s laptop
npm run build
GALLERY_CDP_URL=http://127.0.0.1:9222 npm run test:ui
```

The last command starts and stops the isolated fixture server, uses a fresh
context in the supplied Chrome, checks the desktop/phone review and history,
and captures six full-page images. It disconnects without closing the owner's
Chrome. Without `GALLERY_CDP_URL` it uses Playwright's installed Chromium.
`TASK_UI_OUTPUT=/absolute/path` selects an alternate output directory.
No provider session, real passkey, real approval or native execution is used.

| Synthetic state | Desktop | Phone |
|---|---|---|
| Complete task review | [Image](task-ui/screenshots/desktop-task-review.jpg) | [Image](task-ui/screenshots/mobile-task-review.jpg) |
| Unsupported task held | [Image](task-ui/screenshots/desktop-task-held.jpg) | [Image](task-ui/screenshots/mobile-task-held.jpg) |
| Result with unresolved usage/closure | [Image](task-ui/screenshots/desktop-task-history.jpg) | [Image](task-ui/screenshots/mobile-task-history.jpg) |

These extend the historical [full UI gallery](ui-gallery/README.md). Capture
metadata is in [the task screenshot manifest](task-ui/screenshots/manifest.json).
