import assert from "node:assert/strict";
import { access } from "node:fs/promises";
import test from "node:test";
import {
  createImplementationReviewService, createReviewEpisodeService,
  ImplementationReviewerRuntime, OpenRouterCodexReviewerAdapter,
  projectRuntimeManifest, ReviewerProfileRegistry,
  reviewerRuntimeDigest as digest,
} from "../src/index.mjs";

const subject={commit:"candidate",tree:"tree",patchIdentity:"patch"};
const evidence={path:"app-server/src/index.mjs",startLine:1,endLine:1,sha256:"b".repeat(64)};
const result={schemaVersion:1,subject,verdict:"acceptable_as_is",findings:[],decisiveEvidence:[evidence],limitations:[]};
function profile(){const p={schemaVersion:1,profileId:"openrouter.codex.review-v1",enabled:true,requestedModel:"openai/gpt-5.2-codex",provider:"openrouter",reasoning:"high",capabilities:["structured_output","repository_read"],outputSchema:"work-engine.implementation-review.v1",effectiveInstructions:"Review exact subject.",isolatedHome:true,limitations:["fixture"],acceptingAuthority:"accepted-plan-v1"};p.configurationDigest=digest(p);return p;}
const catalog={schemaVersion:1,catalogId:"fixture",observedAt:"2026-08-30T00:00:00Z",expiresAt:"2026-09-01T00:00:00Z",source:"fixture",sourceSha256:"a".repeat(64),models:[{slug:"openai/gpt-5.2-codex",provider:"openrouter",capabilities:["structured_output","repository_read"],routingConstraints:[]}]};
const policy={classification:"confidential",access:"episode actors",retention:"bounded projection retained",exactRetentionAuthorized:false,redaction:"raw bodies omitted",tamperEvidence:"sha256 digest"};
const ref=(owner,reference,revision,sha256)=>({owner,reference,revision,sha256,freshness:"exact immutable revision"});

test("canonical reviewer role executes admitted profile through unchanged S9 admission and retained episode",async()=>{
  const manifest=projectRuntimeManifest({schema_version:1,manifest_id:"vertical",roles:{"implementation-reviewer":{contract:"../skills/repo-search/SKILL.md",developer_instructions:"Read-only exact-subject review.",thread_options:{cwd:".",approval_policy:"never",sandbox:"read-only"},continuity:"retained",capabilities:["capability.repository_evidence","capability.direct_source_observation"],effects:[],skills:[{name:"repo-search",path:"../skills/repo-search/SKILL.md"}]}}},{baseDirectory:new URL("..",import.meta.url).pathname});
  const registry=new ReviewerProfileRegistry({profiles:[profile()]});
  const homes=[],delivered=[];
  const adapter=new OpenRouterCodexReviewerAdapter({registry,now:()=>Date.parse("2026-08-30T12:00:00Z"),executeProcess:async({args,env,input})=>{homes.push(env.CODEX_HOME);delivered.push(JSON.parse(input).instructions);return{exitCode:0,stderr:"",stdout:JSON.stringify({type:"review.completed",observed:{model:"openai/gpt-5.2-codex",provider:"openrouter",servingVariant:"fixture"},result})+"\n"};}});
  const reviewer=new ImplementationReviewerRuntime({manifest,adapter});
  const initial=await reviewer.review({instanceId:"episode",profileId:"openrouter.codex.review-v1",subject,catalogProjection:catalog,rawEventPolicy:policy});
  assert.equal(initial.isolation.freshEntry,true); assert.equal(initial.isolation.mutationAuthorized,false);
  assert.match(delivered[0],/Read-only exact-subject review/); assert.match(delivered[0],/subordinate to the canonical role instructions/); assert.match(delivered[0],/Review exact subject/); assert.match(delivered[0],/- fixture/);
  const implementationReview=createImplementationReviewService();
  assert.equal(implementationReview.admit({result:initial.result,expectedSubject:subject}).authority.independenceClaimed,false);
  assert.throws(()=>implementationReview.admit({result:{...result,verdict:"schema-drift"},expectedSubject:subject}),/verdict/);
  const episode=createReviewEpisodeService({implementationReview});
  const identity={runId:"run",sliceNumber:2,attemptId:"attempt",planVersion:"plan",reviewObligationId:"review",reviewEpisodeId:"episode"};
  const authority={schemaVersion:1,grantId:"grant",identity,source:ref("supervisor","selection","v1","c".repeat(64)),writer:{actorId:"reviewer",provider:"openrouter",generation:1,runtimeSession:ref("runtime","session-1","v1","d".repeat(64))},readers:["reviewer","builder","supervisor"],initialSubject:ref("checkpoint","candidate","v1",digest(subject)),predecessorRevision:null};
  let state=episode.begin({authority,transitionId:"begin"});
  state=episode.transition({authority,expectedRevision:state.revision,transitionId:"initial",action:"record_result",payload:{result:initial.result,unresolvedQuestions:[]}});
  assert.equal(state.phase,"reported");
  state=episode.transition({authority,expectedRevision:state.revision,transitionId:"remediation",action:"record_remediation_subject",payload:{subject:ref("checkpoint","candidate-2","v2",digest(subject))}});
  const continued=await reviewer.review({instanceId:"episode",profileId:"openrouter.codex.review-v1",subject,catalogProjection:catalog,rawEventPolicy:policy,continuationSessionId:"session-1"});
  assert.equal(continued.isolation.freshEntry,false); assert.equal(continued.isolation.continuation,true);
  assert.equal(homes[0],homes[1]);
  state=episode.transition({authority,expectedRevision:state.revision,transitionId:"reevaluate",action:"record_result",payload:{result:continued.result,unresolvedQuestions:[]}});
  assert.equal(state.continuity,"same_session"); assert.equal(state.phase,"reported");
  assert.equal(await reviewer.retire("episode"),true); await assert.rejects(access(homes[0]));
});
