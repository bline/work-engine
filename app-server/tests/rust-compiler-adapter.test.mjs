import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { spawn, spawnSync } from "node:child_process";
import { chmod, cp, lstat, mkdtemp, readFile, readdir, symlink, unlink, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import test from "node:test";
import { compileRustSkill } from "../src/rust-compiler-adapter.mjs";
import { compileSkill } from "../src/skill-compiler.mjs";
import { compileSkill as compilePinnedLegacy } from "./fixtures/compiler-c2/legacy/skill-compiler.mjs";
import { ManifestRoleRuntime, hydrateRuntimeRequirements, loadRuntimeManifest, loadRuntimeManifestDocument, satisfyRuntimeRequirements } from "../src/runtime-manifest.mjs";

const repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
const binary = path.resolve(process.env.WORK_ENGINE_C2_TEST_BINARY ?? path.join(repo, "rust/target/debug/work-engine-compiler"));
const fixtureRoot = path.join(repo, "app-server/tests/fixtures/compiler-c2/root");
const corpus = path.join(repo, "rust/crates/work-engine-compiler/tests/fixtures/v1/skills");
const digest = (bytes) => createHash("sha256").update(bytes).digest("hex");
const binaryDigest = digest(await readFile(binary));
const expectedScriptDigest = "24583ce14385728da3459acd848368fbb7768258bdb80dfefa3f00cef7545450";
async function sources(id) {
  const dir = path.join(corpus, id);
  return { structureSource: await readFile(path.join(dir, "structure.yaml"), "utf8"), interfaceSource: await readFile(path.join(dir, "interface.yaml"), "utf8"), expected: await readFile(path.join(dir, "expected.md")) };
}
async function rust(id, extras = {}) {
  const { expected, ...input } = await sources(id);
  return { expected, result: await compileRustSkill({ ...input, workspaceRoot: fixtureRoot, binaryPath: binary, expectedBinarySha256: binaryDigest, ...extras }) };
}
function env() { return { ...process.env, WORK_ENGINE_COMPILER_BACKEND: "rust", WORK_ENGINE_COMPILER_RUST_BINARY: binary, WORK_ENGINE_COMPILER_RUST_BINARY_SHA256: binaryDigest }; }
function cli(args, environment = env()) { return spawnSync(process.execPath, [path.join(repo, "app-server/scripts/compile-skill.mjs"), ...args], { cwd: repo, env: environment, encoding: "utf8", timeout: 15_000 }); }

await test("all five immutable skills verify exact bytes and role-free runs omit Python", async () => {
  for (const id of ["p-sup", "p-bld", "p-rep", "p-clm", "p-air"]) {
    const { expected, result } = await rust(id);
    const { structureSource, interfaceSource } = await sources(id);
    const legacy = await compilePinnedLegacy({ structureSource, interfaceSource, workspaceRoot: fixtureRoot });
    assert.deepEqual(result.output, expected, id);
    assert.deepEqual(result.output, legacy.output, `${id} pinned legacy output`);
    const { compiler: rustProducer, ...rustIr } = result.ir;
    const { compiler: legacyProducer, ...legacyIr } = legacy.ir;
    assert.equal(rustProducer, "work-engine.skill-compiler.rust-v1");
    assert.equal(legacyProducer, "work-engine.skill-compiler.bootstrap-v1");
    assert.deepEqual(rustIr, legacyIr, `${id} complete producer-neutral IR`);
    assert.equal(result.ir.runtime_requirements.verified_sources, true, id);
    assert.equal(result.verification.rust_binary_sha256, binaryDigest);
    if (["p-sup", "p-bld"].includes(id)) {
      assert.equal(result.verification.python.direct_child_reaped, true);
      assert.equal(result.verification.python.exit_code, 0);
      assert.equal(result.verification.python.script_sha256, expectedScriptDigest);
      assert.equal(result.ir.role_projection.canonical_role_match, true);
    } else {
      assert.equal(Object.hasOwn(result.verification, "python"), false);
      assert.equal(Object.hasOwn(result.ir, "role_projection"), false);
    }
    const repeat = await rust(id);
    assert.deepEqual(repeat.result.ir, result.ir, `${id} deterministic IR`);
  }
});

await test("BOM-prefixed Buffer and identical string retain the same raw input identity", async () => {
  const { structureSource, interfaceSource, expected } = await sources("p-rep");
  const raw = Buffer.concat([Buffer.from([0xef, 0xbb, 0xbf]), Buffer.from(structureSource)]);
  const selected = { interfaceSource, workspaceRoot: fixtureRoot, binaryPath: binary, expectedBinarySha256: binaryDigest };
  const fromBuffer = await compileRustSkill({ ...selected, structureSource: raw });
  const fromString = await compileRustSkill({ ...selected, structureSource: raw.toString("utf8") });
  assert.deepEqual(fromBuffer.output, expected);
  assert.deepEqual(fromBuffer.ir, fromString.ir);
  assert.equal(fromBuffer.ir.input_sha256.structure, digest(raw));
  assert.equal(fromBuffer.ir.runtime_requirements.verified_sources, true);
});

await test("real hydration compiles all six occurrences and loadRuntimeManifest accepts them", async () => {
  const prior = { backend: process.env.WORK_ENGINE_COMPILER_BACKEND, binary: process.env.WORK_ENGINE_COMPILER_RUST_BINARY, digest: process.env.WORK_ENGINE_COMPILER_RUST_BINARY_SHA256 };
  Object.assign(process.env, env());
  try {
    const manifestPath = path.join(fixtureRoot, "app-server/runtime-manifest.yaml");
    const document = await loadRuntimeManifestDocument(manifestPath);
    const t = performance.now();
    const requirements = await hydrateRuntimeRequirements(document.document, { baseDirectory: path.dirname(manifestPath), workspaceRoot: fixtureRoot });
    assert.deepEqual(Object.keys(requirements), ["slice-supervisor", "slice-builder", "slice-builder:repo-search", "implementation-reviewer:repo-search", "implementation-reviewer:claim-evidence", "implementation-reviewer:agent-instruction-review"]);
    assert.ok(Object.values(requirements).every((entry) => entry.verified_sources === true));
    const manifest = await loadRuntimeManifest(manifestPath);
    assert.ok(manifest);
    console.log(`C2 complete hydration elapsed ${Math.round(performance.now() - t)}ms`);
    const { structureSource, interfaceSource } = await sources("p-rep");
    const unverified = await compileSkill({ structureSource, interfaceSource, workspaceRoot: fixtureRoot, verifySources: false });
    assert.equal(unverified.ir.runtime_requirements.verified_sources, false);
    assert.throws(() => satisfyRuntimeRequirements({ manifest, roleId: "slice-builder", skillName: "repo-search", requirements: unverified.ir.runtime_requirements }), /not bound to verified sources/);
    let deliveries = 0;
    const runtime = new ManifestRoleRuntime({ adapter: { deliverTurn: async () => { deliveries += 1; return {}; } }, manifest, runtimeRequirements: { "slice-builder:repo-search": unverified.ir.runtime_requirements } });
    await assert.rejects(runtime.deliverTurn({ roleId: "slice-builder", instanceId: "rejected", text: "must not deliver" }), /not bound to verified sources/);
    assert.equal(deliveries, 0);
    const options = { baseDirectory: path.dirname(manifestPath), workspaceRoot: fixtureRoot };
    const wrongId = structuredClone(document.document);
    wrongId.roles["slice-builder"].skills.find((skill) => skill.compiled_environment).name = "wrong-secondary-id";
    await assert.rejects(hydrateRuntimeRequirements(wrongId, options), /compiled skill identity differs/);
    const wrongFingerprint = structuredClone(document.document);
    wrongFingerprint.roles["slice-builder"].skills.find((skill) => skill.compiled_environment).compiled_skill_sha256 = "f".repeat(64);
    await assert.rejects(hydrateRuntimeRequirements(wrongFingerprint, options), /compiled skill fingerprint differs/);
    const wrongRole = structuredClone(document.document);
    wrongRole.roles["slice-builder"].compiled_environment = { structure: "migrations/skills/repo-search/structure.yaml", interface: "migrations/skills/repo-search/interface.yaml" };
    await assert.rejects(hydrateRuntimeRequirements(wrongRole, options), /does not define a role profile/);
  } finally {
    for (const [key, value] of [["WORK_ENGINE_COMPILER_BACKEND", prior.backend], ["WORK_ENGINE_COMPILER_RUST_BINARY", prior.binary], ["WORK_ENGINE_COMPILER_RUST_BINARY_SHA256", prior.digest]]) {
      if (value === undefined) delete process.env[key]; else process.env[key] = value;
    }
  }
});

await test("hydration and manifest loading refuse invalid UTF-8 in role and secondary YAML inputs", async () => {
  const temp = await mkdtemp(path.join(os.tmpdir(), "work-engine-c2-hydration-utf8-"));
  const root = path.join(temp, "root");
  await cp(fixtureRoot, root, { recursive: true });
  const manifestPath = path.join(root, "app-server/runtime-manifest.yaml");
  const loaded = await loadRuntimeManifestDocument(manifestPath);
  const options = { baseDirectory: path.dirname(manifestPath), workspaceRoot: root };
  const previous = { backend: process.env.WORK_ENGINE_COMPILER_BACKEND, binary: process.env.WORK_ENGINE_COMPILER_RUST_BINARY, digest: process.env.WORK_ENGINE_COMPILER_RUST_BINARY_SHA256 };
  Object.assign(process.env, env());
  try {
    for (const skill of ["slice-supervisor", "repo-search"]) {
      for (const name of ["structure.yaml", "interface.yaml"]) {
        const file = path.join(root, "app-server/migrations/skills", skill, name);
        const original = await readFile(file);
        await writeFile(file, Buffer.concat([original, Buffer.from([0x0a, 0x23, 0x20, 0xff, 0x0a])]));
        try {
          await assert.rejects(hydrateRuntimeRequirements(loaded.document, options), /must be UTF-8/, `${skill}/${name} hydration`);
          await assert.rejects(loadRuntimeManifest(manifestPath), /must be UTF-8/, `${skill}/${name} load`);
        } finally { await writeFile(file, original); }
      }
    }
  } finally {
    for (const [key, value] of [["WORK_ENGINE_COMPILER_BACKEND", previous.backend], ["WORK_ENGINE_COMPILER_RUST_BINARY", previous.binary], ["WORK_ENGINE_COMPILER_RUST_BINARY_SHA256", previous.digest]]) {
      if (value === undefined) delete process.env[key]; else process.env[key] = value;
    }
  }
});

await test("CLI publishes complete output, preserves old bytes on refusal, preserves mode and refuses symlinks", async () => {
  const temp = await mkdtemp(path.join(os.tmpdir(), "work-engine-c2-cli-test-"));
  const input = path.join(fixtureRoot, "app-server/migrations/skills/slice-supervisor");
  const output = path.join(temp, "result.md");
  const common = ["--structure", path.join(input, "structure.yaml"), "--interface", path.join(input, "interface.yaml"), "--output", output, "--workspace-root", fixtureRoot];
  await writeFile(output, "previous\n", { mode: 0o640 }); await chmod(output, 0o640);
  const success = cli([...common, "--compare", path.join(corpus, "p-sup/expected.md")]);
  assert.equal(success.status, 0, success.stderr);
  assert.equal(JSON.parse(success.stdout).compare_match, true);
  assert.deepEqual(await readFile(output), await readFile(path.join(corpus, "p-sup/expected.md")));
  assert.equal((await lstat(output)).mode & 0o777, 0o640);
  const mismatch = cli([...common, "--compare", path.join(corpus, "p-rep/expected.md")]);
  assert.equal(mismatch.status, 1); assert.equal(JSON.parse(mismatch.stdout).compare_match, false);
  assert.deepEqual(await readFile(output), await readFile(path.join(corpus, "p-sup/expected.md")));
  const badStructure = path.join(temp, "bad.yaml");
  const original = await readFile(path.join(input, "structure.yaml"), "utf8");
  await writeFile(badStructure, original.replace("6b26d4dd8294be02d8fc8f462af9f3c505142fb5c060ef31529f9a28fcf48d9e", "f".repeat(64)));
  await writeFile(output, "preserved\n");
  const refused = cli([...common.slice(0, 1), badStructure, ...common.slice(2)]);
  assert.equal(refused.status, 1); assert.match(refused.stderr, /source_mismatch/);
  assert.equal(await readFile(output, "utf8"), "preserved\n");
  const referent = path.join(temp, "referent.md"); await writeFile(referent, "referent\n");
  await writeFile(output, "regular\n");
  await unlink(output);
  await symlink(referent, output);
  const symlinkRefusal = cli(common);
  assert.equal(symlinkRefusal.status, 1); assert.match(symlinkRefusal.stderr, /symlinks are refused/);
  assert.equal(await readFile(referent, "utf8"), "referent\n");
});

await test("CLI refuses invalid UTF-8 bytes in either YAML input without replacing output", async () => {
  const temp = await mkdtemp(path.join(os.tmpdir(), "work-engine-c2-invalid-utf8-"));
  const input = path.join(fixtureRoot, "app-server/migrations/skills/repo-search");
  const output = path.join(temp, "result.md");
  const structure = path.join(input, "structure.yaml");
  const skillInterface = path.join(input, "interface.yaml");
  const original = Buffer.from("previous output\n");
  for (const [flag, originalPath] of [["--structure", structure], ["--interface", skillInterface]]) {
    const invalid = path.join(temp, `${flag.slice(2)}.yaml`);
    await writeFile(invalid, Buffer.concat([await readFile(originalPath), Buffer.from([0x0a, 0x23, 0x20, 0xff, 0x0a])]));
    await writeFile(output, original);
    const args = ["--structure", structure, "--interface", skillInterface, "--output", output, "--workspace-root", fixtureRoot];
    args[args.indexOf(flag) + 1] = invalid;
    const result = cli(args);
    assert.equal(result.status, 1, result.stderr);
    assert.match(result.stderr, /must be UTF-8/);
    assert.equal(result.stdout, "");
    assert.deepEqual(await readFile(output), original);
    assert.ok(!(await readdir(temp)).some((name) => name.endsWith(".tmp")));
  }
});

await test("CLI detects an intervening output symlink and cleans an interrupted temporary file", async () => {
  const temp = await mkdtemp(path.join(os.tmpdir(), "work-engine-c2-publish-race-"));
  const input = path.join(fixtureRoot, "app-server/migrations/skills/slice-supervisor");
  const output = path.join(temp, "result.md");
  const referent = path.join(temp, "referent.md");
  const args = [path.join(repo, "app-server/scripts/compile-skill.mjs"), "--structure", path.join(input, "structure.yaml"), "--interface", path.join(input, "interface.yaml"), "--output", output, "--workspace-root", fixtureRoot];
  const environment = { ...env(), WORK_ENGINE_C2_TEST_BEFORE_RENAME_PAUSE_MS: "500" };
  const run = () => {
    const child = spawn(process.execPath, args, { cwd: repo, env: environment, stdio: ["ignore", "pipe", "pipe"] });
    let stderr = ""; child.stderr.on("data", (chunk) => { stderr += chunk; });
    return { child, stderr: () => stderr, done: new Promise((resolve) => child.on("close", resolve)) };
  };
  async function waitForTemp() {
    for (let attempt = 0; attempt < 100; attempt++) {
      if ((await readdir(temp)).some((name) => name.startsWith(".result.md.") && name.endsWith(".tmp"))) return;
      await new Promise((resolve) => setTimeout(resolve, 10));
    }
    assert.fail("CLI temporary file did not appear");
  }
  await writeFile(output, "old\n"); await writeFile(referent, "referent\n");
  const first = run(); await waitForTemp(); await unlink(output); await symlink(referent, output);
  assert.equal(await first.done, 1); assert.match(first.stderr(), /symlinks are refused/);
  assert.equal(await readFile(referent, "utf8"), "referent\n");
  assert.ok(!(await readdir(temp)).some((name) => name.endsWith(".tmp")));
  await unlink(output); await writeFile(output, "old again\n");
  const second = run(); await waitForTemp(); second.child.kill("SIGTERM");
  assert.equal(await second.done, 1);
  assert.equal(await readFile(output, "utf8"), "old again\n");
  assert.ok(!(await readdir(temp)).some((name) => name.endsWith(".tmp")));
});

await test("Rust selection fails on missing/wrong binary, malformed Unicode, and unknown selector without fallback", async () => {
  const { structureSource, interfaceSource } = await sources("p-rep");
  await assert.rejects(compileRustSkill({ structureSource, interfaceSource, workspaceRoot: fixtureRoot, binaryPath: binary, expectedBinarySha256: "f".repeat(64) }), /digest mismatch/);
  await assert.rejects(compileRustSkill({ structureSource, interfaceSource, workspaceRoot: fixtureRoot, binaryPath: path.join(fixtureRoot, "missing-binary"), expectedBinarySha256: binaryDigest }), /ENOENT/);
  await assert.rejects(compileRustSkill({ structureSource, interfaceSource, workspaceRoot: fixtureRoot, binaryPath: binary, expectedBinarySha256: binaryDigest, verifySources: false, signal: AbortSignal.abort() }), /aborted|cancelled/i);
  await assert.rejects(compileRustSkill({ structureSource: "\ud800", interfaceSource, workspaceRoot: fixtureRoot, binaryPath: binary, expectedBinarySha256: binaryDigest }), /unsupported Unicode/);
  const prior = process.env.WORK_ENGINE_COMPILER_BACKEND;
  process.env.WORK_ENGINE_COMPILER_BACKEND = "not-a-backend";
  try { await assert.rejects(compileSkill({ structureSource, interfaceSource, workspaceRoot: fixtureRoot }), /unknown compiler backend/); }
  finally { if (prior === undefined) delete process.env.WORK_ENGINE_COMPILER_BACKEND; else process.env.WORK_ENGINE_COMPILER_BACKEND = prior; }
  const priorSelection = { backend: process.env.WORK_ENGINE_COMPILER_BACKEND, binary: process.env.WORK_ENGINE_COMPILER_RUST_BINARY, digest: process.env.WORK_ENGINE_COMPILER_RUST_BINARY_SHA256 };
  Object.assign(process.env, { WORK_ENGINE_COMPILER_BACKEND: "rust", WORK_ENGINE_COMPILER_RUST_BINARY: binary, WORK_ENGINE_COMPILER_RUST_BINARY_SHA256: "f".repeat(64) });
  try {
    await assert.rejects(compileSkill({ structureSource, interfaceSource, workspaceRoot: fixtureRoot }), /digest mismatch/);
    await assert.rejects(compileSkill({ structureSource, interfaceSource, workspaceRoot: fixtureRoot, agentEnvironmentGraphAdapter: {} }), /does not accept a legacy AEG adapter injection/);
  } finally {
    for (const [key, value] of [["WORK_ENGINE_COMPILER_BACKEND", priorSelection.backend], ["WORK_ENGINE_COMPILER_RUST_BINARY", priorSelection.binary], ["WORK_ENGINE_COMPILER_RUST_BINARY_SHA256", priorSelection.digest]]) {
      if (value === undefined) delete process.env[key]; else process.env[key] = value;
    }
  }
});

await test("Rust compilation leaves the Node event loop responsive", async () => {
  const { structureSource, interfaceSource } = await sources("p-sup");
  let ticked = false;
  const timer = setTimeout(() => { ticked = true; }, 0);
  const result = await compileRustSkill({ structureSource, interfaceSource, workspaceRoot: fixtureRoot, binaryPath: binary, expectedBinarySha256: binaryDigest });
  clearTimeout(timer);
  assert.equal(ticked, true);
  assert.equal(result.ir.runtime_requirements.verified_sources, true);
});

await test("selected fake executable cannot forge correlation, producer, output or JSON closure", async () => {
  const { structureSource, interfaceSource } = await sources("p-rep");
  const real = (await rust("p-rep")).result;
  const { rust_binary_sha256: _digest, rust_binary_source: _source, ...runtimeVerification } = real.verification;
  const validResponse = { schema_version: 2, request_id: "replaced", status: "ok", ir: real.ir, output_base64: real.output.toString("base64"), verification: runtimeVerification };
  const temp = await mkdtemp(path.join(os.tmpdir(), "work-engine-c2-fake-"));
  async function fake(body) {
    const file = path.join(temp, `fake-${Math.random().toString(16).slice(2)}.mjs`);
    await writeFile(file, `#!/usr/bin/env node\nlet s='';for await(const c of process.stdin)s+=c;const request=JSON.parse(s);${body}\n`, { mode: 0o700 });
    await chmod(file, 0o700);
    const bytes = await readFile(file);
    return { binaryPath: file, expectedBinarySha256: digest(bytes) };
  }
  for (const body of [
    `process.stdout.write('{"schema_version":2,"request_id":"wrong","status":"ok"}\\n')`,
    `process.stdout.write('{"schema_version":2,"request_id":"'+request.request_id+'","status":"ok","ir":{},"output_base64":"","verification":{},"extra":1}\\n')`,
    `process.stdout.write('{"schema_version":2,"request_id":"'+request.request_id+'","request_id":"'+request.request_id+'","status":"ok"}\\n')`,
    `process.stdout.write('{}\\n{}\\n')`,
    `process.stdout.write('{"schema_version":3,"request_id":"'+request.request_id+'","status":"ok","ir":{},"output_base64":"","verification":{}}\\n')`,
  ]) {
    const selected = await fake(body);
    await assert.rejects(compileRustSkill({ structureSource, interfaceSource, workspaceRoot: fixtureRoot, verifySources: false, ...selected }));
  }
  const wrongCorrelation = await fake(`const r=${JSON.stringify(validResponse)};process.stdout.write(JSON.stringify(r)+'\\n')`);
  await assert.rejects(compileRustSkill({ structureSource, interfaceSource, workspaceRoot: fixtureRoot, verifySources: true, ...wrongCorrelation }), /correlation|exit mismatch/);
  const wrongHash = await fake(`const r=${JSON.stringify(validResponse)};r.request_id=request.request_id;r.ir.output_sha256='f'.repeat(64);process.stdout.write(JSON.stringify(r)+'\\n')`);
  await assert.rejects(compileRustSkill({ structureSource, interfaceSource, workspaceRoot: fixtureRoot, verifySources: true, ...wrongHash }), /output digest mismatch/);
  const successWithStderr = await fake(`const r=${JSON.stringify(validResponse)};r.request_id=request.request_id;process.stdout.write(JSON.stringify(r)+'\\n');process.stderr.write('unexpected')`);
  await assert.rejects(compileRustSkill({ structureSource, interfaceSource, workspaceRoot: fixtureRoot, verifySources: true, ...successWithStderr }), /success response or exit mismatch/);
  const successWithExitOne = await fake(`const r=${JSON.stringify(validResponse)};r.request_id=request.request_id;process.stdout.write(JSON.stringify(r)+'\\n');process.exitCode=1`);
  await assert.rejects(compileRustSkill({ structureSource, interfaceSource, workspaceRoot: fixtureRoot, verifySources: true, ...successWithExitOne }), /success response or exit mismatch/);
  const truncated = await fake(`process.stdout.write('{')`);
  await assert.rejects(compileRustSkill({ structureSource, interfaceSource, workspaceRoot: fixtureRoot, verifySources: true, ...truncated }), /malformed JSON/);
  const oversized = await fake(`process.stdout.write('x'.repeat(16*1024*1024+1))`);
  await assert.rejects(compileRustSkill({ structureSource, interfaceSource, workspaceRoot: fixtureRoot, verifySources: true, ...oversized }), /exceeds limit/);
  const slow = await fake(`setTimeout(()=>{},30000)`);
  const abort = new AbortController();
  setTimeout(() => abort.abort(new Error("test cancellation")), 50);
  const start = performance.now();
  await assert.rejects(compileRustSkill({ structureSource, interfaceSource, workspaceRoot: fixtureRoot, verifySources: false, signal: abort.signal, ...slow }), /test cancellation/);
  assert.ok(performance.now() - start < 6000);
  const emergency = await fake(`setTimeout(()=>{},30000)`);
  await assert.rejects(compileRustSkill({ structureSource, interfaceSource, workspaceRoot: fixtureRoot, verifySources: false, timeoutMs: 50, ...emergency }), /timeout/);
});
