Set up a minimal AGENTS.md (and optionally skills and hooks) for this repo. AGENTS.md is loaded into every Codex session, so it must be concise — only include what the agent would get wrong without it.

## Phase 1: Explore the codebase

Launch a subagent to survey the codebase. Read key files to understand the project: manifest files (package.json, Cargo.toml, pyproject.toml, go.mod, pom.xml, etc.), README, Makefile/build configs, CI config, existing AGENTS.md, .codex/ directory.

Detect:
- Build, test, and lint commands (especially non-standard ones)
- Languages, frameworks, and package manager
- Project structure (monorepo with workspaces, multi-module, or single project)
- Code style rules that differ from language defaults
- Non-obvious gotchas, required env vars, or workflow quirks
- Formatter configuration (prettier, biome, ruff, black, gofmt, rustfmt, or a unified format script)
- Existing .codex/skills/ directories

Note what you could NOT figure out from code alone — these become interview questions.

## Phase 2: Fill in the gaps

Ask the user about things the code can't answer:
- Non-obvious commands, gotchas, branch/PR conventions
- Required env setup, testing quirks
- Code style preferences that differ from language defaults

Skip things already in README or obvious from manifest files. Do not mark any options as "recommended" — this is about how their team works.

## Phase 3: Write AGENTS.md

Write a minimal AGENTS.md at the project root. Every line must pass this test: "Would removing this cause the agent to make mistakes?" If no, cut it.

Include:
- Build/test/lint commands the agent can't guess (non-standard scripts, flags, or sequences)
- Code style rules that DIFFER from language defaults
- Testing instructions and quirks (e.g., "run single test with: pytest -k 'test_name'")
- Repo etiquette (branch naming, PR conventions, commit style)
- Required env vars or setup steps
- Non-obvious gotchas or architectural decisions

Exclude:
- File-by-file structure or component lists (the agent can discover these)
- Standard language conventions the agent already knows
- Generic advice ("write clean code", "handle errors")
- Detailed API docs or long references
- Commands obvious from manifest files (e.g., standard "npm test", "cargo test", "pytest")

Be specific: "Use 2-space indentation in TypeScript" is better than "Format code properly."

Prefix the file with:

```
# AGENTS.md

This file provides guidance to AI coding agents when working with code in this repository.
```

If AGENTS.md already exists: read it, propose specific changes as diffs, and explain why each change improves it. Do not silently overwrite.

## Phase 4: Suggest and create skills

Skills add capabilities the agent can use on demand without bloating every session.

Suggest skills when you find:
- Repeatable workflows (deploy, verify changes, release process)
- Reference knowledge for specific tasks (conventions, patterns, style guides)
- Complex multi-step operations that benefit from a structured guide

For each suggested skill, provide: name, one-line purpose, and why it fits this repo.

Create each skill at `.codex/skills/<skill-name>/SKILL.md`:

```yaml
---
name: <skill-name>
description: <what the skill does and when to use it>
---

<Instructions for the agent>
```

## Phase 5: Suggest hooks

If relevant, suggest hooks — deterministic shell commands that run on tool events (e.g., format after every edit). Hooks can't be skipped by the agent. Good candidates:
- Format-on-edit if a formatter exists
- Lint check after file modifications
- Type checking after edits to typed codebases

## Phase 6: Check environment and suggest optimizations

Check the environment and suggest improvements:
- **GitHub CLI**: If missing and the project uses GitHub, suggest installing it for PR/issue workflows
- **Linting**: If no lint config found, suggest setting up linting
- **Testing**: If tests are missing or sparse, suggest setting up a test framework

## Phase 7: Summary

Recap what was set up — which files were written and the key points in each. Remind the user these files are a starting point: they should review and tweak them, and can run `/init` again anytime to re-scan.

Keep the entire AGENTS.md under 300 words. Shorter is better. An opinionated, direct tone — like notes from a senior dev to a colleague.
