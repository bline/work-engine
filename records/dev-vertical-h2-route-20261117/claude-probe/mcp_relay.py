#!/usr/bin/env python3
"""Prototype MCP stdio server: one snapshot_read tool over a fixture dir; logs calls."""
import json, os, sys, hashlib

ROOT = os.environ["SNAP_ROOT"]
LOG = os.environ.get("RELAY_LOG", "/dev/null")
TOOLS = [{
    "name": "snapshot_read",
    "description": "Read one approved snapshot member and bounded range.",
    "inputSchema": {"type": "object", "properties": {
        "snapshot_id": {"type": "string"}, "member_id": {"type": "string"},
        "offset": {"type": "integer", "minimum": 0},
        "length": {"type": "integer", "minimum": 1, "maximum": 65536}},
        "required": ["snapshot_id", "member_id", "offset", "length"], "additionalProperties": False},
}]


def log(obj):
    with open(LOG, "a") as f:
        f.write(json.dumps(obj) + "\n")


def reply(id_, result=None, error=None):
    msg = {"jsonrpc": "2.0", "id": id_}
    if error is not None:
        msg["error"] = error
    else:
        msg["result"] = result
    sys.stdout.write(json.dumps(msg) + "\n")
    sys.stdout.flush()


for line in sys.stdin:
    if not line.strip():
        continue
    req = json.loads(line)
    log({"in": req})
    m, id_ = req.get("method"), req.get("id")
    if m == "initialize":
        reply(id_, {"protocolVersion": req["params"].get("protocolVersion", "2025-06-18"),
                    "capabilities": {"tools": {}}, "serverInfo": {"name": "we", "version": "0.1"}})
    elif m == "tools/list":
        reply(id_, {"tools": TOOLS})
    elif m == "tools/call":
        a = req["params"].get("arguments", {})
        p = os.path.join(ROOT, a.get("member_id", ""))
        try:
            data = open(p, "rb").read()[a["offset"]:a["offset"] + a["length"]]
            text = json.dumps({"encoding": "utf8", "text": data.decode(), "sha256": hashlib.sha256(data).hexdigest(),
                               "snapshot_id": a["snapshot_id"], "member_id": a["member_id"],
                               "offset": a["offset"], "length": a["length"]})
            reply(id_, {"content": [{"type": "text", "text": text}]})
        except Exception as e:
            reply(id_, {"content": [{"type": "text", "text": f"error: {e}"}], "isError": True})
    elif id_ is not None:
        reply(id_, error={"code": -32601, "message": "method not found"})
