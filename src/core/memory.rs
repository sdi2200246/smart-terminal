use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub use crate::core::folder_index::FolderIndex as MemoryIndex;
use crate::core::session::AgentSession;

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct Conversation {
    pub interactions: Vec<Interaction>,
}

impl Conversation {
    pub const MAX_INTERACTIONS: usize = 8;
    pub fn push(&mut self, entry: Interaction) {
        self.interactions.push(entry);
        if self.interactions.len() > Self::MAX_INTERACTIONS {
            let excess = self.interactions.len() - Self::MAX_INTERACTIONS;
            self.interactions.drain(0..excess);
        }
    }

    pub fn clear(&mut self) {
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
pub enum PersistenceError {
    #[error("folder not registered in memory index")]
    NotRegistered,
    #[error("no conversation loaded — call load() first")]
    NotLoaded,
    #[error("folder is already covered by an existing memory at {0}")]
    OverlapsExisting(PathBuf),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to parse JSON data: {0}")]
    Parse(#[from] serde_json::Error),
}

pub trait Memory: Send + Sync {
    fn load(&mut self, cwd: &Path) -> Result<bool, PersistenceError>;
    fn current(&self) -> Option<&Conversation>;
    fn append(&mut self, entry: Interaction) -> Result<(), PersistenceError>;
    fn register(&mut self, cwd: &Path) -> Result<(), PersistenceError>;
    fn unregister(&mut self, cwd: &Path) -> Result<(), PersistenceError>;
    fn clear(&mut self) -> Result<(), PersistenceError>;
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct InvestigationSession {
    #[serde(default)]
    pub planner: Option<AgentSession>,
    #[serde(default)]
    pub executor: Option<AgentSession>,
    #[serde(default)]
    pub completed_reports: Vec<String>,
    #[serde(default)]
    pub absorbed_roots: Vec<PathBuf>,
}

impl InvestigationSession {
    pub fn is_empty(&self) -> bool {
        self.planner.is_none()
            && self.executor.is_none()
            && self.completed_reports.is_empty()
            && self.absorbed_roots.is_empty()
    }

    pub fn merge_history(&mut self, mut other: Self) {
        merge_agent_history(&mut self.planner, other.planner.take());
        merge_agent_history(&mut self.executor, other.executor.take());
        self.completed_reports.append(&mut other.completed_reports);
        self.absorbed_roots.append(&mut other.absorbed_roots);
    }
}

fn merge_agent_history(target: &mut Option<AgentSession>, source: Option<AgentSession>) {
    let Some(mut source) = source else {
        return;
    };
    match target {
        Some(target) => target.events.append(&mut source.events),
        None => *target = Some(source),
    }
}

/// Stores investigator conversation history under a project identity.
/// Histories include provider-specific metadata and currently require the same
/// compatible provider when they are resumed.
pub trait InvestigationSessionStore: Send + Sync {
    fn load(&self, project_key: &str) -> Result<Option<InvestigationSession>, PersistenceError>;
    fn save(
        &self,
        project_key: &str,
        session: &InvestigationSession,
    ) -> Result<(), PersistenceError>;
    fn clear(&self, project_key: &str) -> Result<(), PersistenceError>;
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
            index.folders.insert(
                PathBuf::from(p),
                format!("{}.json", p.trim_start_matches('/').replace('/', "-")),
            );
        }
        index
    }

    #[test]
    fn root_for_resolves_the_nearest_registered_ancestor() {
        let index = index_with(&["/proj", "/proj/foo"]);

        assert_eq!(
            index.root_for(Path::new("/proj/foo/src")),
            Some(PathBuf::from("/proj/foo"))
        );
        assert_eq!(
            index.root_for(Path::new("/proj/other")),
            Some(PathBuf::from("/proj"))
        );
    }

    #[test]
    fn loads_sessions_saved_before_completed_reports_were_added() {
        let session: InvestigationSession =
            serde_json::from_str(r#"{"planner":null,"executor":null}"#).unwrap();

        assert!(session.completed_reports.is_empty());
        assert!(session.is_empty());
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
            vec![
                PathBuf::from("/proj/foo/src"),
                PathBuf::from("/proj/foo/tests")
            ]
        );
    }

    #[test]
    fn descendants_of_excludes_cwd_itself() {
        let index = index_with(&["/proj/foo", "/proj/foo/src"]);
        let descendants = index.descendants_of(Path::new("/proj/foo"));
        assert_eq!(descendants, vec![PathBuf::from("/proj/foo/src")]);
    }
}
