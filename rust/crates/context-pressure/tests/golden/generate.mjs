// Golden-vector generator for the Rust `context-pressure` crate.
//
// Runs the JS reference (`app-server/src/context-pressure-controller.mjs` and
// `token-usage-pressure-projection.mjs`) over a fixed corpus and writes `vectors.json` next to
// this file. Every input is stored as raw JSON text (`inputJson`) so that values JSON.stringify
// cannot round-trip (-0, 5.0, 1e21) reach both implementations unchanged. Outputs are stored as
// the JSON.parse(JSON.stringify(result)) of the JS result, or as { kind, message } for a thrown
// TypeError. The mapping from JS message to the Rust `ErrorKind::as_str()` name is a regex table;
// a message with no entry fails the run.
//
// Usage: node rust/crates/context-pressure/tests/golden/generate.mjs
import { createHash } from "node:crypto";
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import {
  ContextPressureController,
  validateContextPressurePolicy,
} from "../../../../../app-server/src/context-pressure-controller.mjs";
import {
  TokenUsagePressureProjector,
  validateTokenUsagePressureProfile,
} from "../../../../../app-server/src/token-usage-pressure-projection.mjs";

const here = dirname(fileURLToPath(import.meta.url));
const jsFiles = {
  "app-server/src/context-pressure-controller.mjs":
    join(here, "../../../../../app-server/src/context-pressure-controller.mjs"),
  "app-server/src/token-usage-pressure-projection.mjs":
    join(here, "../../../../../app-server/src/token-usage-pressure-projection.mjs"),
};

const KIND_TABLE = [
  [/^latest token usage observedAt must be an ISO timestamp$/, "invalid_usage_timestamp"],
  [/^latest token usage sequence must be a positive safe integer$/, "invalid_usage_sequence"],
  [/^latest token usage last\.(totalTokens|inputTokens) must be a non-negative safe integer$/, "invalid_token_count"],
  [/ must be an object$/, "not_object"],
  [/ contains unsupported fields: /, "unsupported_fields"],
  [/^unsupported (context pressure policy|context pressure observation|token usage pressure profile) schema version$/, "unsupported_schema_version"],
  [/^unsupported token usage pressure measurement profile$/, "unsupported_profile"],
  [/^context pressure policy unit must be basis_points$/, "unsupported_unit"],
  [/ must be a non-empty string$/, "not_non_empty_string"],
  [/ must be an integer from 0 through 10000 basis points$/, "not_basis_points"],
  [/ must be a positive safe integer$/, "not_positive_integer"],
  [/ must be SHA-256 bound$/, "not_sha_revision"],
  [/ must be an ISO timestamp$/, "not_iso_timestamp"],
  [/\.exit must be lower than /, "band_exit_not_below_enter"],
  [/^context pressure thresholds must be ordered/, "thresholds_not_ordered"],
  [/^context pressure initial disposition is unsupported$/, "unsupported_initial_disposition"],
  [/^context pressure minimum sequence must be /, "invalid_minimum_sequence"],
  [/reused for different evidence$/, "observation_id_reused"],
  [/increase monotonically$/, "sequence_not_monotonic"],
  [/exceed the recovered floor$/, "sequence_below_floor"],
];

function kindOf(message) {
  for (const [pattern, kind] of KIND_TABLE) if (pattern.test(message)) return kind;
  throw new Error(`unmapped JS error message: ${message}`);
}

// Run `fn`; return { ok: JSON-roundtripped value } or { err: { kind, message } }.
function outcome(fn) {
  try {
    const value = fn();
    return { ok: JSON.parse(JSON.stringify(value)) };
  } catch (error) {
    if (!(error instanceof TypeError)) throw error;
    return { err: { kind: kindOf(error.message), message: error.message } };
  }
}

// Seeded PRNG so the fixture is reproducible.
function mulberry32(seed) {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}
const rand = mulberry32(0x5eed1234);
const pick = (items) => items[Math.floor(rand() * items.length)];
const int = (lo, hi) => lo + Math.floor(rand() * (hi - lo + 1));

const J = JSON.stringify;
const DEFAULT_POLICY = {
  schemaVersion: 1,
  unit: "basis_points",
  approaching: { enter: 6500, exit: 5500 },
  replacementCandidate: { enter: 7800, exit: 6800 },
  critical: { enter: 9000, exit: 8000 },
};
const DEFAULT_PROFILE = {
  schemaVersion: 1,
  usageField: "last.totalTokens",
  windowField: "modelContextWindow",
  rounding: "floor",
  saturation: "clamp_10000",
};
const SHA_A = `sha256:${"a".repeat(64)}`;

const policyText = (overrides = {}) => J({ ...DEFAULT_POLICY, ...overrides });
const profileText = (overrides = {}) => J({ ...DEFAULT_PROFILE, ...overrides });

