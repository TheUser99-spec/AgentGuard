use agentguard_core::{GuardError, GuardResult};
use std::path::{Path, PathBuf};

/// Busca `agentguard.toml` desde `start` hacia arriba en el arbol de directorios.
///
/// Devuelve la ruta al fichero si se encuentra, o `ManifestNotFound` si no.
pub fn find_manifest(start: &Path) -> GuardResult<PathBuf> {
    let mut current = if start.is_file() {
        start.parent().unwrap_or(start).to_path_buf()
    } else {
        start.to_path_buf()
    };

    loop {
        let candidate = current.join("agentguard.toml");
        if candidate.exists() {
            return Ok(candidate);
        }

        match current.parent() {
            Some(parent) => current = parent.to_path_buf(),
            None => {
                return Err(GuardError::ManifestNotFound {
                    path: start.display().to_string(),
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn finds_manifest_in_current_dir() {
        let dir = tempdir().unwrap();
        let toml = dir.path().join("agentguard.toml");
        fs::write(&toml, "[project]\nname = \"test\"").unwrap();

        let found = find_manifest(dir.path()).unwrap();
        assert_eq!(found, toml);
    }

    #[test]
    fn finds_manifest_in_parent() {
        let dir = tempdir().unwrap();
        let sub = dir.path().join("src/deep/nested");
        fs::create_dir_all(&sub).unwrap();

        let toml = dir.path().join("agentguard.toml");
        fs::write(&toml, "").unwrap();

        let found = find_manifest(&sub).unwrap();
        assert_eq!(found, toml);
    }

    #[test]
    fn returns_error_when_not_found() {
        let dir = tempdir().unwrap();
        let result = find_manifest(dir.path());
        assert!(matches!(result, Err(GuardError::ManifestNotFound { .. })));
    }
}
