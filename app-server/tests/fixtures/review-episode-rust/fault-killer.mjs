import {readFileSync, readdirSync, writeFileSync, existsSync} from "node:fs";
import path from "node:path";

const [directory, cut, parentPidText, binaryPath] = process.argv.slice(2);
const parentPid = Number(parentPidText);
const ready = path.join(directory, `${cut}.ready`);
const acknowledged = path.join(directory, `${cut}.killed`);
const deadline = Date.now() + 10_000;
const timer = setInterval(() => {
  if (Date.now() > deadline) {clearInterval(timer); process.exitCode = 2; return;}
  if (!existsSync(ready)) return;
  for (const entry of (awaitProc()).values()) {
    if (entry.pid === process.pid || entry.ppid !== parentPid
        || !entry.command.startsWith(`${binaryPath} `)) continue;
    try {
      process.kill(entry.pid, "SIGKILL");
      writeFileSync(acknowledged, JSON.stringify({cut, pid: entry.pid, readyObserved: true}));
      clearInterval(timer);
      return;
    } catch {}
  }
}, 2);

function awaitProc() {
  const entries = [];
  for (const name of readdirSync("/proc")) {
    if (!/^\d+$/.test(name)) continue;
    try {
      const status = readFileSync(`/proc/${name}/status`, "utf8");
      const ppid = Number(status.match(/^PPid:\s*(\d+)$/m)?.[1]);
      const command = readFileSync(`/proc/${name}/cmdline`, "utf8").replaceAll("\0", " ");
      entries.push({pid: Number(name), ppid, command});
    } catch {}
  }
  return entries;
}
