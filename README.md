# smart-terminal
 
An AI-powered shell companion that predicts your next command and investigates your codebase and shell environment.
 
Built in Rust. Uses provider-specific LLMs via Groq, OpenRouter, and Google Gemini.

<img width="1088" height="60" alt="image" src="https://github.com/user-attachments/assets/daada4f4-38dd-44af-a8cc-31634c140816" />


## Project Documentation

The project docs are split into a few focused guides:

- [CONTRIBUTING.md](CONTRIBUTING.md) — guide for contributors, open ideas, and maintainership opportunities.
- [docs/architecture.md](docs/architecture.md) — architecture, data flow, and extension points.
- [docs/development.md](docs/development.md) — local setup, build/test workflow, and contributor expectations.

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

Suggestions may contain multiple lines, including shell blocks and here-documents.
`^F` inserts only the suggested command into the editable buffer; the explanation stays as display-only ghost text and is not added to the command.

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

Investigation resumes the project's saved session by default (using Google Gemini). Use `--one-off` to run without loading or saving history, or `sterm ask` as a shorter alias:

```bash
$ smart-terminal investigate --one-off "What does this repository do?"
$ sterm ask "Why is this test failing?"
$ smart-terminal investigate session clear
```

https://github.com/user-attachments/assets/5ac577fa-13b1-421b-a1a2-649fb0c95211

Every investigation also suggests a follow-up command (shown as ghost text with the Zsh integration, or printed after the report). Useful for exploring unfamiliar codebases, checking git state, debugging failing tests, or comparing branches.

> [!NOTE]
> `investigate` chains multiple Gemini calls per run, while `next-cmd` uses Groq. If you hit a provider rate limit, wait a few seconds and retry.
> For Zsh integration internals and session/scratchpad details, see [docs/architecture.md](docs/architecture.md).

## Architecture Overview

`smart-terminal` separates terminal presentation (`src/cli`), agent reasoning (`src/agent`), background job execution (`src/multithreading`), LLM providers (`src/providers`), persistence (`src/persistence`), and system tools (`src/tools`) behind provider-agnostic contracts in `src/core`.

- [Dependency & Layering Graph](docs/architecture.md#dependency--layering-graph)
- [Runtime & Tool Execution Flow](docs/architecture.md#runtime--tool-execution-flow)

| Command | Workflow | Provider |
|---|---|---|
| `next-cmd` | Single `cmd_predictor` agent (`ReactLoop`) | Groq |
| `investigate` (`ask`) | `planner` → `executor` (`ReactLoop`) | Google Gemini |

> Full layer breakdown, provider codec details (`groq`, `openrouter`, `google`), and extension points live in [docs/architecture.md](docs/architecture.md).

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
- **Conversation management commands** — add commands to inspect or erase the stored investigation history when users want to start fresh.
- **Alternative shell integrations** — native support for Fish, Bash, and Nushell.
- **Offline / Local LLM support** — first-class integration with Ollama and `llama.cpp`.

## Contributing

Contributions, bug reports, and feature suggestions are warmly welcomed! 🎉

As the original author is stepping back from active development, **the project is wide open for community direction, experiments, and new co-maintainers**. If you're interested in shaping an AI shell companion in Rust, we'd love to have you.

Check out [CONTRIBUTING.md](CONTRIBUTING.md) to get started with:
- Local setup & testing guidelines
- High-impact areas needing help (shells, providers, tools, performance)
- Code conventions and PR submission workflow
- Co-maintainership and stewardship opportunities
 
## Author & Contact

Built with 🦀 by **Jason Stefanou**  
Informatics Undergraduate at the National and Kapodistrian University of Athens.

- **Email:** jasonstephanou3@gmail.com

Questions, setup issues, bug reports, and improvement ideas are always welcome.  
If something breaks, feels unclear, or you have suggestions for new features or workflow improvements, feel free to open an issue or reach out directly.
