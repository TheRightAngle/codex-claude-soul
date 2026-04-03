# System Reminder Injection Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Wire all 37 reminder templates into Codex's runtime so the model receives contextual guidance for file state, hooks, plan mode, sessions, tokens, etc.

**Architecture:** Create a `reminder_injection` module in `core/src/` with helpers for injecting reminders as developer messages or tool result annotations. Then wire each reminder into its trigger point across the codebase using Codex's existing `record_conversation_items` + `DeveloperInstructions` pattern.

**Tech Stack:** Rust, codex-protocol reminders module, codex-core session/tool infrastructure

---

### Task 1: Create reminder injection helpers

**Files:**
- Create: `codex-rs/core/src/reminder_injection.rs`
- Modify: `codex-rs/core/src/lib.rs` (add `mod reminder_injection;`)

- [ ] **Step 1: Create the helper module**

```rust
// codex-rs/core/src/reminder_injection.rs

use crate::context_manager::updates::build_developer_update_item;
use crate::Session;
use codex_protocol::models::ResponseItem;

/// Wrap reminder text in system-reminder tags for model consumption.
pub fn wrap_reminder(reminder: &str) -> String {
    format!("<system-reminder>\n{reminder}\n</system-reminder>")
}

/// Append a system-reminder annotation to a tool result string.
pub fn annotate_tool_result(output: &str, reminder: &str) -> String {
    format!("{output}\n\n{}", wrap_reminder(reminder))
}

/// Build a developer message ResponseItem containing a system reminder.
pub fn build_reminder_item(reminder: &str) -> Option<ResponseItem> {
    build_developer_update_item(vec![wrap_reminder(reminder)])
}

/// Inject a reminder as a developer message into the conversation.
pub async fn inject_reminder(
    session: &Session,
    turn_context: &crate::TurnContext,
    reminder: &str,
) {
    if let Some(item) = build_reminder_item(reminder) {
        session
            .record_conversation_items(turn_context, &[item])
            .await;
    }
}
```

- [ ] **Step 2: Register the module**

In `codex-rs/core/src/lib.rs`, add:
```rust
pub(crate) mod reminder_injection;
```

Find the existing `mod` declarations and add it alphabetically near other internal modules.

- [ ] **Step 3: Verify it compiles**

Run: `cargo check -p codex-core`
Expected: Compiles with no errors (warnings about unused functions are OK at this stage)

- [ ] **Step 4: Commit**

```bash
git add codex-rs/core/src/reminder_injection.rs codex-rs/core/src/lib.rs
git commit -m "feat: add reminder_injection helpers for system reminder wiring"
```

---

### Task 2: Wire file state reminders into tool results

**Files:**
- Modify: `codex-rs/core/src/tools/handlers/read_file.rs` (or the shell handler that processes file reads)
- Modify: `codex-rs/core/src/tools/context.rs` (if read_file result is built there)

Note: Codex processes file reads through the `shell` tool. The read_file handler in `tools/src/read_file.rs` returns a `ReadFileResult` that flows through `ExecCommandToolOutput` or `FunctionToolOutput`. Find where the text output is constructed and annotate it.

- [ ] **Step 1: Find exact read_file output construction**

Search for where file content is returned as tool output. Check `tools/src/read_file.rs` for the result type and `core/src/tools/handlers/` for how it becomes a `FunctionToolOutput`.

- [ ] **Step 2: Add file_empty annotation**

Where the read_file result is constructed, check if content is empty:
```rust
use crate::reminder_injection::annotate_tool_result;
use codex_protocol::models::reminders;

// After getting file content:
let output_text = if content.is_empty() {
    annotate_tool_result(&content, reminders::FILE_EMPTY)
} else {
    content
};
```

- [ ] **Step 3: Add file_truncated annotation**

Where file content is truncated due to size limits:
```rust
let output_text = if was_truncated {
    annotate_tool_result(&truncated_content, reminders::FILE_TRUNCATED)
} else {
    content
};
```

- [ ] **Step 4: Add file_shorter_than_offset annotation**

Where offset exceeds file length:
```rust
let output_text = annotate_tool_result(
    &format!("Warning: file has {} lines but offset {} was requested", file_lines, offset),
    reminders::FILE_SHORTER_THAN_OFFSET,
);
```

- [ ] **Step 5: Verify it compiles**

