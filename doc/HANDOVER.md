# Handover

Written 2026-07-26, updated 2026-07-28 at the end of the eighth working session. Read
`/CLAUDE.md` first — it holds the five non-negotiable principles and they are not optional.
**Principle 5 is the one that changes how to work**: the specification is the only source of
truth, and agreement with poppler, mupdf or pdf.js is evidence that we read it right, never
the definition of right. This file is the state of play, the traps, and what to do next.

## What the eighth session changed

**One subclause was missing, and it was worth 15 of the 81 unexplained contradicted pages.**
The previous handover named `issue20504.pdf` as "a good first task" — six scripts drawn as
`!"#$`, silently — and pointed at an unrecognised `/Differences` name. That was one symptom
of something larger. ISO 32000-2 **§9.6.5.4**, the algorithm that turns a PDF character code
into an index into a `TrueType` font's `cmap`, was not implemented at all: the code was
handed to `skrifa`'s `Charmap`, which selects the best *Unicode* subtable. The subclause's
whole subject is that **a character code is not a character** — each `cmap` subtable is
indexed by something else, and the subclause is the set of rules for turning a code into
whichever of those a font carries. Handing the code to a Unicode subtable is right by
coincidence, for ASCII, in a font that has one. All four of `issue20504.pdf`'s subsets carry
a single (1, 0) Macintosh subtable, which is what §9.6.5.4's *own guidelines* tell a producer
to emit, so nothing matched and the fall-through drew glyph number `code` from a nine-glyph
subset. ADR 0015 has the whole argument. **Contradicted fell from 129 to 108.**

**Two silences it uncovered, both larger than the thing that found them.** A **Type 3** font
has no font program — §9.6.4 makes each glyph a content stream in `/CharProcs` — and all 24
corpus documents carrying one were reaching the *substitution* path, resolving procedure
names like `/a192` against a Latin system font. `issue918.pdf` drew 388 text operations of
letter fragments at the wrong places and reported `unsupported: []`. And a substitute was
being judged usable if it reached *any* of the 256 codes, which a Latin face always does, so
a font whose entire `/FirstChar`..`/LastChar` range mapped to nothing still passed —
`tracemonkey.pdf` is missing the © from its copyright line and has never said so. Both now
report. **The incomplete count rose from 250 to 290, and every document in that rise was
drawing wrongly or not at all, in silence.**

The handover's own claim that "bare Type1 and Type3 are reported" was **false for Type 3**,
and had been for as long as the table existed. It is corrected below. That is worth noticing
as a pattern: this file is written from what the code intends, and the corpus is the only
thing that checks it.

## What the seventh session changed

**JBIG2 and JPEG 2000 decode, in a sandboxed worker process, and the corpus's incomplete
count fell from 368 documents to 250.** The largest single fall so far, and the first that
came from a dependency rather than from code written here: the premise this project had been
holding — that neither format has a memory-safe implementation, so both must wait for a
sandbox to contain a C library — stopped being true. `hayro-jbig2` and `hayro-jpeg2000` are
pure-Rust decoders, both `#![forbid(unsafe_code)]`, and with their SIMD feature off they pull
in no unsafe code at all. **Spike D was built anyway** and both codecs run inside it, because
three of the sandbox's reasons survive the decoders being safe — a decoder panic would abort
a `panic = "abort"` viewer with the document open, `RLIMIT_AS` is the only memory ceiling
nobody has to remember to check, and principle 3 asks for the confinement regardless. It is a
flag, `--no-sandbox`, defaulting to on. ADR 0014 has the whole argument including what the
dependency costs. Two things were found on the way that had nothing to do with either codec:
a filter chain ending in an image codec was handing the codec *compressed* bytes, so
`[/FlateDecode /DCTDecode]` never decoded; and **`mupdf` and `ghostscript` are the same JBIG2
decoder**, which breaks the oracle's independence assumption on seven pages — see trap 9.

**And then the same question was asked of ourselves: are we fast?** `hayro` is the only other
feature-complete pure-Rust PDF renderer, which makes it the only reference this project can
compare *speed* against without confounding the language — everything else here is C.
`tools/hayro-compare` does it, and the first answer was that we were **1.61× slower on the
median page**, with outliers up to 225×. Our corpus total is now 7.1 s against their 41.8 s
and our worst page is 34× rather than 225×, but **the median is still 1.62× slower** and that
is the number left on the table. Two causes, both found with `callgrind` and both
fixed: our own RGBA conversion after JPEG decoding was 38% of an image-heavy page, and mesh
shadings were subdivided by colour alone, so a triangle covering a tenth of a pixel was still
split into 4096 pieces and filled one at a time. See "Where the time went" below. `hayro` is
also now in the oracle as a fourth *non-voting* reference, which is the only honest way to
have it: we share its font rasteriser, its deflate, its JPEG decoder and both new image
codecs.

## What the sixth session changed, in one paragraph

The fifth session built the oracle and it named 174 pages we claimed to draw that two
independent renderers contradicted. The sixth worked that list, and **it took them to 120**.
Two things did it. **`CalGray` and `CalRGB` are now converted through CIE XYZ** as §8.6.5.2
and §8.6.5.3 define them, instead of being passed through as their device equivalents: with
`/Gamma 1.0`, `A = 0.35` was rendering as 89 of 255 where all three references give 160 — a
mid grey drawn as a near-black, on every page anywhere using either space. And **annotation
appearance streams are drawn**, which was the largest single group in the contradicted list
and had been entirely absent: 148 of 988 first pages carry a visible annotation with an
`/AP`, and none of them was drawn *or reported*. That group loses 45 of its 47 entries. The
two that stayed turned out never to have been about annotations — they are a one-pixel raster
rounding difference on a small fractional page — which is exactly what the previous handover
said their staying would mean. Along the way the XYZ-to-sRGB matrix, which lived in two
places, became one; and the corpus's incomplete count rose from 291 to 368, every bit of it
new reporting, for the first time *because a feature landed*.

Both numbers have moved since: see the seventh and eighth sessions above, and the gate
reports at the end of this file, which are always current.

## Where we are

A PDF **renderer** that opens real files and draws pages: geometry, colour, images,
shadings, patterns, embedded text and annotation appearances, on both a CPU and a GPU
backend, with JBIG2 and JPEG 2000 images decoded in a confined worker process. It is not yet
a PDF *viewer* in the full sense — no forms, encryption or transparency groups — and the gap
between those two words is measured further down rather than guessed at.

- **246 tests**, `clippy` clean under `pedantic` + `unwrap_used`/`panic`/`arithmetic_side_effects`,
  `cargo fmt --check` clean, `cargo deny` clean on all four checks (verified, not assumed).
- **The 14 specification PDFs in `doc/`** — including ISO 32000-2 itself, 1023 pages and
  101 318 objects — all parse, all render page one with only a soft mask reported on three
  of them, and all extract **100% of the words `pdftotext` finds**.
- **The 974-document pdf.js corpus is a gate, not a survey.** All 974 open, 955 reach page
  one, **665 draw with nothing reported at all**, and everything the other 290 cannot draw
  is named. The counts are ratcheted. 1501 of 1501 PDF functions parse; **all 1793 shadings
  build**, mesh types included. The whole gate runs in **1.3 s** and has **no named slow
  document left**.
- **A second gate asks whether what we drew is *right*.** `oracle.rs` compares us against
  poppler, mupdf and ghostscript over **1794 pages** — every page of the corpus, plus page
  one of each specification PDF — in 79 s, of which about 48 s is our own processor time and
  some 1020 s is the three external renderers'. Of the 1426 pages we claim to draw completely,
  **634 agree with the reference consensus, 108 are contradicted by it and 673 are pages the
  references cannot agree on among themselves**. The 108 are named, grouped and ratcheted in
  both directions. ADR 0011.
- **JBIG2 and JPEG 2000 decode in a sandboxed worker.** `pdf-sandbox` confines it with
  resource limits, Landlock and a seccomp-BPF allow-list; `--no-sandbox` turns it off for
  trusted documents and says what that costs. The strongest evidence the decode is right is
  not a reference renderer: the corpus encodes **one image ninety-six ways** and all ninety-six
  produce byte-identical pixels. ADR 0014.
- **Colour resolves from the document.** `ICCBased` profiles are evaluated by an A2B
  evaluator written here, `CalGray`/`CalRGB`/`Lab` are converted through XYZ, `/DefaultCMYK`
  and output intents are honoured, and there is exactly one route from XYZ to a pixel and
  exactly one `DeviceCMYK` conversion. ADRs 0009 and 0012.
- **Annotations draw.** `/AP /N` is placed by §12.5.5's algorithm and run by the same
  machinery as any other form XObject; nothing is synthesised, and an annotation with no
  appearance is reported. ADR 0013.
- Both backends draw everything the display list can express, and agree on it: **eight**
  headless GPU scenes hold `tiny-skia` and Vello to the same pixels, at more than one scale
  and along both axes — see trap 2 for why that matters.

### Run it

```sh
cargo run --release -p viewer-ui --bin pdf-viewer -- doc/PDF20_AN001-BPC.pdf
```

Arrow keys / Page Up / Down / Space turn pages, Home and End jump to the ends, Escape
quits. The title bar names anything on the page that could not be drawn.

`--no-sandbox` decodes JBIG2 and JPEG 2000 in the viewer's own process instead of in a
confined worker: faster by a process spawn and a pipe round trip, and appropriate for
documents whose origin you trust. It prints a line saying what it gave up.

