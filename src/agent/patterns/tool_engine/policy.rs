#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PermissionDecision {
    Approved,
    Denied(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum ToolPermissionMode {
    Prompt,
    Allow,
    Deny,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolExecutionPolicy {
    pub allow_safe_tools_without_prompt: bool,
    pub auto_execute_mutating_tools: bool,
    pub deny_mutating_tools: bool,
    pub blacklisted_tools: Vec<String>,
}

impl Default for ToolExecutionPolicy {
    fn default() -> Self {
        Self {
            allow_safe_tools_without_prompt: true,
            auto_execute_mutating_tools: false,
            deny_mutating_tools: false,
            blacklisted_tools: Vec::new(),
        }
    }
}

impl ToolPermissionMode {
    pub fn to_policy(self) -> ToolExecutionPolicy {
        match self {
            Self::Prompt => ToolExecutionPolicy::default(),
            Self::Allow => ToolExecutionPolicy {
                allow_safe_tools_without_prompt: true,
                auto_execute_mutating_tools: true,
                deny_mutating_tools: false,
                blacklisted_tools: Vec::new(),
            },
            Self::Deny => ToolExecutionPolicy {
                allow_safe_tools_without_prompt: true,
                auto_execute_mutating_tools: false,
                deny_mutating_tools: true,
                blacklisted_tools: Vec::new(),
            },
        }
    }
}

