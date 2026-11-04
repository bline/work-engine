#!/usr/bin/env bash
# STATIC POSTAPPROVAL TEMPLATE. Must pass fake-endpoint toolset and synthetic boundary gates.
set -euo pipefail
if [ "${S10_LIVE4_ADMISSION:-}" != authorized ] || [ "${S10_LIVE4_ASSESSMENT:-}" != qualified ]; then
  echo 'BLOCKED: owner admission and effective assessment-tool qualification required' >&2
  exit 78
fi
gate_file=${S10_LIVE4_ASSESSMENT_GATE_FILE:-}
if [ -z "$gate_file" ] || [ ! -f "$gate_file" ]; then
  echo 'BLOCKED: exact assessment qualification receipt absent' >&2
  exit 78
fi
python3 - "$gate_file" "$0" <<'PY'
import hashlib,json,sys
g=json.load(open(sys.argv[1]))
assert g.get('schema')=='s10-live4-assessment-toolset-qualified-v1'
assert g.get('status')=='qualified'
assert g.get('model')=='gpt-5.4' and g.get('effort')=='medium'
assert g.get('advertised_tools')==['list_mcp_resources','list_mcp_resource_templates',
                                    'read_mcp_resource','request_user_input','mcp__corpus']
assert g.get('wrapper_sha256')==hashlib.sha256(open(sys.argv[2],'rb').read()).hexdigest()
assert g.get('credential_read_denied') is True and g.get('tool_network_denied') is True
assert g.get('sole_mcp_server')=='corpus' and g.get('mcp_resources_empty') is True
assert g.get('user_input_channel_absent') is True
PY
if [ "$#" -ne 5 ]; then
  echo 'usage: launch-isolated-arm.template.sh INPUT OUT STATE SOCKET AUDIT' >&2
  exit 64
fi
input_dir=$1; output_dir=$2; state_dir=$3; socket_dir=$4; audit_dir=$5
here=$(cd "$(dirname "$0")" && pwd -P)
codex_binary=/usr/lib/node_modules/@openai/codex/node_modules/@openai/codex-linux-x64/vendor/x86_64-unknown-linux-musl/bin/codex
for dir in "$input_dir" "$output_dir" "$state_dir" "$socket_dir" "$audit_dir"; do
  [ -d "$dir" ] && [ ! -L "$dir" ] || { echo 'missing or symlinked directory' >&2; exit 64; }
done
[ -f "$input_dir/prompt.md" ] && [ -f "$state_dir/auth.json" ] || exit 64
[ ! -e "$socket_dir/mcp.sock" ] || { echo 'socket already exists' >&2; exit 64; }
# The MCP server has its OWN network/PID/mount namespace and NO auth bind.
# It receives only this arm's /in, the fixed server program, /socket, /audit.
/usr/bin/bwrap --unshare-all --die-with-parent --new-session \
  --ro-bind /usr /usr --symlink usr/bin /bin --symlink usr/lib /lib --symlink usr/lib64 /lib64 \
  --dev /dev --proc /proc --tmpfs /tmp \
  --ro-bind "$input_dir" /in --ro-bind "$here/corpus_mcp.py" /app.py \
  --bind "$socket_dir" /socket --bind "$audit_dir" /audit \
  --clearenv --setenv PATH /usr/bin:/bin \
  -- /usr/bin/socat UNIX-LISTEN:/socket/mcp.sock,fork EXEC:/usr/bin/python3\ /app.py \
  > "$audit_dir/server.stdout.log" 2> "$audit_dir/server.stderr.log" &
server_pid=$!
cleanup() { kill "$server_pid" 2>/dev/null || true; wait "$server_pid" 2>/dev/null || true; }
trap cleanup EXIT
for _ in $(seq 1 100); do [ -S "$socket_dir/mcp.sock" ] && break; sleep .05; done
[ -S "$socket_dir/mcp.sock" ] || { echo 'MCP socket unavailable' >&2; exit 70; }
# Only the Codex process needs auth/network. No shell, patch, browser, image,
# plugins, apps or other model-selected tools are intentionally exposed.
# The effective advertised tool list must still be proved with a fake endpoint.
/usr/bin/bwrap --unshare-all --share-net --die-with-parent --new-session \
  --ro-bind /usr /usr --symlink usr/bin /bin --symlink usr/lib /lib --symlink usr/lib64 /lib64 \
  --dir /etc --ro-bind /etc/ssl /etc/ssl --ro-bind /etc/resolv.conf /etc/resolv.conf \
  --ro-bind /etc/hosts /etc/hosts --ro-bind /etc/passwd /etc/passwd --ro-bind /etc/group /etc/group \
  --dev /dev --proc /proc --tmpfs /tmp \
  --ro-bind "$input_dir" /in --bind "$output_dir" /out --bind "$state_dir" /state \
  --bind "$socket_dir" /socket \
  --clearenv --setenv HOME /state --setenv CODEX_HOME /state \
  --setenv PATH /usr/bin:/bin --setenv SSL_CERT_FILE /etc/ssl/certs/ca-certificates.crt \
  --chdir /in -- "$codex_binary" \
  --disable shell_tool --disable unified_exec --disable view_image \
  --disable image_generation --disable computer_use --disable browser_use \
  --disable apps --disable plugins --disable multi_agent --disable goals \
  --disable apply_patch_streaming_events --disable code_mode_host \
  -m gpt-5.4 -s read-only -a never --no-daemon exec \
  --ignore-user-config --ignore-rules --strict-config --skip-git-repo-check \
  -c 'web_search="disabled"' -c model_reasoning_effort=medium \
  -c 'mcp_servers.corpus.command="/usr/bin/socat"' \
  -c 'mcp_servers.corpus.args=["STDIO","UNIX-CONNECT:/socket/mcp.sock"]' \
  --json -o /out/final.md - < "$input_dir/prompt.md" > "$output_dir/events.jsonl"
