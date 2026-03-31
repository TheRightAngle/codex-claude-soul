use strum::IntoEnumIterator;
use strum_macros::AsRefStr;
use strum_macros::EnumIter;
use strum_macros::EnumString;
use strum_macros::IntoStaticStr;

/// Commands that can be invoked by starting a message with a leading slash.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, EnumString, EnumIter, AsRefStr, IntoStaticStr,
)]
#[strum(serialize_all = "kebab-case")]
pub enum SlashCommand {
    // DO NOT ALPHA-SORT! Enum order is presentation order in the popup, so
    // more frequently used commands should be listed first.
    Model,
    Fast,
    Approvals,
    Permissions,
    #[strum(serialize = "setup-default-sandbox")]
    ElevateSandbox,
    #[strum(serialize = "sandbox-add-read-dir")]
    SandboxReadRoot,
    Experimental,
    Skills,
    Review,
    Rename,
    New,
    Resume,
    Fork,
    Init,
    Compact,
    Plan,
    Collab,
    Agent,
    // Undo,
    Diff,
    Copy,
    Mention,
    Status,
    DebugConfig,
    Title,
    Statusline,
    Theme,
    Mcp,
    Apps,
    Plugins,
    Logout,
    Quit,
    Exit,
    Feedback,
    Rollout,
    Ps,
    #[strum(to_string = "stop", serialize = "clean")]
    Stop,
    Clear,
    Personality,
    Realtime,
    Settings,
    TestApproval,
    #[strum(serialize = "subagents")]
    MultiAgents,
    // Debugging commands.
    #[strum(serialize = "debug-m-drop")]
    MemoryDrop,
    #[strum(serialize = "debug-m-update")]
    MemoryUpdate,
}

