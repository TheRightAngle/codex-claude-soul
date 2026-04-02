# AGENTS.md - Codex Claude Soul Edition

A fork of [OpenAI's Codex CLI](https://github.com/openai/codex) with Claude Code's personality, prompt architecture, and TUI polish. The runtime, sandbox, and tool execution are stock Codex. Everything that changed is the soul.

- Crate names are prefixed with `codex-`. For example, the `core` folder's crate is named `codex-core`
- When using format! and you can inline variables into {}, always do that.
- Install any commands the repo relies on (for example `just`, `rg`, or `cargo-insta`) if they aren't already available before running instructions here.
- Never add or modify any code related to `CODEX_SANDBOX_NETWORK_DISABLED_ENV_VAR` or `CODEX_SANDBOX_ENV_VAR`.
  - You operate in a sandbox where `CODEX_SANDBOX_NETWORK_DISABLED=1` will be set whenever you use the `shell` tool. Any existing code that uses `CODEX_SANDBOX_NETWORK_DISABLED_ENV_VAR` was authored with this fact in mind. It is often used to early exit out of tests that the author knew you would not be able to run given your sandbox limitations.
  - Similarly, when you spawn a process using Seatbelt (`/usr/bin/sandbox-exec`), `CODEX_SANDBOX=seatbelt` will be set on the child process. Integration tests that want to run Seatbelt themselves cannot be run under Seatbelt, so checks for `CODEX_SANDBOX=seatbelt` are also often used to early exit out of tests, as appropriate.
- Always collapse if statements per https://rust-lang.github.io/rust-clippy/master/index.html#collapsible_if
- Always inline format! args when possible per https://rust-lang.github.io/rust-clippy/master/index.html#uninlined_format_args
- Use method references over closures when possible per https://rust-lang.github.io/rust-clippy/master/index.html#redundant_closure_for_method_calls
- Avoid bool or ambiguous `Option` parameters that force callers to write hard-to-read code such as `foo(false)` or `bar(None)`. Prefer enums, named methods, newtypes, or other idiomatic Rust API shapes when they keep the callsite self-documenting.
- When you cannot make that API change and still need a small positional-literal callsite in Rust, follow the `argument_comment_lint` convention:
  - Use an exact `/*param_name*/` comment before opaque literal arguments such as `None`, booleans, and numeric literals when passing them by position.
  - Do not add these comments for string or char literals unless the comment adds real clarity; those literals are intentionally exempt from the lint.
  - The parameter name in the comment must exactly match the callee signature.
  - You can run `just argument-comment-lint` to run the lint check locally. This is powered by Bazel, so running it the first time can be slow if Bazel is not warmed up, though incremental invocations should take <15s. Most of the time, it is best to update the PR and let CI take responsibility for checking this (or run it asynchronously in the background after submitting the PR). Note CI checks all three platforms, which the local run does not.
- When possible, make `match` statements exhaustive and avoid wildcard arms.
- Newly added traits should include doc comments that explain their role and how implementations are expected to use them.
- When writing tests, prefer comparing the equality of entire objects over fields one by one.
- When making a change that adds or changes an API, ensure that the documentation in the `docs/` folder is up to date if applicable.
- If you change `ConfigToml` or nested config types, run `just write-config-schema` to update `codex-rs/core/config.schema.json`.
- If you change Rust dependencies (`Cargo.toml` or `Cargo.lock`), run `just bazel-lock-update` from the
  repo root to refresh `MODULE.bazel.lock`, and include that lockfile update in the same change.
- After dependency changes, run `just bazel-lock-check` from the repo root so lockfile drift is caught
  locally before CI.
- Bazel does not automatically make source-tree files available to compile-time Rust file access. If
  you add `include_str!`, `include_bytes!`, `sqlx::migrate!`, or similar build-time file or
  directory reads, update the crate's `BUILD.bazel` (`compile_data`, `build_script_data`, or test
  data) or Bazel may fail even when Cargo passes.
