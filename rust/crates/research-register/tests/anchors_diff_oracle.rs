//! Test-only differential: the native line diff against the git binary, run exactly as
//! `register.py` runs it (`AnchorGit.DIFF` options and `AnchorGit.CONFIG` pins, on two blobs).
//! The crate itself never runs git.

mod support;

use research_register::anchor::{line_hunks, normalise_hunks, split_lines};
use std::path::Path;
use support::Fx;

const CONFIG: &[&str] = &[
    "diff.algorithm=myers",
    "diff.renames=false",
    "diff.context=0",
    "diff.interHunkContext=0",
    "diff.indentHeuristic=false",
    "diff.noprefix=false",
    "diff.mnemonicPrefix=false",
    "diff.relative=false",
    "core.quotepath=false",
    "core.autocrlf=false",
    "core.safecrlf=false",
    "core.commitGraph=false",
];
const DIFF: &[&str] = &[
    "diff",
    "--no-ext-diff",
    "--no-textconv",
    "--no-color",
    "--text",
    "--minimal",
    "--no-renames",
    "--no-indent-heuristic",
    "--inter-hunk-context=0",
    "-U0",
];

/// `register.py` `HUNK` parsing: `@@ -a[,b] +c[,d] @@`, a missing count is 1.
fn parse_hunks(out: &str) -> Vec<(usize, usize, usize, usize)> {
    let num = |s: &str| s.parse::<usize>().unwrap();
    let pair = |s: &str| match s.split_once(',') {
        Some((x, y)) => (num(x), num(y)),
        None => (num(s), 1),
    };
    out.lines()
        .filter_map(|l| l.strip_prefix("@@ -"))
        .map(|rest| {
            let (old, rest) = rest.split_once(" +").unwrap();
            let new = rest.split_once(" @@").unwrap().0;
            let ((a, b), (c, d)) = (pair(old), pair(new));
            (a, b, c, d)
        })
        .collect()
}

struct XorShift(u64);

