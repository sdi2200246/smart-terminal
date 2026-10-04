use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use crate::core::memory::{InvestigationSession, InvestigationSessionStore, PersistenceError};
use crate::persistence::ProjectRootResolver;

const INVESTIGATION_SESSIONS_DIR: &str = "investigator-conversations";

pub struct FileInvestigationSessionStore {
    root: PathBuf,
}

impl FileInvestigationSessionStore {
    pub fn project_local() -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("memory")
            .join(INVESTIGATION_SESSIONS_DIR);
        Self { root }
    }

    pub fn for_project_memory_root(memory_root: &Path) -> Self {
        Self {
            root: memory_root.join(INVESTIGATION_SESSIONS_DIR),
        }
    }

    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn session_path(&self, project_key: &str) -> PathBuf {
        let mut hasher = DefaultHasher::new();
        project_key.hash(&mut hasher);
        self.root.join(format!("{:016x}.json", hasher.finish()))
    }

    pub fn migrate_roots(
        &self,
        child_roots: &[PathBuf],
        parent_root: &Path,
    ) -> Result<(), PersistenceError> {
        let mut child_roots = child_roots.to_vec();
        child_roots.sort();

        let mut merged = self
            .load(&parent_root.to_string_lossy())?
            .unwrap_or_default();
        for child_root in &child_roots {
            if merged.absorbed_roots.contains(&child_root) {
                continue;
            }
            if let Some(child_session) = self.load(&child_root.to_string_lossy())? {
                merged.merge_history(child_session);
                merged.absorbed_roots.push(child_root.clone());
            }
        }
        if !merged.is_empty() {
            self.save(&parent_root.to_string_lossy(), &merged)?;
        }
        self.clear_roots(&child_roots)?;
        Ok(())
    }

    pub fn load_for_project_root(
        &self,
        project_root: &Path,
        resolver: &ProjectRootResolver,
    ) -> Result<Option<InvestigationSession>, PersistenceError> {
        let historical_roots = resolver.historical_roots_for(project_root)?;
        self.migrate_roots(&historical_roots, project_root)?;
        self.load(&project_root.to_string_lossy())
    }

    pub fn clear_roots(&self, child_roots: &[PathBuf]) -> Result<(), PersistenceError> {
        for child_root in child_roots {
            self.clear(&child_root.to_string_lossy())?;
        }
        Ok(())
    }
}

impl InvestigationSessionStore for FileInvestigationSessionStore {
    fn load(&self, project_key: &str) -> Result<Option<InvestigationSession>, PersistenceError> {
        let path = self.session_path(project_key);
        match fs::read_to_string(path) {
            Ok(contents) => Ok(Some(serde_json::from_str(&contents)?)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.into()),
        }
    }

    fn save(
        &self,
        project_key: &str,
        session: &InvestigationSession,
    ) -> Result<(), PersistenceError> {
        fs::create_dir_all(&self.root)?;
        let path = self.session_path(project_key);
        let temporary = path.with_extension("tmp");
        let contents = serde_json::to_vec_pretty(session)?;
        fs::write(&temporary, contents)?;
        fs::rename(temporary, path)?;
        Ok(())
    }

    fn clear(&self, project_key: &str) -> Result<(), PersistenceError> {
        let path = self.session_path(project_key);
        match fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::session::AgentSession;
    use tempfile::TempDir;

    #[test]
    fn stores_distinct_project_sessions_and_can_clear_them() {
        let temporary = TempDir::new().unwrap();
        let store = FileInvestigationSessionStore::new(temporary.path());
        let mut planner = AgentSession::builder().user("Question").build();
        planner.add_tool_calls(vec![
            crate::core::session::ToolCall::new(
                "inspect",
                serde_json::json!({"path": "."}),
                "call-1",
            )
            .with_thinking_state(Some("gemini-signature".into())),
        ]);
        let session = InvestigationSession {
            planner: Some(planner),
            executor: Some(AgentSession::builder().user("Question and plan").build()),
            completed_reports: vec!["{\"report\":\"finding\"}".into()],
            absorbed_roots: Vec::new(),
        };

        store.save("/project/a", &session).unwrap();
        let loaded = store.load("/project/a").unwrap().unwrap();
        let loaded_planner = loaded.planner.as_ref().unwrap();
        match &loaded_planner.events[1] {
            crate::core::session::ConversationEvent::ToolCalls(calls) => {
                assert_eq!(calls[0].thinking_state.as_deref(), Some("gemini-signature"))
            }

            _ => panic!("expected persisted tool call"),
        }
        assert!(loaded.executor.is_some());
        assert_eq!(
            loaded.completed_reports,
            vec!["{\"report\":\"finding\"}".to_string()]
        );
        assert!(store.load("/project/b").unwrap().is_none());

        store.clear("/project/a").unwrap();
        assert!(store.load("/project/a").unwrap().is_none());
    }

    #[test]
    fn preserves_json_errors_when_loading_invalid_sessions() {
        let temporary = TempDir::new().unwrap();
        let store = FileInvestigationSessionStore::new(temporary.path());
        let path = store.session_path("/project");
        fs::write(path, "{").unwrap();

        assert!(matches!(
            store.load("/project"),
            Err(PersistenceError::Parse(_))
        ));
    }

    #[test]
    fn preserves_io_errors_when_saving_under_a_file() {
        let temporary = TempDir::new().unwrap();
        let root = temporary.path().join("not-a-directory");
        fs::write(&root, "file").unwrap();
        let store = FileInvestigationSessionStore::new(root);

        assert!(matches!(
            store.save("/project", &InvestigationSession::default()),
            Err(PersistenceError::Io(_))
        ));
    }

    #[test]
    fn migrates_child_sessions_into_the_parent_session_and_clears_children() {
        let temporary = TempDir::new().unwrap();
        let store = FileInvestigationSessionStore::new(temporary.path());
        let parent = PathBuf::from("/project");
        let child = PathBuf::from("/project/child");
        store
            .save(
                &parent.to_string_lossy(),
                &InvestigationSession {
                    planner: Some(AgentSession::builder().user("parent question").build()),
                    ..InvestigationSession::default()
                },
            )
            .unwrap();
        store
            .save(
                &child.to_string_lossy(),
                &InvestigationSession {
                    planner: Some(AgentSession::builder().user("child question").build()),
                    completed_reports: vec!["child finding".into()],
                    ..InvestigationSession::default()
                },
            )
            .unwrap();

        store.migrate_roots(&[child.clone()], &parent).unwrap();
        store.migrate_roots(&[child.clone()], &parent).unwrap();
        store.clear_roots(&[child.clone()]).unwrap();

        let migrated = store.load(&parent.to_string_lossy()).unwrap().unwrap();
        let planner_events = &migrated.planner.unwrap().events;
        assert!(matches!(
            &planner_events[0],
            crate::core::session::ConversationEvent::User(question)
                if question == "parent question"
        ));
        assert!(matches!(
            &planner_events[1],
            crate::core::session::ConversationEvent::User(question)
                if question == "child question"
        ));
        assert_eq!(migrated.completed_reports, vec!["child finding"]);
        assert_eq!(migrated.absorbed_roots, vec![child.clone()]);
        assert!(store.load(&child.to_string_lossy()).unwrap().is_none());
    }
}
