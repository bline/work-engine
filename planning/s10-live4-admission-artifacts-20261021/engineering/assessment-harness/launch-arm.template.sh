#!/usr/bin/env bash
# STATIC TEMPLATE. Do not run before owner authorization and offline boundary gate.
set -euo pipefail
# The proposed Codex CLI mount layout below exposes the auth file and shared
# network to model-generated tools. It is deliberately nonlaunchable until a
# separately reviewed tool boundary replaces this guard and proves both denies.
echo 'BLOCKED: tool credential-read and network-egress isolation unestablished' >&2
exit 78
if [ "$#" -ne 4 ]; then
  echo 'usage: launch-arm.template.sh INPUT_DIR OUTPUT_DIR STATE_DIR AUTH_FILE' >&2
  exit 64
fi
input_dir=$1
output_dir=$2
state_dir=$3
auth_file=$4
codex_binary=/usr/lib/node_modules/@openai/codex/node_modules/@openai/codex-linux-x64/vendor/x86_64-unknown-linux-musl/bin/codex
for path in "$input_dir" "$output_dir" "$state_dir"; do
  [ -d "$path" ] || { echo 'required directory missing' >&2; exit 64; }
done
[ -f "$input_dir/prompt.md" ] && [ -f "$auth_file" ] || { echo 'prompt/auth missing' >&2; exit 64; }
[ -f "$state_dir/auth.json" ] || { echo 'empty auth mountpoint required' >&2; exit 64; }
# The process receives no bind of /home/bline, the repository, key, or other arm.
# The credential is mounted read-only; if refresh is required, fail this gate.
/usr/bin/bwrap --unshare-all --share-net --die-with-parent --new-session \
  --ro-bind /usr /usr --symlink usr/bin /bin --symlink usr/lib /lib --symlink usr/lib64 /lib64 \
  --dir /etc --ro-bind /etc/ssl /etc/ssl \
  --ro-bind /etc/resolv.conf /etc/resolv.conf --ro-bind /etc/hosts /etc/hosts \
  --ro-bind /etc/passwd /etc/passwd --ro-bind /etc/group /etc/group \
  --dev /dev --proc /proc --tmpfs /tmp \
  --ro-bind "$input_dir" /in --bind "$output_dir" /out --bind "$state_dir" /state \
  --ro-bind "$auth_file" /state/auth.json \
  --clearenv --setenv HOME /state --setenv CODEX_HOME /state \
  --setenv PATH /usr/bin:/bin --setenv SSL_CERT_FILE /etc/ssl/certs/ca-certificates.crt \
  --chdir /in -- "$codex_binary" -m gpt-6-sol -s read-only -a never --no-daemon exec \
  --ignore-user-config --ignore-rules --skip-git-repo-check \
  -c model_reasoning_effort=medium --json -o /out/final.md - \
  < "$input_dir/prompt.md" > "$output_dir/events.jsonl"
