# Paired calibration campaign rules

During an active calibration campaign, use its `claude_paired_review.py` front
controller rather than invoking either arm separately. That exact route is
required so every paid realtime attempt is registered in the denominator and
both arms receive the registered packet without one result contaminating the
other. The paired inference route excludes mutable production review-state
tools because identical arm commands would otherwise let the batch writer
contend for the authoritative episode. Preserve and verify the realtime session,
then resume only that session for required production-state bookkeeping before
reporting the operational review state complete. Return only the realtime
review to the workflow, and keep the batch result advisory until the complete
campaign is adjudicated. A campaign score never approves the reviewed slice.

During paired calibration, the exact-command requirement creates a necessary
exception: do not expose mutable episode tools to either inference arm. After
the initial result and session UUID are verified, resume only the realtime
session with the episode-bound state tools to record and read its initial
result. Until that same-session bookkeeping is durably observed, retain the
result but report review state as incomplete; the batch session never writes
the production episode.
