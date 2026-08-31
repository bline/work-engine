let input = "";
for await (const chunk of process.stdin) input += chunk;
const request = JSON.parse(input);
const result = { schemaVersion: 1, subject: request.subject, verdict: "acceptable_as_is", findings: [], decisiveEvidence: [{ path: "fixture", startLine: 1, endLine: 1, sha256: "b".repeat(64) }], limitations: [] };
process.stdout.write(`${JSON.stringify({ type: "review.completed", observed: { model: "openai/gpt-5.2-codex", provider: "openrouter", servingVariant: "fixture" }, result })}\n`);
