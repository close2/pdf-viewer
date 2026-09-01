# Traps: the oracle, and the other implementations

Status: **standing** — each is a mistake somebody actually made in this tree.
Read by: a round that reads the oracle's verdicts, diagnoses a contradicted or ambiguous page,
invokes another renderer, or moves a tolerance. `doc/oracle-and-corpus.md` is the instrument
itself; this file is what it does wrong.

**Principle 5 is over all of it**: another implementation is evidence about our reading of the
specification, never the definition of correct.

`doc/HANDOVER.md` is the index and names which group holds which trap. **Every trap keeps its
number**, because `crates/`, `tools/`, `doc/conformance/ledger.toml` and dozens of ADRs cite them
by number and an ADR is not edited to follow a file that moved underneath it (ADR 0232 §2).

## Traps

### 3. An oracle is only as good as how it invokes the other renderers

The first run reported 54 documents whose page *size* we disagreed about. `pdftoppm` and `gs`
default to the **media box**; `mutool` and we use the **crop box**. Every invocation is now
explicit about the page box, *including* `mutool`'s, whose default was already right: a default
that silently changes is a comparison that silently changes. One level up: `gs` renders for a
**printer**, so Table 167's Print flag decides what it draws, and four link borders disagreed for
that reason alone. **Check what question each reference is being asked before reading its answer
as a verdict.**

**And the same trap on the way *out*: how a reference's answer is collected, not only how it is
asked.** Two instances, both found in the seven-hundred-and-seventh session on the six pages ADR
0542 made visible, and both of them a sentence the harness had written down somewhere else:

- **an empty file passed the test for a file.** `mutool draw` creates its `-o` file *before* it
  decides it cannot draw the page, so a document whose page tree it cannot recover leaves a
  zero-byte PNG behind. `output_path.exists()` accepted it, the PNG decoder then failed, and the
  gate printed `PNG error … unexpected end of file` — the harness's sentence, over a log holding
  the renderer's. Worse, `HarnessError::Png` is the one failure `cache` refuses to remember, and
  rightly, so those pages re-ran `mutool` on every run for ever inside a cache at a 99.8% hit rate.
- **`gs` speaks its diagnosis on stdout, and the harness sent stdout to `/dev/null`.** On a file
  with no §7.5.2 header it prints `Error: /undefined in obj` and its operand stack to stdout and
  only `Unrecoverable error, exit code 1` to stderr. `Reference::version` has carried the comment
  "`gs` prints to stdout; `pdftoppm` and `mutool` print to stderr" since it was written; nothing
  had joined it to the renderer log.

Both fixed, and the general rule is the one worth keeping: **a renderer's refusal is a message, and
a message is only as good as the stream it was read from and the file it was judged by.** The tell
is a failure sentence that names the harness's own machinery — a decoder, a path, a buffer — where
the renderer had a sentence of its own. ADR 0574.

**And the last non-empty line is the *consequence*, not the reason.** All three of these narrate,
so their logs end with `cannot draw '<path>'`, `Unrecoverable error, exit code 1` and the signal
that killed them, under first lines that name the clause the file broke. Take both ends.

**And a renderer's message is now part of a *verdict*, which puts a third demand on it.** Since
ADR 0769 a flat sheet whose renderer's own log says it could not draw the page takes no part in the
consensus — the one thing that separates a genuinely blank page from a page nobody decoded, since
their rasters are identical. Two consequences for a round that touches this machinery:

- **The words have to survive the cache.** They did not: `cache::render` returned on a hit without
  running the code that captures them, so on a run at a 99.8% hit rate the verdict came from
  rasters and the diagnosis from files an earlier run left behind. A rule reading a file that only
  a *miss* writes reaches two different verdicts on two runs of the same corpus. The log is stored
  and restored with the picture now, empty included.
