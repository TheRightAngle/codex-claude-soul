use crate::context_manager::updates::build_developer_update_item;
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
