# 0892 — A Type 3 font's `/Encoding` may be a name, and Table 110 is a requirement on the file

Session 926. Status: **accepted**.

## Context

The nine-hundred-and-twenty-sixth session walked `corpus-cache/tika-issue-tracker/batch5/pdfminer.six`
under `doc/todo/03`'s rule and ranked its first pages by ink. `pdfminer.six-56-0.pdf` came back at
**0.159 against `pdftoppm`'s 0.500 and `mutool draw`'s 0.522** — a bank statement whose two rules
this tree drew and whose every word it did not, with both references drawing the whole table. The
page reported

```
Font { detail: "font /T3_0 is a Type 3 font whose /Encoding names no glyph" }
Font { detail: "font /T3_1 is a Type 3 font whose /Encoding names no glyph" }
```

Both fonts state `/Encoding /WinAnsiEncoding`, and their `/CharProcs` dictionaries are keyed
`/A /B /C … /colon /comma /eight /five /four …` — Annex D's own spellings, so every code the page
shows has a description in the file.

## What the clause says, and where the reading went wrong

**Table 110's `/Encoding` cell is a requirement on the *file*:**

> ( Required ) An encoding dictionary whose Differences array shall specify the complete character
> encoding for this font (see 9.6.5, "Character encoding").

`crates/pdf-model/src/type3.rs` read that as the shape of the **lookup** as well, and returned an
empty table for an entry that is not a dictionary — which makes the font unusable and costs the page
every glyph. **§9.6.4's own algorithm says otherwise**, and it is the half addressed to a processor
rather than to a producer:

> a) Look up the character code in the font's Encoding entry, as described in 9.6.5, "Character
> encoding" to obtain a glyph name.

§9.6.5 is where the entry's permitted values are, in the General subclause that governs every simple
font:

> The value of the Encoding entry shall be either a named encoding (the name of one of the
> predefined encodings MacRomanEncoding , MacExpertEncoding , or WinAnsiEncoding ) or an encoding
> dictionary.

So a Type 3 font whose `/Encoding` is `/WinAnsiEncoding` still has its mapping, in §9.6.5.3's words,
"entirely defined by its Encoding entry" — Annex D defines it — and §9.6.4's step b) then looks each
name up in `/CharProcs` like any other. The file is malformed against Table 110 and the lookup is
nevertheless completely determined.

**This is a distinction the tree already draws one crate over.** `pdf_font`'s `base_encoding` reads
exactly this entry for Type 1 and TrueType fonts and takes the name arm first;
`PERMITTED_ENCODING_NAMES` is where the same four names are listed, with the same deliberate extra —
`StandardEncoding`, which §9.6.5.1 says has "no special meaning in PDF" while making it the base a
nonsymbolic font falls back to. The Type 3 path simply never grew the arm. Its own `/BaseEncoding`
handling has read a name through `BaseEncoding::by_name` since it was written, which is the *inner*
half of the same entry.

## Decision

**`encoding` takes the name arm before the dictionary arm.** An `/Encoding` that is one of the four
predefined names fills the table from Annex D and returns; an `/Encoding` dictionary behaves exactly
as before, `/BaseEncoding` under `/Differences`; anything else still yields an empty table and
`Type3Error::NoEncoding`, which is trap 5 — a mapping the standard does not define stays loud rather
than being invented.

Nothing else moves. §9.6.5.3's NOTE — "Type 3 fonts do not support the concept of a default glyph
name" — is why an *unencoded* code is left out of the table rather than written as `.notdef`, and
that is unchanged; so is step b)'s outcome for a name `/CharProcs` has no key for.

## What it moves, measured

`safedocs survey --dir` over **24 324 documents** — `doc/pdf.js/test/pdfs`,
`corpus-cache/tika-issue-tracker` whole, and the four `doc/corpora/` submodules — the same build
either side, verdict lines diffed. **Four documents change and nothing else does:**

