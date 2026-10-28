#!/usr/bin/env python3
"""Call-level usage ledger for Codex and Claude Code (F1, metrics/experiment-plan-20261026.md).

Usage
  python3 -I usage_ledger.py build --since ISO [--until ISO] [--no-codex] [--no-claude]
                                   [--streams GLOB ...] --out DIR
      Writes DIR/calls.jsonl (one row per model call), DIR/quota.jsonl (provider
      utilisation readings), DIR/level1.json (reconstructed totals against provider
      cumulative counters), DIR/issues.jsonl, DIR/summary.json and, since F1.1,
      DIR/turns.jsonl (completed Codex turns with recorded durations).
  python3 -I usage_ledger.py reconcile-trial MEASUREMENT.json [--out FILE]
      Recomputes every actor of a four-builder-measurement-v1 file from raw sources
      with this ledger's readers and reports per-actor and per-arm differences.
      Exit 0 = every compared value matches, 21 = a difference, 32 = an actor could
      not be recomputed (not_established, never a negative result).
  python3 -I usage_ledger.py windows DIR
      Codex calls grouped by weekly quota window and model, with observed movement
      and modelled cost kept separate.

Counting rules
  Codex. One call per unique response_id from top-level `token_usage_record`
  records (payload.usage). A record whose payload.thread_id differs from the
  rollout's session_meta.id is a copy and is excluded. A response_id seen twice
  with different usage is an issue, never summed. Rollouts with no
  token_usage_record fall back to event_msg/token_count `last_token_usage`, keyed
  by (thread, timestamp, counters) and flagged dedupe_basis=fallback. Model and
  effort come from the latest preceding turn_context, not the thread index.
  Reads .jsonl and .jsonl.zst.
  Claude. One call per message.id across all transcripts (main and subagents/).
  Input and cache counters must agree across a message's records; output_tokens
  takes the maximum. Model `<synthetic>` is not an API call.
  Quota readings are a separate table and are never mixed with modelled cost:
  Codex `rate_limits` on token_count events; Claude `rate_limit_event` in
  stream-json files given with --streams.
  Attribution that cannot be read from the source stays null; role_heuristic is a
  name-pattern guess and is labelled as such.

Pricing is the frozen Standard API scenario of
anthropic-builder-baselines-20261027 (measurement/final-refresh-v2.json "pricing"):
not a bill and not subscription quota. A model without a frozen rate is unpriced
(usd null), never estimated. `weighted` is the pure-Codex baseline unit,
uncached + cached/10 + 5*output.

F1.1 (2026-10-28) adds fields and one file; every F1 field, total and file is
unchanged.
  Tier. Codex calls carry `service_tier`, the thread's tier at the call from the
  latest preceding `thread_settings_applied` (Fast is `priority`, standard is
  `default`; null when the rollout records none). `quota_multiplier` is 2.5 for
  `priority` and 1 for `default` (Codex docs: Fast draws included usage at 2.5x
  the standard rate); null otherwise. `usd` stays the list-price scenario.
  Claude calls carry `service_tier` and `speed` from the message usage.
  Time. `latency_s` is the call's record time minus the latest preceding input
  in the same file (Codex: turn start, user message, tool output; Claude: user
  record), `latency_basis` says how it was derived, and a call with no input since
  the previous call has null latency. It includes queueing and is an estimate.
  turns.jsonl holds each completed Codex turn with the `duration_ms` and
  `time_to_first_token_ms` Codex records.
"""
import argparse, collections, datetime as dt, glob, io, json, os, re, sqlite3, subprocess, sys

CODEX_KEYS = ('input_tokens', 'cached_input_tokens', 'cache_write_input_tokens',
              'output_tokens', 'reasoning_output_tokens', 'total_tokens')
NATIVE_KEYS = ('input_tokens', 'cache_creation_input_tokens', 'cache_read_input_tokens', 'output_tokens')
TTL_KEYS = ('ephemeral_5m_input_tokens', 'ephemeral_1h_input_tokens')
CODEX_RATES = {'gpt-6.1-terra': (1, .05, 1.25, 5), 'gpt-6-sol': (2, .2, 2.5, 10),
               'gpt-6.1-sol': (2, .1, 2.5, 10), 'gpt-6-astra': (10, 1, 12.5, 50)}
NATIVE_RATES = {
    'claude-sonnet-5-5': dict(input=2, output=10, cache_write_5m=2.5, cache_write_1h=4, cache_read=.1),
    'claude-opus-5-5': dict(input=4, output=20, cache_write_5m=5, cache_write_1h=8, cache_read=.2)}