// ---------- policy cases ----------
const policyInputs = [
  ["default", policyText()],
  ["key order differs", '{"critical":{"exit":8000,"enter":9000},"unit":"basis_points","schemaVersion":1,"replacementCandidate":{"exit":6800,"enter":7800},"approaching":{"exit":5500,"enter":6500}}'],
  ["whole range", policyText({ approaching: { enter: 2, exit: 0 }, replacementCandidate: { enter: 5000, exit: 1 }, critical: { enter: 10000, exit: 9999 } })],
  ["equal across bands", policyText({ approaching: { enter: 6000, exit: 5000 }, replacementCandidate: { enter: 6000, exit: 5000 }, critical: { enter: 6000, exit: 5000 } })],
  ["touching bands", policyText({ approaching: { enter: 6000, exit: 5000 }, replacementCandidate: { enter: 7000, exit: 6000 }, critical: { enter: 8000, exit: 7000 } })],
  ["float spellings of integers", '{"schemaVersion":1.0,"unit":"basis_points","approaching":{"enter":6.5e3,"exit":5500.0},"replacementCandidate":{"enter":7800,"exit":6800},"critical":{"enter":9e3,"exit":8000}}'],
  ["negative zero exit", '{"schemaVersion":1,"unit":"basis_points","approaching":{"enter":6500,"exit":-0},"replacementCandidate":{"enter":7800,"exit":6800},"critical":{"enter":9000,"exit":8000}}'],
  ["null", "null"],
  ["array", "[]"],
  ["string", '"policy"'],
  ["number", "7"],
  ["empty object", "{}"],
  ["unknown field", policyText({ sampleEveryTurns: 1 })],
  ["unknown fields sorted", policyText({ zeta: 1, alpha: 2, "😀": 3, "\uffff": 4, B: 5, a: 6 })],
  ["unknown field plus bad version", policyText({ sampleEveryTurns: 1, schemaVersion: 2 })],
  ["version 2", policyText({ schemaVersion: 2 })],
  ["version string", policyText({ schemaVersion: "1" })],
  ["version missing", J({ ...DEFAULT_POLICY, schemaVersion: undefined })],
  ["version 1.5", policyText({ schemaVersion: 1.5 })],
  ["unit wrong", policyText({ unit: "percent" })],
  ["unit missing and bad band", J({ ...DEFAULT_POLICY, unit: undefined, approaching: 3 })],
  ["band missing", J({ ...DEFAULT_POLICY, approaching: undefined })],
  ["band null", policyText({ replacementCandidate: null })],
  ["band array", policyText({ critical: [] })],
  ["band extra field", policyText({ approaching: { enter: 6500, exit: 5500, hold: 1 } })],
  ["band enter missing", policyText({ approaching: { exit: 5500 } })],
  ["band exit missing", policyText({ approaching: { enter: 6500 } })],
  ["enter fractional", policyText({ approaching: { enter: 6500.5, exit: 5500 } })],
  ["exit fractional", policyText({ approaching: { enter: 6500, exit: 5500.5 } })],
  ["enter negative", policyText({ approaching: { enter: -1, exit: -2 } })],
  ["enter 10001", policyText({ critical: { enter: 10001, exit: 8000 } })],
  ["enter string", policyText({ approaching: { enter: "6500", exit: 5500 } })],
  ["enter null", policyText({ approaching: { enter: null, exit: 5500 } })],
  ["enter huge", '{"schemaVersion":1,"unit":"basis_points","approaching":{"enter":1e300,"exit":5500},"replacementCandidate":{"enter":7800,"exit":6800},"critical":{"enter":9000,"exit":8000}}'],
  ["enter unsafe integer", '{"schemaVersion":1,"unit":"basis_points","approaching":{"enter":9007199254740993,"exit":5500},"replacementCandidate":{"enter":7800,"exit":6800},"critical":{"enter":9000,"exit":8000}}'],
  ["exit equals enter", policyText({ approaching: { enter: 6500, exit: 6500 } })],
  ["exit above enter", policyText({ replacementCandidate: { enter: 7800, exit: 7900 } })],
  ["order: candidate exit below approaching enter", policyText({ replacementCandidate: { enter: 7800, exit: 6000 } })],
  ["order: candidate enter below its exit neighbour", policyText({ approaching: { enter: 6500, exit: 5500 }, replacementCandidate: { enter: 6400, exit: 6300 } })],
  ["order: critical exit below candidate enter", policyText({ critical: { enter: 9000, exit: 7000 } })],
  ["order: critical enter below candidate enter", policyText({ critical: { enter: 7700, exit: 7600 } })],
  ["bad first band beats bad second band", policyText({ approaching: { enter: 5, exit: 5 }, replacementCandidate: 1 })],
  ["bad second band beats bad third", policyText({ replacementCandidate: { enter: "x", exit: 1 }, critical: null })],
];
const policyCases = policyInputs.map(([name, inputJson]) => ({
  name,
  inputJson,
  expect: outcome(() => validateContextPressurePolicy(JSON.parse(inputJson))),
}));

// ---------- profile cases ----------
const profileInputs = [
  ["default", profileText()],
  ["key order differs", '{"saturation":"clamp_10000","rounding":"floor","windowField":"modelContextWindow","usageField":"last.totalTokens","schemaVersion":1}'],
  ["float version", '{"schemaVersion":1.0,"usageField":"last.totalTokens","windowField":"modelContextWindow","rounding":"floor","saturation":"clamp_10000"}'],
  ["null", "null"],
  ["array", "[1]"],
  ["number", "0"],
  ["empty", "{}"],
  ["unknown field", profileText({ extra: true })],
  ["unknown fields sorted", profileText({ z: 1, a: 1, "😀": 1, "\uffff": 1 })],
  ["version 2", profileText({ schemaVersion: 2 })],
  ["version missing", J({ ...DEFAULT_PROFILE, schemaVersion: undefined })],
  ["usageField other", profileText({ usageField: "total.totalTokens" })],
  ["windowField other", profileText({ windowField: "window" })],
  ["rounding other", profileText({ rounding: "ceil" })],
  ["saturation other", profileText({ saturation: "none" })],
  ["usageField number", profileText({ usageField: 1 })],
  ["usageField missing", J({ ...DEFAULT_PROFILE, usageField: undefined })],
  ["bad version beats bad field", profileText({ schemaVersion: 3, rounding: "x" })],
  ["unknown beats bad version", profileText({ schemaVersion: 3, zz: 1 })],
];
const profileCases = profileInputs.map(([name, inputJson]) => ({
  name,
  inputJson,
  expect: outcome(() => validateTokenUsagePressureProfile(JSON.parse(inputJson))),
}));

// ---------- projector cases ----------
const OMIT = Symbol("omit");
function snapshotText({ threadId = "thread-1", schemaVersion = 1, usage } = {}) {
  const parts = [];
  if (schemaVersion !== OMIT) parts.push(`"schemaVersion":${J(schemaVersion)}`);
  if (threadId !== OMIT) parts.push(`"threadId":${J(threadId)}`);
  if (usage !== undefined) parts.push(`"latestTokenUsage":${usage}`);
  return `{${parts.join(",")}}`;
}
function usageText({ sequence = 1, observedAt = "2026-08-25T22:00:00.000Z", total = 5000, input = 4000, window = 10000, extra = "" } = {}) {
  const parts = [];
  if (sequence !== OMIT) parts.push(`"sequence":${sequence}`);
  if (observedAt !== OMIT) parts.push(`"observedAt":${J(observedAt)}`);
  const last = [];
  if (total !== OMIT) last.push(`"totalTokens":${total}`);
  if (input !== OMIT) last.push(`"inputTokens":${input}`);
  const details = [`"last":{${last.join(",")}}`];
  if (window !== OMIT) details.push(`"modelContextWindow":${window}`);
  parts.push(`"details":{${details.join(",")}}`);
  if (extra) parts.push(extra);
  return `{${parts.join(",")}}`;
}
const snap = (usageOpts = {}, snapOpts = {}) => snapshotText({ ...snapOpts, usage: usageText(usageOpts) });

