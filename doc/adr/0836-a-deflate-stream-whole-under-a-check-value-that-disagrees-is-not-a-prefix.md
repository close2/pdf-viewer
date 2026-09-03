# ADR 0836 — A deflate stream whole under a check value that disagrees is not a prefix: §7.4.4.1 makes RFC 1950 and RFC 1951 two documents, and one error stood for failing either

Status: accepted. Session 896.
Clauses: ISO 32000-2 §7.4.4.1 (`FlateDecode`, and the two RFCs it makes normative), §7.4.1 (what
invoking a filter achieves and what it does not), §9.9 Table 125 (`/Length1`, an embedded
program's extent), §9.6.5.4 (which supplies the marks the refusal below keeps off the page).
Code: `crates/pdf-syntax/src/filter.rs` (`Damage::CheckValue`, `only_the_check_value`, `inflate`'s
classification and `Inflate::turn`'s), `crates/pdf-font/src/program.rs` (`whole_program`'s third
arm and its two sentences), `crates/pdf-model/src/colour.rs` (§8.6.5.5's guard, which declines the
new value on a reason of its own).
Tests: `crates/pdf-syntax/src/filter.rs::a_whole_stream_under_a_wrong_check_value_is_not_a_prefix`,
`::a_deflate_stream_that_broke_is_still_corrupt`,
`crates/pdf-model/tests/silent_fonts.rs::a_font_that_draws_none_of_its_codes_is_reported`;
`doc/checks/fixed-documents.toml`'s row for `batch5/PDFIUM/PDFIUM-407-0.pdf`.
Measurement: `crates/pdf-model/examples/damaged_stream_census.rs`, which counts the value by
consumer and names every stream that carries it.
Documents: §7.4.4.1's and §9.9's ledger rows, `doc/todo/03` §45.

## Context

`doc/todo/03` §43 left `PDFIUM-407-0.pdf` named — a four-page German Jobcenter form, ours at 8.507
levels of ink against `pdftoppm -cropbox`'s 15.919 and `mutool draw`'s 15.175, with every one of
the form's field labels drawn nowhere. Its report:

```
Font { detail: "font /TT0 could not be parsed: /FontFile2 decoded only as far as its damage
(Corrupt, 70946 bytes): a prefix of a font program is a directory describing bytes that are not
there" }
```

**70 946 is that stream's `/Length1` exactly.** A report that says *prefix* about a decode
producing the whole of what Table 125 says the program is, is a report about something else.

## What the streams are

Inflating each of the file's seven `/FontFile2` streams past RFC 1950's two-byte header with a raw
decoder, which never looks at the trailer:

| object | `/Length1` | raw inflate | reached the final block | Adler-32 stored | over the bytes |
|---|---|---|---|---|---|
| 785 `/Arial-BoldMT` | 748 975 | 748 977 | yes | `66c9d1ca` | `9852c7b7` |
| 819 `/TT0` | 70 946 | 70 946 | yes | `e1bb01d5` | `8b590373` |
| 821 `/TT3` | 56 274 | 56 270 | yes | `f4b126fc` | `840bde75` |
| 10, 820, 822, 823 | — | to the byte | yes | — | equal |

Every one reaches RFC 1951's final block and produces every byte its deflate blocks describe;
`qpdf --qdf` inflates all seven to those same lengths and warns about none of them. What disagrees
on three is the four-byte Adler-32 RFC 1950 wraps around the data. `mutool` says so in as many
words: `ignoring zlib error: incorrect data check`.

## The clause: two documents in one sentence

§7.4.4.1 makes both normative:

> The Flate method is based on the public-domain zlib/deflate compression method, which is a
> variable-length Lempel-Ziv adaptive compression method cascaded with adaptive Huffman coding. It
> is fully defined in Internet RFC 1950 , and Internet RFC 1951 .

RFC 1951 defines the deflate data and its final block. RFC 1950 defines a two-byte header, that
data, and an Adler-32 over the *uncompressed* bytes, and requires a compliant decompressor meeting
a wrong one to indicate an error. A stream can satisfy the first and fail the second — and this
tree had no way to say so. `flate2` answers `Err` for both, because zlib's `inflate` reports
`Z_DATA_ERROR` for `incorrect data check` exactly as it does for a back-reference past the window,
and `filter::turn` mapped every `Err` to `Damage::Corrupt`.

**`Damage::Corrupt`'s own words are what make that wrong rather than merely coarse**: the data "is
not what the filter's grammar admits, **at a definite point in it**", and everything after that
point is unrecoverable. A wrong check value names no point. It says that *something* among the
bytes differs from what was compressed, and it cannot say what or where, because it is one 32-bit
sum over the whole of them.

## The decision, in two halves

### The classification

**`Damage::CheckValue`**, a third value: the decode reached the filter's end-of-data and the check
value over its bytes disagrees. `Damage`'s own first sentence changed with it — two of the three
stop short of the filter's end-of-data and this one does not.

**How it is told from a corruption is decidable rather than guessed**, which is ADR 0744's method
one level on. RFC 1950's check value is the last thing a framed decoder consumes and the only
thing in the framing verified *after* the output is written, so replaying the same bytes through a
**raw** decoder — where there is no check value to satisfy — separates the two: a replay reaching
`StreamEnd` having written the same number of bytes the framed decode produced is a deflate stream
that was whole. A replay that breaks, or ends at a different length, is a decoder disagreeing with
itself and `only_the_check_value` answers `false`. The output length is not a formality: without
it a replay that ended early for its own reasons would read as agreement. One extra inflate, on
the damaged path only, thrown away a scratch buffer at a time.

Every consumer that only *reports* damage now reports the true thing. A content stream still draws
what came out; the word in the report is what moves.

### The refusal that stays, and the reason it needed a new one

**A font program is still refused, and no longer on a sentence about prefixes.**
`pdf_font::whole_program` refused a damaged decode on one argument: a font program is a structure
whose table directory points forward, so a prefix of one is "a directory describing bytes that are
not there". That argument does not reach `CheckValue` — there is no prefix and no shortfall
against Table 125's extent — so the refusal needed a reason of its own or it needed withdrawing.

**Withdrawing it was implemented, measured, and declined**, and the document that declined it is
ADR 0459's own witness. `doc/pdf.js`'s `issue13316_reduced.pdf` has a `/FontFile2` of exactly this
shape: **168 808 bytes, its `/Length1` to the byte**, RFC 1951 whole, Adler-32 disagreeing. Every
one of its ten sfnt table checksums is correct, so the program is internally coherent. Admitted,
it loads, and the page draws **A C E F** where `pdftoppm` draws five CJK glyphs — reporting
nothing at all.

**And the four letters are not the damage**, which is the part worth keeping. The page is
`(ABCBDBEBF) Tj` through an `/Encoding` whose `/Differences` names `/g5167`, `/space`, `/g11927`,
`/g17737`, `/g11540` and `/g2180`. §9.6.5.4 routes a name through the Adobe Glyph List to a (3, 1)
subtable, or through Mac OS Roman to a (1, 0) one, and then:

> In any of these cases, if the glyph name cannot be mapped as specified, the glyph name shall be
> looked up in the font program's "post" table (if one is present) and the associated glyph
> description shall be used.

This program has no `post` table. So every route the clause states runs out, and what takes over
is the clause's own closing permission — a processor "may supply a mapping of its choosing" —
under which this tree offers the code to the font's subtables and gets the code's own character.
That tier is deliberate, documented and has two witnesses of its own; it is not a defect to be
removed. It is simply what an admitted stream of this shape puts on the page: **marks standing in
place of the producer's**, which is ADR 0106's substitutive failure and what ADR 0459 refuses.

So the third arm is a refusal with its own sentence:

> `/FontFile2` decoded whole and its check value disagrees (168808 bytes): RFC 1950's Adler-32
> says these are not the bytes that were compressed, and a font program whose content may not be
> its own draws glyphs in place of the producer's

**What it costs is written down rather than assumed.** `PDFIUM-407-0.pdf` is the other side: two
of its three streams carry a font that draws its page's German labels exactly as both references
draw them — 8.507 levels of ink while refused, 13.102 when admitted, against 15.919 and 15.175 —
and the third is refused by the *parser*, on `units per em is zero`. That is evidence about a
file. It is not evidence about the rule, because the rule has to hold for the file that decided
it as well, and no instrument available at decode time separates the two: a checksum over 168 808
bytes and a checksum over 70 946 say the same thing with the same confidence.

**§8.6.5.5's ICC guard declines it too**, and now says so deliberately rather than by inheriting
the prefix sentence: Table 65 states the producer's own `/Alternate` — "shall be used in case the
one specified in the stream data is not supported" — so a refusal there costs a *stated* colour
space rather than a missing one.

## What it reaches

`pdf-model --example damaged_stream_census` counts damaged streams by the consumer that reads them
and now breaks out this value and names every stream carrying one. Over **`doc/pdf.js`'s 974
documents and `doc/corpora`'s 277 — 1251 files of which 1239 open, 25 435 stream objects — 334
streams are damaged and 158 of those are whole deflate streams whose check value disagrees**:
**47.3%** of all stream damage in this tree's own gated corpora, over 30 documents. By consumer:
71 of 95 damaged `/Contents`, 50 of 135 images, **18 of 44 font programs** over eight documents, 6
of 28 Type 3 glyph descriptions, 4 of 5 ICC profiles, and one apiece of an annotation appearance
and an `Indexed` lookup table.

That is what makes this worth a value rather than a comment: the sentence it corrects was wrong
about nearly half the streams it was printed over, and this is the largest single kind of stream
damage this tree meets rather than one fuzzed file's accident. `doc/history/896` has the table and
the named streams; `doc/todo/03` §45 hands on the four ICC profiles as the population for the
held decision above.

## The general lesson

**A library's error type is a claim about the input, and it is the library's claim rather than the
standard's.** `Z_DATA_ERROR` covers two conditions §7.4.4.1's two RFCs keep apart; this tree
adopted the coarser one and printed a sentence about prefixes on top of it — a sentence then false
of the commoner half. ADR 0823 found the same shape in a codec's message (`unexpected end of
input`, read as a statement about the file); this is the same finding one crate down, and the rule
is the same: **a decoder's error names where its own algorithm stopped, never what is wrong with
the document.**

And a second, about the round rather than the code: **the classification and the consequence are
separate decisions, and only the first one was forced by the clause.** Getting the first right
made the second askable for the first time, and the answer to it came from a page rather than an
argument.
