//! Separate-process readback probe for the controlled evidence tests.
use review_execution_evidence::{
    EvidenceReader, ExecutionEvidenceOwner, ExecutionEvidenceRef, ReaderPin,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() != 10 {
        return Err("expected root, pin and reference fields".into());
    }
    let pin = ReaderPin {
        root_id: args[1].clone(),
        configuration_sha256: args[2].clone(),
        executable_sha256: args[3].clone(),
        source_sha256: args[4].clone(),
        profile: args[5].clone(),
    };
    let reference = ExecutionEvidenceRef {
        schema_version: 1,
        owner: "review-execution-evidence".into(),
        root_id: args[1].clone(),
        profile: args[5].clone(),
        attempt_id: args[6].clone(),
        record_id: args[7].clone(),
        revision: args[8].clone(),
        sha256: args[9].clone(),
    };
    let reader = EvidenceReader::open(std::path::Path::new(&args[0]), pin)?;
    let checked = reader.read_result(&reference)?;
    println!("{}", checked.claim_sha256());
    Ok(())
}
