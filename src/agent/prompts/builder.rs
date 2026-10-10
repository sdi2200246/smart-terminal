use crate::core::capability::ToolMetaData;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PromptSection {
    Role,
    Objective,
    Workflow,
    ToolPolicy,
    ToolUse,
    AvailableTools,
    ToolGuidance,
    Context,
    Completion,
    Output,
}

impl PromptSection {
    const ORDER: [Self; 10] = [
        Self::Role,
        Self::Objective,
        Self::Workflow,
        Self::ToolPolicy,
        Self::ToolUse,
        Self::AvailableTools,
        Self::ToolGuidance,
        Self::Context,
        Self::Completion,
        Self::Output,
    ];
}

impl PromptSection {
    fn heading(self) -> &'static str {
        match self {
            Self::Role => "ROLE",
            Self::Objective => "OBJECTIVE",
            Self::Workflow => "WORKFLOW",
            Self::ToolPolicy => "TOOL-CALL POLICY",
            Self::ToolUse => "TOOL-USE RULES",
            Self::AvailableTools => "AVAILABLE TOOLS",
            Self::ToolGuidance => "AVAILABLE-TOOL GUIDANCE",
            Self::Context => "ENVIRONMENT CONTEXT",
            Self::Completion => "COMPLETION POLICY",
            Self::Output => "FINAL-ANSWER SCHEMA",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct PromptPart {
    pub section: PromptSection,
    pub content: &'static str,
}

impl PromptPart {
    pub const fn new(section: PromptSection, content: &'static str) -> Self {
        Self { section, content }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct AgentPrompt {
    pub role: &'static str,
    pub sections: &'static [PromptPart],
}

impl AgentPrompt {
    pub const fn new(role: &'static str, sections: &'static [PromptPart]) -> Self {
        Self { role, sections }
    }
}

pub struct SystemPromptBuilder {
    prompt: AgentPrompt,
    available_tools: Vec<ToolMetaData>,
    tool_guidance: Vec<String>,
}

impl SystemPromptBuilder {
    pub fn new(prompt: AgentPrompt) -> Self {
        Self {
            prompt,
            available_tools: Vec::new(),
            tool_guidance: Vec::new(),
        }
    }

    pub fn available_tools(mut self, tools: &[ToolMetaData]) -> Self {
        self.available_tools = tools.to_vec();
        self
    }

    pub fn tool_guidance<I, S>(mut self, guidance: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let guidance = guidance
            .into_iter()
            .map(|line| line.as_ref().to_string())
            .filter(|line| !line.trim().is_empty())
            .collect::<Vec<_>>();

        self.tool_guidance = guidance;
        self
    }

    pub fn build(self) -> String {
        let mut sections = Vec::new();
        for section in PromptSection::ORDER {
            let content = if section == PromptSection::ToolGuidance {
                if self.tool_guidance.is_empty() {
                    continue;
                }
                self.tool_guidance.join("\n")
            } else if section == PromptSection::AvailableTools {
                if self.available_tools.is_empty() {
                    continue;
                }
                self.available_tools
                    .iter()
                    .map(|tool| format!("- `{}`: {}", tool.name, tool.description))
                    .collect::<Vec<_>>()
                    .join("\n")
            } else if section == PromptSection::Output {
                let Some(final_answer) = self
                    .available_tools
                    .iter()
                    .find(|tool| tool.name == "final_answer")
                else {
                    continue;
                };
                format!(
                    "The `final_answer` tool accepts this JSON schema. Follow its field descriptions and constraints exactly:\n{}",
                    serde_json::to_string_pretty(&final_answer.parameters)
                        .expect("tool parameters are valid JSON")
                )
            } else {
                let contents = self
                    .prompt
                    .sections
                    .iter()
                    .filter(|part| part.section == section)
                    .map(|part| part.content)
                    .collect::<Vec<_>>();
                if contents.is_empty() {
                    continue;
                }
                contents.join("\n\n")
            };
            if !content.trim().is_empty() {
                sections.push(format!("[{}]\n\n{}", section.heading(), content));
            }
        }
        format!(
            "Agent role: {}\n\n{}",
            self.prompt.role,
            sections.join("\n\n")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assembles_selected_sections_in_canonical_order() {
        static PARTS: [PromptPart; 3] = [
            PromptPart::new(PromptSection::Output, "Return a command."),
            PromptPart::new(PromptSection::Role, "You predict commands."),
            PromptPart::new(PromptSection::Workflow, "Use context first."),
        ];
        let tools = [
            ToolMetaData {
                name: "final_answer".into(),
                description: "Submit the final result.".into(),
                parameters: serde_json::json!({
                    "type": "object",
                    "title": "CommandAnswer",
                    "properties": {"cmd": {"type": "string"}}
                }),
            },
            ToolMetaData {
                name: "inspect".into(),
                description: "Inspect current state.".into(),
                parameters: serde_json::json!({"type": "object"}),
            },
        ];
        let prompt = SystemPromptBuilder::new(AgentPrompt::new("predictor", &PARTS))
            .available_tools(&tools)
            .tool_guidance(["Docker trigger.", "Diff trigger."])
            .build();

        let role = prompt.find("[ROLE]").unwrap();
        let workflow = prompt.find("[WORKFLOW]").unwrap();
        let available = prompt.find("[AVAILABLE TOOLS]").unwrap();
        let guidance = prompt.find("[AVAILABLE-TOOL GUIDANCE]").unwrap();
        let output = prompt.find("[FINAL-ANSWER SCHEMA]").unwrap();
        assert!(
            role < workflow && workflow < available && available < guidance && guidance < output
        );
        assert!(prompt.contains("`inspect`: Inspect current state."));
        assert!(prompt.contains("\"title\": \"CommandAnswer\""));
    }

    #[test]
    fn omits_empty_tool_guidance_section() {
        static PARTS: [PromptPart; 1] = [PromptPart::new(PromptSection::Role, "Keep all details.")];
        let prompt = SystemPromptBuilder::new(AgentPrompt::new("planner", &PARTS))
            .tool_guidance(std::iter::empty::<&str>())
            .build();

        assert!(prompt.contains("Keep all details."));
        assert!(!prompt.contains("[AVAILABLE-TOOL GUIDANCE]"));
    }

    #[test]
    fn registered_tool_descriptions_are_rendered_dynamically() {
        let metadata = [ToolMetaData {
            name: "sample".into(),
            description: "A task-specific tool.".into(),
            parameters: serde_json::json!({"type":"object"}),
        }];
        let prompt = SystemPromptBuilder::new(AgentPrompt::new("worker", &[]))
            .available_tools(&metadata)
            .build();
        assert!(prompt.contains("[AVAILABLE TOOLS]"));
        assert!(prompt.contains("`sample`: A task-specific tool."));
    }

    #[test]
    fn omits_output_contract_without_final_answer_tool_schema() {
        let prompt = SystemPromptBuilder::new(AgentPrompt::new("worker", &[])).build();
        assert!(!prompt.contains("[FINAL-ANSWER SCHEMA]"));
    }
}
