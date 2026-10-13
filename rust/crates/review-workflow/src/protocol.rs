use crate::{Result, refuse, startup::StartupDocument};
use serde::{Deserialize, Serialize};

pub const COMMAND_LIMIT: usize = 64 * 1024;
pub const REPLY_LIMIT: usize = 1024 * 1024;
#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Operation {
    Status,
    InspectInitial,
    ExecuteInitial,
    RecoverInitial,
    StopAdmission,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Command {
    pub version: u32,
    pub request_id: String,
    pub operation: Operation,
    pub obligation_id: Option<String>,
    pub operation_id: Option<String>,
    pub expected_revision: Option<String>,
}
impl Command {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        if bytes.len() > COMMAND_LIMIT {
            return Err(refuse("capacity", "command exceeds 64 KiB"));
        }
        let value: Self = serde_json::from_slice(bytes)?;
        if value.version != 1 || value.request_id.is_empty() || value.request_id.len() > 128 {
            return Err(refuse("contract", "invalid command version or request ID"));
        }
        if matches!(
            value.operation,
            Operation::Status | Operation::StopAdmission
        ) && (value.obligation_id.is_some()
            || value.operation_id.is_some()
            || value.expected_revision.is_some())
        {
            return Err(refuse(
                "contract",
                "control operation contains domain arguments",
            ));
        }
        Ok(value)
    }
    pub(crate) fn check_scope(&self, startup: &StartupDocument) -> Result<()> {
        if self.obligation_id.as_deref() != Some(&startup.obligation_id)
            || self.operation_id.as_deref() != Some(&startup.operation_id)
        {
            return Err(refuse(
                "scope",
                "operation or obligation differs from startup",
            ));
        }
        Ok(())
    }
}