HOME = os.path.expanduser('~')
CODEX_GLOBS = [f'{HOME}/.codex/sessions/**/rollout-*.jsonl', f'{HOME}/.codex/sessions/**/rollout-*.jsonl.zst',
               f'{HOME}/.codex/archived_sessions/*.jsonl', f'{HOME}/.codex/archived_sessions/*.jsonl.zst']
CLAUDE_GLOB = f'{HOME}/.claude/projects/**/*.jsonl'
STATE_DB = f'{HOME}/.codex/state_5.sqlite'
# F1.1: rollout line prefix, read without parsing large tool outputs.
PREFIX = re.compile(r'^\{"timestamp":"([^"]+)",(?:"ordinal":\d+,)?"type":"(\w+)","payload":\{"type":"(\w+)"')
INPUT_ITEMS = ('function_call_output', 'custom_tool_call_output')
QUOTA_MULTIPLIER = {'default': 1.0, 'priority': 2.5}
CLAUDE_LATENCY_START = {}   # message id -> its latency start, for later records of the same message

def seconds_between(start, end):
    if not start or not end: return None
    s = (ts_of(end) - ts_of(start)).total_seconds()
    return s if s >= 0 else None

ISSUES = []
def issue(kind, **kw): ISSUES.append(dict(kind=kind, **kw))
def ts_of(s): return dt.datetime.fromisoformat(s.replace('Z', '+00:00')) if s else None
def in_window(ts, since, until):
    t = ts_of(ts)
    return t is not None and (since is None or t >= since) and (until is None or t <= until)

def open_lines(path):
    if path.endswith('.zst'):
        try:
            from compression import zstd
            with zstd.open(path, 'rt', encoding='utf-8', errors='replace') as fh:
                yield from fh
        except ImportError:
            out = subprocess.run(['zstd', '-dc', path], capture_output=True, check=True).stdout
            yield from io.StringIO(out.decode('utf-8', 'replace'))
    else:
        with open(path, encoding='utf-8', errors='replace') as fh:
            yield from fh

def role_heuristic(agent_path):
    if not agent_path: return 'root'
    leaf = agent_path.rstrip('/').split('/')[-1].lower()
    if re.search(r'review|audit|admission|verif|discriminat|astra|judge|score', leaf): return 'review'
    if re.search(r'supervis|coordinat|orchestr', leaf): return 'supervise'
    if re.search(r'build|worker|implement|repair|remediat|fix|realiz|successor|engineering', leaf): return 'build'
    return None

# ---------------------------------------------------------------- pricing
def codex_usd(u, model):
    if model not in CODEX_RATES: return None
    i, c, w, o = (u.get(k, 0) for k in ('input_tokens', 'cached_input_tokens', 'cache_write_input_tokens', 'output_tokens'))
    if c + w > i: raise ValueError('Codex cache components exceed input')
    ri, rc, rw, ro = CODEX_RATES[model]
    m, mo = (2, 1.5) if i > 272000 else (1, 1)
    return ((i - c - w) * ri + c * rc + w * rw) * m / 1e6 + o * ro * mo / 1e6

def native_usd(u, model):
    """(low, high, unknown_ttl_tokens); low == high when every cache write has a known TTL."""
    r = NATIVE_RATES.get(model)
    if not r or any(k not in u for k in NATIVE_KEYS): return None, None, None
    w = u['cache_creation_input_tokens']; w5 = u.get(TTL_KEYS[0], 0); w1 = u.get(TTL_KEYS[1], 0)
    if w5 + w1 > w: raise ValueError('TTL exceeds cache write')
    un = w - w5 - w1
    v = (u['input_tokens'] * r['input'] + u['output_tokens'] * r['output'] + u['cache_read_input_tokens'] * r['cache_read']
         + w5 * r['cache_write_5m'] + w1 * r['cache_write_1h']) / 1e6
    lo = min(r['cache_write_5m'], r['cache_write_1h']); hi = max(r['cache_write_5m'], r['cache_write_1h'])
    return v + un * lo / 1e6, v + un * hi / 1e6, un

def codex_weighted(u):
    return u.get('input_tokens', 0) - u.get('cached_input_tokens', 0) + u.get('cached_input_tokens', 0) / 10 + 5 * u.get('output_tokens', 0)

def native_weighted(u):
    return u.get('input_tokens', 0) + u.get('cache_creation_input_tokens', 0) + u.get('cache_read_input_tokens', 0) / 10 + 5 * u.get('output_tokens', 0)

