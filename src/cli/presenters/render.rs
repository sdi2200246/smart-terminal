use crate::core::session::AgentToolCall;
use serde_json::Value;

const RESET: &str = "\x1b[0m";
const BOLD:  &str = "\x1b[1m";
const DIM:   &str = "\x1b[2m";
const GREEN: &str = "\x1b[32m";

pub(super) fn format_call(call: &AgentToolCall) -> String {
    let name = call.name();
    let args = summarize_args(call.arguments());
    format!("{GREEN}●{RESET} {BOLD}{name}{RESET}{DIM}({args}){RESET}")
}

fn summarize_args(args: &Value) -> String {
    match args {
        Value::Object(map) if !map.is_empty() => map
            .iter()
            .map(|(k, v)| format!("{k}: {}", truncate(&render_value(v), 60)))
            .collect::<Vec<_>>()
            .join(", "),
        _ => String::new(),
    }
}

fn render_value(v: &Value) -> String {
    match v {
        Value::String(s) => format!("\"{s}\""),
        other => other.to_string(),
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max - 1).collect();
        out.push('…');
        out
    }
}