import assert from "node:assert/strict";
import test from "node:test";

import {
  ContextPressureController,
  InMemoryContextLifecycleEpisodeStore,
  ShadowContextLifecycleCoordinator,
  summarizeContextLifecycleEpisodes,
  validateShadowContextLifecycleSchedule,
  verifyContextLifecycleEpisode,
} from "../src/index.mjs";

const revision = (character) => `sha256:${character.repeat(64)}`;

function policy() {
  return {
    schemaVersion: 1,
    unit: "basis_points",
    approaching: { enter: 6_500, exit: 5_500 },
    replacementCandidate: { enter: 7_800, exit: 6_800 },
    critical: { enter: 9_000, exit: 8_000 },
  };
}

function schedule(overrides = {}) {
  return {
    schemaVersion: 1,
    inspectAt: ["replacement_candidate", "critical"],
    publishAcceptedCheckpoint: true,
    ...overrides,
  };
}

function subject(overrides = {}) {
  return {
    logicalRoleInstanceId: "strategic-planner:main",
    threadId: "thread-shadow-1",
    bindingRevision: 3,
    ...overrides,
  };
}

function pressureObservation(sequence, pressureBasisPoints) {
  return {
    schemaVersion: 1,
    observationId: `pressure-${sequence}`,
    sequence,
    observedAt: `2026-08-25T23:${String(sequence).padStart(2, "0")}:00.000Z`,
    pressureBasisPoints,
    source: "fixture-utilization-projector",
    sourceRevision: revision("a"),
  };
}

function projection(overrides = {}) {
  return {
    sourceRevision: revision("b"),
    observedContext: {
      logicalRoleInstanceId: "strategic-planner:main",
      runtimeBinding: {
        threadId: "thread-shadow-1",
        bindingRevision: 3,
      },
    },
    ...overrides,
  };
}

function acceptedInspection(disposition = "accepted") {
  return {
    sourceRevision: revision("b"),
    candidate: {
      candidateRevision: revision("c"),
      compiler: {
        producer: "fixture-compiler",
        model: "fixture-model",
        version: "1",
        inferenceId: "compiler-1",
      },
    },
    verification: {
      disposition,
      verificationRevision: revision("d"),
      verifier: {
        producer: "fixture-verifier",
        model: "fixture-model",
        version: "1",
        inferenceId: "verifier-1",
      },
      blockers: disposition === "rejected" ? [{ code: "missing_meaning" }] : [],
      uncertainty: disposition === "unresolved" ? [{ code: "uncertain_meaning" }] : [],
    },
    measurements: {
      compiler: {
        durationMs: 4,
        inputTokens: 700,
        cachedInputTokens: 100,
        outputTokens: 90,
        costMicrounits: 12,
      },
      verifier: {
        durationMs: 5,
        inputTokens: 900,
        cachedInputTokens: 0,
        outputTokens: 70,
        costMicrounits: 14,
      },
    },
  };
}

class FakeInferenceRuntime {
  constructor(result = acceptedInspection()) {
    this.result = result;
    this.requests = [];
  }

  async inspect(request) {
    this.requests.push(request);
    if (this.result instanceof Error) throw this.result;
    return this.result;
  }
}

class FakeCheckpointPublisher {
  constructor(outcome = null) {
    this.outcome = outcome ?? {
      status: "published",
      publication: { checkpointRevision: revision("e"), continuationState: { value: true } },
      ledgerEntry: { entryRevision: revision("f") },
    };
    this.requests = [];
  }

  async publish(request) {
    this.requests.push(request);
    if (this.outcome instanceof Error) throw this.outcome;
    return this.outcome;
  }
}

function clocks() {
  let timestampIndex = 0;
  let monotonic = 0;
  return {
    now: () => `2026-08-26T00:00:0${timestampIndex++}.000Z`,
    monotonicNow: () => {
      monotonic += 10;
      return monotonic;
    },
  };
}

function coordinator({ inference, publisher, store, scheduleValue } = {}) {
  return new ShadowContextLifecycleCoordinator({
    logicalRoleInstanceId: "strategic-planner:main",
    pressureController: new ContextPressureController({ policy: policy() }),
    inferenceRuntime: inference ?? new FakeInferenceRuntime(),
    checkpointPublisher: publisher ?? new FakeCheckpointPublisher(),
    episodeStore: store ?? new InMemoryContextLifecycleEpisodeStore(),
    schedule: scheduleValue ?? schedule(),
    ...clocks(),
  });
}

