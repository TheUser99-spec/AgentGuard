use agentguard_core::{
    Bucket, DefaultMode, FileOp, GuardError, GuardResult, PolicyDecision, PolicySource,
};
use globset::{Glob, GlobSet, GlobSetBuilder};
use std::path::{Path, PathBuf};

use crate::parser::ProjectManifest;

/// Manifest compilado: los globs ya estan convertidos a GlobSets
/// para matching O(1). Listo para usar en el hot path.
#[derive(Debug, Clone)]
pub struct CompiledManifest {
    pub workspace_root: PathBuf,
    pub default_mode: DefaultMode,
    deny: GlobSet,
    ask: GlobSet,
    full: GlobSet,
    delete: GlobSet,
    write: GlobSet,
    read: GlobSet,
}

impl CompiledManifest {
    /// Compila un ProjectManifest a GlobSets.
    /// `workspace_root` es la carpeta donde esta el agentguard.toml.
    pub fn compile(manifest: &ProjectManifest, workspace_root: PathBuf) -> GuardResult<Self> {
        Ok(Self {
            workspace_root,
            default_mode: manifest.project.default.clone(),
            deny: build_globset(&manifest.deny.files, "deny")?,
            ask: build_globset(&manifest.ask.files, "ask")?,
            full: build_globset(&manifest.full.files, "full")?,
            delete: build_globset(&manifest.delete.files, "delete")?,
            write: build_globset(&manifest.write.files, "write")?,
            read: build_globset(&manifest.read.files, "read")?,
        })
    }

    /// Evalua si una operacion sobre un path esta permitida.
    ///
    /// IMPORTANTE: el path debe ser absoluto y canonicalizado
    /// antes de llamar a este metodo (proteccion contra symlink bypass).
    pub fn evaluate(&self, abs_path: &Path, op: &FileOp) -> (PolicyDecision, PolicySource) {
        let bucket = self.bucket_for_path(abs_path);

        let decision = match bucket {
            Some(Bucket::Deny) => PolicyDecision::Deny,
            Some(Bucket::Ask) => PolicyDecision::Ask {
                path: abs_path.to_path_buf(),
                op: *op,
            },

            Some(Bucket::Full) => PolicyDecision::Allow,

            Some(Bucket::Delete) => match op {
                FileOp::Read | FileOp::Delete => PolicyDecision::Allow,
                FileOp::Write => self.apply_default(abs_path, op),
            },

            Some(Bucket::Write) => match op {
                FileOp::Read | FileOp::Write => PolicyDecision::Allow,
                FileOp::Delete => PolicyDecision::Deny,
            },

            Some(Bucket::Read) => match op {
                FileOp::Read => PolicyDecision::Allow,
                _ => PolicyDecision::Deny,
            },

            None => self.apply_default(abs_path, op),
        };

        let source = if bucket.is_some() {
            PolicySource::Project
        } else {
            PolicySource::Default
        };

        (decision, source)
    }

    pub fn bucket_for_path(&self, abs_path: &Path) -> Option<Bucket> {
        let rel = abs_path.strip_prefix(&self.workspace_root).ok()?;
        self.winning_bucket(rel)
    }

    fn winning_bucket(&self, rel: &Path) -> Option<Bucket> {
        if self.deny.is_match(rel) {
            return Some(Bucket::Deny);
        }
        if self.ask.is_match(rel) {
            return Some(Bucket::Ask);
        }
        if self.full.is_match(rel) {
            return Some(Bucket::Full);
        }
        if self.delete.is_match(rel) {
            return Some(Bucket::Delete);
        }
        if self.write.is_match(rel) {
            return Some(Bucket::Write);
        }
        if self.read.is_match(rel) {
            return Some(Bucket::Read);
        }
        None
    }

    pub fn apply_default(&self, abs_path: &Path, op: &FileOp) -> PolicyDecision {
        match self.default_mode {
            DefaultMode::Unrestricted => PolicyDecision::Allow,
            DefaultMode::Conservative => match op {
                FileOp::Read => PolicyDecision::Allow,
                FileOp::Write => PolicyDecision::Ask {
                    path: abs_path.to_path_buf(),
                    op: *op,
                },
                FileOp::Delete => PolicyDecision::Deny,
            },
        }
    }
}

fn build_globset(patterns: &[String], _bucket_name: &str) -> GuardResult<GlobSet> {
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        let glob = Glob::new(pattern).map_err(|e| GuardError::InvalidGlob {
            pattern: pattern.clone(),
            reason: e.to_string(),
        })?;
        builder.add(glob);
    }
    builder
        .build()
        .map_err(|e| GuardError::ManifestParse(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::ProjectManifest;

    fn compile(toml: &str) -> CompiledManifest {
        let manifest = ProjectManifest::parse_str(toml).unwrap();
        CompiledManifest::compile(&manifest, PathBuf::from("/workspace")).unwrap()
    }

    #[test]
    fn deny_blocks_all_ops() {
        let cm = compile(
            r#"[deny]
files = [".env"]"#,
        );
        let path = Path::new("/workspace/.env");

        assert_eq!(cm.evaluate(path, &FileOp::Read).0, PolicyDecision::Deny);
        assert_eq!(cm.evaluate(path, &FileOp::Write).0, PolicyDecision::Deny);
        assert_eq!(cm.evaluate(path, &FileOp::Delete).0, PolicyDecision::Deny);
    }

    #[test]
    fn deny_beats_write() {
        let cm = compile(
            r#"
[deny]
files = [".env"]
[write]
files = [".env"]
"#,
        );
        let (decision, _) = cm.evaluate(Path::new("/workspace/.env"), &FileOp::Write);
        assert_eq!(decision, PolicyDecision::Deny);
    }

    #[test]
    fn write_bucket_blocks_delete() {
        let cm = compile(
            r#"[write]
files = ["src/**"]"#,
        );
        let path = Path::new("/workspace/src/main.rs");

        assert_eq!(cm.evaluate(path, &FileOp::Read).0, PolicyDecision::Allow);
        assert_eq!(cm.evaluate(path, &FileOp::Write).0, PolicyDecision::Allow);
        assert_eq!(cm.evaluate(path, &FileOp::Delete).0, PolicyDecision::Deny);
    }

    #[test]
    fn conservative_default_ask_on_write() {
        let cm = compile(
            r#"[project]
default = "conservative""#,
        );
        let path = Path::new("/workspace/anything.txt");

        let (r, _) = cm.evaluate(path, &FileOp::Read);
        let (w, _) = cm.evaluate(path, &FileOp::Write);
        let (d, _) = cm.evaluate(path, &FileOp::Delete);

        assert_eq!(r, PolicyDecision::Allow);
        assert_eq!(
            w,
            PolicyDecision::Ask {
                path: path.to_path_buf(),
                op: FileOp::Write
            }
        );
        assert_eq!(d, PolicyDecision::Deny);
    }

    #[test]
    fn path_outside_workspace_allows() {
        let cm = compile(
            r#"[deny]
files = ["**"]"#,
        );
        let (decision, source) = cm.evaluate(Path::new("/other/secret.key"), &FileOp::Read);
        assert_eq!(decision, PolicyDecision::Allow);
        assert_eq!(source, PolicySource::Default);
    }
}
