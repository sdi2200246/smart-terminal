use std::io::{self, Write};
use std::path::Path;

use crossterm::terminal;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
};
use serde_json::Value;

use crate::agent::{NextCommand, Plan, Report, Reversibility};
use crate::core::{AgentToolCall, Interaction};

pub const OUTPUT_PROTOCOL: &str = "SMART_TERMINAL_NEXT_CMD_V1";

const DEFAULT_PANEL_WIDTH: u16 = 84;
const MIN_PANEL_WIDTH: u16 = 48;
const MAX_PANEL_WIDTH: u16 = 100;
/// Columns consumed by the accent bar, left padding and right padding.
const PANEL_CHROME: u16 = 5;
const BODY_X: u16 = 3;

const PANEL_BG: Color = Color::Rgb(30, 32, 38);
const BADGE_FG: Color = Color::Rgb(18, 18, 22);
const TEXT: Color = Color::Rgb(222, 225, 230);
const MUTED: Color = Color::Rgb(130, 136, 148);
const BLUE: Color = Color::Rgb(122, 162, 247);
const GREEN: Color = Color::Rgb(158, 206, 106);
const CYAN: Color = Color::Rgb(125, 207, 255);
const AMBER: Color = Color::Rgb(224, 175, 104);
const ORANGE: Color = Color::Rgb(255, 158, 100);
const RED: Color = Color::Rgb(247, 118, 142);

fn text() -> Style {
    Style::default().fg(TEXT)
}

fn strong() -> Style {
    Style::default().fg(TEXT).add_modifier(Modifier::BOLD)
}

fn muted() -> Style {
    Style::default().fg(MUTED)
}

fn accent(color: Color) -> Style {
    Style::default().fg(color).add_modifier(Modifier::BOLD)
}

pub(crate) fn panel_width() -> u16 {
    terminal::size()
        .map(|(w, _)| w.clamp(MIN_PANEL_WIDTH, MAX_PANEL_WIDTH))
        .unwrap_or(DEFAULT_PANEL_WIDTH)
}

/// Renders a filled panel: a thin accent bar on the left, a solid badge title,
/// and a body on a subtle dark background.
fn panel(color: Color, badge: &str, subtitle: &str, body: Vec<Line<'static>>) -> String {
    let width = panel_width();
    let inner = width.saturating_sub(PANEL_CHROME) as usize;

    let mut title = vec![Span::styled(
        format!(" {badge} "),
        Style::default()
            .fg(BADGE_FG)
            .bg(color)
            .add_modifier(Modifier::BOLD),
    )];
    if !subtitle.is_empty() {
        title.push(Span::raw("  "));
        title.push(Span::styled(subtitle.to_string(), strong()));
    }

    let mut rows = wrap_styled_lines(vec![Line::from(title)], inner);
    if !body.is_empty() {
        rows.push(Line::default());
        rows.extend(wrap_styled_lines(body, inner));
    }

    let height = rows.len() as u16 + 2;
    let area = Rect::new(0, 0, width, height);
    let mut buffer = Buffer::empty(area);
    buffer.set_style(area, Style::default().bg(PANEL_BG));
    for y in 0..height {
        buffer[(0, y)].set_symbol("▎").set_fg(color);
    }
    for (index, row) in rows.iter().enumerate() {
        buffer.set_line(BODY_X, index as u16 + 1, row, width - PANEL_CHROME + 1);
    }

    buffer_to_ansi(&buffer)
}

pub(crate) fn format_call(call: &AgentToolCall) -> String {
    let args = summarize_args(&call.arguments());
    let mut spans = vec![
        Span::raw("  "),
        Span::styled("●", Style::default().fg(GREEN)),
        Span::raw(" "),
        Span::styled(call.name().to_string(), strong()),
    ];
    if !args.is_empty() {
        spans.push(Span::raw("  "));
        spans.push(Span::styled(args, muted()));
    }
    render_line(&Line::from(spans))
}

pub(crate) fn format_backgrounded(tool: &str, job_id: &str) -> String {
    render_line(&Line::from(vec![
        Span::raw("  "),
        Span::styled("◷", Style::default().fg(AMBER)),
        Span::raw(" "),
        Span::styled(tool.to_string(), strong()),
        Span::styled("  running in background · ", muted()),
        Span::styled(job_id.to_string(), Style::default().fg(AMBER)),
    ]))
}

pub(crate) fn format_approval_request(call: &AgentToolCall) -> String {
    let args = summarize_args(&call.arguments());
    let mut body = vec![Line::from(vec![
        Span::styled("Tool  ", muted()),
        Span::styled(call.name().to_string(), strong()),
    ])];
    if !args.is_empty() {
        body.push(Line::from(vec![
            Span::styled("Args  ", muted()),
            Span::styled(args, text()),
        ]));
    }
    panel(AMBER, "APPROVAL", "This tool can modify your system", body)
}

