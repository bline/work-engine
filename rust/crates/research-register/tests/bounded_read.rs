mod support;
use gix::{
    hash::{Kind, ObjectId},
    objs,
};
use research_register::{GitObjects, bounded_read::ReadProfile};
use std::io::Write;
use support::Fx;
fn compressed(bytes: &[u8]) -> Vec<u8> {
    let mut z =
        gix::zlib::stream::deflate::Write::new(Vec::new(), gix::zlib::Compression::default());
    z.write_all(bytes).unwrap();
    z.flush().unwrap();
    z.into_inner()
}
fn header(kind: u8, mut n: usize) -> Vec<u8> {
    let mut v = vec![(kind << 4) | (n as u8 & 15)];
    n >>= 4;
    if n != 0 {
        v[0] |= 128;
    }
    while n != 0 {
        let mut b = n as u8 & 127;
        n >>= 7;
        if n != 0 {
            b |= 128;
        }
        v.push(b);
    }
    v
}
fn varint(mut n: usize) -> Vec<u8> {
    let mut v = Vec::new();
    loop {
        let mut b = n as u8 & 127;
        n >>= 7;
        if n != 0 {
            b |= 128;
        }
        v.push(b);
        if n == 0 {
            return v;
        }
    }
}
fn fixture(ref_delta: bool) -> (Fx, std::path::PathBuf, ObjectId, ObjectId) {
    let fx = Fx::new(if ref_delta { "ref-delta" } else { "ofs-delta" });
    let repo = fx.root.join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    fx.git(&repo, &["init", "-q"]);
    let base = vec![b'a'; 2000];
    let base_id = objs::compute_hash(Kind::Sha1, objs::Kind::Blob, &base).unwrap();
    let target_id = objs::compute_hash(Kind::Sha1, objs::Kind::Blob, b"x").unwrap();
    let mut pack = b"PACK\0\0\0\x02\0\0\0\x03".to_vec();
    let start = pack.len();
    pack.extend(header(3, base.len()));
    pack.extend(compressed(&base));
    let mut delta = varint(base.len());
    delta.extend(varint(1));
    delta.extend([1, b'x']);
    let distance = pack.len() - start;
    pack.extend(header(if ref_delta { 7 } else { 6 }, delta.len()));
    if ref_delta {
        pack.extend_from_slice(base_id.as_slice());
    } else {
        let mut d = distance;
        let mut encoded = vec![(d & 127) as u8];
        while {
            d >>= 7;
            d != 0
        } {
            d -= 1;
            encoded.push(((d & 127) as u8) | 128);
        }
        encoded.reverse();
        pack.extend(encoded);
    }
    pack.extend(compressed(&delta));
    pack.extend(header(3, 1));
    pack.extend(compressed(b"y"));
    let mut h = gix::hash::hasher(Kind::Sha1);
    h.update(&pack);
    pack.extend_from_slice(h.try_finalize().unwrap().as_slice());
    fx.git_with(&repo, &["index-pack", "--stdin"], Some(&pack), &[]);
    (fx, repo, base_id, target_id)
}
#[test]
fn ofs_and_ref_base_admission_immutable_sources_and_isolated_limits() {
    for ref_delta in [false, true] {
        let (_fx, repo, base, target) = fixture(ref_delta);
        let reader = GitObjects::open_bounded(&repo, ReadProfile::default()).unwrap();
        assert_eq!(reader.blob(target).unwrap(), b"x");
        assert_eq!(reader.blob(base).unwrap(), vec![b'a'; 2000]);
        let p = ReadProfile {
            text_bytes: 1000,
            ..ReadProfile::default()
        };
        let reader = GitObjects::open_bounded(&repo, p).unwrap();
        let e = reader.blob(target).unwrap_err().to_string();
        assert!(
            e.contains("declared object size"),
            "oversized discovered base: {e}"
        );
        assert_eq!(
            GitObjects::open_bounded(&repo, ReadProfile::default())
                .unwrap()
                .blob(target)
                .unwrap(),
            b"x"
        );
        let p = ReadProfile {
            payload_bytes: 2000,
            ..ReadProfile::default()
        };
        assert!(
            GitObjects::open_bounded(&repo, p)
                .unwrap()
                .blob(target)
                .unwrap_err()
                .to_string()
                .contains("aggregate")
        );
        assert_eq!(reader.path(), repo.display().to_string());
        let reader = GitObjects::open_bounded(&repo, ReadProfile::default()).unwrap();
        assert_eq!(reader.blob(target).unwrap(), b"x");
        for p in std::fs::read_dir(repo.join(".git/objects/pack")).unwrap() {
            let p = p.unwrap().path();
            if p.extension().is_some_and(|e| e == "pack") {
                std::fs::remove_file(&p).unwrap();
                std::fs::write(p, b"replaced source").unwrap();
            }
        }
        assert_eq!(reader.blob(base).unwrap(), vec![b'a'; 2000]);
        assert_eq!(reader.blob(target).unwrap(), b"x");
        let other = objs::compute_hash(Kind::Sha1, objs::Kind::Blob, b"y").unwrap();
        assert_eq!(reader.blob(other).unwrap(), b"y");
    }
}

