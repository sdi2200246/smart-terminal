use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use crate::core::memory::{Conversation, Interaction, Memory, MemoryError, MemoryIndex};

const INDEX_FILENAME: &str = "index.json";
const MEMORY_DIRNAME: &str = "memory";


pub struct FolderMemory {
    root: PathBuf,
    cwd: Option<PathBuf>,
    conversation: Option<Conversation>,
}

impl FolderMemory {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            cwd: None,
            conversation: None,
        }
    }

   pub fn project_local() -> Result<Self, MemoryError> {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(MEMORY_DIRNAME);
        fs::create_dir_all(&root)?;
        Ok(Self::new(root))
    }
    
    pub fn with_conversation(
        root: impl Into<PathBuf>,
        cwd: impl Into<PathBuf>,
        conv: Conversation,
    ) -> Self {
        Self {
            root: root.into(),
            cwd: Some(cwd.into()),
            conversation: Some(conv),
        }
    }

    fn index_path(&self) -> PathBuf {
        self.root.join(INDEX_FILENAME)
    }

    fn conv_path(&self, filename: &str) -> PathBuf {
        self.root.join(filename)
    }

    fn load_index(&self) -> Result<MemoryIndex, MemoryError> {
        let path = self.index_path();
        if !path.exists() {
            return Ok(MemoryIndex::default());
        }
        let raw = fs::read_to_string(&path)?;
        Ok(serde_json::from_str(&raw)?)
    }

    fn save_index(&self, index: &MemoryIndex) -> Result<(), MemoryError> {
        let json = serde_json::to_string_pretty(index)?;
        atomic_write(&self.index_path(), &json)
    }

    fn persist(&self) -> Result<(), MemoryError> {
        let cwd = self.cwd.as_ref().ok_or(MemoryError::NotLoaded)?;
        let conv = self.conversation.as_ref().ok_or(MemoryError::NotLoaded)?;

        let index = self.load_index()?;
        let filename = index.resolve(cwd).ok_or(MemoryError::NotRegistered)?;
        let path = self.conv_path(&filename);

        let json = serde_json::to_string_pretty(conv)?;
        atomic_write(&path, &json)
    }
}

impl Memory for FolderMemory {
    fn load(&mut self, cwd: &Path) -> Result<bool, MemoryError> {
        let index = self.load_index()?;
        let Some(filename) = index.resolve(cwd) else {
            self.cwd = Some(cwd.to_path_buf());
            self.conversation = None;
            return Ok(false);
        };
        let path = self.conv_path(&filename);
        let conv = if path.exists() {
            serde_json::from_str(&fs::read_to_string(&path)?)?
        } else {
            Conversation::default()
        };
        self.cwd = Some(cwd.to_path_buf());
        self.conversation = Some(conv);
        Ok(true)
    }

    fn current(&self) -> Option<&Conversation> {
        self.conversation.as_ref()
    }

    fn append(&mut self, entry: Interaction) -> Result<(), MemoryError> {
        let conv = self.conversation.as_mut().ok_or(MemoryError::NotRegistered)?;
        conv.push(entry);
        self.persist()
    }

    fn register(&mut self, cwd: &Path) -> Result<(), MemoryError> {
        let mut index = self.load_index()?;
        let cwd_owned = cwd.to_path_buf();

        if let Some(filename) = index.folders.get(&cwd_owned).cloned() {
            let conv_path = self.conv_path(&filename);
            let conv = if conv_path.exists() {
                serde_json::from_str(&fs::read_to_string(&conv_path)?)?
            } else {
                Conversation::default()
            };
            self.cwd = Some(cwd_owned);
            self.conversation = Some(conv);
            return Ok(());
        }

        if let Some(existing) = index.ancestor_of(&cwd_owned) {
            return Err(MemoryError::OverlapsExisting(existing));
        }

        for desc in index.descendants_of(&cwd_owned) {
            if let Some(filename) = index.folders.remove(&desc) {
                let path = self.conv_path(&filename);
                if path.exists() {
                    fs::remove_file(path)?;
                }
            }
        }

        let filename = slugify(&cwd_owned);
        index.folders.insert(cwd_owned.clone(), filename.clone());
        self.save_index(&index)?;

        let conv_path = self.conv_path(&filename);
        if !conv_path.exists() {
            let empty = serde_json::to_string_pretty(&Conversation::default())?;
            atomic_write(&conv_path, &empty)?;
        }

        self.cwd = Some(cwd_owned);
        self.conversation = Some(Conversation::default());
        Ok(())
    }

    fn unregister(&mut self, cwd: &Path) -> Result<(), MemoryError> {
        let mut index = self.load_index()?;
        if let Some(filename) = index.folders.remove(cwd) {
            self.save_index(&index)?;
            let path = self.conv_path(&filename);
            if path.exists() {
                fs::remove_file(path)?;
            }
        }
        self.cwd = None;
        self.conversation = None;
        Ok(())
    }
}


fn atomic_write(path: &Path, contents: &str) -> Result<(), MemoryError> {
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, contents)?;
    fs::rename(tmp, path)?;
    Ok(())
}

