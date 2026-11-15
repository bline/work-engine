"""Independent audit scratch only; not a production reducer or acceptance gate.

Reads only assigned filtered raw inputs, their manifest, and pinned rates.
Outputs arithmetic and raw predicates, never corpus expected reducer outcomes.
"""
import collections
import hashlib
import json
from decimal import Decimal
from fractions import Fraction as F
from pathlib import Path

ROOT = Path(__file__).resolve().parent
RATE_ROOT = Path('/home/bline/cbm-work/p133-s1r5/packet/sources/producer')

def read(p):
    return json.loads(p.read_text(), parse_float=Decimal)

def sha(p):
    return hashlib.sha256(p.read_bytes()).hexdigest()

def dump(v):
    return json.dumps(v, indent=2, sort_keys=True, default=lambda x: {'json_number_literal':str(x)} if isinstance(x,Decimal) else str(x)) + '\n'

def nat(v):
    return type(v) is int and 0 <= v < 2**64

def key(v):
    return nat(v) and v > 0

def money(v):
    if isinstance(v, bool) or not isinstance(v, (int, Decimal)) or v < 0:
        return None
    n = F(v) * 10**9
    # Exact integral nano plus six fractional USD decimal places.
    return int(n) if n.denominator == 1 and F(v)*10**6 == int(F(v)*10**6) else None

def family(v):
    return 'openai' if v in ('openai', 'openai-codex') else v if v == 'anthropic' else None

def pinned(launch):
    m = launch.get('model')
    if not isinstance(m, str) or '/' not in m:
        return None
    p, name = m.split('/', 1)
    return [family(p), name] if family(p) and name else None

rates = {p:read(RATE_ROOT / (p+'_api_rates.json')) for p in ('openai','anthropic')}
manifest = read(ROOT/'RAW-MANIFEST.json')
cases = []
counterexamples = []

