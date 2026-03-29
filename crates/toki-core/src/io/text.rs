use std::io;
use std::path::Path;

pub(crate) const DEFAULT_TEXT_FILE_SIZE_LIMIT: u64 = 8 * 1024 * 1024;

pub(crate) fn read_text_file_with_limit<E, F>(
    path: impl AsRef<Path>,
    max_bytes: u64,
    map_too_large: F,
) -> Result<String, E>
where
    E: From<io::Error>,
    F: FnOnce(&Path, u64, u64) -> E,
{
    let path = path.as_ref();
    let metadata = std::fs::metadata(path)?;
    if metadata.len() > max_bytes {
        return Err(map_too_large(path, metadata.len(), max_bytes));
    }
    Ok(std::fs::read_to_string(path)?)
}

pub(crate) fn too_large_io_error(
    path: &Path,
    size_bytes: u64,
    max_bytes: u64,
    context: &str,
) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!(
            "{context} is too large to load safely: {} ({} bytes, max {})",
            path.display(),
            size_bytes,
            max_bytes
        ),
    )
}
