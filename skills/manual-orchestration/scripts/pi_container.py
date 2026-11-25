#!/usr/bin/env python3
"""Host side of a containerized Pi turn: mount plan, container command, egress proxy, runner.

role_host.py builds a turn spec and starts `pi_container.py run-turn SPEC` in
its own process group, writing the provider API key to its stdin. The runner:

  1. serves the egress proxy on a Unix socket mounted into the container;
  2. starts the container detached (`docker run -d`), so the Docker client is
     never in the path of Pi's output;
  3. waits for it, records `docker inspect` state, removes it, stops the proxy.

SIGTERM or SIGINT to the runner stops the container (`docker stop -t 10`).

The container runs the host's own /usr and /etc read-only, so tools, versions,
users and paths match the host. It has no network: its only route out is the
proxy, which accepts POST /v1/responses (or, for provider anthropic, POST
/v1/messages) for the pinned model only, refuses
hosted (server-side) tools, sets or removes service_tier, substitutes the real
key for the container's placeholder, and logs the tier and usage the provider
reported. The key never enters the container.

With a dollar limit (UsdBudget), the proxy also caps max_output_tokens, sets
aside each request's worst-case cost before forwarding it, refuses with 429
once spend plus that amount would pass the limit, records the real charge from
the provider's final usage, and has the runner stop the container on the first
such refusal.

Host facts: metrics/probes/pi-docker-host-20261109.json.
"""
from __future__ import annotations

import base64
import hashlib
import http.client
import json
import math
import os
import platform
import re
import shutil
import signal
import socketserver
import subprocess
import sys
import threading
import time
import urllib.parse
from http.server import BaseHTTPRequestHandler
from pathlib import Path

HERE = Path(__file__).resolve().parent
# Digest-pinned Node image shared with rust/crates/work-engine-manual-host
# (pi_docker::IMAGE). Only its skeleton is used: /usr and /etc come from the host.
IMAGE = "node@sha256:4b150b48b4116e2a22f2c39cc27025fbf4f153e2078fd3b14d159fdb5c7279da"
DEFAULT_RUNTIME = "/snap/pi-coding-agent/current"
PLACEHOLDER_KEY = "sk-role-host-container-placeholder"
RELAY_PORT = 8080
# codebase-memory needs an existing private runtime dir whose socket path fits
# in 108 bytes; a container tmpfs gives it one at a short fixed path.
CBM_RUNTIME = "/run/cbm"
USERLAND = ("/usr", "/etc")
# Never offered as user mounts: system trees, and the userland already mounted.
SYSTEM_PREFIXES = ("/usr", "/etc", "/proc", "/sys", "/dev", "/run", "/snap", "/boot", "/var/lib/docker")
ALLOWED_TOOL_TYPES = frozenset({"function", "custom", "namespace"})
# Top-level Responses fields the proxy forwards. Anything else (stored prompts,
# conversations, previous responses, background mode, future fields) is refused
# rather than forwarded unexamined. Pi 1.4.0 sends a subset of these.
ALLOWED_FIELDS = frozenset({
    "model", "input", "instructions", "stream", "stream_options", "store", "include", "reasoning",
    "service_tier", "max_output_tokens", "max_tool_calls", "prompt_cache_key", "prompt_cache_retention",
    "text", "tool_choice", "tools", "parallel_tool_calls", "temperature", "top_p", "top_logprobs",
    "truncation", "metadata", "safety_identifier", "user"})
TOOL_CHOICE_STRINGS = frozenset({"auto", "none", "required"})
TOOL_CHOICE_TYPES = frozenset({"function", "custom", "allowed_tools"})
# Proxy resource bounds: the proxy runs on the host, outside the container's limits.
MAX_CONNECTIONS = 8
MAX_BODY_BYTES = 64 * 1024 * 1024
SOCKET_TIMEOUT = 60
# With a subscription budget, how long a request waits for the in-flight exchange to
# settle before it is refused. Pi sends its next request as soon as it reads the
# terminal event, while the proxy is still awaiting upstream EOF and writing the
# durable row; that next request queues rather than failing with a 429.
ADMISSION_WAIT_SECONDS = 60
TERMINAL_EVENTS = frozenset({"response.completed", "response.done", "response.incomplete", "response.failed"})
MEMORY = re.compile(r"^[1-9][0-9]*[kmg]?$")
RATES_PATH = HERE / "openai_api_rates.json"
# Output cap applied with a dollar limit when none is given. Observed Astra
# review output (8 proxy logs, 136 requests, 2026-11-10): max 7,847 tokens.
DEFAULT_MAX_OUTPUT_TOKENS = 16000
# Framing tokens allowed per input item and once per request on top of the
# byte bound. Not measured; chosen well above typical per-message framing.
ITEM_OVERHEAD_TOKENS = 32

# Anthropic Messages, as Pi 1.4.0's "anthropic-messages" API sends it (stage A
# record: ~/cbm-work/pi-anthropic-p2-20261110/stage-a-precheck.md). Pi treats a
# key containing "sk-ant-oat" as an OAuth token, so the placeholder avoids it.
ANTHROPIC_PLACEHOLDER_KEY = "sk-ant-api-role-host-container-placeholder"
ANTHROPIC_VERSION = "2023-06-01"
ANTHROPIC_ALLOWED_FIELDS = frozenset({
    "model", "messages", "max_tokens", "stream", "system", "temperature", "top_p", "top_k", "stop_sequences",
    "tools", "tool_choice", "thinking", "output_config", "metadata"})
# Betas Pi can request. server-side-fallback is left out: it lets another model
# serve the request, which defeats the model pin.
ANTHROPIC_BETAS = frozenset({
    "fine-grained-tool-streaming-2025-05-14", "interleaved-thinking-2025-05-14",
    "mid-conversation-output-config-2026-07-01", "thinking-binding-controls-2026-08-01",
    "inline-tools-2026-09-15"})
ANTHROPIC_TOOL_CHOICE_TYPES = frozenset({"auto", "any", "tool", "none"})
ANTHROPIC_THINKING_TYPES = frozenset({"adaptive", "enabled", "disabled"})
ANTHROPIC_OUTPUT_CONFIG_KEYS = frozenset({"effort", "format"})
ANTHROPIC_USAGE_KEYS = ("input_tokens", "cache_read_input_tokens", "cache_creation_input_tokens",
                        "ephemeral_5m_input_tokens", "ephemeral_1h_input_tokens", "output_tokens")
ANTHROPIC_RATES_PATH = HERE / "anthropic_api_rates.json"
# Claude subscription through the Claude Agent SDK (Pi extension pi-claude-subscription).
# Claude Code runs inside the container and reaches the proxy through the relay;
# it holds only this synthetic, non-authenticating OAuth credential.
CLAUDE_SDK_PLACEHOLDER = "sk-ant-oat01-" + "D" * 95
CLAUDE_SDK_EXTENSION = "pi-claude-subscription"
DEFAULT_CLAUDE_SDK_BUNDLE = os.path.expanduser("~/.pi/agent/npm/node_modules")
# Claude Code switches for this lane. S0 observed Claude Code resending a request
# after a 500 with x-stainless-retry-count still 0, so retries are disabled here
# and the proxy also refuses every request after an exchange that did not complete.
CLAUDE_SDK_CONTAINER_ENV = {
    "CLAUDE_CODE_MAX_RETRIES": "0", "CLAUDE_CODE_NONSTREAMING_TIMEOUT_RETRIES": "0",
    "CLAUDE_CODE_NO_MODEL_FALLBACK": "1", "CLAUDE_CODE_DISABLE_NONSTREAMING_FALLBACK": "1",
    "CLAUDE_CODE_DISABLE_MODEL_ACCESS_FALLBACK": "1", "CLAUDE_CODE_DISABLE_REFUSAL_RETRY": "1",
    "CLAUDE_CODE_DISABLE_REFUSAL_FALLBACK": "1", "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC": "1",
    "DISABLE_AUTOUPDATER": "1", "DISABLE_TELEMETRY": "1", "DISABLE_ERROR_REPORTING": "1",
}
# Pi's own agent-level retry resends after a provider error (also observed in S0).
CLAUDE_SDK_PI_SETTINGS = {"retry": {"enabled": False, "maxRetries": 0, "provider": {"maxRetries": 0}}}
# Models with exact response-identity evidence for this lane (S0 mock wire; live
# identity is qualified separately). Others need their own evidence first.
CLAUDE_SDK_MODELS = frozenset({"claude-haiku-5-5", "claude-opus-5-5"})
# Wire frozen by the S0 probe (records/pi-container-claude-sdk-s0-20261124).
CLAUDE_SDK_FIELDS = frozenset({"context_management", "max_tokens", "messages", "metadata", "model",
                               "output_config", "stream", "system", "thinking", "tools", "tool_choice"})
CLAUDE_SDK_BETAS = frozenset({
    "oauth-2025-04-20", "interleaved-thinking-2025-05-14", "context-management-2025-06-27",
    "claude-code-20250219", "per-turn-control-2026-07-01", "mid-conversation-tool-changes-2026-07-01",
    "thinking-token-count-2026-05-13", "effort-2025-11-24", "thinking-display-updates-2026-08-18",
    "extended-cache-ttl-2025-04-11"})
CLAUDE_SDK_HEADERS = frozenset({
    "accept", "accept-encoding", "anthropic-beta", "anthropic-dangerous-direct-browser-access",
    "anthropic-version", "authorization", "connection", "content-length", "content-type", "host",
    "user-agent", "x-app", "x-claude-code-session-id", "x-client-app", "x-stainless-arch",
    "x-stainless-lang", "x-stainless-os", "x-stainless-package-version", "x-stainless-retry-count",
    "x-stainless-runtime", "x-stainless-runtime-version", "x-stainless-timeout"})
# Client metadata forwarded upstream after validation; everything else is rebuilt.
CLAUDE_SDK_FORWARD_HEADERS = ("accept", "anthropic-dangerous-direct-browser-access", "user-agent", "x-app",
                              "x-claude-code-session-id", "x-client-app", "x-stainless-arch",
                              "x-stainless-lang", "x-stainless-os", "x-stainless-package-version",
                              "x-stainless-retry-count", "x-stainless-runtime",
                              "x-stainless-runtime-version", "x-stainless-timeout")
CLAUDE_SDK_CONTEXT_EDITS = frozenset({"clear_thinking_20251015"})
CLAUDE_SDK_BILLING_HEADER = "x-anthropic-billing-header:"
CLAUDE_SDK_IDENTITY = "You are a Claude agent, built on Anthropic's Claude Agent SDK."
# Subscription verdict headers. Only these unified statuses release output; an
# overage (extra usage) status is recorded, never used as a fallback.
CLAUDE_SDK_ALLOWED_STATUS = frozenset({"allowed", "allowed_warning"})
# What differs per provider. The openai entry holds the values used before
# Anthropic support, so OpenAI requests, logs and fingerprints are unchanged.
# Public synthetic JWT lets native Pi select its Codex transport without any host identity.
CODEX_PLACEHOLDER = "e30." + base64.b64encode(json.dumps({"https://api.openai.com/auth": {"chatgpt_account_id": "container-placeholder"}}).encode()).decode() + ".placeholder"
PROVIDERS = {
    "openai-codex": {"api": "openai-codex-responses", "route": "/codex/responses",
                     "upstream": "https://chatgpt.com/backend-api/codex", "upstream_env": "ROLE_HOST_PI_UPSTREAM_CODEX",
                     "relay_base": f"http://127.0.0.1:{RELAY_PORT}", "key_env": "OPENAI_CODEX_API_KEY",
                     "placeholder": CODEX_PLACEHOLDER},
    "openai": {"api": "openai-responses", "route": "/v1/responses", "upstream": "https://api.openai.com/v1",
               "upstream_env": "ROLE_HOST_PI_UPSTREAM", "relay_base": f"http://127.0.0.1:{RELAY_PORT}/v1",
               "key_env": "OPENAI_API_KEY", "placeholder": PLACEHOLDER_KEY, "rates": RATES_PATH,
               "rule_keys": ("tier_multiplier", "long_input_threshold", "long_input_multiplier",
                             "long_output_multiplier")},
    "anthropic": {"api": "anthropic-messages", "route": "/v1/messages", "upstream": "https://api.anthropic.com",
                  "upstream_env": "ROLE_HOST_PI_UPSTREAM_ANTHROPIC", "relay_base": f"http://127.0.0.1:{RELAY_PORT}",
                  "key_env": "ANTHROPIC_API_KEY", "placeholder": ANTHROPIC_PLACEHOLDER_KEY,
                  "rates": ANTHROPIC_RATES_PATH, "rule_keys": ("tier_multiplier",)},
    # Claude Code reads ANTHROPIC_BASE_URL and the synthetic credential file, not a key variable.
    "claude-sdk": {"api": "claude-agent-sdk", "route": "/v1/messages", "upstream": "https://api.anthropic.com",
                   "upstream_env": "ROLE_HOST_PI_UPSTREAM_CLAUDE_SDK", "relay_base": f"http://127.0.0.1:{RELAY_PORT}",
                   "key_env": None, "placeholder": CLAUDE_SDK_PLACEHOLDER},
}


