//! Natural-exit image identity join: recipe-pinned serialized metadata and
//! the serving Docker's exact local inspect readback. Readback is not yet
//! the task's materialization; Create, OCI and Held must retain the join.

use super::super::{Startup, Store, artifact_contract::ENTRYPOINT};
use super::{Result, full_hex, json, refuse};
use serde::Serialize;
use serde_json::Value;

const MAX_META: usize = 32 * 1024;
const MAX_LAYERS: usize = 16;

fn sha256(s: &str) -> bool {
    s.strip_prefix("sha256:").is_some_and(full_hex)
}

fn no_list(v: &Value) -> bool {
    v.is_null() || v.as_array().is_some_and(Vec::is_empty)
}

fn no_map(v: &Value) -> bool {
    v.is_null() || v.as_object().is_some_and(serde_json::Map::is_empty)
}

fn no_string(v: &Value) -> bool {
    v.is_null() || v.as_str() == Some("")
}

fn closed_config(config: &Value) -> bool {
    config.is_object()
        && ["Volumes", "ExposedPorts", "Labels"]
            .iter()
            .all(|key| no_map(&config[*key]))
        && ["Env", "Shell", "OnBuild", "Cmd"]
            .iter()
            .all(|key| no_list(&config[*key]))
        && ["Healthcheck", "StopTimeout"]
            .iter()
            .all(|key| config[*key].is_null())
        && (no_list(&config["Entrypoint"]) || config["Entrypoint"] == json!([ENTRYPOINT]))
        && (no_string(&config["WorkingDir"]) || config["WorkingDir"] == "/")
        && (no_string(&config["User"]) || config["User"] == "65534:65534")
        && no_string(&config["StopSignal"])
        && (config["ArgsEscaped"].is_null() || config["ArgsEscaped"] == false)
}

#[derive(Serialize)]
pub(super) struct ImageMetadata {
    pub(super) image: Startup,
    diff_ids: Vec<String>,
    // Preserve consumed bytes for the exact two independently pinned digests.
    manifest: Vec<u8>,
    config: Vec<u8>,
}

