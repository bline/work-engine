//! Unselected hosted initial transport correlation. Pure data: matching a
//! reported name does not authenticate the caller, Host, or returned artifacts.

use serde::{Deserialize, Serialize};
use serde_json::json;
use slice_campaign::{AppliedInitialDispatchProjection, codec::campaign_digest};

use crate::{Result, refuse};

const BINDING_SCHEMA: &str = "work-engine.hosted-initial-transport.v1";
const FIRST_GENERATION: u64 = 1;

/// Expected name for the sole initial hosted job of one owner-applied dispatch.
/// This cannot select an entrypoint or serve as a permit. The projection must
/// be obtained independently from the campaign rather than from a report.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostedInitialTransport {
    job_id: String,
    generation: u64,
    route_revision: String,
}

/// Data reported by an untrusted return. This shape carries no standing.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReportedHostedInitialTransport {
    pub job_id: String,
    pub generation: u64,
    pub route_revision: String,
}

impl HostedInitialTransport {
    /// Freeze a return slot using the independently selected revision already
    /// named by the applied command. One dispatch yields one job with exactly
    /// generation 1; changing the dispatch creates a different job, not retry.
    pub fn derive(
        owner: &AppliedInitialDispatchProjection,
        expected_route_revision: &str,
    ) -> Result<Self> {
        if expected_route_revision.trim().is_empty()
            || owner.reviewer_profile != expected_route_revision
        {
            return Err(refuse(
                "conflict",
                "hosted route revision differs from applied dispatch",
            ));
        }
        // This domain/version names the bytes used for transport correlation;
        // it does not modify CampaignIdentity, journal codecs, or episode IDs.
        let digest = campaign_digest(&json!({
            "schema": BINDING_SCHEMA,
            "rootId": owner.root_id,
            "identity": owner.identity,
            "obligationId": owner.obligation_id,
            "episodeId": owner.episode_id,
            "subjectDigest": owner.subject_digest,
            "selectionDigest": owner.selection.campaign_digest,
            "preparedRequestSha256": owner.request_digest,
            "preparationOperationId": owner.preparation.operation_id,
            "preparationRevision": owner.preparation.result_revision,
            "dispatchOperationId": owner.dispatch.operation_id,
            "dispatchRevision": owner.dispatch.result_revision,
            "dispatchCommandSha256": owner.dispatch.request_digest,
            "routeRevision": expected_route_revision,
            "generation": FIRST_GENERATION,
        }))?;
        Ok(Self {
            job_id: format!("review-{digest}"),
            generation: FIRST_GENERATION,
            route_revision: expected_route_revision.to_owned(),
        })
    }

    pub fn job_id(&self) -> &str {
        &self.job_id
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn route_revision(&self) -> &str {
        &self.route_revision
    }

    /// Check correspondence only. Consumption must additionally check the
    /// authority, custody, ordering, result, and same-attempt settlement.
    pub fn compare_report(&self, reported: &ReportedHostedInitialTransport) -> Result<()> {
        if reported.job_id != self.job_id
            || reported.generation != self.generation
            || reported.route_revision != self.route_revision
        {
            return Err(refuse(
                "conflict",
                "hosted transport differs from applied dispatch",
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use slice_campaign::{
        CampaignIdentity, CampaignRevision, CandidateRef, OperationReceipt, SelectionRef,
    };

    use super::*;

    const ROUTE: &str = "offline-example-messages-v1";

    fn projection(root: &str, attempt: &str) -> AppliedInitialDispatchProjection {
        let identity = CampaignIdentity {
            run_id: "one-run".into(),
            slice_number: 1,
            attempt_id: attempt.into(),
            plan_version: "plan-v1".into(),
        };
        let receipt = |op: &str, sha: &str| OperationReceipt {
            root_id: root.into(),
            identity: identity.clone(),
            operation_id: op.into(),
            kind: op.into(),
            request_digest: "a".repeat(64),
            profile_digest: "b".repeat(64),
            prior_revision: None,
            result_revision: CampaignRevision::new(sha.repeat(64)).unwrap(),
            result_reference: "fixture".into(),
        };
        let preparation = receipt("prepare_initial", "c");
        let dispatch = receipt("authorize_dispatch", "d");
        AppliedInitialDispatchProjection {
            root_id: root.into(),
            identity,
            obligation_id: "review".into(),
            episode_id: "episode".into(),
            reviewer_profile: ROUTE.into(),
            subject_digest: "e".repeat(64),
            candidate: CandidateRef {
                commit: "a".repeat(40),
                tree: "b".repeat(40),
                patch_identity: "9".repeat(64),
                receipt_sha256: "8".repeat(64),
                physical_profile_sha256: "7".repeat(64),
            },
            selection: SelectionRef {
                selection_id: "selection".into(),
                campaign_digest: "f".repeat(64),
                episode_authority_digest: "e".repeat(64),
            },
            request_digest: "6".repeat(64),
            preparation,
            dispatch,
        }
    }

    fn wire(expected: &HostedInitialTransport) -> ReportedHostedInitialTransport {
        serde_json::from_value(serde_json::to_value(expected).unwrap()).unwrap()
    }

    #[test]
    fn initial_transport_preserves_route_and_distinguishes_crossed_roots_and_attempts() {
        let a = HostedInitialTransport::derive(&projection("root-a", "one"), ROUTE).unwrap();
        let b = HostedInitialTransport::derive(&projection("root-b", "one"), ROUTE).unwrap();
        assert_eq!(
            a.job_id(),
            HostedInitialTransport::derive(&projection("root-a", "one"), ROUTE)
                .unwrap()
                .job_id()
        );
        assert_eq!(a.job_id().len(), 71);
        assert_eq!(a.generation(), FIRST_GENERATION);
        assert_eq!(a.route_revision(), ROUTE);
        assert!(a.compare_report(&wire(&a)).is_ok());
        assert!(b.compare_report(&wire(&b)).is_ok());
        assert!(a.compare_report(&wire(&b)).is_err());
        assert!(b.compare_report(&wire(&a)).is_err());
        let other_attempt =
            HostedInitialTransport::derive(&projection("root-a", "two"), ROUTE).unwrap();
        assert!(a.compare_report(&wire(&other_attempt)).is_err());
        let mut report = wire(&a);
        report.generation += 1;
        assert!(a.compare_report(&report).is_err());
        assert!(
            HostedInitialTransport::derive(&projection("root-a", "one"), "other-route").is_err()
        );
    }

    #[test]
    fn changing_only_the_dispatch_revision_invalidates_the_transport() {
        let mut owner = projection("root-a", "one");
        let expected = HostedInitialTransport::derive(&owner, ROUTE).unwrap();
        owner.dispatch.result_revision = CampaignRevision::new("5".repeat(64)).unwrap();
        let another = HostedInitialTransport::derive(&owner, ROUTE).unwrap();
        assert!(expected.compare_report(&wire(&another)).is_err());
    }
}
