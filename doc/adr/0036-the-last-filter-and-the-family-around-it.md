# ADR 0036 — The last filter, and the family around it

Status: accepted, 2026-07-30.

## Context

`LZWDecode` had been the one standard filter of any kind this reader did not implement, and it
had sat on the "what is not implemented" list at **Small / 0 corpus documents** for the whole
life of the project. Three corpus documents contain the string, all three draw completely, and
no corpus *page one* reaches one — so neither gate could rank it and neither gate could see it
land.

That is exactly the condition `CLAUDE.md`'s two-track rule exists for. The demand track had
nothing to say about this filter; the specification track says clause 7 is "complete means
complete, including encryption and every filter", and Table 6 has ten rows.

The family around it — §7.4, ten subclauses — was almost entirely `unreviewed`: only §7.4.6
(`CCITTFaxDecode`) had been read, and §7.4.7 and §7.4.9 were two of the nine clauses in
`REVIEW_OWED`.

## Decision

### The clause states the algorithm, so the algorithm comes from the clause

§7.4.4.2 is four paragraphs and they are complete: the code alphabet, the table's initial
state, when the width grows, the encoder's four steps, and the packing. There is no external
reference to follow, unlike §7.4.6, which hands the coding to ITU-T T.4 and says so.

The table is a prefix code and one byte per entry, which is what lets the clause say "the
encoder and the decoder shall maintain identical copies of this table" without either copying
anything: entry *n* is entry `prefix[n]` followed by `suffix[n]`, so appending is two stores.

Three details decide whether a decoder is right, and each is a sentence:

- **The width grows before the entry that needs it.** "The first output code that is 10 bits
  long shall be the one following the creation of table entry 511", and Table 8's
  `/EarlyChange` moves that one code earlier — **defaulting to doing so**, because a
  widely-copied encoder did. Getting it wrong desynchronises the bit stream from that point
  and produces plausible bytes for ever after.
- **A code may name the entry about to be created.** The encoder emits the code for a sequence
  it has just added, so the decoder must reconstruct it: the previous sequence followed by its
  own first byte. The standard's own worked example reaches this on its *third* code.
- **Bits are packed high-order first**, straddling byte boundaries arbitrarily.

A code past the end of the table, or a stream with no EOD marker, keeps what was decoded — the
reason `flate` keeps a truncated inflate: a partial content stream still renders most of a page.
Output is bounded by `Limits::max_stream_len`, because NOTE 2 says LZW "provides a compression
approaching 1365:1 for long files", which is a decompression bomb written in nine bits.

### Two kinds of test, and the second is the one worth copying

§7.4.4.2 supplies **its own test vector** — EXAMPLE 1's input and EXAMPLE 2's packed bytes —
which makes `lzw_decodes_the_clauses_own_example` the rarest kind of test in this tree: an
expected value the standard states outright rather than one derived from a rule. It is also a
sharp one, because the example's third code is the reconstruct-the-pending-entry case.

The second kind needed no reference decoder either. `crates/pdf-syntax/tests/lzw.rs` decodes
three real streams from two corpus documents and checks them against **the same documents' own
dictionaries**, written by the same producer and compressed separately:

| stream | what says how long it must be | bytes |
|---|---|---|
| `XiaoBiaoSong.pdf` obj 7 | `/Width 57 /Height 78 /BitsPerComponent 8` | 4446 |
| `XiaoBiaoSong.pdf` obj 9 | `[/Indexed /DeviceRGB 255 …]`, 256 × 3 | 768 |
| `bug864847.pdf` obj 6 | a `/ToUnicode` stream is a `CMap` program | starts `/CIDInit /ProcSet findresource begin` |

A decoder one code out of step produces a *different length*, because the codes after the error
name table entries of other lengths. Matching to the byte across 4446 of them, twice, with a
third stream producing exact PostScript text, is stronger evidence than agreement with another
decoder would be — and it is the shape `doc/HANDOVER.md` calls *a corpus stating an invariant
about itself*. Both of `XiaoBiaoSong.pdf`'s streams are `[/ASCII85Decode /LZWDecode]`, so the
same test checks §7.4.1's cascade runs in the right order.

### A test that was designed to fail, and did

`colour_paths.rs::a_content_stream_that_will_not_decode_is_reported` named `/LZWDecode`
deliberately — "the filter name is deliberately real rather than invented: implementing
`LZWDecode` should make this test fail, and that is the right moment to revisit it". That moment
arrived, and the revision is a finding: **there is no standard filter left that this reader does
not implement**, so no name can stand in for "a filter we do not have". What replaced it is a
filter that is real, implemented, and not a *content stream* codec — `/JPXDecode`, which
produces an image raster and which `filter.rs` answers `None` for so that a stream expecting
bytes is visibly unsupported rather than silently empty.

## What reading §7.4 as a family found

Fourteen rows, of which one was already reviewed. Three findings, and one of them is the kind
principle 5 exists for.

**§7.4.8's `/ColorTransform` is unread, and implementing it would break the only file that
exercises it.** Table 13's rule has three cases: an Adobe APP14 marker wins outright; with no
APP14 the dictionary entry governs; with neither, the default is 1 for three components and 0
otherwise. Four corpus documents write the entry and all four write `0`. Two are
single-component images, where the clause says it "shall be ignored". One is three-component
with a JFIF marker. **One — `issue12841_reduced.pdf` — is three-component with neither marker,
which is precisely the case where the clause says the dictionary decides**, and obeying it means
*not* transforming YCbCr to RGB. All four reference renderers transform it; the photograph is
plainly right transformed and would be unrecognisable otherwise; the producer evidently did not
mean what it wrote.

So the entry stays unread and the ledger row records why. This is not "match the other
renderers": it is a measured statement that the clause's sentence has exactly one witness in
974 documents and that witness contradicts it. The next file that writes `/ColorTransform 0` and
means it will be a different question with the same clause, and the row is where that argument
starts.

**§7.4.2 is `partial` for one sentence, and it is a departure rather than an oversight.** "Any
other characters shall cause an error", and `ascii_hex` skips them. A hex stream with a stray
byte decodes to what its producer meant everywhere except at that byte; refusing loses the whole
stream. Recorded as a choice.

**Neither §7.4.7 nor §7.4.9's ban on inline images is enforced.** Both clauses say their filter
"shall not be used with inline images", and `inline_image.rs` decodes one that names them.
Drawing what the producer wrote is the right call for a reader, and it is now written down
rather than accidental.

## Consequences

**All ten of Table 6's filters decode.** Clause 7's filter family is complete, and clause 7 as a
whole is now missing only a public-key security handler and a password prompt.

Neither gate moved: no corpus page one reaches an LZW stream, which was known before the work
started and is the whole reason this item needed the spec track to reach it. The ledger's
unreviewed count fell 462 → 448 — the largest single-session fall so far — and `REVIEW_OWED`
11 → 9, then to 7 as §7.4.7 and §7.4.9 left it.

**A closed list can be finished, and finishing it removes an instrument.** The report that names
an undecodable content stream had a standard filter to point at for the project's whole life.
It no longer does. That is what completing a family looks like from the inside, and it is worth
noticing that the *test* is what noticed.
