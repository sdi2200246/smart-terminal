use super::AgentSignal;
use crate::core::{AgentToolCall, Capability, ToolMetaData};
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
    use crate::agent::{AgentLoopControl, AgentSignalOutcome};
    use crate::core::NoArgs;
    use crate::define_tool;
    use schemars::JsonSchema;
    use serde::Deserialize;
    use serde_json::json;

    define_tool! {
        pub struct FakeBashTool;
        /// bash tool
        fn bash() -> Result<String, ToolError> {
            Ok("bash".into())
        }
    }

    define_tool! {
        pub struct FakeDockerTool;
        /// docker tool
        fn docker() -> Result<String, ToolError> {
            Ok("docker".into())
        }
    }

    define_tool! {
        pub struct FakePlainTool;
        /// plain tool
        fn plain() -> Result<String, ToolError> {
            Ok("plain".into())
        }
    }

    define_tool! {
        pub struct FakeDupTool;
        /// dup_tool tool
        fn dup_tool() -> Result<String, ToolError> {
            Ok("dup_tool".into())
        }
    }

    define_tool! {
        pub struct FakeDupSigTool;
        /// dup_sig tool
        fn dup_sig() -> Result<String, ToolError> {
            Ok("dup_sig".into())
        }
    }

    #[derive(JsonSchema, Deserialize, Debug)]
    pub struct ReadFilePayload {
        pub p: String,
    }

    define_tool! {
        pub struct FakeReadFileTool;
        /// read_file tool
        fn read_file(args: ReadFilePayload) -> Result<String, ToolError> {
            Ok(args.p)
        }
    }

    define_signal! {
        pub struct FakeScratchpadSignal;
        /// update_scratchpad signal
        fn update_scratchpad(_args: NoArgs, ctx: &mut AgentSignalContext<'_>) -> Result<AgentSignalOutcome, ToolError> {
            let _ = ctx;
            Ok(AgentSignalOutcome {
                result: "update_scratchpad".into(),
                control: AgentLoopControl::Continue,
            })
        }
    }

    define_signal! {
        pub struct FakeDupSigSignal;
        /// dup_sig signal
        fn dup_sig(_args: NoArgs, ctx: &mut AgentSignalContext<'_>) -> Result<AgentSignalOutcome, ToolError> {
            let _ = ctx;
            Ok(AgentSignalOutcome {
                result: "dup_sig".into(),
                control: AgentLoopControl::Continue,
            })
        }
    }

    define_signal! {
        pub struct FakeDupToolSignal;
        /// dup_tool signal
        fn dup_tool(_args: NoArgs, ctx: &mut AgentSignalContext<'_>) -> Result<AgentSignalOutcome, ToolError> {
            let _ = ctx;
            Ok(AgentSignalOutcome {
                result: "dup_tool".into(),
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
        space.register_tool(Arc::new(FakeBashTool)).unwrap();
        space
            .register_signal(Arc::new(FakeScratchpadSignal))
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
        space.register_tool(Arc::new(FakeDockerTool)).unwrap();
        let names: Vec<&str> = space.metadata().iter().map(|m| m.name.as_str()).collect();
        assert_eq!(names, vec!["docker"]);
    }

    define_tool! {
        pub struct GuidedTool;
        guidance = "Use for its specialized task.";
        /// guided tool
        fn guided() -> Result<String, ToolError> {
            Ok(String::new())
        }
    }

    #[test]
    fn prompt_guidance_is_collected_from_registered_tools_and_signals() {
        let mut space = AgentActionSpace::new();
        space.register_tool(Arc::new(GuidedTool)).unwrap();
        space.register_tool(Arc::new(FakePlainTool)).unwrap();
        assert_eq!(
            space.prompt_guidance(),
            vec!["Tool 'guided': Use for its specialized task."]
        );
    }

    #[test]
    fn get_unknown_tool_returns_none() {
        let mut space = AgentActionSpace::new();
        space.register_tool(Arc::new(FakeBashTool)).unwrap();
        assert!(space.get("nonexistent").is_none());
    }

    #[test]
    fn duplicate_name_registration_returns_error() {
        let mut space = AgentActionSpace::new();
        space.register_tool(Arc::new(FakeDupTool)).unwrap();
        space
            .register_signal(Arc::new(FakeDupSigSignal))
            .unwrap();

        // tool-tool duplicate
        assert_eq!(
            space.register_tool(Arc::new(FakeDupTool)),
            Err(ActionSpaceError::DuplicateRegistration("dup_tool".into()))
        );
        // signal-signal duplicate
        assert_eq!(
            space.register_signal(Arc::new(FakeDupSigSignal)),
            Err(ActionSpaceError::DuplicateRegistration("dup_sig".into()))
        );
        // tool-signal duplicate (registering signal with existing tool name)
        assert_eq!(
            space.register_signal(Arc::new(FakeDupToolSignal)),
            Err(ActionSpaceError::DuplicateRegistration("dup_tool".into()))
        );
        // signal-tool duplicate (registering tool with existing signal name)
        assert_eq!(
            space.register_tool(Arc::new(FakeDupSigTool)),
            Err(ActionSpaceError::DuplicateRegistration("dup_sig".into()))
        );

        assert_eq!(space.model_schemas().len(), 2);
    }

    #[test]
    fn get_executes_correct_tool() {
        let mut space = AgentActionSpace::new();
        space.register_tool(Arc::new(FakeBashTool)).unwrap();
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
            .register_tool(Arc::new(FakeReadFileTool))
            .unwrap();
        space
            .register_signal(Arc::new(FakeScratchpadSignal))
            .unwrap();

        let calls = vec![
            AgentToolCall::new("read_file".into(), "c0".into(), json!({"p": "a"}), None),
            AgentToolCall::new(
                "update_scratchpad".into(),
                "c1".into(),
                json!({}),
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
                assert_eq!(inv.args, json!({}));
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
