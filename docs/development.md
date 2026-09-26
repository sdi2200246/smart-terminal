# Development Guide

This guide covers local setup, common workflows, and the conventions that matter most when contributing to `smart-terminal`.

## Prerequisites

- macOS or Linux
- zsh
- Rust toolchain (`rustc`, `cargo`)
- API keys for the model providers you plan to use:
  - `GROQ_API_KEY`
  - `GEMINI_API_KEY` or `GOOGLE_API_KEY`

## Local setup

### 1. Install Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
```

### 2. Configure environment variables

Add the keys to your shell profile:

```bash
cat <<'EOF' >> ~/.zshrc
export GROQ_API_KEY="your_groq_api_key"
export GEMINI_API_KEY="your_gemini_api_key"
# or: export GOOGLE_API_KEY="your_gemini_api_key"
EOF
source ~/.zshrc
```

### 3. Build the project

```bash
cd /path/to/smart-terminal
cargo build
```

### 4. Run the CLI

```bash
cargo run -- --help
```

You should see the available commands:

- `next-cmd`
- `memory`
- `investigate`

## Typical development tasks

### Run the full test suite

```bash
cargo test
```

### Run a focused command check

```bash
cargo run -- next-cmd "list files"
```

### Build a release binary

```bash
cargo build --release
```

### Install locally

```bash
cargo install --path .
```

## Project structure

Key source directories:

- `src/cli` — command parsing and terminal presentation
- `src/core` — abstraction layer for capabilities and model contracts
- `src/agent` — workflows, agent loops, and memory orchestration
- `src/providers` — LLM provider implementations
- `src/tools` — environment interaction tools
- `scripts/zsh` — shell integration hook

## Contribution conventions

### Rust style

- Prefer small, clearly named modules
- Keep provider-specific code isolated in `src/providers`
- Keep LLM/tool logic in workflows or agent-specific modules rather than mixing it into the CLI layer
- Use existing abstractions before introducing new dependencies or patterns

### Design guidance

The codebase intentionally separates layers. When making changes:

- do not let `src/cli` become the home for business logic
- do not let providers leak directly into workflows unless necessary
- keep tool functionality consistent with the capability abstraction
- keep memory flows project-scoped rather than global

### Validation before submitting

At minimum, run:

```bash
cargo test
cargo run -- --help
```

If your change affects runtime behavior, prefer an end-to-end check that exercises the command affected by the change.

## Shell integration

The zsh hook lives at:

```bash
scripts/zsh/smart-terminal.zsh
```

To enable it in a shell session:

```bash
source /path/to/smart-terminal/scripts/zsh/smart-terminal.zsh
```

For a persistent setup, include it in `~/.zshrc`:

```bash
cat <<'EOF' >> ~/.zshrc
source /path/to/smart-terminal/scripts/zsh/smart-terminal.zsh
reload() { source ~/.zshrc; }
EOF
source ~/.zshrc
```

## Troubleshooting

### The CLI is not recognized

Make sure Cargo's binary directory is on your `PATH`:

```bash
source "$HOME/.cargo/env"
```

If installed locally with `cargo install --path .`, confirm `~/.cargo/bin` is in your path.

### API calls fail

Check that environment variables are exported in the shell that launches the app:

```bash
echo "$GROQ_API_KEY"
echo "$GEMINI_API_KEY"
```

### The shell hook does not appear

Ensure the integration file path is correct and that you ran `source ~/.zshrc` after editing it.

## Recommended workflow for contributors

1. Read this guide and the architecture overview.
2. Start with a focused feature or bug fix.
3. Validate with targeted Rust commands.
4. Keep changes small and aligned with the modular design.
5. Update docs if the command surface or architecture changes.

## Additional resources

- [README.md](../README.md)
- [docs/architecture.md](architecture.md)
