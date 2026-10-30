import argparse
import contextlib
import io
import json
import os
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, os.path.dirname(os.path.dirname(__file__)))
import usage_ledger as L

class BadRecords(unittest.TestCase):
    def test_bad_usage_and_timestamp_do_not_stop_build(self):
        with tempfile.TemporaryDirectory() as d:
            project = os.path.join(d, '.claude', 'projects', 'p'); os.makedirs(project)
            claude = os.path.join(project, 's.jsonl'); codex = os.path.join(d, 'rollout.jsonl')
            rows = [dict(type='assistant', timestamp='broken', sessionId='s', message=dict(id='bad-time', model='claude-opus-5-5', usage=dict(input_tokens=1, cache_creation_input_tokens=0, cache_read_input_tokens=0, output_tokens=1))),
                    dict(type='assistant', timestamp='2026-01-01T01:00:00Z', sessionId='s', message=dict(id='bad-usage', model='claude-opus-5-5', usage=dict(input_tokens='bad', cache_creation_input_tokens=0, cache_read_input_tokens=0, output_tokens=1))),
                    dict(type='assistant', timestamp='2026-01-01T01:00:01Z', sessionId='s', message=dict(id='good', model='claude-opus-5-5', usage=dict(input_tokens=1, cache_creation_input_tokens=0, cache_read_input_tokens=0, output_tokens=1)))]
            with open(claude, 'w') as f:
                for row in rows: f.write(json.dumps(row, separators=(',', ':'))+'\n')
            records = [dict(type='session_meta', timestamp='2026-01-01T00:00:00Z', payload={'id':'t'}),
                       dict(type='token_usage_record', timestamp='bad', payload=dict(thread_id='t', response_id='r1', usage=dict(input_tokens=1, output_tokens=1))),
                       dict(type='token_usage_record', timestamp='2026-01-01T01:00:00Z', payload=dict(thread_id='t', response_id='r2', usage=dict(input_tokens='oops', output_tokens=1))),
                       dict(type='token_usage_record', timestamp='2026-01-01T01:00:01Z', payload=dict(thread_id='t', response_id='r3', usage=dict(input_tokens=1, output_tokens=1)))]
            with open(codex, 'w') as f:
                for row in records: f.write(json.dumps(row, separators=(',', ':'))+'\n')
            out = os.path.join(d, 'out'); args = argparse.Namespace(since='2026-01-01T00:00:00Z', until=None, out=out, no_codex=False, no_claude=False, streams=None)
            L.ISSUES.clear()
            with patch.object(L, 'CODEX_GLOBS', [codex]), patch.object(L, 'CLAUDE_GLOB', claude), patch.object(L, 'thread_index', return_value={}), contextlib.redirect_stdout(io.StringIO()): L.cmd_build(args)
            with open(os.path.join(out, 'calls.jsonl')) as f: calls = {r['call_id']: r for r in map(json.loads, f)}
            self.assertEqual(set(calls), {'bad-time','bad-usage','good','r1','r2','r3'})
            for name in ('bad-time','bad-usage','r1','r2'): self.assertIsNone(calls[name]['usd'])
            self.assertIsNone(calls['bad-usage']['weighted']); self.assertIsNone(calls['r2']['weighted'])
            self.assertEqual(calls['bad-usage']['usage']['input_tokens'], 'bad')
            with open(os.path.join(out,'issues.jsonl')) as f: kinds = {r['kind'] for r in map(json.loads,f)}
            self.assertTrue({'invalid_call_timestamp', 'invalid_call_usage'} <= kinds)
