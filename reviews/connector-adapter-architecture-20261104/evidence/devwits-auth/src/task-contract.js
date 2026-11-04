// Presentation checks only. The Rust admission owner independently verifies
// current signed authority, reservations and native effects before execution.
export const TASK_SCHEMA = "dot.task-intent/v1-proposal";
export const TASK_PROFILE = "disposable-codex-write-v1";
export const PROJECTION_SCHEMA = "dot.task-projection/v1-proposal";

const object = (x) => x !== null && typeof x === "object" && !Array.isArray(x);
const text = (x) => typeof x === "string" && x.trim().length > 0;
const strings = (x) => Array.isArray(x) && x.every(text);
const exact = (x, names) => object(x) &&
  Object.keys(x).length === names.length && names.every((name) => Object.hasOwn(x, name));
const ref = (x) => exact(x, ["id", "revision", "sha256"]) &&
  text(x.id) && text(x.revision) && /^[a-f0-9]{64}$/.test(x.sha256);
const utc = (x) => typeof x === "string" &&
  /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d{1,3})?Z$/.test(x) &&
  Number.isFinite(Date.parse(x)) && new Date(x).toISOString().slice(0, 19) === x.slice(0, 19);
const money = (x) => typeof x === "string" && /^(0|[1-9]\d*)\.\d{2}$/.test(x);
const cents = (x) => BigInt(x.replace(".", ""));
const oneStart = (x) => Array.isArray(x) && x.length === 1 && x[0] === "start";
const safePath = (x) => text(x) && !x.startsWith("/") && !x.includes("\\") &&
  !x.includes("\0") && !x.split("/").some((part) => !part || part === "." || part === "..");

export function inspectTaskIntent(intent) {
  const issues = [];
  const check = (condition, message) => { if (!condition) issues.push(message); };
  check(exact(intent, ["schema", "task_id", "lane_id", "proposal", "revision",
    "objective", "input", "operation", "provider", "execution_profile", "workspace",
    "model", "effects", "prerequisites", "clock", "budget", "submitter", "authority"]),
  "Task terms are missing fields or contain an unsupported extension.");
  if (!object(intent)) return { ready: false, issues, references: [] };
  check(intent.schema === TASK_SCHEMA, "This task version is not supported.");
  check([intent.task_id, intent.lane_id, intent.revision, intent.objective, intent.submitter].every(text),
    "Task identity, objective or submitting actor is missing.");
  check(ref(intent.input) && ref(intent.proposal), "The exact input and proposal revisions are required.");
  check(intent.operation === "start" && intent.provider === "codex" && intent.execution_profile === TASK_PROFILE,
    "This view supports only a present-mode Codex start in the disposable write profile.");
  check(exact(intent.workspace, ["profile_id", "checkpoint", "configuration"]) &&
    text(intent.workspace.profile_id) && text(intent.workspace.checkpoint) &&
    ref(intent.workspace.configuration), "Workspace, source checkpoint or configuration is incomplete.");
  check(exact(intent.model, ["id", "effort", "tier"]) && Object.values(intent.model).every(text),
    "The effective model, reasoning effort and service tier are required.");
  check(exact(intent.effects, ["write_paths", "network", "max_concurrent_tasks", "max_child_tasks"]) &&
    Array.isArray(intent.effects.write_paths) && intent.effects.write_paths.length > 0 &&
    intent.effects.write_paths.every(safePath) && intent.effects.network === false &&
    intent.effects.max_concurrent_tasks === 1 && intent.effects.max_child_tasks === 0,
  "The first profile requires explicit relative write paths, no task network, one task and no child tasks.");
  check(Array.isArray(intent.prerequisites) && intent.prerequisites.length > 0 && intent.prerequisites.every(ref),
    "The qualified profile and prerequisite evidence are required.");
  check(exact(intent.clock, ["first_funded_rule", "admit_until", "close_by"]) &&
    intent.clock.first_funded_rule === "first_funded_setup_or_readiness" && utc(intent.clock.admit_until) && utc(intent.clock.close_by) &&
    Date.parse(intent.clock.admit_until) <= Date.parse(intent.clock.close_by),
  "Valid admission and closure deadlines and the first-funded clock rule are required.");
  const b = intent.budget;
  const budgetShape = exact(b, ["objective_id", "units", "maximum", "productive_maximum", "closure_reserve", "coverage"]) &&
    text(b.objective_id) && b.units === "speed_adjusted_api_usd" && b.coverage === "all_objective_actors" &&
    [b.maximum, b.productive_maximum, b.closure_reserve].every(money);
  check(budgetShape && cents(b.maximum) > 0n && cents(b.productive_maximum) > 0n &&
    cents(b.closure_reserve) > 0n && cents(b.productive_maximum) + cents(b.closure_reserve) <= cents(b.maximum),
  "An all-actor API-equivalent allowance, productive limit and protected closure reserve are required.");
  check(exact(intent.authority, ["mode", "mandate", "allowed_operations", "reserved_decisions"]) &&
    ref(intent.authority.mode) && intent.authority.mandate === null &&
    oneStart(intent.authority.allowed_operations) && strings(intent.authority.reserved_decisions),
  "The first profile requires an explicit present-mode reference, start only and no Away mandate.");
  if (issues.length) return { ready: false, issues, references: [] };
  return { ready: true, issues, references: [
    { label: "Worker input", ref: intent.input },
    { label: "Exact proposal", ref: intent.proposal },
    { label: "Effective workspace configuration", ref: intent.workspace.configuration },
    ...intent.prerequisites.map((value, i) => ({ label: `Prerequisite evidence ${i + 1}`, ref: value })),
    { label: "Present-mode authority", ref: intent.authority.mode },
  ] };
}

