//! Mutation selector guard — ensures mutation operations (set/add/remove/raw-set)
//! are scoped to a meaningful path. Rejects global root "/" mutations that would
//! blast the entire document, mirroring the C# MutationSelectorGuard.

use crate::HandlerError;

/// A path is "scoped" if it has at least one segment after "/".
/// Mutations on bare "/" would touch the whole document; that's almost never
/// what the user wants.
pub fn ensure_scoped(path: &str, command: &str) -> Result<(), HandlerError> {
    if path.trim() == "/" || path.trim().is_empty() {
        return Err(HandlerError::InvalidArgument(format!(
            "'{}' command requires a scoped path (got '{}'). \
             Target a specific element like '/body/p[1]' or '/slide[1]/shape[1]' \
             instead of the document root.",
            command, path
        )));
    }
    Ok(())
}

/// Some mutation paths are global but legitimate (e.g. doc-level page settings
/// via "/sectPr" or "/styles"). Use this for commands where the scope is meaningful.
pub fn ensure_scoped_or_known_global(
    path: &str,
    command: &str,
    allowed_globals: &[&str],
) -> Result<(), HandlerError> {
    if path.trim() == "/" || path.trim().is_empty() {
        // Allow if there's an explicit global whitelist entry
        if allowed_globals.contains(&path) {
            return Ok(());
        }
        return Err(HandlerError::InvalidArgument(format!(
            "'{}' command requires a scoped path (got '{}').",
            command, path
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scoped_path_allowed() {
        assert!(ensure_scoped("/body/p[1]", "set").is_ok());
        assert!(ensure_scoped("/slide[1]/shape[2]", "remove").is_ok());
    }

    #[test]
    fn test_root_rejected() {
        assert!(ensure_scoped("/", "set").is_err());
        assert!(ensure_scoped("", "remove").is_err());
    }

    #[test]
    fn test_known_global_allowed() {
        assert!(ensure_scoped_or_known_global("/", "set", &["/styles"]).is_err());
        assert!(ensure_scoped_or_known_global("/styles", "set", &["/styles"]).is_ok());
    }
}