# ---------------------------------------------------------------- Codex
def codex_rollout(path, seen, turns=None):
    """Return (meta, calls, quota, level1) for one rollout. `seen` maps response_id -> usage across files.
    F1.1: completed turns are appended to `turns` when a list is given."""
    meta = {}; model = effort = None; calls = []; fallback = []; quota = []; last_thread = None; n_tur = 0
    fork_offset = None   # a forked thread's cumulative counter starts at its source thread's total
    tier = None; last_input = None   # F1.1
    for n, line in enumerate(open_lines(path), 1):
        m = PREFIX.match(line)
        if m:
            mts, mtyp, ptyp = m.groups()
            if mtyp == 'response_item' and (ptyp in INPUT_ITEMS or (ptyp == 'message' and '"role":"user"' in line[:400])):
                last_input = mts
            elif mtyp == 'event_msg' and ptyp == 'task_started':
                last_input = mts
            elif mtyp == 'event_msg' and ptyp in ('thread_settings_applied', 'task_complete'):
                try: e = json.loads(line)
                except ValueError: e = None
                ep = (e or {}).get('payload') or {}
                own = not ep.get('thread_id') or not meta.get('id') or ep['thread_id'] == meta['id']
                if ptyp == 'thread_settings_applied' and own:
                    tier = (ep.get('thread_settings') or {}).get('service_tier')
                elif ptyp == 'task_complete' and e and turns is not None:
                    turns.append(dict(provider='codex', thread=meta.get('id'), turn_id=ep.get('turn_id'), completed_ts=mts,
                                      started_at=ep.get('started_at'), completed_at=ep.get('completed_at'),
                                      duration_ms=ep.get('duration_ms'), time_to_first_token_ms=ep.get('time_to_first_token_ms'),
                                      service_tier=tier, model=model, effort=effort, agent_path=meta.get('agent_path'),
                                      source=f'{path}:{n}'))
        if not any(k in line for k in ('session_meta', 'turn_context', 'token_usage_record', 'token_count')): continue
        try: d = json.loads(line)
        except ValueError:
            issue('malformed_line', path=path, line=n); continue
        typ = d.get('type'); p = d.get('payload') or {}; ts = d.get('timestamp')
        if typ == 'session_meta' and not meta:
            meta = {k: p.get(k) for k in ('id', 'parent_thread_id', 'agent_path', 'agent_nickname', 'originator', 'thread_source', 'timestamp')}
        elif typ == 'turn_context':
            model = p.get('model'); effort = p.get('effort')
        elif typ == 'token_usage_record':
            n_tur += 1
            if p.get('thread_id') and meta.get('id') and p['thread_id'] != meta['id']:
                issue('copied_codex_usage_excluded', path=path, line=n, thread=p['thread_id']); continue
            latency = seconds_between(last_input, ts); latency_basis = 'since_preceding_input' if latency is not None else 'no_preceding_input'
            last_input = None
            rid = p.get('response_id'); u = {k: (p.get('usage') or {}).get(k, 0) for k in CODEX_KEYS}
            if p.get('thread_token_usage'):
                last_thread = p['thread_token_usage']
                if fork_offset is None:
                    fork_offset = {k: last_thread.get(k, 0) - u[k] for k in CODEX_KEYS}
            if not rid:
                issue('missing_response_id', path=path, line=n); continue
            if rid in seen:
                if seen[rid] != u: issue('conflicting_repeated_response', path=path, line=n, response_id=rid)
                continue
            seen[rid] = u
            calls.append(dict(ts=ts, response_id=rid, usage=u, model=model, effort=effort, line=n, dedupe_basis='response_id',
                              tier=tier, latency_s=latency, latency_basis=latency_basis))
        elif typ == 'event_msg' and p.get('type') == 'token_count':
            rl = p.get('rate_limits') or {}
            for wname in ('primary', 'secondary'):
                w = rl.get(wname) or {}
                if w.get('used_percent') is not None:
                    quota.append(dict(provider='codex', ts=ts, window=wname, window_minutes=w.get('window_minutes'),
                                      used_percent=w.get('used_percent'), resets_at=w.get('resets_at'),
                                      limit_id=rl.get('limit_id'), plan_type=rl.get('plan_type'), thread=meta.get('id'), source=path))
            lu = ((p.get('info') or {}).get('last_token_usage')) or {}
            if lu.get('input_tokens', 0) or lu.get('output_tokens', 0):
                fallback.append(dict(ts=ts, usage={k: lu.get(k, 0) for k in CODEX_KEYS}, model=model, effort=effort, line=n,
                                     tier=tier, latency_s=None, latency_basis='fallback_record'))
    if n_tur == 0 and fallback:
        keyed = {}
        for c in fallback:
            k = (meta.get('id') or path, c['ts'], c['usage']['input_tokens'], c['usage']['output_tokens'])
            if k in seen: continue
            seen[k] = c['usage']; keyed[k] = c
        calls = [dict(c, response_id=None, dedupe_basis='fallback') for c in keyed.values()]
    level1 = None
    if last_thread is not None:
        got = {k: sum(c['usage'][k] for c in calls) for k in CODEX_KEYS}
        off = fork_offset or {}
        level1 = dict(thread=meta.get('id'), path=path, reconstructed=got, provider_thread_total=last_thread,
                      starting_offset=off if any(off.values()) else None,
                      delta={k: got[k] - (last_thread.get(k, 0) - off.get(k, 0)) for k in CODEX_KEYS})
    return meta, calls, quota, level1

