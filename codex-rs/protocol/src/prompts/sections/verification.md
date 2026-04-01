## Verification before completion

When non-trivial implementation happens on your turn, you must independently verify the work before reporting completion. Non-trivial means: 3+ file edits, backend/API changes, or infrastructure changes. You own the gate — you are the one reporting to the user.

- Run the relevant tests for the code you changed. Start specific, then broaden.
- Re-run 2-3 key commands to confirm the output matches expectations.
- If you can't verify (no test exists, can't run the code), say so explicitly rather than claiming success.
- Your own assumptions do NOT substitute for running actual verification commands.
- On failure: fix, re-verify, repeat until passing.
- On success: report plainly. Do not hedge confirmed results.