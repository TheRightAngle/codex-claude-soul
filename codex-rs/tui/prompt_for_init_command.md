Generate an AGENTS.md file that serves as a concise guide for AI coding agents working in this repository.

Your goal is a document that an agent can read and immediately know: how to build, how to test, how to lint, and what patterns to follow. Skip anything an agent can infer from the codebase itself.

Document Requirements

- Title the document "AGENTS.md"
- Use Markdown headings for structure.
- Keep it concise. 150-300 words is optimal — shorter is better.
- Be specific and actionable. "Run `npm test`" is better than "Use the testing framework."
- Include exact commands, not descriptions of commands.
- Maintain an opinionated, direct tone — like notes from a senior dev to a colleague.

Recommended Sections

Build & Test Commands

- List the exact commands for building, testing, linting, and running locally.
- Include the command for running a single test file if different from the full suite.
- Note any required setup steps (env vars, database, etc).

Code Style & Conventions

- Specify only conventions that aren't obvious from the code itself.
- Include formatting tools and their commands (e.g., `npx prettier --write .`).
- Note any naming patterns specific to this project.

Architecture Notes (if non-obvious)

- Only include if the project structure would surprise a new developer.
- Focus on "where to find things" not "how things work."

Common Pitfalls

- List things that commonly trip up contributors.
- Include any "don't do X" rules that aren't enforced by linting.

Do NOT include:
- Generic advice that applies to any project
- Copyright or license information
- Information easily discoverable from package.json, Cargo.toml, etc.
- Sections with no project-specific content
