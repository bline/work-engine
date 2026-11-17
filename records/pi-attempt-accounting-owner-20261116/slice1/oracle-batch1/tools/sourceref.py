# Navigation helper (not a reducer): byte-exact token coordinates and lexemes for a case file.
# Usage: python3 -I sourceref.py <case.json> <out.json>
# Emits SourceRef coordinates for every non-output leaf and every raw log line (with in-line field lexemes).
# It computes no money, validity, ownership or any policy consequence.
import hashlib, json, sys

WS = b' \t\r\n'

class Scanner:
    def __init__(self, b):
        self.b = b
        self.i = 0
        self.nodes = {}  # pointer -> (start, end, json_type, children)

    def ws(self):
        while self.i < len(self.b) and self.b[self.i] in WS:
            self.i += 1

    def value(self, ptr):
        self.ws()
        c = self.b[self.i:self.i + 1]
        s = self.i
        if c == b'{':
            self.i += 1
            kids = []
            seen = set()
            self.ws()
            if self.b[self.i:self.i + 1] == b'}':
                self.i += 1
            else:
                while True:
                    self.ws()
                    ks = self.i
                    self.string()
                    key = json.loads(self.b[ks:self.i].decode('utf-8'))
                    if key in seen:
                        raise ValueError('duplicate member %s at %s' % (key, ptr))
                    seen.add(key)
                    self.ws()
                    assert self.b[self.i:self.i + 1] == b':'
                    self.i += 1
                    cp = ptr + '/' + key.replace('~', '~0').replace('/', '~1')
                    kids.append(cp)
                    self.value(cp)
                    self.ws()
                    ch = self.b[self.i:self.i + 1]
                    self.i += 1
                    if ch == b'}':
                        break
                    assert ch == b','
            self.nodes[ptr] = (s, self.i, 'object', kids)
        elif c == b'[':
            self.i += 1
            kids = []
            self.ws()
            if self.b[self.i:self.i + 1] == b']':
                self.i += 1
            else:
                k = 0
                while True:
                    cp = ptr + '/' + str(k)
                    kids.append(cp)
                    self.value(cp)
                    k += 1
                    self.ws()
                    ch = self.b[self.i:self.i + 1]
                    self.i += 1
                    if ch == b']':
                        break
                    assert ch == b','
            self.nodes[ptr] = (s, self.i, 'array', kids)
        elif c == b'"':
            self.string()
            self.nodes[ptr] = (s, self.i, 'string', [])
        else:
            while self.i < len(self.b) and self.b[self.i:self.i + 1] not in (b',', b'}', b']') and self.b[self.i] not in WS:
                self.i += 1
            tok = self.b[s:self.i]
            t = 'null' if tok == b'null' else 'boolean' if tok in (b'true', b'false') else 'number'
            self.nodes[ptr] = (s, self.i, t, [])

    def string(self):
        assert self.b[self.i:self.i + 1] == b'"'
        self.i += 1
        while True:
            ch = self.b[self.i:self.i + 1]
            if ch == b'\\':
                self.i += 2
                continue
            self.i += 1
            if ch == b'"':
                return

def scan(b):
    sc = Scanner(b)
    sc.value('')
    sc.ws()
    assert sc.i == len(b)
    return sc.nodes

def lexeme(b, node):
    s, e, t, _ = node
    if t in ('object', 'array'):
        return None
    raw = b[s:e].decode('utf-8')
    if t == 'string':
        return json.loads(raw)
    return raw

def main(path, out):
    b = open(path, 'rb').read()
    nodes = scan(b)
    doc = json.loads(b)
    res = {
        'file': path.split('/subject/')[-1],
        'file_sha256': hashlib.sha256(b).hexdigest(),
        'inputs_canonical_sha256': hashlib.sha256(json.dumps(doc['inputs'], sort_keys=True, separators=(',', ':'), ensure_ascii=True).encode()).hexdigest(),
        'top_inventory': [p[1:] for p in nodes[''][3]],
        'tokens': {},
        'lines': [],
    }
    for ptr, node in sorted(nodes.items()):
        if ptr == '':
            continue
        top = ptr.split('/')[1]
        if top in ('expected',) or top.startswith('revisions') or top in ('rationale', 'historical'):
            continue
        if top == 'evaluations' and len(ptr.split('/')) >= 4 and ptr.split('/')[3] == 'expected':
            continue
        s, e, t, kids = node
        rec = {'byte_start': s, 'byte_end_exclusive': e, 'json_type': t,
               'token_sha256': hashlib.sha256(b[s:e]).hexdigest()}
        if t not in ('object', 'array'):
            rec['lexeme'] = lexeme(b, node)
        else:
            rec['members'] = [k.split('/')[-1] for k in kids]
        res['tokens'][ptr] = rec
    for si, sess in enumerate(doc['inputs']['sessions']):
        for turn, log in sess.get('logs', {}).items():
            for li, line in enumerate(log['lines']):
                ptr = '/inputs/sessions/%d/logs/%s/lines/%d' % (si, turn, li)
                s, e, t, _ = nodes[ptr]
                lb = line.encode('utf-8')
                rec = {'origin_id': '%s/%s:%d' % (sess['alias'], turn, li + 1), 'pointer': ptr,
                       'byte_start': s, 'byte_end_exclusive': e,
                       'token_sha256': hashlib.sha256(b[s:e]).hexdigest(),
                       'raw_utf8_sha256': hashlib.sha256(lb).hexdigest(), 'raw_utf8_length': len(lb)}
                try:
                    ln = scan(lb)
                    fields = {}
                    for fp, fn in sorted(ln.items()):
                        if fp == '':
                            continue
                        fields[fp] = {'presence': 'null' if fn[2] == 'null' else 'value', 'json_type': fn[2],
                                      'lexeme': lexeme(lb, fn)} if fn[2] not in ('object', 'array') else \
                                     {'presence': 'value', 'json_type': fn[2], 'members': [k.split('/')[-1] for k in fn[3]]}
                    rec['parse_state'] = 'object' if ln[''][2] == 'object' else 'not_object'
                    rec['fields'] = fields
                except Exception as ex:  # unreadable line
                    rec['parse_state'] = 'unreadable:%s' % ex
                res['lines'].append(rec)
            res.setdefault('log_final_newline', {})['%s/%s' % (sess['alias'], turn)] = log.get('final_newline')
    json.dump(res, open(out, 'w'), indent=1, sort_keys=True)

if __name__ == '__main__':
    main(sys.argv[1], sys.argv[2])
