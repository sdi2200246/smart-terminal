# smart-terminal
 
An AI-powered shell companion that predicts your next command and investigates your codebase and shell enviroment.
 
Built in Rust. Powered by open-source LLMs via Groq.

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


https://github.com/user-attachments/assets/eeaf6ddf-c746-493a-a106-3639c9605ea0


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
 
```
$ smart-terminal investigate "what dependencies does this use?"
─── Plan ───
Goal: List the project's direct dependencies.
 
  1. Read Cargo.toml to extract the [dependencies] section.
     Cargo.toml is the canonical source of declared crates.
 
─── Report ───
The project depends on tokio, reqwest, serde, schemars, jsonschema,
clap, anyhow, thiserror, tracing, and a handful of dev-only crates.
```
 
The planner uses `read_dir` to orient and emits a structured plan as JSON. The executor consumes that plan and runs it with `bash`, `read_dir`, and `read_file`, then writes the report.
 
Useful for anything you'd normally answer by poking around — what does this codebase do, where is X implemented, what's installed on this machine, what's the git state, why is this test failing, what changed between two branches.
 
> Under active development. The planner sometimes over- or under-scopes, the executor occasionally repeats steps. Both will sharpen.

## Architecture Overview

<p align="center">
    <img height="500 " alt="smart_terminal_precise_deps" src="https://github.com/user-attachments/assets/3567df53-eab3-44ff-8138-0e566f9eb168" />
</p>

`smart-terminal` is organized into a modular, layered architecture that separates terminal interaction, reasoning workflows, LLM integration, and system tooling.

### Core Layers

| Layer | Responsibility |
|---|---|
| `src/cli` | Parses commands and exposes the terminal interface (`investigate`, `next_cmd`, `memory`, etc.). |
| `src/core` | Handles sessions, memory, capabilities, errors, and provider-agnostic LLM abstractions. |
| `src/agent` | Implements reasoning architectures (`OneShot`, `ReAct`) and workflows like investigation and script generation. |
| `src/groq` | Groq-specific API client, protocol models, and adapters. |
| `src/tools` | Safe utilities for shell execution, Git diffs, Docker, file reading, and JSON handling. |
| `memory/` | Persistent JSON-based session and memory storage. |
| `tests/` | End-to-end integration tests covering workflows and provider interaction. |

### High-Level Code Flow
Every command follows the same call stack. `cli` is the composition root — it constructs `GroqClient` and hands it to the workflow. The workflow spins up one or more agents, each agent assembles a tool registry and delegates to a loop. The loop drives everything: it calls the provider, dispatches tool results, and repeats until the model signals completion, at which point it makes a final structured output call and unwinds back up the stack.
 
Memory is not part of the call chain. The workflow loads it before the loop starts and appends to it after the result returns — nothing below the workflow layer touches it.
 
The only thing that varies per command is what happens inside the workflow box:
 
| Command | Agents | Loop |
|---|---|---|
| `next-cmd` | 1 — `cmd_predictor` | `ReactLoop` |
| `investigate` | 2 — `planner` then `executor` | `ReactLoop` (shared) |
| `exec` | 1 — `architect` + 1 — `generator` | `ReactLoop` + `OneShot` |

<p align="center">
<img height="500" alt="smart_terminal_runtime_flow_clean" src="https://github.com/user-attachments/assets/17b9772b-a549-493f-aade-7859b931ac56" />
</>

### Design Goals

- **Modular** — Clear separation between interface, reasoning, tooling, and provider logic.
- **Pluggable** — New agent architectures and LLM providers can be added easily.
- **Stateful** — Persistent JSON memory enables cross-session continuity.
- **Testable** — Integration tests validate real workflow execution end-to-end.


## Setup
 
**Requirements**: macOS or Linux with **zsh**.
 
**1. Install Rust**
 
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```
 
Follow the prompts (defaults are fine). Then either open a new terminal or run `source $HOME/.cargo/env`.
 
**2. Get a Groq API key**
 
Sign up at [console.groq.com](https://console.groq.com), create an API key, and copy it. The free tier is enough to get going.
 
**3. Clone and install**
 
```bash
git clone https://github.com/yourusername/smart-terminal.git
cd smart-terminal
cargo install --path .
```
 
This builds and installs `smart-terminal` into `~/.cargo/bin`, which `rustup` already added to your `$PATH`.
 
**4. Configure zsh**
 
Still inside the cloned directory, append the integration to your `.zshrc`:
 
```bash
cat <<EOF >> ~/.zshrc
 
# smart-terminal
export GROQ_API_KEY="paste-your-key-here"
source "$(pwd)/scripts/zsh/smart-terminal.zsh"
EOF
```
 
Then open `~/.zshrc` and replace `paste-your-key-here` with the key from step 2.
 
**5. Apply and verify**
 
```bash
source ~/.zshrc
smart-terminal next-cmd "list files"
```
 
You should see a command suggestion printed. Now open a fresh zsh session and press `^G` on an empty prompt — a ghost suggestion should appear inline. If it does, you're done.
 
> **Groq free tier limits**: the free API key has rate limits that can throttle `investigate` (which chains multiple LLM calls across planner + executor) and some `next-cmd` flows that inspect git diffs or docker state before predicting. `next-cmd` on simple completions stays fast. If you hit rate limits, wait a few seconds and retry — or upgrade your Groq plan.

## Planned Improvements

- **More reliable planning** — improve the investigation planner so it scopes tasks more precisely.
- **Faster execution** — reduce repeated tool calls and make multi-step workflows more efficient.

## Future Features

- **Per-project instruction profiles** — allow users to define custom behavior and instructions scoped to specific project folders.
- **Personalized command modeling** — build a lightweight behavioral profile from user workflows and shell habits to create more customized suggestions over time.
- **More integrations** — support additional shells and future LLM providers.

Contributions, bug reports, and feature suggestions are welcome.
 
## Author & Contact

Built with 🦀 by **Jason Stefanou**  
Informatics Undergraduate at the National and Kapodistrian University of Athens.

- **Email:** jasonstephanou3@gmail.com

Questions, setup issues, bug reports, and improvement ideas are always welcome.  
If something breaks, feels unclear, or you have suggestions for new features or workflow improvements, feel free to open an issue or reach out directly.
