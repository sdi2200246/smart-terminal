use crate::core::responce::AgentToolCall;
use serde_json::Value;

pub(crate) fn format_call(call: &AgentToolCall) -> String {
    let name = call.name();
    let args = summarize_args(&call.arguments());
    return format!("\x1b[32m●\x1b[0m \x1b[1m{name}\x1b[0m\x1b[2m({args})\x1b[0m")
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

pub(super) fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max - 1).collect();
        out.push('…');
        out
    }
}
