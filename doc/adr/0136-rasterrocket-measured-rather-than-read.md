# ADR 0136 — `rasterrocket`, measured rather than read

Status: accepted, 2026-08-02. Session 154. **Not added to the oracle, and the reason is a
measurement rather than a preference.** One item is taken from it, priced, and it is not the one
its README advertises.

## What it is

`doc/rasterrocket` is a checkout of `rasterrocket` 1.2.0 (MIT, `pthomasfournet/rasterrocket`), a
PDF-to-pixels pipeline whose stated purpose is feeding Tesseract: "renders PDF pages to 8-bit
grayscale pixel buffers for direct consumption by Tesseract OCR", with a CLI positioned as a
"drop-in `pdftoppm` replacement". 93 218 lines of Rust over twelve crates, 1330 tests.

It is **not** a viewer and does not claim to be one. Comparing it to this tree is therefore
comparing two programs that were asked different questions, and most of the differences below are
consequences of that rather than disagreements.

| | this tree | `rasterrocket` |
|---|---|---|
| target | a page as its producer specified, on a screen | a page legible to an OCR engine |
| glyph outlines | `skrifa` | FreeType through `freetype-rs` |
| rasteriser | `tiny-skia`, and Vello on wgpu | its own, ~17 k lines, AVX2/AVX-512/NEON/SVE2 tiers |
| offload | wgpu (portable) | CUDA and Vulkan compute, nvJPEG, nvJPEG2000, VA-API, NPP |
| `unsafe` | forbidden in every crate | 335 sites — 207 GPU, 71 SIMD, 26 the mmap parser |
| §7.6 encryption | in-tree, both directions (ADRs 0031, 0129) | the external `qpdf` binary, decrypting to a temp file |
| unsupported input | typed, propagated, reported (`Unsupported`, trap 5) | "malformed operators — silently discarded" |
| the specification in the code | 3166 citations, 323 quotations, 823 ledger rows | none; `hinting.rs` cites `SplashFTFont.cc` instead |
| page-level regression tests | corpus 974, oracle 1794 pages, text 974, dates 1545 | a golden-image harness whose case list is empty |

Two rows are worth stating plainly rather than leaving in a table. Its font module's own doc
comment says it "**mirrors `getFTLoadFlags` from `SplashFTFont.cc` exactly**" — poppler's hinting
policy, deliberately, on poppler's own library. And `crates/cli/tests/golden.rs` opens with
"Infrastructure is complete; CASES is empty until fixture PDFs are added", so nothing in the tree
renders a page and compares it to anything.

## What it draws

The second row's consequence is not hypothetical, and finding it took one minimal document.

`doc/pdf.js/test/pdfs/alphatrans.pdf` is a page of five labelled objects: three rectangles painted
with `re B`, an image at `ca 0.5`, and an axial shading through `sh`. Rendered at 72 dpi with
default flags, `rrocket` produces the text and the image — the image at full opacity — and **none
of the three rectangles and none of the shading**, on a canvas of the wrong size (612×792 for a
595×842 page). Exit status 0, nothing on stderr, nothing at `RUST_LOG=trace`. `pdftoppm` and this
tree agree with each other about the page.

Reduced: a four-object document, uncompressed, conventional cross-reference table, one content
stream of `m l l h f`, `re f` and `re f` in three colours. `pdftoppm` marks 3267 sampled pixels of
it. `rrocket` marks **none**, at 72 and at 150 dpi, and again exits 0.

So in this checkout, built for this machine with default features, **path painting does not mark
the page and nothing says so**. The cause was not diagnosed here and might be specific to a build
without CUDA or Vulkan — which would make it this project's trap 12b exactly, a fallback path that
no test in the suite walks. What can be said without diagnosing it is what an empty golden-case
list allows: 1330 tests pass over a renderer that draws no vector art, because not one of them
draws a page and looks at it.

## What it costs

Measured on this machine, page one, best of three, `rrocket` as a subprocess against this tree and
`hayro` in process through `hayro-speed --per-document` (the flag is new, and exists so that a
third renderer that is a *program* rather than a crate can be joined to the same table).

`rrocket`'s spawn, open, render and PPM write of a blank US Letter page is **7.35 ms**, and that
floor does not move with resolution — a 6.3 MB write at 150 dpi costs no more than a 1.4 MB one at
72. Since the floor is larger than what either renderer spends on most corpus pages, only the
pages where `rrocket` does more than 3 ms of work above it can be compared at all. Of a
98-document sample (every tenth of the pdf.js corpus), 91 joined; 17 clear that guard at 72 dpi
and 45 at 150.

| | 72 dpi | 150 dpi |
|---|---|---|
| pages above the floor guard | 17 | 45 |
| total, ours | **147 ms** | **462 ms** |
| total, `hayro` | 86 ms | 320 ms |
| total, `rasterrocket` | 859 ms | 1118 ms |
| median `rasterrocket` / ours | **4.52×** | **1.70×** |
| we are faster on | 16 of 17 | 36 of 45 |

