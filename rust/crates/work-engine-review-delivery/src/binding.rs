//! The private canonical experiment binding (feature `fixture`). Mints no authority.
//!
//! One [`ExperimentBinding`] is the only value the joined entrypoint
//! ([`crate::native::deliver_and_consume`]) accepts. Its constructor derives the
//! [`ReviewPreparation`] from the canonical candidate and request through the existing
//! `prepare` owner (or compares a supplied preparation, in full, with that recomputation), then
//! joins it with the labelled fixture declaration and the episode inputs. A mismatch is refused
//! here, before any native child exists. The fields are private and the joined entrypoint takes
//! no independent candidate, preparation or declaration, so a preparation for one candidate or
//! subject cannot be delivered while another is consumed.
//!
//! Revision, writer, phase, result and finding-lineage validation stay with the unchanged
//! `consume_fixture_result` and the episode reducer; this module only checks that the delivered
//! preparation and the consumed declaration/episode describe the same candidate, obligation,
//! subject and conditions. [`FrozenDelivery`] stays a pure byte type: freezing alone establishes
//! neither canonical derivation nor consumer composition.
//!
//! Compile-time guard: the binding's fields are private, so a replacement preparation cannot be
//! swapped in after construction.
//!
//! ```compile_fail,E0616
//! use work_engine_review_delivery::binding::ExperimentBinding;
//! fn swap(binding: &ExperimentBinding) {
//!     let _ = &binding.preparation;
//! }
//! ```

use review_episode_core::codec::{digest, field};
use review_episode_core::identity::{Authority, Revision};
use review_episode_core::state::ReviewEpisodeState;
use work_engine_organization::OrganizationCandidate;
use work_engine_review_preparation::fixture::{
    ConsumeRefusal, ConsumptionProof, FixtureConsumption, FixtureDeclaration,
    consume_fixture_result, fixture_coverage,
};
use work_engine_review_preparation::{
    PreparationRefusal, PreparationRequest, ReviewPreparation, prepare,
};

use crate::{DeliveryRefusal, FrozenDelivery, Transport};

/// What the consumer needs besides the candidate, declaration and native result. Supplied once to
/// the binding constructor; the binding keeps its own snapshot.
#[derive(Debug, Clone)]
pub struct EpisodeInputs {
    pub state: ReviewEpisodeState,
    pub authority: Authority,
    pub expected_revision: Revision,
    pub transition_id: String,
    pub unresolved_questions: Vec<String>,
    pub admissions_json: String,
}

/// Why the join was refused. Every variant is raised before a native child or prompt exists.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum BindingRefusal {
    #[error("the owner refused to prepare the request: {0}")]
    Preparation(PreparationRefusal),
    /// The supplied preparation is not equal to the canonical recomputation. `part` names the
    /// first differing part; equality of identity strings alone is never enough.
    #[error("supplied preparation differs from canonical recomputation in `{part}`")]
    PreparationMismatch { part: &'static str },
    #[error("declaration {field} `{declared}` conflicts with `{actual}`")]
    Declaration {
        field: &'static str,
        declared: String,
        actual: String,
    },
    #[error("episode {field} `{episode}` conflicts with `{expected}`")]
    Episode {
        field: &'static str,
        episode: String,
        expected: String,
    },
    #[error("declaration refused by the fixture owner: {0}")]
    Coverage(ConsumeRefusal),
    #[error("a required condition in the declaration is not in the prepared context: `{0}`")]
    ConditionNotInPreparation(String),
    #[error("condition `{condition}` is bound to boundary `{declared}`, prepared as `{prepared}`")]
    ConditionBoundary {
        condition: String,
        declared: String,
        prepared: String,
    },
    /// The request carries a realization claim. The description is derived by the host from what
    /// it will run (`native::realization_description`); a caller cannot supply or override it.
    #[error(
        "the request supplies a realization claim; the host derives the realization description"
    )]
    CallerRealization,
    #[error(transparent)]
    Delivery(DeliveryRefusal),
}

fn first_difference(a: &ReviewPreparation, b: &ReviewPreparation) -> Option<&'static str> {
    if a.status != b.status {
        return Some("status");
    }
    if a.model_context.identity != b.model_context.identity {
        return Some("model_context.identity");
    }
    if a.model_context.obligation != b.model_context.obligation {
        return Some("model_context.obligation");
    }
    if a.model_context.vantage != b.model_context.vantage {
        return Some("model_context.vantage");
    }
    if a.model_context.conditions != b.model_context.conditions {
        return Some("model_context.conditions");
    }
    if a.host_enforcement != b.host_enforcement {
        return Some("host_enforcement");
    }
    if a.provenance != b.provenance {
        return Some("provenance");
    }
    if a.realization_evaluation != b.realization_evaluation {
        return Some("realization_evaluation");
    }
    if a.blockers != b.blockers {
        return Some("blockers");
    }
    // Complete structural equality is the final authority, whatever the parts above report.
    if a != b {
        return Some("preparation");
    }
    None
}

