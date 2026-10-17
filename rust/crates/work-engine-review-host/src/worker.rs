use review_workflow::{Workflow, protocol::Command, startup::TrustedStartup};
use serde_json::{Value, json};
use std::path::PathBuf;
use tokio::sync::mpsc;

pub struct Work {
    pub command: Command,
    // Reserved before admission: ordinary control replies cannot consume it.
    pub reply: mpsc::OwnedPermit<Value>,
}
pub enum Event {
    Ready(Value),
    Finished(Value),
    Failed(Value),
}

// A completed attempt can contain joined readbacks larger than the bounded
// status envelope. Keep only its identity and outcome on the control plane.
macro_rules! selected_attempt_status {
    ($snapshot:expr) => {
        $snapshot.progress.attempts.last().map(|attempt| {
            json!({
                "attemptId": attempt.attempt_id,
                "preparationOperationId": attempt.preparation_operation_id,
                "dispatchOperationId": attempt.dispatch.as_ref().map(|d| &d.operation_id),
                "completionPending": attempt.completion_intent.is_some() && attempt.outcome.is_none(),
                "outcome": attempt.outcome,
            })
        })
    };
}
pub fn spawn(
    path: PathBuf,
    mut commands: mpsc::Receiver<Work>,
    events: mpsc::Sender<Event>,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("workflow runtime");
        runtime.block_on(async move {
            let startup = match TrustedStartup::load(&path) {
                Ok(v) => v,
                Err(e) => {
                    let _ = events.send(Event::Failed(json!(e))).await;
                    return;
                }
            };
            let mut workflow = match Workflow::open(startup) {
                Ok(v) => v,
                Err(e) => {
                    let _ = events.send(Event::Failed(json!(e))).await;
                    return;
                }
            };
            let snapshot = match workflow.snapshot() {
                Ok(s) => json!({"revision":s.revision,"phase":s.phase,
                    "selectedAttempt":selected_attempt_status!(s)}),
                Err(e) => {
                    let _ = events.send(Event::Failed(json!(e))).await;
                    return;
                }
            };
            if events.send(Event::Ready(snapshot)).await.is_err() {
                return;
            }
            while let Some(work) = commands.recv().await {
                let result = workflow.run(&work.command).await;
                let response = match result {
                    Ok(value) => {
                        json!({"version":1,"requestId":work.command.request_id,"result":value})
                    }
                    Err(error) => {
                        json!({"version":1,"requestId":work.command.request_id,"error":error})
                    }
                };
                work.reply.send(response);
                let observation = match workflow.snapshot() {
                    Ok(s) => json!({"revision":s.revision,"phase":s.phase,
                        "selectedAttempt":selected_attempt_status!(s)}),
                    Err(e) => json!({"unavailable":e}),
                };
                if events.send(Event::Finished(observation)).await.is_err() {
                    return;
                }
            }
        });
    })
}
