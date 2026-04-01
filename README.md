# Codex Claude Soul Edition

A fork of [OpenAI's Codex CLI](https://github.com/openai/codex) with Claude Code's personality, prompt architecture, and TUI polish transplanted into it. The runtime, sandbox, and tool execution are untouched stock Codex. Everything that changed is the soul — how it thinks, talks, and presents itself.

## Why This Exists

Codex CLI is an excellent open-source agentic coding tool with a solid Rust runtime, sandbox, and tool system. Claude Code has an excellent personality, prompt system, and UX. This project puts Claude Code's soul into Codex's body.

## What Changed

### Prompt System (the big one)

Claude Code assembles its system prompt from ~20 independent function-per-section modules. This fork replicates that architecture:

**21 section files** in `codex-rs/protocol/src/prompts/sections/`:

| Section | Role | Assembly |
|---------|------|----------|
| `identity.md` | Who the agent is, capabilities | Always-on |
| `system.md` | Tool execution, hooks, compression | Always-on |
| `tone.md` | Personality, AGENTS.md spec | Always-on |
| `doing_tasks.md` | Task execution methodology | Always-on |
| `actions.md` | Action patterns | Always-on |
| `tools.md` | Tool usage guidance | Always-on |
| `output.md` | Response formatting | Always-on |
| `how_you_work.md` | Workflow, planning, preambles | Always-on |
| `verification.md` | Verify-before-completing | Feature-gated (default on) |
| `suggestions.md` | Next-step suggestions | Feature-gated (default on) |
| `skills.md` | Skill discovery | Feature-gated (default on) |
| `advisor.md` | Strategic review | Feature-gated (default on) |
| `worktree.md` | Git worktree guidance | Feature-gated (default on) |
| `stuck.md` | Break debugging loops | Feature-gated (default on) |
| `git_protocol.md` | Git conventions | Always-on (tail) |
| `insights.md` | Educational insight blocks | User-togglable (/experimental) |
| `compaction.md` | Context handoff format | Invoked-only (/compact) |
| `simplify.md` | Code review dimensions | Invoked-only (/simplify) |
| `session_titles.md` | Title generation | Invoked-only (system) |
| `memory.md` | Persistent memory | Reference (native system) |
| `dream.md` | Memory consolidation | Reference (native system) |

**Assembler**: `assemble_base_instructions()` in `codex-rs/protocol/src/models.rs` concatenates sections based on `PromptFeatures`. Feature flags in `codex-rs/features/src/lib.rs` control which togglable sections are included.

**Wiring**: Called at session init in `codex-rs/core/src/codex.rs:575`.

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
| Placeholders | `tui/src/chatwidget.rs` | 8-entry PLACEHOLDERS array |
| Approval banner | `tui/src/bottom_pane/pending_thread_approvals.rs` | "Codex needs your approval" |
| Auto-session titles | `core/src/tasks/mod.rs` | `derive_session_title()` on first turn |

### Built-in Skills

- `codex-rs/skills/src/assets/samples/simplify/SKILL.md` — three-dimension code review (reuse, quality, efficiency)
- `codex-rs/skills/src/assets/samples/stuck/SKILL.md` — 5-step protocol to break out of debugging loops

### Improved Prompts

- **Compact**: `codex-rs/core/templates/compact/prompt.md` — structured 5-section handoff (Task Overview, Current State, Important Discoveries, Next Steps, Context to Preserve) with no-tool-call guardrail
- **Init**: `codex-rs/tui/prompt_for_init_command.md` — opinionated senior-dev tone, 150-300 words target
- **Double-print prevention**: `how_you_work.md` section clarifies preamble content should not be repeated in final answer

### Native Features Enabled

- **`Feature::CodexHooks`** — hooks.json lifecycle hooks (Stable, default on)
- **`Feature::CodexGitCommit`** — commit attribution guidance (Stable, default on)
- **`Feature::ChildAgentsMd`** — subagent documentation loading (Stable, default on)

### Native Features Disabled

- **`Feature::MemoryTool`** — `Stage::UnderDevelopment`, `default_enabled: false`. Spawns background agents with a 1-hour lease that block the session on first run when no history exists. The section files (`memory.md`, `dream.md`) are kept as reference.
- **`Feature::GhostCommit`** — `default_enabled: false`. Background snapshot/undo tasks. Disabled for safety — making silent commits can surprise users.

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

UI changes require `insta` snapshot coverage. CI runs with `INSTA_UPDATE=always` to auto-accept snapshots. A trailing-comma bug in upstream's brew cask JSON test was fixed as part of this work.

```sh
INSTA_UPDATE=always cargo test -p codex-tui
```

## Install

Build the release binary and put it on your PATH:

```sh
cd codex-rs
cargo build --release -p codex-cli
# Binary is at: target/release/codex
cp target/release/codex ~/.local/bin/
```

Or grab a pre-built artifact from the [GitHub Actions workflow](.github/workflows/build-claude-soul.yml) (Linux x86_64, macOS arm64, Windows x86_64).

## CI/CD

`.github/workflows/build-claude-soul.yml`:
- Triggers on push to `claude-code-personality` and `main`, PRs to `main`, and manual dispatch
- Builds release binaries for three targets: `x86_64-unknown-linux-gnu`, `aarch64-apple-darwin`, `x86_64-pc-windows-msvc`
- Runs `codex-tui` tests with `INSTA_UPDATE=always`
- Uploads binary artifacts with 30-day retention
- Caches cargo registry and build artifacts

## File Map

Every file we created or meaningfully modified:

```
codex-rs/protocol/src/prompts/sections/
  identity.md            # Agent identity and capabilities
  system.md              # Tool execution, hooks, compression
  tone.md                # Personality, AGENTS.md spec
  doing_tasks.md         # Task execution methodology
  actions.md             # Action patterns
  tools.md               # Tool usage guidance
  output.md              # Response formatting
  how_you_work.md        # Workflow, planning, preambles, double-print prevention
  verification.md        # Verify before claiming completion
  suggestions.md         # Next-step suggestions
  skills.md              # Skill discovery
  advisor.md             # Strategic review
  worktree.md            # Git worktree guidance
  stuck.md               # Break debugging loops
  git_protocol.md        # Git conventions
  insights.md            # Educational insight blocks (user-togglable)
  compaction.md          # Context compaction handoff
  simplify.md            # Code review dimensions
  session_titles.md      # Title generation
  memory.md              # Persistent memory reference
  dream.md               # Memory consolidation reference

codex-rs/protocol/src/
  models.rs              # assemble_base_instructions(), PromptFeatures struct

codex-rs/features/src/
  lib.rs                 # Feature flags: Prompt{Verification,Suggestions,Skills,Insights,Advisor,Worktree}

codex-rs/core/
  src/codex.rs           # Session init wiring (line ~575)
  src/tasks/mod.rs       # derive_session_title() auto-naming
  templates/compact/prompt.md  # Improved compaction prompt

codex-rs/tui/
  src/spinner_verbs.rs   # 171 whimsical spinner verbs
  src/slash_command.rs   # Rewritten command descriptions
  src/chatwidget.rs      # PLACEHOLDERS, heavy chevron prompt
  src/exec_cell/render.rs # Diamond status indicators
  src/markdown_render.rs # Blockquote prefix
  src/markdown_stream.rs # Blockquote rendering
  src/bottom_pane/pending_thread_approvals.rs  # Approval banner text
  tooltips.txt           # Rewritten tooltips
  prompt_for_init_command.md  # Init prompt (senior-dev tone)

codex-rs/skills/src/assets/samples/
  simplify/SKILL.md      # Three-dimension code review
  stuck/SKILL.md         # Debugging loop escape protocol

.github/workflows/
  build-claude-soul.yml  # CI: Linux, macOS, Windows builds
```

## License

This project is based on [Codex CLI](https://github.com/openai/codex), licensed under [Apache-2.0](LICENSE).
