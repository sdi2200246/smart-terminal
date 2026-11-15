use crate::core::{AgentRequest, ModelName};
use crate::providers::openai_compatible::{build_openai_request, OpenAiRequest};

pub fn to_groq_model_string(model: ModelName) -> String {
    match model {
        ModelName::GptOss120B => "openai/gpt-oss-120b".into(),
        ModelName::Llma3p18B => "llama-3.1-8b-instant".into(),
        _ => "openai/gpt-oss-120b".into(),
    }
}

impl From<&AgentRequest<'_>> for OpenAiRequest {
    fn from(request: &AgentRequest<'_>) -> Self {
        build_openai_request(request, to_groq_model_string(request.model.get_name()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{AgentRequest, AgentSession, Model, ModelName, Scratchpad};

    #[test]
    fn groq_request_includes_the_current_scratchpad() {
        let mut session = AgentSession::builder()
            .system("Investigator prompt")
            .user("continue investigation")
            .build();
        session.update_scratchpad(Scratchpad {
            main_goal: "answer question".into(),
            completed_milestones: vec!["inspect config".into()],
            current_focus: "check tool registration".into(),
            vital_findings: vec!["executor supports scratchpad".into()],
        });
        let model = Model::with_default_temp(ModelName::GptOss120B);
        let request = AgentRequest {
            model: &model,
            session: &session,
            tools_metadata: &[],
        };

        let groq_request = OpenAiRequest::from(&request);
        let scratchpad_message = &groq_request.messages[1];
        assert_eq!(scratchpad_message.role, "system");
        assert!(
            scratchpad_message
                .content
                .as_deref()
                .unwrap()
                .contains("check tool registration")
        );
    }
}
