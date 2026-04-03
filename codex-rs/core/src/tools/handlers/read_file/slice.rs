use std::path::Path;

use crate::function_tool::FunctionCallError;
use crate::tools::handlers::read_file::MAX_LINE_LENGTH;

/// Read `limit` lines from `path` starting at 1-indexed `offset`.
///
/// Returns a `Vec<String>` where each entry is formatted as `"L{n}: {content}"`.
/// Lines longer than [`MAX_LINE_LENGTH`] are silently truncated.
///
/// # Errors
///
/// Returns [`FunctionCallError::RespondToModel`] when `offset` exceeds the
/// number of lines in the file.
pub async fn read(
    path: &Path,
    offset: usize,
    limit: usize,
) -> Result<Vec<String>, FunctionCallError> {
    let raw = tokio::fs::read(path)
        .await
        .map_err(|err| FunctionCallError::RespondToModel(format!("failed to read file: {err}")))?;

    let text = String::from_utf8_lossy(&raw);

    let all_lines: Vec<&str> = text.lines().collect();
    let total = all_lines.len();

    if total == 0 {
        return Ok(Vec::new());
    }

    // offset is 1-indexed
    if offset > total {
        return Err(FunctionCallError::RespondToModel(
            "offset exceeds file length".to_string(),
        ));
    }

    let start = offset - 1;
    let end = (start + limit).min(total);
    let selected = &all_lines[start..end];

    let lines = selected
        .iter()
        .enumerate()
        .map(|(i, line)| {
            let line_no = offset + i;
            let truncated = if line.len() > MAX_LINE_LENGTH {
                &line[..MAX_LINE_LENGTH]
            } else {
                line
            };
            // Trim CRLF endings that survived `lines()` on some platforms.
            let trimmed = truncated.trim_end_matches('\r');
            format!("L{line_no}: {trimmed}")
        })
        .collect();

    Ok(lines)
}