- Do not create small helper methods that are referenced only once.
- Avoid large modules:
  - Prefer adding new modules instead of growing existing ones.
  - Target Rust modules under 500 LoC, excluding tests.
  - If a file exceeds roughly 800 LoC, add new functionality in a new module instead of extending
    the existing file unless there is a strong documented reason not to.
  - This rule applies especially to high-touch files that already attract unrelated changes, such
    as `codex-rs/tui/src/app.rs`, `codex-rs/tui/src/bottom_pane/chat_composer.rs`,
    `codex-rs/tui/src/bottom_pane/footer.rs`, `codex-rs/tui/src/chatwidget.rs`,
    `codex-rs/tui/src/bottom_pane/mod.rs`, and similarly central orchestration modules.
  - When extracting code from a large module, move the related tests and module/type docs toward
    the new implementation so the invariants stay close to the code that owns them.
- When running Rust commands (e.g. `just fix` or `cargo test`) be patient with the command and never try to kill them using the PID. Rust lock can make the execution slow, this is expected.

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

`codex-rs/protocol/src/prompts/sections/` contains 23 markdown files. Each is a self-contained prompt section.

**Always-on (8 core sections, every session):**
- `identity.md`, `system.md`, `tone.md`, `doing_tasks.md`, `actions.md`, `tools.md`, `output.md`, `how_you_work.md`
- Note: `output.md` uses Claude Code's ant-only "Communicating with the user" prose version (not the public "Output efficiency" version)
- Note: `tone.md` omits the non-ant "Your responses should be short and concise" line

**Always-on via PromptFeatures (7 sections, stable, default on):**
- `verification.md` (Feature::PromptVerification)
- `suggestions.md` (Feature::PromptSuggestions)
- `skills.md` (Feature::PromptSkills)
- `advisor.md` (Feature::PromptAdvisor)
- `worktree.md` (Feature::PromptWorktree)
- `stuck.md` (PromptFeatures.stuck, default true, no dedicated Feature flag)
- `git_protocol.md` (unconditional, appended at tail)

**Contextual (set by runtime state, not user toggle):**
- `auto_mode.md` (PromptFeatures.auto_mode) -- 6 behavioral rules for autonomous execution, injected when approval_policy is `Never`
- `plan_mode.md` (PromptFeatures.plan_mode) -- 5-phase enhanced plan mode workflow

**User-togglable (1 section, via /experimental):**
- `insights.md` -- Feature::PromptInsights, Stage::Experimental

**Invoked-only (5 sections, NOT assembled into base prompt):**
- `compaction.md` -- used by /compact handler
- `simplify.md` -- used by /simplify skill
- `session_titles.md` -- used by title generation
- `memory.md` -- reference for native memory system (DISABLED, see below)
- `dream.md` -- reference for memory consolidation (DISABLED, see below)

### System Reminders

