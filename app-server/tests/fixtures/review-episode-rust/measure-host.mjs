import { fork, spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { closeSync, fsyncSync, openSync, readFileSync, rmSync, writeFileSync,
  writeSync } from "node:fs";
import os from "node:os";
import path from "node:path";
import {summarizeChildTimeouts} from "./timeout-cause.mjs";

const root = path.resolve(new URL("../../../../", import.meta.url).pathname);
const driver = path.join(root, "app-server/tests/fixtures/review-episode-rust/native-host-driver.mjs");
const profilePath = path.join(root, "app-server/tests/fixtures/review-episode-rust/host-workload-v1.json");
const profile = JSON.parse(readFileSync(profilePath));
const sha = (bytes) => createHash("sha256").update(bytes).digest("hex");
const outputPath = process.argv[2];
if (!outputPath || !path.isAbsolute(outputPath)) throw new Error("absolute HOST result output path required");
const rawEventsPath = `${outputPath}.samples.jsonl`;
const rawFd = openSync(rawEventsPath, "wx", 0o600);
let rawUnflushed = 0;
let activeHostChild = null;
const record = (event) => {
  writeSync(rawFd, `${JSON.stringify(event)}\n`);
  if (++rawUnflushed >= 10) {fsyncSync(rawFd); rawUnflushed = 0;}
};
const flushRaw = () => {fsyncSync(rawFd); rawUnflushed = 0;};
for (const signal of ["SIGINT", "SIGTERM"]) process.on(signal, () => {
  record({type: "interrupted", signal, at: new Date().toISOString()});
  flushRaw();
  activeHostChild?.kill(signal);
  process.exit(signal === "SIGINT" ? 130 : 143);
});
const compilerBinary = process.env.WORK_ENGINE_HOST_COMPILER_BINARY;
const episodeBinary = process.env.WORK_ENGINE_HOST_EPISODE_BINARY;
const compilerSha = process.env.WORK_ENGINE_HOST_COMPILER_SHA256;
const episodeSha = process.env.WORK_ENGINE_HOST_EPISODE_SHA256;
if (!compilerBinary || !episodeBinary || sha(readFileSync(compilerBinary)) !== compilerSha
    || sha(readFileSync(episodeBinary)) !== episodeSha) throw new Error("HOST exact release binary binding differs");
function verifySourceBinding(sourcePath) {
  if (!sourcePath || !path.isAbsolute(sourcePath)) throw new Error("HOST exact source binding path required");
  const manifest = JSON.parse(readFileSync(sourcePath, "utf8"));
  const stream = createHash("sha256");
  const names = manifest.files.map(({path: name}) => name);
  if (new Set(names).size !== names.length || names.join("\0") !== [...names].sort().join("\0")) {
    throw new Error("HOST source inventory is not unique and sorted");
  }
  for (const entry of manifest.files) {
    const absolute = path.resolve(root, entry.path);
    if (!absolute.startsWith(`${root}/`)) throw new Error("HOST source inventory escaped checkout");
    const name = Buffer.from(entry.path);
    const bytes = readFileSync(absolute);
    if (bytes.length !== entry.bytes || sha(bytes) !== entry.sha256) {
      throw new Error(`HOST source file differs: ${entry.path}`);
    }
    const nameLength = Buffer.alloc(4); nameLength.writeUInt32BE(name.length);
    const bodyLength = Buffer.alloc(8); bodyLength.writeBigUInt64BE(BigInt(bytes.length));
    stream.update(nameLength); stream.update(name); stream.update(bodyLength); stream.update(bytes);
  }
  if (stream.digest("hex") !== manifest.aggregateSha256) throw new Error("HOST source aggregate differs");
  return {path: sourcePath, fileSha256: sha(readFileSync(sourcePath)),
    aggregateSha256: manifest.aggregateSha256, fileCount: manifest.files.length};
}
const sourceBindings = {
  c3: verifySourceBinding(process.env.WORK_ENGINE_HOST_C3_SOURCE_BINDING),
  r3: verifySourceBinding(process.env.WORK_ENGINE_HOST_R3_SOURCE_BINDING),
};
const corpusBindings = {compilerFixtureSha256: sha(readFileSync(path.join(root, profile.compilerFixture.path))),
  episodeFixtureSha256: sha(readFileSync(path.join(root, profile.episodeFixture.path)))};
if (corpusBindings.compilerFixtureSha256 !== profile.compilerFixture.sha256
    || corpusBindings.episodeFixtureSha256 !== profile.episodeFixture.sha256) {
  throw new Error("HOST corpus differs before launch");
}
const now = () => process.hrtime.bigint();
const ms = (start, end = now()) => Number(end - start) / 1e6;
const percentile = (values, fraction) => values.length
  ? [...values].sort((a, b) => a - b)[Math.ceil(values.length * fraction) - 1] : null;
const summary = (values) => ({count: values.length, p50: percentile(values, 0.5),
  p95: percentile(values, 0.95), p99: percentile(values, 0.99), maximum: values.length ? Math.max(...values) : null});

function selector(variant) {
  const c3 = variant === "c3_only" || variant === "both";
  const r3 = variant === "r3_only" || variant === "both";
  const value = {compiler: c3 ? {binaryPath: compilerBinary, expectedSha256: compilerSha,
    protocolVersion: 3, launchProfileId: "manifest-sync-v1"} : {backend: "legacy"},
  episode: r3 ? {binaryPath: episodeBinary, expectedSha256: episodeSha,
    protocolVersion: 2, launchProfileId: "native-host-v1"} : {backend: "legacy"}};
  return {...value, digest: sha(Buffer.from(JSON.stringify(value)))};
}

async function startHost(variant, coldOnly = false) {
  const started = now();
  const child = fork(driver, [], {execArgv: [],
    env: {...process.env, WORK_ENGINE_HOST_VARIANT: variant,
      WORK_ENGINE_HOST_COLD: coldOnly ? "1" : "0"},
    stdio: ["ignore", "ignore", "pipe", "ipc"]});
  activeHostChild = child;
  const pending = new Map();
  const probes = [];
  let nextId = 0;
  let ready = null;
  let readyAt = null;
  let exited = false;
  let stderr = "";
  child.stderr.on("data", (chunk) => { stderr += chunk.toString(); });
  let acceptReady;
  let rejectReady;
  const readyPromise = new Promise((resolve, reject) => {acceptReady = resolve; rejectReady = reject;});
  child.on("message", (message) => {
    if (message?.type === "ready") {
      ready = message;
      readyAt = now();
      acceptReady(message);
      record({type: "ready", variant, coldOnly, launchMs: ms(started, readyAt), corpus: message.corpus});
      return;
    }
    const item = pending.get(message?.id);
    if (message?.type === "probe" && item?.type === "probe") {
      item.receivedAt = now();
      item.delayMs = ms(item.sentAt, item.receivedAt);
      item.late = item.missed;
      record({type: "probe_reply", variant, ...{id: item.id, phase: item.phase,
        delayMs: item.delayMs, missed: item.missed, late: item.late}});
      return;
    }
    if (message?.type === "work" && item?.type === "work") {
      pending.delete(message.id);
      const completed = {...message, fullHostRequestMs: ms(item.sentAt)};
      record({type: "work_reply", variant, ...completed});
      item.resolve(completed);
    }
  });
  child.on("exit", (code, signal) => {
    exited = true;
    record({type: "host_exit", variant, code, signal});
    if (activeHostChild === child) activeHostChild = null;
    if (!ready) rejectReady(new Error(`HOST ${variant} exited before ready: ${code}/${signal}: ${stderr}`));
    for (const [id, item] of pending) if (item.type === "work") {
      record({type: "work_interrupted", variant, id, code, signal, elapsedMs: ms(item.sentAt)});
      item.reject(new Error(`HOST exited during work: ${code}/${signal}`));
    }
  });
  const sendProbe = (phase) => {
    const id = `probe-${nextId++}`;
    const item = {id, type: "probe", phase, sentAt: now(), receivedAt: null,
      delayMs: null, missed: false, late: false};
    pending.set(id, item); probes.push(item);
    child.send({type: "probe", id});
    record({type: "probe_sent", variant, id, phase});
    return item;
  };
  const work = (kind) => new Promise((resolve, reject) => {
    const id = `work-${nextId++}`;
    const item = {type: "work", sentAt: now(), resolve, reject};
    pending.set(id, item);
    child.send({type: "work", id, kind});
    const timeout = setTimeout(() => {
      if (pending.delete(id)) {
        record({type: "work_timeout", variant, id, kind, elapsedMs: ms(item.sentAt)});
        reject(new Error(`HOST ${variant} ${kind} route exceeded 30 s`));
      }
    }, 30_000);
    const originalResolve = item.resolve;
    const originalReject = item.reject;
    item.resolve = (value) => {clearTimeout(timeout); originalResolve(value);};
    item.reject = (error) => {clearTimeout(timeout); originalReject(error);};
  });
  const drainProbes = async () => {
    const deadline = Date.now() + 1_000;
    while (probes.some((item) => item.receivedAt === null && !item.missed) && Date.now() < deadline) {
      await new Promise((resolve) => setTimeout(resolve, 10));
    }
    for (const item of probes) if (item.receivedAt === null) item.missed = true;
    for (const item of probes) if (item.missed && !item.missedRecorded) {
      item.missedRecorded = true;
      record({type: "probe_missed", variant, id: item.id, phase: item.phase});
    }
  };
  const close = async () => {
    if (!exited) {
      child.send({type: "close"});
      const deadline = Date.now() + 5_000;
      while (!exited && Date.now() < deadline) await new Promise((resolve) => setTimeout(resolve, 10));
      if (!exited) child.kill("SIGKILL");
    }
    if (ready?.stateRoot?.startsWith(`/tmp/work-engine-host-${variant}-`)) {
      rmSync(ready.stateRoot, {recursive: true, force: true});
    }
  };
  try {
    const coldTimer = coldOnly
      ? setInterval(() => sendProbe("cold"), profile.measurement.probeIntervalMs) : null;
    await Promise.race([readyPromise, new Promise((_, reject) =>
      setTimeout(() => reject(new Error(`HOST ${variant} startup exceeded 30 s`)), 30_000))]);
    if (coldTimer) {
      clearInterval(coldTimer);
      await drainProbes();
    }
    return {variant, child, ready, launchMs: ms(started, readyAt), probes, sendProbe, work, drainProbes, close};
  } catch (error) { child.kill("SIGKILL"); throw error; }
}

async function workload(host, kind) {
  const warmups = [];
  const measured = [];
  const firstProbe = host.probes.length;
  let phase = "warmup";
  const timer = setInterval(() => host.sendProbe(`${kind}:${phase}`), profile.measurement.probeIntervalMs);
  try {
    for (let i = 0; i < profile.measurement.warmupsPerWorkload; i++) warmups.push(await host.work(kind));
    phase = "measured";
    for (let i = 0; i < profile.measurement.measuredPerWorkload; i++) {
      measured.push(await host.work(kind));
      if ((i + 1) % 50 === 0) process.stderr.write(`${host.variant} ${kind} ${i + 1}/${profile.measurement.measuredPerWorkload}\n`);
    }
  } finally { clearInterval(timer); }
  await host.drainProbes();
  const probes = host.probes.slice(firstProbe).map(({id, phase: itemPhase, delayMs, missed, late}) =>
    ({id, phase: itemPhase, delayMs, missed, late}));
  const measuredProbes = probes.filter((item) => item.phase.endsWith(":measured"));
  const byCase = Object.fromEntries(profile.workloads.error_contention.cases.map((name) =>
    [name, measured.filter((item) => item.caseName === name)]));
  const metrics = {operation: summary(measured.map((item) => item.fullHostRequestMs)),
    probe: summary(measuredProbes.filter((item) => item.delayMs !== null).map((item) => item.delayMs)),
    missedProbes: measuredProbes.filter((item) => item.missed).length,
    ...summarizeChildTimeouts(measured),
    outcomes: {ok: measured.filter((item) => item.outcome === "ok").length,
      error: measured.filter((item) => item.outcome === "error").length},
    components: Object.fromEntries([...new Set(measured.flatMap((item) =>
      (item.components ?? []).map((part) => part.name)))].map((name) =>
      [name, {ok: measured.filter((item) => item.components?.some((part) => part.name === name && part.outcome === "ok")).length,
        error: measured.filter((item) => item.components?.some((part) => part.name === name && part.outcome === "error")).length}]))};
  const target = profile.measurement;
  metrics.controlRouteTargetPassed = metrics.probe.p99 !== null
    && metrics.probe.p99 <= target.controlRouteP99BudgetMs
    && metrics.probe.maximum <= target.controlRouteMaximumBudgetMs
    && metrics.missedProbes === 0;
  return {kind, warmups, measured, probes, byCase, metrics};
}

async function stress(host) {
  const firstProbe = host.probes.length;
  const timer = setInterval(() => host.sendProbe("stress"), profile.measurement.probeIntervalMs);
  let requests;
  try { requests = await Promise.all(Array.from({length: profile.measurement.parallelStressRequests},
    () => host.work("normal"))); }
  finally { clearInterval(timer); }
  await host.drainProbes();
  const probes = host.probes.slice(firstProbe).map(({id, delayMs, missed, late}) => ({id, delayMs, missed, late}));
  return {requests, probes, operation: summary(requests.map((item) => item.fullHostRequestMs)),
    probe: summary(probes.filter((item) => item.delayMs !== null).map((item) => item.delayMs))};
}

const receipt = {schemaVersion: 1, profileSha256: sha(readFileSync(profilePath)),
  driverSha256: sha(readFileSync(driver)), compilerBinary: {path: compilerBinary, sha256: compilerSha},
  episodeBinary: {path: episodeBinary, sha256: episodeSha},
  rawEventsPath, sourceBindings, corpusBindings, status: "running",
  machine: {platform: process.platform, release: os.release(), arch: process.arch,
    cpu: os.cpus()[0]?.model, logicalCpus: os.cpus().length, node: process.version,
    rustc: spawnSync("rustc", ["+1.92.0", "--version"], {encoding: "utf8"}).stdout.trim()},
  selections: Object.fromEntries(profile.measurement.variants.map((variant) => [variant, selector(variant)])),
  variants: {}};
const write = () => writeFileSync(outputPath, `${JSON.stringify(receipt, null, 2)}\n`);
write();
record({type: "binding", at: new Date().toISOString(),
  profileSha256: receipt.profileSha256, driverSha256: receipt.driverSha256,
  harnessSha256: sha(readFileSync(new URL(import.meta.url))), sourceBindings,
  corpusBindings, compilerBinary: receipt.compilerBinary, episodeBinary: receipt.episodeBinary,
  selections: receipt.selections, machine: receipt.machine});
flushRaw();
try {
  for (const variant of profile.measurement.variants) {
    const selected = selector(variant);
    const cold = [];
    for (let i = 0; i < profile.measurement.freshHostColdLaunchesPerVariant; i++) {
      const host = await startHost(variant, true);
      try { cold.push({launchMs: host.launchMs, corpus: host.ready.corpus,
        probes: host.probes.map(({id, delayMs, missed, late}) => ({id, delayMs, missed, late}))}); }
      finally { await host.close(); }
    }
    const host = await startHost(variant);
    try {
      const result = {selection: selected, corpus: host.ready.corpus,
        cold, coldLaunch: summary(cold.map((item) => item.launchMs)), workloads: {}, stress: null};
      for (const name of ["normal", "maximum", "error_contention"]) {
        result.workloads[name] = await workload(host, name);
      }
      result.stress = await stress(host);
      receipt.variants[variant] = result;
      write();
      flushRaw();
    } finally { await host.close(); }
  }
} catch (error) {
  receipt.status = "incomplete";
  receipt.error = {name: error.name, message: error.message};
  record({type: "harness_error", ...receipt.error});
  write();
  flushRaw();
  throw error;
}
receipt.status = "complete";
write();
flushRaw();
closeSync(rawFd);
