#!/usr/bin/env node
import { readFile } from "node:fs/promises";
import path from "node:path";
import process from "node:process";

import { runAgyReview } from "./agy-runner.mjs";
import { buildGrokReviewPacket } from "./packet.mjs";

const casePath = process.argv[2];
if (!casePath) {
  process.stderr.write("usage: node run-agy-review.mjs <review-case.json> [repository-root]\n");
  process.exitCode = 2;
} else {
  const repositoryRoot = path.resolve(process.argv[3] ?? process.cwd());
  const reviewCase = JSON.parse(await readFile(path.resolve(casePath), "utf8"));
  const packet = await buildGrokReviewPacket({ repositoryRoot, reviewCase });
  try {
    process.stderr.write(
      `[agy-review] packet=${packet.root} subject=${packet.manifest.subject.commit} files=${packet.manifest.files.length}\n`,
    );
    const executionProfiles = {
      pro: "agy_pro_execution",
      pro_low: "agy_pro_low_execution",
    };
    const executionProfile = executionProfiles[process.env.AGY_EXECUTION_PROFILE] ?? "agy_execution";
    const execution = reviewCase[executionProfile] ?? {};
    const result = await runAgyReview({
      packet,
      model: execution.model,
      effort: execution.effort,
      timeoutMs: execution.timeout_ms,
      conversationId: execution.conversation_id ?? process.env.AGY_CONVERSATION_ID,
      allowPro: execution.allow_pro === true,
      onProgress(update) {
        process.stderr.write(`[agy-review] phase=${update.phase} elapsed_ms=${update.elapsed_ms}\n`);
      },
    });
    const publicResult = { ...result };
    if (publicResult.process) {
      publicResult.process = {
        code: publicResult.process.code,
        signal: publicResult.process.signal,
        timedOut: publicResult.process.timedOut,
        overflow: publicResult.process.overflow,
        stderr: publicResult.process.stderr,
      };
    }
    process.stdout.write(`${JSON.stringify(publicResult, null, 2)}\n`);
    if (result.status !== "completed") process.exitCode = 1;
  } finally {
    await packet.dispose();
  }
}
