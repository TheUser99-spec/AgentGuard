use agentguard_core::{Bucket, GuardResult};
use agentguard_manifest::CompiledManifest;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct Enforcer {
    workspace_root: PathBuf,
}

impl Enforcer {
    pub fn new(workspace_root: PathBuf) -> Self {
        let root = std::fs::canonicalize(&workspace_root).unwrap_or(workspace_root);
        Self {
            workspace_root: root,
        }
    }

    pub fn apply_project_protections(&self, manifest: &CompiledManifest) -> GuardResult<()> {
        let deny_paths = self.collect_paths_for_bucket(manifest, Bucket::Deny);

        for path in &deny_paths {
            crate::ace::apply_deny_ace(path)?;
            self.verify_protection(path)?;
        }

        Ok(())
    }

    pub fn release_project_protections(&self, manifest: &CompiledManifest) -> GuardResult<()> {
        let deny_paths = self.collect_paths_for_bucket(manifest, Bucket::Deny);

        for path in &deny_paths {
            crate::ace::remove_deny_ace(path)?;
        }

        Ok(())
    }

    pub fn temporarily_allow(&self, path: &Path) -> GuardResult<()> {
        crate::ace::remove_deny_ace(path)
    }

    pub fn reapply_ask(&self, path: &Path) -> GuardResult<()> {
        crate::ace::apply_deny_ace(path)
    }

    fn verify_protection(&self, path: &Path) -> GuardResult<()> {
        let health = crate::ace::verify_ace(path)?;
        if health.healthy() {
            return Ok(());
        }

        Err(agentguard_core::GuardError::EnforcementFailed {
            path: path.display().to_string(),
            reason: format!(
                "protection incomplete: exists={} content_deny={} metadata_deny={} mic_high_no_write_up={}",
                health.exists, health.content_deny, health.metadata_deny, health.mic_high_no_write_up
            ),
        })
    }

    fn collect_paths_for_bucket(
        &self,
        manifest: &CompiledManifest,
        target: Bucket,
    ) -> HashSet<PathBuf> {
        let mut result = HashSet::new();

        let walker = walkdir::WalkDir::new(&self.workspace_root)
            .max_depth(10)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file());

        for entry in walker {
            let path = entry.path();
            if manifest.bucket_for_path(path) == Some(target) {
                result.insert(path.to_path_buf());
            }
        }

        result
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use agentguard_core::Bucket;
    use agentguard_manifest::{CompiledManifest, ProjectManifest};
    use std::path::{Path, PathBuf};
    use tempfile::TempDir;

    fn make_manifest(workspace: &Path, _spec: &str) -> CompiledManifest {
        let toml = r#"
[project]
name = "test"
default = "unrestricted"

[deny]
files = ["*.env", "secrets/**"]

[ask]
files = ["*.lock"]

[write]
files = ["src/**"]

[read]
files = ["docs/**"]
"#;
        let manifest = ProjectManifest::parse_str(toml).unwrap();
        manifest.compile(workspace.to_path_buf()).unwrap()
    }

    fn create_files(dir: &TempDir) -> Vec<PathBuf> {
        let paths = vec![
            dir.path().join(".env"),
            dir.path().join("secrets").join("key.pem"),
            dir.path().join("Cargo.lock"),
            dir.path().join("src").join("main.rs"),
            dir.path().join("docs").join("readme.md"),
            dir.path().join("public").join("index.html"),
        ];

        for p in &paths {
            if let Some(parent) = p.parent() {
                std::fs::create_dir_all(parent).unwrap();
            }
            std::fs::write(p, "test content").unwrap();
        }

        paths
    }

    #[test]
    fn collect_deny_paths_finds_dotenv() {
        let tmp = TempDir::new().unwrap();
        create_files(&tmp);
        let root = std::fs::canonicalize(tmp.path()).unwrap();
        let manifest = make_manifest(&root, "");
        let enforcer = Enforcer::new(root.clone());

        let paths = enforcer.collect_paths_for_bucket(&manifest, Bucket::Deny);

        let expected = root.join(".env");
        assert!(paths.contains(&expected));
    }

    #[test]
    fn collect_paths_respects_depth_limit() {
        let tmp = TempDir::new().unwrap();
        let deep = tmp.path().join("a/b/c/d/e/f/g/h/i/j/k/file.txt");
        std::fs::create_dir_all(deep.parent().unwrap()).unwrap();
        std::fs::write(&deep, "deep").unwrap();
        let root = std::fs::canonicalize(tmp.path()).unwrap();
        let manifest = make_manifest(&root, "");
        let enforcer = Enforcer::new(root.clone());

        let paths = enforcer.collect_paths_for_bucket(&manifest, Bucket::Deny);

        // max_depth=10, so a/b/c/d/e/f/g/h/i/j/k = depth 11 → excluded
        assert!(!paths.contains(&deep));
    }

    #[test]
    fn empty_workspace_no_panic() {
        let tmp = TempDir::new().unwrap();
        let root = std::fs::canonicalize(tmp.path()).unwrap();
        let manifest = make_manifest(&root, "");
        let enforcer = Enforcer::new(root.clone());

        let paths = enforcer.collect_paths_for_bucket(&manifest, Bucket::Deny);

        assert!(paths.is_empty());
    }
}
