use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct MemoryIndex {
    pub folders: HashMap<PathBuf, String>,
}

impl MemoryIndex {
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
            .filter(|p| p.as_path() != cwd && p.starts_with(cwd))
            .cloned()
            .collect()
    }

    pub fn resolve(&self, cwd: &Path) -> Option<String> {
        let mut current = cwd;
        loop {
            if let Some(filename) = self.folders.get(current) {
                return Some(filename.clone());
            }
            match current.parent() {
                Some(p) => current = p,
                None => return None,
            }
        }
    }
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct Conversation {
    pub interactions: Vec<Interaction>,
}

impl Conversation {
    pub const MAX_INTERACTIONS: usize = 15;

    pub fn push(&mut self, entry: Interaction) {
        self.interactions.push(entry);
        if self.interactions.len() > Self::MAX_INTERACTIONS {
            self.interactions.remove(0);
        }
    }

    pub fn clear(&mut self){
        self.interactions.clear();
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Interaction {
    pub user_input: String,
    pub predicted_cmd: String,
    pub timestamp: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum MemoryError {
    #[error("folder not registered in memory index")]
    NotRegistered,
    #[error("no conversation loaded — call load() first")]
    NotLoaded,
    #[error("folder is already covered by an existing memory at {0}")]
    OverlapsExisting(PathBuf),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to parse memory file: {0}")]
    Parse(#[from] serde_json::Error),
}

pub trait Memory: Send + Sync {
    fn load(&mut self, cwd: &Path) -> Result<bool, MemoryError>;
    fn current(&self) -> Option<&Conversation>;
    fn append(&mut self, entry: Interaction) -> Result<(), MemoryError>;
    fn register(&mut self, cwd: &Path) -> Result<(), MemoryError>;
    fn unregister(&mut self, cwd: &Path) -> Result<(), MemoryError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(input: &str, cmd: &str) -> Interaction {
        Interaction {
            user_input: input.into(),
            predicted_cmd: cmd.into(),
            timestamp: 0,
        }
    }

    #[test]
    fn push_appends_to_interactions() {
        let mut conv = Conversation::default();
        conv.push(entry("ls", "ls -la"));
        conv.push(entry("git st", "git status"));
        assert_eq!(conv.interactions.len(), 2);
        assert_eq!(conv.interactions[1].predicted_cmd, "git status");
    }

    #[test]
    fn push_caps_at_max_interactions() {
        let mut conv = Conversation::default();
        for i in 0..Conversation::MAX_INTERACTIONS + 5 {
            conv.push(entry(&format!("in{i}"), &format!("cmd{i}")));
        }
        assert_eq!(conv.interactions.len(), Conversation::MAX_INTERACTIONS);
        assert_eq!(conv.interactions[0].user_input, "in5");
        assert_eq!(
            conv.interactions.last().unwrap().user_input,
            format!("in{}", Conversation::MAX_INTERACTIONS + 4)
        );
    }
    fn index_with(paths: &[&str]) -> MemoryIndex {
        let mut index = MemoryIndex::default();
        for p in paths {
            index.folders.insert(PathBuf::from(p), format!("{}.json", p.trim_start_matches('/').replace('/', "-")));
        }
        index
    }

    #[test]
    fn resolve_exact_match() {
        let index = index_with(&["/proj/foo"]);
        assert!(index.resolve(Path::new("/proj/foo")).is_some());
    }

    #[test]
    fn resolve_finds_parent_via_longest_prefix() {
        let index = index_with(&["/proj/foo"]);
        let filename = index.resolve(Path::new("/proj/foo/src/agent"));
        assert!(filename.is_some());
    }

    #[test]
    fn resolve_returns_none_when_no_match() {
        let index = index_with(&["/proj/foo"]);
        assert!(index.resolve(Path::new("/elsewhere")).is_none());
    }

    #[test]
    fn resolve_picks_deepest_match_when_multiple_apply() {
        let index = index_with(&["/proj", "/proj/foo"]);
        let outer = index.folders.get(Path::new("/proj")).unwrap().clone();
        let inner = index.folders.get(Path::new("/proj/foo")).unwrap().clone();
        let resolved = index.resolve(Path::new("/proj/foo/src")).unwrap();
        assert_eq!(resolved, inner);
        let resolved = index.resolve(Path::new("/proj/other")).unwrap();
        assert_eq!(resolved, outer);
    }

    #[test]
    fn ancestor_of_finds_registered_parent() {
        let index = index_with(&["/proj/foo"]);
        let ancestor = index.ancestor_of(Path::new("/proj/foo/src"));
        assert_eq!(ancestor, Some(PathBuf::from("/proj/foo")));
    }

    #[test]
    fn ancestor_of_excludes_cwd_itself() {
        let index = index_with(&["/proj/foo"]);
        let ancestor = index.ancestor_of(Path::new("/proj/foo"));
        assert!(ancestor.is_none(), "cwd should not be its own ancestor");
    }

    #[test]
    fn ancestor_of_returns_none_when_no_match() {
        let index = index_with(&["/elsewhere"]);
        assert!(index.ancestor_of(Path::new("/proj/foo/src")).is_none());
    }

    #[test]
    fn descendants_of_finds_all_under_path() {
        let index = index_with(&["/proj/foo/src", "/proj/foo/tests", "/proj/bar"]);
        let mut descendants = index.descendants_of(Path::new("/proj/foo"));
        descendants.sort();
        assert_eq!(
            descendants,
            vec![PathBuf::from("/proj/foo/src"), PathBuf::from("/proj/foo/tests")]
        );
    }

    #[test]
    fn descendants_of_excludes_cwd_itself() {
        let index = index_with(&["/proj/foo", "/proj/foo/src"]);
        let descendants = index.descendants_of(Path::new("/proj/foo"));
        assert_eq!(descendants, vec![PathBuf::from("/proj/foo/src")]);
    }
}