def claude_sdk_bundle(root: str = DEFAULT_CLAUDE_SDK_BUNDLE) -> dict:
    """The provider extension and its declared dependency closure under one node_modules root.

    Only these package directories are mounted, so an undeclared import fails
    in the container rather than resolving to some other installed package.
    """
    root = os.path.realpath(root)
    packages, versions = [], {}

    def visit(name: str, optional: bool = False):
        if name in versions:
            return
        path = os.path.join(root, name)
        try:
            with open(os.path.join(path, "package.json")) as fh:
                meta = json.load(fh)
        except FileNotFoundError:
            if optional:
                return
            raise ContainerError("runtime_not_found", f"Claude SDK bundle lacks dependency {name} under {root}")
        except (OSError, ValueError) as e:
            raise ContainerError("runtime_not_found", f"Claude SDK bundle package {name}: {e}")
        if os.path.realpath(path) != path:
            raise ContainerError("runtime_not_found", f"Claude SDK bundle package {name} is a symlink")
        versions[name] = meta.get("version")
        packages.append(path)
        for dep in sorted(meta.get("dependencies") or {}):
            visit(dep)
        for dep in sorted(meta.get("optionalDependencies") or {}):
            visit(dep, True)
        for dep in sorted(meta.get("peerDependencies") or {}):
            # Pi supplies its own packages to extensions. An uninstalled peer stays
            # absent in the container too: nothing outside the closure is mounted.
            if not dep.startswith("@earendil-works/"):
                visit(dep, True)

    visit(CLAUDE_SDK_EXTENSION)
    claude = os.path.join(root, "@anthropic-ai/claude-agent-sdk-linux-x64/claude")
    if not os.access(claude, os.X_OK):
        raise ContainerError("runtime_not_found", f"Claude Code executable missing: {claude}")
    return {"root": root, "extension": os.path.join(root, CLAUDE_SDK_EXTENSION), "claude": claude,
            "packages": sorted(packages), "versions": versions}


def claude_sdk_bundle_identity(bundle: dict) -> dict:
    """Content digest of every mounted bundle package, by path."""
    h = hashlib.sha256()
    for path in bundle["packages"]:
        h.update(f"{path} {tree_digest(path)}\n".encode())
    return {"sha256": h.hexdigest(), "packages": len(bundle["packages"]), "versions": bundle["versions"]}


def codex_account(token: str) -> str:
    """Validate credential shape/lifetime before routing; upstream authenticates signature."""
    try:
        parts = token.split(".")
        if len(parts) != 3 or token.startswith("sk-"):
            raise ValueError()
        claims = json.loads(base64.urlsafe_b64decode(parts[1] + "=" * (-len(parts[1]) % 4)))
        account = claims["https://api.openai.com/auth"]["chatgpt_account_id"]
        if claims.get("iss") != "https://auth.openai.com" or not isinstance(claims.get("exp"), (int, float)) or not claims["exp"] > time.time() + 60:
            raise ValueError()
        if not isinstance(account, str) or not account or any(c in account for c in "\r\n"):
            raise ValueError()
        return account
    except (ValueError, KeyError, TypeError, AttributeError):
        raise ValueError("invalid or expired host Codex subscription credential") from None


def claude_oauth_token(token: str) -> str:
    """Shape check of the host's Claude Code OAuth access token; upstream authenticates it."""
    if (not isinstance(token, str) or not re.fullmatch(r"sk-ant-oat01-[A-Za-z0-9_\-]{20,400}", token)
            or token == CLAUDE_SDK_PLACEHOLDER):
        raise ValueError("invalid host Claude subscription credential")
    return token


def provider_of(model: str) -> str:
    """The provider of a '<provider>/<model-id>' selection."""
    return model.split("/", 1)[0]


def model_definition(path: str, *, provider: str, model_id: str) -> dict:
    """An operator's Pi models.json entry for the pinned model, for a model Pi's
    catalog lacks. role_host writes it under the provider with the relay
    baseUrl, so it may not set an endpoint, headers, key or fallback models."""
    try:
        d = json.loads(Path(path).read_text())
    except (OSError, ValueError) as e:
        raise ContainerError("usage", f"--container-model-file {path}: {e}")
    if not isinstance(d, dict) or d.get("id") != model_id:
        raise ContainerError("usage", f"--container-model-file must define the pinned model {model_id!r}")
    api = PROVIDERS[provider]["api"]
    if d.get("api", api) != api:
        raise ContainerError("usage", f"--container-model-file: provider {provider} uses api {api!r}, not {d.get('api')!r}")
    bad = sorted(set(d) & {"baseUrl", "headers", "apiKey", "provider"})
    if bad:
        raise ContainerError("usage", f"--container-model-file may not set {', '.join(bad)}")
    if isinstance(d.get("compat"), dict) and d["compat"].get("allowedFallbackModels"):
        raise ContainerError("usage", "--container-model-file may not allow fallback models; the model is pinned")
    return d


class ContainerError(Exception):
    def __init__(self, status: str, detail: str):
        super().__init__(detail)
        self.status, self.detail = status, detail


# ------------------------------------------------------------- planning ----

def runtime_paths(root: str = DEFAULT_RUNTIME) -> dict:
    """Pi runtime from a snap-shaped tree: bundled node plus the CLI entry point."""
    resolved = os.path.realpath(root)
    node = os.path.join(root, "node/bin/node")
    cli = os.path.join(root, "lib/pi/packages/coding-agent/dist/cli.js")
    package = os.path.join(resolved, "lib/pi/packages/coding-agent/package.json")
    for path in (os.path.join(resolved, "node/bin/node"), os.path.join(resolved, "lib/pi/packages/coding-agent/dist/cli.js"), package):
        if not os.path.isfile(path):
            raise ContainerError("runtime_not_found", f"Pi runtime file missing: {path}")
    with open(package) as fh:
        version = json.load(fh).get("version")
    return {"root": root, "resolved": resolved, "node": node, "cli": cli, "pi_version": version}


def host_trusted_paths(executables=()) -> list[str]:
    """Code/search roots the host may execute while holding a credential.

    Protect directories, not just today's module files: a new module or PATH
    executable can otherwise shadow a trusted dependency on a later turn.
    Keep lexical and resolved roots so a writable parent cannot replace a
    symlink that names the interpreter, implementation or dependency tree.
    """
    roots = [str(HERE), str(Path(__file__).absolute().parent), sys.executable,
             sys.prefix, sys.base_prefix, *sys.path, *os.get_exec_path()]
    if sys.argv and not sys.argv[0].startswith("-"):
        # Python resolves the initial import directory. Retain the lexical
        # launcher path too, including installed skill-directory symlinks.
        roots.append(str(Path(sys.argv[0]).absolute().parent))
    for command in ("docker", "pi", os.environ.get("ROLE_HOST_DOCKER_EXE"), *executables):
        if command:
            resolved = shutil.which(command)
            if resolved or os.path.dirname(command):
                roots.append(resolved or command)
    return sorted({p for root in roots for p in (os.path.abspath(root), os.path.realpath(root))})


def check_host_write_path(path: str, *, executables=()):
    """Model tools must not change code later executed by the trusted host."""
    path = os.path.realpath(path)
    for root in host_trusted_paths(executables):
        if path == root or path.startswith(root.rstrip("/") + "/") or root.startswith(path.rstrip("/") + "/"):
            raise ContainerError("usage", f"writable mount {path} overlaps trusted host code/search path {root}; "
                                 "run an installed host outside the writable workspace")


def mount_plan(cwd: str, ro=(), rw=(), *, home: str, reserved=(), host_executables=()) -> list[list[str]]:
    """Same-path bind mounts: [[path, "ro"|"rw"], ...]. The working directory is
    mounted read-write unless a listed mount already covers it."""
    entries: dict[str, str] = {}
    home = os.path.realpath(home)

    def add(raw: str, mode: str):
        if not os.path.isabs(raw):
            raise ContainerError("usage", f"mount {raw!r} must be an absolute path")
        path = os.path.realpath(raw)
        if not os.path.exists(path):
            raise ContainerError("usage", f"mount {raw!r} does not exist")
        if any(c in path for c in ",=\n\r"):
            raise ContainerError("usage", f"mount {path!r} contains a character Docker's --mount syntax cannot carry")
        if path in ("/", home):
            raise ContainerError("usage", f"mounting {path} would expose everything; mount the folders the task needs")
        if any(path == p or path.startswith(p + "/") for p in SYSTEM_PREFIXES):
            raise ContainerError("usage", f"{path} is a system path; /usr and /etc are already mounted read-only")
        for r in reserved:
            r = os.path.realpath(r)
            if path == r or path.startswith(r + "/") or r.startswith(path + "/"):
                raise ContainerError("usage", f"{path} overlaps role_host state {r}")
        if mode == "rw":
            check_host_write_path(path, executables=host_executables)
        if entries.get(path, mode) != mode:
            raise ContainerError("usage", f"{path} is listed both read-only and read-write")
        entries[path] = mode

    for p in ro:
        add(p, "ro")
    for p in rw:
        add(p, "rw")
    work = os.path.realpath(cwd)
    if not any(work == p or work.startswith(p + "/") for p in entries):
        add(work, "rw")
    return [[p, entries[p]] for p in sorted(entries)]


def validate_limits(memory: str, pids: int, cpus: str) -> dict:
    if not MEMORY.match(str(memory)):
        raise ContainerError("usage", f"container memory {memory!r} must look like 4g, 512m or 1073741824")
    if not isinstance(pids, int) or pids < 16:
        raise ContainerError("usage", f"container pids {pids!r} must be an integer of at least 16")
    try:
        if float(cpus) <= 0:
            raise ValueError
    except ValueError:
        raise ContainerError("usage", f"container cpus {cpus!r} must be a positive number")
    return {"memory": str(memory), "pids": pids, "cpus": str(cpus)}