def thread_index():
    if not os.path.exists(STATE_DB): return {}
    try:
        con = sqlite3.connect(f'file:{STATE_DB}?mode=ro', uri=True)
        cols = [r[1] for r in con.execute('pragma table_info(threads)')]
        want = [c for c in ('id', 'agent_path', 'agent_role', 'model', 'reasoning_effort', 'thread_source') if c in cols]
        return {r[0]: dict(zip(want, r)) for r in con.execute(f'select {",".join(want)} from threads')}
    except sqlite3.Error as e:
        issue('thread_index_unreadable', error=str(e)); return {}

def codex_files(since):
    out = set()
    for g in CODEX_GLOBS:
        for f in glob.glob(g, recursive=True):
            if since is None or os.path.getmtime(f) >= since.timestamp(): out.add(f)
    return sorted(out)

def build_codex(since, until, turns=None):
    idx = thread_index(); seen = {}; metas = {}; rows = []; quota = []; level1 = []
    files = codex_files(since)
    for f in files:
        file_turns = []
        meta, calls, q, l1 = codex_rollout(f, seen, file_turns)
        if turns is not None:
            turns += [t for t in file_turns if in_window(t['completed_ts'], since, until)]
        if meta.get('id'): metas[meta['id']] = meta
        quota += [x for x in q if in_window(x['ts'], since, until)]
        if l1: level1.append(l1)
        for c in calls:
            if not in_window(c['ts'], since, until): continue
            u = c['usage']; tid = meta.get('id')
            rows.append(dict(provider='codex', ts=c['ts'], call_id=c['response_id'], dedupe_basis=c['dedupe_basis'],
                             thread=tid, parent_thread=meta.get('parent_thread_id'), agent_path=meta.get('agent_path'),
                             originator=meta.get('originator'), model=c['model'], effort=c['effort'],
                             role_index=(idx.get(tid) or {}).get('agent_role'), role_heuristic=role_heuristic(meta.get('agent_path')),
                             usage=u, weighted=codex_weighted(u), usd=codex_usd(u, c['model']), source=f'{f}:{c["line"]}',
                             service_tier=c.get('tier'), quota_multiplier=QUOTA_MULTIPLIER.get(c.get('tier')),
                             latency_s=c.get('latency_s'), latency_basis=c.get('latency_basis')))
    for r in rows:   # root ancestor by recorded parent links; unknown stays null
        cur, hops = r['thread'], 0
        while cur in metas and metas[cur].get('parent_thread_id') and hops < 20:
            cur = metas[cur]['parent_thread_id']; hops += 1
        r['root_thread'] = cur if (cur in metas or hops) else None; r['depth'] = hops
    return rows, quota, level1, len(files)