- **The condition is a vocabulary and not a severity, and the difference is measured.** 28 901 of
  `poppler`'s `Syntax Error` lines over the oracle's own population are `Type mismatch in
  PostScript function`, on pages it draws correctly. What is read is what a program says it
  *produced* — `mupdf`'s `library error:`, `ghostscript`'s `FATAL ERROR`, `jbig2dec`'s `failed to
  decode` — and `poppler` is read not at all, because nothing in its wording separates a refusal
  from a defect it recovered from. `Reference::refusals` has the table; the oracle prints both what
  the condition matched and what it did not, which is trap 11's audit for a condition whose
  right-hand side three other projects own and can reword at any release.

### 9. Two references can agree because they share code — or because they share a *gap*

The oracle rests on ADR 0005: two implementations sharing no code agreeing about a page is
evidence. Nine ways for that to fail — and the count has moved five times, so read the list rather
than the number. **The final entries are not further mechanisms**: one is about a page carrying
two of them, one about how a mechanism gets *checked*, and three about how the measurement that
results is read — because a mechanism accounted for in the wrong units is not accounted for, and
because the newest of the three is about a ratio's **denominator** rather than a verdict's
numerator. And the ninth is not a mechanism either: it is a reading that was never taken.

- **A shared gap.** An unimplemented feature falls through to a *default*, so two unrelated
  programs that skipped the same clause produce the same picture. `visibility_expressions.pdf`:
  `mupdf` carries `/* FIXME: Calculate visibility from array */ return 0;` and `ghostscript`
  prints `WARNING: OCMD contains VE ... (ignoring)`, while `poppler` and pdf.js implement `/VE`
  and §8.11.2.2 is unambiguous. The page stays contradicted, with the source citations beside it.
- **Shared data.** `mupdf` and `ghostscript` disagree with us on four `DeviceCMYK` pages and agree
  with each other to under a level, because they run the same ICC profile. What settled it was
  *this tree's own* A2B evaluator pointed at `default_cmyk.icc`. **When two references agree
  suspiciously closely, ask what data they are both reading, and evaluate it yourself.** ADR 0048.
  It is byte identity rather than a family resemblance, and the six-hundred-and-fifty-sixth
  session checked: `/usr/share/ghostscript/iccprofiles/default_cmyk.icc` is 187 484 bytes at
  `md5 fd199526f0a7e0bceb294a777cd84252`, `libgs.so` embeds no profile at all and reads it off the
  disk, and scanning `libmupdf.so` for ICC headers finds **the same 187 484 bytes at the same
  digest** compiled in. Two binaries, one file. **The scan is the instrument worth keeping**:
  every ICC profile in a binary is a four-byte big-endian length followed by `acsp` at offset 36,
  so `objdump` is not needed to find what a reference is reading.
- **A shared *default*, which is neither of the two above and needed a third instrument.** On a page
  whose images run through a profile the *document* embeds, all three references sit within four
  levels of each other and up to twenty from us — and the data is not shared, because it came out of
  the file. What is shared is the engine and the argument it is called with: `objdump -p` says
  `libpoppler` and `libgs` link the same `liblcms2.so.2` and `libmupdf` defines 445 `lcms2mt_*`
  symbols of Artifex's fork, so **on an `ICCBased` page the three voting references are one colour
  library**, and `INTENT_PERCEPTUAL` is 0, which is what a caller passing nothing passes. Pointing
  this tree's own evaluator at the profile's `A2B0` reproduced `poppler` byte for byte where its
  `A2B1` is twenty levels away; ISO 32000-2 says RelativeColorimetric three times (Table 51,
  §8.6.5.8, §11.4.7) and `A2B1` is that table. **The instrument worth copying is the probe**: a
  four-object PDF holding nothing but colour patches in the space under test, rendered by all four
  programs, so the answer is a number per colour instead of a page. ADR 0456.
- **And the sentence "this is trap 9's family" is a hypothesis, not a diagnosis.** Two crawled pages
  were filed under this trap from their *dictionaries* — one all `DCTDecode` under one `ICCBased`
  space, one all `/DeviceCMYK` JPEGs — and six rounds later the second turned out to be a silent
  defect of this tree that the probe above cleared in one run: on plain `DeviceCMYK` patches we and
  `poppler` agree exactly, so whatever the page was, it was not the conversion. **What a page's
  objects are is evidence about where to look and never about who is right**; the trap costs a round
  when it is used as an explanation. ADR 0456.
- **Shared code, wider than `jbig2dec` — and wider than this entry said.** `objdump -p | grep
  NEEDED`, which is what a binary asks for rather than `ldd`'s transitive closure: all three link
  the same `libjpeg.so.8` and the same `libopenjp2.so.7`, so **on a JPEG or JPEG 2000 page the
  three voting references are one decoder**; `poppler` and `gs` share `liblcms2`; `mupdf` and `gs`
  share `jbig2dec`; `poppler` and `mupdf` share `libfreetype.so.6` and **`gs` does not** — it
  carries its own statically linked copy, 194 `FT_*` symbols defined and none undefined, and the
  `ldd` that founded this entry was reaching FreeType through `libfontconfig`. Same family, not
  the same object. Recorded on `Reference::independence` and acted on nowhere — marking all three
  `Shared` for text would leave nothing to vote.
- **And where the corpus states an invariant about itself, ask the *references* that invariant.**
  Shared code is a reason their agreement is not evidence; it is not a reason to believe ours. The
  `bitmap-*` family is one drawing encoded through nearly every path ISO/IEC 14492 defines, so every
  program owes the same picture on all of them and each can be **compared with itself**, no renderer
  treated as truth. This tree returns one image; `poppler`, `mupdf` and `ghostscript` return eight,
  six and six — and the image `jbig2dec` produces on the encodings it is self-consistent about is
  byte-identical to ours. That turns "their agreement proves nothing" into a statement about who is
  right, out of the documents alone. ADR 0381.
- **And the ambiguous bucket measures the font half of it.** Over all 786 ambiguous pages, the
  closest of the ten renderer pairs is `ours + hayro` on 651, and on 612 of the 670 text ones;
  median ours-to-`hayro` 1.94 of 255 against 5.39 for the closest two that vote. `hayro` is a
  separate interpreter that shares `skrifa` with us and is the one reference that may not vote.
  **An `ambiguous` text page is usually two camps, and the voting camp is the one that cannot
  agree with itself.** That is not evidence we are right — agreement with `hayro` never is — it
  is what the verdict is made of.
- **Two answers to two different questions.** `mupdf` constructs no link appearance while
  `ghostscript` renders for paper. Their agreement is a coincidence of two unrelated reasons.
- **And a fifth, found in the hundred-and-seventy-sixth: two references sharing a decoder can
  *disagree*, and that is worse than agreeing wrongly.** On nineteen JBIG2 refinement pages
  `jbig2dec` fails in both of them, `mupdf` renders black and `ghostscript` renders white — so
  instead of contradicting us they produce no consensus at all and the page becomes `ambiguous`,
  which nothing was watching. Shared code does not only manufacture agreement; it can also
  manufacture the *absence* of one, and the second is invisible where the first is at least
  listed. `AMBIGUOUS_SHARED_JBIG2_DECODER` — **which holds one of those nineteen since the
  six-hundred-and-eighty-first session and not because anything was fixed**: a black sheet and a
  white sheet are two rasters with no mark on either, so under the rule in the last bullet below
  both abstain and the pages are `not comparable`. That is the point of the move rather than
  bookkeeping — "invisible where the first is at least listed" was true *inside* `ambiguous`, whose
  own definition invites reading a manufactured absence as a corner of the specification, and the
  bucket that says "the gate has one reading of this page" does not. ADR 0513.
- **And a sixth, which is none of the five above: a shared external *standard*.** On
  `CONTRADICTED_DEVICE_CMYK_CONVERSION`'s five pages `hayro` sits with the `mupdf`/`ghostscript`
  pair — 4 and 5 levels of 255 from them, 48 from us — and it shares nothing with either.
  `objdump -p` on `pdfref-hayro` names `libgcc_s`, `libm` and `libc` and no colour library at all;
  what it carries is its own `CGATS001Compat-v2-micro.icc`, 8 464 bytes, `desc` `uCMY`, `cprt`
  `CC0`, one `A2B0` tag, against Artifex's 187 484 bytes and three `A2B` tables. Different file,
  different author, different licence — and this tree's own evaluator on **either** of the two
  predicts all three renderers to within eight levels while sitting 48 from ours. **That last clause
  is true of the region it was sampled in and of no other**, which the six-hundred-and-eightieth
  session found on the fifth page of the same group: on `transparent.pdf`'s single ink,
  `0.82 0.7 0.54 0.67 k`, the *Artifex* profile through our evaluator is **eleven levels** from all
  three renderers that read it, while the CGATS one is within one — a colorimetric black point
  walked off the device range against Little CMS's round trip through `B2A`, which
  `icc.rs::detect_black` already said agree "everywhere except in the darkest few percent". **A
  profile predicts a renderer over the colours somebody sampled, and that is not a property of the
  file.** ADR 0510. What the three
  share is the *press*: Artifex's `desc` says **SWOP** and CGATS TR 001 is the characterisation
  data SWOP publishes. **So implementations can agree because each independently went and got a
  copy of the same published standard**, which no dependency graph shows, no digest comparison
  finds and no shared file explains — only the profiles' own `desc` tags do. §10.3.2's NOTE is
  where all four assumptions live, ours included. ADR 0484.

- **And a seventh, which is the same agreement being evidence on one line of pixels and a shared
  departure on the next.** On `issue7891_bc1.pdf` page 1 the voting pair agrees about the soft mask
  group's `/BBox` to **0.0003 of 255** while each sits 0.062 from us, because both take a clipping
  region as §10.7.4's *set of pixels* where this tree anti-aliases it — that agreement is the clause
  and it is real evidence. One column over, at the same `/BBox`'s right edge, the same pair agrees to
  0.0001 in **dropping** a column the same sentence admits, and `poppler` is the one that keeps it.
  Nothing distinguishes the two agreements from the outside: same pair, same construction, same
  distance, opposite verdicts against the clause. **A pair that agrees because it rounds something
  the same way agrees whether or not the rounding is right**, so the unit to take back to the
  specification is the *edge*, not the page — and on this page each of the seven lines has its own
  answer. ADR 0489.

- **And an eighth, which no instrument above can find: shared data that exists on no disk, because
  each of the two *manufactures* it from the document.** On `CONTRADICTED_CALRGB_TO_SCREEN`'s five
  pages `mupdf` and `ghostscript` outvote us, and the reason is that both turn Table 63's `/CalRGB`
  dictionary into an ICC profile and hand it to Little CMS — `libgs` carries
  `gsicc_create_from_cal` among its internal names, `libmupdf` exports `fz_new_icc_data_from_cal`
  and defines 437 `lcms2mt_*` symbols of Artifex's fork — where `poppler` and this tree evaluate
  §8.6.5.3 in their own code.
  **`objdump -p`, a digest comparison and a `desc` tag all come back empty here**, because the
  shared file is built at run time out of the page's own bytes: the second bullet's instrument
  scans a binary for profiles, and this one is in neither binary. What produced it was
  `gs -sDEVICE=pdfwrite`, which writes the page back with the space replaced by the 585-byte
  `ICCBased` stream `ghostscript` synthesised — after which ADR 0048's instrument works as usual,
  and this tree pointed at that file reproduces `ghostscript`'s rendering of the *dictionary* to
  0.07 of 255 where our own path is 4.15 from it. The confirmation is the other direction: handed
  that file, `ghostscript` moves 0.03 and `mupdf` 0.83, while ours moves 4.17 and `poppler`'s 4.24.
  **The generalisation is the tell**: where two renderers agree on a page whose colour space is
  *described* rather than embedded, ask whether either of them will write the description back out
  as data. ADR 0494.

- **And a page can carry two of the eight at once, in which case the one it is *named* for need
  not be the one the gate is failing.** `visibility_expressions.pdf` is filed under the first
  bullet and correctly: `mupdf`, `ghostscript` and `hayro` all ignore §8.11.2.2's `/VE` and draw
  two sections this tree hides. It is also a page whose only colours are `0 0 0 k`, so the sixth
  bullet's shared press is on every glyph of it as well. Priced by taking each mechanism out of
  the file in turn — two §7.5.6 incremental updates, `/VE` replaced by an `/OCGs` and `/P` every
  renderer implements, then the `k` colours restated as the `rg` triples they reach here — the
  gap owns 45.42 of a failing worst tile of 50.01 and **0.037 of the 1.35 percentage points** by
  which the failing differing fraction misses its bound; the press owns 4.371 of those points and
  glyph edges 1.938. **Verified is not sufficient**: 668's question was whether a note's mechanism
  is real, and this one is, while the page would stay contradicted with it entirely removed. The
  instrument is the cheap half of ADR 0048's — *edit the document so that one mechanism cannot
  act, and re-measure* — and its control is the other renderers not moving at all (`mupdf`,
  `ghostscript` and `hayro` render the `/VE`-free variant byte for byte as they render the
  original). ADR 0497.

- **And a citation of another project's *source* is a claim with no gate on it, which decays like
  a ledger row.** The same note quoted `ghostscript`'s `WARNING: OCMD contains VE, which is not
  supported (ignoring)`; `strings` on `libgs.so.10` at 10.07.1 finds neither that sentence nor
  `not supported (ignoring)`, and the run without `-q` says nothing about optional content. The
  behaviour had not changed and the evidence for it had. Where a source citation can be replaced
  by an experiment on the installed binary, replace it: a raster is a measurement of the program
  that ran.
- **And where a voting reference's raster is *constant*, the comparison has no second operand.** A
  renderer that decoded nothing returns a sheet that is 255 in every channel, and against a constant
  every one of the gate's four numbers is a statistic of *our* render alone: the mean is exactly
  `255 × (1 − our own mean channel value)`, and the rest follow. On four of
  `CONTRADICTED_SHARED_JBIG2_DECODER`'s pages the whole verdict line — mean 13.12, worst tile 144.56,
  differing 5.15%, ssim 0.8990 — is reproduced to the digit by comparing our render with a synthetic
  white sheet of the same size, and on `CONTRADICTED_REFERENCES_DREW_NOTHING`'s two the failing mean
  *is* our ink, 12.718 against a printed 12.72 and 13.672 against 13.67. **No renderer that drew the
  page could meet any of those bounds**, so the numbers are not a measurement of a disagreement and
  cannot be read as one. The tell costs one command: `magick identify -format '%k'` on the
  reference panel — the count of distinct colours, which is right where `%[fx:minima]
  %[fx:maxima]` is not: a solid *blue* sheet has a minimum of 0 and a maximum of 1 like any page
  with ink on it, and one corpus reference panel is exactly that. ADR 0499.

  **The gate acts on this since the six-hundred-and-eighty-first session, and the condition it
  fires on is not "constant"** (ADR 0513). A raster of one colour is a *failure* on a page with
  marks and a *reading* on a page that is a flat sheet, and our own render cannot tell those apart
  without circularity in both directions — excusing a flat reference because we drew nothing
  forgives every mark we lose, disqualifying one because we drew something dismisses every
  disagreement. So `pdfref::consensus_abstentions` asks the *other references*, and not "did
  anybody draw" but **"does anybody who drew disagree"**: a flat sheet inside `Tolerance` of a
  raster with marks on it is that raster as far as this instrument can measure. Written without
  that clause the rule cost nine corpus agreements; with it, six — and each of those six is a page
  where two flat sheets outvoted a renderer that drew *and ours was one of the flat ones*, which is
  the failure this trap is about arriving with its sign reversed. **The rule has a limit that is a
  page rather than a caveat**: on `recursiveCompositGlyf.pdf` a flat sheet *is* the page and the
  only renderer with marks is the one §9.3.6 does not support, so it abstains the wrong two — named
  in `NOT_COMPARABLE_A_FLAT_SHEET_IS_THE_PAGE`, because every refinement that would rescue it reads
  our own render.

- **And a ninth, which is not a mechanism at all: a reference that *is not there*.** The gate
  tolerates one of the three failing as long as two remain — correct, because many of these files
  are damaged and a renderer refusing one is the right behaviour — and until the
  six-hundred-and-ninety-fourth session it printed the resulting line **identically to a page
  judged on three**. So a page can change verdict with every input unchanged: `mupdf` and
  `ghostscript` agreeing is a consensus, and the same page with no `ghostscript` in it is two
  renderers missing each other and an `ambiguous` verdict. `function_based_shading_cmyk.pdf` page 2
  left the contradicted list on exactly that and stayed off it for six rounds — the figure the
  removal was written on, **29.06**, is `poppler` against `mupdf` to the hundredth, while `mupdf`
  against `ghostscript` is 0.192% of channels where the page's bound is 1.00%. **The tell is now
  printed**: `[judged without: <reference> did not render: …]` on the page's own line, and a count
  in the summary beside the abstention line. The rule that outlives the fix is about what a
  *removal* owes: a round that takes a page off a ratchet because *the references moved* owes the
  measurement that they did — `ls` the reference cache for the page's panels and re-render with the
  gate's own arguments, which costs a minute. Here all three panels were byte-unchanged since
  2026-07-29 and today's binaries reproduced them. ADR 0542.

  **And the six pages it printed have been read, which answers a question the count cannot** (ADR
  0575). *Is a consensus of two the same evidence as a consensus of three?* Same kind, one factor
  less: ADR 0005's inference is about a **pair**, so a third multiplies the improbability rather
  than creating it, and no rule was changed — none of the six is contradicted. What the six are
  actually about is ADR 0541's precondition rather than the count: **five of the six lost their
  third reading because the *document* is outside what ISO 32000-2 describes** — a §7.5.4
  subsection header with an object number of 2³², a file with no §7.5.2 header, a JP2 with no
  signature box, two page trees no repair recovers — so part of what the surviving pair agrees
  about is how to *repair*, which no clause states. The sixth is a reference being **wrong**:
  `pr6531_2.pdf`'s empty password authenticates against its `/O` under §7.6.4.4.11's Algorithm 12,
  which `poppler`, `gs` and this tree act on and `mupdf` 1.28 does not. **So the question to ask of
  a page judged on two is not "how many voted" but "why could the third not read it", and that one
  has a per-page answer.** `JUDGED_WITHOUT_A_THIRD_READING` carries all six, and the gate now names
  any page in the population that is on no list.

- **And a note's figures and the gate's figures are usually in different units, which is a
  conversion rather than a difficulty.** `raster_compare` divides by **width × height × four
  channels** and sums the absolute difference over all four, so: a mark ours paints and a reference
  does not costs `Δink × 255 × 3 ÷ (w × h × 4)` in the mean, because three channels differ and both
  rasters are opaque; a coloured stroke costs `perimeter × 510 ÷ (w × h × 4)` where the colour
  differs from white in two channels; and a *differing fraction* counts channels rather than pixels,
  so one row of 180 columns is 1.35 percentage points and not 1.80. Four of six group notes measured
  in the sixth criterion's own population were answerable that way and none had been asked. **And
  `rank_the_contradicted` prints bounds, not levels of 255** — `Distance::of` takes the largest of
  three ratios against the bounds the page was held to — which two notes had wrong, one of them
  quoting a page's worst tile over its bound as though it were a distance in levels. ADR 0499.

  **The differing fraction is a *threshold* count, so a colour mechanism's whole contribution to it
  is an area times a number of channels.** `JUST_NOTICEABLE` is 4, alpha never differs between two
  opaque rasters, and a flat mark whose colour is off by (3, 3, 6) therefore contributes its own
  area **÷ 4** rather than `× 3 ÷ 4`: on `transparent.pdf` the bottle is 11.4175% of the page, blue
  alone crosses the threshold, 11.50 ÷ 4 = 2.875 of the 3.316 points the gate prints, and the rest
  is the silhouette's edge. **Two levels of one channel decide that verdict**, which is worth
  knowing before reading a five-level disagreement as small. ADR 0510.

  **And the printed line is the worst-ratio member of the *agreeing consensus*, not of every
  reference.** `bug847420.pdf`'s four numbers are `mupdf`'s although `ghostscript` is further from
  us on all four, because `ghostscript` is not in the pair the verdict names. A before-and-after on
  a contradicted page has to be read against that same pair, or it compares two populations.
  ADR 0510.

- **And every mechanism above can act on a ranking's *denominator*, where it accuses us instead of
  excusing somebody.** Each bullet in this list is about shared code or a shared gap manufacturing
  an **agreement**, and the consequence stated for all of them is that the agreement is not
  evidence. But `doc/todo/00`'s *we are alone* ratio divides our distance by the closest pair's, so
  the same shared code makes the ratio **larger** and lifts the page up a list whose name says the
  page is ours. Measured over `freeculture.pdf`'s 321 compared pages: `poppler` and `mupdf` — the
  two voting references that share `libfreetype.so.6`, where `ghostscript` carries its own
  statically linked copy — are the closest pair on **9 of the 11 pages that reach that list** and
  on only **7 of the other 310**, and their own median MAE is **724** over those 11 against
  **1760** over the rest. Three ladders on the head page
  converge with ours between the other two and all three inside 0.032 of 255, so nothing about the
  page is ours. **A high ratio is a question about the denominator before it is a question about
  us**, and the denominator is where this whole list lives. ADR 0647.

  **Measured over the whole ambiguous pool rather than over one book, and it holds** (ADR 0663).
  Taking each page's closest pair by name: `poppler` + `mupdf` is the closest pair on **23 of the
  48** pages that reached that list and on **137 of the other 788**, while `poppler` +
  `ghostscript` — the one pair of the three that shares no glyph rasteriser — is the closest on
  **2 of the 48** against **333 of the 788**, where it is the commonest closest pair there is. The
  list is enriched almost threefold in the pair that shares `libfreetype.so.6` and depleted tenfold
  in the pair that does not. `issue16224.pdf` is the single-page form of it: one line of an embedded
  Type 1C subset, the sharing pair 0.41 bounds apart while each is 3.05 and 3.11 from `ghostscript`,
  and ours 1.13 from `mupdf` — **less than half `ghostscript`'s distance from the same reference**,
  on the page the ratio calls ours.

  **And the mechanism in a divisor need not be shared code at all — a shared *gap* does it, and the
  instrument is the removal.** On `bug766086.pdf` the divisor is `mupdf` + `ghostscript` at 0.45,
  and neither of those two draws the page's link border: `mupdf` constructs no `Link` appearance,
  `ghostscript` is rendering for paper. Replacing `/Annots [4 0 R]` with an empty array of the same
  byte length and re-running all four renderers moves our own number from **2.58 bounds to 0.43**
  and leaves that pair's comparison **byte-identical to the digit** — mean 2.2695, ssim 0.98268,
  worst tile 5.42 at (64, 0), both ways. So the numerator is a clause we implement and the divisor
  is the same clause two renderers do not, and the ratio counts it twice with the sign reversed the
  second time. **Where a ratio is large, take the mechanism out of the document and re-measure both
  halves**: a denominator that does not move is a denominator that was never about the page. ADR
  0663.

  **And the gate says which ratios are worth that removal**, since the seven-hundred-and-sixty-first
  session (ADR 0684). `[widened: outside]` on a row of that list means our nearest is outside the
  bound `Judgement::CORPUS` would have set from the closest pair's own spread — twice it, floored —
  where every other row is outside the class *floor* only, which is what `pdfref::decide` returns
  *because* no consensus formed. Unmarked, a consensus at that spread would have accepted us, and
  the ratio is measuring how closely two references happen to agree rather than anything about the
  page.

  **And the row says which measure each half is, because a mechanism in the divisor need not reach
  the measure the numerator is on** (ADR 0688). Both halves are a maximum over three measures, so
  the ratio is like for like only where the two maxima fall on the same one, and on most of that
  list's head they do not: `bug766086.pdf` divides a structural similarity by a mean, and so do the
  five `freeculture` pages. Naming the measure does not soften a row — on `bug766086.pdf` the
  like-for-like similarity reading is 14.9× where the printed ratio is 5.68× — it says **what is
  left to explain**. `bug1743245.pdf` is the case where the answer changes: its note prices two
  camps in whole-page mean grey, its row is a similarity against a renderer *in our camp*, and
  taking the mechanism out of the document moves our nearest from 31.43 to 2.62 while every
  reference stays byte-identical. The divisor is the mechanism the note names; the numerator is a
  second one the same note records and never numbered.

  **And the same pair decides most of the *contradicted* pool's verdicts, where it is the bound
  rather than the divisor** (ADR 0717). The mechanism above acts on a ranking; on a contradicted
  page it acts on `widened_to`, because the bound is derived from the convicting pair's own
  spread. Measured in the seven-hundred-and-eightieth session over every page the gate convicts
  on the differing fraction with `poppler` and `mupdf` as the consensus — 32 of the pool, and
  the gate's ranking prints the count every run now: the convicting pair's differing fraction
  runs 0.00% to 4.37% (an exact 0.00% on three pages) while **every pair containing
  `ghostscript` runs 5.32% to 13.37%** — no overlap — and `ghostscript`, whose FreeType is its
  own statically linked copy, fails the same bound against both pair members on all 32 pages,
  further than we do on 27 of them. So the one voting pair inside the class floor of each other
  is the pair hinting through one `libfreetype.so.6`, and the bound those verdicts rest on is
  one a voting reference cannot meet. No verdict moved on it: the measurement says what the
  bound is made of, not that our phases are right, and `doc/todo/12` prices what moving the
  bound costs.

The shape recurs with *us* in the minority: `mupdf` and `ghostscript` both refuse two files for
wanting a password, `poppler` and we open them, and §7.6.6 puts the refusal on the stream whose
key is missing. **Two against two is not a tie; it is a question with an answer, and the answer is
in the clause.** When a contradiction looks like "everyone disagrees with us", the cheap next step
is to search the other projects' source for the clause: a `FIXME` there is stronger evidence than
any number of agreeing pixels.

### 12. A bound derived from two agreeing references is tighter than the arithmetic

`oracle.rs` judges us relative to how far the consensus references sit from one another. That is
right — it stops a page where every renderer differs from being called our defect — and **where
two references agree very closely the bound can be tighter than eight-bit arithmetic**.
`issue7891_bc1.pdf` is the standing witness (`CONTRADICTED_TIGHT_CONSENSUS`): two ladders agree to
0.0014 of 255, ours at the page's own scale is 0.004 from the limit and the nearest of all five, and
the two renderers that vote sit 0.09 under — together, which is why they vote.
**Check the closed form** — write the clause's arithmetic down — then list the page with the
calculation beside it. Tightening our rounding until a reference's is matched is curve-fitting
with extra steps.

**And check it against the metric that actually fails.** That ink ladder is a real measurement of a
metric this page *passes*; what fails is the worst tile, and until the six-hundred-and-sixty-second
session nothing had asked what is in it. Written out from the file's own arithmetic — a black fill
through a luminosity mask, so every pixel is `255 × (1 − L)` — ours is the closed form on that tile
to **0.166 of 255 with a worst pixel of one level**, `ghostscript` is 4.596 from it and `mupdf`
6.723, and our 6.725 against `mupdf` is `mupdf`'s own distance from the arithmetic. A ladder over
the whole page and a closed form on the failing tile are two different instruments, and only the
second one answers the verdict. ADR 0489.

**And the closed form can answer the other way, which is what this trap's first witness turned out
to be.** `smask_luminosity_oob_transfer.pdf` stood here for hundreds of sessions with "the closed
form is `(223, 99, 80)`, `mupdf` `(222, 98, 79)`, `ghostscript` `(223, 99, 79)`, we `(223, 100, 81)`
— everybody within a level of the arithmetic". Everybody *except us*: `hayro` is on the closed form
byte for byte, an eight-bit mask predicts the closed form, and our extra level was a library
approximation this tree could turn off (ADR 0418). **Being inside a level of the clause is not the
same as being the clause**, and where one renderer is exactly on it and you are not, the tie has an
owner. The page agrees now.

**And a tight consensus need not be the *tightest* one, because agreement is not transitive and
the gate names only one pair.** `pdfref::decide` takes the largest set of references that all agree
with one another; with three references, `a` agreeing with `b` and `b` with `c` while `a` and `c`
differ leaves **two** maximal sets of two, neither contained in the other and neither a majority the
other is not — and until the seven-hundred-and-twenty-seventh session the loop skipped the second
without counting it, so the survivor was the one whose subset bitmask is smaller, which is the order
`Reference`'s variants are declared in. On `colors.pdf` pages 1 and 2 the pair the verdict rests on
is `poppler` + `ghostscript` at ssim 0.99431 and 0.99201 while `ghostscript` + `mupdf` agree at
0.99625 and 0.99278 and **accept us**; on `colorkeymask.pdf` the discarded pair contains a renderer
our raster is byte-identical to. Four contradicted pages turn on it, all four would have agreed
under the set thrown away. **The tell
costs one command**: run `examples/compare_rasters` over all three reference pairs, not only the one
the gate printed — a third pair sitting inside the same class bounds is a second consensus, and
which one decided the page was never argued. ADR 0616 has the finding.

**And the rule that replaced the enumeration order is worth knowing before you read a verdict**
(ADR 0617): **a verdict is one every maximal consensus reaches**, so a page whose sets divide about
us is `ambiguous` and named in `AMBIGUOUS_DIVIDED_CONSENSUS`. Two things follow for a round reading
this bucket. First, `ambiguous` now covers two different pictures — nobody agreed, or two sets agreed
and parted — and on the second **every renderer in the room, ours included, is inside somebody's
reading**, which the first never is; the page's own line says which it is. Second, the control that
settled the rule is the one to copy when a divided page turns up: **put each reference where our
render stands and ask what the sets it is not a member of conclude about it.** On all four corpus
pages the set that used to decide the verdict contradicts a voting reference that is itself in a
consensus — on `colorkeymask.pdf` that reference is `ghostscript`, whose raster is ours to the byte,
so the numbers accusing us are literally `ghostscript`'s distance from `poppler`. What the rule does
**not** distinguish is a division of *camps* from a division of *width*: on `issue11403_reduced.pdf`
one reference is in both sets and we are further from all three than any two are from each other, so
`ambiguous` there is the absence of a verdict rather than an acquittal.

**And a bound derived from an aggregate is not a bound on the pixels the aggregate is made of.**
`calrgb.pdf` page 1's consensus pair differs by 4.41%, so the gate holds us to 8.82%. Over the
eighty swatch centres that pair is a mean 2.35 of 255 apart — but restricted to the 41 swatches
where the camps disagree at all it is 3.78, and at the swatch carrying the page's largest
difference it is **16 levels apart, further than we are from either of its members**. Three
quarters of that sheet is a mid-tone no camp disputes, and the pair's closeness is mostly a
measurement of that region. So ask *which pixels* a tight consensus is tight over before reading
its doubled spread as a bound: the population the bound is computed on and the population the
verdict is about need not be the same one. ADR 0494.

### 26. The worst tile is measured on a fixed grid, so the same difference is worth twice as much on one page as on another

Trap 12 is about the bound moving under a verdict. This one is about the *number the bound is
compared with*, and it moves for a reason that is nothing to do with either renderer.

`raster_compare` lays its 32-pixel tiles from the raster's origin rather than around the
difference, so a mark that lands inside one tile is measured whole and the same mark straddling a
boundary is measured in halves. The corpus pays it and the witness is a document that states its
own control: `pdfbox/PDFBOX-2984-rotations.pdf` draws one line of 50 pt `/Helvetica` six times, and
the only glyph the substituted face disagrees about — a `registered` sign — occupies device columns
484 to 519 on page 1 and 526 to 561 on page 5. On page 1 that is 28 of its 36 columns inside the
tile at `x = 480`; on page 5 it is split 18 and 18 across the tiles at 512 and 544. Ours against
`ghostscript`, in level-pixels:

```text
                     over the glyph's own columns   worst tile
  page 1  (480, 64)            75 004                 64 072   -> 62.57   contradicted
  page 5  (512, 64)            78 212                 36 170   -> 35.32   agrees
          (544, 64)                                   28 670
