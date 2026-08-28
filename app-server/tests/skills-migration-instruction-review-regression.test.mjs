import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";
import path from "node:path";
import test from "node:test";

import { behavioralPacket } from "../scripts/run-skills-migration-instruction-review-regression.mjs";
import { validateBehavioralResults, validateCorpus } from "../scripts/validate-skills-migration-instruction-review-regression.mjs";

const corpusUrl = new URL("../migrations/skills/instruction-review-regression-v1.json", import.meta.url);
const resultsUrl = new URL("../../reviews/implementations/skills-migration-instruction-review-regression/s1/behavioral-results.json", import.meta.url);
const load = async (url) => JSON.parse(await readFile(url, "utf8"));
const copy = (value) => structuredClone(value);
const sha256 = (bytes) => createHash("sha256").update(bytes).digest("hex");

test("vertical pair differs only in effective causal loading", async () => {
  const corpus = await load(corpusUrl);
  const result = validateCorpus(corpus);
  assert.equal(result.cases, 16);
  const [missing, loaded] = corpus.cases.filter((entry) => entry.pair_id === "vertical-causal-exposure");
  assert.equal(missing.command, loaded.command);
  assert.deepEqual(missing.source_catalog, loaded.source_catalog);
  assert.equal(missing.expected.missing_causal_exposure_finding, true);
  assert.equal(loaded.expected.missing_causal_exposure_finding, false);
});

const invalidCases = [
  ["traceability substituted for loading", (corpus) => { corpus.cases[0].relations.reason_loaded = true; }],
  ["vertical command drift", (corpus) => { corpus.cases[1].command = "A different command."; }],
  ["authority notice removed", (corpus) => { corpus.authority_notice = "Evidence."; }],
  ["conditional omission vocabulary widened", (corpus) => { corpus.cases[6].relations.conditional_omission = "probably_safe"; }],
];

for (const [name, mutate] of invalidCases) {
  test(`rejects ${name}`, async () => {
    const corpus = copy(await load(corpusUrl));
    mutate(corpus);
    assert.throws(() => validateCorpus(corpus));
  });
}

test("emits an identity-bound behavioral packet", async () => {
  const corpus = await load(corpusUrl);
  const packet = behavioralPacket(corpus);
  assert.equal(packet.corpus.id, corpus.corpus_id);
  assert.equal(packet.corpus.sha256, validateCorpus(corpus).corpus_digest);
  assert.equal(packet.cases.length, corpus.cases.length);
  assert.match(packet.purpose, /do not edit, accept, certify, or infer authority/i);
  for (const entry of packet.cases) {
    assert.equal(Object.hasOwn(entry, "expected"), false);
    assert.equal(Object.hasOwn(entry, "pair_id"), false);
    assert.equal(Object.hasOwn(entry, "variant"), false);
  }
});

test("checked-in fresh behavioral results cover the exact corpus", async () => {
  const corpus = await load(corpusUrl);
  const results = await load(resultsUrl);
  assert.equal(validateBehavioralResults(corpus, results).cases, corpus.cases.length);
});

test("review subject and authority bind exact candidate bytes", async () => {
  const repository = path.resolve(new URL("../..", import.meta.url).pathname);
  const subjectUrl = new URL("../../reviews/implementations/skills-migration-instruction-review-regression/s1/subject.json", import.meta.url);
  const authorityUrl = new URL("../../reviews/implementations/skills-migration-instruction-review-regression/s1/independent-review-authority.json", import.meta.url);
  const subjectBytes = await readFile(subjectUrl);
  const subject = JSON.parse(subjectBytes);
  const authority = await load(authorityUrl);
  assert.equal(authority.initial_subject.sha256, sha256(subjectBytes));
  for (const entry of [...subject.canonical_contracts, ...subject.candidate_artifacts]) {
    assert.match(entry.sha256, /^[a-f0-9]{64}$/);
    assert.equal(sha256(await readFile(path.join(repository, entry.path))), entry.sha256, entry.path);
  }
});