fn slugify(path: &Path) -> String {
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("root")
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '-' })
        .collect::<String>();

    let mut hasher = DefaultHasher::new();
    path.hash(&mut hasher);
    let hash = hasher.finish();

    format!("{}-{:x}.json", name, hash & 0xFFFF)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn fresh() -> (FolderMemory, TempDir) {
        let tmp = TempDir::new().unwrap();
        let mem = FolderMemory::new(tmp.path());
        (mem, tmp)
    }

    fn entry(input: &str, cmd: &str) -> Interaction {
        Interaction {
            user_input: input.into(),
            predicted_cmd: cmd.into(),
            timestamp: 0,
        }
    }

    #[test]
    fn load_unregistered_returns_false() {
        let (mut mem, _tmp) = fresh();
        let loaded = mem.load(Path::new("/random")).unwrap();
        assert!(!loaded);
        assert!(mem.current().is_none());
    }

    #[test]
    fn register_then_current_returns_empty() {
        let (mut mem, _tmp) = fresh();
        mem.register(Path::new("/proj/foo")).unwrap();
        assert!(mem.current().unwrap().interactions.is_empty());
    }

    #[test]
    fn append_updates_in_memory_state() {
        let (mut mem, _tmp) = fresh();
        mem.register(Path::new("/proj/foo")).unwrap();
        mem.append(entry("ls", "ls -la")).unwrap();
        mem.append(entry("git st", "git status")).unwrap();

        let conv = mem.current().unwrap();
        assert_eq!(conv.interactions.len(), 2);
        assert_eq!(conv.interactions[1].predicted_cmd, "git status");
    }

    #[test]
    fn append_persists_to_disk_across_instances() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().to_path_buf();

        {
            let mut mem = FolderMemory::new(&root);
            mem.register(Path::new("/proj/foo")).unwrap();
            mem.append(entry("hello", "echo hello")).unwrap();
        }

        let mut mem2 = FolderMemory::new(&root);
        let loaded = mem2.load(Path::new("/proj/foo")).unwrap();
        assert!(loaded);
        let conv = mem2.current().unwrap();
        assert_eq!(conv.interactions.len(), 1);
        assert_eq!(conv.interactions[0].predicted_cmd, "echo hello");
    }

    #[test]
    fn append_before_register_errors() {
        let (mut mem, _tmp) = fresh();
        let result = mem.append(entry("a", "b"));
        assert!(matches!(result, Err(MemoryError::NotRegistered)));
    }

    #[test]
    fn longest_prefix_match_finds_parent() {
        let (mut mem, _tmp) = fresh();
        mem.register(Path::new("/proj/foo")).unwrap();
        let loaded = mem.load(Path::new("/proj/foo/src/agent")).unwrap();
        assert!(loaded);
        assert!(mem.current().unwrap().interactions.is_empty());
    }

    #[test]
    fn with_conversation_seeds_in_memory_state_only() {
        let tmp = TempDir::new().unwrap();
        let seed = Conversation { interactions: vec![entry("prev", "ls")] };
        let mem = FolderMemory::with_conversation(tmp.path(), Path::new("/x"), seed);

        let conv = mem.current().unwrap();
        assert_eq!(conv.interactions.len(), 1);
        assert_eq!(conv.interactions[0].predicted_cmd, "ls");
        assert!(!tmp.path().join(INDEX_FILENAME).exists());
    }

    #[test]
    fn unregister_clears_state_and_file() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().to_path_buf();

        let mut mem = FolderMemory::new(&root);
        mem.register(Path::new("/proj/foo")).unwrap();
        mem.append(entry("a", "b")).unwrap();
        mem.unregister(Path::new("/proj/foo")).unwrap();

        assert!(mem.current().is_none());

        let mut mem2 = FolderMemory::new(&root);
        let loaded = mem2.load(Path::new("/proj/foo")).unwrap();
        assert!(!loaded);
    }

    #[test]
    fn register_subfolder_of_existing_fails() {
        let (mut mem, _tmp) = fresh();
        mem.register(Path::new("/proj/foo")).unwrap();

        let result = mem.register(Path::new("/proj/foo/src"));
        match result {
            Err(MemoryError::OverlapsExisting(p)) => assert_eq!(p, Path::new("/proj/foo")),
            other => panic!("expected OverlapsExisting, got {other:?}"),
        }
    }

    #[test]
    fn register_parent_replaces_existing_children() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().to_path_buf();

        let mut mem = FolderMemory::new(&root);
        mem.register(Path::new("/proj/foo/src")).unwrap();
        mem.append(entry("a", "ls")).unwrap();
        mem.register(Path::new("/proj/foo/tests")).unwrap();
        mem.append(entry("b", "cargo test")).unwrap();

        mem.register(Path::new("/proj/foo")).unwrap();
        assert!(mem.current().unwrap().interactions.is_empty());

        let mut mem2 = FolderMemory::new(&root);
        let loaded_src = mem2.load(Path::new("/proj/foo/src")).unwrap();
        assert!(loaded_src);
        assert!(mem2.current().unwrap().interactions.is_empty());
    }

    #[test]
    fn register_exact_match_refreshes_in_memory_state() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().to_path_buf();

        {
            let mut mem = FolderMemory::new(&root);
            mem.register(Path::new("/proj/foo")).unwrap();
            mem.append(entry("hello", "echo hello")).unwrap();
        }

        let mut mem = FolderMemory::new(&root);
        mem.register(Path::new("/proj/foo")).unwrap();

        let conv = mem.current().unwrap();
        assert_eq!(conv.interactions.len(), 1);
        assert_eq!(conv.interactions[0].predicted_cmd, "echo hello");
    }
    #[test]
    fn project_local_resolves_under_manifest_dir() {
        let mem = FolderMemory::project_local().unwrap();
        let expected_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("memory");
        assert_eq!(mem.root, expected_root);
        assert!(expected_root.exists(), "project_local should create the directory");
    }
}