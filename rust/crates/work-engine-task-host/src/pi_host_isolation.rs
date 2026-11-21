//! Installed-research credential isolation for the Pi Host route, the analog
//! of the ChatGPT access-source isolation. The hosted credentials file and
//! every credential source it names are owner-private and lie outside the
//! repository, the state directories and the native root. Its payer kinds are
//! exactly those the pinned profile's routes allow, as reported by the pinned
//! Host binary's `profile describe`, compared by exact string. The described
//! profile must also be one the lifecycle runner can run, and the native root
//! one every Host `prepare` under it can mount. Only kinds and nonsecret
//! source paths are read here. No secret value is opened, logged or copied.

use crate::research_installation::{O_NOFOLLOW, O_NONBLOCK};
use crate::{HostStartup, current_uid};
use lifecycle_service::{NativeProviderRoute, PiHostCompositionRefusal};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, OpenOptions};
use std::io::Read;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

const CREDENTIALS_SCHEMA: &str = "work-engine.pi-hosted-credentials.v1";
const COMPOSITION_SCHEMA: &str = "work-engine.pi-host-composition.v1";
/// The Host's own bounds on the credentials file.
const MAX_CREDENTIALS_BYTES: u64 = 65_536;
const MAX_PAYERS: usize = 8;
/// The Host's own bound on the composition profile.
const MAX_PROFILE_BYTES: u64 = 65_536;
const MAX_DESCRIPTION_BYTES: u64 = 256 * 1024;
/// The `{status,class,error}` refusal `profile describe` prints on stderr.
const MAX_DESCRIBE_REFUSAL_BYTES: u64 = 64 * 1024;
/// `profile describe` runs a full `prepare` dry run, hashing the Host binary,
/// Docker and the runtime tree. A release Host takes about a second on the
/// real runtime; a debug Host took 32-41 s. One-shot install check: a
/// generous bound, then kill.
const DESCRIBE_DEADLINE: Duration = Duration::from_secs(180);

/// The providers and wire the lifecycle runner admits and retains: it refuses
/// any other provider and retains only Messages captures.
const RUNNABLE_PROVIDERS: [&str; 2] = ["claude-sdk", "openai-codex"];
const RETAINED_WIRE: &str = "messages";

/// Why the installed Pi Host route is refused at the install check.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PiHostIsolationRefusal {
    /// The installed profile or Host binary pin does not hold.
    Composition(PiHostCompositionRefusal),
    /// The credentials file is absent, a symlink, not regular, hard-linked or
    /// oversized.
    CredentialsUnavailable,
    /// The credentials file is not mode 0600/0400 or not owned by this user.
    CredentialsPermissions,
    /// The credentials file lies in the repository, a state directory or the
    /// native root.
    CredentialsInsideProtectedRoot,
    /// Not a `work-engine.pi-hosted-credentials.v1` file of known kinds.
    CredentialsMalformed,
    /// A `*-env` kind: production reads credentials from files only.
    EnvironmentCredential,
    /// A credentials file payer that no route allows, or of a disallowed kind.
    PayerNotAllowed,
    /// A route payer that needs a credential has none.
    PayerWithoutCredential,
    /// A credential source path is not absolute and normal, or is absent, a
    /// symlink, not a regular file or hard-linked.
    SourceUnavailable,
    /// A credential source is not owner-only or not owned by this user.
    SourcePermissions,
    /// A credential source lies in the repository, a state directory or the
    /// native root.
    SourceInsideProtectedRoot,
    /// `profile describe` could not run or did not answer as documented.
    DescribeUnavailable,
    /// `profile describe` refused the profile (exit 1, class `profile`).
    DescribeRefused,
    /// `profile describe` failed in its scratch environment or the composed
    /// job's admission (exit 1, class `root`, `io` or `admission`).
    DescribeEnvironmentFailure,
    /// The described profile is not the pinned profile.
    DescribeProfileMismatch,
    /// A route payer the description omits, or an https route whose payer is
    /// described as needing no credential.
    DescribeInconsistent,
    /// A provider the lifecycle runner refuses.
    ProviderNotRunnable,
    /// A wire whose captures the lifecycle runner does not retain.
    WireNotRetained,
    /// A loopback route: offline qualification only.
    LoopbackRoute,
    /// The native root is not a canonical directory of plain components
    /// (`[A-Za-z0-9._-]`) under a real parent directory, disjoint from the
    /// runtime root: a Host `prepare` under it could not mount its root.
    NativeRootUnmountable,
}

