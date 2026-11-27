use std::path::Component;
use std::path::Path;

use crate::error::PackError;

/// Paths in a pack are relative to the suite folder, with `/` between parts, so a pack
/// reads the same on every system.
pub fn check_path(path: &str) -> Result<(), PackError> {
    let bad = |why| Err(PackError::BadPath(path.to_string(), why));
    if path.is_empty() {
        return bad("it's empty");
    }
    if path.len() > u16::MAX as usize {
        return bad("it's longer than 65535 bytes");
    }
    // `:` is a drive or a hidden stream on Windows, and `\` a separator there only.
    if path.contains(['\\', ':', '\0']) {
        return bad("it has `\\`, `:` or a NUL in it");
    }
    if path.split('/').any(|part| part.is_empty() || part == "." || part == "..") {
        return bad("it starts or ends with `/`, or has an empty, `.` or `..` part");
    }
    Ok(())
}

/// A relative path on disk as a pack path.
pub fn pack_path(path: &Path) -> Result<String, PackError> {
    let bad = |why| PackError::BadPath(path.display().to_string(), why);
    let mut parts = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => parts.push(part.to_str().ok_or_else(|| bad("not UTF-8"))?),
            Component::CurDir => {}
            _ => return Err(bad("it isn't relative, or has `..` in it")),
        }
    }
    let joined = parts.join("/");
    check_path(&joined)?;
    Ok(joined)
}
