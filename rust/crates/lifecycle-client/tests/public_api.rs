use lifecycle_client::{
    ClientError, ObservationTransport, ReadClient, SnapshotHeaderV1, SubjectRef,
};
use lifecycle_wire::{CursorV1, WaitResultV1};

struct LocalReadTransport;

impl ObservationTransport for LocalReadTransport {
    fn snapshot(&self, subject: &SubjectRef) -> Result<SnapshotHeaderV1, ClientError> {
        Ok(SnapshotHeaderV1 {
            protocol_version: 1,
            subject_id: subject.as_str().to_owned(),
            cursor: CursorV1 {
                store_id: "store-1".into(),
                stream_id: "stream-1".into(),
                commit_sequence: "7".into(),
            },
        })
    }

    fn wait(&self, _: &SubjectRef, _: &CursorV1, _: u64) -> WaitResultV1<SnapshotHeaderV1> {
        WaitResultV1::ObservationUnavailable {
            code: lifecycle_wire::WireErrorCode::ObservationUnavailable,
        }
    }
}

#[test]
fn public_consumer_has_only_read_and_wait_operations() {
    let client = ReadClient::new(LocalReadTransport);
    let subject = SubjectRef::parse("subject-1").unwrap();
    let snapshot = client.snapshot(&subject).unwrap();
    assert_eq!(snapshot.cursor.commit_sequence, "7");
    assert!(matches!(
        client.wait(&subject, &snapshot.cursor, 10),
        WaitResultV1::ObservationUnavailable { .. }
    ));
}