def file_sha256(path) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as fh:
        for chunk in iter(lambda: fh.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def tree_digest(root: str) -> str:
    """Digest of every regular file and symlink under root, by relative path."""
    h = hashlib.sha256()
    for dirpath, dirnames, filenames in os.walk(root, followlinks=False):
        dirnames.sort()
        for name in sorted(filenames + [d for d in dirnames if os.path.islink(os.path.join(dirpath, d))]):
            path = os.path.join(dirpath, name)
            rel = os.path.relpath(path, root)
            try:
                if os.path.islink(path):
                    h.update(f"L {rel} {os.readlink(path)}\n".encode())
                elif os.path.isfile(path):
                    h.update(f"F {rel} {file_sha256(path)}\n".encode())
            except OSError as e:
                h.update(f"E {rel} {e.errno}\n".encode())
    return h.hexdigest()


def metadata_digest(root: str) -> str:
    """Digest of path, size, mode and mtime of everything under root: catches
    upgrades and replacements without reading large binaries."""
    h = hashlib.sha256()
    for dirpath, dirnames, filenames in os.walk(root, followlinks=False):
        dirnames.sort()
        for name in sorted(filenames + dirnames):
            path = os.path.join(dirpath, name)
            try:
                st = os.lstat(path)
            except OSError as e:
                h.update(f"E {path} {e.errno}\n".encode())
                continue
            h.update(f"{os.path.relpath(path, root)} {st.st_mode} {st.st_size} {st.st_mtime_ns}\n".encode())
    return h.hexdigest()


def is_squashfs_mount(path: str) -> bool:
    """A snap revision is a read-only squashfs mount: its path identifies its bytes."""
    try:
        for line in Path("/proc/self/mountinfo").read_text().splitlines():
            left, _, right = line.partition(" - ")
            fields = left.split()
            if len(fields) > 4 and fields[4] == path and right.split()[:1] == ["squashfs"]:
                return True
    except OSError:
        pass
    return False


def runtime_identity(resolved: str) -> dict:
    if is_squashfs_mount(resolved):
        return {"kind": "squashfs mount (immutable)", "path": resolved}
    return {"kind": "content digest", "sha256": tree_digest(resolved)}


def userland_identity() -> dict:
    """What the read-only host /usr and /etc give the agent, as far as it can be
    read: the package database (versions of everything apt installed) and the
    metadata of /usr/local. Hand edits under /etc that change no package, and an
    edit under /usr/local that keeps size and mtime, are not covered."""
    out = {}
    try:
        out["dpkg_status_sha256"] = file_sha256("/var/lib/dpkg/status")
    except OSError:
        out["dpkg_status_sha256"] = None
    out["usr_local_metadata_sha256"] = metadata_digest("/usr/local")
    return out


def mount_identities(mounts) -> dict:
    """Device and inode of each mount root: a replaced directory changes them."""
    out = {}
    for path, _mode in mounts:
        try:
            st = os.stat(path)
            out[path] = [st.st_dev, st.st_ino]
        except OSError:
            out[path] = None
    return out


def container_env(env: dict, record_path: str) -> dict:
    """The container's entire environment. Nothing else from the host reaches it."""
    sel = env["selection"]
    p = PROVIDERS[provider_of(sel["model"])]
    out = {"HOME": env["home"], "PATH": "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin",
           "LANG": env["lang"], "USER": env["user"], "LOGNAME": env["user"],
           "PI_CODING_AGENT_DIR": env["agent_dir"], "PI_OFFLINE": "1", "PI_TELEMETRY": "0",
           "WORK_ENGINE_PI_SERVICE_TIER": sel["tier"] or "", "WORK_ENGINE_PI_SERVICE_TIER_MODEL": sel["model"],
           "WORK_ENGINE_PI_SERVICE_TIER_RECORD": record_path}
    if p["key_env"]:
        out[p["key_env"]] = p["placeholder"]
    if provider_of(sel["model"]) == "claude-sdk":
        out.update({"ANTHROPIC_BASE_URL": p["relay_base"], "NO_PROXY": "127.0.0.1,localhost",
                    "PI_CLAUDE_SDK_CLAUDE_PATH": env["claude_sdk"]["claude"],
                    # Forces the extension's Claude prompt over an agent-writable claude-sdk.json.
                    "PI_CLAUDE_SDK_PROMPT": "claude",
                    "WORK_ENGINE_CLAUDE_SDK_CREDENTIAL": "synthetic", **CLAUDE_SDK_CONTAINER_ENV})
    return out


def fingerprint(env: dict) -> dict:
    """The environment's identity: everything that decides what the agent sees or can do."""
    os_release = {}
    try:
        for line in Path("/etc/os-release").read_text().splitlines():
            k, _, v = line.partition("=")
            if k in ("ID", "VERSION_ID"):
                os_release[k] = v.strip('"')
    except OSError:
        pass
    body = {
        "mode": "container", "image": IMAGE, "network": "none (egress proxy only)",
        "userland": list(USERLAND), "userland_identity": userland_identity(),
        "host_os": os_release, "kernel": platform.release(),
        "runtime": env["runtime"]["resolved"], "pi_version": env["runtime"]["pi_version"],
        "runtime_identity": runtime_identity(env["runtime"]["resolved"]),
        "mounts": env["mounts"], "mount_identities": mount_identities(env["mounts"]),
        "limits": env["limits"], "uid_gid": [env["uid"], env["gid"]],
        "selection": env["selection"], "max_requests": env.get("max_requests"),
        "max_output_tokens": env.get("max_output_tokens"), "usd_limit": env.get("usd_limit"),
        "container_env": container_env(env, "<per turn>"),
        "entry_sha256": file_sha256(HERE / "pi_container_entry.py"),
        "extension_sha256": file_sha256(HERE / "pi_service_tier.ts"),
        "pi_container_sha256": file_sha256(HERE / "pi_container.py"),
        "host_isolation": "protected-code-search-paths-and-isolated-runner-v1",
    }
    if env.get("model_definition"):
        body["model_definition"] = env["model_definition"]
    if env.get("claude_sdk"):
        sdk = env["claude_sdk"]
        body["claude_sdk"] = {"root": sdk["root"], "extension": sdk["extension"], "claude": sdk["claude"],
                              "packages": sdk["packages"], "identity": claude_sdk_bundle_identity(sdk),
                              "pi_settings": CLAUDE_SDK_PI_SETTINGS}
    if env.get("codebase_memory"):
        cbm = env["codebase_memory"]
        body["codebase_memory"] = {"binary_sha256": file_sha256(cbm["binary"]), "version": cbm["version"],
                                   "project": cbm["project"], "root_path": cbm["root_path"]}
    canonical = json.dumps(body, sort_keys=True, separators=(",", ":")).encode()
    return {**body, "sha256": hashlib.sha256(canonical).hexdigest()}


def run_profile(env: dict, *, cwd: str, container_dir: str, socket_dir: str | None = None) -> list[str]:
    """Hardened flags and same-path mounts shared by every container this module starts."""
    uid, gid, home = env["uid"], env["gid"], env["home"]
    runtime = env["runtime"]
    argv = ["--network", "none", "--read-only", "--cap-drop", "ALL", "--security-opt", "no-new-privileges",
            "--user", f"{uid}:{gid}", "--pids-limit", str(env["limits"]["pids"]),
            "--memory", env["limits"]["memory"], "--cpus", env["limits"]["cpus"],
            "--tmpfs", "/tmp:rw,nosuid,size=1g",
            "--tmpfs", f"{home}:rw,nosuid,size=1g,uid={uid},gid={gid},mode=0700",
            "--workdir", cwd]
    mounts = [(p, p, True) for p in USERLAND]
    mounts += [(runtime["resolved"], runtime["root"], True),
               (str(HERE / "pi_container_entry.py"), str(HERE / "pi_container_entry.py"), True),
               (str(HERE / "pi_service_tier.ts"), str(HERE / "pi_service_tier.ts"), True),
               (container_dir, container_dir, False)]
    if socket_dir:
        mounts.append((socket_dir, socket_dir, False))
    if env.get("claude_sdk"):
        mounts += [(p, p, True) for p in env["claude_sdk"]["packages"]]
    if env.get("codebase_memory"):
        binary = env["codebase_memory"]["binary"]
        mounts.append((binary, binary, True))
        argv += ["--tmpfs", f"{CBM_RUNTIME}:rw,nosuid,size=16m,uid={uid},gid={gid},mode=0700"]
    mounts += [(p, p, mode == "ro") for p, mode in env["mounts"]]
    for src, dst, readonly in mounts:
        if not readonly:
            # Also covers implicit state/socket mounts and legacy turn specs.
            check_host_write_path(src, executables=env.get("host_executables", ()))
        argv += ["--mount", f"type=bind,src={src},dst={dst}" + (",readonly" if readonly else "")]
    return argv


def docker_argv(spec: dict) -> list[str]:
    """`docker run` arguments for one turn. Contains no credential."""
    env, files = spec["environment"], spec["files"]
    argv = ["run", "-d", "--pull", "never", "--name", spec["container_name"],
            "--label", "work-engine.role-host=pi", "--label", f"work-engine.session={spec['session_id']}",
            "--label", f"work-engine.turn={spec['turn']}"]
    argv += run_profile(env, cwd=spec["cwd"], container_dir=spec["container_dir"], socket_dir=spec["socket_dir"])
    argv += ["--entrypoint", "/usr/bin/python3"]
    for key, value in spec["container_env"].items():
        argv += ["--env", f"{key}={value}"]
    runtime = env["runtime"]
    argv += [IMAGE, str(HERE / "pi_container_entry.py"), spec["socket_path"],
             files["prompt"], files["events"], files["stderr"], files["exit"], "--",
             runtime["node"], runtime["cli"], *spec["pi_argv"]]
    return argv


# ------------------------------------------------------ codebase memory ----

def default_codebase_memory() -> str:
    return os.environ.get("ROLE_HOST_CBM_EXE") or os.path.expanduser("~/.local/bin/codebase-memory-mcp")


def index_codebase_memory(binary: str, repo: str, *, cache: str, runtime: str, env: dict) -> dict:
    """Index exactly `repo` into a cache of its own, on the host, before launch."""
    r = subprocess.run([binary, "cli", "--quiet", "index_repository", "--repo-path", repo, "--index-worktree", "true"],
                       capture_output=True, text=True, timeout=3600, stdin=subprocess.DEVNULL,
                       env={**env, "CBM_CACHE_DIR": cache, "CBM_RUNTIME_DIR": runtime})
    result = None
    for line in reversed(r.stdout.strip().splitlines()):
        try:
            result = json.loads(line)
            break
        except ValueError:
            continue
    if r.returncode != 0 or not isinstance(result, dict) or result.get("status") != "indexed":
        raise ContainerError("codebase_memory_not_established",
                             f"index_repository exit {r.returncode}: {(r.stderr or r.stdout).strip()[-300:]}")
    return result


def check_codebase_memory(docker: str, env: dict, *, cwd: str, container_dir: str) -> dict:
    """Run index_status inside the turn's own container profile; no model, no network."""
    cbm = env["codebase_memory"]
    argv = [docker, "run", "--rm", "--pull", "never", "--label", "work-engine.role-host=pi-check",
            *run_profile(env, cwd=cwd, container_dir=container_dir),
            "--entrypoint", cbm["binary"], "--env", f"HOME={env['home']}",
            "--env", f"CBM_CACHE_DIR={cbm['cache_dir']}", "--env", f"CBM_RUNTIME_DIR={cbm['runtime_dir']}",
            IMAGE, "cli", "index_status", "--project", cbm["project"]]
    r = subprocess.run(argv, capture_output=True, text=True, timeout=300, stdin=subprocess.DEVNULL)
    text = r.stdout
    fields = dict(line.split(": ", 1) for line in text.splitlines() if ": " in line and not line.startswith(" "))
    if r.returncode != 0 or fields.get("status") != "ready" or fields.get("root_path") != cwd:
        raise ContainerError("codebase_memory_not_established",
                             f"in-container index_status exit {r.returncode}: status={fields.get('status')!r} "
                             f"root_path={fields.get('root_path')!r} (expected ready, {cwd}) {r.stderr.strip()[-200:]}")
    return {"status": fields.get("status"), "root_path": fields.get("root_path"), "nodes": fields.get("nodes"),
            "indexed_at": fields.get("indexed_at")}


# ---------------------------------------------------------------- proxy ----

def declared_tool_types(value) -> set:
    """Types of every tool declared in any `tools` list anywhere in the request,
    through namespaces, tool_search_output items and allowed_tools choices."""
    found = set()
    if isinstance(value, dict):
        for key, item in value.items():
            if key == "tools" and isinstance(item, list):
                for tool in item:
                    found.add(tool.get("type") if isinstance(tool, dict) else type(tool).__name__)
            found |= declared_tool_types(item)
    elif isinstance(value, list):
        for item in value:
            found |= declared_tool_types(item)
    return found


def check_request(body, *, model_id: str) -> str | None:
    """A refusal reason, or None when the request may go upstream."""
    if not isinstance(body, dict):
        return "body_not_object"
    unexpected = sorted(set(body) - ALLOWED_FIELDS)
    if unexpected:
        return "unexpected_field:" + ",".join(unexpected)
    if body.get("model") != model_id:
        return "model_not_pinned"
    if body.get("store") is True:
        return "store_refused"
    choice = body.get("tool_choice")
    if choice is not None and not (
            (isinstance(choice, str) and choice in TOOL_CHOICE_STRINGS)
            or (isinstance(choice, dict) and choice.get("type") in TOOL_CHOICE_TYPES)):
        return "tool_choice_refused"
    hosted = declared_tool_types(body) - ALLOWED_TOOL_TYPES
    if hosted:
        return "hosted_tool_refused:" + ",".join(sorted(str(t) for t in hosted))
    return None


def anthropic_tool_definitions(body: dict) -> list:
    """Every tool definition in the request: the top-level list and inline
    tool_addition blocks anywhere in the messages (Pi's native tool changes)."""
    found = list(body.get("tools") or [])

    def walk(value):
        if isinstance(value, dict):
            if value.get("type") == "tool_addition":
                tool = value.get("tool")
                found.append(tool.get("definition") if isinstance(tool, dict) else tool)
            for item in value.values():
                walk(item)
        elif isinstance(value, list):
            for item in value:
                walk(item)

    walk(body.get("messages"))
    return found


def check_anthropic_request(body, *, model_id: str, betas=()) -> str | None:
    """A refusal reason for an Anthropic Messages request, or None when it may go upstream."""
    if not isinstance(body, dict):
        return "body_not_object"
    unexpected = sorted(set(body) - ANTHROPIC_ALLOWED_FIELDS)
    if unexpected:
        return "unexpected_field:" + ",".join(unexpected)
    if body.get("model") != model_id:
        return "model_not_pinned"
    if body.get("stream") is not True:
        return "stream_required"
    refused = sorted(set(betas) - ANTHROPIC_BETAS)
    if refused:
        return "beta_refused:" + ",".join(refused)
    if not isinstance(body.get("tools", []), list):
        return "tools_not_list"
    choice = body.get("tool_choice")
    if choice is not None and not (isinstance(choice, dict) and choice.get("type") in ANTHROPIC_TOOL_CHOICE_TYPES):
        return "tool_choice_refused"
    thinking = body.get("thinking")
    if thinking is not None and not (isinstance(thinking, dict) and thinking.get("type") in ANTHROPIC_THINKING_TYPES):
        return "thinking_refused"
    config = body.get("output_config")
    if config is not None and not (isinstance(config, dict) and set(config) <= ANTHROPIC_OUTPUT_CONFIG_KEYS):
        return "output_config_refused"
    # A custom (client-run) tool has no type or type "custom"; anything else is
    # run by the provider (web search, code execution, MCP, ...).
    hosted = {str(d.get("type")) if isinstance(d, dict) else type(d).__name__
              for d in anthropic_tool_definitions(body)
              if not (isinstance(d, dict) and d.get("type", "custom") == "custom")}
    if hosted:
        return "hosted_tool_refused:" + ",".join(sorted(hosted))
    return None


def claude_sdk_header_refusal(headers: dict, query: str) -> str | None:
    """A refusal reason for the request line and headers Claude Code sent, or None.
    headers: lower-cased names to values (a repeated name is refused by the caller)."""
    if query != "beta=true":
        return "query_refused"
    unexpected = sorted(set(headers) - CLAUDE_SDK_HEADERS)
    if unexpected:
        return "unexpected_header:" + ",".join(unexpected)
    if headers.get("authorization") != "Bearer " + CLAUDE_SDK_PLACEHOLDER:
        return "container_credential_not_placeholder"
    if headers.get("anthropic-version") != ANTHROPIC_VERSION:
        return "anthropic_version_refused"
    if headers.get("content-type", "").split(";")[0].strip() != "application/json":
        return "content_type_refused"
    # Misconfiguration checks of the approved client, not proof of caller identity.
    if not headers.get("user-agent", "").startswith("claude-cli/") or headers.get("x-app") != "cli":
        return "client_not_claude_code"
    if not headers.get("x-client-app", "").startswith(CLAUDE_SDK_EXTENSION + "/"):
        return "client_not_provider_extension"
    # Claude Code also retries above this counter (S0); the proxy halts the turn after
    # any incomplete exchange, so this only refuses a visible client-level retry.
    if headers.get("x-stainless-retry-count") != "0":
        return "retry_refused"
    betas = [b.strip() for b in headers.get("anthropic-beta", "").split(",") if b.strip()]
    refused = sorted(set(betas) - CLAUDE_SDK_BETAS)
    if refused:
        return "beta_refused:" + ",".join(refused)
    if "oauth-2025-04-20" not in betas:
        return "oauth_beta_missing"
    return None


def check_claude_sdk_request(body, *, model_id: str) -> str | None:
    """A refusal reason for a Claude Agent SDK Messages body, or None when it may go upstream."""
    if not isinstance(body, dict):
        return "body_not_object"
    unexpected = sorted(set(body) - CLAUDE_SDK_FIELDS)
    if unexpected:
        return "unexpected_field:" + ",".join(unexpected)
    if body.get("model") != model_id:
        return "model_not_pinned"
    if body.get("stream") is not True:
        return "stream_required"
    tokens = body.get("max_tokens")
    if type(tokens) is not int or tokens < 1:
        return "max_tokens_refused"
    system = body.get("system")
    if not (isinstance(system, list) and len(system) >= 2
            and all(isinstance(b, dict) and b.get("type") == "text" and isinstance(b.get("text"), str) for b in system)):
        return "system_shape_refused"
    head = system[0]["text"]
    if not head.startswith(CLAUDE_SDK_BILLING_HEADER) or "cc_entrypoint=sdk-ts" not in head:
        return "sdk_billing_header_refused"
    if not system[1]["text"].startswith(CLAUDE_SDK_IDENTITY):
        return "sdk_identity_refused"
    messages = body.get("messages")
    if not isinstance(messages, list) or not messages or not all(
            isinstance(m, dict) and m.get("role") in ("user", "assistant", "system") for m in messages):
        return "messages_refused"
    metadata = body.get("metadata")
    if metadata is not None and not (isinstance(metadata, dict) and set(metadata) <= {"user_id"}):
        return "metadata_refused"
    if not isinstance(body.get("tools", []), list):
        return "tools_not_list"
    choice = body.get("tool_choice")
    if choice is not None and not (isinstance(choice, dict) and choice.get("type") in ANTHROPIC_TOOL_CHOICE_TYPES):
        return "tool_choice_refused"
    thinking = body.get("thinking")
    if thinking is not None and not (isinstance(thinking, dict) and thinking.get("type") in ANTHROPIC_THINKING_TYPES):
        return "thinking_refused"
    config = body.get("output_config")
    if config is not None and not (isinstance(config, dict) and set(config) <= ANTHROPIC_OUTPUT_CONFIG_KEYS):
        return "output_config_refused"
    # Provider-fetched sources (URL, file id) would give the networkless container a fetch path.
    source = anthropic_unpriceable(body.get("system")) or anthropic_unpriceable(messages)
    if source:
        return "source_refused:" + source.rsplit(":", 1)[1]
    context = body.get("context_management")
    if context is not None and not (
            isinstance(context, dict) and set(context) <= {"edits"} and isinstance(context.get("edits", []), list)
            and all(isinstance(e, dict) and e.get("type") in CLAUDE_SDK_CONTEXT_EDITS for e in context.get("edits", []))):
        return "context_management_refused"
    # Only Pi's tools (bridged as mcp__pi__*) and Claude Code's deferred-tool
    # placeholder, all client-run; anything typed is run by the provider.
    for d in anthropic_tool_definitions(body):
        if not isinstance(d, dict) or d.get("type", "custom") != "custom":
            return "hosted_tool_refused:" + (str(d.get("type")) if isinstance(d, dict) else type(d).__name__)
        name = d.get("name")
        if not isinstance(name, str) or not (name.startswith("mcp__pi__") or name == "DeferredToolPlaceholder"):
            return "tool_refused"
    return None


def strict_json(raw: bytes):
    """Parse JSON refusing duplicate object keys at any depth and non-JSON constants."""
    def pairs(items):
        out = {}
        for k, v in items:
            if k in out:
                raise ValueError("duplicate key")
            out[k] = v
        return out

    def constant(name):
        raise ValueError(f"non-JSON constant {name}")
    return json.loads(raw, object_pairs_hook=pairs, parse_constant=constant)


def claude_sdk_request_digests(body: dict) -> dict:
    """Receipt digests of what the model was given, without the content itself.
    The billing-header block varies per request and is digested separately."""
    system = body.get("system") or []
    tools = body.get("tools") or []
    return {"system_sha256": hashlib.sha256(_canonical(system[1:])).hexdigest(),
            "billing_block_sha256": hashlib.sha256(_canonical(system[:1])).hexdigest(),
            "tools_sha256": hashlib.sha256(_canonical(tools)).hexdigest(),
            "tool_names": [t.get("name") for t in tools if isinstance(t, dict)],
            "messages": len(body.get("messages") or [])}


def claude_sdk_billing_verdict(headers) -> dict:
    """Subscription verdict from the response headers: allowed, refused or not_established."""
    h = {k.lower(): v for k, v in headers}
    status = h.get("anthropic-ratelimit-unified-status")
    overage = h.get("anthropic-ratelimit-unified-overage-status")
    if status is None:
        verdict = "not_established"
    elif status in CLAUDE_SDK_ALLOWED_STATUS:
        verdict = "allowed"
    else:
        verdict = "refused"
    return {"verdict": verdict, "unified_status": loggable_model(status), "overage_status": loggable_model(overage)}


def anthropic_unpriceable(value) -> str | None:
    """Image or document sources fetched by the provider (url, file id): their bytes are not in the body."""
    if isinstance(value, dict):
        source = value.get("source")
        if value.get("type") in ("image", "document") and isinstance(source, dict) \
                and source.get("type") not in ("base64", "text", "content"):
            return f"usd_limit_unpriceable_input:{value['type']}"
        for item in value.values():
            found = anthropic_unpriceable(item)
            if found:
                return found
    elif isinstance(value, list):
        for item in value:
            found = anthropic_unpriceable(item)
            if found:
                return found
    return None


ANTHROPIC_IDENTITY_HOLD_BYTES = 1 << 20


def anthropic_identity_refusal(event, model_id: str) -> str | None:
    """None when a stream's first event is a message_start naming exactly the
    authorized model; otherwise why the stream is refused. The response's own
    message.model is checked, not the request's."""
    if not isinstance(event, dict) or event.get("type") != "message_start":
        return "response_model_not_first_event"
    message = event.get("message")
    if not isinstance(message, dict):
        return "response_model_malformed"
    if "model" not in message or message["model"] is None:
        return "response_model_missing"
    if not isinstance(message["model"], str) or not message["model"]:
        return "response_model_malformed"
    return None if message["model"] == model_id else "response_model_mismatch"


def loggable_model(value):
    """A response's model field as logged: a short string as is, anything else as a bounded repr."""
    if value is None or (isinstance(value, str) and len(value) <= 200):
        return value
    return repr(value)[:200]


def merge_anthropic_usage(into: dict, raw) -> dict:
    """Fold one message_start or message_delta usage object into the running usage.
    Later values replace earlier ones (message_delta carries cumulative counts)."""
    if not isinstance(raw, dict):
        return into
    for k in ("input_tokens", "cache_read_input_tokens", "cache_creation_input_tokens", "output_tokens"):
        if raw.get(k) is not None:
            into[k] = raw[k]
    split = raw.get("cache_creation")
    if isinstance(split, dict):
        for k in ("ephemeral_5m_input_tokens", "ephemeral_1h_input_tokens"):
            if split.get(k) is not None:
                into[k] = split[k]
    for k in ("service_tier", "server_tool_use"):
        if raw.get(k) is not None:
            into[k] = raw[k]
    return into


# ---------------------------------------------------------- dollar limit ----

def usd_rates(model_id: str, path=None, provider: str = "openai") -> dict:
    """The rate row and pricing rules for one model, or a usage error if it is unpriced."""
    p = PROVIDERS[provider]
    path = path or p["rates"]
    table = json.loads(Path(path).read_text())
    if model_id not in table["models"]:
        raise ContainerError("usage", f"no API rate for {model_id!r} in {Path(path).name}; "
                                      f"priced: {', '.join(sorted(table['models']))}")
    return {"model": model_id, **table["models"][model_id], **{k: table[k] for k in p["rule_keys"]}}


def usage_usd(rates: dict, *, input_tokens, cached, cache_write, output_tokens, multiplier) -> float:
    """Same formula as usage_ledger.codex_usd, times the tier multiplier."""
    long = input_tokens > rates["long_input_threshold"]
    m_in, m_out = (rates["long_input_multiplier"], rates["long_output_multiplier"]) if long else (1, 1)
    base = ((input_tokens - cached - cache_write) * rates["input"] + cached * rates["cached_input"]
            + cache_write * rates["cache_write"]) * m_in + output_tokens * rates["output"] * m_out
    return base / 1e6 * multiplier


def reserve_usd(rates: dict, *, input_bound: int, max_output_tokens: int, multiplier: float) -> float:
    """Worst case: every input token at the dearest input rate (a cache can always miss)."""
    long = input_bound > rates["long_input_threshold"]
    m_in, m_out = (rates["long_input_multiplier"], rates["long_output_multiplier"]) if long else (1, 1)
    dearest = max(rates["input"], rates["cache_write"])
    return (input_bound * dearest * m_in + max_output_tokens * rates["output"] * m_out) / 1e6 * multiplier


def _canonical(value) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":")).encode()


