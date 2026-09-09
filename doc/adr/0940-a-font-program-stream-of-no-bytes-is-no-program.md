# ADR 0940 — A font program stream of no bytes is no program, and Table 120 says why

Status: accepted. Session 943.
Clauses: ISO 32000-2 §9.8.1 Table 120 (`/FontFile`, `/FontFile2`, `/FontFile3`, and the sentence
that says what the rest of the descriptor is for), §9.9 Table 124 (what each subtype's program
shall conform to), §9.6.2.1 (the `/BaseFont` a substitute is chosen against).
Code: `crates/pdf-font/src/program.rs` (`no_program`, and the two doors that ask it).
Tests: `crates/pdf-model/tests/silent_fonts.rs`'s
`an_empty_font_program_stream_is_substituted_for_rather_than_refused` and
`the_witness_states_a_font_program_stream_of_no_bytes`.
Measurement: `crates/pdf-model/examples/empty_font_program_census.rs`.
Documents: §9.8.1's and §9.9's ledger rows, `doc/todo/00` step 7's record of the sweep that found
it.

## Context

`doc/todo/00` step 7 is our ink minus the lightest live reference's over every page the oracle
prints as `ambiguous`, and it exists for the one defect a distance ranking cannot see: a page that
draws less than everybody need not be far from anybody. Re-run whole in the
nine-hundred-and-forty-third session over all 836 ambiguous pages, its negative tail reproduced
the eight-hundred-and-sixth session's to the thousandth — nineteen names at or past −1, sixteen of
them documents this tree reports on. The alarm held.

**What had not been read is the sixteen.** ADR 0433 opened the head of that list for exactly this
reason — `[incomplete]` reads as *explained* and is not the same as *diagnosed* — and named eleven
of them as one cause, §9.7.5.2's `Identity-H` over a non-embedded font. Five were left. Two of
those five are ADR 0836's check-value refusal (`bug1050040.pdf` −11.272, `issue13316_reduced.pdf`
−3.030), one is a widget with neither an `/AP` nor Table 192's `/CA`
(`checkbox_no_appearance.pdf` −1.200), and two were named nowhere in this tree at all. The lower
of those two is `issue5954.pdf` at −7.367 and it is **not** a defect: its page states a
`/Resources` of its own holding no `/Font`, its `/Pages` node states one that defines `/F1`, and
§7.7.3.4 stops the search at the first — so the `Tf` names nothing, this tree draws nothing and
says so, and `poppler` reaches the same reading and prints `Unknown font tag 'F1'`. That reading
is recorded in `doc/todo/00` rather than here.

The higher of the two is **`bug866395.pdf` page 1 at −8.549 of 255**: 200 × 50 points, one line
reading `l’impayé`, **ours 0.000** against `ghostscript` 8.549, `hayro` 11.502, `poppler` 11.780
and `mupdf` 11.801. Its report:

```
Font { detail: "font /F1 could not be parsed: An offset was out of bounds" }
```

## What the file states

The font is `/Subtype /Type1`, `/BaseFont /SSHIDR+Optima-Bold`, `/Encoding /WinAnsiEncoding`, with
`/FirstChar 97`, `/LastChar 233` and a full `/Widths`. Its descriptor states `/FontFamily
(Optima)`, `/FontWeight 700`, `/Flags 32`, `/ItalicAngle 0`, a `/FontBBox`, a `/StemV`, a
`/CapHeight` and an `/XHeight` — and a `/FontFile3` of `/Subtype /Type1C` whose stream is

```
/Length 10  /Filter /FlateDecode
```

Ten bytes of zlib that reach RFC 1951's final block, satisfy RFC 1950's Adler-32, and produce
**zero bytes**. No damage, no prefix, no shortfall against an extent: an empty font program.

Read as a program, an empty byte string is handed to the sfnt reader — `is_bare_cff` needs four
bytes to say otherwise — which answers *An offset was out of bounds*. That is a
`FontError::Malformed`, and `LoadedFont::load_simple` substitutes for `NotEmbedded` and
`UnsupportedProgram` while refusing `Malformed`. So the page drew nothing.

## What the standard says

Table 120 says what each of the three entries **is**, and all three sentences are about content:

> A stream containing a Type 1 font program

> A stream containing a TrueType font program

> A stream containing a font program whose format is specified by the Subtype entry in the stream
> dictionary

