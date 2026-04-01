# AGENTS.md - Codex Claude Soul Edition

A fork of [OpenAI's Codex CLI](https://github.com/openai/codex) with Claude Code's personality, prompt architecture, and TUI polish. The runtime, sandbox, and tool execution are stock Codex. Everything that changed is the soul.

## Build & Test

All Rust code lives in `codex-rs/`. Crate names are prefixed with `codex-` (e.g., `codex-rs/core/` is crate `codex-core`).

```sh
cd codex-rs

# Build the release binary
cargo build --release -p codex-cli

# Run tests for a specific crate (preferred)
cargo test -p codex-tui
cargo test -p codex-protocol

# Full test suite (ask before running -- it's slow)
cargo test

# Format (always run after changes)
just fmt

# Lint (scope to changed crate)
just fix -p codex-tui

# Argument comment lint
just argument-comment-lint

# Snapshot tests -- auto-accept with:
INSTA_UPDATE=always cargo test -p codex-tui
# Or review manually:
cargo insta pending-snapshots -p codex-tui
cargo insta accept -p codex-tui
```

Linux builds require `libcap-dev` for bubblewrap sandbox: `sudo apt-get install pkg-config libcap-dev`.

CI workflow: `.github/workflows/build-claude-soul.yml` (the only active workflow -- all upstream workflows are left in place but only `build-claude-soul.yml` is ours and runs on push/PR).

## Architecture: The Prompt System

The core change is a modular prompt system mirroring Claude Code's function-per-section design.

### Section Files

`codex-rs/protocol/src/prompts/sections/` contains 21 markdown files. Each is a self-contained prompt section.

**Always-on (8 core sections, every session):**
- `identity.md`, `system.md`, `tone.md`, `doing_tasks.md`, `actions.md`, `tools.md`, `output.md`, `how_you_work.md`

**Always-on via PromptFeatures (7 sections, stable, default on):**
- `verification.md` (Feature::PromptVerification)
- `suggestions.md` (Feature::PromptSuggestions)
- `skills.md` (Feature::PromptSkills)
- `advisor.md` (Feature::PromptAdvisor)
- `worktree.md` (Feature::PromptWorktree)
- `stuck.md` (PromptFeatures.stuck, default true, no dedicated Feature flag)
- `git_protocol.md` (unconditional, appended at tail)

**User-togglable (1 section, via /experimental):**
- `insights.md` -- Feature::PromptInsights, Stage::Experimental

**Invoked-only (5 sections, NOT assembled into base prompt):**
- `compaction.md` -- used by /compact handler
- `simplify.md` -- used by /simplify skill
- `session_titles.md` -- used by title generation
- `memory.md` -- reference for native memory system (DISABLED, see below)
- `dream.md` -- reference for memory consolidation (DISABLED, see below)

### Assembly

`assemble_base_instructions()` in `codex-rs/protocol/src/models.rs` concatenates sections based on `PromptFeatures`. Called at session init from `codex-rs/core/src/codex.rs:575`.

Feature flags in `codex-rs/features/src/lib.rs` control the togglable sections: `PromptVerification`, `PromptSuggestions`, `PromptSkills`, `PromptInsights`, `PromptAdvisor`, `PromptWorktree`.

### Compaction & Init Prompts

- `codex-rs/core/templates/compact/prompt.md` -- structured 5-section handoff format with no-tool-call guardrail
- `codex-rs/tui/prompt_for_init_command.md` -- opinionated senior-dev tone, 150-300 words

## TUI Changes

All in `codex-rs/tui/src/`:

- `spinner_verbs.rs` -- 171 whimsical verbs including "Codexing" Easter egg
- `exec_cell/render.rs` -- diamond status indicators: `◇` (running), `◆` (completed)
- `markdown_render.rs` / `markdown_stream.rs` -- blockquote prefix `▎` replacing `>`
- `status/` -- effort level symbols `○◐●◉` in status line
- `chatwidget.rs` -- heavy chevron prompt `❯`, PLACEHOLDERS array (8 entries), dynamic composer placeholder
- `slash_command.rs` -- all command descriptions rewritten
- `tooltips.txt` (in `codex-rs/tui/`) -- tooltip text rewritten
- `bottom_pane/pending_thread_approvals.rs` -- "Codex needs your approval" banner
- Auto-session titles on first turn: `derive_session_title()` in `codex-rs/core/src/tasks/mod.rs`

### Dynamic Composer Placeholder

After each agent turn, the dimmed placeholder text in the composer updates with a contextual next-step suggestion extracted from the agent's response. This replaces Claude Code's separate prompt suggestion chips with a single in-place hint.

How it works:
1. `extract_next_step_suggestion()` in `chatwidget.rs` scans the last 5 lines of the agent response.
2. Matches "Want me to X?" / "Should I X?" / "Shall I X?" patterns and extracts the action as a short phrase.
3. Also matches backtick-wrapped commands (e.g., `` `npm test` ``, `` `/review` ``).
4. If a match is found (3-60 chars for phrases, 2-40 for commands), it is passed to `set_composer_placeholder()` on `BottomPane`, which delegates to `ChatComposer::set_placeholder_text()`.

Defined in `chatwidget.rs` at line ~11121. Wired through `bottom_pane/mod.rs:325`.

## Built-in Skills

`codex-rs/skills/src/assets/samples/`:
- `simplify/SKILL.md` -- three-dimension code review (reuse, quality, efficiency)
- `stuck/SKILL.md` -- 5-step protocol to break out of debugging loops

## MCP Server Instructions

Codex handles MCP server instructions natively through its plugin injection system. `build_plugin_injections()` in `codex-rs/core/src/plugins/injection.rs` assembles plugin capability summaries and tool info into `ResponseItem` injections that are included in the conversation context. No custom work was needed here -- it works out of the box.

## Native Feature Status

| Feature | Flag | Stage | default_enabled | Status |
|---------|------|-------|-----------------|--------|
| Hooks | `Feature::CodexHooks` | Stable | `true` | Working |
| Git commit guidance | `Feature::CodexGitCommit` | Stable | `true` | Working |
| Child agents AGENTS.md | `Feature::ChildAgentsMd` | Stable | `true` | Working |
| Prompt: Verification | `Feature::PromptVerification` | Stable | `true` | Working |
| Prompt: Suggestions | `Feature::PromptSuggestions` | Stable | `true` | Working |
| Prompt: Skills | `Feature::PromptSkills` | Stable | `true` | Working |
| Prompt: Advisor | `Feature::PromptAdvisor` | Stable | `true` | Working |
| Prompt: Worktree | `Feature::PromptWorktree` | Stable | `true` | Working |
| Prompt: Insights | `Feature::PromptInsights` | Experimental | `false` | Working (user-togglable) |
| MemoryTool | `Feature::MemoryTool` | UnderDevelopment | `false` | **DISABLED** |
| GhostCommit (undo) | `Feature::GhostCommit` | Stable | `false` | **DISABLED** |

**Why MemoryTool is disabled:** Spawns background agents with a 1-hour lease that block the session on the first run when no rollout history exists. Reverted to `Stage::UnderDevelopment`, `default_enabled: false`. The prompt section files (`memory.md`, `dream.md`) are kept as reference but are never assembled into the base prompt.

**Why GhostCommit is disabled:** Creates background ghost commits (silent snapshots) for undo support. Reverted to `default_enabled: false` for safety -- silent commits can surprise users and interfere with git workflows.

## CI/CD

`.github/workflows/build-claude-soul.yml` is the only active custom workflow. It:
- Triggers on push to `claude-code-personality` and `main`, PRs to `main`, and manual dispatch
- Builds release binaries for three targets: `x86_64-unknown-linux-gnu`, `aarch64-apple-darwin`, `x86_64-pc-windows-msvc`
- Runs `codex-tui` tests with `INSTA_UPDATE=always`
- Uploads binary artifacts with 30-day retention
- Caches cargo registry and build artifacts

All upstream OpenAI workflows (`rust-ci.yml`, `ci.yml`, `bazel.yml`, etc.) are left in the repo but are not ours and are not relevant to this fork's CI. They trigger on PRs to their own branches and will not fire for our work.

### Trailing-Comma Brew Test Fix

A pre-existing bug in the upstream `codex-rs/tui/src/updates.rs` brew cask JSON test (`extract_version_from_brew_api_json`) used a JSON object without trailing commas, which was fine, but the snapshot serialization for other tests could produce trailing commas that fail `serde_json` parsing. The test JSON was kept clean as part of snapshot acceptance.

## Code Style

Follow the upstream AGENTS.md conventions (they are merged into this file's scope):
- Inline `format!` args. Collapse `if` statements. Use method references over closures.
- Avoid `bool` parameters -- prefer enums. Use `/*param_name*/` comments for opaque literals.
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
| Dynamic placeholder | `codex-rs/tui/src/chatwidget.rs` (`extract_next_step_suggestion`) |
| Placeholder wiring | `codex-rs/tui/src/bottom_pane/mod.rs` (`set_composer_placeholder`) |
| Diamond indicators | `codex-rs/tui/src/exec_cell/render.rs` |
| Compact prompt | `codex-rs/core/templates/compact/prompt.md` |
| Init prompt | `codex-rs/tui/prompt_for_init_command.md` |
| Auto-session titles | `codex-rs/core/src/tasks/mod.rs` (`derive_session_title`) |
| Simplify skill | `codex-rs/skills/src/assets/samples/simplify/SKILL.md` |
| Stuck skill | `codex-rs/skills/src/assets/samples/stuck/SKILL.md` |
| Plugin injection (MCP) | `codex-rs/core/src/plugins/injection.rs` (`build_plugin_injections`) |
| Brew update check | `codex-rs/tui/src/updates.rs` |
| CI workflow | `.github/workflows/build-claude-soul.yml` |
