import { readFile } from "node:fs/promises";
import path from "node:path";

import { compileSkill } from "./skill-compiler.mjs";
import { canonicalJson, extensionDigest, validateRunEnvelope } from "./run-extension-bundle-contract.mjs";

const FORBIDDEN_EFFECTS = new Set([
  "filesystem.write", "network.install", "publication", "claims", "review", "human", "production",
]);

export async function compileRunExtensionBundle(bundle, {
  workspaceRoot, repositoryRevision, allowedCapabilities = [], allowedAdapters = [], allowedProviders = [],
} = {}) {
  if (!bundle || bundle.schema_version !== 1 || typeof bundle.bundle_id !== "string"
      || !/^[0-9a-f]{40,64}$/.test(bundle.source_revision ?? "")) {
    throw new TypeError("extension bundle identity is invalid");
  }
  if (bundle.source_revision !== repositoryRevision) {
    throw new Error("extension bundle source revision is not the admitted repository revision");
  }
  const run = validateRunEnvelope(bundle.run);
  if (!run.repository_subject.includes(repositoryRevision)
      || !run.checkout.includes(repositoryRevision)) {
    throw new Error("extension run repository binding differs from admitted revision");
  }
  const allowed = new Set(allowedCapabilities);
  const capabilities = [...new Set(bundle.capabilities ?? [])].sort();
  if (capabilities.some((value) => !allowed.has(value))) {
    throw new Error("extension capability exceeds host admission");
  }
  if ((bundle.effects ?? []).some((value) => FORBIDDEN_EFFECTS.has(value))) {
    throw new Error("extension requests prohibited authority");
  }
  if ((bundle.authority_requests ?? []).length > 0) {
    throw new Error("extension cannot acquire production authority");
  }
  if ((bundle.adapters ?? []).some((value) => !allowedAdapters.includes(value))) {
    throw new Error("extension adapter is not host mediated");
  }
  if ((bundle.providers ?? []).some((value) => !allowedProviders.includes(value))) {
    throw new Error("extension provider requirement is unavailable");
  }
  const compiledSkills = [];
  for (const skill of bundle.skills ?? []) {
    const structurePath = path.resolve(workspaceRoot, skill.structure);
    const interfacePath = path.resolve(workspaceRoot, skill.interface);
    const structureSource = await readFile(structurePath, "utf8");
    const interfaceSource = await readFile(interfacePath, "utf8");
    if (extensionDigest(structureSource) !== skill.structure_sha256
        || extensionDigest(interfaceSource) !== skill.interface_sha256) {
      throw new Error("extension skill compiler input digest mismatch");
    }
    const compiled = await compileSkill({ structureSource, interfaceSource, workspaceRoot, verifySources: false });
    compiledSkills.push({
      skill_id: compiled.ir.skill_id,
      input_sha256: compiled.ir.input_sha256,
      output_sha256: compiled.ir.output_sha256,
      runtime_requirements: compiled.ir.runtime_requirements,
      output: compiled.output,
    });
  }
  if (compiledSkills.length === 0) throw new TypeError("extension bundle requires compiled skills");
  const descriptor = {
    schema_version: 1, bundle_id: bundle.bundle_id, source_revision: bundle.source_revision,
    run, precedence: bundle.precedence ?? "below_core",
    capabilities, effects: [...new Set(bundle.effects ?? [])].sort(),
    adapters: [...new Set(bundle.adapters ?? [])].sort(),
    providers: [...new Set(bundle.providers ?? [])].sort(),
    dependencies: structuredClone(bundle.dependencies ?? []),
    registry: structuredClone(bundle.registry ?? []), compiled_skills: compiledSkills,
  };
  for (const dependency of descriptor.dependencies) {
    if (dependency.mode !== "sealed_cache") throw new Error("extension dependency requires network");
  }
  const registryNames = new Set();
  for (const entry of descriptor.registry) {
    if (!capabilities.includes(entry.capability) || !descriptor.adapters.includes(entry.adapter)) {
      throw new Error("extension registry entry is not admitted");
    }
    const key = `${run.namespace}:${entry.name}`;
    if (registryNames.has(key)) throw new Error("extension registry conflict");
    registryNames.add(key);
  }
  const sha256 = extensionDigest(descriptor);
  return Object.freeze({ ...descriptor, sha256, canonical_json: canonicalJson(descriptor) });
}
