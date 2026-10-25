#!/usr/bin/env bash
# rootsup2 - root rotation coordinator. tmux manages windows; the chatboard
# carries the handoff; this script validates, gates and swaps. No model calls.
#
#   ROLE/SOCK/STATE      tmux window name, socket, durable evidence dir
#   VALIDATOR            command (with flags) + <handoff> -> 0 pass | 21 defect | 32 not est.
#                        (any other code is treated as not established, never pass)
#   QUIESCENCE_FILE      predecessor's own assertion that it is quiescent
#   CHILDREN_CMD         prints an integer: live children (independent check).
#                        Must exit 0; see metrics/scripts/quiescence_checks.py
#   CLAIMS_CMD           prints claims still held by the predecessor session.
#                        Must exit 0 -- empty output means "none held", so an
#                        erroring check must not be read as a clear result.
#   DESCENDANTS_CMD      prints an integer: session processes beneath the root,
#                        read from the OS process tree. {PANE_PID} is
#                        substituted. Independent of board registration, so a
#                        child that never claimed is still visible; the count is
#                        an upper bound and the gate is == 0.
#   BOARD_CMD            chatboard script, invoked with --repository REPO
#   REPO                 repository the board lives in
#   PRED_SESSION         predecessor session id
#   LAUNCH_TMPL          {SESSION} {TOPIC} {HANDOFF} {DIGEST} substituted
#   ADOPT_TIMEOUT        seconds to wait for the successor's board acknowledgment
#   RETIRE               kill|keep|successor
#   ORPHAN               kill|keep: disposition of a successor that never
#                        adopted. Default kill -- it owns nothing, and left
#                        alive it is a second candidate root the descendant
#                        check cannot see (it is a sibling window).
#   GATE_AUTHORITY       watch: comma-separated authors/session ids whose
#                        approval counts. Unset = refuse to watch (fail closed)
#   APPROVE_TOKEN        watch: token the approval body must contain. The body
#                        must ALSO carry the handoff's sha256, so an approval
#                        binds to the artifact it approved rather than to
#                        whatever the watch happens to be holding.
#   WATCH_TIMEOUT        watch: bounded lifetime, default 3000s
set -uo pipefail
ROLE="${ROLE:?}"; SOCK="${SOCK:?}"; STATE="${STATE:?}"; VALIDATOR="${VALIDATOR:?}"
BOARD_CMD="${BOARD_CMD:?}"; REPO="${REPO:?}"; PRED_SESSION="${PRED_SESSION:?}"
LAUNCH_TMPL="${LAUNCH_TMPL:?}"; QUIESCENCE_FILE="${QUIESCENCE_FILE:-}"
CHILDREN_CMD="${CHILDREN_CMD:-}"; CLAIMS_CMD="${CLAIMS_CMD:-}"
DESCENDANTS_CMD="${DESCENDANTS_CMD:-}"
MAX_ROTATIONS="${MAX_ROTATIONS:-3}"; GATE="${GATE:-require}"
ADOPT_TIMEOUT="${ADOPT_TIMEOUT:-180}"; RETIRE="${RETIRE:-keep}"
# What to do with a successor that never adopted: retire it (default) or keep
# it for inspection. Keeping leaves a second candidate root alive.
ORPHAN="${ORPHAN:-kill}"
# 3000s keeps one watch inside the 1h prompt-cache TTL, so the wake resumes on a
# read prefix rather than rebuilding it. Nothing is spent while blocked.
WATCH_TIMEOUT="${WATCH_TIMEOUT:-3000}"; APPROVE_TOKEN="${APPROVE_TOKEN:-approve-rotation}"
# Case-insensitive: Codex renders "Trust this folder?" and Claude lower-cases
# differently; a case-sensitive scan missed the Codex modal entirely.
STALL_PATTERN="${STALL_PATTERN:-trust this folder|folder access|quick safety check|do you trust|usage limit|approaching rate limit|switch to gpt-|rate limit reminder}"

