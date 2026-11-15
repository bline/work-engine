"""P1.28 D10 check: every producer rate (parsed from the JSON text as exact decimals, never floats), alone and under every
multiplier combination the producer can apply, as nano-USD per token. Reports whether each is an integer, and the
denominators otherwise. Usage: representability.py <openai_api_rates.json> <anthropic_api_rates.json>"""
import json, sys, itertools
from decimal import Decimal
from fractions import Fraction
def load(p):
    return json.loads(open(p).read(), parse_float=Decimal, parse_int=Decimal)
oa, an = load(sys.argv[1]), load(sys.argv[2])
NANO_PER_USD = Fraction(10**9); PER_TOKEN = Fraction(1, 10**6)   # rates are USD per million tokens
rows, worst = [], 1
def nano(rate, mult):
    return Fraction(rate) * Fraction(mult) * PER_TOKEN * NANO_PER_USD
tiers = {k: v for k, v in oa['tier_multiplier'].items()}
li, lo = oa['long_input_multiplier'], oa['long_output_multiplier']
for model, r in oa['models'].items():
    for field, rate in r.items():
        long_m = lo if field == 'output' else li
        for (tname, tm), longp in itertools.product(tiers.items(), (False, True)):
            v = nano(rate, Fraction(tm) * (Fraction(long_m) if longp else 1))
            worst = max(worst, v.denominator); rows.append(('openai', model, field, tname, longp, v))
an_models = an.get('models', an)
for model, r in an_models.items():
    if not isinstance(r, dict): continue
    for field, rate in r.items():
        if isinstance(rate, Decimal):
            v = nano(rate, 1); worst = max(worst, v.denominator); rows.append(('anthropic', model, field, '-', False, v))
non_int = [x for x in rows if x[5].denominator != 1]
print(f'rates checked: {len(rows)} (openai {sum(1 for x in rows if x[0]=="openai")}, anthropic {sum(1 for x in rows if x[0]=="anthropic")})')
print(f'non-integer nano-USD per token: {len(non_int)}; largest denominator: {worst}')
for x in non_int[:12]: print('  ', x[:5], x[5])
print('raw rate literals:', sorted({str(v) for m in oa['models'].values() for v in m.values()} | {str(v) for m in an_models.values() if isinstance(m, dict) for v in m.values() if isinstance(v, Decimal)}))
print('multipliers: tiers', {k: str(v) for k, v in tiers.items()}, 'long input', li, 'long output', lo)
print('anthropic keys', {k: [kk for kk in v] if isinstance(v, dict) else v for k, v in an.items() if k != 'models'})