const exoticExtra = [
  ["astral vs BMP-high keys", `"😀":1,"\uffff":2,"\ud7ff":3,"\ue000":4,"a":5`],
  ["key escapes", `"a\\"b":1,"c\\\\d":2,"\\u0000":3,"\\u001f":4,"\\n\\t\\r\\b\\f":5,"\\u2028":6,"\\u2029":7,"\\u007f":8,"é":9,"/":10`],
  ["string value escapes", `"s":"q\\"b\\\\ \\u0001\\u001f\\u007f \\u2028\\u2029 😀 \\ud7ff\\ue000\\uffff é \\n\\t/"`],
  ["numbers", `"n":[0.1,5.0,1e21,1.5e-7,123456789012345680000,-0,9007199254740991,-9007199254740991,1e-7,1e-6,0.000001,1e300,5e-324,1.7976931348623157e308,100,4.35,123456789.12345679,1e20,1e22,0.5e-6,2.5e-7,-1.5,12345678901234567890]`],
  ["nested", `"o":{"z":[{"b":[],"a":{}},[[],[{}]],null,true,false,"x"],"a":{"y":{"x":{"w":[1,[2,[3,[4]]]]}}}}`],
  ["duplicate keys last wins", `"d":1,"d":2,"e":{"k":1,"k":null}`],
  ["empty key and digits", `"":1,"10":2,"9":3,"1":4,"a b":5`],
  ["case and ordering", `"B":1,"a":2,"A":3,"b":4,"_":5,"~":6,"Z":7,"z":8`],
  ["unicode normal forms", `"e\u0301":1,"\u00e9":2`],
];

const projectorCases = [];
function addProjector(name, profile, snapshotJson, extra = {}) {
  const projector = outcome(() => new TokenUsagePressureProjector({ profile: JSON.parse(profile) }));
  const expect = projector.err
    ? projector
    : outcome(() => new TokenUsagePressureProjector({ profile: JSON.parse(profile) }).project(JSON.parse(snapshotJson)));
  projectorCases.push({ name, profileJson: profile, snapshotJson, expect, ...extra });
}
const P = profileText();
addProjector("basic", P, snap());
addProjector("bad profile", profileText({ rounding: "ceil" }), snap());
addProjector("bad profile beats bad snapshot", profileText({ schemaVersion: 2 }), "null");
for (const [name, extra] of exoticExtra) addProjector(`canonical: ${name}`, P, snap({ extra }));
addProjector("canonical: all exotic extras together", P, snap({ extra: exoticExtra.map(([, extra]) => extra).map((e, i) => `"x${i}":{${e}}`).join(",") }));
addProjector("extra keys on snapshot and usage", P,
  `{"schemaVersion":1,"threadId":"thread-1","zz":[1],"aa":{"b":2},"latestTokenUsage":${usageText({ extra: '"q":1,"details2":{"p":1}' })}}`);
addProjector("extra fields in details and last", P,
  '{"schemaVersion":1,"threadId":"thread-1","latestTokenUsage":{"sequence":3,"observedAt":"2026-08-25T22:00:00Z","details":{"last":{"totalTokens":1,"inputTokens":0,"outputTokens":9,"cachedInputTokens":3},"total":{"totalTokens":99},"modelContextWindow":258400}}}');
for (const threadId of ["thread-1", "", "a:b:c", "😀", "é\u0301", "with space", "a\"b\\c", "\u2028", "x".repeat(300)]) {
  addProjector(`threadId ${J(threadId).slice(0, 24)}`, P, snap({}, { threadId }));
}
for (const schemaVersion of [1, 2, 0, "1", "x", null, 1.5, [1], { a: 1 }, true, 1e21, -0]) {
  addProjector(`lifecycle schemaVersion ${J(schemaVersion)}`, P, snap({}, { schemaVersion }));
}
addProjector("snapshot not object: null", P, "null");
addProjector("snapshot not object: array", P, "[]");
addProjector("snapshot not object: string", P, '"s"');
addProjector("usage null", P, snapshotText({ usage: "null" }));
addProjector("usage missing", P, snapshotText({}));
addProjector("usage array", P, snapshotText({ usage: "[]" }));
addProjector("usage string", P, snapshotText({ usage: '"u"' }));
addProjector("usage number", P, snapshotText({ usage: "3" }));
addProjector("usage empty object", P, snapshotText({ usage: "{}" }));
for (const [name, details] of [
  ["details null", "null"], ["details number", "5"], ["details string", '"abc"'], ["details array", "[]"],
  ["details empty", "{}"], ["last null", '{"last":null,"modelContextWindow":10}'], ["last string", '{"last":"abc","modelContextWindow":10}'],
  ["last array", '{"last":[1],"modelContextWindow":10}'], ["last empty", '{"last":{},"modelContextWindow":10}'],
]) {
  addProjector(name, P, snapshotText({ usage: `{"sequence":1,"observedAt":"2026-08-25T22:00:00.000Z","details":${details}}` }));
}
addProjector("details missing", P, snapshotText({ usage: '{"sequence":1,"observedAt":"2026-08-25T22:00:00.000Z"}' }));
const countValues = ["0", "-1", "1.5", '"5"', "null", "true", "9007199254740991", "9007199254740992", "5.0", "5e3", "-0", "1e21", "[5]", "{}"];
for (const v of countValues) {
  addProjector(`totalTokens ${v}`, P, snap({ total: v }));
  addProjector(`inputTokens ${v}`, P, snap({ input: v }));
  addProjector(`window ${v}`, P, snap({ window: v }));
  addProjector(`sequence ${v}`, P, snap({ sequence: v }));
}
addProjector("totalTokens missing", P, snap({ total: OMIT }));
addProjector("inputTokens missing", P, snap({ input: OMIT }));
addProjector("window missing", P, snap({ window: OMIT }));
addProjector("sequence missing", P, snap({ sequence: OMIT }));
addProjector("order: bad total beats bad input", P, snap({ total: "-1", input: "-1" }));
addProjector("order: bad input beats missing window", P, snap({ input: "-1", window: "0" }));
addProjector("order: missing window beats bad sequence", P, snap({ window: "0", sequence: "0" }));
addProjector("order: bad sequence beats bad timestamp", P, snap({ sequence: "0", observedAt: "x" }));
for (const observedAt of [OMIT, "", "x", "2026-08-25", "2026-08-25T22:00Z", "2026-02-30T00:00:00Z", "2026-08-25T24:00:00Z", "2026-08-25T25:00:00Z", "not a date", " 2026-08-25T22:00:00Z"]) {
  addProjector(`observedAt ${observedAt === OMIT ? "missing" : J(observedAt)}`, P, snap({ observedAt }));
}
addProjector("observedAt number", P, snap({ observedAt: OMIT, extra: '"observedAt":5' }));
addProjector("observedAt null", P, snap({ observedAt: OMIT, extra: '"observedAt":null' }));