### Verify it

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets     # must be silent of lints
cargo test --workspace
# Both gates decode images in a separate program, and -p pdf-model does not rebuild
# another package's binaries. Build it first or the numbers below are somebody else's.
cargo build --release -p pdf-sandbox --bins
cargo test --release -p pdf-model --test corpus -- --ignored --nocapture   # 974 docs, ~1.6 s
cargo test --release -p pdf-model --test oracle -- --ignored --nocapture   # 1794 pages vs 3 voting renderers, ~75 s
# The oracle draws a fourth panel with hayro on every page worth looking at, and the same
# binary answers "are we fast?". Neither is needed for the gate to pass.
cargo build --release -p hayro-compare --bins
cargo run --release -p hayro-compare --bin hayro-speed -- doc/pdf.js/test/pdfs/*.pdf
cargo bench -p pdf-model                   # interpretation, the time-to-first-page path
cargo deny check
cargo +nightly fuzz run lexer -- -runs=50000     # from fuzz/, needs nightly
```

Cargo prints one line about `proc-macro-error2` being rejected by a future compiler. It is
not a lint and not ours: it arrives through `iai-callgrind`, a dev-dependency that reaches no
shipped binary, and `deny.toml` records the same exception with the reasoning. Nothing to
chase.

## Crate map

| Crate | Does | Notes |
|---|---|---|
| `pdf-spec` | Object-model validation tables | Generated from Arlington by `build.rs` |
| `pdf-syntax` | Lexer, objects, xref, filters, `Document` | Touches untrusted bytes first |
| `pdf-model` | Page tree, content interpreter, annotations, image decode | Where PDF semantics live |
| `pdf-font` | Glyph outlines via `skrifa` | Owns both encoding algorithms: §9.6.5.2 for CFF, §9.6.5.4 for `TrueType` (ADR 0015). `cff.rs` adapts `read-fonts`; `encoding.rs` is Annex D and Table 113 data; `substitute.rs` is the only machine-dependent code in the tree. A Type 3 font is refused here — its glyphs are content streams, so it belongs in `pdf-model` |
| `pdf-render` | Display list + `Rasterizer` trait | No PDF semantics, no rasteriser |
| `render-cpu` | `tiny-skia` backend | Correctness oracle **and** startup path |
| `render-gpu` | Vello/wgpu backend | Headless by construction |
| `raster-compare` | Tolerant image metrics | Worst-tile error is the load-bearing one |
| `test-scenes` | Shared fixtures | Holds the same page as a display list *and* as PDF bytes |
| `tools/pdfref` | Reference-comparison harness | Triangulation rule lives here |
| `viewer-ui` | The application | `src/bin/pdf-viewer.rs` |
| `pdf-sandbox` | Confined worker + the two image filters | Its `decode.rs` is the only place a JBIG2 or JPX codestream is looked at |
| `tools/hayro-compare` | Drives `hayro` for the oracle's fourth panel and for speed | Nothing ships it; it is where `hayro`'s forty dependencies live |
| `viewer-core` | Empty | Documented responsibility only |

Architecture decisions are in `doc/adr/`. `doc/PLAN.md` tracks phases and measured results.

## Traps — read these before writing code

### 1. The metrics lie about fonts. Look at the page.

This is the most important thing in this file.

Wiring bare-CFF support in made every affected document report `unsupported: []` — and
render **almost no text**. The font loaded, nothing was reported, the wrong glyphs were
drawn. It was caught only by rendering a page and looking at it.

`Interpretation::is_complete()` tells you what the interpreter *knows* it skipped. It
cannot tell you that a font loaded and produced garbage. For any font or colour work,
render the corpus pages and **look at them**. There is a test that writes the PNGs:
`cargo test -p pdf-model --test render_real_pdf -- --nocapture writes_inspectable`.
It covers both CFF routes, because no metric distinguishes them.

**Both halves of that were live again in the eighth session, and the second half is the one
to carry forward.** `issue5501.pdf` drew `v 0' ' W` where poppler reads
`What's an interval?`: its font's `cmap` was not being read the way §9.6.5.4 says to read it,
and the fall-through drew glyph number `code`. `unsupported: []`, plausible-looking output,
wrong text. And `issue918.pdf` drew 388 text operations of letter fragments because a **Type
3** font — which has no font program at all — was being given a Latin substitute, so the
procedure names in its `/Differences` array resolved to whatever they happened to match.

Both are fixed, and both were found by the oracle. What is worth keeping is the shape: in
each case the font *loaded*, so every metric this crate owns said the page was fine. A
loading font is not a working font, and the only thing that can tell them apart is pixels
someone else produced.

Two automated checks *do* catch a wrong mapping, both in `crates/pdf-font/src/lib.rs`:

- `the_pdf_widths_agree_with_the_font_programs_own_advances` — the document's `/Widths`
  and the CFF charstring's own advance are independent statements of the same fact, so
  they agree only if the code reached the glyph the producer meant. This is the strongest
  check in the tree: it verifies the mapping without consulting the mapping.
- `an_uncovered_code_has_no_glyph_rather_than_a_guessed_one` — pins the absence of the
  code-as-glyph-index fall-through.

Both were confirmed to fail when the defects they describe are deliberately reintroduced.
They are complementary: an off-by-one charset trips only the first, a reinstated
fall-through only the second. Neither replaces looking at the page, and neither caught
`issue20504.pdf`.

The third check is the first that looks at *pixels*: the corpus oracle compares our page
against three renderers that share no code with us. It cannot be fooled by a font that loads
and draws the wrong glyphs. What it cannot do is tell you *which* of a page's differences
matters, so it still does not replace looking.

### 2. A paint is positioned in the *path's* space, not the device's

Both `tiny-skia` and Vello apply the drawing transform to a paint as well as to the shape:
`Pixmap::fill_path` and `stroke_path` post-concatenate it onto the shader, and Vello
encodes a brush transform as `shape transform * brush transform`. So the transform you hand
a gradient, a pattern or an image is read **in the space the path is stated in**, and
composing the page-to-device transform into it yourself applies that transform twice.

Both backends did exactly that, and it shipped:

- Every gradient was **mirrored about the page's horizontal centre line**. At a scale of
  1.0 the page-to-device transform is a y-flip about the page centre and so is its own
  inverse: the second application cancels the geometry and leaves the flip. At any other
  scale it leaves a scale-dependent displacement instead.
- Every image was sampled through a doubled transform. `issue19971.pdf` draws a 2500×1364
  photograph and we drew one flat dark-blue rectangle.

Three things about how this survived are worth carrying forward:

1. **No metric saw it.** `unsupported: []`, the right shape, colours from the right ramp.
   Trap 1's warning about fonts is the same warning: look at the page.
2. **The CPU-versus-GPU comparison could not see it**, because both backends had it and
   therefore agreed. Two implementations agreeing is evidence *only where they can fail
   independently*.
3. **Every scene that compared them used a gradient running along x**, where a y mirror is
   invisible. A test that cannot fail in the axis the defect moves is not a test of it.

The guards now are `render-cpu/tests/shading_placement.rs` and `image_placement.rs`, which
pin values against ISO 32000-2 §8.7.4.5.3 and §8.9.5.2 **at three scales**, and
`headless_gpu.rs`'s vertical-gradient and image scenes. One scale cannot see this class of
defect; that is why every case runs at more than one. All were confirmed to fail when the
defects are reintroduced.

The same reasoning shaped the sixth session's annotation tests: §12.5.5's placement algorithm is
correct for any axis-aligned `/Matrix` even if you measure the *untransformed* `/BBox`, so
the fixtures use a rotation and a non-square `/Rect`. A square rectangle cannot tell the two
axes' scales apart.

### 3. An oracle is only as good as how it invokes the other renderers

The corpus oracle's first run reported 54 documents whose page *size* we disagreed about,
which looked like a `MediaBox` defect in our page tree. It was not. `pdftoppm` and `gs`
default to the **media box**; `mutool` and we use the **crop box**, which §7.7.3.3 defines
as "the region to which the contents of the page shall be clipped (cropped) when displayed
or printed" and is what a viewer shows. The harness had been asking two of the three
references for a different page.

Two things to carry from that. It cost 54 documents of coverage outright — they could not
be compared at all — and it would have been *worse* than useless on a page whose crop box
has the same size as its media box but a different origin, where it would have compared a
correct render against a displaced one and called us wrong. And the fix was decided by the
clause, not by the fact that `mutool` happened to agree with us: agreement was evidence we
had read §7.7.3.3 the same way, which is the only thing agreement is ever evidence of.

Every reference invocation is now explicit about the page box, including `mutool`'s, whose
default was already right — a default that silently changes is a comparison that silently
changes.

### 4. Test against real documents, not hand-written fragments

Cross-reference streams are compressed *and* PNG-predicted. The code said decoding them
was "the caller's responsibility" and then did not do it, so every modern PDF failed with
a misleading `/Root is not a dictionary`. Unit tests on fragments would never have caught
it; the corpus caught it on the first run.

`crates/pdf-syntax/tests/real_documents.rs` and
`crates/pdf-model/tests/render_real_pdf.rs` run over everything in `doc/`. Keep them
passing.

The converse is trap 8: a corpus cannot find what no document in it happens to contain.

### 5. Unsupported input must stay loud

Every layer reports what it could not handle rather than skipping it: `Unsupported` in the
interpreter, `FontError`, `ImageError`, `CpuRasterError::UnsupportedCommand`. This is not
politeness — it is what makes the comparison harness trustworthy and what caught trap 1.
Do not "helpfully" fall back to a default that renders something plausible.

The oracle found three places where this rule had been broken by omission rather than by
intent, and all three were drawing something visibly wrong in silence: text render modes 4
to 7 add the glyphs to the clipping path and we built no clip; an image's `/Mask` was
ignored, so a band the document masks out was painted; and a page's annotations were absent
entirely. **A rise in the incomplete count is not a regression when it is a new report** —
it is this rule being applied to somewhere it had not been.

The lesson generalises: a feature that is *partly* implemented is the easiest place to lose
this rule, because the operator is handled and the code path exists. `Tr` was parsed, the
mode was stored, three of its eight values were reported, and the four that change the clip
were not.

There is now one place where a report accompanies drawing rather than replacing it, and it
is deliberate. An `/AcroForm` setting `/NeedAppearances` is the document saying its stored
appearance streams are not the ones to draw (§12.7.4.3); we draw them anyway, because they
are all the file offers, and report that they may be stale. Two different true statements,
and suppressing either loses information. Do not generalise it further without the same
argument.

### 6. Colour: one conversion, and the specification often does not have an answer

Three separate `DeviceCMYK` → RGB conversions used to live in this tree and they disagreed.
`0.5 0 0 0.5 k` gave a red channel of 0.25; the same colour through `scn` gave 0.0; a CMYK
image gave a third answer. Nothing about a rendered page reveals that — each looks like a
plausible colour. `crates/pdf-model/tests/colour_paths.rs` now drives one value through all
three routes and demands they agree; it was verified to fail when the old code is restored.

Add no fourth path. `ColourSpace::to_rgb` is the only place a colour becomes RGB, and — since
the sixth session — `colour::xyz_d50_to_srgb` is the only place an XYZ becomes a pixel. That
second rule was added because the *same* defect had quietly recurred one level down: `lab()`
and `icc::xyz_to_rgb` each held their own copy of the nine-constant D50-to-sRGB matrix.
Nothing had gone wrong yet. It is one edit away from going wrong invisibly.

The other half is harder to hold onto: **ISO 32000-2 defines no `DeviceCMYK` conversion at
all**. §8.6.4.4 says "concentrations of process colourants" and stops; §8.6.5.7 NOTE 3 says
nothing in PDF describes the device. What the specification *does* say is where to ask —
`/DefaultCMYK` (§8.6.5.6, normative), an output intent's `/DestOutputProfile` (§14.11.5),
and an `ICCBased` profile — and all three are implemented and all three outrank the
fallback table. When you touch that table, do not reach for what another renderer produces:
read ADR 0009, and if you change it, change it as a documented choice.

The same shape recurs for a Cal space's `/BlackPoint`: §8.6.5.9 leaves black point
compensation to the processor whenever `/UseBlackPtComp` is `Default`, which is every real
document. It is read and deliberately not applied, and ADR 0012 has the argument — including
the part that decided it, which is that a stretch built from the entry is *undefined* on
input Table 63 permits.

### 7. `#[expect]`, never `#[allow]`

Every lint exception in the tree is `#[expect(..., reason = "...")]`. It errors when it
stops being necessary, which has already removed several stale ones. A bare `allow` hides
that forever.

### 8. A corpus finds what documents contain, not what the specification says

Added in the sixth session, because it is the mirror of trap 4 and the two are easy to
confuse.

The ICC evaluator agreed with two other readers on every real profile in the corpus. A test
that assembled a profile *by hand*, to check one clause of the ICC encoding, produced a
profile whose darkest colour equalled its white point — and black point compensation divided
by a span of floating-point noise and turned white into pure green. No real profile is shaped
that way.

The same thing happened again in the sixth session, from the other direction: `calrgb.pdf` page 14
states `BlackPoint [0.2 1.0 1.7]` against `WhitePoint [1 1 1]`, which Table 63 permits and
which no sane producer writes. It is what proved that the black point stretch has no
well-defined answer at all. **The corpus is not a specification, and a clause nothing in it
exercises is still a clause.** Synthetic fixtures and real corpora catch different things.

### 9. Two of the three voting references are one implementation, on JBIG2

The oracle's whole authority rests on a premise stated in ADR 0005: two implementations
sharing no code agreeing about a page is evidence. On JBIG2 pages that premise is false.
**`mupdf` and `ghostscript` both link `jbig2dec`**, Artifex's library, and on seven corpus
pages it decodes nothing and renders blank, or renders the drawing strewn with noise blocks,
or prints `segment marks bitmap coding context as retained (NYI)` and gives up. Two renderers
then "agree" and the gate reports us contradicted.

It took looking at a side-by-side to see it, and then a log to prove it: both renderers emit
the *same warning text*, because it is the same code emitting it.

Three things to carry from this.

- **The rule is fine; its premise was not checked.** `poppler` has its own JBIG2 decoder and
  agrees with us on six of the seven. The evidence that settles it, though, is not poppler's
  agreement — that would only be evidence that we read ISO/IEC 14492 the same way. It is
  `tests/jbig2.rs`: the corpus encodes **one image ninety-six ways**, through every coding
  mode the standard defines, and all ninety-six decode to byte-identical pixels here.
- **Ask what a reference is made of, not only what it produced.** `poppler`, `mupdf` and
  `ghostscript` look like three implementations and are three *renderers*; underneath, they
  share libraries per format. This is worth re-checking for any format where two of them
  agree suspiciously often.
- **A corpus can state an invariant about itself.** The ninety-six-way check needs no
  reference at all, so principle 5 is not even in tension — the expectation comes from the
  documents. It is also more sensitive than any tolerance: a decoder subtly wrong about
  refinement still draws a face, but not a byte-identical one.

The seven are `CONTRADICTED_SHARED_JBIG2_DECODER` in `oracle.rs`, and they stay listed rather
than excused, so the gate keeps watching them.

**The general form is now in the type.** `Reference::independence` says whether a renderer's
agreement is evidence, and `Reference::voting` is what the gate iterates, so a reference that
cannot supply evidence cannot silently be counted as supplying it. `hayro` is the first
entry marked `Shared`: it is a fourth renderer, rendered into the artefacts of every page
that is not agreement — a fourth panel in the side-by-side, which is the first thing to open
— and it never votes, because we share its font rasteriser, its deflate, its JPEG decoder and
both new image codecs. `mupdf` and `ghostscript` are deliberately *not* marked `Shared`: they
share only `jbig2dec`, and on every page without a JBIG2 image they are two implementations
of everything that matters, so recording the sharing where it applies keeps the evidence of a
thousand pages that marking them wholesale would throw away.

### 10. The sandbox worker is a separate binary, and Cargo will not rebuild it for you

`cargo test -p pdf-model` builds pdf-model's targets and pdf-sandbox's *library*. It does not
build pdf-sandbox's `pdf-sandbox-worker` binary, because Cargo never builds another package's
binaries. So the tests run against whatever worker was last compiled.

This is not hypothetical. While verifying that `tests/jbig2.rs` can fail, this session
deliberately inverted the black-and-white sense of every JBIG2 sample — and the test passed,
because the stale worker was still decoding correctly. The defect was real, the test was
right, and the two never met.

`cargo test --workspace` builds it. `cargo build -p pdf-sandbox --bins` builds it. Both gates
call `require_the_sandbox()` first, which fails loudly if the worker is *missing* — but a
missing worker and a stale one look nothing alike, and nothing detects the second.

`pdfref-hayro`, which draws the oracle's fourth panel, is found the same way and carries the
same caveat. It is less dangerous there: a stale one produces a stale *picture* next to three
fresh ones, and it never votes, so the worst case is a confusing artefact rather than a wrong
number.

## Environment

The agent runs as user `AI` via `sudo -u AI`, reaching `/home/cl/projects/pdf-viewer`
through the `coders` group. This causes recurring friction:

- **Launch with a login shell** so `umask 002` applies, or every file the agent creates is
  unwritable by `cl`:
  `sudo -u AI bash -lc 'cd /home/cl/projects/pdf-viewer && claude'`
- **`AI` has no X authority cookie.** Anything needing a window fails at
  `XOpenDisplayFailed`. The GPU backend is headless by construction precisely so it can
  still be tested; the viewer binary cannot be run by the agent past event-loop creation.
- **Build directory**: `AI` builds into `/home/AI/cargo-target/pdf-viewer` via
  `~/.cargo/config.toml`, so the two users never fight over `target/`. Do not "fix" this
  by sharing it again.
- **`pdfref` needs `--work-dir`** for the same reason; its default is `./target/pdfref`.
- **`cargo-fuzz` needs `+nightly`** explicitly, because `rust-toolchain.toml` pins stable
  1.97.1. That pin is deliberate.
- The Arlington model is a **submodule** pinned at `ba7d4d61`; `pdf-spec` will not build
  without `git submodule update --init`.

## What is not implemented

Every one of these is *reported* at runtime rather than silently skipped — that is what
makes the corpus numbers below trustworthy, and it is principle 3's requirement, not a
nicety. Sized by the corpus rather than by intuition: the count is how many of the 974
documents' first pages it affects.

| Missing | Corpus | Size | Notes |
|---|---|---|---|
| `CCITTFaxDecode` | 5 | Small | The last image codec absent, and now cheap: `hayro-ccitt` is already in the tree as `hayro-jbig2`'s MMR dependency. |
| Text: CID encodings, embedded `CMap`s | 121 | Medium | The breakdown from the gate's own output, counting *fonts* rather than documents: **39 Type 3** (see below), 27 with no `/ToUnicode` so a substitute cannot be addressed, 26 with a non-identity `/CIDToGIDMap`, 23 whose substitute draws none of the codes the document declares, 14 with an embedded `CMap` stream, 8 with a predefined `CMap` (`90ms-RKSJ-H`, `UniJIS-UTF16-H`, …). Only the last needs vendored data, which is a licensing decision rather than a coding one. |
| Synthesised annotation appearances | 63 | Medium–large | An annotation with **no** `/AP` must be drawn from `/IC`, `/C`, `/BS`, `/Border` and its subtype's own rules — a different routine per subtype. 26 `Widget`, 18 `Link`, and the rest markup annotations. Reported, never guessed. ADR 0013. |
| Transparency groups, soft masks | 45 | Large | 26 report as `Shading`, 19 as `Operator`. The largest *rendering* gap, and the last thing `doc/` reports. **Knockout groups (§11.4.5.6) are a silent subset**: `knockout_*.pdf` render as if the group were not a knockout, and nothing reports it. |
| Encryption | 20 | Medium | RC4/AES, `/Encrypt`. 11 documents cannot reach page one at all and 9 more draw a blank page. |
| Form field appearance construction | 7 | Medium | `/NeedAppearances` (§12.7.4.3). The field's value is known only at viewing time, so its appearance has to be built from `/V`, `/DA` and `/Q`. The stored appearance is drawn and the staleness reported. |
| Optional content (`/OC`) | 5 | Small–medium | 33 documents carry `/OCProperties`, 8 hide something by default, and **5 draw hidden content on page one**; the oracle contradicts 3 of them. `BDC` is parsed and ignored and `/OC` on an XObject is never read, so a hidden layer **is drawn anyway** and *nothing is reported*. `issue12007_reduced.pdf` draws a whole hidden screenshot over a page the references leave nearly blank. |
| `LZWDecode` | 3 | Small | One of the two standard filters still absent, the other being `CCITTFaxDecode` above. A test pins the report and will fail when it lands. |
| Text clipping modes (`Tr` 4–7) | 5 | Medium | Modes 4 to 7 add the glyphs to the clipping path, which takes effect at `ET` and lasts until the state is restored (§9.3.6 Table 106, §9.4.1). We build no clip, so a rectangle painted afterwards to show through the letters covers its whole area — `text_clip_cff_cid.pdf` drew a solid bar over the word. Now reported; a test pins that and will fail when the clip lands. Implementing it means accumulating the glyph outlines of a text object into one clip path. |
| Image `/Mask` | 5 | Medium | Stencil mask stream (§8.9.6.4) and colour-key range array (§8.9.6.5). Only `/SMask` is honoured, so the masked-out part is drawn — `colorkeymask.pdf` painted a band all three references hide. Now reported, with a test. The colour-key form must be applied to the *source* samples, before colour conversion, which is why it is not a two-line change. |
| `/UserUnit` | 2 | Small | §7.7.3.3: the size of a default user-space unit in multiples of 1/72 inch. `mutool` and `gs` scale the page by it, we and `poppler` do not — `bug1947248_*.pdf` come out at 612x792 where they produce 1836x2376. Neither applied nor reported; the oracle lists them under `GEOMETRY`. |
| Annotation `NoZoom`, `NoRotate` | — | Small | Table 167 bits 4 and 5 make an appearance's size or orientation depend on the *view*, which a resolution-independent display list cannot express. Rare. |
| Type1 fonts (`/FontFile`) | 0 | Medium | No corpus page one reaches it, so this is smaller than it looks. `read_fonts::ps::type1` exists — check before writing any. |
| Type3 fonts | 24 | Medium | **Now the largest single font gap, and newly visible.** §9.6.4 makes each glyph a content stream in `/CharProcs`, so it belongs in `pdf-model` rather than `pdf-font` — the operators are `d0`/`d1` plus a nested run of the interpreter under the font's `/FontMatrix`. Until the eighth session these fonts reached the *substitution* path and drew Latin glyphs for procedure names; they are now refused and reported. `issue918.pdf` and `bug1001080.pdf` are whole pages of it. |
| Sampled shadings on the GPU | 2 | Small | Type 1 only; the CPU backend draws them. |
| Rendering intents beyond `AbsoluteColorimetric` | — | Small | Read and recorded; `A2B0` is not yet selected for `Perceptual`. |
| Forms, actions, the rest of clause 12 | — | Large | Interactivity: field values, calculation order, JavaScript, navigation. Not needed to *draw* an annotation, which is why drawing landed without any of it. |
| Tagged PDF, metadata | — | Large | Clause 14 beyond output intents. |
| Sandboxing the *rest* of the renderer | — | Large | Spike D is done for the image codecs (ADR 0014). Interpreting and rasterising still happen in the main process. |
| JPEG 2000 at reduced resolution | 1 | Small | `issue19517.pdf` is a 12608x16806 scan whose full decode wants gigabytes for a page drawn at four megapixels. JPEG 2000's answer is to decode a lower resolution level, which needs the intended scale to reach the decoder. Refused with a clear report today. |
| Image downsampling quality | 1 | Small | `tiny-skia`'s bilinear filter samples four neighbours whatever the reduction, so an eightfold shrink leaves a stair-step on a curved edge. `firefox_logo.pdf` misses the oracle bound by 0.02. A filter averaging over the destination pixel's footprint is the fix, and wants a benchmark first. |

## How much of the specification is implemented

Measured, not estimated. Three answers, because the honest one depends on what you are
counting — and they are in ascending order of how much they should worry you. The first
counts what we *report*, the second what an implementation that shares no code with us
*sees*, and the third what the standard contains.

### By what real documents need

Over the 974-document pdf.js corpus, page one:

| | count | share |
|---|---|---|
| opens | 974 | 100% |
| reaches page one | 955 | 98% |
| **draws with nothing reported** | **665** | **68%** |
| draws, with something reported | 290 | 30% |

That 68% is the number to quote for *reporting*, and it **fell** from 72% this session, for
the third time in three sessions where the fall was the point: 24 documents began saying they
carry a Type 3 font, and 19 that their substitute draws none of the codes the document
declares. Nothing stopped drawing correctly. What stopped is drawing *incorrectly in
silence* — `issue918.pdf` was emitting 388 text operations of letter fragments and reporting
nothing.

It went down in the sixth session too, from 68% to 60%, for the same kind of reason, and up
in the seventh when JBIG2 and JPEG 2000 landed. **This number measures honesty, and honesty
can fall as capability rises** — so a rise is only good news when you can say which
capability caused it, and a fall is only bad news when you cannot say which silence ended.

### By what an independent renderer sees

This is the number to worry about. Over all 1794 pages compared, of the 1426 we claim to
draw completely:

| | count | share of the 1426 |
|---|---|---|
| agree with the reference consensus | 634 | 44% |
| **contradicted by it** | **108** | **8%** |
| the references cannot agree among themselves | 673 | 47% |
| not comparable (geometry, or fewer than two renderers) | 11 | 1% |

**One page in thirteen that we say we drew completely, two independent implementations say we
did not.** The 108 are named in `oracle.rs` and grouped by what the page carries: 25 use a
font nobody embeds so every renderer substitutes differently, **7 are pages where the two
references that agree are the same JBIG2 decoder and we are the ones who are right** (trap
9), 5 are a one-pixel page-rounding difference, 3 hide optional content we ignore, 1 is image
downsampling quality, 1 is a page of glyphs being judged with the tolerance for flat fills,
and **66 have nothing on them to explain it**. That last group is the most valuable list in
the repository. 21 of the 108 are pages beyond the first, which a page-one comparison would
never have seen.

129 → 108, and the fall has two parts that should not be conflated. **15 pages were fixed**,
all of them by §9.6.5.4, and all 15 came out of the unexplained group — the largest single
fall that list has had. The other 7 left because their document now *reports* a Type 3 font,
so the page is no longer gated at all. They left the comparison; they did not get better.

**Read the 47% ambiguous with care.** It is not "half the corpus is unsettled": 372 of those
843 pages are two long books, `freeculture.pdf` (320 pages) and `pdkids.pdf`, whose text uses
fonts nobody embedded, so each renderer substitutes a different one and the structural bound
separates them. Ambiguity concentrated in a handful of documents says more about those
documents than about the gate.

**So read the 665 as "reported nothing", not "drew it right".**

### By clause

ISO 32000-2 has 824 numbered subclauses under its eight technical clauses. Counting them is
a poor proxy — clause 12 is 166 subclauses of annotation subtypes a viewer adds one at a
time, while clause 8's 128 decide whether any page looks right at all — so this is a
judgement about state, not an arithmetic result.

| Clause | Subclauses | State |
|---|---|---|
| 7 Syntax | 138 | **Nearly complete.** Objects, all filters but `LZWDecode` and `CCITTFaxDecode` — JBIG2 and JPEG 2000 landed this session — classic and stream xrefs, object streams, incremental updates, recovery by scanning. **Encryption is absent** and is the largest hole here. |
| 8 Graphics | 128 | **Nearly complete.** Paths, clipping, all eleven colour space families, all seven shading types, both pattern types, form and image XObjects, inline images, ICC colour management. Optional content (`/OC`) is not honoured, so hidden layers draw on 5 corpus first pages; an image's `/Mask` is not applied. |
| 9 Text | 65 | **Partial.** Simple and composite fonts through embedded TrueType, CFF and OpenType programs; the standard 14 by substitution; `/ToUnicode`. §9.6.5.2's CFF encoding algorithm and §9.6.5.4's `TrueType` one are both implemented in full, the second as of the eighth session (ADR 0015). Missing: bare Type1 (`/FontFile`), Type3 fonts, embedded `CMap` streams, predefined `CMap`s, and the clipping text render modes (§9.3.6 modes 4–7). |
| 10 Rendering | 36 | **Partial, and much of it is not applicable.** Colour management and rendering intents are done. Halftones, transfer functions, flatness and smoothness describe a marking device rather than a screen. |
| 11 Transparency | 58 | **Minimal.** All sixteen blend modes are implemented and reach both backends. Transparency groups, soft masks, knockout and isolation are not — this is the largest *rendering* gap. |
| 12 Interactive features | 166 | **Appearances only.** Annotations are placed and drawn from `/AP` (§12.5.5), with the visibility flags of §12.5.3 honoured. Nothing is synthesised, and no forms, actions or navigation exist. |
| 13 Multimedia | 81 | **None**, and unlikely to be a priority. |
| 14 Document interchange | 152 | **Output intents only.** No tagged PDF, no metadata, no marked-content semantics — `BDC`/`EMC` are parsed and ignored. |

So: the parts of the standard that decide whether a page is drawn correctly are largely
done; the parts that make a document *interactive* are not started.

### Feature-by-feature, from the source

| | |
|---|---|
| Content-stream operators | **71 of 73** in Table 50 (`ID`/`EI` are consumed inside the `BI` handler rather than as arms). The two genuinely missing are `d0` and `d1`, which exist only inside Type 3 fonts. `BMC`/`BDC`/`EMC`/`MP`/`DP`/`BX`/`EX`/`i` are matched and deliberately ignored — correct for all but `BDC` with `/OC`. |
| Filters | **7 of 8** standard filters decode: `ASCIIHex`, `ASCII85`, `Flate`, `RunLength`, `Crypt` (pass-through), plus `DCTDecode`, **`JBIG2Decode` and `JPXDecode`** for images. `LZWDecode` is **absent** (3 corpus documents) and `CCITTFaxDecode` is reported, not decoded (5). |
| Colour spaces | **11 of 11** families, and the three CIE-based ones are converted rather than approximated. |
| Function types | **4 of 4** (sampled, exponential, stitching, `PostScript` calculator). |
| Shading types | **7 of 7**, on both backends. |
| Pattern types | **2 of 2** (tiling and shading). |
| Blend modes | **16 of 16**. |
| Font programs | TrueType, CFF, CFF-in-OpenType, CID-keyed CFF. Bare Type1 and Type3 are reported — Type 3 only since the eighth session, and this row said otherwise for two sessions before anything checked. |
| Annotation appearances | Placed and drawn; not synthesised where absent. |

## Done in the eighth session, and what it taught

**ISO 32000-2 §9.6.5.4, in full** (ADR 0015). The contradicted count fell from 129 to 108,
and 15 of the 81 unexplained pages left that list at once. Four things it teaches, and only
the first is about `cmap` tables.

- **A one-line implementation of a two-page subclause does not look unimplemented.** There
  was code at that spot — `charmap.map(code)`, with two fallbacks — and it worked on most
  documents, so nothing in the tree said "§9.6.5.4 is missing". The gap was not a `todo!()`
  or a report; it was an *approximation that had never been compared against the clause it
  approximated*. The habit below about gaps inside implemented features is exactly this, and
  this is its sharpest instance yet: reading the code asking "what have we not built" answers
  "nothing", and reading the clause asking "does the code do this" answers in five minutes.
- **A fallback that fills the page is worse than one that leaves it blank.** The old code
  ended in "if nothing else matched, the code is the glyph index", per code. That is why
  `issue5501.pdf` drew `v 0' ' W` for `What's an interval?` — confident, plausible, wrong,
  and silent. The same fallback still exists, restricted to a font with no readable `cmap` at
  all, and the oracle proves the restriction is load-bearing: put it back per-code and
  `issue17333.pdf` is contradicted immediately.
- **Fixing the mask shows what the mask was hiding.** Neither Type 3 fonts nor the
  substitute-usability rule has anything to do with `cmap` tables. Both became visible
  because the same afternoon was spent looking at what fonts actually did, and both were
  bigger than the thing that led to them: 24 documents and 19 documents against one.
  Budget for that — the fix that a defect leads to is often not the fix it names.
- **The number that went the wrong way is the honest one.** The corpus's incomplete count
  rose by 40 and the "draws with nothing reported" share fell from 72% to 68%. Every
  document in that rise was already drawing wrongly; what changed is that it says so. Three
  sessions running, this number has moved in the direction that looks worse and meant
  better, and it is worth stating plainly that **a project optimising the reported percentage
  would have had to leave `issue918.pdf` drawing letter fragments.**

**A fixture proves a rule; a corpus proves a page.** The corpus said `issue20504.pdf` was
wrong and `issue5501.pdf` was wrong; it could not say *which* of §9.6.5.4's rules was
missing, because every real font carries several `cmap` subtables and any of them might have
been the one that worked. The eleven fixtures in `truetype_encoding_tests` carry exactly one
subtable each, so exactly one rule can apply, and a rule that stops working now fails one
test by name. Trap 8 in reverse, and the two are complementary rather than redundant.

## Done in the seventh session, and what it taught

**JBIG2 and JPEG 2000, in a sandbox, through crates this project does not own** (ADR 0014).
118 documents left the incomplete list. Four things it teaches, and only the first is about
image codecs.

- **A deferred decision has an expiry date nobody sets.** "JBIG2 and JPEG 2000 have no
  memory-safe implementation" was true when `PLAN.md` was written and had quietly stopped
  being true months before anyone re-checked. The plan was re-read many times in between; it
  was never re-*verified*, because a premise that reads like a fact does not look like a
  question. It is worth asking, of any item deferred on an external condition, when that
  condition was last checked.
- **A dependency is a decision, and this project's own precedent decided it.** `zune-jpeg`
  owns `DCTDecode`, `skrifa` owns font parsing, `flate2` owns Flate, `tiny-skia` owns
  rasterisation. Writing 19 400 lines of MQ coding, EBCOT and symbol dictionaries here would
  have been consistent with none of that, and validated by 104 documents against their 20 000.
  The cost is real and written down: two decoders we cannot fix ourselves.
- **A sandbox's justification can change without the sandbox becoming wrong.** The reason
  `PLAN.md` gave for it — containing C — evaporated. Three reasons survived, and one of them
  paid for itself the same afternoon: `issue19517.pdf` carries a 212-megapixel JPEG 2000
  scan, and `RLIMIT_AS` turned "the viewer allocates 847 MB and may die" into "one image
  reports that it is too large". A feature justified by one argument is worth re-justifying
  rather than deleting when that argument goes.
- **The corpus can be its own oracle.** 96 documents encode one image ninety-six ways, and
  demanding they agree is a stronger and cheaper check than any reference comparison — and
  the only one that could settle a page where two of the three references are secretly the
  same decoder. Look for that shape: a corpus that varies one thing while holding another
  fixed is stating a testable invariant.

**Two defects found on the way, neither in a codec.** A filter chain ending in an image codec
handed the codec the bytes of the *previous* stage, so `[/FlateDecode /DCTDecode]` — legal,
and present in the corpus — never decoded and reported a broken JPEG. `Document::image_stream`
now runs everything up to the codec and stops. And `mupdf` and `ghostscript` turn out to share
`jbig2dec`; see trap 9, which is the more important of the two.

## Done in the sixth session, and what it taught

Two changes, and between them the contradicted count fell from 174 to 120.

**`CalGray` and `CalRGB` through XYZ** (ADR 0012). Eight pages left the ratchet, and the
XYZ-to-sRGB matrix went from two copies to one. What it teaches is about the *shape* of the
old shortcut rather than the fix: treating a Cal space as its device equivalent is nearly
correct when the document's `/Gamma` is about 2.2, because decoding by it and re-encoding for
sRGB almost cancel — and §8.6.5.2's own EXAMPLE 2 is exactly that space, so most real
documents are the case the shortcut gets right. **A shortcut that is correct on the common
case is the hardest kind to notice**, because the pages that would reveal it are the rare
ones and they report nothing either.

**Annotation appearance streams** (ADR 0013). 45 of the 47 contradicted pages in that group
fixed, and the two that stayed were never about annotations — the previous handover predicted
that reading. Three things it teaches:

- **Drawing is separable from interactivity.** Clause 12 is 166 subclauses and reads like one
  large thing. §12.5.5 alone — an appearance is a form XObject, place its `/BBox` on the
  annotation's `/Rect` — bought the largest single reduction in the contradicted list, and
  needed no field value, no action and no calculation order.
- **A feature landing can *raise* the incomplete count**, and this is the first time it has.
  Before this, an annotation with no appearance was indistinguishable from no annotation at
  all: neither was drawn and neither was reported. Implementing the first made the second
  visible. The rule that a rise which is a new report is honesty rather than regression still
  holds; it just now also applies to features arriving, not only to gaps being confessed.
- **The test that fails is the one you learn from.** An early version conflated "no `/AP` at
  all" with "`/AS` names a state the dictionary omits". The first is a gap in this crate; the
  second is how *every* unchecked check box is written, and §12.5.5 names displaying nothing
  as the correct answer. A fixture written from the clause caught it; the corpus would have
  buried it in 60-odd other reports.

**Looking at four contradicted pages explained all four.** `bug1922766.pdf`,
`bug1934157.pdf`, `bug1669097.pdf` and `issue19505.pdf` all have a fractional page box, and
our raster is one pixel smaller than poppler's and mupdf's and exactly the same size as
ghostscript's. On a 72-row page a one-row shift moves everything on it. Nothing in
ISO 32000-2 says how a fractional page becomes an integer number of pixels. Five minutes of
looking at side-by-sides has now explained every page it has been spent on — six last session,
five this one — which is the strongest recommendation this file can make about how to spend
the next hour.

## What to do next

The one-line version: **108 pages we claim to draw are contradicted, 66 of them for no
reason visible on the page — that list is still the work. And Type 3 fonts are now the
largest named gap, at 24 documents, because this session stopped pretending we drew them.**

**Type 3 fonts are the item this session most changed the case for.** They were 23 documents
of *silence*; they are now 24 documents that say what they need, and what they need is
smaller than the list of missing features around it: §9.6.4 makes a Type 3 glyph a content
stream in `/CharProcs`, run under the font's `/FontMatrix`, with `d0`/`d1` as the only two
new operators — and the interpreter that has to run it already runs form XObjects, annotation
appearances and tiling patterns the same way. It belongs in `pdf-model`, not `pdf-font`;
`pdf-font` now refuses it with `FontError::Type3` and that refusal is where the new code
attaches. `ContentStreamCycleType3insideType3.pdf` is in the corpus and names the trap: a
Type 3 glyph may show text in another Type 3 font, so the recursion needs the same depth
bound form XObjects already have.

Four cheap items carried over from the seventh session, listed before the big lists because
they are small:

- **`CCITTFaxDecode`** is 5 corpus first pages and is now nearly free — `hayro-ccitt` is
  already in the dependency tree, pulled in as `hayro-jbig2`'s MMR decoder, and the filter
  would be an arm in `image.rs` plus the same `Bilevel` round trip JBIG2 already uses.
- **Sandbox the interpreter and rasteriser too.** Spike D exists and is exercised; the rest
  of the renderer still runs in the main process, which is the half of principle 3 that is
  not yet built. The protocol would have to carry a display list rather than an image, which
  is a real design question and the reason it is not a footnote to this session.
- **Profile the median page.** We are 1.62× slower than `hayro` on the typical corpus page
  and nobody has looked at why — the two fixes this session were both to outliers and moved
  the median not at all. The typical page is small and text-heavy, so the candidates are
  parsing, font loading and per-page setup rather than rasterisation, but that is a guess and
  the handover's own habit says profile before believing an explanation.
  `cargo run --release -p hayro-compare --bin hayro-speed` is the measurement; `callgrind`
  over one median-sized document is the diagnosis.
- **Give the JPEG 2000 decoder a target resolution.** One corpus document is refused for
  being 212 megapixels, and the format's own answer is to decode a lower resolution level.
  It needs the scale a page is about to be drawn at to reach `image.rs`, which the display
  list deliberately does not carry — so this is a question about where decoding belongs, not
  a parameter to thread.

### 1. Work the unexplained list

`CONTRADICTED_UNEXPLAINED` in `oracle.rs`: 66 pages carrying no undrawn annotation, no hidden
optional content and no substituted font, so the difference is in something we believe we
implement. Two causes are identified and live — and read trap 9 before starting, because
one entry in that list may be the same shape: two references that are one implementation.

- **`knockout_*.pdf` are knockout transparency groups** (§11.4.5.6), where an object
  composites against the group's initial backdrop rather than against what is already there.
  `mutool` and `gs` show no blend where two rectangles overlap; we and `poppler` show it.
  Unimplemented and, unlike soft masks, unreported.
- **`mesh_shading_empty.pdf` draws the same mesh displaced horizontally** — a placement
  question, and the class of defect trap 2 is about.

The third entry that used to be here, `issue20504.pdf`, was worth **15 of the 81**. It looked
like one page's `/Differences` quirk and was a whole subclause: see ADR 0015. That is the
argument for spending the hour — an entry on this list is not necessarily one page's problem,
and the only way to find out which kind it is, is to open the artefact.

The other 64 are unexamined. Each is a page where two implementations sharing no code agree
and we differ by more than twice their own disagreement, with the artefacts already written:
`<target>/tmp/oracle/<stem>/p<n>/` holds our render, each reference's, a side-by-side and a
difference heatmap. **Look at the side-by-side first.**

Two cautions. A page may be contradicted for a reason other than the one its group names —
`calgray.pdf` sat under substituted fonts and differed in its colour, which is how the whole
of ADR 0012 started. And principle 5 is not suspended by a list: each entry is a question to
take to the specification, and "make it match mupdf" is exactly the failure this project
forbids.

### 2. Honour optional content

The oracle contradicts 3 of the 5 pages that draw a hidden layer:

| | count |
|---|---|
| documents carrying `/OCProperties` | 33 |
| whose default configuration hides something | 8 |
| **whose page one draws content that should be hidden** | **5** |

One scope correction for whoever takes it: an `/OC` entry usually points at an **OCMD**
rather than at an optional content group directly, so this is not merely "read `/OFF` and
skip `BDC /OC`". It needs `/OCGs` (one group or an array), the `/P` policy
(`AnyOn`/`AllOn`/`AnyOff`/`AllOff`), and both entry points — `BDC /OC` marked-content spans
*and* `/OC` on form and image XObjects, which is how `issue12007_reduced.pdf` hides its
layers. `/VE` visibility expressions also occur in the corpus (`visibility_expressions.pdf`
is named after them) and can be reported rather than implemented at first.

It is now also cheap in a way it was not: the annotation pass established the pattern of a
second walk over the page's own dictionary, and `/OC` on an annotation is one more place the
same visibility question gets asked.

### 3. The three silent gaps still open

Each is loud, each has a test that will fail when it lands, and each is a page drawn visibly
wrong today:

- **Text clipping modes** (`Tr` 4–7, §9.3.6 Table 106 and §9.4.1). The glyphs of a text
  object join the clipping path at `ET` and stay until the state is restored. Implementing it
  means accumulating each shown glyph's outline, transformed into page space, and pushing one
  `Clip` at `ET` — the display list's `Clip` already carries a path, a transform and a
  parent, so nothing new is needed in `pdf-render`. 5 corpus first pages report it.
- **Image `/Mask`**, stencil (§8.9.6.4) and colour-key (§8.9.6.5). The colour-key form must
  be applied to the *source* samples before colour conversion, which is why it is not a
  two-line change in `image.rs`. 5 corpus first pages report it.
- **`/UserUnit`** (§7.7.3.3), which scales the page. 2 corpus documents, and the only reason
  it matters more than that count suggests is that getting a page's *size* wrong invalidates
  every comparison on it.

### 4. Synthesised annotation appearances, if the corpus count is the argument

63 documents carry an annotation with no `/AP`, which is now the third-largest reported gap.
It is genuinely a different drawing routine per subtype and should not be started as one
task. If it is taken, take it one subtype at a time in corpus order: `Widget` (26), `Link`
(18, and its whole appearance is a border — §12.5.6.5 with §12.5.4), then the markup
annotations. Each one that lands should be measured on the oracle rather than assumed to
help, because a synthesised appearance is a *guess at what the producer meant* and the
references guess differently.

### 5. Then, by what the corpus says real documents need

**Soft masks and transparency groups** (45 documents, and the last thing `doc/` reports),
**encryption** (20 documents — 11 cannot reach page one, 9 more draw a blank page and now
say so), and **CID encodings** (76 documents; note that only 3 of those need the predefined
`CMap` data with its licensing question — the other 73 need code). **Type1 fonts** are
smaller than they look: no corpus page one reaches one.

All three announce themselves, which is why they sit below the items above: a gap that
reports is a gap you can measure and schedule, and a gap that does not is a gap that ships.

### Where the time went, and how we know

**There is now something fair to measure against.** Every other renderer here is C, so a
timing difference against `poppler` confounds the language, the allocator and thirty years of
tuning. `hayro` is Rust, forbids unsafe as we do, rasterises on the CPU single-threaded as we
do — so what is left when you subtract all that is the code.
`cargo run --release -p hayro-compare --bin hayro-speed -- <files>` renders page one of each
file with both, alternating, best of N passes, and reports time to first page.

The first run said we were **1.61× slower on the median page**, faster in aggregate only
because a few documents are catastrophic for `hayro`, with our own outliers reaching 225×.
Two causes, both found with `callgrind`, both fixed, and both worth carrying as patterns.

Where it stands now, over the 698 corpus pages we claim to draw completely:

| | before | after |
|---|---|---|
| total, ours | 22.8 s | **7.1 s** |
| total, hayro | 38.6 s | 41.8 s |
| **median page** | 1.61× slower | **1.62× slower** |
| worst page | 225× | 34× |

**The totals and the median answer different questions, and only quoting both is honest.**
In aggregate we are 5.9× faster, because their distribution has a long tail and ours no
longer does. On the median page we are still 1.62× slower and that number did not move at
all — the fixes were to outliers. **The typical corpus page is small and
text-heavy and has never been profiled**, which is the next measurement rather than the next
optimisation. Guessing at it is exactly what the gradient-versus-clip-mask mistake in
"Habits" was.

**Our own per-pixel loop cost more than the codec it was unpacking.** On
`22060_A1_01_Plans.pdf`, `pdf_model::image::decode_jpeg` was 6.89 G instructions — 38% of the
whole run, and nearly twice what `zune-jpeg` spent decoding the JPEG. The cost was structural
and per pixel: a `match` on the component count *inside* the loop, three bounds-checked
`get`s with `unwrap_or`, saturating index arithmetic, and an `extend_from_slice` re-checking
capacity every time. Pairing two `chunks_exact` iterators removed all four and took it to
1.25 G — 5.5× — and the document from 18.0 G instructions to 12.4 G. The lesson is not
"`chunks_exact` is fast": it is that **the safety habits this project enforces everywhere are
expensive in a loop that runs per pixel**, and that is exactly where the profile should be
consulted rather than the habit.

**A mesh shading was subdivided by colour alone.** `Triangle::is_flat` asks whether the
corner colours are close enough to fill flat, and a triangle whose corners differ kept
splitting to `4^6` pieces however small it was on screen — each piece a separate `fill_path`
through `tiny-skia`, each compiling its own raster pipeline. `personwithdog.pdf` spent
**17.3 seconds rasterising a display list of eighteen commands**. The fix is
`Triangle::is_subpixel`, and it is a correctness statement rather than a trade: a triangle
covering less than a pixel cannot display a gradient, because the output raster has one
sample there and every sub-triangle's average lands in it. 17.3 s → 1.06 s.

Three things about that one:

- **It lives in `pdf-render`, not in either backend.** Both backends subdivide, in code
  written to mirror each other down to the constants; a stopping condition added to one would
  have broken the same-scene oracle. The criterion is a property of a triangle and a raster,
  so it belongs where both can call it.
- **The output got *closer* to the references, not further.** Every mesh page's structural
  similarity improved — `tensor-allflags-withfunction.pdf` from 0.9845 to 0.9942 — because
  the sub-pixel pieces were overlapping antialiased fills whose coverage accumulated. A
  change made for speed that improves fidelity is a sign the old code was doing work that was
  not merely useless but harmful.
- **The oracle proved it.** Same 1794 pages, same 751/168/827 verdicts, no page changed
  category. Our processor time across that gate fell from 203 s to 55 s, and the *corpus*
  gate — which renders the same 974 documents with no external renderer to hide behind —
  went from **19 s to 1.6 s**. A twelve-fold change in a gate that had been considered fast
  is the strongest evidence that nobody had profiled it.

**§9.6.5.4 cost nothing measurable, and that was checked rather than assumed.** Resolving all
256 codes when a font loads replaced a `cmap` lookup per character, and added a linear scan of
the `post` table for every name no subtable could map — both plausible regressions. A/B over
the same 391 pages, one sitting, `hayro-speed`: 2.79 s before, 2.78 s after, median 1.64× to
1.65×. Neither direction is visible above the noise, which agrees with the earlier finding
that `FontRef::new` is a zero-copy view rather than a parse. Do not read the whole-corpus
totals across that change — the *set* of pages we draw completely moved by 40 documents, so
the populations differ.

**Still open, and now the largest items.** The profile below predates both fixes and its
shading half is still live:

| on `bug1721218_reduced.pdf`, 16.1 G instructions | share |
|---|---|
| `tiny_skia::pipeline::lowp::gradient` | 29.7% |
| `pdf_model::function::Function::parse` | 23.2% |
| `pdf_model::function::Function::eval` | 13.8% |
| `ColourSpace::to_rgb_at` | 2.6% |

**The gradient stage** is the largest single item because a `Ramp` carries 256 samples, so a
shading becomes a 256-stop gradient and `tiny-skia` scans its stops per pixel batch; handing
the *rasteriser* fewer stops would fix it, while coarsening the `Ramp` in the display list
would lose fidelity and is not the same thing. **Roughly 40% of that run is building the
shadings** rather than drawing them: a PDF function is parsed and then sampled 256 times for
every shading, and that page has 3576 of them. Whether that is 3576 *distinct* functions or
one function re-parsed 3576 times has still not been checked, and it decides whether the fix
is memoisation by object reference or something harder — check before designing.

One caution now that Cal spaces convert properly: `to_rgb_at` was 2.6% when `CalGray` was a
pass-through. It now runs a Bradford adaptation and a matrix per colour, and per *sample* for
a Cal-space image.

### Reproducing the numbers in this section

The oracle survey is `oracle.rs`, it runs on demand, and its per-page lines are the evidence
for every comparison count above. The corpus counts come from `corpus.rs`.

The *classification* counts are throwaway, and deliberately so — scratch-quality diagnostics
do not belong in a repository held to `clippy::pedantic`. Each is worth rebuilding properly
as part of the task it belongs to:

- **The optional-content count** reads `/OCProperties /D /OFF` from the catalog, then looks
  for those groups in page one's `/Properties` resources and on its XObjects' `/OC`. Follow
  OCMDs: the first version of this check looked only for direct group references, reported
  "0 reachable from page one" for `issue12007_reduced.pdf`, and was wrong — the page draws
  a hidden layer through an OCMD.
- **Whether a page's fonts are embedded** walks each `/Font` resource and its
  `/DescendantFonts` looking for `/FontFile`, `/FontFile2` or `/FontFile3` in the descriptor.
- **The annotation subtype breakdown** now comes free: the corpus gate prints the reported
  detail per document, so `grep -o '"[A-Za-z]*: no appearance stream"' | sort | uniq -c`
  over its output is the whole of it.
- **The page census** — 3143 pages across the 988 documents, 869 of them single-page files,
  1382 pages in the 14 specification PDFs — is `Pages::new(&document).len()` over the same
  file list, which is what settled the scope decision in ADR 0011.

### What the corpus gate reports today

Ratcheted in `crates/pdf-model/tests/corpus.rs`; the numbers only ever go down, except where
a rise is a new report and is written down as one.

| | count | |
|---|---|---|
| unopenable | 0 | and it should stay there |
| no page one | 19 | 11 encrypted, 8 with unrecoverable page trees |
| draws incompletely | 290 | 121 text, 65 annotation, 42 image, 28 operator, 26 shading, 7 undecodable content stream, 1 bound reached |
| slower than 30 s | 0 | `KNOWN_SLOW` is empty, and the next document to cross the budget fails the gate |

The image row was 161 before JBIG2 and JPEG 2000 landed. What is left of it is a different
set of problems: 12 inline images, 10 `Indexed` and 3 `DeviceN` colour spaces the image
unpacker does not convert, 6 `/Mask` entries, 5 `CCITTFaxDecode`, 4 malformed streams, and
two files the new decoders refuse — one JBIG2 using a segment type ISO/IEC 14492 does not
define, and one 212-megapixel JPEG 2000 scan.

The text row went from 76 to 121 in the eighth session and every one of the 45 is a new
report: 39 Type 3 fonts and 23 substitutes that draw none of the codes their document
declares, against a handful that left when §9.6.5.4 landed. Counted as *fonts*, not
documents — a page may name several.

### What the oracle gate reports today

Ratcheted in `crates/pdf-model/tests/oracle.rs`, by name and in both directions.

| of the 1426 pages we call complete | count | |
|---|---|---|
| agree with the reference consensus | 634 | |
| **contradicted** | **108** | 5 page rounding, 7 a shared JBIG2 decoder (trap 9), 1 image downsampling, 3 optional content, 1 glyphs judged as vector, 25 substituted fonts, **66 unexplained** |
| ambiguous | 673 | the references disagree with each other; 372 of them are two long books set in fonts nobody embedded |
| our page geometry differs | 3 | 2 are `/UserUnit`, 1 unexamined |
| not comparable | 6 | fewer than two references produced an image, or they disagree on the page size |

The 368 incomplete pages are compared and printed too, but cannot fail the gate: a page we
already say we cannot draw is expected to differ, and listing hundreds of them would drown
the signal. That number rose by 43 this session and the gated total fell by the same, which
is the cost of the honesty above: a page that starts reporting stops being watched by *this*
gate. It is the reason a report should never be reached for as a way of making a
contradiction go away, and the reason `CONTRADICTED_SUBSTITUTED_FONT` now records which of
its departures were fixes and which were exits.

**Where its time goes, measured and printed by the gate itself:** roughly 1000–1300 s of
processor time in the three external renderers against 45–55 s in ours, for a steady 75 s of
wall clock on 24 cores. The processor figures move by a fifth between runs and the wall clock
does not, which is the file's own warning about wall-clock measurement pointing the other
way: under a fixed parallel load the *total* is stable and the split is not. The ratio was
8:1 before this session's rasteriser work and is now above 20:1 — **this gate is essentially
a measurement of `pdftoppm`, `mutool` and `gs`**, which is what to remember if it ever needs
to be faster — and why a content-addressed cache of reference renders is the
obvious lever, with the equally obvious risk that a cache key omitting one variable (the
crop-box flag, the renderer version) would compare against stale renders in silence.

**The time budget reports; it cannot enforce.** A Rust thread cannot be cancelled, so a
document that never returns hangs the suite rather than failing it. A real budget has to
live inside the interpreter and the rasteriser. `PDFVIEWER_CORPUS_TRACE=1` names each
document on stderr as it starts and finishes, which is how a hang gets identified from a
killed run.

**`doc/pdf.js` is a submodule** (Apache-2.0, pinned at v6.1.200), holding those 974 PDFs and
459 more behind link files. It is optional to clone — every test that uses it reports being
skipped rather than failing — but the ratchets only mean anything where it is present, so CI
must have it.

## Habits these sessions earned

**A gap inside a feature you have implemented does not announce itself.** Every missing
subsystem in this tree reports — `LZWDecode`, JBIG2, encryption — because somebody wrote the
report while deciding not to write the feature. The gaps that ship are the ones *inside*
something implemented: `Tr` was parsed and three of its eight modes reported, `/SMask` was
honoured while `/Mask` beside it was not, `CalGray` was resolved and then converted as
`DeviceGray`, and §9.6.5.4's whole algorithm was one line that worked on Latin text. Reading
the specification asking "what have we not built" cannot find those, because the answer is
"nothing". Comparing output against another implementation can, and has, six times now.

**A subclause is a checklist; check the code against it, not the code against itself.**
§9.6.5.4 is two pages and names five distinct routes from a code to a glyph. The code that
stood in for it implemented roughly one and a half of them, and no reading of *that code*
would have said so, because it was self-consistent, commented, and right about the documents
anyone had opened. The cheap move that was never made is: open the clause, list its rules,
and ask of each one where it is. It took five minutes and was worth 15 contradicted pages
and two unrelated silences. Do it for §11.4 and §12.5.6 before implementing either.

**When a report replaces wrong output, the reported count is the wrong scoreboard.** Two of
this session's three changes made the corpus's incomplete count *rise*, by 43 documents, and
the "draws with nothing reported" share fall by four points. Every one of those documents was
already drawing wrongly. A project that watched the percentage would have had to leave
`issue918.pdf` emitting letter fragments to keep it. The rule the ratchet comments now state
in both files: a rise is fine when you can name the silence it ended, and a *fall* in the
contradicted count is only a fix when the page still enters the comparison — seven of this
session's 22 departures left it by becoming incomplete, and saying so is part of reporting
the result honestly.

**A shortcut that is right on the common case is worse than one that is wrong on all of
them.** The Cal-space pass-through was nearly correct for `/Gamma 2.2`, which is what most
documents write, and badly wrong otherwise. Nothing distinguishes the two populations at
runtime, so nothing reported. Prefer the derivation even where the approximation looks close:
"close on the files I tried" is not a property you can test for.

**Ask the reference the same question you asked yourself.** Two of the three renderers were
being asked for the media box while we rendered the crop box, which put 54 documents beyond
comparison and would have produced false failures on any page whose two boxes differ only in
origin. A comparison harness has its own defects, they look exactly like ours, and the way to
tell them apart is to check the invocation against the clause before believing the verdict.

**Look in `read-fonts` before writing font-format code.** An earlier handover specified
~80 lines of CFF charset parsing plus two 256-entry tables, and all of it already existed
in `read_fonts::ps`, which `skrifa` re-exports as `skrifa::raw`. See ADR 0006. The same
module also holds `type1`, `charmap` and `agl` — `agl` is now enabled and carries the
Adobe Glyph List, so nothing needs transcribing.

**Profile before believing an explanation, even one whose arithmetic matches.** An earlier
handover attributed a 48-second page to page-sized clip masks and supported it with
`3576 clips × 485 kB = 1.7 GB`, which is exactly what the process held. The arithmetic was
right about the memory and silent about the time: `callgrind` put the masks at under 4% and
the gradient stage at 78.9%. Fixing what the arithmetic named would have kept nearly all 48
seconds. A number that reproduces one symptom is not a diagnosis.

**Wall-clock benchmarks lie under load; count instructions instead.** A `Command::Fill`
change measured as a 24% *regression* on `cargo bench` and as an 8.5% *improvement* twenty
minutes later, purely from background build load. `valgrind --tool=callgrind` on
`crates/pdf-model/examples/callgrind_interpret.rs` settled it deterministically: 2.065 G
instructions before, 1.951 G after. Always A/B in one sitting, and prefer the instruction
count. `iai-callgrind` wraps this into a bench harness and is the right basis for the CI
perf gates `CLAUDE.md` asks for — not yet wired up.

**Measure against something comparable, or the number means nothing.** This project compared
itself to `poppler`, `mupdf` and `ghostscript` for six sessions and never once asked whether
it was *fast*, because against C the question has no clean answer. `hayro` made it answerable
and the answer was 1.61× slower on the median page. Both causes were in our own code and
neither was where intuition pointed: not the rasteriser, but a per-pixel unpacking loop and a
subdivision criterion missing a term. A benchmark you cannot attribute is a benchmark you
will not act on.

**A premise that reads like a fact does not look like a question.** "JBIG2 and JPEG 2000 have
no memory-safe implementation" sat in `PLAN.md` as the reason two filters were unimplemented,
and it was true when written and false for months before anyone checked. It was re-*read*
constantly. Nothing in a plan marks which of its statements are about the world rather than
about the project, and those are the ones that rot. Any item deferred on an external
condition should carry the date the condition was last verified.

**Two rasterisers disagreeing is information, not noise — and two agreeing is not proof.**
The CPU-versus-GPU agreement test is what found that Vello needed the same mesh seam repair
`tiny-skia` did, after a comment here had confidently claimed otherwise. Where the backends
differ, one of them is wrong. The other half of the rule was learned the hard way: both
backends positioned paints in the wrong space, in the same way, for the same reason — the two
libraries share the convention that was misread — so they agreed with each other perfectly
while both were wrong. **Agreement is evidence only where the implementations can fail
independently.**

**Two copies of a constant is one defect waiting.** Three `DeviceCMYK` conversions disagreed
and nothing looked wrong. When that was fixed, the same shape survived one level down: the
nine-constant D50-to-sRGB matrix sat in `colour.rs` and in `icc.rs`. Nothing had gone wrong
yet, which is exactly the condition under which it is cheap to fix. It is now one function
with a test that recomputes all nine numbers from the two published matrices they were folded
from — so a folded constant, which is otherwise unreadable and unfalsifiable, has a
derivation attached.

**A test written to isolate one rule finds what a corpus cannot.** The ICC evaluator agreed
with two other readers on every real profile in the corpus. Writing a test that assembles a
profile *by hand* produced one whose darkest colour equals its white point, and black point
compensation divided by a span of floating-point noise and turned white into pure green. No
real profile is shaped that way. See trap 8, which is now the general form of this.

**Measure before optimising, and delete what does not measure.** `glyph_for` builds a
`FontRef` per character, which looks like an obvious cache. Caching it changed a dense page
by less than run-to-run noise (3587 lookups, 211 distinct codes), so the cache was removed
and the reason written where the next person will look. The same session's *real* win was
found the same way: hoisting a string allocation out of `substitute::find` took a difficult
lookup from 1.37 ms to 18 µs. `cargo bench -p pdf-model` is the baseline.

## Things worth knowing

- **The sandbox is a flag, and the default is the safe one.** `--no-sandbox` decodes JBIG2
  and JPEG 2000 in the viewer's process. It can be a flag only because both decoders are
  memory-safe either way: what it trades is panic containment and a memory ceiling, which are
  real and bounded, not memory safety, which would not be offerable. There is deliberately no
  path that falls back to in-process decoding when the worker fails to start — a fallback
  that silently removes the confinement is worse than a reported failure.
- **A font is reported as a whole, and that is not fine-grained enough.** `FontError` is the
  only channel a font has, so a font either loads or does not. A font that maps *some* of the
  codes its document declares and not others therefore draws the ones it can and says nothing
  about the rest. The eighth session narrowed this — a substitute reaching *none* of the
  declared codes is now refused, which is what caught `tracemonkey.pdf`'s missing © — but the
  general case needs a report where a glyph is *shown*, in `show_text`, rather than where a
  font is loaded. That needs `LoadedFont` to distinguish "this code has no glyph" from "this
  code's glyph is blank", which a space legitimately is. Not hard; not yet done; and worth
  measuring on the corpus before assuming the volume is manageable.
- **The oracle's artefacts are the fastest diagnostic in the tree.** Every page that is not
  agreement leaves `<target>/tmp/oracle/<stem>/p<n>/` holding our render, each reference's, a
  side-by-side strip and a difference heatmap per reference. Open the side-by-side first: it
  is one image, four panels, ours leftmost, and it has explained every page it was pointed at
  so far — a solid bar where a word should be, a band that should have been masked out, grey
  swatches at the wrong lightness, a page one pixel short. Pages that agree have theirs
  deleted, so what is on disk is exactly the set worth looking at.
- **A page's tolerance class depends on what *we* drew.** The oracle picks a text tolerance
  or a vector one from our own render's content, so a change that adds text to a page also
  loosens its bound — and can move it from "ambiguous" to "judged". Four pages crossed that
  line in the sixth session when annotations started drawing, and all four had *improved*. When a
  page appears in the newly-contradicted list, check whether its bound changed before
  concluding the render got worse.
- **Reference renderers are given 30 seconds and then killed.** A corpus holds files written
  to make a reader loop, and `Command::output` waits forever. `Reference::render_within` polls
  and kills; there is deliberately no unbounded variant.
- **`doc/md/` holds Markdown conversions of every corpus PDF**, with real tables. When you
  need spec data — encoding tables, operator lists, value constraints — extract it from
  there rather than writing it from memory. The `WinAnsiEncoding` and `MacRomanEncoding`
  tables in `pdf-font` came out of `doc/md/ISO_32000-2_sponsored_EC3.md` Table D.2 that
  way, and the extraction caught three things memory would have got wrong: PDF's
  `MacRomanEncoding` is not Mac OS Roman, and Table D.2's *notes* assign `space` at 160
  and 202, `hyphen` at 173, and every unused WinAnsi code above 32 to `bullet`.
  The files carry base64 images inline, so `grep -v '^!\[Image\]'` before reading a range.
- **The Arlington model is the object model, not the semantics.** It says `/BaseEncoding`
  must be one of three names; it does not say what those encodings contain. Do not expect
  glyph data, operator semantics or rendering rules from it.
- **`Interpretation::text` is a readback of what was drawn**, accumulated by the same loop
  that places the glyphs, and `crates/pdf-model/tests/text_extraction.rs` compares it
  against `pdftotext` over the 14 specification PDFs in `doc/` — not the pdf.js corpus,
  which would need a per-document expectation. It is the only check that catches a code
  reaching a *plausible* wrong glyph. It found the operand-cap defect below on its first
  run, and it is known to bite: reverting that fix scores 93.2%, and shifting every
  `/ToUnicode` entry by one code scores 58.7%. Extending it to the pdf.js corpus is a real
  opportunity — 974 documents against 14 — and would need only a tolerance rather than
  expectations, since `pdftotext` supplies the reference for each. `issue20504.pdf` is the
  argument for doing it: nothing we own noticed six scripts rendering as ASCII.
- **Silent caps are defects, not safety.** The interpreter dropped operands past the 64th,
  which truncated any `TJ` array holding a justified line — three sentences on the
  specification's own title page ended mid-word, with `unsupported: []`. Bounds against
  hostile input are right; reaching one without saying so is not. Every bound now reports.
- **A command draws into the rows its clip admits, not into the page.** `Band` in
  `crates/render-cpu/src/lib.rs`, and ADR 0010 for why rows rather than a rectangle. Two
  consequences to keep in mind when touching that backend: the device transform handed to a
  command already carries the band's row offset, so anything new that composes a transform
  must use *that* one; and the clip mask is band-tall and page-wide, because `tiny-skia`
  needs it to share the pixmap's row stride.
- **The display list is deliberately flat.** `tiny-skia` wants per-clip masks, Vello wants
  a layer stack; both translate. That neither library's model is native is the evidence the
  neutral form is right, and it is what lets the CPU backend validate the GPU one on
  byte-identical input.
- **RADV and lavapipe produce byte-identical output**, so goldens need not be per-adapter.
  A test pins this; if it fails, the assumption has broken, not the code.
- **Pixel comparison cannot police text, so there is a second kind of metric now.** The
  reference renderers disagree with each other at worst-tile 26–28 on text pages — glyph
  hinting, not error — and no threshold fixes that, because the noise floor is above the
  signal. `raster_compare::Comparison::structural_similarity` (SSIM) measures whether the
  same shapes are in the same places instead, and `Tolerance` now bounds it: 0.99 for
  vector, 0.90 for text. Both numbers were measured over 153 reference-against-reference
  pairs from the corpus, and the doc comment records that the distribution is *continuous*
  — 0.8990, 0.8993, 0.8998 and 0.9009 all occur — so 0.90 is a choice about which
  population to exclude (font substitution) and not a discovered boundary. Text
  *correctness* still belongs to the extraction metric.
- **`test-scenes` holds the same page twice**, as a display list and as PDF bytes. That
  pairing is what let the harness work before a parser existed, and it is checked by a test
  that renders both and demands identical pixels.
- **`doc/` holds more than ISO 32000-2.** `PDF20_AN001-BPC.md` is the PDF Association's
  application note on black point compensation, written by ISO 32000's own
  co-project-leader, and it settled a design question the base specification leaves to
  ISO 18619 — which black to align, and why `AbsoluteColorimetric` must not compensate. It
  had been sitting unread while the same question was being answered by looking at what
  other renderers do. Check what is already in `doc/md/` before concluding the
  specification is silent.
- **Debug builds are ~15× slower here, and it changes what a test can assert.** The corpus
  gate is 1.6 s in release and minutes in debug. Any test with a timing assertion is
  meaningless at debug speed; run those in release and say so in the test. The oracle gate
  is the exception that proves it: about 95% of its processor time is three external
  renderers, whose speed does not depend on how we were built.
- `cargo-deny` is installed in the agent's `~/.cargo/bin`; run it before pushing rather
  than finding out from a red pipeline.
