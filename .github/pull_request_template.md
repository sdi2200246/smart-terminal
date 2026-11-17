## Description
<!-- Briefly describe the changes introduced by this pull request. -->

## Motivation & Context
<!-- Why is this change required? What problem does it solve? If it fixes an open issue, link to it here (e.g. fixes #123). -->

## Type of Change
- [ ] 🐛 Bug fix (non-breaking change which fixes an issue)
- [ ] ✨ New feature (non-breaking change which adds functionality)
- [ ] ⚡ Performance improvement
- [ ] 🐚 Shell integration improvement (Zsh / Fish / Bash / Nushell)
- [ ] 🤖 Provider / LLM integration (Groq, Gemini, Ollama, etc.)
- [ ] 📝 Documentation update
- [ ] 🧹 Refactoring / Code cleanup

## How Has This Been Tested?
<!-- Please describe the tests you ran to verify your changes. Include any cargo test results or manual terminal verification steps. -->

- [ ] `cargo test` passes locally
- [ ] `cargo clippy --all-targets` passes with no warnings
- [ ] `cargo fmt --check` passes
- [ ] Manual testing (please describe below):

## Protocol & Wire Format Check (if applicable)
- [ ] Does this change modify stdout for `next-cmd` or suggestion handoff?
  - [ ] If yes, was `scripts/zsh/smart-terminal.zsh` updated in lockstep?
  - [ ] Are diagnostic messages directed to stderr instead of stdout?

## Additional Notes / Screenshots
<!-- Add any extra context, screenshots, or screen recordings if applicable. -->
