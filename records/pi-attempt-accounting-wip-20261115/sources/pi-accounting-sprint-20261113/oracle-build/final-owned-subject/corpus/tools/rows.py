"""Producer-row and expectation constructors for the P1.28 corpus builder. Rows mirror the release producer's writers
(pi_container.py 92a03789; cited per constructor). Expectations are literal values supplied by each case; nothing here
prices, validates or decides anything."""
import json
from decimal import Decimal

S1, S2, S3 = ('00000000-0000-4000-8000-0000000000c1', '00000000-0000-4000-8000-0000000000c2',
              '00000000-0000-4000-8000-0000000000c3')
ASTRA, TERRA, SOL = 'gpt-6-astra', 'gpt-6.1-terra', 'gpt-6.1-sol'
SONNET, FABLE, FABLE51, HAIKU = 'claude-sonnet-5', 'claude-fable-5-2', 'claude-fable-5-1', 'claude-haiku-5-5'
RATES = {'openai_api_rates.json': 'c0bdfd66ee69f1644eaf188f87f6af5e47beacc0739d20aac5846c923d66fd58',
         'anthropic_api_rates.json': '8a286433d9e08b37533f2f908f2db60c4357e1a566018c9ca8364937652bdef5'}


def nano(usd):
    """Exact USD decimal string -> nano-USD integer string."""
    v = Decimal(str(usd)) * 10 ** 9
    assert v == v.to_integral_value(), usd
    return str(int(v))


def t(hms, day='2026-11-10'):
    return f'{day}T{hms}Z'


def line(row):
    """One producer line: json.dumps with the default separators (the proxy's own `json.dumps({...})`)."""
    return row if isinstance(row, str) else json.dumps(row)


def oa_usage(i=1000, o=100, cached=0, cw=0, reasoning=0):
    return dict(input_tokens=i, output_tokens=o, cached_tokens=cached, cache_write_tokens=cw, reasoning_tokens=reasoning)


def an_usage(i=0, read=0, cw=0, o=0, w5=None, w1=None):
    u = dict(input_tokens=i, cache_read_input_tokens=read, cache_creation_input_tokens=cw, output_tokens=o)
    if w5 is not None: u['ephemeral_5m_input_tokens'] = w5
    if w1 is not None: u['ephemeral_1h_input_tokens'] = w1
    return u


# ---- producer rows (pi_container.py) ---------------------------------------------------------------------------
def reserve(n, rid, usd, at, bound=6000):
    """usd_reserved (:1186, :1446)."""
    return {'at': t(at), 'event': 'usd_reserved', 'n': n, 'rid': rid, 'reserved_usd': usd, 'input_bound': bound,
            'bound_basis': 'bytes'}


def oa_request(n, at, usage=None, model=ASTRA, response_model=ASTRA, tier='default', resp='resp_1', status=200,
               terminal='response.completed', usd=None, **extra):
    """OpenAI request row (finish(...), :1554-1571); `usd` only on budgeted sessions."""
    row = {'at': t(at), 'event': 'request', 'n': n, 'http_status': status, 'model': model, 'requested_tier': None,
           'observed_tier': tier, 'terminal': terminal, 'response_id': resp, 'response_model': response_model,
           'usage': usage if usage is not None else oa_usage(), 'seconds': 1.0}
    if usd is not None: row['usd'] = usd
    row.update(extra)
    return row


def settle(rid, reserved, charged, basis):
    return {'rid': rid, 'reserved_usd': reserved, 'charged_usd': charged, 'charge_basis': basis,
            'session_spent_usd': charged, 'multiplier_floor': 0.0}


def an_request(n, at, usage, model=FABLE, response_model=FABLE, tier='standard', resp='msg_1', status=200,
               terminal='message_stop', check='verified', usd=None, **extra):
    """Anthropic request row (forward_anthropic, :1307-1321)."""
    row = {'at': t(at), 'event': 'request', 'provider': 'anthropic', 'n': n, 'http_status': status, 'model': model,
           'requested_tier': None, 'observed_tier': tier, 'terminal': terminal, 'response_id': resp,
           'response_model': response_model, 'response_model_check': check, 'usage': usage, 'seconds': 1.0}
    if response_model and response_model != model: row['model_mismatch'] = True
    if usd is not None: row['usd'] = usd
    row.update(extra)
    return row


def identity_refusal(n, at, authorized, served, resp='msg_x', reason='response_model_mismatch'):
    """response_model_refused (:1300-1306)."""
    return {'at': t(at), 'event': 'response_model_refused', 'provider': 'anthropic', 'n': n, 'reason': reason,
            'authorized_model': authorized, 'response_model': served, 'response_id': resp,
            'verified_bytes_forwarded': False, 'detail': None}


def upstream_error(n, at, usd=None, labels=None):
    """upstream_error (:1216, :1499)."""
    row = {'at': t(at), 'event': 'upstream_error', 'n': n, 'detail': 'connection reset'}
    if labels: row.update(labels)
    if usd is not None: row['usd'] = usd
    return row


def stream_error(n, at, read_failure=True, **extra):
    """Read failure (:1546, with detail) or header-write failure (:1513, n only)."""
    row = {'at': t(at), 'event': 'stream_error'}
    if n is not None: row['n'] = n
    if read_failure: row['detail'] = 'Connection reset by peer'
    row.update(extra)
    return row


def refused(at, reason='usd_limit: reached earlier in this turn', status=429, **extra):
    """refused (:1145): no n, no labels."""
    row = {'at': t(at), 'event': 'refused', 'method': 'POST', 'path': '/v1/responses', 'http_status': status,
           'reason': reason}
    row.update(extra)
    return row


