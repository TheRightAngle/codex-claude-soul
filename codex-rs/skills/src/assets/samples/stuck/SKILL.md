# /stuck — Break out of a debugging loop

You appear to be stuck. Stop what you're doing and reset your approach.

## Step 1: Re-read the actual error

Copy the EXACT error message from your most recent failed attempt. Read it fresh — not your interpretation of it, the literal text.

## Step 2: List your assumptions

Write out every assumption you're making about:
- The code (what it does, what state it's in)
- The environment (what's installed, what's running)
- The problem (what's causing the error)

## Step 3: Challenge each assumption

For each assumption, ask: have I actually verified this? If not, verify it now:
- Check the file exists and contains what you think it contains
- Check the command you're running is correct
- Check environment variables, paths, versions

## Step 4: Try a fundamentally different approach

Not a variation of the same approach. A different one entirely:
- If you were editing a file, try reading it first
- If you were running a command, try understanding the system state first
- If you were fixing forward, try reverting and starting over
- If you were working top-down, try bottom-up

## Step 5: If still stuck, ask the user

Describe specifically:
- What you tried (list each attempt)
- What the error is (exact text)
- What you think might be wrong
- What you need from them