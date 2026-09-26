# smart-terminal
 
An AI-powered shell companion that predicts your next command and investigates your codebase and shell environment.
 
Built in Rust. Uses provider-specific LLMs via Groq and Google Gemini.

<img width="1088" height="60" alt="image" src="https://github.com/user-attachments/assets/daada4f4-38dd-44af-a8cc-31634c140816" />


## Who this is for
 
Shell users at every level — beginners learning the ropes, intermediates getting better, and professionals who already know their tools and want to move faster.
 
Every prediction comes with a one-line explanation and a reversibility flag. Nothing runs without your keystroke.
 
Not a chat box that wraps the terminal. A completion layer that respects it.

## Features Demo
 
### `next-cmd` — ghost completion

 The following demo shows `next-cmd` reacting to real shell context while working through a small Git workflow.

The model infers intent from:
- the current directory
- recent shell history
- partial commands and ambiguous prompts
 
Notice how suggestions stay context-aware while remaining fast enough to feel native to the terminal experience.

<!-- TODO: replace with uploaded video -->
<p align="center">

https://github.com/user-attachments/assets/3d984007-d8ac-477c-a0d3-5c22b70f3240

Press `^G` to fetch a suggestion, `^F` to accept, `^B` to clear.

The inline # description next to each suggestion is color-coded by how reversible the predicted command is — at a glance you know the cost of pressing ^F

> [!NOTE]
> These levels are intentionally approximate — they are not strict safety guarantees.  
> They exist to provide a quick intuition about the potential impact and reversibility of a command, so the color alone gives the user an immediate signal about how careful they should be before pressing `^F`.

| Level | Color | Meaning | Example |
|---|---|---|---|
| **Full** | 🟢 | read-only or fully reversible | `ls`, `grep`, `git log` |
| **Mostly** | 🔵 | undoable in one step | `git stash`, `git commit` |
| **Partial** | 🟡 | some effects stick | `mkdir`, `touch`, `git add` |
| **Hard** | 🔴 | requires manual cleanup | `git switch`, `docker system prune` |
| **Irreversible** | 🟥 | cannot be undone | `rm -rf`, `git push --force` |


### `memory` — per-project context
Memory is scoped per folder. When registered, `next-cmd` reads prior interactions in this project and feeds them back into the model, so suggestions sharpen over time.
 
```
$ smart-terminal memory init
✓ registered /home/jsn/projects/smart-terminal
 
$ smart-terminal memory show
memory for /home/jsn/projects/smart-terminal (2 interactions):
 
  1. git st → git status
  2. cargo t → cargo test --workspace
 
$ smart-terminal memory clear
✓ cleared interactions for /home/jsn/projects/smart-terminal
 
$ smart-terminal memory delete
✓ deleted memory for /home/jsn/projects/smart-terminal
```

### `investigate` — answer questions about your project
 
Pose a question; a planner agent forms a plan, an executor agent runs it against your filesystem and shell, and you get a grounded answer.
 

https://github.com/user-attachments/assets/5ac577fa-13b1-421b-a1a2-649fb0c95211
 
The planner uses `read_dir` to orient and emits a structured plan as JSON. The executor consumes that plan and runs it with `bash`, `read_dir`, and `read_file`, then writes the report.
 
Useful for anything you'd normally answer by poking around — what does this codebase do, where is X implemented, what's installed on this machine, what's the git state, why is this test failing, what changed between two branches.
 
> Under active development. The planner sometimes over- or under-scopes, the executor occasionally repeats steps. Both will sharpen.
> **Provider rate limits**: `investigate` uses Google Gemini and chains multiple LLM calls across the planner and executor, while `next-cmd` uses Groq and may make additional calls when inspecting git diffs or Docker state. Simple `next-cmd` completions stay fast. If a provider rate limit is reached, wait a few seconds and retry or review that provider's plan.

## Architecture Overview

<p align="center">
<img width="680" height="540" alt="smart_terminal_dependency_graph_v4" src="https://github.com/user-attachments/assets/9c5cfc28-c376-4fa0-882f-1890f519d46c" />
</p>

`smart-terminal` is organized into a modular, layered architecture that separates terminal interaction, reasoning workflows, LLM integration, and system tooling. `src/cli` is the sole composition root — the only layer that wires concrete providers, tools, and memory together.

### Core Layers

