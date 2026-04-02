---
name: debugging
description: Help debug an issue in the current Codex session by reviewing logs, errors, and system state.
---

# Debug Skill

Help the user debug an issue they're encountering in this current Codex session.

## Instructions

1. **Review the issue description** — Read what the user reported. If they didn't describe a specific issue, check the debug log and summarize any errors, warnings, or notable issues.

2. **Scan the debug log** — Look for `[ERROR]` and `[WARN]` entries, stack traces, and failure patterns. Check the last 200 lines first, then grep for error patterns across the full file. Common patterns to look for:
   - API connection failures or timeouts
   - Tool execution errors
   - Permission denials
   - Configuration parsing errors
   - Out-of-memory or resource exhaustion

3. **Check system state** — Review relevant configuration:
   - Settings files (user, project, local)
   - Environment variables that affect behavior
   - MCP server connection status
   - Sandbox policy and approval mode

4. **Explain findings** — Describe what you found in plain language. Connect the log evidence to the user's reported symptoms. Don't just dump log lines — interpret them.

5. **Suggest concrete fixes** — Provide specific, actionable next steps:
   - Configuration changes with exact file paths and values
   - Commands to run for diagnosis or repair
   - Workarounds if a fix isn't immediately available
   - When to restart the session vs. when a runtime fix suffices

## Tips

- If debug logging was not active, tell the user to restart with `codex --debug` to capture logs from startup, then reproduce the issue.
- For intermittent issues, suggest enabling debug logging and leaving it on until the problem recurs.
- If the issue is a known limitation, say so directly rather than troubleshooting further.