// Declared Rust-only strictness (packet deviation 2): the JS coerces these into the evidence.
// `jsExpect` is the JS outcome; the Rust test asserts error kind invalid_snapshot_field instead.
const deviationCases = [];
function addDeviation(name, snapshotJson) {
  deviationCases.push({
    name,
    profileJson: P,
    snapshotJson,
    jsExpect: outcome(() => new TokenUsagePressureProjector({ profile: JSON.parse(P) }).project(JSON.parse(snapshotJson))),
    rustExpectKind: "invalid_snapshot_field",
  });
}
addDeviation("threadId missing", snap({}, { threadId: OMIT }));
addDeviation("threadId number", `{"schemaVersion":1,"threadId":7,"latestTokenUsage":${usageText()}}`);
addDeviation("threadId null", `{"schemaVersion":1,"threadId":null,"latestTokenUsage":${usageText()}}`);
addDeviation("threadId object", `{"schemaVersion":1,"threadId":{"a":1},"latestTokenUsage":${usageText()}}`);
addDeviation("schemaVersion missing", snap({}, { schemaVersion: OMIT }));

// ---------- pressure sweep ----------
const SWEEP_TEMPLATE = '{"schemaVersion":1,"threadId":"sweep","latestTokenUsage":{"sequence":1,"observedAt":"2026-08-25T22:00:00.000Z","details":{"last":{"totalTokens":__M__,"inputTokens":0},"modelContextWindow":__W__}}}';
const edgeBps = new Set([0, 1, 2, 5499, 5500, 5501, 6499, 6500, 6501, 6799, 6800, 6801, 7799, 7800, 7801, 7999, 8000, 8001, 8999, 9000, 9001, 9999, 10000, 10001, 10002, 3333, 6667, 1429, 5714, 9091, 1111, 1587]);
const windows = [1, 2, 3, 7, 9, 10, 11, 13, 29, 97, 100, 101, 999, 1000, 1003, 9999, 10000, 10001, 12345, 100000, 128000, 200000, 258400, 272000, 1000000, 1000003, 1048576, 2000000, 4294967296, 1099511627776, 9007199254740991];
const sweepPairs = new Set();
const addPair = (m, w) => {
  if (Number.isSafeInteger(m) && m >= 0 && Number.isSafeInteger(w) && w >= 1) sweepPairs.add(`${m},${w}`);
};
for (const w of windows) {
  for (const bps of edgeBps) {
    const exact = (bps * w) / 10000;
    for (const base of [Math.floor(exact), Math.ceil(exact), Math.round(exact)]) {
      for (const d of [-2, -1, 0, 1, 2]) addPair(base + d, w);
    }
  }
  addPair(0, w); addPair(w, w); addPair(w + 1, w); addPair(2 * w, w);
}
addPair(9007199254740991, 1); addPair(9007199254740991, 9007199254740991); addPair(9007199254740990, 9007199254740991);
for (let i = 0; i < 2500; i += 1) {
  const w = Math.max(1, Math.floor(2 ** (rand() * 40)));
  const m = Math.floor(w * rand() * 1.3);
  addPair(m, w);
}
const sweepProjector = new TokenUsagePressureProjector({ profile: DEFAULT_PROFILE });
const sweepRows = [];
for (const key of [...sweepPairs].sort()) {
  const [m, w] = key.split(",").map(Number);
  const text = SWEEP_TEMPLATE.replace("__M__", String(m)).replace("__W__", String(w));
  const projected = JSON.parse(JSON.stringify(sweepProjector.project(JSON.parse(text))));
  if (projected.status !== "projected") throw new Error(`sweep ${key} not projected`);
  sweepRows.push([m, w, projected.observation.pressureBasisPoints, projected.observation.sourceRevision]);
}

// ---------- controller cases ----------
function observationText(index, pressure, overrides = {}) {
  const base = {
    schemaVersion: 1,
    observationId: `observation-${index}`,
    sequence: index,
    observedAt: `2026-08-25T22:${String(index % 60).padStart(2, "0")}:00.000Z`,
    pressureBasisPoints: pressure,
    source: "test-host-projection",
    sourceRevision: SHA_A,
  };
  return J({ ...base, ...overrides });
}

const controllerCases = [];
function addController(name, optionsJson, stepInputs) {
  const record = { name, optionsJson };
  let controller;
  const built = outcome(() => {
    controller = new ContextPressureController(JSON.parse(optionsJson));
    return controller.snapshot();
  });
  if (built.err) {
    record.construct = built;
    record.steps = [];
  } else {
    record.construct = built;
    record.steps = stepInputs.map((inputJson) => {
      const result = outcome(() => controller.observe(JSON.parse(inputJson)));
      return { inputJson, expect: result, snapshot: JSON.parse(JSON.stringify(controller.snapshot())) };
    });
  }
  controllerCases.push(record);
}
const opts = (extra = {}, policy = DEFAULT_POLICY) => J({ policy, ...extra });

