use super::AgentSignal;
use crate::core::capability::{Capability, ToolMetaData};
use crate::core::responce::AgentToolCall;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;

#[derive(Clone)]
pub enum AgentAction {
    Tool(Arc<dyn Capability>),
    Signal(Arc<dyn AgentSignal>),
}

impl AgentAction {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Tool(cap) => cap.name(),
            Self::Signal(sig) => sig.name(),
        }
    }

    pub fn metadata(&self) -> ToolMetaData {
        match self {
            Self::Tool(cap) => cap.metadata(),
            Self::Signal(sig) => sig.metadata(),
        }
    }

    pub fn prompt_guidance(&self) -> Option<&'static str> {
        match self {
            Self::Tool(cap) => cap.prompt_guidance(),
            Self::Signal(sig) => sig.prompt_guidance(),
        }
    }
}

/// One decoded call. `pos` is the call's index in the original batch.
#[derive(Clone)]
pub struct ToolInvocation {
    pub pos: usize,
    pub cap: Arc<dyn Capability>,
    pub args: Value,
}

#[derive(Clone)]
pub struct AgentSignalInvocation {
    pub pos: usize,
    pub sig: Arc<dyn AgentSignal>,
    pub args: Value,
}

pub enum ModelCall {
    Tool(ToolInvocation),
    Signal(AgentSignalInvocation),
    Unknown { pos: usize, name: String },
}

#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum ActionSpaceError {
    #[error("Action '{0}' is already registered")]
    DuplicateRegistration(String),
}

#[derive(Default)]
pub struct AgentActionSpace {
    entries: HashMap<String, AgentAction>,
    metadata: Vec<ToolMetaData>,
}

