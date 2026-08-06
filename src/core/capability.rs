use crate::tools::error::ToolError;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

pub struct ToolRegistry {
    tools: HashMap<&'static str, Box<dyn Capability>>,
    metadata: Vec<ToolMetaData>,
}

impl ToolRegistry {
    pub fn new(tools: Vec<Box<dyn Capability>>) -> ToolRegistry {
        let mut map = HashMap::new();
        let mut metadata = Vec::new();
        for t in tools {
            metadata.push(t.metadata());
            map.insert(t.name(), t);
        }
        return ToolRegistry {
            tools: map,
            metadata: metadata,
        };
    }
    pub fn get(&self, name: &str) -> Option<&dyn Capability> {
        self.tools.get(name).map(|b| b.as_ref())
    }

    pub fn metadata(&self) -> &Vec<ToolMetaData> {
        return &self.metadata;
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct ToolMetaData {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

pub trait Capability: Send + Sync {
    fn name(&self) -> &'static str;
    fn metadata(&self) -> ToolMetaData;
    fn execute(&self, args: Value) -> Result<String, ToolError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    struct FakeTool(&'static str);
    impl Capability for FakeTool {
        fn name(&self) -> &'static str {
            self.0
        }
        fn metadata(&self) -> ToolMetaData {
            ToolMetaData {
                name: self.0.into(),
                description: format!("{} tool", self.0),
                parameters: json!({"type": "object", "properties": {}}),
            }
        }
        fn execute(&self, _args: Value) -> Result<String, ToolError> {
            Ok(self.0.into())
        }
    }

    #[test]
    fn empty_registry_has_no_tools_or_metadata() {
        let reg = ToolRegistry::new(vec![]);
        assert!(reg.metadata().is_empty());
        assert!(reg.get("anything").is_none());
    }

    #[test]
    fn new_populates_both_map_and_metadata() {
        let reg = ToolRegistry::new(vec![
            Box::new(FakeTool("bash")),
            Box::new(FakeTool("read_dir")),
        ]);

        assert_eq!(reg.metadata().len(), 2);
        assert!(reg.get("bash").is_some());
        assert!(reg.get("read_dir").is_some());
    }

    #[test]
    fn metadata_names_match_inserted_tools() {
        let reg = ToolRegistry::new(vec![Box::new(FakeTool("docker"))]);
        let names: Vec<&str> = reg.metadata().iter().map(|m| m.name.as_str()).collect();
        assert_eq!(names, vec!["docker"]);
    }

    #[test]
    fn get_unknown_tool_returns_none() {
        let reg = ToolRegistry::new(vec![Box::new(FakeTool("bash"))]);
        assert!(reg.get("nonexistent").is_none());
    }

    #[test]
    fn duplicate_names_last_write_wins_in_map_but_metadata_keeps_both() {
        let reg = ToolRegistry::new(vec![Box::new(FakeTool("dup")), Box::new(FakeTool("dup"))]);

        // map collapses to one entry
        assert!(reg.get("dup").is_some());

        // metadata vec still has both — this is the drift risk worth knowing about
        assert_eq!(reg.metadata().len(), 2);
    }

    #[test]
    fn get_executes_correct_tool() {
        let reg = ToolRegistry::new(vec![Box::new(FakeTool("bash"))]);
        let result = reg.get("bash").unwrap().execute(json!({})).unwrap();
        assert_eq!(result, "bash");
    }
}
