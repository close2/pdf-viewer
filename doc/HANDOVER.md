# Handover

Written 2026-07-26, updated 2026-07-29 at the end of the **fourteenth** working session. Read
`/CLAUDE.md` first — it holds the five non-negotiable principles, what *done* means, and the
closed list of exclusions. **Principle 5 is the one that changes how to work**: the
specification is the only source of truth, and agreement with poppler, mupdf or pdf.js is
evidence that we read it right, never the definition of right. `doc/PLAN.md` holds the phases
and the conformance ledger's design; `doc/adr/` holds every decision's argument. **This file
is only the state of play, the traps, and what to do next** — when something here is also
written there, it is a pointer.

## What the fourteenth session changed

**`/Mask` applies, in both the forms one dictionary key spells.** §8.9.6.3's explicit mask —
a second image saying which parts of this one are painted — and §8.9.6.4's colour key ranges
were the last two of the four masking mechanisms §8.9.6 names, and the only reason
`colorkeymask.pdf` had been this project's standing example of a silently wrong page. Both are
implemented, with tests at the pixel, and five corpus documents draw completely that did not.
ADR 0023.

Three of the decisions were the clause's rather than the code's:

- **The two rasters are combined on the finer grid.** "The base image and the image mask need
  not have the same resolution … their boundaries on the page will coincide", so the true
  composite happens at *output* resolution, which an image decoder does not know. Taking the
  finer of the two grids per axis discards nothing either carries; `issue4246.pdf` masks a
  50×40 gradient with a 1000×800 stencil spelling three words, and on the image's own grid
  those words are eight blocks. Bounded at 2^24 samples, because the grid is a product of two
  numbers a document controls.
- **A colour key is a test on samples, so it lives in the unpacker.** The ranges are compared
  against "colour values before decoding with the Decode array"; after conversion those values
  are gone, and for every space but the device ones they were never in the raster. Filtered
  images are refused and reported — which is the pair the clause's own NOTE 2 warns about.
- **A `/Mask` stream that is not an image mask is reported rather than interpreted.** Table 87
  and §8.9.6.3 both require one; `issue6621.pdf` writes a one-bit `DeviceGray` image instead.
  The lenient stencil reading was written, and the page rejected it — see below.

**Then reading clause 11's half of masking produced a rule neither §8.9.6 nor Table 87
states.** §11.6.4.3: an image's `/SMask`, or a non-zero `/SMaskInData`, "shall override any
explicit or colour key mask specified by the image dictionary's Mask entry". So an image
carrying both does not get both, and the `/Mask` is not a gap to report — it is a key the file
itself has superseded. No corpus document writes both, and nothing but reading the clause the
session's own work cited would have found it.

**And §9.3.8, text knockout, stopped being silent — but not for the reason the last handover
predicted.** That file said reporting it "costs one key lookup", meaning `/TK` in an
`/ExtGState`. It does not: `Tk`'s **initial value is true**, so the gap is the default and no
key needs to appear for a page to be drawn under the wrong model. What a report has to test is
whether the two models can differ at all — the paint has to composite (a constant alpha below
one, or a blend mode other than Normal; opaque Normal painting gives both models identical
pixels) and two glyphs of one text object have to overlap. Both are now checked, and `/TK`
itself is read, including the clause's rule that a value set between `BT` and `ET` "shall be
ignored". **2 corpus documents report.** The looser check — two glyphs under a compositing
paint, without the overlap test — reported 7 and took three *agreeing* pages out of the
oracle's gated set for a difference that could not have been on any of them.

| | was | is |
|---|---|---|
| **`/Mask` as a colour key** | reported, the masked samples painted | applied, on the raw samples, bounds inclusive |
| **`/Mask` as an image** | reported, the masked areas painted | applied, combined on the finer of the two grids |
| **a `/Mask` beside an `/SMask`** | would have applied both | superseded, per §11.6.4.3 |
| **a `/Mask` that is not an image mask** | reported as `/Mask` | reported as what it is, and drawn unmasked |
| **`/TK`** | read nowhere, nothing said | read, and the gap reported where it can show |
| **§8.9.6, §11.6.4** | `partial`/`reported`, and `unreviewed` | reviewed, with one precedence rule found |

**The numbers:**

| | before | now |
|---|---|---|
| corpus documents drawing with nothing reported | 720 | **723** |
| corpus documents reporting something | 235 | **232** |
| of those, reporting an *image* | 18 | **13** |
| pages we call complete, in the oracle | 1508 | **1512** |
| of those, agreeing with the reference consensus | 672 | 672 |
| of those, contradicted | 103 | **104** |
| ledger subclauses nobody has read | 673 | **668** |
| ledger rows that are `silent` | 3 | **2** |
| `§` citations the checker verified | 382 | **447** |
| tests | 349 | **361** |

The one arrival on the contradicted list is `colorkeymask.pdf`, and it is not about masking:
its raster is 595 pixels wide where `poppler`'s and `mupdf`'s are 596, and on a page whose only
content is two coloured bands, three one-pixel edges are the whole difference. The heatmap
shows three vertical lines and nothing else. It joins `CONTRADICTED_PAGE_ROUNDING` as the third
page to arrive there by becoming *comparable* — the same thing happened to `bug1065245.pdf` and
`french_diacritics.pdf` when inline images landed.

What it taught:

- **The page rejected the lenient reading, and that is the result.** `issue6621.pdf`'s `/Mask`
  is a one-bit greyscale image with no `/ImageMask`. Treating it as a stencil is the only
  reading its samples admit, so it was implemented — and §8.9.6.2's "a sample value of 0 shall
  mark the page" then made the *background* of a court seal the painted part, blanking a page
  three renderers draw. The reading those three use is §11.6.5.2's, luminosity as opacity,
  which is a different clause about a different key and would invert every stencil whose author
  merely forgot `/ImageMask`. Where the standard defines nothing, refusing and saying so beats
  both readings.
- **A report is worth exactly its precision.** The first §9.3.8 check named 7 documents and
  cost three agreeing pages their place in the strongest gate this project has. The second
  names 2. Nothing about the *gap* changed between them — what changed is that the second one
  tests both of the clause's conditions instead of one. A report that fires where the output is
  provably identical is not caution; it is noise that removes pages from a comparison.
- **The estimate in this file was wrong in the interesting direction.** "One key lookup" assumed
  the gap arrives with the key. It arrives with the *default*. A parameter whose initial value
  is the unimplemented one is a gap on every page in the world, and the only reason that is
  bearable is that it is invisible almost everywhere — which is a fact about compositing, not
  about the key.
- **Two clauses in different parts of the standard describe one dictionary.** §8.9.6 says what
  `/Mask` means and §11.6.4.3 says when it loses. Reading either alone gives an implementation
  that is complete by its own lights. The session took the demand item from clause 8 and the
  family review from clause 11 for unrelated reasons, and the second corrected the first.

## What the thirteenth session changed

**All eight text rendering modes, and none of them needed a new display-list command.**
§9.3.6 Table 104's eight modes are three operations — fill, stroke, add to the clipping path
— and this tree did one of them properly. Modes 1 and 2 were drawn as a plain fill in the
*non-stroking* colour on 14 corpus documents, so a page that outlines its display type came
out solid and usually in the wrong colour. Modes 4 to 7 built no clip on 5 more, so
`text_clip_cff_cid.pdf` drew a solid blue bar where four renderers show "ABC123". Both were
*reported*, which is the only reason either was schedulable. `Command::Stroke` and `Clip` have
carried everything needed since the first display list; what was missing was the middle, which
is ADR 0018's `d` operator again. ADR 0022.

Three of the clause's own rules decided more than the modes did:

- **A stroke's width is in user space** — "shall be interpreted in user space rather than in
  text space" — and a `Command::Stroke`'s width is in its *path's* space. Scaling the width by
  the inverse of the glyph transform is only right when that transform is uniform, so the
  glyph outline is moved into user space instead and the command carries the state's stroke
  parameters unchanged. Exact for any text matrix, and the copy per stroked glyph is paid only
  by the four modes that stroke.
- **An empty accumulator sets no clip.** "If no glyphs are shown or if the only glyphs shown
  have no outlines … no clipping shall occur." The natural implementation clips to whatever
  accumulated, and an empty clip hides everything drawn after the text object — so a mode-7
  text object showing one space, which is a blank line of OCR text, would blank the page.
- **A hidden optional-content layer still accumulates the clip.** §8.11.3.1 lists clipping
  among the graphics state operations that "shall still be applied", and the clip outlives its
  `ET`. `end_path` had already read that clause the same way for a path's `W`; the two now
  agree instead of one clause being read twice, differently.

**Then the family review found what the mode did not.** §9.3 and §9.4 were read entire —
thirteen ledger rows — and produced a defect nothing could have seen from a page: **§9.3.3's
word spacing is a rule about a code's encoded length, and we had implemented it as a rule about
its value.** Word spacing applies to "the single-byte character code 32" and "shall not apply
to occurrences of the byte value 32 in multiple-byte codes"; we applied it to any code equal to
32, so an `Identity-H` string containing the bytes `00 20` was pushed right by `Tw` for every
one of them. No page of Latin text can show this, because a composite font's space is usually
some other CID entirely. It also produced **§9.3.8, text knockout**, as the ledger's third
`silent` row: `/TK` arrives only in an `/ExtGState` and nothing looks for the key.

**And the checker learned to read table numbers, after one of them turned out to be wrong.**
Four comments, two tests and a written report said "§9.3.6 Table 106" for the text rendering
modes. They are Table 104; Table 106 is the text-*positioning* operators. Every check passed,
because the clause exists and the table exists and only the pair is wrong. The strong gate —
the clause beside a reference must be one the standard discusses that table in — was built and
**rejected by its own output**: it fails fourteen of the tree's twenty-five references and all
fourteen are correct writing. So the assertion is the weaker true one and the gate *prints the
title of every distinct table the tree cites*, which is thirty-one lines in which the wrong
pairing is visible at a glance. `PLAN.md` §5a has the argument.

| | was | is |
|---|---|---|
| **`Tr` modes 1 and 2** | filled in the non-stroking colour, reported | stroked in the stroking colour, in user space |
| **`Tr` modes 4 to 7** | no clip built, reported | one non-zero-filled clip at `ET`, lasting until `Q` |
| **an undefined `Tr` operand** | stored, and would now have drawn nothing at all | reported, and the mode left as it was |
| **word spacing on a two-byte code 32** | applied | refused, per §9.3.3's second sentence |
| **`Table N` in a comment** | unchecked, and one was wrong | checked against the standard, and every title printed |
| **§9.3 and §9.4** | `unreviewed`, thirteen rows | reviewed, with one defect and one `silent` row |

**The numbers:**

| | before | now |
|---|---|---|
| corpus documents drawing with nothing reported | 704 | **720** |
| corpus documents reporting something | 251 | **235** |
| of those, reporting an *operator* | 33 | **15** |
| pages we call complete, in the oracle | 1492 | **1508** |
| of those, agreeing with the reference consensus | 661 | **672** |
| of those, contradicted | 103 | 103 |
| ledger subclauses nobody has read | 686 | **673** |
| cited clauses still owing a review | 25 | **23** |
| `§` citations the checker verified | 340 | **382** |
| `Table N` references the checker verified | — | **31 distinct** |
| tests | 324 | **349** |

**Sixteen documents became complete and not one of them is contradicted** — eleven agree with
the reference consensus, three are ambiguous, two are not comparable. That is unusual: every
feature since the ninth session has put at least one newly-drawable page on the contradicted
list. What it means is that these pages were not *nearly* right before; they were a solid bar
where a word belongs, and there was nothing else on them to be wrong about.

What it taught:

