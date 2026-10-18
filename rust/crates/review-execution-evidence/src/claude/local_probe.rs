//! Non-authorizing installed-program probe. This module is absent from normal
//! builds. It cannot construct campaign Applied entry or E1 production custody.
use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;

use serde::Serialize;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use super::qualified::{command_arguments, command_digest};
use super::{CandidateBoundary, Rp0Error, TrustedClaudeProfile};

const CANARY: &str = "RP3_LOCAL_NO_EGRESS_CANARY";
const PROMPT: &[u8] = b"RP3 local launch probe. External inference is disabled.\n";
const OUTPUT_LIMIT: u64 = 2 * 16 * 1024 * 1024 + 4 * 1024 * 1024 + 1024 * 1024;
const STDERR_LIMIT: u64 = 4 * 1024 * 1024;

/// Local observations only. `spawned` means the pinned executable was entered
/// with selected review flags after guard READY in an isolated network
/// namespace. It does not establish authentication, tool use, response shape,
/// full-review memory fit, remote model identity, cost or provider settlement.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalProbeResult {
    pub schema_version: u32,
    pub resource_policy: String,
    pub combined_limit: u64,
    pub host_custody_charge: u64,
    pub quota_page_size: u64,
    pub program_sha256: String,
    pub command_sha256: String,
    pub profile_sha256: String,
    pub guard_ready: bool,
    pub network_denied_before_ready: bool,
    pub spawned: bool,
    pub local_milestone_met: bool,
    pub selected_child_pid: Option<u32>,
    pub guard_exit_code: Option<i32>,
    pub timed_out: bool,
    pub stdout_bytes: usize,
    pub stderr_bytes: usize,
    pub memory_max: u64,
    pub memory_peak: u64,
    pub swap_max_zero: bool,
    pub swap_current_zero: bool,
    pub oom_kill: u64,
}

