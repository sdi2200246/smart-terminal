# Contributing to smart-terminal

First off, thank you for taking the time to contribute! 🎉

`smart-terminal` is an AI-powered shell companion built in Rust that predicts your next command and investigates your environment. The project was created to make terminal interactions smoother, safer, and faster without turning the shell into a clumsy chat wrapper.

---

## Project Status: Looking for Contributors & Maintainers!

> [!IMPORTANT]  
> The original creator is stepping back from full-time active development.  
> **This means the project is wide open for community ownership, fresh ideas, and new co-maintainers!**

If you've been looking for an open-source Rust project where your contributions can directly shape the roadmap and architecture—welcome aboard! There is no red tape or fear of stepping on toes here: whether you want to fix a minor typo, improve prompt engineering, add support for your favorite shell or LLM provider, or take on a co-maintainer role, your contributions are sincerely welcomed and deeply appreciated.

---

## Ways to Contribute

You don't need to be a seasoned Rust expert to help out. Here are some high-impact areas where help is needed:

### 1. Shell Integrations
- **Fish, Bash, and Nushell**: Currently, the shell hook is written for Zsh (`scripts/zsh/smart-terminal.zsh`). Creating native integrations for Fish, Bash, or Nushell would bring `smart-terminal` to millions of additional developers.

### 2. LLM Providers & Local Models
- **Local LLMs**: Add support for local offline models via [Ollama](https://ollama.com/), `llama.cpp`, or `vLLM` so users don't need cloud API keys.
- **Provider Enhancements**: Expand providers (OpenAI, Anthropic, DeepSeek, OpenRouter) or improve streaming latency and error recovery in `src/providers/`.

### 3. Investigation Tools & Capabilities
- **New Agent Tools**: Add domain-aware tools under `src/tools/` (e.g., `git status` / `git diff` analyzers, package manager inspectors, test runner parsers).
- **Planner & Executor Prompts**: Fine-tune prompt strategies in `src/agent/prompts/` to make investigation plans tighter and tool loops faster.

### 4. Performance & UX
- **Ghost text speed**: Shell suggestions need to feel instantaneous. Profile and optimize startup latency, memory footprint, and background job handling (`src/multithreading`).
- **Safety heuristic tuning**: Improve command reversibility detection (`Full`, `Mostly`, `Partial`, `Hard`, `Irreversible`).

### 5. Testing & Cross-Platform Support
- Cross-platform verification on Linux distributions and macOS.
- Expand integration tests (`tests/integration/`) and provider mocking to keep CI robust.

### 6. Documentation & DX
- Improving setup guides, recording terminal demos, writing tutorials, and clarifying architecture documentation.

---

## Getting Started

### Prerequisites

- **Rust** (stable toolchain, edition 2024 recommended):  
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```
- **zsh** (for testing the interactive shell integration)
- An API key for at least one supported provider:
  - `GROQ_API_KEY` (recommended for fast command suggestions)
  - `GEMINI_API_KEY` or `GOOGLE_API_KEY` (for investigate / complex queries)

### Setting up locally

1. **Fork and clone the repository:**
   ```bash
   git clone https://github.com/<your-username>/smart-terminal.git
   cd smart-terminal
   ```

2. **Configure your environment:**
   ```bash
   export GROQ_API_KEY="your-groq-key"
   export GEMINI_API_KEY="your-gemini-key"
   ```

3. **Build the project:**
   ```bash
   cargo build
   ```

4. **Run the test suite:**
   ```bash
   cargo test
   ```

5. **Test the CLI directly:**
   ```bash
   cargo run -- next-cmd "list all git branches sorted by date"
   cargo run -- memory show
   cargo run -- investigate --one-off "Explain the project layout"
   ```

6. **Test the Zsh shell integration:**
   ```bash
   source scripts/zsh/smart-terminal.zsh
   ```
   Press `^G` on your prompt to test ghost command generation.

---

## Codebase Architecture at a Glance

Before jumping into code, check out our comprehensive guides:
- [docs/architecture.md](docs/architecture.md) — Architectural overview, data flows, and layer boundaries.
- [docs/development.md](docs/development.md) — Developer setup, protocol details, and testing notes.

Here is a quick overview of how code is organized:

```
src/
├── cli/              # Clap argument parsing, command dispatch, and terminal presentation
├── core/             # Infrastructure-neutral contracts (traits) and domain models
├── agent/            # Agent workflows (next_cmd, investigator), ReAct loop, action space, prompts
├── providers/        # LLM client implementations (Groq, Gemini, OpenRouter, OpenAI-compatible)
├── tools/            # Agent capabilities (bash, read_file, read_dir, docker, git)
├── persistence/      # Memory storage and session state adapters (JSON/filesystem)
└── multithreading/   # Background job execution engine
```

### Key Design Principles

- **Layer Separation**: `src/cli` should handle argument parsing and output presentation only, not business logic.
- **Provider Agnostic**: Workflows interact with abstract LLM and tool contracts (`src/core`), not direct provider APIs.
- **Wire Protocol Synchronization**: The stdout protocol between the Rust binary and shell hooks (`scripts/zsh/smart-terminal.zsh`) must be kept in sync. Diagnostic messages must go to `stderr`, never `stdout`.

---

## Development Workflow

1. **Create a branch:**
   ```bash
   git checkout -b feature/my-feature-name
   ```
   or
   ```bash
   git checkout -b fix/issue-description
   ```

2. **Format and lint your code:**
   Ensure your code matches standard Rust formatting and passes Clippy:
   ```bash
   cargo fmt --check
   cargo clippy --all-targets --all-features
   ```

3. **Run tests:**
   ```bash
   cargo test
   ```
   If you add new functionality, please consider adding corresponding unit or integration tests under `tests/`.

4. **Commit your changes:**
   Keep commit messages clear and descriptive:
   ```bash
   git commit -m "feat(providers): add Ollama local provider support"
   ```

5. **Submit a Pull Request:**
   Push your branch to your fork and open a Pull Request against the `main` branch.

---

## Pull Request Guidelines

- **Keep it focused**: Smaller, focused PRs are easier to review, test, and merge.
- **Explain the "why"**: Describe the problem you are solving and how you verified your changes.
- **Include command output or screenshots**: For UI/CLI changes, a quick terminal snippet or GIF is appreciated!
- **Don't break CI**: Ensure `cargo test` passes locally before submitting.

---

## Interested in Becoming a Co-Maintainer?

If you enjoy working on terminal tools or LLM agents and would like to help guide `smart-terminal` into its next chapter (triaging issues, reviewing PRs, planning releases), we would love to have you!

Feel free to open an issue or reach out directly to **Jason Stefanou**:
- **Email**: jasonstephanou3@gmail.com
- **GitHub**: [@sdi2200246](https://github.com/sdi2200246)

Thank you for helping keep open source vibrant and collaborative! 🚀
