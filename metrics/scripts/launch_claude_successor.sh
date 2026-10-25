#!/usr/bin/env bash
# Launch a real Claude successor with a pre-minted session id and an argv
# instruction to collect its handoff from the board.
# NOTE: --add-dir is variadic (<directories...>) and swallows a trailing
# positional prompt. Keep the prompt immediately after a single-value flag.
set -uo pipefail
SID="$1"; TOPIC="$2"; HANDOFF="$3"; DIGEST="$4"
BOARD_REPO="$5"; CB="$6"; STATE="$7"
# REQUIRED before OPTIONAL. These two were first appended at 10 and 11, after
# the optional pair, to preserve older positions -- which created the defect
# below: under RETIRE=keep the coordinator substitutes {PRED_WID} and {SOCK}
# as EMPTY strings, and an unquoted empty word vanishes from the command, so
# every later argument shifted left. ROLE landed in the PRED_WID slot and the
# predecessor uuid in the SOCK slot, PRED_SESSION came through empty, and
# `adopt --predecessor-session` exited 2 on a missing value -- surfacing as a
# 241s fail_no_adoption. Observed 2026-10-25.
ROLE="${8:-claude-root}"; PRED_SESSION="${9:-}"
PRED_WID="${10:-}"; SOCK="${11:-}"

# Fail loudly rather than emitting an adopt call that cannot succeed: an empty
# predecessor session is a caller error, and discovering it through a model
# session and an adoption timeout wastes both.
if [[ -z "$PRED_SESSION" ]]; then
  echo "launch_claude_successor: PRED_SESSION (arg 9) is empty; adopt would be refused." >&2
  echo "Check the LAUNCH_TMPL argument order, and QUOTE optional placeholders:" >&2
  echo "  ... {ROLE} {PRED_SESSION} '{PRED_WID}' '{SOCK}'" >&2
  exit 2
fi

RETIRE_STEP=""
if [[ -n "$PRED_WID" && -n "$SOCK" ]]; then
RETIRE_STEP="
5. Retire the predecessor's window as your LAST action, and only after confirming your own acknowledgement is visible on the board:
   python3 $CB --repository $BOARD_REPO read --limit 5
   Confirm a message authored by successor-root with session id $SID is present. If it is not, stop and do not retire anything.
   Then run exactly:
   tmux -L $SOCK kill-window -t $PRED_WID
   Kill only the window id $PRED_WID, which was given to you. Do not choose a different target, do not kill your own window, and do not kill the server."
fi

read -r -d '' PROMPT <<P
You are the successor root for role $ROLE. Your handoff was posted to the coordination board before you started. Do exactly these steps and then stop and wait.

1. Collect it:
   python3 $CB --repository $BOARD_REPO read --limit 10
   Find the message whose topic is $TOPIC.

2. Verify the handoff artifact it references:
   sha256sum $HANDOFF
   The expected digest is $DIGEST. If it does not match, say so in your final reply and POST NOTHING TO THE BOARD. A report about the handoff must never be posted on this topic: the supervisor reads a post here as your adoption of it.

3. Reconcile claims before taking ownership:
   python3 $CB --repository $BOARD_REPO read --active-claims --limit 10
   Report which claims exist and whether any still name a predecessor session.

4. Acknowledge adoption with the dedicated adopt command. \`post\` cannot write an adoption at all; it is refused by the kind registry. Run exactly:
   python3 $CB --repository $BOARD_REPO adopt --author successor-root --session-id $SID --role $ROLE --predecessor-session $PRED_SESSION --handoff-sha256 $DIGEST --handoff-path $HANDOFF --body "Adoption acknowledged: digest verified, claims reconciled, ownership taken."
   This writes a typed adoption row on your own session topic. It re-checks the digest against the file itself, so it fails if they disagree. If it fails, say so in your final reply and do not retry with \`post\`.
$RETIRE_STEP

Then report what you did in two or three lines and take no further action. You hold no assignment authority beyond these steps.
P
# A successor that holds a blocking board wait needs a ceiling above the
# 600000 ms default, which is a hard clamp: a longer request is silently
# clamped and the command is then MOVED TO THE BACKGROUND rather than killed.
# Set it here so the mechanism does not depend on the host's settings.json.
BASH_MAX_TIMEOUT_MS="${BASH_MAX_TIMEOUT_MS:-3900000}"
export BASH_MAX_TIMEOUT_MS

exec claude \
  --permission-mode auto \
  --disallowedTools "Edit,Write,NotebookEdit" \
  --session-id "$SID" \
  "$PROMPT"
