import path from "node:path";

import { createImplementationReviewService } from "../implementation-review/service.mjs";
import { createReviewSubjectService } from "../review-subject/service.mjs";
import {
  supervisorCampaignCapabilityOperations,
  validateSupervisorCapabilityInput,
  validateSupervisorCapabilityOutput,
} from "./capability-contract.mjs";
import { createSupervisorCampaignHostEffectRuntime } from "./host-effect-runtime.mjs";
import { createLegacySupervisorControlAdapter } from "./legacy-control-adapter.mjs";
import { createSliceCampaignService } from "./service.mjs";
import { openSqliteSliceCampaignStore } from "./sqlite-store.mjs";

function wrap(generationId, capability, operation, result) {
  return {
    schema_version: 1,
    generation_id: generationId,
    capability,
    operation,
    result,
  };
}

export async function createSupervisorCampaignCapabilityHostRuntime({
  workspaceRoot, stateRoot, legacyAdapterFactory = createLegacySupervisorControlAdapter,
} = {}) {
  const legacy = await legacyAdapterFactory({ workspaceRoot });
  const store = await openSqliteSliceCampaignStore({
    filePath: path.join(path.resolve(stateRoot), "slice-campaign.sqlite3"),
  });
  let closed = false;
  try {
    const reviewSubject = createReviewSubjectService({ workspaceRoot });
    const implementationReview = createImplementationReviewService();
    const service = createSliceCampaignService({
      store,
      reviewSubject,
      implementationReview,
      receiptFinalizer: {
        async finalize({ receipt }) { return legacy.validateReceipt(receipt); },
      },
      completionOffer: {
        async open({ request }) { return legacy.offer("open", { request }); },
      },
    });

    const handlers = {
      "capability.preflight/run": ({ input }) => legacy.preflight(input),
      "capability.lifecycle_control/admit": ({ input }) => service.admit(input),
      "capability.lifecycle_control/recover": ({ input }) => service.recover(input.identity),
      "capability.lifecycle_control/advance": ({ input }) => service.advance(input),
      "capability.lifecycle_control/bind_review_selection": ({ input }) =>
        service.bindReviewSelection(input),
      "capability.lifecycle_control/terminalize": ({ input }) => service.terminalize(input),
      "capability.receipt_finalization/finalize_named_campaign": ({ input }) =>
        legacy.finalize(input),
      "capability.checkpoint_lifecycle/bind_candidate": ({ input }) =>
        service.bindCandidate(input),
      "capability.checkpoint_lifecycle/accept": ({ input }) => legacy.checkpoint("accept", {
        candidate: input.candidate,
        review_result: input.review_result,
        gate_receipt_digest: input.gate_receipt_digest,
        expected_accepted: input.expected_accepted ?? null,
      }),
      "capability.checkpoint_lifecycle/stop": ({ input }) =>
        legacy.checkpoint("stop", { candidate: input.candidate }),
      "capability.completion_offer/open": async ({ input }) => {
        const campaign = await service.openCompletionOffer({
          identity: input.identity,
          expectedRevision: input.expected_revision,
          request: input.request,
        });
        return campaign.terminal.completionOffer;
      },
      "capability.completion_offer/load": ({ input }) => legacy.offer("load", input),
      "capability.completion_offer/resolve": ({ input }) => legacy.offer("resolve", input),
      "capability.completion_offer/reconcile": ({ input }) => legacy.offer("reconcile", input),
      "capability.completion_offer/expire": ({ input }) => legacy.offer("expire", input),
      "capability.resume/recover_active": ({ input }) => service.recover(input.identity),
      "capability.resume/recover_terminal": ({ input }) => legacy.resumeTerminal(input),
    };
    const registrations = [];
    for (const [capability, operations] of Object.entries(supervisorCampaignCapabilityOperations)) {
      for (const operation of operations) {
        const handler = handlers[`${capability}/${operation}`];
        if (typeof handler !== "function") {
          throw new Error(`missing real supervisor capability handler ${capability}/${operation}`);
        }
        registrations.push({
          capability,
          operation,
          validateInput(value) {
            return validateSupervisorCapabilityInput(capability, operation, value);
          },
          async handler({ generationId, input }) {
            const result = await handler({ generationId, input });
            return wrap(generationId, capability, operation, result);
          },
          validateOutput(value) {
            return validateSupervisorCapabilityOutput(capability, operation, value);
          },
        });
      }
    }
    const runtime = createSupervisorCampaignHostEffectRuntime({ registrations });
    return Object.freeze({
      dispatch(request) { return runtime.dispatch(request); },
      close() {
        if (closed) return;
        closed = true;
        runtime.close();
        store.close();
      },
      identity: Object.freeze({
        schema_version: 1,
        capabilities: Object.keys(supervisorCampaignCapabilityOperations).sort(),
        legacy: legacy.identity,
        state_path: store.filePath,
      }),
    });
  } catch (error) {
    if (!closed) store.close();
    throw error;
  }
}