test("shadow schedule is explicit, revision-bound, and closed", () => {
  const first = validateShadowContextLifecycleSchedule(schedule());
  const second = validateShadowContextLifecycleSchedule(schedule());
  assert.equal(first.scheduleRevision, second.scheduleRevision);
  assert.throws(
    () => validateShadowContextLifecycleSchedule(schedule({ inspectAt: [] })),
    /non-empty array/,
  );
  assert.throws(
    () => validateShadowContextLifecycleSchedule(schedule({ inspectAt: ["urgent"] })),
    /unsupported pressure disposition/,
  );
  assert.throws(
    () => validateShadowContextLifecycleSchedule({ ...schedule(), retireAcceptedCheckpoint: true }),
    /unsupported fields/,
  );
});

test("comfortable pressure records telemetry without invoking semantic work", async () => {
  const inference = new FakeInferenceRuntime();
  const publisher = new FakeCheckpointPublisher();
  const store = new InMemoryContextLifecycleEpisodeStore();
  const result = await coordinator({ inference, publisher, store }).observe({
    episodeId: "episode-comfortable",
    subject: subject(),
    pressureObservation: pressureObservation(1, 5_000),
    contextTelemetry: {
      retainedInputTokens: 20_000,
      reportedContextWindowTokens: 100_000,
      growthSincePreviousTokens: 1_200,
    },
  });
  assert.equal(result.status, "recorded");
  assert.equal(result.episode.inference.status, "not_scheduled");
  assert.equal(result.episode.checkpoint.reason, "inspection_not_scheduled");
  assert.deepEqual(result.episode.transition, {
    status: "not_requested",
    retirementAttempted: false,
  });
  assert.equal(inference.requests.length, 0);
  assert.equal(publisher.requests.length, 0);
  assert.equal(verifyContextLifecycleEpisode(result.episode), true);
});

test("replacement pressure inspects, publishes, and emits comparable measurements", async () => {
  const inference = new FakeInferenceRuntime();
  const publisher = new FakeCheckpointPublisher();
  const store = new InMemoryContextLifecycleEpisodeStore();
  const runtime = coordinator({ inference, publisher, store });
  const first = await runtime.observe({
    episodeId: "episode-candidate",
    subject: subject(),
    pressureObservation: pressureObservation(1, 8_100),
    projection: projection(),
    sourceMaterials: [{ reference: "bounded-fixture" }],
    contextTelemetry: {
      retainedInputTokens: 42_000,
      reportedContextWindowTokens: 100_000,
      growthSincePreviousTokens: 3_000,
      estimatedAvoidedInputTokens: 60_000,
      estimateMethod: "fixture-counterfactual-v1",
      estimateRevision: revision("9"),
    },
  });
  assert.equal(first.episode.inference.status, "accepted");
  assert.equal(first.episode.checkpoint.status, "published");
  assert.equal(first.episode.measurements.compiler.inputTokens, 700);
  assert.equal(first.episode.measurements.inspectionDurationMs, 10);
  assert.equal(first.episode.measurements.publicationDurationMs, 10);
  assert.ok(first.episode.measurements.checkpointBytes > 0);
  assert.equal(inference.requests.length, 1);
  assert.equal(publisher.requests.length, 1);

  const replay = await runtime.observe({
    episodeId: "episode-candidate",
    subject: subject(),
    pressureObservation: pressureObservation(1, 8_100),
    projection: projection(),
    sourceMaterials: [{ reference: "bounded-fixture" }],
    contextTelemetry: {
      retainedInputTokens: 42_000,
      reportedContextWindowTokens: 100_000,
      growthSincePreviousTokens: 3_000,
      estimatedAvoidedInputTokens: 60_000,
      estimateMethod: "fixture-counterfactual-v1",
      estimateRevision: revision("9"),
    },
  });
  assert.equal(replay.status, "replayed");
  assert.equal(inference.requests.length, 1);

  const summary = summarizeContextLifecycleEpisodes(store.receipts());
  assert.equal(summary.counts.inspected, 1);
  assert.equal(summary.counts.published, 1);
  assert.deepEqual(summary.measurements.estimatedAvoidedInputTokens, {
    observed: 1,
    missing: 0,
    total: 60_000,
  });
});