Table 124 says what that content owes, once per subtype — of `Type1C` and `CIDFontType0C`, "[t]he
font program provided as the value of this key shall conform to Adobe Technical Note #5176", and
of `OpenType`, "[t]he font program provided as the value of this key shall conform to
ISO/IEC 14496-22:2019". No byte string of length zero conforms to any of the three: each format
begins with a header the empty string does not have.

So a descriptor whose program stream decodes to nothing has **provided no program**, and that is
the state §9.8.1 gives the rest of the descriptor for:

> These font metrics provide information that enables a PDF processor to synthesise a substitute
> font or select a similar font when the font program is unavailable.

`FontError::NotEmbedded` is this crate's name for that state, and it is already the state a
descriptor with no `/FontFile*` key at all reaches.

## Decision

**A font-program stream whose decoded bytes are empty states no program**, so the key is passed
over as though it were absent: `no_program` answers `true`, `/FontFile2` and `/FontFile3` continue
the loop, `/FontFile` returns the `NotEmbedded` the tail of the function would have returned, and
the font takes §9.8.1's substitution route.

Three things about where the line falls, because this module refuses four other damages and it
would be easy to read this as softening them.

- **It is asked before `whole_program`, and the order carries the argument.** ADR 0343's refusal
  turns on what a *prefix* of a program is — a table directory describing bytes that are not there
  — and ADR 0836's on a check value saying that whole bytes are not the compressed ones. Both
  refuse because reading the bytes produces glyphs the producer never wrote, which is ADR 0106's
  substitutive failure. **Zero bytes describe nothing and can be read as nothing**, so no mark can
  stand in place of the producer's, and the test those two ADRs are decided by returns the other
  answer here.
- **The condition is emptiness, not shortness.** "A stream of no bytes holds no program" is settled
  by the standard's own sentences above; "eleven bytes are too few for a CFF" is a claim about
  three format specifications and would be this tree's rather than the clause's. Nothing turns on
  it: the census finds **no** font-program stream anywhere between one byte and eleven.
- **It is not a licence to substitute for a program that failed to parse.** A `Malformed` from a
  program with bytes in it is still refused, unchanged, and `issue13316_reduced.pdf` still draws
  nothing and says why.

## What it costs and what it buys, measured

`crates/pdf-model/examples/empty_font_program_census` over five populations — the 964 pdf.js
documents that open, `pdf20examples` (7), `pdf-differences` (37), `pdfbox` (64) and
`format-corpus` (165), **1 237 in all**, holding 342 `/FontFile`, 831 `/FontFile2` and 420
`/FontFile3` streams:

| | |
|---|---|
| streams decoding to zero bytes, decode whole | **1** — `bug866395.pdf`'s `/FontFile3` |
| streams decoding to zero bytes at damage | 0 |
| streams decoding to between 1 and 11 bytes | 0 |

So the rule reaches one page in every corpus this tree has, and it is the page the sweep produced.

On that page, `(1 − mean) × 255` after `-alpha off -channel R -colorspace Gray`:

```text
        before   after
ours     0.000  11.706
hayro   11.502  11.502
poppler 11.780  11.780
mupdf   11.801  11.801
ghostscript      8.549
```

Ours lands inside the references' own spread and 0.07 from `poppler`. The four-panel strip is the
half the numbers cannot carry: ours, `poppler`, `mupdf` and `hayro` draw `l’impayé` in a bold sans
and `ghostscript` draws it in a regular weight, which is where its 8.549 comes from — Table 120's
`/FontWeight 700` honoured by four readers and not by the fifth. That agreement is **evidence about
our reading** and never its ground; what settles the page is the two sentences quoted above.

The oracle: **agrees 984 → 985, ambiguous 836 → 835, contradicted 61 → 61.** The page was
`ambiguous (incomplete)` and is `agrees` on a document this tree now calls complete. Step 7's sweep
re-run after the change is **byte-identical on all 835 remaining rows** and the only row that left
is this page — which is the sweep's own property rather than a weak result: a page crossing
`ambiguous` to `agrees` is invisible to an instrument whose population is the ambiguous bucket.

## The correction beside it

Two doc comments in `program.rs` had been **welded**: `extracted_cff`'s carried `is_bare_cff`'s
first paragraph and `simple_units_per_em`'s carried `parsed_type1`'s, so two private functions had
no documentation and two others had two opening sentences. It is `doc/todo/00`'s *A group's
diagnosis can migrate to the group above it* one directory over — Rust attaches a doc comment to
whatever item follows, and nothing is malformed, so neither `rustc` nor `clippy` nor
`missing_docs` sees it. Both are restored, each with a line saying where it had been.
