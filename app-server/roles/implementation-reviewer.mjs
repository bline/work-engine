import { ReviewerRuntimeError } from "../src/services/reviewer-runtime/contract.mjs";
import { createAgentInstructionReviewService } from "../src/services/agent-instruction-review/service.mjs";

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

  async review({ instanceId, profileId, subject, catalogProjection, rawEventPolicy, continuationSessionId = null }) {
    const projection = this.projection(instanceId);
    return this.adapter.execute({
      instanceId, profileId, subject, catalogProjection, rawEventPolicy,
      continuationSessionId, roleInstructions: projection.role.developerInstructions,
    });
  }

  async reviewAgentInstructions({
    instanceId, profileId, subject, closure, catalogProjection, rawEventPolicy,
    continuationSessionId = null,
  }) {
    const projection = this.projection(instanceId);
    const delivery = await this.agentInstructionReview.renderDelivery({
      reviewerRoleProjection: projection,
      closure,
    });
    const execution = await this.adapter.execute({
      instanceId, profileId, subject, catalogProjection, rawEventPolicy,
      continuationSessionId, roleInstructions: delivery.roleInstructions,
    });
    if (execution.failure || !execution.result) return execution;
    const specialistReview = this.agentInstructionReview.admit({ result: execution.result, closure });
    return Object.freeze({ ...execution, specialistReview, deliveryRevision: delivery.deliveryRevision });
  }

  async retire(instanceId) { return this.adapter.retire(instanceId); }
}
