# ADR 0467 — The three filters whose end nobody had walked

Status: accepted, 2026-08-21. Session 635. Finishes what ADRs 0464 and 0466 started from opposite
ends: §8.9.7's filtered inline image now has a **derived** extent for every filter the clause
admits. Amends the ledger rows for §7.4.5, §7.4.6, §7.4.8, §7.3.8.2 and §8.9.7.

## What was left

Sessions 631 and 633 gave answer 3 two routes — run the first filter's decoder and ask what it
consumed (`FlateDecode`, `LZWDecode`), or find a marker the alphabet cannot contain
(`ASCII85Decode`'s `~>`, `ASCIIHexDecode`'s `>`). Everything else fell to **answer 4**, the search
for a token-delimited `EI`, which is wrong exactly when the encoded bytes carry a
whitespace-`EI`-delimiter sequence.

633's census said what that left, and the number is why this round exists rather than tidiness:
of the filtered inline images with no `/L` in the crawl, `CCITTFaxDecode` is **1 272 430** against
`FlateDecode`'s 1 367 073, with `DCTDecode` 3 778 and `RunLengthDecode` 1 655 behind. Nearly half
the population had its end guessed at.

633's own note said three of the five "need no decoder at all". That was right about
`RunLengthDecode` and understated the other two: **none of the three needs a decoder**, and the
reason is different for each.

## What each clause makes derivable

### §7.4.5 — `RunLengthDecode`: every byte is a header or is counted by one

> The encoded data shall be a sequence of runs, where each run shall consist of a length byte
> followed by 1 to 128 bytes of data. If the length byte is in the range 0 to 127, the following
> length + 1 (1 to 128) bytes shall be copied literally during decompression. If length is in the
> range 129 to 255, the following single byte shall be copied 257 length (2 to 128) times during
> decompression. A length value of 128 shall denote EOD.

A header at a known offset says where the next header stands. The walk reads one byte in every two
to a hundred and twenty-nine and stops on the 128; it reconstructs nothing. This is the cheapest of
the three and the one 633 had already named.

### §7.4.8 — `DCTDecode`: ISO/IEC 10918-1's marker segments, and why a search would be wrong

§7.4.8 states the framing by normative reference — the data is "encoded in the JPEG baseline
format in accordance with ISO/IEC 10918 (all parts)" — so the end-of-data §7.3.8.2 promises is
that standard's EOI marker.

**Searching the bytes for `FFD9` is not the answer, and the reason is a finding rather than a
worry.** `FFD9` stands freely inside a marker segment's payload, and an `APPn` segment is allowed
to carry an entire second JPEG — which is exactly what a camera's thumbnail is. Its EOI comes
hundreds of bytes before the real one. So the walk steps over each segment by the length it
states, and inside entropy-coded data it relies on 10918-1's byte stuffing: a coder that emits
`FF` writes `00` after it, so the only `FF` pairs there are `FF00` and the restart markers
`FFD0`–`FFD7`. No coefficient is decoded and no table is read.

### §7.4.6 — `CCITTFaxDecode`: Table 11's end-of-block, and T.4's uniqueness

Table 11's `/EndOfBlock` is the whole permission:

> A flag indicating whether the filter shall expect the encoded data to be terminated by an
> end-of-block pattern, overriding the Rows parameter. If false , the filter shall stop when it
> has decoded the number of lines indicated by Rows or when its data has been exhausted, whichever
> occurs first. The end-of-block pattern shall be the CCITT end-of-facsimile-block (EOFB) or
> return-to-control (RTC) appropriate for the K parameter. Default value: true .

Two halves, and both matter.

**Where the flag is true — its default, and what a file that never mentions the entry has — the
data *shall* be terminated by a pattern**, and both patterns are runs of ITU-T T.4's end-of-line
code `000000000001`: EOFB is two of them and RTC is six. Table 11 says only "appropriate for the K
parameter", so `/K` is what chooses: negative is Group 4 and T.6's EOFB, zero and positive are
Group 3 and T.4's RTC.

**Finding that pattern is a reading rather than a guess**, because T.4 constructs the end-of-line
code so that no sequence of valid codewords can contain it — that is what lets a fax receiver
resynchronise on it. Eleven zero bits followed by a one cannot stand inside encoded scan lines.
The converse gives the walk its shape and its speed: only the *leading* and *trailing* zero runs
of a byte can reach eleven, because a run bounded by one-bits inside a byte is at most six long,
so one pass over the bytes finds every candidate. T.4's fill — a variable run of zeros before an
end-of-line — is absorbed by the run being counted, and Group 3 mixed mode's tag bit after each
end-of-line is why two patterns count as consecutive when the second's zeros begin at most one bit
past the first's one-bit.

The byte count is the clause's own, one paragraph up:

> When a filter reaches EOD, it shall always skip to the next byte boundary following the encoded
> data.

