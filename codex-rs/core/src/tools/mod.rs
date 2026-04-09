pub(crate) mod code_mode;
pub(crate) mod context;
pub(crate) mod events;
pub(crate) mod handlers;
pub(crate) mod js_repl;
pub(crate) mod network_approval;
pub(crate) mod orchestrator;
pub(crate) mod parallel;
pub(crate) mod registry;
pub(crate) mod router;
pub(crate) mod runtimes;
pub(crate) mod sandboxing;
pub(crate) mod spec;

use codex_protocol::exec_output::ExecToolCallOutput;
use codex_utils_output_truncation::TruncationPolicy;
use codex_utils_output_truncation::formatted_truncate_text;
use codex_utils_output_truncation::truncate_text;
pub use router::ToolRouter;
use serde::Serialize;

// Telemetry preview limits: keep log events smaller than model budgets.
pub(crate) const TELEMETRY_PREVIEW_MAX_BYTES: usize = 2 * 1024; // 2 KiB
pub(crate) const TELEMETRY_PREVIEW_MAX_LINES: usize = 64; // lines
pub(crate) const TELEMETRY_PREVIEW_TRUNCATION_NOTICE: &str =
    "[... telemetry preview truncated ...]";

/// Format the combined exec output for sending back to the model.
/// Includes exit code and duration metadata; truncates large bodies safely.
pub fn format_exec_output_for_model_structured(
    exec_output: &ExecToolCallOutput,
    truncation_policy: TruncationPolicy,
) -> String {
    let ExecToolCallOutput {
        exit_code,
        duration,
        ..
    } = exec_output;

    #[derive(Serialize)]
    struct ExecMetadata {
        exit_code: i32,
        duration_seconds: f32,
    }

    #[derive(Serialize)]
    struct ExecOutput<'a> {
        output: &'a str,
        metadata: ExecMetadata,
    }

    // round to 1 decimal place
    let duration_seconds = ((duration.as_secs_f32()) * 10.0).round() / 10.0;

    let formatted_output = format_exec_output_str(exec_output, truncation_policy);

    let payload = ExecOutput {
        output: &formatted_output,
        metadata: ExecMetadata {
            exit_code: *exit_code,
            duration_seconds,
        },
    };

    #[expect(clippy::expect_used)]
    serde_json::to_string(&payload).expect("serialize ExecOutput")
}

pub fn format_exec_output_for_model_freeform(
    exec_output: &ExecToolCallOutput,
    truncation_policy: TruncationPolicy,
) -> String {
    // round to 1 decimal place
    let duration_seconds = ((exec_output.duration.as_secs_f32()) * 10.0).round() / 10.0;

    let content = build_content_with_timeout(exec_output);

    let total_lines = content.lines().count();

    let formatted_output = truncate_text(&content, truncation_policy);
    let was_truncated = total_lines != formatted_output.lines().count();

    let mut sections = Vec::new();

    sections.push(format!("Exit code: {}", exec_output.exit_code));
    sections.push(format!("Wall time: {duration_seconds} seconds"));
    if was_truncated {
        sections.push(format!("Total output lines: {total_lines}"));
    }

    sections.push("Output:".to_string());
    sections.push(formatted_output);

    let output = sections.join("\n");

    // Post-truncation file-state annotations.
    annotate_exec_output(output, was_truncated)
}

pub fn format_exec_output_str(
    exec_output: &ExecToolCallOutput,
    truncation_policy: TruncationPolicy,
) -> String {
    let content = build_content_with_timeout(exec_output);

    // Check if truncation will be applied (same heuristic as formatted_truncate_text).
    let will_truncate = content.len() > truncation_policy.byte_budget();

    // Truncate for model consumption before serialization.
    let output = formatted_truncate_text(&content, truncation_policy);

    // Post-truncation file-state annotations.
    annotate_exec_output(output, will_truncate)
}

/// Detect file-state patterns in formatted output and append system notice annotations.
///
/// Patterns detected:
/// - Empty or whitespace-only output -> FILE_EMPTY (unless it looks like an error)
/// - Truncated output -> FILE_TRUNCATED
/// - Offset-past-end errors -> FILE_SHORTER_THAN_OFFSET
/// - Suspicious obfuscation patterns -> MALWARE_ANALYSIS_WARNING
///
/// Real errors like "No such file or directory" are left unannotated.
fn annotate_exec_output(output: String, was_truncated: bool) -> String {
    // If the output indicates a real filesystem error, don't add file-state annotations.
    if output.contains("No such file or directory")
        || output.contains("cannot open")
        || output.contains("Permission denied")
    {
        return output;
    }

    let mut output = if was_truncated {
        crate::reminder_injection::annotate_tool_result(
            &output,
            codex_protocol::models::reminders::FILE_TRUNCATED,
        )
    } else if output.is_empty() || output.trim().is_empty() {
        crate::reminder_injection::annotate_tool_result(
            &output,
            codex_protocol::models::reminders::FILE_EMPTY,
        )
    } else {
        output
    };

    // FILE_SHORTER_THAN_OFFSET: Detect shell errors indicating an offset past end of file.
    if output.contains("offset") && output.contains("exceeds")
        || output.contains("line count") && output.contains("shorter")
        || output.contains("tail: cannot open")
    {
        output = crate::reminder_injection::annotate_tool_result(
            &output,
            codex_protocol::models::reminders::FILE_SHORTER_THAN_OFFSET,
        );
    }

    // MALWARE_ANALYSIS_WARNING: Detect suspicious content patterns that may indicate
    // obfuscated or malicious code in shell output.
    output = maybe_annotate_suspicious_content(output);

    output
}

/// Check for common obfuscation/malware patterns in tool output and annotate if found.
fn maybe_annotate_suspicious_content(output: String) -> String {
    // Detect eval+base64 combos, hex-encoded payloads, encoded powershell,
    // and piped-download-to-shell patterns.
    let has_eval_base64 = output.contains("eval(") && output.contains("base64");
    let has_hex_payload =
        output.contains("\\x") && output.len() > 500 && output.matches("\\x").count() > 20;
    let has_encoded_ps =
        output.contains("powershell -enc") || output.contains("powershell -e ");
    let has_piped_download =
        output.contains("curl | sh") || output.contains("wget -O - |");

    if has_eval_base64 || has_hex_payload || has_encoded_ps || has_piped_download {
        crate::reminder_injection::annotate_tool_result(
            &output,
            codex_protocol::models::reminders::MALWARE_ANALYSIS_WARNING,
        )
    } else {
        output
    }
}

/// Extracts exec output content and prepends a timeout message if the command timed out.
fn build_content_with_timeout(exec_output: &ExecToolCallOutput) -> String {
    if exec_output.timed_out {
        format!(
            "command timed out after {} milliseconds\n{}",
            exec_output.duration.as_millis(),
            exec_output.aggregated_output.text
        )
    } else {
        exec_output.aggregated_output.text.clone()
    }
}
