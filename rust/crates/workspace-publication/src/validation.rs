use crate::codec::digest;
use crate::contract::{GitOid, ValidationReceipt, WorkspaceConfig};
use crate::{Error, Result};
use std::path::Path;
use std::process::Command;

pub fn run(
    config: &WorkspaceConfig,
    worktree: &Path,
    tree: &GitOid,
    root_id: &str,
    record_id: &str,
) -> Result<ValidationReceipt> {
    let (program, args) = config
        .validation_program
        .split_first()
        .ok_or_else(|| Error::Invalid("validator not installed".into()))?;
    if !Path::new(program).is_absolute() {
        return Err(Error::Invalid("validator must be absolute".into()));
    }
    let mut cmd = Command::new(program);
    cmd.env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("LC_ALL", "C")
        .env("HOME", "/");
    let output = cmd
        .args(args)
        .current_dir(worktree)
        .env("WORK_ENGINE_VALIDATED_TREE", &tree.hex)
        .output()
        .map_err(|e| Error::Io(e.to_string()))?;
    let receipt = ValidationReceipt {
        schema_version: 1,
        root_id: root_id.into(),
        record_id: record_id.into(),
        tree: tree.clone(),
        policy_digest: config.validation_policy_digest.clone(),
        argv_digest: digest("validation-argv", &config.validation_program)?,
        exit_code: output.status.code().unwrap_or(-1),
        stdout_sha256: crate::codec::raw_sha256(&output.stdout),
        stderr_sha256: crate::codec::raw_sha256(&output.stderr),
    };
    if !output.status.success() {
        return Err(Error::Conflict(format!(
            "integrated-tree validator failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    Ok(receipt)
}
