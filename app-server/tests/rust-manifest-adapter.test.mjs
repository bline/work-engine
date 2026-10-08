import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { chmod, mkdtemp, readFile, readdir, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";
import { ManifestRoleRuntime, RuntimeManifest, projectRuntimeManifest, satisfyRuntimeRequirements } from "../src/runtime-manifest.mjs";
import { invokeRustManifest, observeRustManifestSelection } from "../src/rust-manifest-adapter.mjs";

const root = fileURLToPath(new URL("../..", import.meta.url));
const fixture = JSON.parse(await readFile(new URL("fixtures/compiler-c3/projection-oracle.json", import.meta.url)));
const releaseBinary = process.env.WORK_ENGINE_COMPILER_RUST_BINARY ?? path.join(root, "rust/target/release/work-engine-compiler");
let digest;
try { digest = createHash("sha256").update(await readFile(releaseBinary)).digest("hex"); } catch { digest = null; }
function rustOptions() {
  if (!digest) throw new Error("C3 test requires WORK_ENGINE_COMPILER_RUST_BINARY release executable");
  return { ...fixture.options, compilerSelection: { binaryPath: releaseBinary, expectedSha256: digest, protocolVersion: 3, launchProfileId: "manifest-sync-v1" } };
}
function setup(t) {
  const old = process.env.WORK_ENGINE_COMPILER_BACKEND;
  process.env.WORK_ENGINE_COMPILER_BACKEND = "legacy";
  t.after(() => { if (old == null) delete process.env.WORK_ENGINE_COMPILER_BACKEND; else process.env.WORK_ENGINE_COMPILER_BACKEND = old; });
}
test("frozen manifest projection and six satisfaction receipts match legacy", (t) => {
  setup(t);
  const legacy = projectRuntimeManifest(fixture.document, fixture.options);
  assert.deepEqual({manifestId:legacy.manifestId,source:legacy.source,roles:legacy.roles,requirementsBaseDirectory:legacy.requirementsBaseDirectory},fixture.oracle);
  const rust = projectRuntimeManifest(fixture.document, rustOptions());
  assert.ok(rust instanceof RuntimeManifest);
  assert.deepEqual(rust,legacy);
  assert.equal(Object.isFrozen(rust.roles),true);
  assert.equal(Object.isFrozen(rust.projectRole("slice-supervisor","one")),true);
  assert.deepEqual(rust.roleIds,Object.keys(fixture.oracle.roles));
  for(const entry of fixture.satisfactionCases) {
    const receipt=satisfyRuntimeRequirements({manifest:rust,roleId:entry.roleId,requirements:entry.requirements,skillName:entry.skillName});
    assert.deepEqual(receipt,entry.receipt);
    assert.equal(Object.isFrozen(receipt),true);
  }
  const observation=observeRustManifestSelection(rustOptions().compilerSelection);
  assert.deepEqual(observation,{binaryPath:releaseBinary,observedSha256:digest,protocolVersion:3,launchProfileId:"manifest-sync-v1"});
  assert.equal(Object.isFrozen(observation),true);
});
test("Rust selection refuses altered grants before fake delivery", async (t) => {
  setup(t);
  const options=rustOptions();
  const manifest=projectRuntimeManifest(fixture.document,options);
  let deliveries=0;
  const adapter={deliverTurn:async()=>{deliveries++;return {turnId:"one"}}};
  const runtime=new ManifestRoleRuntime({adapter,manifest});
  const success=await runtime.deliverTurn({roleId:"slice-supervisor",instanceId:"one"});
  assert.equal(deliveries,1);
  assert.equal(success.runtimeSatisfaction.sha256,fixture.satisfactionCases.find(({roleId,skillName})=>roleId === "slice-supervisor" && skillName === null).receipt.sha256);
  const doc=structuredClone(fixture.document);
  doc.roles["slice-supervisor"].effects.push("state.user_history");
  const excessive=projectRuntimeManifest(doc,options);
  await assert.rejects(new ManifestRoleRuntime({adapter,manifest:excessive}).deliverTurn({roleId:"slice-supervisor",instanceId:"one"}),/effect ceiling|prohibited effect/);
  assert.equal(deliveries,1);
  const bad=structuredClone(manifest.roles["slice-supervisor"].runtimeRequirements);
  bad.contract.path="wrong-path";
  await assert.rejects(new ManifestRoleRuntime({adapter,manifest,runtimeRequirements:{"slice-supervisor":bad}}).deliverTurn({roleId:"slice-supervisor",instanceId:"one"}),/digest mismatch/);
  assert.equal(deliveries,1);
});
test("captured selection rejects missing, wrong-hash, nonexecutable and mutated binary", async (t)=>{
  setup(t);
  const directory=await mkdtemp(path.join(os.tmpdir(),"c3-select-"));t.after(()=>rm(directory,{recursive:true,force:true}));
  const binaryPath=path.join(directory,"compiler");
  const options=rustOptions();
  options.compilerSelection.binaryPath=binaryPath;
  assert.throws(()=>projectRuntimeManifest(fixture.document,options),{code:"binary_unavailable"});
  await writeFile(binaryPath,await readFile(releaseBinary));await chmod(binaryPath,0o700);
  options.compilerSelection.expectedSha256="0".repeat(64);
  assert.throws(()=>projectRuntimeManifest(fixture.document,options),{code:"binary_mismatch"});
  options.compilerSelection.expectedSha256=digest;
  const selected=projectRuntimeManifest(fixture.document,options);
  options.compilerSelection.expectedSha256="0".repeat(64);
  assert.deepEqual(satisfyRuntimeRequirements({manifest:selected,roleId:"slice-supervisor",requirements:selected.roles["slice-supervisor"].runtimeRequirements}),fixture.satisfactionCases.find(({roleId,skillName})=>roleId === "slice-supervisor" && skillName === null).receipt);
  await chmod(binaryPath,0o600);
  assert.throws(()=>projectRuntimeManifest(fixture.document,{...options,compilerSelection:{...options.compilerSelection,expectedSha256:digest}}),{code:"binary_unavailable"});
});
test("non-JSON direct-object values refuse before process", (t)=>{
  setup(t);const o=rustOptions();
  for(const value of [undefined,Infinity,1n,()=>{},Symbol("x")]){
    const d=structuredClone(fixture.document);d.roles["slice-supervisor"].unknown=value;
    assert.throws(()=>projectRuntimeManifest(d,o),TypeError);
  }
  const proxied=structuredClone(fixture.document);proxied.roles["slice-supervisor"].trap=new Proxy({},{});
  assert.throws(()=>projectRuntimeManifest(proxied,o),/proxy input is unsupported/);
  const d=structuredClone(fixture.document);Object.defineProperty(d.roles["slice-supervisor"],"trap",{enumerable:true,get(){throw new Error("getter invoked")}});
  assert.throws(()=>projectRuntimeManifest(d,o),/accessor is unsupported/);
});

test("v3 process failures discard projections and reap timed-out direct child", async (t) => {
  setup(t);
  const directory = await mkdtemp(path.join(os.tmpdir(), "c3-protocol-"));
  t.after(() => rm(directory, { recursive: true, force: true }));
  const oldTmp = process.env.TMPDIR;
  process.env.TMPDIR = directory;
  t.after(() => { if (oldTmp == null) delete process.env.TMPDIR; else process.env.TMPDIR = oldTmp; });
  const pidFile = path.join(directory, "pid");
  const scripts = [
    ["wrong-correlation", `const r=JSON.parse(require('fs').readFileSync(0,'utf8'));process.stdout.write(JSON.stringify({schema_version:3,request_id:'wrong',operation:r.operation,status:'ok',producer:'work-engine.manifest-compiler.rust-v1',result:{}}));`, "protocol_error"],
    ["wrong-producer", `const r=JSON.parse(require('fs').readFileSync(0,'utf8'));process.stdout.write(JSON.stringify({schema_version:3,request_id:r.request_id,operation:r.operation,status:'ok',producer:'wrong',result:{}}));`, "protocol_error"],
    ["duplicate-json", `process.stdout.write('{"schema_version":3,"schema_version":3}');`, "protocol_error"],
    ["trailing-json", `process.stdout.write('{}{}');`, "protocol_error"],
    ["invalid-utf8", `process.stdout.write(Buffer.from([0xff]));`, "protocol_error"],
    ["malformed-error", `const r=JSON.parse(require('fs').readFileSync(0,'utf8'));process.stdout.write(JSON.stringify({schema_version:3,request_id:r.request_id,operation:r.operation,status:'error',producer:'work-engine.manifest-compiler.rust-v1',error:{code:'bad'}}));process.exit(2);`, "protocol_error"],
    ["stderr", `const r=JSON.parse(require('fs').readFileSync(0,'utf8'));process.stderr.write('noise');process.stdout.write(JSON.stringify({schema_version:3,request_id:r.request_id,operation:r.operation,status:'ok',producer:'work-engine.manifest-compiler.rust-v1',result:{}}));`, "protocol_error"],
    ["nonzero-after-success", `const r=JSON.parse(require('fs').readFileSync(0,'utf8'));process.stdout.write(JSON.stringify({schema_version:3,request_id:r.request_id,operation:r.operation,status:'ok',producer:'work-engine.manifest-compiler.rust-v1',result:{}}));process.exitCode=1;`, "protocol_error"],
    ["timeout", `require('fs').writeFileSync(${JSON.stringify(pidFile)},String(process.pid));while(true){}`, "timeout"],
    ["overflow", `process.stdout.write('x'.repeat(${4 * 1024 * 1024 + 100}));`, "resource_limit"],
  ];
  for (const [name, script, code] of scripts) {
    const file = path.join(directory, name);
    await writeFile(file, `#!/usr/bin/node\n${script}\n`);
    await chmod(file, 0o700);
    const expectedSha256 = createHash("sha256").update(await readFile(file)).digest("hex");
    const opts = { ...fixture.options, compilerSelection: { binaryPath: file, expectedSha256, protocolVersion: 3, launchProfileId: "manifest-sync-v1" } };
    assert.throws(() => projectRuntimeManifest(fixture.document, opts), { code }, name);
    assert.equal((await readdir(directory)).some((entry) => entry.startsWith("work-engine-manifest-")), false, `${name} scratch cleanup`);
    if (name === "timeout") {
      const pid = Number(await readFile(pidFile, "utf8"));
      assert.throws(() => process.kill(pid, 0), { code: "ESRCH" });
    }
  }
});


test("capture bounds oversized strings and small shared DAGs before binary access", { timeout: 5000 }, (t) => {
  setup(t);
  const missing = { binaryPath: "/absent/c3-compiler", expectedSha256: "0".repeat(64), protocolVersion: 3, launchProfileId: "manifest-sync-v1" };
  const oversized = { ...fixture.options, runtimeRequirementsByRole: { "slice-supervisor": { opaque: "x".repeat(4 * 1024 * 1024) } }, compilerSelection: missing };
  const started = performance.now();
  assert.throws(() => projectRuntimeManifest(fixture.document, oversized), { code: "resource_limit" });
  let shared = ["x"];
  for (let i = 0; i < 23; i++) shared = [shared, shared];
  const expanded = { ...fixture.options, runtimeRequirementsByRole: { "slice-supervisor": { opaque: shared } }, compilerSelection: missing };
  assert.throws(() => projectRuntimeManifest(fixture.document, expanded), { code: "resource_limit" });
  assert.ok(performance.now() - started < 1000, "bounded capture must avoid exponential cloning");
  const payload = { document: { opaque: "" }, options: {} };
  const base = Buffer.byteLength(JSON.stringify({ schema_version: 3, request_id: "0".repeat(36), operation: "project_runtime_manifest", ...payload }));
  payload.document.opaque = "x".repeat(4 * 1024 * 1024 - base);
  assert.equal(Buffer.byteLength(JSON.stringify({ schema_version: 3, request_id: "0".repeat(36), operation: "project_runtime_manifest", ...payload })), 4 * 1024 * 1024);
  assert.throws(() => invokeRustManifest("project_runtime_manifest", payload, missing), { code: "binary_unavailable" }, "exact 4 MiB request remains admitted through capture");
  payload.document.opaque += "x";
  assert.throws(() => invokeRustManifest("project_runtime_manifest", payload, missing), { code: "resource_limit" });
});

test("malicious child nested success and error-code/exit mismatches are protocol failures", async (t) => {
  setup(t);
  const directory = await mkdtemp(path.join(os.tmpdir(), "c3-malicious-"));
  t.after(() => rm(directory, { recursive: true, force: true }));
  const oraclePath = fileURLToPath(new URL("fixtures/compiler-c3/projection-oracle.json", import.meta.url));
  const defects = [
    ["null-contract", "v.roles['slice-supervisor'].roleContract.path=null;"],
    ["nonstring-grant", "v.roles['slice-supervisor'].capabilities[0]=123;"],
    ["bad-compiled-metadata", "v.roles['slice-builder'].skills.find(s=>s.name==='repo-search').compiledEnvironment.interface=null;"],
    ["extra-skill-field", "v.roles['slice-builder'].skills.find(s=>s.name==='repo-search').authority=true;"],
    ["deep-requirement", "let nested=0;for(let i=0;i<70;i++)nested=[nested];v.roles['slice-supervisor'].runtimeRequirements.opaque=nested;"],
  ];
  let deliveries = 0;
  const adapter = { deliverTurn: async () => { deliveries++; return {}; } };
  for (const [name, defect] of defects) {
    const script = `const fs=require('fs');const r=JSON.parse(fs.readFileSync(0,'utf8'));const v=JSON.parse(fs.readFileSync(${JSON.stringify(oraclePath)},'utf8')).oracle;${defect}process.stdout.write(JSON.stringify({schema_version:3,request_id:r.request_id,operation:r.operation,status:'ok',producer:'work-engine.manifest-compiler.rust-v1',result:v}));`;
    const file = path.join(directory, name);
    await writeFile(file, `#!/usr/bin/node\n${script}\n`);
    await chmod(file, 0o700);
    const expectedSha256 = createHash("sha256").update(await readFile(file)).digest("hex");
    const options = { ...fixture.options, compilerSelection: { binaryPath: file, expectedSha256, protocolVersion: 3, launchProfileId: "manifest-sync-v1" } };
    assert.throws(() => {
      const manifest = projectRuntimeManifest(fixture.document, options);
      return new ManifestRoleRuntime({ adapter, manifest });
    }, { code: "protocol_error" }, name);
  }
  assert.equal(deliveries, 0);
  for (const [name, code, exit] of [["unknown-code", "arbitrary_domain", 2], ["wrong-domain-exit", "invalid_manifest", 1], ["wrong-host-exit", "timeout", 2]]) {
    const script = `const r=JSON.parse(require('fs').readFileSync(0,'utf8'));process.stdout.write(JSON.stringify({schema_version:3,request_id:r.request_id,operation:r.operation,status:'error',producer:'work-engine.manifest-compiler.rust-v1',error:{code:${JSON.stringify(code)},path:null,message:'refused'}}));process.exit(${exit});`;
    const file = path.join(directory, name);
    await writeFile(file, `#!/usr/bin/node\n${script}\n`);
    await chmod(file, 0o700);
    const expectedSha256 = createHash("sha256").update(await readFile(file)).digest("hex");
    assert.throws(() => projectRuntimeManifest(fixture.document, { ...fixture.options, compilerSelection: { binaryPath: file, expectedSha256, protocolVersion: 3, launchProfileId: "manifest-sync-v1" } }), { code: "protocol_error" }, name);
  }
});

test("correlated numeric ID, nonfinite number, and lone surrogate child responses refuse", async (t) => {
  setup(t);
  const directory = await mkdtemp(path.join(os.tmpdir(), "c3-scalars-"));
  t.after(() => rm(directory, { recursive: true, force: true }));
  const oraclePath = fileURLToPath(new URL("fixtures/compiler-c3/projection-oracle.json", import.meta.url));
  for (const [name, defect, numericId] of [
    ["numeric-id", "v.manifestId=123;v.source.manifestId=123;", true],
    ["nonfinite", "v.roles['slice-supervisor'].runtimeRequirements.opaque='__OPAQUE_NUMBER__';", false],
    ["lone-surrogate", "v.roles['slice-supervisor'].runtimeRequirements.opaque='\\uD800';", false],
  ]) {
    const script = `const fs=require('fs');const r=JSON.parse(fs.readFileSync(0,'utf8'));const v=JSON.parse(fs.readFileSync(${JSON.stringify(oraclePath)},'utf8')).oracle;${defect}let out=JSON.stringify({schema_version:3,request_id:r.request_id,operation:r.operation,status:'ok',producer:'work-engine.manifest-compiler.rust-v1',result:v});if(${JSON.stringify(name === "nonfinite")})out=out.replace('"__OPAQUE_NUMBER__"','1e999');process.stdout.write(out);`;
    const file = path.join(directory, name);
    await writeFile(file, `#!/usr/bin/node\n${script}\n`);
    await chmod(file, 0o700);
    const expectedSha256 = createHash("sha256").update(await readFile(file)).digest("hex");
    const document = structuredClone(fixture.document);
    if (numericId) document.manifest_id = 123;
    assert.throws(() => projectRuntimeManifest(document, { ...fixture.options, compilerSelection: { binaryPath: file, expectedSha256, protocolVersion: 3, launchProfileId: "manifest-sync-v1" } }), { code: "protocol_error" }, name);
  }
});

test("opaque JSON requirement values retain the exact legacy projection profile", (t) => {
  setup(t);
  for (const opaque of [false, 0, "opaque", ["🚀", { finite: 1.25 }], { finite: 1.25, unicode: "🚀" }]) {
    const options = { ...fixture.options, runtimeRequirementsByRole: { "slice-supervisor": opaque, "slice-builder:repo-search": opaque } };
    const legacy = projectRuntimeManifest(fixture.document, options);
    const rust = projectRuntimeManifest(fixture.document, { ...options, compilerSelection: rustOptions().compilerSelection });
    assert.deepEqual(rust, legacy);
    assert.deepEqual(rust.roles["slice-supervisor"].runtimeRequirements, opaque);
    assert.deepEqual(rust.roles["slice-builder"].skills.find((skill) => skill.name === "repo-search").runtimeRequirements, opaque);
  }
});