/// One checked, immutable join of canonical preparation, fixture declaration, episode inputs and
/// the synthetic result. Private fields; no mutating accessor.
#[derive(Debug, Clone)]
pub struct ExperimentBinding {
    candidate: OrganizationCandidate,
    request: PreparationRequest,
    declaration: FixtureDeclaration,
    preparation: ReviewPreparation,
    frozen: FrozenDelivery,
    episode: EpisodeInputs,
    synthetic_result: String,
    /// Deterministic observation seam: how many times the existing consumer was actually
    /// invoked through this binding (and its clones). Not an authority; evidence for tests.
    consumer_invocations: std::sync::Arc<std::sync::atomic::AtomicU64>,
    /// Deterministic observation seam: native spawn attempts through this binding.
    spawn_attempts: std::sync::Arc<std::sync::atomic::AtomicU64>,
}

impl ExperimentBinding {
    /// Derives the preparation internally from `candidate` and `request` through the existing
    /// preparation owner, joins it with `declaration` and `episode`, and freezes the delivery
    /// bytes. Refuses a mismatched join.
    pub fn new(
        candidate: &OrganizationCandidate,
        request: &PreparationRequest,
        declaration: &FixtureDeclaration,
        episode: EpisodeInputs,
        synthetic_result: &str,
        transport: Transport,
    ) -> Result<Self, BindingRefusal> {
        Self::build(
            candidate,
            request,
            declaration,
            episode,
            synthetic_result,
            transport,
            None,
        )
    }

    /// As [`ExperimentBinding::new`], but the caller also supplies a preparation, which must equal
    /// the canonical recomputation in full. The recomputation, never the supplied value, is what
    /// is delivered.
    pub fn from_supplied_preparation(
        candidate: &OrganizationCandidate,
        request: &PreparationRequest,
        declaration: &FixtureDeclaration,
        episode: EpisodeInputs,
        synthetic_result: &str,
        transport: Transport,
        supplied: &ReviewPreparation,
    ) -> Result<Self, BindingRefusal> {
        Self::build(
            candidate,
            request,
            declaration,
            episode,
            synthetic_result,
            transport,
            Some(supplied),
        )
    }

