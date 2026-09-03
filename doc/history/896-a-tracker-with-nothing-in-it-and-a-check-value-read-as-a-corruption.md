# 896 — A tracker with nothing in it, and a check value read as a corruption: `batch5/DSS` held whole, §7.4.4.1's two RFCs told apart, and the page that declined the consequence

Date: 2026-09-03.
ADRs: [0836](../adr/0836-a-deflate-stream-whole-under-a-check-value-that-disagrees-is-not-a-prefix.md).
0837 was allocated to this round and not needed: one decision, one ADR.
Touched: `crates/pdf-syntax/src/filter.rs` (`Damage::CheckValue`, `only_the_check_value`,
`inflate`'s classification, `Inflate::turn`'s, two tests),
`crates/pdf-font/src/program.rs` (`whole_program`'s third arm and its two sentences),
`crates/pdf-model/src/colour.rs` (§8.6.5.5's guard, declining the new value on its own reasoning),
`crates/pdf-model/tests/silent_fonts.rs` (the witness's doc comment, corrected, and its
assertion), `crates/pdf-model/examples/damaged_stream_census.rs` (the value counted by consumer
and every stream carrying it named), `doc/checks/fixed-documents.toml` (one row),
`doc/conformance/ledger.toml` (§7.4.4.1, §9.9), `doc/todo/03-more-corpora.md` §45.

## The tracker

`batch5/DSS`, the largest unwalked issue tracker, surveyed whole under the four rules — twelve
rayon threads, `tools/bounded.sh --data 8 --tree 12`, **0.5 s and a 0.04 GiB peak**:

| directory | documents | line |
|---|---|---|
| `batch5/DSS` | 243 | 1 unopenable, 0 locked, 0 encrypted beyond us, 0 pageless, 3 incomplete, 0 slow |

**1.23% incomplete is the lowest rate of any tracker so far by a factor of two** — `MOZILLA` is
2.47%, the pdf.js gate 6.98%, `PDFIUM` 17.4% — and the reason is what a DSS issue attachment is: a
*signed* document somebody could not validate, rather than one somebody could not read. A
signature is applied to a file that was well formed when it was written. The one unusable file is
fifteen bytes.

Ranked by ink against `pdftoppm -cropbox` and `mutool draw` at 72 dpi, all three incomplete pages:

| document | ours | poppler | mupdf |
|---|---|---|---|
| `DSS-1356-8.pdf` | 9.174 | 9.126 | 9.135 |
| `DSS-1356-4.pdf` | 9.394 | 9.401 | 9.425 |
| `DSS-1441-2.pdf` | 0 | 0 | 0 |

**Every row is within 0.04 of a level of both references, so the whole ranking was held by name.**
`DSS-1356-8.pdf` is byte-identical to `batch1/PDFBOX/PDFBOX-3631-15.pdf` (`md5sum`), round 894's
fixed-documents row; `-4` is the same SignRequest template one revision earlier; both report
§9.9's closed-by-decision population. `DSS-1441-2.pdf` is blank in all three renderers. A tracker
that gives a round nothing is a result and is recorded as one — the walk cost half a second.

The round then moved down to the two documents earlier rounds had left named.

## The finding

`PDFIUM-407-0.pdf` — `doc/todo/03` §43's second row — is a four-page German Jobcenter form, ours
at 8.507 levels of ink against `poppler` 15.919 and `mupdf` 15.175. **Trap 1 opened this round and
trap 1 closed it.** The report named three fonts and one short image; what the *page* showed is
that every one of the form's field labels was drawn nowhere — `Anlage`, `Anrede`, `Vorname`,
`Familienname`, `Geburtsdatum` — while the green instructional text and every rule were there.

```
Font { detail: "font /TT0 could not be parsed: /FontFile2 decoded only as far as its damage
(Corrupt, 70946 bytes): a prefix of a font program is a directory describing bytes that are not
there" }
```

**70 946 is that stream's `/Length1` exactly.** Inflating all seven of the file's `/FontFile2`
streams past RFC 1950's two-byte header with a raw decoder:

| object | `/Length1` | raw inflate | reached the final block | Adler-32 stored | over the bytes |
|---|---|---|---|---|---|
| 785 `/Arial-BoldMT` | 748 975 | 748 977 | yes | `66c9d1ca` | `9852c7b7` |
| 819 `/TT0` | 70 946 | 70 946 | yes | `e1bb01d5` | `8b590373` |
| 821 `/TT3` | 56 274 | 56 270 | yes | `f4b126fc` | `840bde75` |
| 10, 820, 822, 823 | — | to the byte | yes | — | equal |

Every one reaches RFC 1951's final block and produces every byte its deflate blocks describe;
`qpdf --qdf` warns about none of them; `mutool` prints `ignoring zlib error: incorrect data check`.
**§7.4.4.1 makes two documents normative in one sentence** — the Flate method "is fully defined in
Internet RFC 1950 , and Internet RFC 1951 " — and they say different things. `flate2` answers
`Err` for failing either, because zlib's `inflate` reports `Z_DATA_ERROR` for `incorrect data
check` exactly as it does for a back-reference past the window, and `filter::turn` mapped every
`Err` to `Damage::Corrupt` — whose own documentation says the data is not what the grammar admits
"**at a definite point in it**". A checksum over a whole stream names no such point.

ADR 0836: **`Damage::CheckValue`**, decided by `only_the_check_value` the way ADR 0744 decided a
flush — a raw replay over the same bytes, where there is no check value to satisfy, reaching
`StreamEnd` at the length the framed decode produced. One extra inflate, on the damaged path only.

## The consequence, implemented and then declined by a page

The classification made a second question askable for the first time: `pdf_font::whole_program`
refuses a damaged decode on one argument — a prefix of a program is a table directory describing
bytes that are not there — and that argument does not reach a stream that is whole. **So the
admission was written, and the gate refused it.**

`cargo nextest run --workspace` failed on
`pdf-model::silent_fonts::a_font_that_draws_none_of_its_codes_is_reported`, whose document is ADR
0459's own witness. `doc/pdf.js/test/pdfs/issue13316_reduced.pdf`'s `/FontFile2` is exactly this
shape — **168 808 bytes, its `/Length1` to the byte**, RFC 1951 whole, Adler-32 disagreeing, and
**all ten of its sfnt table checksums correct**, so the program is internally coherent. Admitted,
it loads, and the page draws **A C E F** where `pdftoppm` draws 开票通知单 — five CJK glyphs —
and reports nothing at all.

**The four letters are not the damage, and finding that out is what settled the round.** The page
is `(ABCBDBEBF) Tj` through an `/Encoding` whose `/Differences` names `/g5167`, `/space`,
`/g11927`, `/g17737`, `/g11540` and `/g2180`. §9.6.5.4 carries a name through the Adobe Glyph List
to a (3, 1) subtable or through Mac OS Roman to a (1, 0) one, and then: "if the glyph name cannot
be mapped as specified, the glyph name shall be looked up in the font program's "post" table (if
one is present)". **This program has no `post` table.** Every route the clause states runs out, and
what takes over is the clause's own closing permission — a processor "may supply a mapping of its
choosing" — under which this tree offers the code to the font's subtables and gets the code's own
character. That tier is deliberate, documented, and has two witnesses justifying it; it is not a
defect to remove. It is simply what an admitted stream of this shape puts on the page: marks
standing *in place of* the producer's, ADR 0106's substitutive failure, which ADR 0459 refuses.

So the refusal stays, with a sentence of its own:

```
/FontFile2 decoded whole and its check value disagrees (168808 bytes): RFC 1950's Adler-32 says
these are not the bytes that were compressed, and a font program whose content may not be its own
draws glyphs in place of the producer's
```

**What it costs is on the record rather than assumed.** Admitting the three streams takes
`PDFIUM-407-0.pdf` from 838 display-list commands to 1806, its lost text operations from 197 to
49, and its ink from 8.2913 to 12.8857 by the fixed-documents instrument (8.507 to 13.102 by the
ranking's) against 15.919 and 15.175, with the third font refused by the *parser* on `units per em
is zero` — where `poppler` also fails (`Couldn't create a font for 'BKMWRB+Arial-BoldMT'`) and
substitutes. That is evidence about a file. It is not evidence about the rule, and no instrument
available at a decode separates the two files: a 32-bit sum over 168 808 bytes and one over 70 946
say the same thing with the same confidence. The sfnt's own per-table checksums were tried as a
discriminator and point the wrong way — `PDFIUM-407-0.pdf`'s two usable fonts fail theirs and draw
correctly, `issue13316_reduced.pdf` passes all ten and draws wrongly — so adopting them would be
fitting a rule to two files. `doc/todo/03` §45 holds it with that reason.

**§8.6.5.5's ICC guard declines the value too**, and now says so deliberately: Table 65 states the
producer's own `/Alternate`, so a refusal there costs a *stated* colour space rather than a
missing one.

## The population

`pdf-model --example damaged_stream_census` now counts the value by the consumer that reads the
stream and names every stream carrying one.

Over **`doc/pdf.js`'s 974 documents and `doc/corpora`'s 277 — 1251 files of which 1239 open,
25 435 stream objects — 334 streams are damaged, and 158 of those are whole deflate streams whose
check value disagrees.** That is **47.3%** of all stream damage in the tree's own gated corpora,
over 30 documents, and it is what makes this a value rather than a comment: the report it corrects
was wrong about nearly half the streams it was printed over.

By the consumer that reads them, damaged first and of those the check-value ones:

| consumer | damaged | of them a check value |
|---|---|---|
| a page's `/Contents` | 95 | **71** |
| an image | 135 | **50** |
| a font program | 44 | **18** |
| a Type 3 glyph description | 28 | 6 |
| an ICC profile | 5 | 4 |
| a `/ToUnicode` CMap | 4 | 2 |
| an annotation appearance | 1 | 1 |
| an `Indexed` lookup table | 1 | 1 |
| unclassified | 12 | 5 |
| a form `XObject`, a function, an object stream, a shading | 14 | 0 |

**The eighteen font programs are over eight documents**, and `issue13316_reduced.pdf`'s object 31
is one of them — the largest, at 168 808 bytes. The other seven are ordinary crawled documents
(`178360.pdf` alone holds eight such programs), and what the refusal costs on each is now a
question a round can ask with the file in front of it rather than a silence. The **four ICC
profiles** are over three documents (`050734.pdf`, `435321.pdf`, `498264.pdf`), which is the
population `doc/todo/03` §45 hands on for §8.6.5.5's held decision.

Two figures in the run are *not* this population and are worth separating: the "names damage in
107 reports" line counts this tree rather than the files, and the `/Contents` line counts page one
only — the 71 above are every page.

## The other named document, opened and held

`sumatrapdf-LINK-1532-1.pdf` — `doc/todo/03` §44's dark-end row, ours **61.3** against two blank
references — is a 41 MB Library of Congress scan of a 339-page book. Page one is the tan cover with
its accession label, and **we draw it correctly in two commands**; neither reference finds a page
in the file at all (`poppler`: "Top-level pages object is wrong type (null)"; `mutool`: "cannot
find startxref"). So the 61.3 is a page we draw and they do not, and the ranking's instrument reads
it as a gap it is not — worth recording, because a one-reference row at the *dark* end can mean
either. What is actually lost is 8 text operations of the scan's own OCR layer, behind a `/Font`
entry `F1` that is a reference to an object the file does not define — §7.3.10's null, ADR 0789's
side. **Held**: no mark on the page, and what it costs is text extraction.

## Gates

The whole `doc/todo/02` §2 sequence ran in the worktree under `tools/bounded.sh` (`--tree 8` for a
build, `--data 12 --tree 12` for a walk, one walk at a time), on a quiet machine after the census
was stopped for exactly that reason. **Every one of its twenty-one lines exits 0**, plus
`--bin quotations` and `--bin pointers` because documents moved.

- formatting and `clippy` under `RUSTFLAGS="-D warnings"` silent for the workspace and for
  `fuzz/`; the `viewer-qt` `cargo:warning=` lines are gcc's about generated code on a cold build,
  as `doc/todo/02` §2 says;
- **3080 tests passed, 22 skipped** (`the_bounded_wrappers_self_test_holds` SLOW at 69 s and
  green); doctests green;
- corpus gate **974 documents in 10.2 s — 0 unopenable, 9 locked, 1 encrypted beyond us, 5
  pageless, 64 incomplete, 0 slow**, which is round 894's figure unmoved: the refusal this round
  keeps is the refusal those pages already had, under a truer sentence;
- oracle **1945 pages in 30.8 s, 1841 complete, 104 incomplete**, 61 contradicted and every one
  held by a group by name, 33 not comparable;
- text extraction **11 094 of 11 131 matched words in bounds (99.67%), 493 of 503 documents fully
  in**; selection census **1000 of 1011 words (98.91%) over 453 documents**; accessibility census
  green over **104 documents with a structure tree**, 102 853 elements;
- dates **1514 of 1545 (97.99%)**; XMP **318 of 319 streams read**; JPEG 2000 green;
- quorra **958 pages, 929 agree, 22 differ, 7 refused, 16 not comparable**;
- fixed documents **68 checked, 0 absent, 68 rows**, this round's among them;
- transform gate **194.1 pages/s over a floor of 40**; the writer over **974 documents in 6.8 s**,
  0 unexplained refusals;
- conformance green — **875 subclauses, 0 owing a review, 13 094 citations, 1192 quotations
  verbatim**; `quotations` and `pointers` naming nothing of this round's.

**The fixed-documents row failed once and taught its own rule**: a row's `reports` must name
*every* report the page carries, not only the one it is about, so the first version failed with
`unexpected report Text { operations: 197 }`. Listing that count is the better row anyway — it is
exactly the text those three fonts carry, so it is the entry that fails if the streams are ever
admitted without the decision being revisited.

**One thing this round did not measure, named rather than left implicit.** The census was also run
over `corpus-cache`'s 89 000 documents and killed at 55 minutes, because it holds the machine's one
walk slot and the gate sequence needed it. The population above — 1251 documents, 25 435 stream
objects, the tree's own gated corpora — is what is quoted everywhere, and it is stated as that
rather than as the crawl's. A round that wants the crawl's figure should budget an hour and take
the slot first.