- **A clause's exceptions are where the failures are, and they fail in the dangerous
  direction.** Three of §9.3.6's sentences are exceptions to its own table, and the one that
  matters most — no outlines, no clipping — is the one an implementation drops silently and
  the one whose omission blanks a page. The rules are cheap; finding out they exist costs a
  careful read.
- **A rule about an encoding, implemented as a rule about a value, looks correct forever.**
  §9.3.3 has been wrong since composite fonts landed and no corpus page could show it. The
  clause states the distinction in its own second sentence. This is the same shape as
  §9.6.5.4: the code was self-consistent and right about every document anybody had opened.
- **Build the strong gate, then let it tell you it is wrong.** The table-attribution check was
  written, run, and thrown away on the evidence of its own fourteen false positives — all of
  which were correct writing. Keeping it behind an exception list would have made a checker
  that is mostly exceptions, which teaches a reader to ignore it. What replaced it asserts
  less and *prints* more, and the printing is what would have caught the original error.
- **The tree was not `clippy` clean, and this file said it was.** Eleven warnings, all from
  the twelfth session's own new files — `allow-panic-in-tests` does not reach an integration
  test's helper functions, which is not obvious and is worth knowing. Fixed. A claim in this
  file that nothing recomputes is a claim that drifts, and "clippy clean" is one somebody has
  to actually run.

## What the twelfth session changed

**The loop got three times shorter, and that was the session's first job.** The oracle gate
took 75 seconds, of which 1021 seconds of processor time were spent in `pdftoppm`, `mutool` and
`gs` against 46 in our own pipeline. Nothing about their answers changes between runs, so
`pdfref::cache` remembers them: **75 s → 25 s, with every count identical**. That claim is
checked rather than asserted — `PDFREF_CACHE=off` runs the same gate the old way, and on the
finished tree it takes 98 s and produces the same eight counts. (98 rather than the 75 measured
at the start of the session, on a busier machine and with 14 more pages in the comparison; the
number that matters here is that the two columns of verdicts match, not the two clocks.) The danger was
named in this file before it was built — "a cache key omitting one variable would compare
against stale renders in silence" — and the answer is that the key is *derived from the
invocation itself*, `Reference::build_command`'s own argument list with the two paths that vary
per page replaced, plus the renderer's version and the document's SHA-256. A flag that is not
in the key is a flag that is not passed to the renderer either. ADR 0020.

**`CCITTFaxDecode` decodes** (§7.4.6), the last image codec absent and the largest named image
gap at 12 documents. Group 3 and Group 4 through `hayro-ccitt` in the sandboxed worker, with
Table 11's eight parameters resolved on this side of the pipe so the worker holds no opinion
about PDF. Two entries are refused rather than approximated — `/DamagedRowsBeforeError`, which
is error concealment the decoder has none of, and a `/Columns` that disagrees with the image's
`/Width` — and one silence of the standard's is recorded as a choice: a stream whose data runs
out before `/Rows` is legal, and the rows never delivered are left blank. ADR 0021.

**And then `/Rotate` had been turning pages the wrong way since the first page tree.** Drawing
`issue5747.pdf`'s fax-encoded scan for the first time put it on the screen **upside down** beside
four renderers that agree with each other. §7.7.3.3 Table 31 says *clockwise*; page space here
is y-up and the flip to a raster happens later, so a clockwise turn is a negative rotation in
this space — and the 90 and 270 matrices were exchanged, which is a 180° error. Six pages were
contradicted by it, five of them filed under substituted fonts because they also carry one.

| | was | is |
|---|---|---|
| **`/Rotate 90` and `270`** | exchanged, so every rotated page was drawn 180° out | §7.7.3.3's clockwise turn, pinned by where a corner lands rather than by a matrix |
| **the oracle's reference renders** | recomputed every run, 1021 s of other programs' processor time | remembered, keyed on the invocation itself; 99.7% hits |
| **a reference timeout** | 30 s of waiting per page per run, twice over | remembered for a week, counted and printed separately |
| **`CCITTFaxDecode`** | reported on 12 documents | decoded, in the sandboxed worker |
| **§7.4.6 and §8.6.4.2** | `unreviewed` | `partial` and `implemented`, with what was refused named |

**The numbers:**

| | before | now |
|---|---|---|
| oracle gate, wall clock | 74.9 s | **25 s** |
| corpus documents drawing with nothing reported | 692 | **704** |
| corpus documents reporting something | 263 | 251 |
| pages we call complete, in the oracle | 1478 | **1492** |
| of those, agreeing with the reference consensus | 652 | **661** |
| of those, **contradicted** | 108 | **103** |
| ledger subclauses nobody has read | 688 | **686** |
| cited clauses still owing a review | 25 | 25 |
| `§` citations the checker verified | 317 | 340 |
| tests | 314 | **324** |

Six contradicted pages left by being fixed and one arrived: `bug1001080.pdf`, which is a page
of Type 3 glyphs each drawn as an inline CCITT image mask, and which we now render as
`pinL LesL` where four renderers render `pint test`. That is not a fax defect — it is
`tiny-skia` sampling four neighbours of a 39×53 bitmap drawn at five pixels — and it is the
same defect `firefox_logo.pdf` has carried for four sessions at 0.02 outside the bound. The
item was sized "Small"; it is unreadable text.

What it taught:

- **The instrument was the bottleneck, and nobody had measured it.** Eleven sessions treated
  85 seconds as the price of the oracle. It was 95% three other programs answering a question
  they had answered the day before. When a loop feels slow, measure what is in it before
  deciding what to build less of.
- **Measurement rejected the first design of the cache, and that is the interesting part.**
  Refusing to remember timeouts is obviously the safe rule; with everything else cached it
  left **two pages out of 1794 accounting for 46 of the run's 57 seconds**. The rule that
  replaced it is written down with its cost and its expiry, which is what principle 1 asks
  of a shortcut.
- **A hypothesis about a group of contradicted pages is not a diagnosis of its members.**
  `CONTRADICTED_SUBSTITUTED_FONT` held 25 pages; six of them were one line in `content.rs`
  and had nothing to do with fonts. They were only ever grouped there because they *also*
  name a font nobody embedded. The picture said so in ten seconds; six sessions of counting
  had not.
- **An upside-down page has the right ink in the right quantity.** No metric this project
  owns — not `unsupported`, not mean error, not the ink coverage a fax test could check —
  can see a 180° rotation. Only pixels somebody else produced can, which is trap 1 again in
  a form that has nothing to do with fonts.
- **A test that skips silently is worse than no test.** The first draft of `tests/ccitt.rs`
  named `bug1001080.pdf` as its fixture, whose fax data turned out to be inside a Type 3
  font's glyph descriptions rather than in an image `XObject`; both tests "passed" by doing
  nothing. A missing corpus is a skip; a present corpus that lacks what the test needs is a
  panic.

## How the project got here

Each session's argument is in its ADR; this file keeps only what is still load-bearing.

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

The contradicted count has gone 174 → 120 → 108 → 106 → 104 → 108 → 103 → 103 → 104 across
sessions 6 to 14, and the corpus's incomplete count 291 → 368 → 250 → 290 → 283 → 263 → 251 →
235 → 232 —
both move in both directions on purpose: a rise in the first can mean pages *joined* the comparison and a
rise in the second is honesty when a silence ends, and the sections below say which.

## Where we are

A PDF **renderer** that opens real files and draws pages: geometry, colour, images,
shadings, patterns, embedded text and annotation appearances, on both a CPU and a GPU
backend, with JBIG2 and JPEG 2000 images decoded in a confined worker process. It is not yet
a PDF *viewer* in the full sense — no forms, encryption or transparency groups — and the gap
between those two words is measured further down rather than guessed at.