```

**The same glyph, the same difference to four percent, and the measure a factor of 1.77 apart.**
And the reading that was in the tree instead is the one to recognise, because it is the natural
one: `CONTRADICTED_SUBSTITUTED_FONT`'s note explained the split as the references' — *their
consensus pair happens to sit further apart and the bound derived from it is wider* — which is
trap 12's shape and would have been a good answer. Measured, the pair on pages 5 and 6 sits
**closer** (25.33 against 28.40) and the bound is therefore **narrower** (50.66 against 56.81). The
bound went the other way and our own number carried all of it.

Three things follow for a round reading a worst-tile verdict:

- **A worst tile is comparable between two renderings of one page and not between two pages.** Two
  pages of the same document with the same defect are not ranked by it.
- **`worst_tile_at` is the first thing to print**, because it turns a statistic into a place: on all
  four contradicted pages of that document, ours against every reference *and* every pair of
  references names the same tile, which is what said the whole verdict was one glyph before
  anything had been ablated.
- **A page that agrees is not thereby a page that matches.** Pages 5 and 6 carry the identical face
  deficit and the gate is silent about them, so a group whose membership is read off the verdict
  will be missing them — which is why that note's own last line says its membership is a
  measurement and never a verdict.

`raster_compare::DEFAULT_TILE`'s doc comment carries the same paragraph beside the constant, and
`the_same_difference_reads_half_as_much_when_it_straddles_the_tile_grid` pins the halving in
arithmetic. ADR 0755.

## Things worth knowing

- **The oracle's artefacts are the fastest diagnostic in the tree.** Every non-agreeing page leaves
  `<target>/tmp/oracle/<stem>/p<n>/` with our render, each reference's, a side-by-side strip and a
  heatmap per reference. **Open the side-by-side first**: one image, four panels, ours leftmost,
  and it has explained every page it was pointed at. Agreeing pages have theirs deleted, so what is
  on disk is exactly the set worth looking at.
- **A page's tolerance class depends on what *we* drew.** The oracle picks a text or vector
  tolerance from our own render's content, so a change that adds glyphs also loosens the bound.
  Since session 31 the question is `Interpretation::glyphs` — "did glyphs mark the page" — rather
  than "did we read text back", which had made a page of unnameable CJK a vector page and a page of
  invisible OCR text a text page, both backwards.
- **Pixel comparison cannot police text, so there is a second metric.** The references disagree
  with each other at worst-tile 26–28 on text pages — hinting, not error — and no threshold fixes
  that. `raster_compare::Comparison::structural_similarity` asks whether the same shapes are in the
  same places; `Tolerance` bounds it at 0.99 for vector, 0.90 for text, measured over 153
  reference-against-reference pairs. The distribution is **continuous** — 0.8990, 0.8993, 0.8998,
  0.9009 all occur — so 0.90 is a choice about which population to exclude, not a discovered
  boundary.
- **Reference renderers are given 30 seconds and then killed.** `Command::output` waits forever;
  `Reference::render_within` polls and kills, and there is deliberately no unbounded variant. It is
  not only a timeout: over 7000 crawled documents ranked twice with nothing changed between the
  runs, one `poppler` panel was absent in the first and present in the second, which is a row moving
  for a reason that is not the tree's.
- **An instrument built into a fresh target directory is missing `pdf-sandbox-worker`, and every
  codec behind the sandbox then refuses in silence.** `cargo build -p pdf-model --example
  render_at` does not build the worker, and without it a JBIG2, CCITT or JPEG 2000 image reports
  `the sandbox worker … was not found` and the page draws without it — so the ranking measures a
  tree with no bilevel decoder. Session 619 caught it only because it re-measured the previously
  fixed documents first: seven of sixteen came back *worse than before their fix*, one of them
  −156.436 against a recorded −6.390. **A round that had skipped that check would have read seven
  regressions off its own missing binary**, which is why the check is not a formality. Two
  commands, and the second is `cargo build --release -p pdf-sandbox --bins`.
- **And the twin of that, which is worse because it is silent rather than loud: a
  `pdf-sandbox-worker` that *is* found and is not this build.** `worker_program` searches beside
  the running executable **first** and only then one directory up, so a copy left once in
  `target/<profile>/examples/` outranks every rebuild of `target/<profile>/pdf-sandbox-worker` for
  as long as it sits there — and an example binary is exactly what a person reproducing a ranked
  document runs. An older worker answers every request out of older decoders, and its refusals
  reach the page **worded as the decoder's own**, so they read as the file's defect. Session 621's
  `hayro-jbig2` fix was reported gone by session 623 for this reason and for no other; three
  sessions went into a tree that was correct (ADR 0458). **The greeting names it now** —
  `SandboxError::WorkerMismatch`, with both identities and the worker's path — so the shape to
  recognise is that message rather than the hunt. What is still worth doing by hand is the
  one-line check that made the attribution: run the same instrument twice with
  `PDF_SANDBOX_WORKER` naming each candidate, because *substituting* a worker only eliminates the
  worker if the substituted file is the one that runs.
- **A ranking against the *lightest* live reference is sensitive to one reference failing
  quietly**, and on the open web that is commoner than a defect of ours. `doc/todo/00` step 7's
  number is our ink minus the smallest of the references', so a renderer that draws a page's rules
  and none of its body font sets the bound alone. **22 documents of 5000 crawled ones are that
  shape** — `poppler` under a quarter of our ink while `mupdf` and `ghostscript` sit within 30% of
  it — and they fill the whole positive head down to +2.6 (session 613). Read a positive gap as a
  question about *which* reference is light before reading it as ink of ours; the panel sizes are
  printed beside the number for the same reason.