pub(crate) fn format_plan(plan: &Plan) -> String {
    let mut body = Vec::new();
    for (index, step) in plan.steps.iter().enumerate() {
        body.push(Line::from(vec![
            Span::styled(format!("{:>2}  ", index + 1), accent(BLUE)),
            Span::styled(step.action.clone(), text()),
        ]));
        body.push(Line::from(vec![
            Span::raw("    "),
            Span::styled(step.rationale.clone(), muted()),
        ]));
    }
    panel(BLUE, "PLAN", &plan.goal, body)
}

pub(crate) fn format_report(report: &Report) -> String {
    let body = report
        .report
        .lines()
        .map(|line| {
            if line.trim().is_empty() {
                Line::default()
            } else {
                Line::from(Span::styled(line.to_string(), text()))
            }
        })
        .collect();
    panel(GREEN, "REPORT", "", body)
}

fn scale_style(scale: &Reversibility) -> (Color, &'static str) {
    match scale {
        Reversibility::Full => (GREEN, "Full"),
        Reversibility::Mostly => (BLUE, "Mostly"),
        Reversibility::Partial => (AMBER, "Partial"),
        Reversibility::Hard => (ORANGE, "Hard"),
        Reversibility::Irreversible => (RED, "Irreversible"),
    }
}

pub(crate) fn format_recommendation(command: &NextCommand) -> String {
    let (color, label) = scale_style(&command.scale);
    let mut body: Vec<Line<'static>> = command
        .cmd
        .lines()
        .enumerate()
        .map(|(index, line)| {
            let prefix = if index == 0 { "$ " } else { "  " };
            Line::from(vec![
                Span::styled(prefix, accent(color)),
                Span::styled(line.to_string(), strong()),
            ])
        })
        .collect();
    body.push(Line::default());
    body.push(Line::from(Span::styled(command.man.clone(), muted())));
    panel(color, "NEXT", &format!("{label} reversibility"), body)
}

pub(crate) fn format_memory_interactions(cwd: &Path, interactions: &[Interaction]) -> String {
    let body = interactions
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            Line::from(vec![
                Span::styled(format!("{:>2}  ", index + 1), accent(CYAN)),
                Span::styled(entry.user_input.trim().to_string(), text()),
                Span::styled("  →  ", muted()),
                Span::styled(entry.predicted_cmd.clone(), strong()),
            ])
        })
        .collect();
    panel(
        CYAN,
        "MEMORY",
        &format!("{} · {} interactions", cwd.display(), interactions.len()),
        body,
    )
}

pub(crate) fn format_success(action: &str, detail: &str) -> String {
    let mut spans = vec![
        Span::styled("✓ ", accent(GREEN)),
        Span::styled(action.to_string(), strong()),
    ];
    if !detail.is_empty() {
        spans.push(Span::styled(format!(" {detail}"), muted()));
    }
    render_line(&Line::from(spans))
}

pub(crate) fn format_error(message: &str) -> String {
    render_line(&Line::from(vec![
        Span::styled("✗ ", accent(RED)),
        Span::styled(message.to_string(), Style::default().fg(RED)),
    ]))
}

pub(crate) fn format_info(message: &str) -> String {
    render_line(&Line::from(Span::styled(message.to_string(), muted())))
}

pub(crate) fn format_prompt(question: &str) -> String {
    render_line(&Line::from(vec![
        Span::raw("  "),
        Span::styled("? ", accent(AMBER)),
        Span::styled(question.to_string(), strong()),
        Span::styled(" ", muted()),
    ]))
}

pub(crate) fn write_prediction(
    output: &mut impl Write,
    prediction: &NextCommand,
) -> io::Result<()> {
    // The command is the complete remaining payload after the versioned metadata header.
    writeln!(output, "{OUTPUT_PROTOCOL}")?;
    writeln!(
        output,
        "{}",
        prediction.man.lines().collect::<Vec<_>>().join(" ")
    )?;
    writeln!(output, "{:?}", prediction.scale)?;
    writeln!(output, "{}", prediction.cmd)
}

