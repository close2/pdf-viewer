# Q738 — Is A35's host-supplied substitute for a null `/Font` owed, or only permitted?

Source: round 1865, the audit of every answered `A` file against the tree (ADR 2564).

## The question

`A35` approved `Q35`'s recommendation: "Keep the refusal as the default, and make the guess a thing the
reader can ask for, if it is wanted at all" — a host option that turns a null font entry into a
substituted face with default advances, the page still carrying its report. The refusal and the report
are in the tree (`crates/pdf-model/src/content/font.rs`); no host option exists. Does "if it is wanted
at all" make the option owed (a build), or a permission the tree may take when a reader asks for it?

## Why it cannot be settled without the owner

The approved sentence is conditional on a want only the owner can state. `A47` later says
"Substitution is the default, never a refusal" for a font the file does not carry, which bears on the
neighbouring case and could be read as wanting the option — or as being about embedding only.

## What the tree does meanwhile

A text-showing operator under a font name whose resource entry is null is refused and reported on the
page, as Q35 recommended for the default; nothing is guessed.

## Recommendation

**Owed, as a reader setting off by default**, in the place a host supplies (the shape `CLAUDE.md`
principle 3 gives a document's restrictions): the report stays on the page either way. It is one round
of about 60 tool calls, `pdf-model`'s option and one host's menu, with a test at each setting.
