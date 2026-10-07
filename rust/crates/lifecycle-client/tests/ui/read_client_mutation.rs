use lifecycle_client::{ClientError, ObservationTransport, ReadClient, SnapshotHeaderV1, SubjectRef};
use lifecycle_wire::{CursorV1, WaitResultV1};

struct LocalTransport;

impl ObservationTransport for LocalTransport {
    fn snapshot(&self, _: &SubjectRef) -> Result<SnapshotHeaderV1, ClientError> {
        todo!()
    }

    fn wait(&self, _: &SubjectRef, _: &CursorV1, _: u64) -> WaitResultV1<SnapshotHeaderV1> {
        todo!()
    }
}

fn main() {
    let client = ReadClient::new(LocalTransport);
    let subject = SubjectRef::parse("subject-1").unwrap();
    client.submit_command(&subject, "advance");
}
