# 464 — The clause that names the space

**Finding.** Session 463 left the finding that `separate_text` cannot see a gap inside a show
string. It cannot, and §9.4.4 says why that is correct rather than missing: between two codes of
one string the `TJ` term of `tx = ((w0 − Tj/1000) × Tfs + Tc + Tw) × Th` is absent, so what is
left is the glyph's own advance, `Tc` — which applies to every pair alike and is tracking — and
`Tw`, which §9.3.3 applies to the single-byte code 32 alone. **The only word gap a show string can
state is that code, and it is identified by its encoding rather than by a distance.** §9.3.3 also
*names* it: "Word spacing works the same way as character spacing but shall apply only to the
ASCII SPACE character (20h)", of "every occurrence of the single-byte character code 32 in a
string". So `Font::text` reads such a code back as U+0020 where every one of §9.10.2's methods and
its closing permission have declined — last, never first, so a `/Differences` naming code 32
`/bullet` is still believed. Five corpus documents show one; `issue4304.pdf` is 895 bytes named
*Words that should have spaces between them*, whose picture the four-hundred-and-fifth session
fixed and which went on reading back `Wordsthatshouldhavespacesbetweenthem.` for fifty-nine
sessions after. **No gate moved, and both text gates are built not to see this**: they strip
whitespace from the comparison by design. Selection, search, `pdf-retrieve` and the screen reader
are what changed.

**Date.** 2026-08-13.
**ADR.** [0299](../adr/0299-the-clause-that-names-the-space-and-the-gap-a-show-string-cannot-state.md).
**Touched.** `crates/pdf-model/src/content.rs` (`Font::text`, `show_text`'s call site,
`separate_text`'s doc comment), `crates/pdf-model/tests/text_state.rs` (two fixture fonts, one
`CMap`, three tests), `crates/viewer-core/src/select.rs` (one claim that stopped being true),
`crates/pdf-font/src/{predefined,tounicode}.rs` and `crates/pdf-model/tests/text_extraction.rs`
(the clippy warnings below), `doc/conformance/ledger.toml` (§9.3.3, §9.4.4, §9.10.2, §14.8.2.6.2),
`doc/habits.md`, `doc/adr/0299-*`, this file.

## The numbers

Every gate ran before and after and not one moved:

| gate | before | after |
|---|---|---|
| pdf.js text | 24 012/24 191 words, 22 named | identical |
| PDFBox frozen text | 14 257/14 281, 4 below floor | identical |
| corpus | 65 incomplete, 5 codes with no glyph over 2 documents, 57 blank over 9 | identical |
| oracle | 905 agrees, 68 contradicted, 786 ambiguous, 18 no render | identical |
| quorra | 919 agree, 37 differ, 1 refused | identical |

The instrument that *did* move is `examples/readback`, and that is the round's point rather than
its consolation. `doc/habits.md` carries the habit it earned.

## Two things found on the way, neither of them the work

- **The clippy gate was not clean at `937332e`.** Seven `clippy::doc_markdown` warnings in
  `pdf-font/src/{predefined,tounicode}.rs` and `pdf-model/tests/text_extraction.rs`, all from the
  previous round, all on `CMap`/`ToUnicode`/`UseCMap`/`MacRomanEncoding` inside quotations of the
  standard. Fixed the way this tree already writes them — backticks inside the quote, which is
  what §9.7.6.3's blockquote in `pdf-font/src/lib.rs` has always done. `doc/todo/02` §2 says the
  lint run "must be silent of lints" and CI makes warnings errors, so this was a broken gate
  rather than a nit.
- **A code that draws no glyph and reads back *nothing* is counted as a space.** `show_text`'s
  classification is `self.text[start..].chars().all(char::is_whitespace)`, and an empty slice
  satisfies that vacuously — which is why this change moved neither the "codes reaching no glyph"
  tally nor the blank-glyph one. It is not obviously wrong and it is certainly not measured; ADR
  0299's last section states it, and it wants its own population count before anything is done
  to it.
