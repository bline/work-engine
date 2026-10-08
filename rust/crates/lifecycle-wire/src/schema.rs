//! Pinned public schema generation. Checked-in output is verified by tests.

use std::collections::BTreeMap;

use schemars::JsonSchema;
use schemars::generate::SchemaSettings;
use serde_json::Value;

use crate::{
    CommandDto, CommandDtoV2, CommandResultV2, ErrorDtoV1, LifecycleSnapshotV1, WaitRequestV1,
    WaitResultV1,
};

fn schema_for<T: JsonSchema>() -> Value {
    let mut generator = SchemaSettings::draft2020_12().into_generator();
    serde_json::to_value(generator.root_schema_for::<T>()).expect("schema serializes")
}

pub fn schema_bundle_v1() -> BTreeMap<&'static str, Value> {
    let mut command = schema_for::<CommandDto>();
    command
        .as_object_mut()
        .expect("command schema object")
        .insert("unevaluatedProperties".into(), Value::Bool(false));
    BTreeMap::from([
        ("command.schema.json", command),
        ("error.schema.json", schema_for::<ErrorDtoV1>()),
        ("snapshot.schema.json", schema_for::<LifecycleSnapshotV1>()),
        ("wait-request.schema.json", schema_for::<WaitRequestV1>()),
        (
            "wait-result.schema.json",
            schema_for::<WaitResultV1<LifecycleSnapshotV1>>(),
        ),
    ])
}

pub fn schema_bundle_v2() -> BTreeMap<&'static str, Value> {
    let mut command = schema_for::<CommandDtoV2>();
    command
        .as_object_mut()
        .expect("command schema object")
        .insert("unevaluatedProperties".into(), Value::Bool(false));
    BTreeMap::from([
        ("command.schema.json", command),
        (
            "command-result.schema.json",
            schema_for::<CommandResultV2>(),
        ),
    ])
}
