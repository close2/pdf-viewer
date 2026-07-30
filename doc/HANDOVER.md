# Handover

Written 2026-07-26, updated 2026-07-30 at the end of the **thirtieth** working session. Read
`/CLAUDE.md` first — it holds the five non-negotiable principles, what *done* means, and the
closed list of exclusions. **Principle 5 is the one that changes how to work**: the specification
is the only source of truth, and agreement with poppler, mupdf or pdf.js is evidence that we read
it right, never the definition of right. `doc/PLAN.md` holds the phases and the conformance
ledger's design; `doc/adr/` holds every decision's argument. **This file is only the state of
play, the traps, and what to do next** — where something is also written there, this is a pointer.

Each session's own reasoning lives in its ADR. This file keeps a lesson exactly once: in a trap
if it changes how you write code, in "Habits" if it changes how you work, and in the numbers if
it is a fact about today.

## What the thirtieth session changed

**A font program brings an encoding with it, and for an embedded program that encoding is
the base.** §9.6.5.1's Table 112 answers `/BaseEncoding`'s absence three ways and only the
last two turn on the Symbolic flag: "For a font program that is embedded in the PDF file,
the default base encoding shall be the font program's built-in encoding". `simple_code_table`
is reached *only* by an embedded bare CFF and asked the flag anyway, giving a nonsymbolic font
`StandardEncoding` — the rule for a font it would be substituting. It no longer takes a
descriptor at all. ADR 0039.

**`/MissingWidth`'s default is 0 and this tree had it as half an em.** Table 120 states the
default; Table 109 sends every code outside `/FirstChar`..`/LastChar` to it. `issue7439.pdf`
shows code 2 six times against a `/FirstChar` of 3, so six half-ems of invented space opened
between `Issue` and `7439`. The page was contradicted and now agrees.

**The oracle entry blaming `issue20232.pdf`'s contradictory `/Flags` was wrong, and §9.8.2
says why it had to be.** "The use of the two flags to represent a single binary choice is a
historical accident. A PDF processor should always check the Symbolic flag" — which is what
`is_symbolic` does. The page is blank there because the (3, 0) subtable's glyph 34 is one of
the 158 glyphs that subset embeds with **no outline**; only 0 and 90 have a contour, and 90 is
the `Ccedilla` the `/Differences` names, in the array §9.6.5.4 says to ignore. Seventh for
seventh on a contradicted page's label being a hypothesis rather than a diagnosis.

**One of the two pages that left the contradicted list is not a fix, and the digest proves
it.** `issue3566.pdf`'s raster is byte-identical before and after; what moved is which *bound*
judged it. Nothing could name what its symbolic bare CFF drew, so `has_text` was false and a
page that is nothing but the word `different` was held to the tolerance measured on flat
fills. Giving it the program's own glyph names made the readback work. **That is the second
witness for a known defect in the instrument**: `has_text` asks whether we could name what we
drew, and what it means to ask is whether we drew glyphs at all.

| | was | is |
|---|---|---|
| **an embedded CFF's default base encoding** | `StandardEncoding` when the descriptor is nonsymbolic | the program's own, as Table 112 says for an embedded program |
| **a code with no `/Widths` entry** | half an em, a preference | `/MissingWidth`, default 0, as Table 120 says |
| **`issue20232.pdf`'s entry** | "a font that claims both flags leaves that route unreachable" | the route is taken and its glyph is empty; §9.8.2 chose the flag |
| **§9.6 and §9.8's 14 `unreviewed` rows** | nobody had read them | 4 implemented, 8 partial, 2 silent |

**The numbers:**

| | before | now |
|---|---|---|
| corpus documents drawing with nothing reported | 823 | **823** |
| **pages agreeing with the reference consensus** | 758 | **760** |
| **pages contradicted by it** | 101 | **99** |
| contradicted pages with nothing to explain them | 59 | **58** |
| ledger subclauses nobody has read | 420 | **406** |
| ledger rows that are `silent` | 1 | **3** |
| cited clauses still owing a review | 4 | **4** |
| `§` citations the checker verified | 1250 | **1267** |
| rustdoc quotations checked verbatim | 124 | **127** |
| tests | 558 | **566** |

What it taught:

- **A default stated in a table is not a suggestion, and a comment arguing for a nicer one is
  a preference wearing a reason.** "Spacing degrades gracefully rather than collapsing to
  zero" is true and has nothing to do with what the clause says. A producer who wants half an
  em can write half an em.
- **When two subclauses each condition a branch on one of two flags, the clause that defines
  the flags is where the tie is broken.** §9.6.5.4 cannot decide a font that sets both;
  §9.8.2 says a processor "should always check the Symbolic flag" and calls the pair a
  historical accident.
- **A page can leave the contradicted list without a pixel moving.** Check the raster's digest
  before writing "fixed" — the oracle picks a page's tolerance class from what we could *name*,
  so a change to text extraction is a change to the bound.
- **Read a rule for which of its cases you are actually in.** Table 112's sentence about
  embedded programs and its sentence about the Symbolic flag are alternatives, and this code
  can only ever be in the first.

## How the project got here

One line per session; the argument is in the ADR, and every durable lesson is in Traps or Habits
below rather than here.

| Session | What landed | Where the reasoning is |
|---|---|---|
| 5 | The reference oracle, over every page of the corpus | ADR 0011 |
| 6 | `CalGray`/`CalRGB` through XYZ; annotation appearance streams | ADRs 0012, 0013 |
| 7 | JBIG2 and JPEG 2000, in a sandboxed worker; the first speed comparison | ADR 0014 |
| 8 | §9.6.5.4, the `TrueType` code-to-glyph algorithm, in full | ADR 0015 |
| 9 | The conformance ledger and citation checker; optional content | ADRs 0016, 0017 |
| 10 | Type 3 fonts; dashed lines, which had never been dashed | ADR 0018 |
| 11 | Inline images; `/Interpolate`; `Indexed`, `Separation` and `DeviceN` images | ADR 0019 |
| 12 | A cache for the oracle's reference renders; `CCITTFaxDecode`; `/Rotate` | ADRs 0020, 0021 |
| 13 | All eight text rendering modes; §9.3 and §9.4 reviewed; table numbers checked | ADR 0022 |
| 14 | `/Mask` in both its forms; §11.6.4 reviewed; §9.3.8 reports | ADR 0023 |
| 15 | Soft masks at any resolution and `/Matte`; §11.3.7, §11.5, §11.6 reviewed; a shading carries `ca` | ADR 0024 |
| 16 | Area averaging for reduced images; §10.7 reviewed, and it forbids what was built | ADR 0025 |
| 17 | Transparency groups; §8.10 and §11.4 reviewed; the page group is isolated | ADR 0026 |
| 18 | Soft masks in an `/ExtGState`; §11.7 reviewed, and overprinting is silent | ADR 0027 |
| 19 | `/SA` and the device's thinnest line; §8.6.6 and §8.6.7 reviewed, and overprinting is *not* a gap | ADR 0028 |
| 20 | Embedded `CMap`s and `/CIDToGIDMap`; the whole of §9.7 reviewed | ADR 0029 |
| 21 | Constructed annotation appearances; the whole of §12.5 reviewed; `/CA` belongs to the construction | ADR 0030 |
| 22 | Encryption, every revision and method §7.6 states; the whole of §7.6 reviewed; a locked file is not an unreadable one | ADR 0031 |
| 23 | §12.7.4.3's variable text; §12.7.4, §12.7.5 and §7.9.2 reviewed; regenerating an appearance is a splice | ADR 0032 |
| 24 | §8.5.3.2's degenerate strokes and zero-length dashes; the whole of §8.4 and §8.5 reviewed; an empty clipping path admits nothing | ADR 0033 |
| 25 | §8.9.5.2's `/Decode` array in full, Table 88 included; the whole of §8.6.5 reviewed; a fast path inherits no clauses | ADR 0034 |
| 26 | An image's colour space is a fill's; §8.6.4 reviewed; an exact memo where a lookup grid was the obvious answer | ADR 0035 |
| 27 | `LZWDecode`, the last standard filter; the whole of §7.4 reviewed; a corpus stating an invariant about itself | ADR 0036 |
| 28 | A shading's `/BBox`; the whole of §8.7 reviewed; a contradicted page's diagnosis refuted by measuring it | ADR 0037 |
| 29 | `/UserUnit`, and the geometry list emptied; the whole of §7.7 reviewed | ADR 0038 |
| 30 | An embedded program's own encoding is the base; `/MissingWidth` is 0; §9.6 and §9.8 reviewed | ADR 0039 |

The contradicted count has gone 174 → 120 → 108 → 106 → 104 → 108 → 103 → 103 → 104 → 103 → 100
→ 93 → 96 → 96 → 98 → 102 → 102 → 102 → 102 → 102 → 102 → 102 → 101 → 101 → 99 across sessions
6 to 30, and the corpus's incomplete count 291 → 368 → 250 → 290 → 283 → 263 → 251 → 235 → 232 →
231 → 231 → 237 → 220 → 220 → 189 → 147 → 137 → 129 → 130 → 130 → 130 → 130 → 130 → 130 → 130.
Both move in both directions on purpose: a rise in the first can mean pages *joined* the
comparison, and a rise in the second is honesty when a silence ends. The sections below say
which.

## Where we are

A PDF **renderer** that opens real files and draws pages: geometry, colour, images, shadings,
patterns, embedded text, transparency groups, soft masks, and annotations both from their stored
appearance streams and constructed where the standard states one — on a CPU and a GPU backend,
with JBIG2 and JPEG 2000 decoded in a confined worker, encrypted files decrypted at every
revision and method §7.6 states, and **a form field's value laid out from its `/DA` string**. It
is not yet a PDF *viewer* in the full sense — nothing edits a field, follows a link or asks a
person for a password — and the gap is measured below rather than guessed at.

- **566 tests**, `clippy` clean under `pedantic` + `unwrap_used`/`panic`/`arithmetic_side_effects`,
  `cargo fmt --check` clean, `cargo deny` clean on all four checks — verified by running them, not
  assumed. (The thirteenth session found this line had been *wrong*: eleven warnings had
  accumulated because `allow-panic-in-tests` does not reach an integration test's helper
  functions.)
- **The 14 specification PDFs in `doc/`** — including ISO 32000-2 itself, 1023 pages and 101 318
  objects — all parse, all render page one with **nothing reported at all**, and all extract
  **100% of the words `pdftotext` finds**.
- **The 974-document pdf.js corpus is a gate, not a survey.** All 974 open except ten that are
  encrypted — 8 waiting for a password, 2 by something §7.6 does not specify or we do not
  implement — 953 reach page one, **823 draw with nothing reported**, and everything the other 130
  cannot draw is named. 1501 of 1501 PDF functions parse; all 1793 shadings build, mesh types
  included. The whole gate runs in **~2 s** with no named slow document left. Counts are
  ratcheted.
- **A second gate asks whether what we drew is *right*.** `oracle.rs` compares us against poppler,
  mupdf and ghostscript over **1794 pages** — every corpus page plus page one of each
  specification PDF — in **~26–33 s**, because the references' renders are remembered between runs
  (ADR 0020). Of the 1620 pages we claim to draw completely, **760 agree with the reference
  consensus, 99 are contradicted and 751 are pages the references cannot agree about among
  themselves**. The 99 are named, grouped and ratcheted in both directions. Twenty-five pages
  do not rasterise at all: 13 documents that have no such page, 10 encrypted ones, and 2 whose
  target size is degenerate or past the pixel limit. **None is a page we decline to draw** —
  the last four of those left in the twenty-fourth session (ADR 0033). ADR 0011.
- **JBIG2 and JPEG 2000 decode in a sandboxed worker.** `pdf-sandbox` confines it with resource
  limits, Landlock and a seccomp-BPF allow-list; `--no-sandbox` turns it off for trusted documents
  and says what that costs. The strongest evidence the decode is right is not a reference
  renderer: the corpus encodes **one image ninety-six ways** and all ninety-six produce
  byte-identical pixels. ADR 0014.
- **Colour resolves from the document.** `ICCBased` profiles are evaluated by an A2B evaluator
  written here, `CalGray`/`CalRGB`/`Lab` convert through XYZ, `/DefaultCMYK` and output intents
  are honoured, and there is exactly one route from XYZ to a pixel and one `DeviceCMYK`
  conversion. ADRs 0009, 0012.
