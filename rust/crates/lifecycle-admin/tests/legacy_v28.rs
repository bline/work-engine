use lifecycle_admin::legacy_v28::{translate_v28_capture, verify_completed_translation};
use rusqlite::{Connection, params};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};
use tempfile::TempDir;

const BINARY: &str = "f9b67b1a05b1ab3e975e9c0fe012a1909d29e7f66ca54e77c092c68e6a1eaea9";
const SOURCE: &str = "969e52ee4254bd9e8a1f6ad2290b77e08d30f59f880da747ba145b3835e7d5e5";
const NAMES: [&str; 4] = [
    "semantic-context.sqlite3",
    "role-bindings.json",
    "generation-state.json",
    "switchboard-attachment.json",
];
const SYNTHETIC_REVISION: &str =
    "sha256:0000000000000000000000000000000000000000000000000000000000000000";
const MAX_LEGACY_SAFE_INTEGER: i64 = 9_007_199_254_740_991;
// New external derivation receipt SHA-256 72c36c5d51dda36b58e31efd8d0d6f6e327cb264a2d8838219e9a804514e3547.
// These pins were calculated from exact UTF-8 bytes before they were embedded
// here, independently of revised() and the production Rust canonicalizer.
const Q4_UNICODE_INPUT_REVISION: &str =
    "sha256:3507d737c249c0dbaecf17b210713ea5a4b29352d203bb1b57bd2dfad12645e4";
const Q4_UNICODE_WIRE_SHA256: &str =
    "77e6ba7e5f557d5d71bc527093f6bb6ce7fb93d2fd47692c83ce97f7e3d8a5b5";
const Q4_UNICODE_ACCEPTANCE_REVISION: &str =
    "sha256:9b1f5ece5a9a1d9feb4a9d318b60dfd183431bd84e060a0c4beedeaf3e7ec9a5";
const Q4_UNICODE_DISPOSITION_REVISION: &str =
    "sha256:fefda658bd3c243cbe310ede2bafd92a99056cc04ec0e75fe1c56e39223ac75f";
const Q3Q4_UNICODE_LINEAGE_REVISION: &str =
    "sha256:ca405dd10e56bdc43d2c0f60d096266c849a9884e3edbcb30585f0ba96600735";

// Exact schema-12 table and column names from the archived migrations. The
// fixture keeps unrelated tables empty and uses synthetic, non-sensitive rows.
const TABLES: &[(&str, &str)] = &[
    (
        "legacy_context_input_custody_lineages",
        "custody_lineage_id,lineage_json,created_at",
    ),
    (
        "legacy_context_input_custody_owners",
        "custody_lineage_id,owner_revision,owner_generation_id,owner_state_root,owner_epoch,state,owner_json,updated_at",
    ),
    (
        "context_input_queue",
        "sequence,queue_id,logical_role_instance_id,transition_revision,client_user_message_id,input_revision,input_json,status,queued_at,released_at,delivery_json",
    ),
    (
        "legacy_context_input_dispositions",
        "queue_id,logical_role_instance_id,thread_id,binding_revision,transition_revision,client_user_message_id,input_revision,input_json_sha256,disposition,canonical_inventory_revision,authority_revision,successor_acceptance_revision,successor_logical_role_instance_id,successor_delivery_identity,record_revision,record_json,committed_at",
    ),
    (
        "legacy_context_input_successor_deliveries",
        "queue_id,claim_revision,claim_json,status,claimed_at,completed_at,delivery_revision,delivery_json",
    ),
    (
        "context_input_admissions",
        "logical_role_instance_id,thread_id,binding_revision,transition_revision,status,reconciliation_revision,closed_at,reopened_at",
    ),
    (
        "aborted_context_input_releases",
        "authorization_revision,logical_role_instance_id,thread_id,binding_revision,transition_revision,authorization_json,authorized_at",
    ),
    (
        "context_domain_input_custody",
        "transfer_id,source_queue_id,logical_role_instance_id,client_message_id,transition_id,effect_id,input_revision,source_sequence,destination_scheduler_identity,custody_state,receipt_revision,accepted_at,payload_json",
    ),
    (
        "context_transitions",
        "transition_id,logical_role_instance_id,binding_revision,provider_thread_id,predecessor_turn_id,semantic_state,committed_transition_revision,terminal,created_at,payload_json",
    ),
    (
        "context_transition_provider_observations",
        "provider_observation_id,provider_event_kind,provider_thread_id,provider_turn_id,provider_event_identity,raw_evidence_revision,observed_at,payload_json",
    ),
    (
        "context_transition_host_facts",
        "host_fact_id,host_fact_kind,logical_role_instance_id,host_thread_id,host_turn_id,stable_host_source_identity,raw_evidence_revision,observed_at,payload_json",
    ),
    (
        "context_transition_events",
        "transition_event_id,transition_id,transition_event_kind,classification_basis_revision,source_kind,provider_observation_id,host_fact_id,created_at,payload_json",
    ),
    (
        "context_transition_event_applications",
        "application_id,transition_event_id,transition_id,expected_transition_revision,observed_transition_revision,semantic_disposition,storage_result,committed_transition_revision,application_json",
    ),
    (
        "context_transition_admission_owners",
        "logical_role_instance_id,owner_kind,transition_id,recovery_operation_id,owner_revision,projection_json",
    ),
    (
        "context_transition_effects",
        "effect_id,transition_id,effect_kind,predecessor_effect_id,expected_transition_revision,input_revision,status,effect_revision,payload_json",
    ),
    (
        "context_transition_effect_applications",
        "application_id,effect_id,expected_effect_revision,observed_effect_revision,storage_result,committed_effect_revision,application_json",
    ),
    (
        "context_transition_effect_evidence",
        "evidence_id,effect_id,evidence_revision,observed_at,payload_json",
    ),
    (
        "context_transition_recovery_operations",
        "recovery_operation_id,logical_role_instance_id,failed_transition_id,failed_transition_revision,authority_revision,custody_disposition_revision,recovery_state,recovery_revision,payload_json",
    ),
    (
        "context_transition_queue_drain_receipts",
        "queue_drain_receipt_id,transition_id,effect_id,receipt_revision,payload_json",
    ),
    (
        "active_turn_lifecycle",
        "identity_key,turn_key,revision,status,next_eligible_at,state_json",
    ),
    (
        "active_turn_lifecycle_observations",
        "identity_key,turn_key,owner_identity_key,disposition,triggering_observation_sequence,observed_at",
    ),
];

fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn write_json(path: &Path, value: &Value) {
    let mut bytes = serde_json::to_vec_pretty(value).unwrap();
    bytes.push(b'\n');
    fs::write(path, bytes).unwrap();
}

fn revised(mut body: Value, key: &str) -> Value {
    let revision = format!(
        "sha256:{}",
        sha(serde_json::to_string(&body).unwrap().as_bytes())
    );
    body.as_object_mut()
        .unwrap()
        .insert(key.to_string(), json!(revision));
    body
}

fn rerevised(mut body: Value, key: &str) -> Value {
    body.as_object_mut().unwrap().remove(key);
    revised(body, key)
}

fn input(number: u8) -> Value {
    revised(
        json!({
            "schemaVersion": 1,
            "type": "work-engine.context-transition-input",
            "logicalRoleInstanceId": "slice-supervisor:legacy",
            "roleId": "slice-supervisor",
            "instanceId": "legacy",
            "threadId": format!("synthetic-thread-{number}"),
            "bindingRevision": 1,
            "clientUserMessageId": format!("synthetic-message-{number}"),
            "sourceKind": "human",
            "text": format!("synthetic non-sensitive obligation {number}"),
        }),
        "inputRevision",
    )
}

fn disposition(number: u8, input: &Value, input_bytes: &[u8]) -> Value {
    let target = json!({
        "queueId": format!("synthetic-q{number}"),
        "logicalRoleInstanceId": "slice-supervisor:legacy",
        "threadId": format!("synthetic-thread-{number}"),
        "bindingRevision": 1,
        "transitionRevision": SYNTHETIC_REVISION,
        "clientUserMessageId": format!("synthetic-message-{number}"),
        "inputRevision": input["inputRevision"],
        "inputJsonSha256": sha(input_bytes),
    });
    let authority = SYNTHETIC_REVISION;
    let acceptance = if number == 4 {
        revised(
            json!({
                "schemaVersion": 1,
                "type": "work-engine.context-input-successor-acceptance",
                "canonicalInventoryRevision": SYNTHETIC_REVISION,
                "target": target,
                "successorLogicalRoleInstanceId": "slice-supervisor:synthetic-successor",
                "successorSchedulerIdentity": "synthetic-scheduler",
                "successorDeliveryIdentity": "work-engine.context-input-successor-delivery:sha256:0000000000000000000000000000000000000000000000000000000000000000",
                "transferAuthorityRevision": authority,
                "acceptedAt": "2026-10-13T00:00:00Z",
            }),
            "acceptanceRevision",
        )
    } else {
        Value::Null
    };
    revised(
        json!({
            "schemaVersion": 1,
            "type": "work-engine.legacy-context-input-disposition",
            "canonicalInventoryRevision": SYNTHETIC_REVISION,
            "target": target,
            "disposition": if number == 3 { "terminally_retired" } else { "durable_restart" },
            "authorityRevision": authority,
            "evidenceRefs": ["synthetic-evidence"],
            "successorAcceptance": acceptance,
        }),
        "recordRevision",
    )
}

