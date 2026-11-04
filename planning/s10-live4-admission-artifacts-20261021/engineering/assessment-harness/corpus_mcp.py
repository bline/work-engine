#!/usr/bin/python3
"""Static MCP stdio reader; run only in a separate, credential-free bwrap.

Protocol/toolset still requires fake-endpoint and actual offline qualification.
"""
import base64, hashlib, json, os, pathlib, sys, time
ROOT = pathlib.Path('/in')
AUDIT = pathlib.Path('/audit/reads.jsonl')
MAX = 32768

def inventory():
    result = {}
    for p in ROOT.rglob('*'):
        if p.is_symlink(): raise ValueError('symlink in input')
        if p.is_file():
            rel = p.relative_to(ROOT).as_posix()
            result[rel] = {'bytes': p.stat().st_size, 'sha256': hashlib.sha256(p.read_bytes()).hexdigest()}
    return dict(sorted(result.items()))
FILES = inventory()

def log(method, arguments, outcome):
    with AUDIT.open('a') as f:
        f.write(json.dumps({'at': time.time(), 'method': method, 'arguments': arguments,
                            'outcome': outcome}, sort_keys=True) + '\n')

def tool(name, arguments):
    if name == 'list_artifacts':
        log(name, {}, 'listed')
        return {'content': [{'type': 'text', 'text': json.dumps(FILES, sort_keys=True)}]}
    if name != 'read_artifact':
        log('unknown', {}, 'denied')
        return {'isError': True, 'content': [{'type': 'text', 'text': 'Unavailable'}]}
    path = arguments.get('path'); offset = arguments.get('offset', 0); length = arguments.get('length', MAX)
    if not isinstance(path, str) or path not in FILES or not isinstance(offset, int) or not isinstance(length, int) or offset < 0 or length < 1 or length > MAX:
        log(name, {'path': str(path)[:100]}, 'denied')
        return {'isError': True, 'content': [{'type': 'text', 'text': 'Unavailable'}]}
    p = ROOT / path
    if p.is_symlink() or not p.is_file() or not p.resolve().is_relative_to(ROOT.resolve()):
        log(name, {'path': path}, 'denied')
        return {'isError': True, 'content': [{'type': 'text', 'text': 'Unavailable'}]}
    with p.open('rb') as f:
        f.seek(offset); data = f.read(length)
    log(name, {'path': path, 'offset': offset, 'length': length}, {'returned_bytes': len(data), 'sha256': hashlib.sha256(data).hexdigest()})
    chunk = {'path': path, 'offset': offset, 'returned_bytes': len(data),
             'file_bytes': FILES[path]['bytes'], 'file_sha256': FILES[path]['sha256']}
    try:
        chunk['utf8'] = data.decode('utf-8')
    except UnicodeDecodeError:
        chunk['base64'] = base64.b64encode(data).decode()
    return {'content': [{'type': 'text', 'text': json.dumps(chunk)}]}

TOOLS = [
 {'name': 'list_artifacts', 'description': 'List exact admitted input artifact names, sizes and SHA-256 hashes.', 'inputSchema': {'type': 'object', 'properties': {}, 'additionalProperties': False}},
 {'name': 'read_artifact', 'description': 'Read at most 32768 bytes from an exact listed input artifact as base64.', 'inputSchema': {'type': 'object', 'properties': {'path': {'type': 'string'}, 'offset': {'type': 'integer', 'minimum': 0}, 'length': {'type': 'integer', 'minimum': 1, 'maximum': MAX}}, 'required': ['path'], 'additionalProperties': False}}
]
for raw in sys.stdin:
    try:
        req = json.loads(raw)
        if 'id' not in req: continue
        method = req.get('method')
        if method in ('initialize', 'tools/list', 'resources/list', 'resources/templates/list', 'resources/read'):
            log('protocol:'+method, {}, 'served')
        if method == 'initialize':
            result = {'protocolVersion': req.get('params', {}).get('protocolVersion', '2025-06-18'), 'capabilities': {'tools': {}}, 'serverInfo': {'name': 's10-live4-corpus', 'version': '0.1'}}
        elif method == 'tools/list': result = {'tools': TOOLS}
        elif method == 'resources/list': result = {'resources': []}
        elif method == 'resources/templates/list': result = {'resourceTemplates': []}
        elif method == 'resources/read': result = None
        elif method == 'tools/call':
            params = req.get('params', {}); result = tool(params.get('name'), params.get('arguments') or {})
        elif method == 'ping': result = {}
        else: result = None
        if result is None:
            response = {'jsonrpc': '2.0', 'id': req['id'], 'error': {'code': -32601, 'message': 'Method not found'}}
        else: response = {'jsonrpc': '2.0', 'id': req['id'], 'result': result}
        sys.stdout.write(json.dumps(response, separators=(',', ':')) + '\n'); sys.stdout.flush()
    except Exception as e:
        # Fail closed; never echo paths or secret-bearing exception text to model.
        sys.stderr.write(type(e).__name__ + '\n'); sys.stderr.flush()
        if 'id' in locals().get('req', {}):
            sys.stdout.write(json.dumps({'jsonrpc': '2.0', 'id': req['id'], 'error': {'code': -32603, 'message': 'Unavailable'}}) + '\n'); sys.stdout.flush()