// Every case of app-server/tests/context-pressure-controller.test.mjs.
addController("test: policy validation baseline", opts(), []);
addController("test: advances through dispositions", opts(), [
  observationText(1, 6000), observationText(2, 6500), observationText(3, 7800), observationText(4, 9000),
]);
addController("test: hysteresis holds each exit band", opts(), [
  observationText(1, 9500), observationText(2, 8500), observationText(3, 7500), observationText(4, 7000),
  observationText(5, 6500), observationText(6, 6000), observationText(7, 5000),
]);
addController("test: skip stages", opts(), [observationText(1, 9400)]);
addController("test: replay, conflicting reuse, stale sequence", opts(), [
  observationText(1, 7900), observationText(1, 7900), observationText(1, 9500),
  observationText(2, 8000, { observationId: "stale-observation", sequence: 1 }),
]);

// Constructor options.
for (const [name, extra] of [
  ["initial default", {}],
  ["initial comfortable", { initialDisposition: "comfortable" }],
  ["initial approaching", { initialDisposition: "approaching" }],
  ["initial replacement_candidate", { initialDisposition: "replacement_candidate" }],
  ["initial critical", { initialDisposition: "critical" }],
  ["initial unknown", { initialDisposition: "high" }],
  ["initial number", { initialDisposition: 1 }],
  ["initial null", { initialDisposition: null }],
  ["initial empty string", { initialDisposition: "" }],
  ["initial wrong case", { initialDisposition: "Critical" }],
  ["floor 0", { minimumSequence: 0 }],
  ["floor 5", { minimumSequence: 5 }],
  ["floor 5.0", { minimumSequence: 5.0 }],
  ["floor max safe", { minimumSequence: 9007199254740991 }],
  ["floor beyond safe", { minimumSequence: 9007199254740992 }],
  ["floor negative", { minimumSequence: -1 }],
  ["floor fractional", { minimumSequence: 1.5 }],
  ["floor string", { minimumSequence: "1" }],
  ["floor null", { minimumSequence: null }],
  ["bad initial beats bad floor", { initialDisposition: "x", minimumSequence: -1 }],
  ["bad floor beats bad policy", { minimumSequence: -1 }],
]) {
  const policy = name === "bad floor beats bad policy" ? { ...DEFAULT_POLICY, unit: "x" } : DEFAULT_POLICY;
  addController(`options: ${name}`, opts(extra, policy), [observationText(1, 7000), observationText(2, 7000), observationText(6, 7000), observationText(7, 100)]);
}
addController("options: initial beats bad policy", opts({ initialDisposition: "x" }, { ...DEFAULT_POLICY, unit: "x" }), []);
addController("options: bad policy", opts({}, { ...DEFAULT_POLICY, critical: { enter: 9000, exit: 9000 } }), []);
addController("options: policy missing", "{}", []);
addController("options: policy null", '{"policy":null}', []);
addController("options: float-spelled extras", '{"policy":' + policyText() + ',"minimumSequence":3.0,"initialDisposition":"approaching"}', [observationText(3, 6000), observationText(4, 6000)]);
addController("options: unknown option ignored", opts({ ignored: true }), [observationText(1, 6500)]);
addController("floor: first sequence equals floor", opts({ minimumSequence: 3 }), [observationText(3, 7000), observationText(4, 7000)]);
addController("floor: rejected observation leaves first accepted", opts({ minimumSequence: 3 }), [observationText(2, 7000), observationText(4, 7000), observationText(4, 7000), observationText(5, 100)]);
addController("floor: not applied after first", opts({ minimumSequence: 1 }), [observationText(10, 7000), observationText(5, 7000)]);