fn create_schema(conn: &Connection) {
    conn.execute_batch("CREATE TABLE schema_migrations(version INTEGER PRIMARY KEY, applied_at TEXT NOT NULL) STRICT;")
        .unwrap();
    for version in 1..=12 {
        conn.execute(
            "INSERT INTO schema_migrations(version,applied_at) VALUES (?1,'2026-10-13T00:00:00Z')",
            [version],
        )
        .unwrap();
    }
    // Material reached DDL from published v28 commit 2cfdf83, migrations
    // 2/11/12. Unrelated empty transition tables retain their pinned columns.
    conn.execute_batch(r#"
      CREATE TABLE context_input_queue (
        sequence INTEGER PRIMARY KEY AUTOINCREMENT,
        queue_id TEXT NOT NULL UNIQUE,
        logical_role_instance_id TEXT NOT NULL,
        transition_revision TEXT NOT NULL,
        client_user_message_id TEXT NOT NULL,
        input_revision TEXT NOT NULL,
        input_json TEXT NOT NULL,
        status TEXT NOT NULL CHECK(status IN ('queued', 'releasing', 'released')),
        queued_at TEXT NOT NULL,
        released_at TEXT,
        delivery_json TEXT,
        UNIQUE(logical_role_instance_id, client_user_message_id)
      ) STRICT;
      CREATE TABLE legacy_context_input_dispositions (
        queue_id TEXT PRIMARY KEY,
        logical_role_instance_id TEXT NOT NULL,
        thread_id TEXT NOT NULL,
        binding_revision INTEGER NOT NULL CHECK(binding_revision > 0),
        transition_revision TEXT NOT NULL,
        client_user_message_id TEXT NOT NULL,
        input_revision TEXT NOT NULL,
        input_json_sha256 TEXT NOT NULL,
        disposition TEXT NOT NULL CHECK(disposition IN ('terminally_retired','withdrawn','durable_restart')),
        canonical_inventory_revision TEXT NOT NULL,
        authority_revision TEXT NOT NULL,
        successor_acceptance_revision TEXT,
        successor_logical_role_instance_id TEXT,
        successor_delivery_identity TEXT,
        record_revision TEXT NOT NULL UNIQUE,
        record_json TEXT NOT NULL,
        committed_at TEXT NOT NULL,
        FOREIGN KEY(queue_id) REFERENCES context_input_queue(queue_id),
        CHECK((disposition = 'durable_restart' AND successor_acceptance_revision IS NOT NULL
          AND successor_logical_role_instance_id IS NOT NULL AND successor_delivery_identity IS NOT NULL)
          OR (disposition != 'durable_restart' AND successor_acceptance_revision IS NULL
          AND successor_logical_role_instance_id IS NULL AND successor_delivery_identity IS NULL))
      ) STRICT;
      CREATE TABLE legacy_context_input_successor_deliveries (
        queue_id TEXT PRIMARY KEY,
        claim_revision TEXT NOT NULL UNIQUE,
        claim_json TEXT NOT NULL,
        status TEXT NOT NULL CHECK(status IN ('pending','completed')),
        claimed_at TEXT NOT NULL,
        completed_at TEXT,
        delivery_revision TEXT,
        delivery_json TEXT,
        FOREIGN KEY(queue_id) REFERENCES legacy_context_input_dispositions(queue_id),
        CHECK((status = 'pending' AND completed_at IS NULL AND delivery_revision IS NULL AND delivery_json IS NULL)
          OR (status = 'completed' AND completed_at IS NOT NULL AND delivery_revision IS NOT NULL AND delivery_json IS NOT NULL))
      ) STRICT;
      CREATE TABLE legacy_context_input_custody_lineages (
        custody_lineage_id TEXT PRIMARY KEY,
        lineage_json TEXT NOT NULL,
        created_at TEXT NOT NULL
      ) STRICT;
      CREATE TABLE legacy_context_input_custody_owners (
        custody_lineage_id TEXT PRIMARY KEY,
        owner_revision TEXT NOT NULL UNIQUE,
        owner_generation_id TEXT NOT NULL,
        owner_state_root TEXT NOT NULL,
        owner_epoch INTEGER NOT NULL CHECK(owner_epoch > 0),
        state TEXT NOT NULL CHECK(state IN ('owned','adopted_inactive','transferred','recovery_required')),
        owner_json TEXT NOT NULL,
        updated_at TEXT NOT NULL,
        FOREIGN KEY(custody_lineage_id) REFERENCES legacy_context_input_custody_lineages(custody_lineage_id)
      ) STRICT;
      CREATE INDEX context_input_queue_release_order
        ON context_input_queue(logical_role_instance_id,transition_revision,status,sequence);
      CREATE INDEX legacy_context_input_dispositions_release_fence
        ON legacy_context_input_dispositions(logical_role_instance_id,transition_revision,queue_id);
      CREATE UNIQUE INDEX legacy_context_input_dispositions_successor_delivery
        ON legacy_context_input_dispositions(successor_delivery_identity)
        WHERE successor_delivery_identity IS NOT NULL;
    "#).unwrap();
    for (table, names) in TABLES {
        if matches!(
            *table,
            "context_input_queue"
                | "legacy_context_input_dispositions"
                | "legacy_context_input_successor_deliveries"
                | "legacy_context_input_custody_lineages"
                | "legacy_context_input_custody_owners"
        ) {
            continue;
        }
        let columns = names
            .split(',')
            .map(|name| {
                let kind = if matches!(
                    name,
                    "sequence"
                        | "binding_revision"
                        | "owner_epoch"
                        | "source_sequence"
                        | "terminal"
                        | "triggering_observation_sequence"
                ) {
                    "INTEGER"
                } else {
                    "TEXT"
                };
                format!("{name} {kind}")
            })
            .collect::<Vec<_>>()
            .join(",");
        conn.execute_batch(&format!("CREATE TABLE {table} ({columns}) STRICT;"))
            .unwrap();
    }
    conn.pragma_update(None, "user_version", 12).unwrap();
}

fn create_data(conn: &Connection, source_root: &Path) -> (Vec<u8>, Vec<u8>) {
    let mut raw_q3 = Vec::new();
    let mut raw_q4 = Vec::new();
    let mut revisions = Vec::new();
    let mut queue_ids = Vec::new();
    for number in [3_u8, 4] {
        let input = input(number);
        let input_bytes = serde_json::to_vec(&input).unwrap();
        let record = disposition(number, &input, &input_bytes);
        let record_bytes = serde_json::to_vec(&record).unwrap();
        conn.execute(
            "INSERT INTO context_input_queue VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
            params![
                i64::from(number),
                format!("synthetic-q{number}"),
                "slice-supervisor:legacy",
                SYNTHETIC_REVISION,
                format!("synthetic-message-{number}"),
                input["inputRevision"].as_str().unwrap(),
                String::from_utf8(input_bytes.clone()).unwrap(),
                "queued",
                "2026-10-13T00:00:00Z",
                Option::<String>::None,
                Option::<String>::None
            ],
        )
        .unwrap();
        let acceptance = &record["successorAcceptance"];
        conn.execute(
            "INSERT INTO legacy_context_input_dispositions VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17)",
            params![format!("synthetic-q{number}"), "slice-supervisor:legacy",
                format!("synthetic-thread-{number}"), 1_i64, SYNTHETIC_REVISION,
                format!("synthetic-message-{number}"), input["inputRevision"].as_str().unwrap(),
                sha(&input_bytes), if number == 3 { "terminally_retired" } else { "durable_restart" },
                SYNTHETIC_REVISION, SYNTHETIC_REVISION,
                acceptance["acceptanceRevision"].as_str(), acceptance["successorLogicalRoleInstanceId"].as_str(),
                acceptance["successorDeliveryIdentity"].as_str(), record["recordRevision"].as_str().unwrap(),
                String::from_utf8(record_bytes.clone()).unwrap(), "2026-10-13T00:00:00Z"],
        ).unwrap();
        queue_ids.push(format!("synthetic-q{number}"));
        revisions.push(record["recordRevision"].as_str().unwrap().to_string());
        if number == 3 {
            raw_q3 = input_bytes;
        } else {
            raw_q4 = input_bytes;
        }
    }
    revisions.sort();
    let lineage = revised(
        json!({
            "schemaVersion":1,"type":"work-engine.legacy-context-input-custody-lineage",
            "canonicalInventoryRevision":SYNTHETIC_REVISION,"queueIds":queue_ids,
            "dispositionRecordRevisions":revisions,"sourceGenerationId":"synthetic-generation"
        }),
        "custodyLineageId",
    );
    let lineage_id = lineage["custodyLineageId"].as_str().unwrap();
    conn.execute(
        "INSERT INTO legacy_context_input_custody_lineages VALUES (?1,?2,?3)",
        params![
            lineage_id,
            serde_json::to_string(&lineage).unwrap(),
            "2026-10-13T00:00:00Z"
        ],
    )
    .unwrap();
    let owner = revised(
        json!({
            "schemaVersion":1,"type":"work-engine.legacy-context-input-custody-owner",
            "custodyLineageId":lineage_id,"ownerGenerationId":"synthetic-generation",
            "ownerStateRoot":source_root.display().to_string(),"ownerEpoch":1,"state":"owned",
            "handoffRevision":null,"capsuleRevision":null,"predecessorOwnerRevision":null,
            "successorOwner":null,"manifestOwnershipRevision":null
        }),
        "ownerRevision",
    );
    conn.execute(
        "INSERT INTO legacy_context_input_custody_owners VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
        params![
            lineage_id,
            owner["ownerRevision"].as_str().unwrap(),
            "synthetic-generation",
            source_root.display().to_string(),
            1_i64,
            "owned",
            serde_json::to_string(&owner).unwrap(),
            "2026-10-13T00:00:00Z"
        ],
    )
    .unwrap();
    (raw_q3, raw_q4)
}

pub(crate) struct Fixture {
    _temp: TempDir,
    pub(crate) source: PathBuf,
    pub(crate) attempt: PathBuf,
    pub(crate) output: PathBuf,
    q3_bytes: Vec<u8>,
    q4_bytes: Vec<u8>,
}

pub(crate) fn fixture() -> Fixture {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("synthetic-source");
    let attempt = temp.path().join("attempt");
    let image = attempt.join("capture/image");
    fs::create_dir(&source).unwrap();
    fs::create_dir_all(&image).unwrap();
    let db = source.join(NAMES[0]);
    let conn = Connection::open(&db).unwrap();
    create_schema(&conn);
    let (q3_bytes, q4_bytes) = create_data(&conn, &source);
    drop(conn);
    write_json(
        &source.join(NAMES[1]),
        &json!({"schemaVersion":1,"revision":1,"bindings":{},"deliveries":{}}),
    );
    write_json(
        &source.join(NAMES[2]),
        &json!({"schemaVersion":1,"generation":"synthetic"}),
    );
    write_json(
        &source.join(NAMES[3]),
        &json!({"schemaVersion":1,"attachment":"synthetic"}),
    );
    for name in NAMES {
        fs::copy(source.join(name), image.join(name)).unwrap();
    }
    let mut artifacts = BTreeMap::new();
    let files = NAMES
        .iter()
        .map(|name| {
            let digest = sha(&fs::read(image.join(name)).unwrap());
            artifacts.insert(format!("capture/image/{name}"), json!({"sha256":digest}));
            json!({"source":source.join(name),"copy":image.join(name),"sha256":digest,
            "copy_sha256":digest,"identity":{}})
        })
        .collect::<Vec<_>>();
    let manifest = json!({
        "schema_version":1,"source_root":source,"files":files,
        "selection_claim":"current_selection_unestablished","selector_present":false,
        "coherence":"conditional_no_relevant_inotify_event_and_stable_bytes_under_normal_writer_model"
    });
    write_json(&attempt.join("capture/source-manifest.json"), &manifest);
    let manifest_sha = sha(&fs::read(attempt.join("capture/source-manifest.json")).unwrap());
    let source_hashes = json!({"Cargo.toml":"synthetic-source-hash"});
    let child = json!({
        "schema_version":1,"status":"qualified_candidate_root","primary_error":null,"cleanup_error":null,
        "manifest_sha256":manifest_sha,"source_root":source,"source_hashes":source_hashes,
        "executable_sha256":BINARY,"selection_claim":"current_selection_unestablished","event_ledger":[]
    });
    write_json(&attempt.join("capture/receipt.json"), &child);
    let child_sha = sha(&fs::read(attempt.join("capture/receipt.json")).unwrap());
    let absent = json!({"semantic-context.sqlite3-wal":{"present":false},"semantic-context.sqlite3-shm":{"present":false}});
    let outer = json!({
        "schema_version":1,"status":"qualified_candidate_root","raw_exit_status":0,
        "primary_error":null,"cleanup_error":null,"child_primary_error":null,"child_cleanup_error":null,
        "child_status":"qualified_candidate_root","child_event_ledger":[],
        "preflight_source_metadata":absent,"postflight_source_metadata":absent,
        "binary":{"sha256":BINARY},"source_aggregate_sha256":SOURCE,
        "child_receipt":{"sha256":child_sha},"child_manifest_sha256":manifest_sha,
        "source_hashes_before":source_hashes,"source_hashes_after":source_hashes,
        "selection_claim":"current_selection_unestablished","duration_seconds":0.381,
        "argv":["synthetic-capture","--source-root",source,"--output-root",attempt.join("capture")],
        "artifacts":artifacts
    });
    write_json(&attempt.join("attempt-receipt.json"), &outer);
    let output = temp.path().join("translation");
    Fixture {
        _temp: temp,
        source,
        attempt,
        output,
        q3_bytes,
        q4_bytes,
    }
}

fn rebind_image(attempt: &Path, name: &str) {
    let image = attempt.join("capture/image").join(name);
    let digest = sha(&fs::read(&image).unwrap());
    let manifest_path = attempt.join("capture/source-manifest.json");
    let child_path = attempt.join("capture/receipt.json");
    let outer_path = attempt.join("attempt-receipt.json");
    let mut manifest: Value = serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
    let item = manifest["files"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|entry| entry["copy"].as_str().unwrap().ends_with(name))
        .unwrap();
    item["sha256"] = json!(digest);
    item["copy_sha256"] = json!(digest);
    write_json(&manifest_path, &manifest);
    let manifest_digest = sha(&fs::read(&manifest_path).unwrap());
    let mut child: Value = serde_json::from_slice(&fs::read(&child_path).unwrap()).unwrap();
    child["manifest_sha256"] = json!(manifest_digest);
    write_json(&child_path, &child);
    let mut outer: Value = serde_json::from_slice(&fs::read(&outer_path).unwrap()).unwrap();
    outer["artifacts"][format!("capture/image/{name}")]["sha256"] = json!(digest);
    outer["child_manifest_sha256"] = json!(manifest_digest);
    outer["child_receipt"]["sha256"] = json!(sha(&fs::read(&child_path).unwrap()));
    write_json(&outer_path, &outer);
}

// A clearly synthetic semantic fixture: this does not claim native capture
// provenance. Keep the stored record, lineage and owner revisions coherent so
// each negative reaches the named v28 semantic validator.
fn rewrite_record(f: &Fixture, queue_id: &str, mutate: impl FnOnce(&mut Value, &Connection)) {
    let db = f.attempt.join("capture/image/semantic-context.sqlite3");
    let conn = Connection::open(&db).unwrap();
    conn.execute_batch("PRAGMA foreign_keys=OFF").unwrap();
    let raw: String = conn
        .query_row(
            "SELECT record_json FROM legacy_context_input_dispositions WHERE queue_id=?1",
            [queue_id],
            |row| row.get(0),
        )
        .unwrap();
    let mut record: Value = serde_json::from_str(&raw).unwrap();
    mutate(&mut record, &conn);
    if !record["successorAcceptance"].is_null() {
        record["successorAcceptance"]["target"] = record["target"].clone();
        record["successorAcceptance"] =
            rerevised(record["successorAcceptance"].clone(), "acceptanceRevision");
    }
    record = rerevised(record, "recordRevision");
    let rebound_queue_id = record["target"]["queueId"].as_str().unwrap();
    if rebound_queue_id != queue_id {
        conn.execute(
            "UPDATE context_input_queue SET queue_id=?1 WHERE queue_id=?2",
            params![rebound_queue_id, queue_id],
        )
        .unwrap();
    }
    conn.execute(
        "UPDATE legacy_context_input_dispositions SET queue_id=?1,successor_acceptance_revision=?2,record_revision=?3,record_json=?4 WHERE queue_id=?5",
        params![rebound_queue_id, record["successorAcceptance"]["acceptanceRevision"].as_str(),
            record["recordRevision"].as_str().unwrap(), serde_json::to_string(&record).unwrap(),queue_id],
    ).unwrap();

    let lineage_raw: String = conn
        .query_row(
            "SELECT lineage_json FROM legacy_context_input_custody_lineages",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let mut lineage: Value = serde_json::from_str(&lineage_raw).unwrap();
    let mut queue_statement = conn
        .prepare("SELECT queue_id FROM legacy_context_input_dispositions ORDER BY queue_id")
        .unwrap();
    let queue_ids = queue_statement
        .query_map([], |row| row.get::<_, String>(0))
        .unwrap()
        .map(Result::unwrap)
        .collect::<Vec<_>>();
    drop(queue_statement);
    lineage["queueIds"] = json!(queue_ids);
    let mut statement = conn
        .prepare("SELECT record_revision FROM legacy_context_input_dispositions ORDER BY queue_id")
        .unwrap();
    let mut revisions = statement
        .query_map([], |row| row.get::<_, String>(0))
        .unwrap()
        .map(Result::unwrap)
        .collect::<Vec<_>>();
    drop(statement);
    revisions.sort();
    lineage["dispositionRecordRevisions"] = json!(revisions);
    lineage = rerevised(lineage, "custodyLineageId");
    let lineage_id = lineage["custodyLineageId"].as_str().unwrap();
    conn.execute(
        "UPDATE legacy_context_input_custody_lineages SET custody_lineage_id=?1,lineage_json=?2",
        params![lineage_id, serde_json::to_string(&lineage).unwrap()],
    )
    .unwrap();
    let owner_raw: String = conn
        .query_row(
            "SELECT owner_json FROM legacy_context_input_custody_owners",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let mut owner: Value = serde_json::from_str(&owner_raw).unwrap();
    owner["custodyLineageId"] = json!(lineage_id);
    owner = rerevised(owner, "ownerRevision");
    conn.execute(
        "UPDATE legacy_context_input_custody_owners SET custody_lineage_id=?1,owner_revision=?2,owner_json=?3",
        params![lineage_id, owner["ownerRevision"].as_str().unwrap(),
            serde_json::to_string(&owner).unwrap()],
    ).unwrap();
    drop(conn);
    rebind_image(&f.attempt, NAMES[0]);
}

fn rewrite_q4_record(f: &Fixture, mutate: impl FnOnce(&mut Value, &Connection)) {
    rewrite_record(f, "synthetic-q4", mutate);
}

fn rewrite_q4_unicode_maxsafe(f: &Fixture) -> Vec<u8> {
    let mut input = input(4);
    input["text"] = json!("é 雪");
    input["bindingRevision"] = json!(MAX_LEGACY_SAFE_INTEGER);
    input = rerevised(input, "inputRevision");
    assert_eq!(input["inputRevision"], Q4_UNICODE_INPUT_REVISION);
    // The raw queue JSON order is intentionally noncanonical. The pinned
    // revision is over the sorted body; inputJsonSha256 is over these bytes.
    let raw = r#"{"type":"work-engine.context-transition-input","text":"é 雪","sourceKind":"human","clientUserMessageId":"synthetic-message-4","bindingRevision":9007199254740991,"threadId":"synthetic-thread-4","instanceId":"legacy","roleId":"slice-supervisor","logicalRoleInstanceId":"slice-supervisor:legacy","schemaVersion":1,"inputRevision":"sha256:3507d737c249c0dbaecf17b210713ea5a4b29352d203bb1b57bd2dfad12645e4"}"#.as_bytes().to_vec();
    assert_eq!(serde_json::from_slice::<Value>(&raw).unwrap(), input);
    assert_eq!(sha(&raw), Q4_UNICODE_WIRE_SHA256);
    rewrite_q4_record(f, |record, conn| {
        record["target"]["bindingRevision"] = json!(MAX_LEGACY_SAFE_INTEGER);
        record["target"]["inputRevision"] = input["inputRevision"].clone();
        record["target"]["inputJsonSha256"] = json!(Q4_UNICODE_WIRE_SHA256);
        conn.execute(
            "UPDATE context_input_queue SET input_revision=?1,input_json=?2 WHERE queue_id='synthetic-q4'",
            params![Q4_UNICODE_INPUT_REVISION, String::from_utf8(raw.clone()).unwrap()],
        )
        .unwrap();
        conn.execute(
            "UPDATE legacy_context_input_dispositions SET binding_revision=?1,input_revision=?2,input_json_sha256=?3 WHERE queue_id='synthetic-q4'",
            params![MAX_LEGACY_SAFE_INTEGER, Q4_UNICODE_INPUT_REVISION, Q4_UNICODE_WIRE_SHA256],
        )
        .unwrap();
    });
    let conn = Connection::open(f.attempt.join("capture/image/semantic-context.sqlite3")).unwrap();
    let (acceptance, disposition): (String, String) = conn
        .query_row(
            "SELECT successor_acceptance_revision,record_revision FROM legacy_context_input_dispositions WHERE queue_id='synthetic-q4'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    let lineage: String = conn
        .query_row(
            "SELECT custody_lineage_id FROM legacy_context_input_custody_lineages",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(acceptance, Q4_UNICODE_ACCEPTANCE_REVISION);
    assert_eq!(disposition, Q4_UNICODE_DISPOSITION_REVISION);
    assert_eq!(lineage, Q3Q4_UNICODE_LINEAGE_REVISION);
    raw
}

fn replace_lineage_json_column(attempt: &Path, declaration: &str) {
    let db = attempt.join("capture/image/semantic-context.sqlite3");
    let conn = Connection::open(&db).unwrap();
    conn.execute_batch(&format!(
        "PRAGMA foreign_keys=OFF;
         BEGIN;
         CREATE TABLE synthetic_lineage_shape (
           custody_lineage_id TEXT PRIMARY KEY,
           lineage_json {declaration},
           created_at TEXT NOT NULL
         ) STRICT;
         INSERT INTO synthetic_lineage_shape
           SELECT custody_lineage_id,lineage_json,created_at
           FROM legacy_context_input_custody_lineages;
         DROP TABLE legacy_context_input_custody_lineages;
         ALTER TABLE synthetic_lineage_shape
           RENAME TO legacy_context_input_custody_lineages;
         COMMIT;
         PRAGMA foreign_keys=ON;"
    ))
    .unwrap();
    drop(conn);
    rebind_image(attempt, NAMES[0]);
}

// Independently spelled canonical bytes under the published-v28
// context-input-custody.mjs revision contract at Git 2cfdf83. SHA-256 was
// calculated over these UTF-8 bytes by sha256sum, not by the Rust translator.
#[test]
fn pinned_v28_canonical_input_vector() {
    assert_eq!(
        input(3)["inputRevision"],
        "sha256:5b32ec912aba30628cd10a6911a141a9bc4bdfbd4444b0be0bafa431b7e5ed50"
    );
}

// Expected values are static SHA-256 over the independently spelled canonical
// UTF-8 bodies from pinned v28 constructors, not translator output. The fixture
// and the translator must agree with these bytes/revisions before qualification.
#[test]
fn pinned_v28_disposition_acceptance_lineage_and_owner_vectors() {
    let f = fixture();
    let conn = Connection::open(f.attempt.join("capture/image/semantic-context.sqlite3")).unwrap();
    let (q3, q4): (String, String) = (
        conn.query_row("SELECT record_revision FROM legacy_context_input_dispositions WHERE queue_id='synthetic-q3'", [], |r| r.get(0)).unwrap(),
        conn.query_row("SELECT record_revision FROM legacy_context_input_dispositions WHERE queue_id='synthetic-q4'", [], |r| r.get(0)).unwrap(),
    );
    assert_eq!(
        q3,
        "sha256:599677f55da766ea3cb1ae7786af04656f32b45b6a9f98e2a0cf476769034056"
    );
    assert_eq!(
        q4,
        "sha256:c65b902cd5b0baf88772bf34838ce04ebff7ba2038ee3df709a956cce7528c80"
    );
    let acceptance: String = conn.query_row(
        "SELECT successor_acceptance_revision FROM legacy_context_input_dispositions WHERE queue_id='synthetic-q4'",
        [], |r| r.get(0),
    ).unwrap();
    assert_eq!(
        acceptance,
        "sha256:1bfe3ef50f6601ec9f35144fb71104a9f5bba00cab043e0ff221c29cfdc0846d"
    );
    let lineage: String = conn
        .query_row(
            "SELECT custody_lineage_id FROM legacy_context_input_custody_lineages",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        lineage,
        "sha256:93452679aca6b99f459a1a1368b1289ba15f8d747bbfc894b4d72f29233f826d"
    );
    let static_owner = revised(
        json!({
            "schemaVersion":1,"type":"work-engine.legacy-context-input-custody-owner",
            "custodyLineageId":lineage,"ownerGenerationId":"synthetic-generation",
            "ownerStateRoot":"/synthetic/source","ownerEpoch":1,"state":"owned",
            "handoffRevision":null,"capsuleRevision":null,"predecessorOwnerRevision":null,
            "successorOwner":null,"manifestOwnershipRevision":null
        }),
        "ownerRevision",
    );
    assert_eq!(
        static_owner["ownerRevision"],
        "sha256:d5db6bded4e16b02589dc4cf27815477d66122b33dbf4c42b9fd86d9eb2f53ed"
    );
}

#[test]
fn pinned_v28_unicode_order_and_unknown_field_vectors() {
    let positive = fixture();
    let reordered_wire = rewrite_q4_unicode_maxsafe(&positive);
    let result = translate_v28_capture(&positive.attempt, &positive.output).unwrap();
    assert_eq!(result.translation_status, "qualified");
    assert_eq!(result.transfer_eligibility, "requires_separate_B_authority");
    assert_eq!(
        fs::read(positive.output.join("raw/q4-input.json")).unwrap(),
        reordered_wire
    );
    let capsule: Value =
        serde_json::from_slice(&fs::read(positive.output.join("capsule.json")).unwrap()).unwrap();
    assert_eq!(capsule["q3"]["physical_delivery"], "not_applicable");
    assert_eq!(capsule["q4"]["physical_delivery"], "not_proved");
    assert_eq!(capsule["no_execution"], true);
    assert_eq!(capsule["no_rust_custody"], true);
    verify_completed_translation(&positive.attempt, &positive.output, &result).unwrap();
    let static_owner = revised(
        json!({
            "schemaVersion":1,"type":"work-engine.legacy-context-input-custody-owner",
            "custodyLineageId":Q3Q4_UNICODE_LINEAGE_REVISION,
            "ownerGenerationId":"synthetic-generation","ownerStateRoot":"/synthetic/source",
            "ownerEpoch":MAX_LEGACY_SAFE_INTEGER,"state":"owned",
            "handoffRevision":null,"capsuleRevision":null,"predecessorOwnerRevision":null,
            "successorOwner":null,"manifestOwnershipRevision":null
        }),
        "ownerRevision",
    );
    assert_eq!(
        static_owner["ownerRevision"],
        "sha256:a070fe64feb371c677c50547c18c460c0966859cd510fda70291b15c7d857d58"
    );

    let mut body = json!({
        "type":"work-engine.context-transition-input","text":"é 雪",
        "sourceKind":"human","clientUserMessageId":"synthetic-message-4",
        "bindingRevision":1,"threadId":"synthetic-thread-4","instanceId":"legacy",
        "roleId":"slice-supervisor","logicalRoleInstanceId":"slice-supervisor:legacy",
        "schemaVersion":1
    });
    let expected = "sha256:0c59ff0008d028b09c5c2821eca2efcf368ff7592da18ad615a5200d9a84b2bc";
    assert_eq!(
        revised(body.clone(), "inputRevision")["inputRevision"],
        expected
    );
    let reordered = body
        .as_object()
        .unwrap()
        .iter()
        .rev()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    body = Value::Object(reordered);
    assert_eq!(
        revised(body.clone(), "inputRevision")["inputRevision"],
        expected
    );
    body["unknownField"] = json!(true);
    let f = fixture();
    rewrite_q4_record(&f, |record, conn| {
        let mut input = body;
        input = revised(input, "inputRevision");
        let bytes = serde_json::to_vec(&input).unwrap();
        record["target"]["inputRevision"] = input["inputRevision"].clone();
        record["target"]["inputJsonSha256"] = json!(sha(&bytes));
        conn.execute("UPDATE context_input_queue SET input_revision=?1,input_json=?2 WHERE queue_id='synthetic-q4'",
            params![input["inputRevision"].as_str().unwrap(),String::from_utf8(bytes).unwrap()]).unwrap();
        conn.execute("UPDATE legacy_context_input_dispositions SET input_revision=?1,input_json_sha256=?2 WHERE queue_id='synthetic-q4'",
            params![input["inputRevision"].as_str().unwrap(),record["target"]["inputJsonSha256"].as_str().unwrap()]).unwrap();
    });
    let error = translate_v28_capture(&f.attempt, &f.output)
        .unwrap_err()
        .to_string();
    assert!(error.contains("context input codec"), "{error}");
}

#[test]
fn valid_hash_invalid_legacy_timestamp_is_refused() {
    let f = fixture();
    rewrite_q4_record(&f, |record, _| {
        record["successorAcceptance"]["acceptedAt"] = json!("not-a-timestamp");
    });
    let error = translate_v28_capture(&f.attempt, &f.output)
        .unwrap_err()
        .to_string();
    assert!(error.contains("acceptedAt unsupported"), "{error}");
}

#[test]
fn valid_hash_unsupported_but_parseable_offset_timestamp_is_refused() {
    let f = fixture();
    rewrite_q4_record(&f, |record, _| {
        record["successorAcceptance"]["acceptedAt"] = json!("2026-10-13T00:00:00+00:00");
    });
    let error = translate_v28_capture(&f.attempt, &f.output)
        .unwrap_err()
        .to_string();
    assert!(error.contains("acceptedAt unsupported"), "{error}");
}

#[test]
fn accepted_utc_millisecond_timestamp_preserves_revision_identity() {
    let f = fixture();
    rewrite_q4_record(&f, |record, _| {
        record["successorAcceptance"]["acceptedAt"] = json!("2024-02-29T23:59:59.123Z");
    });
    let result = translate_v28_capture(&f.attempt, &f.output).unwrap();
    verify_completed_translation(&f.attempt, &f.output, &result).unwrap();
}

#[test]
fn valid_hash_invalid_gregorian_day_is_refused() {
    let f = fixture();
    rewrite_q4_record(&f, |record, _| {
        record["successorAcceptance"]["acceptedAt"] = json!("2025-02-29T23:59:59.123Z");
    });
    let error = translate_v28_capture(&f.attempt, &f.output)
        .unwrap_err()
        .to_string();
    assert!(error.contains("acceptedAt unsupported"), "{error}");
}

#[test]
fn valid_hash_invalid_transition_revision_shape_is_refused() {
    let f = fixture();
    rewrite_q4_record(&f, |record, conn| {
        record["target"]["transitionRevision"] = json!("not-a-revision");
        conn.execute(
            "UPDATE context_input_queue SET transition_revision='not-a-revision' WHERE queue_id='synthetic-q4'",
            [],
        ).unwrap();
        conn.execute(
            "UPDATE legacy_context_input_dispositions SET transition_revision='not-a-revision' WHERE queue_id='synthetic-q4'",
            [],
        ).unwrap();
    });
    let error = translate_v28_capture(&f.attempt, &f.output)
        .unwrap_err()
        .to_string();
    assert!(error.contains("transition revision shape"), "{error}");
}

#[test]
fn target_only_guard_order_identity_cases_are_not_coherent_producer_evidence() {
    // These adversarial target mutations verify guard order only. Linked
    // input/SQL cases and the queue-ID discriminator are covered separately.
    for (field, invalid) in [
        ("queueId", ""),
        ("logicalRoleInstanceId", "  "),
        ("threadId", "\u{FEFF}"),
        ("clientUserMessageId", ""),
    ] {
        let f = fixture();
        rewrite_q4_record(&f, |record, _| {
            record["target"][field] = json!(invalid);
        });
        let error = translate_v28_capture(&f.attempt, &f.output)
            .unwrap_err()
            .to_string();
        assert!(
            error.contains(&format!("empty legacy {field}")),
            "{field}: {error}"
        );
    }
}

#[test]
fn evidence_refs_reject_javascript_only_whitespace() {
    for invalid in ["\u{FEFF}", "\u{00A0}", "\u{2028}"] {
        let f = fixture();
        rewrite_q4_record(&f, |record, _| {
            record["evidenceRefs"] = json!([invalid]);
        });
        let error = translate_v28_capture(&f.attempt, &f.output)
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("evidence refs not canonical/nonempty"),
            "{error}"
        );
        assert!(!f.output.exists());
    }
}

#[test]
fn coherent_q4_empty_queue_id_is_refused_without_replay() {
    let f = fixture();
    // queueId is not derived from the transition input. rewrite_q4_record
    // rebinds the target, stored SQL key, record, lineage, owner and image.
    rewrite_q4_record(&f, |record, _| {
        record["target"]["queueId"] = json!("");
    });
    let error = translate_v28_capture(&f.attempt, &f.output)
        .unwrap_err()
        .to_string();
    assert!(error.contains("empty legacy queueId"), "{error}");
    assert!(!f.output.exists());
}

#[test]
fn retired_q3_target_empty_identity_is_refused_without_replay() {
    let f = fixture();
    rewrite_record(&f, "synthetic-q3", |record, _| {
        record["target"]["queueId"] = json!("");
    });
    let error = translate_v28_capture(&f.attempt, &f.output)
        .unwrap_err()
        .to_string();
    assert!(error.contains("empty legacy queueId"), "{error}");
    assert!(!f.output.exists());
}

#[test]
fn valid_hash_empty_input_identity_is_refused() {
    for (field, invalid) in [
        ("roleId", ""),
        ("instanceId", "  "),
        ("threadId", "\u{FEFF}"),
        ("clientUserMessageId", ""),
        ("text", "\u{FEFF}"),
    ] {
        let f = fixture();
        rewrite_q4_record(&f, |record, conn| {
            let raw: String = conn
                .query_row(
                    "SELECT input_json FROM context_input_queue WHERE queue_id='synthetic-q4'",
                    [],
                    |row| row.get(0),
                )
                .unwrap();
            let mut input: Value = serde_json::from_str(&raw).unwrap();
            input[field] = json!(invalid);
            if field == "roleId" || field == "instanceId" {
                input["logicalRoleInstanceId"] = json!(format!(
                    "{}:{}",
                    input["roleId"].as_str().unwrap(),
                    input["instanceId"].as_str().unwrap()
                ));
            }
            input = rerevised(input, "inputRevision");
            let bytes = serde_json::to_vec(&input).unwrap();
            if field == "threadId" || field == "clientUserMessageId" {
                record["target"][field] = json!(invalid);
            }
            if field == "roleId" || field == "instanceId" {
                record["target"]["logicalRoleInstanceId"] = input["logicalRoleInstanceId"].clone();
            }
            record["target"]["inputRevision"] = input["inputRevision"].clone();
            record["target"]["inputJsonSha256"] = json!(sha(&bytes));
            conn.execute(
                "UPDATE context_input_queue SET logical_role_instance_id=?1,client_user_message_id=?2,input_revision=?3,input_json=?4 WHERE queue_id='synthetic-q4'",
                params![input["logicalRoleInstanceId"].as_str().unwrap(),
                    input["clientUserMessageId"].as_str().unwrap(),
                    input["inputRevision"].as_str().unwrap(),String::from_utf8(bytes).unwrap()],
            ).unwrap();
            conn.execute(
                "UPDATE legacy_context_input_dispositions SET logical_role_instance_id=?1,thread_id=?2,client_user_message_id=?3,input_revision=?4,input_json_sha256=?5 WHERE queue_id='synthetic-q4'",
                params![input["logicalRoleInstanceId"].as_str().unwrap(),
                    input["threadId"].as_str().unwrap(),input["clientUserMessageId"].as_str().unwrap(),
                    input["inputRevision"].as_str().unwrap(),record["target"]["inputJsonSha256"].as_str().unwrap()],
            ).unwrap();
        });
        let error = translate_v28_capture(&f.attempt, &f.output)
            .unwrap_err()
            .to_string();
        assert!(
            error.contains(&format!("empty legacy {field}")),
            "{field}: {error}"
        );
    }
}

#[test]
fn pinned_schema12_material_shapes_match_strict_sqlite() {
    // Migration 1/2/11/12 DDL at published v28 commit 2cfdf83. This probes
    // the same bundled SQLite realization as the translator, not a host CLI.
    type ColumnShape = (&'static str, &'static str, i64, i64);
    type TableShape = (&'static str, &'static [ColumnShape]);
    const EXPECTED: &[TableShape] = &[
        (
            "schema_migrations",
            &[("version", "INTEGER", 0, 1), ("applied_at", "TEXT", 1, 0)],
        ),
        (
            "legacy_context_input_custody_lineages",
            &[
                ("custody_lineage_id", "TEXT", 1, 1),
                ("lineage_json", "TEXT", 1, 0),
                ("created_at", "TEXT", 1, 0),
            ],
        ),
        (
            "legacy_context_input_custody_owners",
            &[
                ("custody_lineage_id", "TEXT", 1, 1),
                ("owner_revision", "TEXT", 1, 0),
                ("owner_generation_id", "TEXT", 1, 0),
                ("owner_state_root", "TEXT", 1, 0),
                ("owner_epoch", "INTEGER", 1, 0),
                ("state", "TEXT", 1, 0),
                ("owner_json", "TEXT", 1, 0),
                ("updated_at", "TEXT", 1, 0),
            ],
        ),
        (
            "context_input_queue",
            &[
                ("sequence", "INTEGER", 0, 1),
                ("queue_id", "TEXT", 1, 0),
                ("logical_role_instance_id", "TEXT", 1, 0),
                ("transition_revision", "TEXT", 1, 0),
                ("client_user_message_id", "TEXT", 1, 0),
                ("input_revision", "TEXT", 1, 0),
                ("input_json", "TEXT", 1, 0),
                ("status", "TEXT", 1, 0),
                ("queued_at", "TEXT", 1, 0),
                ("released_at", "TEXT", 0, 0),
                ("delivery_json", "TEXT", 0, 0),
            ],
        ),
        (
            "legacy_context_input_dispositions",
            &[
                ("queue_id", "TEXT", 1, 1),
                ("logical_role_instance_id", "TEXT", 1, 0),
                ("thread_id", "TEXT", 1, 0),
                ("binding_revision", "INTEGER", 1, 0),
                ("transition_revision", "TEXT", 1, 0),
                ("client_user_message_id", "TEXT", 1, 0),
                ("input_revision", "TEXT", 1, 0),
                ("input_json_sha256", "TEXT", 1, 0),
                ("disposition", "TEXT", 1, 0),
                ("canonical_inventory_revision", "TEXT", 1, 0),
                ("authority_revision", "TEXT", 1, 0),
                ("successor_acceptance_revision", "TEXT", 0, 0),
                ("successor_logical_role_instance_id", "TEXT", 0, 0),
                ("successor_delivery_identity", "TEXT", 0, 0),
                ("record_revision", "TEXT", 1, 0),
                ("record_json", "TEXT", 1, 0),
                ("committed_at", "TEXT", 1, 0),
            ],
        ),
        (
            "legacy_context_input_successor_deliveries",
            &[
                ("queue_id", "TEXT", 1, 1),
                ("claim_revision", "TEXT", 1, 0),
                ("claim_json", "TEXT", 1, 0),
                ("status", "TEXT", 1, 0),
                ("claimed_at", "TEXT", 1, 0),
                ("completed_at", "TEXT", 0, 0),
                ("delivery_revision", "TEXT", 0, 0),
                ("delivery_json", "TEXT", 0, 0),
            ],
        ),
    ];
    let conn = Connection::open_in_memory().unwrap();
    create_schema(&conn);
    for (table, expected) in EXPECTED {
        let mut statement = conn
            .prepare(&format!("PRAGMA table_info({table})"))
            .unwrap();
        let actual: Vec<(String, String, i64, i64)> = statement
            .query_map([], |row| {
                Ok((row.get(1)?, row.get(2)?, row.get(3)?, row.get(5)?))
            })
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        let expected: Vec<(String, String, i64, i64)> = expected
            .iter()
            .map(|(name, kind, notnull, pk)| (name.to_string(), kind.to_string(), *notnull, *pk))
            .collect();
        assert_eq!(actual, expected, "schema-12 material shape: {table}");
    }
}

#[test]
fn custody_envelopes_are_required_and_lineage_bound() {
    let f = fixture();
    let db = f.attempt.join("capture/image/semantic-context.sqlite3");
    let conn = Connection::open(&db).unwrap();
    conn.execute(
        "UPDATE legacy_context_input_custody_owners SET owner_json='{}'",
        [],
    )
    .unwrap();
    drop(conn);
    rebind_image(&f.attempt, NAMES[0]);
    let error = translate_v28_capture(&f.attempt, &f.output)
        .unwrap_err()
        .to_string();
    assert!(error.contains("custody owner"), "{error}");
}

#[test]
fn canonical_lineage_with_wrong_membership_is_rejected() {
    let f = fixture();
    let db = f.attempt.join("capture/image/semantic-context.sqlite3");
    let conn = Connection::open(&db).unwrap();
    let raw: String = conn
        .query_row(
            "SELECT lineage_json FROM legacy_context_input_custody_lineages",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let mut lineage: Value = serde_json::from_str(&raw).unwrap();
    lineage["queueIds"] = json!(["synthetic-q3"]);
    lineage.as_object_mut().unwrap().remove("custodyLineageId");
    lineage = revised(lineage, "custodyLineageId");
    conn.execute(
        "UPDATE legacy_context_input_custody_lineages SET lineage_json=?1",
        [serde_json::to_string(&lineage).unwrap()],
    )
    .unwrap();
    drop(conn);
    rebind_image(&f.attempt, NAMES[0]);
    let error = translate_v28_capture(&f.attempt, &f.output)
        .unwrap_err()
        .to_string();
    assert!(error.contains("lineage"), "{error}");
}

#[test]
fn legacy_release_is_not_inferred_undelivered() {
    let f = fixture();
    let db = f.attempt.join("capture/image/semantic-context.sqlite3");
    let conn = Connection::open(&db).unwrap();
    conn.execute("UPDATE context_input_queue SET status='releasing',released_at='2026-10-13T00:01:00Z' WHERE queue_id='synthetic-q4'",[]).unwrap();
    drop(conn);
    rebind_image(&f.attempt, NAMES[0]);
    let result = translate_v28_capture(&f.attempt, &f.output).unwrap();
    assert_eq!(result.transfer_eligibility, "blocked_unclassified");
    let capsule: Value =
        serde_json::from_slice(&fs::read(f.output.join("capsule.json")).unwrap()).unwrap();
    assert_eq!(capsule["q4"]["physical_delivery"], "unresolved");
    assert!(
        capsule["q3"]["disposition_revision"]
            .as_str()
            .unwrap()
            .starts_with("sha256:")
    );
}

#[test]
fn queued_with_delivery_is_contradictory() {
    let f = fixture();
    let db = f.attempt.join("capture/image/semantic-context.sqlite3");
    let conn = Connection::open(&db).unwrap();
    conn.execute(
        "UPDATE context_input_queue SET delivery_json='{}' WHERE queue_id='synthetic-q4'",
        [],
    )
    .unwrap();
    drop(conn);
    rebind_image(&f.attempt, NAMES[0]);
    let error = translate_v28_capture(&f.attempt, &f.output)
        .unwrap_err()
        .to_string();
    assert!(error.contains("contradictory release"), "{error}");
}

#[test]
fn nonempty_registry_obligation_blocks_eligibility() {
    let f = fixture();
    let registry = f.attempt.join("capture/image/role-bindings.json");
    let key =
        serde_json::to_string(&json!(["slice-supervisor:legacy", "synthetic-message-4"])).unwrap();
    write_json(
        &registry,
        &json!({"schemaVersion":1,"revision":2,"bindings":{},"deliveries":{
            (key): {"logicalRoleInstanceId":"slice-supervisor:legacy","clientUserMessageId":"synthetic-message-4",
            "threadId":"synthetic-thread-4","requestFingerprint":"synthetic-fingerprint","status":"pending",
            "turnId":null,"startedAt":"2026-10-13T00:00:00Z","completedAt":null}
        }}),
    );
    rebind_image(&f.attempt, NAMES[1]);
    let result = translate_v28_capture(&f.attempt, &f.output).unwrap();
    assert_eq!(result.transfer_eligibility, "blocked_unclassified");
    let inventory: Value =
        serde_json::from_slice(&fs::read(f.output.join("inventory-private.json")).unwrap())
            .unwrap();
    assert!(
        inventory
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["table"] == "role-bindings.json/deliveries"
                && row["classification"] == "unclassified")
    );
}

#[test]
fn wrong_registry_delivery_key_fails_closed() {
    let f = fixture();
    let registry = f.attempt.join("capture/image/role-bindings.json");
    write_json(
        &registry,
        &json!({"schemaVersion":1,"revision":2,"bindings":{},"deliveries":{
            "[\"wrong-role\",\"synthetic-message-4\"]": {"logicalRoleInstanceId":"slice-supervisor:legacy","clientUserMessageId":"synthetic-message-4",
            "threadId":"synthetic-thread-4","requestFingerprint":"synthetic-fingerprint","status":"pending",
            "turnId":null,"startedAt":"2026-10-13T00:00:00Z","completedAt":null}
        }}),
    );
    rebind_image(&f.attempt, NAMES[1]);
    let error = translate_v28_capture(&f.attempt, &f.output)
        .unwrap_err()
        .to_string();
    assert!(error.contains("registry delivery key"), "{error}");
}

#[test]
fn valid_hash_with_wrong_disposition_type_is_rejected() {
    let f = fixture();
    let db = f.attempt.join("capture/image/semantic-context.sqlite3");
    let conn = Connection::open(&db).unwrap();
    let raw: String = conn.query_row("SELECT record_json FROM legacy_context_input_dispositions WHERE queue_id='synthetic-q4'",[],|r|r.get(0)).unwrap();
    let mut value: Value = serde_json::from_str(&raw).unwrap();
    value["type"] = json!("work-engine.unknown-disposition");
    value.as_object_mut().unwrap().remove("recordRevision");
    value = revised(value, "recordRevision");
    conn.execute("UPDATE legacy_context_input_dispositions SET record_revision=?1,record_json=?2 WHERE queue_id='synthetic-q4'",
        params![value["recordRevision"].as_str().unwrap(),serde_json::to_string(&value).unwrap()]).unwrap();
    drop(conn);
    rebind_image(&f.attempt, NAMES[0]);
    let error = translate_v28_capture(&f.attempt, &f.output)
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("envelope") || error.contains("disposition"),
        "{error}"
    );
}

#[test]
fn valid_hash_with_wrong_successor_scheduler_is_rejected() {
    let f = fixture();
    let db = f.attempt.join("capture/image/semantic-context.sqlite3");
    let conn = Connection::open(&db).unwrap();
    let raw: String = conn.query_row("SELECT record_json FROM legacy_context_input_dispositions WHERE queue_id='synthetic-q4'",[],|r|r.get(0)).unwrap();
    let mut record: Value = serde_json::from_str(&raw).unwrap();
    let acceptance = &mut record["successorAcceptance"];
    acceptance["successorSchedulerIdentity"] = json!("");
    acceptance
        .as_object_mut()
        .unwrap()
        .remove("acceptanceRevision");
    *acceptance = revised(acceptance.clone(), "acceptanceRevision");
    record.as_object_mut().unwrap().remove("recordRevision");
    record = revised(record, "recordRevision");
    conn.execute("UPDATE legacy_context_input_dispositions SET successor_acceptance_revision=?1,record_revision=?2,record_json=?3 WHERE queue_id='synthetic-q4'",
        params![record["successorAcceptance"]["acceptanceRevision"].as_str().unwrap(),record["recordRevision"].as_str().unwrap(),serde_json::to_string(&record).unwrap()]).unwrap();
    drop(conn);
    rebind_image(&f.attempt, NAMES[0]);
    let error = translate_v28_capture(&f.attempt, &f.output)
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("empty legacy successorSchedulerIdentity"),
        "{error}"
    );
    assert!(!f.output.exists());
}

#[test]
fn js_only_whitespace_acceptance_identity_is_refused_after_full_rebinding() {
    let f = fixture();
    rewrite_q4_unicode_maxsafe(&f);
    rewrite_q4_record(&f, |record, _| {
        record["successorAcceptance"]["successorSchedulerIdentity"] = json!("\u{FEFF}");
    });
    let conn = Connection::open(f.attempt.join("capture/image/semantic-context.sqlite3")).unwrap();
    let (acceptance_revision, record_revision, record_json): (String, String, String) = conn
        .query_row(
            "SELECT successor_acceptance_revision,record_revision,record_json FROM legacy_context_input_dispositions WHERE queue_id='synthetic-q4'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    let record: Value = serde_json::from_str(&record_json).unwrap();
    assert_eq!(
        record["successorAcceptance"]["successorSchedulerIdentity"],
        "\u{FEFF}"
    );
    assert_eq!(
        record["successorAcceptance"]["acceptanceRevision"],
        acceptance_revision
    );
    assert_eq!(record["recordRevision"], record_revision);
    assert_eq!(
        acceptance_revision,
        "sha256:4d861df67c8c74727db77ccd9d0a0d33b80fad47f71d45fa9f6476e088084ba8"
    );
    assert_eq!(
        record_revision,
        "sha256:8b5e9dc8fc5f7a602553ed73606a9eb61d0c82e3c1c4e32afa0338a9dc871866"
    );
    drop(conn);
    let error = translate_v28_capture(&f.attempt, &f.output)
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("empty legacy successorSchedulerIdentity"),
        "{error}"
    );
    assert!(!f.output.exists());

    let nonblank = fixture();
    rewrite_q4_unicode_maxsafe(&nonblank);
    rewrite_q4_record(&nonblank, |record, _| {
        record["successorAcceptance"]["successorSchedulerIdentity"] =
            json!("\u{FEFF}synthetic-scheduler");
    });
    let result = translate_v28_capture(&nonblank.attempt, &nonblank.output).unwrap();
    verify_completed_translation(&nonblank.attempt, &nonblank.output, &result).unwrap();
}

#[test]
fn above_js_safe_integer_binding_is_refused_without_capsule() {
    let f = fixture();
    let unsupported = MAX_LEGACY_SAFE_INTEGER + 1;
    rewrite_q4_record(&f, |record, conn| {
        let raw: String = conn
            .query_row(
                "SELECT input_json FROM context_input_queue WHERE queue_id='synthetic-q4'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let mut input: Value = serde_json::from_str(&raw).unwrap();
        input["bindingRevision"] = json!(unsupported);
        input = rerevised(input, "inputRevision");
        let bytes = serde_json::to_vec(&input).unwrap();
        record["target"]["bindingRevision"] = json!(unsupported);
        record["target"]["inputRevision"] = input["inputRevision"].clone();
        record["target"]["inputJsonSha256"] = json!(sha(&bytes));
        conn.execute(
            "UPDATE context_input_queue SET input_revision=?1,input_json=?2 WHERE queue_id='synthetic-q4'",
            params![input["inputRevision"].as_str().unwrap(), String::from_utf8(bytes).unwrap()],
        )
        .unwrap();
        conn.execute(
            "UPDATE legacy_context_input_dispositions SET binding_revision=?1,input_revision=?2,input_json_sha256=?3 WHERE queue_id='synthetic-q4'",
            params![unsupported, input["inputRevision"].as_str().unwrap(), record["target"]["inputJsonSha256"].as_str().unwrap()],
        )
        .unwrap();
    });
    let error = translate_v28_capture(&f.attempt, &f.output)
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("unsupported legacy canonical JSON number"),
        "{error}"
    );
    assert!(!f.output.exists());
}

#[test]
fn keyless_migration_shape_fails_before_translation() {
    let f = fixture();
    let db = f.attempt.join("capture/image/semantic-context.sqlite3");
    let conn = Connection::open(&db).unwrap();
    conn.execute_batch("DROP TABLE schema_migrations; CREATE TABLE schema_migrations(version INTEGER,applied_at TEXT) STRICT;").unwrap();
    for version in 1..=12 {
        conn.execute(
            "INSERT INTO schema_migrations VALUES (?1,'2026-10-13T00:00:00Z')",
            [version],
        )
        .unwrap();
    }
    drop(conn);
    rebind_image(&f.attempt, NAMES[0]);
    let error = translate_v28_capture(&f.attempt, &f.output)
        .unwrap_err()
        .to_string();
    assert!(error.contains("type/nullability/key"), "{error}");
}

#[test]
fn nullable_material_lineage_field_fails_before_translation() {
    let f = fixture();
    replace_lineage_json_column(&f.attempt, "TEXT");
    let error = translate_v28_capture(&f.attempt, &f.output)
        .unwrap_err()
        .to_string();
    assert!(
        error.contains(
            "copied schema-12 type/nullability/key mismatch: legacy_context_input_custody_lineages"
        ),
        "{error}"
    );
    assert!(!f.output.exists());
}

#[test]
fn wrong_material_lineage_type_fails_before_translation() {
    let f = fixture();
    replace_lineage_json_column(&f.attempt, "ANY NOT NULL");
    let error = translate_v28_capture(&f.attempt, &f.output)
        .unwrap_err()
        .to_string();
    assert!(
        error.contains(
            "copied schema-12 type/nullability/key mismatch: legacy_context_input_custody_lineages"
        ),
        "{error}"
    );
    assert!(!f.output.exists());
}

#[test]
fn missing_material_foreign_key_fails_schema_gate() {
    let f = fixture();
    let db = f.attempt.join("capture/image/semantic-context.sqlite3");
    let conn = Connection::open(&db).unwrap();
    conn.execute_batch("DROP TABLE legacy_context_input_successor_deliveries; CREATE TABLE legacy_context_input_successor_deliveries(queue_id TEXT PRIMARY KEY,claim_revision TEXT NOT NULL UNIQUE,claim_json TEXT NOT NULL,status TEXT NOT NULL CHECK(status IN ('pending','completed')),claimed_at TEXT NOT NULL,completed_at TEXT,delivery_revision TEXT,delivery_json TEXT,CHECK((status='pending' AND completed_at IS NULL AND delivery_revision IS NULL AND delivery_json IS NULL) OR (status='completed' AND completed_at IS NOT NULL AND delivery_revision IS NOT NULL AND delivery_json IS NOT NULL))) STRICT;").unwrap();
    drop(conn);
    rebind_image(&f.attempt, NAMES[0]);
    let error = translate_v28_capture(&f.attempt, &f.output)
        .unwrap_err()
        .to_string();
    assert!(error.contains("foreign-key mismatch"), "{error}");
}

#[test]
fn completed_readback_rejects_tamper() {
    let f = fixture();
    let result = translate_v28_capture(&f.attempt, &f.output).unwrap();
    verify_completed_translation(&f.attempt, &f.output, &result).unwrap();
    fs::write(f.output.join("capsule.json"), b"{}").unwrap();
    assert!(verify_completed_translation(&f.attempt, &f.output, &result).is_err());
}

#[test]
fn completed_readback_requires_bound_success_result() {
    let f = fixture();
    let mut result = translate_v28_capture(&f.attempt, &f.output).unwrap();
    result.publication_status = "not_established_in_directory".to_string();
    assert!(verify_completed_translation(&f.attempt, &f.output, &result).is_err());
}

#[test]
fn completed_readback_binds_exact_receipt_bytes() {
    let f = fixture();
    let result = translate_v28_capture(&f.attempt, &f.output).unwrap();
    let path = f.output.join("receipt.json");
    assert_eq!(sha(&fs::read(&path).unwrap()), result.receipt_sha256);
    let mut receipt: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    receipt["argv"] = json!(["another-invocation"]);
    write_json(&path, &receipt);
    let error = verify_completed_translation(&f.attempt, &f.output, &result).unwrap_err();
    assert!(
        error.contains("receipt/capsule/capture readback mismatch"),
        "{error}"
    );
}

#[test]
fn completed_readback_rejects_hash_rebound_error_path_and_provenance() {
    for field in [
        "primary_error",
        "attempt_root",
        "code_identity",
        "schema_version",
    ] {
        let f = fixture();
        let mut result = translate_v28_capture(&f.attempt, &f.output).unwrap();
        let path = f.output.join("receipt.json");
        let mut receipt: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        receipt[field] = match field {
            "primary_error" => json!("synthetic failure despite success status"),
            "attempt_root" => json!("/different-synthetic-attempt"),
            "code_identity" => json!({"crate":"other","version":"1","executable_sha256":null}),
            _ => json!(999),
        };
        write_json(&path, &receipt);
        result.receipt_sha256 = sha(&fs::read(&path).unwrap());
        let error = verify_completed_translation(&f.attempt, &f.output, &result).unwrap_err();
        assert!(
            error.contains("receipt/capsule/capture readback mismatch"),
            "{field}: {error}"
        );
    }
}

#[test]
fn completed_readback_rejects_mixed_attempt_receipt() {
    let first = fixture();
    let second = fixture();
    let result = translate_v28_capture(&first.attempt, &first.output).unwrap();
    let _ = translate_v28_capture(&second.attempt, &second.output).unwrap();
    fs::copy(
        second.output.join("receipt.json"),
        first.output.join("receipt.json"),
    )
    .unwrap();
    assert!(verify_completed_translation(&first.attempt, &first.output, &result).is_err());
}

#[test]
fn completed_readback_rejects_file_set_and_mode_mutations() {
    let extra = fixture();
    let extra_result = translate_v28_capture(&extra.attempt, &extra.output).unwrap();
    fs::write(extra.output.join("unlisted.json"), b"{}").unwrap();
    assert!(verify_completed_translation(&extra.attempt, &extra.output, &extra_result).is_err());

    let mode = fixture();
    let mode_result = translate_v28_capture(&mode.attempt, &mode.output).unwrap();
    fs::set_permissions(
        mode.output.join("receipt.json"),
        fs::Permissions::from_mode(0o644),
    )
    .unwrap();
    assert!(verify_completed_translation(&mode.attempt, &mode.output, &mode_result).is_err());

    let missing = fixture();
    let missing_result = translate_v28_capture(&missing.attempt, &missing.output).unwrap();
    fs::remove_file(missing.output.join("raw/q3-input.json")).unwrap();
    assert!(
        verify_completed_translation(&missing.attempt, &missing.output, &missing_result).is_err()
    );
}

#[test]
fn completed_readback_rejects_same_length_bytes_with_restored_mtime() {
    let f = fixture();
    let result = translate_v28_capture(&f.attempt, &f.output).unwrap();
    let path = f.output.join("raw/q4-input.json");
    let metadata = fs::metadata(&path).unwrap();
    let mut bytes = fs::read(&path).unwrap();
    let position = bytes.iter().position(|byte| *byte == b's').unwrap();
    bytes[position] = b'S';
    fs::write(&path, &bytes).unwrap();
    let times = fs::FileTimes::new()
        .set_accessed(metadata.accessed().unwrap())
        .set_modified(metadata.modified().unwrap());
    fs::File::options()
        .write(true)
        .open(&path)
        .unwrap()
        .set_times(times)
        .unwrap();
    let after = fs::metadata(&path).unwrap();
    assert_eq!(after.len(), metadata.len());
    assert_eq!(after.modified().unwrap(), metadata.modified().unwrap());
    let error = verify_completed_translation(&f.attempt, &f.output, &result).unwrap_err();
    assert!(error.contains("output manifest byte mismatch"), "{error}");
}

#[test]
fn synthetic_outer_success_record_write_and_sync_faults_withhold_consumer_evidence() {
    // There is no production B consumer in A. This isolated harness tests the
    // boundary: A's returned success does not turn a failed outer evidence
    // write/sync into durable downstream authorization.
    for fault in ["write", "sync"] {
        let f = fixture();
        let result = translate_v28_capture(&f.attempt, &f.output).unwrap();
        verify_completed_translation(&f.attempt, &f.output, &result).unwrap();
        let outer = f.attempt.join("caller-success-record.json");
        let staging = f.attempt.join("caller-success-record.partial");
        let attempted = (|| -> Result<(), &'static str> {
            if fault == "write" {
                return Err("injected outer write failure");
            }
            let bytes = serde_json::to_vec(&result).unwrap();
            fs::write(&staging, bytes).unwrap();
            if fault == "sync" {
                return Err("injected outer sync failure");
            }
            fs::File::open(&staging).unwrap().sync_all().unwrap();
            fs::rename(&staging, &outer).unwrap();
            Ok(())
        })();
        assert!(attempted.is_err());
        assert!(
            !outer.exists(),
            "{fault}: no durable caller success evidence"
        );
        assert!(f.output.is_dir(), "{fault}: A publication is not undone");
    }
}

#[test]
fn valid_hash_fractional_binding_revision_is_refused() {
    let f = fixture();
    rewrite_q4_record(&f, |record, conn| {
        let raw: String = conn
            .query_row(
                "SELECT input_json FROM context_input_queue WHERE queue_id='synthetic-q4'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let mut input: Value = serde_json::from_str(&raw).unwrap();
        input["bindingRevision"] = json!(1.5);
        input = rerevised(input, "inputRevision");
        let bytes = serde_json::to_vec(&input).unwrap();
        record["target"]["inputRevision"] = input["inputRevision"].clone();
        record["target"]["inputJsonSha256"] = json!(sha(&bytes));
        conn.execute("UPDATE context_input_queue SET input_revision=?1,input_json=?2 WHERE queue_id='synthetic-q4'",
            params![input["inputRevision"].as_str().unwrap(),String::from_utf8(bytes).unwrap()]).unwrap();
        conn.execute("UPDATE legacy_context_input_dispositions SET input_revision=?1,input_json_sha256=?2 WHERE queue_id='synthetic-q4'",
            params![input["inputRevision"].as_str().unwrap(),record["target"]["inputJsonSha256"].as_str().unwrap()]).unwrap();
    });
    let error = translate_v28_capture(&f.attempt, &f.output)
        .unwrap_err()
        .to_string();
    // This is strict legacy JSON type/parser evidence, not a reach of the
    // later input-identity validator.
    assert!(
        error.contains("floating-point legacy JSON is unsupported"),
        "{error}"
    );
    assert!(!f.output.exists());
}

#[test]
fn stage_parent_sync_failure_reports_created_stage_and_diagnostic() {
    let f = fixture();
    unsafe {
        std::env::set_var("S9_A_SYNTHETIC_TEST", "1");
        std::env::set_var("S9_A_TEST_FAULT", "stage_parent_sync");
    }
    let error = translate_v28_capture(&f.attempt, &f.output).unwrap_err();
    unsafe {
        std::env::remove_var("S9_A_SYNTHETIC_TEST");
        std::env::remove_var("S9_A_TEST_FAULT");
    }
    assert_eq!(error.phase, "stage_parent_sync");
    assert!(
        error
            .primary_error
            .contains("synthetic injected stage_parent_sync")
    );
    assert!(Path::new(&error.actual_artifact).is_dir());
    assert!(
        Path::new(&error.actual_artifact)
            .join("failure-receipt.json")
            .is_file()
    );
    assert!(!f.output.exists());
}

#[test]
fn pre_publication_fault_matrix_preserves_actual_location_and_primary_error() {
    for fault in ["file_sync", "raw_sync", "stage_sync", "rename"] {
        let f = fixture();
        unsafe {
            std::env::set_var("S9_A_SYNTHETIC_TEST", "1");
            std::env::set_var("S9_A_TEST_FAULT", fault);
        }
        let error = translate_v28_capture(&f.attempt, &f.output).unwrap_err();
        unsafe {
            std::env::remove_var("S9_A_SYNTHETIC_TEST");
            std::env::remove_var("S9_A_TEST_FAULT");
        }
        assert!(
            error
                .primary_error
                .contains(&format!("synthetic injected {fault}")),
            "{fault}: {error}"
        );
        assert!(
            Path::new(&error.actual_artifact).is_dir(),
            "{fault}: {error}"
        );
        assert!(!f.output.exists(), "{fault}");
    }
    let f = fixture();
    unsafe {
        std::env::set_var("S9_A_SYNTHETIC_TEST", "1");
        std::env::set_var("S9_A_TEST_FAULT", "stage_parent_sync,failure_receipt_sync");
    }
    let error = translate_v28_capture(&f.attempt, &f.output).unwrap_err();
    unsafe {
        std::env::remove_var("S9_A_SYNTHETIC_TEST");
        std::env::remove_var("S9_A_TEST_FAULT");
    }
    assert!(error.primary_error.contains("stage_parent_sync"));
    assert!(
        error
            .receipt_error
            .as_deref()
            .unwrap()
            .contains("failure_receipt_sync")
    );
    assert!(Path::new(&error.actual_artifact).is_dir());
}

#[test]
fn post_rename_readback_fault_retains_uncertain_output() {
    let f = fixture();
    unsafe {
        std::env::set_var("S9_A_SYNTHETIC_TEST", "1");
        std::env::set_var("S9_A_TEST_FAULT", "readback");
    }
    let error = translate_v28_capture(&f.attempt, &f.output).unwrap_err();
    unsafe {
        std::env::remove_var("S9_A_SYNTHETIC_TEST");
        std::env::remove_var("S9_A_TEST_FAULT");
    }
    assert_eq!(error.phase, "postflight_readback");
    assert_eq!(error.actual_artifact, f.output.display().to_string());
    assert!(f.output.is_dir());
    assert!(error.primary_error.contains("synthetic injected readback"));
}

#[test]
fn post_rename_failure_reports_actual_output_and_no_success_result() {
    let f = fixture();
    unsafe {
        std::env::set_var("S9_A_SYNTHETIC_TEST", "1");
        std::env::set_var("S9_A_TEST_FAULT", "parent_sync");
    }
    let error = translate_v28_capture(&f.attempt, &f.output).unwrap_err();
    unsafe {
        std::env::remove_var("S9_A_SYNTHETIC_TEST");
        std::env::remove_var("S9_A_TEST_FAULT");
    }
    assert_eq!(error.phase, "parent_sync");
    assert_eq!(error.actual_artifact, f.output.display().to_string());
    assert!(f.output.exists());
    assert!(error.primary_error.contains("synthetic injected"));
}

#[test]
fn pre_rename_failure_retains_actual_stage_and_secondary_receipt_error() {
    let f = fixture();
    unsafe {
        std::env::set_var("S9_A_SYNTHETIC_TEST", "1");
        std::env::set_var("S9_A_TEST_FAULT", "rename,failure_receipt");
    }
    let error = translate_v28_capture(&f.attempt, &f.output).unwrap_err();
    unsafe {
        std::env::remove_var("S9_A_SYNTHETIC_TEST");
        std::env::remove_var("S9_A_TEST_FAULT");
    }
    assert_eq!(error.phase, "no_replace_rename");
    assert!(Path::new(&error.actual_artifact).is_dir());
    assert!(!f.output.exists());
    assert!(error.primary_error.contains("synthetic injected rename"));
    assert!(
        error
            .receipt_error
            .as_deref()
            .unwrap()
            .contains("failure_receipt")
    );
}

#[test]
fn sidecar_absent_copy_translates_without_original_path_access() {
    let f = fixture();
    fs::remove_dir_all(&f.source).unwrap();
    let receipt = translate_v28_capture(&f.attempt, &f.output).unwrap();
    assert_eq!(receipt.translation_status, "qualified");
    assert_eq!(
        receipt.transfer_eligibility,
        "requires_separate_B_authority"
    );
    assert_eq!(receipt.unclassified_rows, 0);
    assert_eq!(
        fs::read(f.output.join("raw/q3-input.json")).unwrap(),
        f.q3_bytes
    );
    assert_eq!(
        fs::read(f.output.join("raw/q4-input.json")).unwrap(),
        f.q4_bytes
    );
    assert_eq!(
        fs::metadata(&f.output).unwrap().permissions().mode() & 0o777,
        0o700
    );
    assert_eq!(
        fs::metadata(f.output.join("capsule.json"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
    let capsule: Value =
        serde_json::from_slice(&fs::read(f.output.join("capsule.json")).unwrap()).unwrap();
    assert_eq!(capsule["q3"]["physical_delivery"], "not_applicable");
    assert_eq!(capsule["q4"]["physical_delivery"], "not_proved");
    assert_eq!(capsule["q3"]["successor_delivery_identity"], Value::Null);
    assert_eq!(
        capsule["q4"]["successor_delivery_identity"],
        "work-engine.context-input-successor-delivery:sha256:0000000000000000000000000000000000000000000000000000000000000000"
    );
    assert_eq!(capsule["no_rust_custody"], true);
    assert_eq!(capsule["no_admission"], true);
    assert_eq!(capsule["no_delivery"], true);
    assert_eq!(capsule["no_execution"], true);
    assert_eq!(
        capsule["selection_claim"],
        "current_selection_unestablished"
    );
    let report = fs::read_to_string(f.output.join("report-redacted.json")).unwrap();
    assert!(!report.contains("synthetic non-sensitive obligation"));
    let report_json: Value = serde_json::from_str(&report).unwrap();
    assert_eq!(report_json["q3"]["status"], "terminally_retired_no_replay");
    assert_eq!(
        report_json["q4"]["status"],
        "pending_accepted_undelivered_or_unresolved"
    );
    assert_eq!(report_json["authority_conferred"], false);
    verify_completed_translation(&f.attempt, &f.output, &receipt).unwrap();
}

#[test]
fn copied_sidecar_rejected_before_sqlite_read() {
    let f = fixture();
    fs::write(
        f.attempt.join("capture/image/semantic-context.sqlite3-wal"),
        b"synthetic-sidecar",
    )
    .unwrap();
    fs::write(
        f.attempt.join("capture/image/semantic-context.sqlite3"),
        b"not sqlite",
    )
    .unwrap();
    let error = translate_v28_capture(&f.attempt, &f.output)
        .unwrap_err()
        .to_string();
    assert!(error.contains("sidecar"), "{error}");
    assert!(!f.output.exists());
}

#[test]
fn missing_absence_evidence_fails_closed() {
    let f = fixture();
    let path = f.attempt.join("attempt-receipt.json");
    let mut receipt: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    receipt["preflight_source_metadata"]["semantic-context.sqlite3-shm"] = Value::Null;
    write_json(&path, &receipt);
    let error = translate_v28_capture(&f.attempt, &f.output)
        .unwrap_err()
        .to_string();
    assert!(error.contains("absence"), "{error}");
}

#[test]
fn duplicate_outer_json_key_fails_before_sqlite_read() {
    let f = fixture();
    let path = f.attempt.join("attempt-receipt.json");
    let original = fs::read_to_string(&path).unwrap();
    let duplicate = original.replacen("{\n", "{\n  \"schema_version\": 1,\n", 1);
    fs::write(&path, duplicate).unwrap();
    fs::write(
        f.attempt.join("capture/image/semantic-context.sqlite3"),
        b"not sqlite",
    )
    .unwrap();
    let error = translate_v28_capture(&f.attempt, &f.output)
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("duplicate JSON key schema_version"),
        "{error}"
    );
}

#[test]
fn manifest_hash_drift_refused() {
    let f = fixture();
    let path = f.attempt.join("capture/source-manifest.json");
    let mut manifest: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    manifest["selection_claim"] = json!("invented-current");
    write_json(&path, &manifest);
    let error = translate_v28_capture(&f.attempt, &f.output)
        .unwrap_err()
        .to_string();
    assert!(error.contains("binding mismatch"), "{error}");
}

#[test]
fn copied_image_drift_between_classification_and_final_readback_is_refused() {
    let f = fixture();
    let marker = f.attempt.join("synthetic-image-readback-barrier");
    let copied_registry = f.attempt.join("capture/image/role-bindings.json");
    let marker_for_thread = marker.clone();
    let worker = thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(4);
        while !marker_for_thread.exists() {
            assert!(
                Instant::now() < deadline,
                "translator did not reach image readback barrier"
            );
            thread::sleep(Duration::from_millis(10));
        }
        use std::io::Write as _;
        fs::OpenOptions::new()
            .append(true)
            .open(copied_registry)
            .unwrap()
            .write_all(b"\n")
            .unwrap();
        fs::remove_file(marker_for_thread).unwrap();
    });
    unsafe {
        std::env::set_var("S9_A_SYNTHETIC_TEST", "1");
        std::env::set_var("S9_A_TEST_BARRIER", "before_final_image_readback");
        std::env::set_var("S9_A_TEST_BARRIER_MARKER", &marker);
    }
    let error = translate_v28_capture(&f.attempt, &f.output).unwrap_err();
    unsafe {
        std::env::remove_var("S9_A_SYNTHETIC_TEST");
        std::env::remove_var("S9_A_TEST_BARRIER");
        std::env::remove_var("S9_A_TEST_BARRIER_MARKER");
    }
    worker.join().unwrap();
    assert!(
        error
            .primary_error
            .contains("copied image drift after SQLite read"),
        "{error}"
    );
    assert!(!f.output.exists());
}

#[test]
fn isolated_translation_child() {
    let Some(attempt) = std::env::var_os("S9_A_CHILD_ATTEMPT") else {
        return;
    };
    let output = PathBuf::from(std::env::var_os("S9_A_CHILD_OUTPUT").unwrap());
    let result_path = PathBuf::from(std::env::var_os("S9_A_CHILD_RESULT").unwrap());
    let result = translate_v28_capture(&PathBuf::from(attempt), &output).unwrap();
    write_json(&result_path, &serde_json::to_value(&result).unwrap());
}

#[test]
fn resultless_pre_and_post_rename_child_interruption_is_quarantined() {
    for barrier in ["pre_rename", "post_rename"] {
        let f = fixture();
        let marker = f.attempt.join(format!("{barrier}-barrier"));
        let result = f.attempt.join(format!("{barrier}-returned-success.json"));
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args(["isolated_translation_child", "--nocapture"])
            .env("S9_A_CHILD_ATTEMPT", &f.attempt)
            .env("S9_A_CHILD_OUTPUT", &f.output)
            .env("S9_A_CHILD_RESULT", &result)
            .env("S9_A_SYNTHETIC_TEST", "1")
            .env("S9_A_TEST_BARRIER", barrier)
            .env("S9_A_TEST_BARRIER_MARKER", &marker)
            .env_remove("S9_A_TEST_FAULT")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(4);
        while !marker.exists() {
            if Instant::now() >= deadline {
                let _ = child.kill();
                let status = child.wait().unwrap();
                panic!("{barrier} child never reached barrier; status={status}");
            }
            assert!(
                child.try_wait().unwrap().is_none(),
                "{barrier} child ended early"
            );
            thread::sleep(Duration::from_millis(10));
        }
        child.kill().unwrap();
        let status = child.wait().unwrap();
        assert!(!status.success());
        assert!(
            !result.exists(),
            "{barrier} must not yield a caller-held success result"
        );
        if barrier == "pre_rename" {
            assert!(!f.output.exists());
            let basename = f.output.file_name().unwrap().to_str().unwrap();
            assert!(
                fs::read_dir(f.output.parent().unwrap())
                    .unwrap()
                    .any(|entry| {
                        entry
                            .unwrap()
                            .file_name()
                            .to_string_lossy()
                            .starts_with(&format!(".{basename}.s9-a-"))
                    })
            );
        } else {
            assert!(
                f.output.is_dir(),
                "post-rename bytes may survive without caller success"
            );
        }
        println!(
            "synthetic resultless {barrier} child_pid={} exit={status}",
            child.id()
        );
    }
}

#[test]
fn unsupported_schema_is_not_a_valid_capsule() {
    let f = fixture();
    let db = f.attempt.join("capture/image/semantic-context.sqlite3");
    let conn = Connection::open(&db).unwrap();
    conn.pragma_update(None, "user_version", 11).unwrap();
    drop(conn);
    let mut outer: Value =
        serde_json::from_slice(&fs::read(f.attempt.join("attempt-receipt.json")).unwrap()).unwrap();
    let mut manifest: Value =
        serde_json::from_slice(&fs::read(f.attempt.join("capture/source-manifest.json")).unwrap())
            .unwrap();
    let digest = sha(&fs::read(&db).unwrap());
    outer["artifacts"]["capture/image/semantic-context.sqlite3"]["sha256"] = json!(digest);
    manifest["files"][0]["sha256"] = json!(digest);
    manifest["files"][0]["copy_sha256"] = json!(digest);
    write_json(&f.attempt.join("capture/source-manifest.json"), &manifest);
    let manifest_digest = sha(&fs::read(f.attempt.join("capture/source-manifest.json")).unwrap());
    outer["child_manifest_sha256"] = json!(manifest_digest);
    let mut child: Value =
        serde_json::from_slice(&fs::read(f.attempt.join("capture/receipt.json")).unwrap()).unwrap();
    child["manifest_sha256"] = json!(manifest_digest);
    write_json(&f.attempt.join("capture/receipt.json"), &child);
    outer["child_receipt"]["sha256"] = json!(sha(&fs::read(
        f.attempt.join("capture/receipt.json")
    )
    .unwrap()));
    write_json(&f.attempt.join("attempt-receipt.json"), &outer);
    let error = translate_v28_capture(&f.attempt, &f.output)
        .unwrap_err()
        .to_string();
    assert!(error.contains("schema 11"), "{error}");
}

#[test]
fn open_admission_remains_unclassified_and_blocks_transfer() {
    let f = fixture();
    let db = f.attempt.join("capture/image/semantic-context.sqlite3");
    let conn = Connection::open(&db).unwrap();
    conn.execute(
        "INSERT INTO context_input_admissions VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
        params![
            "slice-supervisor:legacy",
            "synthetic-thread",
            1_i64,
            "sha256:synthetic-transition",
            "open",
            Option::<String>::None,
            "2026-10-13T00:00:00Z",
            Option::<String>::None
        ],
    )
    .unwrap();
    drop(conn);
    let digest = sha(&fs::read(&db).unwrap());
    let mut manifest: Value =
        serde_json::from_slice(&fs::read(f.attempt.join("capture/source-manifest.json")).unwrap())
            .unwrap();
    manifest["files"][0]["sha256"] = json!(digest);
    manifest["files"][0]["copy_sha256"] = json!(digest);
    write_json(&f.attempt.join("capture/source-manifest.json"), &manifest);
    let manifest_digest = sha(&fs::read(f.attempt.join("capture/source-manifest.json")).unwrap());
    let mut child: Value =
        serde_json::from_slice(&fs::read(f.attempt.join("capture/receipt.json")).unwrap()).unwrap();
    child["manifest_sha256"] = json!(manifest_digest);
    write_json(&f.attempt.join("capture/receipt.json"), &child);
    let mut outer: Value =
        serde_json::from_slice(&fs::read(f.attempt.join("attempt-receipt.json")).unwrap()).unwrap();
    outer["artifacts"]["capture/image/semantic-context.sqlite3"]["sha256"] = json!(digest);
    outer["child_manifest_sha256"] = json!(manifest_digest);
    outer["child_receipt"]["sha256"] = json!(sha(&fs::read(
        f.attempt.join("capture/receipt.json")
    )
    .unwrap()));
    write_json(&f.attempt.join("attempt-receipt.json"), &outer);
    let receipt = translate_v28_capture(&f.attempt, &f.output).unwrap();
    assert_eq!(receipt.transfer_eligibility, "blocked_unclassified");
    assert_eq!(receipt.unclassified_rows, 1);
}

// Rebind only envelope bytes for deliberately malformed synthetic metadata.
fn rebind_capture_envelopes(attempt: &Path) {
    let manifest_sha = sha(&fs::read(attempt.join("capture/source-manifest.json")).unwrap());
    let child_path = attempt.join("capture/receipt.json");
    let mut child: Value = serde_json::from_slice(&fs::read(&child_path).unwrap()).unwrap();
    child["manifest_sha256"] = json!(manifest_sha);
    write_json(&child_path, &child);
    let outer_path = attempt.join("attempt-receipt.json");
    let mut outer: Value = serde_json::from_slice(&fs::read(&outer_path).unwrap()).unwrap();
    outer["child_manifest_sha256"] = json!(manifest_sha);
    outer["child_receipt"]["sha256"] = json!(sha(&fs::read(child_path).unwrap()));
    write_json(&outer_path, &outer);
}

#[test]
fn producer_shaped_outer_metadata_preserves_raw_hash_and_completed_readback() {
    let f = fixture();
    let path = f.attempt.join("attempt-receipt.json");
    let original = fs::read(&path).unwrap();
    let outer: Value = serde_json::from_slice(&original).unwrap();
    assert!(outer.get("source_root").is_none());
    assert_eq!(outer["duration_seconds"], json!(0.381));
    let result = translate_v28_capture(&f.attempt, &f.output).unwrap();
    assert_eq!(result.source_attempt_sha256, sha(&original));
    verify_completed_translation(&f.attempt, &f.output, &result).unwrap();
    assert_eq!(fs::read(path).unwrap(), original);
}

#[test]
fn contradictory_capture_root_provenance_is_rejected() {
    for target in ["child", "manifest", "outer", "argv"] {
        let f = fixture();
        let path = f.attempt.join(match target {
            "child" => "capture/receipt.json",
            "manifest" => "capture/source-manifest.json",
            _ => "attempt-receipt.json",
        });
        let mut value: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        if target == "argv" {
            value["argv"][2] = json!("/synthetic-contradictory-root");
        } else {
            value["source_root"] = json!("/synthetic-contradictory-root");
        }
        write_json(&path, &value);
        rebind_capture_envelopes(&f.attempt);
        let error = translate_v28_capture(&f.attempt, &f.output)
            .unwrap_err()
            .to_string();
        assert!(error.contains("source-root"), "{target}: {error}");
        assert!(!f.output.exists());
    }
}

#[test]
fn ambiguous_or_malformed_capture_root_arguments_are_rejected() {
    for argv in [
        Value::Null,
        json!([]),
        json!(["synthetic-capture"]),
        json!(["synthetic-capture", "--source-root"]),
        json!(["synthetic-capture", "--source-root", 1]),
        json!(["synthetic-capture", "--source-root=/synthetic", "ignored"]),
    ] {
        let f = fixture();
        let path = f.attempt.join("attempt-receipt.json");
        let mut outer: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        outer["argv"] = argv;
        write_json(&path, &outer);
        let error = translate_v28_capture(&f.attempt, &f.output)
            .unwrap_err()
            .to_string();
        assert!(error.contains("source-root"), "{error}");
    }
    let f = fixture();
    let path = f.attempt.join("attempt-receipt.json");
    let mut outer: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    outer["argv"]
        .as_array_mut()
        .unwrap()
        .extend([json!("--source-root"), json!(f.source)]);
    write_json(&path, &outer);
    let error = translate_v28_capture(&f.attempt, &f.output)
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("source-root argv missing or repeated"),
        "{error}"
    );
}

#[test]
fn capture_metadata_numbers_do_not_relax_required_integer_versions() {
    for name in [
        "attempt-receipt.json",
        "capture/receipt.json",
        "capture/source-manifest.json",
    ] {
        for version in [json!(1.0), json!(1.5), json!("1"), Value::Null] {
            let f = fixture();
            let path = f.attempt.join(name);
            let mut value: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
            value["schema_version"] = version;
            write_json(&path, &value);
            rebind_capture_envelopes(&f.attempt);
            let error = translate_v28_capture(&f.attempt, &f.output)
                .unwrap_err()
                .to_string();
            assert!(
                error.contains("receipt/manifest version"),
                "{name}: {error}"
            );
        }
    }
}

#[test]
fn capture_envelopes_reject_nested_duplicate_keys_and_nonfinite_numbers() {
    for name in [
        "attempt-receipt.json",
        "capture/receipt.json",
        "capture/source-manifest.json",
    ] {
        for (member, expected) in [
            (
                "\"timing\": {\"seconds\": 0.381, \"seconds\": 0.382},",
                "duplicate JSON key seconds",
            ),
            ("\"duration_seconds\": 1e400,", "number out of range"),
        ] {
            let f = fixture();
            let path = f.attempt.join(name);
            let original = fs::read_to_string(&path).unwrap();
            fs::write(path, original.replacen('{', &format!("{{{member}"), 1)).unwrap();
            let error = translate_v28_capture(&f.attempt, &f.output)
                .unwrap_err()
                .to_string();
            assert!(error.contains(expected), "{name}: {error}");
            assert!(!f.output.exists());
        }
    }
}
