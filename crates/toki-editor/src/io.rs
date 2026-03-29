use std::path::Path;

pub(crate) const DEFAULT_TEXT_FILE_SIZE_LIMIT: u64 = 8 * 1024 * 1024;

pub(crate) fn read_text_file_with_limit(
    path: &Path,
    max_bytes: u64,
    context: &str,
) -> anyhow::Result<String> {
    let metadata = std::fs::metadata(path).map_err(|error| {
        anyhow::anyhow!(
            "Failed to inspect {context} '{}': {}",
            path.display(),
            error
        )
    })?;
    if metadata.len() > max_bytes {
        anyhow::bail!(
            "{context} is too large to load safely: '{}' ({} bytes, max {})",
            path.display(),
            metadata.len(),
            max_bytes
        );
    }
    std::fs::read_to_string(path).map_err(|error| {
        anyhow::anyhow!("Failed to read {context} '{}': {}", path.display(), error)
    })
}