| Layer | Responsibility |
|---|---|
| `src/cli` | Parses commands (`investigate`, `next_cmd`, `memory`) and streams tool-call output via `cli::presenters`. |
| `src/core` | Provider-agnostic contracts: `LLMProvider`, `Capability`, `AgentSession`, `Model`, `Memory`, error types. Never imports `agent`, `tools`, or `providers`. |
| `src/agent` | `agent::workflows` (NextCmd, Investigator), `agent::agents` (planner/executor/architect/cmd_predictor), `agent::patterns` (ReactLoop, OneShot, hooks), `agent::memory` (FolderMemory). |
| `src/providers` | `groq` and `google` clients, both built on the shared `providers::client::GenericLlmClient<ProviderCodec>` codec layer. |
| `src/tools` | Bash, ReadDir, ReadFile, Docker, GitDiff/GitLog, AskUser, ReadLastError, Json — all implement `core::Capability`. |
| `memory/` | Persistent JSON session storage, keyed by project folder. |

### High-Level Code Flow
Every command follows the same call stack. `cli` is the composition root — it constructs the command's configured provider (`GroqClient` for `next-cmd`, `GoogleClient` for `investigate`) and hands it to the workflow. The workflow spins up one or more agents, each agent assembles a tool registry and delegates to a loop. The loop drives everything: it calls the provider, dispatches tool results, and repeats until the model signals completion, at which point it makes a final structured output call and unwinds back up the stack.
 
Memory is not part of the call chain. The workflow loads it before the loop starts and appends to it after the result returns — nothing below the workflow layer touches it.
 
The only thing that varies per command is what happens inside the workflow box:
 
| Command | Agents | Loop |
|---|---|---|
| `next-cmd` | 1 — `cmd_predictor` | `ReactLoop` |
| `investigate` | 2 — `planner` then `executor` | `ReactLoop` (shared) |

<p align="center">
<img height="500" alt="smart_terminal_runtime_flow_clean" src="https://github.com/user-attachments/assets/17b9772b-a549-493f-aade-7859b931ac56" />
</p>

### Design Goals

- **Modular** — Clear separation between interface, reasoning, tooling, and provider logic.
- **Pluggable** — New agent architectures and LLM providers can be added easily.
- **Stateful** — Persistent JSON memory enables cross-session continuity.
- **Testable** — Integration tests validate real workflow execution end-to-end.


## Setup

**Requirements**: macOS or Linux with **zsh** and Rust installed.

### 1. Install Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
```

If Rust is already installed, `source "$HOME/.cargo/env"` is enough to add Cargo to your `PATH` in the current shell.

### 2. Get the required API keys

`smart-terminal` uses Groq for command prediction, and the `investigate` feature uses Gemini/Google.

- Create a Groq key at [console.groq.com](https://console.groq.com)
- Create a Google/Gemini key in the relevant Google AI console if you plan to use `smart-terminal investigate`

Add them to your shell profile so the app can read them at runtime:

```bash
cat <<'EOF' >> ~/.zshrc
export GROQ_API_KEY="your_groq_api_key"
export GEMINI_API_KEY="your_gemini_api_key"
# or: export GOOGLE_API_KEY="your_gemini_api_key"
EOF
source ~/.zshrc
```

### 3. Clone and install the binary

```bash
git clone https://github.com/sdi2200246/smart-terminal.git
cd smart-terminal
cargo install --path .
```

This builds the Rust binary and installs it to `~/.cargo/bin`, which is already on your `PATH` via `rustup`.

### 4. Enable the zsh integration

The shell hook lives in `scripts/zsh/smart-terminal.zsh`.

```bash
cat <<'EOF' >> ~/.zshrc
source /path/to/smart-terminal/scripts/zsh/smart-terminal.zsh
reload() { source ~/.zshrc; }
EOF
source ~/.zshrc
```

Replace `/path/to/smart-terminal` with the actual location where you cloned the repo.

### 5. Verify the install

```bash
smart-terminal next-cmd "list files"
```

You should see a command suggestion printed. Then open a fresh zsh session and press `^G` on an empty prompt — the ghost suggestion should appear inline. If it does, setup is complete.

## Updating

Pull the latest changes, rebuild the binary, and reload the zsh integration:

```bash
cd /path/to/smart-terminal
git pull
cargo install --path . --force
reload
```

`--force` replaces the existing binary in `~/.cargo/bin`, and `reload` re-sources your zsh config so the latest hook is active.

> Other open terminal tabs keep the old shell integration until you run `reload` in them or open a fresh session.
 
## Roadmap

- **Folder-scoped investigation state** — maintain one investigation conversation history per project folder, so the `investigate` agent can preserve context between sessions without mixing unrelated projects.
- **Conversation management commands** — add commands to compact or erase the stored investigation history when users want to reduce context or start fresh.

Contributions, bug reports, and feature suggestions are welcome.
 
## Author & Contact

Built with 🦀 by **Jason Stefanou**  
Informatics Undergraduate at the National and Kapodistrian University of Athens.

- **Email:** jasonstephanou3@gmail.com

Questions, setup issues, bug reports, and improvement ideas are always welcome.  
If something breaks, feels unclear, or you have suggestions for new features or workflow improvements, feel free to open an issue or reach out directly.
