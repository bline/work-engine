import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { mkdtemp, readFile, rm } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";

import { ExperimentalReviewBenchService } from "../../../src/index.mjs";

const ROOT=path.resolve(new URL("../../../..",import.meta.url).pathname), PACKAGE=path.join(ROOT,"skills/review-bench"), FIXTURES=path.join(PACKAGE,"tests/fixtures"), SCRIPT=path.join(PACKAGE,"scripts/review_bench.py");
const json=async(relative)=>JSON.parse(await readFile(path.join(FIXTURES,relative),"utf8"));

test("App Server comparison is byte-semantically equal to legacy fixture report",async()=>{const corpus=await json("corpus.json"),truth=await json("truth.json"),results=await Promise.all(["claude-clean.json","claude-defect.json","web-sol-clean.json","web-sol-defect.json"].map((name)=>json(`results/${name}`))),scorings=await Promise.all(["claude-clean.json","claude-defect.json","web-sol-clean.json","web-sol-defect.json"].map((name)=>json(`scoring/${name}`)));const actual=new ExperimentalReviewBenchService().compare({corpus,truth,results,scorings});const expected=JSON.parse(execFileSync("python3",[SCRIPT,"compare","--corpus",path.join(FIXTURES,"corpus.json"),"--truth",path.join(FIXTURES,"truth.json"),"--results-dir",path.join(FIXTURES,"results"),"--scoring-dir",path.join(FIXTURES,"scoring")],{encoding:"utf8"}));assert.deepEqual(actual,expected);});

test("App Server blinded prompt is exactly the legacy prompt",async(t)=>{const directory=await mkdtemp(path.join(os.tmpdir(),"review-bench-parity."));t.after(()=>rm(directory,{recursive:true,force:true}));execFileSync("python3",[SCRIPT,"export-case","--corpus",path.join(FIXTURES,"corpus.json"),"--case-id","defective-state","--output-dir",directory]);const expected=await readFile(path.join(directory,"review-prompt.md"),"utf8"),actual=new ExperimentalReviewBenchService().renderCase({corpus:await json("corpus.json"),caseId:"defective-state"}).prompt;assert.equal(actual,expected);});
