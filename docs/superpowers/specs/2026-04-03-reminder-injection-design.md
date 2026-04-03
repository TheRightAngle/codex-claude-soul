# System Reminder Injection Design

## Problem

37 reminder templates exist in `protocol/src/prompts/reminders/` and are compiled into the binary via `include_str!` constants in `protocol/src/models.rs`, but none are wired into the runtime. This means the model never receives contextual guidance for file state, hooks, plan mode, session resumption, token usage, etc.

## Architecture

### Two injection mechanisms

1. **Tool result annotations** — reminder text appended to a tool's output string before it's returned to the model. Used when the reminder is about a specific tool result.

2. **Developer messages** — `ResponseItem::Message` with `role="developer"` recorded via `record_conversation_items()`. Used for session-level or event-driven guidance.

### New module: `core/src/reminder_injection.rs`

A thin module that provides helper functions for injecting reminders. Each function takes a `&Session` (or `&Arc<Session>`) and a `&TurnContext`, formats the reminder with any dynamic data, and records it.

```rust
pub fn annotate_tool_result(output: &str, reminder: &str) -> String {
    format!("{output}\n\n<system-reminder>\n{reminder}\n</system-reminder>")
}

pub async fn inject_developer_reminder(
    session: &Session,
    turn_context: &TurnContext,
    reminder: &str,
) {
    let item = build_developer_update_item(vec![
        format!("<system-reminder>\n{reminder}\n</system-reminder>")
    ]);
    if let Some(item) = item {
        session.record_conversation_items(turn_context, &[item]).await;
    }
}
```

## Injection Points — Tool Result Annotations

### File state (5 reminders)

**Where:** `tools/src/read_file.rs` (or equivalent tool handler)

| Reminder | Trigger |
|----------|---------|
| `file_empty` | read_file returns empty content |
| `file_truncated` | read_file output exceeds max size and is cut |
| `file_shorter_than_offset` | requested offset > file length |
| `malware_analysis_warning` | file content matches obfuscation/malware heuristics |

**Where:** Tool rejection handler in `core/src/tools/`

| Reminder | Trigger |
|----------|---------|
| `tool_denied` | User denies a tool permission request |

**Where:** MCP tool result handler

| Reminder | Trigger |
|----------|---------|
| `mcp_no_content` | MCP tool returns empty/null content |

## Injection Points — Developer Messages

### Turn-start context (in `build_initial_context`)

**Where:** `core/src/codex.rs` → `build_initial_context()` (lines 3574-3746)

| Reminder | Trigger condition |
|----------|------------------|
| `session_continuation` | `conversation_history` is `Resumed` or `Forked` |
| `compact_reference` | History has been compacted (check compaction flag) |
| `output_style` | Output style config is set and non-default |
| `plan_file_reference` | Plan file exists in workspace and is relevant |
| `invoked_skills` | Skills were invoked in prior turns |
| `task_tools` / `todowrite_reminder` | N turns elapsed since last `update_plan` call (track counter on Session) |

### Plan mode lifecycle

**Where:** `core/src/codex.rs` → collaboration mode transition handlers

| Reminder | Trigger |
|----------|---------|
| `plan_mode_5_phase` | Entering standard plan mode |
| `plan_mode_iterative` | Entering iterative plan mode |
| `plan_mode_subagent` | Entering subagent plan mode |
| `plan_mode_re_entry` | Re-entering plan mode (was in plan, left, came back) |
| `exited_plan_mode` | Leaving plan mode |
| `ultraplan_mode` | Entering ultra plan mode |

### Hook lifecycle

**Where:** `core/src/hook_runtime.rs` → existing hook handling (lines 240-300)

The hook system already injects developer messages via `additional_contexts`. Wire reminders into the existing flow:

| Reminder | Trigger |
|----------|---------|
| `hook_blocking` | Hook returns error/blocks action (append to existing additional_contexts) |
| `hook_success` | Hook completes successfully |
| `hook_stopped_continuation` | Hook stops auto-continuation |

### Event-driven (mid-turn)

**Where:** Various event handlers in `core/src/codex.rs`

| Reminder | Trigger | Location |
|----------|---------|----------|
| `file_modified_externally` | File watcher detects external change | File watch event handler |
| `token_usage` | Context usage exceeds 80% threshold | Token count event handler |
| `budget_warning` | Token budget approaching limit | Budget check in turn completion |
| `usd_budget` | USD budget tracking active | Budget check |
| `mcp_server_status` | MCP server connects/disconnects | MCP connection event handler |
| `diagnostics_detected` | LSP diagnostics received | Diagnostics event handler |
| `ide_file_opened` | IDE sends file-open event | IDE event handler |
| `ide_lines_selected` | IDE sends selection event | IDE event handler |
| `skill_invoked` | Skill is loaded via /command | Skill invocation handler |
| `verify_plan` | Plan implementation completing | Plan completion detection |
| `verify_plan_reminder` | All plan items marked done | Post-plan verification |
| `agent_mention` | User @-mentions an agent | Input parser detecting @agent |
| `btw_side_question` | Side question spawned | Side question agent system prompt |
| `team_coordination` | Multi-agent session active | Team session init |
| `team_shutdown` | Non-interactive mode with active team | Pre-response team check |

## Implementation Order

### Phase 1: Infrastructure (1 file)
- Create `core/src/reminder_injection.rs` with `annotate_tool_result()` and `inject_developer_reminder()` helpers

### Phase 2: Tool result annotations (6 reminders)
- Wire `file_empty`, `file_truncated`, `file_shorter_than_offset` into file read tool
- Wire `tool_denied` into tool rejection handler
- Wire `mcp_no_content` into MCP tool result handler
- Wire `malware_analysis_warning` into file content analysis

### Phase 3: Turn-start context (6 reminders)
- Wire `session_continuation`, `compact_reference`, `output_style`, `plan_file_reference`, `invoked_skills`, `task_tools` into `build_initial_context()`

### Phase 4: Plan mode lifecycle (7 reminders)
- Wire all plan mode reminders into collaboration mode transition handlers

### Phase 5: Hook lifecycle (3 reminders)
- Wire `hook_blocking`, `hook_success`, `hook_stopped_continuation` into existing hook runtime

### Phase 6: Event-driven (15 reminders)
- Wire remaining reminders into their respective event handlers

## Testing

Each reminder injection should be testable by:
1. Setting up the trigger condition in a test
2. Verifying the reminder text appears in the conversation items or tool output
3. Existing test infrastructure in `codex_tests.rs` provides session/turn mocking