#[test]
fn exact_payload_boundary_control_and_next_distinct_object_refusal() {
    let fx = Fx::new("eight-mib");
    let repo = fx.root.join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    fx.git(&repo, &["init", "-q"]);
    let mut ids = Vec::new();
    for n in 0..9 {
        let mut bytes = vec![b'a'; 1 << 20];
        bytes[0] = b'A' + n;
        let id = fx.git_with(&repo, &["hash-object", "-w", "--stdin"], Some(&bytes), &[]);
        ids.push(ObjectId::from_hex(id.as_bytes()).unwrap());
    }
    let reader = GitObjects::open_bounded(&repo, ReadProfile::default()).unwrap();
    for id in &ids[..8] {
        assert_eq!(reader.blob(*id).unwrap().len(), 1 << 20);
    }
    assert_eq!(
        reader.read_accounting().unwrap()["distinct_payload_bytes"],
        8 << 20
    );
    assert!(
        reader
            .blob(ids[8])
            .unwrap_err()
            .to_string()
            .contains("aggregate")
    );
}
#[test]
fn actual_repository_bounded_eleven_decision_control() {
    use research_register::{Outcome, PinnedSnapshot, anchors, validate};
    let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let at = GitObjects::open(&repo)
        .unwrap()
        .resolve("HEAD")
        .unwrap()
        .to_string();
    let snapshot = PinnedSnapshot::build(
        GitObjects::open_bounded(&repo, ReadProfile::default()).unwrap(),
        &at,
        "metrics/register",
    )
    .unwrap();
    let Outcome::Valid(valid) = validate(&snapshot).unwrap() else {
        panic!("current register invalid")
    };
    let report = anchors(&valid, &snapshot).unwrap();
    assert_eq!(report.entries.len(), 11);
    println!(
        "bounded actual repository subject {at} accounting {}",
        snapshot.objects().read_accounting().unwrap()
    );
}

