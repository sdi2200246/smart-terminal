mod client;
mod google;
mod groq;
pub mod openai_compatible;
mod openrouter;

pub use client::{ClientConfig, GenericLlmClient, ProviderCodec};
pub use google::{GoogleClient, GoogleError, GoogleProtocol};
pub use groq::{GroqClient, GroqError, GroqProtocol};
pub use openai_compatible::{
    Choice, JsonSchemaSpec, LlmStructuredOutput, LlmToolCall, Message, OpenAiError, OpenAiRequest,
    OpenAiResponse, ResponseFormat, Tool, ToolCall, ToolMetaData, build_messages_with_scratchpad,
    build_openai_request, build_openai_structured_request, build_tools,
};
pub use openrouter::{OpenRouterClient, OpenRouterError, OpenRouterProtocol};
