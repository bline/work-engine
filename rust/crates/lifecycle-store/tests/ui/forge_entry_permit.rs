use lifecycle_core::{AttemptId, EffectId, RuntimeIncarnation, SubjectId};
use lifecycle_store::AuthorizedEntry;

fn main() {
    let _entry = AuthorizedEntry {
        effect: EffectId::parse("effect-1").unwrap(),
        attempt: AttemptId::parse("attempt-1").unwrap(),
        subject: SubjectId::parse("subject-1").unwrap(),
        incarnation: RuntimeIncarnation::parse("incarnation-1").unwrap(),
    };
}