Run: `cargo check -p codex-core`

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "feat: wire file state reminders into read_file tool results"
```

---

### Task 3: Wire tool_denied and mcp_no_content reminders

**Files:**
- Modify: `codex-rs/core/src/tools/parallel.rs:136` (failure_response function)
- Modify: `codex-rs/core/src/mcp_tool_call.rs:236` (MCP rejection message)

- [ ] **Step 1: Annotate tool denial in parallel.rs**

In `failure_response()` at line 136, annotate the denial message:
```rust
fn failure_response(call: ToolCall, err: FunctionCallError) -> ResponseInputItem {
    let message = err.to_string();
    let message = if message.contains("denied") || message.contains("rejected") {
        crate::reminder_injection::annotate_tool_result(&message, codex_protocol::models::reminders::TOOL_DENIED)
    } else {
        message
    };
    // ... rest of match
```

- [ ] **Step 2: Annotate MCP rejection in mcp_tool_call.rs**

At line 236, annotate the MCP rejection:
```rust
let message = crate::reminder_injection::annotate_tool_result(
    "user rejected MCP tool call",
    codex_protocol::models::reminders::TOOL_DENIED,
);
```

- [ ] **Step 3: Annotate MCP empty results**

In the MCP tool result handler, where results are checked for content, add:
```rust
if result_content.is_empty() {
    let message = crate::reminder_injection::annotate_tool_result(
        "",
        codex_protocol::models::reminders::MCP_NO_CONTENT,
    );
}
```

- [ ] **Step 4: Verify and commit**

```bash
cargo check -p codex-core
git add -A
git commit -m "feat: wire tool_denied and mcp_no_content reminders"
```

---

### Task 4: Wire turn-start context reminders

**Files:**
- Modify: `codex-rs/core/src/codex.rs` → `build_initial_context()` (lines 3574-3746)

- [ ] **Step 1: Add session_continuation reminder**

In `build_initial_context()`, after the existing developer_sections setup, check for resumed/forked session:
```rust
// After line 3596 (where session state is read)
let is_resumed = matches!(&conversation_history_kind, InitialHistoryKind::Resumed | InitialHistoryKind::Forked);

// Later, before building the developer message:
if is_resumed {
    developer_sections.push(
        crate::reminder_injection::wrap_reminder(codex_protocol::models::reminders::SESSION_CONTINUATION)
    );
}
```

Note: You'll need to pass the history kind into `build_initial_context` or detect it from existing state. Check if `session_source` or another field indicates resume.

- [ ] **Step 2: Add compact_reference reminder**

Check if compaction has occurred (the reference_context_item being None after a compaction):
```rust
if self.was_compacted().await {
    developer_sections.push(
        crate::reminder_injection::wrap_reminder(codex_protocol::models::reminders::COMPACT_REFERENCE)
    );
}
```

- [ ] **Step 3: Add task_tools reminder**

Track turns since last update_plan usage. Add a counter to Session state, increment each turn, reset when update_plan is called. If counter > 3:
```rust
if self.turns_since_plan_update().await > 3 {
    developer_sections.push(
        crate::reminder_injection::wrap_reminder(codex_protocol::models::reminders::TASK_TOOLS)
    );
}
```

This requires adding a `turns_since_plan_update: u32` field to the session state and incrementing it in the turn completion path.

- [ ] **Step 4: Add output_style reminder**

If an output style is configured:
```rust
if turn_context.config.output_style.is_some() {
    developer_sections.push(
        crate::reminder_injection::wrap_reminder(codex_protocol::models::reminders::OUTPUT_STYLE)
    );
}
```

- [ ] **Step 5: Add plan_file_reference reminder**

If a plan file exists from plan mode:
```rust
if self.has_active_plan_file().await {
    developer_sections.push(
        crate::reminder_injection::wrap_reminder(codex_protocol::models::reminders::PLAN_FILE_REFERENCE)
    );
}
```

- [ ] **Step 6: Add invoked_skills reminder**

If skills were invoked in prior turns:
```rust
if !turn_context.turn_skills.outcome.is_empty() {
    developer_sections.push(
        crate::reminder_injection::wrap_reminder(codex_protocol::models::reminders::INVOKED_SKILLS)
    );
}
```

- [ ] **Step 7: Verify and commit**

```bash
cargo check -p codex-core
git add -A
git commit -m "feat: wire turn-start context reminders (session, compaction, tasks, style)"
```

---

### Task 5: Wire plan mode lifecycle reminders

**Files:**
- Modify: `codex-rs/core/src/context_manager/updates.rs` → `build_collaboration_mode_update_item()`
- Modify: `codex-rs/core/src/codex.rs` → collaboration mode transition handling

- [ ] **Step 1: Identify collaboration mode transition point**

In `context_manager/updates.rs`, the `build_collaboration_mode_update_item()` function (around line 54-68) detects mode changes. This is where plan mode lifecycle reminders should be injected alongside the existing collaboration mode developer message.

- [ ] **Step 2: Add plan mode entry reminders**

When entering plan mode, detect the plan variant and inject the appropriate reminder:
```rust
use codex_protocol::models::reminders;

// In the collaboration mode update builder:
let plan_reminder = match &new_mode {
    CollaborationMode::Plan { variant: PlanVariant::FivePhase } => Some(reminders::PLAN_MODE_5_PHASE),
    CollaborationMode::Plan { variant: PlanVariant::Iterative } => Some(reminders::PLAN_MODE_ITERATIVE),
    CollaborationMode::Plan { variant: PlanVariant::Subagent } => Some(reminders::PLAN_MODE_SUBAGENT),
    CollaborationMode::Plan { variant: PlanVariant::Ultra } => Some(reminders::ULTRAPLAN_MODE),
    _ => None,
};

if let Some(reminder) = plan_reminder {
    sections.push(crate::reminder_injection::wrap_reminder(reminder));
}
```

- [ ] **Step 3: Add plan mode exit reminder**

When transitioning FROM plan mode to default mode:
```rust
if was_plan_mode && !is_plan_mode {
    sections.push(crate::reminder_injection::wrap_reminder(reminders::EXITED_PLAN_MODE));
}
```

- [ ] **Step 4: Add plan mode re-entry reminder**

When transitioning from non-plan to plan and a plan file already exists:
```rust
if !was_plan_mode && is_plan_mode && plan_file_exists {
    sections.push(crate::reminder_injection::wrap_reminder(reminders::PLAN_MODE_RE_ENTRY));
}
```

- [ ] **Step 5: Verify and commit**

```bash
cargo check -p codex-core
git add -A
git commit -m "feat: wire plan mode lifecycle reminders into collaboration mode transitions"
```

---

### Task 6: Wire hook lifecycle reminders

**Files:**
- Modify: `codex-rs/core/src/hook_runtime.rs` (lines 240-300)

- [ ] **Step 1: Add hook_blocking reminder**

In the hook runtime, when a hook blocks an action (returns error), append the HOOK_BLOCKING reminder to the additional_contexts:
```rust
// Where hook errors are processed:
additional_contexts.push(
    crate::reminder_injection::wrap_reminder(codex_protocol::models::reminders::HOOK_BLOCKING)
);
```

- [ ] **Step 2: Add hook_success reminder**

When a hook succeeds and has output:
```rust
additional_contexts.push(
    crate::reminder_injection::wrap_reminder(codex_protocol::models::reminders::HOOK_SUCCESS)
);
```

- [ ] **Step 3: Add hook_stopped_continuation reminder**

When a hook stops auto-continuation:
```rust
additional_contexts.push(
    crate::reminder_injection::wrap_reminder(codex_protocol::models::reminders::HOOK_STOPPED_CONTINUATION)
);
```

- [ ] **Step 4: Verify and commit**

```bash
cargo check -p codex-core
git add -A
git commit -m "feat: wire hook lifecycle reminders into hook runtime"
```

---

### Task 7: Wire token/budget reminders

**Files:**
- Modify: `codex-rs/core/src/codex.rs` → token count handling

- [ ] **Step 1: Add token_usage high-water reminder**

In `send_token_count_event()` (line 3901), add threshold check:
```rust
async fn send_token_count_event(&self, turn_context: &TurnContext) {
    let (info, rate_limits) = {
        let state = self.state.lock().await;
        state.token_info_and_rate_limits()
    };

    // Check if context usage is high (>80%)
    if let Some(ref info) = info {
        if info.usage_fraction() > 0.8 {
            crate::reminder_injection::inject_reminder(
                self,
                turn_context,
                codex_protocol::models::reminders::TOKEN_USAGE,
            ).await;
        }
    }

    let event = EventMsg::TokenCount(TokenCountEvent { info, rate_limits });
    self.send_event(turn_context, event).await;
}
```

Note: Check if `TokenInfo` has a `usage_fraction()` method or equivalent. If not, compute from `used_tokens / max_tokens`.

- [ ] **Step 2: Add budget_warning reminder**

If budget tracking exists, add near-limit check:
```rust
if let Some(budget) = &turn_context.config.token_budget {
    if info.total_tokens > (budget * 90 / 100) {
        crate::reminder_injection::inject_reminder(
            self, turn_context,
            codex_protocol::models::reminders::BUDGET_WARNING,
        ).await;
    }
}
```

- [ ] **Step 3: Add usd_budget reminder**

Similar pattern for USD budget if it exists in config.

- [ ] **Step 4: Verify and commit**

```bash
cargo check -p codex-core
git add -A
git commit -m "feat: wire token usage and budget warning reminders"
```

---

### Task 8: Wire event-driven reminders (file_modified, MCP, diagnostics, IDE)

**Files:**
- Modify: `codex-rs/core/src/codex.rs` → various event handlers

- [ ] **Step 1: Wire file_modified_externally**

Find the file watcher event handler (if it exists). If Codex has file watching, inject on change detection:
```rust
crate::reminder_injection::inject_reminder(
    self, turn_context,
    codex_protocol::models::reminders::FILE_MODIFIED_EXTERNALLY,
).await;
```

If no file watcher exists, skip this reminder (it's only useful with IDE integration).

- [ ] **Step 2: Wire mcp_server_status**

Find MCP connection/disconnection events and inject:
```rust
crate::reminder_injection::inject_reminder(
    self, turn_context,
    codex_protocol::models::reminders::MCP_SERVER_STATUS,
).await;
```

- [ ] **Step 3: Wire diagnostics_detected**

If LSP/diagnostic integration exists, inject on new diagnostics:
```rust
crate::reminder_injection::inject_reminder(
    self, turn_context,
    codex_protocol::models::reminders::DIAGNOSTICS_DETECTED,
).await;
```

- [ ] **Step 4: Wire IDE reminders**

If IDE event handling exists (file opened, lines selected), inject:
```rust
// On file opened:
crate::reminder_injection::inject_reminder(self, turn_context, reminders::IDE_FILE_OPENED).await;
// On lines selected:
crate::reminder_injection::inject_reminder(self, turn_context, reminders::IDE_LINES_SELECTED).await;
```

- [ ] **Step 5: Verify and commit**

```bash
cargo check -p codex-core
git add -A
git commit -m "feat: wire event-driven reminders (file watch, MCP, diagnostics, IDE)"
```

---

### Task 9: Wire team and agent reminders

**Files:**
- Modify: `codex-rs/core/src/codex.rs` or multi-agent handler files

- [ ] **Step 1: Wire team_coordination**

In multi-agent session initialization, inject for spawned agents:
```rust
crate::reminder_injection::inject_reminder(
    self, turn_context,
    codex_protocol::models::reminders::TEAM_COORDINATION,
).await;
```

- [ ] **Step 2: Wire team_shutdown**

In non-interactive mode pre-response check:
```rust
crate::reminder_injection::inject_reminder(
    self, turn_context,
    codex_protocol::models::reminders::TEAM_SHUTDOWN,
).await;
```

- [ ] **Step 3: Wire agent_mention**

In user input parsing where @agent mentions are detected:
```rust
crate::reminder_injection::inject_reminder(
    self, turn_context,
    codex_protocol::models::reminders::AGENT_MENTION,
).await;
```

- [ ] **Step 4: Wire skill_invoked**

In skill loading/invocation handler:
```rust
crate::reminder_injection::inject_reminder(
    self, turn_context,
    codex_protocol::models::reminders::SKILL_INVOKED,
).await;
```

- [ ] **Step 5: Wire btw_side_question**

This is used as the system prompt for side-question agents. Add it to the side question agent's base instructions rather than injecting as a reminder.

- [ ] **Step 6: Wire verify_plan and verify_plan_reminder**

When all plan items are marked complete, inject:
```rust
crate::reminder_injection::inject_reminder(
    self, turn_context,
    codex_protocol::models::reminders::VERIFY_PLAN,
).await;
```

After the verification turn:
```rust
crate::reminder_injection::inject_reminder(
    self, turn_context,
    codex_protocol::models::reminders::VERIFY_PLAN_REMINDER,
).await;
```

- [ ] **Step 7: Wire malware_analysis_warning**

In file content analysis (if heuristics exist), annotate suspicious file reads:
```rust
let output = crate::reminder_injection::annotate_tool_result(
    &content,
    codex_protocol::models::reminders::MALWARE_ANALYSIS_WARNING,
);
```

- [ ] **Step 8: Verify and commit**

```bash
cargo check -p codex-core
git add -A
git commit -m "feat: wire team, agent, skill, and security reminders"
```

---

### Task 10: Final integration test

- [ ] **Step 1: Run full test suite**

```bash
cargo test -p codex-core
cargo test -p codex-tui
```

- [ ] **Step 2: Build the binary**

```bash
cargo build -p codex-cli
```

- [ ] **Step 3: Manual smoke test**

Install the binary and test:
- Start a session, say "hello" — spinner should stop (no regression)
- Check session JSONL for system-reminder tags in developer messages
- Enter plan mode — should see plan mode reminder in context

- [ ] **Step 4: Final commit and push**

```bash
git add -A
git commit -m "feat: complete system reminder injection — all 37 templates wired"
git push origin-private main
```
