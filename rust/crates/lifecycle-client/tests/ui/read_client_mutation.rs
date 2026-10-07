use std::time::Duration;
use lifecycle_client::{ClientError, ObservationTransport, ReadClient, SubjectRef, WaitHintV1};
use lifecycle_wire::{CursorV1, LifecycleSnapshotV1, WaitTargetV1};

struct LocalTransport;
impl ObservationTransport for LocalTransport {
    fn snapshot(&self, _: &SubjectRef, _: Option<&WaitTargetV1>, _: Duration) -> Result<LifecycleSnapshotV1, ClientError> { todo!() }
    fn wait_hint(&self, _: &SubjectRef, _: &WaitTargetV1, _: &CursorV1, _: Duration) -> Result<WaitHintV1, ClientError> { todo!() }
}
fn main() {
    let client = ReadClient::new(LocalTransport);
    let subject = SubjectRef::parse("subject-1").unwrap();
    client.submit_command(&subject, "advance");
}