# ---------------------------------------------------------------- Claude
def claude_transcript(path, msgs, cost_state, members=None):
    """members, if given, maps transcript path -> set of message ids it contains, including copies."""
    sub = '/subagents/' in path
    mine = members.setdefault(path, set()) if members is not None else set()
    agent = os.path.basename(path)[:-6] if sub else None
    agent_type = None
    if sub and os.path.exists(path[:-6] + '.meta.json'):
        try: agent_type = json.load(open(path[:-6] + '.meta.json')).get('agentType')
        except (ValueError, OSError): pass
    project = path.split('/.claude/projects/')[1].split('/')[0]
    last_user = None   # F1.1
    for n, line in enumerate(open_lines(path), 1):
        if '"type":"user"' in line[:300]:
            try: uo = json.loads(line)
            except ValueError: uo = None
            if uo and uo.get('type') == 'user' and uo.get('timestamp'): last_user = uo['timestamp']
        if '"cost-state"' in line:
            try:
                o = json.loads(line)
                if o.get('type') == 'cost-state': cost_state[o.get('sessionId')] = dict(o, _path=path)
            except ValueError: pass
            continue
        if '"usage"' not in line or '"assistant"' not in line: continue
        try: o = json.loads(line)
        except ValueError:
            issue('malformed_line', path=path, line=n); continue
        if o.get('type') != 'assistant': continue
        m = o.get('message') or {}; u = m.get('usage') or {}; mid = m.get('id'); model = m.get('model')
        if model == '<synthetic>' or not mid or not u: continue
        mine.add(mid)
        flat = {k: u[k] for k in NATIVE_KEYS if k in u}
        for k in TTL_KEYS:
            if k in (u.get('cache_creation') or {}): flat[k] = u['cache_creation'][k]
        if mid in msgs:
            row = msgs[mid]
            if row.get('latency_s') is not None:   # F1.1: a message's records span the call; latency runs to the last one
                later = seconds_between(CLAUDE_LATENCY_START.get(mid), o.get('timestamp'))
                if later is not None and later > row['latency_s']: row['latency_s'] = later
            for k, v in flat.items():
                if k == 'output_tokens': row['usage'][k] = max(row['usage'].get(k, 0), v)
                elif k in row['usage'] and row['usage'][k] != v:
                    issue('conflicting_native_counter', message_id=mid, counter=k, path=path, line=n)
                else: row['usage'][k] = v
            row['records'] += 1
            continue
        latency = seconds_between(last_user, o.get('timestamp'))
        if latency is not None: CLAUDE_LATENCY_START[mid] = last_user
        last_user = None
        msgs[mid] = dict(provider='anthropic', ts=o.get('timestamp'), call_id=mid, dedupe_basis='message_id',
                         session=o.get('sessionId'), project=project, subagent=sub, agent=agent, agent_type=agent_type,
                         model=model, effort=o.get('effort'), usage=flat, records=1, source=f'{path}:{n}',
                         service_tier=u.get('service_tier'), speed=u.get('speed'), quota_multiplier=None,
                         latency_s=latency, latency_basis='since_preceding_user_record' if latency is not None else 'no_preceding_input')

def build_claude(since, until):
    msgs = {}; cost_state = {}; members = {}
    files = sorted(f for f in glob.glob(CLAUDE_GLOB, recursive=True)
                   if since is None or os.path.getmtime(f) >= since.timestamp())
    for f in files: claude_transcript(f, msgs, cost_state, members)
    rows = []
    for r in msgs.values():
        if not in_window(r['ts'], since, until): continue
        lo, hi, un = native_usd(r['usage'], r['model'])
        r.update(weighted=native_weighted(r['usage']), usd=lo if lo == hi else None, usd_low=lo, usd_high=hi, unknown_ttl_tokens=un)
        rows.append(r)
    # Level 1: cost-state modelUsage (the CLI's running counter) against every message
    # the session's transcript and its subagents/ files contain, copies included.
    # The counter's semantics across resume are not established, so each difference
    # is classified, not forced to zero.
    level1 = []
    names = dict(inputTokens='input_tokens', outputTokens='output_tokens', cacheReadInputTokens='cache_read_input_tokens',
                 cacheCreationInputTokens='cache_creation_input_tokens')
    for sid, cs in cost_state.items():
        base = cs['_path']
        mids = set(members.get(base, set()))
        for f in glob.glob(os.path.join(os.path.dirname(base), sid, 'subagents', '*.jsonl')):
            mids |= members.get(f, set())
        per_model = collections.defaultdict(collections.Counter)
        for mid in mids:
            r = msgs[mid]
            for k in NATIVE_KEYS: per_model[r['model']][k] += r['usage'].get(k, 0)
        for model, mu in (cs.get('modelUsage') or {}).items():
            prov = {names[k]: v for k, v in mu.items() if k in names}
            got = per_model.get(model, collections.Counter())
            delta = {k: got.get(k, 0) - prov[k] for k in prov}
            if all(v == 0 for v in delta.values()): cls = 'exact'
            elif not any(got.values()): cls = 'model_absent_from_transcript'
            elif all(v <= 0 for v in delta.values()): cls = 'counter_exceeds_transcript'
            elif all(v >= 0 for v in delta.values()): cls = 'transcript_exceeds_counter'
            else: cls = 'mixed'
            level1.append(dict(session=sid, model=model, path=base, cli_cost_usd=mu.get('costUSD'), provider_session_total=prov,
                               reconstructed={k: got.get(k, 0) for k in prov}, delta=delta, cls=cls))
    return rows, level1, len(files)

