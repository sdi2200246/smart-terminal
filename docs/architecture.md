# Architecture

This document explains how the project is structured and how the runtime moves from CLI input to LLM-driven command generation or investigation.

## Project overview

`smart-terminal` is a Rust CLI that augments shell usage with:

- `next-cmd`: suggests the next shell command from context
- `investigate`: answers questions about the local filesystem, repo, or environment
- `memory`: stores project-scoped interaction history for future shell suggestions

The project is organized around a clean separation of concerns:

- CLI entrypoints and command parsing
- core abstractions for LLM providers, capabilities, and session state
- workflow logic for each feature
- provider-specific adapters for Groq and Google Gemini
- reusable shell and filesystem tools

## High-level layering

The following layers are intentionally distinct:

### 1. CLI layer
Location: `src/cli`

Responsibilities:

- parse CLI arguments via Clap
- route commands to the appropriate workflow
- render terminal output and tool call progress

Main files:

- `src/cli/cli.rs` — top-level command definitions
- `src/cli/cmds/*.rs` — command-specific execution entrypoints
- `src/cli/presenters/*.rs` — output rendering

### 2. Core layer
Location: `src/core`

Responsibilities:

- define reusable contracts like `LLMProvider`, `Capability`, `Session`, and `Memory`
- centralize error types and model metadata
- avoid importing concrete providers or tool implementations

This layer is intentionally dependency-light and is meant to be stable as the project evolves.

### 3. Agent layer
Location: `src/agent`

Responsibilities:

- define workflows such as `NextCmd` and `Investigate`
- hold agent orchestration logic
- operate the main reasoning loop that invokes providers and tools
- manage persistent project memory

Important files:

- `src/agent/workflows/next_cmd.rs`
- `src/agent/workflows/investigator.rs`
- `src/agent/patterns/react.rs`
- `src/agent/patterns/tool_engine.rs`
- `src/agent/memory/mod.rs`

### 4. Provider layer
Location: `src/providers`

Responsibilities:

- adapter implementations for external LLM providers
- model request/response translation
- provider-specific API handling and error mapping

Current providers:

- `src/providers/groq/*`
- `src/providers/google/*`

Both provider implementations share a codec/client abstraction via `providers::client`.

### 5. Tool layer
Location: `src/tools`

Responsibilities:

- provide concrete capabilities available to agents
- perform actions like shell execution, directory reads, file reads, git inspection, and Docker inspection

Examples:

- `src/tools/bash.rs`
- `src/tools/read_dir.rs`
- `src/tools/read_file.rs`
- `src/tools/git_diff.rs`
- `src/tools/git_log.rs`
- `src/tools/docker.rs`

Each tool implements the project capability abstraction, allowing the agent loop to treat them uniformly.

## Runtime flow

The runtime entrypoint is `src/main.rs`.

1. The binary parses the CLI input.
2. `Router::dispatch` matches the command:
   - `next-cmd`
   - `memory`
   - `investigate`
3. The corresponding workflow loads any needed memory or runtime state.
4. The workflow sets up one or more agents.
5. Each agent assembles a tool registry and runs a loop.
6. The loop:
   - calls the selected LLM
   - receives plan/tool calls
   - executes tools
   - sends results back into the model
   - stops when the model signals completion
7. The workflow converts the final agent response into user-facing output.

## Command-level behavior

### `next-cmd`

`next-cmd` is designed to predict the next shell command based on:

- current directory or working context
- shell prompt state
- partial input
- prior project memory

This feature is the most lightweight and fast path, and it is the one most directly tied to shell interaction.

### `investigate`

`investigate` uses a multi-step planning/execution pattern:

- planner agent: reads filesystem and shell context and proposes an investigation plan
- executor agent: runs the plan with tools and returns grounded findings

This makes it useful for questions such as:

- what does this codebase do?
- where is a feature implemented?
- why is a test failing?
- what changed between branches?
- what is currently installed or configured on the machine?

### `memory`

`memory` stores interaction patterns in a project-scoped way so that future command suggestions can use prior behavior without being global across unrelated folders.

Typical commands:

- `smart-terminal memory init`
- `smart-terminal memory show`
- `smart-terminal memory clear`
- `smart-terminal memory delete`

## Dependency flow

The dependency direction is intentionally layered:

- `cli` depends on workflows and commands
- workflows depend on agents and tool capabilities
- agents depend on the core contracts and providers
- providers depend on the generic transport and response codecs
- tools depend on shell and filesystem primitives

This helps keep concrete implementation choices isolated and easier to replace.

## Extension points

### Adding a new tool

1. Add a new implementation in `src/tools`
2. Implement the capability contract expected by the core layer
3. Register the tool in the relevant agent or workflow setup
4. Validate tool behavior via the existing cmd/feature flow

### Adding a provider

1. Add a provider module under `src/providers`
2. Implement the provider interface and response adaptation
3. Wire it into the CLI setup logic
4. Ensure the provider-specific prompt or codec aligns with the existing model contracts

### Adding a new workflow

1. Create a workflow module under `src/agent/workflows`
2. Define the orchestration pattern
3. Add a command in `src/cli/cli.rs`
4. Connect the CLI dispatch and output rendering

## Testing and validation

The repository is Rust-based and should be validated using Cargo commands such as:

```bash
cargo test
cargo run -- --help
```

In practice, the most valuable validation for this application is often end-to-end CLI behavior: verifying the command is routed correctly and the LLM-backed workflow still produces useful output.

## Notes

The codebase has a strong separation between reasoning, environment access, and presentation. That design is the project’s biggest architectural strength: it keeps the shell integration and provider logic independent from the user-facing command flow.
