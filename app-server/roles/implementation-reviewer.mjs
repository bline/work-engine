import { ReviewerRuntimeError } from "../src/services/reviewer-runtime/contract.mjs";
import { createAgentInstructionReviewService } from "../src/services/agent-instruction-review/service.mjs";
import { AgentInstructionReviewError } from "../src/services/agent-instruction-review/contract.mjs";
import { ImplementationReviewError } from "../src/services/implementation-review/contract.mjs";
import { renderReviewerClaimContext } from "../src/services/claim-evidence/reviewer-projection.mjs";

export class ImplementationReviewerRuntime {
  constructor({ manifest, adapter, agentInstructionReview = createAgentInstructionReviewService() }) {
    if (!manifest?.projectRole || !adapter?.execute) throw new TypeError("reviewer runtime requires manifest and adapter");
    if (!agentInstructionReview?.renderDelivery || !agentInstructionReview?.admit) {
      throw new TypeError("reviewer runtime requires an agent-instruction-review service");
    }
    Object.assign(this, { manifest, adapter, agentInstructionReview });
  }

  projection(instanceId) {
    const projection = this.manifest.projectRole("implementation-reviewer", instanceId);
    if (projection.role.threadOptions.sandbox !== "read-only" || projection.role.effects.length) {
      throw new ReviewerRuntimeError("configuration", "reviewer role exceeds read-only effect ceiling");
    }
    return projection;
  }

  async review({ instanceId, profileId, subject, catalogProjection, rawEventPolicy,
    continuationSessionId = null, claimContext = null, resultCorrection = null,
    refreshCredentials = false }) {
    const projection = this.projection(instanceId);
    const roleInstructions = claimContext === null
      ? projection.role.developerInstructions
      : `${projection.role.developerInstructions.trim()}\n\n${renderReviewerClaimContext(claimContext)}`;
    return this.adapter.execute({
      instanceId, profileId, subject, catalogProjection, rawEventPolicy,
      continuationSessionId, roleInstructions, resultCorrection, refreshCredentials,
    });
  }

  async reviewAgentInstructions({
    instanceId, profileId, subject, closure, catalogProjection, rawEventPolicy,
    continuationSessionId = null, claimContext = null,
    resultCorrection = null, refreshCredentials = false,
  }) {
    const projection = this.projection(instanceId);
    const delivery = await this.agentInstructionReview.renderDelivery({
      reviewerRoleProjection: projection,
      closure,
    });
    const roleInstructions = claimContext === null
      ? delivery.roleInstructions
      : `${delivery.roleInstructions.trim()}\n\n${renderReviewerClaimContext(claimContext)}`;
    const execution = await this.adapter.execute({
      instanceId, profileId, subject, catalogProjection, rawEventPolicy,
      continuationSessionId, roleInstructions, resultCorrection, refreshCredentials,
    });
    if (execution.failure || !execution.result) return execution;
    let specialistReview;
    try { specialistReview = this.agentInstructionReview.admit({ result: execution.result, closure }); }
    catch (error) {
      if (!(error instanceof AgentInstructionReviewError)
          && !(error instanceof ImplementationReviewError)) throw error;
      return Object.freeze({...execution, specialistReview: null, contractError: error});
    }
    return Object.freeze({ ...execution, specialistReview, deliveryRevision: delivery.deliveryRevision });
  }

  async retire(instanceId) { return this.adapter.retire(instanceId); }
}
