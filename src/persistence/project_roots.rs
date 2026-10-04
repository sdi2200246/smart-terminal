use std::fs;
use std::path::{Path, PathBuf};

use crate::core::folder_index::FolderIndex;
use crate::core::memory::PersistenceError;

const INDEX_FILENAME: &str = "index.json";
const MEMORY_DIRNAME: &str = "memory";

#[derive(Clone)]
pub struct ProjectRootResolver {
    index_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegisteredProjectRoot {
    pub root: PathBuf,
    pub memory_filename: String,
}

impl ProjectRootResolver {
    pub fn project_local() -> Self {
        let index_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join(MEMORY_DIRNAME)
            .join(INDEX_FILENAME);
        Self { index_path }
    }

    pub fn new(index_path: impl Into<PathBuf>) -> Self {
        Self {
            index_path: index_path.into(),
        }
    }

    fn load_index(&self) -> Result<FolderIndex, PersistenceError> {
        let json = match fs::read_to_string(&self.index_path) {
            Ok(json) => json,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(FolderIndex::default());
            }
            Err(error) => return Err(error.into()),
        };
        Ok(serde_json::from_str(&json)?)
    }

    pub fn resolve_root(&self, cwd: &Path) -> Result<PathBuf, PersistenceError> {
        Ok(self
            .resolve_registered_root(cwd)?
            .map(|resolved| resolved.root)
            .unwrap_or_else(|| cwd.to_path_buf()))
    }

    pub fn resolve_registered_root(
        &self,
        cwd: &Path,
    ) -> Result<Option<RegisteredProjectRoot>, PersistenceError> {
        let index = self.load_index()?;
        let Some(root) = index.root_for(cwd) else {
            return Ok(None);
        };
        let memory_filename = index
            .folders
            .get(&root)
            .expect("resolved folder roots must have an index entry")
            .clone();
        Ok(Some(RegisteredProjectRoot {
            root,
            memory_filename,
        }))
    }

    pub fn historical_roots_for(&self, root: &Path) -> Result<Vec<PathBuf>, PersistenceError> {
        Ok(self.load_index()?.historical_roots_for(root))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tempfile::TempDir;

    #[test]
    fn resolves_registered_ancestors_and_falls_back_to_cwd() {
        let temporary = TempDir::new().unwrap();
        let index_path = temporary.path().join("index.json");
        let index = FolderIndex {
            folders: HashMap::from([(PathBuf::from("/project"), "project.json".into())]),
            ..FolderIndex::default()
        };
        fs::write(&index_path, serde_json::to_vec(&index).unwrap()).unwrap();
        let resolver = ProjectRootResolver::new(index_path);

        assert_eq!(
            resolver.resolve_root(Path::new("/project/src")).unwrap(),
            Path::new("/project")
        );
        assert_eq!(
            resolver.resolve_root(Path::new("/outside")).unwrap(),
            Path::new("/outside")
        );
        assert_eq!(
            resolver
                .resolve_registered_root(Path::new("/outside"))
                .unwrap(),
            None
        );
        assert_eq!(
            resolver
                .resolve_registered_root(Path::new("/project/src"))
                .unwrap(),
            Some(RegisteredProjectRoot {
                root: PathBuf::from("/project"),
                memory_filename: "project.json".into(),
            })
        );
    }

    #[test]
    fn preserves_json_errors_when_loading_an_invalid_index() {
        let temporary = TempDir::new().unwrap();
        let index_path = temporary.path().join("index.json");
        fs::write(&index_path, "{").unwrap();
        let resolver = ProjectRootResolver::new(index_path);

        assert!(matches!(
            resolver.resolve_root(Path::new("/project")),
            Err(PersistenceError::Parse(_))
        ));
    }
}
