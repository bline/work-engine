#!/usr/bin/env node

import { readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { digest, validateBehavioralResults, validateCorpus } from "./validate-skills-migration-instruction-review-regression.mjs";

function fail(message) { throw new Error(message); }

export function behavioralPacket(corpus) {
  const validated = validateCorpus(corpus);
  const cases = corpus.cases.map(({ expected: _expected, pair_id: _pairId, variant: _variant, ...reviewCase }) => reviewCase);
  return Object.freeze({
    packet_version: 1,
    purpose: "Fresh advisory semantic evaluation of effective causal loading; do not edit, accept, certify, or infer authority.",
    corpus: { id: corpus.corpus_id, sha256: validated.corpus_digest },
    instructions: [
      "Evaluate every case under the canonical agent-instruction-review diagnosis and finding contracts.",
      "Judge actual effective loading, not source traceability or reviewer reconstruction.",
      "Return the declared applicability and missing-causal-exposure consequence only when semantically supported.",
      "Preserve limitations; do not require preferred wording, co-location, or a particular projection mechanism."
    ],
    cases
  });
}

async function main(argv) {
  const corpusFlag = argv.indexOf("--corpus");
  const resultsFlag = argv.indexOf("--results");
  const emitPacket = argv.includes("--emit-packet");
  const check = argv.includes("--check");
  if (corpusFlag < 0 || (!emitPacket && !check)) fail("usage: --corpus <path> (--emit-packet | --check --results <path>)");
  const corpus = JSON.parse(await readFile(path.resolve(argv[corpusFlag + 1]), "utf8"));
  if (emitPacket) process.stdout.write(`${JSON.stringify(behavioralPacket(corpus))}\n`);
  if (check) {
    if (resultsFlag < 0) fail("--check requires --results <path>");
    const results = JSON.parse(await readFile(path.resolve(argv[resultsFlag + 1]), "utf8"));
    process.stdout.write(`${JSON.stringify(validateBehavioralResults(corpus, results))}\n`);
  }
}

if (process.argv[1] && fileURLToPath(import.meta.url) === path.resolve(process.argv[1])) {
  main(process.argv.slice(2)).catch((error) => { process.stderr.write(`${error.message}\n`); process.exitCode = 1; });
}
