# 514 — The door that did not ask the signature, and the swatch that killed a label

**Finding.** The round was pointed at the oracle's **contradicted** bucket, highest disagreement
first. Ranked the way `doc/habits.md` says to rank a suspect — **our worst measurement over the bound
it is held to** — the head of the list is not on the ranking the gate prints, and was in no group in
`oracle.rs`: `xobject-image.pdf` page 1 at **127.75×**, failing all four bounds, and
`issue5751.pdf` page 1 at **12.66×**, failing all four. Both are pages this tree *reports*, and
`check_the_ratchets` filters on `complete` — rightly, on the module's own argument — so the two
largest disagreements on the list had never been diagnosed by anything. One was a defect. The other
turned out to be three renderers failing at two different things.

**Second finding, from the entry ADR 0296 left open.** `CONTRADICTED_CALIBRATED_COLOUR` said `mupdf`
and `ghostscript` "take its components for `DeviceRGB`". Asked with a swatch carrying `issue9940.pdf`'s
own `/CalRGB` dictionary, **nobody does**: ours and `poppler`'s centre pixel is §8.6.5.3 plus IEC
61966-2-1's closed form exactly, the pair that contradicts us is fifteen levels away **in red alone**,
and the `DeviceRGB` reading would have moved all three channels. **Ten for ten on a group's name
naming a hypothesis rather than a diagnosis.**

**Date.** 2026-08-14.
**ADR.** [0349](../adr/0349-the-door-that-did-not-ask-the-signature.md).

## The nine pages, and what each turned out to be

| page | prior label | verdict | evidence |
|---|---|---|---|
| `issue5751.pdf` p1 | none — outside every group | **(a) our defect, fixed** | `/FontFile` holding a bare CFF; `01 00 04 03` + `MyriadArabic-Regular` |
| `xobject-image.pdf` p1 | none — outside every group | **(b) + (c)** | the three references' own logs: two blanks for two unrelated reasons |
| `issue9940.pdf` p1 | `CONTRADICTED_CALIBRATED_COLOUR` | **(b), label wrong** | swatch fixture; two closed forms; the page's `R − G` |
| `bitmap-symbol-context-reuse.pdf` p1 | `CONTRADICTED_SHARED_JBIG2_DECODER` | **(b), verified** | `jbig2dec`'s NYI string printed by *both* `mupdf` and `gs` |
| `function_based_shading_cmyk.pdf` p2 | `CONTRADICTED_DEVICE_CMYK_CONVERSION` | **(c), verified** | two camps: ours ≡ `poppler`, `mupdf` ≡ `gs` ≡ `hayro` |
| `bug847420.pdf` p1 | `CONTRADICTED_SUBSTITUTED_FONT` | **(c), measured** | cap `T` 77 rows against 82 |
| `bug850854.pdf` p1 | `CONTRADICTED_SUBSTITUTED_FONT` | **(c), measured** | cap `B` 110 against 117 |
| `issue6069.pdf` p1 | `CONTRADICTED_SUBSTITUTED_FONT` | **(c), measured** | cap `M` 77 against 82 |
| `issue11403_reduced.pdf` p1 | `CONTRADICTED_SUBSTITUTED_FONT` | **(c), measured** | cap `E` 99 against 105 |

## The fix, and why it is one line of decision

`pdf_font::program`'s first paragraph has always said the reader is chosen "by the bytes' own
signature rather than by the key's spelling", and §9.9's ledger row asserted it of all three of
Table 124's keys. **`/FontFile2` and `/FontFile3` asked. `/FontFile` did not.** It took the key at
its word and handed every stream to the Type 1 reader — so `issue5751.pdf`, whose descriptor is a
`CIDFontType0`'s and whose stream is a bare CFF, was `InvalidFontFormat` and the page drew nothing
where four renderers draw *Open Access*.

