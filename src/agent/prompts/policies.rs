pub const TOOL_CALL_ORDER: &str = r####"
If a tool call depends on information from another tool, wait for that result before making the dependent call. Independent tool calls should be requested together in the same response; the runtime executes them concurrently as background jobs so slow commands never block your investigation.
- Fast calls return their result in the same turn; slow calls return a pending status with a Job ID and keep running in the background.
- Start potentially slow commands early alongside other independent calls.
- When a call returns a pending Job ID, DO NOT call `wait_job` immediately if you still have other independent steps or tool calls you can run first. Keep advancing the investigation with other tools while the background job runs.
- Only call `wait_job` when you have no other independent work left or when your next step strictly depends on the pending job's output. `wait_job` must be called on its own with no other calls in that turn.
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

pub const STRUCTURING_SYSTEM_PROMPT: &str =
    "Your one and ONLY job is to return the following text into the scheema provided to you";

pub const ENVIRONMENT_CONTEXT: &str = r####"
The `Context:` system message may include:
- `shell`: the active shell; match its syntax when relevant.
- `os`: the operating system; account for platform-specific tools and flags.
- `cwd` and `cwd_contents`: the working directory and its top-level entries.
- `history`: recent shell commands, most recent last.
- `shell_tools`: installed command-line tools and their versions.

Treat provided context values as ground truth. Use only the fields present in the context.
"####;

pub const FINAL_ANSWER_DESCRIPTION: &str = "Submits the final, structured JSON response to the user. Use this tool when you have successfully gathered all required information and are ready to end the loop.";

pub const SCRATCHPAD_DESCRIPTION: &str =
    "Overwrite the working scratchpad with its complete updated state.";

pub const SCRATCHPAD_GUIDANCE: &str = "Before calling `final_answer`, you MUST call `update_scratchpad` with the complete updated state. Update it after every meaningful investigation milestone. Maintain a concise, durable summary of the main goal, completed milestones, current focus, and vital findings so the investigation can resume from it. Always submit the full updated scratchpad, not a partial patch. The scratchpad is working state, not the final response.";

pub const WAIT_JOB_DESCRIPTION: &str = "Waits for all specified background job_ids to finish and returns their outputs in the order requested. Only call this after you have no other independent tool calls left to run. Must be called alone in its turn, never alongside other tool calls.";

pub const WAIT_JOB_GUIDANCE: &str = "Do NOT call `wait_job` immediately after receiving a pending `Job ID` if you still have other independent investigation steps to perform. Continue calling other tools while the background job runs so slow commands do not stall your progress. Call `wait_job` only when you are blocked on the pending job's result or ready to finalize, and ALWAYS call it alone in its turn with no other tool or signal calls.";

pub fn pending_job_message(threshold_secs: u64, job_id: &str, wait_job_name: &str) -> String {
    format!(
        "Still running in the background after {}s. Job ID: {}. Continue with other independent tool calls first so this slow command does not slow you down; when you actually need its output, call {} with this ID on its own in a later turn.",
        threshold_secs, job_id, wait_job_name
    )
}

pub fn must_run_alone_message(sig_name: &str) -> String {
    format!(
        "`{}` must be called on its own, with no other calls in the same turn.",
        sig_name
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_call_batch_policy_matches_runtime_execution() {
        assert!(TOOL_CALL_ORDER.contains("executes them concurrently as background jobs"));
        assert!(TOOL_CALL_ORDER.contains("pending status with a Job ID"));
        assert!(TOOL_CALL_ORDER.contains("wait_job"));
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
