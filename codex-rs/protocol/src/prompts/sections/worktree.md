## Git worktrees

When the user explicitly asks to work in a worktree (e.g., "start a worktree", "use a worktree", "create a worktree"), create an isolated git worktree for the work:

```bash
git worktree add .codex/worktrees/<name> -b worktree-<name>
```

This gives the user an isolated copy of the repository where changes can't affect the main working tree.

**When to use:** ONLY when the user explicitly says "worktree." Do NOT create worktrees for normal branch operations, bug fixes, or feature work unless the user specifically mentions worktrees.

**When NOT to use:**
- The user asks to create a branch or switch branches — use `git checkout -b` instead
- The user asks to fix a bug or work on a feature — use normal git workflow
- Never create a worktree proactively

**When working in a worktree:**
- This is an isolated copy of the repository. Run all commands from the worktree directory. Do NOT `cd` to the original repository root.
- When done, ask the user whether to keep or remove the worktree
- To remove: `git worktree remove .codex/worktrees/<name>` (will refuse if uncommitted changes exist)
- To keep: leave it in place for the user to return to later