def input_basis(body: dict, items_key: str = "input") -> dict:
    """Byte sizes and a hash chain over the request's input items
    (`input` for Responses, `messages` for Anthropic Messages).

    chains[k] identifies the first k items, so a later request can be checked
    for starting with an earlier request's whole input. A token is at least
    one byte of its text and JSON escaping only adds bytes, so a canonical
    item's byte length bounds its text tokens.
    """
    items = body.get(items_key, [])
    if not isinstance(items, list):
        items = [items]
    rest = _canonical({k: v for k, v in body.items() if k != items_key})
    h = hashlib.sha256(b"").digest()
    chains, sizes = [h.hex()], []
    for item in items:
        raw = _canonical(item)
        sizes.append(len(raw))
        h = hashlib.sha256(h + hashlib.sha256(raw).digest()).digest()
        chains.append(h.hex())
    return {"items": len(items), "chains": chains, "item_bytes": sizes,
            "rest_sha256": hashlib.sha256(rest).hexdigest(), "rest_bytes": len(rest)}


def input_bound(basis: dict, prefix: dict | None) -> tuple[int, str]:
    """Upper bound on the request's input tokens, and how it was found.

    prefix: the provider's input_tokens for an earlier request whose input
    this request starts with. Those tokens already cover its instructions and
    tools; changed ones are counted again in full.
    """
    n, sizes = basis["items"], basis["item_bytes"]
    if prefix and isinstance(prefix.get("input_tokens"), int) and prefix["items"] <= n \
            and basis["chains"][prefix["items"]] == prefix["chain_sha256"]:
        k = prefix["items"]
        bound = prefix["input_tokens"] + sum(sizes[k:]) + ITEM_OVERHEAD_TOKENS * (n - k)
        if basis["rest_sha256"] != prefix["rest_sha256"]:
            bound += basis["rest_bytes"] + ITEM_OVERHEAD_TOKENS
        return bound, "prefix"
    return basis["rest_bytes"] + sum(sizes) + ITEM_OVERHEAD_TOKENS * (n + 1), "bytes"


def unpriceable_input(value) -> str | None:
    """Inputs whose tokens the body's bytes do not bound: files, and images by URL or file id.

    An inline (data:) image is bounded by its base64 length; that an image's
    token count stays below it is assumed, not verified for every detail level.
    """
    if isinstance(value, dict):
        kind = value.get("type")
        if kind == "input_file":
            return "usd_limit_unpriceable_input:input_file"
        if kind == "input_image" and not str(value.get("image_url") or "").startswith("data:"):
            return "usd_limit_unpriceable_input:input_image"
        for item in value.values():
            found = unpriceable_input(item)
            if found:
                return found
    elif isinstance(value, list):
        for item in value:
            found = unpriceable_input(item)
            if found:
                return found
    return None


def usd_totals(rows: list) -> dict:
    """Spend recorded in one proxy log: settled charges plus any reservation never settled."""
    reserved, charged, prefix, floor, bases = {}, 0.0, None, 0.0, {}
    for r in rows:
        if r.get("event") == "usd_reserved":
            reserved[r["rid"]] = r["reserved_usd"]
            bases[r["bound_basis"]] = bases.get(r["bound_basis"], 0) + 1
        elif r.get("event") == "request" and r.get("usd"):
            u = r["usd"]
            reserved.pop(u["rid"], None)
            charged += u["charged_usd"]
            floor = max(floor, u.get("multiplier_floor") or 0.0)
            # Anthropic's input_tokens exclude cache reads and writes; its rows carry the total.
            tokens = u.get("input_total", (r.get("usage") or {}).get("input_tokens"))
            if u.get("charge_basis") == "usage" and r.get("input_basis") and isinstance(tokens, int):
                prefix = {**r["input_basis"], "input_tokens": tokens}
    return {"charged_usd": charged, "unsettled_reserved_usd": sum(reserved.values()), "prefix": prefix,
            "multiplier_floor": floor, "bound_basis": bases,
            "limit_reached": any(r.get("event") == "refused" and str(r.get("reason", "")).startswith("usd_limit")
                                 for r in rows)}


def prior_usd_state(paths) -> dict:
    """Session spend before a new turn, from earlier turns' host-only proxy logs."""
    spent, prefix, floor = 0.0, None, 0.0
    for path in paths:
        try:
            rows = [json.loads(line) for line in Path(path).read_text().splitlines() if line.strip()]
        except (OSError, ValueError) as e:
            raise ContainerError("usd_spend_not_established", f"{path}: {e}")
        t = usd_totals(rows)
        spent += t["charged_usd"] + t["unsettled_reserved_usd"]
        prefix = t["prefix"] or prefix
        floor = max(floor, t["multiplier_floor"])
    return {"spent_before_usd": round(spent, 6), "prefix": prefix, "multiplier_floor": floor}


