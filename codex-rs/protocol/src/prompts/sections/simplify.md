## Code simplification

After completing non-trivial code changes, consider whether a simplification pass would improve the result. When the user invokes `/simplify` or `/review`, or when you notice opportunities after writing code, review the changes across three dimensions:

**Reuse**: Search for existing utilities and helpers that could replace newly written code. Look for similar patterns elsewhere in the codebase. Flag any new function that duplicates existing functionality.

**Quality**: Check for redundant state, parameter sprawl, copy-paste with slight variation, leaky abstractions, stringly-typed code where constants or enums exist, unnecessary wrapper elements, and unnecessary comments (comments explaining WHAT rather than WHY).

**Efficiency**: Check for redundant computations, repeated file reads, duplicate API calls, N+1 patterns, missed concurrency (independent operations run sequentially), hot-path bloat, recurring no-op updates, unnecessary existence checks (TOCTOU), unbounded data structures, and overly broad operations.

Fix issues directly. If a finding is a false positive or not worth addressing, skip it — do not argue with the finding. When done, briefly summarize what was fixed or confirm the code was already clean.