use std::path::Path;

use crate::function_tool::FunctionCallError;
use crate::tools::handlers::read_file::MAX_LINE_LENGTH;

/// Configuration for an indentation-guided read operation.
#[derive(Clone, Debug, Default)]
pub struct IndentationArgs {
    /// 1-indexed line number to use as the structural anchor. When `None` the
    /// whole requested range is returned unchanged.
    pub anchor_line: Option<usize>,
    /// Whether to include sibling blocks at the same indentation level as the
    /// anchor.
    pub include_siblings: bool,
    /// Maximum number of indentation levels to expand upward from the anchor.
    /// 0 means return only the anchor line itself.
    pub max_levels: usize,
    /// Whether to include the outermost enclosing header line when expanding
    /// levels.
    pub include_header: bool,
}

impl IndentationArgs {
    /// Return `true` if the defaults should be applied (no structural options
    /// requested).
    fn is_plain(&self) -> bool {
        self.anchor_line.is_none()
    }
}

/// Leading-space count of a line (tab = 1 space for our purposes).
fn indent_level(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

/// Read a block of lines from `path` anchored around `anchor_line` (1-indexed)
/// within the window `[offset, offset + limit)`.
///
/// The returned lines are formatted as `"L{n}: {content}"` and respect
/// [`MAX_LINE_LENGTH`].
pub async fn read_block(
    path: &Path,
    offset: usize,
    limit: usize,
    options: IndentationArgs,
) -> Result<Vec<String>, FunctionCallError> {
    if options.is_plain() {
        return crate::tools::handlers::read_file::slice::read(path, offset, limit).await;
    }

    let raw = tokio::fs::read(path)
        .await
        .map_err(|err| FunctionCallError::RespondToModel(format!("failed to read file: {err}")))?;

    let text = String::from_utf8_lossy(&raw);
    let all_lines: Vec<&str> = text.lines().collect();
    let total = all_lines.len();

    if total == 0 {
        return Ok(Vec::new());
    }

    // Clamp the window to available lines (1-indexed, inclusive).
    let window_start = offset.saturating_sub(1).min(total);
    let window_end = (window_start + limit).min(total);

    // Resolve anchor to 0-indexed within the full file.
    let anchor_0 = options
        .anchor_line
        .map(|a| a.saturating_sub(1).min(total - 1))
        .unwrap_or(window_start);

    let anchor_indent = indent_level(all_lines[anchor_0]);

    // Collect the enclosing block hierarchy starting from the anchor.
    let mut include_range = collect_block(
        &all_lines,
        anchor_0,
        anchor_indent,
        options.include_siblings,
        options.max_levels,
        options.include_header,
        window_start,
        window_end,
    );

    // Sort and deduplicate so output is ordered by line number.
    include_range.sort_unstable();
    include_range.dedup();

    let lines = include_range
        .into_iter()
        .filter(|&i| i >= window_start && i < window_end)
        .map(|i| {
            let line_no = i + 1;
            let line = all_lines[i];
            let truncated = if line.len() > MAX_LINE_LENGTH {
                &line[..MAX_LINE_LENGTH]
            } else {
                line
            };
            let trimmed = truncated.trim_end_matches('\r');
            format!("L{line_no}: {trimmed}")
        })
        .collect();

    Ok(lines)
}

/// Collect line indices for the block containing `anchor_0`, expanding
/// outward by up to `max_levels` indentation levels.
fn collect_block(
    lines: &[&str],
    anchor_0: usize,
    anchor_indent: usize,
    include_siblings: bool,
    max_levels: usize,
    include_header: bool,
    window_start: usize,
    window_end: usize,
) -> Vec<usize> {
    let mut result = Vec::new();

    // Walk up from the anchor to find its direct parent block boundary and,
    // optionally, higher-level enclosures.
    let mut current_anchor = anchor_0;
    let mut current_indent = anchor_indent;

    for level in 0..=max_levels {
        // Find the header line for the current block (first line above
        // current_anchor with strictly smaller indentation).
        let header = if current_indent == 0 {
            None
        } else {
            find_parent_header(lines, current_anchor, current_indent)
        };

        // Determine the block's start: just after the header (or the window
        // start if there is no header).
        let block_start = header.map(|h| h + 1).unwrap_or(window_start);

        // Determine the block's end: the line before the next sibling at the
        // same or lower indent than the header (or window_end if none).
        let header_indent = header.map(|h| indent_level(lines[h])).unwrap_or(0);
        let block_end = find_block_end(lines, current_anchor, current_indent, header_indent)
            .unwrap_or(window_end)
            .min(window_end);

        if level == 0 || include_siblings {
            // Include the anchor's own block and, when requested, sibling
            // blocks at the same indentation level.
            if include_siblings {
                // All lines in the parent block at the anchor's indent.
                collect_same_indent_blocks(
                    lines,
                    block_start,
                    block_end,
                    current_indent,
                    &mut result,
                );
            } else {
                // Only the specific sub-block containing the anchor.
                let sub_start =
                    find_sub_block_start(lines, current_anchor, current_indent, block_start);
                let sub_end =
                    find_sub_block_end(lines, current_anchor, current_indent, block_end)
                        .min(window_end);
                for i in sub_start..sub_end {
                    result.push(i);
                }
            }
        }

        // Include the header line itself if requested.
        if include_header || level + 1 < max_levels {
            if let Some(h) = header {
                if h >= window_start && h < window_end {
                    result.push(h);
                }
            }
        }

        // Move to the next level up.
        match header {
            None => break,
            Some(h) => {
                current_anchor = h;
                current_indent = header_indent;
                if current_indent == 0 {
                    // Include the outermost header if we reach the top.
                    if h >= window_start && h < window_end {
                        result.push(h);
                    }
                    // Also collect the closure line (matching brace/end) of
                    // the outermost block.
                    if let Some(close) = find_closure(lines, h, window_end) {
                        if close >= window_start && close < window_end {
                            result.push(close);
                        }
                    }
                    break;
                }
            }
        }
    }

    result
}

/// Find the first line above `from` (exclusive) with indent < `current_indent`.
fn find_parent_header(lines: &[&str], from: usize, current_indent: usize) -> Option<usize> {
    for i in (0..from).rev() {
        let trimmed = lines[i].trim();
        if trimmed.is_empty() {
            continue;
        }
        if indent_level(lines[i]) < current_indent {
            return Some(i);
        }
    }
    None
}

/// Find the end (exclusive) of the block containing `anchor_0` at indent
/// `anchor_indent`.  The block ends when a line with indent <= `header_indent`
/// appears after the anchor.
fn find_block_end(
    lines: &[&str],
    anchor_0: usize,
    anchor_indent: usize,
    header_indent: usize,
) -> Option<usize> {
    let _ = anchor_indent; // kept for symmetry / future use
    for i in (anchor_0 + 1)..lines.len() {
        let trimmed = lines[i].trim();
        if trimmed.is_empty() {
            continue;
        }
        if indent_level(lines[i]) <= header_indent {
            return Some(i + 1);
        }
    }
    None
}

/// Find the start of the specific sub-block that contains `anchor_0` (the
/// last line before anchor with indent strictly less than `anchor_indent`).
fn find_sub_block_start(
    lines: &[&str],
    anchor_0: usize,
    anchor_indent: usize,
    floor: usize,
) -> usize {
    for i in (floor..anchor_0).rev() {
        let trimmed = lines[i].trim();
        if trimmed.is_empty() {
            continue;
        }
        if indent_level(lines[i]) < anchor_indent {
            return i + 1;
        }
    }
    floor
}

/// Find the end (exclusive) of the sub-block containing `anchor_0` (first
/// line after anchor with indent strictly less than `anchor_indent`).
fn find_sub_block_end(
    lines: &[&str],
    anchor_0: usize,
    anchor_indent: usize,
    ceiling: usize,
) -> usize {
    for i in (anchor_0 + 1)..ceiling {
        let trimmed = lines[i].trim();
        if trimmed.is_empty() {
            continue;
        }
        if indent_level(lines[i]) < anchor_indent {
            return i + 1;
        }
    }
    ceiling
}

/// Collect all lines within `[block_start, block_end)` that belong to
/// same-indent blocks (i.e. have indent >= `target_indent`), along with their
/// closing lines.
fn collect_same_indent_blocks(
    lines: &[&str],
    block_start: usize,
    block_end: usize,
    target_indent: usize,
    result: &mut Vec<usize>,
) {
    for i in block_start..block_end {
        let trimmed = lines[i].trim();
        if trimmed.is_empty() {
            result.push(i);
            continue;
        }
        let ind = indent_level(lines[i]);
        if ind >= target_indent {
            result.push(i);
        } else {
            // This is the header/parent line — include it as well (closing
            // braces, `}`, etc.)
            result.push(i);
        }
    }
}

/// Given a header line, find the corresponding closure line (e.g. a `}` at
/// the same indentation level).
fn find_closure(lines: &[&str], header_0: usize, ceiling: usize) -> Option<usize> {
    let header_indent = indent_level(lines[header_0]);
    for i in (header_0 + 1)..ceiling.min(lines.len()) {
        let trimmed = lines[i].trim();
        if trimmed.is_empty() {
            continue;
        }
        if indent_level(lines[i]) == header_indent {
            return Some(i);
        }
    }
    None
}
