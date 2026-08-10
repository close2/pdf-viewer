# ADR 0259 — A frozen opinion, and the permission it found declined

Date: 2026-08-10 (session 423)
Status: accepted

## Context

`tests/text_extraction.rs` has measured this tree against `pdftotext` since the sixty-third
session: 974 documents, one reference, run at gate time. `CLAUDE.md`'s principle 5 says what
that measurement is worth — agreement raises confidence that the clause was read correctly,
disagreement is a question to take back to the standard — and it says nothing about how many
references there should be. One is what there was.

Session 422 added `doc/corpora/pdfbox` and noted, in `doc/todo/03` item 4, that the repository
carries `*.pdf.txt` and `*.pdf-sorted.txt` beside 40 of its PDFs: **Apache PDFBox's own
`PDFTextStripper` output, checked in as a fixture**. That is a different instrument from
`pdftotext` in a way worth naming. It is *frozen* — pinned by the submodule's commit, so it
cannot drift under this tree the way a distribution's poppler can — and it was written by people
who read §9.10.2 independently and made their own choices about it.

It also mattered more than usual this session. Session 421 built `pdf-retrieve` on the premise
that this tree's extraction can be trusted, and the owner's condition for that was "there should
of course a test, which extracts some text". A second, independent, frozen opinion is that
condition strengthened.

## Decision 1 — the gate, and where it was put

`the_text_we_draw_agrees_with_pdfboxs_frozen_extraction`, in the same file and reusing the same
machinery: `fold`, `without_hyphens`, `reference_words` and `without_spaces`, so the two
references are scored by one rule and a difference between them is a difference about the
documents rather than about the comparison. The floor is the same 0.90 and the below-floor list
is named and checked in both directions, exactly as `TEXT_BELOW_FLOOR` is.

Three things about it differ from the pdf.js gate, each for a stated reason:

- **The whole document, not page one.** `PDFTextStripper` walks every page unless told
  otherwise, and `cweb.pdf` has 28 of them. Scoring 28 pages of expectation against one page of
  readback would have measured the page count.
- **The population is 40, not 64.** Only 40 of the checked-out PDFs carry a `.pdf.txt`; the rest
  are there for rendering, merging and compression tests, and a document with no expected text is
  not this instrument's business.
- **Both of PDFBox's texts are read and only one gates.** `-sorted.txt` is the same extraction
  with `setSortByPosition(true)`, so printing both separates the two questions a shortfall
  confuses: where they agree, reading order is not what is at stake. On these 40 they agree
  exactly — 14257 of 14281 either way — which is itself the answer to the question the `-sorted`
  variant was expected to raise.

**Where it was put, and what it costs.** `doc/todo/02` §2 line 28 is
`cargo test --profile gates -p pdf-model --test text_extraction -- --ignored --nocapture`, which
runs *every* ignored test in that binary — so the new gate is in the default sequence already,
with **no new line**, on a binary that line was building anyway. Alone it is **0.4 s** for 40
documents, because its reference is a file rather than a process: the pdf.js gate spends 30 s of
its 31 waiting for 974 `pdftotext` invocations. That is the whole argument for it having earned
its place rather than taken one.

## Decision 2 — the defect it found, which is a permission being declined

The first run: **40 documents, 99.8% (14254/14281 words), 5 below the floor.** Each of the five
was read before anything was ratcheted, which is the rule `doc/todo/03` states for a new
population. **Three of them turned out to be one defect.**

`pdf_font::LoadedFont::text_from_program` — §9.10.2's last resort, the choice the clause permits
where its three methods fail — began with:

```rust
if !matches!(self.mapping, CodeMapping::Named(_)) {
    return false;
}
```

on a note saying "a composite one selects by CID through a `CMap`, and §9.10.2's third method is
the route the clause states for those". That reads the third method as applying to every
composite font. **It does not, and the clause says so in its own first line:**