- **A composite font is a `CMap` and a `CIDFont`, and both are read.** §9.7 in full except for
  data: an embedded `CMap` stream decides how many bytes each code takes and which CID it selects,
  byte by byte against its codespace ranges, with §9.7.6.3's recovery for a code that matches
  none; a CID reaches a glyph through a CID-keyed CFF's charset or a `/CIDToGIDMap` stream. What
  is left is Table 116's predefined `CMap`s (registered data files, so a licensing question) and
  vertical writing (§9.2.4's `/W2`). The parser is fuzzed on the property that matters: a `CMap`
  that consumed zero bytes per code would hang a page. ADR 0029.
- **An encrypted document is decrypted, and a locked one says so.** §7.6's standard security
  handler at revisions 2, 3, 4 and 6 over `/V` 1, 2, 4 and 5, with `V2`, `AESV2`, `AESV3` and
  `Identity`; every one of the clause's numbered algorithms is written out against its own
  subclause. `Document::open` tries the empty password §7.6.4.1 requires first and returns
  `PasswordRequired` when that fails, which is a *locked* file rather than an unreadable one.
  Refused by name: `/R 5`, which Table 21 says "shall not be used" and states no algorithm for;
  §7.6.5's public-key handlers; and a revision 4 password outside the range where PDFDocEncoding
  and Unicode provably agree. ADR 0031.
- **A glyph may be a content stream** — Type 3 fonts (§9.6.4), read in `pdf-model` because drawing
  one means running the interpreter. ADR 0018.
- **Every standard filter a PDF may name decodes** — all ten of Table 6's, `LZWDecode` last
  (§7.4.4.2, ADR 0036) and `CCITTFaxDecode` before it (§7.4.6, ADR 0021). No corpus page one
  reaches an LZW stream, which is why it took the specification track to get there. An image
  may be written into the content stream (§8.9.7, ADR 0019).
- **An image is masked every way §8.9.6 and §11.6.5.2 define**: its own stencil, an explicit
  `/Mask`, a colour-key `/Mask` (ADR 0023), and an `/SMask` of any size (ADR 0024), combined on
  the finer of the two grids — a documented choice, since the clause puts both on the unit square.
  §11.6.4.3's precedence is honoured and Table 144's `/Matte` is undone where the arithmetic is
  exact.
- **A reduced image is averaged**, by `Image::area_averaged` — a **documented departure from
  §10.7.4**, which requires point sampling and says "there shall not be averaging over the pixel
  area". §10.7.1 licenses it, this tree already takes two others in the same subclause by
  anti-aliasing at all, and the page that argues for it is otherwise illegible. ADR 0025.
- **A `/Group` is composited as one object** (§11.6.6), with the blend mode and both alpha
  constants reset *inside* it, and **the page is a group too, an isolated one** (§11.4.7) — so a
  page is drawn onto transparency and the medium's white imposed on the result. Non-isolated
  groups that blend, knockout groups, and a blending space that is not the device's are reported.
  ADR 0026.
- **A soft mask is a group evaluated for its opacity** (§11.5): positioned by `/Matrix` and the
  transform at the `gs`, rasterised by each backend at its own target, with `SoftMask::value` the
  one place rendered pixels become mask values — because §11.5.3's coefficients are not the
  luminance either rasteriser offers. ADR 0027.
- **Annotations draw, and are constructed where they have to be.** `/AP /N` is placed by §12.5.5's
  algorithm; an annotation without one gets a content stream built from its subtype's clause, or a
  report naming what the clause does not state. ADRs 0013, 0030.
- **A field's value is laid out from its `/DA` string** — §12.7.4.3 in full for the two field
  types that hold text, with quadding, auto-sizing, wrapping, comb cells and a password field's
  bullets. A stored appearance under `/NeedAppearances` is *spliced* rather than rebuilt: the
  clause replaces the stream "from … BMC to the matching EMC" and everything outside that pair
  survives. Refused by name: a `/DA` font `/DR` does not define, a composite `/DA` font, and
  §12.7.5.4's list-box selection, for which the clause states no appearance. ADR 0032.
- **A text string is decoded as §7.9.2.2 defines one** — UTF-16BE with surrogate pairs, UTF-8, or
  Annex D Table D.3's `PDFDocEncoding`, with §7.9.2.2.2's language escapes removed. The table is
  compiled in and is *not* ISO Latin 1.
- **A layer the document turns off is not drawn** — §8.11 in full as far as it decides what is
  marked, including `/VE` visibility expressions. ADR 0017.
- **One device pixel is the thinnest line, and both backends agree what that means.**
  `Stroke::device_width` is §8.4.3.2's zero-width minimum and §10.7.5's stroke adjustment in one
  function. ADR 0028.
- **A stroke that spans no distance still marks the page** — §8.5.3.2's degenerate subpaths are
  filled circles under round caps and nothing under the other two, a zero-length dash paints
  every cap oriented along the path, and both rules are `pdf-render`'s rather than either
  rasteriser's. So is what an empty clipping path admits, which is nothing (§8.5.4). ADR 0033.
- **Overprinting is ignored, and §8.6.7 is what says to ignore it**: this device has three
  additive colourants and no separations, and both §8.6.7 and §11.7.4's Table 146 reach the same
  answer — the special blend function is the source colour, which is Normal. The one configuration
  that would differ is a `DeviceCMYK` group space, which §11.6.6 already reports. `/Separation`
  `/All` and `/None` are honoured before the tint transform is parsed. ADR 0028.
- **The citations are checked.** `tools/conformance` holds every `§` in the tree to a clause the
  standard has — 1267 of them — every rustdoc blockquote to the standard's own words, and the
  ledger's 823 rows to the standard's subclauses. It prints the title of every table the tree
  cites, which is how the twentieth session found six comments calling Table 57 "Table 58". ADR
  0016, `doc/PLAN.md` §5a.
- Both backends draw everything the display list can express and agree on it: **fourteen**
  headless GPU scenes hold `tiny-skia` and Vello to the same pixels at more than one scale and
  along both axes (see trap 2), plus one single-pixel test, `vello_hands_back_straight_alpha`.

### Run it

```sh
cargo run --release -p viewer-ui --bin pdf-viewer -- doc/PDF20_AN001-BPC.pdf
```

Arrow keys / Page Up / Down / Space turn pages, Home and End jump to the ends, Escape quits. The
title bar names anything on the page that could not be drawn. `--no-sandbox` decodes JBIG2 and
JPEG 2000 in the viewer's own process — faster by a process spawn and a pipe round trip,
appropriate for documents whose origin you trust, and it prints a line saying what it gave up.

### Verify it

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets     # must be silent of lints
cargo test --workspace
# The conformance gate is part of that run; its summary is worth reading rather than only passing.
cargo test -p conformance -- --nocapture   # 1267 citations, 127 quotations, 77 tables, 823 rows
cargo run -p conformance --bin ledger      # regenerates the rows, keeps every status
# Both gates decode images in a separate program, and -p pdf-model does not rebuild another
# package's binaries. Build it first or the numbers below are somebody else's.
cargo build --release -p pdf-sandbox --bins
cargo test --release -p pdf-model --test corpus -- --ignored --nocapture   # 974 docs, ~2 s
cargo test --release -p pdf-model --test oracle -- --ignored --nocapture   # 1794 pages, ~30 s
# The first oracle run on a fresh build directory is ~95 s and writes 319 MB of remembered
# reference renders; every run after it is the ~30 s above. Read the printed hit rate rather than
# the clock. Two environment variables matter:
#   PDFREF_CACHE=off              ask the three renderers again, which is how "the cache changes
#                                 no verdict" is re-checked over the whole corpus
#   PDFVIEWER_ORACLE_ONLY=a,b     compare only pages whose names contain a or b — 0.2 s for a
#                                 handful of documents; a filtered run refuses to check the
#                                 ratchets and says so
cargo build --release -p hayro-compare --bins
cargo run --release -p hayro-compare --bin hayro-speed -- doc/pdf.js/test/pdfs/*.pdf
cargo bench -p pdf-model                   # interpretation, the time-to-first-page path
# Two callgrind examples measuring different halves: the first stops at the display list, so a
# backend change measures as exactly zero there; the second rasterises.
valgrind --tool=callgrind --callgrind-out-file=/dev/null \
  target/release/examples/callgrind_interpret
valgrind --tool=callgrind --callgrind-out-file=/dev/null \
  target/release/examples/callgrind_rasterise [file.pdf] [page]
cargo deny check
cargo +nightly fuzz run lexer -- -runs=50000     # from fuzz/, needs nightly
cargo +nightly fuzz run cmap  -- -runs=50000     # §9.7's CMap parser and its decoder
cargo +nightly fuzz run crypt -- -runs=50000     # §7.6's encryption dictionary and key algorithms
cargo +nightly fuzz run variable_text -- -runs=50000  # §12.7.4.3's /DA parser and its layout
```

Cargo prints one line about `proc-macro-error2` being rejected by a future compiler. It arrives
through `iai-callgrind`, a dev-dependency that reaches no shipped binary, and `deny.toml` records
the exception with its reasoning. Nothing to chase.

## Crate map

| Crate | Does | Notes |
|---|---|---|
| `pdf-spec` | Object-model validation tables | Generated from Arlington by `build.rs` |
| `pdf-syntax` | Lexer, objects, xref, filters, `Document`, decryption | Touches untrusted bytes first. `crypt.rs` is §7.6's standard security handler — every algorithm the clause numbers, written against its own subclause; `document.rs` is where §7.6.2 decides *what* is decrypted, because that is where an object's identity is known (ADR 0031). `text_string.rs` is §7.9.2.2 and Annex D's Table D.3, which is a code-to-Unicode table and so belongs here rather than beside `pdf-font`'s glyph-name encodings. `filter.rs` is §7.4's ten standard filters — four decoded here, one a pass-through for §7.6.6, four image codecs deliberately answered `None` so a *content* stream naming one is visibly unsupported |
| `pdf-model` | Page tree, content interpreter, annotations, optional content, Type 3 fonts, image decode | Where PDF semantics live. `annotation.rs` is selection and placement (§12.5.5) and knows no subtype; `appearance.rs` is where a missing appearance is *constructed* from what its subtype's clause states, where a stored one is *spliced* under `/NeedAppearances`, and where the refusals are argued (ADRs 0030, 0032). `variable_text.rs` is §12.7.4.3 and the one place in the tree that writes a content stream rather than reading one — it knows nothing about annotations or field types, only about a string, a box and a `/DA`. `soft_mask.rs` reads Table 142 and nothing else. `optional_content.rs` answers "is this layer on". `type3.rs` reads a font whose glyphs are content streams. `inline_image.rs` turns `BI` … `EI` into the stream an image `XObject` would have been. `image.rs` owns §8.9.6's and §11.6.5.2's masking, with `combine_on_the_finer_grid` the one place two rasters of different sizes are combined rather than refused; its `Decode` is §8.9.5.2's map held as one table per component and its `Conversion` is an *exact* per-image memo, which is what makes converting every image through its real colour space affordable (ADRs 0034, 0035). `page.rs` is §7.7.3: the tree walk, the four inheritable entries and the twelve that are not, and `/UserUnit` (ADR 0038) |
| `pdf-font` | Glyph outlines via `skrifa` | Owns both simple-font encoding algorithms (§9.6.5.2 for CFF, §9.6.5.4 for `TrueType`, ADR 0015). `simple_code_table` takes no font descriptor, which is the shape of ADR 0039's finding: Table 112 makes an *embedded* program's own built-in encoding the base, and the Symbolic flag decides only among the cases where nothing is embedded. `DEFAULT_WIDTH` is Table 120's 0 rather than a preference. `code_for` is the one *backwards* route — a character to the code that draws it — and it is built by running the forward mapping over every code the font defines, so the two cannot disagree. `cff.rs` adapts `read-fonts`; `encoding.rs` is Annex D data; `substitute.rs` is the only machine-dependent code in the tree. `cmap.rs` is §9.7's composite encoding, where `Code` carries a value *and* a length because the clause looks a code up "in the character code mappings for codes of that length" (ADR 0029). Deliberately not `tounicode.rs`: same file format, different destination. A Type 3 font is refused here |
| `pdf-render` | Display list + `Rasterizer` trait | No PDF semantics, no rasteriser. Three device decisions live here so the two backends cannot make them differently: `Image::is_smoothed`, `Image::area_averaged` (a departure from §10.7.4, ADR 0025) and `Stroke::device_width` (§8.4.3.2 with §10.7.5, ADR 0028). `soft_mask.rs` turns rendered pixels into §11.5's mask values. `Command::Group` is the one nested command (ADR 0026) and `impose_on_medium` is §11.4.7. `Path::extend_transformed` is the one place geometry moves rather than travelling with a transform (§9.3.6, ADR 0022). `Transform::max_stretch` is *not* `determinant().abs().sqrt()`: a shear separates the singular values without changing the determinant |
| `render-cpu` | `tiny-skia` backend | Correctness oracle **and** startup path |
| `render-gpu` | Vello/wgpu backend | Headless by construction. `soft_mask.rs` renders each mask to a texture and reads it back, because Vello's own luminance mask is the SVG formula and no blend mode is a `/TR` |
| `raster-compare` | Tolerant image metrics | Worst-tile error is the load-bearing one |
| `test-scenes` | Shared fixtures | Holds the same page as a display list *and* as PDF bytes |
| `tools/pdfref` | Reference-comparison harness | Triangulation rule lives here. `cache.rs` remembers what each renderer produced, keyed on the invocation itself (ADR 0020); `digest.rs` is the SHA-256 that key is built from |
| `viewer-ui` | The application | `src/bin/pdf-viewer.rs` |
| `pdf-sandbox` | Confined worker + the three image filters | Its `decode.rs` is the only place a JBIG2, JPX or CCITT codestream is looked at |
| `tools/hayro-compare` | Drives `hayro` for the oracle's fourth panel and for speed | Nothing ships it |
| `tools/conformance` | Citation checker and the conformance ledger | Depends on nothing but `thiserror`. The one crate the citation scan skips — its own comments cite clauses that do not exist, deliberately |
| `viewer-core` | Empty | Documented responsibility only |

## Traps — read these before writing code

### 1. The metrics lie. Look at the page.

This is the most important thing in this file. `Interpretation::is_complete()` tells you what the
interpreter *knows* it skipped. It cannot tell you that a font loaded and produced garbage, that
a page is upside down, or that a gradient came out opaque.

The archetype: wiring bare-CFF support in made every affected document report `unsupported: []`
and render **almost no text**. The font loaded, nothing was reported, the wrong glyphs were drawn.
`cargo test -p pdf-model --test render_real_pdf -- --nocapture writes_inspectable` writes PNGs;
the oracle's artefacts are better (see "Things worth knowing").

Two automated checks catch a wrong mapping, both in `crates/pdf-font/src/lib.rs`:
`the_pdf_widths_agree_with_the_font_programs_own_advances` — the document's `/Widths` and the CFF
charstring's own advance are independent statements of the same fact, so this verifies the mapping
without consulting the mapping — and `an_uncovered_code_has_no_glyph_rather_than_a_guessed_one`.
Both were confirmed to fail when their defects are reintroduced. Neither replaces looking.

**Every page a new feature makes drawable is a page nobody has ever looked at**, and the habit has
paid every session since the tenth. What it found, in order: dashed squares that should not have
been solid (the `d` operator, nothing to do with the Type 3 fonts being written); `/Interpolate`,
a `Lab` table scaled 0..1, and a dropped soft mask (none of them inline-image defects); a
fax-encoded page **upside down** because `/Rotate` 90 and 270 had been exchanged since the first
page tree; a solid red page that turned out to be §9.3.6 behaving *correctly* on a malformed
composite glyph; `alphatrans.pdf`'s gradient painted opaque because one `return` dropped
§11.6.4.4's alpha; a knockout group whose report had been hidden by the soft-mask report; a `0 w`
line invisible on the GPU; `issue7901.pdf` drawing `üãÍ†Ë` because Table 115's presence
condition had been read as a condition on meaning; and a shading painted across a rectangle its
clipping path admits none of, on the first page that §8.5.3.3.1's trailing-`m` rule made
rasterisable at all.

**A page a feature makes drawable can be one that never rendered *at all*.** The
twenty-fourth session's rule turned four `no render` pages into drawn ones, and the group's
label — "path is empty or contains non-finite coordinates" — described the *symptom* of a
missing rule, not the defect the pages then revealed. A `no render` count is a to-do list of
pages nobody has looked at, and it is now 25, all of them documents rather than pages.

**A contradicted page's group names a hypothesis, not a diagnosis — seven for seven on being
wrong.** Type 3 fonts, `/Rotate`, `alphatrans.pdf`'s gradient and `french_diacritics.pdf` all sat
under labels whose stories were *true about the page* and not the disagreement. The fifth and
sixth came together in the twenty-eighth session: `mesh_shading_empty.pdf`'s entry said
"displaced horizontally" and the mesh is not displaced at all, and `issue8092.pdf` sat under
*substituted fonts* while its difference was a shading's `/BBox`. The seventh is
`issue20232.pdf`, whose entry said a descriptor setting both the Symbolic and the Nonsymbolic
flag left §9.6.5.4's symbolic route "unreachable here" — it is not unreachable, it is taken,
and the glyph at the far end of it is one this subset embeds with no outline. Open the artefact
before believing the label — **and measure it, because a label this project wrote is still a
label**. Twice now the instrument that settled one was the font's own `cmap`, `loca` and `post`
tables read directly, which costs ten minutes and answers exactly.

**And the rule inverts, which is the version worth having**: twice the picture has rejected a
*reading of the specification* rather than finding a defect in code. `issue6621.pdf` blanked a
court seal under the only reading its `/Mask` samples admit, and `issue7901.pdf` drew garbage
under a defensible reading of Table 115. In both the code was right about the clause it cited.

### 2. A paint is positioned in the *path's* space, not the device's

Both `tiny-skia` and Vello apply the drawing transform to a paint as well as to the shape, so the
transform you hand a gradient, a pattern or an image is read **in the space the path is stated
in**; composing the page-to-device transform into it yourself applies it twice. Both backends did
exactly that, and it shipped: every gradient was mirrored about the page's horizontal centre line
(at scale 1.0 the page-to-device transform is its own inverse, so the second application leaves
just the flip), and `issue19971.pdf`'s 2500×1364 photograph came out as one flat rectangle.

Three things about how it survived:

1. **No metric saw it** — `unsupported: []`, right shape, colours from the right ramp.
2. **The CPU-versus-GPU comparison could not see it**, because both backends had it and therefore
   agreed. Two implementations agreeing is evidence *only where they can fail independently*.
3. **Every scene compared them with a gradient running along x**, where a y mirror is invisible.

The guards are `render-cpu/tests/shading_placement.rs` and `image_placement.rs`, pinning values
against §8.7.4.5.3 and §8.9.5.2 **at three scales**, plus `headless_gpu.rs`'s vertical-gradient
and image scenes. All were confirmed to fail when the defects are reintroduced.

**The sharpest form is about a convention rather than an axis.** `tiny-skia` treats a stroke width
of `0.0` as a hairline, which is exactly what §8.4.3.2 requires — so the CPU backend got the
clause right without anybody writing the rule down, and `kurbo` expands a zero-width stroke into
an *empty* outline, so every `0 w` line in every document was invisible on the GPU for fifteen
sessions. **Where two backends are the oracle, a decision either of them can make alone is a
decision neither has made**, which is why the device decisions live in `pdf-render`.

**It has now happened four times, and the fourth is the clearest.** §8.5.3.2's stroke with no
length: `tiny-skia` paints a projecting square cap where the clause asks for no output, `kurbo`
drops the contour before a cap is considered, and a path of one `m` is an *error* on one and
silence on the other — three different answers, none of them the standard's. `pdf-render`'s
`degenerate.rs` states it once, with the circle as this crate's own geometry rather than either
round cap's. `Clip::admits_nothing` is the same story for an empty clipping path, where Vello
happens to be right and was *verified* to be right by convention rather than by the clause —
which is exactly the position that reads as agreement and is not.

**And a scene must be able to fail at the defect's *magnitude* as well as in its axis.** The
sixteenth session's first reduced-image scene was in the right axis and **passed with the GPU's
filter removed altogether**: 32 differing channels out of 160 000 is under
`MAX_DIFFERING_FRACTION`. It now draws an 800×800 image across most of the page and fails at mean
6.50 against 0.5. Deleting the code a scene guards is one command, and it is the only thing that
establishes the scene guards it.

### 3. An oracle is only as good as how it invokes the other renderers

The corpus oracle's first run reported 54 documents whose page *size* we disagreed about, which
looked like a `MediaBox` defect. `pdftoppm` and `gs` default to the **media box**; `mutool` and we
use the **crop box**, which §14.11.2.1 defines as the region "to which the contents of the page
shall be clipped (cropped) when displayed or printed". The harness had been asking two of three
references for a different page — and on a page whose crop box has the same size as its media box
but a different origin it would have compared a correct render against a displaced one and called
us wrong. Every invocation is now explicit about the page box, *including* `mutool`'s, whose
default was already right: a default that silently changes is a comparison that silently changes.

The twenty-first session found the same shape one level up: `gs` renders for a **printer**, so
Table 167's Print flag decides what it draws, and four link borders disagreed for that reason
alone. Check what question each reference is being asked before reading its answer as a verdict.

### 4. Test against real documents, not hand-written fragments

Cross-reference streams are compressed *and* PNG-predicted. The code said decoding them was "the
caller's responsibility" and then did not, so every modern PDF failed with a misleading `/Root is
not a dictionary`. Unit tests on fragments would never have caught it; the corpus caught it on the
first run. `crates/pdf-syntax/tests/real_documents.rs` and
`crates/pdf-model/tests/render_real_pdf.rs` run over everything in `doc/`. The converse is trap 8.

### 5. Unsupported input must stay loud

Every layer reports what it could not handle rather than skipping it: `Unsupported` in the
interpreter, `FontError`, `ImageError`, `CpuRasterError::UnsupportedCommand`. This is what makes
the comparison harness trustworthy and what caught trap 1. Do not "helpfully" fall back to a
default that renders something plausible. **A rise in the incomplete count is not a regression
when it is a new report.**

The rule is easiest to lose *inside* a feature that is partly implemented, because the operator is
handled and the code path exists: `Tr` was parsed with three of its eight modes reported and the
four that change a clip silently absent; `/TK` was not read at all. The twenty-fourth session
found the same shape one level up — Table 57's `/LC`, `/LJ` and `/ML` read nothing at all while
`J`, `j` and `M` set the very same parameters, so three corpus documents silently drew with the
wrong caps and joins. **Where a clause gives a parameter two routes, implementing one of them is
the failure mode that reports nothing.**

There are now three places where a report accompanies drawing rather than replacing it, each
deliberate. An `/AcroForm` setting `/NeedAppearances` says its stored appearances may be stale and
we draw them anyway, because they are all the file offers (§12.7.4.3). §11.6.5.2's `/Matte` in a
colour space whose pre-blending cannot be undone after conversion is applied, because refusing it
would draw a rectangle of pure matte colour. And a constructed appearance draws what its clause
states while reporting what it does not — a widget's background with its field's value named
(ADR 0030). Two different true statements; suppressing either loses information. Do not generalise
it further without the same argument.

### 6. Colour: one conversion, and the specification often has no answer

Three separate `DeviceCMYK` → RGB conversions used to live here and they disagreed: `0.5 0 0 0.5
k` gave a red channel of 0.25, the same colour through `scn` gave 0.0, and a CMYK image gave a
third answer. Nothing about a rendered page reveals that. `crates/pdf-model/tests/colour_paths.rs`
drives one value through all three routes and demands they agree, and was verified to fail when
the old code is restored.

Add no fourth path. `ColourSpace::to_rgb` is the only place a colour becomes RGB, and
`colour::xyz_d50_to_srgb` the only place an XYZ becomes a pixel — that second rule exists because
the same defect had recurred one level down, with `lab()` and `icc::xyz_to_rgb` each holding their
own copy of the nine-constant D50-to-sRGB matrix.

The other half is harder to hold onto: **ISO 32000-2 defines no `DeviceCMYK` conversion at all.**
§8.6.4.4 says "concentrations of process colourants" and stops. What the standard *does* say is
where to ask — `/DefaultCMYK` (§8.6.5.6), an output intent's `/DestOutputProfile` (§14.11.5), an
`ICCBased` profile — and all three are implemented and all three outrank the fallback table. When
you touch that table, read ADR 0009 and change it as a documented choice. The same shape recurs
for a Cal space's `/BlackPoint`: §8.6.5.9 leaves black point compensation to the processor
whenever `/UseBlackPtComp` is `Default`, which is every real document, and ADR 0012 explains why a
stretch built from the entry is *undefined* on input Table 63 permits.

### 7. `#[expect]`, never `#[allow]`

Every lint exception in the tree is `#[expect(..., reason = "...")]`. It errors when it stops
being necessary, which has already removed several stale ones. A bare `allow` hides that forever.

### 8. A corpus finds what documents contain, not what the specification says

The mirror of trap 4. The ICC evaluator agreed with two other readers on every real profile in the
corpus; a test that assembled a profile *by hand* produced one whose darkest colour equalled its
white point, and black point compensation divided by floating-point noise and turned white into
pure green. `calrgb.pdf` page 14 states `BlackPoint [0.2 1.0 1.7]` against `WhitePoint [1 1 1]`,
which Table 63 permits and no sane producer writes — and it is what proved the black point stretch
has no well-defined answer.

**Three rules have now been measured to be unreachable by all 974 documents, and the method is
worth as much as the finding.** §9.7.6.2's per-byte codespace test (as against comparing the whole
code numerically) and §12.5.2's rule that a stored appearance ignores `/CA` were each measured by
breaking the rule deliberately and running both gates: all 1794 oracle verdicts identical. §7.6.2's
signature exception was measured differently and more cheaply — **eight corpus documents carry a
signature dictionary, twenty-six carry an `/Encrypt`, and the two sets are disjoint**, which is one
`grep` rather than two gate runs. Each rule is required of any valid PDF, and in each case the only
thing defending it is one synthetic test. **That turns "the corpus does not cover this" from a
suspicion into a fact — sometimes for the price of a gate run, sometimes for the price of a
question about what the corpus contains.**

This trap is why `CLAUDE.md` principle 5 defines *done* against the specification with a closed
exclusion list, and why the conformance ledger exists. A caution that changes no plan changes
nothing.

### 9. Two references can agree because they share code — or because they share a *gap*

The oracle's authority rests on a premise from ADR 0005: two implementations sharing no code
agreeing about a page is evidence. There are three ways for that to fail, and the second is the
common one.

**A shared gap.** An unimplemented feature almost always falls through to a *default*, so two
unrelated programs that skipped the same clause produce the same picture and the gate reads it as
agreement. `visibility_expressions.pdf` is the case: `mupdf`'s `pdf-layer.c` carries `/* FIXME:
Calculate visibility from array */ return 0;` and `ghostscript`'s `pdf_optcontent.c` prints
`WARNING: OCMD contains VE, which is not supported (ignoring)`, while `poppler` and pdf.js
implement `/VE` and §8.11.2.2 is unambiguous. So the page stays contradicted, listed with the
source citations beside it.

**Shared code.** `mupdf` and `ghostscript` both link `jbig2dec`, and on seven corpus pages it
decodes nothing, renders noise, or prints `segment marks bitmap coding context as retained (NYI)`.
Both emit the *same warning text*, because it is the same code emitting it. What settles those
pages is `tests/jbig2.rs`'s ninety-six encodings of one image, not anybody's agreement.

**Two answers to two different questions**, found in the twenty-first session: `mupdf` constructs
no link appearance at all while `ghostscript` renders for paper, where Table 167's Print flag says
not to draw one. Their agreement is a coincidence of two unrelated reasons.

The shape recurred immediately, and in a form where *we* are the minority: `mupdf` and
`ghostscript` both refuse `encrypted-attachment.pdf` and `auth-event-ef-open.pdf` for wanting a
password, `poppler` and this tree open them, and §7.6.6 says the refusal belongs to the stream
whose key is missing rather than to the file. Two against two is not a tie; it is a question with
an answer, and the answer is in the clause.

**So ask what a reference is made of and what it was asked, not only what it produced.** The
general form is in the type: `Reference::independence` says whether a renderer's agreement is
evidence and `Reference::voting` is what the gate iterates. `hayro` is marked `Shared` — it draws
a fourth panel and never votes, because we share its font rasteriser, its deflate, its JPEG
decoder and both new image codecs. `mupdf` and `ghostscript` are deliberately *not* marked
`Shared`: they share only `jbig2dec`, so recording the sharing where it applies keeps the evidence
of a thousand pages that marking them wholesale would throw away.

When a contradiction looks like "everyone disagrees with us", the cheap next step is not to
re-read our own code: search the other projects' source for the clause. A `FIXME` there is
stronger evidence than any number of agreeing pixels.

### 10. The sandbox worker is a separate binary, and Cargo will not rebuild it for you

`cargo test -p pdf-model` builds pdf-model's targets and pdf-sandbox's *library*, not its
`pdf-sandbox-worker` binary — Cargo never builds another package's binaries. So the tests run
against whatever worker was last compiled. This is not hypothetical: while verifying that
`tests/jbig2.rs` can fail, the seventh session inverted the black-and-white sense of every JBIG2
sample and the test passed, because the stale worker was still decoding correctly.

`cargo test --workspace` or `cargo build -p pdf-sandbox --bins` builds it. Both gates call
`require_the_sandbox()`, which fails loudly if the worker is *missing* — but a missing worker and
a stale one look nothing alike, and nothing detects the second. `pdfref-hayro` carries the same
caveat, less dangerously: it never votes.

### 10a. A cached reference render is a fourth thing that can be stale

The oracle remembers what `pdftoppm`, `mutool` and `gs` said (ADR 0020), which took it from 75 s
to 25 and introduced exactly one new way to be wrong. The key is built from the invocation
itself — `Reference::build_command`'s own argument list, plus the renderer's version and the
document's SHA-256 — so **a flag that is not in the key is a flag that is not passed to the
renderer either**. What it cannot see is a renderer whose output changes while its version string
does not.

- `PDFREF_CACHE=off` runs the gate the old way, which is how "the cache changes no verdict" is
  checked over the whole corpus. **The variable names a *directory*, and only the literal `off`
  disables it** — so `PDFREF_CACHE=on` silently starts a fresh 319 MB cache in a directory called
  `on`. If a run takes 95 s and reports a 0% hit rate, look at the variable before the corpus.
- **The hit rate is printed and it is the tell.** Under 99% on an unchanged tree means the corpus
  or a renderer moved.
- **A remembered *timeout* is the one entry whose truth decays**, so it is counted on its own line
  and expires after a week. The argument for remembering it at all — two decompression bombs were
  46 of a 57-second run — is in `pdfref::cache`.

### 11. A report is only as good as the condition it fires on

Trap 5's other edge. Principle 3 says unsupported input must stay loud, and the reflex that
produces is to report whenever the unimplemented thing *could* be involved. Four instances, and
each cost something to get right:

- **§9.3.8, text knockout.** `Tk`'s initial value is true, so every text object in every document
  is composited under a model we do not implement. The first draft asked one of the clause's two
  conditions and named 7 documents — and took **three pages that agreed with the reference
  consensus out of the gated set**, for a difference that could not have appeared on any of them.
  Asking both conditions (the paint composites *and* two glyphs overlap) names 2.
- **§11.6.2, one object in parts.** The first check named six documents, two of which had been
  agreeing. Printing the actual alphas showed three of the six set `ca` or `CA` to **zero**, so
  one of the two parts paints nothing and there are no two portions to composite. The clause says
  "portions", plural; the code had taken the operator as proof of them.
- **§11.7.4, overprinting.** 63 documents, six `silent` rows, top of the demand list — and the
  honest condition has **no members** on this device. The instrument that settled it was not a
  corpus run but Table 146 read against a list of this device's colourants.
- **§12.5.6.19, an empty widget.** The report fired where the clause asks for nothing at all: a
  field with no `/MK` and no value *states* no appearance, and 23 documents were being named for
  it.

So: **derive the condition from the clause, print what it matched before trusting the count, and
cost it in gated pages** — a page that reports is a page the oracle stops judging. And the reverse
worry is real too: **a report can hide another report.** `knockout_smask.pdf`'s knockout gap was
covered by its soft-mask report for four sessions, which is an argument for closing reports rather
than accumulating them.

### 12. A bound derived from two agreeing references is tighter than the arithmetic

`oracle.rs` judges us relative to how far the consensus references sit from one another, widened
by a factor. That is the right rule — it stops a page where every renderer differs from being
called our defect — and **where two references agree very closely the bound can be tighter than
eight-bit arithmetic.** `smask_luminosity_oob_transfer.pdf` is one flat composite through a mask
of 0.75: the closed form is `(223, 99, 80)`, `mupdf` gives `(222, 98, 79)`, `ghostscript`
`(223, 99, 79)`, we give `(223, 100, 81)`. Everybody is within a level of the arithmetic, but the
two references are within a level of *each other*, so the bound is a mean of 1.11 and ours is
2.02.

What to do with such an entry is not to chase it: check the *closed form* — write the clause's
arithmetic down and see whether we are within a level of it, which `render-cpu/tests/soft_mask.rs`
now does — then list the page with the calculation beside it. The reflex the number invites,
tightening our rounding until a reference's rounding is matched, is curve-fitting with extra
steps. The same effect makes small-text pages judged against two `FreeType`-based references
harsher than two independent rasterisers can be.

## Environment

The agent runs as user `AI` via `sudo -u AI`, reaching `/home/cl/projects/pdf-viewer` through the
`coders` group. This causes recurring friction:

- **Launch with a login shell** so `umask 002` applies, or every file the agent creates is
  unwritable by `cl`: `sudo -u AI bash -lc 'cd /home/cl/projects/pdf-viewer && claude'`
- **`AI` has no X authority cookie.** Anything needing a window fails at `XOpenDisplayFailed`. The
  GPU backend is headless by construction precisely so it can still be tested; the viewer binary
  cannot be run by the agent past event-loop creation.
- **Build directory**: `AI` builds into `/home/AI/cargo-target/pdf-viewer` via
  `~/.cargo/config.toml`, so the two users never fight over `target/`. Do not "fix" this.
- **`pdfref` needs `--work-dir`** for the same reason; its default is `./target/pdfref`.
- **`cargo-fuzz` needs `+nightly`** explicitly, because `rust-toolchain.toml` pins stable 1.97.1.
  That pin is deliberate.
- The Arlington model is a **submodule** pinned at `ba7d4d61`; `pdf-spec` will not build without
  `git submodule update --init`.

## What is not implemented

Every one of these is *reported* at runtime rather than silently skipped — that is what makes the
corpus numbers trustworthy, and it is principle 3's requirement rather than a nicety. Sized by the
corpus: the count is how many of the 974 documents' first pages it affects.

| Missing | Corpus | Size | Notes |
|---|---|---|---|
| Variable text: a `/DA` font `/DR` does not define | 5 | Small | What is left of §12.7.4.3 (ADR 0032), and it is a *malformed file* rather than a clause gap: the clause requires the `/DA`'s font name to "match a resource name in the Font entry of the default resource dictionary" and states no recovery. Four are `FreeText` annotations in files with no interactive form dictionary at all, naming `/Helv`. Reported by name, exactly as a content stream naming an absent font already is; inventing Helvetica from the resource name would need a name-to-typeface table no clause states. |
| Variable text: a composite `/DA` font, a list box, `/DS` and `/RV` | 0 | Medium | The rest of §12.7.4.3's edges, and none is reached by any corpus document. A composite font needs a `CMap`'s codespace ranges inverted (§9.7.6.2) to turn a character into a code; §12.7.5.4's list box states which items are selected and nothing about how a selection *looks*; `/DS` and `/RV` are XFA rich text, which principle 5 excludes. |
| Text markup appearances (§12.5.6.10) | 8 | Medium | Highlight, Underline, StrikeOut and Squiggly with no `/AP`. **Refused because the standard states no mark**, not because the drawing is hard: `/QuadPoints` and the orientation edge are given, and nothing says an underline's thickness, where a strikeout crosses, a squiggle's period, or how a highlight leaves text legible. The three references draw three different pictures. Any implementation here is a documented choice, and it should be argued as one. |
| Encryption: a password prompt | 8 | Small | §7.6 is implemented (ADR 0031); what is missing is the *interaction* §7.6.4.1 describes — "the interactive PDF processor should prompt for a password". `Document::open_with_password` takes one and nothing asks for it, so 8 corpus documents are refused at the gate that a viewer with a window would open. This is `viewer-ui` work, not clause work. |
| Encryption: public-key handlers (§7.6.5) | 0 | Medium | Refused by name. Needs CMS enveloped data (RFC 5652), X.509 certificates and access to the user's private keys — a public-key infrastructure and a threat model rather than a cipher. No corpus document uses one. |
| Encryption: `/R` 5, and a non-ASCII revision-4 password | 1 | Small | Two refusals, one of them now cheap to close. Table 21 says `/R` 5 "shall not be used" and states no algorithm for it, so implementing it would mean copying another reader; `issue21579.pdf` writes it anyway. §7.6.4.3.2 step (a) wants a password in `PDFDocEncoding`, which `crypt.rs` refuses outside the range where it and Unicode provably agree — **and `pdf-syntax` now holds Table D.3**, since §12.7.4.3 needed it, so inverting that table would close this refusal outright. Nobody has done it and no corpus document needs it. |
| Annotation icons (§12.5.6.4, .12, .15, .16) | 2 | Small | A `Text`, `Stamp`, `FileAttachment` or `Sound` annotation with no `/AP` displays an icon whose artwork no clause states. Refused and named. Every stamp in the corpus carries an `/AP`, which is what a producer who cares has to do. |
| Predefined `CMap`s (§9.7.5.2) | 12 | Medium | 15 fonts name one of Table 116's registered `CMap` files (`90ms-RKSJ-H`, `UniJIS-UTF16-H`, …), which are not in the tree. Vendoring them is a licensing decision; guessing draws plausible text that says something else. The machinery they would plug into exists. |
| Text: a substitute that cannot be addressed | 42 | Medium | Counting *fonts*: 27 composite fonts with no `/ToUnicode`, so a CID cannot be taken to a character a substitute could draw, and 23 whose substitute draws none of the declared codes. Honest refusals rather than clause gaps; closing them means better substitution. |
| Optional content: the interactive half | — | Medium | §8.11 is honoured wherever it decides what is *drawn* (ADR 0017). Missing: a layer panel and what feeds it — `/Usage` and the `/AS` usage application dictionaries (§8.11.4.4), which switch groups by zoom, language or print state, plus `/Order`, `/ListMode`, `/RBGroups`, `/Locked` and alternate `/Configs`. §8.11.4.4 is **the ledger's only `silent` row**: a layer that should switch itself off is drawn with nothing said. |
| Text knockout (`Tk`, §9.3.8) | 1 | Medium | Table 102's ninth text state parameter, and the only one absent. Its initial value is `true`, which makes a text object a non-isolated knockout group; we composite each glyph separately, which is indistinguishable while glyphs are opaque under Normal. Reported where both of the clause's conditions hold. Implementing it is §11.4.6's knockout groups seen from clause 9. |
| Compositing an object in parts (§11.6.2) | 3 | Medium | "Portions of an object shall not be composited with one another", and `B` paints one object as a `Fill` and a `Stroke`, so the band they share composites twice. Reported where the paint composites and both parts mark the page. The fix is the same as `Tk`'s. |
| Transparency group and mask departures (§11.4, §11.5.3) | 24 | Medium | Three answers a `/Group` may give that are drawn as the isolated, non-knockout group instead, each reported where it can change a pixel (ADR 0026): **knockout** (§11.4.6, 6 documents; for an isolated knockout group the implementation is a Porter-Duff Source composite modulated by coverage and nothing more), **non-isolated with a blend mode inside it** (§11.4.4, 9 documents; without one the two computations are provably identical), and **a blending colour space that is not the device's three components** (§11.6.6, 4 documents, all `/DeviceCMYK`, which means a second raster format). Plus **a soft mask's group with such a space** (§11.5.3, 7 documents). |
| Grid-fitting a stroke's coordinates (`/SA`, §10.7.5) | — | Small | The clause's single-pixel rule is implemented; adjusting "the line width and the coordinates of a stroke … to produce lines of uniform thickness" is a **documented departure**, because the non-uniformity it removes is an artefact of the binary scan conversion §10.7.4 requires and this tree already departs from by anti-aliasing. Nothing reports it: there is no page on which this device could do better. |
| Smoothness tolerance (`/SM`, §10.7.3) | 23 | Small | Read nowhere. This renderer has one fixed internal bound — a 256-sample `Ramp`, and `Triangle::is_subpixel` — where the clause asks for a per-document one, and "each output device may have internal limits" contemplates that. A document asking for a *coarser* shading gets a finer one; one asking for finer than 1/256 of a component is not honoured and nothing says so. That silence hides inside a `partial` row. |
| Image `/Mask` on a filtered image, `/Matte` outside the device spaces | 0 | Small | What is left of §8.9.6 and §11.6.5.2 after ADRs 0023 and 0024, and no corpus document writes any of it. A colour key is a test on the samples a filter delivers, and a `DCTDecode` or `JPXDecode` image has become RGBA before the unpacker sees it — the clause's own NOTE 2 names that pair as the one lossy coding makes unreliable. A `/Mask` stream that is not an image mask is here too, which Table 87 excludes and 1 document writes. So is a `/Matte` on an image whose space is not `DeviceGray` or `DeviceRGB`: §11.6.5.2 requires the pre-blending to be undone *before* colour conversion, and this crate holds one RGBA raster per image, so the inversion is exact only where that conversion was the identity on components. |
| A font selected by `/ExtGState` `/Font` (§8.4.5) | 1 | Small | Table 57's `/Font` is `[font size]` with the font an **indirect reference**, where `Tf` and this crate's font cache are both keyed by a resource *name*. `extgstate.pdf` writes one, and what it decides is which glyphs the page draws, so it is reported rather than passed over. Closing it means a font cache keyed by object identity as well as by name. |
| A degenerate subpath's single device pixel (§8.5.3.3.1) | — | Small | "[A] degenerate subpath … shall be considered to enclose the single device pixel lying under that point" when *filled* — distinct from §8.5.3.2's stroking rule, which is implemented. Neither backend paints it, and the clause calls the result "device-dependent and not generally useful" in the same breath. Recorded in the ledger rather than reported, because a report would name pages on which no reader could tell. |
| Annotation `NoZoom`, `NoRotate`, `/FixedPrint` | — | Small | Table 167 bits 4 and 5, and a watermark's `/FixedPrint`, make an appearance's size or orientation depend on the *view*, which a resolution-independent display list cannot express. Rare. |
| Type1 fonts (`/FontFile`) | 0 | Medium | No corpus page one reaches it. `read_fonts::ps::type1` exists — check before writing any. |
| Bit depths 2, 4 and 16 | 3 | Small | §8.9.3 permits five component widths and the unpacker reads two. Refused and reported. |
| Vertical writing (`Identity-V`, `/W2`) | 4 | Medium | §9.2.4 gives a glyph in writing mode 1 a second set of metrics — `w1` and `v`, from `/W2` and `/DW2` (§9.7.4.3). None is read. Refused and reported, including an embedded `CMap` declaring `/WMode 1`, because being drawn horizontally is not a near miss. |
| Soft masks and `/Mask` at a grid the bound refuses | 1 | Small | `issue16263.pdf` gives a 2x2 image a 34862x4332 mask — 151 million samples, 604 MB — and that pair is refused and named. The answer the clause describes is compositing at *device* resolution, which means the display list carrying an image and its mask separately. |
| JPEG 2000 at reduced resolution | 1 | Small | `issue19517.pdf` is a 12608x16806 scan whose full decode wants gigabytes for a page drawn at four megapixels. The format's answer is to decode a lower resolution level, which needs the intended scale to reach the decoder. |
| Sampled shadings on the GPU | 2 | Small | Type 1 only; the CPU backend draws them. |
| Rendering intents beyond `AbsoluteColorimetric` | — | Small | Read and recorded; `A2B0` is not yet selected for `Perceptual`. |
| Forms, actions, the rest of clause 12 | — | Large | A field's *appearance* is built (ADR 0032); its **behaviour** is not — §12.6's actions, §12.7.6's submit, reset and import, §12.7.8's FDF, calculation order, validation, navigation. In scope wherever it *displays*. **JavaScript and script-driven field behaviour are excluded** by principle 5. |
| Tagged PDF, metadata | — | Large | Clause 14 beyond output intents. In scope as far as accessibility needs it. |
| Sandboxing the *rest* of the renderer | — | Large | Spike D is done for the image codecs (ADR 0014). Interpreting and rasterising still happen in the main process. |

## How much of the specification is implemented

Four answers, in ascending order of how much they should worry you: what we *report*, what an
independent implementation *sees*, what the standard contains, and what a person has actually
read. The first two are measured. **The third is a self-assessment and has been wrong twice** — it
called clause 9's encoding algorithms "implemented in full" while §9.6.5.4 was one line covering
about one and a half of its five routes, and the feature table said Type 3 fonts were reported for
two sessions in which they were not. Both errors were found by pixels.

**The fourth is the conformance ledger**, and its headline is a count of unasked questions: **406
of 823 subclauses are `unreviewed`**, and 417 have been read against this code — 82 of those
carrying principle 5's exclusions, almost all of them clause 13. So the honest summary is that the
project has measured 39% of its clause coverage. That number is meant to look bad; the alternative
was not knowing.

**The ledger has been wrong twice**, which is worth knowing before trusting a row: §8.9.5.3's note
said reduction was something the standard does not address, and §10.7.4 addresses it in the
opposite direction; and §8.4.3.2's row said a zero width "reaches the rasteriser as the thinnest
line it draws", which was true of `tiny-skia` and false of Vello. **A row that names a rasteriser's
behaviour has recorded that rasteriser rather than the clause.** The defence is to read the
*family*, not the row, which is why the review unit is a clause family.

### By what real documents need

Over the 974-document pdf.js corpus, page one:

| | count | share |
|---|---|---|
| opens | 964 | 99% |
| of the 10 that do not, need a password | 8 | — |
| of the 10 that do not, are encrypted beyond us | 2 | — |
| reaches page one | 953 | 98% |
| **draws with nothing reported** | **823** | **85%** |
| draws, with something reported | 130 | 13% |

That 85% is the number to quote for *reporting*. It **fell by one** in the twenty-fourth session,
which is the whole of trap 5 in one document: `extgstate.pdf` now says that Table 57's `/Font`
selects a font this crate cannot address, where it used to draw the text in whatever font was
current and say nothing. It rose by eight in the twenty-third, all of them form fields and free
text annotations; by eight in the
twenty-second, while ten left the reporting column — six of them by saying they need a password
instead of describing ciphertext as an operator. Before that it **rose by forty-two** in
the twenty-first, the largest movement it has ever had, all of it annotations that were refused
rather than drawn wrongly; by thirty-one in the twentieth (embedded `CMap`s); by seventeen in the
eighteenth (soft masks in an `/ExtGState`); and it *fell* by six in the seventeenth, when seven
documents began saying their `/Group` is a knockout or non-isolated one.

**The "opens" row lost its 100% and that is not a regression.** Ten documents are refused where
they used to be opened and drawn as noise; eight of them a viewer with a password prompt would
open, and the prompt is `viewer-ui` work rather than clause work.

**This number measures honesty, and honesty can fall as capability rises** — it fell from 72% in
the eighth session when 24 documents began saying they carry a Type 3 font and 19 that their
substitute draws none of the declared codes, while `issue918.pdf` had been emitting 388 text
operations of letter fragments in silence. So a rise is only good news when you can name the
capability that caused it, and a fall is only bad news when you cannot name the silence that
ended.

### By what an independent renderer sees

This is the number to worry about. Over all 1794 pages compared, of the 1620 we claim to draw
completely:

| | count | share of the 1620 |
|---|---|---|
| agree with the reference consensus | 760 | 47% |
| **contradicted by it** | **99** | **6%** |
| the references cannot agree among themselves | 751 | 46% |
| not comparable (geometry, or fewer than two renderers) | 10 | 1% |

**One page in sixteen that we say we drew completely, two independent implementations say we did
not.** The 99 are named in `oracle.rs` and grouped by what the page carries: 15 use a font nobody
embeds so every renderer substitutes differently, **12 are pages where the references that agree
are not reading the clause differently** — 7 sharing a JBIG2 decoder, 1 sharing a `/VE` gap, 4
link borders where one reference has no such feature and the other is rendering for paper (trap 9
has all three shapes) — 8 are a one-pixel page-rounding difference, 1 an image half a device pixel
tall, 1 a `CalRGB` alternate two references do not convert, 2 pages of glyphs judged with the
tolerance for flat fills, 1 a level of mask quantisation on a flat page (trap 12), 1 a symbolic
font whose (3, 0) subtable reaches an empty glyph, and **58 have nothing on them to explain it**.
That last group is the most valuable list in the repository, and 21 of them are pages beyond the
first, which a page-one comparison would never have seen. **One page left the `substituted fonts`
group in the twenty-eighth session by being fixed** — `issue8092.pdf`, whose difference was a
shading's `/BBox` and had nothing to do with its fonts (ADR 0037).

**The pattern to read this table by**: a feature that makes pages drawable adds them to the set
being judged, so the numerator and the denominator move together and only one of those is news.
The denominator has not moved for seven sessions, so the last six sessions' movements are all
numerator: the twenty-eighth fixed one contradicted page (a shading's `/BBox`), the
twenty-ninth moved three out of the geometry bucket into agreement (`/UserUnit`), and the
thirtieth moved two — one a fix (`/MissingWidth`) and one a *bound*, `issue3566.pdf`, whose
raster is byte-identical and which changed tolerance class when its glyph names became
readable. **Only one of those two is news, and the other is news about the instrument.**
The twenty-fourth session exchanged one page for another — one joined by becoming drawable, one
left by starting to report — and made four pages that had *never rasterised* draw, three of which
agree; the twenty-third added 9 pages, 7 of them agreeing and **none contradicted**; the
twenty-second added 8, 5 agreeing and none contradicted — three sessions in a row where that count
did not move, after four in which it did. The twenty-first added 46 with 36
agreeing; the twentieth added 32 with 18 agreeing; the eleventh's 42 new pages took the count from 104 to 108 with nothing getting
worse. Conversely a *fall* is only a fix when the page stays in the comparison — the seventeenth
session's 100 → 93 was one fix and six honest withdrawals, and the fifteenth's fixed
`alphatrans.pdf` and then removed it from the comparison in the same session.

**Read the 47% ambiguous with care.** It is not "half the corpus is unsettled": 372 of those pages
are two long books, `freeculture.pdf` (320 pages) and `pdkids.pdf`, whose text uses fonts nobody
embedded, so each renderer substitutes differently and the structural bound separates them.
Ambiguity concentrated in a handful of documents says more about those documents than about the
gate. **So read them as "reported nothing", not "drew it right".**

### By clause

ISO 32000-2 carries 823 subclauses under its eight technical clauses, and counting them is a poor
proxy for work: clause 12 is 166 of annotation subtypes a viewer adds one at a time, while clause
8's 128 decide whether any page looks right at all. **Every entry below is a judgement about
state, not a measurement**; the ledger's `status` column is what turns one into a measurement, and
where the two disagree the ledger is the one that had to name a code site.

| Clause | Subclauses | State |
|---|---|---|
| 7 Syntax | 138 | **Nearly complete**, 70 rows reviewed — the whole of §7.4, §7.6 and §7.7 as families. Objects, **every standard filter**, classic and stream xrefs, object streams, incremental updates, recovery by scanning, and **encryption at every revision and method §7.6 states**. What is left is a public-key handler and a password prompt. §7.9.2's string object types are read, including Annex D Table D.3's `PDFDocEncoding`. |
| 8 Graphics | 128 | **Nearly complete**, and the clause with the most ledger coverage: 107 rows reviewed, with §8.4, §8.5, §8.6.4, §8.6.5, §8.6.6, §8.6.7, §8.7, §8.9 and §8.10 done as families. The whole of the graphics state and of path construction and painting, including §8.5.3.2's strokes with no length and §8.5.4's empty clipping path. Paths, clipping, all eleven colour space families, all seven shading types, both pattern types, form and image XObjects, inline images, `/Interpolate`, an image's `/Mask` in both forms, ICC colour management, optional content (§8.11) wherever it decides what is drawn, a form clipped by its `/BBox` (§8.10.1), and §8.6.6.4's `/All` and `/None` colourants. §8.9.5.2's `/Decode` array in full, Table 88's per-space defaults included, and an image's colour space is the one a fill gets — `ICCBased` profiles and §8.6.5.6's default spaces both (ADRs 0034, 0035). 2, 4 and 16 bits per component are refused. |
| 9 Text | 65 | **Partial**, 52 rows reviewed — §9.3, §9.4, §9.6, §9.8 and the whole of §9.7 as families. Simple and composite fonts through embedded TrueType, CFF and OpenType programs; the standard 14 by substitution; `/ToUnicode`; Type 3 fonts; all eight text rendering modes; both simple-font encoding algorithms in full; §9.7's two mappings in full. An embedded program's own built-in encoding is the base encoding Table 112 says it is, and `/MissingWidth` defaults to Table 120's 0 (ADR 0039). Missing: bare Type1, Table 116's predefined `CMap`s, vertical writing, text knockout (§9.3.8, reported), and §9.8.3's `/Style` and `/FD`, which are the ledger's two new `silent` rows and reach nothing but a substitute's choice. |
| 10 Rendering | 36 | **Partial**, 6 rows reviewed — the whole of §10.7. Colour management and rendering intents are done. Halftones and transfer functions describe a marking device. **Flatness is not "inapplicable"**: §10.7.2 makes ignoring it an explicit permission, which is a better answer. §10.7.4 is `partial` with three deliberate departures named — anti-aliasing twice over and area averaging — and §10.7.5 with a fourth. |
| 11 Transparency | 58 | **Partial**, 46 rows reviewed — everything from §11.4 onwards, leaving only §11.1–§11.3.5 and §11.3.8, which are the model rather than its PDF representation. All sixteen blend modes reach both backends, including §11.6.3's rule for choosing among an array of names; `ca` and `CA` reach a shading as well as a colour; an image's `/SMask` supplies alpha at any resolution with `/Matte` undone; a `/Group` is composited as one object with the page itself an isolated group; a graphics-state `/SMask` is a group evaluated for alpha or luminosity with `/BC` and `/TR`. Left: knockout, a non-isolated group whose elements blend, and a blending space that is not the device's — all reported. **Overprinting (§11.7.4) was six `silent` rows and is not a gap.** `/AIS` is argued in ADR 0027: with one alpha per pixel, shape and opacity multiply to the same number. |
| 12 Interactive features | 166 | **Appearances, constructed ones, and a field's own text**: 51 rows reviewed — the whole of §12.5, and the whole of §12.7.4 and §12.7.5 with §12.7 to §12.7.3 above them. An annotation is placed and drawn from `/AP` (§12.5.5) with §12.5.3's flags and §8.11.3.3's `/OC` honoured; one with no `/AP` is constructed from its subtype's clause or refused with the reason named (ADR 0030); and a field's value, caption or free text is laid out from its `/DA` by §12.7.4.3 (ADR 0032). What does not exist is *behaviour*: no actions (§12.7.6), no FDF (§12.7.8), no navigation, no signature validation (§12.8). |
| 13 Multimedia | 81 | **Excluded** by name on principle 5's closed list. Its rows carry that exclusion rather than being omitted, because an invisible exclusion is indistinguishable from an oversight. |
| 14 Document interchange | 152 | **Output intents, and marked content as a bracket.** No tagged PDF, no metadata, no marked-content *semantics* — but §14.6.1's nesting rule is now read twice over: `BDC`/`EMC` maintain the optional-content stack, and §12.7.4.3's splice has to find the `EMC` matching a `/Tx BMC`, which is the same sentence as an algorithm. §14.3.2 is read only as far as Table 21's `/EncryptMetadata` needs. |

So: the parts of the standard that decide whether a page is drawn correctly are largely done; the
parts that make a document *interactive* are not started.

### Feature by feature, from the source

| | |
|---|---|
| Content-stream operators | **73 of 73** in Table 50 (`ID`/`EI` are consumed inside the `BI` handler). `MP`/`DP`/`BX`/`EX`/`i` are matched and deliberately ignored. |
| Filters | **10 of 10** standard filters decode: `ASCIIHex`, `ASCII85`, `Flate`, `LZW`, `RunLength`, `Crypt` (pass-through, because §7.6.6's crypt filter is applied when the object is loaded), `DCTDecode`, `JBIG2Decode`, `JPXDecode`, `CCITTFaxDecode`. `LZWDecode` was the last one absent and landed in the twenty-seventh session, written from §7.4.4.2 including Table 8's `/EarlyChange`. Table 92's abbreviations are expanded in `inline_image.rs`. Not read: Table 13's `/ColorTransform`, whose one corpus witness contradicts the clause (§7.4.8's ledger row). |
| Encryption (§7.6) | **Revisions 2, 3, 4 and 6**, `/V` 1, 2, 4 and 5, methods `V2`, `AESV2`, `AESV3` and `Identity`. Every numbered algorithm a *reader* runs — 1, 1.A, 2, 2.A, 2.B, 4, 5, 6, 7, 11, 12, 13, and 3's first four steps. All four of §7.6.2's exceptions, plus Table 20's two. Refused by name: `/R` 5, public-key handlers, `/CFM /None`, a non-ASCII revision-4 password. |
| Colour spaces | **11 of 11** families, the three CIE-based ones converted rather than approximated, plus §8.6.5.1's withdrawn `CalCMYK`, which the clause redirects to `DeviceCMYK`. An *image* in an `ICCBased` space is still unpacked as a device space where a fill in it is not (§8.6.5.5). |
| Function types | **4 of 4**. Shading types **7 of 7**, on both backends. Pattern types **2 of 2**. Blend modes **16 of 16**. |
| Font programs | TrueType, CFF, CFF-in-OpenType, CID-keyed CFF, and Type 3 — whose glyphs are content streams and are run by `pdf-model`. Bare Type1 is reported. |
| Composite fonts (§9.7) | **Both of the clause's mappings** (ADR 0029): codespace ranges matched byte by byte and deciding a code's length from 1 to 4, `cidrange`, `cidchar`, `notdefrange`, `notdefchar`, `bfchar`, `/WMode`, `usecmap`, Table 118's `/UseCMap`, §9.7.6.3's recovery; then a CID-keyed CFF's charset, a `/CIDToGIDMap` stream, or the identity, chosen by what the embedded program *is* rather than by `/Subtype`. `/W` and `/DW` are indexed by CID. |
| Text rendering modes | **8 of 8** in §9.3.6 Table 104: fill, stroke in user space, both per glyph, invisible, and the four that add glyphs to the clipping path at `ET`. An operand outside 0..7 is reported. |
| Text state parameters | 8 of Table 102's 9. Missing: `Tk` (§9.3.8), read from `/TK` and reported where it can show. |
| Word spacing (§9.3.3) | A property of the *code's encoded length*, not of the font: an embedded `CMap` may define codes of several lengths in one font and four of the corpus's do. |
| Annotations | Placed by §12.5.5, drawn from `/AP`, and **constructed** where there is none: a link's border, a square, a circle, a polygon, a polyline, an ink scribble, a line, a widget's `/MK` frame, and **its field's text** (ADRs 0030, 0032). Icons and text markup are refused and named. |
| Form fields (§12.7.4.3) | A text field's `/V`, a choice field's selection, a button's Table 192 caption and a `FreeText`'s `/Contents`, laid out from a `/DA` string resolved in `/DR`: quadding, auto-sizing, wrapping, Table 232's comb cells, and a password field's bullets. `/NeedAppearances` splices the `/Tx` region of a stored stream and keeps the rest. |
| Text strings (§7.9.2.2) | All three encodings, chosen by the clause's prefix, with surrogate pairs paired, §7.9.2.2.2's language escapes removed and Annex D Table D.3 compiled in. |
| Image masking | All four mechanisms an image can carry plus the graphics state's own, combined on the finer of the two grids with a bound on the growth; a graphics-state mask is combined at *device* resolution. §11.6.4.3's precedence decides which wins. |
| Transparency groups | §11.6.6's `/Group` with the blend mode and both alphas reset inside, and §11.4.7's page group, which is why a page is drawn onto transparency and imposed on the medium afterwards. |
| Sample decoding (§8.9.5.2) | The clause's linear map in full, per component, with Table 88's defaults — including `Lab`'s `[0 100 …]` and `Indexed`'s `[0 2^n − 1]` — and its closing clamp. One lookup table per component, built once per image, so the unpacker's arms do not know what a `/Decode` array is. Applied on all five routes, `DCTDecode` included. |
| Image resampling | Magnification is §8.9.5.3's `/Interpolate`; reduction is §10.7.4's, and is the one place this tree knowingly does what a clause forbids (ADR 0025). Both decisions live in `pdf-render`. |
| Scan conversion (§10.7) | **Four** deliberate departures, all licensed by §10.7.1's NOTE — anti-aliasing twice over, area averaging, and §10.7.5's grid-fitting. `/FL` is ignored by the clause's own permission; `/SM` is read nowhere; `/SA`'s single-pixel rule **is** implemented. |
| Line width (§8.4.3.2) | A zero width is one device pixel on both backends, in `Stroke::device_width` alongside §10.7.5's rule, because the clause's own NOTE makes them the same width. |
| Overprint control (§8.6.7, §11.7.4) | Ignored, and the clause says to. Special colourants `/All` and `/None` are honoured before the alternate space and tint transform are parsed. |
| Font descriptors (§9.8) | Table 120's `/Flags`, `/MissingWidth` — default 0, not a guess — and the three `/FontFile` entries, plus `/FontWeight` and `/ItalicAngle` for choosing a substitute. Table 121's Symbolic bit decides §9.6.5.4's route, and §9.8.2's "historical accident" paragraph decides a descriptor that sets Symbolic and Nonsymbolic together. The dimensional metrics are unread because this tree selects an installed face rather than synthesising one. |
| Simple font encodings (§9.6.5) | The base encoding, `/Differences` over it, and — for an *embedded* program — the program's own built-in encoding as the base Table 112 says it is, with the Symbolic flag deciding only among the cases where nothing is embedded (ADR 0039). |
| Page geometry (§7.7.3.3) | Table 31's `/MediaBox`, `/CropBox` intersected with it, `/Rotate` clockwise as displayed — which in this y-up space is a negative rotation — and `/UserUnit`, "the size of default user space units, in multiples of 1/72 inch", which scales the page and everything on it. The four inheritable entries are inherited and the twelve that are not, are not (§7.7.3.4). |
| Optional content | §8.11 wherever it decides what is drawn: configuration, membership, `/VE`, intent, and all three places `/OC` can appear. The interactive half — `/Usage`, `/AS`, `/Order` — is not read. |

## What to do next

**Two tracks, and the discipline is to take from both in every session.** *Demand-driven* is
everything the corpus and the oracle name — 99 contradicted pages, 58 of them unexplained, and a
feature list sized by how many documents want each item. *Spec-driven* is what the ledger and
§6.3.2.2's ranking name: **406 of 823 subclauses are `unreviewed`**. A project running only the
first track finishes when the corpus goes quiet, which can happen with a great deal of the
standard unimplemented and nothing able to say which parts.

This is a `CLAUDE.md` principle-5 rule, not a suggestion. In practice: **one item from each track
per session**, with the spec item usually the smaller, because reviewing a clause family against
code that exists is cheaper than writing a feature. Three shapes have worked:

- **The same family for both**, which is the best when available: §8.11 in the ninth session, §9.7
  in the twentieth, §12.5 in the twenty-first, §7.6 in the twenty-second, §12.7.4 with §12.7.5 in
  the twenty-third.
- **Take the demand item, then review the family the code you just wrote cites.** Sessions ten to
  eighteen. **Read the family before writing the feature, not only after** — the sixteenth session
  found that the clause governing its demand item *forbids what the demand item asked for*, which
  turned an obvious improvement into a documented departure with three parts.
- **Take the demand item from the ledger's own silence list.** The nineteenth session, where
  reading §8.6.6 with §8.6.7 found two unimplemented colourants and dissolved the demand item
  entirely.

Every one of the twenty family reviews so far has produced findings the demand item could not
have reached — fifty-three of them, most recently three in §9.6 and §9.8: Table 112's default
base encoding for an embedded program, Table 120's `/MissingWidth`, and §9.8.2's sentence that
settles a descriptor setting both flags. **The demand item those three came with is still
contradicted**, and the review is what said why it should be. Before them, three in §12.7: §12.7.4.3's closing paragraph making
regeneration a *splice* rather than a rebuild, the three field types whose own subclauses say the
flag cannot reach them, and §12.7.5.2.3's `/AS`-over-`/V` rule, which `annotation.rs` already
satisfied without anybody having written the sentence down. **A gap sized by a corpus is a hypothesis about a clause**, and the only instrument that
can test it is the clause.

**And a third thing, on neither track: the instrument.** 95% of the oracle's cost was three other
programs answering a question they had already answered, and nobody had looked because 85 seconds
is not obviously wrong. The thirteenth session found the citation checker blind to table numbers,
and one wrong. The tree was also not `clippy` clean while this file said it was. **Whatever this
file asserts about the tooling, run it once before believing it.**

The one-line version of the demand track: **99 pages we claim to draw are contradicted, 58 of
them for no reason visible on the page**, and the largest thing left that any corpus document
names is §9.7.5.2's predefined `CMap`s at 12 — a licensing decision rather than code — followed by
§12.5.6.10's text markup at 8, which is a decision about what a highlight looks like. **Variable
text has left this list, as encryption did before it**, and what replaced both is not clause work:
eight documents need a password prompt and five write a `/DA` naming a font their own `/DR` does
not define. **A shading's `/BBox`, `/UserUnit` and `/MissingWidth` are the three rendering
items that came back onto it and off it again** in the twenty-eighth, twenty-ninth and
thirtieth sessions, and none was announced by a document: all three were found by reading a
clause family, and each fixed a page the gate had been carrying (ADRs 0037, 0038, 0039). The one-line version of the spec track: **4 clauses
the code already cites have never been read against it**, named in `REVIEW_OWED`, and **406 of
823 subclauses have never been read at all**.

Six sessions in a row now, no rendering feature that any corpus document *announces* has been
left on either list — the corpus going quiet, and exactly the condition `CLAUDE.md`'s two-track
rule exists for. Everything that moved a gate number in those five sessions came from the
specification track: `/Decode`'s general map, an image's colour space, `LZWDecode`, a shading's
`/BBox`, `/UserUnit`, and now Table 112's base encoding with Table 120's `/MissingWidth`. **A
demand curve cannot rank a requirement no file exercises, and five of those six were invisible
to it.**

### 0. The ledger, and the cheapest reviews available

- **Work `REVIEW_OWED` down.** 4 clauses, each already cited by the code that implements it, so
  the reading is against something that exists. Take them by family — §8.6.4's two rows and
  §8.7's two are the pairs left — because that is how the standard distributes its requirements, and because §9.6.5.4 was missed
  for the opposite reason: nobody had read §9.6.5 as a unit. **Expect findings.**
- **Prefer the family belonging to whatever else the session is doing.** Done: §7.4.6, §7.6,
  §7.9.2, §8.6.4.2, §8.6.6, §8.6.7, §8.6.8, all of §8.9, §8.10, §9.3, §9.4, §9.6.4, §9.6.5, §9.7,
  §10.7, §11.3.7, §11.4, §11.5, §11.6, §11.7 — the whole of clause 11 — §12.5, §12.7 through
  §12.7.5, the whole of §8.4, §8.5 and §8.6.5, §8.6.4, and now **the whole of §9.6 and §9.8**.
  So the families left are
  elsewhere: §9.9 whenever an embedded program's *packaging* is (`/Length1`, `/Length2`, subset
  tags), §7.8 whenever a content stream's structure is, and **§12.7.6 with
  §12.7.8** whenever anything about a form's *behaviour* is — those two and §12.8 are the whole of what is
  left in clause 12 outside §12.6's actions. Record every row, including the `inapplicable` ones —
  a clause read and dismissed is worth as much as one implemented, and costs a minute.
- **Three `silent` rows are left, and two of them are new.** §8.11.4.4's usage dictionaries — a
  layer that should switch itself off by zoom, language or print state, drawn with nothing said —
  is last on purpose, because it needs a layer panel to be worth more than a report. The other
  two arrived in the thirtieth session by reading §9.8.3: `/Style /Panose` and `/FD` are read by
  nobody, and while neither can change an *embedded* CIDFont's glyph, both would change which
  installed face stands in for one that is not. Their debt is substitution quality rather than a
  clause gap, and it is written on the rows. Only §8.11.4.4 is a silence where a report is the
  cheapest honest move. Its method is trap 11's, unchanged: an `eprintln!` naming the
  documents that carry an `/AS` array *and* a group whose `/Usage` would turn it off at the
  resolution we draw, before any condition and long before any code.
- **One silence still hides *inside* a `partial` row** — §10.7.3's `/SM`. Three others were
  there a session ago: §8.9.5.2's general `/Decode` array, and §8.6.5.5's and §8.6.5.6's
  treatment of an image's colour space, all closed by ADRs 0034 and 0035. Worth remembering when
  reading the ledger by status: a clause can be half implemented and quiet about the other half,
  and a `partial` row's note describes what somebody found rather than what is there.
- **A silence is not the same as a gap.** The nineteenth session closed two and they closed
  differently: §10.7.5's `/SA` was implemented in the half a display can state and recorded as a
  departure in the half it cannot, and §11.7.4's overprinting was six rows that a reading of Table
  146 removed altogether. So the first move on a silence is neither a report nor a feature: work
  out what the clause asks *of this device*.

Five small items, listed before the big lists because they are small:

- **Give §8.11.4.4's usage dictionaries a condition, and then a report.** The last `silent` row.
- **Bound a group's buffer to the band its clip admits.** The CPU backend gives every transparency
  group a page-sized pixmap, because a group's elements resolve their clips against the *target*.
  No corpus page pays for it, but a page with hundreds of groups would. Measure before building:
  `callgrind_rasterise` over a group-heavy page, and the sixteenth session's lesson about a
  benchmark that measures nothing applies.
- **Sandbox the interpreter and rasteriser too.** Spike D exists and is exercised; the rest of the
  renderer still runs in the main process, which is the half of principle 3 not yet built. The
  protocol would have to carry a display list rather than an image, which is a real design
  question.
- **Profile the median page.** We are 1.66× slower than `hayro` on the typical corpus page and
  nobody has looked at why — the seventh session's two fixes were both to outliers and moved the
  median not at all. The typical page is small and text-heavy, so the candidates are parsing, font
  loading and per-page setup rather than rasterisation, but that is a guess.
- **Carry an image and its sampling intent to the backends, rather than a finished raster.** One
  `pdf-render` change that unblocks three items on this list, and the reason they are one question
  rather than three: reduction happens at *decode* resolution today (`Image::area_averaged` works
  in whole source samples and leaves a residual under two-to-one to the backends' own filters,
  which is a good approximation of §10.7.4's per-device-pixel rule and not the thing itself); a
  mask of a very different size is bounded rather than composited at device resolution (ADR 0024,
  and `issue16263.pdf` still trips it); and **the JPEG 2000 decoder cannot be given a target
  resolution**, so `issue19517.pdf` is refused for being 212 megapixels where the format's own
  answer is to decode a lower resolution level. All three need the scale a page is about to be
  drawn at to reach `image.rs`, which the display list deliberately does not carry — so this is a
  question about where decoding and resampling belong, not a parameter to thread.

### 1. Work the unexplained list

`CONTRADICTED_UNEXPLAINED` in `oracle.rs`: 58 pages carrying no undrawn annotation, no hidden
optional content and no substituted font, so the difference is in something we believe we
implement. **Read trap 9 before starting**, because an entry may be any of its three shapes, and
checking costs a web search of the other project's source.

One cause is identified, measured and live: **`mesh_shading_empty.pdf` differs by the
subdivision lattice** — filling a Gouraud triangle as many flat sub-triangles rather than
interpolating per pixel (§8.7.4.5.5). Its entry used to say "displaced horizontally", which the
twenty-eighth session refuted in ten minutes with the two rasters: the edges are in the same
columns to the pixel and only *structural* similarity fails, 0.972 against 0.990. Closing it
needs a Gouraud rasteriser in **both** backends, since the cross-backend scenes hold them to
identical pixels. **Measure an entry before believing its label, including a label written
here.**

Three entries that used to be here are the argument for spending the hour, because none was one
page's problem. `issue20504.pdf` was worth **15 of the 81**: it looked like one page's
`/Differences` quirk and was a whole subclause (ADR 0015). `close-path-bug.pdf` looked like one
page's closed path and was **every dashed line in every document**. `issue11279.pdf` looked like
one page and was §8.10.1 step c) — a form XObject's `/BBox` clipping nothing, on every form since
the first one. Against that, four `knockout_*.pdf` entries left this list by starting to *report*
rather than by being fixed. The only way to find out which kind an entry is, is to open the
artefact: `<target>/tmp/oracle/<stem>/p<n>/` holds our render, each reference's, a side-by-side
strip and a difference heatmap. **Look at the side-by-side first.**

Two cautions. A page may be contradicted for a reason other than the one its group names —
`calgray.pdf` sat under substituted fonts and differed in its colour, which is how ADR 0012
started. And principle 5 is not suspended by a list: each entry is a question to take to the
specification, and "make it match mupdf" is exactly the failure this project forbids.

### 2. The features the corpus still names

- **A password prompt** (8 documents) is all that is left of encryption, and it is not a clause:
  §7.6.4.1 says "the interactive PDF processor should prompt for a password" and
  `Document::open_with_password` already takes one. It needs a dialogue, a retry loop and a
  decision about where a wrong password is reported — `viewer-ui` work that nothing else on this
  list depends on.
- **§12.5.6.19's redaction overlay and a widget's `/R`** are the two edges §12.7.4.3's layout left
  behind, and neither is reached by any corpus document. Table 193's `/OverlayText` is text drawn
  over a redacted region and the layout routine now exists for it; Table 192's `/R` rotates a
  widget's contents inside `/Rect`, which no background could see and one line of text can — it is
  reported where a widget states one and has text to put in it.
- **§12.5.6.10's text markup appearances** (8 documents) are *not* that job. The clause states no
  mark, and the three references draw three different pictures; anything built here is a
  documented choice about what a highlight looks like, and it should be argued as one rather than
  copied from a renderer. Read ADR 0030's refusal argument before starting.
- **Colour-managing an image in parallel** is what the twenty-sixth session left behind rather
  than a clause gap. An `ICCBased` image is now converted through its profile (ADR 0035), which
  is work that was not being done, and interpreting `issue19971.pdf`'s 3.4-megapixel photograph
  went from 30 ms to 120 ms. The loop is embarrassingly parallel apart from its memo, one cache
  per row band would keep it exact, and this tree already has rayon. Nobody has tried it, and
  the sixteenth session's lesson about benchmarks that measure nothing applies.
- **Predefined `CMap`s** (12 documents) are a decision about vendoring third-party data and its
  licence, not an algorithm. **Vertical writing** (4) is §9.2.4's `/W2` metrics rather than §9.7.
  **Type1 fonts** are smaller than they look: no corpus page one reaches one.

### 3. Where the time went, and where it still goes

**There is one fair thing to measure against.** Every other renderer here is C, so a timing
difference against `poppler` confounds the language, the allocator and thirty years of tuning.
`hayro` is Rust, forbids unsafe as we do, and rasterises on the CPU single-threaded as we do.
`cargo run --release -p hayro-compare --bin hayro-speed -- <files>` renders page one of each file
with both, alternating, best of N.

| | |
|---|---|
| total, ours | **7.1 s** against `hayro`'s 32–41 s, over 818 complete pages |
| **median page** | **2.14× slower** |
| worst page | 32× (was 34×, was 225×) |

**The totals and the median answer different questions and only quoting both is honest.** In
aggregate we are 4.5× to 5.8× faster — the range is `hayro`'s own run-to-run variance on this
machine, which is a reminder that wall-clock numbers lie — because their distribution has a long
tail and ours no longer does.

**The median is 2.14× and the number an earlier version of this file quoted was 1.66×, and the
denominator is the whole difference.** That measurement was over 685 pages; there are now 818,
because four sessions of features moved 133 more pages into the "we draw this completely" set, and
the ones that moved are annotation-heavy and text-heavy rather than typical. Measured at the
previous commit on this machine in the same sitting, the median is **2.17×** — so the
twenty-third session moved it by nothing, and neither did the growth of the set move *our* total,
which was 7.01 s before and 7.07 s after. Recent sessions' interpretation costs, by callgrind on
`examples/callgrind_interpret`: text rendering modes +0.46%, masking +0.12%, soft masks +0.05%,
composite fonts +0.44%, constructed appearances +0.34%, variable text +0.31%, and §8.4 and §8.5's
path rules **−0.21%** — collapsing consecutive `m` operators and dropping a trailing one leaves
fewer commands to build than the rules cost to apply. On the far side of the display list,
§8.5.3.2's split costs **+0.15%** on the corpus's most stroke-heavy page
(`22060_A1_01_Plans.pdf`, 35.69 G against 35.64 G) and **+0.001%** on the specification page,
because `split_degenerate` returns without allocating for a path that has no degenerate subpath
and `dashes_showing_direction` returns immediately for a butt cap or a pattern with no zero-length
dash. `callgrind_rasterise.rs` exists because
the first example stops at the display list, so a backend change measures as exactly zero there;
area averaging cost between −2.4% and +9.0% depending on the page, and the corpus gate could not
see the difference.

**Still open, and the largest items.** This profile predates two fixes and its shading half is
still live:

| on `bug1721218_reduced.pdf`, 16.1 G instructions | share |
|---|---|
| `tiny_skia::pipeline::lowp::gradient` | 29.7% |
| `pdf_model::function::Function::parse` | 23.2% |
| `pdf_model::function::Function::eval` | 13.8% |
| `ColourSpace::to_rgb_at` | 2.6% |

**The gradient stage** is the largest single item because a `Ramp` carries 256 samples, so a
shading becomes a 256-stop gradient and `tiny-skia` scans its stops per pixel batch; handing the
*rasteriser* fewer stops would fix it, while coarsening the `Ramp` in the display list would lose
fidelity and is not the same thing. **Roughly 40% of that run is building the shadings** rather
than drawing them: a function is parsed and then sampled 256 times per shading, and that page has
3576 of them. Whether that is 3576 *distinct* functions or one re-parsed 3576 times has never been
checked, and it decides whether the fix is memoisation by object reference or something harder.
One caution: `to_rgb_at` was 2.6% when `CalGray` was a pass-through; it now runs a Bradford
adaptation and a matrix per colour, and per *sample* for a Cal-space image.

Two fixes are worth carrying as patterns. Unpacking JPEG output was 6.89 G instructions on one
page — nearly twice what `zune-jpeg` spent decoding it — because a `match`, three bounds-checked
`get`s, saturating arithmetic and a re-checked `extend_from_slice` all ran *per pixel*; two paired
`chunks_exact` iterators took it to 1.25 G. **The safety habits this project enforces everywhere
are expensive in a loop that runs per pixel, and that is exactly where the profile should be
consulted rather than the habit.** And a mesh triangle was subdivided by colour alone, so one
covering a tenth of a pixel still split into 4096 filled pieces; `Triangle::is_subpixel` is a
correctness statement rather than a trade, and it took `personwithdog.pdf` from 17.3 s to 1.06 s
**while moving every mesh page closer to the references**. A change made for speed that improves
fidelity means the old code was doing work that was worse than useless.

### 4. Reproducing the numbers above

The oracle survey is `oracle.rs` and the corpus counts are `corpus.rs`; both print their evidence
per document. The ledger's counts come from `cargo test -p conformance -- --nocapture`.

Two classification counts are still throwaway, deliberately — scratch-quality diagnostics do not
belong in a repository held to `clippy::pedantic`. **Whether a page's fonts are embedded** walks
each `/Font` resource and its `/DescendantFonts` for `/FontFile`, `/FontFile2` or `/FontFile3`.
**The annotation subtype breakdown** comes free from the corpus gate's own output:
`grep -o 'Annotation { detail: "[^"]*"' | sort | uniq -c`.

### 5. What the two gates report today

Corpus, ratcheted in `crates/pdf-model/tests/corpus.rs`; the numbers only go down, except where a
rise is a new report and is written down as one.

| | count | |
|---|---|---|
| unopenable | 0 | and it should stay there |
| needs a password | 8 | §7.6.4.1's prompt is the missing piece, not the clause |
| encrypted beyond this reader | 2 | 1 is `/R` 5, which the standard states no algorithm for; 1 is a file whose `/Encrypt` does not resolve to a dictionary |
| no page one | 11 | unrecoverable page trees; 2 of them are encrypted files that authenticate and then fail to inflate, which `poppler` reports of them too |
| draws incompletely | 130 | Counted by each document's *first* report, so the column sums: 66 a font, 18 a transparency group or mask departure, 17 an annotation, 12 an image, 9 an operator, 4 an object composited in parts, 1 a font selected by an `/ExtGState`'s `/Font` (§8.4.5), 1 an undecodable content stream, 1 a text knockout, 1 a bound reached |
| slower than 30 s | 0 | `KNOWN_SLOW` is empty, and the next document to cross the budget fails the gate |

- **The `Content` row was 10 and is 1, and the `Operator` row 12 and is 9** (ADR 0031). Nine of
  those ten content reports were an encrypted `/Contents` refusing to inflate because it was
  ciphertext, and three of the operator reports were the same ciphertext lexing as operator names.
  Six of those twelve documents now draw with nothing reported and six say they need a password.
  Nothing on either row is a feature.
- **The annotation row was 67, then 24, and is 17** (ADRs 0030, 0032): 13 text markup (§12.5.6.10,
  counted per annotation rather than per document, which is why they outnumber the 8 documents),
  5 a `/DA` naming a font the `/DR` does not define, 1 a check box the file calls on with no mark
  stated for it, 1 an appearance stream with no `/BBox`, 1 an unknown subtype, 2 a `Line` whose
  `/LL` or line endings state no geometry, 1 an `Ink` with no usable `/Rect`. **Nothing on it is a
  `/NeedAppearances` and nothing is a field value.**
- **The font row was 100 before ADR 0029 and is 67.** Counted as *fonts* rather than documents:
  27 with no `/ToUnicode`, 23 whose substitute draws none of their declared codes, 15 naming a
  predefined `CMap`, 4 asking for vertical writing, and the rest malformed programs. Nothing on it
  is a `CMap` question any longer.
- **The operator row was 33** until the text rendering modes landed and is 15. Nothing on it is a
  feature: `BT` without `ET`, `BDC` without `EMC`, and the byte soup a fuzzed content stream lexes
  as operator names.
- **The image row was 161 before JBIG2 and JPEG 2000, and is 11** — one image apiece, and nothing
  on it is a feature: 4 malformed streams, 3 bit depths the unpacker refuses, one `/Mask` that is
  not an image mask, one JBIG2 segment type ISO/IEC 14492 does not define, one 212-megapixel JPEG
  2000 scan, and one `/SMask` of 34862x4332 against a 2x2 image.
- **The shading row is gone.** It held 28 documents and every one was a soft mask in an
  `/ExtGState`, filed under shading because nothing else fitted.

Oracle, ratcheted in `crates/pdf-model/tests/oracle.rs` by name and in both directions.

| of the 1620 pages we call complete | count | |
|---|---|---|
| agree with the reference consensus | 760 | |
| **contradicted** | **99** | 8 page rounding, 7 a shared JBIG2 decoder, 1 a shared *gap*, 4 a link border two references do not draw for two unrelated reasons, 1 a sub-pixel image, 1 a `CalRGB` alternate, 1 an eight-bit mask value, 2 glyphs judged as vector, 1 a symbolic font reaching an empty glyph, 15 substituted fonts, **58 unexplained** |
| ambiguous | 751 | the references disagree with each other; 372 are two long books set in fonts nobody embedded |
| our page geometry differs | 0 | all three were `/UserUnit`, applied in the twenty-ninth session (ADR 0038) |
| not comparable | 8 | fewer than two references produced an image, or they disagree on the page size |

The 174 incomplete pages are compared and printed too, but cannot fail the gate: a page we already
say we cannot draw is expected to differ. **The gated set has been the same 1620 pages for seven
sessions**, which is why the last six moved `agrees` and `contradicted` without moving either
denominator: every one of them fixed or clarified a page already in the comparison rather than
adding one. Before that it **grew by 9 in the twenty-third,
by 8 in the twenty-second and by 46 in the twenty-first**, and by 32 in the twentieth, all as
reports stopped firing; it *shrank* by 8 in the seventeenth as
two silences ended, and by 43 in the eighth, which is the cost of honesty and the reason a report
should never be reached for as a way of making a contradiction go away.

**Where the oracle's time goes, measured and printed by the gate itself.** It used to be roughly
1000–1300 s of processor time in the three external renderers against 45–55 s in ours — so **the
gate was essentially a measurement of `pdftoppm`, `mutool` and `gs`**. ADR 0020 is the answer, and
the run is now ~34 s with ~23 s in them at a 99.7% hit rate, every verdict unchanged, which was
checked by running the whole corpus both ways. What is left is ours: roughly 600 s of processor
time over 24 cores on our own render, the comparison and the artefacts — the SSIM and heatmaps for
the thousand pages that are not agreement — so if 34 s ever becomes the constraint, that is where
to look and not at the subprocesses.

**The time budget reports; it cannot enforce.** A Rust thread cannot be cancelled, so a document
that never returns hangs the suite rather than failing it. A real budget has to live inside the
interpreter and the rasteriser. `PDFVIEWER_CORPUS_TRACE=1` names each document on stderr as it
starts and finishes, which is how a hang gets identified from a killed run.

**`doc/pdf.js` is a submodule** (Apache-2.0, pinned at v6.1.200) holding those 974 PDFs and 459
more behind link files. It is optional to clone — every test that uses it reports being skipped
rather than failing — but the ratchets only mean anything where it is present, so CI must have it.

## Habits these sessions earned

Each of these was paid for once. The traps above are about code; these are about how to work.

**A default written in a table is not a suggestion, and a comment arguing for a nicer one is a
preference wearing a reason.** `/MissingWidth` defaults to 0 (Table 120) and this tree used half
an em, with "spacing degrades gracefully rather than collapsing to zero" written above it. That
sentence is true and is about nothing the standard says; a producer who wants half an em can
write half an em. It cost `issue7439.pdf` six half-ems of invented space in one line of text.
When a constant carries a justification, check whether the justification is a *reading* or a
taste.

**A page can leave the contradicted list without a pixel moving.** The oracle picks a page's
tolerance class from whether we could read text back, so anything that improves text extraction
can loosen a bound. `issue3566.pdf`'s raster is byte-identical before and after the change that
"fixed" it. Take the digest of the raster before writing "fixed", and if it did not move, the
news is about the instrument: `has_text` asks whether we could *name* what we drew and means to
ask whether we drew glyphs.

**Where two subclauses each condition a branch on one of two flags, the clause that defines the
flags breaks the tie.** §9.6.5.4 has one branch for the Nonsymbolic flag and one for the
Symbolic flag and cannot decide a font that sets both; §9.8.2 calls the pair "a historical
accident" and says a processor "should always check the Symbolic flag". Two pages away, in the
clause about the *dictionary* rather than about the algorithm.

**A clause's last paragraph can invert its first.** §12.7.4.3 opens by describing a processor
constructing an appearance stream and closes by describing it *splicing* one — "replace the
existing contents of the appearance stream from … BMC to the matching EMC" — and only the closing
sentence says what happens to a stored stream. The rebuild reading is defensible from the opening,
agrees with the splice on every document that has a value, and differs on exactly the one that has
none. Read the whole subclause before believing the sentence that answered your question.

**Two references agreeing is evidence — once you can say what they agree *about*.** Trap 9's three
shapes are all about agreement that means nothing, and this is the converse that belongs beside
them: `poppler` and `mupdf` blanked a page we drew, the clause said why, and the reading that
explained their output was the one the clause states. Agreement is evidence *after* the clause is
read, never instead of reading it.

**An eager lookup on a cold path is a hot-path cost when the path runs per object.** Reading the
catalog's `/AcroForm` to give every constructed appearance its `/Resources` is obviously cheap and
was 2.7× the whole feature's cost, because a specification page is full of link borders and none
of them names a resource. Measure what runs per object, not what looks expensive.

**Ask what a feature looks like when its parameters are not their defaults.** §9.7 gives a
composite font two mappings and under `Identity-H` with `/CIDToGIDMap /Identity` both collapse to
nothing, so nineteen sessions of real documents never asked what either one *is* — the tree was
not missing an edge case, it was missing the clause, and it could not tell because the clause's
degenerate case is the common case. `Tk`'s initial value is the same lesson from the other end: a
parameter whose default is the unimplemented behaviour is a gap on every page in the world.

**A presence condition is not a restriction on meaning.** Table 115 says `/CIDToGIDMap` is
"Required for Type 2 CIDFonts with embedded font programs" and then, in the next sentence, what it
*means*. The first implementation read the first sentence as bounding the second and drew one page
as garbage. When a clause conditions something, read what the condition is *about*.

**A rule whose common case is the identity is a rule nobody tests, and the test written beside it
will agree with it.** §7.6.4.3.2 step (a) appends "the first 32 − n bytes of the padding string";
the first implementation overlaid the password onto the padding string in place. For the *empty*
password — the one §7.6.4.1 makes every reader try first — the two produce the same 32 bytes, so
nineteen documents opened correctly and every document with a password was refused. The unit test
beside the code asserted `padded[3..] == PAD[3..]`, which is the implementation restated. Write the
assertion from the clause's sentence, not from what the code does.

**When two clauses disagree, ask which reading makes a file's own words mean nothing.** §12.5.2
and Table 166 have a reader ignore `/CA` beside an appearance stream; §12.5.5 says to composite
with it. Honouring both applies `highlight.pdf`'s 0.8 twice and gives 0.64 — so the two-statement
reading is also the one that preserves what the producer said. The twentieth session's `bfchar`
case broke the same way: follow the subclause describing what a *processor* does.

**A bucket that means "we failed" must not also come to mean "you have not told us the
password".** The corpus gate had one count for "cannot be opened", ratcheted at zero because
*every file yields something* is worth guaranteeing. Eight password-protected documents would have
broken it, and widening it would have thrown the guarantee away to make room for something that is
not a failure at all. Splitting the bucket kept the invariant and produced two new counts, one of
which — encrypted by something we do not implement — is the only row in that gate that is a
*decision* rather than a debt. **When a ratchet fires on a change you believe in, ask whether the
category is wrong before you ask whether the number is.**

**Ask what the clause requires of *this* device before deciding it is a gap.** Overprinting was 63
documents and six `silent` rows, and Table 146 read against a list of this device's colourants
says the special blend function is Normal here. **A gap sized by a corpus is a hypothesis about a
clause.** The same reading split §10.7.5 into one requirement to implement and one already
satisfied by a departure taken years earlier for another reason.

**"The clause says nothing" and "the clause says the opposite" are different findings, and only
one is a licence.** Two places recorded image reduction as unspecified, meaning §8.9.5.3, which is
about magnification and genuinely is silent. §10.7.4 is not: "there shall not be averaging over
the pixel area". Both sentences produce the same code; only the second produces a *departure*,
which has to be argued, recorded and costed. When a comment says the standard is silent, ask not
"is that true of this clause" but "which clause would say it".

**A departure is only honest once you have looked for the others.** Finding §10.7.4's image
sentence made it necessary to read the rest of the subclause — and its first rule, painting any
pixel a shape touches at all, has been departed from since the first commit by anti-aliasing, with
no clause cited anywhere near it. One departure looks like a compromise; three in one subclause,
all in the same direction, is a *reading* of what the clause describes.

**Where the standard defers to another document, the deferral is a citation.** §9.7.5.3 hands a
`CMap` file's syntax to Adobe Technical Note #5014, so ISO 32000-2 never states that a
`notdefrange` gives its whole range one CID while a `cidrange` numbers upward. A test caught it;
reading had not, because the sentence is in a document the standard points at.

**Where the standard defines nothing, refusing is a result.** `issue6621.pdf`'s `/Mask` is a
one-bit greyscale image where Table 87 requires an image mask; the only reading its samples admit
blanked a court seal three renderers draw, and the alternative reading would invert every stencil
whose author forgot `/ImageMask`. So: neither, and the entry is named. The same argument keeps
§12.5.6.10's text markup unbuilt.

**A convention that agrees with the specification is worse than one that does not, because it
removes the reason to write the rule down.** `tiny-skia` draws a zero-width stroke as one device
pixel, which is exactly §8.4.3.2, so the clause was never stated anywhere and every `0 w` line was
invisible on the GPU for fifteen sessions. The twenty-fourth session found the same shape
twice more and one of them the other way round: for §8.5.3.2's stroke with no length the two
rasterisers gave *three* different answers and none was the clause's, while for §8.5.4's empty
clipping path Vello happened to be right — which was verified by deleting the rule and watching
the scene still pass. **A backend that is right by convention is a rule that is not written
down**, and the test that would catch it going wrong does not exist yet either.

**A clause whose operators are implemented can still be unread, and reports nothing while it is.**
`J`, `j` and `M` set the line parameters from the interpreter's first commit; Table 57's `/LC`,
`/LJ` and `/ML` — the *same three parameters* by §8.4.1's other route — read nothing for
twenty-three sessions, on three corpus documents, in silence. Where a clause offers a parameter
two routes, implementing one and not the other is invisible to every gate that renders a page.

**A rule that changes nothing today can become load-bearing tomorrow, and the trigger is the
clause beside it.** Table 58's rule that one `m` overrides the previous one changed no pixel while
a single-point subpath painted nothing, and became mandatory the instant §8.5.3.2 made one a dot —
205 unwanted dots on one page of `bug1743245.pdf`. That is the argument for reading a family
rather than a subclause: the sentence that makes another sentence matter is usually two pages
away.

**An assumption a test cannot exercise is not tested, however many tests run over it.** The GPU
backend demultiplied Vello's output for fifteen sessions; Vello does not produce premultiplied
alpha. Nine cross-backend scenes and 1794 oracle pages could not see it because every one was
rendered onto an opaque background, where the conversion is the identity. A *constant* input
property is invisible to every test that shares it.

**A clause about the whole page can be invisible until one construction needs it.** §11.4.7 is two
paragraphs saying the page is an isolated group, and it decides how *every* blend mode in *every*
document composites against unpainted paper. It stayed `unreviewed` through three reviews of clause
11's other families, because nothing had a reason to render onto transparency. What made it
findable was building the thing one level down.

**One dictionary, two clauses, and only the second says who wins.** §8.9.6 defines what an image's
`/Mask` means; that an `/SMask` beside it "shall override any explicit or colour key mask" is in
§11.6.4.3. An implementation reading only clause 8 is complete by its own lights and wrong on any
file writing both. When a key appears in more than one clause's index entry, the clause that owns
the feature is rarely the one that states the precedence.

**A rule about how something is *encoded*, implemented as a rule about its value, is invisible
forever.** §9.3.3 applies word spacing to "the single-byte character code 32" and says in its next
sentence that it does not apply to a byte 32 inside a multiple-byte code. This tree applied it to
any code numerically equal to 32, so every `Identity-H` string containing `00 20` had the rest of
its line pushed right. It took five minutes of reading the clause.

**A subclause is a checklist; check the code against it, not the code against itself.** §9.6.5.4
is two pages naming five distinct routes from a code to a glyph, and the code that stood in for it
implemented about one and a half — self-consistent, commented, and right about every document
anyone had opened. Open the clause, list its rules, ask of each one where it is.

**A gap inside a feature you have implemented does not announce itself.** Every missing
*subsystem* here reports, because somebody wrote the report while deciding not to write the
feature. The gaps that ship are the ones *inside* something implemented: `Tr` parsed with four
modes silently absent, `/SMask` honoured while `/Mask` was not, `CalGray` resolved and then
converted as `DeviceGray`, and `/Decode` read on four routes out of five — the fifth being
`DCTDecode`, which bypasses the unpacker because `zune-jpeg` hands back channels rather than
packed samples. **A fast path inherits none of the rules of the path it skips**, and the one it
skipped here drew a JPEG in complementary colours on a corpus page for the project's whole
life. Reading the specification asking "what have we not built" cannot find
those, because the answer is "nothing". Comparing output against another implementation can, and
has, ten times now.

**The purest instance is the `d` operator, and it is worth keeping as the archetype.** Every layer
of the dash feature existed — `Stroke` carried an array and a phase, both backends consumed it,
`set_dash` had a doc comment — and the one line that mattered read only the *empty* array, because
the content lexer flattens an operator's array operand and nobody wrote the case for a non-empty
one. Result: four crates that all look right, and not one dashed line in 974 documents. When a
feature looks finished, check the operand path from the content stream to the state.

**A constant that is a property of the state must reach every paint, including the ones that
replace the colour.** `ca` is not part of a colour; §11.6.4.4 makes it a property of the graphics
state applied to painting. A shading replaces the current colour, so the one line that returns it
dropped the alpha along with the colour it did not use — and the page that shows it says
`Gradient: .5` on its own face.

**Read this project's own lists for the sentences that admit ignorance, not only for the
counts.** "Has not been looked into" sat in `oracle.rs`'s geometry list next to its own answer
for many sessions, while the two entries above it named the very clause that explained it. A
count gets read every session; a comment saying *we do not know* gets read when somebody goes
looking for something to do, and nothing schedules that.

**A corpus document can check a decoder against itself, and it beats a second decoder.** An
LZW-compressed image must decode to exactly `width × height` bytes; a colour table to
`(hival + 1) × components`; a `/ToUnicode` stream must begin with a `CMap` preamble. All three
are stated in the *same document*, by the same producer, in dictionaries compressed separately
from the stream they describe. A decoder one code out of step produces a different length, so
matching to the byte across four thousand of them settles the question with nobody else's code
involved. Look for this shape before reaching for a reference: **what does this file already
say about itself?**

**The exact fix is often available, and it is usually better than the approximate one.** The
obvious answer to a per-pixel ICC conversion is a lookup grid with interpolation — what every
colour engine does, and it would have been a documented approximation with an error to argue
about. A memo keyed on the *input tuple* is exact, is simpler, and measured at 3249 M
instructions down to 1075 M on the page that hurt. Reach for the approximation after the exact
thing has been measured, not before.

**Look at what a safe idiom compiles to in a loop that runs per pixel.** `.round()` on a clamped
float is `roundf`, a library call: 205 M instructions on one page, 10.7% of it, to round three
numbers per pixel. `+ 0.5` and a truncating cast is the same answer on a non-negative domain.
Third time this project has found real money in a per-pixel loop, and every time the profile said
so and the reading did not.

**Measure the corpus before choosing between reporting a gap and closing it.** §8.9.5.2's
general `/Decode` array sat on the "not implemented" list with "reporting it would be a good
first move" beside it — and every `/Decode` array in all 974 documents is Table 88's default or
its exact reversal, so the report would have fired on nothing. Trap 11 prices a report in gated
pages; this one would have cost none and bought none. The scan that settled it took ten minutes
and also found the three things the ledger's note did not know about the same clause.

**A page can be visibly wrong inside a verdict the gate cannot fail on.** `issue7406.pdf` drew
a JPEG cyan-on-black against four references drawing red-on-white, and its oracle verdict was
`ambiguous` before the fix and after it, because the page is text-heavy and the references
disagree about its text. A ratchet watches `agrees` and `contradicted`; nothing watches a page
getting better or worse inside `ambiguous`, and 46% of the judged set lives there.

**Print what a condition matched before trusting its count.** Twice a report's first draft was
defensible from the clause and wrong about the corpus, and both times one `eprintln!` settled it in
a single run. A count is not evidence that a condition is right; the matched cases are.

**A report has a price, and it is paid in gated pages.** Trap 11 has the full form. Ask, before
adding a report, "on how many pages can this actually be seen?", and make the condition answer
that question rather than a looser one.

**Build the strong gate, then let its own output tell you it is wrong.** The table-attribution
checker — a table reference must name a table the clause beside it discusses — failed fourteen of
the tree's twenty-five references and **all fourteen were correct writing**. Behind an exception
list it would have been a gate that is mostly exceptions, which teaches a reader to ignore it.
What shipped asserts the weaker true thing and *prints* the title of every table cited, in which a
wrong pairing is obvious at a glance — and it caught a second wrong table three sessions later. **A
check whose false positives are all correct code is measuring the wrong property.**

**A suspiciously clean measurement is a reason to check the instrument.** The first four callgrind
numbers for area averaging were flat to four significant figures across pages that obviously do
different work: the benchmark was passing 4096 as a *total pixel* budget rather than an extent, so
every run panicked and callgrind faithfully counted the panic.

**Wall-clock benchmarks lie under load; count instructions instead.** A `Command::Fill` change
measured as a 24% regression and an 8.5% improvement twenty minutes apart, purely from background
build load. `cargo bench` claimed 8% for a change callgrind put at +0.46%. Always A/B in one
sitting, prefer the instruction count, and measure the *baseline on this machine* rather than
trusting a number in this file.

**Measure the instrument before deciding you are slow.** Eleven sessions treated the oracle's 85
seconds as the price of having an oracle. When a loop is slow enough to change *what you attempt*,
that loop is a thing to measure, not a constraint to design around.

**And when the first design of a fix is the obviously safe one, still measure it.** Refusing to
cache timeouts is unarguable in principle; with everything else cached it left two pages out of
1794 accounting for 46 of the run's 57 seconds. The rule that replaced it remembers them for a
week, prints how many it used, and has its cost and expiry written down.

**Measure before optimising, and delete what does not measure.** `glyph_for` builds a `FontRef`
per character, which looks like an obvious cache; caching it changed a dense page by less than
run-to-run noise, so the cache was removed and the reason written where the next person will look.
The same session's *real* win was hoisting a string allocation out of `substitute::find`: 1.37 ms
to 18 µs.

**A bound written for the pathological case can refuse a reasonable one.** `MAX_MASK_GRID` exists
because a 2×2 image with a 34862×4332 mask asks for 604 MB; applied flatly it also refused a
12608×16806 mask on an image of the same size, where combining costs what the image already costs.
The bound belongs on the *growth*.

**Silent caps are defects, not safety.** The interpreter dropped operands past the 64th, which
truncated any `TJ` array holding a justified line — three sentences on the specification's own
title page ended mid-word, with `unsupported: []`. Bounds against hostile input are right;
reaching one without saying so is not. Every bound now reports, including the one §12.7.4.1
forbids outright.

**A test that skips silently is worse than no test.** `tests/ccitt.rs` was first written against a
document whose fax data turned out to be inside a Type 3 font's glyph descriptions; both tests
printed "skipped: the submodule is not checked out" and passed while checking nothing. A missing
corpus is a skip; a present corpus that lacks what the test needs is a **panic**.

**A test written to isolate one rule finds what a corpus cannot** (trap 8), and **a corpus can
state an invariant about itself, which beats any reference**: 96 corpus documents encode one image
ninety-six ways, and demanding they agree needs no external renderer, so principle 5 is not even
in tension. Look for that shape — a corpus varying one thing while holding another fixed is
stating a testable invariant.

**A citation nothing checks is a citation that rots.** The first tooling pointed at this tree's
146 references found two clause numbers naming nothing and three of five sampled quotations that
were paraphrases inside quotation marks. What is worth carrying is that **it kept finding errors
after the obvious ones were fixed**: the corrected `/Mask` citations were still wrong, because
§8.9.6.2 is stencil masking and `/Mask` naming another image is §8.9.6.3, and no amount of
checking numbers against an index would have caught that. Only reading the clause did.

**A clause read and dismissed is worth as much as one implemented.** The ledger's statuses include
`inapplicable` and `writer-side` for exactly that, and filling one costs a minute against the 20
to 60 a real review costs. The trap is treating the ledger as a to-do list of features; it is a
record of questions asked, and "asked, and it does not apply to a screen" is a complete answer.

**A fallback that fills the page is worse than one that leaves it blank.** §9.6.5.4's predecessor
ended in "if nothing else matched, the code is the glyph index", and `issue5501.pdf` drew `v 0' '
W` for `What's an interval?` — confident, plausible, wrong and silent. The same fallback survives,
restricted to a font with no readable `cmap` at all, and the oracle proves the restriction is
load-bearing: put it back per-code and `issue17333.pdf` is contradicted immediately.

**A shortcut that is right on the common case is worse than one that is wrong on all of them.**
The Cal-space pass-through was nearly correct for `/Gamma 2.2`, which is what most documents
write, and badly wrong otherwise. Nothing distinguishes the two populations at runtime, so nothing
reported. "Close on the files I tried" is not a property you can test for.

**Two copies of a constant is one defect waiting.** Three `DeviceCMYK` conversions disagreed and
nothing looked wrong; when that was fixed the same shape survived one level down, with the
nine-constant D50-to-sRGB matrix in two files. It is now one function with a test that recomputes
all nine numbers from the two published matrices they were folded from — so a folded constant,
otherwise unreadable and unfalsifiable, has a derivation attached.

**Two rasterisers disagreeing is information, not noise — and two agreeing is not proof.** The
cross-backend test found that Vello needed the same mesh seam repair `tiny-skia` did, after a
comment here had confidently claimed otherwise. The other half was learned the hard way: both
backends positioned paints in the wrong space, in the same way, because the two libraries share
the convention that was misread.

**Two references against two is not a tie, and not a vote — it is a question with an answer.**
`Type3WordSpacing.pdf` splits them over a `d1` glyph's stroke colour, and Table 111 settles it:
"its colour" in the singular, the description "executed solely to determine the glyph's shape",
and an image mask admissible inside one because a mask "merely defines a region of the page to be
painted with the current colour". Read the clause first, and let a split tell you *where to look*
rather than what to conclude.

**An unimplemented feature has a default, and the default is usually "draw it"** — or, once,
"don't". That is why two unrelated renderers can agree while both being wrong, and it is a more
common failure of the oracle's premise than shared code. `mupdf`'s `FIXME: Calculate visibility
from array` and `ghostscript`'s `WARNING: OCMD contains VE` took minutes to find and settled a
page that had looked like three-against-one.

**Ask the reference the same question you asked yourself.** Two of three renderers were being
asked for the media box while we rendered the crop box, which put 54 documents beyond comparison.
A comparison harness has its own defects, they look exactly like ours, and the way to tell them
apart is to check the invocation against the clause before believing the verdict.

**What you measure decides what you build, so check what the measurement cannot see.** Eight
sessions were steered by two gates that both take the pdf.js corpus as their universe. The
ordering they produce is a demand curve, and a demand curve cannot rank a requirement no file
exercises, cannot notice a clause nobody implemented, and converges on "done" the moment the last
file goes green. §6.3.2.2 ranks optional content first among what is left; the corpus ranked it
seventh.

**A dependency is a decision, and this project's own precedent decides it.** `zune-jpeg` owns
`DCTDecode`, `skrifa` font parsing, `flate2` Flate, `tiny-skia` rasterisation, and
`hayro-jbig2`/`hayro-jpeg2000` the two hardest image codecs. Writing 19 400 lines of MQ coding
here would have been consistent with none of that. The cost is written down rather than assumed
away: two decoders we cannot fix ourselves. ADR 0014, and `deny.toml` for what the tree refuses.

**Look in `read-fonts` before writing font-format code.** An earlier handover specified ~80 lines
of CFF charset parsing plus two 256-entry tables, all of which already existed in
`read_fonts::ps`, which `skrifa` re-exports as `skrifa::raw`. ADR 0006. The same module holds
`type1`, `charmap` and `agl` — `agl` is enabled and carries the Adobe Glyph List.

**Profile before believing an explanation, even one whose arithmetic matches.** An earlier
handover attributed a 48-second page to page-sized clip masks and supported it with `3576 clips ×
485 kB = 1.7 GB`, which is exactly what the process held. The arithmetic was right about the
memory and silent about the time: callgrind put the masks under 4% and the gradient stage at
78.9%. A number that reproduces one symptom is not a diagnosis.

**A premise that reads like a fact does not look like a question.** "JBIG2 and JPEG 2000 have no
memory-safe implementation" sat in `PLAN.md` as the reason two filters were unimplemented, and it
was true when written and false for months before anyone checked. Any item deferred on an external
condition should carry the date the condition was last verified.

**A gate's numerator moves when its denominator does, and only one of those is news.** Keep the
denominator beside the numerator wherever a count is quoted, and say *which* pages moved and why.

**"Clippy clean" is a claim.** This file made it while eleven warnings sat in the tree, every one
from new files, because `allow-panic-in-tests` covers `#[test]` bodies and **not** an integration
test's helper functions. Whatever this file asserts about the tooling, run it once.

## Things worth knowing

- **The oracle's artefacts are the fastest diagnostic in the tree.** Every page that is not
  agreement leaves `<target>/tmp/oracle/<stem>/p<n>/` holding our render, each reference's, a
  side-by-side strip and a difference heatmap per reference. Open the side-by-side first: it is one
  image, four panels, ours leftmost, and it has explained every page it was pointed at. Pages that
  agree have theirs deleted, so what is on disk is exactly the set worth looking at.
- **A page's tolerance class depends on what *we* drew.** The oracle picks a text or vector
  tolerance from our own render's content, so a change that adds text to a page also loosens its
  bound — and can move it from "ambiguous" to "judged". When a page appears in the
  newly-contradicted list, check whether its bound changed before concluding the render got worse.
- **The sandbox is a flag, and the default is the safe one.** `--no-sandbox` decodes JBIG2 and
  JPEG 2000 in the viewer's process. It can be a flag only because both decoders are memory-safe
  either way: what it trades is panic containment and a memory ceiling. There is deliberately no
  path that falls back to in-process decoding when the worker fails to start.
- **A font is reported as a whole, and that is not fine-grained enough.** `FontError` is the only
  channel a font has, so a font that maps *some* of its document's codes draws those and says
  nothing about the rest. The eighth session narrowed this — a substitute reaching *none* of the
  declared codes is refused — but the general case needs a report where a glyph is *shown*, in
  `show_text`, which needs `LoadedFont` to distinguish "this code has no glyph" from "this code's
  glyph is blank", which a space legitimately is. Not hard; not done; worth measuring on the
  corpus before assuming the volume is manageable.
- **`Interpretation::text` is a readback of what was drawn**, accumulated by the same loop that
  places the glyphs, and `crates/pdf-model/tests/text_extraction.rs` compares it against
  `pdftotext` over the 14 specification PDFs. It is the only check that catches a code reaching a
  *plausible* wrong glyph, and it is known to bite: reverting the operand-cap fix scores 93.2%,
  and shifting every `/ToUnicode` entry by one code scores 58.7%. Extending it to the pdf.js
  corpus is a real opportunity — 974 documents against 14, needing only a tolerance, since
  `pdftotext` supplies the reference for each.
- **`doc/md/` is the specification, in a form code can read.** Markdown conversions of the 14
  specification PDFs, with real tables, committed — so a test may depend on it without a skip path.
  `ISO_32000-2_sponsored_EC3.md` is 24 MB and its 860 `##` headings give a clause number, a title
  and a line range apiece, which is the whole basis of the citation checker and the ledger. Two
  caveats: it is a *conversion*, so a quotation the checker cannot find may be an artefact — check
  `doc/`'s PDF before editing the comment — and one heading number (`14.8.4.7.3`) occurs twice.
  When you need spec data, extract it from there rather than writing it from memory: the
  `WinAnsiEncoding` and `MacRomanEncoding` tables came out of Table D.2 that way, and the
  extraction caught three things memory would have got wrong. The files carry base64 images
  inline, so `grep -v '^!\[Image\]'` before reading a range.
- **`doc/` holds more than ISO 32000-2.** `PDF20_AN001-BPC.md` is the PDF Association's
  application note on black point compensation, written by ISO 32000's own co-project-leader, and
  it settled a design question the base specification leaves to ISO 18619. It had been sitting
  unread while the same question was being answered by looking at other renderers. The sharper
  form of the same lesson: image reduction was recorded as unspecified in two places, and §10.7.4
  specifies it four clauses from one the tree cites constantly. `grep -n '^## '` over the
  conversion and read the *titles* around your subject; it takes a minute.
- **The Arlington model is the object model, not the semantics.** It says `/BaseEncoding` must be
  one of three names; it does not say what those encodings contain. Do not expect glyph data,
  operator semantics or rendering rules from it.
- **A command draws into the rows its clip admits, not into the page.** `Band` in
  `crates/render-cpu/src/lib.rs`, and ADR 0010 for why rows rather than a rectangle. Two
  consequences: the device transform handed to a command already carries the band's row offset, so
  anything new that composes a transform must use *that* one; and the clip mask is band-tall and
  page-wide, because `tiny-skia` needs it to share the pixmap's row stride.
- **The display list is deliberately flat.** `tiny-skia` wants per-clip masks, Vello wants a layer
  stack; both translate. That neither library's model is native is the evidence the neutral form is
  right, and it is what lets the CPU backend validate the GPU one on byte-identical input.
- **RADV and lavapipe produce byte-identical output**, so goldens need not be per-adapter. A test
  pins this; if it fails, the assumption has broken, not the code.
- **Pixel comparison cannot police text, so there is a second kind of metric.** The reference
  renderers disagree with each other at worst-tile 26–28 on text pages — glyph hinting, not error
  — and no threshold fixes that, because the noise floor is above the signal.
  `raster_compare::Comparison::structural_similarity` measures whether the same shapes are in the
  same places instead, and `Tolerance` bounds it: 0.99 for vector, 0.90 for text. Both were
  measured over 153 reference-against-reference pairs, and the doc comment records that the
  distribution is *continuous* — 0.8990, 0.8993, 0.8998 and 0.9009 all occur — so 0.90 is a choice
  about which population to exclude, not a discovered boundary.
- **Reference renderers are given 30 seconds and then killed.** A corpus holds files written to
  make a reader loop, and `Command::output` waits forever. `Reference::render_within` polls and
  kills; there is deliberately no unbounded variant.
- **`test-scenes` holds the same page twice**, as a display list and as PDF bytes. That pairing is
  what let the harness work before a parser existed, and a test renders both and demands identical
  pixels.
- **Debug builds are ~15× slower here, and it changes what a test can assert.** The corpus gate is
  2 s in release and minutes in debug. Any test with a timing assertion is meaningless at debug
  speed; run those in release and say so. The oracle gate is the exception that proves it: about
  95% of its processor time was three external renderers, whose speed does not depend on how we
  were built.
- `cargo-deny` is installed in the agent's `~/.cargo/bin`; run it before pushing rather than
  finding out from a red pipeline.