**We are faster, and by more than the table says**, because a renderer that omits every path fill
is timed doing less work than the comparison asks of it. The direction of that error is the one
that flatters them.

**The interesting number is that the ratio halves between the two resolutions, and it is not
because their rasteriser scales better.** Two dense text documents, timed at both:

| | 72 dpi | 150 dpi |
|---|---|---|
| `tracemonkey.pdf`, ours | 18.56 ms | 34.56 ms |
| `tracemonkey.pdf`, `rasterrocket` | 106.4 ms | **106.4 ms** |
| ISO 32000-2 page 1, ours | 25.45 ms | 29.96 ms |
| ISO 32000-2 page 1, `rasterrocket` | 156.5 ms | **156.5 ms** |

Their time on a text page does not move with resolution at all, to within a tenth of a
millisecond, while ours grows with the pixels. Whatever dominates their page — the shape of the
numbers says glyph loading rather than glyph *drawing*, and it correlates with text density and
not with file size — it is a fixed cost per page, and ours is not. Extrapolated, the gap closes
somewhere above 300 dpi. **A viewer is a program that is asked for the same page at many
resolutions**, so that is the axis this comparison actually measured, and it is ours.

## Should it join the oracle

No, on four grounds, three of which are this project's own recorded traps.

1. **It does not report what it cannot draw.** Trap 9's first shape is a reference whose
   unimplemented feature falls through to a default; here the default is "draw nothing" and the
   exit status is success. A reference that votes confidently for a blank region is worse than no
   fourth vote, and the oracle's whole premise (ADR 0005) is that agreement is evidence.
2. **On text it would be poppler again.** Its own source says its load flags mirror
   `SplashFTFont.cc`, and it renders outlines FreeType produced. Trap 9's third shape already
   notes that `pdftoppm`, `mutool` and `gs` share `libfreetype.so.6`; adding a fourth reader of
   the same library under poppler's own policy adds a vote, not an independent one.
3. **It is not in this repository.** A gate that depends on an untracked directory skips silently
   everywhere else, and "a test that skips silently is worse than no test".
4. **As a speed reference the process boundary defeats it.** 7.35 ms of floor exceeds what page
   one costs on four fifths of the corpus. `hayro` stays the fair comparison: Rust, `forbid(unsafe_code)`,
   CPU, single-threaded, and linkable, so the clock starts inside the process on both sides.

## What is taken from it

- **Rasterisation is not parallel here, and their measurement is the second independent argument
  for making it so.** Session 153 measured a dense text page spending four to six times as long
  being drawn as being read; the table above adds that our page-one time nearly doubles from 72 to
  150 dpi while theirs does not move. `rasterrocket` splits the target into horizontal bands above
  a height threshold and fills them on rayon with no synchronisation — each band an independent
  `&mut` slice. `render-cpu` already has `Band` (ADR 0010) for exactly that geometry, for a
  different reason, and rayon is already in the stack. **Not built here, and not before the
  ceiling is measured**: our masks are built per clip band, so replaying a display list into *S*
  strips rebuilds a mask once per strip it spans, and `bug1721218_reduced.pdf` is 87% of
  `MASK_BUDGET` with 3608 chains. The next step is a number, not a patch.
- **Their glyph cache confirms ADR 0131 from the outside.** Its key is
  `{face_id, code, size_px, base_idx, aa}` — no sub-pixel phase, which is the quantisation this
  tree measured and refused. Their `tracemonkey.pdf` render is where the cost shows: adjacent runs
  collide, `jruderm@mozilla.com` overprinting the affiliation beside it. That is what the refusal
  bought, seen in someone else's output.
- **`libdeflate` is a real item and its answer is not a C library.** They swap `flate2` for
  `libdeflater` by default. Inflation is 28.0% of interpretation on the median page here, which
  makes it the largest single item in that profile, and principle 3 forbids the way they took it —
  untrusted bytes never reach unsafe code. The item stays open, with the price attached and the
  route closed.
- **`PageDiagnostics::suggested_dpi` is our open image item from the API side.** A render that
  tells its caller the resolution the page's images actually hold is the same information this
  tree needs to carry *into* the backends so that reduction happens at decode resolution. Evidence
  that the item is real, not a design to copy.

## What is not taken

Its GPU work is large, careful, and aimed elsewhere: CUDA kernels with Slang/SPIR-V twins and 15
parity tests is a serious piece of engineering, and it is engineering for a machine with an NVIDIA
card in it. This tree's offload is wgpu because a viewer runs where it is installed. Nothing about
their approach argues against that, and the parity-test idea — the same kernel written twice and
compared to within one level — is already this tree's cross-backend scene suite.