| document | before | after |
|---|---|---|
| `batch5/pdfminer.six/pdfminer.six-56-0.pdf` | `Text { operations: 16 }` + two Type 3 refusals | complete |
| `batch1/PDFBOX/PDFBOX-4516-4.pdf` | `Text { operations: 12 }` + one | complete |
| `batch1/PDFBOX/PDFBOX-4863-3.pdf` | `Text { operations: 15 }` + two | complete |
| `batch1/PDFBOX/PDFBOX-4228-0.pdf` | `Text { operations: 503 }` + four | one report left, a different population |

The ink says the same thing, ours flattened on white against `pdftoppm -cropbox` and
`mutool draw -b CropBox` at 72 dpi:

| document | ours before | ours after | poppler | mupdf |
|---|---|---|---|---|
| `pdfminer.six-56-0.pdf` | 0.1595 | **0.5292** | 0.5002 | 0.5223 |
| `PDFBOX-4516-4.pdf` | — | **1.6729** | 1.6670 | 1.6452 |
| `PDFBOX-4863-3.pdf` | — | **0.8757** | 0.9228 | 0.8607 |
| `PDFBOX-4228-0.pdf` | — | **12.3234** | 13.1647 | 11.9202 |

Two of the four land inside the interval the references bracket and two are 0.006 and 0.007 above
its top, which is under an eighth of this directory's largest anti-aliasing departure. The pages
were read rather than the numbers (trap 1): `pdfminer.six-56-0.pdf`'s table and `PDFBOX-4228-0.pdf`'s
newspaper page are line for line what `pdftoppm` draws.

**And the crawl states it nowhere.** The same before/after over `CC-MAIN-2021-31`'s **65 944**
documents — `corpus-cache/safedocs`, whole — produces a verdict diff of **zero lines**, and not one
of those 65 944 carries the Type 3 encoding report in either direction. That is session 908's
finding arriving again from the same direction (`doc/todo/03` §47): a document served by a web
server is a document that worked, and an issue-tracker attachment is a document that broke a
program. A corpus of the first kind cannot rank this defect, and it is worth saying so with the
number rather than leaving the population unstated.

**Six documents of the 24 324 carried this report and two still do**, and those two are the
condition the clause really does leave nowhere to go: both state `/Encoding <ref>` into an object
their damaged files do not define, so there is no dictionary *and* no name, and §9.6.5.3's NOTE
denies a default. They are `GHOSTSCRIPT-699675-0.pdf` and `poppler-101548-0.zip-3.pdf`.

## Consequences

- **All four documents gain a row in `doc/checks/fixed-documents.toml`**, which is `doc/todo/03`
  §20's rule: no gate walks them, so the fix is measured once by the round that makes it and held
  by that file afterwards. Their numbers are in **the check's own units** — its "mean of `255 - luma`
  at scale 1.0", roughly twice the flattened-and-negated figure the ink ranking above prints — and
  two of the four take a band narrower than the file's usual ±1.0 and say why in the row, because
  the marks they stand against are worth 0.739 and 0.080 of a level.
- `doc/conformance/ledger.toml`'s §9.6.4 and §9.6.5.3 rows say which half of Table 110's cell is a
  requirement on the file and which is not.
- Three tests in `crates/pdf-model/tests/type3.rs` hold the three arms: a named encoding maps codes
  out of Annex D, a named encoding's names that `/CharProcs` has no key for paint nothing and leave
  the page complete, and a name that is not a predefined encoding is still no encoding.
- **The general shape is worth more than the instance**: a table cell that says *shall be an X*
  constrains the producer, and the clause that says what a *processor* does with that entry is
  somewhere else — here one subclause away, and §9.6.4 cites it by number. Reading the first as the
  second turns a malformed file into an undrawable one. **The check is cheap and is what this ADR
  asks a later round to do**: where a refusal is justified by a Table cell, find the clause that
  states the algorithm and see whether it delegates. ADR 0779 is the neighbouring entry — `/Font`
  naming an object the file does not define — and there the delegation runs out: §7.3.10 states an
  outcome (the null object) rather than an alternative, so the refusal stands.
