use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use toki_templates::{
    TemplateDescriptor, TemplateInstantiation, TemplateProvider, TemplateProviderError,
    TemplateProviderErrorCode, TemplateProviderRequest, TemplateProviderResponse, TemplateValue,
    TEMPLATE_PROTOCOL_VERSION,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectTemplateSettings {
    #[serde(default = "default_templates_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub crate_path: Option<String>,
    #[serde(default)]
    pub binary_name: Option<String>,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
}

impl Default for ProjectTemplateSettings {
    fn default() -> Self {
        Self {
            enabled: default_templates_enabled(),
            crate_path: None,
            binary_name: None,
            timeout_ms: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetectedProjectTemplateCrate {
    pub crate_dir: PathBuf,
    pub manifest_path: PathBuf,
    pub binary_name: String,
    pub timeout: Duration,
    pub cache_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct ProjectTemplateProvider {
    detected: DetectedProjectTemplateCrate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectTemplateRunnerError {
    pub code: TemplateProviderErrorCode,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct BuildCacheRecord {
    fingerprint: String,
    binary_path: PathBuf,
}

#[derive(Debug, Deserialize)]
struct CargoManifest {
    package: CargoPackage,
    #[serde(default)]
    bin: Vec<CargoBinary>,
}

#[derive(Debug, Deserialize)]
struct CargoPackage {
    name: String,
}

#[derive(Debug, Deserialize)]
struct CargoBinary {
    name: String,
}

const DEFAULT_TEMPLATE_TIMEOUT_MS: u64 = 10_000;

fn default_templates_enabled() -> bool {
    true
}

impl ProjectTemplateSettings {
    pub fn resolved_crate_dir(&self, project_root: &Path) -> PathBuf {
        match &self.crate_path {
            Some(path) => project_root.join(path),
            None => project_root.join("templates"),
        }
    }

    pub fn timeout(&self) -> Duration {
        Duration::from_millis(self.timeout_ms.unwrap_or(DEFAULT_TEMPLATE_TIMEOUT_MS))
    }
}

impl ProjectTemplateRunnerError {
    fn new(code: TemplateProviderErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    fn into_provider_error(self) -> TemplateProviderError {
        TemplateProviderError::new(self.code, self.message)
    }
}

impl ProjectTemplateProvider {
    pub fn detect(
        project_root: &Path,
        settings: &ProjectTemplateSettings,
    ) -> Result<Option<Self>, ProjectTemplateRunnerError> {
        let Some(detected) = detect_project_template_crate(project_root, settings)? else {
            return Ok(None);
        };
        Ok(Some(Self { detected }))
    }

    fn ensure_built(&self) -> Result<PathBuf, ProjectTemplateRunnerError> {
        let fingerprint = fingerprint_template_crate(&self.detected.crate_dir)?;
        if let Some(cached) = read_cache_record(&self.detected.cache_path)? {
            if cached.fingerprint == fingerprint && cached.binary_path.exists() {
                return Ok(cached.binary_path);
            }
        }

        let binary_path = expected_binary_path(&self.detected);
        let output = Command::new("cargo")
            .arg("build")
            .arg("--manifest-path")
            .arg(&self.detected.manifest_path)
            .arg("--bin")
            .arg(&self.detected.binary_name)
            .current_dir(&self.detected.crate_dir)
            .output()
            .map_err(|error| {
                ProjectTemplateRunnerError::new(
                    TemplateProviderErrorCode::BuildFailed,
                    format!(
                        "failed to invoke cargo build for project templates at '{}': {}",
                        self.detected.crate_dir.display(),
                        error
                    ),
                )
            })?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(ProjectTemplateRunnerError::new(
                TemplateProviderErrorCode::BuildFailed,
                format!(
                    "project template build failed for '{}': {}",
                    self.detected.crate_dir.display(),
                    stderr.trim()
                ),
            ));
        }
        if !binary_path.exists() {
            return Err(ProjectTemplateRunnerError::new(
                TemplateProviderErrorCode::BuildFailed,
                format!(
                    "project template build completed but binary '{}' was not produced",
                    binary_path.display()
                ),
            ));
        }

        write_cache_record(
            &self.detected.cache_path,
            &BuildCacheRecord {
                fingerprint,
                binary_path: binary_path.clone(),
            },
        )?;
        Ok(binary_path)
    }

    fn send_request(
        &self,
        request: TemplateProviderRequest,
    ) -> Result<TemplateProviderResponse, ProjectTemplateRunnerError> {
        let binary_path = self.ensure_built()?;
        let request_json = serde_json::to_vec(&request).map_err(|error| {
            ProjectTemplateRunnerError::new(
                TemplateProviderErrorCode::Internal,
                format!("failed to serialize template runner request: {error}"),
            )
        })?;

        let mut child = Command::new(&binary_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| {
                ProjectTemplateRunnerError::new(
                    TemplateProviderErrorCode::InvocationFailed,
                    format!(
                        "failed to spawn project template runner '{}': {}",
                        binary_path.display(),
                        error
                    ),
                )
            })?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(&request_json).map_err(|error| {
                ProjectTemplateRunnerError::new(
                    TemplateProviderErrorCode::InvocationFailed,
                    format!("failed to write request to project template runner: {error}"),
                )
            })?;
        }

        wait_for_child(&mut child, self.detected.timeout)?;
        let output = child.wait_with_output().map_err(|error| {
            ProjectTemplateRunnerError::new(
                TemplateProviderErrorCode::InvocationFailed,
                format!("failed to collect project template runner output: {error}"),
            )
        })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(ProjectTemplateRunnerError::new(
                TemplateProviderErrorCode::InvocationFailed,
                format!("project template runner exited with failure: {}", stderr.trim()),
            ));
        }

        let response: TemplateProviderResponse = serde_json::from_slice(&output.stdout).map_err(|error| {
            ProjectTemplateRunnerError::new(
                TemplateProviderErrorCode::ProtocolViolation,
                format!("project template runner returned invalid JSON: {error}"),
            )
        })?;

        if response_protocol_version(&response) != TEMPLATE_PROTOCOL_VERSION {
            return Err(ProjectTemplateRunnerError::new(
                TemplateProviderErrorCode::UnsupportedProtocolVersion,
                format!(
                    "project template runner protocol version {} is incompatible with ToKi protocol version {}",
                    response_protocol_version(&response),
                    TEMPLATE_PROTOCOL_VERSION
                ),
            ));
        }

        Ok(response)
    }
}

impl TemplateProvider for ProjectTemplateProvider {
    fn list_templates(&self) -> Result<Vec<TemplateDescriptor>, TemplateProviderError> {
        match self.send_request(TemplateProviderRequest::List {
            protocol_version: TEMPLATE_PROTOCOL_VERSION,
        }) {
            Ok(TemplateProviderResponse::List { templates, .. }) => {
                validate_project_template_descriptors(&templates).map_err(|error| error.into_provider_error())?;
                Ok(templates)
            }
            Ok(TemplateProviderResponse::Error { error, .. }) => Err(error),
            Ok(_) => Err(TemplateProviderError::new(
                TemplateProviderErrorCode::ProtocolViolation,
                "project template runner returned a non-list response to list request",
            )),
            Err(error) => Err(error.into_provider_error()),
        }
    }

    fn describe_template(
        &self,
        template_id: &str,
    ) -> Result<TemplateDescriptor, TemplateProviderError> {
        match self.send_request(TemplateProviderRequest::Describe {
            protocol_version: TEMPLATE_PROTOCOL_VERSION,
            template_id: template_id.to_string(),
        }) {
            Ok(TemplateProviderResponse::Describe { descriptor, .. }) => {
                validate_project_template_descriptors(std::slice::from_ref(&descriptor))
                    .map_err(|error| error.into_provider_error())?;
                Ok(descriptor)
            }
            Ok(TemplateProviderResponse::Error { error, .. }) => Err(error),
            Ok(_) => Err(TemplateProviderError::new(
                TemplateProviderErrorCode::ProtocolViolation,
                "project template runner returned a non-describe response to describe request",
            )),
            Err(error) => Err(error.into_provider_error()),
        }
    }

    fn instantiate_template(
        &self,
        template_id: &str,
        parameters: BTreeMap<String, TemplateValue>,
    ) -> Result<TemplateInstantiation, TemplateProviderError> {
        match self.send_request(TemplateProviderRequest::Instantiate {
            protocol_version: TEMPLATE_PROTOCOL_VERSION,
            template_id: template_id.to_string(),
            parameters,
        }) {
            Ok(TemplateProviderResponse::Instantiate { descriptor, plan, .. }) => {
                validate_project_template_descriptors(std::slice::from_ref(&descriptor))
                    .map_err(|error| error.into_provider_error())?;
                Ok(TemplateInstantiation { descriptor, plan })
            }
            Ok(TemplateProviderResponse::Error { error, .. }) => Err(error),
            Ok(_) => Err(TemplateProviderError::new(
                TemplateProviderErrorCode::ProtocolViolation,
                "project template runner returned a non-instantiate response to instantiate request",
            )),
            Err(error) => Err(error.into_provider_error()),
        }
    }
}

pub fn detect_project_template_crate(
    project_root: &Path,
    settings: &ProjectTemplateSettings,
) -> Result<Option<DetectedProjectTemplateCrate>, ProjectTemplateRunnerError> {
    if !settings.enabled {
        return Ok(None);
    }

    let crate_dir = settings.resolved_crate_dir(project_root);
    let manifest_path = crate_dir.join("Cargo.toml");
    if !manifest_path.exists() {
        return Ok(None);
    }

    let manifest_contents = fs::read_to_string(&manifest_path).map_err(|error| {
        ProjectTemplateRunnerError::new(
            TemplateProviderErrorCode::Internal,
            format!(
                "failed to read project template manifest '{}': {}",
                manifest_path.display(),
                error
            ),
        )
    })?;
    let manifest: CargoManifest = toml::from_str(&manifest_contents).map_err(|error| {
        ProjectTemplateRunnerError::new(
            TemplateProviderErrorCode::Internal,
            format!(
                "failed to parse project template manifest '{}': {}",
                manifest_path.display(),
                error
            ),
        )
    })?;

    let binary_name = resolve_binary_name(settings, &manifest)?;
    Ok(Some(DetectedProjectTemplateCrate {
        crate_dir,
        manifest_path,
        binary_name,
        timeout: settings.timeout(),
        cache_path: project_root
            .join(".toki")
            .join("project_template_runner_cache.json"),
    }))
}

fn resolve_binary_name(
    settings: &ProjectTemplateSettings,
    manifest: &CargoManifest,
) -> Result<String, ProjectTemplateRunnerError> {
    if let Some(binary_name) = &settings.binary_name {
        return Ok(binary_name.clone());
    }
    if manifest.bin.len() > 1 {
        return Err(ProjectTemplateRunnerError::new(
            TemplateProviderErrorCode::Internal,
            "project template manifest declares multiple binaries; configure templates.binary_name explicitly",
        ));
    }
    if let Some(binary) = manifest.bin.first() {
        return Ok(binary.name.clone());
    }
    Ok(manifest.package.name.clone())
}

fn validate_project_template_descriptors(
    descriptors: &[TemplateDescriptor],
) -> Result<(), ProjectTemplateRunnerError> {
    let mut ids = BTreeSet::new();
    for descriptor in descriptors {
        descriptor.validate().map_err(|error| {
            ProjectTemplateRunnerError::new(
                TemplateProviderErrorCode::ProtocolViolation,
                format!("project template descriptor '{}' is invalid: {}", descriptor.id, error),
            )
        })?;
        if !descriptor.id.starts_with("project/") {
            return Err(ProjectTemplateRunnerError::new(
                TemplateProviderErrorCode::ProtocolViolation,
                format!(
                    "project template '{}' must use the reserved 'project/' namespace",
                    descriptor.id
                ),
            ));
        }
        if !ids.insert(descriptor.id.clone()) {
            return Err(ProjectTemplateRunnerError::new(
                TemplateProviderErrorCode::ProtocolViolation,
                format!("duplicate project template id '{}'", descriptor.id),
            ));
        }
    }
    Ok(())
}

fn response_protocol_version(response: &TemplateProviderResponse) -> u32 {
    match response {
        TemplateProviderResponse::List { protocol_version, .. }
        | TemplateProviderResponse::Describe { protocol_version, .. }
        | TemplateProviderResponse::Instantiate { protocol_version, .. }
        | TemplateProviderResponse::Error { protocol_version, .. } => *protocol_version,
    }
}

fn expected_binary_path(detected: &DetectedProjectTemplateCrate) -> PathBuf {
    let mut file_name = OsString::from(&detected.binary_name);
    let exe_extension = std::env::consts::EXE_EXTENSION;
    if !exe_extension.is_empty() {
        file_name.push(format!(".{exe_extension}"));
    }
    detected
        .crate_dir
        .join("target")
        .join("debug")
        .join(file_name)
}

fn fingerprint_template_crate(crate_dir: &Path) -> Result<String, ProjectTemplateRunnerError> {
    let mut files = Vec::new();
    collect_template_source_files(crate_dir, crate_dir, &mut files)?;
    files.sort_by(|a, b| a.0.cmp(&b.0));

    let mut hasher = blake3::Hasher::new();
    for (relative_path, absolute_path) in files {
        hasher.update(relative_path.as_os_str().as_encoded_bytes());
        let contents = fs::read(&absolute_path).map_err(|error| {
            ProjectTemplateRunnerError::new(
                TemplateProviderErrorCode::Internal,
                format!(
                    "failed to read template source '{}' while computing fingerprint: {}",
                    absolute_path.display(),
                    error
                ),
            )
        })?;
        hasher.update(&contents);
    }
    Ok(hasher.finalize().to_hex().to_string())
}

fn collect_template_source_files(
    root: &Path,
    current: &Path,
    files: &mut Vec<(PathBuf, PathBuf)>,
) -> Result<(), ProjectTemplateRunnerError> {
    if !current.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(current).map_err(|error| {
        ProjectTemplateRunnerError::new(
            TemplateProviderErrorCode::Internal,
            format!("failed to read template crate directory '{}': {}", current.display(), error),
        )
    })? {
        let entry = entry.map_err(|error| {
            ProjectTemplateRunnerError::new(
                TemplateProviderErrorCode::Internal,
                format!("failed to inspect template crate entry in '{}': {}", current.display(), error),
            )
        })?;
        let path = entry.path();
        let file_name = path.file_name().and_then(|name| name.to_str()).unwrap_or_default();
        if file_name == "target" || file_name == ".git" {
            continue;
        }
        if path.is_dir() {
            collect_template_source_files(root, &path, files)?;
            continue;
        }
        let relative_path = path.strip_prefix(root).map_err(|error| {
            ProjectTemplateRunnerError::new(
                TemplateProviderErrorCode::Internal,
                format!(
                    "failed to relativize template source '{}' against '{}': {}",
                    path.display(),
                    root.display(),
                    error
                ),
            )
        })?;
        files.push((relative_path.to_path_buf(), path));
    }
    Ok(())
}

fn read_cache_record(
    cache_path: &Path,
) -> Result<Option<BuildCacheRecord>, ProjectTemplateRunnerError> {
    if !cache_path.exists() {
        return Ok(None);
    }
    let json = fs::read_to_string(cache_path).map_err(|error| {
        ProjectTemplateRunnerError::new(
            TemplateProviderErrorCode::Internal,
            format!("failed to read template runner cache '{}': {}", cache_path.display(), error),
        )
    })?;
    let record = serde_json::from_str(&json).map_err(|error| {
        ProjectTemplateRunnerError::new(
            TemplateProviderErrorCode::Internal,
            format!("failed to parse template runner cache '{}': {}", cache_path.display(), error),
        )
    })?;
    Ok(Some(record))
}

fn write_cache_record(
    cache_path: &Path,
    record: &BuildCacheRecord,
) -> Result<(), ProjectTemplateRunnerError> {
    if let Some(parent) = cache_path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            ProjectTemplateRunnerError::new(
                TemplateProviderErrorCode::Internal,
                format!(
                    "failed to create template runner cache directory '{}': {}",
                    parent.display(),
                    error
                ),
            )
        })?;
    }
    let json = serde_json::to_string_pretty(record).map_err(|error| {
        ProjectTemplateRunnerError::new(
            TemplateProviderErrorCode::Internal,
            format!("failed to serialize template runner cache: {error}"),
        )
    })?;
    fs::write(cache_path, json).map_err(|error| {
        ProjectTemplateRunnerError::new(
            TemplateProviderErrorCode::Internal,
            format!("failed to write template runner cache '{}': {}", cache_path.display(), error),
        )
    })
}

fn wait_for_child(
    child: &mut std::process::Child,
    timeout: Duration,
) -> Result<(), ProjectTemplateRunnerError> {
    let start = Instant::now();
    loop {
        if child.try_wait().map_err(|error| {
            ProjectTemplateRunnerError::new(
                TemplateProviderErrorCode::InvocationFailed,
                format!("failed to wait on project template runner: {error}"),
            )
        })?.is_some() {
            return Ok(());
        }
        if start.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait();
            return Err(ProjectTemplateRunnerError::new(
                TemplateProviderErrorCode::TimedOut,
                format!("project template runner exceeded timeout of {} ms", timeout.as_millis()),
            ));
        }
        thread::sleep(Duration::from_millis(10));
    }
}
