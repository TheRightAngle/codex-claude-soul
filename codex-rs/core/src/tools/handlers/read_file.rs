pub mod indentation;
pub mod slice;

use std::path::PathBuf;

use indentation::IndentationArgs;
use serde::Deserialize;

use crate::function_tool::FunctionCallError;
use crate::tools::context::FunctionToolOutput;
use crate::tools::context::ToolInvocation;
use crate::tools::context::ToolPayload;
use crate::tools::handlers::parse_arguments;
use crate::tools::registry::ToolHandler;
use crate::tools::registry::ToolKind;
use codex_protocol::models::reminders;

/// Maximum characters to show per line before silent truncation.
pub(crate) const MAX_LINE_LENGTH: usize = 2000;

/// Maximum lines that may be returned in a single call before the output is
/// considered truncated and the truncation reminder is attached.
const DEFAULT_LIMIT: usize = 250;

fn default_offset() -> usize {
    1
}

fn default_limit() -> usize {
    DEFAULT_LIMIT
}

#[derive(Deserialize)]
struct ReadFileArgs {
    /// Absolute path of the file to read.
    path: String,
    /// 1-indexed starting line number.
    #[serde(default = "default_offset")]
    offset: usize,
    /// Maximum number of lines to return.
    #[serde(default = "default_limit")]
    limit: usize,
    /// Optional indentation-aware navigation options.
    #[serde(default)]
    indentation: Option<IndentationArgsInput>,
}

/// Serializable form of [`IndentationArgs`] as it arrives from the model.
#[derive(Deserialize, Default)]
struct IndentationArgsInput {
    anchor_line: Option<usize>,
    #[serde(default)]
    include_siblings: bool,
    #[serde(default)]
    max_levels: usize,
    #[serde(default = "default_include_header")]
    include_header: bool,
}

fn default_include_header() -> bool {
    true
}

impl From<IndentationArgsInput> for IndentationArgs {
    fn from(input: IndentationArgsInput) -> Self {
        IndentationArgs {
            anchor_line: input.anchor_line,
            include_siblings: input.include_siblings,
            max_levels: input.max_levels,
            include_header: input.include_header,
        }
    }
}

pub struct ReadFileHandler;

impl ToolHandler for ReadFileHandler {
    type Output = FunctionToolOutput;

    fn kind(&self) -> ToolKind {
        ToolKind::Function
    }

    async fn handle(&self, invocation: ToolInvocation) -> Result<Self::Output, FunctionCallError> {
        let ToolInvocation { payload, .. } = invocation;

        let arguments = match payload {
            ToolPayload::Function { arguments } => arguments,
            _ => {
                return Err(FunctionCallError::RespondToModel(
                    "read_file handler received unsupported payload".to_string(),
                ));
            }
        };

        let args: ReadFileArgs = parse_arguments(&arguments)?;

        if args.offset == 0 {
            return Err(FunctionCallError::RespondToModel(
                "offset must be a 1-indexed line number".to_string(),
            ));
        }

        if args.limit == 0 {
            return Err(FunctionCallError::RespondToModel(
                "limit must be greater than zero".to_string(),
            ));
        }

        let path = PathBuf::from(&args.path);
        if !path.is_absolute() {
            return Err(FunctionCallError::RespondToModel(
                "path must be an absolute path".to_string(),
            ));
        }

        // Check whether the file exists and get its metadata.
        let metadata = tokio::fs::metadata(&path).await.map_err(|err| {
            FunctionCallError::RespondToModel(format!("failed to read file: {err}"))
        })?;

        // Empty file — return an empty result with the FILE_EMPTY reminder.
        if metadata.len() == 0 {
            let output =
                crate::reminder_injection::annotate_tool_result("", reminders::FILE_EMPTY);
            return Ok(FunctionToolOutput::from_text(output, Some(true)));
        }

        // Perform the read.
        let lines_result = if let Some(indent_input) = args.indentation {
            let indent_args = IndentationArgs::from(indent_input);
            indentation::read_block(&path, args.offset, args.limit, indent_args).await
        } else {
            slice::read(&path, args.offset, args.limit).await
        };

        let lines = match lines_result {
            Ok(lines) => lines,
            Err(FunctionCallError::RespondToModel(msg))
                if msg == "offset exceeds file length" =>
            {
                // Offset past end — return reminder instead of an error so the
                // model can self-correct without failing the tool call.
                let output = crate::reminder_injection::annotate_tool_result(
                    &format!("Error: {msg}"),
                    reminders::FILE_SHORTER_THAN_OFFSET,
                );
                return Ok(FunctionToolOutput::from_text(output, Some(false)));
            }
            Err(e) => return Err(e),
        };

        let total_lines_returned = lines.len();
        let text = lines.join("\n");

        // If the caller requested more lines than were returned AND the file
        // has more lines (i.e. output was truncated), attach the FILE_TRUNCATED
        // reminder so the model knows to paginate.
        let output = if total_lines_returned == args.limit {
            // Exactly limit lines were returned — the file likely has more.
            crate::reminder_injection::annotate_tool_result(&text, reminders::FILE_TRUNCATED)
        } else {
            text
        };

        Ok(FunctionToolOutput::from_text(output, Some(true)))
    }
}

#[cfg(test)]
#[path = "read_file_tests.rs"]
mod tests;
