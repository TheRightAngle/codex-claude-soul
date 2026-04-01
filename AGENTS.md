# AGENTS.md - Codex Claude Soul Edition

This is a fork of [OpenAI's Codex CLI](https://github.com/openai/codex) rewritten to behave like Anthropic's Claude Code. The personality, prompt system, and TUI have been overhauled. The runtime, sandbox, and tool execution are stock Codex.

## Build & Test

All Rust code lives in `codex-rs/`. Crate names are prefixed with `codex-` (e.g., `codex-rs/core/` is crate `codex-core`).

```sh
cd codex-rs

# Build the release binary
cargo build --release -p codex-cli

# Run tests for a specific crate (preferred)
cargo test -p codex-tui
cargo test -p codex-protocol

# Full test suite (ask before running — it's slow)
cargo test

# Format (always run after changes)
just fmt

# Lint (scope to changed crate)
just fix -p codex-tui

# Argument comment lint
just argument-comment-lint

# Snapshot tests — auto-accept with:
INSTA_UPDATE=always cargo test -p codex-tui
# Or review manually:
cargo insta pending-snapshots -p codex-tui
cargo insta accept -p codex-tui
```

Linux builds require `libcap-dev` for bubblewrap sandbox: `sudo apt-get install pkg-config libcap-dev`.

CI workflow: `.github/workflows/build-claude-soul.yml` builds Linux, macOS, Windows.

## Architecture: The Prompt System

The core change is a modular prompt system mirroring Claude Code's function-per-section design.

### Section Files

`codex-rs/protocol/src/prompts/sections/` contains 21 markdown files. Each is a self-contained prompt section.

**Always-on (8 sections, every session):**
- `identity.md`, `system.md`, `tone.md`, `doing_tasks.md`, `actions.md`, `tools.md`, `output.md`, `how_you_work.md`

**Feature-gated (7 sections, stable defaults-on):**
- `verification.md`, `suggestions.md`, `skills.md`, `advisor.md`, `worktree.md`, `stuck.md`, `git_protocol.md`

**User-togglable (1 section, via /experimental):**
- `insights.md` — educational insight blocks

**Invoked-only (5 sections, NOT assembled into base prompt):**
- `compaction.md` — used by /compact handler
- `simplify.md` — used by /simplify skill
- `session_titles.md` — used by title generation
- `memory.md` — reference for native memory system
- `dream.md` — reference for memory consolidation

### Assembly

`assemble_base_instructions()` in `codex-rs/protocol/src/models.rs` concatenates sections based on `PromptFeatures`. Called at session init from `codex-rs/core/src/codex.rs:575`.

Feature flags in `codex-rs/features/src/lib.rs` control the togglable sections: `PromptVerification`, `PromptSuggestions`, `PromptSkills`, `PromptInsights`, `PromptAdvisor`, `PromptWorktree`.

### Compaction & Init Prompts

- `codex-rs/core/templates/compact/prompt.md` — structured 5-section handoff format with no-tool-call guardrail
- `codex-rs/tui/prompt_for_init_command.md` — opinionated senior-dev tone, 150-300 words

## TUI Changes

All in `codex-rs/tui/src/`:

- `spinner_verbs.rs` — 171 whimsical verbs including "Codexing" Easter egg
- `exec_cell/render.rs` — diamond status indicators: `◇` (running), `◆` (completed)
- `markdown_render.rs` / `markdown_stream.rs` — blockquote prefix `▎` replacing `>`
- `status/` — effort level symbols `○◐●◉` in status line
- `chatwidget.rs` — heavy chevron prompt `❯`, composer PLACEHOLDERS array (8 entries)
- `slash_command.rs` — all command descriptions rewritten
- `tooltips.txt` (in `codex-rs/tui/`) — tooltip text rewritten
- `bottom_pane/pending_thread_approvals.rs` — "Codex needs your approval" banner
- Auto-session titles on first turn: `derive_session_title()` in `codex-rs/core/src/tasks/mod.rs`

## Built-in Skills

`codex-rs/skills/src/assets/samples/`:
- `simplify/SKILL.md` — three-dimension code review (reuse, quality, efficiency)
- `stuck/SKILL.md` — break out of debugging loops (5-step reset protocol)

## Native Features

**Enabled (Stable):**
- `Feature::CodexHooks` — hooks.json lifecycle hooks
- `Feature::CodexGitCommit` — commit attribution guidance
- `Feature::ChildAgentsMd` — subagent documentation loading

**Disabled (known issues):**
- `Feature::MemoryTool` — `Stage::UnderDevelopment`, `default_enabled: false`. Spawns background agents with 1-hour lease that block session on first run with no history.
- `Feature::GhostCommit` — `default_enabled: false`. Background snapshot tasks, disabled for safety.

## Code Style

Follow the upstream AGENTS.md conventions (they are merged into this file's scope):
- Inline `format!` args. Collapse `if` statements. Use method references over closures.
- Avoid `bool` parameters — prefer enums. Use `/*param_name*/` comments for opaque literals.
- Modules under 500 LoC (800 max). Do not grow `codex-core` without good reason.
- TUI: use ratatui `Stylize` helpers (`.dim()`, `.bold()`, `.cyan()`). Avoid `.white()`.
- Snapshot tests required for UI changes. Use `pretty_assertions::assert_eq`.
- Never modify `CODEX_SANDBOX_NETWORK_DISABLED_ENV_VAR` or `CODEX_SANDBOX_ENV_VAR` code.

## Key Files Quick Reference

| What | Where |
|------|-------|
| Prompt sections | `codex-rs/protocol/src/prompts/sections/*.md` |
| Section assembler | `codex-rs/protocol/src/models.rs` (`assemble_base_instructions`) |
| Feature flags | `codex-rs/features/src/lib.rs` |
| Session init wiring | `codex-rs/core/src/codex.rs:575` |
| Spinner verbs | `codex-rs/tui/src/spinner_verbs.rs` |
| Slash commands | `codex-rs/tui/src/slash_command.rs` |
| Tooltips | `codex-rs/tui/tooltips.txt` |
| Composer placeholders | `codex-rs/tui/src/chatwidget.rs` (PLACEHOLDERS const) |
| Diamond indicators | `codex-rs/tui/src/exec_cell/render.rs` |
| Compact prompt | `codex-rs/core/templates/compact/prompt.md` |
| Init prompt | `codex-rs/tui/prompt_for_init_command.md` |
| Auto-session titles | `codex-rs/core/src/tasks/mod.rs` (`derive_session_title`) |
| Simplify skill | `codex-rs/skills/src/assets/samples/simplify/SKILL.md` |
| Stuck skill | `codex-rs/skills/src/assets/samples/stuck/SKILL.md` |
| CI workflow | `.github/workflows/build-claude-soul.yml` |