SUBSCRIPTION_PROXY_BASIS = "api_equivalent_cost_proxy"
SUBSCRIPTION_PROXY_LIMITATION = "Stopping threshold; one admitted response can overshoot. Subscription billing and quota are unknown."


def subscription_proxy_totals(rows, *, malformed=False) -> dict:
    """Replay admissions without discarding known charges when another observation is unknown."""
    admitted, settled, charges = {}, set(), []
    unknown = malformed
    for row in rows:
        if not isinstance(row, dict):
            unknown = True
            continue
        if row.get("event") == "subscription_proxy_admitted":
            rid = row.get("rid")
            if not isinstance(rid, int) or isinstance(rid, bool) or rid in admitted:
                unknown = True
            else:
                admitted[rid] = row
        measurement = row.get("subscription_proxy")
        if row.get("event") in ("request", "upstream_error", "stream_error") and measurement is None:
            unknown = True
        if measurement is not None:
            if not isinstance(measurement, dict):
                unknown = True
                continue
            rid, amount = measurement.get("rid"), measurement.get("cost_proxy_usd")
            if (type(rid) is not int or rid < 1 or rid not in admitted or rid in settled
                    or measurement.get("status") != "valued"
                    or type(amount) not in (int, float) or not 0 <= amount <= sys.float_info.max
                    or not math.isfinite(amount)):
                unknown = True
                continue
            settled.add(rid)
            charges.append(amount)
    unresolved = [rid for rid in admitted if rid not in settled]
    return {"cost_proxy_usd": math.fsum(charges), "accounting_unknown": unknown or bool(unresolved),
            "unresolved_admissions": unresolved, "admitted_requests": len(admitted)}


def subscription_proxy_records(path):
    rows, malformed = [], False
    try:
        data = Path(path).read_bytes()
    except OSError:
        return [], True
    lines = data.split(b"\n")
    malformed = bool(lines.pop())  # A final record without LF is not committed.
    for line in lines:
        if not line.strip():
            continue
        try:
            rows.append(json.loads(line))
        except (ValueError, UnicodeDecodeError):
            malformed = True
    return rows, malformed


def prior_subscription_proxy_state(paths) -> dict:
    totals = [subscription_proxy_totals(rows, malformed=malformed)
              for rows, malformed in (subscription_proxy_records(path) for path in paths)]
    return {"spent_before_usd": math.fsum(t["cost_proxy_usd"] for t in totals),
            "accounting_unknown": any(t["accounting_unknown"] for t in totals)}


CLAUDE_SDK_LIMITATION = ("Claude subscription: usage is what the provider reported per request; quota consumption "
                         "and any overage charge are not measured. A dispatched request without a terminal "
                         "record may have been billed.")


def claude_sdk_turn_summary(path) -> dict:
    """One claude-sdk turn's host proxy log: each admission matched to its disposition.

    evidence: complete (every admission settled), unresolved (an admission without a
    disposition: possibly dispatched, outcome unknown), corrupt (unreadable or
    inconsistent rows) or missing (no log). Neither unresolved nor corrupt means zero.
    """
    rows, malformed = subscription_proxy_records(path)
    if not os.path.exists(path):
        return {"evidence": "missing", "outcome_known": True, "admitted": 0, "completed": 0, "not_completed": 0,
                "unresolved": [], "refused": [], "usage": {}, "billing_verdicts": [], "response_models": [],
                "requests": []}
    admitted, settled, refused, corrupt = {}, {}, [], malformed
    for row in rows:
        if not isinstance(row, dict):
            corrupt = True
            continue
        event, n = row.get("event"), row.get("n")
        if event == "claude_sdk_admitted":
            if type(n) is not int or n in admitted:
                corrupt = True
            else:
                admitted[n] = row
        elif event == "request" and row.get("provider") == "claude-sdk":
            if type(n) is not int or n not in admitted or n in settled:
                corrupt = True
            else:
                settled[n] = row
        elif event == "refused":
            refused.append(row.get("reason"))
    unresolved = sorted(n for n in admitted if n not in settled)
    done = [r for r in settled.values() if r.get("disposition") == "completed"]
    usage = {k: sum(v for r in settled.values() if type(v := (r.get("usage") or {}).get(k)) is int and v >= 0)
             for k in ANTHROPIC_USAGE_KEYS}
    requests = [{"n": n, "disposition": settled[n].get("disposition") if n in settled else "dispatch_unknown",
                 "effort": a.get("effort"), "system_sha256": a.get("system_sha256"),
                 "tools_sha256": a.get("tools_sha256"), "tool_names": a.get("tool_names"),
                 "response_model": (settled.get(n) or {}).get("response_model"),
                 "billing_verdict": ((settled.get(n) or {}).get("billing_verdict") or {}).get("verdict"),
                 "usage_status": (settled.get(n) or {}).get("usage_status")} for n, a in sorted(admitted.items())]
    evidence = "corrupt" if corrupt else "unresolved" if unresolved else "complete"
    return {"evidence": evidence,
            # complete only says every admission has a disposition; a dispatched exchange that
            # did not complete (no response, truncated, unverified, no usage) is still unknown.
            "outcome_known": evidence == "complete" and len(done) == len(settled),
            "admitted": len(admitted), "completed": len(done), "not_completed": len(settled) - len(done),
            "unresolved": unresolved, "refused": refused, "usage": usage,
            "usage_complete": not corrupt and not unresolved and all(
                r.get("usage_status") == "reported" for r in settled.values()),
            "billing_verdicts": sorted({str(((r.get("billing_verdict") or {}).get("verdict"))) for r in settled.values()}),
            "response_models": sorted({str(r.get("response_model")) for r in settled.values() if r.get("response_model")}),
            "requests": requests}


class SubscriptionProxyBudget:
    """Serialized stopping threshold, with no reservation or hard output cap.

    The proxy owns locking and durably writes admission before any upstream effect.
    An unknown exchange keeps its admission unresolved and permanently stops this session.
    """
    max_output_tokens = None

    def __init__(self, *, max_usd, rates, spent_before_usd=0.0):
        self.max_usd, self.rates = max_usd, rates
        self.settled = spent_before_usd
        self.inflight = None
        self.exhausted = spent_before_usd >= max_usd
        self.unknown = False
        self.serial = 0

    def admit(self):
        if self.exhausted:
            return None, "usd_limit: subscription proxy threshold reached or accounting unknown"
        if self.inflight is not None:
            return None, "subscription_proxy_request_in_flight"
        self.serial += 1
        self.inflight = self.serial
        return {"rid": self.serial}, None

    def settle(self, ticket, *, http_status, response, terminal, stream_error):
        usage = response.get("usage")
        reason = None
        if (http_status != 200 or stream_error or terminal not in ("response.completed", "response.done")):
            reason = "exchange_not_completed"
        elif (response.get("model") != self.rates["model"] or not isinstance(response.get("id"), str)
              or not response["id"].strip()):
            reason = "response_identity_not_established"
        elif response.get("status") not in (None, "completed") or response.get("error"):
            reason = "exchange_not_completed"
        elif not isinstance(usage, dict):
            reason = "usage_not_established"
        else:
            counts = [usage.get(k) for k in ("input_tokens", "output_tokens")]
            if not all(type(v) is int and v >= 0 for v in counts):
                reason = "usage_not_established"
        amount, basis = None, None
        if reason is None:
            i, o = counts
            details = usage.get("input_tokens_details")
            if details is not None and not isinstance(details, dict):
                reason = "invalid_cache_usage"
            else:
                details = details or {}
                c, w = details.get("cached_tokens"), details.get("cache_write_tokens")
                if any(v is not None and (type(v) is not int or v < 0 or v > i) for v in (c, w)) or (c or 0) + (w or 0) > i:
                    reason = "invalid_cache_usage"
                else:
                    if c is None or w is None:
                        # Missing optional cache counts: value all input at the dearer rate.
                        c, w = 0, i if self.rates["cache_write"] > self.rates["input"] else 0
                        basis = "conservative_all_input_at_dearest_rate"
                    else:
                        basis = "reported_cache_usage"
                    try:
                        amount = usage_usd(self.rates, input_tokens=i, cached=c, cache_write=w,
                                           output_tokens=o, multiplier=self.rates["tier_multiplier"]["default"])
                        if not math.isfinite(amount) or amount < 0:
                            reason = "valuation_not_established"
                    except (ArithmeticError, ValueError):
                        reason = "valuation_not_established"
        if reason:
            amount = None
            self.unknown = self.exhausted = True
        else:
            self.settled = math.fsum((self.settled, amount))
            self.exhausted = self.settled >= self.max_usd
        return {"rid": ticket["rid"], "status": "unknown" if reason else "valued",
                "cost_proxy_usd": amount, "valuation_basis": basis, "reason": reason,
                "session_cost_proxy_usd": self.settled, "limit_reached": self.exhausted,
                "measurement": SUBSCRIPTION_PROXY_BASIS, "valuation_tier": "default",
                "model": self.rates["model"], "rates_sha256": hashlib.sha256(_canonical(self.rates)).hexdigest(),
                "billing": "unknown", "quota": "unknown", "limitation": SUBSCRIPTION_PROXY_LIMITATION}


class UsdBudget:
    """A session dollar limit, enforced per request. Callers hold the proxy lock.

    reserve() sets aside the request's worst-case cost or refuses; settle()
    replaces it with the charge from the provider's final usage. Reservations
    use the requested tier's multiplier (the dearer of it and default with
    strict_tier). A response served at another tier is charged at that tier,
    and later reservations use at least the dearer of it and default; a
    response reporting no tier is charged at the dearest multiplier, which
    then becomes that floor. After the first refusal every later request is
    refused.
    """

    def __init__(self, *, max_usd: float, rates: dict, tier: str | None, max_output_tokens: int,
                 spent_before_usd: float = 0.0, prefix: dict | None = None,
                 multiplier_floor: float = 0.0, strict_tier: bool = False):
        if (tier or "default") not in rates["tier_multiplier"]:
            raise ContainerError("usage", f"a dollar limit needs a priced tier, not {tier!r}")
        self.max_usd, self.rates, self.tier = max_usd, rates, tier or "default"
        self.max_output_tokens, self.strict_tier = max_output_tokens, strict_tier
        self.settled, self.outstanding = spent_before_usd, 0.0
        self.prefix, self.floor = prefix, multiplier_floor
        self.exhausted, self.seq = False, 0

    def multiplier(self) -> float:
        table = self.rates["tier_multiplier"]
        m = max(table[self.tier], table["default"]) if self.strict_tier else table[self.tier]
        return max(m, self.floor)

    def worst_case(self, bound: int) -> float:
        return reserve_usd(self.rates, input_bound=bound, max_output_tokens=self.max_output_tokens,
                           multiplier=self.multiplier())

    def reserve(self, basis: dict):
        """(ticket, None) when the request may go, or (None, refusal detail)."""
        bound, how = input_bound(basis, self.prefix)
        amount = self.worst_case(bound)
        if self.exhausted or self.settled + self.outstanding + amount > self.max_usd:
            self.exhausted = True
            return None, (f"usd_limit: spent {self.settled + self.outstanding:.4f} + worst case {amount:.4f} "
                          f"> limit {self.max_usd:.4f}")
        self.seq += 1
        self.outstanding += amount
        return {"rid": self.seq, "reserved_usd": amount, "input_bound": bound, "bound_basis": how,
                "basis": basis}, None

    def settle(self, ticket: dict, *, http_status, response: dict) -> dict:
        """Record the charge. http_status None: the upstream exchange failed part way."""
        self.outstanding -= ticket["reserved_usd"]
        table = self.rates["tier_multiplier"]
        usage = response.get("usage") or {}
        details = usage.get("input_tokens_details") or {}
        i, o = usage.get("input_tokens"), usage.get("output_tokens")
        c, w = details.get("cached_tokens") or 0, details.get("cache_write_tokens") or 0
        if http_status is not None and http_status != 200:
            # An error status before any stream: treated as unbilled (assumed, not verified).
            charged, how = 0.0, "error_status"
        elif all(isinstance(x, int) and x >= 0 for x in (i, o, c, w)) and c + w <= i:
            observed = response.get("service_tier")
            m = table[observed] if observed in table else max(table.values())
            if observed != self.tier:
                self.floor = max(self.floor, table["default"], m)
            charged, how = usage_usd(self.rates, input_tokens=i, cached=c, cache_write=w, output_tokens=o,
                                     multiplier=m), "usage"
            basis = ticket["basis"]
            self.prefix = {"items": basis["items"], "chain_sha256": basis["chains"][-1],
                           "rest_sha256": basis["rest_sha256"], "input_tokens": i}
        else:
            charged, how = ticket["reserved_usd"], "reservation"
        self.settled += charged
        return {"rid": ticket["rid"], "reserved_usd": round(ticket["reserved_usd"], 6),
                "charged_usd": round(charged, 6), "charge_basis": how,
                "session_spent_usd": round(self.settled + self.outstanding, 6), "multiplier_floor": self.floor}