impl SlashCommand {
    /// User-visible description shown in the popup.
    pub fn description(self) -> &'static str {
        match self {
            SlashCommand::Feedback => "Something look off? Send logs to the maintainers",
            SlashCommand::New => "Start fresh — previous session stays in history",
            SlashCommand::Init => "Create an AGENTS.md with project-specific guidance",
            SlashCommand::Compact => "Summarize conversation and free up context",
            SlashCommand::Review => "Get a code review of your current changes",
            SlashCommand::Rename => "Rename this thread for easier resuming",
            SlashCommand::Resume => "Pick up where you left off in a saved chat",
            SlashCommand::Clear => "Clear the terminal and start a new chat",
            SlashCommand::Fork => "Branch this chat into a new thread for exploration",
            // SlashCommand::Undo => "ask Codex to undo a turn",
            SlashCommand::Quit | SlashCommand::Exit => "Exit Codex",
            SlashCommand::Diff => "Show git diff including untracked files",
            SlashCommand::Copy => "Copy the latest output to your clipboard",
            SlashCommand::Mention => "Reference a file in your message",
            SlashCommand::Skills => "List available skills or ask Codex to use one",
            SlashCommand::Status => "See current model, approvals, and token usage",
            SlashCommand::DebugConfig => "Inspect config layers and requirement sources",
            SlashCommand::Title => "Choose what shows in the terminal title",
            SlashCommand::Statusline => "Choose what shows in the status bar",
            SlashCommand::Theme => "Pick a syntax highlighting theme",
            SlashCommand::Ps => "List background terminals",
            SlashCommand::Stop => "Stop all background terminals",
            SlashCommand::MemoryDrop => "DO NOT USE",
            SlashCommand::MemoryUpdate => "DO NOT USE",
            SlashCommand::Model => "Switch models or adjust reasoning effort",
            SlashCommand::Fast => "Toggle Fast mode for fastest inference (2X plan usage)",
            SlashCommand::Personality => "Adjust how Codex communicates",
            SlashCommand::Realtime => "Toggle realtime voice mode (experimental)",
            SlashCommand::Settings => "Configure realtime microphone and speaker",
            SlashCommand::Plan => "Switch to Plan mode",
            SlashCommand::Collab => "Change collaboration mode (experimental)",
            SlashCommand::Agent | SlashCommand::MultiAgents => "Switch between agent threads",
            SlashCommand::Approvals => "Control when Codex asks for confirmation",
            SlashCommand::Permissions => "Control when Codex asks for confirmation",
            SlashCommand::ElevateSandbox => "Set up elevated agent sandbox",
            SlashCommand::SandboxReadRoot => {
                "Grant sandbox read access to a directory"
            }
            SlashCommand::Experimental => "Toggle experimental features",
            SlashCommand::Mcp => "List your configured MCP tools",
            SlashCommand::Apps => "Manage apps",
            SlashCommand::Plugins => "Browse plugins",
            SlashCommand::Logout => "Log out of Codex",
            SlashCommand::Rollout => "Print the rollout file path",
            SlashCommand::TestApproval => "Test an approval request",
        }
    }

    /// Command string without the leading '/'. Provided for compatibility with
    /// existing code that expects a method named `command()`.
    pub fn command(self) -> &'static str {
        self.into()
    }

    /// Whether this command supports inline args (for example `/review ...`).
    pub fn supports_inline_args(self) -> bool {
        matches!(
            self,
            SlashCommand::Review
                | SlashCommand::Rename
                | SlashCommand::Plan
                | SlashCommand::Fast
                | SlashCommand::SandboxReadRoot
        )
    }

    /// Whether this command can be run while a task is in progress.
    pub fn available_during_task(self) -> bool {
        match self {
            SlashCommand::New
            | SlashCommand::Resume
            | SlashCommand::Fork
            | SlashCommand::Init
            | SlashCommand::Compact
            // | SlashCommand::Undo
            | SlashCommand::Model
            | SlashCommand::Fast
            | SlashCommand::Personality
            | SlashCommand::Approvals
            | SlashCommand::Permissions
            | SlashCommand::ElevateSandbox
            | SlashCommand::SandboxReadRoot
            | SlashCommand::Experimental
            | SlashCommand::Review
            | SlashCommand::Plan
            | SlashCommand::Clear
            | SlashCommand::Logout
            | SlashCommand::MemoryDrop
            | SlashCommand::MemoryUpdate => false,
            SlashCommand::Diff
            | SlashCommand::Copy
            | SlashCommand::Rename
            | SlashCommand::Mention
            | SlashCommand::Skills
            | SlashCommand::Status
            | SlashCommand::DebugConfig
            | SlashCommand::Ps
            | SlashCommand::Stop
            | SlashCommand::Mcp
            | SlashCommand::Apps
            | SlashCommand::Plugins
            | SlashCommand::Feedback
            | SlashCommand::Quit
            | SlashCommand::Exit => true,
            SlashCommand::Rollout => true,
            SlashCommand::TestApproval => true,
            SlashCommand::Realtime => true,
            SlashCommand::Settings => true,
            SlashCommand::Collab => true,
            SlashCommand::Agent | SlashCommand::MultiAgents => true,
            SlashCommand::Statusline => false,
            SlashCommand::Theme => false,
            SlashCommand::Title => false,
        }
    }

    fn is_visible(self) -> bool {
        match self {
            SlashCommand::SandboxReadRoot => cfg!(target_os = "windows"),
            SlashCommand::Copy => !cfg!(target_os = "android"),
            SlashCommand::Rollout | SlashCommand::TestApproval => cfg!(debug_assertions),
            _ => true,
        }
    }
}

/// Return all built-in commands in a Vec paired with their command string.
pub fn built_in_slash_commands() -> Vec<(&'static str, SlashCommand)> {
    SlashCommand::iter()
        .filter(|command| command.is_visible())
        .map(|c| (c.command(), c))
        .collect()
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use std::str::FromStr;

    use super::SlashCommand;

    #[test]
    fn stop_command_is_canonical_name() {
        assert_eq!(SlashCommand::Stop.command(), "stop");
    }

    #[test]
    fn clean_alias_parses_to_stop_command() {
        assert_eq!(SlashCommand::from_str("clean"), Ok(SlashCommand::Stop));
    }
}