impl AgentActionSpace {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            metadata: Vec::new(),
        }
    }

    pub fn register_tool(&mut self, cap: Arc<dyn Capability>) -> Result<(), ActionSpaceError> {
        let name = cap.name().to_string();
        if self.entries.contains_key(&name) {
            return Err(ActionSpaceError::DuplicateRegistration(name));
        }
        self.metadata.push(cap.metadata());
        self.entries.insert(name, AgentAction::Tool(cap));
        Ok(())
    }

    pub fn register_signal(&mut self, sig: Arc<dyn AgentSignal>) -> Result<(), ActionSpaceError> {
        let name = sig.name().to_string();
        if self.entries.contains_key(&name) {
            return Err(ActionSpaceError::DuplicateRegistration(name));
        }
        self.metadata.push(sig.metadata());
        self.entries.insert(name, AgentAction::Signal(sig));
        Ok(())
    }

    pub fn get(&self, name: &str) -> Option<&AgentAction> {
        self.entries.get(name)
    }

    /// Outbound: both kinds, merged. This is the list the model sees.
    pub fn model_schemas(&self) -> Vec<ToolMetaData> {
        self.metadata.clone()
    }

    pub fn metadata(&self) -> &[ToolMetaData] {
        &self.metadata
    }

    pub fn prompt_guidance(&self) -> Vec<String> {
        self.metadata
            .iter()
            .filter_map(|metadata| {
                self.entries
                    .get(metadata.name.as_str())
                    .and_then(|entry| entry.prompt_guidance())
                    .map(|guidance| format!("Tool '{}': {}", metadata.name, guidance))
            })
            .collect()
    }

    /// Inbound: classify every call once. The result has one entry per call and
    /// each entry carries its original position.
    pub fn decode(&self, calls: &[AgentToolCall]) -> Vec<ModelCall> {
        calls
            .iter()
            .enumerate()
            .map(|(pos, call)| match self.entries.get(call.name()) {
                Some(AgentAction::Tool(cap)) => ModelCall::Tool(ToolInvocation {
                    pos,
                    cap: Arc::clone(cap),
                    args: call.arguments(),
                }),
                Some(AgentAction::Signal(sig)) => ModelCall::Signal(AgentSignalInvocation {
                    pos,
                    sig: Arc::clone(sig),
                    args: call.arguments(),
                }),
                None => ModelCall::Unknown {
                    pos,
                    name: call.name().to_string(),
                },
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::signals::{AgentLoopControl, AgentSignalContext, AgentSignalOutcome};
    use crate::core::capability::ToolSafety;
    use crate::tools::error::ToolError;
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
                safety: ToolSafety::Safe,
            }
        }
        fn execute(&self, _args: Value) -> Result<String, ToolError> {
            Ok(self.0.into())
        }
    }

    struct FakeSignal(&'static str);
    impl AgentSignal for FakeSignal {
        fn name(&self) -> &'static str {
            self.0
        }
        fn metadata(&self) -> ToolMetaData {
            ToolMetaData {
                name: self.0.into(),
                description: format!("{} signal", self.0),
                parameters: json!({"type": "object", "properties": {}}),
                safety: ToolSafety::Safe,
            }
        }
        fn handle(
            &self,
            _args: Value,
            _ctx: &mut AgentSignalContext<'_>,
        ) -> Result<AgentSignalOutcome, ToolError> {
            Ok(AgentSignalOutcome {
                result: self.0.into(),
                control: AgentLoopControl::Continue,
            })
        }
    }

    #[test]
    fn empty_action_space_has_no_tools_or_metadata() {
        let space = AgentActionSpace::new();
        assert!(space.model_schemas().is_empty());
        assert!(space.get("anything").is_none());
    }

    #[test]
    fn register_populates_entries_and_model_schemas_for_both_kinds() {
        let mut space = AgentActionSpace::new();
        space.register_tool(Arc::new(FakeTool("bash"))).unwrap();
        space
            .register_signal(Arc::new(FakeSignal("update_scratchpad")))
            .unwrap();

        let schemas = space.model_schemas();
        assert_eq!(schemas.len(), 2);
        let names: Vec<&str> = schemas.iter().map(|m| m.name.as_str()).collect();
        assert_eq!(names, vec!["bash", "update_scratchpad"]);
        assert!(matches!(space.get("bash"), Some(AgentAction::Tool(_))));
        assert!(matches!(
            space.get("update_scratchpad"),
            Some(AgentAction::Signal(_))
        ));
    }

    #[test]
    fn metadata_names_match_inserted_tools() {
        let mut space = AgentActionSpace::new();
        space.register_tool(Arc::new(FakeTool("docker"))).unwrap();
        let names: Vec<&str> = space.metadata().iter().map(|m| m.name.as_str()).collect();
        assert_eq!(names, vec!["docker"]);
    }

    struct GuidedTool;
    impl Capability for GuidedTool {
        fn name(&self) -> &'static str {
            "guided"
        }
        fn metadata(&self) -> ToolMetaData {
            ToolMetaData {
                name: self.name().into(),
                description: "guided tool".into(),
                parameters: json!({"type": "object", "properties": {}}),
                safety: ToolSafety::Safe,
            }
        }
        fn prompt_guidance(&self) -> Option<&'static str> {
            Some("Use for its specialized task.")
        }
        fn execute(&self, _args: Value) -> Result<String, ToolError> {
            Ok(String::new())
        }
    }

    #[test]
    fn prompt_guidance_is_collected_from_registered_tools_and_signals() {
        let mut space = AgentActionSpace::new();
        space.register_tool(Arc::new(GuidedTool)).unwrap();
        space.register_tool(Arc::new(FakeTool("plain"))).unwrap();
        assert_eq!(
            space.prompt_guidance(),
            vec!["Tool 'guided': Use for its specialized task."]
        );
    }

    #[test]
    fn get_unknown_tool_returns_none() {
        let mut space = AgentActionSpace::new();
        space.register_tool(Arc::new(FakeTool("bash"))).unwrap();
        assert!(space.get("nonexistent").is_none());
    }

    #[test]
    fn duplicate_name_registration_returns_error() {
        let mut space = AgentActionSpace::new();
        space.register_tool(Arc::new(FakeTool("dup_tool"))).unwrap();
        space
            .register_signal(Arc::new(FakeSignal("dup_sig")))
            .unwrap();

        // tool-tool duplicate
        assert_eq!(
            space.register_tool(Arc::new(FakeTool("dup_tool"))),
            Err(ActionSpaceError::DuplicateRegistration("dup_tool".into()))
        );
        // signal-signal duplicate
        assert_eq!(
            space.register_signal(Arc::new(FakeSignal("dup_sig"))),
            Err(ActionSpaceError::DuplicateRegistration("dup_sig".into()))
        );
        // tool-signal duplicate (registering signal with existing tool name)
        assert_eq!(
            space.register_signal(Arc::new(FakeSignal("dup_tool"))),
            Err(ActionSpaceError::DuplicateRegistration("dup_tool".into()))
        );
        // signal-tool duplicate (registering tool with existing signal name)
        assert_eq!(
            space.register_tool(Arc::new(FakeTool("dup_sig"))),
            Err(ActionSpaceError::DuplicateRegistration("dup_sig".into()))
        );

        assert_eq!(space.model_schemas().len(), 2);
    }

    #[test]
    fn get_executes_correct_tool() {
        let mut space = AgentActionSpace::new();
        space.register_tool(Arc::new(FakeTool("bash"))).unwrap();
        let Some(AgentAction::Tool(tool)) = space.get("bash") else {
            panic!("expected tool");
        };
        let result = tool.execute(json!({})).unwrap();
        assert_eq!(result, "bash");
    }

    #[test]
    fn decode_classifies_calls_and_preserves_positions() {
        let mut space = AgentActionSpace::new();
        space
            .register_tool(Arc::new(FakeTool("read_file")))
            .unwrap();
        space
            .register_signal(Arc::new(FakeSignal("update_scratchpad")))
            .unwrap();

        let calls = vec![
            AgentToolCall::new("read_file".into(), "c0".into(), json!({"p": "a"}), None),
            AgentToolCall::new(
                "update_scratchpad".into(),
                "c1".into(),
                json!({"s": 1}),
                None,
            ),
            AgentToolCall::new("unknown_op".into(), "c2".into(), json!({}), None),
        ];

        let decoded = space.decode(&calls);
        assert_eq!(decoded.len(), 3);

        match &decoded[0] {
            ModelCall::Tool(inv) => {
                assert_eq!(inv.pos, 0);
                assert_eq!(inv.cap.name(), "read_file");
                assert_eq!(inv.args, json!({"p": "a"}));
            }
            _ => panic!("expected Tool at index 0"),
        }

        match &decoded[1] {
            ModelCall::Signal(inv) => {
                assert_eq!(inv.pos, 1);
                assert_eq!(inv.sig.name(), "update_scratchpad");
                assert_eq!(inv.args, json!({"s": 1}));
            }
            _ => panic!("expected Signal at index 1"),
        }

        match &decoded[2] {
            ModelCall::Unknown { pos, name } => {
                assert_eq!(*pos, 2);
                assert_eq!(name, "unknown_op");
            }
            _ => panic!("expected Unknown at index 2"),
        }
    }
}

