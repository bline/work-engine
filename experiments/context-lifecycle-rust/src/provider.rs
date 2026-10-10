//! Deliberately synthetic provider with an independently persisted ledger.
use crate::storage::Store;
use crate::*;
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderRecord {
    pub request: ProviderRequest,
    pub evidence: Vec<Evidence>,
    pub executions: u64,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(clippy::large_enum_variant)] // Serialized local protocol; preserve its explicit unboxed variants.
pub enum ProviderAction {
    Submit(ProviderRequest),
    Publish {
        effect: EffectId,
        outcome: Outcome,
        sealed: bool,
    },
    /// Explicit broken-provider injection. Normal Publish cannot contradict a seal.
    Contradict {
        effect: EffectId,
        outcome: Outcome,
    },
    /// Native interruption request; acknowledgment is not completion or settlement.
    Interrupt {
        effect: EffectId,
    },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderCommand {
    pub key: CommandId,
    pub action: ProviderAction,
}
impl ProviderCommand {
    pub fn submit(request: ProviderRequest) -> Self {
        Self {
            key: CommandId::new(format!("submit:{}", request.effect.as_str())),
            action: ProviderAction::Submit(request),
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[allow(clippy::large_enum_variant)] // Serialized local protocol; preserve its explicit unboxed variants.
pub enum ProviderRequestMessage {
    Command(ProviderCommand),
    Lookup(EffectId),
    Snapshot,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(clippy::large_enum_variant)] // Serialized local protocol; preserve its explicit unboxed variants.
pub enum ProviderReply {
    Record(ProviderRecord),
    Codex(crate::codex::Observation),
    Snapshot {
        records: Vec<ProviderRecord>,
        submit_calls: u64,
    },
}
#[derive(Clone, Default, Serialize, Deserialize)]
struct Ledger {
    records: BTreeMap<EffectId, ProviderRecord>,
    commands: BTreeMap<CommandId, (ProviderCommand, std::result::Result<ProviderReply, String>)>,
    submit_calls: u64,
}
pub(crate) struct FakeProvider {
    store: Store,
    ledger: Ledger,
}
impl FakeProvider {
    pub fn open(path: &Path) -> Result<Self> {
        let store = Store::open(path)?;
        let ledger = store.load()?;
        Ok(Self { store, ledger })
    }
    pub fn handle(&mut self, request: ProviderRequestMessage) -> Result<ProviderReply> {
        match request {
            ProviderRequestMessage::Lookup(id) => self
                .ledger
                .records
                .get(&id)
                .cloned()
                .map(ProviderReply::Record)
                .ok_or_else(|| anyhow::anyhow!("evidence unavailable: no provider record")),
            ProviderRequestMessage::Snapshot => Ok(ProviderReply::Snapshot {
                records: self.ledger.records.values().cloned().collect(),
                submit_calls: self.ledger.submit_calls,
            }),
            ProviderRequestMessage::Command(command) => {
                ensure!(
                    !command.key.as_str().is_empty(),
                    "empty provider command identity"
                );
                // Independent diagnostic count includes even a repeated network submission.
                if matches!(command.action, ProviderAction::Submit(_)) {
                    let mut observed = self.ledger.clone();
                    observed.submit_calls += 1;
                    self.store.save(&observed)?;
                    self.ledger = observed;
                }
                if let Some((prior, reply)) = self.ledger.commands.get(&command.key) {
                    ensure!(*prior == command, "provider command key conflict");
                    return reply.clone().map_err(anyhow::Error::msg);
                }
                let mut next = self.ledger.clone();
                let reply: Result<ProviderReply> = (|| {
                    let reply = match &command.action {
                        ProviderAction::Submit(request) => {
                            ensure!(
                                !next.records.contains_key(&request.effect),
                                "effect authorization already used"
                            );
                            ensure!(
                                !next
                                    .records
                                    .values()
                                    .any(|r| r.request.delivery == request.delivery),
                                "delivery authorization already used"
                            );
                            ensure!(
                                request.input_digest == digest(&request.payload)
                                    && !request.authority.approval.is_empty(),
                                "invalid synthetic request"
                            );
                            let outcome = match request.authority.mode {
                                Mode::ObserveCodex | Mode::CodexText | Mode::CodexTextLive => {
                                    anyhow::bail!("Codex observation requires its explicit adapter")
                                }
                                Mode::Hold => None,
                                Mode::Succeed => Some(Outcome::Success("synthetic success".into())),
                                Mode::Fail => {
                                    Some(Outcome::Failure("synthetic immediate failure".into()))
                                }
                            };
                            let evidence = outcome
                                .map(|outcome| Evidence {
                                    effect: request.effect.clone(),
                                    request_digest: digest(request),
                                    outcome,
                                    sealed: true,
                                })
                                .into_iter()
                                .collect();
                            let record = ProviderRecord {
                                request: request.clone(),
                                evidence,
                                executions: 1,
                            };
                            next.records.insert(request.effect.clone(), record.clone());
                            ProviderReply::Record(record)
                        }
                        ProviderAction::Publish {
                            effect,
                            outcome,
                            sealed,
                        } => {
                            let record = next
                                .records
                                .get_mut(effect)
                                .ok_or_else(|| anyhow::anyhow!("unknown provider effect"))?;
                            ensure!(
                                !record
                                    .evidence
                                    .iter()
                                    .any(|e| e.sealed && (e.outcome != *outcome || !sealed)),
                                "sealed authorization cannot change or execute again"
                            );
                            let evidence = Evidence {
                                effect: effect.clone(),
                                request_digest: digest(&record.request),
                                outcome: outcome.clone(),
                                sealed: *sealed,
                            };
                            if !record.evidence.contains(&evidence) {
                                record.evidence.push(evidence);
                            }
                            ProviderReply::Record(record.clone())
                        }
                        ProviderAction::Contradict { effect, outcome } => {
                            let record = next
                                .records
                                .get_mut(effect)
                                .ok_or_else(|| anyhow::anyhow!("unknown provider effect"))?;
                            record.evidence.push(Evidence {
                                effect: effect.clone(),
                                request_digest: digest(&record.request),
                                outcome: outcome.clone(),
                                sealed: true,
                            });
                            ProviderReply::Record(record.clone())
                        }
                        ProviderAction::Interrupt { .. } => {
                            anyhow::bail!("fake provider has no native turn")
                        }
                    };
                    Ok(reply)
                })();
                if reply.is_err() {
                    next = self.ledger.clone();
                }
                let durable_reply = reply.as_ref().cloned().map_err(ToString::to_string);
                next.commands
                    .insert(command.key.clone(), (command, durable_reply));
                self.store.save(&next)?;
                self.ledger = next;
                reply
            }
        }
    }
}
