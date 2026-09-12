use serde_json::Value;

#[derive(Debug, Clone)]
pub struct AgentToolCall {
    id: String,
    arguments: Value,
    name: String,
    thinking_state: Option<String>,
}
impl AgentToolCall {
    pub fn new(name: String, id: String, arguments: Value, thinking_state: Option<String>) -> Self {
        Self {
            name,
            id,
            arguments,
            thinking_state,
        }
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn arguments(&self) -> Value {
        self.arguments.clone()
    }
    pub fn into_arguments(self) -> Value {
        self.arguments
    }
    pub fn thinking_state(&self) -> Option<String> {
        self.thinking_state.clone()
    }
}



#[derive(Debug, Clone, Default)]
pub struct AgentResponse {
    calls: Vec<AgentToolCall>,
}

impl AgentResponse {
    pub fn new() -> Self {
        Self { calls: Vec::new() }
    }

    pub fn single(call: AgentToolCall) -> Self {
        Self { calls: vec![call] }
    }

    pub fn push(&mut self, call: AgentToolCall) {
        self.calls.push(call);
    }

    pub fn calls(&self) -> &[AgentToolCall] {
        &self.calls
    }

    pub fn into_calls(self) -> Vec<AgentToolCall> {
        self.calls
    }

    pub fn len(&self) -> usize {
        self.calls.len()
    }

    pub fn is_empty(&self) -> bool {
        self.calls.is_empty()
    }

    pub fn is_stop(&self) -> bool {
        return matches!(self.calls.as_slice(), [call] if call.name() == "stop")
    }
}

impl From<Vec<AgentToolCall>> for AgentResponse {
    fn from(calls: Vec<AgentToolCall>) -> Self {
        Self { calls }
    }
}

impl IntoIterator for AgentResponse {
    type Item = AgentToolCall;
    type IntoIter = std::vec::IntoIter<AgentToolCall>;
    fn into_iter(self) -> Self::IntoIter {
        self.calls.into_iter()
    }
}