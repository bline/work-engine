import { PROJECTION_SCHEMA, reviewTaskRequest } from "./task-contract.js";

const object = (x) => x !== null && typeof x === "object" && !Array.isArray(x);
const text = (x) => typeof x === "string" && x.length > 0;
const ref = (x) => object(x) && Object.keys(x).length === 3 && text(x.id) && text(x.revision) && /^[a-f0-9]{64}$/.test(x.sha256);
const sameRef = (a, b) => ref(a) && ref(b) && a.id === b.id && a.revision === b.revision && a.sha256 === b.sha256;
const keys = (x, names) => object(x) && Object.keys(x).length === names.length && names.every((n) => Object.hasOwn(x, n));
const stamp = (x) => typeof x === "string" && /Z$/.test(x) && Number.isFinite(Date.parse(x));

// Bounded transport shape, not an authority or runtime state validator.
export function validProjectionEnvelope(x) {
  return keys(x, ["schema", "observed_at", "source_revision", "tasks"]) &&
    x.schema === PROJECTION_SCHEMA && stamp(x.observed_at) && text(x.source_revision) &&
    Array.isArray(x.tasks) && x.tasks.length <= 100 && x.tasks.every((entry) =>
      keys(entry, ["task", "request_json", "admission", "result"]) && ref(entry.task) &&
      typeof entry.request_json === "string" &&
      (entry.admission === null || object(entry.admission)) && (entry.result === null || object(entry.result)));
}

const admissionFields = ["schema", "admission_id", "task", "decision", "answer_id", "revision", "status",
  "hold_reasons", "reservation_ref", "authority_revision", "native_operation_id", "observed_at", "source_revision"];
const statuses = ["held", "rejected", "reserved", "delivery_pending", "start_unknown", "running", "closure_pending", "closed"];
const resultFields = ["schema", "task", "admission_id", "session_id", "turn_id", "worker_contract_revision", "outcome",
  "operational_status", "exceptions", "artifacts", "evidence", "clock", "accounting", "custody", "observed_at", "source_revision"];
const outcomes = ["completed", "partial", "blocked", "failed", "interrupted", "limit", "cancelled"];

function validAdmission(x, task) {
  return keys(x, admissionFields) && x.schema === "dot.admission-view/v1-proposal" &&
    sameRef(x.task, task) && ref(x.decision) && text(x.admission_id) && text(x.answer_id) && text(x.revision) &&
    statuses.includes(x.status) && Array.isArray(x.hold_reasons) && x.hold_reasons.every(text) &&
    (x.reservation_ref === null || ref(x.reservation_ref)) && (x.authority_revision === null || text(x.authority_revision)) &&
    (x.native_operation_id === null || text(x.native_operation_id)) && stamp(x.observed_at) && text(x.source_revision);
}
const nullableStamp = (x) => x === null || stamp(x);
const amount = (x) => (typeof x === "number" && Number.isFinite(x) && x >= 0) ||
  (typeof x === "string" && /^(0|[1-9]\d*)(\.\d+)?$/.test(x));
const nullableAmount = (x) => x === null || amount(x);
const dispositions = ["unknown", "pending", "running", "terminal", "closed", "exited", "interrupted", "cancelled", "failed"];
const disposition = (x) => keys(x, ["id", "disposition"]) && (x.id === null || text(x.id)) && dispositions.includes(x.disposition);
function validObservations(x) {
  return keys(x.clock, ["first_funded_at", "admitted_at", "observed_start_at", "close_by"]) &&
    Object.values(x.clock).every(nullableStamp) &&
    keys(x.accounting, ["units", "limit", "reservation", "observed_usage", "coverage", "unsettled"]) &&
    x.accounting.units === "speed_adjusted_api_usd" && amount(x.accounting.limit) &&
    [x.accounting.reservation, x.accounting.observed_usage, x.accounting.unsettled].every(nullableAmount) &&
    ["complete", "partial", "unknown", "pending", "unavailable"].includes(x.accounting.coverage) &&
    keys(x.custody, ["native", "children", "processes"]) && disposition(x.custody.native) &&
    [x.custody.children, x.custody.processes].every((items) => Array.isArray(items) && items.every(disposition));
}
function validResult(x, task, admission) {
  return keys(x, resultFields) && x.schema === "dot.result-view/v1-proposal" && sameRef(x.task, task) &&
    admission !== null && x.admission_id === admission.admission_id &&
    [x.session_id, x.turn_id].every((v) => v === null || text(v)) &&
    (x.worker_contract_revision === null || x.worker_contract_revision === 1) &&
    (x.outcome === null || (outcomes.includes(x.outcome) && [x.session_id, x.turn_id].every(text) && x.worker_contract_revision === 1)) && text(x.operational_status) &&
    Array.isArray(x.exceptions) && x.exceptions.every(text) &&
    Array.isArray(x.artifacts) && x.artifacts.every(ref) && Array.isArray(x.evidence) && x.evidence.every(ref) &&
    validObservations(x) && stamp(x.observed_at) && text(x.source_revision);
}

export async function readTaskProjection(projection) {
  if (projection === null || projection === undefined) return null;
  if (!validProjectionEnvelope(projection)) return { available: false, tasks: [], issues: ["Task history uses an unsupported or incomplete snapshot."] };
  const tasks = [], issues = [], counts = new Map();
  for (const item of projection.tasks) counts.set(item.task.id, (counts.get(item.task.id) || 0) + 1);
  for (const item of projection.tasks) {
    try {
      if (counts.get(item.task.id) !== 1) throw new Error("duplicate task identity");
      const bytes = new Uint8Array(await crypto.subtle.digest("SHA-256", new TextEncoder().encode(item.request_json)));
      const hash = [...bytes].map((x) => x.toString(16).padStart(2, "0")).join("");
      const request = JSON.parse(item.request_json);
      if (hash !== item.task.sha256 || request.data?.intent?.task_id !== item.task.id || request.data.intent.revision !== item.task.revision)
        throw new Error("request bytes or task revision changed");
      const review = await reviewTaskRequest(request);
      if (review.kind !== "task" || !review.ready) throw new Error("task terms are unsupported or incomplete");
      if (item.admission !== null && !validAdmission(item.admission, item.task)) throw new Error("admission does not match this task");
      if (item.result !== null && !validResult(item.result, item.task, item.admission)) throw new Error("result is unsupported or does not match this admission");
      tasks.push({ ...item, request, review });
    } catch (error) {
      issues.push(`${item.task.id}: ${error.message}.`);
    }
  }
  return { available: true, tasks, issues, observed_at: projection.observed_at, source_revision: projection.source_revision };
}
