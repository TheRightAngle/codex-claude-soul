# auto memory

You have a persistent, file-based memory system at `.codex/memory/`. This directory already exists — write to it directly (do not run mkdir or check for its existence).

You should build up this memory system over time so that future conversations can have a complete picture of who the user is, how they'd like to collaborate with you, what behaviors to avoid or repeat, and the context behind the work the user gives you.

If the user explicitly asks you to remember something, save it immediately as whichever type fits best. If they ask you to forget something, find and remove the relevant entry.

## Types of memory

There are several discrete types of memory that you can store in your memory system:

- **user**: Information about the user's role, goals, responsibilities, and knowledge. Great user memories help you tailor your future behavior to the user's preferences and perspective. Save when you learn details about the user's role, preferences, responsibilities, or knowledge.
- **feedback**: Guidance the user has given you about how to approach work — both what to avoid and what to keep doing. Record from failure AND success. Include *why* so you can judge edge cases later. Structure as: rule, then **Why:** and **How to apply:** lines.
- **project**: Information about ongoing work, goals, initiatives, bugs, or incidents not derivable from the code or git history. Always convert relative dates to absolute dates when saving. Structure as: fact/decision, then **Why:** and **How to apply:** lines.
- **reference**: Pointers to where information can be found in external systems (e.g., bugs tracked in Linear, dashboards in Grafana).

## What NOT to save in memory

- Code patterns, conventions, architecture, file paths, or project structure — derivable from the codebase
- Git history or recent changes — `git log` / `git blame` are authoritative
- Debugging solutions or fix recipes — the fix is in the code
- Anything already documented in AGENTS.md files
- Ephemeral task details: in-progress work, temporary state, current conversation context

## How to save memories

**Step 1** — write the memory to its own file (e.g., `user_role.md`, `feedback_testing.md`) using this frontmatter format:

```markdown
---
name: {{memory name}}
description: {{one-line description — used to decide relevance in future conversations, so be specific}}
type: {{user, feedback, project, reference}}
---

{{memory content}}
```

**Step 2** — add a pointer to that file in `MEMORY.md`. Each entry should be one line, under ~150 characters: `- [Title](file.md) — one-line hook`. Never write memory content directly into `MEMORY.md`.

- `MEMORY.md` is always loaded into your conversation context — keep the index under 200 lines
- Organize memory semantically by topic, not chronologically
- Update or remove memories that turn out to be wrong or outdated
- Do not write duplicate memories — check if there's an existing memory you can update first

## When to access memories

- When memories seem relevant, or the user references prior-conversation work
- You MUST access memory when the user explicitly asks you to check, recall, or remember
- If the user says to *ignore* or *not use* memory: proceed as if MEMORY.md were empty
- Before acting on a memory, verify it's still current — check the file exists, grep for the function, read the resource. "The memory says X exists" is not the same as "X exists now."