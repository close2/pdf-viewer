# 0286 — The expert set, and the punctuation a fallback would have got right

**Status.** Accepted.
**Context.** `D.4` was one of the ledger's eighteen `reported` rows: `MacExpertEncoding` is a name
Table 109 and Table 112 permit, this crate had no table for it, and a font naming it was refused
by name — so its page drew no text at all.

## What was done

Table D.4's 165 assignments are transcribed as `MAC_EXPERT`, beside `WIN_ANSI` and `MAC_ROMAN`,
and `BaseEncoding::MacExpert` resolves them. That is the whole change; the refusal branch it
retires is `FontError::UnsupportedEncoding`, which now has no member and is kept, with a test
asserting that the two lists — the names the table permits and the names this crate has a table
for — give the same four answers. A later edition adding a fifth permitted name fails there rather
than starting to refuse fonts quietly.

## The transcription was cross-checked twice, and that is not a formality

`doc/HANDOVER.md` records that `doc/md/`'s conversion **drops content** and that a *table* is where
to expect it — Table 164's `/Di` row is the standing example. So the extraction from the markdown
was compared against `pdftotext -layout` over `doc/ISO_32000-2_sponsored_EC3.pdf`: **165 entries
each, no code in one and not the other, no name differing.** Clean, this time, and cheap to know.

Three hazards in the table itself are what the transcription tests pin, because each would produce
a plausible wrong answer: it is printed in **two column-pairs per row**, so reading left to right
interleaves two alphabets; its codes are **octal**, so `276` is 190; and it assigns **165 of 256**
codes, so the sparseness is the table rather than a gap in the reading.

## What the transcription measured, which is the finding

The row's stated reason for refusing was that substituting Latin glyph names "would map every code
to the wrong glyph in silence". With both tables in hand that is checkable, and it is **not quite
true — it is worse**:

> **Six codes mean the same thing in the expert set and in `WinAnsiEncoding`, and all six are
> punctuation**: space, comma, hyphen, period, colon, semicolon. Every other assignment differs.

A Latin fallback would therefore have drawn a document's punctuation *correctly* and every letter,
ligature, fraction and small capital as something else. That is trap 1's shape exactly — a page
that looks like a page, with the commas in the right places — and it is a stronger argument for the
refusal than the one the row gave. It is now a test, because the count is a fact about two tables
that a later edition could change.

## What it does not buy

Nothing in the corpus names the encoding: **0 of 974**. So no gate moves, and this is a purely
spec-driven round — `CLAUDE.md`'s second track, which exists precisely because a corpus cannot rank
a requirement no document exercises. What changes is that a document naming `MacExpertEncoding`
draws instead of drawing nothing.

And the annex's own caveat is recorded beside the table rather than acted on: "[t]he built-in
encoding in an expert font program can be different from `MacExpertEncoding`". Which encoding wins
is §9.6.5.1's question and is answered there.