- **361 tests**, `clippy` clean under `pedantic` + `unwrap_used`/`panic`/`arithmetic_side_effects`,
  `cargo fmt --check` clean, `cargo deny` clean on all four checks (verified, not assumed — and
  the thirteenth session found this line had been *wrong*: eleven warnings had accumulated in
  the twelfth session's own new files, because `allow-panic-in-tests` does not reach an
  integration test's helper functions).
- **The 14 specification PDFs in `doc/`** — including ISO 32000-2 itself, 1023 pages and
  101 318 objects — all parse, all render page one with only a soft mask reported on three
  of them, and all extract **100% of the words `pdftotext` finds**.
- **The 974-document pdf.js corpus is a gate, not a survey.** All 974 open, 955 reach page
  one, **723 draw with nothing reported at all**, and everything the other 232 cannot draw
  is named. The counts are ratcheted. 1501 of 1501 PDF functions parse; **all 1793 shadings
  build**, mesh types included. The whole gate runs in **1.5 s** and has **no named slow
  document left**.
- **A second gate asks whether what we drew is *right*.** `oracle.rs` compares us against
  poppler, mupdf and ghostscript over **1794 pages** — every page of the corpus, plus page
  one of each specification PDF — **in 25 s**, because the references' renders are remembered
  between runs rather than recomputed (ADR 0020). Of the 1512 pages we claim to draw
  completely, **672 agree with the reference consensus, 104 are contradicted by it and 723 are
  pages the references cannot agree on among themselves**. The 103 are named, grouped and
  ratcheted in both directions. ADR 0011.
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
- **A glyph may be a content stream.** Type 3 fonts (§9.6.4) are read in `pdf-model`, since
  drawing one means running the interpreter — `/FontMatrix` for glyph space, `/Widths` read
  through it, an encoding that is the whole mapping, and `d0`/`d1` including the rule that an
  uncoloured figure takes its colour from outside. ADR 0018.
- **An image may be written into the content stream.** Inline images (§8.9.7) are scanned into
  the dictionary an image `XObject` would have had and decoded by the same route, so nothing
  downstream knows one from the other. Where the data ends is answered by `/L`, by §8.9.3's
  sample layout, or — for filtered data in a file with no `/L` — by a search. ADR 0019.
- **Every image codec a PDF may name now decodes.** `CCITTFaxDecode` was the last one
  absent (§7.4.6, ADR 0021): Group 3 and Group 4 through `hayro-ccitt` in the same sandboxed
  worker, with Table 11's parameters resolved before they cross the pipe. `LZWDecode` is the
  only standard filter of any kind still missing, and no corpus first page reaches it.
- **An image is masked the three ways §8.9.6 defines.** Its own `/ImageMask` stencil, an
  explicit `/Mask` naming a second image, and a colour-key `/Mask` naming ranges of sample
  values — the last two from the fourteenth session (ADR 0023). Two images of different sizes
  are combined on the finer grid, which is a documented choice rather than a derivation: the
  clause puts both on the unit square and leaves the sampling to the device. §11.6.4.3's
  precedence is honoured, so an `/SMask` beside a `/Mask` supersedes it.
- **A rotated page turns the way the standard says.** §7.7.3.3 Table 31's `/Rotate` is a
  *clockwise* turn as displayed, which in this y-up space is a negative rotation; 90 and 270
  had been exchanged since the first page tree, so every rotated page in the corpus was drawn
  180° out. Six contradicted pages were this one line.
- **An image is filtered only where the document allows it.** §8.9.5.3's `/Interpolate`
  decides whether a *magnified* image is smoothed, and both backends ask
  `Image::is_smoothed` so they cannot disagree. A reduced image is still filtered, which the
  clause does not address and this does not pretend it does.
- **A layer the document turns off is not drawn.** §8.11 in full as far as it decides what is
  marked: the default configuration, membership dictionaries including `/VE` visibility
  expressions, intent, and `/OC` on marked-content spans, XObjects and annotations. ADR 0017.
- **The citations are checked.** `tools/conformance` holds every `§` in the tree to a clause
  the standard has, every rustdoc blockquote to the standard's own words, and the conformance
  ledger's 823 rows to the standard's subclauses. ADR 0016, `doc/PLAN.md` §5a.
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
# The conformance gate is part of that run and needs nothing but the tree. Its summary is
# worth reading rather than only passing:
cargo test -p conformance -- --nocapture   # 447 citations, 21 quotations, 32 tables, 823 rows
cargo run -p conformance --bin ledger      # regenerates the rows, keeps every status
# Both gates decode images in a separate program, and -p pdf-model does not rebuild
# another package's binaries. Build it first or the numbers below are somebody else's.
cargo build --release -p pdf-sandbox --bins
cargo test --release -p pdf-model --test corpus -- --ignored --nocapture   # 974 docs, ~1.6 s
cargo test --release -p pdf-model --test oracle -- --ignored --nocapture   # 1794 pages vs 3 voting renderers, ~25 s
# The first run of that on a fresh build directory is ~95 s and writes 319 MB of remembered
# reference renders; every run after it is the 25 s above. Two environment variables matter:
#   PDFREF_CACHE=off              ask the three renderers again, which is how "the cache
#                                 changes no verdict" is re-checked over the whole corpus
#   PDFVIEWER_ORACLE_ONLY=a,b     compare only pages whose names contain a or b — 0.2 s for a
#                                 handful of documents. A filtered run refuses to check the
#                                 ratchets and says so.
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
| `pdf-model` | Page tree, content interpreter, annotations, optional content, Type 3 fonts, image decode | Where PDF semantics live. `optional_content.rs` answers "is this layer on"; the interpreter asks it in three places (§8.11.3.2 and §8.11.3.3). `type3.rs` reads a font whose glyphs are content streams, because running one needs the interpreter (§9.6.4, ADR 0018). `inline_image.rs` turns a `BI` … `EI` sequence into the stream an image `XObject` would have been, so `image.rs` stays the only decoder (§8.9.7, ADR 0019). `image.rs` also owns §8.9.6's masking: `mask_entry` reads `/Mask` once and decides which of its two mechanisms a file means, and it is the one place where two rasters of different sizes are combined rather than refused (ADR 0023) |
| `pdf-font` | Glyph outlines via `skrifa` | Owns both encoding algorithms: §9.6.5.2 for CFF, §9.6.5.4 for `TrueType` (ADR 0015). `cff.rs` adapts `read-fonts`; `encoding.rs` is Annex D and Table 113 data; `substitute.rs` is the only machine-dependent code in the tree. A Type 3 font is refused here — its glyphs are content streams, so it belongs in `pdf-model` |
| `pdf-render` | Display list + `Rasterizer` trait | No PDF semantics, no rasteriser. `Path::extend_transformed` is the one place geometry is moved rather than travelling with a transform, and both callers are §9.3.6's text (ADR 0022) |
| `render-cpu` | `tiny-skia` backend | Correctness oracle **and** startup path |
| `render-gpu` | Vello/wgpu backend | Headless by construction |
| `raster-compare` | Tolerant image metrics | Worst-tile error is the load-bearing one |
| `test-scenes` | Shared fixtures | Holds the same page as a display list *and* as PDF bytes |
| `tools/pdfref` | Reference-comparison harness | Triangulation rule lives here. `cache.rs` remembers what each reference renderer produced, keyed on the invocation itself so a changed flag cannot be answered from a render made under the old one (ADR 0020); `digest.rs` is the SHA-256 that key is built from, and is the one file in the tree citing a standard other than ISO 32000-2 |
| `viewer-ui` | The application | `src/bin/pdf-viewer.rs` |
| `pdf-sandbox` | Confined worker + the three image filters | Its `decode.rs` is the only place a JBIG2, JPX or CCITT codestream is looked at. `CCITTFaxDecode` joined in the twelfth session (§7.4.6, ADR 0021), through the same protocol with Table 11's parameters resolved before they cross the pipe |
| `tools/hayro-compare` | Drives `hayro` for the oracle's fourth panel and for speed | Nothing ships it; it is where `hayro`'s forty dependencies live |
| `tools/conformance` | Citation checker and the conformance ledger | Depends on nothing but `thiserror`. The one crate the citation scan skips — its own comments cite clauses that do not exist, deliberately. Since the thirteenth session it also reads `Table N` references and prints every cited table's title, which is the check a wrong table number needs |
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

**And the rule is not really about fonts.** The tenth session put Type 3 fonts in, opened ten
pages beside `poppler`'s, and the difference on the first one was that our squares were solid
where three renderers drew them dashed — a defect in the `d` operator, which has nothing to do
with fonts, text, or the feature being written. It was on the screen in the first minute.
Whatever you have just built, render a page that uses it and look at the whole page.

**The eleventh session is the same lesson with a bigger yield.** Inline images landed, five
pages appeared in the oracle's newly-contradicted list, and opening them one at a time gave
`/Interpolate` (a four-sample image drawn as a blur where three renderers draw four squares), a
`Lab` colour table scaled as though its components ran 0 to 1 (a black bar where four
renderers draw pale grey), and a soft mask silently dropped for being a different size (black
bars across a page of text). None of the three is an inline-image defect. **Every page a new
feature makes drawable is a page nobody has ever looked at.**

**The twelfth session found the largest one yet, and it was not in the newly-contradicted
list at all.** `CCITTFaxDecode` landed; one of the twelve documents it made drawable was a
scan on a `/Rotate 270` page, and opening the side-by-side showed our panel **upside down**
beside four that agree. 90 and 270 had been exchanged since the first page tree. Two things
about that are worth carrying:

- **No metric this project owns can see a rotation.** An upside-down page reports nothing, has
  the right ink in the right quantity, the right page size, and a mean error that looks like a
  font difference. Only pixels somebody else produced can tell you, and only if you look.
- **It was hiding inside a group of contradicted pages that named the wrong cause.** Five of
  the six affected pages sat in `CONTRADICTED_SUBSTITUTED_FONT` for six sessions, because they
  also happen to use a font nobody embedded. Open the artefact before believing the label.

**The thirteenth session is the rule's happier form and still worth the hour.** Sixteen
documents became drawable when the text rendering modes landed, and looking at all of them
found nothing wrong with the new code — but `recursiveCompositGlyf.pdf` came out a solid red
page and it took a side-by-side to learn why: its font is a deliberately malformed TrueType
whose composite glyph refers to itself, `skrifa` produces no outline, and §9.3.6's "if the only
glyphs shown have no outlines … no clipping shall occur" then leaves the page unclipped. Two
of the four references do the same. The picture was not a defect and it *was* the fastest way
to understand what the code now does with a broken font — which is the other reason to look.

**The fourteenth session is the rule inverted, and that is the version worth having.** The
picture did not find a defect in new code — it *rejected a reading of the specification*.
`issue6621.pdf` writes a `/Mask` that is a one-bit greyscale image rather than an image mask,
which the standard does not define; the only reading its samples admit is §8.9.6.2's stencil,
where a zero sample marks the page. That was implemented, and the side-by-side showed our
panel blank beside three renderers drawing a court seal, because the seal's *background* is
what a zero sample turns out to be there. No test could have said so and no metric could:
the code was right about the clause it cited, and the clause does not apply. **When the
standard defines nothing, the page is the only thing that can tell you your reading is
untenable — and "untenable" is a stronger result than "wrong", because it is what justifies
refusing rather than guessing.**

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
entirely. All three are implemented now, the last of them in the fourteenth session. **A rise in the incomplete count is not a regression when it is a new report** —
it is this rule being applied to somewhere it had not been.

The lesson generalises: a feature that is *partly* implemented is the easiest place to lose
this rule, because the operator is handled and the code path exists. `Tr` was parsed, the
mode was stored, three of its eight values were reported, and the four that change the clip
were not. All eight are implemented as of the thirteenth session (ADR 0022) — and the same
review found the *next* instance one clause away, in the ninth text state parameter: `/TK`
was not read at all and nothing said so (§9.3.8). It reports as of the fourteenth session,
and trap 11 is what that cost to get right.

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

This trap was stated in the sixth session and acted on in the ninth: it is the reason
`CLAUDE.md` principle 5 now defines *done* as every PDF rendering as its producer specified —
scope stated as a closed exclusion list rather than inferred from what the corpus asks for —
and the reason the conformance ledger (`PLAN.md` §5a) exists at all. A caution that changes
no plan changes nothing.

### 9. Two references can agree because they share code — or because they share a *gap*

The oracle's whole authority rests on a premise stated in ADR 0005: two implementations
sharing no code agreeing about a page is evidence. There are two ways for that premise to
fail, and the ninth session found the second.

**The second way is the common one, and it needs no shared code at all.** An unimplemented
feature almost always falls through to *draw it*, so two unrelated programs that have both
skipped the same clause produce the same picture — and the gate reads that as agreement.
`visibility_expressions.pdf` is the case: two of its five `/VE` visibility expressions are
false, so two lines stay pale. We draw them pale, and so does `poppler`; `mupdf` and
`ghostscript` draw all five dark. Their source says why, and it was read rather than inferred
— `mupdf`'s `pdf-layer.c` carries `/* FIXME: Calculate visibility from array */ return 0;`
and `ghostscript`'s `pdf_optcontent.c` carries `WARNING: OCMD contains VE, which is not
supported (ignoring)`. Meanwhile `poppler` exports `OCGs::evalOCVisibilityExpr` and pdf.js
implements it in `src/core/evaluator_utils.js`, in this repository under `doc/pdf.js`.

Three implementations against two, and §8.11.2.2 is not ambiguous: "If the VE key is present
it shall be used in preference to the OCGs and P keys." So the page stays contradicted, listed
under `CONTRADICTED_VISIBILITY_EXPRESSION` with the source citations beside it.

**What to do when a contradiction looks like this.** Read the clause first, then go and look
at what the disagreeing renderers actually *do* — their source is a search away, and "does it
implement this at all" is a much cheaper question than "is its answer right". A `FIXME` in
another project's source is stronger evidence than any number of agreeing pixels.

The first way is narrower and was found in the seventh session, on JBIG2.

**`mupdf` and `ghostscript` both link `jbig2dec`**, Artifex's library, and on seven corpus
pages it decodes nothing and renders blank, or renders the drawing strewn with noise blocks,
or prints `segment marks bitmap coding context as retained (NYI)` and gives up. Two renderers
then "agree" and the gate reports us contradicted.

It took a side-by-side to see it and a log to prove it: both renderers emit the *same warning
text*, because it is the same code emitting it. What settles those pages is not poppler's
agreement — that would only be evidence that we read ISO/IEC 14492 the same way — but
`tests/jbig2.rs`, where the corpus encodes one image ninety-six ways and all ninety-six decode
to byte-identical pixels here. The seven are `CONTRADICTED_SHARED_JBIG2_DECODER` in
`oracle.rs`, listed rather than excused.

**So ask what a reference is made of, not only what it produced.** `poppler`, `mupdf` and
`ghostscript` look like three implementations and are three *renderers*; underneath they share
libraries per format, and separately they share whichever clauses none of them has got round
to. Both are worth checking wherever two of them agree suspiciously often.

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

### 10a. A cached reference render is a fourth thing that can be stale

Added in the twelfth session, when the oracle stopped asking `pdftoppm`, `mutool` and `gs`
every run and started remembering what they said (ADR 0020). It went from 75 seconds to 25,
and it introduced exactly one new way to be wrong: comparing against a render that is no
longer what the renderer would produce.

The key is built from the invocation itself — `Reference::build_command`'s own argument list,
plus the renderer's version and the document's SHA-256 — so **a flag that is not in the key is
a flag that is not passed to the renderer either**, and the class of mistake this file warned
about before the cache existed cannot happen by omission. What it cannot see is a renderer
whose output changes while its version string does not, which is a distribution's problem.

Three things to know:

- `PDFREF_CACHE=off` runs the gate the old way. Doing that and comparing the counts is how
  "the cache changes no verdict" is checked over the whole corpus rather than on one fixture.
- **The hit rate is printed, and it is the tell.** A run over an unchanged tree that reports
  less than 99% is telling you the corpus or a renderer moved.
- **A remembered *timeout* is the one entry whose truth decays**, and it is counted on its own
  line for that reason. It expires after a week; the argument for remembering it at all — two
  decompression bombs were 46 of a 57-second run — is in `pdfref::cache`.

### 10. The sandbox worker is a separate binary, and Cargo will not rebuild it for you

`cargo test -p pdf-model` builds pdf-model's targets and pdf-sandbox's *library*. It does not
build pdf-sandbox's `pdf-sandbox-worker` binary, because Cargo never builds another package's
binaries. So the tests run against whatever worker was last compiled.

This is not hypothetical. While verifying that `tests/jbig2.rs` can fail, the seventh session
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

### 11. A report is only as good as the condition it fires on

Added in the fourteenth session, and it is trap 5's other edge. Principle 3 says unsupported
input must stay loud, and the reflex it produces is to report whenever the unimplemented thing
*could* be involved. §9.3.8 is the case: `Tk`'s initial value is true, so every text object in
every document is composited under a model we do not implement, and a report on that fact
alone would name several hundred documents and mean nothing.

The first draft asked one of the clause's two conditions — glyphs drawn under a paint that
composites — and named 7 documents. It also took **three pages that agreed with the reference
consensus out of the oracle's gated set**, because a page that reports is a page the oracle
stops judging (see the eighth session's 43). Those three could not have shown the difference:
their glyphs do not overlap, and knockout changes nothing where nothing overlaps.

The second draft asks both conditions — the paint composites *and* two glyphs of the object
overlap — and names 2. The gap did not change. What changed is that the report now marks pages
where the two models can actually produce different pixels.

Two things to carry:

- **Costing a report in gated pages is part of designing it.** A report is not free: it moves
  a page from "judged against three renderers" to "expected to differ". Over-reporting buys
  honesty about a page that was already right and pays for it in coverage.
- **The condition belongs in the clause, not in intuition.** Both halves of this one are
  written in §9.3.8 and §11.3.7 — knockout is about *the area of overlap*, and alpha is shape
  times opacity, so opaque Normal painting composites identically either way. Reading them is
  what made the narrow test writable at all.

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
| Text: CID encodings, embedded `CMap`s | 100 | Medium | The breakdown from the gate's own output, counting *fonts* rather than documents: 27 with no `/ToUnicode` so a substitute cannot be addressed, 26 with a non-identity `/CIDToGIDMap`, 23 whose substitute draws none of the codes the document declares, 14 with an embedded `CMap` stream, 6 with a predefined `CMap` (`90ms-RKSJ-H`, `UniJIS-UTF16-H`, …), 3 asking for vertical writing (below). Only the predefined `CMap`s need vendored data, which is a licensing decision rather than a coding one. |
| Synthesised annotation appearances | 63 | Medium–large | An annotation with **no** `/AP` must be drawn from `/IC`, `/C`, `/BS`, `/Border` and its subtype's own rules — a different routine per subtype. 26 `Widget`, 18 `Link`, and the rest markup annotations. Reported, never guessed. ADR 0013. |
| Transparency groups, soft masks | 29 | Large | The largest *rendering* gap, and the last thing `doc/` reports. What reports is a soft mask in an `/ExtGState`, on 29 documents, as `Shading { name: "SMask in /GSn" }`. A transparency **group** reports nothing at all: it is drawn as an ordinary form `XObject`, so `/Group`, its isolation and its knockout flag are silent. §11.4.6 is the ledger's first `silent` row for that reason. (This row said 45 documents, "26 as `Shading`, 19 as `Operator`", until the eleventh session checked it: the 19 `Operator` reports were the text rendering modes, which the thirteenth session implemented — so the `Operator` row of the corpus gate has fallen from 33 to 15 and holds nothing but malformed streams. A number in this file that nothing recomputes is a number that drifts.) |
| Encryption | 20 | Medium | RC4/AES, `/Encrypt`. 11 documents cannot reach page one at all and 9 more draw a blank page. |
| Form field appearance construction | 7 | Medium | `/NeedAppearances` (§12.7.4.3). The field's value is known only at viewing time, so its appearance has to be built from `/V`, `/DA` and `/Q`. The stored appearance is drawn and the staleness reported. |
| Optional content: the interactive half | — | Medium | §8.11 is honoured wherever it decides what is *drawn* (ADR 0017). What is missing is a layer panel and what feeds it: `/Usage` and the `/AS` usage application dictionaries (§8.11.4.4), which switch groups by zoom, language or print state, and `/Order`, `/ListMode`, `/RBGroups`, `/Locked` and the alternate `/Configs`. §8.11.4.4 is the ledger's second `silent` row: this viewer has a window, so those requirements do apply to it, and a layer that should switch itself off is drawn with nothing said. |
| `LZWDecode` | 0 | Small | **The last standard filter absent of any kind**, now that `CCITTFaxDecode` decodes. **This row said 3 and the three were miscounted**: `bug864847.pdf`, `XiaoBiaoSong.pdf` and `SimFang-variant.pdf` contain the string `LZWDecode` and all three draw page one completely, so nothing in the corpus exercises it on a first page. `colour_paths.rs` pins the report on a synthetic file and will fail when the filter lands — which is the only instrument that covers it, and trap 8 in one line. |
| Text knockout (`Tk`, §9.3.8) | 2 | Medium | Table 102's ninth text state parameter, and the only one absent. Its initial value is `true`, which makes a whole text object a non-isolated knockout group so a later glyph overwrites an earlier one where they overlap; we composite each glyph separately, which is the `Tk` false model — indistinguishable while glyphs are opaque under the Normal blend mode, and wrong otherwise. **Reported since the fourteenth session**, on the two documents where both of the clause's conditions hold: the paint composites, and two glyphs of one object overlap. `/TK` is read, including the rule that a value set between `BT` and `ET` is ignored. Implementing it is §11.4.6's knockout groups seen from clause 9, and belongs with them. |
| Image `/Mask` on a filtered image | 0 | Small | **Both forms landed in the fourteenth session** (§8.9.6.3, §8.9.6.4, ADR 0023) and this row is what is left of them. A colour key is a test on the samples a filter delivers, and a `DCTDecode` or `JPXDecode` image has become RGBA before the unpacker sees it — the clause's own NOTE 2 names that pair as the one lossy coding makes unreliable. JBIG2 and CCITT are refused with them rather than special-cased. No corpus document writes one. Also here: a `/Mask` stream that is not an image mask, which Table 87 excludes and 1 document writes — see trap 11. |
| **Image reduction quality** | 2 | **Medium** | The *other* half of §8.9.5.3, which the clause does not address: `/Interpolate` is honoured for magnification (eleventh session), and a reduced image is still filtered bilinearly whatever the reduction. This row said "Small, 1 document, 0.02 outside the bound" for four sessions, on `firefox_logo.pdf`'s eightfold shrink of a logo. **`bug1001080.pdf` is the same defect and the cost is legibility**: its text is a Type 3 font whose every glyph is an inline CCITT image mask, a 39x53 bitmap drawn at about five pixels, so the crossbar of a `t` is one source row in fifty-three and bilinear's four neighbours never touch it — we draw `pinL LesL` where four renderers draw `pint test`. The fix is a filter averaging over the destination pixel's footprint, in both backends, and wants a benchmark first. It is now the best-argued small item on this list. |
| `/UserUnit` | 2 | Small | §7.7.3.3: the size of a default user-space unit in multiples of 1/72 inch. `mutool` and `gs` scale the page by it, we and `poppler` do not — `bug1947248_*.pdf` come out at 612x792 where they produce 1836x2376. Neither applied nor reported; the oracle lists them under `GEOMETRY`. |
| Annotation `NoZoom`, `NoRotate` | — | Small | Table 167 bits 4 and 5 make an appearance's size or orientation depend on the *view*, which a resolution-independent display list cannot express. Rare. |
| Type1 fonts (`/FontFile`) | 0 | Medium | No corpus page one reaches it, so this is smaller than it looks. `read_fonts::ps::type1` exists — check before writing any. |
| Soft masks of a different size | 3 | **Small–medium** | §11.6.5.2 Table 143 makes an `/SMask`'s dimensions "independent" of its image's, with "both images mapped to the unit square … regardless of whether the samples coincide individually". We hold one raster per image, so combining them means choosing a grid. **The fourteenth session answered exactly this question for `/Mask` and did not carry the answer here** — `image::apply_explicit_mask` combines on the finer of the two grids, bounded at 2^24 samples, and the same function would close most of this row. Two reasons it was left: the pathological case is on this side (`issue16263.pdf` gives a 2x2 image a 34862x4332 mask, 604 MB, which the bound would refuse and report as today), and a soft mask carries continuous values where a stencil carries two, so nearest-neighbour deserves an argument rather than a copy. The real answer is still compositing at *device* resolution, which is a display-list question and belongs with transparency groups. This is now the cheapest of the four gaps that draw a page visibly wrong. |
| Bit depths 2, 4 and 16 | 3 | Small | §8.9.3 permits five component widths and the unpacker reads two. Refused and reported, which is honest, and is now the largest *codec-shaped* image gap left — though the `/Mask` row above it affects more documents. |
| Vertical writing (`Identity-V`, `/W2`) | 3 | Medium | §9.2.4 gives a glyph in writing mode 1 a second set of metrics — a displacement vector `w1` and a position vector `v`, from the CIDFont's `/W2` and `/DW2` (§9.7.4.3). None of it is read. `Identity-V` was accepted beside `Identity-H` until the tenth session, because the two map codes identically, and `vertical.pdf` came out as one overlapping line across the top of a page where two columns belong down the right edge. Now refused and reported. |
| Sampled shadings on the GPU | 2 | Small | Type 1 only; the CPU backend draws them. |
| Rendering intents beyond `AbsoluteColorimetric` | — | Small | Read and recorded; `A2B0` is not yet selected for `Perceptual`. |
| Forms, actions, the rest of clause 12 | — | Large | Interactivity: field values, calculation order, navigation. In scope wherever it *displays* — field appearance construction, outlines, destinations, page labels. **JavaScript and script-driven field behaviour are excluded** by principle 5's closed list; field appearance is not. Not needed to *draw* an annotation, which is why drawing landed without any of it. |
| Tagged PDF, metadata | — | Large | Clause 14 beyond output intents. In scope as far as accessibility needs it. |
| Sandboxing the *rest* of the renderer | — | Large | Spike D is done for the image codecs (ADR 0014). Interpreting and rasterising still happen in the main process. |
| JPEG 2000 at reduced resolution | 1 | Small | `issue19517.pdf` is a 12608x16806 scan whose full decode wants gigabytes for a page drawn at four megapixels. JPEG 2000's answer is to decode a lower resolution level, which needs the intended scale to reach the decoder. Refused with a clear report today. |

## How much of the specification is implemented

Four answers, because the honest one depends on what you are counting — and they are in
ascending order of how much they should worry you. The first counts what we *report*, the
second what an implementation that shares no code with us *sees*, the third what the
standard contains, and the fourth is what a person has actually read.

The first two are measured. **The third is a self-assessment and has been wrong twice**: it
called clause 9's encoding algorithms "implemented in full" while §9.6.5.4 was one line
covering about one and a half of its five routes, and the feature table below said Type 3
fonts were reported for two sessions in which they were not. Both errors were found by
pixels, not by reading. Read the "By clause" table as what the code's authors believe.

**The fourth now exists**, and it is the conformance ledger. Its headline is not a percentage
implemented but a count of unasked questions: **668 of 823 subclauses are `unreviewed`**, and
155 have been read against this code — 81 of those being clause 13, which principle 5
excludes by name. So the honest summary of clause coverage is that the project has begun
measuring it and has measured 9% of it. That number is meant to look bad; the alternative was
not knowing.

### By what real documents need

Over the 974-document pdf.js corpus, page one:

| | count | share |
|---|---|---|
| opens | 974 | 100% |
| reaches page one | 955 | 98% |
| **draws with nothing reported** | **723** | **74%** |
| draws, with something reported | 232 | 24% |

That 74% is the number to quote for *reporting*. It moved by three documents in the fourteenth
session and the arithmetic is worth having, because both directions happened at once: five
left the reported list when `/Mask` began to apply (§8.9.6.3 and §8.9.6.4) and two joined it
when §9.3.8's text knockout began to report. It **rose** by two points in the thirteenth
session, by one in the twelfth, by two in the eleventh and by one in the tenth: the eight text
rendering modes account for 16 documents, `CCITTFaxDecode` for 12, inline images and the four
colour space families the image unpacker refused for 23, and three began saying that their
soft mask is not the size of its image. Both halves are the point. It **fell** from 72% in the eighth session,
when 24 documents began saying they carry a Type 3 font and 19 that their substitute draws
none of the codes the document declares — nothing had stopped drawing correctly, what stopped
was drawing *incorrectly in silence*, and `issue918.pdf` was emitting 388 text operations of
letter fragments while reporting nothing.

It went down in the sixth session too, from 68% to 60%, for the same kind of reason, and up
in the seventh when JBIG2 and JPEG 2000 landed. **This number measures honesty, and honesty
can fall as capability rises** — so a rise is only good news when you can say which
capability caused it, and a fall is only bad news when you cannot say which silence ended.

### By what an independent renderer sees

This is the number to worry about. Over all 1794 pages compared, of the 1512 we claim to
draw completely:

| | count | share of the 1512 |
|---|---|---|
| agree with the reference consensus | 672 | 44% |
| **contradicted by it** | **104** | **7%** |
| the references cannot agree among themselves | 723 | 48% |
| not comparable (geometry, or fewer than two renderers) | 13 | 1% |

**One page in fourteen that we say we drew completely, two independent implementations say we
did not.** The 104 are named in `oracle.rs` and grouped by what the page carries: 19 use a
font nobody embeds so every renderer substitutes differently, **8 are pages where the two
references that agree are wrong and we are right** — 7 where they are the same JBIG2 decoder
and 1 where neither implements `/VE` (trap 9 has both) — 8 are a one-pixel page-rounding
difference, 2 are image reduction quality, 1 is an image half a device pixel tall, 1 is a
`CalRGB` alternate space two references do not convert, 1 is a page of glyphs being judged
with the tolerance for flat fills, and **64 have nothing on them to explain it**. That last
group is the most valuable list in the repository. 21 of them are pages beyond the first,
which a page-one comparison would never have seen.

**103 → 104 in the fourteenth session, and the one arrival is the page-rounding group's
third.** `colorkeymask.pdf` became comparable when colour key masking landed; its raster is
595 wide against `poppler`'s and `mupdf`'s 596, and the diff heatmap is three vertical lines
one pixel wide at the three band edges. The masking itself agrees. Read that as the pattern
this file keeps describing rather than as a regression: a feature that makes a page drawable
adds it to the set being judged, and a page holding two coloured bands has nothing else on it
for a one-pixel edge to be averaged against.

**The session before it was 103 → 103, with sixteen pages added to the denominator and none of
them contradicted.**
That is the thirteenth session, and it is unusual enough to be worth stating: every feature
since the ninth has put at least one newly-drawable page on this list. Eleven of the sixteen
agree with the reference consensus outright. What it means is that these pages were not
*nearly* right before — they were a solid bar where a word belongs, with nothing else on them
to be wrong about.

**The session before it went 108 → 103, and the arithmetic was six out and one in.** Six left
by being *fixed*, and all six were `/Rotate 90` pages drawn 180° out — five of
`hello_world_rotated.pdf` and `issue6019.pdf`, every one of them filed under substituted fonts
because they also carry one. That is the caution above arriving in quantity: **which group a
page is in is a hypothesis, and six of the twenty-five in the largest group had nothing to do
with fonts.** The one that arrived is `bug1001080.pdf`, which joined the comparison when
`CCITTFaxDecode` landed and is about image reduction rather than about fax.

**Read the 48% ambiguous with care.** It is not "half the corpus is unsettled": 372 of those
720 pages are two long books, `freeculture.pdf` (320 pages) and `pdkids.pdf`, whose text uses
fonts nobody embedded, so each renderer substitutes a different one and the structural bound
separates them. Ambiguity concentrated in a handful of documents says more about those
documents than about the gate.

**So read the 720 as "reported nothing", not "drew it right".**

### By clause

ISO 32000-2 carries 823 subclauses under its eight technical clauses, and counting them is a
poor proxy for work: clause 12 is 166 of annotation subtypes a viewer adds one at a time,
while clause 8's 128 decide whether any page looks right at all.

**Every entry in the table below is a judgement about state, not a measurement**, and the
thing that turns one into a measurement is the ledger's `status` column. 150 of the 823
subclauses now carry one, 81 of those being clause 13's exclusion — so read this table as
belief, and the ledger as what has been checked. Where the two disagree, the ledger is the
one that had to name a code site.

| Clause | Subclauses | State |
|---|---|---|
| 7 Syntax | 138 | **Nearly complete**, and 4 of its 138 rows are now reviewed. Objects, **every standard filter but `LZWDecode`** — JBIG2 and JPEG 2000 in the seventh session, `CCITTFaxDecode` in the twelfth (§7.4.6, ADR 0021) — classic and stream xrefs, object streams, incremental updates, recovery by scanning. **Encryption is absent** and is the largest hole here. |
| 8 Graphics | 128 | **Nearly complete**, and the clause with the most ledger coverage: 38 of its 128 rows are reviewed, §8.9 now as a family. Paths, clipping, all eleven colour space families, all seven shading types, both pattern types, form and image XObjects, inline images (§8.9.7, eleventh session), `/Interpolate`, an image's `/Mask` in both forms (§8.9.6, fourteenth session), ICC colour management, and — since the ninth session — optional content (§8.11) wherever it decides what is drawn. A general `/Decode` array is still not applied and not reported, and 2, 4 and 16 bits per component are refused. |
| 9 Text | 65 | **Partial**, and 23 of its 65 rows are reviewed — §9.3 and §9.4 as two whole families in the thirteenth session. Simple and composite fonts through embedded TrueType, CFF and OpenType programs; the standard 14 by substitution; `/ToUnicode`; Type 3 fonts, whose glyphs are content streams (§9.6.4, ADR 0018); and all eight text rendering modes (§9.3.6, ADR 0022). §9.6.5.2's CFF encoding algorithm and §9.6.5.4's `TrueType` one are both implemented in full, the second as of the eighth session (ADR 0015). Missing: bare Type1 (`/FontFile`), embedded `CMap` streams, predefined `CMap`s, vertical writing mode, and text knockout (§9.3.8), which since the fourteenth session is `reported` rather than `silent`. |
| 10 Rendering | 36 | **Partial, and much of it is `inapplicable` rather than missing.** Colour management and rendering intents are done. Halftones, transfer functions, flatness and smoothness describe a marking device rather than a screen — a ledger status of its own, and not the same as excluded. |
| 11 Transparency | 58 | **Minimal**, and 6 of its 58 rows are reviewed — §11.6.4 as a family in the fourteenth session, which is where the precedence between an image's masks is stated. All sixteen blend modes are implemented and reach both backends, `ca` and `CA` are two constants with a test apiece, and an `/SMask` sample-for-sample with its image supplies its alpha (§11.6.5.2). Transparency groups, knockout and isolation are not — this is the largest *rendering* gap, §11.4.6 is the ledger's remaining `silent` row alongside §8.11.4.4, and a soft mask whose grid is not its image's is reported rather than resampled. `/AIS` is read nowhere; it can only show once groups exist. §9.3.8's text knockout is the same gap seen from clause 9 and now reports. |
| 12 Interactive features | 166 | **Appearances only.** Annotations are placed and drawn from `/AP` (§12.5.5), with the visibility flags of §12.5.3 honoured. Nothing is synthesised, and no forms, actions or navigation exist. |
| 13 Multimedia | 81 | **Excluded**, by name, on principle 5's closed list: a media engine rather than a rendering question. Its rows still appear in the ledger carrying that exclusion, because an invisible exclusion is indistinguishable from an oversight. |
| 14 Document interchange | 152 | **Output intents only.** No tagged PDF, no metadata, no marked-content semantics — `BDC`/`EMC` are parsed and ignored. |

So: the parts of the standard that decide whether a page is drawn correctly are largely
done; the parts that make a document *interactive* are not started.

### Feature-by-feature, from the source

| | |
|---|---|
| Content-stream operators | **73 of 73** in Table 50 (`ID`/`EI` are consumed inside the `BI` handler rather than as arms). `d0` and `d1` landed with Type 3 fonts and were the last two. `BMC`/`EMC` maintain the optional-content stack; `MP`/`DP`/`BX`/`EX`/`i` are matched and deliberately ignored. `BI` draws its image as of the eleventh session; before it, the row was the standing example that an operator having an arm is not the same as its being implemented. |
| Filters | **9 of 10** standard filters decode: `ASCIIHex`, `ASCII85`, `Flate`, `RunLength`, `Crypt` (pass-through), plus `DCTDecode`, `JBIG2Decode`, `JPXDecode` and — since the twelfth session — **`CCITTFaxDecode`**. `LZWDecode` is the only one **absent**, and no corpus first page reaches one. Table 92's abbreviations are expanded in `inline_image.rs`, so the filter layer sees full names; `/CCF` reaches `image.rs` as `CCITTFaxDecode`. |
| Colour spaces | **11 of 11** families, and the three CIE-based ones are converted rather than approximated. |
| Function types | **4 of 4** (sampled, exponential, stitching, `PostScript` calculator). |
| Shading types | **7 of 7**, on both backends. |
| Pattern types | **2 of 2** (tiling and shading). |
| Blend modes | **16 of 16**. |
| Font programs | TrueType, CFF, CFF-in-OpenType, CID-keyed CFF, and Type 3 — whose glyphs are content streams and are run by `pdf-model` rather than read by `pdf-font`. Bare Type1 is reported. (This row claimed Type 3 was reported for two sessions in which it was not — the corpus is the only thing that checks this file.) |
| Annotation appearances | Placed and drawn; not synthesised where absent. |
| Line dash patterns | §8.4.3.6, from the tenth session. Before it the `d` operator set nothing and every dashed line in every document was drawn solid. |
| Text rendering modes | **8 of 8** in §9.3.6 Table 104, from the thirteenth session (ADR 0022): fill, stroke in user space, both per glyph, invisible, and the four that add the glyphs to the clipping path at `ET`. An operand outside 0..7 is reported. |
| Text state parameters | 8 of Table 102's 9. Missing: `Tk`, text knockout (§9.3.8) — read from `/TK` and *reported* where it can show since the fourteenth session, and a corner of the transparency gap. |
| Optional content | §8.11 wherever it decides what is drawn: configuration, membership, `/VE`, intent, and all three places `/OC` can appear. The interactive half — `/Usage`, `/AS`, `/Order` — is not read. |
| Inline images | §8.9.7 in full, from the eleventh session: both abbreviation tables, the resource-named colour space, and three ways of finding where the data ends. ADR 0019. |
| Image colour spaces | All eleven families unpack, `Indexed` through a table converted once per entry rather than once per sample. Bit depths 1 and 8; 2, 4 and 16 are refused and reported. |
| Image masking | 3 of §8.9.6's 4 mechanisms, plus §11.6.5.2's `/SMask`: the image's own `/ImageMask` stencil, an explicit `/Mask` (§8.9.6.3) and a colour-key `/Mask` (§8.9.6.4), the last two from the fourteenth session (ADR 0023). The fourth is the graphics state's own soft mask, reported on 28 documents. |
| Page rotation | §7.7.3.3 Table 31's `/Rotate`, clockwise as displayed, from the twelfth session. Before it 90 and 270 were exchanged and every rotated page was drawn 180° out. |

## What to do next

**Two tracks now, and the discipline is to take from both in every session.**

*Demand-driven* is everything the corpus and the oracle name — 104 contradicted pages, 64 of
them unexplained, and a feature list sized by how many documents want each item. It has been
productive for twelve sessions, it is where the low-hanging fruit is, and it stays.

*Spec-driven* is what the ledger and §6.3.2.2's ranking name — coverage against the
specification rather than against a file set. It exists now, and it has a number: **668 of
823 subclauses are `unreviewed`**. A project running only the first track finishes when the
corpus goes quiet, which can happen with a great deal of the standard unimplemented and
nothing anywhere able to say which parts.

Taking from both is a `CLAUDE.md` principle-5 rule now, not a suggestion in this file. In
practice: **one item from each track per session**, and the spec-driven item is usually the
smaller of the two, because reviewing a clause family against code that already exists is
cheaper than writing a feature. The ninth session did §8.11 as both at once — it was first by
clause 6 and seventh by corpus count — and that is the ideal shape when it is available.

Sessions ten to fourteen took the other good shape, and it is cheaper to arrange: take the
demand item, then review the clause family the code you just wrote *cites*. Type 3 fonts made
§9.6.4, §9.6.5 and §8.6.8 the obvious families; inline images made the whole of §8.9, and
reading it produced four defects that had nothing to do with inline images and one wrong
citation that named a real clause; `CCITTFaxDecode` made §7.4.6, and reading it decided two
refusals and one documented choice before a line was written; the text rendering modes made
§9.3 and §9.4, and reading those produced a defect in word spacing that no page of Latin text
could have shown and the ledger's third `silent` row; `/Mask` made §11.6.4, and reading it
produced a precedence rule — an image's `/SMask` supersedes its `/Mask` — that neither §8.9.6
nor Table 87 states and that the implementation had just got wrong.

**A third thing is worth taking from the twelfth and thirteenth sessions, and it is not on
either track: the instrument.** 95% of the oracle's cost was three other programs answering a
question they had already answered, and nobody had looked because 85 seconds is not obviously
wrong. The thirteenth found the citation checker blind to table numbers, and one wrong. The
tree was also not `clippy` clean while this file said it was. **Whatever this file asserts
about the tooling, run it once before believing it.**

The one-line version of the demand track: **104 pages we claim to draw are contradicted, 64 of
them for no reason visible on the page. The two largest gaps of any kind are text — 100
documents naming a CID encoding or an embedded `CMap` — and synthesised annotation appearances
at 63; the best-argued small ones are image reduction, which renders a page of Type 3 fax
glyphs illegible, and a soft mask of a different size, which the fourteenth session's grid
choice would close on 13 of its 14 reports.** The one-line version of the spec track:
**23 clauses the code already cites have never been read against it**, and they are named in
`REVIEW_OWED`.

### 0. The ledger, and the cheapest reviews available

The machinery is built (ADR 0016). What it needs now is use, and the first rows to fill are
the ones the code already points at.

- **Work `REVIEW_OWED` down.** 23 clauses, each already cited by the code that implements it,
  so the reading is against something that exists rather than against a blank. Take them by
  family — §8.6.5 is five of them, §12.5 another five — because that is how the standard
  distributes its requirements, and because §9.6.5.4 was missed for the opposite reason:
  nobody had read §9.6.5 as a unit. **Expect findings**: eight families have now been reviewed
  and they have produced sixteen, four of them in §8.9, three in one clause (§8.6.8) that had
  looked like a formality, two in §7.4.6 that turned into refusals rather than code, and two in
  §9.3 — a defect and a `silent` row.
- **Prefer the family belonging to whatever else the session is doing.** §7.4.6, §8.6.4.2,
  §8.6.8, §8.9 (all of it), §9.3, §9.4, §9.6.4 and §9.6.5 are done; §11.4 if transparency
  groups are the demand item, §12.5.6 if synthesised appearances are, §7.6 if encryption is.
  Record every row, including the ones that turn out to be `inapplicable` — a clause read and
  dismissed is worth as much as one implemented, and costs a minute.
- **Two `silent` rows are open**, and they are the most valuable kind. §11.4.6 (knockout
  groups) and §8.11.4.4 (usage dictionaries) are drawn wrong with nothing said. Either
  implementing them or making them *report* is progress; the second is much cheaper and is
  what principle 3 actually requires. The third, §9.3.8, was closed in the fourteenth session
  and is worth reading before taking either — the cost of the report turned out to be not the
  key lookup this file predicted but the *precision*, because `Tk`'s initial value is the
  unimplemented one and a report has to name the pages where that can show rather than every
  page there is. A fourth silence hides *inside* a `partial` row — §8.9.5.2's general
  `/Decode` array — which is worth remembering when reading the ledger by status: a clause can
  be half implemented and quiet about the other half.

Four small items, listed before the big lists because they are small. The first is the
twelfth session's leftover; the other three have been carried since the seventh:

- **An area-averaging filter for reduced images**, in both backends. See the row in the
  not-implemented table: this stopped being cosmetic when a page of Type 3 glyphs drawn as
  eleven-times-reduced CCITT bitmaps came out unreadable. It wants a benchmark first, and it
  must land on the CPU and GPU backends together or the agreement scenes will separate them.
- **Sandbox the interpreter and rasteriser too.** Spike D exists and is exercised; the rest
  of the renderer still runs in the main process, which is the half of principle 3 that is
  not yet built. The protocol would have to carry a display list rather than an image, which
  is a real design question and the reason it has not been a footnote to any session so far.
- **Profile the median page.** We are 1.66× slower than `hayro` on the typical corpus page
  and nobody has looked at why — the seventh session's two fixes were both to outliers and
  moved the median not at all. The typical page is small and text-heavy, so the candidates are
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

`CONTRADICTED_UNEXPLAINED` in `oracle.rs`: 64 pages carrying no undrawn annotation, no hidden
optional content and no substituted font, so the difference is in something we believe we
implement. Two causes are identified and live — and **read trap 9 before starting**, because
an entry may be either of its shapes: two references that are one implementation, or two that
have both skipped the same clause. The second was found in the ninth session and is the more
common one; checking it costs a web search of the other project's source.

- **`knockout_*.pdf` are knockout transparency groups** (§11.4.6), where an object
  composites against the group's initial backdrop rather than against what is already there.
  `mutool` and `gs` show no blend where two rectangles overlap; we and `poppler` show it.
  Unimplemented and, unlike soft masks, unreported.
- **`mesh_shading_empty.pdf` draws the same mesh displaced horizontally** — a placement
  question, and the class of defect trap 2 is about.

Two entries that used to be here are the argument for spending the hour, because neither was
one page's problem. `issue20504.pdf` was worth **15 of the 81**: it looked like one page's
`/Differences` quirk and was a whole subclause (ADR 0015). `close-path-bug.pdf` looked like
one page's closed path and was **every dashed line in every document** — the `d` operator set
nothing at all, and both backends had been able to dash from the start. The only way to find
out which kind an entry is, is to open the artefact.

The other 62 are unexamined. Each is a page where two implementations sharing no code agree
and we differ by more than twice their own disagreement, with the artefacts already written:
`<target>/tmp/oracle/<stem>/p<n>/` holds our render, each reference's, a side-by-side and a
difference heatmap. **Look at the side-by-side first.**

Two cautions. A page may be contradicted for a reason other than the one its group names —
`calgray.pdf` sat under substituted fonts and differed in its colour, which is how the whole
of ADR 0012 started. And principle 5 is not suspended by a list: each entry is a question to
take to the specification, and "make it match mupdf" is exactly the failure this project
forbids.

### 2. Image reduction, carried from the twelfth session

Nobody took it in the thirteenth, which took the text rendering modes instead. It stays here
and it stays the best-argued small item on the list.

`tiny-skia`'s bilinear filter samples four neighbours whatever the reduction, so an image
drawn much smaller than its samples loses most of them. That was a 0.02 miss on
`firefox_logo.pdf` for four sessions and looked cosmetic. `bug1001080.pdf` shows the other end
of it: its text is a Type 3 font whose every glyph is an inline CCITT image mask, 39x53 samples
drawn at about five device pixels, and the crossbar of a `t` is one row in fifty-three. Four
renderers draw `pint test`; we draw `pinL LesL`.

The fix is a filter that averages over the area a destination pixel covers — a mip chain, or
box-filtering the source down to roughly the device size before handing it to the rasteriser.
Three things to get right: it belongs in both backends or the headless GPU scenes will
separate them (trap 2's other half), it wants an instruction-count benchmark before and after
because it is on the image path of every page, and the bound to prove it against is the oracle
rather than an eye — both pages are named in `CONTRADICTED_IMAGE_RESAMPLING`.

### 3. The three gaps that draw a page visibly wrong

The text clipping modes used to head this list and landed in the thirteenth session (§9.3.6,
ADR 0022); image `/Mask` headed it after them and landed in the fourteenth (§8.9.6, ADR 0023).
Both are worth noting because the estimates here were right about the mechanism — the second
one said the colour-key form "must be applied to the *source* samples before colour
conversion, which is why it is not a two-line change", and that is exactly where the work went.
Writing one down before taking it is what makes that possible.

All three below are loud except one — they report, and most have a test that will fail when the
gap is closed. The `/Decode` one is the one to take if a silence is what you are after:

- **A general `/Decode` array** (§8.9.5.2), and this is the silent one. Only the
  fully-inverted form `[1 0]` is applied; any other linear map is ignored **and not
  reported**. The ledger records it inside a `partial` row rather than as a `silent` one,
  because the clause's defaults *are* implemented — which is a limit of a one-word status
  worth knowing about when hunting for silence. The formula is two multiplications per
  component; the reason it has not been written is that the device fast paths unpack `u8`
  without touching floating point, so applying it everywhere would cost the hot loop. A
  `Decode` that is neither the default nor the inversion is rare enough that reporting it
  would be a good first move.
- **`/UserUnit`** (§7.7.3.3), which scales the page. 2 corpus documents, and the only reason
  it matters more than that count suggests is that getting a page's *size* wrong invalidates
  every comparison on it.
- **A soft mask whose sample grid is not its image's** (§11.6.5.2 Table 143). 3 corpus
  documents, reported since the eleventh session, and `issue16263.pdf` draws black bars across
  a page of text because of it. **This was the design question of the four and the fourteenth
  session answered it for the other key**: `image::apply_explicit_mask` combines a `/Mask` and
  its image on the finer of the two grids, bounded at 2^24 samples, and 13 of this row's 14
  reports are pairs that bound admits. What is genuinely still open is the 604 MB one — a 2x2
  image with a 34862x4332 mask, which stays reported either way — and whether nearest-neighbour
  is the right sampling for a mask carrying continuous values rather than two. The answer the
  clause actually describes is compositing at *device* resolution, which means the display list
  carrying an image and its mask separately and both backends sampling them; that is a
  `pdf-render` change and belongs to whoever takes transparency groups, since a group's soft
  mask has the same shape. Taking the cheap half first is defensible and should be measured on
  the oracle rather than assumed to help.

### 4. Synthesised annotation appearances, if the corpus count is the argument

63 documents carry an annotation with no `/AP`, second only to the 100 that report a font.
It is genuinely a different drawing routine per subtype and should not be started as one
task. If it is taken, take it one subtype at a time in corpus order: `Widget` (26), `Link`
(18, and its whole appearance is a border — §12.5.6.5 with §12.5.4), then the markup
annotations. Each one that lands should be measured on the oracle rather than assumed to
help, because a synthesised appearance is a *guess at what the producer meant* and the
references guess differently.

### 5. Then, by what the corpus says real documents need

**Soft masks and transparency groups** (29 documents report a soft mask, and it is the last
thing `doc/` reports; a group itself reports nothing),
**encryption** (20 documents — 11 cannot reach page one, 9 more draw a blank page and now
say so), and **CID encodings** (100 fonts; note that only 6 of those need the predefined
`CMap` data with its licensing question, and 3 need vertical writing — the rest need code).
**Type1 fonts** are smaller than they look: no corpus page one reaches one.

All three announce themselves — with one exception worth knowing about, since it is the kind
this file keeps warning about: a transparency *group* is drawn as an ordinary form `XObject`
and says nothing, and what reports on those 29 documents is the soft mask beside it. Otherwise
they sit below the items above because a gap that reports is a gap you can measure and
schedule, and a gap that does not is a gap that ships.

### Where the time went, and where it still goes

**There is one fair thing to measure against.** Every other renderer here is C, so a timing
difference against `poppler` confounds the language, the allocator and thirty years of tuning.
`hayro` is Rust, forbids unsafe as we do, and rasterises on the CPU single-threaded as we do.
`cargo run --release -p hayro-compare --bin hayro-speed -- <files>` renders page one of each
file with both, alternating, best of N passes.

Over the corpus pages we claim to draw completely:

| | |
|---|---|
| total, ours | **5.4 s** against `hayro`'s 40.1 s, over 685 pages |
| **median page** | **1.66× slower** |
| worst page | 31× (was 34×, was 225×) |

**The totals and the median answer different questions, and only quoting both is honest.** In
aggregate we are 7.4× faster because their distribution has a long tail and ours no longer
does. On the median page we are still 1.66× slower and **that number has never moved** — it
was 1.61×, then 1.62×, and the eleventh session's image work moved it by less than the noise
between runs. The thirteenth session did not re-run `hayro-speed`; what it did measure is that
the text rendering modes cost **+0.46% of interpretation instructions** (1.912 G → 1.921 G by
callgrind on `examples/callgrind_interpret.rs`), against a `cargo bench` that claimed 8% on one
case and was wrong. The fourteenth measured the same example at **1.923 G** — the masking work
adds one `Option` check per sample to a loop that had none, and it costs +0.12%, which is below
the run-to-run noise of ordinary code motion; the corpus gate is unchanged at 1.4 s.
The seventh session's two fixes were both to outliers. **The typical corpus page
is small and text-heavy and has never been profiled**, which is the next measurement rather
than the next optimisation.

Two fixes are worth carrying as patterns rather than as history. Unpacking JPEG output was
6.89 G instructions on one page — nearly twice what `zune-jpeg` spent decoding it — because a
`match`, three bounds-checked `get`s, saturating arithmetic and a re-checked `extend_from_slice`
all ran *per pixel*; two paired `chunks_exact` iterators took it to 1.25 G. **The safety
habits this project enforces everywhere are expensive in a loop that runs per pixel, and that
is exactly where the profile should be consulted rather than the habit.** And a mesh triangle
was subdivided by colour alone, so one covering a tenth of a pixel still split into 4096
filled pieces; `Triangle::is_subpixel` is a correctness statement rather than a trade — a
triangle smaller than a sample cannot display a gradient — and it took `personwithdog.pdf`
from 17.3 s to 1.06 s **while moving every mesh page closer to the references**. A change made
for speed that improves fidelity means the old code was doing work that was worse than
useless.

**Still open, and the largest items.** This profile predates both fixes and its shading half
is still live:

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
shadings** rather than drawing them: a function is parsed and then sampled 256 times per
shading, and that page has 3576 of them. Whether that is 3576 *distinct* functions or one
re-parsed 3576 times has never been checked, and it decides whether the fix is memoisation by
object reference or something harder — check before designing.

One caution: `to_rgb_at` was 2.6% when `CalGray` was a pass-through. It now runs a Bradford
adaptation and a matrix per colour, and per *sample* for a Cal-space image.

### Reproducing the numbers above

The oracle survey is `oracle.rs` and the corpus counts are `corpus.rs`; both print their
evidence per document. The ledger's counts come from `cargo test -p conformance -- --nocapture`.

Two classification counts are still throwaway, and deliberately so — scratch-quality
diagnostics do not belong in a repository held to `clippy::pedantic`. **Whether a page's fonts
are embedded** walks each `/Font` resource and its `/DescendantFonts` for `/FontFile`,
`/FontFile2` or `/FontFile3`. **The annotation subtype breakdown** comes free from the corpus
gate's own output: `grep -o '"[A-Za-z]*: no appearance stream"' | sort | uniq -c`.

### What the corpus gate reports today

Ratcheted in `crates/pdf-model/tests/corpus.rs`; the numbers only ever go down, except where
a rise is a new report and is written down as one.

| | count | |
|---|---|---|
| unopenable | 0 | and it should stay there |
| no page one | 19 | 11 encrypted, 8 with unrecoverable page trees |
| draws incompletely | 232 | Counted by each document's *first* report, so the column sums: 100 a font, 66 an annotation, 28 a shading, 15 an operator, 13 an image, 7 an undecodable content stream, 2 a text knockout, 1 a bound reached |
| slower than 30 s | 0 | `KNOWN_SLOW` is empty, and the next document to cross the budget fails the gate |

**The operator row was 33** until the thirteenth session implemented §9.3.6's eight text
rendering modes, and is 15. Nothing left on it is a feature anybody could implement: it is
`BT` without `ET`, `BDC` without `EMC`, and the byte soup a fuzzed content stream lexes as
operator names.

The image row was 161 before JBIG2 and JPEG 2000 landed, 53 before inline images did, 31
before `CCITTFaxDecode` did, 19 before `/Mask` applied, and is **13** now. What is left,
counted per image rather than per document: 14 soft masks whose grid is not their image's,
4 malformed streams, 3 bit depths the unpacker refuses, one `/Mask` that is not an image mask,
and two files the decoders refuse — one JBIG2 using a segment type ISO/IEC 14492 does not
define, and one 212-megapixel JPEG 2000 scan. Nothing on this row is a *missing codec* any
more, and the largest thing on it is one document: `issue16263.pdf` writes 13 of the 14 soft
masks.

The font row is unchanged by the eleventh session and is now the largest: counted as *fonts*
rather than documents because a page may name several, 27 with no `/ToUnicode` so a substitute
cannot be addressed, 26 with a non-identity `/CIDToGIDMap`, 23 whose substitute draws none of
their declared codes, 14 with an embedded `CMap` stream, 6 with a predefined `CMap`, 3 asking
for vertical writing.

### What the oracle gate reports today

Ratcheted in `crates/pdf-model/tests/oracle.rs`, by name and in both directions.

| of the 1512 pages we call complete | count | |
|---|---|---|
| agree with the reference consensus | 672 | |
| **contradicted** | **104** | 8 page rounding, 7 a shared JBIG2 decoder and 1 a shared *gap* (trap 9, both halves), **2 image reduction**, 1 a sub-pixel image, 1 a `CalRGB` alternate, 1 glyphs judged as vector, 19 substituted fonts, **64 unexplained** |
| ambiguous | 723 | the references disagree with each other; 372 of them are two long books set in fonts nobody embedded |
| our page geometry differs | 3 | 2 are `/UserUnit`, 1 unexamined |
| not comparable | 8 | fewer than two references produced an image, or they disagree on the page size |

The 282 incomplete pages are compared and printed too, but cannot fail the gate: a page we
already say we cannot draw is expected to differ, and listing hundreds of them would drown
the signal. It fell by 4 in the fourteenth session — seven pages gained as `/Mask` began to
apply, three lost as §9.3.8 began to report, and trap 11 is about the second half of that
trade. It fell by 16 in the thirteenth session as the text rendering modes landed, by 14
in the twelfth as fax-encoded images started drawing, by 42
in the eleventh as inline and `Indexed` images did, and by 10 in the tenth as Type 3 pages did — every one of those pages moved *into*
the gated set, which is why the contradicted count rose while nothing got worse. In the eighth
it rose by 43 and the gated total fell by the same, which is the cost of honesty: a page that
starts reporting stops being watched by *this* gate. It is the reason a report should never be
reached for as a way of making a contradiction go away, and the reason
`CONTRADICTED_SUBSTITUTED_FONT` now records which of its departures were fixes and which were
exits.

**Where its time goes, measured and printed by the gate itself.** It used to be roughly
1000–1300 s of processor time in the three external renderers against 45–55 s in ours, for 75 s
of wall clock on 24 cores — a ratio above 20:1, so **the gate was essentially a measurement of
`pdftoppm`, `mutool` and `gs`**. That was the twelfth session's first item and ADR 0020 is the
answer: the references' renders are remembered, keyed on the invocation itself, and the run is
**25 s with 17 s left in the three renderers**. Every verdict is unchanged, which was checked
by running the whole corpus both ways and comparing the counts.

What is left is ours. Roughly 600 s of processor time over 24 cores goes on our own render,
the comparison and the artefacts — the SSIM and heatmaps for the thousand pages that are not
agreement — so if 25 s ever becomes the constraint, that is where to look, and not at the
subprocesses. The cache prints its hit rate for the same reason the versions are printed: a
run over an unchanged tree that reports less than 99% hits means the corpus or a renderer
moved, and it is better to see that than to infer it from the clock.

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

**One dictionary, two clauses, and only the second one says who wins.** §8.9.6 defines what an
image's `/Mask` means, in two forms, over two pages. What it does not say — and what Table 87
does not say either — is that an `/SMask` beside it "shall override any explicit or colour key
mask". That sentence is in §11.6.4.3, in the transparency clause, and an implementation that
reads only clause 8 is complete by its own lights and wrong on any file writing both. It was
found because the session's spec-track item was the family its *demand* item cited from the
other direction. The general form: when a key appears in more than one clause's index entry,
the clause that owns the feature is rarely the one that states the precedence.

**When the standard defines nothing, a plausible reading can still be untenable — and the page
is what says so.** `issue6621.pdf`'s `/Mask` is a one-bit greyscale image where Table 87 and
§8.9.6.2 both require an image mask. There is exactly one reading its samples admit under the
clause cited, §8.9.6.2's "a sample value of 0 shall mark the page", and applying it blanked a
court seal three renderers draw, because on that page the zero samples are the background. The
alternative reading is §11.6.5.2's luminosity-as-opacity, which is a different clause about a
different key and would invert every stencil whose author forgot `/ImageMask`. So: neither, and
the entry is named. **Refusing is a result, not a failure to decide** — it is what principle 5
asks for where the specification is silent and the choices are opposite.

**A report has a price, and it is paid in gated pages.** §9.3.8's first check named 7 documents
and moved three *agreeing* pages out of the oracle's comparison, for a difference that could
not have appeared on any of them. The second check named 2. Nothing about the gap changed —
only whether the condition matched the clause. Trap 11 has the full form; the habit is to ask,
before adding a report, "on how many pages can this actually be seen?" and to make the
condition answer that question rather than a looser one.

**Build the strong gate, then let its own output tell you it is wrong.** The thirteenth
session found the tree citing "§9.3.6 Table 106" for the text rendering modes — Table 104 —
in four comments, two tests and a written report, and wrote the obvious checker: a table
reference must name a table the clause beside it discusses. It failed fourteen of the tree's
twenty-five references and **all fourteen were correct writing**, because a comment about one
clause routinely names a table belonging to another. Behind an exception list it would have
been a gate that is mostly exceptions, which teaches a reader to ignore it. What shipped
instead asserts the weaker true thing and *prints* the title of every table the tree cites, in
which the wrong pairing is obvious at a glance. **A check whose false positives are all correct
code is measuring the wrong property, not finding a backlog.**

**A rule about how something is *encoded*, implemented as a rule about its value, is invisible
forever.** §9.3.3 applies word spacing to "the single-byte character code 32" and says in its
next sentence that it does not apply to a byte 32 inside a multiple-byte code. This tree
applied it to any code numerically equal to 32, so every `Identity-H` string containing the
bytes `00 20` had the rest of its line pushed right. No corpus page could show it, no metric
could see it, and it had been wrong since composite fonts landed. It took five minutes of
reading the clause. The same shape as §9.6.5.4: self-consistent code, right about every
document anyone had opened.

**Wall-clock lied again, in the same direction, and callgrind settled it in two runs.**
`cargo bench` reported an 8% regression on one interpretation benchmark from a change that
adds three branches per glyph; `valgrind --tool=callgrind` over
`examples/callgrind_interpret.rs` put it at 1.912 G instructions before and 1.921 G after —
**+0.46%**. This file has said "count instructions" since the seventh session and it was still
tempting to believe the 8%. Run the two callgrinds; they take a minute each.

**"Clippy clean" is a claim, and this file made it while eleven warnings sat in the tree.**
Every one came from the twelfth session's own new files, and the reason is worth knowing:
`allow-panic-in-tests` in `clippy.toml` covers `#[test]` bodies, and **not** the helper
functions of an integration test — so a `panic!` in a `fn` that a test calls is a warning
while the identical line inside the test is not. Whatever this file asserts about the tooling,
run it once before believing it.

**Measure the instrument before deciding you are slow.** Eleven sessions treated the oracle's
85 seconds as the price of having an oracle. It was 95% three other programs answering a
question they had answered the day before, and the fix took an afternoon and made every
subsequent change three times cheaper to check. The general form: when a loop is slow enough
to change *what you attempt*, that loop is a thing to measure, not a constraint to design
around. The tempting alternative here — batch ten features, then work out which files moved —
would have traded the gate's completeness for speed the gate did not actually need.

**And when the first design of a fix is the obviously safe one, still measure it.** Refusing
to cache timeouts is unarguable in principle: a timeout is a fact about the machine, not about
the document. With everything else cached it left **two pages out of 1794 accounting for 46 of
the run's 57 seconds**. The rule that replaced it remembers them for a week, prints how many
it used, and has its cost and its expiry written down — which is what principle 1 asks of a
shortcut, as against taking one silently or refusing to take one at all.

**A test that skips silently is worse than no test.** `tests/ccitt.rs` was first written
against `bug1001080.pdf`, whose fax data turned out to be inside a Type 3 font's glyph
descriptions rather than in an image `XObject`. The helper returned `None`, both tests printed
"skipped: the submodule is not checked out", and both passed while checking nothing. A missing
corpus is a skip; a present corpus that does not contain what the test needs is a **panic**.
The two are one `?` apart and they look identical in the output.

**A citation nothing checks is a citation that rots — so now something checks them.** The
tree carried 146 references to ISO 32000-2 and the first tooling ever pointed at them found
two clause numbers that name nothing and three of five sampled quotations that are
paraphrases inside quotation marks. Nothing had gone wrong yet — the same condition under
which the duplicated D50 matrix was cheap to fix. A reference decays at the rate of the
attention paid to it, and in this project the attention is spent on pixels, correctly. The
checker is `tools/conformance`, it runs with the workspace's tests, and the thing to carry is
that **it kept finding errors after the obvious ones were fixed**: the corrected `/Mask`
citations were still wrong, because §8.9.6.2 is stencil masking and `/Mask` naming another
image is §8.9.6.3, and no amount of checking numbers against an index would have caught that.
Only reading the clause did.

**A fallback that fills the page is worse than one that leaves it blank.** §9.6.5.4's
predecessor ended in "if nothing else matched, the code is the glyph index", per code, and
`issue5501.pdf` drew `v 0' ' W` for `What's an interval?` — confident, plausible, wrong and
silent. The same fallback survives, restricted to a font with no readable `cmap` at all, and
the oracle proves the restriction is load-bearing: put it back per-code and `issue17333.pdf`
is contradicted immediately. Prefer the blank that reports.

**Fixing the mask shows what the mask was hiding, so budget for it.** §9.6.5.4 had nothing to
do with Type 3 fonts or with substitutes that reach none of a document's codes, and both
became visible in the afternoon spent looking at what fonts actually did — 24 documents and
19 against the one that led there. The same shape recurred in the ninth session: reading
§8.9.6 for the ledger produced a wrong citation, a missing test and an unimplemented sentence,
none of them the thing being looked for. The fix a defect leads to is often not the fix it
names.

**A dependency is a decision, and this project's own precedent decides it.** `zune-jpeg` owns
`DCTDecode`, `skrifa` owns font parsing, `flate2` owns Flate, `tiny-skia` owns rasterisation,
and `hayro-jbig2`/`hayro-jpeg2000` own the two hardest image codecs. Writing 19 400 lines of
MQ coding and symbol dictionaries here would have been consistent with none of that. The cost
is written down rather than assumed away: two decoders we cannot fix ourselves. See ADR 0014,
and `deny.toml` for what the tree refuses outright.

**A corpus can state an invariant about itself, and that beats any reference.** 96 corpus
documents encode one image ninety-six ways, and demanding they agree needs no external
renderer, so principle 5 is not even in tension — the expectation comes from the documents.
It is also more sensitive than any tolerance, and it is the only thing that could settle a
page where two of the three references are secretly the same decoder. Look for that shape: a
corpus varying one thing while holding another fixed is stating a testable invariant.

**A clause read and dismissed is worth as much as one implemented.** The ledger's statuses
include `inapplicable` and `writer-side` for exactly that, and filling one costs a minute
against the 20 to 60 a real review costs. The trap is treating the ledger as a to-do list of
features; it is a record of questions asked, and "asked, and it does not apply to a screen"
is a complete answer.

**An unimplemented feature has a default, and the default is usually "draw it".** That is why
two unrelated renderers can agree about a page while both being wrong, and it is a much more
common failure of the oracle's premise than shared code is. When a contradiction looks like
"everyone disagrees with us", the cheap next step is not to re-read our own code: it is to
search the other projects' source for the clause. `mupdf`'s `FIXME: Calculate visibility from
array` and `ghostscript`'s `WARNING: OCMD contains VE, which is not supported (ignoring)` took
minutes to find and settled a page that had looked like three-against-one.

**Two references against two is not a tie, and not a vote — it is a question with an answer.**
`Type3WordSpacing.pdf` splits them: `poppler` and `ghostscript` paint a `d1` glyph's stroke in
the fill colour, `mupdf` in the stroking colour, and we were with `mupdf`. There is no rule
that says "go with the majority", and there did not need to be — Table 111 says "its colour"
in the singular, says the description "is executed solely to determine the glyph's shape", and
explains that an image mask is admissible inside one because a mask "merely defines a region
of the page to be painted with the current colour". One region, one colour, and the two
renderers that agreed with the clause agreed with it for that reason rather than by weight.
The habit to keep is the order: read the clause first, and let the split tell you *where to
look* rather than what to conclude.

**What you measure decides what you build, so check what the measurement cannot see.** Eight
sessions were steered by two gates that both take the pdf.js corpus as their universe. They
are good gates and the work they directed was real. But the ordering they produce is a
demand curve — features ranked by how many of 974 files want them — and a demand curve
cannot rank a requirement no file exercises, cannot notice a clause nobody implemented, and
converges on "done" the moment the last file goes green. §6.3.2.2 ranks optional content
first among what is left; the corpus ranks it seventh, tied with three other five-document
items. Neither ordering is wrong; running only one of them is.

**A gap inside a feature you have implemented does not announce itself.** Every missing
subsystem in this tree reports — `LZWDecode`, JBIG2, encryption — because somebody wrote the
report while deciding not to write the feature. The gaps that ship are the ones *inside*
something implemented: `Tr` was parsed and three of its eight modes reported, `/SMask` was
honoured while `/Mask` beside it was not, `CalGray` was resolved and then converted as
`DeviceGray`, and §9.6.5.4's whole algorithm was one line that worked on Latin text. Reading
the specification asking "what have we not built" cannot find those, because the answer is
"nothing". Comparing output against another implementation can, and has, nine times now.

**The purest instance of that is the `d` operator, and it is worth keeping as the archetype.**
Every layer of the dash feature existed: `pdf-render`'s `Stroke` carried an array and a phase,
`render-cpu` built a `tiny-skia` `StrokeDash` from them, `render-gpu` handed Vello a dash
iterator, and `set_dash` had a doc comment. The one line that mattered read only the *empty*
array, because the content lexer flattens an operator's array operand and nobody had written
the case for a non-empty one. Result: an implemented feature, two agreeing backends, four
crates that all look right — and not one dashed line in 974 documents. When a feature looks
finished, check the operand path from the content stream to the state, because that seam is
the one nothing else in this tree looks at.

**A subclause is a checklist; check the code against it, not the code against itself.**
§9.6.5.4 is two pages and names five distinct routes from a code to a glyph. The code that
stood in for it implemented roughly one and a half of them, and no reading of *that code*
would have said so, because it was self-consistent, commented, and right about the documents
anyone had opened. The cheap move that was never made is: open the clause, list its rules,
and ask of each one where it is. It took five minutes and was worth 15 contradicted pages
and two unrelated silences. Do it for §11.4 and §12.5.6 before implementing either.

**A gate's numerator moves when its denominator does, and only one of those is news.** The
oracle judges pages we claim to draw completely, so a feature that makes 42 pages drawable
adds 42 pages to the set being judged — and four of them came out contradicted, none of them
by getting worse. The eleventh session's contradicted count rose from 104 to 108 in exactly
that way. A ratchet that only ever counts failures will read this as a regression; the fix is
the one this file already uses for the corpus, which is to say *which* pages moved and why,
and to keep the denominator beside the numerator wherever the count is quoted.

**When a report replaces wrong output, the reported count is the wrong scoreboard.** Two of
the eighth session's three changes made the corpus's incomplete count *rise*, by 43 documents, and
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
- **`doc/md/` is the specification, in a form code can read.** It holds Markdown conversions
  of the 14 specification PDFs in `doc/`, with real tables, and it is committed — so a test
  may depend on it without a skip path, unlike the pdf.js submodule. `ISO_32000-2_sponsored_EC3.md`
  is 24 MB and its 860 `##` headings give a clause number, a title and a line range apiece,
  which is the whole basis of the citation checker and the conformance ledger (`PLAN.md`
  §5a). Two caveats for whoever builds those: it is a *conversion*, so a quotation the
  checker cannot find may be a conversion artefact rather than a bad quote — check `doc/`'s
  PDF before editing the comment — and one heading number (`14.8.4.7.3`) occurs twice.

  When you need spec data — encoding tables, operator lists, value constraints — extract it from
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