The file states three things that disagree with the key it wrote: Table 124 gives `/FontFile` "Type
1 font program, in the original (noncompact) format" for "a Type1 or MMType1 font dictionary", Table
125 makes `/Length1`, `/Length2` and `/Length3` "Required for Type 1 font programs" and the stream
carries none of them, and §9.7.4.2 puts a Type 0 CIDFont's CFF under `/FontFile3`. The bytes are the
only one of the four a producer cannot fake. Neither of Type 1's own packagings can collide with the
test — a PFA begins `%!`, a PFB `80 01` — so the change reroutes a program that could not be read at
all and leaves every readable one where it was.

**This is the row-describing-what-the-code-should-do failure** `CLAUDE.md` names, caught by the
instrument rather than by a re-reading: the ledger's sentence was the design and two doors of three
implemented it.

## What `xobject-image.pdf` is, in the references' own words

```text
poppler  Syntax Error (1274): Missing 'endstream' or incorrect stream length
         Syntax Error: Unknown operator 'endstream'
mupdf    warning: PDF stream Length incorrect / warning: padding truncated image
gs       Incorrect /Length for stream object ... recoverable image error ... bad DecodeParms
```

Its content stream is 33 bytes under a `/Length 14` that stops mid-`cm`, and its image XObject says
`/Width 200 /Height 100` over a **1 × 1** JPEG. So `poppler` never reaches the image — its blank page
is about Table 5's `/Length` and nothing else — while `ghostscript` repairs the stream and refuses
the image, and `mupdf` repairs both and pads the samples to the dictionary's grid, which makes the
visible corner (source rows 75 to 99) entirely pad and therefore black. Three renderers, three
pictures, no two failing at the same thing; the "consensus" that contradicts us is two blanks with
unrelated causes. §8.9.3 and Table 87 make the dictionary's dimensions required, §7.4.8 puts the
codestream's in the encoded data, and neither says which governs when a file contradicts itself.
This tree draws the codestream's samples and reports the contradiction beside them (ADR 0340).

## The swatch, which needed no page of the corpus