impl PiHostIsolationRefusal {
    pub(crate) fn reason(self) -> &'static str {
        match self {
            Self::Composition(refusal) => refusal.reason(),
            Self::CredentialsUnavailable => "pi host credentials file unavailable",
            Self::CredentialsPermissions => "pi host credentials file not owner-only",
            Self::CredentialsInsideProtectedRoot => {
                "pi host credentials file inside repository, state or native root"
            }
            Self::CredentialsMalformed => "pi host credentials file malformed",
            Self::EnvironmentCredential => "pi host environment credential kind refused",
            Self::PayerNotAllowed => "pi host credential payer or kind not allowed by routes",
            Self::PayerWithoutCredential => "pi host route payer without credential",
            Self::SourceUnavailable => "pi host credential source path unavailable",
            Self::SourcePermissions => "pi host credential source not owner-only",
            Self::SourceInsideProtectedRoot => {
                "pi host credential source inside repository, state or native root"
            }
            Self::DescribeUnavailable => "pi host profile describe unavailable",
            Self::DescribeRefused => "pi host profile describe refused the profile",
            Self::DescribeEnvironmentFailure => "pi host profile describe environment failure",
            Self::DescribeProfileMismatch => "pi host described profile is not the pinned profile",
            Self::DescribeInconsistent => "pi host profile description inconsistent",
            Self::ProviderNotRunnable => "pi host provider not runnable by the lifecycle runner",
            Self::WireNotRetained => "pi host wire not retained by the lifecycle runner",
            Self::LoopbackRoute => "pi host loopback route refused outside controlled proof",
            Self::NativeRootUnmountable => {
                "pi host native root not canonical, plain and disjoint from the runtime"
            }
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CredentialsFile {
    schema: String,
    payers: BTreeMap<String, CredentialSource>,
}

/// One payer's source: its kind and nonsecret location, never a secret.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CredentialSource {
    kind: String,
    #[serde(default)]
    path: Option<PathBuf>,
    #[serde(default)]
    variable: Option<String>,
}

/// The fields of the `profile describe` answer this check uses.
#[derive(Deserialize)]
struct ProfileDescription {
    schema: String,
    profile_sha256: String,
    provider: String,
    wire: String,
    routes: Vec<DescribedRoute>,
    payers: BTreeMap<String, BTreeSet<String>>,
}

#[derive(Deserialize)]
struct DescribedRoute {
    endpoint: String,
    payer: String,
}

/// The class of a `profile describe` refusal; its message is not used.
#[derive(Deserialize)]
struct DescribeRefusal {
    class: String,
}

/// The production install check for `PiHostV1`: the installed pins, the
/// pinned profile described by the pinned Host, then credential isolation
/// against the described payers.
pub(crate) fn admit_installation(startup: &HostStartup) -> Result<(), PiHostIsolationRefusal> {
    let native = startup
        .lifecycle
        .native
        .as_ref()
        .ok_or(PiHostIsolationRefusal::Composition(
            PiHostCompositionRefusal::Missing,
        ))?;
    if native.provider_route != NativeProviderRoute::PiHostV1 {
        return Err(PiHostIsolationRefusal::Composition(
            PiHostCompositionRefusal::WrongRoute,
        ));
    }
    lifecycle_service::verify_pi_host_composition(native)
        .map_err(PiHostIsolationRefusal::Composition)?;
    let composition =
        native
            .pi_host_composition
            .as_ref()
            .ok_or(PiHostIsolationRefusal::Composition(
                PiHostCompositionRefusal::Missing,
            ))?;
    let description =
        describe_profile(&native.executable, &composition.profile, DESCRIBE_DEADLINE)?;
    if description.schema != COMPOSITION_SCHEMA
        || description.profile_sha256 != composition.profile_sha256
    {
        return Err(PiHostIsolationRefusal::DescribeProfileMismatch);
    }
    // An https route always needs a credential; an empty kind list means a
    // loopback-only payer.
    if description.routes.iter().any(|route| {
        description
            .payers
            .get(&route.payer)
            .is_none_or(|kinds| kinds.is_empty() && route.endpoint.starts_with("https:"))
    }) {
        return Err(PiHostIsolationRefusal::DescribeInconsistent);
    }
    if !RUNNABLE_PROVIDERS.contains(&description.provider.as_str()) {
        return Err(PiHostIsolationRefusal::ProviderNotRunnable);
    }
    if description.wire != RETAINED_WIRE {
        return Err(PiHostIsolationRefusal::WireNotRetained);
    }
    if !cfg!(feature = "controlled-proof")
        && description
            .routes
            .iter()
            .any(|route| !route.endpoint.starts_with("https:"))
    {
        return Err(PiHostIsolationRefusal::LoopbackRoute);
    }
    check_native_root(
        &native.root,
        profile_runtime_root(&composition.profile).as_deref(),
    )?;
    check_credential_isolation(
        &composition.credentials_file,
        &protected_roots(startup),
        &description.payers,
    )
}

/// The repository, every state directory and the native root.
fn protected_roots(startup: &HostStartup) -> Vec<PathBuf> {
    let mut roots = vec![
        startup.source_root.clone(),
        startup.lifecycle.store_root.clone(),
    ];
    roots.extend(
        [
            startup.task_store_path.parent(),
            startup.socket_path.parent(),
            startup.lifecycle.socket_path.parent(),
        ]
        .into_iter()
        .flatten()
        .map(Path::to_path_buf),
    );
    if let Some(installed) = startup
        .research
        .as_ref()
        .and_then(|research| research.installation.as_ref())
    {
        roots.push(installed.root().to_path_buf());
    }
    if let Some(native) = &startup.lifecycle.native {
        roots.push(native.root.clone());
    }
    roots
}

/// The runtime root the pinned profile names (`runtime.root`), when it names
/// one. Read without following a final symlink and within the Host's bound.
pub(crate) fn profile_runtime_root(profile: &Path) -> Option<PathBuf> {
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(O_NOFOLLOW | O_NONBLOCK)
        .open(profile)
        .ok()?;
    let meta = file.metadata().ok()?;
    if !meta.is_file() || meta.len() > MAX_PROFILE_BYTES {
        return None;
    }
    let mut bytes = Vec::new();
    file.take(MAX_PROFILE_BYTES + 1)
        .read_to_end(&mut bytes)
        .ok()?;
    let profile: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    profile["runtime"]["root"].as_str().map(PathBuf::from)
}

/// The native root predicate (the Host's `prepare` root grammar, applied to
/// every launch root under it): an absolute existing directory whose
/// canonical path is its configured path, whose immediate parent is a real
/// directory, whose every component is `[A-Za-z0-9._-]` (not `.` or `..`),
/// and which is disjoint from the runtime root when the profile names one.
pub(crate) fn check_native_root(
    root: &Path,
    runtime_root: Option<&Path>,
) -> Result<(), PiHostIsolationRefusal> {
    let refused = Err(PiHostIsolationRefusal::NativeRootUnmountable);
    let plain = root.components().skip(1).all(|c| match c {
        Component::Normal(name) => name.to_str().is_some_and(|name| {
            name != "."
                && name != ".."
                && name
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
        }),
        _ => false,
    });
    let Some(parent) = root.parent() else {
        return refused;
    };
    if !root.is_absolute() || !plain {
        return refused;
    }
    if !fs::symlink_metadata(parent).is_ok_and(|meta| meta.is_dir())
        || !fs::symlink_metadata(root).is_ok_and(|meta| meta.is_dir())
        || fs::canonicalize(root).ok().as_deref() != Some(root)
    {
        return refused;
    }
    if let Some(runtime) = runtime_root {
        let runtime = fs::canonicalize(runtime).unwrap_or_else(|_| runtime.to_path_buf());
        if root.starts_with(&runtime) || runtime.starts_with(root) {
            return refused;
        }
    }
    Ok(())
}

fn normal_absolute(path: &Path) -> bool {
    path.is_absolute()
        && path
            .components()
            .all(|c| !matches!(c, Component::ParentDir | Component::CurDir))
}

/// Whether `path`, lexically or canonically, lies in any protected root.
fn inside_any(path: &Path, roots: &[PathBuf]) -> bool {
    let canonical = fs::canonicalize(path).ok();
    roots.iter().any(|root| {
        let canonical_root = fs::canonicalize(root).ok();
        [Some(path), canonical.as_deref()]
            .into_iter()
            .flatten()
            .any(|p| {
                p.starts_with(root) || canonical_root.as_deref().is_some_and(|r| p.starts_with(r))
            })
    })
}

/// The credential isolation predicate over an explicit `payer -> allowed
/// kinds` input (an empty set: a loopback-only payer, which takes no
/// credential).
pub(crate) fn check_credential_isolation(
    credentials_file: &Path,
    protected_roots: &[PathBuf],
    allowed: &BTreeMap<String, BTreeSet<String>>,
) -> Result<(), PiHostIsolationRefusal> {
    use PiHostIsolationRefusal as R;
    if !normal_absolute(credentials_file) {
        return Err(R::CredentialsUnavailable);
    }
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(O_NOFOLLOW | O_NONBLOCK)
        .open(credentials_file)
        .map_err(|_| R::CredentialsUnavailable)?;
    let meta = file.metadata().map_err(|_| R::CredentialsUnavailable)?;
    // A hard link would carry the file's bytes into a protected root.
    if !meta.is_file() || meta.nlink() != 1 || meta.len() > MAX_CREDENTIALS_BYTES {
        return Err(R::CredentialsUnavailable);
    }
    let uid = current_uid().map_err(|_| R::CredentialsPermissions)?;
    if meta.uid() != uid || !matches!(meta.permissions().mode() & 0o7777, 0o600 | 0o400) {
        return Err(R::CredentialsPermissions);
    }
    if inside_any(credentials_file, protected_roots) {
        return Err(R::CredentialsInsideProtectedRoot);
    }
    let mut bytes = Vec::new();
    file.take(MAX_CREDENTIALS_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| R::CredentialsUnavailable)?;
    let parsed: CredentialsFile =
        serde_json::from_slice(&bytes).map_err(|_| R::CredentialsMalformed)?;
    if parsed.schema != CREDENTIALS_SCHEMA
        || parsed.payers.is_empty()
        || parsed.payers.len() > MAX_PAYERS
    {
        return Err(R::CredentialsMalformed);
    }
    let mut sources = Vec::new();
    for (payer, source) in &parsed.payers {
        let kind = source.kind.as_str();
        let environment = kind.ends_with("-env");
        if environment != source.variable.is_some() || environment == source.path.is_some() {
            return Err(R::CredentialsMalformed);
        }
        if environment {
            return Err(R::EnvironmentCredential);
        }
        if !allowed.get(payer).is_some_and(|kinds| kinds.contains(kind)) {
            return Err(R::PayerNotAllowed);
        }
        sources.extend(source.path.as_deref());
    }
    if allowed
        .iter()
        .any(|(payer, kinds)| !kinds.is_empty() && !parsed.payers.contains_key(payer))
    {
        return Err(R::PayerWithoutCredential);
    }
    for source in sources {
        if !normal_absolute(source) {
            return Err(R::SourceUnavailable);
        }
        let meta = fs::symlink_metadata(source).map_err(|_| R::SourceUnavailable)?;
        if !meta.is_file() || meta.nlink() != 1 {
            return Err(R::SourceUnavailable);
        }
        if meta.uid() != uid || meta.permissions().mode() & 0o077 != 0 {
            return Err(R::SourcePermissions);
        }
        if inside_any(source, protected_roots) {
            return Err(R::SourceInsideProtectedRoot);
        }
    }
    Ok(())
}

/// Reads at most `limit + 1` bytes of `stream` on a separate thread.
fn read_bounded<R: Read + Send + 'static>(
    stream: R,
    limit: u64,
) -> Receiver<std::io::Result<Vec<u8>>> {
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let read = stream.take(limit + 1).read_to_end(&mut bytes);
        let _ = sender.send(read.map(|_| bytes));
    });
    receiver
}

