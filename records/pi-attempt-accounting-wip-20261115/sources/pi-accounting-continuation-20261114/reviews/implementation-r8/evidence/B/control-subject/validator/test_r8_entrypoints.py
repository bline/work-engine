"""Independent bounded controls; exercises the shipped checker, not a copy."""
import copy, importlib.util, json, pathlib, unittest
ROOT = pathlib.Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('actual_checker', ROOT/'validator/validate_corpus.py')
v = importlib.util.module_from_spec(spec); spec.loader.exec_module(v)
def case(name): return json.loads((ROOT/'corpus/cases'/f'{name}.json').read_text())
class ActualConsumerControls(unittest.TestCase):
    def test_byte_distinct_reservation_cardinality(self):
        a={'evidence':['r1','r2']}
        self.assertEqual(v.raw_reservation(a, {'r1':'{"event":"usd_reserved","reserved_usd":0.5}', 'r2':'{"reserved_usd":0.5,"event":"usd_reserved"}'})[1], 2)
    def test_exact_byte_copy_idempotent(self):
        row='{"event":"usd_reserved","reserved_usd":0.5}'
        self.assertEqual(v.raw_reservation({'evidence':['a','b']},{'a':row,'b':row}), (500000000,1))
    def test_local_supported_lows_survive_rejection(self):
        for cid, low in [('A06-reservation-usage-null-status-no-id',15000000),('P133-A09-conflicting-reservations-with-settlement',15000000),('P130-R05-conflicting-keeps-valuation-high',20000000),('V01-cheaper-response-model-undercharge',60000000)]:
            c=case(cid); a=c['expected']['attempts'][0]; F=v.Failures(); lines,_=v.source_lines(c)
            a['exposure']['low']={'nano':str(low),'basis':'usage_valuation'}
            _,kind,high=v.att_exp(a); v.table_x(cid,a['key'],a,low,kind,high,F,lines)
            self.assertEqual(F,[],cid)
            F=v.Failures(); v.table_x(cid,a['key'],a,0,kind,high,F,lines)
            self.assertTrue(F,cid+' forced-zero mutant must fail')
    def test_ambiguous_copies_aggregate_low(self):
        c=case('DUP-response-id-across-attempts'); e=c['evaluations'][0]
        e['expected']['objectives'][0]['owned_closed']['low']='15000000'
        F=v.Failures(); lines,order=v.source_lines(c)
        v.check_evaluation(c['case_id'],c,e,{a['key'] for a in c['expected']['attempts']},{s['alias']:s for s in c['inputs']['sessions']},lines,order,F)
        self.assertEqual(F,[])
        e['expected']['objectives'][0]['owned_closed']['low']='30000000'
        F=v.Failures(); v.check_evaluation(c['case_id'],c,e,{a['key'] for a in c['expected']['attempts']},{s['alias']:s for s in c['inputs']['sessions']},lines,order,F)
        self.assertTrue(any('independent low assertion' in f for f in F),F)
if __name__=='__main__': unittest.main()