fn append_ofs(pack: &mut Vec<u8>, base_offset: usize, base_len: usize, target: u8) -> usize {
    let offset = pack.len();
    let mut delta = varint(base_len);
    delta.extend(varint(1));
    delta.extend([1, target]);
    pack.extend(header(6, delta.len()));
    let mut d = offset - base_offset;
    let mut encoded = vec![(d & 127) as u8];
    while {
        d >>= 7;
        d != 0
    } {
        d -= 1;
        encoded.push(((d & 127) as u8) | 128);
    }
    encoded.reverse();
    pack.extend(encoded);
    pack.extend(compressed(&delta));
    offset
}
fn install_pack(fx: &Fx, repo: &std::path::Path, mut pack: Vec<u8>, name: &str) {
    let mut h = gix::hash::hasher(Kind::Sha1);
    h.update(&pack);
    let hash = h.try_finalize().unwrap();
    pack.extend_from_slice(hash.as_slice());
    fx.git_with(repo, &["index-pack", "--stdin"], Some(&pack), &[]);
    let dir = repo.join(".git/objects/pack");
    for ext in ["pack", "idx"] {
        std::fs::rename(
            dir.join(format!("pack-{hash}.{ext}")),
            dir.join(format!("{name}.{ext}")),
        )
        .unwrap();
    }
}
#[test]
fn duplicate_oid_cannot_hide_physical_ofs_chain() {
    for size in [2 << 20, 2000] {
        let fx = Fx::new(&format!("duplicate-ofs-{size}"));
        let repo = fx.root.join("repo");
        std::fs::create_dir_all(&repo).unwrap();
        fx.git(&repo, &["init", "-q"]);
        let mut a = b"PACK\0\0\0\x02\0\0\0\x01".to_vec();
        a.extend(header(3, 1));
        a.extend(compressed(b"b"));
        install_pack(&fx, &repo, a, "a");
        let mut b = b"PACK\0\0\0\x02\0\0\0\x03".to_vec();
        let c = b.len();
        b.extend(header(3, size));
        b.extend(compressed(&vec![b'c'; size]));
        let middle = append_ofs(&mut b, c, size, b'b');
        append_ofs(&mut b, middle, 1, b't');
        install_pack(&fx, &repo, b, "z");
        let middle_id = objs::compute_hash(Kind::Sha1, objs::Kind::Blob, b"b").unwrap();
        let target = objs::compute_hash(Kind::Sha1, objs::Kind::Blob, b"t").unwrap();
        let reader = GitObjects::open_bounded(&repo, ReadProfile::default()).unwrap();
        assert_eq!(reader.blob(middle_id).unwrap(), b"b"); // prime earlier-pack OID cache
        if size > 1 << 20 {
            assert!(
                reader
                    .blob(target)
                    .unwrap_err()
                    .to_string()
                    .contains("declared object size")
            );
            assert_eq!(
                reader.read_accounting().unwrap()["distinct_payload_bytes"],
                1
            );
        } else {
            assert_eq!(reader.blob(target).unwrap(), b"t");
            assert_eq!(
                reader.read_accounting().unwrap()["distinct_payload_bytes"],
                size + 2
            );
            let limited = GitObjects::open_bounded(
                &repo,
                ReadProfile {
                    depth: 1,
                    ..ReadProfile::default()
                },
            )
            .unwrap();
            assert!(
                limited
                    .blob(target)
                    .unwrap_err()
                    .to_string()
                    .contains("delta depth")
            );
            let seed = fx.git_with(
                &repo,
                &["hash-object", "-w", "--stdin"],
                Some(&[b's'; 300]),
                &[],
            );
            let limited = GitObjects::open_bounded(
                &repo,
                ReadProfile {
                    payload_bytes: size + 200,
                    ..ReadProfile::default()
                },
            )
            .unwrap();
            limited
                .blob(ObjectId::from_hex(seed.as_bytes()).unwrap())
                .unwrap();
            assert!(
                limited
                    .blob(target)
                    .unwrap_err()
                    .to_string()
                    .contains("aggregate")
            );
        }
    }
}
#[test]
fn first_packed_read_requires_index_origin() {
    let (_fx, repo, _, _) = fixture(false);
    let id = objs::compute_hash(Kind::Sha1, objs::Kind::Blob, b"y").unwrap();
    assert_eq!(
        GitObjects::open_bounded(&repo, ReadProfile::default())
            .unwrap()
            .blob(id)
            .unwrap(),
        b"y"
    );
    let reader = GitObjects::open_bounded(&repo, ReadProfile::default()).unwrap();
    for entry in std::fs::read_dir(repo.join(".git/objects/pack")).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|e| e == "idx") {
            std::fs::remove_file(path).unwrap();
        }
    }
    assert!(reader.blob(id).is_err());
}
