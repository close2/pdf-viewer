# ADR 0468 — Two entries a dictionary states, and a reader that would not read them

Status: accepted, 2026-08-21. Session 636. Takes `doc/todo/03` §23's named successor — the next
chunk of the SafeDocs crawl — and fixes the two defects its ten thousand documents produced.
Amends §7.4.9's, §8.9.5.2's and §9.6.3's ledger rows.

## The chunk, and why these archives

§23 leaves "23 944 crawled documents unranked … in archive-sized pieces". This round took **ten
whole archives — `1407`, `1899`, `2514`, `2637`, `3006`, `3744`, `3867`, `4728`, `5343` and
`5958`, 10 000 documents** — none of the forty-two archives sessions 603, 613, 615, 619, 625 and
631 ranked. *Which* archives is immaterial and that is ADR 0261's finding: the crawl is sorted by
SHA-256 and cut into equal pieces, so an archive is a hash bucket and any set of them is an
unbiased sample.

The instrument is 603's, unchanged and reused rather than rewritten: page one at 72 dpi from this
tree and from `pdftoppm`, `mutool` and `gs`, every invocation copied from
`tools/pdfref/src/reference.rs` and explicit about the page box (trap 3), ranked by our ink minus
the lightest **live** reference's — a panel of zero ink is not a page — with each panel's raster
size beside the number. **13 minutes 28 seconds for the ten thousand** at fourteen workers, at a
load average of 23 to 33 on a 24-thread machine shared with three other rounds; 9966 of the 10 000
produce a number and 34 do not.

Two checks before anything was read, both of them somebody else's lesson taken rather than
re-learnt:

- **Both binaries built** (619): `cargo build --release -p pdf-model --example render_at` *and*
  `cargo build --release -p pdf-sandbox --bins`, with `target/release/examples/` confirmed to hold
  no stale `pdf-sandbox-worker` of its own (624, ADR 0458).