T() { tmux -L "$SOCK" "$@"; }
B() { python3 "$BOARD_CMD" --repository "$REPO" "$@"; }
log() { printf '%s [sup:%s] %s\n' "$(date -u +%FT%T.%3NZ)" "$ROLE" "$*" | tee -a "$STATE/supervisor.log" >&2; }
rec() { printf '%s\n' "$2" > "$STATE/$1"; }
mkdir -p "$STATE"; C="$STATE/rotation-count"; [[ -f $C ]] || echo 0 > "$C"

validate() {    # 0 pass | 21 defect found | 32 could not be established
  local handoff="$1" phase="$2" rc
  # VALIDATOR is a COMMAND, not a path: a real validator needs its own flags
  # (--repo, --require). Word-split it so those survive.
  local -a argv; read -r -a argv <<< "$VALIDATOR"
  "${argv[@]}" "$handoff" >>"$STATE/validator.log" 2>&1; rc=$?
  case "$rc" in
    0)  log "validator: pass ($phase)"; return 0 ;;
    21) log "validator: DEFECT found in handoff ($phase)"
        rec terminal-status "fail_validation_${phase}"; return 21 ;;
    32) log "validator: could not establish validity ($phase)"
        rec terminal-status "validation_not_established_${phase}"; return 32 ;;
    *)  log "validator: unusable exit $rc ($phase); treating as not established"
        rec terminal-status "validation_not_established_${phase}"; return 32 ;;
  esac
}