A 100 × 100 fixture filled with `0.5 0.25 0.75 sc` in `issue9940.pdf`'s own `/CalRGB` dictionary.
Both readings are arithmetic from the file — §8.6.5.3's decoding gives `X Y Z = (0.20686, 0.04737,
0.57830)`:

| | centre pixel |
|---|---|
| **the closed form, §8.6.5.3 + XYZ → sRGB** | **(151, 0, 205)** |
| ours | (151, 0, 205) |
| `poppler` | (151, 0, 205) |
| `mupdf` | (166, 0, 205) |
| `ghostscript` | (166, 0, 207) |
| **the closed form, components as `DeviceRGB`** | **(128, 64, 191)** |

The page agrees over 484 704 pixels: per-channel means give `R − G` of −2.05 for ours, `poppler` and
`hayro` and +2.02 and +1.72 for `mupdf` and `ghostscript`, with green and blue agreeing across all
five to 0.6 of 255. **One channel moves**, which is not what taking three components for `DeviceRGB`
looks like. The page joins `CONTRADICTED_CALRGB_TO_SCREEN` and its mechanism is §10.3.1's sentence
putting the destination space "beyond the scope of this document".

## Two things a band of rows got wrong before a crop got them right

`CONTRADICTED_SUBSTITUTED_FONT`'s cap-row table had four empty cells — four of the seven sans pages
carrying the `/CapHeight` diagnosis on their `/BaseFont` and their ink alone. All four are filled and
all four land on 0.6875/0.729167 of the reference's to within half a row, on the same baseline, short
only at the top. But `issue6069.pdf`'s *whole-line* box is 106 rows against 107 — no difference at
all, because the line's tallest ink is an ascender and an `i` dot — and `issue11403_reduced.pdf`'s
leading `2.` reads 101 against 104, a ratio that fits nothing because a digit's height is not a cap
height in either face. **A page-level box cannot test a per-glyph metric** (ADR 0174's lesson in a
new instrument), and `doc/todo/00`'s own warning about a band of rows is what the two cost.

One thing seen while cropping and worth keeping: on `issue11403_reduced.pdf` **`mupdf` draws a stray
acute accent** 32 device columns left of the line, so the pair the gate calls agreement there differs
by a mark one of them invented.

## Gates

Every one green, on the sequence `doc/todo/02` §2 states, with the pixel-changing fix in place.
`cargo fmt --all --check` clean; `cargo clippy --workspace --all-targets` silent of lints. **1862
tests**, 15 skipped, two of them this round's. Doctests: 24 packages ok, `pdf-spec`'s one real
doctest passes.

| gate | verdict |
|---|---|
| corpus | 974 documents in 11.1 s: 0 unopenable, 8 locked, 2 encrypted beyond us, 6 pageless, **61 incomplete**, 0 slow |
| oracle | 1794 pages (1694 complete, 100 incomplete): **906 agrees, 67 contradicted, 786 ambiguous**, 19 no render |
| text extraction | **10969/11163 matched words in bounds (98.26%)**, 508 of 974 judged; PDFBox half passes |
| dates | 1545 strings, 1514 conform to §7.9.4 (97.99%) |
| xmp | 319 documents carry the stream, 318 read, 3191 properties |
| jpeg 2000 | 14 codestreams byte-identical to OpenJPEG |
| quorra | 956 pages: 917 agree, 37 differ, 2 refused, 18 not comparable |
| conformance | 5 tests pass |

**One page of 1794 moved and the oracle says so line by line.** The gate was run before the change
and after it, and `diff` over the per-page lines has exactly one entry: `issue5751.pdf page 1` gone
from the contradicted list. Every other page's verdict and every printed metric is byte-identical.
Corpus incomplete went 62 → 61 and the judged extraction set 507 → 508, both the same document.

**`doc/todo/00` step 7 was re-run whole**, because a round that changes what gets drawn owes it. All
786 ambiguous pages: twenty at or past −1, sixteen of them documents this tree calls incomplete, and
on the complete documents `issue16038.pdf` −5.734, `issue12295.pdf` −2.823, `issue14297.pdf` −1.145,
`issue7821.pdf` −1.000 and nothing else past −0.840 — the same four names in the same order the
four-hundred-and-forty-fourth recorded, all diagnosed. Nothing unexplained.

**The reference cache was the main checkout's, read and written**, which is a deliberate choice and
not a shortcut: entries are content-addressed on the invocation, the renderer's version and the
document's SHA-256, and every write goes to a temporary name carrying the writer's process id before
being renamed into place — so a second round sharing it can neither read a stale entry nor see a
half-written one. It cost nothing and saved about a thousand seconds of `pdftoppm`, `mutool` and
`gs`; the hit rate printed 99.8%, which is trap 10a's tell reading correctly on a warm cache.

## §5, run here and owed again on `main`

The six binaries and `libviewer_ffi.so` are built in release and installed into this worktree's
`target/`. As in the four-hundred-and-eleventh's note, they are **not** the ones a person's shell
finds: this round's build directory is the worktree's, so §5 for `main` belongs to whoever merges
this, along with §2's whole sequence.

**Touched.** `crates/pdf-font/src/program.rs` (the signature test on `/FontFile`, and two tests),
`crates/pdf-model/tests/oracle.rs` (`CONTRADICTED_ON_A_PAGE_WE_REPORT` and its staleness check;
`CONTRADICTED_CALIBRATED_COLOUR` emptied and corrected; `issue9940.pdf` into
`CONTRADICTED_CALRGB_TO_SCREEN`; the four cap-row cells; the JBIG2 re-check; the CMYK group's stale
header count), `doc/conformance/ledger.toml` (§9.9), `doc/HANDOVER.md` (trap 1's tally),
`doc/oracle-and-corpus.md` (§3b's two rankings, §3c's tally), `doc/todo/00-ambiguous-bucket.md`
(step 7's run), `doc/adr/0349-*` (new), this file.