def stream_quota(globs):
    rows = []
    for g in globs:
        for f in sorted(glob.glob(os.path.expanduser(g), recursive=True)):
            last_ts = None
            for n, line in enumerate(open_lines(f), 1):
                if '"timestamp"' in line:
                    m = re.search(r'"timestamp"\s*:\s*"([^"]+)"', line)
                    if m: last_ts = m.group(1)
                if '"rate_limit_event"' not in line: continue
                try: o = json.loads(line)
                except ValueError: continue
                info = o.get('rate_limit_info') or o
                for wname, w in (info.get('unifiedWindows') or {}).items():
                    rows.append(dict(provider='anthropic', ts=o.get('timestamp') or last_ts, ts_basis='record' if o.get('timestamp') else 'preceding_record',
                                     window=wname, used_percent=None if w.get('utilization') is None else 100 * w['utilization'],
                                     resets_at=w.get('resetsAt'), status=w.get('status'), source=f'{f}:{n}'))
    return rows

# ---------------------------------------------------------------- commands
def cmd_build(a):
    since = ts_of(a.since) if a.since else None; until = ts_of(a.until) if a.until else None
    os.makedirs(a.out, exist_ok=True)
    calls, quota, l1, turns = [], [], {}, []
    cov = {}
    if not a.no_codex:
        r, q, l, nf = build_codex(since, until, turns); calls += r; quota += q; l1['codex_threads'] = l; cov['codex_files'] = nf
    if not a.no_claude:
        r, l, nf = build_claude(since, until); calls += r; l1['claude_sessions'] = l; cov['claude_files'] = nf
    if a.streams: quota += stream_quota(a.streams)
    calls.sort(key=lambda r: r['ts'] or '')
    with open(os.path.join(a.out, 'calls.jsonl'), 'w') as fh:
        for r in calls: fh.write(json.dumps(r, sort_keys=True) + '\n')
    with open(os.path.join(a.out, 'quota.jsonl'), 'w') as fh:
        for r in quota: fh.write(json.dumps(r, sort_keys=True) + '\n')
    with open(os.path.join(a.out, 'issues.jsonl'), 'w') as fh:
        for r in ISSUES: fh.write(json.dumps(r, sort_keys=True) + '\n')
    json.dump(l1, open(os.path.join(a.out, 'level1.json'), 'w'), indent=1, sort_keys=True)
    turns.sort(key=lambda t: t['completed_ts'] or '')
    with open(os.path.join(a.out, 'turns.jsonl'), 'w') as fh:
        for t in turns: fh.write(json.dumps(t, sort_keys=True) + '\n')
    agg = collections.defaultdict(lambda: dict(calls=0, weighted=0.0, usd=0.0, unpriced_calls=0))
    for r in calls:
        g = agg[f"{r['provider']}|{r['model']}"]; g['calls'] += 1; g['weighted'] += r['weighted']
        if r['usd'] is None: g['unpriced_calls'] += 1
        else: g['usd'] += r['usd']
    def l1_summary(rows):
        bad = [x for x in rows if any(v != 0 for v in x['delta'].values())]
        out = dict(compared=len(rows), exact=len(rows) - len(bad), differing=len(bad),
                   with_starting_offset=sum(bool(x.get('starting_offset')) for x in rows))
        if rows and 'cls' in rows[0]:
            out['classes'] = collections.Counter(x['cls'] for x in rows)
            out['cli_cost_usd_by_class'] = {c: round(sum(x['cli_cost_usd'] or 0 for x in rows if x['cls'] == c), 2)
                                            for c in out['classes']}
        return out
    summary = dict(since=a.since, until=a.until, coverage=cov, calls=len(calls), quota_readings=len(quota),
                   fallback_calls=sum(r.get('dedupe_basis') == 'fallback' for r in calls),
                   unknown_model_calls=sum(r['model'] is None for r in calls),
                   level1={k: l1_summary(v) for k, v in l1.items()}, issues=collections.Counter(i['kind'] for i in ISSUES),
                   by_provider_model=dict(sorted(agg.items())), pricing='frozen Standard API scenario; not a bill or quota',
                   f1_1=f11_summary(calls, turns))
    json.dump(summary, open(os.path.join(a.out, 'summary.json'), 'w'), indent=1, sort_keys=True)
    print(json.dumps(summary, indent=1, sort_keys=True))

