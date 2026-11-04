//! Disposable qualification harness. Every native effect belongs to the existing Host.
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use work_engine_manual_host::{
    Artifact, digest,
    pi_docker::{
        Host, Job, Tree,
        coding::{PRESSURE_PROFILE, PressurePacket, SHIM},
        inventory, request_cancel,
    },
};
fn artifact(path: &Path, b: &[u8]) -> Artifact {
    fs::write(path, b).unwrap();
    Artifact {
        path: path.into(),
        sha256: digest(b),
    }
}
fn tree(root: PathBuf, path: &Path) -> Tree {
    Tree {
        manifest: artifact(
            path,
            &serde_json::to_vec(&inventory(&root).unwrap()).unwrap(),
        ),
        root,
    }
}
fn save(path: &Path, v: &Value) -> Result<(), Box<dyn std::error::Error>> {
    fs::write(path, serde_json::to_vec_pretty(v)?)?;
    Ok(())
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    match args.get(1).map(String::as_str){
 Some("prepare")=>{
  let started=Instant::now();let out=PathBuf::from(&args[2]);fs::create_dir(&out)?;
  let mut job:Job=serde_json::from_slice(&fs::read(&args[3])?)?;
  let input=out.join("input");fs::create_dir(&input)?;fs::create_dir(input.join("src"))?;fs::write(input.join("src/lib.rs"),"pub fn checkpoint()->bool{false}\n")?;
  let state=out.join("state");fs::create_dir(&state)?;
  job.job_id=format!("s10-pressure-{}",out.file_name().unwrap().to_string_lossy());job.state_dir=state;
  job.workspace=tree(input,&out.join("input.json"));let runtime=Instant::now();job.runtime=tree(job.runtime.root,&out.join("runtime.json"));let runtime_ms=runtime.elapsed().as_millis();
  job.authority=artifact(&out.join("authority.json"),b"Root continuation-20261104T2217/assignment.json; provider-disabled pressure fixture only");
  let budget_path=fs::canonicalize(&args[4])?;let budget=Artifact{sha256:digest(&fs::read(&budget_path)?),path:budget_path};
  let packet=PressurePacket{schema:PRESSURE_PROFILE.into(),source:"s10:pi-sdk".into(),generation:format!("{}:g1",job.job_id),budget,max_age_ms:30000};
  job.packet=artifact(&out.join("packet.json"),&serde_json::to_vec(&packet)?);
  let p=job.coding.as_mut().ok_or("coding profile required")?;p.shim=artifact(&out.join("pi-coding.mjs"),SHIM);
  let toolchain=Instant::now();p.toolchain=tree(p.toolchain.root.clone(),&out.join("toolchain.json"));let toolchain_ms=toolchain.elapsed().as_millis();
  p.acceptance=artifact(&out.join("acceptance.rs"),b"#[path=\"/workspace/src/lib.rs\"]mod x;#[test]fn checkpoint(){assert!(x::checkpoint());}\n");
  p.fixture_command="printf 'pub fn checkpoint()->bool{true}\\n' > src/lib.rs".into();
  job.deadline_unix_seconds=SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs()+240;job.command_timeout_seconds=30;
  save(&out.join("job.json"),&serde_json::to_value(job)?)?;
  save(&out.join("preparation.json"),&json!({"elapsed_ms":started.elapsed().as_millis(),"runtime_inventory_ms":runtime_ms,"toolchain_inventory_ms":toolchain_ms,"deadline_assigned_after_inventories":true}))?;
 }
 Some("run")=>{
  let path=PathBuf::from(&args[2]);let job:Job=serde_json::from_slice(&fs::read(&path)?)?;let dir=job.state_dir.clone();let out=path.parent().ok_or("job parent")?.to_path_buf();let observer:Artifact=serde_json::from_slice(&fs::read(&args[3])?)?;let mode=args.get(4).map(String::as_str);let cancel=mode==Some("--cancel-at-ready")||mode==Some("--cancel-and-interrupt-at-ready");let interrupt=mode==Some("--cancel-and-interrupt-at-ready");
  let packet:PressurePacket=serde_json::from_slice(&job.packet.verify()?)?;let budget=String::from_utf8(packet.budget.verify()?)?;
  let mut host=Host::admit(job)?;let done=Arc::new(AtomicBool::new(false));let signal=done.clone();let publisher_dir=dir.clone();
  let publisher=std::thread::spawn(move||->Result<(),String>{
   let until=Instant::now()+Duration::from_secs(180);let ready=publisher_dir.join("coding-pressure-ready.json");
   while !ready.exists(){if signal.load(Ordering::Acquire)||Instant::now()>=until{return Err("no ready; publisher stopped".into());}std::thread::sleep(Duration::from_millis(10));}
   if cancel {request_cancel(&publisher_dir).map_err(|e|e.to_string())?;if interrupt {std::process::exit(75);}return Ok(());}
   let barrier:Value=serde_json::from_slice(&fs::read(ready).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
   let release=pi_runtime_pressure::compose(&packet,&barrier,&budget,&observer)?;
   fs::write(publisher_dir.join("release.tmp"),serde_json::to_vec(&release).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
   fs::rename(publisher_dir.join("release.tmp"),publisher_dir.join("coding-pressure-release.json")).map_err(|e|e.to_string())?;Ok(())
  });
  let result=host.run();done.store(true,Ordering::Release);let published=publisher.join().map_err(|_|"publisher panic")?;
  save(&out.join("publisher.json"),&json!({"result":published,"cancel_at_ready":cancel}))?;
  let value=result?;save(&out.join("result.json"),&value)?;
  let ack=host.acknowledge(value["receipt_id"].as_str().ok_or("receipt missing")?)?;save(&out.join("ack.json"),&ack)?;
  if !cancel&&value["receipt"]["status"]!="component_passed" {return Err(format!("qualification held: {}",value["receipt"]).into());}
  if cancel&&value["receipt"]["status"]=="component_passed" {return Err("cancelled run claimed success".into());}
 }
 Some("recover")=>{
  let dir=PathBuf::from(&args[2]);let mut host=Host::open(&dir)?;let before=host.journal().processes.len();let value=host.reconcile()?;
  save(&dir.join("same-binary-recovery.json"),&json!({"before_process_count":before,"after_process_count":host.journal().processes.len(),"result":value}))?;
  println!("{}",serde_json::to_string(&value)?);
 }
 Some("cancel")=>{println!("{}",request_cancel(Path::new(&args[2]))?);}
 Some("inventory")=>{let entries=inventory(Path::new(&args[2]))?;save(Path::new(&args[3]),&serde_json::to_value(entries)?)?;}
 _=>return Err("prepare OUT TEMPLATE BUDGET | run JOB OBSERVER_PIN [--cancel-at-ready] | recover STATE | cancel STATE | inventory ROOT OUT".into())
 }
    Ok(())
}
