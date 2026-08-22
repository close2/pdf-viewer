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
evidence. Eight ways for that to fail — and the count has moved four times, so read the list rather
than the number.

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
  listed. `AMBIGUOUS_SHARED_JBIG2_DECODER`.
- **And a sixth, which is none of the five above: a shared external *standard*.** On
  `CONTRADICTED_DEVICE_CMYK_CONVERSION`'s five pages `hayro` sits with the `mupdf`/`ghostscript`
  pair — 4 and 5 levels of 255 from them, 48 from us — and it shares nothing with either.
  `objdump -p` on `pdfref-hayro` names `libgcc_s`, `libm` and `libc` and no colour library at all;
  what it carries is its own `CGATS001Compat-v2-micro.icc`, 8 464 bytes, `desc` `uCMY`, `cprt`
  `CC0`, one `A2B0` tag, against Artifex's 187 484 bytes and three `A2B` tables. Different file,
  different author, different licence — and this tree's own evaluator on **either** of the two
  predicts all three renderers to within eight levels while sitting 48 from ours. What the three
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

**And a bound derived from an aggregate is not a bound on the pixels the aggregate is made of.**
`calrgb.pdf` page 1's consensus pair differs by 4.41%, so the gate holds us to 8.82%. Over the
eighty swatch centres that pair is a mean 2.35 of 255 apart — but restricted to the 41 swatches
where the camps disagree at all it is 3.78, and at the swatch carrying the page's largest
difference it is **16 levels apart, further than we are from either of its members**. Three
quarters of that sheet is a mid-tone no camp disputes, and the pair's closeness is mostly a
measurement of that region. So ask *which pixels* a tight consensus is tight over before reading
its doubled spread as a bound: the population the bound is computed on and the population the
verdict is about need not be the same one. ADR 0494.

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