def price_observation(row, launch):
    u = row.get('usage')
    out = {'presence':'absent' if 'usage' not in row else 'null' if u is None else 'object' if isinstance(u,dict) else 'malformed',
           'status_200':type(row.get('http_status')) is int and row.get('http_status') == 200,
           'terminal':row.get('terminal'), 'schema_signals':[], 'policy_blocks':[]}
    pf = family(row.get('provider','openai'))
    out['declared_provider_family'] = pf
    out['status_integer']=type(row.get('http_status')) is int
    out['status_non200']=out['status_integer'] and not out['status_200']
    out['terminal_success_marker'] = row.get('terminal') == ('response.completed' if pf=='openai' else 'message_stop')
    out['terminal_status_consistent']=not out['terminal_success_marker'] or out['status_200']
    ident = pinned(launch)
    ids = [ident] if ident else []
    for k in ('model','response_model'):
        if isinstance(row.get(k),str) and row[k]:
            ids.append([pf,row[k]])
    out['identities'] = sorted({tuple(x) for x in ids},key=str)
    out['launch_identity_unresolved'] = ident is None
    out['provider_family_contradiction'] = bool(ident and pf != ident[0])
    if not isinstance(u,dict):
        return out
    anth = any(k in u for k in ('cache_read_input_tokens','cache_creation_input_tokens','ephemeral_5m_input_tokens','ephemeral_1h_input_tokens'))
    oa = any(k in u for k in ('cached_tokens','cache_write_tokens','reasoning_tokens'))
    schema = 'mixed' if anth and oa else 'anthropic' if anth else 'openai' if oa else 'unspecified'
    out['schema_signals'] = schema
    out['provider_schema_contradiction'] = schema not in (pf,'unspecified')
    if out['provider_schema_contradiction']:
        out['policy_blocks'].append('provider_schema_contradiction_no_reinterpretation')
    if pf not in rates or out['provider_schema_contradiction']:
        out['arithmetic_state'] = 'unestablished_schema_or_provider'
        return out
    required = ('input_tokens','output_tokens') if pf == 'openai' else ('input_tokens','cache_read_input_tokens','cache_creation_input_tokens','output_tokens')
    out['count_fields']={k:('absent' if k not in u else 'null' if u[k] is None else 'present_integer' if nat(u[k]) else 'malformed') for k in set(required)|set(u)}
    missing=[k for k in required if u.get(k) is None]
    bad = [k for k in required if u.get(k) is not None and not nat(u[k])]
    splits=[]
    if pf == 'openai':
        for k in ('cached_tokens','cache_write_tokens'):
            if u.get(k) is not None and not nat(u[k]): bad.append(k)
        if not bad and not missing:
            i=u['input_tokens']; c=u.get('cached_tokens'); w=u.get('cache_write_tokens')
            if c is not None and w is not None:
                splits=[(c,w)] if c+w<=i else []
            elif c is not None:
                splits=[(c,0),(c,i-c)] if c<=i else []
            elif w is not None:
                splits=[(0,w),(i-w,w)] if w<=i else []
            else: splits=[(0,0),(i,0),(0,i)]
            if not splits: bad.append('cache_sum_exceeds_input')
        out['unknown_cache_fields']=[k for k in ('cached_tokens','cache_write_tokens') if u.get(k) is None]
    else:
        w5=u.get('ephemeral_5m_input_tokens') or 0
        w1=u.get('ephemeral_1h_input_tokens') or 0
        for k in ('ephemeral_5m_input_tokens','ephemeral_1h_input_tokens'):
            if u.get(k) is not None and not nat(u[k]):bad.append(k)
        inconsistent=not bad and not missing and w5+w1>u['cache_creation_input_tokens']
        out['inconsistent_cache_split']=inconsistent
        splits=[(0,0)] if inconsistent else [(w5,w1)]
    out['malformed_count_fields']=bad
    out['required_count_unavailable_fields']=missing
    if bad or missing:
        out['arithmetic_state']='malformed_counts' if bad else 'required_counts_unavailable';return out
    tier=row.get('observed_tier')
    if pf=='openai':
        established=isinstance(tier,str) and tier in rates[pf]['tier_multiplier']
        ts=[F(rates[pf]['tier_multiplier'][tier])] if established else [max(map(F,rates[pf]['tier_multiplier'].values()))]
    else:
        established=tier in (None,'standard');ts=[F(1)]
    out['tier_established_under_legacy_policy']=established
    if not established:out['policy_blocks'].append('tier_set_unestablished')
    out['terminal_success_marker'] = row.get('terminal') == ('response.completed' if pf=='openai' else 'message_stop')
    out['complete_conjunction_candidate']=bool(out['status_200'] and out['terminal_success_marker'] and not out.get('inconsistent_cache_split',False))
    if out['status_200'] and not out['terminal_success_marker']:
        out['policy_blocks'].append('status_success_without_terminal_success_legacy_V5_ambiguity')
    if out['status_non200'] and out['terminal_success_marker']:
        out['policy_blocks'].append('non200_terminal_success_payload_consistency')
    prices=[]; unpriced=[]
    for f,m in out['identities']:
        r=rates.get(f,{}).get('models',{}).get(m)
        if not r or f!=pf:
            unpriced.append([f,m]);continue
        for c,w in splits:
            for mult in ts:
                if pf=='openai':
                    i=u['input_tokens']; long=i>rates[pf]['long_input_threshold']
                    im=F(rates[pf]['long_input_multiplier']) if long else F(1)
                    om=F(rates[pf]['long_output_multiplier']) if long else F(1)
                    terms=[(i-c-w)*F(r['input'])*im,c*F(r['cached_input'])*im,w*F(r['cache_write'])*im,u['output_tokens']*F(r['output'])*om]
                    n=sum(terms)*mult*1000
                    prices.append({'identity':[f,m],'cached':c,'write':w,'tier_multiplier':str(mult),'nano':str(n),'terms_before_tier_nano':[str(t*1000) for t in terms]})
                else:
                    residual=u['cache_creation_input_tokens']-c-w
                    for residual_rate in sorted({F(r['cache_write_5m']),F(r['cache_write_1h'])}):
                        terms=[u['input_tokens']*F(r['input']),u['cache_read_input_tokens']*F(r['cache_read']),c*F(r['cache_write_5m']),w*F(r['cache_write_1h']),residual*residual_rate,u['output_tokens']*F(r['output'])]
                        n=sum(terms)*1000
                        prices.append({'identity':[f,m],'w5m':c,'w1h':w,'residual_rate':str(residual_rate),'nano':str(n),'terms_nano':[str(t*1000) for t in terms]})
    out['derivations']=prices;out['unpriced_identities']=unpriced
    out['arithmetic_state']='partial_pricing' if unpriced or not established or ident is None else 'valued'
    out['supported_low_nano']=str(min(F(x['nano']) for x in prices)) if prices and established and not unpriced and ident is not None else None
    out['priced_high_nano']=str(max(F(x['nano']) for x in prices)) if prices else None
    return out