async function digest(text) {
  const hash = new Uint8Array(await crypto.subtle.digest("SHA-256", new TextEncoder().encode(text)));
  return [...hash].map((b) => b.toString(16).padStart(2, "0")).join("");
}

export async function reviewTaskRequest(request) {
  if (!object(request?.data) || !Object.hasOwn(request.data, "intent"))
    return { kind: "legacy", ready: true, issues: [] };
  const intent = request.data.intent;
  const review = { kind: "task", intent, ...inspectTaskIntent(intent) };
  if (!review.ready) return review;
  const artifacts = request.data.intent_artifacts;
  if (!Array.isArray(artifacts)) return { ...review, ready: false,
    issues: ["Referenced task material is unavailable. Approval is held until the complete terms can be reviewed."] };
  const resolved = [];
  for (const entry of review.references) {
    const matches = artifacts.filter((x) => object(x) && ref(x.ref) &&
      x.ref.id === entry.ref.id && x.ref.revision === entry.ref.revision && x.ref.sha256 === entry.ref.sha256);
    if (matches.length !== 1 || typeof matches[0].content_json !== "string" ||
      await digest(matches[0].content_json) !== entry.ref.sha256) {
      review.issues.push(`${entry.label}: missing, ambiguous or changed content.`);
      continue;
    }
    try {
      const content = JSON.parse(matches[0].content_json);
      if (entry.label === "Present-mode authority" && (!exact(content, ["schema", "owner_id", "revision", "mode",
        "effective_at", "pending_transition", "mandates", "in_flight", "observed_at", "source_revision"]) ||
        content.schema !== "dot.mode-view/v1-proposal" || !text(content.owner_id) || content.revision !== entry.ref.revision ||
        content.mode !== "present" || content.pending_transition !== null || !utc(content.effective_at) ||
        !utc(content.observed_at) || !text(content.source_revision) || !Array.isArray(content.mandates) ||
        !content.mandates.every(ref) || !Array.isArray(content.in_flight) || !content.in_flight.every(ref)))
        review.issues.push("The referenced mode is not a supported present-mode decision.");
      resolved.push({ ...entry, content_json: matches[0].content_json, content });
    } catch {
      review.issues.push(`${entry.label}: content is not valid JSON.`);
    }
  }
  return { ...review, ready: review.issues.length === 0, references: resolved };
}

export function taskApprovalIssue(review, decision, comment, now = Date.now()) {
  if (review?.kind !== "task" || decision !== "approve") return null;
  if (!review.ready) return "Approval is held until the complete supported task terms are available.";
  if (comment !== "") return "Use Ask a question for conditions or comments so the task terms can be reconciled before approval.";
  if (now >= Date.parse(review.intent.clock.admit_until) || now >= Date.parse(review.intent.clock.close_by))
    return "This task's admission deadline has passed. Request updated terms before approving.";
  return null;
}