quiescent() {   # predecessor assertion PLUS independent checks; unknown != clear
  if [[ -z "$QUIESCENCE_FILE" || ! -s "$QUIESCENCE_FILE" ]]; then
    log "predecessor has not asserted quiescence"; rec terminal-status "quiescence_not_established"; return 2; fi
  if [[ -z "$CHILDREN_CMD" || -z "$CLAIMS_CMD" || -z "$DESCENDANTS_CMD" ]]; then
    log "independent quiescence checks not configured"; rec terminal-status "quiescence_not_established"; return 2; fi
  local kids claims krc crc
  kids=$($CHILDREN_CMD 2>>"$STATE/supervisor.log"); krc=$?
  claims=$($CLAIMS_CMD 2>>"$STATE/supervisor.log"); crc=$?
  # The board checks above establish "no unexpired claim is held", which is only
  # as good as the registration discipline. This one reads the OS process tree,
  # so a child that never registered is still seen: a child must evade BOTH.
  local pane_pid desc drc
  pane_pid=$(T display-message -p -t "roots:$ROLE" '#{pane_pid}' 2>>"$STATE/supervisor.log")
  if [[ ! "$pane_pid" =~ ^[0-9]+$ ]]; then
    log "cannot resolve the predecessor's pane pid; the descendant check cannot run"
    rec terminal-status "quiescence_not_established"; return 2; fi
  local -a dargv; read -r -a dargv <<< "${DESCENDANTS_CMD//\{PANE_PID\}/$pane_pid}"
  desc=$("${dargv[@]}" 2>>"$STATE/supervisor.log"); drc=$?
  # The claims check is read by EXIT STATUS as well as output. Reading output
  # alone fails open: a check that errors prints nothing, and empty output is
  # how "no claims held" is spelled, so a broken check would clear the gate.
  (( crc == 0 )) || { log "claims check failed (exit $crc); cannot establish that none are held"
    rec terminal-status "quiescence_not_established"; return 2; }
  (( krc == 0 )) || { log "child check failed (exit $krc)"; rec terminal-status "quiescence_not_established"; return 2; }
  [[ "$kids" =~ ^[0-9]+$ ]] || { log "child count unreadable ('$kids')"; rec terminal-status "quiescence_not_established"; return 2; }
  (( drc == 0 )) || { log "descendant check failed (exit $drc); cannot establish that no child runs"
    rec terminal-status "quiescence_not_established"; return 2; }
  [[ "$desc" =~ ^[0-9]+$ ]] || { log "descendant count unreadable ('$desc')"
    rec terminal-status "quiescence_not_established"; return 2; }
  # Report EVERY failing condition, not just the first. Checking children before
  # claims and returning early hid a held claim behind a live child, which reads
  # as a diagnosis of the wrong problem.
  local -a why=()
  if (( kids > 0 )); then log "$kids live child(ren) remain"; why+=("fail_children_live"); fi
  if (( desc > 0 )); then
    log "$desc session process(es) remain under the root (upper bound on children)"
    why+=("fail_descendants_live"); fi
  if [[ -n "${claims//[[:space:]]/}" ]]; then
    log "predecessor still holds claims: $(echo "$claims" | tr '\n' ' ')"; why+=("fail_claims_held"); fi
  if (( ${#why[@]} > 0 )); then
    rec terminal-status "$(IFS=+; echo "${why[*]}")"; return 1; fi
  log "quiescence established: 0 claimed children, 0 claims held, 0 descendant session processes"
  return 0
}

prepare() {
  local handoff="$1" n; n=$(<"$C")
  (( n < MAX_ROTATIONS )) || { log "CAP $n/$MAX_ROTATIONS"; rec terminal-status "fail_cap_reached"; return 20; }
  validate "$handoff" prepare || return $?
  local dig; dig=$(sha256sum "$handoff" | cut -d' ' -f1)
  log "validated $handoff sha256=${dig:0:12}"
  quiescent || return $(( $? == 2 ? 32 : 22 ))
  rec validation-record "handoff=$handoff sha256=$dig validated_at=$(date -u +%FT%T%Z)"
  if [[ "$GATE" == require ]]; then
    rec awaiting-authority "handoff=$handoff sha256=$dig"
    # STATE persists across rotations, so leaving terminal-status untouched
    # leaves the PREVIOUS run's verdict standing beside this one's gate hold.
    rec terminal-status "awaiting_authority"
    log "GATE: validated and quiescent; awaiting authority. NO successor exists."; return 10
  fi
  commit "$handoff"
}

commit() {
  local handoff="${1:-}"
  [[ -n "$handoff" ]] || handoff=$(sed -n 's/^handoff=\([^ ]*\).*/\1/p' "$STATE/awaiting-authority" 2>/dev/null)
  [[ -s "${handoff:-}" ]] || { log "no handoff to commit"; return 29; }
  validate "$handoff" commit || return $?
  local dig want; dig=$(sha256sum "$handoff" 2>/dev/null | cut -d' ' -f1)
  [[ "$dig" =~ ^[0-9a-f]{64}$ ]] || { log "HOLD: cannot digest '$handoff'"
    rec terminal-status "validation_not_established_commit"; return 32; }
  # EXPECT_DIG is set by watch(), which has its own binding digest and never
  # runs prepare. Falling back to "no record means unchanged" turned "could not
  # check" into a pass, so the bytes could change between approval and launch.
  want="${EXPECT_DIG:-}"
  [[ -n "$want" ]] || want=$(sed -n 's/.*sha256=\([0-9a-f]*\).*/\1/p' "$STATE/validation-record" 2>/dev/null)
  [[ -n "$want" ]] || { log "HOLD: no validated digest to compare against; cannot establish the bytes are unchanged"
    rec terminal-status "validation_not_established_commit"; return 32; }
  [[ "$dig" == "$want" ]] || { log "HOLD: bytes changed since validation"; rec terminal-status "fail_handoff_mutated"; return 30; }
  quiescent || return $(( $? == 2 ? 32 : 22 ))
  # The cap is enforced HERE as well as in prepare: commit is what increments
  # the count, and watch() reaches commit without ever running prepare, so a
  # cap checked only in prepare does not bind the automated path.
  local n; n=$(<"$C")
  (( n < MAX_ROTATIONS )) || { log "CAP $n/$MAX_ROTATIONS"; rec terminal-status "fail_cap_reached"; return 20; }

  # Successor identity is chosen BEFORE launch, so the handoff can be waiting.
  local sid topic; sid=$(cat /proc/sys/kernel/random/uuid); topic="session:$sid"
  local seq
  seq=$(B post --author "supervisor:$ROLE" --session-id "$PRED_SESSION" --topic "$topic" \
        --body "Root handoff. Verify sha256 $dig, reconcile active claims, then acknowledge adoption on $topic." \
        --ref "$handoff" 2>>"$STATE/supervisor.log" \
        | python3 -I -c 'import sys,json; print(json.load(sys.stdin)["message"]["sequence"])' 2>/dev/null)
  [[ -n "${seq:-}" ]] || { log "HOLD: board post failed; nothing launched"; rec terminal-status "fail_board_post"; return 33; }
  rec successor-identity "session=$sid topic=$topic handoff_sequence=$seq digest=$dig"
  log "posted handoff to $topic at sequence $seq"

  # Capture the predecessor's window ID before anything moves: names travel
  # with windows through swap-window, so only IDs are stable identities.
  local pred_wid pred_cwd
  pred_wid=$(T display-message -p -t "roots:$ROLE" '#{window_id}')
  # Launch in the PREDECESSOR'S cwd: directory trust is per-directory, so a
  # successor started elsewhere stalls on a modal that looks alive (observed).
  pred_cwd=$(T display-message -p -t "$pred_wid" '#{pane_current_path}')
  [[ -d "${pred_cwd:-}" ]] || pred_cwd="$REPO"
  local pred_idx; pred_idx=$(T display-message -p -t "$pred_wid" '#{window_index}')
  log "predecessor window $pred_wid index $pred_idx cwd $pred_cwd"
  local cmd wid
  cmd=${LAUNCH_TMPL//\{SESSION\}/$sid}; cmd=${cmd//\{TOPIC\}/$topic}
  cmd=${cmd//\{HANDOFF\}/$handoff}; cmd=${cmd//\{DIGEST\}/$dig}
  if [[ "$RETIRE" == successor ]]; then
    cmd=${cmd//\{PRED_WID\}/$pred_wid}; cmd=${cmd//\{SOCK\}/$SOCK}
  else
    cmd=${cmd//\{PRED_WID\}/}; cmd=${cmd//\{SOCK\}/}
  fi
  wid=$(T new-window -P -F '#{window_id}' -d -n "${ROLE}-next" -c "$pred_cwd" "$cmd" 2>>"$STATE/supervisor.log")
  [[ -n "${wid:-}" ]] || { log "HOLD: launch failed; predecessor untouched"; rec terminal-status "fail_launch"; return 23; }
  echo $(( $(<"$C") + 1 )) > "$C"
  log "launched successor $sid in window $wid"

  # Adoption arrives on a BLOCKING board wait, not a poll: the board wakes us on
  # the message instead of being asked for it every second. The wait must be
  # one-shot -- --follow rearms inside its own process, never exits, and so never
  # returns anything to act on.
  #
  # The pane still has to be watched for a startup dialog, and a blocking wait
  # cannot also watch it, so the wait runs as a child process and only the pane
  # is polled -- a tmux capture, with no python subprocess per tick.
  local ack="" stalled="" since="$seq" waitout="$STATE/adoption-wait.json" wrc=0
  while :; do
    B wait --since "$since" --topic "$topic" --timeout-seconds "$ADOPT_TIMEOUT" \
      >"$waitout" 2>>"$STATE/supervisor.log" &
    local wpid=$!
    while kill -0 "$wpid" 2>/dev/null; do
      if T capture-pane -p -t "$wid" 2>/dev/null | grep -qEi "$STALL_PATTERN"; then
        stalled=1; kill "$wpid" 2>/dev/null; break
      fi
      sleep 0.5
    done
    wait "$wpid"; wrc=$?
    [[ -n "$stalled" ]] && break
    (( wrc == 0 )) || break          # 2 = the wait's own timeout
    # A wait returns on the first message matching the topic, which need not be
    # the successor's. Re-arm from the returned cursor rather than accept it.
    ack=$(python3 -I -c 'import sys,json
d=json.load(open(sys.argv[1]))
print(next((m["message_id"] for m in d.get("messages",[])
            if m.get("session_id")==sys.argv[2]), ""))' "$waitout" "$sid" 2>/dev/null)
    [[ -n "${ack:-}" ]] && break
    local nxt; nxt=$(python3 -I -c 'import sys,json;print(json.load(open(sys.argv[1])).get("next_since",""))' "$waitout" 2>/dev/null)
    [[ -n "$nxt" && "$nxt" != "$since" ]] || break
    log "board woke on a non-successor message; re-arming from $nxt"
    since="$nxt"
  done
  # A successor that never adopted owns nothing, but left running it becomes a
  # second candidate root: the next rotation launches another one beside it, and
  # it is a sibling window, so the descendant check cannot see it. Name it in
  # STATE either way -- an orphan only in the log is not an orphan anyone can
  # find -- and retire it unless asked to preserve it for inspection.
  abandon() {
    local reason="$1"
    rec orphan-successor "window=$wid session=$sid topic=$topic reason=$reason disposition=${ORPHAN}"
    if [[ "$ORPHAN" == keep ]]; then
      log "unadopted successor $wid KEPT for inspection; it is not the role window"
    elif T kill-window -t "$wid" 2>>"$STATE/supervisor.log"; then
      log "unadopted successor window $wid retired ($reason)"
    else
      log "WARNING: could not retire unadopted successor window $wid; it is still running"
      rec orphan-successor "window=$wid session=$sid topic=$topic reason=$reason disposition=kill_failed"
    fi
  }
  if [[ -n "$stalled" ]]; then
    T capture-pane -p -t "$wid" > "$STATE/stall-capture.txt" 2>/dev/null
    log "HOLD: successor stalled on a startup dialog; see $STATE/stall-capture.txt; predecessor NOT retired"
    abandon startup_dialog
    rec terminal-status "fail_startup_dialog"; return 27
  fi
  if [[ -z "${ack:-}" ]]; then
    log "HOLD: no adoption acknowledgment in ${ADOPT_TIMEOUT}s (wait exit $wrc); predecessor NOT retired"
    abandon no_adoption
    rec terminal-status "fail_no_adoption"; return 34
  fi
  log "adoption acknowledged by $sid (message $ack)"

  if [[ "$RETIRE" == successor ]]; then
    # The successor was handed its predecessor's window id and retires it as its
    # own last action. The supervisor verifies the outcome and positions the role.
    local k=0 gone=no
    while (( k < 60 )); do
      local wins; wins=$(T list-windows -t roots -F '#{window_id}' 2>>"$STATE/supervisor.log"); local lrc=$?
      if (( lrc != 0 )); then
        # A failed listing is not evidence of retirement. Stop claiming either way.
        log "cannot list windows (exit $lrc); retirement by the successor is not established"
        gone=unknown; break
      fi
      grep -qx "$pred_wid" <<< "$wins" || { gone=yes; break; }
      sleep 1; ((k++))
    done
    if [[ "$gone" == unknown ]]; then
      rec predecessor-retired-by "not_established"
      log "HOLD: cannot establish the predecessor's window state; nothing swapped or killed"
      rec terminal-status "fail_window_state_unknown"; return 36
    fi
    if [[ "$gone" == yes ]]; then
      log "successor retired predecessor window $pred_wid"
      rec predecessor-retired-by "successor"
      T move-window -s "$wid" -t "roots:$pred_idx" 2>>"$STATE/supervisor.log" \
        || log "note: could not move successor to index $pred_idx"
    else
      log "successor did NOT retire $pred_wid within 60s; supervisor is not substituting for it"
      rec predecessor-retired-by "not_established"
      T swap-window -s "$wid" -t "$pred_wid" || { log "HOLD: swap failed"; rec terminal-status "fail_swap"; return 35; }
      T rename-window -t "$pred_wid" "${ROLE}-retired"
    fi
    T rename-window -t "$wid" "$ROLE"
    log "successor window $wid now occupies the $ROLE position"
  else
    T swap-window -s "$wid" -t "$pred_wid" || { log "HOLD: swap failed; nothing retired"; rec terminal-status "fail_swap"; return 35; }
    T rename-window -t "$wid" "$ROLE"
    log "successor window $wid now occupies the $ROLE position"
    if [[ "$RETIRE" == kill ]]; then
      T kill-window -t "$pred_wid" && log "retired predecessor window $pred_wid"
    else
      T rename-window -t "$pred_wid" "${ROLE}-retired" && log "predecessor $pred_wid kept as ${ROLE}-retired for questions only"
    fi
  fi
  local map; map=$(T list-windows -t roots -F '#{window_index}:#{window_name}:#{window_id}' | tr '\n' ' ')
  log "post-swap map: $map"
  rec window-map "$map"
  rm -f "$STATE/awaiting-authority"; rec terminal-status "committed_rotation_$(<"$C")"
  return 0
}

watch() {      # block on the board until an AUTHORIZED approval arrives
  local handoff="$1"
  # Fail closed on authority. A board message is advisory by construction -- the
  # board stamps its own claims authority=advisory_coordination_only, and
  # context-control.md holds that an observation grants no root-transfer
  # authority. So a trigger counts only from a named author or session id, and
  # with no allowlist this refuses to watch at all: an unauthenticated trigger is
  # worse than a manual rotation. Whether a board message may carry transfer
  # authority is a doctrine question for the skill owner; the mechanism's part is
  # to require it be named and to record which message was acted on.
  [[ -n "${GATE_AUTHORITY:-}" ]] || {
    log "REFUSING to watch: GATE_AUTHORITY names no authorized approver"
    rec terminal-status "watch_refused_no_authority"; return 77; }
  # Bind the approval to the ARTIFACT. Author + token alone approves "a
  # rotation", not "this handoff": a stale or replayed approval would commit
  # whatever this watch was started with. Requiring the digest in the body makes
  # the approval content-addressed. NOTE this binds to content, not to a
  # generation: two rotations of a byte-identical handoff share a digest, and
  # the board identifies authors without authenticating them.
  local want_dig; want_dig=$(sha256sum "$handoff" 2>/dev/null | cut -d' ' -f1)
  # An EMPTY digest is a wildcard, not a binding: the body test is a substring
  # test and "" is a substring of every message, so an unreadable handoff would
  # approve every token-bearing post from a named approver. Refuse to watch.
  [[ "$want_dig" =~ ^[0-9a-f]{64}$ ]] || {
    log "REFUSING to watch: cannot compute a sha256 for '$handoff'"
    rec terminal-status "watch_refused_no_digest"; return 79; }
  local topic="control:rotation:$ROLE" since deadline
  since=$(B read --limit 1 2>/dev/null | python3 -I -c 'import sys,json
d=json.load(sys.stdin); m=d.get("messages",[]); print(m[-1]["sequence"] if m else 0)' 2>/dev/null)
  [[ "$since" =~ ^[0-9]+$ ]] || { log "cannot read the board cursor"; rec terminal-status "watch_board_unreadable"; return 32; }
  deadline=$(( $(date +%s) + WATCH_TIMEOUT ))
  log "watching $topic from sequence $since for ${WATCH_TIMEOUT}s; approvers: $GATE_AUTHORITY; must cite sha256 ${want_dig:0:12}"
  rec watch-state "watching topic=$topic since=$since authority=$GATE_AUTHORITY digest=$want_dig"
  while (( $(date +%s) < deadline )); do
    local left=$(( deadline - $(date +%s) )); (( left > 0 )) || break
    local out="$STATE/watch-wait.json" rc
    B wait --since "$since" --topic "$topic" --timeout-seconds "$left" \
      >"$out" 2>>"$STATE/supervisor.log"; rc=$?
    (( rc == 0 )) || { log "watch ended without an approval (wait exit $rc)"
      rec terminal-status "watch_expired"; return 78; }
    local verdict
    # EVERY message in the batch is examined before the cursor moves. Stopping
    # at the first token-bearing message dropped a valid approval that arrived
    # behind a rejected one in the same wake -- and since the cursor then
    # advanced past the whole batch, the approval was never seen again. That
    # let any party who can post suppress an approval by posting a rejectable
    # message alongside it.
    verdict=$(AUTH="$GATE_AUTHORITY" TOKEN="$APPROVE_TOKEN" DIG="$want_dig" python3 -I -c '
import sys, json, os
d = json.load(open(sys.argv[1]))
ok = {x.strip() for x in os.environ["AUTH"].split(",") if x.strip()}
tok, dig = os.environ["TOKEN"], os.environ["DIG"]
approved, rejects = None, []
for m in d.get("messages", []):
    body = m.get("body") or ""
    if tok not in body:
        continue
    ident = (m.get("message_id"), m.get("author"))
    if dig not in body:
        rejects.append(("UNBOUND",) + ident); continue
    if m.get("author") in ok or m.get("session_id") in ok:
        if approved is None:
            approved = ident
        continue
    rejects.append(("UNAUTHORIZED",) + ident)
with open(sys.argv[2], "w") as fh:
    for r in rejects:
        fh.write(" ".join(str(x) for x in r) + "\n")
if approved:
    print("APPROVED", approved[0], approved[1])
elif rejects:
    print(rejects[0][0], rejects[0][1], rejects[0][2])
else:
    print("NONE", d.get("next_since", ""), "")
' "$out" "$STATE/watch-rejects" 2>/dev/null)
    local kind id who; read -r kind id who <<< "$verdict"
    case "$kind" in
      APPROVED)
        log "approval accepted from '$who' (message $id) citing sha256 ${want_dig:0:12}"
        rec authorized-by "author=$who message_id=$id token=$APPROVE_TOKEN digest=$want_dig rotation=$(<"$C")"
        [[ -s "$STATE/watch-rejects" ]] && { log "$(wc -l < "$STATE/watch-rejects") rejected trigger(s) in the same batch"
          cat "$STATE/watch-rejects" >> "$STATE/rejected-triggers"; }
        GATE=open; EXPECT_DIG="$want_dig" commit "$handoff"; return $?;;
      UNAUTHORIZED)
        log "IGNORED: approval token from unauthorized '$who' (message $id)"
        rec last-rejected-trigger "unauthorized author=$who message_id=$id"
        cat "$STATE/watch-rejects" >> "$STATE/rejected-triggers" 2>/dev/null;;
      UNBOUND)
        log "IGNORED: approval from '''$who''' (message $id) cites no sha256 for this handoff"
        rec last-rejected-trigger "unbound author=$who message_id=$id expected_digest=$want_dig";;
      *) : ;;
    esac
    local nxt; nxt=$(python3 -I -c 'import sys,json;print(json.load(open(sys.argv[1])).get("next_since",""))' "$out" 2>/dev/null)
    [[ -n "$nxt" && "$nxt" != "$since" ]] && since="$nxt" || break
  done
  log "watch window closed with no authorized approval"
  rec terminal-status "watch_expired"; return 78
}

case "${1:-prepare}" in
  prepare) prepare "${2:?handoff}";;
  commit)  commit "${2:-}";;
  watch)   watch "${2:?handoff}";;
  *) echo "usage: $0 {prepare <handoff>|commit [handoff]|watch <handoff>}" >&2; exit 64;;
esac