// Observation validation, run after one accepted observation and from a fresh controller.
const observationFaults = [
  ["null", "null"], ["array", "[]"], ["string", '"o"'], ["empty", "{}"],
  ["unknown field", observationText(1, 7000, { extra: 1 })],
  ["unknown fields sorted", observationText(1, 7000, { zeta: 1, alpha: 1 })],
  ["version 2", observationText(1, 7000, { schemaVersion: 2 })],
  ["version missing", observationText(1, 7000, { schemaVersion: undefined })],
  ["id missing", observationText(1, 7000, { observationId: undefined })],
  ["id empty", observationText(1, 7000, { observationId: "" })],
  ["id whitespace", observationText(1, 7000, { observationId: "  \t\n" })],
  ["id number", observationText(1, 7000, { observationId: 5 })],
  ["id null", observationText(1, 7000, { observationId: null })],
  ["sequence 0", observationText(1, 7000, { sequence: 0 })],
  ["sequence negative", observationText(1, 7000, { sequence: -1 })],
  ["sequence fractional", observationText(1, 7000, { sequence: 1.5 })],
  ["sequence string", observationText(1, 7000, { sequence: "1" })],
  ["sequence missing", observationText(1, 7000, { sequence: undefined })],
  ["sequence unsafe", '{"schemaVersion":1,"observationId":"o","sequence":9007199254740992,"observedAt":"2026-08-25T22:00:00.000Z","pressureBasisPoints":1,"source":"s","sourceRevision":"' + SHA_A + '"}'],
  ["sequence max safe", '{"schemaVersion":1,"observationId":"o","sequence":9007199254740991,"observedAt":"2026-08-25T22:00:00.000Z","pressureBasisPoints":1,"source":"s","sourceRevision":"' + SHA_A + '"}'],
  ["sequence 3.0", '{"schemaVersion":1,"observationId":"o","sequence":3.0,"observedAt":"2026-08-25T22:00:00.000Z","pressureBasisPoints":1,"source":"s","sourceRevision":"' + SHA_A + '"}'],
  ["observedAt missing", observationText(1, 7000, { observedAt: undefined })],
  ["observedAt empty", observationText(1, 7000, { observedAt: "" })],
  ["observedAt blank", observationText(1, 7000, { observedAt: "   " })],
  ["observedAt bad", observationText(1, 7000, { observedAt: "tomorrow" })],
  ["observedAt number", observationText(1, 7000, { observedAt: 0 })],
  ["pressure -1", observationText(1, -1)],
  ["pressure 10001", observationText(1, 10001)],
  ["pressure 10000", observationText(1, 10000)],
  ["pressure 0", observationText(1, 0)],
  ["pressure fractional", observationText(1, 12.5)],
  ["pressure string", observationText(1, "7000")],
  ["pressure null", observationText(1, null)],
  ["pressure missing", observationText(1, undefined)],
  ["pressure float spelling", '{"schemaVersion":1,"observationId":"o","sequence":1,"observedAt":"2026-08-25T22:00:00.000Z","pressureBasisPoints":7e3,"source":"s","sourceRevision":"' + SHA_A + '"}'],
  ["pressure negative zero", '{"schemaVersion":1,"observationId":"o","sequence":1,"observedAt":"2026-08-25T22:00:00.000Z","pressureBasisPoints":-0,"source":"s","sourceRevision":"' + SHA_A + '"}'],
  ["source empty", observationText(1, 7000, { source: "" })],
  ["source missing", observationText(1, 7000, { source: undefined })],
  ["revision missing", observationText(1, 7000, { sourceRevision: undefined })],
  ["revision empty", observationText(1, 7000, { sourceRevision: "" })],
  ["revision no prefix", observationText(1, 7000, { sourceRevision: "a".repeat(64) })],
  ["revision uppercase", observationText(1, 7000, { sourceRevision: `sha256:${"A".repeat(64)}` })],
  ["revision short", observationText(1, 7000, { sourceRevision: `sha256:${"a".repeat(63)}` })],
  ["revision long", observationText(1, 7000, { sourceRevision: `sha256:${"a".repeat(65)}` })],
  ["revision non-hex", observationText(1, 7000, { sourceRevision: `sha256:${"g".repeat(64)}` })],
  ["revision trailing newline", observationText(1, 7000, { sourceRevision: `sha256:${"a".repeat(64)}\n` })],
  ["revision number", observationText(1, 7000, { sourceRevision: 5 })],
  ["order: version beats id", observationText(1, 7000, { schemaVersion: 2, observationId: "" })],
  ["order: id beats sequence", observationText(1, 7000, { observationId: "", sequence: 0 })],
  ["order: sequence beats observedAt", observationText(1, 7000, { sequence: 0, observedAt: "x" })],
  ["order: observedAt beats pressure", observationText(1, 7000, { observedAt: "x", pressureBasisPoints: -1 })],
  ["order: pressure beats source", observationText(1, 7000, { pressureBasisPoints: -1, source: "" })],
  ["order: source beats revision", observationText(1, 7000, { source: "", sourceRevision: "x" })],
  ["order: unknown beats version", observationText(1, 7000, { schemaVersion: 2, extra: 1 })],
];
for (const [name, text] of observationFaults) {
  addController(`observation: ${name}`, opts(), [text, observationText(1, 7000), text]);
}
// JS trim set for the non-empty-string rule.
const whitespaceProbes = ["\u00a0", "\ufeff", "\u2028", "\u2029", "\u180e", "\u200b", "\u0085", "\u1680", "\u2000", "\u200a", "\u202f", "\u205f", "\u3000", "\u000b\u000c", "\u001c", "\u001f", "\u0009\u0020", "\u200c", "\u2060", "\u00a0x"];
for (const probe of whitespaceProbes) {
  addController(`id trim probe ${J(probe)}`, opts(), [observationText(1, 7000, { observationId: probe }), observationText(2, 7000, { source: probe })]);
}
// Strings in id and source are carried verbatim.
addController("id and source are verbatim", opts(), [
  observationText(1, 7000, { observationId: " padded ", source: "  src  " }),
  observationText(2, 7000, { observationId: "😀\u2028é\u0301", source: "\"q\"\\" }),
]);
addController("replay compares canonical not text", opts(), [
  observationText(1, 7000),
  '{"sourceRevision":"' + SHA_A + '","source":"test-host-projection","pressureBasisPoints":7000.0,"observedAt":"2026-08-25T22:01:00.000Z","sequence":1,"observationId":"observation-1","schemaVersion":1}',
  observationText(1, 7000, { observedAt: "2026-08-25T22:01:00Z" }),
  observationText(1, 7000, { source: "other" }),
  observationText(1, 7000, { sourceRevision: `sha256:${"b".repeat(64)}` }),
  observationText(1, 7000, { sequence: 2 }),
]);
addController("replay only matches the last observation", opts(), [
  observationText(1, 7000), observationText(2, 9500), observationText(1, 7000), observationText(2, 9500), observationText(3, 100), observationText(2, 9500),
]);
addController("same id after a different last observation", opts(), [
  observationText(1, 7000), observationText(2, 7000, { observationId: "observation-1" }),
  observationText(3, 7000, { observationId: "observation-1" }),
]);
addController("replay of a floor-equal first observation is not special", opts({ minimumSequence: 2 }), [
  observationText(1, 7000), observationText(3, 7000), observationText(3, 7000),
]);
addController("id reuse is checked before sequence", opts(), [
  observationText(5, 7000), observationText(3, 7000, { observationId: "observation-5" }),
]);

// Boundary sweep: one observation from each held disposition, around every threshold.
const policies = {
  default: DEFAULT_POLICY,
  tight: { schemaVersion: 1, unit: "basis_points", approaching: { enter: 6000, exit: 5999 }, replacementCandidate: { enter: 7000, exit: 6999 }, critical: { enter: 8000, exit: 7999 } },
  touching: { schemaVersion: 1, unit: "basis_points", approaching: { enter: 6000, exit: 5000 }, replacementCandidate: { enter: 7000, exit: 6000 }, critical: { enter: 8000, exit: 7000 } },
  equal: { schemaVersion: 1, unit: "basis_points", approaching: { enter: 6000, exit: 5000 }, replacementCandidate: { enter: 6000, exit: 5000 }, critical: { enter: 6000, exit: 5000 } },
  extremes: { schemaVersion: 1, unit: "basis_points", approaching: { enter: 2, exit: 0 }, replacementCandidate: { enter: 5, exit: 1 }, critical: { enter: 10000, exit: 9999 } },
};
const dispositions = ["comfortable", "approaching", "replacement_candidate", "critical"];
for (const [policyName, policy] of Object.entries(policies)) {
  const thresholds = [policy.approaching, policy.replacementCandidate, policy.critical].flatMap((band) => [band.enter, band.exit]);
  const pressures = new Set([0, 1, 9999, 10000]);
  for (const t of thresholds) for (const d of [-1, 0, 1]) if (t + d >= 0 && t + d <= 10000) pressures.add(t + d);
  for (const held of dispositions) {
    for (const pressure of [...pressures].sort((a, b) => a - b)) {
      addController(`edge ${policyName} ${held} ${pressure}`, opts({ initialDisposition: held }, policy), [observationText(1, pressure)]);
    }
  }
}

