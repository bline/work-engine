//! UI projection of independently observed task components. No command port.
use crate::task::{Availability, Case, Component, Node, SummarySection, Task};
use serde_json::Value;
use std::{
    path::PathBuf,
    sync::mpsc::{self, Receiver, SyncSender},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};
use work_engine_terminal_ui::task_observation::{
    self as wire, Component as Observed, Request, Selection,
};

pub struct Metadata {
    pub request: Request,
    pub endpoint: PathBuf,
    pub host: String,
    pub status: String,
    pub result_absent: bool,
    pub revision: u64,
}
pub struct Connection {
    pub endpoint: PathBuf,
    pub request: Request,
    pub selection: Selection,
    pub generation: u64,
    pub disconnected: bool,
    pub refreshing: bool,
    pub pending: bool,
    pub next_refresh: Instant,
    pub host: String,
    pub revision: u64,
}
pub struct Job {
    pub generation: u64,
    pub endpoint: PathBuf,
    pub request: Request,
}
pub struct Reply {
    pub generation: u64,
    pub result: Result<wire::Observation, wire::ReadError>,
}
pub struct Worker {
    sender: Option<SyncSender<Job>>,
    receiver: Receiver<Reply>,
    thread: Option<JoinHandle<()>>,
}
impl Worker {
    pub fn start() -> Self {
        let (sender, jobs) = mpsc::sync_channel::<Job>(1);
        let (replies, receiver) = mpsc::channel();
        let thread = thread::spawn(move || {
            while let Ok(job) = jobs.recv() {
                let result = wire::read(&job.endpoint, &job.request, Duration::from_millis(600));
                if replies
                    .send(Reply {
                        generation: job.generation,
                        result,
                    })
                    .is_err()
                {
                    break;
                }
            }
        });
        Self {
            sender: Some(sender),
            receiver,
            thread: Some(thread),
        }
    }
    pub fn request(&self, job: Job) -> Result<(), ()> {
        self.sender.as_ref().unwrap().try_send(job).map_err(|_| ())
    }
    pub fn try_reply(&self) -> Option<Reply> {
        self.receiver.try_recv().ok()
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        self.sender.take();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
impl Connection {
    pub fn new(endpoint: PathBuf, request: Request) -> Self {
        Self {
            endpoint,
            request,
            selection: Selection::default(),
            generation: 0,
            disconnected: false,
            refreshing: false,
            pending: false,
            next_refresh: Instant::now(),
            host: "not observed".into(),
            revision: 0,
        }
    }
    pub fn read(&mut self) {
        if self.refreshing {
            return;
        }
        self.disconnected = false;
        self.generation = self
            .selection
            .begin(self.endpoint.clone(), self.request.clone());
        self.refreshing = true;
        self.pending = true;
    }
    pub fn disconnect(&mut self) {
        self.generation = self
            .selection
            .begin(self.endpoint.clone(), self.request.clone());
        self.disconnected = true;
        self.refreshing = false;
        self.pending = false;
        self.selection
            .apply(self.generation, Err(wire::ReadError::Unavailable));
        self.revision += 1;
    }
    pub fn apply(&mut self, reply: Reply) -> bool {
        if self.disconnected || !self.refreshing || reply.generation != self.generation {
            return false;
        }
        if let Ok(observation) = &reply.result {
            self.host = serde_json::to_string(&observation.host_identity).unwrap();
        }
        if !self.selection.apply(reply.generation, reply.result) {
            return false;
        }
        self.refreshing = false;
        self.next_refresh = Instant::now() + Duration::from_secs(4);
        self.revision += 1;
        true
    }
    pub fn project(&self, previous: Option<&Task>) -> Task {
        let status = if self.disconnected {
            "Disconnected · last known only".into()
        } else if self.refreshing {
            "Reading · previous data is last known".into()
        } else if let Some(error) = &self.selection.transport_error {
            format!("{error:?} · last known only")
        } else {
            "Read-only · independent component reads".into()
        };
        let mut task = Task {
            live: Some(Metadata {
                request: self.request.clone(),
                endpoint: self.endpoint.clone(),
                host: self.host.clone(),
                status,
                result_absent: false,
                revision: self.revision,
            }),
            case: Case::Pending,
            objective: "Objective unavailable".into(),
            execution: "Execution unavailable".into(),
            question_pending: false,
            selected: 0,
            components: vec![],
            nodes: vec![],
        };
        for (name, title) in wire::COMPONENTS.into_iter().zip([
            "Owner",
            "Lifecycle",
            "Native segments",
            "Command",
            "Control",
        ]) {
            let view = self.selection.components.get(name);
            let availability = view.map_or(Availability::Unavailable, |v| match v.latest {
                Observed::Present { .. } => Availability::Present,
                Observed::Absent { .. } => Availability::Absent,
                Observed::NotRequested { .. } => Availability::NotRequested,
                Observed::NotApplicable { .. } => Availability::NotApplicable,
                Observed::Unavailable { .. } => Availability::Unavailable,
                Observed::Refused { .. } => Availability::Refused,
            });
            task.components.push(Component {
                name: title,
                availability,
                stale: view.is_some_and(|v| v.stale),
                binding: view.map_or_else(
                    || "No observation received".into(),
                    |v| serde_json::to_string(&v.latest).unwrap(),
                ),
                detail: view
                    .and_then(|v| v.last_known.as_ref())
                    .map_or_else(String::new, |c| {
                        format!(
                            "Last observed present component: {}",
                            serde_json::to_string(c).unwrap()
                        )
                    }),
            });
        }
        let root = format!("task:{}", self.request.task_id);
        task.nodes.push(Node {
            id: root.clone().into(),
            title: format!("Task · {}", self.request.task_id),
            parent: None,
            text: None,
            source: self.request.task_id.clone(),
            component: 0,
        });
        let value = |name: &str| {
            self.selection
                .components
                .get(name)
                .and_then(|v| v.last_known.as_ref())
                .and_then(Observed::value)
        };
        if let Some(owner) = value("owner") {
            task.objective = string(owner, "objective");
            task.live.as_mut().unwrap().result_absent = owner.get("result") == Some(&Value::Null);
            for (group, key, text_key) in [
                ("Input", "inputs", "engineering_question"),
                ("Question", "questions", "text"),
                ("Effect", "completed_effects", ""),
            ] {
                for (index, item) in owner[key].as_array().into_iter().flatten().enumerate() {
                    let identity = ["input_id", "id", "effect_id"]
                        .iter()
                        .find_map(|key| item[*key].as_str())
                        .map(str::to_owned)
                        .unwrap_or_else(|| index.to_string());
                    let mut text = item[text_key].as_str().map(str::to_owned);
                    if group == "Question" {
                        task.question_pending |= item["state"] == "pending";
                        if let Some(body) = &mut text {
                            if let Some(answer) = item
                                .pointer("/state/answered/answer")
                                .and_then(Value::as_str)
                            {
                                body.push_str(&format!(
                                    "\n\nRecorded answer: {answer}\nAuthority: {}",
                                    item.pointer("/state/answered/authority_ref")
                                        .unwrap_or(&Value::Null)
                                ));
                            } else {
                                body.push_str(&format!("\n\nQuestion state: {}", item["state"]));
                            }
                        }
                    }
                    task.nodes.push(Node {
                        id: format!("{group}:{identity}").into(),
                        title: format!("{group} {identity}"),
                        parent: Some(root.clone()),
                        text,
                        source: item.to_string(),
                        component: 0,
                    });
                }
            }
            if let Some(result) = owner["result"].as_object() {
                let result = Value::Object(result.clone());
                task.nodes.push(Node {
                    id: format!("result:{}", string(&result, "result_id")).into(),
                    title: format!(
                        "Result {} · published, settlement independent",
                        string(&result, "result_id")
                    ),
                    parent: Some(root.clone()),
                    text: result["body"].as_str().map(str::to_owned),
                    source: result.to_string(),
                    component: 0,
                });
            }
        }
        for (component, name) in [
            (1, "lifecycle"),
            (2, "native_delivery_status"),
            (3, "command_result"),
            (4, "control_result"),
        ] {
            let parent = if component <= 2 {
                Some(root.clone())
            } else {
                None
            };
            task.nodes.push(Node {
                id: name.into(),
                title: format!(
                    "{} · {}{}",
                    task.components[component].name,
                    task.components[component].availability.label(),
                    if component >= 3 {
                        " · task binding not established"
                    } else {
                        ""
                    }
                ),
                parent,
                text: None,
                source: task.components[component].binding.clone(),
                component,
            });
            if let Some(value) = value(name) {
                if component == 1
                    && let Ok(snapshot) =
                        serde_json::from_value::<lifecycle_wire::LifecycleSnapshotV1>(value.clone())
                {
                    let projection = crate::contract::snapshot_conversation(
                        &snapshot,
                        crate::presentation::SourceId::Client(0),
                        "independent lifecycle read",
                    );
                    task.execution = projection
                        .entries
                        .iter()
                        .skip(1)
                        .take(2)
                        .map(|e| e.text.as_str())
                        .collect::<Vec<_>>()
                        .join("\n");
                }
                if component == 2 {
                    for (index, segment) in value["segments"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .enumerate()
                    {
                        task.nodes.push(Node {
                            id: format!(
                                "segment:{}:{}:{}",
                                string(segment, "delivery_id"),
                                string(segment, "invocation_id"),
                                index
                            )
                            .into(),
                            title: format!(
                                "Segment {} · {}",
                                string(segment, "invocation_id"),
                                string(segment, "segment_state")
                            ),
                            parent: Some(name.into()),
                            text: None,
                            source: segment.to_string(),
                            component,
                        });
                    }
                }
            }
        }
        if let Some(old) = previous {
            task.selected = task
                .nodes
                .iter()
                .position(|n| n.id == old.selected_node().id)
                .unwrap_or(0);
        }
        task
    }
}
fn string(value: &Value, key: &str) -> String {
    value[key].as_str().unwrap_or("unavailable").into()
}
pub fn details(task: &Task) -> String {
    let live = task.live.as_ref().unwrap();
    let node = task.selected_node();
    let components = task
        .components
        .iter()
        .enumerate()
        .filter(|(i, _)| task.selected == 0 || *i == node.component)
        .map(|(_, c)| format!("{}\n{}\n{}", c.summary(), c.binding, c.detail))
        .collect::<Vec<_>>()
        .join("\n\n");
    format!(
        "{}\nTask: {}\nEndpoint: {}\nHost: {}\n{}\n\nSource: {}\n\nIndependent component reads; non-atomic. Revisions and completeness belong to each component. Command/task binding is not established.\n\n{}",
        node.title,
        live.request.task_id,
        live.endpoint.display(),
        live.host,
        live.status,
        node.source,
        components
    )
}
pub fn overview(task: &Task) -> Vec<SummarySection> {
    let result = task.nodes.iter().position(|n| n.id.starts_with("result:"));
    let stale = task.components[0].stale;
    let result_body = result
        .map(|i| {
            task.nodes[i]
                .text
                .clone()
                .unwrap_or_else(|| "Published result body unavailable".into())
        })
        .unwrap_or_else(|| {
            if task.live.as_ref().unwrap().result_absent {
                "No result published at this owner read.".into()
            } else {
                "Result unavailable; no absence inferred.".into()
            }
        });
    let attention = task
        .components
        .iter()
        .map(Component::summary)
        .collect::<Vec<_>>()
        .join("\n");
    vec![
        SummarySection {
            title: "OBJECTIVE",
            body: format!(
                "{}{}",
                if stale { "LAST KNOWN · " } else { "" },
                task.objective
            ),
            node: Some(0),
            warning: stale,
        },
        SummarySection {
            title: "NEEDS ATTENTION",
            body: format!(
                "{}\n{}\n{}",
                task.live.as_ref().unwrap().status,
                if task.question_pending {
                    "Pending owner question; execution may continue independently."
                } else {
                    "Inspect owner questions and their availability below."
                },
                attention
            ),
            node: task
                .nodes
                .iter()
                .position(|n| n.id.starts_with("Question:")),
            warning: task.question_pending
                || task.components.iter().any(|c| {
                    c.stale
                        || matches!(
                            c.availability,
                            Availability::Unavailable | Availability::Refused
                        )
                }),
        },
        SummarySection {
            title: "LATEST RESULT",
            body: format!("{}{result_body}", if stale { "LAST KNOWN · " } else { "" }),
            node: result,
            warning: stale,
        },
        SummarySection {
            title: "EXECUTION",
            body: format!(
                "{}{}\nExecution does not establish task completion.",
                if task.components[1].stale {
                    "LAST KNOWN · "
                } else {
                    ""
                },
                task.execution
            ),
            node: task.nodes.iter().position(|n| n.id == "lifecycle"),
            warning: task.components[1].stale,
        },
    ]
}
