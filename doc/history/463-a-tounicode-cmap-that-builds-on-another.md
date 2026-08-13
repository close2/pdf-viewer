# 463 — A `/ToUnicode` `CMap` that builds on another

**Finding.** The text gate's shortfall was asked for as a *population*, and listing every document
under 100% rather than under the floor made it small enough to read in one sitting: **27 documents
and 184 words of 24 191**, twenty-three of them already named in the gate's own constant. Of the
four that had never been looked at, three are the reference doing layout analysis or misreading an
encoding and one is §9.10.2 exhausted on a dvips Type 3 font. **The defect was not in that band at
all** — it was in a document reading back the empty string. `issue5010.pdf`'s `/ToUnicode` states
five mappings for codes its page never shows and `/Adobe-Korea1-UCS2 usecmap` for the rest, which
§9.10.3 permits — "UseCMap , which may be used if the CMap is based on another ToUnicode CMap" —
and which nothing in this tree followed: **§9.10.3's ledger row said "Table 118's `/UseCMap` is
read" and that was true of `read_cmap`, a different function, reading a different entry of a
different `CMap`.** The page now reads `인터뷰●홍인혜카피라이터`, derived from Adobe's own published
`Adobe-Korea1-UCS2` rather than matched against anybody. §9.7.5.4 a) is what licenses following the
in-file operator where the dictionary is silent: it requires the two to agree, so on a conforming
file they say the same thing. The pdf.js gate went **24 007 → 24 012 words and 23 → 22 named
documents**; every other gate is line for line what it was.

**Date.** 2026-08-13.
**ADR.** [0298](../adr/0298-a-tounicode-cmap-that-builds-on-another.md).
**Touched.** `crates/pdf-font/src/tounicode.rs` (`base`, `parse_on`, three lookups, two tests),
`crates/pdf-font/src/predefined.rs` (`unicode_cmap`, the memo's key, `used_by` shared),
`crates/pdf-font/src/lib.rs` (`read_to_unicode`, `read_cmap`'s `Object::Null` arm),
`crates/pdf-model/tests/composite_fonts.rs` (one test),
`crates/pdf-model/tests/text_extraction.rs` (`without_hyphens`, the named list and three stale
paragraphs of its doc comment), `doc/conformance/ledger.toml` (§9.7.5.3, §9.7.5.4, §9.10.3),
`doc/adr/0298-*`, this file.

## How the population was listed, and why it is worth doing again

The gate prints a line per document *below the floor* and one summary line. Everything between the
floor and a hundred percent had therefore never been named — which is what "a shortfall nobody has
characterised" meant. One `println!` behind `score.ratio() < 1.0`, one run, and it is 27 lines:

| | |
|---|---|
| documents in the pdf.js corpus | 974 |
| skipped — unopenable, no page one, or `pdftotext` refused | 24 |
| not gated because this tree *reports* something | 62 |
| gated | 888 |
| **short of 100%** | **27** |
| of those, below the 0.90 floor and already named | 23 |
| words missing in total | 184 of 24 191 |

The instrument cost nothing and was removed again. It is worth writing down that the answer was
**small**: the phrase "a little under a hundred percent" sounds like a long tail and it is
seventeen words outside a list that already existed.

## The four that had never been read

- **`bug1997343.pdf`, 8 words.** Four are §14.9.4's `/ActualText`: a tagged LaTeX document whose
  logo spans carry `/ActualText <FEFF004C0061005400650058>` — `LaTeX` — while the glyphs spell
  `LATEX`. This tree reads the entry off the structure element and `pdftotext` does not, so the
  reference's word is the one the standard says to replace. Four more are §14.8.2.3 soft hyphens,
  which the gate was scoring as missing; see below.
- **`issue918.pdf`, 7 words.** dvips Type 3 fonts naming every glyph `/aNN` after its own code.
  §9.10.2's closing permission answers the printable ASCII range and cannot answer `á` or the `fi`
  ligature, which sit at OT1 codes below 0x21. `pdftotext` answers those with U+001C and U+001E,
  which are not characters, so neither reader has the answer and the clause says as much.
- **`issue20489.pdf`, 1 word.** `Date>SCALE` is two form labels this tree draws forty lines apart
  and `pdftotext`'s column analysis ran into one token.
- **`issue1350.pdf`, 1 word.** The reference reads `beginnerÕs`; 0xD5 is `quoteright` in
  MacRomanEncoding and this tree reads `beginner’s`.

Three of the four are the reference. That is not a complaint about poppler — it is the reason
`doc/corpora/pdfbox`'s frozen extraction exists as a second instrument, and it is principle 5's
direction of inference working normally.

## The gate's hyphen rule, and why changing it is not fitting the instrument to the population

`without_hyphens` strips U+002D from both sides because "nothing in the content stream says which
hyphens those are". §14.8.2.3 is the clause it already cites for that, and the clause's actual
subject is the opposite case: a producer that *does* say so, by writing U+00AD. `bug1997343.pdf` is
one. The readback is unchanged and correct — the reader's half of that clause is to deliver the
character and leave the rejoining to a consumer, which §14.8.2.3's ledger row has said for a long
time — and what moved is the comparison, symmetrically, under the clause the function was already
appealing to.

## What was deliberately not done

- **A base `CMap` this binary does not carry answers nothing rather than reporting.** A
  `FontError` would take the page out of the oracle's judged set for a shortfall §9.10.2's later
  methods and closing permission are built to absorb (trap 11).
- **`french_diacritics.pdf` stays refused**, and it is the sharpest of the remaining twenty-two:
  a pdfTeX Type 3 font whose `/Differences` names `/a192`, `/a224` … — the code in decimal — for
  the Latin-1 accented letters. `pdftotext` reads the code as the character and gets all
  twenty-seven; doing the same would mean extending `text_from_the_code` past 0x7E, where
  §9.6.5's encodings stop agreeing with Latin-1, and the name `a192` is a producer's convention
  rather than anything the standard states. The refusal is the honest answer and the picture is
  right either way.
- **`Type3WordSpacing.pdf` was diagnosed and is not a glyph question.** Its four missing words
  are `pdftotext`'s column analysis on six lines of ` ab ba abba` at six different `Tw` values.
  What the diagnosis *did* turn up, and what is worth a later round rather than this one, is that
  `separate_text` cannot see a gap inside a show string at all — `text_cursor` is set to the
  post-advance matrix and read again at the next code, so the two are equal by construction and
  only a `Td`, `T*` or `Tm` between show operations ever produces an inferred separator. On this
  file the page shows a five-em gap between words and the readback is `abbaabba`.