for item in manifest['cases']:
    p=Path(item['raw_path']); assert sha(p)==item['raw_sha256'],p
    d=read(p);assert set(d)=={'case_id','inputs'}
    entry={'case_id':d['case_id'],'raw_sha256':sha(p),'selected':False,'sessions':[]}
    for s in d['inputs']['sessions']:
        launch=s.get('launch') or {}
        ss={'alias':s['alias'],'launch':launch,'sources':[]}
        for source,log in s.get('logs',{}).items():
            rows=[]
            for n,line in enumerate(log['lines'],1):
                try: obj=json.loads(line,parse_float=Decimal)
                except Exception:obj=None
                rows.append({'ref':f'{s["alias"]}/{source}:{n}','line_sha256':hashlib.sha256(line.encode()).hexdigest(),'raw':obj,'adapter_final_line_unreadable':n==len(log['lines']) and not log['final_newline']})
            dollar=any(isinstance(r['raw'],dict) and (r['raw'].get('event')=='usd_reserved' or 'usd' in r['raw']) for r in rows)
            if launch.get('auth_lane')!='api' or not dollar:continue
            entry['selected']=True
            out={'source':source,'rows':[],'reservation_groups':[]}
            groups=collections.defaultdict(list)
            for r in rows:
                x=r['raw']; rr={k:v for k,v in r.items() if k!='raw'}
                if not isinstance(x,dict):rr['state']='non_object_or_unparseable';out['rows'].append(rr);continue
                rr['event']=x.get('event');rr['raw']=x
                rr['usage_presence']='absent' if 'usage' not in x else 'null' if x['usage'] is None else 'object' if isinstance(x['usage'],dict) else 'malformed'
                if x.get('event')=='usd_reserved':
                    g='rid='+str(x['rid']) if key(x.get('rid')) else 'unusable-rid@'+r['ref']
                    groups[g].append((r,x));rr['reservation_group']=g
                    rr['n_usable']=key(x.get('n'));rr['reserved_nano']=money(x.get('reserved_usd'))
                if x.get('event') in ('stream_error','response_model_refused'):
                    same=[q for q in rows if isinstance(q['raw'],dict) and key(x.get('n')) and type(q['raw'].get('n')) is int and q['raw'].get('n')==x['n']]
                    rr['shape_join_raw_predicates']={'n_usable':key(x.get('n')),'read_failure_shape':x.get('event')=='stream_error' and 'detail' in x,'same_n_request_or_upstream_refs':[q['ref'] for q in same if q['raw'].get('event') in ('request','upstream_error')],'same_n_reservation_rids':sorted({q['raw']['rid'] for q in same if q['raw'].get('event')=='usd_reserved' and key(q['raw'].get('rid'))}),'no_join_decision_claimed':True}
                if x.get('event') in ('request','response_model_refused'):
                    rr['usage_facts']=price_observation(x,launch)
                if isinstance(x.get('usd'),dict):
                    usd=x['usd'];rid=usd.get('rid')
                    matched=[q for q in rows if isinstance(q['raw'],dict) and q['raw'].get('event')=='usd_reserved' and key(rid) and type(q['raw'].get('rid')) is int and q['raw'].get('rid')==rid]
                    rr['reservation_candidates']=[q['ref'] for q in matched]
                    rr['comparison_unique_reservation']=len({q['line_sha256'] for q in matched})==1
                    rr['claimed_charge_nano']=money(usd.get('charged_usd'))
                    rr['settlement_reserved_nano']=money(usd.get('reserved_usd'))
                    if rr['comparison_unique_reservation']:
                        a=money(matched[0]['raw'].get('reserved_usd'))
                        rr['unique_reservation_comparison']={'reservation_nano':a,'claimed_reserved_equal':a is not None and rr['settlement_reserved_nano']==a,'charge_minus_reservation_nano':rr['claimed_charge_nano']-a if a is not None and rr['claimed_charge_nano'] is not None else None}
                    if not rr['comparison_unique_reservation']:rr['comparison_block']='no_unique_reservation_do_not_compare_to_arbitrary_R'
                    if usd.get('charge_basis')=='error_status':rr['error_status_predicates']={'status_integer':type(x.get('http_status')) is int,'status_non200':type(x.get('http_status')) is int and x['http_status']!=200,'charge_zero':money(usd.get('charged_usd'))==0,'terminal':x.get('terminal'),'usage_present':'usage' in x,'policy':'retain_reservation_and_independent_usage_no_ordinary_usage_replacement'}
                out['rows'].append(rr)
            for g,rs in groups.items():
                distinct={r['line_sha256']: (r,x) for r,x in rs}
                amounts=[money(x.get('reserved_usd')) for r,x in distinct.values()]
                amounts=[a for a in amounts if a is not None]
                gg={'raw_group':g,'refs':[r['ref'] for r,x in rs],'distinct_rows':len(distinct),'distinct_amounts_nano':sorted(set(amounts)),'maximum_known_reservation_nano':max(amounts,default=0),'malformed_amount_refs':[r['ref'] for r,x in rs if money(x.get('reserved_usd')) is None],'conflict':len(distinct)>1,'writer_floor_checks':[]}
                ident=pinned(launch)
                for r,x in distinct.values():
                    f,m=ident or (None,None);rates_m=rates.get(f,{}).get('models',{}).get(m)
                    bound=x.get('input_bound');a=money(x.get('reserved_usd'))
                    z={'ref':r['ref'],'exact_writer_amount':'unestablished_output_cap_carried_floor_config_and_input_basis_not_fully_present'}
                    if rates_m and nat(bound):
                        if f=='openai':
                            dearest=max(F(rates_m[k]) for k in ('input','cache_write'))
                            mult=min(map(F,rates[f]['tier_multiplier'].values()))
                            long=F(rates[f]['long_input_multiplier']) if bound>rates[f]['long_input_threshold'] else F(1)
                        else:dearest=max(F(rates_m[k]) for k in ('input','cache_write_5m','cache_write_1h'));mult=long=F(1)
                        floor=bound*dearest*mult*long*1000
                        z.update(input_only_minimum_exact_nano=str(floor),minimum_multiplier=str(mult),rounding_allowance_nano=500,logged_below_proven_minimum=a is not None and F(a)<floor-500)
                        if z['logged_below_proven_minimum']:counterexamples.append({'case_id':d['case_id'],**z,'logged_reserved_nano':a})
                    else:z['input_only_minimum_exact_nano']=None
                    gg['writer_floor_checks'].append(z)
                out['reservation_groups'].append(gg)
            ss['sources'].append(out)
        if ss['sources']:entry['sessions'].append(ss)
    entry['coverage_disposition']='selected_raw_api_dollar_evidence' if entry['selected'] else 'excluded_no_api_launch_source_with_reservation_or_dollar_settlement'
    entry['scope_unestablished_missing_launch']=not entry['selected'] and any(s.get('launch') is None for s in d['inputs']['sessions'])
    cases.append(entry)

