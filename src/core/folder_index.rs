use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct FolderIndex {
    pub folders: HashMap<PathBuf, String>,
    #[serde(default)]
    pub superseded_roots: HashMap<PathBuf, PathBuf>,
}

impl FolderIndex {
    pub fn root_for(&self, cwd: &Path) -> Option<PathBuf> {
        let mut current = cwd;
        loop {
            if self.folders.contains_key(current) {
                return Some(current.to_path_buf());
            }
            match current.parent() {
                Some(parent) => current = parent,
                None => return None,
            }
        }
    }

    pub fn ancestor_of(&self, cwd: &Path) -> Option<PathBuf> {
        let mut current = cwd.parent()?;
        loop {
            if self.folders.contains_key(current) {
                return Some(current.to_path_buf());
            }
            current = current.parent()?;
        }
    }

    pub fn descendants_of(&self, cwd: &Path) -> Vec<PathBuf> {
        self.folders
            .keys()
            .filter(|path| path.as_path() != cwd && path.starts_with(cwd))
            .cloned()
            .collect()
    }

    pub fn historical_roots_for(&self, root: &Path) -> Vec<PathBuf> {
        self.superseded_roots
            .iter()
            .filter_map(|(old_root, current_root)| (current_root == root).then(|| old_root.clone()))
            .collect()
    }

    pub fn reparent_roots(&mut self, replaced: &[PathBuf], new_root: &Path) {
        for current_root in self.superseded_roots.values_mut() {
            if replaced.contains(current_root) {
                *current_root = new_root.to_path_buf();
            }
        }
        for old_root in replaced {
            self.superseded_roots
                .insert(old_root.clone(), new_root.to_path_buf());
        }
    }
}