impl CandidateBoundary {
    /// Probe a descriptive installed profile without accepting it for live
    /// execution. The caller must supply an exact profile/session selection;
    /// this function chooses no model, credentials, budget or production root.
    pub async fn probe_installed_local(
        &self,
        profile: &TrustedClaudeProfile,
        probe_root: &Path,
        nonce: &str,
        session_id: &str,
        horizon: Duration,
    ) -> Result<LocalProbeResult, Rp0Error> {
        if !profile.installed()
            || session_id.is_empty()
            || session_id.len() > 128
            || horizon.is_zero()
            || horizon > Duration::from_secs(600)
        {
            return Err(Rp0Error::IsolationFailed);
        }
        fs::create_dir(probe_root)?;
        fs::set_permissions(probe_root, fs::Permissions::from_mode(0o700))?;
        let live = probe_root.join("session-live");
        fs::create_dir(&live)?;
        fs::set_permissions(&live, fs::Permissions::from_mode(0o700))?;
        let args = command_arguments(profile, session_id, true);
        eprintln!("RP3_PROBE_STAGE=before_guarded_launch");
        let mut launch = self.guarded_local_probe_launch(profile, &live, nonce, &args)?;
        eprintln!("RP3_PROBE_STAGE=after_guarded_launch");
        let mut child = launch
            .command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()?;
        eprintln!("RP3_PROBE_STAGE=after_bwrap_spawn");
        let mut stdin = child.stdin.take().ok_or(Rp0Error::IsolationFailed)?;
        let stdout = child.stdout.take().ok_or(Rp0Error::IsolationFailed)?;
        let mut stderr = child.stderr.take().ok_or(Rp0Error::IsolationFailed)?;
        let mut ready = Vec::new();
        let read_ready = async {
            while ready.len() <= 4096 {
                let mut byte = [0];
                stderr.read_exact(&mut byte).await?;
                ready.push(byte[0]);
                if byte[0] == b'\n' {
                    return Ok::<(), std::io::Error>(());
                }
            }
            Err(std::io::Error::other("probe guard READY oversized"))
        };
        if !matches!(
            tokio::time::timeout(Duration::from_secs(5), read_ready).await,
            Ok(Ok(()))
        ) || ready != launch.ready_line.as_bytes()
        {
            let mut diagnostic = Vec::new();
            let _ = tokio::time::timeout(
                Duration::from_secs(1),
                stderr.take(4096).read_to_end(&mut diagnostic),
            )
            .await;
            let status = tokio::time::timeout(Duration::from_secs(1), child.wait()).await;
            eprintln!(
                "RP3_PROBE_STAGE=ready_refused bytes={:?} status={status:?} stderr={:?}",
                ready,
                String::from_utf8_lossy(&diagnostic)
            );
            let _ = child.start_kill();
            let _ = tokio::time::timeout(Duration::from_secs(5), child.wait()).await;
            return Err(Rp0Error::IsolationFailed);
        }
        eprintln!("RP3_PROBE_STAGE=after_guard_ready");
        let mut frame = Vec::with_capacity(CANARY.len() + PROMPT.len() + 8);
        frame.extend_from_slice(&(CANARY.len() as u32).to_be_bytes());
        frame.extend_from_slice(CANARY.as_bytes());
        frame.extend_from_slice(&(PROMPT.len() as u32).to_be_bytes());
        frame.extend_from_slice(PROMPT);
        let mut out = Vec::new();
        let mut err = Vec::new();
        let mut out_reader = stdout.take(OUTPUT_LIMIT + 1);
        let mut err_reader = stderr.take(STDERR_LIMIT + 1);
        let work = async {
            tokio::join!(
                async {
                    let result = stdin.write_all(&frame).await;
                    drop(stdin);
                    result
                },
                out_reader.read_to_end(&mut out),
                err_reader.read_to_end(&mut err),
                child.wait(),
            )
        };
        let (exit, timed_out) = match tokio::time::timeout(horizon, work).await {
            Ok((write, out_read, err_read, waited)) => {
                if write.is_err() || out_read.is_err() || err_read.is_err() {
                    return Err(Rp0Error::IsolationFailed);
                }
                (Some(waited?), false)
            }
            Err(_) => {
                let _ = child.start_kill();
                let waited = tokio::time::timeout(Duration::from_secs(5), child.wait())
                    .await
                    .map_err(|_| Rp0Error::IsolationFailed)??;
                (Some(waited), true)
            }
        };
        eprintln!("RP3_PROBE_STAGE=after_capture exit={exit:?} timed_out={timed_out}");
        if out.len() as u64 > OUTPUT_LIMIT || err.len() as u64 > STDERR_LIMIT {
            return Err(Rp0Error::OutputLimit);
        }
        let expected = fs::metadata(profile.program().path())?;
        let marker = format!("SPAWNED:{nonce}:");
        let mut matches = err
            .split(|byte| *byte == b'\n')
            .filter_map(|line| line.strip_prefix(marker.as_bytes()))
            .filter_map(|line| std::str::from_utf8(line).ok())
            .filter_map(|line| {
                let mut fields = line.split(':');
                let pid = fields.next()?.parse::<u32>().ok()?;
                let dev = fields.next()?.parse::<u64>().ok()?;
                let ino = fields.next()?.parse::<u64>().ok()?;
                if fields.next().is_none() && (dev, ino) == (expected.dev(), expected.ino()) {
                    Some(pid)
                } else {
                    None
                }
            });
        let pid = matches.next();
        if matches.next().is_some() {
            return Err(Rp0Error::IsolationFailed);
        }
        profile.verify()?;
        eprintln!("RP3_PROBE_STAGE=before_quota");
        let quota = launch.relay.local_quota_snapshot()?;
        // Dropping the relay stops its private CBM socket and cleans only this
        // attempt's cgroup. No provider/evidence root was ever installed.
        drop(launch.relay);
        let combined_limit = match self.resource_policy() {
            super::session::STRICT_POLICY => super::session::SESSION_LIMIT,
            super::session::COMBINED_1G_POLICY => super::session::COMBINED_1G_LIMIT,
            _ => return Err(Rp0Error::IsolationFailed),
        };
        Ok(LocalProbeResult {
            schema_version: 1,
            resource_policy: self.resource_policy().into(),
            combined_limit,
            host_custody_charge: launch.quota.host_charge,
            quota_page_size: launch.quota.page_size,
            program_sha256: profile.program().sha256().into(),
            command_sha256: command_digest(profile, session_id, true),
            profile_sha256: profile.profile_digest().into(),
            guard_ready: true,
            network_denied_before_ready: true,
            spawned: pid.is_some(),
            local_milestone_met: pid.is_some()
                && !timed_out
                && quota.oom_kill == 0
                && quota.swap_max_zero
                && quota.swap_current_zero
                && quota.memory_max == launch.quota.memory_max
                && quota.memory_peak <= quota.memory_max,
            selected_child_pid: pid,
            guard_exit_code: exit.and_then(|value| value.code()),
            timed_out,
            stdout_bytes: out.len(),
            stderr_bytes: ready.len() + err.len(),
            memory_max: quota.memory_max,
            memory_peak: quota.memory_peak,
            swap_max_zero: quota.swap_max_zero,
            swap_current_zero: quota.swap_current_zero,
            oom_kill: quota.oom_kill,
        })
    }
}