result={'schema':'independent-raw-api-axis-facts/1','authority':'candidate_arithmetic_and_raw_predicates_only_no_reducer_acceptance','raw_decimal_encoding':'Decimal JSON numeric values are tagged {json_number_literal:string}; these are numeric inputs, not JSON strings. Raw file and line digests are authoritative.','selection':'all cases having an api-launched session source with usd_reserved event or usd field, regardless of rid, amount, final-newline validity, expected key or profile; all such sources covered; future-gate duplicates included because profile stripped','derivation_rules':{'D1':'source/line identity and distinct reservation maximum; no amount-based correlation; groups are lexical facts including final-line unreadable flags, not typed-adapter or reducer attempts','D2':'provider-qualified launch split once; row provider defaults OpenAI; no cross-provider reinterpretation','D3':'exact Fraction rates ×1000 yields nano; OpenAI feasible cache vertices; Anthropic residual min/max and inconsistent split entire-write range','D4':'status and terminal predicates separate; complete conjunction is a candidate not policy disposition','D5':'input-only writer lower bound: bound×dearest_input_rate×minimum_possible_multiplier×long_multiplier×1000; output term nonnegative; logged reservation may round down ≤500nano'},'manifest_sha256':sha(ROOT/'RAW-MANIFEST.json'),'rate_sha256':{f:sha(RATE_ROOT/(f+'_api_rates.json')) for f in rates},'case_count':len(cases),'selected_case_count':sum(c['selected'] for c in cases),'cases':cases,'writer_floor_counterexamples':counterexamples}
(ROOT/'FACTS.json').write_text(dump(result))
print(dump({'cases':len(cases),'selected':result['selected_case_count'],'writer_floor_counterexamples':counterexamples,'facts_sha256':sha(ROOT/'FACTS.json')}))
