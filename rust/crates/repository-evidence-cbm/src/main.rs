use repository_evidence_cbm::{
    Binding, CaptureRequest, capture, parse_json, publish_artifact, read_artifact, source_currency,
};
use serde::de::DeserializeOwned;
use serde_json::json;
use std::{
    collections::BTreeMap,
    io::Read,
    path::{Path, PathBuf},
};

fn input<T: DeserializeOwned>(path: &Path) -> Result<T, Box<dyn std::error::Error>> {
    let file = std::fs::File::open(path)?;
    let mut bytes = Vec::new();
    file.take(1024 * 1024 + 1).read_to_end(&mut bytes)?;
    if bytes.len() > 1024 * 1024 {
        return Err("input exceeds capacity".into());
    }
    Ok(serde_json::from_value(parse_json(&bytes)?)?)
}
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let action = args.next().ok_or("missing action")?;
    let mut flags = BTreeMap::new();
    while let Some(key) = args.next() {
        let value = args.next().ok_or("missing flag value")?;
        if !["--binding", "--request", "--artifacts", "--sha256"].contains(&key.as_str())
            || flags.insert(key, value).is_some()
        {
            return Err("unknown or duplicate flag".into());
        }
    }
    let binding: Binding = input(Path::new(flags.get("--binding").ok_or("missing binding")?))?;
    let root = PathBuf::from(flags.get("--artifacts").ok_or("missing artifact root")?);
    let value=match action.as_str() {
        "capture" if flags.len()==3=>{
            let request:CaptureRequest=input(Path::new(flags.get("--request").ok_or("missing request")?))?;
            let artifact=capture(&binding,&request)?;publish_artifact(&root,&artifact)?;
            json!({"schema_version":1,"reference":artifact.reference(),"documentation":artifact.capture().documentation_status()?,"limitations":artifact.capture().limitations()})
        }
        "read" | "currency" if flags.len()==3=>{
            let digest=flags.get("--sha256").ok_or("missing capture digest")?;
            let artifact=read_artifact(&root,digest,&binding.repository_id)?;
            if action=="read" {json!({"schema_version":1,"reference":artifact.reference(),"capture":artifact.capture()})}
            else {source_currency(&artifact,&binding.repository)?}
        }
        _=>return Err("usage: repository-evidence-cbm capture --binding FILE --request FILE --artifacts ROOT | read|currency --binding FILE --sha256 SHA --artifacts ROOT".into()),
    };
    println!("{}", serde_json::to_string(&value)?);
    Ok(())
}
fn main() {
    if let Err(error) = run() {
        eprintln!("repository-evidence-cbm: {error}");
        std::process::exit(1);
    }
}