// Random walks over edge pressures with injected faults.
for (let walk = 0; walk < 30; walk += 1) {
  const policy = pick(Object.values(policies));
  const thresholds = [policy.approaching, policy.replacementCandidate, policy.critical].flatMap((band) => [band.enter, band.exit]);
  const choices = [0, 10000, ...thresholds, ...thresholds.map((t) => Math.max(0, t - 1)), ...thresholds.map((t) => Math.min(10000, t + 1)), int(0, 10000), int(0, 10000)];
  const initialDisposition = pick(dispositions);
  const floor = pick([0, 0, 0, 4, 10]);
  const steps = [];
  let sequence = floor;
  for (let i = 0; i < 24; i += 1) {
    const roll = rand();
    if (roll < 0.1 && steps.length > 0) steps.push(steps[steps.length - 1]);
    else if (roll < 0.17) steps.push(observationText(Math.max(1, sequence), pick(choices), { observationId: `stale-${walk}-${i}` }));
    else if (roll < 0.22 && steps.length > 0) steps.push(JSON.stringify({ ...JSON.parse(steps[steps.length - 1]), pressureBasisPoints: pick(choices) }));
    else {
      sequence += int(1, 3);
      steps.push(observationText(sequence, pick(choices), { observationId: `w${walk}-${sequence}` }));
    }
  }
  addController(`walk ${walk} ${initialDisposition} floor ${floor}`, opts({ initialDisposition, minimumSequence: floor }, policy), steps);
}

// ---------- pipeline: projector output fed to the controller ----------
const pipelineCases = [];
{
  const pipelines = [
    ["rising and falling", [100, 4000, 6500, 7800, 9000, 9500, 8500, 7000, 6000, 5000, 100].map((pct) => [pct, 10000])],
    ["odd windows", [[1, 3], [2, 3], [3, 3], [4, 3], [5, 7], [6, 7], [65, 100], [78, 100], [901, 1000]]],
  ];
  for (const [name, pairs] of pipelines) {
    const snapshots = pairs.map(([m, w], i) => {
      const text = snapshotText({
        threadId: "pipe-thread",
        usage: usageText({ sequence: i + 1, observedAt: `2026-08-25T22:${String(i).padStart(2, "0")}:00.000Z`, total: m, window: w }),
      });
      return text;
    });
    const projector = new TokenUsagePressureProjector({ profile: DEFAULT_PROFILE });
    const controller = new ContextPressureController({ policy: DEFAULT_POLICY });
    const steps = snapshots.map((snapshotJson) => {
      const projected = JSON.parse(JSON.stringify(projector.project(JSON.parse(snapshotJson))));
      const observed = outcome(() => controller.observe(projected.observation));
      return { snapshotJson, projected, observed, snapshot: JSON.parse(JSON.stringify(controller.snapshot())) };
    });
    pipelineCases.push({ name, steps });
  }
}