> If the font is a composite font that uses one of the predefined CMaps listed in "Table 116
> -Predefined CJK CMap names" (except Identity -H and Identity -V ) or whose descendant CIDFont
> uses the Adobe-GB1, Adobe-CNS1, Adobe-Japan1, Adobe-Korea1 (deprecated in PDF 2.0 (2020)) or
> Adobe-KR (added in PDF 2.0 (2020)) character collection

An `Identity-H` font whose descendant is `Adobe-Identity` is excluded **by name** from the third
method and cannot use the second, which is for simple fonts. So its `/ToUnicode` is the only
method it has, and where that answers nothing the clause's own next sentence is in force:

> If these methods fail to produce a Unicode value, there is no way to determine what the
> character code represents in which case a PDF processor may choose a character code of their
> choosing.

The refusal declined a permission whose precondition the file had met. The route is the same data
one step longer than the simple-font case: the `CMap` gives a CID, §9.7.4.2's `/CIDToGIDMap` gives
the glyph, and the program's `post` table or inverted `cmap` names it. §9.7.6.3's notdef fallbacks
are deliberately *not* taken — a code that reached CID 0 drew a substitute, and naming what the
substitute is called would put a character on a page that shows none.

`program_by_glyph` replaces `post_by_code` and is keyed by glyph index rather than by character
code, which is what lets one table serve both routes into it. Both of the program's statements are
now read at once where the `cmap` inversion used to be deferred: a subset embedded for a composite
font is normally `post` version 3.0, which holds no names at all, so the second source is needed
for essentially every glyph and deferring it buys a branch rather than a table walk.

**Measured rather than assumed**, which is this clause's standing rule in this tree after the
sixty-fourth session measured the same permission and the three-hundred-and-twenty-eighth
measured it again:

| | before | after |
|---|---|---|
| pdf.js corpus, words matched | 23987 / 24187 | **24003 / 24187** |
| documents below the 0.90 floor | 25 | **23** |
| documents that moved the other way | — | **0** |

The two are `issue16553.pdf` and `javauninstall-7r.pdf`, and neither was worked on. Both now read
back exactly what `pdftotext` reads — an Okular signature appearance in Czech, and a page of
Japanese. `issue16553.pdf` had sat on that gate's named list for **357 sessions**, under the
heading "partial for reasons nobody has diagnosed further". **An entry parked as undiagnosed is
not an entry that cannot be diagnosed**, and what moved this one was a second population rather
than a second look.

The sharpest witness is the one that found it: `PDFBOX-5838-0024320-reduced.pdf`, whose
`/ToUnicode` maps 8 of the 15 codes its page shows. It read back `H Reeach Pec` for
`Honors Research Project` — with nothing reported, because nothing was missing except the names.

## Decision 3 — where this tree and PDFBox part, and it is a choice

Two of the four documents left below the floor are the same font shape and are **not** a defect.

`PDFBOX-4322-Empty-ToUnicode-reduced.pdf` shows `<004a0075007300740069006e>` in an `Identity-H`
Calibri subset whose `/ToUnicode` stream is a verbatim copy of the `Identity-H` CID `CMap`:
`/CMapType 1`, one `begincidrange`, and not one of the operators §9.10.3 requires —

> It shall use the beginbfchar , endbfchar , beginbfrange , and endbfrange operators to define
> the mapping from character codes to Unicode character sequences expressed in UTF-16BE encoding.

`sample_fonts_solidconvertor.pdf` does the same thing more briefly, writing the bare name
`/Identity-H` for two of its ten fonts, and loses two whole lines of its page. Both embedded
programs then say nothing either: **neither subset carries a `cmap` table or a `post` one**,
checked by reading their table directories rather than assumed. So every route the standard
describes has failed and the permission is reached with nothing to choose from.

PDFBox chooses the code itself, read as a Unicode value; its own source calls that "the
undocumented case". It is right on both of these files because their producers numbered the CIDs
by code point, and it is mojibake on any file that did not. **This tree declines**, and the
declining is now written down rather than left in the shape of a passing gate:
`text_from_the_code` takes that step for a **one-byte** code, where §9.6.5's encodings make a byte
and a code point the same character, and a guard added this session refuses it for anything wider.
The guard costs nothing today and would have been wrong the moment a composite font could reach
that function — which it now can, and `<004a>` would have read back `J` from arithmetic rather
than from a font.

