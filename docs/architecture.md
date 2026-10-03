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

- define reusable contracts and domain types for providers, capabilities, agent sessions, command memory, and investigation sessions
- centralize error types and model metadata
- avoid importing concrete providers, persistence implementations, or tool implementations

Memory-related core types and contracts are grouped in `src/core/memory.rs`. `Memory` and `InvestigationSessionStore` remain separate contracts because command interactions and planner/executor histories have different data and behavior. The shared project-folder index is defined in `src/core/folder_index.rs`.

This layer is intentionally infrastructure-neutral and is meant to be stable as the project evolves. Investigation snapshots and storage contracts do not prescribe files, JSON, or a database.

### 3. Agent layer
Location: `src/agent`

Responsibilities:

- define workflows such as `NextCmd` and `Investigate`
- hold agent orchestration logic
- operate the main reasoning loop that invokes providers and tools

Important files:

- `src/agent/workflows/next_cmd.rs`
- `src/agent/workflows/investigator.rs`
- `src/agent/patterns/react.rs`
- `src/agent/patterns/tool_engine.rs`

### 4. Persistence layer
Location: `src/persistence`

Responsibilities:

- implement the core memory and investigation-session storage contracts
- contain storage-specific details such as paths, JSON encoding, and atomic writes
- keep investigator history separate from next-command interaction memory

Important files:

- `src/persistence/next_cmd_memory.rs` — next-command interaction memory and registration updates
- `src/persistence/investigation_sessions.rs` — investigator conversation history
- `src/persistence/project_roots.rs` — shared lookup of the nearest registered project root

Both stores use the same project-folder index for root resolution, but keep their contents in separate files/directories. Investigator sessions use the resolved root as their key; if no root is registered, they fall back to the current directory. When registering a parent replaces descendant roots, the index retains those old roots as aliases. The investigator store migrates any existing child sessions into the parent session on load and removes the old files after saving the merged history.

The current implementation uses project-keyed JSON files. The CLI composes these adapters with workflows; replacing the file adapter with a database adapter should not require changes to core types or agent workflows.

### 5. Provider layer
Location: `src/providers`

Responsibilities:

- adapter implementations for external LLM providers
- model request/response translation
- provider-specific API handling and error mapping

Current providers:

- `src/providers/groq/*`
- `src/providers/google/*`

Both provider implementations share a codec/client abstraction via `providers::client`.

### 6. Tool layer
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
3. The CLI uses the shared project-root resolver to identify the registered project root. Investigations load planner/executor history from their separate session store; `next-cmd` loads its own interaction memory through its persistence adapter.
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

The project main investigation session is resumed by default. The planner and executor have separate serialized `AgentSession` histories, including tool calls, results, and Gemini thought signatures. Each run appends its new question to the existing planner conversation, then appends the new question and plan to the existing executor conversation. Structured output uses a separate temporary session so it does not erase the active transcript. A run from a subdirectory resolves to its registered ancestor root; without a registered root, it uses the current directory. If a parent replaces registered child roots, existing investigator sessions from those child roots are migrated into the parent session. Provider/model switching and migration between incompatible provider transcript formats are not supported.

Use `investigate --one-off "question"` to avoid reading or changing the main session. Use `investigate session clear` to delete it. Each resumed invocation gets a fresh tool-step allowance; historical calls do not consume the new run's budget.

### `memory`

`memory` stores interaction patterns in a project-scoped way so that future command suggestions can use prior behavior without being global across unrelated folders.

`memory init` registers the current directory as a project root. A registered root applies to its subdirectories unless a more specific root is registered. Registering a parent replaces registered descendant roots; their next-command interaction files are removed according to the existing memory behavior, while investigator session history is migrated to the parent-root session by the investigator store.

Typical commands:

- `smart-terminal memory init`
- `smart-terminal memory show`
- `smart-terminal memory clear`
- `smart-terminal memory delete`

## Dependency flow

The dependency direction is intentionally layered:

- `cli` is the composition root: it wires provider, persistence adapter, workflows, and commands
- `persistence` depends on `core` contracts; core never depends on persistence or storage infrastructure
- workflows depend on agents, tool capabilities, and core contracts
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

### Changing persistence backends

1. Keep the domain snapshot and storage trait in `src/core`.
2. Implement the trait in `src/persistence` (or a future infrastructure adapter module) without importing agent or provider modules.
3. Change only the CLI composition to select the new implementation.
4. Keep serialization round-trips and provider-compatible event mapping covered by tests.

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
