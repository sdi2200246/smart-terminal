use crate::core::ModelName;

pub fn to_openrouter_model_string(model: ModelName) -> String {
    match model {
        // ModelName::Nemotron3_5SuperVlFree => "nvidia/nemotron-3.5-super-vl:free".into(),
        // ModelName::Llama3_3_70BFree => "meta-llama/llama-3.3-70b-instruct:free".into(),
        // ModelName::Gemini2_0FlashExpFree => "google/gemini-2.0-flash-exp:free".into(),
        // ModelName::Qwen2_5Coder32BFree => "qwen/qwen-2.5-coder-32b-instruct:free".into(),
        // ModelName::Mistral7BInstructFree => "mistralai/mistral-7b-instruct:free".into(),
        // ModelName::Llma3p18B => "meta-llama/llama-3.1-8b-instruct".into(),
        // ModelName::Gemini1_5Pro => "google/gemini-pro-1.5".into(),
        // ModelName::Gemini1_5Flash => "google/gemini-flash-1.5".into(),
        // ModelName::Gemini2_5Flash => "google/gemini-2.0-flash-001".into(),
        _ => "nvidia/nemotron-3.5-super-vl:free".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_model_names_to_openrouter_slugs() {
        assert_eq!(
            to_openrouter_model_string(ModelName::Nemotron3_5SuperVlFree),
            "nvidia/nemotron-3.5-super-vl:free"
        );
        assert_eq!(
            to_openrouter_model_string(ModelName::Llama3_3_70BFree),
            "meta-llama/llama-3.3-70b-instruct:free"
        );
        assert_eq!(
            to_openrouter_model_string(ModelName::Gemini2_0FlashExpFree),
            "google/gemini-2.0-flash-exp:free"
        );
        assert_eq!(
            to_openrouter_model_string(ModelName::Qwen2_5Coder32BFree),
            "qwen/qwen-2.5-coder-32b-instruct:free"
        );
        assert_eq!(
            to_openrouter_model_string(ModelName::Mistral7BInstructFree),
            "mistralai/mistral-7b-instruct:free"
        );
        assert_eq!(
            to_openrouter_model_string(ModelName::Llma3p18B),
            "meta-llama/llama-3.1-8b-instruct"
        );
        assert_eq!(
            to_openrouter_model_string(ModelName::Gemini2_5Flash),
            "google/gemini-2.0-flash-001"
        );
        assert_eq!(
            to_openrouter_model_string(ModelName::GptOss120B),
            "meta-llama/llama-3.3-70b-instruct:free"
        );
    }
}