test("concurrent exact replay shares one shadow inspection", async () => {
  let release;
  const gate = new Promise((resolve) => { release = resolve; });
  const inference = new FakeInferenceRuntime();
  const originalInspect = inference.inspect.bind(inference);
  inference.inspect = async (request) => {
    await gate;
    return originalInspect(request);
  };
  const runtime = coordinator({ inference });
  const request = {
    episodeId: "episode-concurrent",
    subject: subject(),
    pressureObservation: pressureObservation(1, 8_100),
    projection: projection(),
  };
  const first = runtime.observe(request);
  const second = runtime.observe(structuredClone(request));
  release();
  const [firstResult, secondResult] = await Promise.all([first, second]);
  assert.equal(firstResult.episode.episodeRevision, secondResult.episode.episodeRevision);
  assert.equal(inference.requests.length, 1);

  const conflictRuntime = coordinator({ inference: new FakeInferenceRuntime() });
  const pending = conflictRuntime.observe(request);
  await assert.rejects(
    conflictRuntime.observe({ ...request, pressureObservation: pressureObservation(1, 9_500) }),
    /already running with different evidence/,
  );
  await pending;
});

test("a rejected verification is measured but never published", async () => {
  const inference = new FakeInferenceRuntime(acceptedInspection("rejected"));
  const publisher = new FakeCheckpointPublisher();
  const result = await coordinator({ inference, publisher }).observe({
    episodeId: "episode-rejected",
    subject: subject(),
    pressureObservation: pressureObservation(1, 9_200),
    projection: projection(),
  });
  assert.equal(result.episode.inference.status, "rejected");
  assert.equal(result.episode.inference.blockerCount, 1);
  assert.equal(result.episode.checkpoint.status, "not_attempted");
  assert.equal(result.episode.checkpoint.reason, "verification_not_accepted");
  assert.equal(publisher.requests.length, 0);
});

test("projection and publication failures remain observable and fail closed", async () => {
  const mismatch = await coordinator().observe({
    episodeId: "episode-mismatch",
    subject: subject(),
    pressureObservation: pressureObservation(1, 8_100),
    projection: projection({
      observedContext: {
        logicalRoleInstanceId: "strategic-planner:other",
        runtimeBinding: { threadId: "thread-shadow-1", bindingRevision: 3 },
      },
    }),
  });
  assert.equal(mismatch.status, "failed");
  assert.equal(mismatch.episode.failure.stage, "projection_validation");
  assert.equal(mismatch.episode.inference.status, "failed");
  assert.equal(mismatch.episode.transition.retirementAttempted, false);

  const publisher = new FakeCheckpointPublisher(new Error("fixture store unavailable"));
  const publication = await coordinator({ publisher }).observe({
    episodeId: "episode-publication-failure",
    subject: subject(),
    pressureObservation: pressureObservation(1, 8_100),
    projection: projection(),
  });
  assert.equal(publication.status, "failed");
  assert.equal(publication.episode.inference.status, "accepted");
  assert.equal(publication.episode.checkpoint.status, "failed");
  assert.equal(publication.episode.failure.stage, "checkpoint_publication");
});

test("summaries fail closed across policy or schedule revisions", async () => {
  const first = await coordinator().observe({
    episodeId: "episode-first-policy",
    subject: subject(),
    pressureObservation: pressureObservation(1, 5_000),
  });
  const alternate = coordinator({
    scheduleValue: schedule({ inspectAt: ["approaching", "replacement_candidate", "critical"] }),
  });
  const second = await alternate.observe({
    episodeId: "episode-second-schedule",
    subject: subject(),
    pressureObservation: pressureObservation(1, 5_000),
  });
  assert.throws(
    () => summarizeContextLifecycleEpisodes([first.episode, second.episode]),
    /cannot mix policy or schedule revisions/,
  );
  const tampered = structuredClone(first.episode);
  tampered.measurements.context.retainedInputTokens = 1;
  assert.equal(verifyContextLifecycleEpisode(tampered), false);
});
