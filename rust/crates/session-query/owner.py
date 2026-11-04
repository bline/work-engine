#!/usr/bin/python3
"""Bounded local adapter to the pinned session-events MCP tool owner."""
import hashlib, importlib.util, json, pathlib, sys

OWNER_SHA = 'e4ceb6d7f5f7685f8c9c0ad84ce7ddd54fd1dc6bae8cc71f35be86373e53c716'

def main():
    request = json.load(sys.stdin)
    profile, name, arguments = request['profile'], request['name'], request['arguments']
    allowed = {'list_sessions': {'query', 'host', 'limit'},
               'read_session': {'session_id', 'entry_id', 'before', 'after', 'tail', 'roles', 'text_offset', 'max_chars'},
               'read_session_operation': {'request_id', 'grant_credential'},
               'read_session_result': {'request_id', 'grant_credential', 'text_offset', 'max_chars'}}
    if name not in allowed or not isinstance(arguments, dict) or set(arguments) - allowed[name]:
        raise ValueError('invalid query')
    owner = pathlib.Path(profile['owner'])
    if not owner.is_absolute() or profile['owner_sha256'] != OWNER_SHA or hashlib.sha256(owner.read_bytes()).hexdigest() != OWNER_SHA:
        raise ValueError('owner pin mismatch')
    spec = importlib.util.spec_from_file_location('session_events', owner)
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    homes = module.Homes(*(pathlib.Path(profile[k]) for k in ('codex_home', 'claude_home', 'state_dir')))
    bridge = None
    if name in ('read_session_operation', 'read_session_result'):
        b = profile['bridge']
        bridge = module.Bridge(b['binary'], b['binary_sha256'], b['config'], b['config_sha256'])
    server = module.Server(homes, 'local-query', module.CallbackPolicy(), bridge=bridge)
    result = server.call_tool({'name': name, 'arguments': arguments})
    print(json.dumps(result, separators=(',', ':')))

if __name__ == '__main__':
    try:
        main()
    except Exception:
        print('{"error":"owner_unavailable"}')
        sys.exit(1)
