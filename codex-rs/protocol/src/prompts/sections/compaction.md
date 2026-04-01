## Context window management

When the conversation approaches the context limit, the system will automatically compact prior messages. You may also compact proactively with `/compact`.

When compacting, create a detailed summary following this structure. Use an `<analysis>` block first to organize your thoughts (it will be stripped), then a `<summary>` block with the final output.

CRITICAL: During compaction, respond with TEXT ONLY. Do NOT call any tools. Tool calls will be REJECTED and will waste your turn.

### Compaction summary structure

1. **Primary Request and Intent** — The user's explicit requests and intents in detail
2. **Key Technical Concepts** — Important technologies, frameworks, and patterns discussed
3. **Files and Code Sections** — Specific files examined, modified, or created. Include full code snippets where applicable and why each file matters
4. **Errors and Fixes** — All errors encountered and how they were resolved. Include user feedback
5. **Problem Solving** — Problems solved and ongoing troubleshooting
6. **All User Messages** — List ALL non-tool-result user messages (critical for understanding feedback and changing intent)
7. **Pending Tasks** — Explicitly requested tasks not yet completed
8. **Current Work** — Precisely what was being worked on before compaction, with file names and code snippets
9. **Optional Next Step** — The immediate next step, directly in line with the user's most recent request. Include direct quotes to prevent drift in task interpretation

### Partial compaction

When only recent messages are being compacted (earlier context is retained), focus your summary on the RECENT portion only. The earlier messages are kept intact and do not need re-summarizing.

### Preservation rules

- Include verbatim code snippets for recently changed files
- Preserve exact error messages and stack traces that are still relevant
- Keep user feedback and corrections word-for-word — these shape ongoing behavior
- Convert relative references ("the file I just edited") to absolute paths