**And `/EndOfBlock false` is where the clause itself puts the end outside the data.** The filter
then stops on `/Rows` or on exhaustion, which are facts about the decode; that arm answers
`Unknown` and keeps the search, and that is now a statement about the standard rather than about
this tree.

## An erratum, read before writing

`doc/errata-read.md`'s rule again — `spec-errata emit` over clauses 7 and 8 before a line was
written — and it paid on the sentence this whole round turns on. Errata Collection 3, §7.3.8.2,
Issue #319, `/State` `Review` `Completed`, inserts:

> NOTE: The 'encoded data' of a stream encompasses all enveloping markers of the encoding, e.g.
> end-of-data markers, if the encoding scheme uses them.

That is what decides whether each of the five extents is inclusive of its marker, and every arm
counts it in: the `>` and the `~>`, the 128 byte, the `FFD9`, and the byte boundary past the last
end-of-line of an EOFB. §7.3.8.2's row already recorded the erratum from the other side (session
592); this is the first round whose arithmetic rests on it.

A second one is read and is **not** load-bearing, and it is worth saying why rather than leaving it
found-and-unmentioned. §8.9.7, Issue #20 replaces "[u]nless the image uses ASCIIHexDecode or
ASCII85Decode as one of its filters" with "its final or only filter". Read as the last element of
the `/Filter` array that would contradict Table 5, which orders a chain "in the order in which they
are to be applied" and therefore makes the *first* element the one whose input the bytes after `ID`
are. It does not contradict it: "final" here is the last filter applied when **encoding**, which is
the outermost and is the first named — the arrangement §8.9.7's own EXAMPLE writes as `/F [/A85
/LZW]`, base-85 on the outside. So the filter this code asks is unchanged, and the erratum sharpens
NOTE 2's skipping advice rather than the chain order.

## The population, measured before the change

Trap 11, and trap 8's rule about the instrument: `examples/token_window_census` builds its **own**
`Delimiting` from each image's dictionary and compares it with where the scan stopped, so the
predicate is not the code under test. The run below is over **66 960 documents that open** of
67 213 (`corpus-cache`'s 145 archives and everything under `doc/`), **926 308 pages**, 3 979 750
inline images, of which **2 672 351** are filtered with no `/L` — the population at issue. 356 s.

| first filter | images | derivable | agreeing | **early** | over-run | no framing |
|---|---|---|---|---|---|---|
| `FlateDecode` | 1 367 153 | 1 366 707 | 1 366 707 | 0 | 0 | 446 |
| **`CCITTFaxDecode`** | **1 272 438** | **1 272 438** | **1 272 438** | **0** | 0 | 0 |
| `ASCII85Decode` | 23 018 | 23 018 | 23 018 | 0 | 0 | 0 |
| `ASCIIHexDecode` | 4 302 | 4 302 | 4 302 | 0 | 0 | 0 |
| **`DCTDecode`** | **3 781** | **3 781** | 3 740 | **1** | 40 | 0 |
| **`RunLengthDecode`** | **1 655** | **1 655** | 466 | 0 | 1 189 | 0 |
| `/F` not a name | 4 | 0 | 0 | 0 | 0 | 4 |

**Three numbers in that table are the finding.**

**`CCITTFaxDecode`: 1 272 438 of 1 272 438 answerable, and every one of them agrees with the
search.** Not one of nearly half the population was ending early. That is worth saying plainly:
this round moves no pixel on the `CCITTFaxDecode` row and changes what the answer *is* — a
derivation from Table 11 instead of a search that happened to be right on 1.27 million files and
is wrong on the first one whose compressed bits spell ` EI `. Trap 8's shape, from the other
direction: the corpus cannot show you a rule it does not break.

**`DCTDecode`: one image ends early**, 163 bytes of it, on `7311536.pdf` page 9 — and it is the
one number in this census that was already visible somewhere else and unattributed. The same run's
largest single lexical object over 8.88 **billion** content tokens is a **798.20 KiB string, on
`7311536.pdf` page 9**: the 163 bytes of JPEG the image lost were handed back to the lexer, one of
them was an opening parenthesis, and the string it began ran to the end of the content stream.
That figure is what `doc/todo/14`'s road D sizes a reader's window against, and it was an artefact
of this defect.

**The 1 229 over-runs are all exactly one byte** (widest 1 B, 1.20 KiB in total over 50 pages),
and they are the derivation being *right* rather than a hazard. §8.9.7 defines the length as
excluding "the white-space delimiting those operators", singular, and `search_for_terminator` duly
drops one byte; a producer that writes two whitespace bytes before `EI` leaves the second in the
data, and the derived end does not. One stray byte at the end of a `RunLengthDecode` run or past a
JPEG's EOI changes no sample. **They are counted apart from the early one for exactly that
reason** — a wide over-run would be a different thing entirely — and the census prints the widest
so that a later round does not have to take this paragraph's word for it.

## And afterwards

The same census, re-run over the same 66 960 documents with the three arms live:

| | before | after |
|---|---|---|
| filtered, no `/L` | 2 672 351 | 2 672 782 |
| answerable | 2 671 901 | 2 672 332 |
| **agreeing** | 2 670 671 | **2 672 332** |
| **ending early** | **1** | **0** |
| **over-running** | **1 229** | **0** |
| pages where the two answers disagree | 50 | **0** |

The 450 that are still unanswerable are the 446 corrupt `FlateDecode` streams and 4 images whose
`/F` is not a name.

**The denominators are not quite the same object, and that is the defect's own signature** — 633
recorded the same effect: where an image ends decides what the rest of the content stream lexes as,
so 3 979 750 inline images become 3 980 974 and the `DCTDecode` row moves 3 781 → 4 212. The
`CCITTFaxDecode` row does not move by one, which is the other half of what the table above says.

**And the largest lexical object in the whole census moved**, which is the number `doc/todo/14`'s
road D sizes a window against: **798.20 KiB (`7311536.pdf` page 9) → 390.16 KiB (`219789.pdf` page
9)**, and the count of tokens past 64 KiB from 13 to 12. The old figure was 163 bytes of JPEG being
lexed.

`7311536.pdf` page 9 itself, this tree's own ink over its own raster: **1.617 with 8 reports →
54.406 with none**, against `pdftoppm` 57.99, `mutool` 57.61 and `gs` 62.44 measured through
ImageMagick's mean, which is a different formula and is why it is quoted as a neighbourhood rather
than as a match. It is a row in `doc/checks/fixed-documents.toml`.

## The change

**`pdf_syntax::Delimiting`** is new and is the shape of the answer: five arms, one per way a filter
states its end — `Decoded(Pumping)`, `Marker(&[u8])`, `RunLength`, `Jpeg`, `EndOfBlock { .. }`.
`filter::encoded_extent` takes one and dispatches; `Document::filtered_extent` builds one from the
image's dictionary and its `/DecodeParms`, which is where `/K` and `/EndOfBlock` are read.

**That folds 631's route in.** `inline_image::terminating_marker` and its `find` are gone: answer 3
was two places asking two halves of one question, and it is now one call and one match. The module
keeps answer 4 and the reasons it can still be reached are now nameable — `/EndOfBlock false`, a
producer that wrote no end-of-block pattern, `FlateDecode` data corrupt before its marker (446 of
the crawl's).

**Where the walks live is a boundary this crate states**, and the module header says so rather than
leaving a reader to wonder: `pdf-syntax` decodes no image codec and still does not. Walking
§7.4.6's bit pattern and §7.4.8's marker segments reconstructs no sample — it reads lengths and
markers, which is what a parser does — so no codec arrived in the crate that must be safe against
untrusted bytes, and `#![forbid(unsafe_code)]` covers all of it.