impl XorShift {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

const ALPHABETS: &[&[&str]] = &[
    &["a", "b"],
    &["a", "b", ""],
    &["a", "b", "c", "", ""],
    &["x", "y", "z", "", " ", "x "],
    &["# A", "## B", "text", "", "```", "- item"],
    &["a", "b", "a\r", "", "\t"],
];

fn gen_lines(rng: &mut XorShift, alphabet: &[String]) -> Vec<String> {
    let n = rng.below(41);
    (0..n)
        .map(|_| alphabet[rng.below(alphabet.len())].clone())
        .collect()
}

fn mutate(rng: &mut XorShift, src: &[String], alphabet: &[String]) -> Vec<String> {
    let mut v = src.to_vec();
    for _ in 0..1 + rng.below(4) {
        let len = v.len();
        match rng.below(5) {
            0 => {
                let at = rng.below(len + 1);
                v.insert(at, alphabet[rng.below(alphabet.len())].clone());
            }
            1 if len > 0 => {
                v.remove(rng.below(len));
            }
            2 if len > 0 => {
                let at = rng.below(len);
                v[at] = alphabet[rng.below(alphabet.len())].clone();
            }
            3 if len > 1 => {
                // duplicate a block elsewhere
                let s = rng.below(len);
                let e = (s + 1 + rng.below(4)).min(len);
                let block: Vec<String> = v[s..e].to_vec();
                let at = rng.below(v.len() + 1);
                v.splice(at..at, block);
            }
            4 if len > 1 => {
                // move a block
                let s = rng.below(len);
                let e = (s + 1 + rng.below(4)).min(len);
                let block: Vec<String> = v.drain(s..e).collect();
                let at = rng.below(v.len() + 1);
                v.splice(at..at, block);
            }
            _ => {}
        }
        if v.len() > 40 {
            v.truncate(40);
        }
    }
    v
}

fn join(lines: &[String], final_newline: bool) -> Vec<u8> {
    let mut s = lines.join("\n");
    if final_newline && !lines.is_empty() {
        s.push('\n');
    }
    s.into_bytes()
}

fn cases() -> Vec<(String, Vec<u8>, Vec<u8>)> {
    let mut out: Vec<(String, Vec<u8>, Vec<u8>)> = vec![
        ("AXA-A".into(), b"A\nX\nA\n".to_vec(), b"A\n".to_vec()),
        ("empty-empty-nl".into(), b"".to_vec(), b"\n".to_vec()),
        (
            "final-nl-added".into(),
            b"a\nb".to_vec(),
            b"a\nb\n".to_vec(),
        ),
        (
            "final-nl-removed".into(),
            b"a\nb\n".to_vec(),
            b"a\nb".to_vec(),
        ),
        ("crlf".into(), b"a\r\nb\r\n".to_vec(), b"a\nb\r\n".to_vec()),
        (
            "nul-bytes".into(),
            b"a\0\nb\n".to_vec(),
            b"a\0\nc\n".to_vec(),
        ),
        (
            "bad-utf8".into(),
            b"\xff\nb\n\xfe\n".to_vec(),
            b"b\n\xff\n".to_vec(),
        ),
        ("only-nl".into(), b"\n\n\n".to_vec(), b"\n\n".to_vec()),
    ];
    // Long common tails, so git's block-wise tail trimming applies.
    let long = |c: char, n: usize| -> String { std::iter::repeat_n(c, n).collect() };
    for (i, (head_a, head_b)) in [("a\nb\n", "c\n"), ("x\n", "x\ny\n"), ("", "z\n")]
        .iter()
        .enumerate()
    {
        for tail_lines in [9, 10, 11, 21, 31] {
            let tail: String = (0..tail_lines)
                .map(|k| format!("{}\n", long(if k % 3 == 0 { 'p' } else { 'q' }, 99)))
                .collect();
            out.push((
                format!("tail-{i}-{tail_lines}"),
                format!("{head_a}{tail}").into_bytes(),
                format!("{head_b}{tail}").into_bytes(),
            ));
        }
    }
    // Runs of identical long lines: the deleted or inserted line can sit anywhere in the run, and
    // git places it before the trimmed tail.
    let same = format!("{}\n", long('L', 99));
    for (na, nb) in [(30, 29), (29, 30), (25, 21), (11, 12), (40, 38)] {
        out.push((
            format!("run-{na}-{nb}"),
            same.repeat(na).into_bytes(),
            same.repeat(nb).into_bytes(),
        ));
        out.push((
            format!("run-mid-{na}-{nb}"),
            format!("h\n{}m\n{}", same.repeat(na), same.repeat(14)).into_bytes(),
            format!("h\n{}m\n{}", same.repeat(nb), same.repeat(14)).into_bytes(),
        ));
    }
    let mut rng = XorShift(0x9e37_79b9_7f4a_7c15);
    for n in 0..3200 {
        let base = ALPHABETS[rng.below(ALPHABETS.len())];
        let alphabet: Vec<String> = if rng.below(10) == 0 {
            // long lines: files exceed 1024 bytes and share long tails
            base.iter()
                .map(|s| format!("{s}{}", long('.', 60)))
                .collect()
        } else {
            base.iter().map(|s| s.to_string()).collect()
        };
        let a = gen_lines(&mut rng, &alphabet);
        let b = if rng.below(3) == 0 {
            gen_lines(&mut rng, &alphabet)
        } else {
            mutate(&mut rng, &a, &alphabet)
        };
        let (fa, fb) = (rng.below(4) != 0, rng.below(4) != 0);
        out.push((format!("rand-{n}"), join(&a, fa), join(&b, fb)));
    }
    out
}

fn git_hunks(fx: &Fx, repo: &Path, a: &str, b: &str) -> Vec<(usize, usize, usize, usize)> {
    let mut args: Vec<&str> = vec!["--no-optional-locks", "--no-replace-objects"];
    for kv in CONFIG {
        args.push("-c");
        args.push(kv);
    }
    args.extend_from_slice(DIFF);
    args.push(a);
    args.push(b);
    let env = [
        ("GIT_CONFIG_GLOBAL", "/dev/null"),
        ("GIT_NO_REPLACE_OBJECTS", "1"),
        ("GIT_OPTIONAL_LOCKS", "0"),
    ];
    let out = fx.cmd(repo, &args, &env).output().unwrap();
    assert!(out.status.success(), "git diff {a} {b}");
    parse_hunks(&String::from_utf8_lossy(&out.stdout))
}

fn short(data: &[u8]) -> String {
    String::from_utf8_lossy(data).chars().take(300).collect()
}

#[test]
fn native_diff_equals_git_diff() {
    let started = std::time::Instant::now();
    let fx = Fx::new("diff-oracle");
    let repo = fx.root.join("r");
    std::fs::create_dir_all(&repo).unwrap();
    fx.git(&repo, &["init", "-q"]);
    let cases = cases();
    let files = fx.root.join("files");
    std::fs::create_dir_all(&files).unwrap();
    let mut paths = Vec::new();
    for (i, (_, a, b)) in cases.iter().enumerate() {
        for (side, data) in [("a", a), ("b", b)] {
            let p = files.join(format!("{i}-{side}"));
            std::fs::write(&p, data).unwrap();
            paths.push(format!("{}\n", p.display()));
        }
    }
    // In chunks, so git's output never fills the pipe while its input is still being written.
    let mut ids = Vec::new();
    for chunk in paths.chunks(500) {
        let out = fx.git_with(
            &repo,
            &["hash-object", "-w", "--no-filters", "--stdin-paths"],
            Some(chunk.concat().as_bytes()),
            &[],
        );
        ids.extend(out.lines().map(str::to_owned));
    }
    let ids: Vec<&str> = ids.iter().map(String::as_str).collect();
    assert_eq!(ids.len(), cases.len() * 2);
    let mut disagreements = Vec::new();
    for (i, (name, a, b)) in cases.iter().enumerate() {
        let want = git_hunks(&fx, &repo, ids[2 * i], ids[2 * i + 1]);
        let got = line_hunks(a, b);
        let (la, lb) = (split_lines(a), split_lines(b));
        if want != got || normalise_hunks(&want, &la, &lb) != normalise_hunks(&got, &la, &lb) {
            disagreements.push(format!(
                "{name}: old {:?} new {:?}\n  git    {want:?}\n  native {got:?}",
                short(a),
                short(b)
            ));
        }
    }
    eprintln!(
        "diff oracle: {} cases, {} disagreements, {:.1?}",
        cases.len(),
        disagreements.len(),
        started.elapsed()
    );
    assert!(
        disagreements.is_empty(),
        "{} of {} cases disagree:\n{}",
        disagreements.len(),
        cases.len(),
        disagreements[..disagreements.len().min(10)].join("\n")
    );
}
