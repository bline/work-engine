#!/usr/bin/env node
import { readFile } from "node:fs/promises";
import path from "node:path";
import process from "node:process";

import { buildGrokReviewPacket } from "./packet.mjs";
import { runGrokReview } from "./runner.mjs";

const casePath = process.argv[2];
if (!casePath) {
  process.stderr.write("usage: node run-review.mjs <review-case.json> [repository-root]\n");
  process.exitCode = 2;
} else {
  const repositoryRoot = path.resolve(process.argv[3] ?? process.cwd());
  const reviewCase = JSON.parse(await readFile(path.resolve(casePath), "utf8"));
  const packet = await buildGrokReviewPacket({ repositoryRoot, reviewCase });
  try {
    process.stderr.write(
      `[grok-review] packet=${packet.root} subject=${packet.manifest.subject.commit} files=${packet.manifest.files.length}\n`,
    );
    const execution = reviewCase.execution ?? {};
    const result = await runGrokReview({
      packet,
      model: execution.model,
      reasoningEffort: execution.reasoning_effort,
      maxTurns: execution.max_turns,
      timeoutMs: execution.timeout_ms,
      onProgress(update) {
        process.stderr.write(`[grok-review] phase=${update.phase} elapsed_ms=${update.elapsed_ms}\n`);
      },
    });
    process.stdout.write(`${JSON.stringify(result, null, 2)}\n`);
    if (result.status !== "completed") process.exitCode = 1;
  } finally {
    await packet.dispose();
  }
}