fn wrap_styled_lines(lines: Vec<Line<'static>>, max_width: usize) -> Vec<Line<'static>> {
    let max_width = max_width.max(1);
    let mut out = Vec::new();
    for line in lines {
        if line.spans.is_empty() {
            out.push(Line::default());
            continue;
        }

        let mut current_spans: Vec<Span<'static>> = Vec::new();
        let mut current_len = 0usize;

        for span in line.spans {
            let style = span.style;
            let content = span.content.into_owned();
            let mut remaining = content.as_str();

            while !remaining.is_empty() {
                let space_left = max_width.saturating_sub(current_len);
                if space_left == 0 {
                    out.push(Line::from(std::mem::take(&mut current_spans)));
                    current_len = 0;
                    continue;
                }

                let char_count = remaining.chars().count();
                if char_count <= space_left {
                    current_spans.push(Span::styled(remaining.to_string(), style));
                    current_len += char_count;
                    break;
                }

                let split_chars = find_wrap_split(remaining, space_left);
                let byte_idx = remaining
                    .char_indices()
                    .nth(split_chars)
                    .map(|(idx, _)| idx)
                    .unwrap_or(remaining.len());

                let (chunk, rest) = remaining.split_at(byte_idx);
                current_spans.push(Span::styled(chunk.to_string(), style));
                out.push(Line::from(std::mem::take(&mut current_spans)));
                current_len = 0;
                remaining = rest.trim_start_matches(' ');
            }
        }

        if !current_spans.is_empty() {
            out.push(Line::from(current_spans));
        }
    }
    out
}

fn find_wrap_split(text: &str, max_chars: usize) -> usize {
    let prefix: String = text.chars().take(max_chars).collect();
    if let Some(space_byte_idx) = prefix.rfind(' ') {
        let chars_before_space = prefix[..space_byte_idx].chars().count();
        if chars_before_space > 0 {
            return chars_before_space;
        }
    }
    max_chars
}

pub(crate) fn render_line(line: &Line<'_>) -> String {
    let width: usize = line.spans.iter().map(|s| s.content.chars().count()).sum();
    if width == 0 {
        return String::new();
    }
    let area = Rect::new(0, 0, width as u16, 1);
    let mut buffer = Buffer::empty(area);
    buffer.set_line(0, 0, line, width as u16);
    buffer_to_ansi(&buffer)
}

pub(crate) fn buffer_to_ansi(buffer: &Buffer) -> String {
    let area = buffer.area;
    let mut out = String::new();

    for y in 0..area.height {
        if y > 0 {
            out.push('\n');
        }
        let mut last_x = area.width;
        while last_x > 0 {
            let cell = &buffer[(last_x - 1, y)];
            if cell.symbol() == " " && cell.bg == Color::Reset {
                last_x -= 1;
            } else {
                break;
            }
        }

        let mut prev_style: Option<(Color, Color, Modifier)> = None;
        for x in 0..last_x {
            let cell = &buffer[(x, y)];
            let style_tuple = (cell.fg, cell.bg, cell.modifier);
            if prev_style != Some(style_tuple) {
                out.push_str("\x1b[0m");
                out.push_str(&style_to_ansi(cell.fg, cell.bg, cell.modifier));
                prev_style = Some(style_tuple);
            }
            out.push_str(cell.symbol());
        }
        if prev_style.is_some() {
            out.push_str("\x1b[0m");
        }
    }

    out
}

fn style_to_ansi(fg: Color, bg: Color, modifier: Modifier) -> String {
    let mut codes: Vec<String> = Vec::new();
    if modifier.contains(Modifier::BOLD) {
        codes.push("1".into());
    }
    if modifier.contains(Modifier::DIM) {
        codes.push("2".into());
    }
    if modifier.contains(Modifier::ITALIC) {
        codes.push("3".into());
    }
    if modifier.contains(Modifier::UNDERLINED) {
        codes.push("4".into());
    }
    if let Some(code) = color_code(fg, false) {
        codes.push(code);
    }
    if let Some(code) = color_code(bg, true) {
        codes.push(code);
    }
    if codes.is_empty() {
        String::new()
    } else {
        format!("\x1b[{}m", codes.join(";"))
    }
}

fn color_code(color: Color, background: bool) -> Option<String> {
    let base: u8 = match color {
        Color::Reset => return None,
        Color::Rgb(r, g, b) => {
            let kind = if background { 48 } else { 38 };
            return Some(format!("{kind};2;{r};{g};{b}"));
        }
        Color::Indexed(i) => {
            let kind = if background { 48 } else { 38 };
            return Some(format!("{kind};5;{i}"));
        }
        Color::Black => 30,
        Color::Red => 31,
        Color::Green => 32,
        Color::Yellow => 33,
        Color::Blue => 34,
        Color::Magenta => 35,
        Color::Cyan => 36,
        Color::Gray => 37,
        Color::DarkGray => 90,
        Color::LightRed => 91,
        Color::LightGreen => 92,
        Color::LightYellow => 93,
        Color::LightBlue => 94,
        Color::LightMagenta => 95,
        Color::LightCyan => 96,
        Color::White => 97,
    };
    Some((base + if background { 10 } else { 0 }).to_string())
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