SUB_LABELS = {'provider': 'openai-codex', 'auth_lane': 'subscription'}


def admitted(n, rid, at, model=ASTRA, **extra):
    """subscription_proxy_admitted (:1419): unlabelled."""
    row = {'at': t(at), 'event': 'subscription_proxy_admitted', 'n': n, 'rid': rid, 'model': model}
    row.update(extra)
    return row


def measurement(rid, status='valued', cost=0.015, reason=None):
    return {'rid': rid, 'status': status, 'cost_proxy_usd': cost if status == 'valued' else None, 'valuation_basis':
            'reported_cache_usage' if status == 'valued' else None, 'reason': reason,
            'measurement': 'api_equivalent_cost_proxy', 'billing': 'unknown', 'quota': 'unknown'}


def sub_request(n, at, rid, usage=None, resp='resp_s1', **extra):
    """Subscription request with its measurement (finish, :1459-1471)."""
    row = oa_request(n, at, usage=usage, resp=resp, tier=None, provider='openai-codex', auth_lane='subscription',
                     billing='unknown', quota='unknown', subscription_proxy=measurement(rid))
    row['requested_tier'] = None
    row.update(extra)
    return row


# ---- inputs ----------------------------------------------------------------------------------------------------
def session(sid=S1, model='openai/' + ASTRA, lane='api', lifecycle=None, logs=None, launch=True, extra_files=None,
            ignored=None, parent='root'):
    lc = lifecycle if lifecycle is not None else [('launch', '06:00:10', 1), ('retire', '06:50:00', None)]
    return {'session_id': sid, 'launch': ({'model': model, 'auth_lane': lane, 'parent': parent} if launch else None),
            'lifecycle': [{'event': e, 'at': (t(a) if a else None), **({'turn': n} if n else {})} for e, a, n in lc],
            'logs': {k: ({'lines': [line(r) for r in v], 'final_newline': True} if isinstance(v, list) else v)
                     for k, v in (logs or {}).items()},
            **({'extra_files': extra_files} if extra_files else {}), **({'ignored_sources': ignored} if ignored else {})}


def journal_admission(aid, sid, cap, launched=True, at='06:00:05', rows=None):
    r = rows if rows is not None else ([{'event': 'pending', 'admission_id': aid, 'max_usd': cap, 'at': t(at)}]
                                       + ([{'event': 'launched', 'admission_id': aid, 'session_id': sid, 'at': t('06:00:10')}]
                                          if launched else []))
    return {'admission_id': aid, 'session': sid, 'source': 'journal', 'rows': r}


def legacy_admission(sid, cap, at='06:00:05'):
    return {'admission_id': f'legacy:{sid}', 'session': sid, 'source': 'role_host_objective_admission',
            'rows': [{'event': 'objective_admission', 'session_id': sid, 'max_usd': cap, 'at': t(at)}]}


def objective(name='o', start='06:00:00', closed=None, limit='1.0', admissions=None, tree=(S1,), predecessor=None):
    return {'objective': name, 'started_at': t(start), 'closed_at': (t(closed) if closed else None), 'limit': nano(limit),
            'tree_sessions': list(tree), 'predecessor': predecessor,
            'admissions': admissions if admissions is not None else [journal_admission('a1', S1, 1.0)]}


# ---- expectations ----------------------------------------------------------------------------------------------
def deriv(provider, model, usage, nano_value, tier='1', bound='both'):
    return {'provider': provider, 'model': model, 'usage': usage, 'tier_multiplier': tier, 'nano': str(nano_value),
            'for': bound}


def valued(ids, low, high, complete, derivations):
    return {'state': 'valued', 'identities': ids, 'low': str(low), 'high': str(high), 'complete': complete,
            'derivations': derivations}


def unpriceable(reasons, supported_low=None, priced_high=None, derivations=()):
    return {'state': 'unpriceable', 'reasons': reasons, 'supported_low': None if supported_low is None else str(supported_low),
            'priced_high': None if priced_high is None else str(priced_high), 'derivations': list(derivations)}


NOT_REPORTED = {'state': 'not_reported'}


def exposure(low, low_basis, kind, high, high_basis):
    return {'pool': 'pi_api', 'low': {'nano': str(low), 'basis': low_basis},
            'high': {'kind': kind, 'nano': str(high), 'basis': high_basis}}


CODEX = {'pool': 'codex', 'billing': 'unknown'}


def attempt(key, evidence, settlement, obligation, valuation, exp, blocking=(), diagnostics=(), correlation='exact',
            rule=None, candidates=None, reserved=None, charged=None):
    c = {'kind': correlation}
    if rule: c['rule'] = rule
    if candidates: c['candidates'] = candidates
    s = dict(settlement)
    if reserved is not None: s['reserved'] = nano(reserved)
    if charged is not None: s['charged'] = nano(charged)
    return {'key': key, 'correlation': c, 'evidence': evidence, 'settlement': s, 'obligation': obligation,
            'valuation': valuation, 'exposure': exp, 'blocking': sorted(blocking), 'diagnostics': sorted(diagnostics)}


def sd(decision, basis=None, reasons=()):
    d = {'decision': decision}
    if basis: d['basis'] = basis
    d['reasons'] = sorted(reasons)
    return d


CLOSED_SETTLED = {'state': 'closed', 'amount_source': 'settlement_amount'}
CLOSED_RES = {'state': 'closed', 'amount_source': 'reservation_bound'}
OUTSTANDING = {'state': 'outstanding', 'amount_source': 'reservation_bound'}
NO_OBLIGATION = {'state': 'none', 'amount_source': 'valuation_or_unresolved'}
