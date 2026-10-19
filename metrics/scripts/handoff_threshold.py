#!/usr/bin/env python3
"""What an earlier context handoff is worth, replayed against real builder traces.

Usage:
  python3 -I handoff_threshold.py ~/.codex/sessions/2026/10/*/rollout-*.jsonl
  python3 -I handoff_threshold.py --model gpt-6-sol --min-calls 80 <rollouts...>

Method. For each thread, context per call is last_token_usage.input_tokens.
The *work stream* is the per-call increment d_i = C_i - C_{i-1}, with a drop
below 60% of the previous context read as a context replacement whose new base
enters cold. Replaying those increments under a policy "reset to R whenever
context would exceed T" gives the counterfactual cost of handing off earlier,
holding the work the session performs constant.

R is the whole restart, not just the written brief: standing floor + brief +
whatever the successor re-reads before it is productive. That third term is
what decides the answer, so sweep it rather than assuming it is zero.

Caveat the result cannot escape: the replay carries only the rediscovery that
the observed traces already contained at their own replacements. Rediscovery on
the *additional* resets a policy introduces is modelled by --rediscovery, not
measured. Per-call usage is info.last_token_usage; info.total_token_usage is
account-lifetime cumulative.
"""
import json, sys, collections

PRICE = 6.16 / 1e6     # Plus-units per weighted token, Pro, 2026-10-15 window
OPENING = 18600        # measured standing floor of a fresh thread


def threads(paths, model, min_calls):
    th = collections.defaultdict(list)
    path_of = {}
    for f in paths:
        tid = mdl = None
        for line in open(f, errors="ignore"):
            if '"token_count"' not in line and '"session_meta"' not in line \
               and '"turn_context"' not in line:
                continue
            try:
                d = json.loads(line)
            except Exception:
                continue
            p = d.get("payload")
            if not isinstance(p, dict):
                continue
            if d.get("type") == "session_meta":
                tid = p.get("id")
                path_of[tid] = p.get("agent_path") or "-"
            elif d.get("type") == "turn_context":
                mdl = p.get("model")
            elif p.get("type") == "token_count":
                l = (p.get("info") or {}).get("last_token_usage") or {}
                i_, o = l.get("input_tokens", 0), l.get("output_tokens", 0)
                if not (i_ or o) or (model and mdl != model):
                    continue
                th[tid].append((d.get("timestamp"), i_, o))
    out = []
    for tid, rs in th.items():
        if len(rs) < min_calls:
            continue
        rs.sort()
        seq, prev = [], None
        for _, C, o in rs:
            d = C if prev is None or C < prev * 0.6 else max(0, C - prev)
            seq.append((d, o))
            prev = C
        out.append((path_of.get(tid, "-"), seq, max(r[1] for r in rs)))
    return out


def sim(seq, T, R):
    """Weighted-token cost of replaying seq under reset-at-T, restart-cost-R."""
    C = None
    cost = 0.0
    resets = 0
    for d, o in seq:
        if C is None:
            C = d
            cost += d + 5 * o
            continue
        if C + d > T:
            cost += R          # restart is re-sent cold: nothing cached
            C = R
            resets += 1
        cost += d + C / 10.0 + 5 * o
        C += d
    return cost, resets


def main(argv):
    model, min_calls, redis = "gpt-6-sol", 80, None
    paths = []
    i = 0
    while i < len(argv):
        a = argv[i]
        if a == "--model":
            i += 1; model = argv[i] if argv[i] != "any" else None
        elif a == "--min-calls":
            i += 1; min_calls = int(argv[i])
        elif a == "--rediscovery":
            i += 1; redis = [int(x) for x in argv[i].split(",")]
        else:
            paths.append(a)
        i += 1
    if redis is None:
        redis = [0, 18000, 36000]

    th = threads(paths, model, min_calls)
    if not th:
        print("no threads matched"); return 1
    seqs = [t[1] for t in th]
    ncalls = sum(len(s) for s in seqs)
    ceiling = max(t[2] for t in th)
    base = sum(sim(s, ceiling, 0)[0] for s in seqs)
    print("%d threads, %d calls, observed ceiling %dK" % (len(th), ncalls, ceiling // 1000))
    print("replayed baseline %.1f Mweighted = %.0f Plus-units\n" % (base / 1e6, base * PRICE))

    print("%-8s %-10s | %-8s %7s %8s %8s %12s" % (
        "brief", "redisc.", "best T", "Mwt", "units", "save%", "calls/reset"))
    for brief in (20000, 30000, 50000):
        for rd in redis:
            R = OPENING + brief + rd
            best = None
            for T in range(50000, ceiling + 1, 10000):
                if T <= R + 10000:
                    continue
                tot = rst = 0
                for s in seqs:
                    c, n = sim(s, T, R)
                    tot += c; rst += n
                sav = 100 * (1 - tot / base)
                if best is None or sav > best[0]:
                    best = (sav, T, tot, rst)
            if best is None:
                print("%-8s %-10s | restart exceeds every threshold" % (
                    "%dK" % (brief // 1000), "%dK" % (rd // 1000)))
                continue
            sav, T, tot, rst = best
            print("%-8s %-10s | %-8s %7.1f %8.0f %+7.1f%% %12.1f" % (
                "%dK" % (brief // 1000), "%dK" % (rd // 1000), "%dK" % (T // 1000),
                tot / 1e6, tot * PRICE, sav, ncalls / rst if rst else 0))

    print("\nbreak-even on TOTAL restart size (floor + brief + rediscovery):")
    for R in range(40000, 160001, 20000):
        best = None
        for T in range(R + 20000, ceiling + 1, 10000):
            tot = sum(sim(s, T, R)[0] for s in seqs)
            sav = 100 * (1 - tot / base)
            if best is None or sav > best[0]:
                best = (sav, T)
        print("   restart %3dK -> best threshold %3dK, saving %+5.1f%%" % (
            R // 1000, best[1] // 1000, best[0]))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
