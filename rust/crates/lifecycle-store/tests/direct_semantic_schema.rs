//! The direct-control tables admit exactly two evidence classes, so a stored
//! class can never name something the admission predicate was not written for.
//! Rows are written with a raw connection: this checks the schema constraint,
//! not the store's own insert paths (those are covered through the service).

use lifecycle_store::SqliteLifecycleStore;
use rusqlite::{params, Connection};
use tempfile::tempdir;

fn open() -> (tempfile::TempDir, Connection) {
    use std::os::unix::fs::PermissionsExt;
    let temp = tempdir().unwrap();
    let root = temp.path().join("store");
    std::fs::create_dir(&root).unwrap();
    std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700)).unwrap();
    drop(SqliteLifecycleStore::open(&root, "issuer:schema".into()).unwrap());
    let connection = Connection::open(root.join("lifecycle.sqlite")).unwrap();
    // Only the CHECK constraints are under test, not the references.
    connection.execute_batch("PRAGMA foreign_keys=OFF").unwrap();
    (temp, connection)
}

fn attempt(connection: &Connection, id: &str, class: &str) -> rusqlite::Result<usize> {
    connection.execute(
        "INSERT INTO direct_semantic_attempts(operation_id,capture_id,role,attempt_id,exposure_id,policy_id,grant_id,grant_revision,principal,evidence_class,evidence_ref,requested_provider,requested_model,input_sha256,prepared_binding_sha256,prepared_wire_sha256,prepared_artifact_sha256,state,created_wall_ms) VALUES(?1,'capture','compile',?1||'-a','exposure','policy','grant',1,'principal',?2,'ref','provider','model',?1||'-i',?1||'-b',?1||'-w','artifact','intent_recorded',1)",
        params![id, class],
    )
}

fn result(connection: &Connection, id: &str, class: &str) -> rusqlite::Result<usize> {
    connection.execute(
        "INSERT INTO direct_semantic_results(result_id,transition_id,capture_id,compile_operation_id,verifier_operation_id,candidate_artifact_sha256,findings_artifact_sha256,verdict,eligibility,basis_sha256,policy_id,evidence_class,evidence_ref,created_wall_ms) VALUES(?1,'transition','capture','compile','verify','candidate','findings','accepted','current','basis','policy',?2,'ref',1)",
        params![id, class],
    )
}

#[test]
fn only_synthetic_local_and_public_api_classes_can_be_stored() {
    let (_temp, connection) = open();
    for (index, class) in ["synthetic_local", "public_api"].iter().enumerate() {
        attempt(&connection, &format!("ok-{index}"), class).unwrap();
        result(&connection, &format!("ok-{index}"), class).unwrap();
    }
    for class in ["", "live_api", "PUBLIC_API", "public_api ", "native", "admitted"] {
        let attempt = attempt(&connection, &format!("bad-{}", class.len()), class);
        assert!(attempt.is_err(), "attempt accepted class {class:?}");
        let result = result(&connection, &format!("bad-{}", class.len()), class);
        assert!(result.is_err(), "result accepted class {class:?}");
    }
}

#[test]
fn a_result_is_proof_scoped_until_an_admission_grant_is_fixed_on_it() {
    let (_temp, connection) = open();
    result(&connection, "r", "public_api").unwrap();
    let (grant, revision): (Option<String>, Option<i64>) = connection
        .query_row(
            "SELECT admission_grant_id,admission_grant_revision FROM direct_semantic_results WHERE result_id='r'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!((grant, revision), (None, None));
}
