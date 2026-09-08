import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";
import path from "node:path";
import test from "node:test";

import { compileRunExtensionBundle, createExperimentalReviewBenchAdapter, createRunExtensionRegistry, extensionDigest } from "../src/index.mjs";
import { compileSkill } from "../src/skill-compiler.mjs";

const ROOT=path.resolve(new URL("../..",import.meta.url).pathname), LEGACY=path.join(ROOT,"skills/review-bench/tests/fixtures"), MIGRATION="app-server/migrations/skills/review-bench";
const json=async(relative)=>JSON.parse(await readFile(path.join(LEGACY,relative),"utf8"));
const sha=async(relative)=>extensionDigest(await readFile(path.join(ROOT,relative),"utf8"));
const rawSha=async(relative)=>createHash("sha256").update(await readFile(path.join(ROOT,relative))).digest("hex");

test("sealed S12E Review Bench extension dispatch stays descriptive and non-authoritative",async()=>{
  const revision=execFileSync("git",["-C",ROOT,"rev-parse","HEAD"],{encoding:"utf8"}).trim();
  const structure=`${MIGRATION}/structure.yaml`,iface=`${MIGRATION}/interface.yaml`;
  const structureSource=await readFile(path.join(ROOT,structure),"utf8"),interfaceSource=await readFile(path.join(ROOT,iface),"utf8");
  const exact=await compileSkill({structureSource,interfaceSource,workspaceRoot:ROOT,verifySources:true});
  assert.equal(exact.ir.output_sha256,await rawSha("skills/review-bench/SKILL.md"));
  const compiledText=exact.output.toString("utf8");
  assert.match(compiledText,/use the host-mediated\n`research\.review-bench\.evaluate` interface/);
  assert.match(compiledText,/do not emulate them in model reasoning/);
  assert.match(compiledText,/collapse evaluation evidence into\nthe production decision it is meant to measure/);
  assert.deepEqual(exact.ir.runtime_requirements.prohibited_effects,["state.campaign_acceptance","state.production_review","state.provider_execution","state.review_acceptance","state.review_routing","state.reviewer_profile_admission","state.truth_adjudication"]);
  const bundle={schema_version:1,bundle_id:"s13-review-bench-fixture",source_revision:revision,skills:[{structure,structure_sha256:await sha(structure),interface:iface,interface_sha256:await sha(iface)}],precedence:"below_core",capabilities:["research.review-bench.evaluate"],effects:[],authority_requests:[],adapters:["research.review-bench.evaluate"],providers:[],dependencies:[],registry:[{capability:"research.review-bench.evaluate",adapter:"research.review-bench.evaluate",name:"evaluate",description:"Validate or compare supplied immutable Review Bench artifacts without production review authority.",input_schema:{type:"object",additionalProperties:false,required:["operation","payload"],properties:{operation:{enum:["validate","compare","render_case","inventory"]},payload:{type:"object"}}}}],run:{repository_subject:`work-engine@${revision}`,checkout:`detached:${revision}`,artifact_root:"s13-review-bench/artifacts",scratch_root:"s13-review-bench/scratch",retention:"clean",credentials:"none",network:"sealed",namespace:"s13-review-bench"}};
  const attachment=await compileRunExtensionBundle(bundle,{workspaceRoot:ROOT,repositoryRevision:revision,allowedCapabilities:["research.review-bench.evaluate"],allowedAdapters:["research.review-bench.evaluate"],allowedProviders:[]});
  assert.equal(attachment.compiled_skills[0].skill_id,"review-bench");assert.equal(attachment.precedence,"below_core");assert.equal(attachment.providers.length,0);assert.equal(attachment.effects.length,0);
  const registry=createRunExtensionRegistry(attachment,new Map([["research.review-bench.evaluate",createExperimentalReviewBenchAdapter()]]));
  const results=await Promise.all(["claude-clean.json","claude-defect.json","web-sol-clean.json","web-sol-defect.json"].map((name)=>json(`results/${name}`)));
  const scorings=await Promise.all(["claude-clean.json","claude-defect.json","web-sol-clean.json","web-sol-defect.json"].map((name)=>json(`scoring/${name}`)));
  const response=await registry.bridge.dispatch({namespace:"s13-review-bench",tool:"evaluate",arguments:{operation:"compare",payload:{corpus:await json("corpus.json"),truth:await json("truth.json"),results,scorings}}});
  assert.equal(response.success,true);const report=JSON.parse(response.contentItems[0].text);assert.equal(report.status,"descriptive_only");assert.equal(report.pairwise_complementarity.length,1);
  assert.equal((await registry.bridge.dispatch({namespace:"s13-review-bench",tool:"accept_production_review",arguments:{}})).success,false);
  assert.equal(registry.specs[0].tools.some((tool)=>/accept|admit|provider|route/.test(tool.name)),false);
});