Principle 5 is what decides this, and it decides it against the easier answer: agreement with
PDFBox would have been one line, and there is no clause under it.

## Decision 4 — the other two, which are conventions

`hello3.pdf` and `FC60_Times.pdf` are right-to-left text. This tree returns the letters in their
Arabic Presentation Forms-B contextual shapes in painting order — U+FEE3 U+FEA4 U+FEE4 U+FEAA for
`hello3.pdf` — where PDFBox returns U+0645 U+062D U+0645 U+062F in logical order. Three
conventions differ and none is about which glyph was drawn:

- **Order.** §14.8.2.5.1 puts this tree on its own side of the line: page content order "shall be
  defined by the sequencing of graphics objects within a page's content stream", and
  `Interpretation::text` is that by construction. What PDFBox adds is the Unicode bidirectional
  algorithm, which is layout analysis and is the same thing `pdftotext` does for the seven
  right-to-left documents already on the pdf.js gate's list.
- **§14.8.2.5.3 is the mechanism that would settle it, and neither file writes one.**
  `/ReversedChars` has been obeyed here since the eighty-third session. Measured this session over
  all 108 documents of the three new submodules, with every `FlateDecode` stream inflated:
  **not one writes the tag**, which extends the ledger row's existing measurement over the 953
  pdf.js first pages rather than contradicting it.
- **Presentation form against base letter** is `fold`'s Latin-ligature argument in another script,
  and it is deliberately *not* folded: Arabic Presentation Forms-B is 141 code points against
  `fold`'s nine Latin ones, and folding a block this instrument has two witnesses for would be
  fitting the instrument to the population.

## The SafeDocs chunk

`safedocs fetch --archive 3500 --count 24 --download` — somewhere nobody had been, the manifest
having recorded only archive `0000`. **16.8 MiB transferred, inflating to 19.8 MiB, 24 documents,
every CRC-32 matched.** The survey line, which is a baseline for that chunk and not a ratchet:

> 24 documents: 0 unopenable, 0 locked, 0 encrypted beyond us, 0 pageless, **2 incomplete**, 0 slow

and read against the populations they belong to, **both reports are already named**:
`3500006.pdf`'s page group composites in `/DeviceCMYK` (§11.4.7, ADR 0251's population) and
`3500011.pdf` names three `/Font` resources and one `/ExtGState` its resource dictionary does not
define (§7.8.3, ADR 0255's). One document shows 2 codes that reach no glyph while reporting
nothing, which is `doc/todo/21`'s measurement. **Nothing was promoted**, which is the rule
working rather than the round being empty: a second witness for a population already named buys
nothing, and the running total against the 20 MB budget stays at **0 MB**.

## Consequences

- Text extraction has two references and one of them cannot move. 40 documents, 99.8%, four named
  differences, all four read.
- §9.10.2's permission reaches every font the clause sends to it, and §9.10.2's row records the
  measurement rather than the intention.
- `doc/todo/02` §2 gains an instrument and no line, at 0.4 s.
- The gates: tests 1539 → **1542** and 10 skipped → **11**, citations 6203 → **6223**, quotations
  583 → **584**, the text gate 23987/24187 with 25 named → **24003/24187 with 23**. Every other
  corpus-scale count is identical — the corpus's 70 incomplete, the oracle's 905/68/786 over
  1688/106, quorra's 912/36/9/17 — which is what "drawing cannot see this" looks like from the
  outside. From the inside it is provable: `text_from_program`'s two other production callers,
  `substitutes_notdef` and `codes_by_character`, both return early for anything but a simple font,
  and its simple-font branch reads the same two sources in the same order it always did. So
  `doc/todo/00`'s step 7 is not owed.
