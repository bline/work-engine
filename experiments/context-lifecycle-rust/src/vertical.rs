//! One bounded native text lifecycle, driven only through coordinator commands.
use crate::codex::TextReceipt;
use crate::*;
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct TextVerticalConfig {
    pub first_input: Vec<u8>,
    pub second_input: Vec<u8>,
    pub first_fact: String,
    pub second_fact: String,
    pub continuation_objective: String,
    pub limited_package_adequacy: String,
    pub runtime: RuntimeId,
    pub authority: SyntheticAuthority,
    pub transition_deadline: u64,
    pub observation_timeout_ms: u64,
}

#[derive(Serialize, Deserialize)]
pub struct TextVerticalResult {
    pub first: TextReceipt,
    pub readiness: TextReceipt,
    pub second: TextReceipt,
    pub package: Bootstrap,
    pub old_owner: Stamp,
}

fn accepted_text(client: &Client, id: EffectId, timeout_ms: u64) -> Result<TextReceipt> {
    client.observer().wait(
        Condition::Effect(id.clone(), EffectState::Success),
        timeout_ms,
    )?;
    let effect = client.observer().result(id)?;
    ensure!(
        effect.domain == EvidenceDomain::CodexTextTurnCompletion
            && effect.state == EffectState::Success,
        "native text operation is not complete"
    );
    effect
        .codex
        .and_then(|observation| observation.text_receipt)
        .ok_or_else(|| anyhow::anyhow!("accepted native text receipt missing"))
}

/// No inference retry or provider control is hidden here. Every entered turn is
/// a coordinator-owned effect, including successor readiness.
pub fn run_text_vertical(
    client: &Client,
    config: TextVerticalConfig,
) -> Result<TextVerticalResult> {
    ensure!(
        matches!(config.authority.mode, Mode::CodexText | Mode::CodexTextLive),
        "native text authority required"
    );
    ensure!(
        !config.first_fact.trim().is_empty()
            && !config.second_fact.trim().is_empty()
            && !config.limited_package_adequacy.trim().is_empty(),
        "explicit bounded acceptance fields required"
    );
    let owner_a = client.register(
        CommandId::new("vertical:register"),
        ContextId::new("vertical-context"),
    )?;
    let old_owner = owner_a.stamp();
    for (key, id, payload) in [
        (
            "vertical:queue:first",
            "vertical-input-1",
            config.first_input,
        ),
        (
            "vertical:queue:second",
            "vertical-input-2",
            config.second_input,
        ),
    ] {
        client.execute(Command {
            key: CommandId::new(key),
            action: Action::Enqueue {
                owner: old_owner.clone(),
                input: InputId::new(id),
                payload,
                lineage: format!("native-vertical:{id}"),
            },
        })?;
    }
    let first_id = client.admit(
        CommandId::new("vertical:admit:first"),
        &owner_a,
        InputId::new("vertical-input-1"),
        DeliveryId::new("vertical-delivery-1"),
        EffectId::new("vertical-effect-1"),
        config.authority.clone(),
    )?;
    let first = accepted_text(client, first_id, config.observation_timeout_ms)?;
    ensure!(
        first.output == config.first_fact,
        "first continuation fact mismatch"
    );

    let permit = client.replace(
        CommandId::new("vertical:replace"),
        owner_a,
        TransitionId::new("vertical-transition"),
        config.transition_deadline,
    )?;
    client.execute(Command {
        key: CommandId::new("vertical:capture"),
        action: Action::Capture {
            permit: permit.clone(),
            continuation: Continuation {
                objective: config.continuation_objective,
                authority: config.authority.approval.clone(),
                commitments: vec!["one successor readiness turn before queued input".into()],
                references: vec![first.output.clone()],
            },
        },
    })?;
    let readiness_id = client.begin_readiness(
        CommandId::new("vertical:readiness"),
        permit,
        EffectId::new("vertical-readiness-effect"),
        DeliveryId::new("vertical-readiness-delivery"),
        config.authority.clone(),
    )?;
    let readiness = accepted_text(client, readiness_id, config.observation_timeout_ms)?;
    let package = client
        .observer()
        .readiness_package(TransitionId::new("vertical-transition"))?;
    ensure!(
        readiness.package_digest.as_deref() == Some(digest(&package).as_str())
            && readiness.thread_id != first.thread_id,
        "successor readiness binding mismatch"
    );
    let owner_b = client.activate_text(
        CommandId::new("vertical:activate"),
        Attestation {
            context: package.context.clone(),
            transition: package.transition.clone(),
            successor: package.successor,
            revision: package.revision,
            digest: digest(&package),
            runtime: config.runtime,
            model_context: readiness.thread_id.clone(),
        },
        config.limited_package_adequacy,
    )?;
    let second_id = client.admit(
        CommandId::new("vertical:admit:second"),
        &owner_b,
        InputId::new("vertical-input-2"),
        DeliveryId::new("vertical-delivery-2"),
        EffectId::new("vertical-effect-2"),
        config.authority,
    )?;
    let second = accepted_text(client, second_id, config.observation_timeout_ms)?;
    ensure!(
        second.output == config.second_fact && second.thread_id == readiness.thread_id,
        "second input did not preserve successor fact/thread"
    );
    Ok(TextVerticalResult {
        first,
        readiness,
        second,
        package,
        old_owner,
    })
}
