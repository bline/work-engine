//! Stable acceptance: stop all model writers, snapshot, validate read-only, export
//! that same identity. Recovery only collects; it never reruns acceptance.
use super::*;
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Proof {
    pub process_index: usize,
    pub stdout_sha256: String,
    pub acceptance_sha256: String,
    pub workspace_manifest_sha256: String,
    pub validator_id: String,
}
pub(super) fn name(j: &Journal) -> String {
    format!("{}-validation", j.container_name)
}
pub(super) fn command() -> Vec<String> {
    strings(&[
        "-c",
        "set -eu; rustc --edition=2024 --test /acceptance.rs -o /tmp/final-acceptance; /tmp/final-acceptance",
    ])
}
fn boundary(before: &BTreeMap<String, Entry>, after: &BTreeMap<String, Entry>) -> Result<()> {
    let changes: BTreeSet<_> = before
        .keys()
        .chain(after.keys())
        .filter(|key| before.get(*key) != after.get(*key))
        .cloned()
        .collect();
    if changes != BTreeSet::from(["src/lib.rs".into()])
        || after
            .values()
            .any(|entry| !matches!(entry, Entry::File { .. }))
    {
        return refuse("final exported coding edits exceed exact src/lib.rs boundary");
    }
    Ok(())
}
impl Host {
    fn coding_inspect_validation(&mut self) -> Result<Value> {
        let (ok, bytes) = self.command(
            "coding-validation-inspect",
            vec!["inspect".into(), name(&self.journal)],
            false,
        )?;
        if !ok {
            return refuse("validator inspect unavailable");
        }
        let rows: Vec<Value> = serde_json::from_slice(&bytes)?;
        if rows.len() != 1 {
            return refuse("validator identity ambiguous");
        }
        verify(&self.journal, &rows[0], true, true)?;
        self.journal.coding.as_mut().unwrap().validation_id =
            rows[0]["Id"].as_str().map(String::from);
        self.save()?;
        Ok(rows[0].clone())
    }
    pub(in crate::pi_docker) fn coding_validate_task(&mut self) -> Result<()> {
        self.coding_collect_tool()?; // removal and fresh absence exclude every model-controlled writer
        if !self.journal.coding.as_ref().unwrap().tool_removed {
            return refuse("model writers not excluded");
        }
        let workspace = self.journal.job.state_dir.join("coding-workspace");
        let entries = inventory(&workspace)?;
        boundary(&inventory(&self.journal.job.workspace.root)?, &entries)?;
        let snapshot = self.journal.job.state_dir.join("validation-workspace");
        fs::create_dir(&snapshot)?;
        for (path, entry) in &entries {
            let Entry::File { sha256 } = entry else {
                return refuse("validation snapshot symlink");
            };
            let target = snapshot.join(path);
            fs::create_dir_all(target.parent().unwrap())?;
            let source = Artifact {
                path: workspace.join(path),
                sha256: sha256.clone(),
            };
            let bytes = source.verify()?;
            fs::write(target, bytes)?;
        }
        let manifest = self.journal.job.state_dir.join("validation-manifest.json");
        atomic(&manifest, &entries)?;
        let tree = Tree {
            root: snapshot,
            manifest: Artifact {
                sha256: file_digest(&manifest, None)?,
                path: manifest,
            },
        };
        tree.verify()?;
        self.journal.coding.as_mut().unwrap().validation_snapshot = Some(tree);
        self.save()?;
        let mut args = base(&self.journal, &name(&self.journal));
        for (src, dst, rw) in expected_mounts(&self.journal, true, true) {
            mount(&mut args, &src, dst, rw);
        }
        args.extend(strings(&["--entrypoint", "/bin/sh"]));
        args.push(TOOL_IMAGE.into());
        args.extend(command());
        self.journal.coding.as_mut().unwrap().validation_intent = true;
        self.save()?;
        let (ok, _) = self.command("coding-validation-create", args, false)?;
        if !ok {
            return refuse("validator create unknown; no replay");
        }
        self.coding_inspect_validation()?;
        if self.expired() || self.cancelled() {
            return refuse("validation expired/cancelled");
        }
        let index = self.journal.processes.len();
        let (ok, _) = self.command(
            "coding-validation-start",
            vec!["start".into(), "--attach".into(), name(&self.journal)],
            false,
        )?;
        if !ok {
            return refuse("original pinned coding acceptance failed or unknown");
        }
        self.coding_collect_validation()?;
        let state = self.journal.coding.as_ref().unwrap();
        let tree = state.validation_snapshot.as_ref().unwrap();
        tree.verify()?;
        if !state.validation_removed || state.validation_exit != Some(0) {
            return refuse("validator completion unavailable");
        }
        let process = &self.journal.processes[index];
        let proof = Proof {
            process_index: index,
            stdout_sha256: process
                .stdout_sha256
                .clone()
                .ok_or_else(|| Error::Refused("validation stdout absent".into()))?,
            acceptance_sha256: self
                .journal
                .job
                .coding
                .as_ref()
                .unwrap()
                .acceptance
                .sha256
                .clone(),
            workspace_manifest_sha256: tree.manifest.sha256.clone(),
            validator_id: state.validation_id.clone().unwrap(),
        };
        self.journal.coding.as_mut().unwrap().final_acceptance = Some(proof);
        self.save()
    }
    pub(in crate::pi_docker) fn coding_collect_validation(&mut self) -> Result<()> {
        let state = self.journal.coding.as_ref().unwrap();
        if !state.validation_intent || state.validation_removed {
            return Ok(());
        }
        let name = name(&self.journal);
        let mut v = match self.coding_inspect_validation() {
            Ok(v) => v,
            Err(error) => {
                let (ok, out) = self.command(
                    "coding-validation-absence",
                    vec![
                        "ps".into(),
                        "--all".into(),
                        "--no-trunc".into(),
                        "--filter".into(),
                        format!("name=^/{name}$"),
                        "--format".into(),
                        "{{.ID}}".into(),
                    ],
                    false,
                )?;
                if ok && out.iter().all(u8::is_ascii_whitespace) {
                    self.journal.coding.as_mut().unwrap().validation_removed = true;
                    self.save()?;
                    return Ok(());
                }
                return Err(error);
            }
        };
        if v["State"]["Running"] == true {
            let (ok, _) = self.command(
                "coding-validation-stop",
                vec!["stop".into(), "--time".into(), "1".into(), name.clone()],
                false,
            )?;
            if !ok {
                return refuse("validator stop unknown");
            }
            v = self.coding_inspect_validation()?;
        }
        if v["State"]["Running"] != false || v["State"]["OOMKilled"] != false {
            return refuse("validator terminal state unavailable");
        }
        if v["State"]["Status"] == "exited" {
            let (ok, out) = self.command(
                "coding-validation-wait",
                vec!["wait".into(), name.clone()],
                false,
            )?;
            let exit = String::from_utf8_lossy(&out)
                .trim()
                .parse::<i64>()
                .map_err(|_| Error::Refused("validator exit invalid".into()))?;
            if !ok || v["State"]["ExitCode"] != exit {
                return refuse("validator wait/inspect mismatch");
            }
            self.journal.coding.as_mut().unwrap().validation_exit = Some(exit);
            self.save()?;
        } else if v["State"]["Status"] != "created" {
            return refuse("validator state invalid");
        }
        let (ok, _) = self.command(
            "coding-validation-remove",
            vec!["rm".into(), name.clone()],
            false,
        )?;
        if !ok {
            return refuse("validator remove unknown");
        }
        let (ok, out) = self.command(
            "coding-validation-absence",
            vec![
                "ps".into(),
                "--all".into(),
                "--no-trunc".into(),
                "--filter".into(),
                format!("name=^/{name}$"),
                "--format".into(),
                "{{.ID}}".into(),
            ],
            false,
        )?;
        if !ok || !out.iter().all(u8::is_ascii_whitespace) {
            return refuse("validator absence unestablished");
        }
        self.journal.coding.as_mut().unwrap().validation_removed = true;
        self.save()
    }
    pub(in crate::pi_docker) fn coding_export_validation(&mut self) -> Result<()> {
        let state = self.journal.coding.as_ref().unwrap();
        if state.final_acceptance.is_none() {
            return Ok(());
        }
        let tree = state
            .validation_snapshot
            .clone()
            .ok_or_else(|| Error::Refused("tested snapshot absent".into()))?;
        tree.verify()?;
        let entries = inventory(&tree.root)?;
        boundary(&inventory(&self.journal.job.workspace.root)?, &entries)?;
        for (path, entry) in entries {
            let Entry::File { sha256 } = entry else {
                return refuse("validated export symlink");
            };
            let artifact = Artifact {
                path: tree.root.join(&path),
                sha256,
            };
            let pin = Artifact {
                path: artifact.snapshot(&self.journal.job.state_dir.join("snapshots"))?,
                sha256: artifact.sha256,
            };
            self.journal
                .exported_files
                .insert(format!("workspace/{path}"), pin.clone());
            if !self.journal.artifacts.iter().any(|a| a.path == pin.path) {
                self.journal.artifacts.push(pin);
            }
        }
        self.save()
    }
    pub(in crate::pi_docker) fn coding_final_proof_valid(&self) -> bool {
        let Some(state) = &self.journal.coding else {
            return true;
        };
        let Some(proof) = &state.final_acceptance else {
            return false;
        };
        let Some(tree) = &state.validation_snapshot else {
            return false;
        };
        let Some(process) = self.journal.processes.get(proof.process_index) else {
            return false;
        };
        if !state.tool_removed
            || !state.validation_removed
            || state.validation_exit != Some(0)
            || state.validation_id.as_deref() != Some(&proof.validator_id)
            || proof.acceptance_sha256
                != self.journal.job.coding.as_ref().unwrap().acceptance.sha256
            || self
                .journal
                .job
                .coding
                .as_ref()
                .unwrap()
                .acceptance
                .verify()
                .is_err()
            || proof.workspace_manifest_sha256 != tree.manifest.sha256
            || tree.verify().is_err()
            || process.status != Some(0)
            || process.timed_out
            || !process.leftovers.is_empty()
            || process.stdout_sha256.as_deref() != Some(&proof.stdout_sha256)
            || file_digest(&process.stdout, None).ok().as_deref() != Some(&proof.stdout_sha256)
        {
            return false;
        }
        let Ok(entries) = inventory(&tree.root) else {
            return false;
        };
        let Ok(before) = inventory(&self.journal.job.workspace.root) else {
            return false;
        };
        if boundary(&before, &entries).is_err() {
            return false;
        }
        let exports: BTreeMap<_, _> = self
            .journal
            .exported_files
            .iter()
            .filter_map(|(key, value)| {
                key.strip_prefix("workspace/")
                    .map(|p| (p.to_string(), value))
            })
            .collect();
        exports.len() == entries.len()
            && entries.iter().all(|(path, entry)| match entry {
                Entry::File { sha256 } => exports
                    .get(path)
                    .is_some_and(|a| a.sha256 == *sha256 && a.verify().is_ok()),
                _ => false,
            })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn missing_acceptance_after_rpc_never_succeeds_on_cleanup_only_recovery() {
        let (dir, mut h) = super::super::tests::fixture();
        let output = dir.path().join("rpc.stdout");
        fs::write(&output, b"rpc proof").unwrap();
        let hash = digest(b"rpc proof");
        h.journal.session_id = Some("native".into());
        h.journal.rpc_verified = true;
        h.journal.rpc_completion = Some(super::super::super::RpcCompletion {
            process_index: 0,
            stdout_sha256: hash.clone(),
            session_id: "native".into(),
        });
        h.journal.processes.push(ProcessRecord {
            argv: vec!["already completed".into()],
            pid: None,
            start: None,
            status: Some(0),
            stdout: output.clone(),
            stderr: output.clone(),
            stdout_sha256: Some(hash.clone()),
            stderr_sha256: Some(hash.clone()),
            timed_out: false,
            leftovers: vec![],
        });
        h.journal.artifacts.push(Artifact {
            path: output,
            sha256: hash,
        });
        h.journal.daemon_exit = Some(0);
        h.journal.removed = true;
        h.journal.phase = Phase::Collected;
        h.journal.coding.as_mut().unwrap().completed = true;
        h.journal.coding.as_mut().unwrap().tool_removed = true;
        fs::create_dir(dir.path().join("receipts")).unwrap();
        assert!(!h.coding_final_proof_valid());
        let before = h.journal.processes.len();
        let result = h.reconcile().unwrap();
        assert_eq!(result["receipt"]["status"], "not_established");
        assert_eq!(h.journal.processes.len(), before);
    }
    #[test]
    fn final_export_boundary_rejects_test_time_added_files() {
        let before = BTreeMap::from([(
            "src/lib.rs".into(),
            Entry::File {
                sha256: "old".into(),
            },
        )]);
        let mut after = BTreeMap::from([(
            "src/lib.rs".into(),
            Entry::File {
                sha256: "new".into(),
            },
        )]);
        assert!(boundary(&before, &after).is_ok());
        after.insert(
            "injected.txt".into(),
            Entry::File {
                sha256: "injected".into(),
            },
        );
        assert!(boundary(&before, &after).is_err());
    }
}
