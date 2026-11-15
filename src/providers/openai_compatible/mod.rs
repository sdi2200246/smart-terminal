pub mod adapters;
pub mod error;
pub mod protocol;

pub use adapters::{
    build_messages_with_scratchpad, build_openai_request, build_openai_structured_request,
    build_tools,
};
pub use error::OpenAiError;
pub use protocol::{
    Choice, JsonSchemaSpec, LlmStructuredOutput, LlmToolCall, Message, OpenAiRequest,
    OpenAiResponse, ResponseFormat, Tool, ToolCall, ToolMetaData,
};
