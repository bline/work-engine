#!/usr/bin/env node
const args = process.argv.slice(2);
const sessionIndex = args.indexOf("--session-id");
const sessionId = sessionIndex === -1 ? null : args[sessionIndex + 1];

if (process.env.FAKE_GROK_ERROR === "quota") {
  process.stderr.write("You have reached your Grok usage limit\n");
  process.exit(1);
}

const text = process.env.FAKE_GROK_RESPONSE;
if (!text) {
  process.stderr.write("FAKE_GROK_RESPONSE is required\n");
  process.exit(2);
}

process.stdout.write(`${JSON.stringify({
  text,
  stopReason: "end_turn",
  sessionId,
  usage: { total_tokens: 100 },
})}\n`);

