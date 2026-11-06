//! Synthetic trusted operator inputs; successful preparation is never execution authority.
use serde_json::{Value, json};
use std::{fs, path::Path, process::Command};
use work_engine_manual_host::{
    digest,
    pi_docker::{Job, inventory},
};
fn pin(path: &Path) -> Value {
    json!({"path":path,"sha256":digest(&fs::read(path).unwrap())})
}
fn call(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_coding-job-prepare"))
        .args(args)
        .output()
        .unwrap()
}
#[test]
fn exact_preparation_preserves_inputs_and_cannot_create_provider_permission() {
    for sol in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let r = root.path();
        for d in [
            "runtime/node_modules/@earendil-works/pi-ai/dist/providers/data",
            "workspace/src",
            "toolchain",
            "config",
        ] {
            fs::create_dir_all(r.join(d)).unwrap();
        }
        fs::write(r.join("task"), b"Explicit operator task").unwrap();
        fs::write(r.join("workspace/src/lib.rs"), b"pub fn answer() {}\n").unwrap();
        fs::write(r.join("acceptance.rs"), b"// immutable acceptance").unwrap();
        let catalog = r.join(
            "runtime/node_modules/@earendil-works/pi-ai/dist/providers/data/openai-codex.json",
        );
        fs::write(&catalog,json!({"openai-codex-responses":{"chat:gpt-6.1-sol":{"id":"gpt-6.1-sol","provider":"openai-codex","api":"openai-codex-responses","baseUrl":"https://chatgpt.com/backend-api","contextWindow":500000,"maxTokens":128000,"cost":{"input":2,"output":10,"cacheRead":0.1,"cacheWrite":2.5,"tiers":[{"input":4,"output":15,"cacheRead":0.2,"cacheWrite":5}]}}}}).to_string()).unwrap();
        let tuple = json!({"schema":"work-engine.sol-high.v1","reasoning_effort":"high","payer_account_id":"fixture-payer"});
        let native = r.join("unopened-and-nonexistent-native-reference");
        let admission = r.join("existing-runtime-admission.json");
        fs::write(&admission,json!({"schema":"work-engine.pi-coding-runtime-admission.v2","job_id":"new-job","binary_sha256":digest(&fs::read("/usr/bin/gnutrue").unwrap()),"model":"gpt-6.1-sol","sol_high":tuple,"task_sha256":digest(b"Explicit operator task"),"catalog_sha256":digest(&fs::read(&catalog).unwrap()),"pi_config_manifest_sha256":digest(&serde_json::to_vec(&inventory(&r.join("config")).unwrap()).unwrap()),"native_auth_source":native,"max_requests":4,"budget_microusd":17_680_004u64,"allow_provider":true,"allow_native_access_projection":true}).to_string()).unwrap();
        let mut spec = json!({"schema":"work-engine.coding-preparation.v1","job_id":"new-job","output_dir":r.join("bundle"),"state_dir":r.join("state"),"operation_root":r.join("ops"),"host_binary":"/usr/bin/gnutrue","task":r.join("task"),"workspace":r.join("workspace"),"validator":r.join("acceptance.rs"),"runtime":r.join("runtime"),"toolchain":r.join("toolchain"),"docker":"/usr/bin/gnutrue","image":format!("node@sha256:{}","a".repeat(64)),"deadline_unix_seconds":123456,"command_timeout_seconds":10,"collector_deadline_seconds":60,"cancel_grace_seconds":3,"fixture_command":if sol{""}else{"true"}});
        if sol {
            spec["sol_high"] = json!({"pi_config":r.join("config"),"native_auth_source":native,"payer_account_id":"fixture-payer","runtime_admission":pin(&admission),"budget_control":{"authority":"observation_only"}});
        }
        let sp = r.join("spec.json");
        fs::write(&sp, serde_json::to_vec(&spec).unwrap()).unwrap();
        let inspected = call(&["inspect", sp.to_str().unwrap()]);
        assert!(
            inspected.status.success(),
            "{}",
            String::from_utf8_lossy(&inspected.stderr)
        );
        let inspected: Value = serde_json::from_slice(&inspected.stdout).unwrap();
        let mut auth = json!({"schema":"work-engine.coding-preparation-authority.v1","spec_sha256":inspected["spec_sha256"],"inputs_sha256":inspected["inputs_sha256"],"job_id":"new-job","authorization_id":"separate-authorization","caller":"operator","credential_sha256":"b".repeat(64),"allow_prepare":true,"allow_provider":true});
        let ap = r.join("authority.json");
        fs::write(&ap, auth.to_string()).unwrap();
        let denied = call(&["prepare", sp.to_str().unwrap(), ap.to_str().unwrap()]);
        assert!(!denied.status.success());
        assert!(!r.join("bundle").exists());
        auth["allow_provider"] = json!(false);
        fs::write(&ap, auth.to_string()).unwrap();
        let prepared = call(&["prepare", sp.to_str().unwrap(), ap.to_str().unwrap()]);
        assert!(
            prepared.status.success(),
            "{}",
            String::from_utf8_lossy(&prepared.stderr)
        );
        let result: Value = serde_json::from_slice(&prepared.stdout).unwrap();
        assert_eq!(result["provider_execution"], false);
        let job: Job =
            serde_json::from_slice(&fs::read(r.join("bundle/job.json")).unwrap()).unwrap();
        assert_eq!(
            result["host_input_digest"],
            digest(&serde_json::to_vec(&job).unwrap())
        );
        assert_eq!(job.coding.as_ref().unwrap().provider.is_some(), sol);
        assert!(!native.exists());
        if sol {
            assert_eq!(
                job.coding
                    .as_ref()
                    .unwrap()
                    .provider
                    .as_ref()
                    .unwrap()
                    .reservation_microusd()
                    .unwrap(),
                17_680_004
            );
        }
        let duplicate = call(&["prepare", sp.to_str().unwrap(), ap.to_str().unwrap()]);
        assert!(!duplicate.status.success());
    }
}
