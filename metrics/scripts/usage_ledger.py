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

Measurement projection v1 (2026-11-01) adds explicit API-equivalent and
speed-adjusted API cost aliases, pricing-scenario identity, and per-call input
context length. Raw token categories, observed model/effort/tier, call identity
and timestamps remain on calls.jsonl. Codex `api_long_context_input` describes
the API price scenario, not a subscription surcharge. Tree/control snapshots
separate objective costs from account-wide meter observations and leave
uncalibrated objective quota estimates null. Existing numeric fields and budget
thresholds retain their meaning; cost bounds are not quota confidence bounds.

Objective time (2026-11-07). Budgets limit dollars and scope additions, not
time: wall-clock limits expired while work waited for a person. Tree and
control snapshots carry `time`, information only: the window's length and
model-busy hours, the union of the tree's model-call intervals from
`latency_s`. It leaves out tool execution and waits for input. Calls without
a latency are counted in `calls_without_latency`, and status is `partial`.
"""
import argparse, base64, binascii, collections, datetime as dt, glob, hashlib, io, json, math, os, re, sqlite3, subprocess, sys

CODEX_KEYS = ('input_tokens', 'cached_input_tokens', 'cache_write_input_tokens',
              'output_tokens', 'reasoning_output_tokens', 'total_tokens')
NATIVE_KEYS = ('input_tokens', 'cache_creation_input_tokens', 'cache_read_input_tokens', 'output_tokens')
TTL_KEYS = ('ephemeral_5m_input_tokens', 'ephemeral_1h_input_tokens')
CODEX_RATES = {'gpt-6.1-terra': (1, .05, 1.25, 5), 'gpt-6-sol': (2, .2, 2.5, 10),
               'gpt-6.1-sol': (2, .1, 2.5, 10), 'gpt-6-astra': (10, 1, 12.5, 50)}
NATIVE_RATES = {
    'claude-sonnet-5-5': dict(input=2, output=10, cache_write_5m=2.5, cache_write_1h=4, cache_read=.1),
    'claude-opus-5-5': dict(input=4, output=20, cache_write_5m=5, cache_write_1h=8, cache_read=.2)}
NATIVE_RATES.update({
    'claude-fable-5-1': dict(input=10, output=50, cache_write_5m=12.5, cache_write_1h=20, cache_read=.25),
    'claude-opus-5': dict(input=5, output=25, cache_write_5m=6.25, cache_write_1h=10, cache_read=.5),
    'claude-opus-4-8': dict(input=5, output=25, cache_write_5m=6.25, cache_write_1h=10, cache_read=.5),
    'claude-haiku-5-5': dict(input=.1, output=.5, cache_write_5m=.125, cache_write_1h=.2, cache_read=.01)})
HOME = os.path.expanduser('~')
CODEX_GLOBS = [f'{HOME}/.codex/sessions/**/rollout-*.jsonl', f'{HOME}/.codex/sessions/**/rollout-*.jsonl.zst',
               f'{HOME}/.codex/archived_sessions/*.jsonl', f'{HOME}/.codex/archived_sessions/*.jsonl.zst']
CLAUDE_GLOB = f'{HOME}/.claude/projects/**/*.jsonl'
STATE_DB = f'{HOME}/.codex/state_5.sqlite'
# F1.1: rollout line prefix, read without parsing large tool outputs.
PREFIX = re.compile(r'^\{"timestamp":"([^"]+)",(?:"ordinal":\d+,)?"type":"(\w+)","payload":\{"type":"(\w+)"')
INPUT_ITEMS = ('function_call_output', 'custom_tool_call_output')
QUOTA_MULTIPLIER = {'default': 1.0, 'priority': 2.5}
PRICING_RULES = dict(codex_long_input_threshold=272000, codex_long_input_multiplier=2,
                     codex_long_output_multiplier=1.5, unknown_tier_low=1, unknown_tier_high=2.5,
                     haiku_long_input_threshold=100000, haiku_long_multiplier=5)
PRICING_SCENARIO_ID = 'usage-ledger-standard-api-v1'
METER_MAX_AGE_SECONDS = 300
CLAUDE_LATENCY_START = {}   # message id -> its latency start, for later records of the same message
# Objectives no longer carry a time limit. Cursors written by earlier readers may
# still hold these trigger states; they are accepted and dropped, never raised.
RETIRED_TRIGGERS = {'hours_fraction', 'hours_reached'}

def seconds_between(start, end):
    try:
        if not start or not end: return None
        s = (ts_of(end) - ts_of(start)).total_seconds()
        return s if s >= 0 else None
    except (ValueError, TypeError, AttributeError, OverflowError): return None

ISSUES = []
SESSION_ID = re.compile(r'[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}')
def issue(kind, **kw): ISSUES.append(dict(kind=kind, **kw))
def ts_of(s): return dt.datetime.fromisoformat(s.replace('Z', '+00:00')) if s else None
def valid_timestamp(ts):
    try:
        t = ts_of(ts)
        return t is not None and t.utcoffset() is not None
    except (ValueError, TypeError, AttributeError, OverflowError): return False

def in_window(ts, since, until):
    # An undated record has unknown membership; retain it for coverage, not as zero.
    if not valid_timestamp(ts): return True
    t = ts_of(ts)
    return (since is None or t >= since) and (until is None or t <= until)

def valid_usage(u, keys, required=()):
    try:
        return isinstance(u, dict) and all(k in u for k in required) and all(
            k not in u or (isinstance(u[k], (int, float)) and not isinstance(u[k], bool)
                           and math.isfinite(u[k]) and u[k] >= 0) for k in keys)
    except (TypeError, OverflowError): return False


def pricing_scenario():
    """Identity of the frozen API scenario, separate from subscription tariffs."""
    rates = dict(codex=CODEX_RATES, claude=NATIVE_RATES)
    scenario = dict(rates=rates, rules=PRICING_RULES, speed_multipliers=QUOTA_MULTIPLIER)
    return dict(id=PRICING_SCENARIO_ID,
                rates_sha256=hashlib.sha256(json.dumps(rates, sort_keys=True, separators=(',', ':')).encode()).hexdigest(),
                scenario_sha256=hashlib.sha256(json.dumps(scenario, sort_keys=True, separators=(',', ':')).encode()).hexdigest(),
                kind='modelled_api_equivalent', actual_bill=False, subscription_quota=False,
                rules=PRICING_RULES, speed_multipliers=QUOTA_MULTIPLIER)


def cost_measurements(spend):
    """Derive aliases after aggregation; never add nullable data to spend counters."""
    return {pool: dict(
        api_equivalent_usd=dict(low=s['usd_low'], high=s['usd_high']),
        speed_adjusted_api_usd=dict(low=s['quota_usd_low'], high=s['quota_usd_high'], kind='proxy'),
        unpriced_calls=s['unpriced_calls'],
        estimated_quota_points=dict(status='not_established', low=None, high=None, calibration_version=None))
        for pool, s in spend.items()}


def account_quota_observation(quota, snapshot_at):
    """Last account reading and usable headroom, without attributing it to an objective.

    Freshness is age of the recorded observation, not a charge-settlement bound.
    Historical/undated/reset-unknown readings cannot establish current headroom.
    """
    weekly = [q for q in quota if q.get('provider') in (None, 'codex') and q.get('limit_id') in (None, 'codex')
              and q.get('window_minutes') == 10080
              and valid_timestamp(q.get('ts'))
              and valid_usage(q, ('used_percent',), ('used_percent',)) and q['used_percent'] <= 100]
    result = dict(status='not_established', pool='codex', account_wide=True, used_percent=None,
                  remaining_percent=None, observed_at=None, resets_at=None, age_seconds=None,
                  max_age_seconds=METER_MAX_AGE_SECONDS, reason='no_reading')
    if not weekly: return result
    q = max(weekly, key=lambda x: ts_of(x['ts']))
    age = (ts_of(snapshot_at) - ts_of(q['ts'])).total_seconds()
    reset = q.get('resets_at')
    reset_known = valid_usage({'reset': reset}, ('reset',), ('reset',))
    status, reason = 'observed', None
    if age < 0: status, reason = 'not_established', 'future_reading'
    elif reset_known and reset <= ts_of(snapshot_at).timestamp(): status, reason = 'expired', 'window_reset'
    elif not reset_known: status, reason = 'not_established', 'reset_unknown'
    elif age > METER_MAX_AGE_SECONDS: status, reason = 'stale', 'reading_age'
    result.update(status=status, reason=reason, used_percent=q['used_percent'],
                  last_reported_remaining_percent=100 - q['used_percent'],
                  remaining_percent=100 - q['used_percent'] if status == 'observed' else None,
                  observed_at=q['ts'], resets_at=reset, age_seconds=age,
                  limit_id=q.get('limit_id'), plan_type=q.get('plan_type'), source=q.get('source'))
    return result


def measurement_metadata():
    return dict(schema='usage-ledger-measurements-v1', pricing_scenario=pricing_scenario(),
                labels=dict(usd='API-equivalent dollars', quota_usd='API-equivalent dollars adjusted for subscription speed'),
                legacy_aliases=dict(usd='api_equivalent_usd', quota_usd='speed_adjusted_api_usd'),
                cost_bounds='scenario bounds for priced calls and unknown tier/cache details; not quota confidence intervals',
                quota_estimates='not calibrated; unknown is null, never zero')


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
    if c + w > i:
        issue('inconsistent_codex_cache', model=model, usage=u)
        return None
    ri, rc, rw, ro = CODEX_RATES[model]
    m, mo = (PRICING_RULES['codex_long_input_multiplier'], PRICING_RULES['codex_long_output_multiplier']) if i > PRICING_RULES['codex_long_input_threshold'] else (1, 1)
    return ((i - c - w) * ri + c * rc + w * rw) * m / 1e6 + o * ro * mo / 1e6

def native_usd(u, model):
    """(low, high, unknown_ttl_tokens); low == high when every cache write has a known TTL."""
    r = NATIVE_RATES.get(model)
    if not r or any(k not in u for k in NATIVE_KEYS): return None, None, None
    if model == 'claude-haiku-5-5' and sum(u[k] for k in NATIVE_KEYS if k != 'output_tokens') > PRICING_RULES['haiku_long_input_threshold']:
        r = {k: v * PRICING_RULES['haiku_long_multiplier'] for k, v in r.items()}
    w = u['cache_creation_input_tokens']; w5 = u.get(TTL_KEYS[0], 0); w1 = u.get(TTL_KEYS[1], 0)
    if w5 + w1 > w:
        issue('inconsistent_cache_split', model=model, usage=u)
        base = (u['input_tokens'] * r['input'] + u['output_tokens'] * r['output'] + u['cache_read_input_tokens'] * r['cache_read']) / 1e6
        return base + w * min(r['cache_write_5m'], r['cache_write_1h']) / 1e6, base + (w5 * r['cache_write_5m'] + w1 * r['cache_write_1h']) / 1e6, w
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
        d = None
        try:
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
                rid = p.get('response_id'); raw = p.get('usage')
                u = {k: raw.get(k, 0) for k in CODEX_KEYS} if isinstance(raw, dict) else raw
                valid = valid_usage(raw, CODEX_KEYS, ('input_tokens', 'output_tokens'))
                if not valid: issue('invalid_call_usage', path=path, line=n, usage=raw)
                if not valid_timestamp(ts): issue('invalid_call_timestamp', path=path, line=n, timestamp=ts)
                if valid and isinstance(p.get('thread_token_usage'), dict) and valid_usage(p['thread_token_usage'], CODEX_KEYS):
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
                                  tier=tier, latency_s=latency, latency_basis=latency_basis, invalid_usage=not valid))
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
        except (TypeError, ValueError, KeyError, AttributeError, OverflowError) as e:
            issue('malformed_record', path=path, line=n, error=str(e))
            if isinstance(d, dict) and d.get('type') == 'token_usage_record':
                payload = d.get('payload')
                if isinstance(payload, dict):
                    rid = payload.get('response_id')
                    if isinstance(rid, str) and rid and rid not in seen:
                        seen[rid] = payload.get('usage')
                        calls.append(dict(ts=d.get('timestamp'), response_id=rid, usage=payload.get('usage'), model=model,
                                          effort=effort, line=n, dedupe_basis='response_id', tier=tier,
                                          latency_s=None, latency_basis='malformed_record', invalid_usage=True))
    if n_tur == 0 and fallback:
        keyed = {}
        for c in fallback:
            k = (meta.get('id') or path, c['ts'], c['usage']['input_tokens'], c['usage']['output_tokens'])
            try:
                if k in seen: continue
                seen[k] = c['usage']; keyed[k] = c
            except TypeError:
                issue('invalid_fallback_usage', path=path, line=c['line']); continue
        calls = [dict(c, response_id=None, dedupe_basis='fallback') for c in keyed.values()]
    level1 = None
    if last_thread is not None:
        if not all(not c.get('invalid_usage') and valid_usage(c['usage'], CODEX_KEYS, CODEX_KEYS) for c in calls):
            issue('level1_unavailable', path=path); return meta, calls, quota, None
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

def validate_session_metadata(meta, path):
    for key in ('id', 'parent_thread_id', 'agent_path', 'agent_nickname'):
        if meta.get(key) is not None and not isinstance(meta[key], str):
            issue('invalid_session_metadata', path=path, field=key)
            meta[key] = None
    return meta

def build_codex(since, until, turns=None, metadata=None):
    idx = thread_index(); seen = {}; metas = {} if metadata is None else metadata; rows = []; quota = []; level1 = []
    files = codex_files(since)
    for f in files:
        file_turns = []
        meta, calls, q, l1 = codex_rollout(f, seen, file_turns)
        if turns is not None:
            turns += [t for t in file_turns if in_window(t['completed_ts'], since, until)]
        validate_session_metadata(meta, f)
        if meta.get('id'): metas[meta['id']] = meta
        quota += [x for x in q if in_window(x['ts'], since, until)]
        if l1: level1.append(l1)
        for c in calls:
            if not in_window(c['ts'], since, until): continue
            u = c['usage']; tid = meta.get('id')
            if not isinstance(c['model'], (str, type(None))): issue('invalid_model', path=f, line=c['line']); c['model'] = None
            if not isinstance(c.get('tier'), (str, type(None))): issue('invalid_tier', path=f, line=c['line']); c['tier'] = None
            try: price = codex_usd(u, c['model']) if not c.get('invalid_usage') and valid_usage(u, CODEX_KEYS, CODEX_KEYS) and valid_timestamp(c['ts']) else None
            except (ValueError, TypeError, ArithmeticError) as e:
                issue('invalid_call_usage', path=f, line=c['line'], error=str(e)); price = None
            if price is None: issue('unpriced_call', path=f, line=c['line'])
            rows.append(dict(provider='codex', ts=c['ts'], call_id=c['response_id'], dedupe_basis=c['dedupe_basis'],
                             thread=tid, parent_thread=meta.get('parent_thread_id'), agent_path=meta.get('agent_path'),
                             originator=meta.get('originator'), model=c['model'], effort=c['effort'],
                             role_index=(idx.get(tid) or {}).get('agent_role'), role_heuristic=role_heuristic(meta.get('agent_path')),
                             usage=u, weighted=codex_weighted(u) if not c.get('invalid_usage') and valid_usage(u, CODEX_KEYS, CODEX_KEYS) else None, usd=price, source=f'{f}:{c["line"]}', pools=['codex'],
                             service_tier=c.get('tier'), quota_multiplier=QUOTA_MULTIPLIER.get(c.get('tier')),
                             latency_s=c.get('latency_s'), latency_basis=c.get('latency_basis')))
            annotate_call_measurements(rows[-1])
    if metadata is not None:
        # Session metadata is needed for ancestry even when that file has no
        # calls in the window. Do not re-read current rollouts or charge calls.
        for path in sorted(set(codex_files(None)) - set(files)):
            for line in open_lines(path):
                if 'session_meta' not in line: continue
                try:
                    rec = json.loads(line); payload = rec.get('payload') or {}
                    if rec.get('type') == 'session_meta' and isinstance(payload, dict):
                        meta = validate_session_metadata({k: payload.get(k) for k in ('id', 'parent_thread_id', 'agent_path', 'agent_nickname', 'originator', 'thread_source', 'timestamp')}, path)
                        if meta.get('id'): metas.setdefault(meta['id'], meta)
                        break
                except (ValueError, TypeError, AttributeError):
                    issue('malformed_session_metadata', path=path)
    for r in rows:   # root ancestor by recorded parent links; unknown stays null
        cur, hops = r['thread'], 0
        while cur in metas and metas[cur].get('parent_thread_id') and hops < 20:
            cur = metas[cur]['parent_thread_id']; hops += 1
        r['root_thread'] = cur if (cur in metas or hops) else None; r['depth'] = hops
    return rows, quota, level1, len(files)

# ---------------------------------------------------------------- Claude
def observe_compaction(o, state, rows, path, since, until):
    if o.get('type') == 'assistant':
        message = o.get('message') or {}
        model = message.get('model') if isinstance(message, dict) else None
        if isinstance(model, str) and model != '<synthetic>': state['model'] = model
    if o.get('type') == 'system' and o.get('subtype') == 'compact_boundary':
        state['pending'] = None
        if in_window(o.get('timestamp'), since, until):
            metadata = o.get('compactMetadata') or {}
            tokens = metadata.get('preTokens') if isinstance(metadata, dict) else None
            if not valid_usage({'tokens':tokens}, ('tokens',), ('tokens',)):
                issue('invalid_compaction_tokens', path=path, value=tokens); tokens = None
            model = state.get('model'); rate = NATIVE_RATES.get(model)
            if model == 'claude-haiku-5-5' and tokens is not None and tokens > 100000:
                rate = {k:v*5 for k,v in rate.items()}
            pending = dict(session_id=o.get('sessionId'), kind='compaction', at=o.get('timestamp'), context_tokens=tokens,
                           model=model, source=path, usd_low=tokens * rate['cache_read'] / 1e6 if rate and tokens is not None else None,
                           usd_high=None)
            rows.append(pending); state['pending'] = pending
    elif state.get('pending') is not None and o.get('type') == 'user' and o.get('isCompactSummary'):
        pending = state['pending']; message = o.get('message') or {}
        content = message.get('content') if isinstance(message, dict) else None
        if isinstance(content, str): chars = len(content)
        elif isinstance(content, list) and all(isinstance(x, dict) and isinstance(x.get('text', ''), str) for x in content):
            chars = sum(len(x.get('text', '')) for x in content)
        else:
            issue('invalid_compaction_summary', path=path); state['pending'] = None; return
        rate = NATIVE_RATES.get(pending['model']); tokens = pending['context_tokens']
        if pending['model'] == 'claude-haiku-5-5' and tokens is not None and tokens > 100000:
            rate = {k:v*5 for k,v in rate.items()}
        if rate and tokens is not None:
            pending['usd_high'] = (tokens * rate['cache_write_1h'] + chars / 3 * rate['output']) / 1e6
        state['pending'] = None

def claude_transcript(path, msgs, cost_state, members=None, unrecorded=None, since=None, until=None):
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
    compact_state = {}
    for n, line in enumerate(open_lines(path), 1):
        o = None
        try:
            if unrecorded is not None and ('"assistant"' in line or '"compact_boundary"' in line or '"isCompactSummary"' in line):
                try: o = json.loads(line)
                except ValueError: o = None
                if isinstance(o, dict): observe_compaction(o, compact_state, unrecorded, path, since, until)
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
            if '"assistant"' not in line: continue
            if o is None:
                try: o = json.loads(line)
                except ValueError:
                    issue('malformed_line', path=path, line=n); continue
            if o.get('type') != 'assistant': continue
            m = o.get('message') or {}
            if not isinstance(m, dict): issue('malformed_line', path=path, line=n); continue
            u = m.get('usage'); mid = m.get('id'); model = m.get('model')
            if model == '<synthetic>' or not mid: continue
            if not isinstance(u, dict):
                issue('invalid_call_usage', path=path, line=n, usage=u); u = {'_invalid': u}
            if not valid_timestamp(o.get('timestamp')): issue('invalid_call_timestamp', path=path, line=n, timestamp=o.get('timestamp'))
            mine.add(mid)
            flat = {k: u[k] for k in NATIVE_KEYS if k in u}
            if '_invalid' in u: flat['_invalid'] = u['_invalid']
            split = u.get('cache_creation') or {}
            if not isinstance(split, dict): flat['_invalid_split'] = split; split = {}
            for k in TTL_KEYS:
                if k in split: flat[k] = split[k]
            if mid in msgs:
                row = msgs[mid]
                if not valid_timestamp(o.get('timestamp')): row['usage']['_invalid_timestamp'] = o.get('timestamp')
                if row.get('latency_s') is not None:   # F1.1: a message's records span the call; latency runs to the last one
                    later = seconds_between(CLAUDE_LATENCY_START.get(mid), o.get('timestamp'))
                    if later is not None and later > row['latency_s']: row['latency_s'] = later
                if not valid_usage(flat, NATIVE_KEYS + TTL_KEYS): row['usage']['_invalid_record'] = flat.copy()
                for k, v in flat.items():
                    if k == 'output_tokens' and valid_usage({k:v}, (k,)) and valid_usage(row['usage'], (k,)): row['usage'][k] = max(row['usage'].get(k, 0), v)
                    elif k == 'output_tokens': row['usage'][k] = v
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
        except (TypeError, ValueError, KeyError, AttributeError, OverflowError) as e:
            issue('malformed_record', path=path, line=n, error=str(e))
            if isinstance(o, dict) and o.get('type') == 'assistant':
                message = o.get('message')
                if isinstance(message, dict):
                    mid = message.get('id')
                    if isinstance(mid, str) and mid:
                        if mid in msgs:
                            msgs[mid]['usage']['_invalid_record'] = True
                        else:
                            mine.add(mid)
                            msgs[mid] = dict(provider='anthropic', ts=o.get('timestamp'), call_id=mid, dedupe_basis='message_id',
                                             session=o.get('sessionId'), project=project, subagent=sub, agent=agent,
                                             agent_type=agent_type, model=message.get('model'), effort=o.get('effort'),
                                             usage={'_invalid_record': message.get('usage')}, records=1, source=f'{path}:{n}',
                                             service_tier=None, speed=None, quota_multiplier=None, latency_s=None,
                                             latency_basis='malformed_record')

def build_claude(since, until, unrecorded=None):
    msgs = {}; cost_state = {}; members = {}
    files = sorted(f for f in glob.glob(CLAUDE_GLOB, recursive=True)
                   if since is None or os.path.getmtime(f) >= since.timestamp())
    for f in files: claude_transcript(f, msgs, cost_state, members, unrecorded, since, until)
    rows = []
    for r in msgs.values():
        if not in_window(r['ts'], since, until): continue
        if not isinstance(r['model'], (str, type(None))): issue('invalid_model', source=r['source']); r['model'] = None
        usage_ok = valid_usage(r['usage'], NATIVE_KEYS + TTL_KEYS, NATIVE_KEYS) and not any(k.startswith('_invalid') for k in r['usage'])
        if not usage_ok: issue('invalid_call_usage', source=r['source'], usage=r['usage'])
        if r.get('speed') == 'fast':
            issue('fast_mode_unpriced', source=r['source']); lo, hi, un = None, None, None
        else:
            try: lo, hi, un = native_usd(r['usage'], r['model']) if usage_ok and valid_timestamp(r['ts']) else (None, None, None)
            except (ValueError, TypeError, ArithmeticError) as e:
                issue('invalid_call_usage', source=r['source'], error=str(e)); lo, hi, un = None, None, None
        if lo is None: issue('unpriced_call', source=r['source'])
        r['pools'] = ['claude_all_models'] + (['claude_fable'] if (r['model'] or '').startswith('claude-fable-') else [])
        r.update(weighted=native_weighted(r['usage']) if usage_ok else None, usd=lo if lo == hi else None, usd_low=lo, usd_high=hi, unknown_ttl_tokens=un)
        annotate_call_measurements(r)
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
        if not isinstance(sid, str): issue('level1_unavailable', path=base); continue
        mids = set(members.get(base, set()))
        for f in glob.glob(os.path.join(os.path.dirname(base), sid, 'subagents', '*.jsonl')):
            mids |= members.get(f, set())
        per_model = collections.defaultdict(collections.Counter)
        invalid_models = set()
        for mid in mids:
            r = msgs[mid]
            if not isinstance(r['model'], (str, type(None))): issue('invalid_model', source=r['source']); r['model'] = None
            if not valid_usage(r['usage'], NATIVE_KEYS, NATIVE_KEYS):
                invalid_models.add(r['model']); continue
            for k in NATIVE_KEYS: per_model[r['model']][k] += r['usage'].get(k, 0)
        model_usage = cs.get('modelUsage') or {}
        if not isinstance(model_usage, dict): issue('level1_unavailable', path=base); continue
        for model, mu in model_usage.items():
            if model in invalid_models or not isinstance(mu, dict) or not valid_usage(mu, names):
                issue('level1_unavailable', path=base, model=model); continue
            prov = {names[k]: v for k, v in mu.items() if k in names}
            got = per_model.get(model, collections.Counter())
            delta = {k: got.get(k, 0) - prov[k] for k in prov}
            if all(v == 0 for v in delta.values()): cls = 'exact'
            elif not any(got.values()): cls = 'model_absent_from_transcript'
            elif all(v <= 0 for v in delta.values()): cls = 'counter_exceeds_transcript'
            elif all(v >= 0 for v in delta.values()): cls = 'transcript_exceeds_counter'
            else: cls = 'mixed'
            cli_cost = mu.get('costUSD')
            if cli_cost is not None and not valid_usage({'costUSD':cli_cost}, ('costUSD',)):
                issue('invalid_counter_cost', path=base, model=model); cli_cost = None
            level1.append(dict(session=sid, model=model, path=base, cli_cost_usd=cli_cost, provider_session_total=prov,
                               reconstructed={k: got.get(k, 0) for k in prov}, delta=delta, cls=cls))
    return rows, level1, len(files)

def unrecorded_claude(since, until):
    """Standalone compaction reader for callers that do not build the ledger."""
    rows = []
    for path in sorted(glob.glob(CLAUDE_GLOB, recursive=True)):
        if since and os.path.getmtime(path) < since.timestamp(): continue
        state = {}
        for line in open_lines(path):
            try:
                o = json.loads(line)
                if isinstance(o, dict): observe_compaction(o, state, rows, path, since, until)
            except (TypeError, ValueError, KeyError, AttributeError, OverflowError) as e:
                issue('malformed_compaction_record', path=path, error=str(e))
    return rows

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
                try:
                    info = o.get('rate_limit_info') or o
                    for wname, w in (info.get('unifiedWindows') or {}).items():
                        utilization = w.get('utilization')
                        if utilization is not None and not valid_usage({'utilization':utilization}, ('utilization',)):
                            issue('invalid_quota_reading', path=f, line=n); utilization = None
                        rows.append(dict(provider='anthropic', ts=o.get('timestamp') or last_ts, ts_basis='record' if o.get('timestamp') else 'preceding_record',
                                         window=wname, used_percent=None if utilization is None else 100 * utilization,
                                         resets_at=w.get('resetsAt'), status=w.get('status'), source=f'{f}:{n}'))
                except (TypeError, ValueError, AttributeError): issue('malformed_quota_record', path=f, line=n)
    return rows

# ---------------------------------------------------------------- commands
def cmd_build(a):
    since = ts_of(a.since) if a.since else None; until = ts_of(a.until) if a.until else None
    os.makedirs(a.out, exist_ok=True)
    calls, quota, l1, turns, unrecorded = [], [], {}, [], []
    cov = {}
    if not a.no_codex:
        r, q, l, nf = build_codex(since, until, turns); calls += r; quota += q; l1['codex_threads'] = l; cov['codex_files'] = nf
    if not a.no_claude:
        r, l, nf = build_claude(since, until, unrecorded=unrecorded); calls += r; l1['claude_sessions'] = l; cov['claude_files'] = nf
    if a.streams: quota += stream_quota(a.streams)
    calls.sort(key=lambda r: str(r['ts'] or ''))
    with open(os.path.join(a.out, 'calls.jsonl'), 'w') as fh:
        for r in calls: fh.write(json.dumps(r, sort_keys=True) + '\n')
    with open(os.path.join(a.out, 'quota.jsonl'), 'w') as fh:
        for r in quota: fh.write(json.dumps(r, sort_keys=True) + '\n')
    with open(os.path.join(a.out, 'unrecorded.jsonl'), 'w') as fh:
        for r in unrecorded:
            fh.write(json.dumps(r, sort_keys=True) + '\n')
    with open(os.path.join(a.out, 'issues.jsonl'), 'w') as fh:
        for r in ISSUES: fh.write(json.dumps(r, sort_keys=True) + '\n')
    json.dump(l1, open(os.path.join(a.out, 'level1.json'), 'w'), indent=1, sort_keys=True)
    turns.sort(key=lambda t: str(t['completed_ts'] or ''))
    with open(os.path.join(a.out, 'turns.jsonl'), 'w') as fh:
        for t in turns: fh.write(json.dumps(t, sort_keys=True) + '\n')
    agg = collections.defaultdict(lambda: dict(calls=0, weighted=0.0, usd=0.0, unpriced_calls=0))
    for r in calls:
        g = agg[f"{r['provider']}|{r['model']}"]; g['calls'] += 1
        if r['weighted'] is None: g['weighted'] = None
        elif g['weighted'] is not None: g['weighted'] += r['weighted']
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
                   f1_1=f11_summary(calls, turns), measurements=measurement_metadata())
    summary['measurements']['account_quota'] = account_quota_observation(quota, dt.datetime.now(dt.timezone.utc).isoformat())
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
    tturns = [t for t in turns if valid_usage(t, ('duration_ms',), ('duration_ms',))]
    return dict(key='provider|model|effort|tier (Codex service_tier; Claude speed)', by_cell=out,
                turns=len(turns), turn_duration_median_ms=med([t['duration_ms'] for t in tturns]),
                quota_note='quota_weighted_usd is API-equivalent dollars adjusted for subscription speed; a proxy, not measured quota')

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

POOLS = ('codex', 'claude_all_models', 'claude_fable')

def empty_spend():
    return dict(calls=0, unpriced_calls=0, usd_low=0.0, usd_high=0.0, quota_usd_low=0.0, quota_usd_high=0.0)

def pools():
    return {p: empty_spend() for p in POOLS}

def add_spend(dst, src):
    for k in dst: dst[k] += src[k]

def call_spend(c):
    lo = c.get('usd_low', c.get('usd')); hi = c.get('usd_high', c.get('usd'))
    mult = c.get('quota_multiplier') if c['provider'] == 'codex' else 1
    return dict(calls=1, unpriced_calls=int(lo is None or hi is None), usd_low=lo or 0, usd_high=hi or 0,
                quota_usd_low=(lo or 0) * (mult if mult is not None else PRICING_RULES['unknown_tier_low']),
                quota_usd_high=(hi or 0) * (mult if mult is not None else PRICING_RULES['unknown_tier_high']))


def annotate_call_measurements(row):
    spend = call_spend(row)
    priced = not spend['unpriced_calls']
    for prefix, legacy in (('api_equivalent_usd', 'usd'), ('speed_adjusted_api_usd', 'quota_usd')):
        for bound in ('low', 'high'):
            row[prefix + '_' + bound] = spend[legacy + '_' + bound] if priced else None
    row['pricing_scenario_id'] = PRICING_SCENARIO_ID
    u = row.get('usage') or {}
    context = None
    if row.get('weighted') is not None:
        context = u.get('input_tokens', 0)
        if row['provider'] != 'codex':
            context += u.get('cache_read_input_tokens', 0) + u.get('cache_creation_input_tokens', 0)
    row['input_context_tokens'] = context
    row['api_long_context_input'] = context > PRICING_RULES['codex_long_input_threshold'] if row['provider'] == 'codex' and context is not None else None


def finish_measurements(result, quota):
    result['measurements'] = measurement_metadata()
    result['measurements'].update(objective=cost_measurements(result['totals']),
                                  account_quota=account_quota_observation(quota, result['snapshot_at']))
    for n in result['nodes']:
        n['measurements'] = dict(direct=cost_measurements(n['direct']), inclusive=cost_measurements(n['inclusive']))
    return result

def digest(path):
    with open(path, 'rb') as f: return hashlib.sha256(f.read()).hexdigest()

def transcript_available(provider, sid, metadata, node_id=None):
    if provider == 'codex': return sid in metadata
    if provider == 'claude':
        for path in find_transcripts(sid):
            if node_id is not None and node_id != sid:
                if '/subagents/' not in path or node_id != sid + '/agent:' + os.path.basename(path)[:-6]: continue
            elif '/subagents/' in path: continue
            try:
                with open(path, 'rb') as fh: fh.read(1)
                return True
            except OSError: continue
    return False

def model_time(intervals, calls, missing, since, until, status):
    """Model-busy time of an objective tree: information, never a limit.

    Each interval runs from a call's latest preceding input to its record
    (`latency_s`), so it leaves out tool execution and time spent waiting for a
    person, and includes provider queueing. Parallel calls in different sessions
    count once in model_busy_hours and separately in model_call_hours. A call
    without a latency is counted, never taken as zero time.
    """
    clipped = sorted((max(a, since), min(b, until)) for a, b in intervals if min(b, until) > max(a, since))
    busy, total, cur = 0.0, 0.0, None
    for a, b in clipped:
        total += (b - a).total_seconds()
        if cur and a <= cur[1]: cur[1] = max(cur[1], b)
        else:
            if cur: busy += (cur[1] - cur[0]).total_seconds()
            cur = [a, b]
    if cur: busy += (cur[1] - cur[0]).total_seconds()
    known = status != 'not_established'
    return dict(basis='model_call_latency', status=status,
                window_hours=round((until - since).total_seconds() / 3600, 4),
                model_busy_hours=round(busy / 3600, 4) if known else None,
                model_call_hours=round(total / 3600, 4) if known else None,
                calls=calls, calls_without_latency=missing)

def cost_tree(record_path, since_arg=None, until_arg=None, role_ledger=None):
    with open(record_path) as fh: sb = json.load(fh).get('scope_budget', {})
    now = dt.datetime.now(dt.timezone.utc).isoformat()
    since_s = since_arg or sb.get('started_at'); until_s = until_arg or sb.get('closed_at') or now
    since, until = ts_of(since_s), ts_of(until_s)
    # Cost-window closure is not an account-observation cutoff or evidence of
    # settlement. Read through the snapshot, then keep the requested totals
    # separate from later bound-session candidates.
    observation_until = max(until, ts_of(now))
    result = dict(schema='objective-cost-tree-v1', snapshot_at=now, window=dict(since=since_s, until=until_s, closed_at=sb.get('closed_at')),
                  record=dict(path=os.path.abspath(record_path), sha256=digest(record_path), objective_id=sb.get('objective_id')),
                  reader=dict(path=os.path.abspath(__file__), sha256=digest(__file__)),
                  pricing=dict(source='https://platform.claude.com/docs/en/about-claude/pricing', fetched='2026-10-30'),
                  status='not_established', totals=pools(), window_totals=pools(), nodes=[],
                  unattributed=dict(totals=pools(), sessions=[]), unrecorded=[], issues=[],
                  undated=dict(counts={p: 0 for p in POOLS}, sessions=[]),
                  outside=dict(issues={}, undated_calls=0),
                  conservation_scope='dated calls in window',
                  known_gaps=['Claude Code title and other side calls are not in transcripts'])
    if sb.get('closed_at'):
        result['post_close'] = dict(
            closed_at=sb['closed_at'], observed_until=now,
            excluded_after=max(ts_of(sb['closed_at']), until).isoformat(),
            coverage='not_established', candidate_totals=None, sessions=[],
            settlement=dict(status='not_established',
                reason='Closure does not establish that all attributable calls have been reported. '
                       'Later bound-session calls are candidates, not automatically attributable: '
                       'a reused session may perform another objective.'))
    ISSUES.clear()
    try:
        metadata = {}
        codex, quota, _, _ = build_codex(since, observation_until, metadata=metadata)
        unrecorded = []
        claude, _, _ = build_claude(since, observation_until, unrecorded=unrecorded)
    except (OSError, ValueError, TypeError, KeyError, subprocess.SubprocessError) as e:
        result['issues'].append(dict(kind='build_failed', error=str(e)))
        return finish_measurements(result, []), [], sb
    observed_calls = codex + claude
    calls = [c for c in observed_calls if in_window(c.get('ts'), since, until)]
    nodes = {}; starts = {}; links = []
    def add_node(node_id, sid, provider, parent, link, role=None, start=None):
        # Reject links whose parent chain would make this node its own ancestor.
        cur = parent; visited = set()
        while cur is not None and cur in nodes and cur not in visited:
            if cur == node_id:
                result['issues'].append(dict(kind='lineage_cycle', node_id=node_id, parent_node_id=parent, link=link))
                return False
            visited.add(cur); cur = nodes[cur]['parent_node_id']
        if node_id in nodes:
            old = nodes[node_id]
            if (old['parent_node_id'], old['link']) != (parent, link):
                priority = {'root': 0, 'successor': 1, 'binding': 2, 'codex_parent': 3, 'role_host': 4, 'claude_subagent': 5}
                replace = priority[link] < priority[old['link']]
                result['issues'].append(dict(kind='multiple_parents', node_id=node_id, retained=link if replace else old['link'], rejected=old['link'] if replace else link, parent_node_id=parent))
                if replace:
                    old.update(parent_node_id=parent, link=link, role_name=role or old['role_name'])
                    starts[node_id] = start
                    return True
            return False
        nodes[node_id] = dict(node_id=node_id, session_id=sid, provider=provider, parent_node_id=parent, link=link,
                              role_name=role, models=[], tiers=[], first_call=None, last_call=None, direct=pools(), inclusive=pools())
        starts[node_id] = start
        return True
    sessions = sb.get('sessions') or []
    for i, root in enumerate(sessions):
        add_node(root['session_id'], root['session_id'], root['provider'], None if i == 0 else sessions[0]['session_id'],
                 'root' if i == 0 else 'successor', start=root.get('from'))
    missing_root = False; checked_sessions = set()
    for i, bound in enumerate(sessions + (sb.get('bindings') or [])):
        sid = bound.get('session_id'); provider = bound.get('provider')
        checked_sessions.add((sid, provider))
        if not transcript_available(provider, sid, metadata):
            result['issues'].append(dict(kind='transcript_missing', session_id=sid, provider=provider))
            if i == 0 and sessions: missing_root = True
    for b in sb.get('bindings') or []:
        links.append((0, b['session_id'], b['session_id'], b.get('provider'), b.get('parent_session_id') or (sessions[0]['session_id'] if sessions else None), 'binding', None))
    for sid, meta in metadata.items():
        if meta.get('parent_thread_id'):
            links.append((1, sid, sid, 'codex', meta['parent_thread_id'], 'codex_parent', meta.get('agent_nickname')))
    ledger = role_ledger or os.path.join(os.environ.get('XDG_STATE_HOME', os.path.expanduser('~/.local/state')), 'work-engine/role-host/ledger.jsonl')
    try:
        with open(ledger) as fh:
            for number, line in enumerate(fh, 1):
                try: row = json.loads(line)
                except ValueError: row = None
                if not isinstance(row, dict):
                    result['issues'].append(dict(kind='malformed_role_host_row', line=number)); continue
                if row.get('event') == 'launch' and row.get('session_id'):
                    sid = row['session_id']
                    if sid in nodes and row.get('name'): nodes[sid]['role_name'] = row['name']
                    if row.get('parent_session_id'):
                        links.append((2, sid, sid, row.get('host'), row['parent_session_id'], 'role_host', row.get('name')))
    except FileNotFoundError:
        result['issues'].append(dict(kind='role_host_ledger_absent', path=ledger))
    except OSError as e:
        result['issues'].append(dict(kind='role_host_ledger_unreadable', path=ledger, error=str(e)))
    for c in claude:
        if c.get('subagent'):
            sid = c['session']; nid = sid + '/agent:' + c['agent']
            links.append((3, nid, sid, 'claude', sid, 'claude_subagent', c.get('agent_type')))
    links = sorted(set(links), key=lambda x: (x[0], str(x[1]), str(x[4])))
    remaining = links
    while remaining:
        later = []; changed = False
        for _, nid, sid, provider, parent, link, role in remaining:
            if parent not in nodes:
                later.append((_, nid, sid, provider, parent, link, role)); continue
            if add_node(nid, sid, provider, parent, link, role, starts[parent]): changed = True
        if not changed: break
        remaining = links if changed else later
    # Explicit bindings are checked above even if unreachable; every joined
    # member must also have its own readable transcript, including subagents.
    for node in nodes.values():
        sid = node['session_id']; provider = node['provider']; nid = node['node_id']
        if nid == sid and (sid, provider) in checked_sessions: continue
        if not transcript_available(provider, sid, metadata, nid):
            result['issues'].append(dict(kind='transcript_missing', session_id=sid, provider=provider, node_id=nid))
    # The winning graph is now fixed. Derive descendant boundaries from it,
    # rather than retaining the boundary of an earlier, displaced parent.
    def inherited_start(nid):
        node = nodes[nid]
        if node['link'] in ('root', 'successor'): return starts[nid]
        parent = node['parent_node_id']
        return inherited_start(parent) if parent in nodes else None
    for nid in nodes:
        starts[nid] = inherited_start(nid)
    if 'post_close' in result:
        tail = result['post_close']; tail['candidate_totals'] = pools()
        tail_sessions = {}; after = ts_of(tail['excluded_after'])
        for c in observed_calls:
            if not valid_timestamp(c.get('ts')) or not (after < ts_of(c['ts']) <= ts_of(now)):
                continue
            sid = c.get('thread') if c['provider'] == 'codex' else c.get('session')
            nid = sid + '/agent:' + c['agent'] if c.get('subagent') and sid else sid
            if nid not in nodes or (starts[nid] and ts_of(c['ts']) < ts_of(starts[nid])):
                continue
            spend = call_spend(c)
            row = tail_sessions.setdefault(nid, dict(node_id=nid, session_id=sid,
                provider=c['provider'], totals=pools(), first_call=c['ts'], last_call=c['ts']))
            row['first_call'] = min(row['first_call'], c['ts'], key=ts_of)
            row['last_call'] = max(row['last_call'], c['ts'], key=ts_of)
            for p in c['pools']:
                add_spend(tail['candidate_totals'][p], spend); add_spend(row['totals'][p], spend)
        tail['sessions'] = list(tail_sessions.values())
        tail['unrecorded'] = [x for x in unrecorded if x['session_id'] in nodes and
                              valid_timestamp(x.get('at')) and after < ts_of(x['at']) <= ts_of(now) and
                              (not starts[x['session_id']] or ts_of(x['at']) >= ts_of(starts[x['session_id']]))]
        tail['measurements'] = cost_measurements(tail['candidate_totals'])
    unattributed = {}; undated_sessions = set(); undated_outside = 0
    busy = []; busy_calls = 0; busy_missing = 0
    # Issues and undated calls count against the objective only when they come from
    # one of its sessions; the window holds other objectives' sessions too.
    members = {n['session_id'] for n in nodes.values()}
    for c in calls:
        sid = c.get('thread') if c['provider'] == 'codex' else c.get('session')
        nid = sid + '/agent:' + c['agent'] if c.get('subagent') and sid else sid
        if not valid_timestamp(c.get('ts')):
            if sid not in members:
                undated_outside += 1
                continue
            for p in c['pools']: result['undated']['counts'][p] += 1
            undated_sessions.add((sid, c['provider']))
            continue
        spend = call_spend(c)
        for p in c['pools']: add_spend(result['window_totals'][p], spend)
        n = nodes.get(nid)
        attributable = n is not None and (not starts[nid] or in_window(c['ts'], ts_of(starts[nid]), None))
        if not attributable:
            key = (sid, c['provider']); row = unattributed.setdefault(key, dict(session_id=sid, provider=c['provider'], calls=0, usd_low=0, usd_high=0))
            row['calls'] += 1; row['usd_low'] += spend['usd_low']; row['usd_high'] += spend['usd_high']
            for p in c['pools']: add_spend(result['unattributed']['totals'][p], spend)
            continue
        for p in c['pools']: add_spend(n['direct'][p], spend); add_spend(result['totals'][p], spend)
        latency = c.get('latency_s'); busy_calls += 1
        if isinstance(latency, (int, float)) and not isinstance(latency, bool) and math.isfinite(latency) and latency >= 0:
            end = ts_of(c['ts']); busy.append((end - dt.timedelta(seconds=latency), end))
        else: busy_missing += 1
        if c.get('model') not in n['models']: n['models'].append(c.get('model'))
        tier = c.get('service_tier') if c['provider'] == 'codex' else c.get('speed')
        if tier not in n['tiers']: n['tiers'].append(tier)
        if valid_timestamp(c['ts']) and (n['first_call'] is None or c['ts'] < n['first_call']): n['first_call'] = c['ts']
        if valid_timestamp(c['ts']) and (n['last_call'] is None or c['ts'] > n['last_call']): n['last_call'] = c['ts']
    for n in nodes.values():
        cur = n; visited = set()
        while cur and cur['node_id'] not in visited:
            visited.add(cur['node_id'])
            for p in POOLS: add_spend(cur['inclusive'][p], n['direct'][p])
            cur = nodes.get(cur['parent_node_id'])
    result['nodes'] = list(nodes.values()); result['unattributed']['sessions'] = list(unattributed.values())
    result['undated']['sessions'] = [dict(session_id=sid, provider=provider) for sid, provider in sorted(undated_sessions, key=str)]
    result['unrecorded'] = [x for x in unrecorded if x['session_id'] in nodes and in_window(x.get('at'), since, until) and
                            (not starts[x['session_id']] or in_window(x['at'], ts_of(starts[x['session_id']]), None))]
    file_sessions = collections.defaultdict(set)
    for c in calls:
        src = c.get('source'); sid = c.get('thread') if c['provider'] == 'codex' else c.get('session')
        if isinstance(src, str) and isinstance(sid, str): file_sessions[src.rsplit(':', 1)[0]].add(sid)
    def in_scope(i):
        # An issue is outside only when everything known about its file points to
        # other sessions. A file with any member session (copied history can put
        # several sessions in one transcript), a path naming a member, or no file or
        # session at all keeps the issue in scope (invariant 2).
        if isinstance(i.get('path'), str): path = i['path']
        elif isinstance(i.get('source'), str): path = i['source'].rsplit(':', 1)[0]
        else: return True
        known = file_sessions.get(path, set()) | set(SESSION_ID.findall(path))
        return not known or bool(known & members)
    outside = collections.Counter()
    for i in list({json.dumps(x, sort_keys=True): x for x in ISSUES}.values()):
        if in_scope(i): result['issues'].append(i)
        else: outside[i['kind']] += 1
    result['issues'] = list({json.dumps(x, sort_keys=True): x for x in result['issues']}.values())
    result['outside'] = dict(issues=dict(sorted(outside.items())), undated_calls=undated_outside)
    ranged = any(s['usd_low'] != s['usd_high'] or s['quota_usd_low'] != s['quota_usd_high'] or s['unpriced_calls'] for s in result['totals'].values())
    substantive_issues = [i for i in result['issues'] if i['kind'] != 'role_host_ledger_absent']
    result['status'] = 'not_established' if not sessions or missing_root else ('partial' if ranged or result['unrecorded'] or substantive_issues or undated_sessions else 'complete')
    # Pricing gaps do not affect timing; missing transcripts, unrecorded or undated calls do.
    lineage_gap = any(i['kind'] == 'transcript_missing' for i in result['issues'])
    time_status = ('not_established' if result['status'] == 'not_established' else
                   'partial' if busy_missing or lineage_gap or result['unrecorded'] or undated_sessions else 'complete')
    result['time'] = model_time(busy, busy_calls, busy_missing, since, until, time_status)
    if 'post_close' in result:
        tail = result['post_close']
        tail_ranged = any(s['usd_low'] != s['usd_high'] or s['quota_usd_low'] != s['quota_usd_high']
                          or s['unpriced_calls'] for s in tail['candidate_totals'].values())
        tail['coverage'] = ('not_established' if not sessions or missing_root else
            'partial' if tail_ranged or tail['unrecorded'] or substantive_issues or undated_sessions else 'complete')
        result['coverage_scope'] = 'requested_cost_window; post_close coverage and settlement are separate'
    return finish_measurements(result, quota), quota, sb

def cmd_tree(a):
    doc, _, _ = cost_tree(a.record, a.since, a.until, a.role_host_ledger)
    out = json.dumps(doc, indent=1, sort_keys=True)
    if a.out:
        with open(a.out, 'w') as fh: fh.write(out + '\n')
    print(out)

def cmd_control(a):
    doc, quota, sb = cost_tree(a.record, role_ledger=a.role_host_ledger)
    objective = sb.get('objective_id'); topic = 'control:budget:' + str(objective)
    previous = {}; previous_account_status = 'observed'
    if a.cursor:
        try:
            raw = base64.b64decode(a.cursor + '=' * (-len(a.cursor) % 4), altchars=b'-_', validate=True)
            cursor = json.loads(raw)
            if not isinstance(cursor, dict) or not isinstance(cursor.get('states'), dict): raise ValueError('invalid cursor shape')
            if cursor.get('objective_id') != objective: raise ValueError('cursor belongs to another objective')
            previous = cursor['states']
            previous_account_status = cursor.get('account_quota_status', 'observed')
            if previous_account_status not in ('observed', 'stale', 'expired', 'not_established'):
                raise ValueError('invalid account quota status')
            for key, state in previous.items():
                parts = json.loads(key)
                kinds = {'mark_crossed', 'node_mark_crossed', 'limit_fraction', 'limit_reached',
                         'coverage_changed', 'account_ceiling', 'objective_closed'} | RETIRED_TRIGGERS
                if not isinstance(parts, list) or len(parts) not in (4, 5) or parts[0] not in kinds:
                    raise ValueError('invalid trigger key')
                if parts[1] is not None and parts[1] not in POOLS: raise ValueError('invalid trigger pool')
                if parts[2] is not None and not isinstance(parts[2], str): raise ValueError('invalid trigger node')
                if len(parts) == 5 and parts[4] not in ('user', 'supervisor'): raise ValueError('invalid trigger setter')
                if parts[0] == 'objective_closed':
                    if parts != ['objective_closed', None, None, 'closed'] or not isinstance(state, bool):
                        raise ValueError('invalid objective closed state')
                elif parts[0] == 'coverage_changed':
                    if parts[3] != 'coverage' or not isinstance(state, dict) or state.get('status') not in ('complete', 'partial', 'not_established') or not isinstance(state.get('unpriced_calls'), int) or isinstance(state.get('unpriced_calls'), bool) or state['unpriced_calls'] < 0 or not isinstance(state.get('ranged'), bool):
                        raise ValueError('invalid coverage state')
                    if 'undated_calls' in state and (not isinstance(state['undated_calls'], int) or isinstance(state['undated_calls'], bool) or state['undated_calls'] < 0): raise ValueError('invalid undated state')
                    if 'unknown_spend' in state and not isinstance(state['unknown_spend'], bool): raise ValueError('invalid unknown state')
                elif not isinstance(state, bool) or not valid_usage({'level': parts[3]}, ('level',), ('level',)):
                    raise ValueError('invalid trigger state')
        except (ValueError, KeyError, TypeError, UnicodeError, binascii.Error) as e:
            raise ValueError('invalid cursor: ' + str(e))
    states = {k: v for k, v in previous.items() if json.loads(k)[0] not in RETIRED_TRIGGERS}
    events = []; evaluated = set(); cost_controls = []
    def trigger(kind, pool, node, level, active, observed, summary, set_by=None):
        parts = [kind, pool, node, level]
        if set_by is not None: parts.append(set_by)
        key = json.dumps(parts, separators=(',', ':'))
        if active is None or key in evaluated: return
        evaluated.add(key)
        states[key] = active
        if active != previous.get(key, False):
            event = dict(topic=topic, authority='observation_only', kind=kind, pool=pool,
                         node_id=node, level=level, observed=observed, snapshot_at=doc['snapshot_at'], summary=summary)
            if set_by is not None: event['set_by'] = set_by
            events.append(event)
    limits = sb.get('limits') or {}
    account = account_quota_observation(quota, doc['snapshot_at'])
    account['ceiling_percent'] = limits.get('max_weekly_percent')
    account['headroom_to_ceiling_percent'] = (account['ceiling_percent'] - account['used_percent']
        if account['ceiling_percent'] is not None and account['status'] == 'observed' else None)
    notification_status = previous_account_status
    closed_at = sb.get('closed_at')
    if closed_at is not None:
        trigger('objective_closed', None, None, 'closed', True,
                dict(closed_at=closed_at, disposition=sb.get('close', {}).get('disposition'), totals=doc['totals']),
                'objective closed')
    else:
        notification_status = 'observed' if account['status'] == 'stale' else account['status']
        fractions = [float(x) for x in a.limit_fractions.split(',')]
        nodes = {n['node_id']: n for n in doc['nodes']}
        for limit in sb.get('cost_limits') or []:
            pool = limit['pool']; node = limit.get('node')
            if pool not in POOLS: continue
            if doc['status'] == 'not_established': continue
            if node and node not in nodes:
                doc['status'] = 'partial'
                continue
            spend = nodes[node]['inclusive'][pool] if node else doc['totals'][pool]
            value = spend['quota_usd_high']
            observed = dict(quota_usd_low=spend['quota_usd_low'], quota_usd_high=value, unpriced_calls=spend['unpriced_calls'],
                            cost_basis='speed_adjusted_api_usd', kind='proxy',
                            api_equivalent_usd_low=spend['usd_low'], api_equivalent_usd_high=spend['usd_high'])
            setter = limit.get('set_by', 'supervisor')
            if setter == 'coordinator': setter = 'supervisor'
            maximum = limit.get('max_usd')
            cost_controls.append(dict(pool=pool, node_id=node, set_by=setter, max_usd=maximum,
                                      basis='speed_adjusted_api_usd', observed=observed,
                                      remaining_usd=(maximum - value) if maximum is not None and doc['status'] == 'complete' and not spend['unpriced_calls'] else None))
            def spend_trigger(kind, level, summary):
                active = True if value >= level else (None if spend['unpriced_calls'] else False)
                trigger(kind, pool, node, level, active, observed, summary, setter)
            mark_kind = 'node_mark_crossed' if node else 'mark_crossed'
            for mark in limit.get('marks_usd') or []:
                spend_trigger(mark_kind, mark, 'spend mark')
            if maximum is not None:
                if setter == 'user':
                    for f in sorted(set(fractions + [1.0])):
                        spend_trigger('limit_reached' if f == 1 else 'limit_fraction', maximum * f, 'spend limit')
                else:
                    spend_trigger(mark_kind, maximum, 'spend mark')
        if account['status'] in ('observed', 'stale') and limits.get('max_weekly_percent') is not None:
            trigger('account_ceiling', None, None, limits['max_weekly_percent'], account['used_percent'] >= limits['max_weekly_percent'],
                    dict(used_percent=account['used_percent'], account_wide=True, account_quota=account), 'last recorded account-wide weekly meter')
        # Routine idle/resume freshness changes remain in the snapshot and do not
        # wake the supervisor. Window loss/expiry and recovery remain observable.
        previous_account_status = 'observed' if previous_account_status == 'stale' else previous_account_status
        account_changed = limits.get('max_weekly_percent') is not None and notification_status != previous_account_status
        coverage = dict(status=doc['status'], unpriced_calls=sum(s['unpriced_calls'] for s in doc['totals'].values()),
                        ranged=any(s['quota_usd_low'] != s['quota_usd_high'] for s in doc['totals'].values()),
                        unknown_spend=any(s['unpriced_calls'] for s in doc['totals'].values()),
                        undated_calls=sum(doc.get('undated', {}).get('counts', {}).values()))
        key = json.dumps(['coverage_changed', None, None, 'coverage'], separators=(',', ':'))
        states[key] = coverage
        default_coverage = dict(status='complete', unpriced_calls=0, ranged=False, unknown_spend=False, undated_calls=0)
        cost_coverage_changed = coverage != previous.get(key, default_coverage)
        if cost_coverage_changed or account_changed:
            # The cursor's cost-coverage state retains its original shape. Account
            # freshness travels in an additive observation, coalesced with any cost
            # coverage change rather than producing duplicate notifications.
            observed_coverage = dict(coverage)
            if account_changed: observed_coverage['account_quota'] = account
            events.append(dict(topic=topic, authority='observation_only', kind='coverage_changed', pool=None, node_id=None,
                               level='coverage', observed=observed_coverage, snapshot_at=doc['snapshot_at'],
                               cost_coverage_changed=cost_coverage_changed,
                               summary='coverage changed' if cost_coverage_changed else 'account quota observation changed'))
    cursor = base64.urlsafe_b64encode(json.dumps(dict(objective_id=objective, states=states,
        account_quota_status=notification_status), sort_keys=True).encode()).decode().rstrip('=')
    measurements = dict(doc.get('measurements') or measurement_metadata())
    measurements.update(objective=cost_measurements(doc['totals']), account_quota=account)
    print(json.dumps(dict(schema='objective-budget-control-v1', topic=topic, events=events, next_budget_cursor=cursor,
                          closed_at=closed_at, snapshot=dict(snapshot_at=doc['snapshot_at'], status=doc['status'], totals=doc['totals'],
                                        measurements=measurements, cost_controls=cost_controls,
                                        time=doc.get('time'), post_close=doc.get('post_close'))), sort_keys=True))

def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sp = ap.add_subparsers(dest='cmd', required=True)
    b = sp.add_parser('build'); b.add_argument('--since'); b.add_argument('--until'); b.add_argument('--out', required=True)
    b.add_argument('--no-codex', action='store_true'); b.add_argument('--no-claude', action='store_true'); b.add_argument('--streams', nargs='*')
    r = sp.add_parser('reconcile-trial'); r.add_argument('measurement'); r.add_argument('--out')
    w = sp.add_parser('windows'); w.add_argument('dir')
    t = sp.add_parser('tree'); t.add_argument('--record', required=True); t.add_argument('--since'); t.add_argument('--until'); t.add_argument('--role-host-ledger'); t.add_argument('--out')
    c = sp.add_parser('control'); c.add_argument('--record', required=True); c.add_argument('--cursor'); c.add_argument('--limit-fractions', default='0.5,0.8,1.0'); c.add_argument('--role-host-ledger')
    a = ap.parse_args()
    if a.cmd == 'control':
        try: cmd_control(a)
        except ValueError as e: ap.error(str(e))
    elif a.cmd == 'tree': cmd_tree(a)
    elif a.cmd == 'build': cmd_build(a)
    elif a.cmd == 'reconcile-trial': sys.exit(cmd_reconcile(a))
    elif a.cmd == 'windows': cmd_windows(a)

if __name__ == '__main__':
    main()