## What it is pinned by

Trap 8's rule, so every fixture is hand-built and each asserts its own premise before asserting
anything else — a test whose encoded bytes turn out not to contain ` EI ` proves nothing.

| test | what it is |
|---|---|
| `run_length_data_ends_at_its_own_eod_byte` | a literal run holding ` EI `, ended by the 128 |
| `run_length_data_without_an_ei_inside_it_ends_where_it_always_did` | the twin, where both answers agree |
| `a_window_that_cuts_the_run_length_eod_asks_for_more_bytes` | the same image through a window cut before the EOD: `Truncated`, not a search |
| `a_jpeg_ends_at_its_own_end_of_image_marker` | ` EI ` inside entropy-coded data |
| `a_jpeg_without_an_ei_inside_it_ends_where_it_always_did` | the twin |
| `a_jpeg_thumbnails_own_end_of_image_does_not_end_the_outer_one` | an `APP1` carrying a whole second JPEG, so the fixture holds **two** EOI markers and the outer one is the answer |
| `a_group_4_fax_ends_at_its_end_of_block_pattern` | ` EI ` before an EOFB, `/K -1` |
| `a_group_4_fax_without_an_ei_inside_it_ends_where_it_always_did` | the twin |
| `a_group_3_fax_ends_on_six_end_of_lines_rather_than_two` | `/K 0`, where two end-of-lines are not the end |

**Trap 13, run rather than assumed.** With the three arms of `Document::delimiting` held at their
pre-round answers and nothing else changed, exactly those six defect-facing tests **fail** and all
three twins pass, alongside the sixteen the file already had.

## What is not done, and it is the clause's boundary rather than a shortfall

- **`/EndOfBlock false`** — the clause puts the end outside the data. The search remains, and it is
  the only answer available.
- **A Group 3 or Group 4 stream with no end-of-block pattern**, although Table 11's default says
  there shall be one. `EncodedExtent::Short`, then the search. The crawl has none: all 1 272 438
  carry one.
- **`FlateDecode` data corrupt before its marker** — 446 in the crawl, `EncodedExtent::Unknown`,
  and the search is the right answer there.
- **`JBIG2Decode`, `JPXDecode` and `Crypt`** are forbidden to an inline image by §8.9.7 itself, so
  there is no fourth thing to add.
