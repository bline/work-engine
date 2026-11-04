//! Data-only projection assembly. Source selection and protected publication are
//! deliberately unwired until the authoritative task/admission store is bound.
use crate::{Ref, hash, nonempty};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskProjection {
    pub task: Ref,
    pub request_json: String,
    pub admission: Option<Value>,
    pub result: Option<Value>,
}
#[derive(Serialize)]
struct Snapshot<'a> {
    schema: &'static str,
    observed_at: &'a str,
    source_revision: &'a str,
    tasks: &'a [TaskProjection],
}
#[derive(Debug)]
pub struct ProjectionError;
/// Assemble a snapshot only when each embedded task identity is the digest of
/// the exact request bytes and every present view binds that same identity.
pub fn snapshot(
    observed_at: &str,
    source_revision: &str,
    tasks: &[TaskProjection],
) -> Result<String, ProjectionError> {
    let observed =
        time::OffsetDateTime::parse(observed_at, &time::format_description::well_known::Rfc3339)
            .map_err(|_| ProjectionError)?;
    if observed.offset() != time::UtcOffset::UTC || source_revision.is_empty() {
        return Err(ProjectionError);
    }
    for item in tasks {
        #[derive(Deserialize)]
        struct IntentIdentity {
            task_id: String,
            revision: String,
        }
        #[derive(Deserialize)]
        struct Data {
            intent: IntentIdentity,
        }
        #[derive(Deserialize)]
        struct Request {
            data: Data,
        }
        let row: Request = serde_json::from_str(&item.request_json).map_err(|_| ProjectionError)?;
        if !nonempty(&row.data.intent.task_id)
            || !nonempty(&row.data.intent.revision)
            || item.task.id != row.data.intent.task_id
            || item.task.revision != row.data.intent.revision
            || item.task.sha256 != hash(&item.request_json)
        {
            return Err(ProjectionError);
        }
        for (view, schema) in [
            (&item.admission, "dot.admission-view/v1-proposal"),
            (&item.result, "dot.result-view/v1-proposal"),
        ] {
            if let Some(value) = view
                && (value.get("schema").and_then(Value::as_str) != Some(schema)
                    || serde_json::from_value::<Ref>(
                        value.get("task").cloned().ok_or(ProjectionError)?,
                    )
                    .map_err(|_| ProjectionError)?
                        != item.task)
            {
                return Err(ProjectionError);
            }
        }
        for (view, fields) in [
            (
                &item.admission,
                &[
                    "schema",
                    "admission_id",
                    "task",
                    "decision",
                    "answer_id",
                    "revision",
                    "status",
                    "hold_reasons",
                    "reservation_ref",
                    "authority_revision",
                    "native_operation_id",
                    "observed_at",
                    "source_revision",
                ][..],
            ),
            (
                &item.result,
                &[
                    "schema",
                    "task",
                    "admission_id",
                    "session_id",
                    "turn_id",
                    "worker_contract_revision",
                    "outcome",
                    "operational_status",
                    "exceptions",
                    "artifacts",
                    "evidence",
                    "clock",
                    "accounting",
                    "custody",
                    "observed_at",
                    "source_revision",
                ][..],
            ),
        ] {
            if let Some(value) = view {
                let obj = value.as_object().ok_or(ProjectionError)?;
                if obj.len() != fields.len() || fields.iter().any(|key| !obj.contains_key(*key)) {
                    return Err(ProjectionError);
                }
            }
        }
        if let (Some(admission), Some(result)) = (&item.admission, &item.result)
            && (admission
                .get("admission_id")
                .and_then(Value::as_str)
                .filter(|id| !id.is_empty())
                != result
                    .get("admission_id")
                    .and_then(Value::as_str)
                    .filter(|id| !id.is_empty())
                || admission
                    .get("admission_id")
                    .and_then(Value::as_str)
                    .filter(|id| !id.is_empty())
                    .is_none())
        {
            return Err(ProjectionError);
        }
    }
    serde_json::to_string(&Snapshot {
        schema: "dot.task-projection/v1-proposal",
        observed_at,
        source_revision,
        tasks,
    })
    .map_err(|_| ProjectionError)
}

/// Atomically publish a snapshot to an operator-configured absolute path.
/// This routine does not qualify the directory's integrity boundary; deployment
/// must protect it against workers and same-UID peers, including path races.
/// No MCP argument should ever select this path.
#[cfg(unix)]
pub fn publish(
    path: &std::path::Path,
    observed_at: &str,
    source_revision: &str,
    tasks: &[TaskProjection],
) -> Result<(), ProjectionError> {
    use std::{
        io::Write,
        os::unix::fs::OpenOptionsExt,
        sync::atomic::{AtomicU64, Ordering},
    };
    static NEXT: AtomicU64 = AtomicU64::new(0);
    if !path.is_absolute() {
        return Err(ProjectionError);
    }
    let parent = path.parent().ok_or(ProjectionError)?;
    if !std::fs::symlink_metadata(parent)
        .map_err(|_| ProjectionError)?
        .is_dir()
    {
        return Err(ProjectionError);
    }
    match std::fs::symlink_metadata(path) {
        Ok(meta) if !meta.is_file() || meta.file_type().is_symlink() => {
            return Err(ProjectionError);
        }
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => return Err(ProjectionError),
    }
    let body = snapshot(observed_at, source_revision, tasks)?;
    let tmp = parent.join(format!(
        ".task-projection-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let result = (|| -> std::io::Result<()> {
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&tmp)?;
        f.write_all(body.as_bytes())?;
        f.write_all(b"\n")?;
        f.sync_all()?;
        std::fs::rename(&tmp, path)?;
        std::fs::File::open(parent)?.sync_all()?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result.map_err(|_| ProjectionError)
}
