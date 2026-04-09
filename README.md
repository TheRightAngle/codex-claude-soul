# Codex Claude Soul Edition

A fork of [OpenAI's Codex CLI](https://github.com/openai/codex) with Claude Code's personality, prompt architecture, and TUI polish transplanted into it. The runtime, sandbox, and tool execution are untouched stock Codex. Everything that changed is the soul — how it thinks, talks, and presents itself.

## Why This Exists

Codex CLI is an excellent open-source agentic coding tool with a solid Rust runtime, sandbox, and tool system. Claude Code has an excellent personality, prompt system, and UX. This project puts Claude Code's soul into Codex's body.

## What Changed

### Prompt System (the big one)

Claude Code assembles its system prompt from ~20 independent function-per-section modules. This fork replicates that architecture with the full ant-only (Anthropic-internal) tier of behavioral instructions — the richest version available.

**23 section files** in `codex-rs/protocol/src/prompts/sections/`:

| Section | Role | Assembly |
|---------|------|----------|
| `identity.md` | Who the agent is, capabilities | Always-on |
| `system.md` | Tool execution, hooks, compression (CommonMark spec) | Always-on |
| `tone.md` | Personality, AGENTS.md spec (ant-only: no "short and concise") | Always-on |
| `doing_tasks.md` | Task execution methodology (ant-only: comment discipline, faithful reporting, verification) | Always-on |
| `actions.md` | Executing actions with care | Always-on |
| `tools.md` | Tool usage guidance | Always-on |
| `output.md` | "Communicating with the user" (ant-only prose version, not public "Output efficiency") | Always-on |
| `how_you_work.md` | Workflow, planning, preambles, formatting guidelines | Always-on |
| `verification.md` | Verify-before-completing | Feature-gated (default on) |
| `suggestions.md` | Next-step suggestions | Feature-gated (default on) |
| `skills.md` | Skill discovery | Feature-gated (default on) |
| `advisor.md` | Strategic review | Feature-gated (default on) |
| `worktree.md` | Git worktree guidance | Feature-gated (default on) |
| `stuck.md` | Break debugging loops | Feature-gated (default on) |
| `git_protocol.md` | Git conventions, commit/PR protocol | Always-on (tail) |
| `auto_mode.md` | 6 rules for autonomous execution | Contextual (when approval_policy=Never) |
| `plan_mode.md` | 5-phase enhanced plan mode workflow | Contextual (when plan mode active) |
| `insights.md` | Educational insight blocks | User-togglable (/experimental) |
| `compaction.md` | Context handoff format | Invoked-only (/compact) |
| `simplify.md` | Code review dimensions | Invoked-only (/simplify) |
| `session_titles.md` | Title generation | Invoked-only (system) |
| `memory.md` | Persistent memory | Reference only (feature DISABLED) |
| `dream.md` | Memory consolidation | Reference only (feature DISABLED) |

**37 system reminder templates** in `codex-rs/protocol/src/prompts/reminders/` — contextual developer messages injected at runtime, adapted from Claude Code's full set. Covers: plan mode lifecycle (7 variants including 5-phase, iterative, subagent, ultraplan), file state warnings, hook lifecycle, task tracking nudges, token/budget alerts, team coordination, IDE integration, security, and more.

**29 built-in agent roles** in `codex-rs/core/src/agent/builtins/` — TOML configs adapted from Claude Code's 34 agent prompt definitions. Includes: explorer, planner, general-purpose, worker-fork, verifier (adversarial with type-specific strategies), security monitor (24 BLOCK rules), security review, auto-mode-reviewer, quick-commit, quick-PR, PR-comments, review-PR, batch-orchestrator, session-search, title/branch generators, conversation/recent summarizers, webfetch-summarizer, bash-description/prefix-detection, hook-evaluator, agent-hook, codex-guide, agent-architect, AGENTS.md-creation, and suggestion-generator. All 29 are registered in `codex-rs/core/src/agent/role.rs` via `include_str!()` (compile-time embedding) and a `BTreeMap` role registry (31 total entries: 29 TOML-backed + `default` and `worker` description-only roles).

**Assembler**: `assemble_base_instructions()` in `codex-rs/protocol/src/models.rs` concatenates sections based on `PromptFeatures`. The `reminders` module exposes all 37 templates as compile-time constants. Feature flags in `codex-rs/features/src/lib.rs` control togglable sections. Runtime state flags (`auto_mode`, `plan_mode`) are set from approval policy and collaboration mode.

**Wiring**: Called at session init in `codex-rs/core/src/codex.rs:575`. Auto mode is activated when `AskForApproval::Never`. Agent roles are resolved via `resolve_role_config()` in `role.rs`, which checks user-defined roles first, then falls back to the built-in registry.

### TUI Polish

| Change | File | Detail |
|--------|------|--------|
| Spinner verbs | `tui/src/spinner_verbs.rs` | 171 whimsical verbs (including "Codexing" Easter egg) |
| Diamond indicators | `tui/src/exec_cell/render.rs` | `◇` running, `◆` completed (replacing bullets) |
| Blockquote prefix | `tui/src/markdown_render.rs` | `▎` replacing `>` |
| Effort symbols | `tui/src/status/` | `○◐●◉` in status line |
| Heavy chevron | `tui/src/chatwidget.rs` | `❯` prompt replacing `›` |
| Slash commands | `tui/src/slash_command.rs` | All descriptions rewritten |
| Tooltips | `tui/tooltips.txt` | Rewritten for Codex identity |
| Static placeholders | `tui/src/chatwidget.rs` | 8-entry PLACEHOLDERS array (random on session start) |
| Dynamic placeholder | `tui/src/chatwidget.rs` | Contextual suggestion after each turn |
| Approval banner | `tui/src/bottom_pane/pending_thread_approvals.rs` | "Codex needs your approval" |
| Auto-session titles | `core/src/tasks/mod.rs` | `derive_session_title()` on first turn |

### Dynamic Composer Placeholder (Prompt Suggestions)

Claude Code shows "prompt suggestion" chips after each agent turn. This fork achieves the same effect through the existing dimmed placeholder text in the composer, which updates dynamically after each turn.

After the agent responds, `extract_next_step_suggestion()` in `chatwidget.rs` scans the last 5 lines for "Want me to X?" / "Should I X?" patterns and backtick-wrapped commands. Matches are passed to `set_composer_placeholder()` on `BottomPane`, guiding the user toward the natural next action.

### Built-in Skills

`codex-rs/skills/src/assets/samples/`:
- `simplify/SKILL.md` — three-dimension code review (reuse, quality, efficiency)
- `stuck/SKILL.md` — 5-step protocol to break out of debugging loops
- `debugging/SKILL.md` — 5-step debug flow: review issue, scan logs, check system state, explain findings, suggest fixes

### Improved Prompts

- **Compact**: `codex-rs/core/templates/compact/prompt.md` — structured 5-section handoff (Task Overview, Current State, Important Discoveries, Next Steps, Context to Preserve) with no-tool-call guardrail
- **Init**: `codex-rs/tui/prompt_for_init_command.md` — 7-phase onboarding flow: explore codebase, fill gaps, write AGENTS.md, suggest skills, suggest hooks, check environment, summary

### MCP Server Instructions

Codex handles MCP server instructions natively through its plugin injection system. `build_plugin_injections()` in `codex-rs/core/src/plugins/injection.rs` assembles `PluginCapabilitySummary` data and MCP tool info into `ResponseItem` injections. This works out of the box.

## Coverage vs Claude Code v2.1.90

| Category | Claude Code | Codex Soul Edition | Coverage |
|----------|-------------|-------------------|----------|
| Prompt sections | 16 assembled + 5 on-demand | 18 assembled + 5 on-demand | **100%** + auto_mode, plan_mode |
| System reminders | 37 | 37 | **100%** |
| Agent sub-prompts | 34 | 29 | **85%** (5 CC-specific omitted) |
| Skills | 7 | 8 (3 shared + 5 Codex-native) | **Adapted** |
| Ant-only enhancements | Internal build only | All included | **Enhanced** |

Ant-only features included that Claude Code's public build omits: comment writing discipline, collaborator mindset, faithful outcome reporting, verification before completion, "Communicating with the user" output prose.

## Feature Status

### Working

| Feature | Flag | Stage | default_enabled |
|---------|------|-------|-----------------|
| Hooks lifecycle | `Feature::CodexHooks` | Stable | `true` |
| Commit attribution | `Feature::CodexGitCommit` | Stable | `true` |
| Subagent AGENTS.md | `Feature::ChildAgentsMd` | Stable | `true` |
| Prompt: Verification | `Feature::PromptVerification` | Stable | `true` |
| Prompt: Suggestions | `Feature::PromptSuggestions` | Stable | `true` |
| Prompt: Skills | `Feature::PromptSkills` | Stable | `true` |
| Prompt: Advisor | `Feature::PromptAdvisor` | Stable | `true` |
| Prompt: Worktree | `Feature::PromptWorktree` | Stable | `true` |
| Prompt: Insights | `Feature::PromptInsights` | Experimental | `false` |
| Auto mode rules | `PromptFeatures.auto_mode` | Runtime | (when approval=Never) |
| Plan mode (5-phase) | `PromptFeatures.plan_mode` | Runtime | (when plan mode active) |
| Dynamic placeholder | (custom code in chatwidget.rs) | — | — |
| Static placeholders | (PLACEHOLDERS array) | — | — |
| Spinner verbs | (custom code in spinner_verbs.rs) | — | — |
| Diamond indicators | (custom code in exec_cell/render.rs) | — | — |
| Auto-session titles | (derive_session_title in tasks/mod.rs) | — | — |
| MCP plugin injection | (build_plugin_injections, native) | — | — |
| Built-in agent roles (29) | (role.rs: configs + config_file_contents) | — | — |

### Disabled

| Feature | Flag | Why |
|---------|------|-----|
| MemoryTool | `Feature::MemoryTool` | Spawns background agents with 1-hour lease that block the session on first run. Section files `memory.md` and `dream.md` kept as reference only. |
| GhostCommit (undo) | `Feature::GhostCommit` | Creates silent background commits for undo. Disabled for safety. |

## Build

```sh
cd codex-rs

# Build
cargo build --release -p codex-cli

# Test (scope to the crate you changed)
cargo test -p codex-tui --release

# Format
just fmt

# Lint
just fix -p codex-tui
```

### Linux

Requires `libcap-dev` for the bubblewrap sandbox:

```sh
sudo apt-get install -y pkg-config libcap-dev
```

### Snapshot Tests

UI changes require `insta` snapshot coverage. CI runs with `INSTA_UPDATE=always`.

```sh
INSTA_UPDATE=always cargo test -p codex-tui
```

## Install

Build the release binary and put it on your PATH:

```sh
cd codex-rs
cargo build --release -p codex-cli
cp target/release/codex ~/.local/bin/
```

Or grab a pre-built artifact from the [GitHub Actions workflow](.github/workflows/build-claude-soul.yml) (Linux x86_64, macOS arm64, Windows x86_64).

Run `codex` and select **Sign in with ChatGPT**. We recommend signing into your ChatGPT account to use Codex as part of your Plus, Pro, Business, Edu, or Enterprise plan. [Learn more about what's included in your ChatGPT plan](https://help.openai.com/en/articles/11369540-codex-in-chatgpt).

## CI/CD

`.github/workflows/build-claude-soul.yml` is the only active custom workflow:
- Triggers on push to `claude-code-personality` and `main`, PRs to `main`, and manual dispatch
- Builds release binaries for three targets: `x86_64-unknown-linux-gnu`, `aarch64-apple-darwin`, `x86_64-pc-windows-msvc`
- Runs `codex-tui` tests with `INSTA_UPDATE=always`
- Uploads binary artifacts with 30-day retention

All upstream OpenAI workflows remain in `.github/workflows/` but do not fire for this fork's work.

## File Map

Every file we created or meaningfully modified:

```
codex-rs/protocol/src/prompts/
  sections/                     # 23 prompt section files
    identity.md                 # Agent identity and capabilities
    system.md                   # Tool execution, hooks, compression (CommonMark)
    tone.md                     # Personality, AGENTS.md spec (ant-only)
    doing_tasks.md              # Task execution methodology (ant-only enhancements)
    actions.md                  # Executing actions with care
    tools.md                    # Tool usage guidance
    output.md                   # "Communicating with the user" (ant-only prose)
    how_you_work.md             # Workflow, planning, preambles, formatting
    verification.md             # Verify before claiming completion
    suggestions.md              # Next-step suggestions
    skills.md                   # Skill discovery
    advisor.md                  # Strategic review
    worktree.md                 # Git worktree guidance
    stuck.md                    # Break debugging loops
    git_protocol.md             # Git conventions, commit/PR protocol
    auto_mode.md                # 6 rules for autonomous execution (NEW)
    plan_mode.md                # 5-phase enhanced plan mode (NEW)
    insights.md                 # Educational insight blocks (user-togglable)
    compaction.md               # Context compaction handoff
    simplify.md                 # Code review dimensions
    session_titles.md           # Title generation
    memory.md                   # Persistent memory reference (DISABLED)
    dream.md                    # Memory consolidation reference (DISABLED)
  reminders/                    # 37 system reminder templates (NEW)
    plan_mode_5_phase.md        # 5-phase plan mode instructions
    plan_mode_iterative.md      # Iterative plan mode with user interview
    plan_mode_subagent.md       # Simplified plan mode for sub-agents
    plan_mode_re_entry.md       # Re-entering plan mode
    exited_plan_mode.md         # Exited plan mode
    plan_file_reference.md      # Existing plan file reference
    ultraplan_mode.md           # Multi-agent thorough planning
    task_tools.md               # Nudge to use update_plan
    todowrite_reminder.md       # Nudge to track progress
    verify_plan.md              # Verification before completion
    verify_plan_reminder.md     # Post-implementation verification
    session_continuation.md     # Resumed session context
    compact_reference.md        # Post-compaction context
    output_style.md             # Active output style
    invoked_skills.md           # Active skill guidelines
    skill_invoked.md            # Skill loaded notification
    token_usage.md              # Context window stats
    budget_warning.md           # Budget limit approaching
    usd_budget.md               # USD budget stats
    file_modified_externally.md # External file change
    file_empty.md               # Empty file warning
    file_truncated.md           # Truncated file warning
    file_shorter_than_offset.md # Offset exceeds file length
    hook_blocking.md            # Hook blocked action
    hook_success.md             # Hook succeeded
    hook_stopped_continuation.md # Hook stopped continuation
    tool_denied.md              # User rejected tool call
    agent_mention.md            # User wants to invoke agent
    btw_side_question.md        # Side question (no tools)
    mcp_server_status.md        # MCP status update
    mcp_no_content.md           # MCP empty resource
    diagnostics_detected.md     # New diagnostics found
    ide_file_opened.md          # File opened in IDE
    ide_lines_selected.md       # Lines selected in IDE
    malware_analysis_warning.md # Potential malicious code
    team_coordination.md        # Multi-agent team context
    team_shutdown.md            # Team shutdown required
  permissions/                  # Sandbox/approval templates (upstream)

codex-rs/protocol/src/
  models.rs                     # assemble_base_instructions(), PromptFeatures, pub mod reminders

codex-rs/features/src/
  lib.rs                        # Feature flags

codex-rs/core/
  src/codex.rs                  # Session init wiring, auto_mode detection
  src/tasks/mod.rs              # derive_session_title()
  src/plugins/injection.rs      # MCP plugin injection
  src/agent/role.rs             # Agent role registry: configs() + config_file_contents()
  src/agent/builtins/           # 29 agent role TOML configs (NEW)
    explorer.toml               # Codebase exploration (read-only)
    planner.toml                # Implementation planning (read-only)
    general_purpose.toml        # Multi-step tasks (NEW)
    researcher.toml             # Targeted research (NEW)
    worker_fork.toml            # Isolated directive execution (NEW)
    awaiter.toml                # Background task polling
    verifier.toml               # Adversarial verification (ENHANCED)
    security_monitor.toml       # Autonomous action monitor (ENHANCED)
    security_review.toml        # Code security review (NEW)
    auto_mode_reviewer.toml     # Auto mode rule review (NEW)
    quick_commit.toml           # Git commit creation (NEW)
    quick_pr.toml               # Commit + push + PR (NEW)
    pr_comments.toml            # PR comment fetching (NEW)
    review_pr.toml              # PR code review (NEW)
    batch_orchestrator.toml     # Parallel work orchestration (NEW)
    title_generator.toml        # Session title generation (NEW)
    title_branch_generator.toml # Title + branch name (NEW)
    conversation_summarizer.toml # 9-section summary (NEW)
    recent_summarizer.toml      # Recent-only summary (NEW)
    session_search.toml         # Session search by query (NEW)
    webfetch_summarizer.toml    # Web content summarizer (NEW)
    bash_description.toml       # Command description writer (NEW)
    bash_prefix_detection.toml  # Command prefix extractor (NEW)
    hook_evaluator.toml         # Hook condition evaluator (NEW)
    agent_hook.toml             # Stop condition verifier (NEW)
    codex_guide.toml            # Codex CLI/SDK/API guide (NEW)
    agent_architect.toml        # Custom agent design (NEW)
    agentsmd_creation.toml      # AGENTS.md generator (NEW)
    suggestion_generator.toml   # Next-input predictor (NEW)
  templates/compact/prompt.md   # Improved compaction prompt

codex-rs/tui/
  src/spinner_verbs.rs          # 171 whimsical spinner verbs
  src/slash_command.rs          # Rewritten command descriptions
  src/chatwidget.rs             # PLACEHOLDERS, heavy chevron, dynamic placeholder
  src/exec_cell/render.rs       # Diamond status indicators
  src/markdown_render.rs        # Blockquote prefix
  src/markdown_stream.rs        # Blockquote rendering
  src/bottom_pane/mod.rs        # set_composer_placeholder() wiring
  src/bottom_pane/chat_composer.rs  # set_placeholder_text()
  src/bottom_pane/pending_thread_approvals.rs  # Approval banner text
  src/updates.rs                # Brew cask JSON trailing-comma fix
  tooltips.txt                  # Rewritten tooltips
  prompt_for_init_command.md    # 7-phase init onboarding (ENHANCED)

codex-rs/skills/src/assets/samples/
  simplify/SKILL.md             # Three-dimension code review
  stuck/SKILL.md                # Debugging loop escape protocol
  debugging/SKILL.md            # 5-step debug flow (NEW)

.github/workflows/
  build-claude-soul.yml         # CI: Linux, macOS, Windows builds
```

## License

This project is based on [Codex CLI](https://github.com/openai/codex), licensed under [Apache-2.0](LICENSE).
