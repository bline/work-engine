#!/usr/bin/env python3
"""Supervisor and worker spend in the shape the orchestration brief asks for:
units, waits, messages and status probes per dispatch, timeout rate, context
per call against the handoff trigger, and whether any turn was started by a
child completion.

Usage: python3 -I supervision_profile.py ~/.codex/sessions/2026/10/13/rollout-*.jsonl
       FROM=2026-10-12 TO=2026-10-13 python3 -I supervision_profile.py <rollouts>

FROM/TO bound the calls counted by their entry timestamp, which is what makes a
long-lived root comparable with itself across two eras: the supervising roots in
this repository run for days, so a whole-file total mixes eras and answers
nothing.

Waiting is read from paired call/output entries: wait_agent's requested
timeout_ms comes from the call, and whether it expired comes from the output's
"timed_out" flag, so a wait released by child completion is never counted as a
timeout. A model call is "wait-only" when every tool call it issued was
coordination (wait, list_agents, sleep, status probes) and none was exec,
dispatch or messaging: such a call pays the full standing-context charge and
learns nothing new.

Turn triggers come from task_started.turn_attribution.turn_trigger, so
"did the parent resume on completion" is answered by the harness's own
attribution rather than inferred. task_started also carries
model_context_window, which the handoff trigger is computed from (60% of the
window, per the brief) instead of hardcoding a token count.

Per-call token usage is info.last_token_usage. info.total_token_usage is
account-lifetime cumulative and must not be read as a session total.
"""
import json, sys, os, collections

FROM = os.environ.get("FROM", "0000-00-00")
TO = os.environ.get("TO", "9999-99-99")

COEF = 7.826/1e6
WAITY = {"wait_agent", "wait", "sleep", "list_agents", "request_user_input_async"}
DISPATCH = {"spawn_agent", "followup_task", "interrupt_agent"}

def scan(f):
    meta = {}
    calls = {}                 # call_id -> name
    pending = []               # tool-call names since the last token_count
    units = collections.Counter()
    n = collections.Counter()
    waitonly = collections.Counter()
    triggers = collections.Counter()
    window = 0
    names = collections.Counter()
    waits = []                 # (timeout_ms, timed_out)
    ctx = []
    for i, line in enumerate(open(f, errors="ignore")):
        if i == 0:
            try: meta = json.loads(line)["payload"]
            except Exception: pass
            continue
        try: o = json.loads(line)
        except Exception: continue
        p = o.get("payload") or {}
        t = p.get("type")
        stamp = (o.get("timestamp") or "")[:10]
        if t in ("function_call", "custom_tool_call"):
            nm = p.get("name")
            if FROM <= stamp <= TO: names[nm] += 1
            pending.append(nm)
            calls[p.get("call_id")] = (nm, p.get("input") or p.get("arguments") or "")
            continue
        if t in ("function_call_output", "custom_tool_call_output"):
            nm, body = calls.get(p.get("call_id"), ("?", ""))
            if nm == "wait_agent":
                try: req = json.loads(body).get("timeout_ms")
                except Exception: req = None
                out = p.get("output")
                if isinstance(out, str):
                    try: out = json.loads(out)
                    except Exception: out = {}
                if FROM <= stamp <= TO:
                    waits.append((req, bool((out or {}).get("timed_out"))))
            continue
        if t == "task_started":
            if FROM <= stamp <= TO:
                triggers[(p.get("turn_attribution") or {}).get("turn_trigger") or "?"] += 1
                window = max(window, p.get("model_context_window") or 0)
            continue
        if t != "token_count": continue
        if not (FROM <= stamp <= TO):
            pending = []
            continue
        l = ((p.get("info") or {}).get("last_token_usage")) or {}
        i_, c, o = l.get("input_tokens", 0), l.get("cached_input_tokens", 0), l.get("output_tokens", 0)
        acts, pending = pending, []
        if i_ == 0 and o == 0: continue
        u = ((i_ - c) + c/10 + 5*o) * COEF
        ctx.append(i_)
        if acts and all(a in WAITY for a in acts):
            k = "wait-only"
            waitonly[" + ".join(sorted(set(acts)))] += 1
        elif any(a in DISPATCH for a in acts): k = "dispatch"
        elif any(a == "send_message" for a in acts): k = "message"
        elif acts: k = "work"
        else: k = "respond/reason"
        units[k] += u; n[k] += 1
    return meta, units, n, names, waits, ctx, waitonly, triggers, window

rows = []
for f in sys.argv[1:]:
    try: rows.append((f,) + scan(f))
    except Exception as e: print("ERR", f, e)
rows.sort(key=lambda r: (r[1].get("timestamp") or ""))

tot = sum(sum(r[2].values()) for r in rows)
sup = 0.0
for f, meta, units, n, names, waits, ctx, waitonly, triggers, window in rows:
    if not sum(units.values()): continue
    u = sum(units.values())
    disp = sum(names[k] for k in DISPATCH)
    if disp: sup += u
    to = sum(1 for _, x in waits if x)
    secs = sum((r or 0) for r, _ in waits)/1000.0
    print("%-14s %-38s %-7s units=%7.1f calls=%-4d ctx=%5.0fK" % (
        (meta.get("timestamp") or "")[5:16], (meta.get("agent_path") or "-")[:38],
        meta.get("agent_role") or "root", u, sum(n.values()), (sum(ctx)/len(ctx)/1000 if ctx else 0)))
    probes = sum(names[k] for k in ("list_agents", "sleep", "wait"))
    msgs = names["send_message"]
    d = max(disp, 1)
    print("     wait-only %5.1f u (%4.1f%%, %d calls) | dispatch %d | waits %d (timed out %d of %d = %.0f%%, requested %.0fs total%s)" % (
        units["wait-only"], 100*units["wait-only"]/u, n["wait-only"], disp, len(waits), to, len(waits),
        100*to/max(len(waits), 1), secs,
        ", longest %.0fs" % (max((r or 0) for r, _ in waits)/1000.0) if waits else ""))
    if disp:
        print("     per dispatch: units %.2f | waits %.2f | messages %.2f | status probes %.2f | wait-only calls %.2f" % (
            u/d, len(waits)/d, msgs/d, probes/d, n["wait-only"]/d))
    if ctx:
        trigger = 0.6*window if window else 0
        peak = max(ctx)
        print("     context: mean %.0fK, peak %.0fK of a %.0fK window%s" % (
            sum(ctx)/len(ctx)/1000, peak/1000, window/1000,
            "; peak %s the %.0fK handoff trigger" % ("CROSSED" if peak >= trigger else "under", trigger/1000) if trigger else ""))
    if triggers:
        print("     turn triggers:", dict(triggers.most_common()),
              "<- a completion-started turn would not say 'user'")
    print("     by kind:", {k: round(v, 1) for k, v in units.most_common()},
          "| coord calls:", {k: v for k, v in names.items() if k in WAITY | DISPATCH | {"send_message"}})
    if waitonly: print("     wait-only calls made of:", dict(waitonly.most_common(6)))
print("\nwindow total %.1f units; sessions that dispatched: %.1f (%.0f%%)" % (tot, sup, 100*sup/max(tot, 1e-9)))
