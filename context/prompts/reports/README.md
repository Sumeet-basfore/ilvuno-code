# Reports from the prompts

Every prompt in `context/prompts/` finishes by leaving one file here:
`<YYYY-MM-DD>-<prompt-name>.md`, with these five sections and nothing else.

```
## Goal
One line: which prompt this answers.

## What changed
Commits and files. Hashes, not adjectives.

## Verified
What you ran, and the result. Test counts, real runs, screenshots you read, numbers you measured.

## Could not verify
The honest list. An empty list is a claim, so only leave it empty if it is true.

## Still open
What the next prompt should pick up.
```

Why here and not in `context/timeline.md`: the timeline is the long memory, this is the working record of one
run of one prompt. The timeline gets one line per report, pointing at the file.
