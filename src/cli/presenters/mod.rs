use crate::agent::agents::{AgentEvent, AgentState};
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tokio::task::JoinHandle;
use tokio::time::{interval, Duration};
use indicatif::{ProgressBar, ProgressStyle, ProgressDrawTarget};
use colored::Colorize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

pub mod render;

const WIDTH: usize = 6;
const HEIGHT: usize = 4;
const TICK_MS: u64 = 160;
const RESEED_AFTER: u32 = 50;
const DENSITY: f32 = 0.25;

pub struct Presenter {
    rx: UnboundedReceiver<AgentEvent>,
}

impl Presenter {
    pub fn new() -> (Self, UnboundedSender<AgentEvent>) {
        let (tx, rx) = mpsc::unbounded_channel();
        (Self { rx }, tx)
    }

    pub fn spawn(mut self) -> JoinHandle<()> {
        tokio::spawn(async move {
            let pb = ProgressBar::new_spinner();
            pb.set_draw_target(ProgressDrawTarget::stdout()); // <-- add this
            pb.set_style(ProgressStyle::default_spinner().template("{msg}").unwrap());
            let mut board = Board::seeded(WIDTH, HEIGHT, next_seed());
            let mut state = AgentState::Thinking;
            let mut message = "Waiting for agent activity".to_string();
            pb.set_message(board.render(state, &message));

            let mut ticker = interval(Duration::from_millis(TICK_MS));
            let mut gens_since_reseed: u32 = 0;

            loop {
                tokio::select! {
                    biased;

                    event_opt = self.rx.recv() => {
                        match event_opt {
                            Some(AgentEvent::StateUpdate(next_state)) => {
                                state = next_state;
                                message = state_message(state).to_string();
                                pb.set_message(board.render(state, &message));
                            }
                            Some(AgentEvent::ToolCall(call)) => {
                                let line = render::format_call(&call);
                                pb.suspend(|| println!("{line}"));
                            }
                            None => break,
                        }
                    }

                    _ = ticker.tick() => {
                        board.step();
                        gens_since_reseed += 1;

                        if board.is_empty() || gens_since_reseed >= RESEED_AFTER {
                            board = Board::seeded(WIDTH, HEIGHT, next_seed());
                            gens_since_reseed = 0;
                        }

                        pb.set_message(board.render(state, &message));
                    }
                }
            }

            pb.finish();
        })
    }
}

struct Board {
    width: usize,
    height: usize,
    cells: Vec<bool>,
}

impl Board {
    fn seeded(width: usize, height: usize, seed: u64) -> Self {
        let mut rng = XorShift64::new(seed);
        let cells = (0..width * height).map(|_| rng.next_f32() < DENSITY).collect();
        Self { width, height, cells }
    }

    fn idx(&self, x: usize, y: usize) -> usize {
        y * self.width + x
    }

    fn alive(&self, x: isize, y: isize) -> bool {
        let x = x.rem_euclid(self.width as isize) as usize;
        let y = y.rem_euclid(self.height as isize) as usize;
        self.cells[self.idx(x, y)]
    }

    fn step(&mut self) {
        let mut next = vec![false; self.cells.len()];
        for y in 0..self.height {
            for x in 0..self.width {
                let mut n = 0;
                for dy in [-1isize, 0, 1] {
                    for dx in [-1isize, 0, 1] {
                        if dx == 0 && dy == 0 { continue; }
                        if self.alive(x as isize + dx, y as isize + dy) {
                            n += 1;
                        }
                    }
                }
                let alive_now = self.cells[self.idx(x, y)];
                next[self.idx(x, y)] = matches!((alive_now, n), (true, 2) | (true, 3) | (false, 3));
            }
        }
        self.cells = next;
    }

    fn is_empty(&self) -> bool {
        !self.cells.iter().any(|&c| c)
    }

    fn render(&self, state: AgentState, message: &str) -> String {
        let mut out = format!("{}\n", format!("Agent: {}", state.label()).dimmed());
        out.push_str(&format!(
            "┌{}┐\n",
            "─".repeat(self.width * 2)
        ));

        for y in 0..self.height {
            out.push('│');
            for x in 0..self.width {
                let pixel = if self.cells[self.idx(x, y)] {
                    state_cell(state)
                } else {
                    "  ".on_black().to_string()
                };
                out.push_str(&pixel);
            }
            out.push('│');
            if y == self.height / 2 {
                out.push_str(&format!("  {}", render::truncate(message, 48)));
            }
            out.push('\n');
        }
        out.push_str(&format!("└{}┘", "─".repeat(self.width * 2)));
        out
    }
}

fn state_cell(state: AgentState) -> String {
    match state {
        AgentState::Thinking => "  ".on_green().to_string(),
        AgentState::ExecutingTool => "  ".on_yellow().to_string(),
        AgentState::StructuringOutput => "  ".on_blue().to_string(),
        AgentState::Completed => "  ".on_blue().to_string(),
        AgentState::Failed => "  ".on_red().to_string(),
    }
}

fn state_message(state: AgentState) -> &'static str {
    match state {
        AgentState::Thinking => "Waiting for model response",
        AgentState::ExecutingTool => "Running requested tool",
        AgentState::StructuringOutput => "Formatting final answer",
        AgentState::Completed => "Agent finished",
        AgentState::Failed => "Agent failed",
    }
}

/// Tiny self-contained PRNG — avoids pulling in the `rand` crate for a spinner.
struct XorShift64(u64);

impl XorShift64 {
    fn new(seed: u64) -> Self {
        Self(if seed == 0 { 0x9E3779B97F4A7C15 } else { seed })
    }
    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    fn next_f32(&mut self) -> f32 {
        (self.next_u64() % 1_000_000) as f32 / 1_000_000.0
    }
}

static SEED_COUNTER: AtomicU64 = AtomicU64::new(0);

fn next_seed() -> u64 {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    let count = SEED_COUNTER.fetch_add(1, Ordering::Relaxed);
    nanos ^ count.wrapping_mul(0x9E3779B97F4A7C15)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn board_frame_displays_state_and_message_beside_cells() {
        let board = Board::seeded(WIDTH, HEIGHT, 1);

        let frame = board.render(AgentState::ExecutingTool, "Running requested tool");

        assert!(frame.contains("Agent: Executing tool"));
        assert!(frame.contains("Running requested tool"));
        assert!(frame.contains("┌"));
        assert!(frame.contains("└"));
    }
}