// ---------- timestamp corpus ----------
const timestampCases = [];
function v8Accepts(text) {
  try {
    new ContextPressureController({ policy: DEFAULT_POLICY }).observe({
      schemaVersion: 1, observationId: "t", sequence: 1, observedAt: text,
      pressureBasisPoints: 1, source: "s", sourceRevision: SHA_A,
    });
    return true;
  } catch (error) {
    if (error instanceof TypeError && kindOf(error.message) === "not_iso_timestamp") return false;
    if (error instanceof TypeError && kindOf(error.message) === "not_non_empty_string") return false;
    throw error;
  }
}
const seenTimestamps = new Set();
// A string is `es_iso` when it matches the ECMAScript date-time string grammar (the spec forbids
// year -000000); everything else is `legacy`, whatever list it came from, because V8 falls back
// to its legacy parser when the ISO parser rejects a string.
const ES_GRAMMAR = /^(?:\d{4}|[+-]\d{6})(?:-\d{2}(?:-\d{2})?)?(?:T\d{2}:\d{2}(?::\d{2}(?:\.\d+)?)?(?:Z|[+-]\d{2}:\d{2})?)?$/;
function addTimestamp(text) {
  const tag = ES_GRAMMAR.test(text) && !text.startsWith("-000000") ? "es_iso" : "legacy";
  if (seenTimestamps.has(text)) return;
  seenTimestamps.add(text);
  timestampCases.push({ text, tag, v8: v8Accepts(text) });
}
const esIso = [
  "2026-08-25T22:00:00.000Z", "2026-08-25T22:00:00Z", "2026-08-25T22:00Z", "2026-08-25", "2026-08", "2026",
  "0000-01-01T00:00:00.000Z", "9999-12-31T23:59:59.999Z", "0000", "0000-00", "2026-00", "2026-13", "2026-12",
  "2026-00-10", "2026-13-10", "2026-01-00", "2026-01-31", "2026-01-32", "2026-02-28", "2026-02-29", "2024-02-29", "2026-02-30", "2026-04-30", "2026-04-31", "2026-06-31", "2026-11-31", "2100-02-29", "2000-02-29",
  "2026-08-25T00:00:00Z", "2026-08-25T23:59:59Z", "2026-08-25T24:00:00Z", "2026-08-25T24:00:00.000Z", "2026-08-25T24:00:00.0Z", "2026-08-25T24:00:00.1Z", "2026-08-25T24:00:00.000000000Z", "2026-08-25T24:00:00.0000000001Z", "2026-08-25T24:00:01Z", "2026-08-25T24:01Z", "2026-08-25T24:00Z", "2026-08-25T25:00Z",
  "2026-08-25T22:60Z", "2026-08-25T22:59:60Z", "2026-08-25T22:59:59.9999Z", "2026-08-25T22:00:00.1Z", "2026-08-25T22:00:00.12Z", "2026-08-25T22:00:00.123456789Z", "2026-08-25T22:00:00.1234567890123Z", "2026-08-25T22:00:00.Z", "2026-08-25T22:00:00,5Z",
  "2026-08-25T22:00:00+00:00", "2026-08-25T22:00:00-00:00", "2026-08-25T22:00:00+05:30", "2026-08-25T22:00:00-23:59", "2026-08-25T22:00:00+23:59", "2026-08-25T22:00:00+24:00", "2026-08-25T22:00:00-24:00", "2026-08-25T22:00:00+00:60", "2026-08-25T22:00:00+25:00", "2026-08-25T22:00:00+0530", "2026-08-25T22:00:00+05", "2026-08-25T22:00:00 +05:30",
  "2026-08-25T22:00+05:30", "2026-08-25T22:00:00.5+05:30", "2026-08-25T22+05:30", "2026-08-25T22",
  "2026-08-25T22:00:00", "2026-08-25T22:00", "2026-08-25T22:00:00.000", "2026-08-25T", "2026-08-25T:", "T22:00:00Z",
  "+002026-08-25T22:00:00.000Z", "-000001-01-01T00:00:00.000Z", "-000000-01-01T00:00:00.000Z", "+000000-01-01T00:00:00.000Z", "+275760-09-13T00:00:00.000Z", "+275760-09-13T00:00:00.001Z", "+275760-09-13", "+275760-09-14", "+275761-01-01", "-271821-04-20T00:00:00.000Z", "-271821-04-19T23:59:59.999Z", "-271821-04-20", "-271821-04-19", "-271822-01-01", "+002026", "+002026-08", "+02026-08-25", "+2026-08-25", "-2026-08-25", "+0020260-08-25",
  "2026-8-25", "26-08-25", "02026-08-25", "2026-08-5", "2026/08/25", "20260825", "20260825T220000Z",
  "2026-08-25t22:00:00z", "2026-08-25T22:00:00z", "2026-08-25t22:00:00Z", "2026-08-25 22:00:00Z", "2026-08-25 22:00:00",
  "", " ", "Z", "T", "2026-08-25T22:00:00ZZ", "2026-08-25T22:00:00Z ", " 2026-08-25T22:00:00Z", "2026-08-25T22:00:00Z\n", "\u00a02026-08-25T22:00:00Z",
  "２０２６-08-25", "2026-08-25T22:00:00Z\u0000",
];
for (const text of esIso) addTimestamp(text);
// Fuzz around the ES date-time grammar, including out-of-range components.
const pad = (n, width) => String(n).padStart(width, "0");
for (let i = 0; i < 3000; i += 1) {
  const yearForm = rand();
  let year;
  if (yearForm < 0.8) year = pad(int(0, 9999), 4);
  else if (yearForm < 0.9) year = `${pick(["+", "-"])}${pad(int(0, 999999), 6)}`;
  else year = `${pick(["+", "-"])}${pick(["275760", "275761", "271821", "271820", "000000", "000001"])}`;
  const month = rand() < 0.1 ? "" : pad(int(0, 13), 2);
  const day = month === "" || rand() < 0.1 ? "" : pad(int(0, 32), 2);
  let text = year + (month ? `-${month}` : "") + (day ? `-${day}` : "");
  if (day && rand() < 0.8) {
    const hh = pad(pick([int(0, 25), 24, 24, 23, 0]), 2);
    const mm = pad(pick([int(0, 61), 0, 59]), 2);
    text += `T${hh}:${mm}`;
    if (rand() < 0.8) {
      text += `:${pad(pick([int(0, 61), 0, 59]), 2)}`;
      if (rand() < 0.5) text += `.${"0123456789".repeat(2).slice(int(0, 5), int(6, 12)).padEnd(int(1, 9), "0")}`;
    }
    const tz = rand();
    if (tz < 0.4) text += "Z";
    else if (tz < 0.8) text += `${pick(["+", "-"])}${pad(int(0, 25), 2)}:${pad(int(0, 61), 2)}`;
  }
  addTimestamp(text);
}
// Legacy forms V8 may accept; Rust may reject them, but must not accept what V8 rejects.
const legacy = [
  "Aug 25 2026", "August 25, 2026", "Tue Aug 25 2026 22:00:00 GMT+0000", "Tue, 25 Aug 2026 22:00:00 GMT", "25 Aug 2026", "8/25/2026", "08/25/2026 22:00", "2026/08/25", "2026/08/25 22:00:00", "2026-8-25", "2026-08-25 22:00:00", "2026-08-25 22:00:00 UTC", "2026-08-25T22:00:00 GMT", "2026-08-25T22:00:00UTC", "2026-08-25 22:00:00 +0530", "2026-08-25T22:00:00+0530", "0", "1", "12", "2026-08-25T22:00:00z", "2026-08-25t22:00:00Z", " 2026-08-25T22:00:00Z", "2026-08-25T22:00:00Z ", "Z", "now", "tomorrow", "Infinity", "NaN", "1e3", "2026 Aug 25", "Aug 2026", "2026-08-25T22:00:00.000Z junk", "(comment) 2026-08-25", "2026-08-25T22:00:00 (UTC)", "12:00", "12:00 PM", "Aug 25 2026 10:00 PM", "Thu, 01 Jan 1970 00:00:00 GMT+0100", "20260825", "2026.08.25", "25.08.2026", "2026-08-25T22:00:00.000+05:30:00", "2026-W35-2", "2026-237",
];
for (const text of legacy) addTimestamp(text);

// ---------- output ----------
const caseCount = policyCases.length + profileCases.length + projectorCases.length + deviationCases.length
  + sweepRows.length + controllerCases.length + controllerCases.reduce((n, c) => n + c.steps.length, 0)
  + pipelineCases.length + timestampCases.length;
const fixture = {
  header: {
    generator: "rust/crates/context-pressure/tests/golden/generate.mjs",
    nodeVersion: process.version,
    jsSha256: Object.fromEntries(Object.entries(jsFiles).map(([name, path]) => [name, createHash("sha256").update(readFileSync(path)).digest("hex")])),
    caseCount,
    sections: {
      policy: policyCases.length,
      profile: profileCases.length,
      projector: projectorCases.length,
      projectorDeviations: deviationCases.length,
      pressureSweepRows: sweepRows.length,
      controllerScenarios: controllerCases.length,
      controllerSteps: controllerCases.reduce((n, c) => n + c.steps.length, 0),
      pipelines: pipelineCases.length,
      timestamps: timestampCases.length,
    },
  },
  sweepTemplate: SWEEP_TEMPLATE,
  policyCases,
  profileCases,
  projectorCases,
  deviationCases,
  sweepRows,
  controllerCases,
  pipelineCases,
  timestampCases,
};
writeFileSync(join(here, "vectors.json"), `${JSON.stringify(fixture, null, 0)}\n`);
console.log(JSON.stringify(fixture.header));