/// Runs `<host> profile describe --profile <profile>` with an empty
/// environment (the subcommand reads none but `TMPDIR`, which then defaults
/// to `/tmp`), null stdin, a deadline after which the Host is killed, and
/// bounded stdout and stderr. Exit 1 is a refusal whose class (stderr
/// `{status,class,error}`) separates a refused profile from an environment
/// failure; describe never reads credentials, and only the class is used.
fn describe_profile(
    executable: &Path,
    profile: &Path,
    deadline: Duration,
) -> Result<ProfileDescription, PiHostIsolationRefusal> {
    use PiHostIsolationRefusal as R;
    let mut child = Command::new(executable)
        .args(["profile", "describe", "--profile"])
        .arg(profile)
        .env_clear()
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| R::DescribeUnavailable)?;
    let stdout = read_bounded(
        child.stdout.take().ok_or(R::DescribeUnavailable)?,
        MAX_DESCRIPTION_BYTES,
    );
    let stderr = read_bounded(
        child.stderr.take().ok_or(R::DescribeUnavailable)?,
        MAX_DESCRIBE_REFUSAL_BYTES,
    );
    let deadline = Instant::now() + deadline;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(10));
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(R::DescribeUnavailable);
            }
        }
    };
    let collect = |receiver: Receiver<std::io::Result<Vec<u8>>>, limit: u64| {
        receiver
            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
            .ok()
            .and_then(Result::ok)
            .filter(|bytes| bytes.len() as u64 <= limit)
            .ok_or(R::DescribeUnavailable)
    };
    if status.code() == Some(1) {
        let refusal: DescribeRefusal =
            serde_json::from_slice(&collect(stderr, MAX_DESCRIBE_REFUSAL_BYTES)?)
                .map_err(|_| R::DescribeUnavailable)?;
        return Err(match refusal.class.as_str() {
            "profile" => R::DescribeRefused,
            "root" | "io" | "admission" => R::DescribeEnvironmentFailure,
            _ => R::DescribeUnavailable,
        });
    }
    if !status.success() {
        return Err(R::DescribeUnavailable);
    }
    serde_json::from_slice(&collect(stdout, MAX_DESCRIPTION_BYTES)?)
        .map_err(|_| R::DescribeUnavailable)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    fn sha(bytes: &[u8]) -> String {
        use sha2::{Digest, Sha256};
        format!("{:x}", Sha256::digest(bytes))
    }

    fn private(path: &Path, bytes: &[u8], mode: u32) {
        fs::write(path, bytes).unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
    }

    /// Installation layout: protected roots under `installed/`, credentials
    /// and their sources under `operator/`, outside every protected root.
    struct Layout {
        _temp: tempfile::TempDir,
        installed: PathBuf,
        operator: PathBuf,
    }

    impl Layout {
        fn new() -> Self {
            let temp = tempfile::tempdir().unwrap();
            let base = temp.path().canonicalize().unwrap();
            let installed = base.join("installed");
            let operator = base.join("operator");
            for dir in [&installed, &operator] {
                fs::create_dir(dir).unwrap();
                fs::set_permissions(dir, fs::Permissions::from_mode(0o700)).unwrap();
            }
            for name in ["repository", "state", "native"] {
                fs::create_dir(installed.join(name)).unwrap();
            }
            Self {
                _temp: temp,
                installed,
                operator,
            }
        }

        fn roots(&self) -> Vec<PathBuf> {
            ["repository", "state", "native"]
                .iter()
                .map(|name| self.installed.join(name))
                .collect()
        }

        /// A credentials file at `path` naming one Claude subscription payer
        /// whose source file is `source`.
        fn credentials(&self, path: &Path, source: &Path) -> Value {
            private(source, b"{}", 0o600);
            let value = json!({"schema":CREDENTIALS_SCHEMA,"payers":{
                "claude-subscription":{"kind":"claude-code-auth","path":source}}});
            private(path, &serde_json::to_vec(&value).unwrap(), 0o600);
            value
        }
    }

    fn allowed(entries: &[(&str, &[&str])]) -> BTreeMap<String, BTreeSet<String>> {
        entries
            .iter()
            .map(|(payer, kinds)| {
                (
                    payer.to_string(),
                    kinds.iter().map(|k| k.to_string()).collect(),
                )
            })
            .collect()
    }

    fn claude() -> BTreeMap<String, BTreeSet<String>> {
        allowed(&[("claude-subscription", &["claude-code-auth"])])
    }

    #[test]
    fn credential_isolation_accepts_owner_only_file_outside_protected_roots() {
        let layout = Layout::new();
        let file = layout.operator.join("hosted-credentials.json");
        layout.credentials(&file, &layout.operator.join("claude.json"));
        assert_eq!(
            check_credential_isolation(&file, &layout.roots(), &claude()),
            Ok(())
        );
        fs::set_permissions(&file, fs::Permissions::from_mode(0o400)).unwrap();
        assert_eq!(
            check_credential_isolation(&file, &layout.roots(), &claude()),
            Ok(())
        );
        // A loopback-only payer takes no credential and need not appear.
        let mut with_loopback = claude();
        with_loopback.insert("offline".into(), BTreeSet::new());
        assert_eq!(
            check_credential_isolation(&file, &layout.roots(), &with_loopback),
            Ok(())
        );
        // Kinds compare by exact string with the described kinds. The Host
        // also accepts an `open-router-api-key-*` alias, which describe never
        // lists, so the task host refuses it (stricter than the Host).
        let source = layout.operator.join("openrouter.key");
        private(&source, b"key", 0o600);
        let openrouter = layout.operator.join("openrouter-credentials.json");
        private(
            &openrouter,
            &serde_json::to_vec(&json!({"schema":CREDENTIALS_SCHEMA,"payers":{
                "router":{"kind":"openrouter-api-key-file","path":source}}}))
            .unwrap(),
            0o600,
        );
        assert_eq!(
            check_credential_isolation(
                &openrouter,
                &layout.roots(),
                &allowed(&[(
                    "router",
                    &["openrouter-api-key-env", "openrouter-api-key-file"]
                )])
            ),
            Ok(())
        );
    }

    #[test]
    fn credential_isolation_refuses_missing_symlinked_and_shared_files() {
        use PiHostIsolationRefusal as R;
        let layout = Layout::new();
        let file = layout.operator.join("hosted-credentials.json");
        layout.credentials(&file, &layout.operator.join("claude.json"));
        let check = |path: &Path| check_credential_isolation(path, &layout.roots(), &claude());
        assert_eq!(
            check(&layout.operator.join("absent.json")),
            Err(R::CredentialsUnavailable)
        );
        assert_eq!(
            check(Path::new("hosted-credentials.json")),
            Err(R::CredentialsUnavailable)
        );
        let link = layout.operator.join("linked.json");
        std::os::unix::fs::symlink(&file, &link).unwrap();
        assert_eq!(check(&link), Err(R::CredentialsUnavailable));
        assert_eq!(check(&layout.operator), Err(R::CredentialsUnavailable));
        for mode in [0o640, 0o604, 0o644, 0o700, 0o660] {
            fs::set_permissions(&file, fs::Permissions::from_mode(mode)).unwrap();
            assert_eq!(check(&file), Err(R::CredentialsPermissions), "{mode:o}");
        }
    }

    #[test]
    fn credential_isolation_refuses_files_and_sources_in_protected_roots() {
        use PiHostIsolationRefusal as R;
        let layout = Layout::new();
        let outside_source = layout.operator.join("claude.json");
        for root in layout.roots() {
            let file = root.join("hosted-credentials.json");
            layout.credentials(&file, &outside_source);
            assert_eq!(
                check_credential_isolation(&file, &layout.roots(), &claude()),
                Err(R::CredentialsInsideProtectedRoot),
                "{}",
                root.display()
            );
            let file = layout.operator.join("hosted-credentials.json");
            layout.credentials(&file, &root.join("claude.json"));
            assert_eq!(
                check_credential_isolation(&file, &layout.roots(), &claude()),
                Err(R::SourceInsideProtectedRoot),
                "{}",
                root.display()
            );
        }
        // A symlinked directory into a protected root is resolved.
        let alias = layout.operator.join("alias");
        std::os::unix::fs::symlink(layout.installed.join("native"), &alias).unwrap();
        let file = layout.operator.join("hosted-credentials.json");
        layout.credentials(&file, &alias.join("claude.json"));
        assert_eq!(
            check_credential_isolation(&file, &layout.roots(), &claude()),
            Err(R::SourceInsideProtectedRoot)
        );
        layout.credentials(&file, &outside_source);
        let mut value: Value = serde_json::from_slice(&fs::read(&file).unwrap()).unwrap();
        value["payers"]["claude-subscription"]["path"] = json!(layout.operator.join("absent"));
        fs::set_permissions(&file, fs::Permissions::from_mode(0o600)).unwrap();
        private(&file, &serde_json::to_vec(&value).unwrap(), 0o600);
        assert_eq!(
            check_credential_isolation(&file, &layout.roots(), &claude()),
            Err(R::SourceUnavailable)
        );
    }

    #[test]
    fn credential_isolation_refuses_shared_symlinked_and_irregular_sources() {
        use PiHostIsolationRefusal as R;
        let layout = Layout::new();
        let file = layout.operator.join("hosted-credentials.json");
        let source = layout.operator.join("claude.json");
        let check = || check_credential_isolation(&file, &layout.roots(), &claude());
        for mode in [0o640, 0o604, 0o644] {
            layout.credentials(&file, &source);
            fs::set_permissions(&source, fs::Permissions::from_mode(mode)).unwrap();
            assert_eq!(check(), Err(R::SourcePermissions), "{mode:o}");
        }
        let link = layout.operator.join("claude-link.json");
        layout.credentials(&file, &source);
        std::os::unix::fs::symlink(&source, &link).unwrap();
        layout.credentials(&file, &source);
        let mut value: Value = serde_json::from_slice(&fs::read(&file).unwrap()).unwrap();
        for path in [link, layout.operator.clone(), PathBuf::from("claude.json")] {
            value["payers"]["claude-subscription"]["path"] = json!(path);
            private(&file, &serde_json::to_vec(&value).unwrap(), 0o600);
            assert_eq!(check(), Err(R::SourceUnavailable), "{}", path.display());
        }
    }

    #[test]
    fn credential_isolation_refuses_disallowed_payers_kinds_and_shapes() {
        use PiHostIsolationRefusal as R;
        let layout = Layout::new();
        let file = layout.operator.join("hosted-credentials.json");
        let base = layout.credentials(&file, &layout.operator.join("claude.json"));
        let check_with = |edit: &dyn Fn(&mut Value), allowed: &BTreeMap<_, _>| {
            let mut value = base.clone();
            edit(&mut value);
            fs::set_permissions(&file, fs::Permissions::from_mode(0o600)).unwrap();
            private(&file, &serde_json::to_vec(&value).unwrap(), 0o600);
            check_credential_isolation(&file, &layout.roots(), allowed)
        };
        let check = |edit: &dyn Fn(&mut Value)| check_with(edit, &claude());
        let payer = "claude-subscription";
        assert_eq!(
            check(&|v| v["schema"] = json!("other")),
            Err(R::CredentialsMalformed)
        );
        assert_eq!(
            check(&|v| v["payers"] = json!({})),
            Err(R::CredentialsMalformed)
        );
        assert_eq!(
            check_with(
                &|v| v["payers"][payer]["kind"] = json!("open-router-api-key-file"),
                &allowed(&[(payer, &["openrouter-api-key-file"])])
            ),
            Err(R::PayerNotAllowed),
            "kinds compare by exact string"
        );
        assert_eq!(
            check(&|v| v["payers"][payer]["secret"] = json!("x")),
            Err(R::CredentialsMalformed)
        );
        assert_eq!(
            check(&|v| v["payers"][payer] = json!({"kind":"claude-code-auth"})),
            Err(R::CredentialsMalformed)
        );
        assert_eq!(
            check(&|v| v["payers"][payer] =
                json!({"kind":"anthropic-api-key-env","variable":"ANTHROPIC_API_KEY"})),
            Err(R::EnvironmentCredential)
        );
        assert_eq!(
            check(&|v| v["payers"][payer]["kind"] = json!("anthropic-api-key-file")),
            Err(R::PayerNotAllowed)
        );
        assert_eq!(
            check(&|v| v["payers"][payer]["kind"] = json!("chatgpt-invalid-for-qualification")),
            Err(R::PayerNotAllowed)
        );
        assert_eq!(
            check(&|v| v["payers"]["unrouted"] = v["payers"][payer].clone()),
            Err(R::PayerNotAllowed)
        );
        assert_eq!(
            check_with(&|_| {}, &allowed(&[(payer, &[])])),
            Err(R::PayerNotAllowed),
            "a loopback-only payer takes no credential"
        );
        assert_eq!(
            check_with(
                &|_| {},
                &allowed(&[
                    (payer, &["claude-code-auth"]),
                    ("second", &["api-key-file"])
                ])
            ),
            Err(R::PayerWithoutCredential)
        );
    }

    /// A fake pinned Host whose `profile describe` runs `body` (sh).
    fn scripted_host(dir: &Path, body: &str) -> PathBuf {
        let path = dir.join("work-engine-pi-host");
        let script = format!(
            "#!/bin/sh\n[ \"$1 $2 $3\" = \"profile describe --profile\" ] || exit 2\n{body}\n"
        );
        private(&path, script.as_bytes(), 0o700);
        path
    }

    /// A fake pinned Host whose `profile describe` prints `output` and exits
    /// with `code`: on stdout, or on stderr for a refusal (exit 1).
    fn fake_host(dir: &Path, output: &str, code: u8) -> PathBuf {
        let redirect = if code == 1 { " >&2" } else { "" };
        scripted_host(
            dir,
            &format!(
                "printf '%s\\n' '{}'{redirect}\nexit {code}",
                output.replace('\'', "'\\''")
            ),
        )
    }

    /// A Pi Host installed startup on the research installation fixture in
    /// `dir` (which is also its repository), with the profile, Host and
    /// credentials in `operator`, outside every protected root.
    fn pi_host_startup(
        dir: &Path,
        operator: &Path,
        describe: &dyn Fn(&str) -> (String, u8),
    ) -> HostStartup {
        let dir = dir.canonicalize().unwrap();
        let operator = operator.canonicalize().unwrap();
        let operator = operator.as_path();
        let (mut startup, _, _) = crate::research_installation::tests::fixture(&dir);
        let profile = operator.join("composition.json");
        let runtime = operator.join("runtime");
        fs::create_dir(&runtime).unwrap();
        private(
            &profile,
            &serde_json::to_vec(&json!({"schema":"profile","runtime":{"root":runtime}})).unwrap(),
            0o600,
        );
        let profile_sha256 = sha(&fs::read(&profile).unwrap());
        let (stdout, code) = describe(&profile_sha256);
        let host = fake_host(operator, &stdout, code);
        let credentials = operator.join("hosted-credentials.json");
        let source = operator.join("claude.json");
        private(&source, b"{}", 0o600);
        private(
            &credentials,
            &serde_json::to_vec(&json!({"schema":CREDENTIALS_SCHEMA,"payers":{
                "claude-subscription":{"kind":"claude-code-auth","path":source}}}))
            .unwrap(),
            0o600,
        );
        let native = startup.lifecycle.native.as_mut().unwrap();
        native.provider_route = NativeProviderRoute::PiHostV1;
        native.simulator_base_url.clear();
        native.executable = host.clone();
        native.pi_host_composition = Some(lifecycle_service::PiHostComposition {
            profile,
            profile_sha256,
            host_sha256: sha(&fs::read(&host).unwrap()),
            credentials_file: credentials,
        });
        startup.provider_identity = NativeProviderRoute::PiHostV1.identity().into();
        startup
    }

    fn description(profile_sha256: &str, endpoint: &str) -> String {
        json!({"schema":COMPOSITION_SCHEMA,"profile_sha256":profile_sha256,
            "provider":"claude-sdk","model":"claude-haiku-5-5","wire":"messages",
            "routes":[{"id":"primary","endpoint":endpoint,"payer":"claude-subscription"}],
            "payers":{"claude-subscription":["claude-code-auth"]}})
        .to_string()
    }

    #[test]
    fn install_check_admits_described_profile_with_isolated_credentials() {
        let (dir, operator) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
        let startup = pi_host_startup(dir.path(), operator.path(), &|pin| {
            (description(pin, "https:anthropic-oauth"), 0)
        });
        assert_eq!(admit_installation(&startup), Ok(()));
        // The credentials file named inside the native root is refused.
        let mut inside = startup.clone();
        let native = inside.lifecycle.native.as_mut().unwrap();
        let moved = native.root.join("hosted-credentials.json");
        let composition = native.pi_host_composition.as_mut().unwrap();
        fs::copy(&composition.credentials_file, &moved).unwrap();
        fs::set_permissions(&moved, fs::Permissions::from_mode(0o600)).unwrap();
        composition.credentials_file = moved;
        assert_eq!(
            admit_installation(&inside),
            Err(PiHostIsolationRefusal::CredentialsInsideProtectedRoot)
        );
    }

    /// The production route gate admits the Pi Host only through this check
    /// and leaves the ChatGPT access-lease gate unchanged. Under
    /// controlled-proof every production route stays refused.
    #[test]
    fn production_route_gate_adds_pi_host_and_keeps_codex_access_gate() {
        use crate::research_installation::admit_production_route;
        let production = !cfg!(feature = "controlled-proof");
        let (dir, operator) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
        let startup = pi_host_startup(dir.path(), operator.path(), &|pin| {
            (description(pin, "https:anthropic-oauth"), 0)
        });
        assert_eq!(admit_production_route(&startup).is_ok(), production);
        let credentials = startup.lifecycle.native.as_ref().unwrap();
        let credentials = &credentials
            .pi_host_composition
            .as_ref()
            .unwrap()
            .credentials_file;
        fs::set_permissions(credentials, fs::Permissions::from_mode(0o644)).unwrap();
        assert!(admit_production_route(&startup).is_err());

        let codex_dir = tempfile::tempdir().unwrap();
        let dir = codex_dir.path();
        let (mut codex, _, _) = crate::research_installation::tests::fixture(dir);
        assert!(admit_production_route(&codex).is_err(), "simulator route");
        let native = codex.lifecycle.native.as_mut().unwrap();
        native.provider_route = NativeProviderRoute::OpenaiChatgptAccessLeaseV1;
        native.chatgpt_access_source = Some(
            serde_json::from_value(json!({
                "codex_home":dir.join("codex-home"),"account_id":"account",
                "credentials_store":"file"}))
            .unwrap(),
        );
        assert!(admit_production_route(&codex).is_err(), "no isolation");
        let native = codex.lifecycle.native.as_mut().unwrap();
        native.chatgpt_access_source.as_mut().unwrap().isolation = Some(
            serde_json::from_value(json!({
                "helper_executable":dir.join("helper"),"helper_sha256":"a".repeat(64),
                "proxy_socket":dir.join("proxy/socket"),
                "proxy_policy_executable":dir.join("policy"),
                "proxy_policy_sha256":"b".repeat(64)}))
            .unwrap(),
        );
        assert_eq!(admit_production_route(&codex).is_ok(), production);
    }

    #[test]
    fn install_check_refuses_pins_describe_failures_and_unrunnable_profiles() {
        use PiHostIsolationRefusal as R;
        let edited = |pin: &str, edit: &dyn Fn(&mut Value)| {
            let mut v: Value =
                serde_json::from_str(&description(pin, "https:anthropic-oauth")).unwrap();
            edit(&mut v);
            (v.to_string(), 0)
        };
        let loopback = if cfg!(feature = "controlled-proof") {
            // Controlled proof admits the route; the file's payer then pays a
            // loopback-only payer, which takes no credential.
            R::PayerNotAllowed
        } else {
            R::LoopbackRoute
        };
        type Describe<'a> = &'a dyn Fn(&str) -> (String, u8);
        let cases: [(&str, Describe, R); 10] = [
            (
                "describe refusal",
                &|_| {
                    (
                        json!({"status":"refused","class":"profile","error":"x"}).to_string(),
                        1,
                    )
                },
                R::DescribeRefused,
            ),
            (
                "describe environment failure",
                &|_| {
                    (
                        json!({"status":"refused","class":"io","error":"x"}).to_string(),
                        1,
                    )
                },
                R::DescribeEnvironmentFailure,
            ),
            (
                "describe usage",
                &|_| (String::new(), 2),
                R::DescribeUnavailable,
            ),
            (
                "describe garbage",
                &|_| ("not json".into(), 0),
                R::DescribeUnavailable,
            ),
            (
                "other profile",
                &|_| (description(&"e".repeat(64), "https:anthropic-oauth"), 0),
                R::DescribeProfileMismatch,
            ),
            (
                "loopback",
                &|pin| {
                    edited(pin, &|v| {
                        v["routes"][0]["endpoint"] = json!("loopback:8080");
                        v["payers"]["claude-subscription"] = json!([]);
                    })
                },
                loopback,
            ),
            (
                "unnamed route payer",
                &|pin| edited(pin, &|v| v["payers"] = json!({})),
                R::DescribeInconsistent,
            ),
            (
                "https route without credential kinds",
                &|pin| edited(pin, &|v| v["payers"]["claude-subscription"] = json!([])),
                R::DescribeInconsistent,
            ),
            (
                "openrouter provider",
                &|pin| {
                    edited(pin, &|v| {
                        v["provider"] = json!("openrouter");
                        v["wire"] = json!("completions");
                    })
                },
                R::ProviderNotRunnable,
            ),
            (
                "responses wire",
                &|pin| {
                    edited(pin, &|v| {
                        v["provider"] = json!("openai-codex");
                        v["wire"] = json!("responses");
                    })
                },
                R::WireNotRetained,
            ),
        ];
        for (name, describe, refusal) in cases {
            let (dir, operator) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
            let startup = pi_host_startup(dir.path(), operator.path(), describe);
            assert_eq!(admit_installation(&startup), Err(refusal), "{name}");
        }
        let (dir, operator) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
        let startup = pi_host_startup(dir.path(), operator.path(), &|pin| {
            (description(pin, "https:anthropic-oauth"), 0)
        });
        let composition = |edit: &dyn Fn(&mut lifecycle_service::PiHostComposition)| {
            let mut changed = startup.clone();
            edit(
                changed
                    .lifecycle
                    .native
                    .as_mut()
                    .unwrap()
                    .pi_host_composition
                    .as_mut()
                    .unwrap(),
            );
            admit_installation(&changed)
        };
        assert_eq!(
            composition(&|c| c.profile_sha256 = "d".repeat(64)),
            Err(R::Composition(PiHostCompositionRefusal::ProfilePinMismatch))
        );
        assert_eq!(
            composition(&|c| c.host_sha256 = "d".repeat(64)),
            Err(R::Composition(
                PiHostCompositionRefusal::HostBinaryPinMismatch
            ))
        );
        let mut missing = startup.clone();
        missing
            .lifecycle
            .native
            .as_mut()
            .unwrap()
            .pi_host_composition = None;
        assert_eq!(
            admit_installation(&missing),
            Err(R::Composition(PiHostCompositionRefusal::Missing))
        );
        let mut codex = startup.clone();
        codex.lifecycle.native.as_mut().unwrap().provider_route =
            NativeProviderRoute::OpenaiChatgptAccessLeaseV1;
        assert_eq!(
            admit_installation(&codex),
            Err(R::Composition(PiHostCompositionRefusal::WrongRoute))
        );
    }

    /// A pinned profile and a description of it, for `describe_profile`.
    fn described_profile(dir: &Path) -> (PathBuf, String) {
        let profile = dir.join("composition.json");
        private(&profile, b"{\"schema\":\"profile\"}", 0o600);
        let pin = sha(&fs::read(&profile).unwrap());
        (profile, description(&pin, "https:anthropic-oauth"))
    }

    #[test]
    fn describe_is_killed_at_its_deadline() {
        let dir = tempfile::tempdir().unwrap();
        let (profile, _) = described_profile(dir.path());
        let pid_file = dir.path().join("describe.pid");
        let host = scripted_host(
            dir.path(),
            &format!("echo $$ > '{}'\nexec /bin/sleep 30", pid_file.display()),
        );
        let started = Instant::now();
        assert_eq!(
            describe_profile(&host, &profile, Duration::from_millis(300)).err(),
            Some(PiHostIsolationRefusal::DescribeUnavailable)
        );
        assert!(started.elapsed() < Duration::from_secs(10), "bounded wait");
        let pid = fs::read_to_string(&pid_file).unwrap();
        assert!(
            !Path::new(&format!("/proc/{}", pid.trim())).exists(),
            "the Host is killed and reaped"
        );
        assert_eq!(DESCRIBE_DEADLINE, Duration::from_secs(180));
    }

    #[test]
    fn describe_output_is_capped() {
        use PiHostIsolationRefusal as R;
        let dir = tempfile::tempdir().unwrap();
        let (profile, description) = described_profile(dir.path());
        // Trailing whitespace keeps the JSON valid at any length, so only the
        // cap can refuse the long answer. The script survives a closed pipe.
        let answer = |name: &str, padding: u64| {
            let path = dir.path().join(name);
            let mut bytes = description.clone().into_bytes();
            bytes.resize(bytes.len() + padding as usize, b' ');
            fs::write(&path, bytes).unwrap();
            scripted_host(
                dir.path(),
                &format!("trap '' PIPE\n/bin/cat '{}'\nexit 0", path.display()),
            )
        };
        let host = answer("within.json", MAX_DESCRIPTION_BYTES - 4096);
        assert!(describe_profile(&host, &profile, Duration::from_secs(30)).is_ok());
        let host = answer("over.json", MAX_DESCRIPTION_BYTES);
        assert_eq!(
            describe_profile(&host, &profile, Duration::from_secs(30)).err(),
            Some(R::DescribeUnavailable)
        );
    }

    #[test]
    fn describe_runs_with_an_empty_environment() {
        let dir = tempfile::tempdir().unwrap();
        let (profile, description) = described_profile(dir.path());
        let answer = dir.path().join("answer.json");
        fs::write(&answer, &description).unwrap();
        let dump = dir.path().join("environment");
        let host = scripted_host(
            dir.path(),
            &format!(
                "/usr/bin/env > '{}'\n/bin/cat '{}'",
                dump.display(),
                answer.display()
            ),
        );
        // Variables this test process carries (cargo sets CARGO_MANIFEST_DIR).
        let inherited: Vec<&str> = ["PATH", "HOME", "CARGO_MANIFEST_DIR"]
            .into_iter()
            .filter(|name| std::env::var_os(name).is_some())
            .collect();
        assert!(
            inherited.contains(&"PATH"),
            "sentinel present in the parent"
        );
        assert!(describe_profile(&host, &profile, Duration::from_secs(30)).is_ok());
        let environment = fs::read_to_string(&dump).unwrap();
        for name in inherited {
            assert!(
                !environment
                    .lines()
                    .any(|line| line.starts_with(&format!("{name}="))),
                "{name} reached describe"
            );
        }
    }

    #[test]
    fn describe_refusal_class_separates_profile_from_environment() {
        use PiHostIsolationRefusal as R;
        let dir = tempfile::tempdir().unwrap();
        let (profile, _) = described_profile(dir.path());
        let refusal = |class: &str| json!({"status":"refused","class":class,"error":"x"});
        for (stderr, expected) in [
            (refusal("profile").to_string(), R::DescribeRefused),
            (refusal("root").to_string(), R::DescribeEnvironmentFailure),
            (refusal("io").to_string(), R::DescribeEnvironmentFailure),
            (
                refusal("admission").to_string(),
                R::DescribeEnvironmentFailure,
            ),
            (refusal("snapshot-id").to_string(), R::DescribeUnavailable),
            (
                json!({"status":"refused"}).to_string(),
                R::DescribeUnavailable,
            ),
            ("not json".into(), R::DescribeUnavailable),
        ] {
            let host = fake_host(dir.path(), &stderr, 1);
            assert_eq!(
                describe_profile(&host, &profile, Duration::from_secs(30)).err(),
                Some(expected),
                "{stderr}"
            );
        }
    }

    /// Each protected root of a real installed startup, made disjoint from
    /// the others, refuses the credentials file and a source inside it.
    #[test]
    fn install_check_protects_every_startup_root() {
        use PiHostIsolationRefusal as R;
        let (dir, operator, roots) = (
            tempfile::tempdir().unwrap(),
            tempfile::tempdir().unwrap(),
            tempfile::tempdir().unwrap(),
        );
        let mut startup = pi_host_startup(dir.path(), operator.path(), &|pin| {
            (description(pin, "https:anthropic-oauth"), 0)
        });
        let base = roots.path().canonicalize().unwrap();
        let separate = |name: &str| {
            let path = base.join(name);
            fs::create_dir(&path).unwrap();
            path
        };
        startup.source_root = separate("repository");
        startup.lifecycle.store_root = separate("lifecycle-store");
        startup.task_store_path = separate("task-store").join("tasks.sqlite");
        startup.socket_path = separate("host-socket").join("host.sock");
        startup.lifecycle.socket_path = separate("lifecycle-socket").join("lifecycle.sock");
        assert_eq!(admit_installation(&startup), Ok(()));
        let installation = startup
            .research
            .as_ref()
            .and_then(|research| research.installation.as_ref())
            .unwrap()
            .root()
            .to_path_buf();
        let protected = [
            ("source root", startup.source_root.clone()),
            ("lifecycle store", startup.lifecycle.store_root.clone()),
            (
                "task store parent",
                startup.task_store_path.parent().unwrap().to_path_buf(),
            ),
            (
                "host socket parent",
                startup.socket_path.parent().unwrap().to_path_buf(),
            ),
            (
                "lifecycle socket parent",
                startup
                    .lifecycle
                    .socket_path
                    .parent()
                    .unwrap()
                    .to_path_buf(),
            ),
            ("installation root", installation),
            (
                "native root",
                startup.lifecycle.native.as_ref().unwrap().root.clone(),
            ),
        ];
        let composition = startup
            .lifecycle
            .native
            .as_ref()
            .unwrap()
            .pi_host_composition
            .clone()
            .unwrap();
        let credentials: Value =
            serde_json::from_slice(&fs::read(&composition.credentials_file).unwrap()).unwrap();
        for (name, root) in protected {
            let mut inside = startup.clone();
            let moved = root.join("hosted-credentials.json");
            private(&moved, &serde_json::to_vec(&credentials).unwrap(), 0o600);
            let native = inside.lifecycle.native.as_mut().unwrap();
            native
                .pi_host_composition
                .as_mut()
                .unwrap()
                .credentials_file = moved.clone();
            assert_eq!(
                admit_installation(&inside),
                Err(R::CredentialsInsideProtectedRoot),
                "credentials file in {name}"
            );
            fs::remove_file(&moved).unwrap();

            let source = root.join("claude.json");
            private(&source, b"{}", 0o600);
            let mut redirected = credentials.clone();
            redirected["payers"]["claude-subscription"]["path"] = json!(source);
            let file = operator
                .path()
                .canonicalize()
                .unwrap()
                .join("redirected.json");
            private(&file, &serde_json::to_vec(&redirected).unwrap(), 0o600);
            let mut inside = startup.clone();
            let native = inside.lifecycle.native.as_mut().unwrap();
            native
                .pi_host_composition
                .as_mut()
                .unwrap()
                .credentials_file = file.clone();
            assert_eq!(
                admit_installation(&inside),
                Err(R::SourceInsideProtectedRoot),
                "credential source in {name}"
            );
            fs::remove_file(&source).unwrap();
            fs::remove_file(&file).unwrap();
        }
    }

    #[test]
    fn native_root_predicate_requires_a_canonical_plain_root_disjoint_from_the_runtime() {
        use PiHostIsolationRefusal as R;
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().canonicalize().unwrap();
        let dir = |name: &str| {
            let path = base.join(name);
            fs::create_dir_all(&path).unwrap();
            path
        };
        let native = dir("native");
        let runtime = dir("runtime");
        assert_eq!(check_native_root(&native, None), Ok(()));
        assert_eq!(check_native_root(&native, Some(&runtime)), Ok(()));
        let refused = Err(R::NativeRootUnmountable);
        for name in ["na+tive", "na tive", "na@tive", "na:tive"] {
            assert_eq!(check_native_root(&dir(name), None), refused, "{name}");
        }
        assert_eq!(check_native_root(Path::new("native"), None), refused);
        assert_eq!(check_native_root(Path::new("/"), None), refused);
        assert_eq!(check_native_root(&base.join("absent"), None), refused);
        let file = base.join("file");
        fs::write(&file, b"").unwrap();
        assert_eq!(check_native_root(&file, None), refused);
        let link = base.join("linked-native");
        std::os::unix::fs::symlink(&native, &link).unwrap();
        assert_eq!(check_native_root(&link, None), refused, "symlinked root");
        let parent = base.join("linked-parent");
        std::os::unix::fs::symlink(dir("real-parent"), &parent).unwrap();
        fs::create_dir(parent.join("native")).unwrap();
        assert_eq!(
            check_native_root(&parent.join("native"), None),
            refused,
            "symlinked immediate parent"
        );
        assert_eq!(
            check_native_root(&dir("runtime/native"), Some(&runtime)),
            refused,
            "inside the runtime"
        );
        assert_eq!(
            check_native_root(&base, Some(&runtime)),
            refused,
            "containing the runtime"
        );
        // The runtime root is compared canonically.
        let runtime_link = base.join("runtime-link");
        std::os::unix::fs::symlink(&runtime, &runtime_link).unwrap();
        assert_eq!(
            check_native_root(&runtime.join("native"), Some(&runtime_link)),
            refused
        );
    }

    #[test]
    fn install_check_refuses_an_unmountable_native_root() {
        let (dir, operator) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
        let mut startup = pi_host_startup(dir.path(), operator.path(), &|pin| {
            (description(pin, "https:anthropic-oauth"), 0)
        });
        let composition = startup.lifecycle.native.as_ref().unwrap();
        let profile = &composition.pi_host_composition.as_ref().unwrap().profile;
        assert_eq!(
            profile_runtime_root(profile),
            Some(profile.parent().unwrap().join("runtime"))
        );
        let native = startup.lifecycle.native.as_mut().unwrap();
        native.root = native.root.with_file_name("na+tive");
        fs::create_dir(&native.root).unwrap();
        assert_eq!(
            admit_installation(&startup),
            Err(PiHostIsolationRefusal::NativeRootUnmountable)
        );
    }

    #[test]
    fn credential_isolation_refuses_hard_links_and_fifos() {
        use PiHostIsolationRefusal as R;
        let layout = Layout::new();
        let file = layout.operator.join("hosted-credentials.json");
        let source = layout.operator.join("claude.json");
        layout.credentials(&file, &source);
        let check = |path: &Path| check_credential_isolation(path, &layout.roots(), &claude());
        assert_eq!(check(&file), Ok(()));
        let file_link = layout.installed.join("native/hosted-credentials.json");
        fs::hard_link(&file, &file_link).unwrap();
        assert_eq!(
            check(&file),
            Err(R::CredentialsUnavailable),
            "hard-linked file"
        );
        fs::remove_file(&file_link).unwrap();
        let source_link = layout.installed.join("native/claude.json");
        fs::hard_link(&source, &source_link).unwrap();
        assert_eq!(
            check(&file),
            Err(R::SourceUnavailable),
            "hard-linked source"
        );
        fs::remove_file(&source_link).unwrap();
        assert_eq!(check(&file), Ok(()));

        let fifo = layout.operator.join("credentials.fifo");
        assert!(
            Command::new("mkfifo")
                .arg(&fifo)
                .status()
                .unwrap()
                .success()
        );
        fs::set_permissions(&fifo, fs::Permissions::from_mode(0o600)).unwrap();
        let roots = layout.roots();
        let (sender, receiver) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let _ = sender.send(check_credential_isolation(&fifo, &roots, &claude()));
        });
        assert_eq!(
            receiver.recv_timeout(Duration::from_secs(10)),
            Ok(Err(R::CredentialsUnavailable)),
            "a FIFO is refused without waiting for a writer"
        );
    }

    /// Outside controlled proof only an installed research startup may select
    /// the Pi Host, until step 2 binds the Host's environment.
    #[test]
    fn pi_host_startup_requires_installed_research_outside_controlled_proof() {
        let (dir, operator) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
        let mut startup = pi_host_startup(dir.path(), operator.path(), &|pin| {
            (description(pin, "https:anthropic-oauth"), 0)
        });
        assert!(crate::pi_host_startup_admitted(&startup));
        startup.research.as_mut().unwrap().installation = None;
        assert_eq!(
            crate::pi_host_startup_admitted(&startup),
            cfg!(feature = "controlled-proof")
        );
        startup.research = None;
        assert_eq!(
            crate::pi_host_startup_admitted(&startup),
            cfg!(feature = "controlled-proof")
        );
    }

    /// The install check against a real Pi Host binary: set
    /// `WORK_ENGINE_PI_HOST_BINARY` (a copied, digest-pinned Host),
    /// `WORK_ENGINE_PI_HOST_PROFILE` (a composition profile) and
    /// `WORK_ENGINE_PI_HOST_EXPECT` (`admitted` or the expected refusal
    /// reason). Credential sources are placeholder files of the first
    /// file-kind each described payer allows; no secret is involved.
    #[test]
    #[ignore = "needs a built Pi Host binary and a composition profile"]
    fn install_check_runs_the_real_host_describe() {
        let var = |name: &str| std::env::var(name).unwrap_or_else(|_| panic!("{name} unset"));
        let host = PathBuf::from(var("WORK_ENGINE_PI_HOST_BINARY"));
        let profile = PathBuf::from(var("WORK_ENGINE_PI_HOST_PROFILE"));
        let expect = var("WORK_ENGINE_PI_HOST_EXPECT");
        let started = Instant::now();
        let described = describe_profile(&host, &profile, DESCRIBE_DEADLINE);
        println!(
            "describe: {:?} in {:?}",
            described.as_ref().err(),
            started.elapsed()
        );
        let (dir, operator) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
        let mut startup = pi_host_startup(dir.path(), operator.path(), &|pin| {
            (description(pin, "https:anthropic-oauth"), 0)
        });
        let operator = operator.path().canonicalize().unwrap();
        let mut payers = serde_json::Map::new();
        for (payer, kinds) in described
            .as_ref()
            .map(|d| d.payers.clone())
            .unwrap_or_default()
        {
            if let Some(kind) = kinds.iter().find(|kind| !kind.ends_with("-env")) {
                let source = operator.join(format!("{payer}.source"));
                private(&source, b"placeholder", 0o600);
                payers.insert(payer, json!({"kind":kind,"path":source}));
            }
        }
        let credentials = operator.join("real-host-credentials.json");
        private(
            &credentials,
            &serde_json::to_vec(&json!({"schema":CREDENTIALS_SCHEMA,"payers":payers})).unwrap(),
            0o600,
        );
        let native = startup.lifecycle.native.as_mut().unwrap();
        native.executable = host.clone();
        native.pi_host_composition = Some(lifecycle_service::PiHostComposition {
            profile_sha256: sha(&fs::read(&profile).unwrap()),
            profile,
            host_sha256: sha(&fs::read(&host).unwrap()),
            credentials_file: credentials,
        });
        let started = Instant::now();
        let admitted = admit_installation(&startup);
        println!("install check: {admitted:?} in {:?}", started.elapsed());
        match admitted {
            Ok(()) => assert_eq!(expect, "admitted"),
            Err(refusal) => assert_eq!(expect, refusal.reason()),
        }
    }
}
