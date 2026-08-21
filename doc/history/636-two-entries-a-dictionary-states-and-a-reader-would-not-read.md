# 636 — Two entries a dictionary states, and a reader that would not read them

`doc/todo/03`'s chunk again, over the SafeDocs crawl, for the seventh round running. Ten whole
archives, and two defects that are one sentence in two clauses: **a dictionary says something
about its own data, and this tree would not read it.** One is a `/Decode` array a filter had been
excused from; the other is a `loca` record a font understated. Both were found by reading the head
at *both* ends, and the sharper of the two is on the positive side for the second round running.

Date: 2026-08-21.
ADR: [0468](../adr/0468-two-entries-a-dictionary-states-and-a-reader-would-not-read.md).

Touched: `crates/pdf-model/src/image.rs` (`jpx_decode` and `jpx_stencil` factored out of
`decode_jpx`, `jpx_samples_to_rgba`, the module comment's false claim),
`crates/pdf-font/src/sfnt.rs` (`repaired_loca_extent`, `repaired_font_program`),
`crates/pdf-font/src/truetype.rs` (three tests and the `understate` fixture helper),
`crates/pdf-model/tests/jpx_decode_array.rs` (new), `doc/conformance/ledger.toml` (§7.4.9,
§8.9.5.2, §9.6.3), `doc/checks/fixed-documents.toml` (two rows), `doc/todo/03` §24, the ADR and
this file.

## The chunk

**`1407`, `1899`, `2514`, `2637`, `3006`, `3744`, `3867`, `4728`, `5343`, `5958` — 10 000
documents**, none of the forty-two archives sessions 603, 613, 615, 619, 625 and 631 ranked. An
archive is a hash bucket (ADR 0261), so any set is unbiased. 603's instrument reused rather than
rewritten, at **13 minutes 28 seconds** for the ten thousand on fourteen workers — at a load
average between 23 and 33, because three other rounds were compiling on the same 24 threads. 9966
rows produce a number; 34 do not.

**Checked before it was trusted.** Both binaries built (619's lesson), `target/release/examples/`
confirmed to hold no worker of its own (624's), and §20's own check run first:
`cargo test --profile gates -p pdf-model --test fixed_documents -- --ignored` — **29 checked, 0
absent, 29 rows, green** — which is what 623 paid for and this round's first command.

## The two defects

**`3867366.pdf` +77.113**, the top row of the ten thousand and silent, at 146.044 against 68.931 /
69.675 / 69.362 — three references inside 0.75 of one another. Side by side it is a product
catalogue cover drawn as its own **complement**: a green photographic background as dark purple, a
black textured header as beige, white pipework as brown. Its two photographs are `/JPXDecode`
images with `/ColorSpace [/ICCBased …]` — a four-component *Coated FOGRA27* profile — and
`/Decode [1 0 1 0 1 0 1 0]`. `opj_dump` says the JP2's `colr` box states enumerated space 12, CMYK;
`opj_decompress` gives the background sample as `(155, 239, 92, 255)`, which read as CMYK is black
and read through the file's own array is the green.

`jpx_samples_to_rgba` divided every sample by 255 and consulted no array, and the module comment
and §7.4.9's ledger row both said why — "`/Decode` is ignored unless the image is a mask". **That
is not the condition the clause states.** The bullet is *If ColorSpace is absent, then the Decode
array shall be ignored unless ImageMask is true*, and Table 87's own `/Decode` row says the same:
the condition is `/ColorSpace`'s **absence**, not the filter. It is the one of §7.4.9's three
rearranged entries that does not pass to the codestream, for a reason worth stating — a codestream
says what its samples are and never what a producer meant them to mean. So the route goes through
§8.9.5.2's map like every other, which also carries a space's own units where the division could
not: an `Indexed` space's default is `[0 2^n − 1]` so an index passes through, and a `Lab`
lightness runs to 100, which is ADR 0464's finding one route along on the arm that had no witness.
**+77.113 → −0.449**, 146.044 → 68.482, still silent, and the page is indistinguishable from
`pdftoppm`'s.

**`3867363.pdf` −6.915**, and *reported*: a full-page statistics report drawn as a blank sheet at
0.225 against 7.278 / 7.139 / 8.299, one command, "font /F1's program has no outline for any of
the 3938 code(s) the page shows through it". The font is a 3254-glyph `CourierNew` subset whose
`/FontFile2` decompresses to exactly its `/Length1`, so nothing is truncated. Its directory says
`loca` is **6510** bytes where 3255 long offsets need 13 020 — and `hmtx`'s record carries the
length `loca` should have had, the two lengths being each other's. Read at the full length the
bytes are a whole table: 3255 ascending offsets whose last is exactly `glyf`'s 33 170, ending at
19 828 with `glyf` at 19 832, the four-byte boundary after them.

**Which record was the blocker was established rather than assumed** (621's lesson): the font's
directory was rewritten inside the PDF with each length corrected in turn, plus a control
recompressed with nothing corrected. Control 0.225, `hmtx` alone 0.225, `loca` alone 7.198. The
first attempt at that experiment was itself wrong — a slicing bug wrote `streamstream` into the
object and every variant "passed" on a substituted font — which is trap 1 one directory over and
the reason the control exists.

`sfnt.rs` gains a third repair beside its two, on the same derivation both of those rest on: **a
file that states one fact twice can check itself.** §9.9 Table 126 sends a `/FontFile2` to the
TrueType Reference Manual, which with ISO/IEC 14496-22 defines `loca` as one offset per glyph plus
a terminator — so the extent is in the directory record *and* in `maxp`, and only one reading is a
`loca`. `repaired_loca_extent` corrects the record's four bytes, and only where the extended read
ascends throughout, ends at `glyf`'s length and lies inside the program. It runs **first**, because
the two after it read what it corrects. **−6.915 → +0.059**, 3939 commands, nothing reported.

## What moved

**The reach is measured over our own panel** (631's rule: a reference's panel cannot depend on our
build and has been measured to differ between runs), before and after, across all **52 archives any
chunk round has ranked plus the 243 documents the two censuses name — 52 043 documents**, with the
fixes reverted by patch for the before pass and re-applied for the after.

**Nine rows move, and every one moves toward agreement** when put back in front of the three
references:

| document | ours before → after | references | gap after |
|---|---|---|---|
| `3867/3867366.pdf` | 146.044 → 68.482 | 68.931 / 69.675 / 69.362 | −0.449 |
| `1530/1530098.pdf` | 100.877 → 53.664 | 54.365 / 53.178 / 53.806 | +0.487 |
| `7065/7065048.pdf` | 190.560 → 167.629 | 168.497 / 165.747 / 165.988 | +1.882 |
| `1038/1038753.pdf` | 191.293 → 181.834 | 182.821 / 181.398 / 181.503 | +0.437 |
| `3867/3867363.pdf` | 0.225 → 7.198 | 7.278 / 7.139 / 8.299 | +0.058 |
| `6696/6696167.pdf` | 0.225 → 6.571 | 6.592 / 6.463 / 7.482 | +0.108 |
| `3867/3867814.pdf` | 30.034 → 27.155 | 27.661 / 27.676 / 27.173 | −0.017 |
| `3375/3375814.pdf` | 91.081 → 88.364 | 88.832 / 88.318 / 88.411 | +0.046 |
| `3129/3129989.pdf` | 54.8670 → 54.8671 | 61.840 / 55.637 / 56.081 | −0.770 |

**Five are in archives an earlier chunk took** — `1530` and `3129` are 625's, `1038` is 631's,
`6696` and `3375` are 615's — the seventh round running that a fix has reached back; a sixth,
`7065048.pdf`, is in an archive no chunk has ranked and reached the panel only because the census
named it. **One of the five is §21's own open lead**: `1530098.pdf` is listed there among "five
silent rows diagnosed no further than their numbers" at +47.699, and it is this `/Decode` defect.
The last row moves by one ten-thousandth — an `Indexed` space's `[0 255]` array now going through
the map instead of the old special case — which is the change being the identity where the clause
says it is.

`decode_jpx` was refactored afterwards to satisfy `too_many_lines`, and all nine documents were
re-measured against the refactored binary: identical to the ten-thousandth.

## The two populations, both measured before the change

Trap 11's rule, with instruments that are not this tree's (trap 8) — the files' own bytes, read
with a regular expression and `zlib`.

- **`/Decode` on a `JPXDecode` image**: **99 031** such image dictionaries over the 65 944 crawled
  documents, 98 490 stating a `/ColorSpace` and **2298 stating a `/Decode` array** — every one of
  those beside a `/ColorSpace` — over **200 documents**. 92 of the arrays invert; 443 are `[0 255]`
  and all of those sit on `/Indexed` spaces, which is Table 88's own default at eight bits. **The
  curated corpora carry none at all.**
- **A short `loca` record**: over the crawl *and* `doc/corpora` and `doc/pdf.js` — 66 920 files —
  **62 short records in 6 documents**, 59 of them whole tables. The two curated documents are the
  **negative** cases: `bug868745.pdf`'s extended read descends and `issue14618.pdf`'s runs outside
  the program, so the repair declines both. No gate in this tree could have shown either defect,
  and no gate can move because of the fix.

## The erratum, looked for and not found

`doc/errata-read.md`'s standing rule is `spec-errata emit` before writing, and this round ran it
over all fourteen documents. §7.4.9 carries one annotation group — Issue #29, `Completed`,
"except for" → "excluding" in the baseline-features sentence — and §8.9.5.2 carries none. Table
87's page has three live errata (#366 striking "if a predictor function is used", #215 rewriting
two entries around "(if present, it shall be ignored)") and **none touches the `/Decode` row**.
Four of the last six chunk rounds found a live erratum this way; this one did not, and the negative
is recorded because "no erratum here" is a claim with a command behind it.

## Gates

The full §2 sequence, because the change is in `pdf-model` and `pdf-font`. `RUSTFLAGS="-D
warnings"` on the clippy line, which caught five lints this round's own additions introduced —
a fixture helper taking a four-byte array by reference and doing unchecked arithmetic, a
`too_many_lines` on `decode_jpx` that the two extractions answered, a `doc_markdown` on a
quotation, and a `panic!` in a new test file. The conformance gate caught two blockquotes twice
over: one whose sentence ended with a full stop the standard does not print, and one whose
condition clause the standard spells `true ,` with a space, which is why the quotation is now the
half of the sentence that survives verbatim.

**§5's binaries were not installed.** This is a worktree round whose `target/` is not the one a
person runs, and the round measured nothing that needs them — the merge owns that line.

## What the head still holds

`doc/todo/03` §24 has it in full. The one worth naming here is the only row of this head that is
neither an image nor a font: **`1407194.pdf` −6.304**, a `/Text` annotation with `/Rect [0 542 400
792]` and no `/AP`, whose synthesised icon this tree draws at the whole 400×250 rectangle where
three references draw a small note at its corner. §12.5.6.4 makes such an annotation behave "as if
the NoZoom and NoRotate annotation flags … were always set", and §12.5.3's NoZoom is about
magnification rather than about `/Rect`; what a synthesised icon does with an oversized rectangle
is a question this round did not settle, and ADR 0109 is where the icon's artwork was decided.