class AnthropicUsdBudget(UsdBudget):
    """UsdBudget for Anthropic Messages, with the usage ledger's rates (native_usd).

    No tiers: the proxy refuses service_tier and speed, so a request runs at
    standard rates; a response reporting another tier keeps its reservation.
    Worst case: every input token at the dearest of the input and both
    cache-write rates, plus the output cap (thinking counts as output). Cache
    writes are charged by lifetime; writes the provider did not split by
    lifetime are charged at the dearer rate. No long-context rule is applied,
    as in the ledger.
    """

    def multiplier(self) -> float:
        return 1.0

    def worst_case(self, bound: int) -> float:
        r = self.rates
        dearest = max(r["input"], r["cache_write_5m"], r["cache_write_1h"])
        return (bound * dearest + self.max_output_tokens * r["output"]) / 1e6

    def settle(self, ticket: dict, *, http_status, response: dict) -> dict:
        self.outstanding -= ticket["reserved_usd"]
        u, r = response.get("usage") or {}, self.rates
        i, cr, cw, o = (u.get(k) for k in ("input_tokens", "cache_read_input_tokens",
                                             "cache_creation_input_tokens", "output_tokens"))
        w5, w1 = u.get("ephemeral_5m_input_tokens") or 0, u.get("ephemeral_1h_input_tokens") or 0
        total = None
        if http_status is not None and http_status != 200:
            charged, how = 0.0, "error_status"  # assumed unbilled, not verified
        elif http_status is not None \
                and all(isinstance(x, int) and not isinstance(x, bool) and x >= 0 for x in (i, cr, cw, o, w5, w1)) \
                and w5 + w1 <= cw and u.get("service_tier") in (None, "standard"):
            # http_status None: the stream ended before message_stop, so the
            # usage seen so far (message_start) understates output; keep the reservation.
            charged = (i * r["input"] + cr * r["cache_read"] + w5 * r["cache_write_5m"] + w1 * r["cache_write_1h"]
                       + (cw - w5 - w1) * max(r["cache_write_5m"], r["cache_write_1h"]) + o * r["output"]) / 1e6
            how, total = "usage", i + cr + cw
            basis = ticket["basis"]
            self.prefix = {"items": basis["items"], "chain_sha256": basis["chains"][-1],
                           "rest_sha256": basis["rest_sha256"], "input_tokens": total}
        else:
            charged, how = ticket["reserved_usd"], "reservation"
        self.settled += charged
        out = {"rid": ticket["rid"], "reserved_usd": round(ticket["reserved_usd"], 6),
               "charged_usd": round(charged, 6), "charge_basis": how,
               "session_spent_usd": round(self.settled + self.outstanding, 6), "multiplier_floor": self.floor}
        if total is not None:
            out["input_total"] = total
        return out


BUDGETS = {"openai": UsdBudget, "anthropic": AnthropicUsdBudget}


