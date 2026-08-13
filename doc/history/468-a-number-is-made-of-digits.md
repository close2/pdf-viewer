# 468 — A number is made of digits

**Finding.** §7.3.3 writes both numeric forms as "one or more decimal digits", so a run of regular
characters holding none — `.`, `-`, `+`, `-.` — is not a numeric object at all. This lexer read
every one of them as `Integer(0)`, on a comment saying that is "what other viewers do", which is
principle 5's forbidden direction written down and unchallenged for four hundred and sixty-seven
sessions. The witness is the second of the two defects ADR 0302 diagnosed and left:
`T02-05-01_006_font-size-operator-missing.pdf` writes `/F0 . Tf`, so this reader set a text font
size of **nought**, drew invisible text and said nothing. Such a run is now the `Keyword` it
lexically is, which every consumer downstream already knew what to do with — the parser refuses a
keyword where an object belongs, and the interpreter reports one it does not recognise (§7.8.2),
after which the `Tf` states no size and §9.3.1's "[t]here is no initial value for either font or
size" makes the show a reported loss.

**Date.** 2026-08-13.
**ADR.** [0303](../adr/0303-a-number-is-made-of-digits.md).
**Touched.** `crates/pdf-syntax/src/lexer.rs` (`read_number`, five lines and a clause),
`crates/pdf-syntax/src/parser.rs` (one test), `crates/pdf-model/tests/numbers_without_digits.rs`
(new), `doc/conformance/ledger.toml` (§7.3.3, §7.8.2, §9.3.1), `doc/oracle-and-corpus.md` (§2's
table and §2b), `doc/todo/03-more-corpora.md` §7, `doc/adr/0303-*`, this file.

## The population, before believing the condition

Trap 11, and it is why this round is mostly measurement. `tools/safedocs survey --dir` ran over
every corpus on this disk on both sides of the change, one process per SafeDocs archive:

| population | documents | incomplete before → after | reports changed |
|---|---|---|---|
| pdf.js corpus | 974 | 65 → 67 | 3 |
| `doc/corpora/` | 108 | 9 → 9 | 0 |
| `openpreserve/format-corpus` | 267 | 22 → 23 | 2 |
| SafeDocs `CC-MAIN-2021-31`, all 145 archives | 65 944 | 823 → 823 | 7 |

**Twelve documents of 67 293, and nine of the twelve were already reporting something else** — the
seven crawled ones report between one and eighteen other malformed operators apiece, and on those
the change adds a name to a list. That is §7.3.3 predicting its own reach: no producer writes `.`
where a number belongs, so this is a defect of *damaged* files.

## What it moves

`examples/display_list_digest` on both sides, in one sitting: **1** of the 974, 0 of the 108, 0 of
the 267, **2** of 10 000 SafeDocs documents — the four whole archives ADR 0302 used plus the six
that changed a report. The two crawled ones are already-broken files whose pages do not move:
`0300856.pdf` loses one command of 474 and rasterises **pixel-identically**, and `4605705.pdf`
cannot be rasterised on either side because a `cm` in it is singular by twenty-two orders of
magnitude.

The pdf.js one was opened (trap 1), and it is the honest cost of the change. `issue9252.pdf` is sharpPDF's
output and its content stream says `. .59 .84 rg`, meaning `0 .59 .84` and not saying so. The word
*Test* was teal and is now black, because `rg` is left with two operands and is refused rather than
half-applied. **A guess that happens to be right is still a guess**, and the page now carries a
report naming the token instead of a colour nobody wrote. The other two pdf.js documents are
pixel-identical at `magick compare -metric AE 0`: `bug1953099.pdf`'s `-` sits inside a `TJ` array
where it was a kerning adjustment of zero either way, and `issue5039.pdf`'s `-inf` sat beside an
`inf` that was already an unrecognised operator, in front of a `d1` already refused for want of
operands.

## What was left

The other half of `doc/todo/03` §7 — a page tree node with no `/Kids`, which §7.7.3.2 Table 30
makes malformed and which this tree turns into a blank leaf in silence. It is a different clause in
a different crate and it wants a population nobody has counted. The hand-built corpus is down to
**four** files blank in silence, two of them rightly so.
