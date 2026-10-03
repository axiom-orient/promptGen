use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};

use super::{CodexExecutionError, io_error, lexical_absolute, png, sha256_hex};

pub(super) struct ReferenceSnapshot {
    pub path: PathBuf,
    pub sha256: String,
    pub bytes: u64,
}

pub(super) fn read(source: &Path, output: &Path) -> Result<Vec<u8>, CodexExecutionError> {
    let metadata = fs::symlink_metadata(source)
        .map_err(|error| io_error("inspect base image", source, error))?;
    if !metadata.file_type().is_file() || metadata.len() > png::MAX_PNG_FILE_BYTES as u64 {
        return Err(CodexExecutionError::new(
            "CODEX_REFERENCE_TYPE",
            "base image must be a bounded regular PNG file; symlinks are rejected",
        ));
    }
    if aliases(source, output)? {
        return Err(CodexExecutionError::new(
            "CODEX_REFERENCE_OUTPUT_ALIAS",
            "base image and output must be different filesystem objects",
        ));
    }
    let file = File::open(source).map_err(|error| io_error("open base image", source, error))?;
    let opened = file
        .metadata()
        .map_err(|error| io_error("inspect opened base image", source, error))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if metadata.dev() != opened.dev() || metadata.ino() != opened.ino() {
            return Err(CodexExecutionError::new(
                "CODEX_REFERENCE_CHANGED",
                "base file identity changed while opening",
            ));
        }
    }
    if !opened.is_file() {
        return Err(CodexExecutionError::new(
            "CODEX_REFERENCE_TYPE",
            "opened base image is not a regular file",
        ));
    }
    let mut bytes = Vec::new();
    (&file)
        .take(png::MAX_PNG_FILE_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| io_error("read base image", source, error))?;
    if bytes.len() > png::MAX_PNG_FILE_BYTES {
        return Err(CodexExecutionError::new(
            "CODEX_REFERENCE_SIZE",
            "base image exceeds the PNG byte limit",
        ));
    }
    let after = file
        .metadata()
        .map_err(|error| io_error("recheck base image", source, error))?;
    if opened.len() != after.len() || opened.modified().ok() != after.modified().ok() {
        return Err(CodexExecutionError::new(
            "CODEX_REFERENCE_CHANGED",
            "base image changed while reading",
        ));
    }
    png::inspect_bytes(&bytes)
        .map_err(|error| CodexExecutionError::new("CODEX_REFERENCE_PNG", error.to_string()))?;
    Ok(bytes)
}

pub(super) fn snapshot(
    source: Option<&Path>,
    output: &Path,
    working: &Path,
) -> Result<Option<ReferenceSnapshot>, CodexExecutionError> {
    let Some(source) = source else {
        return Ok(None);
    };
    let bytes = read(source, output)?;
    let snapshot = working.join("base-reference.png");
    fs::write(&snapshot, &bytes)
        .map_err(|error| io_error("snapshot base image", &snapshot, error))?;
    Ok(Some(ReferenceSnapshot {
        path: snapshot,
        sha256: sha256_hex(&bytes),
        bytes: bytes.len() as u64,
    }))
}

fn aliases(left: &Path, right: &Path) -> Result<bool, CodexExecutionError> {
    if resolved(left)? == resolved(right)? {
        return Ok(true);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if let (Ok(a), Ok(b)) = (fs::metadata(left), fs::metadata(right)) {
            return Ok(a.dev() == b.dev() && a.ino() == b.ino());
        }
    }
    Ok(false)
}

fn resolved(path: &Path) -> Result<PathBuf, CodexExecutionError> {
    let absolute = lexical_absolute(path)
        .map_err(|error| io_error("resolve base/output path", path, error))?;
    let mut prefix = absolute.clone();
    let mut missing = Vec::new();
    loop {
        match fs::symlink_metadata(&prefix) {
            Ok(_) => break,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let name = prefix.file_name().ok_or_else(|| {
                    CodexExecutionError::new(
                        "CODEX_REFERENCE_PATH",
                        "path has no existing ancestor",
                    )
                })?;
                missing.push(name.to_os_string());
                prefix.pop();
            }
            Err(error) => return Err(io_error("resolve base/output ancestor", &prefix, error)),
        }
    }
    let mut result = fs::canonicalize(&prefix)
        .map_err(|error| io_error("canonicalize base/output ancestor", &prefix, error))?;
    for name in missing.iter().rev() {
        result.push(name);
    }
    Ok(result)
}
