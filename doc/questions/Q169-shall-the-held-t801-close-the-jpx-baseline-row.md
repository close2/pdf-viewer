# Q169 — Shall the held T.801 close the JPX-baseline row?

Source: this round, asking the question A51 said would be "asked then", at the owner's instruction of
2026-09-28. Asked and answered together; the answer is `A169` beside this file.

## The question A51 left, now asked

A51 bought ISO/IEC 15444-1, declined part 2, and made the remainder a condition of its own: "Part 2
is bought only if we ever want a conforming-validator claim with no gaps — and that is a new
question, asked then." What changed beneath that sentence since it was written: **the text is
held.** `doc/T-REC-T.801-200208.pdf` is ITU-T T.801 (08/2002) — the identical joint text of
ISO/IEC 15444-2 with Amendment 1 and Corrigenda 1 and 2 integrated, fetched from the ITU's
free-of-charge catalogue at the owner's request on 2026-09-22 — and it carries M.9.2 "Support for
JPX feature set", the subclause that defines the JPX baseline the one `Unchecked` PDF/A row needs.
A51's "Part 2, ISO/IEC 15444-2, is not in doc/" was true when written and is no longer.

So the question is not whether to buy anything. It is whether the row is wanted: shall ISO 19005-2
section 6.2.8.3's and ISO 19005-4 section 6.2.7.3's rule — "only the JPX baseline set of features is used" — be
implemented by reading the held text, closing the gapless conforming-validator claim's one known
hole?

## Why it needs the owner

A51's remainder is the owner's own condition, and the answer is a coverage decision the tree's
two-questions rule names: the build buys ledger coverage, and its cost is a reading round plus
whatever M.9.2 turns out to require of a validator. Nothing blocks on it — the reader owes nothing
here (ADR 1184 ruled checking the restriction is not a reader's job), and the row is `Unchecked`
with a truthful reason today.

## What the tree does meanwhile

The row stays `Unchecked` with its reason, and the reason is amended to say the text is held — the
half of it that still reads "not bought" is no longer true.

## Recommendation

**Yes — read the held text and close the row.** The text costs nothing now, A46 still owes "finish
and certify PDF/A-4", and the answer converts the last unheld-specification floor on the validator
side into work. The reading round's first acts: `python3 tools/spec-md.py
doc/T-REC-T.801-200208.pdf` under the ignored `doc/md/`, a `doc/third-party-data.md` entry for the
ITU free-of-charge route (ADR 0187's discipline: cite the section and paraphrase, never quote), and
M.9.2 read alone — with the 2004 second edition as the revisit condition: the held text is the 2002
one, and if the editions are ever found to differ on the baseline set, the row's reading is
revisited then.