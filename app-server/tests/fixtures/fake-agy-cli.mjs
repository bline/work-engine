import { readFileSync } from "node:fs";
import path from "node:path";

const args = process.argv.slice(2);
const valueAfter = (flag) => args[args.indexOf(flag) + 1];
const model = valueAfter("--model");
const agentName = valueAfter("--agent");
const newProject = args.includes("--new-project");
if (!model || (!model.includes("flash") && !model.includes("pro"))) {
  process.stderr.write("Only Gemini Flash or explicitly admitted Pro models are supported\n");
  process.exit(2);
}
if (!newProject) {
  process.stderr.write("Agy must create an isolated project for the immutable packet root\n");
  process.exit(2);
}
if (agentName !== "work-engine-evidence-reviewer") {
  process.stderr.write("Agy reviewer agent must be selected by its discovered name\n");
  process.exit(2);
}
const agentPath = path.join(process.cwd(), ".agents", "agents", agentName, "agent.md");
const agent = readFileSync(agentPath, "utf8");
if (!agent.includes("tools:\n  - view_file\n  - grep_search\n  - finish")
  || !agent.includes("commandExecutionPolicy: off")) {
  process.stderr.write("Agy reviewer agent must expose only its explicit read tool set\n");
  process.exit(2);
}
if (!agentPath.startsWith(path.join(process.cwd(), ".agents", "agents"))) {
  process.stderr.write("Agy reviewer agent must be staged inside the immutable packet project\n");
  process.exit(2);
}
if (process.env.FAKE_AGY_ERROR) {
  process.stderr.write(process.env.FAKE_AGY_ERROR === "quota" ? "quota exhausted\n" : `${process.env.FAKE_AGY_ERROR}\n`);
  process.exit(1);
}
const response = JSON.parse(process.env.FAKE_AGY_RESPONSE);
const readPath = process.env.FAKE_AGY_READ_PATH;
const conversationId = "fake-agy-conversation";
const events = [
  { event: "init", init: {
    conversation_id: conversationId,
    model,
    agent: agentName,
    tools: ["view_file", "grep_search", "finish"],
  } },
];
if (readPath) {
  events.push({ event: "step_update", step_update: {
    step_type: "tool",
    tool_name: "view_file",
    tool_info: { parameters: { path: readPath }, output: "file bytes" },
  } });
}
events.push({ event: "result", result: {
  status: "SUCCESS",
  conversation_id: conversationId,
  structured_output: response,
  usage: { input_tokens: 10, output_tokens: 5, total_tokens: 15 },
} });
for (const event of events) process.stdout.write(`${JSON.stringify(event)}\n`);
