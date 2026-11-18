// In-container: loopback HTTP listener -> framed lines on stdout; replies on stdin.
// Spawns pi (claude-sdk provider) with ANTHROPIC_BASE_URL at the listener.
import http from "node:http";
import fs from "node:fs";
import { spawn } from "node:child_process";
import readline from "node:readline";

const out = "/out";
const log = fs.createWriteStream(`${out}/forwarder.log`);
const pending = new Map();
let next = 0;
const send = (o) => process.stdout.write(JSON.stringify(o) + "\n");

const server = http.createServer((req, res) => {
  const chunks = [];
  req.on("data", (c) => chunks.push(c));
  req.on("end", () => {
    const id = ++next;
    const body = Buffer.concat(chunks);
    log.write(JSON.stringify({ id, method: req.method, url: req.url, host: req.headers.host, len: body.length }) + "\n");
    pending.set(id, res);
    send({ t: "req", id, method: req.method, path: req.url, headers: req.headers, body_b64: body.toString("base64") });
  });
});
readline.createInterface({ input: process.stdin }).on("line", (line) => {
  const m = JSON.parse(line);
  const res = pending.get(m.id);
  if (!res) return;
  if (m.t === "head") res.writeHead(m.status, m.headers);
  else if (m.t === "chunk") res.write(Buffer.from(m.b64, "base64"));
  else if (m.t === "end") { res.end(); pending.delete(m.id); }
});
server.listen(0, "127.0.0.1", () => {
  const port = server.address().port;
  const home = "/tmp/home";
  fs.mkdirSync(`${home}/.claude`, { recursive: true, mode: 0o700 });
  fs.writeFileSync(`${home}/.claude/.credentials.json`, fs.readFileSync("/probe-in/dummy-credentials.json"), { mode: 0o600 });
  fs.mkdirSync("/tmp/pi-agent", { recursive: true });
  const args = [
    "/pi/node_modules/@earendil-works/pi-coding-agent/dist/bundle/cli.js",
    "-p", "--no-session", "--offline", "-ne", "-e", "/ext/node_modules/pi-claude-subscription",
    "--no-context-files", "--no-skills", "--no-prompt-templates", "--no-mcp",
    "--tools", "read", "--model", process.env.PROBE_MODEL,
    fs.readFileSync("/probe-in/prompt.txt", "utf8"),
  ];
  const child = spawn("node", args, {
    cwd: "/workspace",
    env: {
      PATH: process.env.PATH, HOME: home, PI_CODING_AGENT_DIR: "/tmp/pi-agent", PI_OFFLINE: "1",
      ANTHROPIC_BASE_URL: `http://127.0.0.1:${port}`, NO_PROXY: "127.0.0.1,localhost",
      PI_CLAUDE_SDK_DEBUG: `${out}/sdk-debug.log`, TMPDIR: "/tmp",
    },
    stdio: ["ignore", fs.openSync(`${out}/pi.out`, "w"), fs.openSync(`${out}/pi.err`, "w")],
  });
  child.on("exit", (code, signal) => {
    send({ t: "done", code, signal });
    setTimeout(() => process.exit(0), 200);
  });
});
