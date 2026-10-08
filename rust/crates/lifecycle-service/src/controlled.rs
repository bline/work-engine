#[cfg(feature = "controlled-proof")]
use std::time::Duration;
use std::{future::Future, path::PathBuf, pin::Pin};

#[cfg(feature = "controlled-proof")]
use lifecycle_core::{
    EffectSettlement, EvidenceId, ExecutionOutcome, ProviderThreadId, ProviderTurnId,
};
use lifecycle_provider::{
    OperationProfile, ProviderError, TextTurnObservation, TextTurnPort, TextTurnRequest,
};
#[cfg(feature = "controlled-proof")]
use lifecycle_runtime::{BoundedFrameReader, BoundedFrameWriter};
#[cfg(feature = "controlled-proof")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "controlled-proof")]
use tokio::net::UnixStream;

#[cfg(feature = "controlled-proof")]
const PEER_FRAME_MAX: usize = 131_072;
#[cfg(feature = "controlled-proof")]
const PEER_DEADLINE: Duration = Duration::from_secs(10);

#[derive(Clone)]
pub struct ControlledPort {
    #[cfg_attr(not(feature = "controlled-proof"), allow(dead_code))]
    pub socket: PathBuf,
}

#[cfg(feature = "controlled-proof")]
#[derive(Serialize)]
struct PeerRequest<'a> {
    subject_id: &'a str,
    effect_id: &'a str,
    attempt_id: &'a str,
    incarnation_id: &'a str,
    input_id: &'a str,
    text: &'a str,
    text_digest: String,
}

#[cfg(feature = "controlled-proof")]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PeerResult {
    source_id: String,
    effect_id: String,
    attempt_id: String,
    incarnation_id: String,
    provider_thread_id: String,
    provider_turn_id: String,
    final_text: Option<String>,
    outcome: String,
    settlement_kind: String,
    settlement_evidence: Option<String>,
}

impl TextTurnPort for ControlledPort {
    type Run = Pin<Box<dyn Future<Output = Result<TextTurnObservation, ProviderError>> + Send>>;

    fn profile(&self) -> OperationProfile {
        OperationProfile::ControlledText
    }

    fn execute(&self, request: TextTurnRequest) -> Self::Run {
        #[cfg(feature = "controlled-proof")]
        {
            let socket = self.socket.clone();
            Box::pin(async move { execute(socket, request).await })
        }
        #[cfg(not(feature = "controlled-proof"))]
        {
            let _ = request;
            Box::pin(async { Err(ProviderError::UnsupportedProfile) })
        }
    }
}

#[cfg(feature = "controlled-proof")]
async fn execute(
    socket: PathBuf,
    request: TextTurnRequest,
) -> Result<TextTurnObservation, ProviderError> {
    let stream = tokio::time::timeout(PEER_DEADLINE, UnixStream::connect(socket))
        .await
        .map_err(|_| ProviderError::EntryUncertain)?
        .map_err(|_| ProviderError::ObservationUnavailable)?;
    let (read, write) = stream.into_split();
    let mut writer = BoundedFrameWriter::new(write, PEER_FRAME_MAX)
        .map_err(|_| ProviderError::ObservationUnavailable)?;
    let mut reader = BoundedFrameReader::new(read, PEER_FRAME_MAX)
        .map_err(|_| ProviderError::ObservationUnavailable)?;
    let message = PeerRequest {
        subject_id: request.subject.as_str(),
        effect_id: request.effect.as_str(),
        attempt_id: request.attempt.as_str(),
        incarnation_id: request.incarnation.as_str(),
        input_id: request.input.input_id().as_str(),
        text: request.input.text(),
        text_digest: request.input.digest().hex(),
    };
    let bytes = serde_json::to_vec(&message).map_err(|_| ProviderError::ObservationUnavailable)?;
    tokio::time::timeout(PEER_DEADLINE, writer.write_frame(&bytes))
        .await
        .map_err(|_| ProviderError::EntryUncertain)?
        .map_err(|_| ProviderError::EntryUncertain)?;
    let bytes = tokio::time::timeout(PEER_DEADLINE, reader.read_frame())
        .await
        .map_err(|_| ProviderError::ObservationUnavailable)?
        .map_err(|_| ProviderError::ObservationUnavailable)?
        .ok_or(ProviderError::ObservationUnavailable)?;
    let result: PeerResult =
        serde_json::from_slice(&bytes).map_err(|_| ProviderError::ObservationUnavailable)?;
    if result
        .final_text
        .as_ref()
        .is_some_and(|text| text.len() > 65_536)
    {
        return Err(ProviderError::ObservationUnavailable);
    }
    let outcome = match result.outcome.as_str() {
        "pending" => ExecutionOutcome::Pending,
        "completed" => ExecutionOutcome::Completed,
        "failed" => ExecutionOutcome::Failed,
        "interrupted" => ExecutionOutcome::Interrupted,
        "unknown" => ExecutionOutcome::Unknown,
        _ => return Err(ProviderError::ObservationUnavailable),
    };
    let settlement = match (result.settlement_kind.as_str(), result.settlement_evidence) {
        ("unresolved", None) => EffectSettlement::Unresolved,
        ("established", Some(id)) => EffectSettlement::Established(
            EvidenceId::parse(id).map_err(|_| ProviderError::ObservationUnavailable)?,
        ),
        ("conflict", Some(id)) => EffectSettlement::Conflict(
            EvidenceId::parse(id).map_err(|_| ProviderError::ObservationUnavailable)?,
        ),
        _ => return Err(ProviderError::ObservationUnavailable),
    };
    Ok(TextTurnObservation {
        source: EvidenceId::parse(result.source_id)
            .map_err(|_| ProviderError::ObservationUnavailable)?,
        effect: lifecycle_core::EffectId::parse(result.effect_id)
            .map_err(|_| ProviderError::ObservationUnavailable)?,
        attempt: lifecycle_core::AttemptId::parse(result.attempt_id)
            .map_err(|_| ProviderError::ObservationUnavailable)?,
        incarnation: lifecycle_core::RuntimeIncarnation::parse(result.incarnation_id)
            .map_err(|_| ProviderError::ObservationUnavailable)?,
        thread: ProviderThreadId::parse(result.provider_thread_id)
            .map_err(|_| ProviderError::ObservationUnavailable)?,
        turn: ProviderTurnId::parse(result.provider_turn_id)
            .map_err(|_| ProviderError::ObservationUnavailable)?,
        final_text: result.final_text,
        outcome,
        settlement,
    })
}
