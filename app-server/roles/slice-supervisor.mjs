import { requireRecord, requireText } from "../src/services/slice-campaign/contract.mjs";

export class SliceSupervisorRuntime {
  constructor({ adapter, manifest }) { this.adapter = adapter; this.manifest = manifest; }
  async requestDecision({ instanceId, clientUserMessageId, campaign, pendingObligation, signal }) {
    requireText(instanceId, "supervisor instanceId"); requireRecord(campaign, "campaign projection"); requireText(pendingObligation, "pending obligation");
    const projection = this.manifest.projectRole("slice-supervisor", instanceId);
    const delivery = await this.adapter.deliverTurn({ role: projection.role, skills: projection.skills, clientUserMessageId,
      requestContext: { "work-engine.slice-campaign": { kind: "application", value: JSON.stringify(campaign) },
        "work-engine.pending-obligation": { kind: "application", value: pendingObligation } },
      text: "Decide the pending campaign obligation within the supplied authority. Return only the bounded phase decision; do not perform builder work or deterministic service operations." });
    const completion = await this.adapter.waitForTurnCompletion({ ...delivery, signal });
    return Object.freeze({ ...delivery, completion });
  }
}
