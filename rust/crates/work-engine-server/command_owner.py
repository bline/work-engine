"""Embedded adapter: invoke the actual session-events MCP dispatcher."""
import json, pathlib, sys, types

def main():
    request = json.load(sys.stdin)
    profile, name, arguments = request['profile'], request['name'], request['arguments']
    # The Rust client has hashed this exact byte snapshot before passing it here.
    module = types.ModuleType('session_events')
    module.__file__ = profile['owner']
    sys.modules[module.__name__] = module
    exec(compile(request['owner_source'], profile['owner'], 'exec'), module.__dict__)
    homes = module.Homes(*(pathlib.Path(profile[k]) for k in ('codex_home', 'claude_home', 'state_dir')))
    bridge = None
    if name in ('submit_session_request', 'read_session_operation', 'read_session_result'):
        b = profile['bridge']
        bridge = module.Bridge(b['binary'], b['binary_sha256'], b['config'], b['config_sha256'])
    configured = profile.get('context')
    context = None
    if configured is not None:
        path = lambda key: pathlib.Path(configured[key]) if configured.get(key) else None
        context = module.Context.build(homes, path('ledger_path'), configured.get('ledger_sha256'), path('role_ledger'), path('budget_root'))
    server = module.Server(homes, 'local-query', module.CallbackPolicy(), context=context, bridge=bridge)
    result = server.call_tool({'name': name, 'arguments': arguments})
    # Do not relay owner error details, which can contain caller-provided data.
    if result.get('isError'):
        reason = 'query_rejected'
        try:
            candidate = json.loads(result['content'][0]['text'])['error']['reason']
            if candidate in {'unauthorized', 'history_unavailable', 'bridge_unavailable', 'identity_conflict', 'expired', 'target_mismatch', 'payload_conflict', 'session_busy', 'entry_uncertain', 'custody_lost'}:
                reason = candidate
        except (KeyError, TypeError, IndexError, ValueError):
            pass
        print(json.dumps({'error': reason}))
        return
    result = result['structuredContent']
    credential = arguments.get('grant_credential')
    def scrub(value):
        if isinstance(value, str):
            return value.replace(credential, '[REDACTED]') if credential else value
        if isinstance(value, list): return [scrub(x) for x in value]
        if isinstance(value, dict): return {scrub(k): scrub(v) for k, v in value.items()}
        return value
    print(json.dumps({'result': scrub(result)}, separators=(',', ':')))

if __name__ == '__main__':
    try:
        main()
    except Exception:
        print('{"error":"owner_unavailable"}')
