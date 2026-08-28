import assert from "node:assert/strict";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import test from "node:test";

import { runAuthorizedClaimsVertical } from "../../../src/services/claim-evidence/authorized-vertical.mjs";
import { openSqliteClaimEvidenceStore } from "../../../src/services/claim-evidence/sqlite-store.mjs";

const exactReference = (reference, revision, byte = "a") => ({
  owner: "repository", reference, revision, integrity_sha256: byte.repeat(64),
  freshness: "current", status: "verified",
});

const authority = (profile, suffix) => ({
  schema_version: 1, grant_id: `grant:${suffix}`, actor: `producer:${suffix}`, profile,
  permissions: ["create_claim", "publish_revision"], decision_scope: `scope:${suffix}`,
  authority_reference: exactReference(`authority/${suffix}.json`, `authority-${suffix}`, suffix === "research" ? "b" : "c"),
});

const revision = (profile, sourceBinding, suffix) => ({
  proposition: `bounded ${suffix} proposition`, support_qualification: "supported",
  assumptions: [], limitations: [`${suffix} limitation`], confidence: { estimate: 0.9 },
  evidence_references: [sourceBinding], sensitivity_references: [], evidence_mode: "direct_source",
  judgment_kind: "semantic", decision_scope: `scope:${suffix}`,
  profile_payload: profile === "proposal-research-v1"
    ? { materiality: "material", support_qualification: "supported" }
    : { finding_id: "AIR-001", severity: "high", episode: "review-episode-1", outcome: "open" },
  reopening_conditions: [], tombstone: false,
});

const publication = (profile, suffix, sourceBinding, admittedAuthority = authority(profile, suffix)) => ({
  source_binding: sourceBinding,
  authority: admittedAuthority,
  operation: {
    schema_version: 1, operation_id: `publish:${suffix}`, action: "create_claim", profile,
    expected_state: null,
    payload: {
      subject: {
        namespace: suffix === "research" ? "proposal-research" : "review",
        subject_kind: suffix === "research" ? "proposal" : "revision-bound-finding",
        stable_subject_id: `${suffix}-subject`, evidence_baseline: sourceBinding,
        content_set: [sourceBinding.reference],
      },
      statement_identity: `${suffix} statement`,
      initial_revision: revision(profile, sourceBinding, suffix),
    },
  },
});

async function temporaryDatabase(t) {
  const directory = await mkdtemp(path.join(tmpdir(), "authorized-claims-vertical-test-"));
  t.after(async () => rm(directory, { recursive: true, force: true }));
  return path.join(directory, "claims.sqlite3");
}

function verticalRequest(researchPublication, findingPublication) {
  return {
    schema_version: 1,
    vertical_id: "claims-migration-slice-6",
    publications: [researchPublication, findingPublication],
    relevant_revisions: [
      { publication_operation_id: researchPublication.operation.operation_id, selection_reason: "governs the accepted slice placement" },
      { publication_operation_id: findingPublication.operation.operation_id, selection_reason: "review finding applies to the immutable candidate" },
    ],
  };
}

test("authorized host vertical publishes both profiles and supplies one bounded builder input", async (t) => {
  const filePath = await temporaryDatabase(t);
  const researchAuthority = authority("proposal-research-v1", "research");
  const findingAuthority = authority("revision-bound-review-finding-v1", "finding");
  const research = publication("proposal-research-v1", "research", exactReference("proposals/one.md", "proposal-commit", "d"), researchAuthority);
  const finding = publication("revision-bound-review-finding-v1", "finding", exactReference("reviews/one.json", "review-commit", "e"), findingAuthority);
  let store = await openSqliteClaimEvidenceStore({ filePath, bootstrapAuthorities: [researchAuthority, findingAuthority] });

  const result = runAuthorizedClaimsVertical(store, verticalRequest(research, finding));
  assert.equal(result.publications.length, 2);
  assert.deepEqual(result.publications.map((item) => item.profile), ["proposal-research-v1", "revision-bound-review-finding-v1"]);
  assert.equal(result.builder_input.context_kind, "relevant_exact_revisions");
  assert.equal(result.builder_input.relevant_exact_revisions.length, 2);
  assert.equal(result.builder_input.projection.completeness, "available");
  assert.equal(result.builder_input.projection.freshness, "current_after_verified_rebuild");
  assert.deepEqual(
    result.builder_input.relevant_exact_revisions.map((item) => item.revision.id),
    result.publications.map((item) => item.revision_id),
  );
  for (const selected of result.builder_input.relevant_exact_revisions) {
    assert.equal(typeof selected.selection_reason, "string");
    assert.equal(selected.authority_reference.status, "verified");
    assert.equal("permissions" in selected, false);
    assert.equal("operation" in selected, false);
    assert.equal("store" in selected, false);
  }
  assert.deepEqual(Object.keys(result.builder_input).sort(), ["context_kind", "projection", "relevant_exact_revisions", "schema_version"]);
  store.close();

  store = await openSqliteClaimEvidenceStore({ filePath });
  t.after(() => store.close());
  assert.deepEqual(store.exportStore().revisions.map((item) => item.id), result.publications.map((item) => item.revision_id));
  const replay = runAuthorizedClaimsVertical(store, verticalRequest(research, finding));
  assert.ok(replay.publications.every((item) => item.idempotent));
  assert.deepEqual(replay.builder_input, result.builder_input);
});

