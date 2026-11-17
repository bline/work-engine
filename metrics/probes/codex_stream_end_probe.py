#!/usr/bin/env python3
"""Live probe: how the ChatGPT Codex backend ends an SSE response.

Sends one minimal tool-free request (model given, store:false, stream:true)
over raw TLS with Pi's openai-codex OAuth access (read from Pi's auth store,
never printed or retained), and records per-read timing, HTTP framing, any
bytes after the first `response.completed`, a `data: [DONE]` sentinel, the
framed end, and when the server closes. Raw bytes are retained only after a
screen for the access token. Prints a JSON summary without secrets.
Usage: codex_stream_end_probe.py <out-dir> <model> [tag] [codex|api]
`api` sends the same body to the public Responses API with OPENAI_API_KEY.
"""
import hashlib, json, os, socket, ssl, sys, time, uuid
from pathlib import Path

out, model = Path(sys.argv[1]), sys.argv[2]
tag = sys.argv[3] if len(sys.argv) > 3 else 'run'
route = sys.argv[4] if len(sys.argv) > 4 else 'codex'
if route == 'codex':
    auth = json.loads(Path('~/.pi/agent/auth.json').expanduser().read_text())['openai-codex']
    token, account = auth['access'], auth['accountId']
    if auth['expires'] / 1000 < time.time() + 60:
        sys.exit('access expired; not refreshing in a probe')
    host, path = 'chatgpt.com', '/backend-api/codex/responses'
else:
    token, account = os.environ['OPENAI_API_KEY'], None
    host, path = 'api.openai.com', '/v1/responses'
session = str(uuid.uuid4())
body = json.dumps({
    'model': model, 'store': False, 'stream': True,
    'instructions': 'You are a terse assistant.',
    'input': [{'role': 'user', 'content': [{'type': 'input_text', 'text': 'Reply with exactly: PROBE_OK'}]}],
    'text': {'verbosity': 'low'}, 'include': ['reasoning.encrypted_content'],
    'prompt_cache_key': session, 'tool_choice': 'auto', 'parallel_tool_calls': True,
    'reasoning': {'effort': 'low', 'summary': 'auto'},
}).encode()
head = (f'POST {path} HTTP/1.1\r\nHost: {host}\r\n'
        f'Authorization: Bearer {token}\r\n' + (f'chatgpt-account-id: {account}\r\n' if account else '') + 'originator: pi\r\n'
        f'OpenAI-Beta: responses=experimental\r\naccept: text/event-stream\r\ncontent-type: application/json\r\n'
        f'session-id: {session}\r\nx-client-request-id: {session}\r\n'
        f'Content-Length: {len(body)}\r\nConnection: keep-alive\r\n\r\n').encode()
ctx = ssl.create_default_context()
t0 = time.monotonic()
raw = socket.create_connection((host, 443), timeout=20)
s = ctx.wrap_socket(raw, server_hostname=host)
s.sendall(head + body)
s.settimeout(1.0)
reads, buf = [], b''
deadline = t0 + 90
completed_at = None
closed = None
while time.monotonic() < deadline:
    try:
        b = s.recv(65536)
    except socket.timeout:
        if completed_at and time.monotonic() - completed_at > 15:
            break  # observe up to 15 s after the terminal event
        continue
    now = round(time.monotonic() - t0, 3)
    if not b:
        closed = now
        break
    reads.append((now, len(b)))
    buf += b
    if completed_at is None and b'response.completed' in buf:
        completed_at = time.monotonic()
s.close()
hdr, _, rest = buf.partition(b'\r\n\r\n')
status = hdr.split(b'\r\n')[0].decode(errors='replace')
headers = [l.decode(errors='replace').split(':')[0].lower() for l in hdr.split(b'\r\n')[1:]]
chunked = b'transfer-encoding: chunked' in hdr.lower()
# Decode chunked framing.
payload, framed_end, p = b'', False, rest
if chunked:
    while p:
        line, sep, after = p.partition(b'\r\n')
        if not sep: break
        n = int(line.split(b';')[0] or b'0', 16)
        if n == 0: framed_end = True; break
        payload += after[:n]; p = after[n + 2:]
else:
    payload = rest
text = payload.decode(errors='replace')
first = text.find('"type":"response.completed"')
blk_end = text.find('\n\n', first) + 2 if first >= 0 else -1
tail = text[blk_end:] if blk_end > 1 else None
leak = token.encode() in buf
summary = {
    'tag': tag, 'route': route, 'model': model, 'status': status, 'chunked': chunked, 'framed_end': framed_end,
    'server_closed_s': closed, 'reads': len(reads), 'first_byte_s': reads[0][0] if reads else None,
    'last_read_s': reads[-1][0] if reads else None,
    'completed_seen_s': round(completed_at - t0, 3) if completed_at else None,
    'completed_count': text.count('"type":"response.completed"'),
    'tail_after_first_completed': tail, 'done_sentinel': 'data: [DONE]' in text,
    'event_types': sorted({json.loads(l[5:])['type'] for l in text.splitlines() if l.startswith('data:') and l[5:].strip() not in ('', '[DONE]')}),
    'response_headers': sorted(set(headers)), 'payload_bytes': len(payload),
    'access_echoed': leak,
}
if not leak:
    (out / f'{tag}.raw').write_bytes(buf)
    summary['raw_sha256'] = hashlib.sha256(buf).hexdigest()
(out / f'{tag}.json').write_text(json.dumps({**summary, 'read_timeline': reads}, indent=1))
print(json.dumps(summary, indent=1))