def f11_summary(calls, turns):
    """F1.1 additions: per provider|model|effort|tier counts, cost with the quota multiplier, and latency."""
    def med(xs):
        xs = sorted(xs); n = len(xs)
        return None if not n else (xs[n // 2] if n % 2 else (xs[n // 2 - 1] + xs[n // 2]) / 2)
    cells = collections.defaultdict(lambda: dict(calls=0, usd=0.0, quota_weighted_usd=0.0, quota_unweighted_calls=0, latencies=[]))
    for r in calls:
        tier = r.get('service_tier') if r['provider'] == 'codex' else r.get('speed')
        g = cells[f"{r['provider']}|{r['model']}|{r.get('effort')}|{tier}"]; g['calls'] += 1
        if r['usd'] is not None:
            g['usd'] += r['usd']
            if r.get('quota_multiplier') is None: g['quota_unweighted_calls'] += 1
            else: g['quota_weighted_usd'] += r['usd'] * r['quota_multiplier']
        if r.get('latency_s') is not None: g['latencies'].append(r['latency_s'])
    out = {}
    for k, g in sorted(cells.items()):
        lat = g.pop('latencies')
        out[k] = dict(g, latency_n=len(lat), latency_median_s=med(lat))
    tturns = [t for t in turns if t.get('duration_ms') is not None]
    return dict(key='provider|model|effort|tier (Codex service_tier; Claude speed)', by_cell=out,
                turns=len(turns), turn_duration_median_ms=med([t['duration_ms'] for t in tturns]),
                quota_note='quota_weighted_usd = usd x quota_multiplier where the tier is known; an estimate, not a reading')

def find_transcripts(sid):
    return sorted(set(glob.glob(f'{HOME}/.claude/projects/*/{sid}.jsonl') + glob.glob(f'{HOME}/.claude/projects/*/{sid}/subagents/*.jsonl')))

def recompute_codex_actor(act):
    path = act.get('rollout')
    if not path or not os.path.exists(path): return None
    _, calls, _, _ = codex_rollout(path, {})
    s = ts_of(act.get('start_boundary')); e = ts_of(act.get('cutoff'))
    calls = [c for c in calls if in_window(c['ts'], s, e)]
    return dict(calls=len(calls), usage={k: sum(c['usage'][k] for c in calls) for k in CODEX_KEYS},
                usd=sum(codex_usd(c['usage'], c['model']) for c in calls))

def recompute_native(nb, cutoff, include_subagents):
    msgs, cs = {}, {}
    for sid in nb.get('sessions') or []:
        for f in find_transcripts(sid):
            if include_subagents or '/subagents/' not in f: claude_transcript(f, msgs, cs)
    rows = [r for r in msgs.values() if r['session'] in set(nb['sessions']) and in_window(r['ts'], None, ts_of(cutoff))]
    usage = collections.Counter()
    lo = hi = 0.0
    for r in rows:
        usage.update({k: r['usage'].get(k, 0) for k in NATIVE_KEYS + TTL_KEYS})
        l, h, _ = native_usd(r['usage'], r['model']); lo += l or 0; hi += h or 0
    return dict(calls=len(rows), usage=dict(usage), usd_low=lo, usd_high=hi)

def cmd_reconcile(a):
    m = json.load(open(a.measurement)); out = dict(measurement=a.measurement, actors=[], groups=[]); status = 0
    def cmp(label, provider, exp, got, keys):
        nonlocal status
        if got is None:
            out['actors'].append(dict(label=label, provider=provider, result='not_established')); status = max(status, 32); return
        d = {'calls': got['calls'] - exp['calls']}
        for k in keys: d[k] = got['usage'].get(k, 0) - (exp['usage'] or {}).get(k, 0)
        d['usd'] = round(got.get('usd', got.get('usd_low', 0)) - exp['usd'], 9)
        ok = all(v == 0 for v in d.values())
        if not ok: status = max(status, 21)
        out['actors'].append(dict(label=label, provider=provider, match=ok, expected=exp, recomputed=got, delta=d))
    actors = []
    for name, h in (m.get('historical') or {}).items():
        actors += [(f'historical.{name}', x) for x in h.get('actors') or []]
        if h.get('root_coordination'): actors.append((f'historical.{name}', h['root_coordination']))
    for name, arm in (m.get('arms') or {}).items():
        actors += [(f'arms.{name}', x) for x in arm.get('codex_actors') or []]
    actors += [('common_overhead', x) for x in (m.get('common_overhead') or {}).get('actors') or []]
    for grp, act in actors:
        exp = dict(calls=act['calls'], usage=act['usage'], usd=act['standard_api_scenario_usd'])
        cmp(f"{grp}:{act.get('role')}:{act.get('agent_path')}:{act['session'][:8]}", 'codex', exp, recompute_codex_actor(act), CODEX_KEYS)
    for name, arm in (m.get('arms') or {}).items():
        nb = arm.get('native_builder')
        if not nb: continue
        exp = dict(calls=nb['calls'], usage=nb['usage'], usd=nb['standard_api_scenario_usd'])
        for sub in (False, True):
            got = recompute_native(nb, m.get('cutoff'), sub)
            cmp(f"arms.{name}:native_builder:{'with' if sub else 'without'}_subagents", 'anthropic', exp, got, NATIVE_KEYS + TTL_KEYS)
    # per-arm groups from the measurement's own grouping, recomputed from matched actors
    for name, arm in (m.get('arms') or {}).items():
        for g in ('builder_only', 'builder_plus_reviewers', 'coordination', 'attributable_total_excludes_common'):
            out['groups'].append(dict(arm=name, group=g, expected_usd=(arm.get(g) or {}).get('standard_api_scenario_usd'),
                                      expected_calls=(arm.get(g) or {}).get('calls')))
    out['status'] = {0: 'pass', 21: 'defect', 32: 'not_established'}[status]
    out['summary'] = dict(actors=len(out['actors']), matched=sum(x.get('match') is True for x in out['actors']),
                          differing=sum(x.get('match') is False for x in out['actors']),
                          not_established=sum(x.get('result') == 'not_established' for x in out['actors']))
    s = json.dumps(out, indent=1, sort_keys=True)
    if a.out: open(a.out, 'w').write(s)
    print(json.dumps(dict(status=out['status'], summary=out['summary'],
                          rows=[(x['label'], x.get('match', x.get('result')), x.get('delta')) for x in out['actors']]), indent=1))
    return status

def cmd_windows(a):
    calls = [json.loads(l) for l in open(os.path.join(a.dir, 'calls.jsonl')) if '"codex"' in l]
    quota = [json.loads(l) for l in open(os.path.join(a.dir, 'quota.jsonl')) if '"codex"' in l]
    weekly = sorted((q for q in quota if q.get('window_minutes') == 10080), key=lambda q: q['ts'])
    def win_of(ts):   # latest weekly reading at or before ts gives the window; null if none yet
        best = None
        for q in weekly:
            if q['ts'] <= ts: best = q
            else: break
        return round(best['resets_at'] / 300) * 300 if best else None
    groups = collections.defaultdict(lambda: collections.defaultdict(lambda: dict(calls=0, usd=0.0, weighted=0.0, unpriced=0)))
    for c in sorted(calls, key=lambda c: c['ts']):
        g = groups[win_of(c['ts'])][c['model']]; g['calls'] += 1; g['weighted'] += c['weighted']
        if c['usd'] is None: g['unpriced'] += 1
        else: g['usd'] += c['usd']
    for w, models in sorted(groups.items(), key=lambda kv: kv[0] or 0):
        us = [q['used_percent'] for q in weekly if round(q['resets_at'] / 300) * 300 == w]
        print(f"window resets_at={dt.datetime.fromtimestamp(w, dt.timezone.utc).isoformat() if w else None} "
              f"observed used% {min(us) if us else None}->{max(us) if us else None} (readings {len(us)})")
        for model, g in sorted(models.items(), key=lambda kv: -kv[1]['usd']):
            print(f"   {str(model):16s} calls={g['calls']:6d} modelled_usd={g['usd']:9.2f} weighted={g['weighted'] / 1e6:8.2f}M unpriced={g['unpriced']}")

def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sp = ap.add_subparsers(dest='cmd', required=True)
    b = sp.add_parser('build'); b.add_argument('--since'); b.add_argument('--until'); b.add_argument('--out', required=True)
    b.add_argument('--no-codex', action='store_true'); b.add_argument('--no-claude', action='store_true'); b.add_argument('--streams', nargs='*')
    r = sp.add_parser('reconcile-trial'); r.add_argument('measurement'); r.add_argument('--out')
    w = sp.add_parser('windows'); w.add_argument('dir')
    a = ap.parse_args()
    if a.cmd == 'build': cmd_build(a)
    elif a.cmd == 'reconcile-trial': sys.exit(cmd_reconcile(a))
    elif a.cmd == 'windows': cmd_windows(a)

if __name__ == '__main__':
    main()
