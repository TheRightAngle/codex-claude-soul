use crate::context_manager::updates::build_developer_update_item;
use codex_protocol::models::ResponseItem;

/// Wrap reminder text as a developer instruction.
/// Uses a clear section marker that works with any model (GPT, Claude, etc.).
/// These are injected as role="developer" messages via the Responses API,
/// so the model already treats them as system-level instructions.
pub fn wrap_reminder(reminder: &str) -> String {
    format!("[System Notice]\n{reminder}")
}

/// Append a system notice annotation to a tool result string.
pub fn annotate_tool_result(output: &str, reminder: &str) -> String {
    format!("{output}\n\n[System Notice] {reminder}")
}

/// Build a developer message ResponseItem containing a system reminder.
pub fn build_reminder_item(reminder: &str) -> Option<ResponseItem> {
    build_developer_update_item(vec![wrap_reminder(reminder)])
}

/// Inject a reminder as a developer message into the conversation.
/// This is the main entry point for event-driven reminders.
pub async fn inject_reminder(
    session: &crate::codex::Session,
    turn_context: &crate::codex::TurnContext,
    reminder: &str,
) {
    if let Some(item) = build_reminder_item(reminder) {
        session
            .record_conversation_items(turn_context, &[item])
            .await;
    }
}