test("representative host and store fences refuse without pre-publication mutation", async (t) => {
  const cases = [
    {
      name: "profile uniqueness",
      mutate(request) {
        request.publications[1].operation.profile = "proposal-research-v1";
        request.publications[1].authority.profile = "proposal-research-v1";
      },
      message: /one publication for each profile/,
    },
    {
      name: "decision scope",
      mutate(request) { request.publications[0].operation.payload.initial_revision.decision_scope = "wrong-scope"; },
      message: /decision scope mismatch/,
    },
    {
      name: "verified exact source status",
      mutate(request) {
        request.publications[0].source_binding = { ...request.publications[0].source_binding, status: "unavailable" };
      },
      message: /source binding is not verified/,
    },
    {
      name: "source content containment",
      mutate(request) { request.publications[0].operation.payload.subject.content_set = ["different-source.md"]; },
      message: /outside the subject content set/,
    },
    {
      name: "closed publication count",
      mutate(request) { request.publications.pop(); },
      message: /exactly two publications/,
    },
    {
      name: "unknown relevant publication",
      mutate(request) { request.relevant_revisions[0].publication_operation_id = "publish:unknown"; },
      message: /does not name a publication/,
    },
    {
      name: "duplicate relevant publication",
      mutate(request) {
        request.relevant_revisions[1].publication_operation_id = request.relevant_revisions[0].publication_operation_id;
      },
      message: /relevant revisions must be unique/,
    },
    {
      name: "bounded relevant publications",
      mutate(request) {
        request.relevant_revisions.push({
          publication_operation_id: request.publications[0].operation.operation_id,
          selection_reason: "out-of-bound duplicate",
        });
      },
      message: /from 1 through 2 items/,
    },
    {
      name: "admitted grant remains store-owned",
      mutate(request) {
        request.publications[0].authority = {
          ...request.publications[0].authority,
          actor: "conflicting-producer",
        };
      },
      message: /authority grant conflict/,
    },
  ];

  for (const scenario of cases) {
    await t.test(scenario.name, async (subtest) => {
      const filePath = await temporaryDatabase(subtest);
      const researchAuthority = authority("proposal-research-v1", "research");
      const findingAuthority = authority("revision-bound-review-finding-v1", "finding");
      const research = publication("proposal-research-v1", "research", exactReference("proposals/one.md", "proposal-commit", "d"), researchAuthority);
      const finding = publication("revision-bound-review-finding-v1", "finding", exactReference("reviews/one.json", "review-commit", "e"), findingAuthority);
      const request = verticalRequest(research, finding);
      scenario.mutate(request);
      const store = await openSqliteClaimEvidenceStore({ filePath, bootstrapAuthorities: [researchAuthority, findingAuthority] });
      subtest.after(() => store.close());
      assert.throws(() => runAuthorizedClaimsVertical(store, request), scenario.message);
      assert.equal(store.exportStore().operations.length, 0);
    });
  }
});

test("independent authorization does not imply pair atomicity", async (t) => {
  const filePath = await temporaryDatabase(t);
  const researchAuthority = authority("proposal-research-v1", "research");
  const findingAuthority = authority("revision-bound-review-finding-v1", "finding");
  const research = publication("proposal-research-v1", "research", exactReference("proposals/one.md", "proposal-commit", "d"), researchAuthority);
  const finding = publication("revision-bound-review-finding-v1", "finding", exactReference("reviews/one.json", "review-commit", "e"), findingAuthority);
  const store = await openSqliteClaimEvidenceStore({ filePath, bootstrapAuthorities: [researchAuthority] });
  t.after(() => store.close());
  assert.throws(() => runAuthorizedClaimsVertical(store, verticalRequest(research, finding)), /not admitted/);
  assert.equal(store.exportStore().revisions.length, 1);
  assert.equal(store.exportStore().claims[0].profile, "proposal-research-v1");
});