    fn build(
        candidate: &OrganizationCandidate,
        request: &PreparationRequest,
        declaration: &FixtureDeclaration,
        episode: EpisodeInputs,
        synthetic_result: &str,
        transport: Transport,
        supplied: Option<&ReviewPreparation>,
    ) -> Result<Self, BindingRefusal> {
        if request.realization.is_some() {
            return Err(BindingRefusal::CallerRealization);
        }
        // The preparation is derived with the host's own description of what it will run, so the
        // delivered path always carries a realization evaluation.
        let request = &PreparationRequest {
            realization: Some(crate::native::runner_realization()),
            ..request.clone()
        };
        let preparation = prepare(candidate, request).map_err(BindingRefusal::Preparation)?;
        if let Some(part) = supplied.and_then(|s| first_difference(&preparation, s)) {
            return Err(BindingRefusal::PreparationMismatch { part });
        }
        let conflict =
            |field: &'static str, declared: String, actual: String| BindingRefusal::Declaration {
                field,
                declared,
                actual,
            };
        // Declaration versus the canonical preparation and request.
        let identity = &preparation.model_context.identity;
        if declaration.candidate_content_digest != identity.candidate_content_digest {
            return Err(conflict(
                "candidate_content_digest",
                declaration.candidate_content_digest.clone(),
                identity.candidate_content_digest.clone(),
            ));
        }
        if declaration.obligation_id != preparation.model_context.obligation.id {
            return Err(conflict(
                "obligation_id",
                declaration.obligation_id.clone(),
                preparation.model_context.obligation.id.clone(),
            ));
        }
        if declaration.subject != preparation.model_context.obligation.subject {
            return Err(conflict(
                "subject",
                format!("{:?}", declaration.subject),
                format!("{:?}", preparation.model_context.obligation.subject),
            ));
        }
        for (name, declared, actual) in [
            (
                "problem_scope.run_id",
                declaration.problem_scope.run_id.clone(),
                identity.run_id.clone(),
            ),
            (
                "problem_scope.plan_lineage",
                declaration.problem_scope.plan_lineage.clone(),
                identity.plan_lineage.clone(),
            ),
            (
                "problem_scope.slice_number",
                declaration.problem_scope.slice_number.to_string(),
                identity.slice_number.to_string(),
            ),
        ] {
            if declared != actual {
                return Err(conflict(name, declared, actual));
            }
        }
        // Conditions: the fixture owner derives the required set from the same candidate; each
        // must be a prepared condition with the same boundary.
        let coverage =
            fixture_coverage(candidate, declaration).map_err(BindingRefusal::Coverage)?;
        for covered in &coverage.conditions {
            let prepared = preparation
                .model_context
                .conditions
                .iter()
                .find(|c| c.id == covered.condition)
                .ok_or_else(|| {
                    BindingRefusal::ConditionNotInPreparation(covered.condition.clone())
                })?;
            if prepared.boundary != covered.boundary {
                return Err(BindingRefusal::ConditionBoundary {
                    condition: covered.condition.clone(),
                    declared: covered.boundary.clone(),
                    prepared: prepared.boundary.clone(),
                });
            }
        }
        // Episode versus the same obligation and subject. Revision, writer, phase and lineage are
        // not checked here; the episode owner checks them at consumption.
        let state_identity = episode.state.identity();
        let episode_obligation = state_identity
            .review_obligation_id
            .to_string_checked()
            .map_err(|e| BindingRefusal::Episode {
                field: "review_obligation_id",
                episode: e.to_string(),
                expected: preparation.model_context.obligation.id.clone(),
            })?;
        if episode_obligation != preparation.model_context.obligation.id {
            return Err(BindingRefusal::Episode {
                field: "review_obligation_id",
                episode: episode_obligation,
                expected: preparation.model_context.obligation.id.clone(),
            });
        }
        let subject_sha = digest(&preparation.model_context.obligation.subject.to_js_value());
        let episode_subject = field(episode.state.get("subject"), "sha256")
            .and_then(|v| v.as_text())
            .and_then(|t| t.to_string_checked())
            .map_err(|e| BindingRefusal::Episode {
                field: "subject.sha256",
                episode: e.to_string(),
                expected: subject_sha.clone(),
            })?;
        if episode_subject != subject_sha {
            return Err(BindingRefusal::Episode {
                field: "subject.sha256",
                episode: episode_subject,
                expected: subject_sha,
            });
        }
        let frozen = FrozenDelivery::freeze(preparation.clone(), transport)
            .map_err(BindingRefusal::Delivery)?;
        Ok(Self {
            candidate: candidate.clone(),
            request: request.clone(),
            // `request` is the host-derived request, not the caller's.
            declaration: declaration.clone(),
            preparation,
            frozen,
            episode,
            synthetic_result: synthetic_result.to_owned(),
            consumer_invocations: Default::default(),
            spawn_attempts: Default::default(),
        })
    }

    pub(crate) fn candidate(&self) -> &OrganizationCandidate {
        &self.candidate
    }

    /// Runtime count of native child spawn attempts through this binding (and its clones),
    /// incremented by the private runner immediately before it starts the child. A gate refusal
    /// before the spawn must show 0. Evidence for tests, not an authority.
    pub fn spawn_attempts(&self) -> u64 {
        self.spawn_attempts
            .load(std::sync::atomic::Ordering::SeqCst)
    }

    pub(crate) fn record_spawn_attempt(&self) {
        self.spawn_attempts
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }

    /// The canonical preparation the binding derived.
    pub fn preparation(&self) -> &ReviewPreparation {
        &self.preparation
    }
    pub fn frozen(&self) -> &FrozenDelivery {
        &self.frozen
    }
    /// The request the preparation was derived from: the caller's obligation and subject with the
    /// host-derived realization description.
    pub fn request(&self) -> &PreparationRequest {
        &self.request
    }
    pub fn declaration(&self) -> &FixtureDeclaration {
        &self.declaration
    }
    /// The synthetic result text Rust supplied for the one attempt.
    pub fn synthetic_result(&self) -> &str {
        &self.synthetic_result
    }
    /// The episode state before any consumption (never modified by the consumer).
    pub fn episode_state(&self) -> &ReviewEpisodeState {
        &self.episode.state
    }
    pub fn expected_revision(&self) -> &Revision {
        &self.episode.expected_revision
    }
    pub fn admissions_json(&self) -> &str {
        &self.episode.admissions_json
    }
    pub fn transition_id(&self) -> &str {
        &self.episode.transition_id
    }
    /// Runtime count of actual consumer invocations through this binding (incremented only by
    /// `consume`, immediately before the existing consumer runs). A refused attempt must show 0.
    /// There is no modelled increment: the count is only what the consumer actually did.
    pub fn consumer_invocations(&self) -> u64 {
        self.consumer_invocations
            .load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Consumption through the existing consumer, exclusively from this binding's own inputs and
    /// the consumption gate's proof (which the consumer checks against those inputs itself).
    pub(crate) fn consume(
        &self,
        result_text: &str,
        proof: &ConsumptionProof<'_>,
    ) -> Result<FixtureConsumption, ConsumeRefusal> {
        self.consumer_invocations
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let questions: Vec<&str> = self
            .episode
            .unresolved_questions
            .iter()
            .map(String::as_str)
            .collect();
        consume_fixture_result(
            &self.candidate,
            &self.declaration,
            &self.episode.state,
            &self.episode.authority,
            self.episode.expected_revision.clone(),
            &self.episode.transition_id,
            result_text,
            &questions,
            &self.episode.admissions_json,
            proof,
        )
    }
}
