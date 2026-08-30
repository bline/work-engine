import { requireRecord, requireText } from "../src/services/slice-campaign/contract.mjs";

export class SliceBuilderRuntime {
  constructor({ adapter, manifest }) { this.adapter = adapter; this.manifest = manifest; }
  async deliverPhase({ instanceId, clientUserMessageId, acceptedBoundary, campaign, requestContext = {}, text, signal }) {
    requireText(instanceId, "builder instanceId"); requireText(text, "builder turn text"); requireRecord(acceptedBoundary, "accepted boundary");
    const projection = this.manifest.projectRole("slice-builder", instanceId);
    const delivery = await this.adapter.deliverTurn({ role: projection.role, skills: projection.skills, clientUserMessageId,
      requestContext: { ...requestContext, "work-engine.accepted-boundary": { kind: "application", value: JSON.stringify(acceptedBoundary) },
        "work-engine.slice-campaign": { kind: "application", value: JSON.stringify(campaign) } }, text });
    const completion = await this.adapter.waitForTurnCompletion({ ...delivery, signal });
    return Object.freeze({ ...delivery, completion });
  }
}
