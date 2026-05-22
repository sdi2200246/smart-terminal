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
  <img 
    src="https://github.com/user-attachments/assets/58023429-2c73-42b1-a246-ad342ad71b77" 
    alt="smart-terminal architecture"
    width="600"
  />
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

```text
CLI Command
    ↓
Core Session + Memory
    ↓
Agent Architecture (OneShot / ReAct)
    ↓
Workflow Execution
    ↓
LLM + Tool Calls
    ↓
Formatted Response + Persisted Memory
```

### Design Goals

- **Modular** — Clear separation between interface, reasoning, tooling, and provider logic.
- **Pluggable** — New agent architectures and LLM providers can be added easily.
- **Stateful** — Persistent JSON memory enables cross-session continuity.
- **Testable** — Integration tests validate real workflow execution end-to-end.