impl ImageMetadata {
    pub(super) fn read(image: &Startup, manifest: Vec<u8>, config: Vec<u8>) -> Result<Self> {
        if manifest.len() > MAX_META
            || config.len() > MAX_META
            || crate::digest(&manifest) != image.manifest_sha256
            || crate::digest(&config) != image.config_sha256
            || image.image_id
                != format!(
                    "sha256:{}",
                    match image.store {
                        Store::DockerImageConfig => &image.config_sha256,
                        Store::ContainerdLocalTarget => &image.manifest_sha256,
                    }
                )
        {
            return refuse("serialized image metadata does not match Campaign image pins");
        }
        let m: Value = serde_json::from_slice(&manifest)?;
        let c: Value = serde_json::from_slice(&config)?;
        let config_descriptor = &m["config"];
        if m["schemaVersion"] != 2
            || !matches!(
                m["mediaType"].as_str(),
                Some(
                    "application/vnd.oci.image.manifest.v1+json"
                        | "application/vnd.docker.distribution.manifest.v2+json"
                )
            )
            || !m["manifests"].is_null()
            || !m["subject"].is_null()
            || !m["artifactType"].is_null()
            || !matches!(
                config_descriptor["mediaType"].as_str(),
                Some(
                    "application/vnd.oci.image.config.v1+json"
                        | "application/vnd.docker.container.image.v1+json"
                )
            )
            || config_descriptor["digest"] != format!("sha256:{}", image.config_sha256)
            || config_descriptor["size"].as_u64() != Some(config.len() as u64)
            || !no_list(&config_descriptor["urls"])
            || !config_descriptor["data"].is_null()
            || c["os"] != "linux"
            || c["architecture"] != "amd64"
            || !no_string(&c["variant"])
            || c["rootfs"]["type"] != "layers"
            || !closed_config(&c["config"])
        {
            return refuse("expected exact single-platform, closed image manifest and config");
        }
        let (Some(layers), Some(ids)) =
            (m["layers"].as_array(), c["rootfs"]["diff_ids"].as_array())
        else {
            return refuse("image layer closure is unavailable");
        };
        if !(1..=MAX_LAYERS).contains(&layers.len())
            || layers.len() != ids.len()
            || layers.iter().any(|layer| {
                !matches!(
                    layer["mediaType"].as_str(),
                    Some(
                        "application/vnd.oci.image.layer.v1.tar"
                            | "application/vnd.oci.image.layer.v1.tar+gzip"
                            | "application/vnd.docker.image.rootfs.diff.tar.gzip"
                    )
                ) || !layer["digest"].as_str().is_some_and(sha256)
                    || !layer["size"].as_u64().is_some_and(|size| size != 0)
                    || !no_list(&layer["urls"])
                    || !layer["data"].is_null()
            })
        {
            return refuse("image requires selected local, complete rootfs layers");
        }
        let diff_ids = ids
            .iter()
            .map(|id| id.as_str().filter(|id| sha256(id)).map(str::to_owned))
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| super::invalid("malformed pinned rootfs diff identifiers"))?;
        Ok(Self {
            image: image.clone(),
            diff_ids,
            manifest,
            config,
        })
    }

    pub(super) fn check_local_inspect(&self, bytes: &[u8]) -> Result<()> {
        if bytes.len() > MAX_META {
            return refuse("selected local image inspect exceeds its bound");
        }
        let v: Value = serde_json::from_slice(bytes)?;
        if v["Id"] != self.image.image_id
            || v["Os"] != "linux"
            || v["Architecture"] != "amd64"
            || !no_string(&v["Variant"])
            || v["RootFS"]["Type"] != "layers"
            || v["RootFS"]["Layers"] != json!(self.diff_ids)
            || !closed_config(&v["Config"])
        {
            return refuse("local image identity, rootfs layers or startup defaults disagree");
        }
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn natural_image_requires_both_exact_metadata_pins_and_local_resolved_root() {
    let merkle = format!("sha256:{}", "9".repeat(64));
    let config = serde_json::to_vec(&json!({"os":"linux", "architecture":"amd64", "config":{}, "rootfs":{"type":"layers", "diff_ids":[merkle]}})).unwrap();
    let mut image = crate::pi_docker::init_only_ready::tests::recipe().startup;
    image.store = Store::ContainerdLocalTarget;
    image.config_sha256 = crate::digest(&config);
    let manifest = serde_json::to_vec(&json!({
        "schemaVersion":2,"mediaType":"application/vnd.oci.image.manifest.v1+json",
        "config":{"digest":format!("sha256:{}", image.config_sha256),"size":config.len(),"mediaType":"application/vnd.oci.image.config.v1+json"},
        "layers":[{"digest":format!("sha256:{}", "8".repeat(64)),"size":420,"mediaType":"application/vnd.oci.image.layer.v1.tar"}]
    })).unwrap();
    image.manifest_sha256 = crate::digest(&manifest);
    image.image_id = format!("sha256:{}", image.manifest_sha256);
    let pin = ImageMetadata::read(&image, manifest.clone(), config.clone()).unwrap();
    assert!(ImageMetadata::read(&image, manifest.clone(), b"{}".to_vec()).is_err());
    assert!(ImageMetadata::read(&image, b"{}".to_vec(), config.clone()).is_err());
    let local = json!({"Id":image.image_id,"Os":"linux","Architecture":"amd64","RootFS":{"Type":"layers","Layers":[merkle]},"Config":{"Env":null,"Volumes":{}}});
    assert!(
        pin.check_local_inspect(&local.to_string().into_bytes())
            .is_ok()
    );
    for (slot, value) in [
        ("/Id", json!(format!("sha256:{}", "a".repeat(64)))),
        ("/Architecture", json!("arm64")),
        ("/RootFS/Layers", json!([])),
        ("/Config/Env", json!(["PATH=/other"])),
        ("/Config/Volumes", json!({"/host":{}})),
    ] {
        let mut crossed = local.clone();
        *crossed.pointer_mut(slot).unwrap() = value;
        assert!(
            pin.check_local_inspect(crossed.to_string().as_bytes())
                .is_err(),
            "{slot}"
        );
    }
}