`codex-rs/protocol/src/prompts/reminders/` contains 37 contextual reminder templates (matching Claude Code's full set) injected as developer messages at runtime. Accessible via `codex_protocol::models::reminders::*`.

**Task/plan tracking (4):** `task_tools.md`, `todowrite_reminder.md`, `verify_plan.md`, `verify_plan_reminder.md`

**Plan mode lifecycle (7):** `plan_mode_5_phase.md`, `plan_mode_iterative.md`, `plan_mode_subagent.md`, `plan_mode_re_entry.md`, `exited_plan_mode.md`, `plan_file_reference.md`, `ultraplan_mode.md`

**Session & context (5):** `session_continuation.md`, `compact_reference.md`, `output_style.md`, `invoked_skills.md`, `skill_invoked.md`

**Budget & usage (3):** `token_usage.md`, `budget_warning.md`, `usd_budget.md`

**File state (4):** `file_modified_externally.md`, `file_empty.md`, `file_truncated.md`, `file_shorter_than_offset.md`

**Hook lifecycle (3):** `hook_blocking.md`, `hook_success.md`, `hook_stopped_continuation.md`

**Tool & agent events (3):** `tool_denied.md`, `agent_mention.md`, `btw_side_question.md`

**MCP & diagnostics (3):** `mcp_server_status.md`, `mcp_no_content.md`, `diagnostics_detected.md`

**IDE integration (2):** `ide_file_opened.md`, `ide_lines_selected.md`

**Security (1):** `malware_analysis_warning.md`

**Team coordination (2):** `team_coordination.md`, `team_shutdown.md`

### Assembly

`assemble_base_instructions()` in `codex-rs/protocol/src/models.rs` concatenates sections based on `PromptFeatures`. Called at session init from `codex-rs/core/src/codex.rs:575`.

Feature flags in `codex-rs/features/src/lib.rs` control the togglable sections: `PromptVerification`, `PromptSuggestions`, `PromptSkills`, `PromptInsights`, `PromptAdvisor`, `PromptWorktree`.

Runtime state flags: `auto_mode` (set when `AskForApproval::Never`), `plan_mode` (set when plan collaboration mode is active).

### Compaction & Init Prompts

- `codex-rs/core/templates/compact/prompt.md` -- structured 5-section handoff format with no-tool-call guardrail
- `codex-rs/tui/prompt_for_init_command.md` -- 7-phase onboarding flow: explore codebase, fill gaps, write AGENTS.md, suggest skills, suggest hooks, check environment, summary

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
- `debugging/SKILL.md` -- 5-step debug flow: review issue, scan logs, check system state, explain findings, suggest fixes

## Built-in Agent Roles

`codex-rs/core/src/agent/builtins/` contains 29 TOML configs for built-in sub-agent roles, adapted from Claude Code's 34 agent prompt definitions.

**Core agents (6):**
- `explorer.toml` -- codebase exploration (read-only, fast, parallel searches)
- `planner.toml` -- implementation planning (read-only, trade-off analysis, 40-line plan limit)
- `general_purpose.toml` -- multi-step tasks, searching, architecture analysis
- `researcher.toml` -- targeted research (focused queries, evidence-based)
- `worker_fork.toml` -- isolated directive execution (no sub-agents, commits before reporting)
- `awaiter.toml` -- background task polling (exponential timeouts)

**Verification & security (4):**
- `verifier.toml` -- adversarial verification (type-specific strategies, mandatory adversarial probes, PASS/FAIL/PARTIAL)
- `security_monitor.toml` -- autonomous action monitor (24 BLOCK rules, 6 ALLOW exceptions, threat classification)
- `security_review.toml` -- code review for exploitable vulnerabilities (OWASP categories, false positive filtering)
- `auto_mode_reviewer.toml` -- reviews user-defined auto mode classifier rules

**Git & PR (4):**
- `quick_commit.toml` -- single commit creation with git safety protocol
- `quick_pr.toml` -- commit + push + PR creation via gh CLI
- `pr_comments.toml` -- fetch and display GitHub PR comments (inline + PR-level)
- `review_pr.toml` -- code review (correctness, conventions, performance, tests, security)

**Session management (5):**
- `title_generator.toml` -- concise session title (3-7 words, JSON output)
- `title_branch_generator.toml` -- session title + git branch name (JSON output)
- `conversation_summarizer.toml` -- 9-section detailed conversation summary
- `recent_summarizer.toml` -- recent-only summarization (post-compaction)
- `session_search.toml` -- find sessions by query (metadata matching, ranked results)

**Content processing (3):**
- `webfetch_summarizer.toml` -- summarize fetched web content (trusted vs untrusted rules)
- `bash_description.toml` -- generate concise command descriptions in active voice
- `bash_prefix_detection.toml` -- extract command prefix, detect injection

**Infrastructure (4):**
- `hook_evaluator.toml` -- evaluate hook conditions (JSON ok/not-ok output)
- `agent_hook.toml` -- verify stop conditions against conversation transcript
- `batch_orchestrator.toml` -- parallel work orchestration (5-30 independent units)
- `codex_guide.toml` -- help users with Codex CLI, Agent SDK, and API usage

**Creation & setup (3):**
- `agent_architect.toml` -- design custom agent configurations (JSON identifier + whenToUse + systemPrompt)
- `agentsmd_creation.toml` -- analyze codebase and create AGENTS.md
- `suggestion_generator.toml` -- predict user's natural next input (2-12 words)

**Not ported from CC (5 -- platform-specific):** status-line-setup (CC PS1 conversion), /schedule (CC remote triggers), dream-memory-consolidation (memory disabled), memory-file-selector (memory disabled), session-memory-updates (memory disabled)

### Registration

All 29 TOML configs are registered in `codex-rs/core/src/agent/role.rs`:

- `built_in::configs()` — 31 entries in a `BTreeMap<String, AgentRoleConfig>` (29 TOML-backed agents + `default` and `worker` which are description-only with no config_file). Each entry provides a `description` (shown in the spawn tool spec) and a `config_file` path. User-spawnable agents have rich multi-line descriptions; system-internal agents use concise `"System agent: ..."` descriptions.
- `built_in::config_file_contents()` — 29 `include_str!()` declarations that embed the TOML files into the binary at compile time, with a `match` statement resolving each filename to its content.

`resolve_role_config()` checks user-defined roles first (from `config.agent_roles`), then falls back to `built_in::configs()`. This means user-defined roles with the same name override built-in roles.

## MCP Server Instructions

Codex handles MCP server instructions natively through its plugin injection system. `build_plugin_injections()` in `codex-rs/core/src/plugins/injection.rs` assembles plugin capability summaries and tool info into `ResponseItem` injections that are included in the conversation context. No custom work was needed here -- it works out of the box.

## Native Feature Status

| Feature | Flag | Stage | default_enabled | Status |
|---------|------|-------|-----------------|--------|
| Hooks | `Feature::CodexHooks` | Stable | `true` | Working |
| Git commit guidance | `Feature::CodexGitCommit` | Stable | `true` | Working |
| Child agents AGENTS.md | `Feature::ChildAgentsMd` | Stable | `true` | Working |
| Built-in agent roles (29) | `role.rs` (`configs`, `config_file_contents`) | Stable | — | Working |
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
| Prompt sections (23) | `codex-rs/protocol/src/prompts/sections/*.md` |
| System reminders (37) | `codex-rs/protocol/src/prompts/reminders/*.md` |
| Agent builtins (29) | `codex-rs/core/src/agent/builtins/*.toml` |
| Agent role registry (31) | `codex-rs/core/src/agent/role.rs` (`configs()`, `config_file_contents()`) |
| Section assembler | `codex-rs/protocol/src/models.rs` (`assemble_base_instructions`, `pub mod reminders`) |
| Feature flags | `codex-rs/features/src/lib.rs` |
| Session init wiring | `codex-rs/core/src/codex.rs:575` |
| Skills (8) | `codex-rs/skills/src/assets/samples/*/SKILL.md` |
| Spinner verbs | `codex-rs/tui/src/spinner_verbs.rs` |
| Slash commands | `codex-rs/tui/src/slash_command.rs` |
| Tooltips | `codex-rs/tui/tooltips.txt` |
| Composer placeholders | `codex-rs/tui/src/chatwidget.rs` (PLACEHOLDERS const) |
| Dynamic placeholder | `codex-rs/tui/src/chatwidget.rs` (`extract_next_step_suggestion`) |
| Placeholder wiring | `codex-rs/tui/src/bottom_pane/mod.rs` (`set_composer_placeholder`) |
| Diamond indicators | `codex-rs/tui/src/exec_cell/render.rs` |
| Compact prompt | `codex-rs/core/templates/compact/prompt.md` |
| Init prompt (7-phase) | `codex-rs/tui/prompt_for_init_command.md` |
| Auto-session titles | `codex-rs/core/src/tasks/mod.rs` (`derive_session_title`) |
| Plugin injection (MCP) | `codex-rs/core/src/plugins/injection.rs` (`build_plugin_injections`) |
| Sandbox/approval templates | `codex-rs/protocol/src/prompts/permissions/` |
| CI workflow | `.github/workflows/build-claude-soul.yml` |
