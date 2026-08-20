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

### 9. Two references can agree because they share code — or because they share a *gap*

The oracle rests on ADR 0005: two implementations sharing no code agreeing about a page is
evidence. Four ways for that to fail.

- **A shared gap.** An unimplemented feature falls through to a *default*, so two unrelated
  programs that skipped the same clause produce the same picture. `visibility_expressions.pdf`:
  `mupdf` carries `/* FIXME: Calculate visibility from array */ return 0;` and `ghostscript`
  prints `WARNING: OCMD contains VE ... (ignoring)`, while `poppler` and pdf.js implement `/VE`
  and §8.11.2.2 is unambiguous. The page stays contradicted, with the source citations beside it.
- **Shared data.** `mupdf` and `ghostscript` disagree with us on four `DeviceCMYK` pages and agree
  with each other to under a level, because they run the same ICC profile. What settled it was
  *this tree's own* A2B evaluator pointed at `default_cmyk.icc`. **When two references agree
  suspiciously closely, ask what data they are both reading, and evaluate it yourself.** ADR 0048.
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
  listed. `AMBIGUOUS_SHARED_JBIG2_DECODER`.

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

**And the closed form can answer the other way, which is what this trap's first witness turned out
to be.** `smask_luminosity_oob_transfer.pdf` stood here for hundreds of sessions with "the closed
form is `(223, 99, 80)`, `mupdf` `(222, 98, 79)`, `ghostscript` `(223, 99, 79)`, we `(223, 100, 81)`
— everybody within a level of the arithmetic". Everybody *except us*: `hayro` is on the closed form
byte for byte, an eight-bit mask predicts the closed form, and our extra level was a library
approximation this tree could turn off (ADR 0418). **Being inside a level of the clause is not the
same as being the clause**, and where one renderer is exactly on it and you are not, the tie has an
owner. The page agrees now.

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
- **A ranking against the *lightest* live reference is sensitive to one reference failing
  quietly**, and on the open web that is commoner than a defect of ours. `doc/todo/00` step 7's
  number is our ink minus the smallest of the references', so a renderer that draws a page's rules
  and none of its body font sets the bound alone. **22 documents of 5000 crawled ones are that
  shape** — `poppler` under a quarter of our ink while `mupdf` and `ghostscript` sit within 30% of
  it — and they fill the whole positive head down to +2.6 (session 613). Read a positive gap as a
  question about *which* reference is light before reading it as ink of ours; the panel sizes are
  printed beside the number for the same reason.
