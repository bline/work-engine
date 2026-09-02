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
import { openWorkspaceDevelopmentRuntime } from "../workspace-coordination/runtime.mjs";
import { createCompletionPublicationService } from "./completion-publication.mjs";
import { createStrategicReconciliationHost } from "./strategic-reconciliation.mjs";
import { createChatboardAdapter } from "../operational-coordination/chatboard-adapter.mjs";
import { createNativeReviewHost, createNativeReviewHostOwners } from "./native-review-host.mjs";

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
  workspaceRoot, stateRoot, canonicalBranches,
  legacyAdapterFactory = createLegacySupervisorControlAdapter,
  nativeReviewOwnersFactory = createNativeReviewHostOwners,
} = {}) {
  const legacy = await legacyAdapterFactory({ workspaceRoot });
  const store = await openSqliteSliceCampaignStore({
    filePath: path.join(path.resolve(stateRoot), "slice-campaign.sqlite3"),
  });
  let closed = false;
  let workspace = null;
  let nativeReviewOwners = null;
  try {
    workspace = await openWorkspaceDevelopmentRuntime({
      repository: workspaceRoot,
      runtimeRoot: path.join(path.resolve(stateRoot), "workspace-development"),
      canonicalBranches,
    });
    const reviewSubject = createReviewSubjectService({ workspaceRoot });
    const implementationReview = createImplementationReviewService();
    const completionPublication = createCompletionPublicationService({ workspace });
    const strategicReconciliation = createStrategicReconciliationHost();
    const operationalCoordination = createChatboardAdapter({ workspaceRoot });
    nativeReviewOwners = await nativeReviewOwnersFactory({workspaceRoot, stateRoot});
    const service = createSliceCampaignService({
      store,
      reviewSubject,
      implementationReview: nativeReviewOwners.implementationReview ?? implementationReview,
      nativeReview: nativeReviewOwners.nativeReview,
      receiptFinalizer: {
        async finalize({ receipt }) { return legacy.validateReceipt(receipt); },
      },
      completionOffer: {
        async open({ request }) { return legacy.offer("open", { request }); },
      },
    });
    const nativeReview = createNativeReviewHost({workspaceRoot, campaignService: service,
      owners: nativeReviewOwners});

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
      "capability.workspace_coordination/acquire": ({ input }) => workspace.acquireResource({
        resource: input.resource, holder: input.holder, intentId: input.intent_id, ttlMs: input.ttl_ms,
      }),
      "capability.workspace_coordination/inspect": ({ input }) =>
        workspace.inspectResource(input.resource),
      "capability.workspace_coordination/release": ({ input }) => ({
        released: workspace.releaseResource(input.lease),
      }),
      "capability.worktree_lifecycle/allocate": ({ input }) => workspace.allocateAgentWorktree({
        operationId: input.operation_id, agentId: input.agent_id, intentId: input.intent_id,
        baselineCommit: input.baseline_commit, ...(input.ttl_ms === undefined ? {} : { ttlMs: input.ttl_ms }),
      }),
      "capability.worktree_lifecycle/cleanup": ({ input }) =>
        workspace.cleanupAgentWorktree(input.allocation),
      "capability.canonical_publication/prepare": ({ input }) => workspace.preparePublication({
        operationId: input.operation_id, targetBranch: input.target_branch,
        expectedParent: input.expected_parent, checkpoint: input.checkpoint,
        manifest: input.manifest, authorization: input.authorization, message: input.message,
      }),
      "capability.canonical_publication/seal_validation": ({ input }) =>
        workspace.sealPublication({ operationId: input.operation_id,
          preparationRevision: input.preparation_revision, validation: input.validation }),
      "capability.canonical_publication/promote": ({ input }) =>
        workspace.promotePublication({ operationId: input.operation_id,
          preparedRevision: input.prepared_revision,
          ...(input.publication_ttl_ms === undefined ? {} : { publicationTtlMs: input.publication_ttl_ms }) }),
      "capability.canonical_publication/reconcile": ({ input }) =>
        workspace.reconcilePublication({ operationId: input.operation_id,
          preparedRevision: input.prepared_revision }),
      "capability.completion_publication/prepare": ({ input }) =>
        completionPublication.prepare({ offer: input.offer,
          acceptedCheckpoint: input.accepted_checkpoint }),
      "capability.completion_publication/complete": ({ input }) =>
        completionPublication.complete({ offer: input.offer,
          preparationRevision: input.preparation_revision, validation: input.validation }),
      "capability.completion_publication/reconcile": ({ input }) =>
        completionPublication.reconcile({ offer: input.offer,
          preparationRevision: input.preparation_revision }),
      "capability.operational_coordination/read": ({ input }) =>
        operationalCoordination.execute("read", input),
      "capability.operational_coordination/claim": ({ input }) =>
        operationalCoordination.execute("claim", input),
      "capability.operational_coordination/post": ({ input }) =>
        operationalCoordination.execute("post", input),
      "capability.operational_coordination/release": ({ input }) =>
        operationalCoordination.execute("release", input),
      "capability.native_review/execute": ({ input }) => nativeReview.execute(input),
      "capability.native_review/recover": ({ input }) => nativeReview.recover(input),
      "capability.native_review/retry": ({ input }) => nativeReview.retry(input),
      "capability.native_review/record_finding_evaluation": ({ input }) =>
        nativeReview.recordFindingEvaluation(input),
      "capability.native_review/execute_remediation": ({ input }) =>
        nativeReview.executeRemediation(input),
    };
    const registrations = [];
    for (const [capability, operations] of Object.entries(supervisorCampaignCapabilityOperations)) {
      for (const operation of operations) {
        if (capability === "capability.strategic_reconciliation" && operation === "reconcile") {
          registrations.push({
            capability, operation,
            validateInput: strategicReconciliation.validateInput,
            async handler({ generationId, input }) {
              return wrap(generationId, capability, operation,
                await strategicReconciliation.handle({ generationId, input }));
            },
            validateOutput(value) {
              return validateSupervisorCapabilityOutput(capability, operation, value);
            },
          });
          continue;
        }
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
        try { runtime.close(); }
        finally {
          try { strategicReconciliation.close(); }
          finally {
            try { nativeReviewOwners.close(); }
            finally { try { workspace.close(); } finally { store.close(); } }
          }
        }
      },
      identity: Object.freeze({
        schema_version: 1,
        capabilities: Object.keys(supervisorCampaignCapabilityOperations).sort(),
        legacy: legacy.identity,
        state_path: store.filePath,
        workspace: Object.freeze({
          repository: workspace.repository,
          canonical_branches: workspace.canonicalBranches,
        }),
        operational_coordination: operationalCoordination.identity,
        native_review: Object.freeze({profile_id: "anthropic.claude-code.sonnet-review-v1",
          harness: "claude-code", gateway: "anthropic"}),
      }),
    });
  } catch (error) {
    if (!closed) {
      try { nativeReviewOwners?.close(); }
      finally { try { workspace?.close(); } finally { store.close(); } }
    }
    throw error;
  }
}