- **§20's check run first** (623): `cargo test --profile gates -p pdf-model --test fixed_documents
  -- --ignored` — **29 checked, 0 absent, 29 rows, green**, which is what says the tree under the
  instrument is the tree the last six rounds left.

## What the two ends said

The negative head is the **shallowest any chunk has produced** — the deepest row is −10.174 where
613's was −20.3, 619's −84.2 and 625's −112.6 — and the sharpest row of the ten thousand is on the
positive side for the second round running. Both defects are the same sentence in two clauses: **a
dictionary states something about its own data, and this tree would not read it.**

### `3867366.pdf` at +77.113 — a `/Decode` array a filter had been excused from

A 16-page product catalogue's cover. Silent, 135 commands, ours at 146.044 against
`pdftoppm` 68.931, `mutool` 69.675 and `gs` 69.362 — three references inside 0.75 of each other
and this tree a hundred and forty levels of ink away. Opened side by side it is a page drawn as
its own complement: a green photographic background as dark purple, a black textured header as
beige, white pipework as brown.

Its two photographs are `/JPXDecode` images with `/ColorSpace [/ICCBased …]` — a four-component
*Coated FOGRA27* profile — and `/Decode [1 0 1 0 1 0 1 0]`. The codestream is a JP2 whose `colr`
box states enumerated colour space 12, CMYK, four eight-bit components; `opj_decompress` on the
extracted stream gives the page background as `(155, 239, 92, 255)`, which read as CMYK is black
and read through the file's own array is a light green. The array is the whole difference.

**`image.rs`'s JPEG 2000 route consulted no array at all.** `jpx_samples_to_rgba` divided every
sample by 255 — with one special case for `Indexed`, whose components are not fractions — and the
module comment and §7.4.9's ledger row both said why: "`/Decode` is ignored unless the image is a
mask". That is not the condition the clause states. §7.4.9's bullet is

> If ColorSpace is absent, then the Decode array shall be ignored unless ImageMask is true

and Table 87's own `/Decode` row states it the same way round — "If the image uses the JPXDecode
filter and if ColorSpace is absent, the Decode array shall be ignored unless ImageMask is true".
The condition is `/ColorSpace`'s **absence**, not the filter. It is the one of §7.4.9's three
rearranged entries that does *not* pass to the codestream, and for a reason worth stating: a
codestream says what its samples are and never what a producer meant them to mean.

So the fix is not a special case beside §8.9.5.2's map — it is that map, which this route had
never reached. `Decode::read` is built from the declared space where the dictionary states one and
from Table 88's defaults where it does not, and `jpx_samples_to_rgba` looks each sample up in it.
Two things follow that the division could not do: an `Indexed` space needs no special case any
more, because Table 88 gives it `[0 2^n − 1]` and the map passes an index through; and a `Lab`
space's lightness runs to 100 rather than to 1, which is ADR 0464's finding one route along, on
the arm that had no witness then and still has none.

`3867366.pdf` **+77.113 → −0.449**, 146.044 → 68.482, still silent, and the page is
indistinguishable from `pdftoppm`'s.

### `3867363.pdf` at −6.915 — a `loca` record that understated its own table

A full-page statistics report drawn as a **blank sheet**: 0.225 of 255 against 7.278 / 7.139 /
8.299, one command, and *reported* — "font /F1's program has no outline for any of the 3938 code(s)
the page shows through it". A whole page of text, loudly lost.

The font is a 3254-glyph `CourierNew` subset in a `/FontFile2` that decompresses to exactly its
`/Length1`, so nothing is truncated. Its table directory says `loca` is **6510** bytes, where 3255
long offsets need 13 020. Read at the full length the bytes are a whole table: 3255 ascending
offsets whose last is exactly `glyf`'s 33 170, ending at 19 828 with `glyf` beginning at 19 832 —
the four-byte boundary after them. The record beside it is wrong in the same direction:
`hmtx`'s says 13 016 where its own `numberOfHMetrics` needs 6514. The two lengths are each
other's.

`skrifa` reads the record, finds a `loca` too short to hold `numGlyphs` entries, and produces no
outline for **any** glyph — not the glyphs past 1626, which no code on this page uses, but all of
them. That was established rather than assumed: rewriting the font's directory inside the PDF with
each of the two lengths corrected in turn, and a third copy recompressed with nothing corrected,
puts the whole defect on `loca`'s number — 0.225 with the control and with `hmtx` alone, 7.198 with
`loca` alone.

**The repair is the same shape as the two `sfnt.rs` already carries**: a file that states one fact
twice can check itself. §9.9 Table 126 sends a `/FontFile2` to the TrueType Reference Manual, which
with ISO/IEC 14496-22 defines `loca` as one offset per glyph plus a terminator — so the extent is
stated in the directory record *and* as `numGlyphs + 1` in `maxp`, and only one of the two readings
is a `loca` at all. `repaired_loca_extent` corrects the record's four bytes, and only where the
extended read ascends throughout, ends at `glyf`'s length, and lies inside the program. A run of
arbitrary bytes satisfying the first two is not something a font can produce by accident.

It goes **first** in `repaired_font_program`, because the two repairs after it read what it
corrects: `repaired_loca_order` refuses outright a record too short for its entries, and
`repaired_loca_format` measures that length to decide which format the file is in. The two
`indexToLocFormat` repairs cannot both fire — one needs a value of 0 or 1, the other a value above
1.

`3867363.pdf` **−6.915 → +0.059**, 0.225 → 7.198, 3939 commands, nothing reported.

## The populations, both measured before the change (trap 11)

With instruments that are not this tree's (trap 8) — the files' own bytes, read with a regular
expression and `zlib`.

- **`/Decode` on a `JPXDecode` image**: over the 65 944 crawled documents there are **99 031**
  `JPXDecode` image dictionaries, 98 490 of which state a `/ColorSpace` and **2298** of which state
  a `/Decode` array — every one of those 2298 beside a `/ColorSpace`, over **200 documents**. Of
  the arrays, 92 invert (`[1 0 …]` in its two spellings) and five expand a range slightly; the rest
  are Table 88's own defaults written out, including 443 `[0 255]` pairs which are all on `/Indexed`
  spaces and are that table's default at eight bits. **The curated corpora carry none at all.**
- **A short `loca` record**: the walk inflates every `/FontFile2` and reads its table directory with
  `struct`, classifying a record shorter than `numGlyphs + 1` entries by whether the bytes there
  are a `loca`. The count is in `doc/history/636`; what matters here is its shape — every hit is
  `whole`, and hits fall in archives earlier chunks ranked as well as in this one.

Both numbers say the same thing `CLAUDE.md`'s two denominators say: no gate in this tree walks a
population that carries either construct, which is why both documents go into
`doc/checks/fixed-documents.toml` rather than into a gate.

## The erratum, looked for and not found

`doc/errata-read.md`'s standing rule is that a round implementing a clause runs `spec-errata emit`
on the document *before* it writes. Over all fourteen PDFs it prints one annotation group on
§7.4.9 — Issue #29, `Completed`, replacing "except for" with "excluding" in the baseline-features
sentence — and none at all on §8.9.5.2. Table 87's own rows carry three live ones on the page this
round read (§8.9.5.1, p. 274–275): Issue #366 strikes "if a predictor function is used" from
`/BitsPerComponent`, and Issue #215 rewrites two entries around "(if present, it shall be
ignored)". **None of them touches the `/Decode` row**, whose sentence stands as EC3 prints it. Four
of the last six chunk rounds found a live erratum this way and this one did not; the negative is
recorded because "no erratum here" is a claim with a command behind it.

## What this does not do

- **It does not make `/BitsPerComponent` mean anything on this filter.** Table 87 still says it "is
  optional and shall be ignored if present", and the map is built at the eight bits `pdf-sandbox`
  normalises every codestream to. §8.9.5.2's map is linear in the sample, so composing it with that
  normalisation is the same linear map: a pair still lands D min at sample 0 and D max at the
  largest sample.
- **It does not read a `/Decode` array where `/ColorSpace` is absent.** That is the clause's own
  condition and the third test in `tests/jpx_decode_array.rs` is it: the same array beside no
  declared space changes nothing.
- **It does not read past a stated table length in general.** `repaired_loca_extent` fires on one
  table, on one arithmetic disagreement, with two structural tests on what it finds.
