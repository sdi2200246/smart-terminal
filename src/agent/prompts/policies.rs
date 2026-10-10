pub const TOOL_CALL_ORDER: &str = r####"
If a tool call depends on information from another tool, wait for that result before making the dependent call. Independent tool calls may be requested in the same response. The runtime executes calls in a batch sequentially and returns their results together; it does not execute them concurrently.
"####;

pub const FINAL_ANSWER_TOOL: &str = r####"
When you have completed the task, call `final_answer` once with the complete result in the required output shape. Do not call more tools after submitting the final answer.
"####;

pub const STRUCTURED_STOP: &str = r####"
When you have completed the task, return the complete result in the required output shape and finish normally. Do not call `final_answer`; the runtime will pass the final payload through its structured-output flow.
"####;

pub const FINAL_ANSWER_OR_STRUCTURED_STOP: &str = r####"
When you have completed the task, submit the complete result exactly once. If the `final_answer` tool is available, call it with the required output shape. Otherwise, return the complete result in the required output shape and finish normally so the runtime can use its structured-output flow. Do not submit the same result through both paths.
"####;

pub const ENVIRONMENT_CONTEXT: &str = r####"
The `Context:` system message may include:
- `shell`: the active shell; match its syntax when relevant.
- `os`: the operating system; account for platform-specific tools and flags.
- `cwd` and `cwd_contents`: the working directory and its top-level entries.
- `history`: recent shell commands, most recent last.
- `shell_tools`: installed command-line tools and their versions.

Treat provided context values as ground truth. Use only the fields present in the context.
"####;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_call_batch_policy_matches_runtime_execution() {
        assert!(TOOL_CALL_ORDER.contains("executes calls in a batch sequentially"));
        assert!(TOOL_CALL_ORDER.contains("does not execute them concurrently"));
    }

    #[test]
    fn shared_completion_policy_uses_the_runtime_final_answer_tool() {
        assert!(FINAL_ANSWER_TOOL.contains("call `final_answer` once"));
        assert!(FINAL_ANSWER_TOOL.contains("required output shape"));
    }

    #[test]
    fn structured_stop_policy_selects_the_runtime_structuring_flow() {
        assert!(STRUCTURED_STOP.contains("Do not call `final_answer`"));
        assert!(STRUCTURED_STOP.contains("structured-output flow"));
    }

    #[test]
    fn combined_completion_policy_selects_one_supported_path() {
        assert!(
            FINAL_ANSWER_OR_STRUCTURED_STOP.contains("If the `final_answer` tool is available")
        );
        assert!(
            FINAL_ANSWER_OR_STRUCTURED_STOP
                .contains("Do not submit the same result through both paths")
        );
    }
}