class EgressProxy:
    """HTTP/1.1 on a Unix socket; one upstream request per accepted request."""

    def __init__(self, socket_path: str, *, upstream: str, key: str, model_id: str,
                 tier: str | None, log_path: str, max_requests: int | None = None,
                 max_output_tokens: int | None = None, budget: UsdBudget | None = None,
                 on_usd_exhausted=None,
                 max_connections: int = MAX_CONNECTIONS, max_body: int = MAX_BODY_BYTES,
                 socket_timeout: float = SOCKET_TIMEOUT, provider: str = "openai",
                 admission_wait: float = ADMISSION_WAIT_SECONDS):
        if provider == "openai-codex" and (tier or max_output_tokens is not None):
            raise ValueError("Codex subscription takes no service tier, API budget or output cap")
        if budget is not None and max_output_tokens != budget.max_output_tokens:
            raise ValueError("a dollar limit needs the proxy's output cap to be the budget's")
        if provider not in PROVIDERS:
            raise ValueError(f"no egress route for provider {provider!r}")
        if budget is not None and type(budget) is not ({**BUDGETS, "openai-codex": SubscriptionProxyBudget})[provider]:
            raise ValueError(f"wrong budget type for {provider}")
        if provider == "openai-codex":
            self.account = codex_account(key)
        if provider == "anthropic" and tier:
            raise ValueError("the anthropic route sets no service tier")
        if provider == "claude-sdk":
            if tier or max_output_tokens is not None or budget is not None:
                raise ValueError("Claude subscription takes no service tier, dollar budget or output cap")
            if type(max_requests) is not int or max_requests < 1:
                raise ValueError("Claude subscription needs a finite positive request cap")
            if model_id not in CLAUDE_SDK_MODELS:
                raise ValueError(f"no Claude subscription identity evidence for {model_id!r}")
            claude_oauth_token(key)
        # Claude subscription: set after any exchange that did not complete; every
        # later request in the turn is refused (no replay of an uncertain request).
        self.halted = None
        self.on_halt = on_usd_exhausted
        self.inflight = None
        self.provider, self.route = provider, PROVIDERS[provider]["route"]
        self.socket_path, self.key, self.model_id, self.tier = socket_path, key, model_id, tier
        self.upstream = urllib.parse.urlsplit(upstream)
        self.log_path, self.max_requests = log_path, max_requests
        self.max_output_tokens, self.budget, self.on_usd_exhausted = max_output_tokens, budget, on_usd_exhausted
        self.max_connections, self.max_body, self.socket_timeout = max_connections, max_body, socket_timeout
        self.accepted = 0
        self.dropped_connections = 0
        self.lock = threading.Lock()
        # Signalled under self.lock whenever a subscription admission is released.
        self.settled = threading.Condition(self.lock)
        self.admission_wait = admission_wait
        self.server = None

    def log(self, row: dict):
        with self.lock:
            self._log_locked(row)

    def _log_locked(self, row, *, durable=False):
        with open(self.log_path, "a") as fh:
            fh.write(json.dumps({"at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()), **row}) + "\n")
            if durable:
                fh.flush()
                os.fsync(fh.fileno())

    def start(self):
        proxy = self
        # Exists from the start, so a turn with no requests still leaves a
        # readable (empty) log for the next turn's spend total.
        open(self.log_path, "a").close()
        if isinstance(self.budget, SubscriptionProxyBudget) or self.provider == "claude-sdk":
            # Make the log's directory entry durable before an admitted request can leave.
            fd = os.open(str(Path(self.log_path).parent), os.O_RDONLY | os.O_DIRECTORY)
            try:
                os.fsync(fd)
            finally:
                os.close(fd)

        class Handler(BaseHTTPRequestHandler):
            # HTTP/1.0 framing: the streamed body ends when the connection closes.
            timeout = proxy.socket_timeout  # bounds slow headers and bodies

            def log_message(self, *args):
                pass

            def refuse(self, code: int, reason: str):
                proxy.log({"event": "refused", "method": self.command, "path": self.path, "http_status": code, "reason": reason})
                if proxy.provider == "claude-sdk":
                    # Before the reply: a client reacting to it must already find the turn halted.
                    self.halt(reason)
                payload = json.dumps({"error": {"message": f"role_host egress proxy: {reason}", "type": "egress_refused"}}).encode()
                self.send_response(code)
                self.send_header("content-type", "application/json")
                self.send_header("content-length", str(len(payload)))
                self.send_header("connection", "close")
                self.end_headers()
                self.wfile.write(payload)
                self.close_connection = True

            def forward_anthropic(self, body, n, release):
                """Anthropic Messages: check, cap, reserve, forward with the host key, stream back, log usage."""
                query = urllib.parse.urlsplit(self.path).query
                betas = [b.strip() for b in (self.headers.get("anthropic-beta") or "").split(",") if b.strip()]
                reason = "query_refused" if query not in ("", "beta=true") else \
                    check_anthropic_request(body, model_id=proxy.model_id, betas=betas)
                if reason is None and proxy.max_output_tokens is not None:
                    asked = body.get("max_tokens")
                    if not (isinstance(asked, int) and not isinstance(asked, bool)
                            and 0 < asked < proxy.max_output_tokens):
                        body["max_tokens"] = proxy.max_output_tokens
                    thinking = body.get("thinking") or {}
                    budget_tokens = thinking.get("budget_tokens")
                    if thinking.get("type") == "enabled" and not (
                            isinstance(budget_tokens, int) and budget_tokens < body["max_tokens"]):
                        reason = "thinking_budget_not_below_output_cap"
                if reason is None and proxy.budget is not None:
                    reason = anthropic_unpriceable(body.get("system")) or anthropic_unpriceable(body.get("messages"))
                if reason:
                    release()
                    return self.refuse(403, reason)
                ticket = None
                if proxy.budget is not None:
                    basis = input_basis(body, items_key="messages")
                    with proxy.lock:
                        ticket, refusal = proxy.budget.reserve(basis)
                    if ticket is None:
                        release()
                        self.refuse(429, refusal)
                        if proxy.on_usd_exhausted:
                            proxy.on_usd_exhausted(refusal)
                        return
                    proxy.log({"event": "usd_reserved", "n": n, "rid": ticket["rid"],
                               "reserved_usd": round(ticket["reserved_usd"], 6),
                               "input_bound": ticket["input_bound"], "bound_basis": ticket["bound_basis"]})

                def settle(http_status, response):
                    if ticket is None:
                        return {}
                    with proxy.lock:
                        usd = proxy.budget.settle(ticket, http_status=http_status, response=response)
                    b = ticket["basis"]
                    return {"usd": usd, "input_basis": {"items": b["items"], "chain_sha256": b["chains"][-1],
                                                        "rest_sha256": b["rest_sha256"]}}

                data = json.dumps(body).encode()
                # Headers are rebuilt, not copied: the container's x-api-key and
                # authorization never go upstream, and only allowed betas do.
                headers = {"content-type": "application/json", "anthropic-version": ANTHROPIC_VERSION,
                           "x-api-key": proxy.key, "content-length": str(len(data))}
                if betas:
                    headers["anthropic-beta"] = ",".join(betas)
                up = proxy.upstream
                conn_cls = http.client.HTTPSConnection if up.scheme == "https" else http.client.HTTPConnection
                started = time.time()
                try:
                    conn = conn_cls(up.hostname, up.port, timeout=900)
                    conn.request("POST", up.path.rstrip("/") + proxy.route + (f"?{query}" if query else ""),
                                 data, headers)
                    resp = conn.getresponse()
                except OSError as e:
                    proxy.log({"event": "upstream_error", "provider": "anthropic", "n": n, "detail": str(e)[:300],
                               **settle(None, {})})
                    return self.refuse(502, "upstream unreachable")
                replied = []

                def reply_headers():
                    replied.append(True)
                    self.send_response(resp.status)
                    for k, v in resp.getheaders():
                        if k.lower() not in ("transfer-encoding", "connection", "content-length", "keep-alive"):
                            self.send_header(k, v)
                    self.send_header("connection", "close")
                    self.end_headers()

                # A 200 stream is held, line by line, until its first event (pings
                # aside) is a message_start naming exactly the authorized model;
                # nothing from an unverified model reaches the container. Error
                # statuses carry no model output and pass through as before.
                streaming = resp.status == 200
                if not streaming:
                    reply_headers()
                usage, buf, head = {}, b"", b""
                message, terminal, stop_reason, error_type, stream_error = {}, None, None, None, None
                verified, identity_refusal, held, held_bytes = not streaming, None, [], 0
                try:
                    while identity_refusal is None:
                        chunk = resp.read1(65536)
                        if not chunk:
                            break
                        if not streaming:
                            self.wfile.write(chunk)
                            self.wfile.flush()
                            if len(head) < 2000:
                                head += chunk[: 2000 - len(head)]
                        buf += chunk
                        *lines, buf = buf.split(b"\n")
                        for line in lines:
                            event, kind = None, None
                            if line.startswith(b"data:"):
                                try:
                                    event = json.loads(line[5:].strip())
                                except ValueError:
                                    event = None
                                kind = event.get("type") if isinstance(event, dict) else None
                                if streaming and not verified and kind != "ping":
                                    identity_refusal = anthropic_identity_refusal(event, proxy.model_id)
                                    verified = identity_refusal is None
                                elif streaming and kind == "message_start":
                                    identity_refusal = "response_model_restated"
                            if streaming and identity_refusal is None:
                                held.append(line + b"\n")
                                held_bytes += len(line) + 1
                            if isinstance(event, dict):
                                if kind == "message_start" and isinstance(event.get("message"), dict):
                                    message = event["message"]
                                    merge_anthropic_usage(usage, message.get("usage"))
                                elif kind == "message_delta":
                                    merge_anthropic_usage(usage, event.get("usage"))
                                    stop_reason = (event.get("delta") or {}).get("stop_reason") or stop_reason
                                elif kind == "message_stop":
                                    terminal = kind
                                elif kind == "error":
                                    error_type = (event.get("error") or {}).get("type") or "error"
                            if identity_refusal:
                                break
                        if streaming and verified and identity_refusal is None and held:
                            if not replied:
                                reply_headers()
                            self.wfile.write(b"".join(held))
                            self.wfile.flush()
                            held, held_bytes = [], 0
                        elif streaming and not verified and held_bytes + len(buf) > ANTHROPIC_IDENTITY_HOLD_BYTES:
                            identity_refusal = "response_model_not_established"
                    if streaming and verified and identity_refusal is None and buf:
                        self.wfile.write(buf)
                        self.wfile.flush()
                except OSError as e:
                    # The container went away (cancellation) or the upstream broke.
                    stream_error = str(e)[:300]
                    proxy.log({"event": "stream_error", "provider": "anthropic", "n": n, "detail": stream_error})
                finally:
                    conn.close()
                if streaming and not verified and identity_refusal is None:
                    identity_refusal = "response_model_not_established"
                response_model = loggable_model(message.get("model"))
                if identity_refusal:
                    # Fail closed: no retry, no fallback; the reservation stays charged below.
                    proxy.log({"event": "response_model_refused", "provider": "anthropic", "n": n,
                               "reason": identity_refusal, "authorized_model": proxy.model_id,
                               "response_model": response_model, "response_id": message.get("id"),
                               "verified_bytes_forwarded": bool(replied), "detail": stream_error})
                proxy.log({
                    "event": "request", "provider": "anthropic", "n": n, "http_status": resp.status,
                    "model": body.get("model"), "requested_tier": None, "observed_tier": usage.get("service_tier"),
                    "terminal": terminal, "response_id": message.get("id"), "response_model": response_model,
                    **({"model_mismatch": True} if response_model and response_model != proxy.model_id else {}),
                    **({"response_model_check": identity_refusal or "verified"} if streaming else {}),
                    "stop_reason": stop_reason, "thinking": (body.get("thinking") or {}).get("type"),
                    "effort": (body.get("output_config") or {}).get("effort"), "betas": betas,
                    "usage": {k: usage.get(k) for k in ANTHROPIC_USAGE_KEYS},
                    **({"server_tool_use": usage["server_tool_use"]} if usage.get("server_tool_use") else {}),
                    **({"stream_error_type": error_type} if error_type else {}),
                    "seconds": round(time.time() - started, 3),
                    **({"error_head": head.decode("utf-8", "replace")} if resp.status != 200 else {}),
                    **settle(None if identity_refusal else
                             resp.status if terminal is not None or resp.status != 200 else None, {"usage": usage}),
                })
                self.close_connection = True
                if identity_refusal and not replied:
                    try:
                        self.refuse(502, identity_refusal)
                    except OSError:
                        pass

            def halt(self, reason):
                with proxy.lock:
                    first = proxy.halted is None
                    if first:
                        proxy.halted = reason
                if first and proxy.on_halt:
                    proxy.on_halt(f"claude_sdk_turn_halted: {reason}")

            def refuse_claude_sdk(self, code, reason):
                """Refuse and halt (refuse() halts every claude-sdk refusal)."""
                self.refuse(code, reason)

            def forward_claude_sdk(self, raw, body, n, release):
                """Claude subscription: validate, admit durably, forward the same bytes with the
                host bearer, hold output until model and billing verdict are established."""
                names = [k.lower() for k in self.headers.keys()]
                headers = {k.lower(): v for k, v in self.headers.items()}
                query = urllib.parse.urlsplit(self.path).query
                reason = ("duplicate_header" if len(names) != len(set(names)) else
                          claude_sdk_header_refusal(headers, query)
                          or check_claude_sdk_request(body, model_id=proxy.model_id))
                if reason:
                    release()
                    return self.refuse_claude_sdk(403, reason)
                digests = claude_sdk_request_digests(body)
                admitted = {"event": "claude_sdk_admitted", "provider": "claude-sdk", "auth_lane": "subscription",
                            "n": n, "model": proxy.model_id, "body_sha256": hashlib.sha256(raw).hexdigest(),
                            "effort": (body.get("output_config") or {}).get("effort"),
                            "thinking": (body.get("thinking") or {}).get("type"),
                            "betas": sorted(b.strip() for b in headers.get("anthropic-beta", "").split(",") if b.strip()),
                            **digests}
                with proxy.lock:
                    # Claude Code sends its next request as soon as it reads message_stop,
                    # while this proxy may still be writing the previous disposition.
                    proxy.settled.wait_for(lambda: proxy.inflight is None or proxy.halted is not None,
                                           timeout=proxy.admission_wait)
                    refusal = (f"turn_halted: {proxy.halted}" if proxy.halted is not None else
                               "request_in_flight" if proxy.inflight is not None else None)
                    if refusal is None:
                        try:
                            proxy._log_locked(admitted, durable=True)
                            proxy.inflight = n
                        except OSError:
                            refusal = "admission_persistence_failed"
                if refusal:
                    release()
                    return self.refuse_claude_sdk(429, refusal)

                def finish(row, *, completed):
                    reason, newly = row.get("disposition") or "exchange_not_completed", False
                    with proxy.lock:
                        try:
                            proxy._log_locked(row, durable=True)
                        except OSError:
                            completed, reason = False, "disposition_persistence_failed"
                        finally:
                            # Halt before waking waiters: a queued request must find the turn halted.
                            if not completed and proxy.halted is None:
                                proxy.halted, newly = reason, True
                            proxy.inflight = None
                            proxy.settled.notify_all()
                    if newly and proxy.on_halt:
                        proxy.on_halt(f"claude_sdk_turn_halted: {reason}")

                up = proxy.upstream
                out = {"content-type": "application/json", "anthropic-version": ANTHROPIC_VERSION,
                       "anthropic-beta": headers["anthropic-beta"], "authorization": f"Bearer {proxy.key}",
                       "accept-encoding": "identity", "content-length": str(len(raw))}
                out.update({k: headers[k] for k in CLAUDE_SDK_FORWARD_HEADERS if k in headers})
                conn_cls = http.client.HTTPSConnection if up.scheme == "https" else http.client.HTTPConnection
                started = time.time()
                base = {"event": "request", "provider": "claude-sdk", "auth_lane": "subscription", "n": n,
                        "model": proxy.model_id, "billing": "unknown", "quota": "unknown"}
                try:
                    conn = conn_cls(up.hostname, up.port, timeout=900)
                    conn.request("POST", up.path.rstrip("/") + proxy.route + "?beta=true", raw, out)
                    resp = conn.getresponse()
                except (OSError, http.client.HTTPException) as e:
                    # The request may have reached the provider: dispatched, outcome unknown.
                    finish({**base, "disposition": "dispatched_no_response", "detail": type(e).__name__},
                           completed=False)
                    return self.refuse(502, "upstream unreachable")
                verdict = claude_sdk_billing_verdict(resp.getheaders())
                if resp.status != 200:
                    head = resp.read(4096)
                    conn.close()
                    try:
                        error_type = (json.loads(head).get("error") or {}).get("type")
                    except (ValueError, AttributeError):
                        error_type = None
                    # Upstream error bodies are neither relayed nor logged: they can carry account detail.
                    finish({**base, "disposition": "upstream_error", "http_status": resp.status,
                            "error_type": loggable_model(error_type), "billing_verdict": verdict,
                            "seconds": round(time.time() - started, 3)}, completed=False)
                    return self.refuse(502, f"upstream_status_{resp.status}")
                usage, buf, held, held_bytes = {}, b"", [], 0
                message, terminal, stop_reason, error_type, stream_error = {}, None, None, None, None
                refusal = None if verdict["verdict"] == "allowed" else f"billing_{verdict['verdict']}"
                verified, replied = False, False
                try:
                    while refusal is None:
                        chunk = resp.read1(65536)
                        if not chunk:
                            break
                        buf += chunk
                        *lines, buf = buf.split(b"\n")
                        for line in lines:
                            event, kind = None, None
                            if line.startswith(b"data:"):
                                try:
                                    event = json.loads(line[5:].strip())
                                except ValueError:
                                    event = None
                                kind = event.get("type") if isinstance(event, dict) else None
                                if not verified and kind != "ping":
                                    refusal = anthropic_identity_refusal(event, proxy.model_id)
                                    verified = refusal is None
                                elif kind == "message_start":
                                    refusal = "response_model_restated"
                                if kind == "message_start" and isinstance(event.get("message"), dict):
                                    message = event["message"]
                                    merge_anthropic_usage(usage, message.get("usage"))
                                elif kind == "message_delta":
                                    merge_anthropic_usage(usage, event.get("usage"))
                                    stop_reason = (event.get("delta") or {}).get("stop_reason") or stop_reason
                                elif kind == "message_stop":
                                    terminal = kind
                                elif kind == "error":
                                    error_type = (event.get("error") or {}).get("type") or "error"
                            if refusal:
                                break
                            held.append(line + b"\n")
                            held_bytes += len(line) + 1
                        if refusal is None and verified and held:
                            if not replied:
                                replied = True
                                self.send_response(200)
                                self.send_header("content-type", "text/event-stream")
                                self.send_header("connection", "close")
                                self.end_headers()
                            self.wfile.write(b"".join(held))
                            self.wfile.flush()
                            held, held_bytes = [], 0
                        elif not verified and held_bytes + len(buf) > ANTHROPIC_IDENTITY_HOLD_BYTES:
                            refusal = "response_model_not_established"
                    if refusal is None and verified and buf:
                        self.wfile.write(buf)
                        self.wfile.flush()
                except (OSError, http.client.HTTPException) as e:
                    stream_error = type(e).__name__
                finally:
                    conn.close()
                if refusal is None and not verified:
                    refusal = "response_model_not_established"
                counts = [usage.get(k) for k in ("input_tokens", "output_tokens")]
                usage_ok = all(type(v) is int and v >= 0 for v in counts) and all(
                    usage.get(k) is None or (type(usage.get(k)) is int and usage[k] >= 0)
                    for k in ANTHROPIC_USAGE_KEYS)
                completed = (refusal is None and terminal == "message_stop" and stream_error is None
                             and error_type is None and usage_ok)
                disposition = ("completed" if completed else refusal or ("stream_error" if stream_error else
                               "stream_error_event" if error_type else "terminal_missing" if terminal is None
                               else "usage_not_established"))
                finish({**base, "disposition": disposition, "http_status": resp.status,
                        "billing_verdict": verdict, "response_id": loggable_model(message.get("id")),
                        "response_model": loggable_model(message.get("model")),
                        "response_model_check": "verified" if verified and refusal != "response_model_restated"
                        else refusal, "terminal": terminal, "stop_reason": loggable_model(stop_reason),
                        "usage": {k: usage.get(k) for k in ANTHROPIC_USAGE_KEYS}, "usage_status":
                        "reported" if usage_ok else "not_established", "output_released": replied,
                        **({"stream_error": stream_error} if stream_error else {}),
                        **({"stream_error_type": loggable_model(error_type)} if error_type else {}),
                        "seconds": round(time.time() - started, 3)}, completed=completed)
                self.close_connection = True
                if refusal and not replied:
                    try:
                        self.refuse(502, refusal)
                    except OSError:
                        pass

            def do_GET(self):
                if proxy.provider == "claude-sdk":
                    return self.refuse_claude_sdk(403, f"only POST {proxy.route} is allowed")
                self.refuse(403, f"only POST {proxy.route} is allowed")

            do_PUT = do_DELETE = do_PATCH = do_GET

            def do_POST(self):
                if self.path.split("?", 1)[0] != proxy.route:
                    if proxy.provider == "claude-sdk":
                        return self.refuse_claude_sdk(403, f"only POST {proxy.route} is allowed")
                    return self.refuse(403, f"only POST {proxy.route} is allowed")
                if proxy.provider == "claude-sdk" and proxy.halted is not None:
                    return self.refuse(429, f"turn_halted: {proxy.halted}")
                if self.headers.get("transfer-encoding"):
                    return self.refuse(411, "chunked or encoded request bodies are not accepted")
                length = self.headers.get("content-length", "")
                if not length.isdigit() or not 0 < int(length) <= proxy.max_body:
                    return self.refuse(413 if length.isdigit() else 411,
                                       f"content-length must be between 1 and {proxy.max_body}")
                # Reserve a slot under the cap before any body is buffered; a
                # request refused below gives its slot back.
                with proxy.lock:
                    spent_out = proxy.budget is not None and proxy.budget.exhausted
                    if spent_out or (proxy.max_requests is not None and proxy.accepted >= proxy.max_requests):
                        n = None
                    else:
                        proxy.accepted += 1
                        n = proxy.accepted
                if spent_out:
                    return self.refuse(429, "usd_limit: reached earlier in this turn")
                if n is None:
                    return self.refuse(429, f"request cap {proxy.max_requests} reached")

                def release():
                    with proxy.lock:
                        proxy.accepted -= 1

                try:
                    raw = self.rfile.read(int(length))
                except OSError:
                    release()
                    if proxy.provider == "claude-sdk":
                        self.halt("body not received in time")
                    return proxy.log({"event": "refused", "reason": "body not received in time"})
                if len(raw) != int(length):
                    release()
                    return self.refuse(400, "body shorter than content-length")
                try:
                    encoding = self.headers.get("content-encoding", "identity")
                    if proxy.provider == "openai-codex" and encoding == "zstd":
                        import zstandard
                        frame = zstandard.get_frame_parameters(raw)
                        if (frame.window_size > proxy.max_body or
                                frame.content_size not in (zstandard.CONTENTSIZE_UNKNOWN, zstandard.CONTENTSIZE_ERROR)
                                and frame.content_size > proxy.max_body):
                            raise ValueError()
                        # One native frame only. Reject concatenated/skippable trailing frames;
                        # never let an unrestricted decoder process a later unvalidated window.
                        raw = zstandard.ZstdDecompressor().decompress(
                            raw, max_output_size=proxy.max_body, allow_extra_data=False)
                    elif encoding != "identity":
                        raise ValueError()
                    # claude-sdk forwards the accepted bytes, so the parse it validates must be the
                    # only reading: no duplicate keys (any depth), no NaN/Infinity.
                    body = strict_json(raw) if proxy.provider == "claude-sdk" else json.loads(raw)
                except Exception:
                    # Do not retain compressed body/decoder errors: they may contain private data.
                    release()
                    return self.refuse(400, "body is not JSON")
                if proxy.provider == "anthropic":
                    return self.forward_anthropic(body, n, release)
                if proxy.provider == "claude-sdk":
                    return self.forward_claude_sdk(raw, body, n, release)
                reason = check_request(body, model_id=proxy.model_id)
                if reason:
                    release()
                    return self.refuse(403, reason)
                if proxy.provider == "openai-codex" and (body.get("stream") is not True or body.get("store") is not False
                                                          or "service_tier" in body or "max_output_tokens" in body):
                    release()
                    return self.refuse(403, "subscription_request_fields_refused")
                if proxy.tier:
                    body["service_tier"] = proxy.tier
                else:
                    body.pop("service_tier", None)
                if proxy.max_output_tokens is not None:
                    asked = body.get("max_output_tokens")
                    if not (isinstance(asked, int) and not isinstance(asked, bool)
                            and 0 < asked < proxy.max_output_tokens):
                        body["max_output_tokens"] = proxy.max_output_tokens
                ticket = None
                subscription_budget = isinstance(proxy.budget, SubscriptionProxyBudget)
                if subscription_budget:
                    with proxy.lock:
                        # Serialize, do not refuse: wait for the previous exchange to settle.
                        proxy.settled.wait_for(lambda: proxy.budget.inflight is None or proxy.budget.exhausted,
                                               timeout=proxy.admission_wait)
                        ticket, refusal = proxy.budget.admit()
                        if ticket is not None:
                            try:
                                proxy._log_locked({"event": "subscription_proxy_admitted", "n": n,
                                                   "rid": ticket["rid"], "model": proxy.model_id}, durable=True)
                            except OSError:
                                proxy.budget.unknown = proxy.budget.exhausted = True
                                ticket, refusal = None, "usd_limit: admission persistence failed"
                                if proxy.on_usd_exhausted:
                                    proxy.on_usd_exhausted(refusal)
                    if ticket is None:
                        release()
                        self.refuse(429, refusal)
                        if proxy.budget.exhausted and proxy.on_usd_exhausted:
                            proxy.on_usd_exhausted(refusal)
                        return
                elif proxy.budget is not None:
                    reason = unpriceable_input(body.get("input"))
                    if reason:
                        release()
                        return self.refuse(403, reason)
                    basis = input_basis(body)
                    with proxy.lock:
                        ticket, refusal = proxy.budget.reserve(basis)
                    if ticket is None:
                        release()
                        self.refuse(429, refusal)
                        if proxy.on_usd_exhausted:
                            proxy.on_usd_exhausted(refusal)
                        return
                    proxy.log({"event": "usd_reserved", "n": n, "rid": ticket["rid"],
                               "reserved_usd": round(ticket["reserved_usd"], 6),
                               "input_bound": ticket["input_bound"], "bound_basis": ticket["bound_basis"]})

                def settle(http_status, response):
                    if ticket is None or subscription_budget:
                        return {}
                    with proxy.lock:
                        usd = proxy.budget.settle(ticket, http_status=http_status, response=response)
                    b = ticket["basis"]
                    return {"usd": usd, "input_basis": {"items": b["items"], "chain_sha256": b["chains"][-1],
                                                        "rest_sha256": b["rest_sha256"]}}

                def finish(row, *, http_status=None, response=None, terminal=None, stream_error=False):
                    if not subscription_budget:
                        proxy.log(row)
                        return
                    with proxy.lock:
                        try:
                            measurement = proxy.budget.settle(ticket, http_status=http_status, response=response or {},
                                                              terminal=terminal, stream_error=stream_error)
                            try:
                                proxy._log_locked({**row, "subscription_proxy": measurement}, durable=True)
                            except OSError:
                                proxy.budget.unknown = proxy.budget.exhausted = True
                                raise
                            finally:
                                if proxy.budget.exhausted and proxy.on_usd_exhausted:
                                    proxy.on_usd_exhausted("subscription proxy threshold reached or accounting unknown")
                            proxy.budget.inflight = None
                        finally:
                            # Wake waiters even on failure: they re-check exhaustion and refuse.
                            proxy.settled.notify_all()

                data = json.dumps(body).encode()
                headers = {k: v for k, v in self.headers.items()
                           if k.lower() not in ("host", "authorization", "content-length", "connection",
                                                "transfer-encoding", "keep-alive")}
                headers.update({"authorization": f"Bearer {proxy.key}", "content-length": str(len(data))})
                if proxy.provider == "openai-codex":
                    headers = {"authorization": f"Bearer {proxy.key}", "chatgpt-account-id": proxy.account,
                               "originator": "pi", "OpenAI-Beta": "responses=experimental",
                               "accept": "text/event-stream", "content-type": "application/json",
                               "content-length": str(len(data))}
                up = proxy.upstream
                conn_cls = http.client.HTTPSConnection if up.scheme == "https" else http.client.HTTPConnection
                started = time.time()
                try:
                    conn = conn_cls(up.hostname, up.port, timeout=900)
                    conn.request("POST", up.path.rstrip("/") + "/responses", data, headers)
                    resp = conn.getresponse()
                except (OSError, http.client.HTTPException) as e:
                    # The request may have reached the provider, so a reservation stays charged.
                    finish({"event": "upstream_error", "n": n, "detail": str(e)[:300],
                               **({"provider": "openai-codex", "auth_lane": "subscription", "model": proxy.model_id}
                                  if proxy.provider == "openai-codex" else {}), **settle(None, {})})
                    return self.refuse(502, "upstream unreachable")
                stream_error = False
                try:
                    self.send_response(resp.status)
                    for k, v in resp.getheaders():
                        if k.lower() not in ("transfer-encoding", "connection", "content-length", "keep-alive"):
                            self.send_header(k, v)
                    self.send_header("connection", "close")
                    self.end_headers()
                except OSError:
                    conn.close()
                    finish({"event": "stream_error", "n": n}, stream_error=True)
                    return
                terminal, buf, head = None, b"", b""
                try:
                    while True:
                        chunk = resp.read1(65536)
                        if not chunk:
                            # read1() can silently return EOF with declared bytes still missing.
                            # Chunked framing raises IncompleteRead itself; close-delimited SSE
                            # has no remaining length and relies on its terminal event.
                            if subscription_budget and resp.length not in (None, 0):
                                stream_error = True
                            break
                        self.wfile.write(chunk)
                        self.wfile.flush()
                        if resp.status != 200 and len(head) < 2000:
                            head += chunk[: 2000 - len(head)]
                        buf += chunk
                        *lines, buf = buf.split(b"\n")
                        for line in lines:
                            if line.startswith(b"data: "):
                                try:
                                    event = json.loads(line[6:])
                                except ValueError:
                                    continue
                                if isinstance(event, dict):
                                    if subscription_budget and (event.get("type") == "error" or
                                            event.get("type") in TERMINAL_EVENTS and terminal is not None):
                                        stream_error = True
                                    if event.get("type") in TERMINAL_EVENTS:
                                        terminal = event
                except (OSError, http.client.HTTPException) as e:
                    stream_error = True
                    proxy.log({"event": "stream_error", "n": n, "detail": str(e)[:300]})
                finally:
                    conn.close()
                response = (terminal or {}).get("response") or {}
                response = response if isinstance(response, dict) else {}
                usage = response.get("usage") or {}
                usage = usage if isinstance(usage, dict) else {}
                input_details = usage.get("input_tokens_details")
                input_details = input_details if isinstance(input_details, dict) else {}
                output_details = usage.get("output_tokens_details")
                output_details = output_details if isinstance(output_details, dict) else {}
                finish({
                    "event": "request", "n": n, "http_status": resp.status, "model": body.get("model"),
                    **({"provider": "openai-codex", "auth_lane": "subscription", "billing": "unknown", "quota": "unknown"}
                       if proxy.provider == "openai-codex" else {}),
                    "requested_tier": body.get("service_tier"), "observed_tier": response.get("service_tier"),
                    "terminal": (terminal or {}).get("type"), "response_id": response.get("id"),
                    "response_model": response.get("model"),
                    "usage": {"input_tokens": usage.get("input_tokens"), "output_tokens": usage.get("output_tokens"),
                              "cached_tokens": input_details.get("cached_tokens"),
                              "cache_write_tokens": input_details.get("cache_write_tokens"),
                              "reasoning_tokens": output_details.get("reasoning_tokens")},
                    "seconds": round(time.time() - started, 3),
                    **({"error_head": head.decode("utf-8", "replace")} if resp.status != 200 and proxy.provider != "openai-codex" else {}),
                    **settle(resp.status if terminal is not None or resp.status != 200 else None, response),
                }, http_status=resp.status, response=response, terminal=(terminal or {}).get("type"), stream_error=stream_error)
                self.close_connection = True

        slots = threading.BoundedSemaphore(self.max_connections)

        class Server(socketserver.ThreadingMixIn, socketserver.UnixStreamServer):
            daemon_threads = True

            def process_request(self, request, client_address):
                # Refuse beyond the connection bound instead of spawning a thread.
                if not slots.acquire(blocking=False):
                    with proxy.lock:
                        proxy.dropped_connections += 1
                    self.shutdown_request(request)
                    return
                super().process_request(request, client_address)

            def process_request_thread(self, request, client_address):
                try:
                    super().process_request_thread(request, client_address)
                finally:
                    slots.release()

        if os.path.exists(self.socket_path):
            os.unlink(self.socket_path)
        self.server = Server(self.socket_path, Handler)
        os.chmod(self.socket_path, 0o600)
        threading.Thread(target=self.server.serve_forever, daemon=True).start()
        return self

    def stop(self):
        if self.server:
            self.server.shutdown()
            self.server.server_close()


# --------------------------------------------------------------- runner ----

def run_turn(spec_path: str) -> int:
    spec = json.loads(Path(spec_path).read_text())
    key = sys.stdin.readline().strip()
    sys.stdin.close()
    docker = spec["docker"]
    record = spec["files"]["runner_record"]

    def note(event: str, **fields):
        with open(record, "a") as fh:
            fh.write(json.dumps({"at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()), "event": event, **fields}) + "\n")

    # First record: this runner's identity, so a turn without a launch row can still be
    # recognised as live (status, retire) until it records container_removed.
    note("runner_started", pid=os.getpid(), pid_start=process_start_time(os.getpid()))
    if not key:
        note("refused", detail="no provider key on stdin")
        return 64
    os.makedirs(spec["socket_dir"], mode=0o700, exist_ok=True)
    name = spec["container_name"]
    stopping = threading.Event()

    def stop(**why):
        if not stopping.is_set():
            stopping.set()
            note("stop_requested", **why)
            threading.Thread(target=subprocess.run, args=([docker, "stop", "-t", "10", name],),
                             kwargs={"capture_output": True}, daemon=True).start()

    def stop_container(signum, _frame):
        stop(signal=signum)

    provider = spec.get("provider", "openai")
    budget = None
    if spec.get("usd_limit"):
        lim = spec["usd_limit"]
        if provider == "openai-codex":
            budget = SubscriptionProxyBudget(max_usd=lim["max_usd"], rates=lim["rates"],
                                              spent_before_usd=lim["spent_before_usd"])
        else:
            budget = BUDGETS[provider](max_usd=lim["max_usd"], rates=lim["rates"], tier=spec.get("service_tier"),
                           max_output_tokens=spec["max_output_tokens"], spent_before_usd=lim["spent_before_usd"],
                           prefix=lim.get("prefix"), multiplier_floor=lim.get("multiplier_floor") or 0.0,
                           strict_tier=bool(lim.get("strict_tier")))
    try:
        proxy = make_proxy(spec, key, budget, provider, stop)
    except ValueError as e:
        note("refused", detail=str(e)[:200])
        return 64
    del key
    note("proxy_listening", socket=spec["socket_path"], upstream=spec["upstream"])
    return run_container(spec, proxy, note, stop, stopping, stop_container)


def process_start_time(pid):
    """Kernel start time (clock ticks since boot) of a live, non-zombie process, else None."""
    try:
        fields = Path(f"/proc/{pid}/stat").read_text().rsplit(")", 1)[1].split()
    except (OSError, IndexError):
        return None
    return None if fields[0] == "Z" else int(fields[19])


def make_proxy(spec, key, budget, provider, stop):
    return EgressProxy(spec["socket_path"], upstream=spec["upstream"], key=key, model_id=spec["model_id"],
                        tier=spec.get("service_tier"), log_path=spec["files"]["proxy_log"],
                        max_requests=spec.get("max_requests"), max_output_tokens=spec.get("max_output_tokens"),
                        budget=budget, on_usd_exhausted=lambda detail: stop(
                            reason="claude_sdk_halt" if provider == "claude-sdk" else "usd_limit", detail=detail),
                        provider=provider).start()


def run_container(spec, proxy, note, stop, stopping, stop_container):
    docker, name = spec["docker"], spec["container_name"]
    signal.signal(signal.SIGTERM, stop_container)
    signal.signal(signal.SIGINT, stop_container)
    rc = 0
    try:
        if stopping.is_set():
            note("container_not_started", detail="stop requested before start")
            return 0
        started = subprocess.run([docker, *docker_argv(spec)], capture_output=True, text=True)
        if started.returncode != 0:
            note("container_not_started", exit=started.returncode, stderr=started.stderr.strip()[-800:])
            return 23
        note("container_started", id=started.stdout.strip())
        waited = subprocess.run([docker, "wait", name], capture_output=True, text=True)
        note("container_exited", wait_exit=waited.returncode, status=waited.stdout.strip())
        inspected = subprocess.run([docker, "inspect", "--format", "{{json .State}}", name], capture_output=True, text=True)
        if inspected.returncode == 0:
            try:
                state = json.loads(inspected.stdout)
                note("container_state", **{k: state.get(k) for k in ("ExitCode", "OOMKilled", "Error", "StartedAt", "FinishedAt")})
            except ValueError:
                note("container_state_unreadable")
    finally:
        removed = subprocess.run([docker, "rm", "-f", name], capture_output=True, text=True)
        note("container_removed", exit=removed.returncode)
        proxy.stop()
        shutil.rmtree(spec["socket_dir"], ignore_errors=True)
    return rc


def main(argv=None) -> int:
    argv = sys.argv[1:] if argv is None else argv
    if len(argv) == 2 and argv[0] == "run-turn":
        return run_turn(argv[1])
    print("usage: pi_container.py run-turn SPEC_JSON  (the provider key is read from stdin)", file=sys.stderr)
    return 64


if __name__ == "__main__":
    sys.exit(main())
