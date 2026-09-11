# Habits: judging against other implementations

Status: **standing** — method, not code.
Read by: a round that reads a verdict, diagnoses a page against poppler, mupdf, ghostscript or
pdf.js, or moves a tolerance. `doc/traps/oracle-and-references.md` is the code half;
`doc/oracle-and-corpus.md` is the instrument.

`doc/habits.md` is the index of the six, and it states what a habit is and what each keeps.

- **Compare the references with each other before opening a page.** Four unexplained contradicted
  pages sorted themselves into one group from a table of pairwise means.
- **A tolerance is a claim about a population, so measure the population.** A fixed bound says "two
  independent implementations of this clause are not further apart than *this* on a page of this
  kind", which is a statement the corpus can check: take every reference pair, and take each
  measure over the pairs the **other** bounds admit — a bound measured over the pairs it already
  admits returns the bound. Run over 9898 pairs it found one of eight sitting below its own
  references' spread, rejecting **29.4%** of them where its three siblings reject 0.0%, 1.2% and
  0.5%, and the sentence that claimed to derive it naming a *different* measure's number. ADR 0243.
- **Then check what the number is used for before moving it.** The same bound decided whether two
  references agree at all, so the derived value took 68 contradicted pages to 309 and emptied 457
  out of `ambiguous`. A derivation says where a number should be; it does not say the number has
  only one job.
- **Rank the suspects by a ratio, not a distance** — our worst measurement over the bound it is
  held to. Five times it has chosen the next item before an artefact was opened. The oracle prints
  it for the contradicted pool itself since ADR 0636, so it is no longer a thing to take off a log
  by hand — and the ratio carries the *name* of the measure it belongs to, because the same number
  on the differing fraction and on the mean describes two different pages.
- **Two numbers on one line are comparable only if one instrument produced both.** The oracle
  printed the closest reference pair's distance in four measures beside ours in three, for two
  hundred rounds, on a line whose own comment asks a reader to compare them — and on its head page
  that reads *35.12 between them, 5.03 ours* where in one unit it is 35.12 against 32.42. Over the
  bucket the mixed reading names 13 pages as ones we are alone on where either single unit names 48
  or 569. It is `-alpha off`'s lesson in ratios rather than in ink: **a wrong comparison that looks
  like a finding**. ADR 0643.
- **Before believing "one pixel out" is rounding, compare the raster sizes.** One reference put
  type a row above ours from a raster *the same size as ours*, which no disagreement about row
  counts can explain. ADR 0064.
- **`magick identify` every panel before believing any number, and the flags before that.**
  `pdftoppm` renders the **`/MediaBox`** unless told `-cropbox`; this tree, the oracle and
  `mutool draw` render the **`/CropBox`**. On `freeculture.pdf` the areas differ by 1.378, so a
  ladder taken without the flag put `poppler` at 9.10 against our 12.18 and would have
  manufactured a 34% defect on four pages that agree to 0.03 of 255. This is the twin of
  `-alpha off`, which returns exactly half the ink on a panel that carries an alpha channel;
  **both are a wrong measurement that looks like a finding**, and both now sit in
  `doc/todo/00`'s step 6 where a session reaches for the command.
- **Before trusting a clean fuzz run, ask what fraction of it got past the first branch.** The
  sfnt target ran 50 000 unseeded inputs in under a second and tested nothing: random bytes do
  not form a table directory, so every run left on the first `?`. Seeded with sixty real
  `/FontFile2` streams it produced two crashers inside a minute. A format with a magic number, a
  count and a directory needs a corpus; a content stream or a date does not. ADR 0175.
- **A rewrite driven by untrusted structure is a larger surface than a reader over the same
  bytes.** Both glyph-table repairs had been reviewed and never fuzzed, and both wrote at an
  offset a document supplied. ADR 0175.
- **A count of "marks missed" is a count of something else until you look at what they read
  back as.** 50 codes over 9 documents drew nothing and said nothing; 26 of them were one code
  of `pr12564.pdf`, and `pdftotext` reads that page as `1101#Strayer#Drive` — the code is the
  document's *space*, and having no outline is correct. The exemption that catches an ordinary
  space is "reads back as whitespace", which is blind to a font that reads a space back as `#`.
  `PDFVIEWER_TRACE_MISSING_GLYPH=1` is the trace that settled it in one run.
- **A page-level number cannot clear a mechanism of a defect that is five glyphs wide.** ADR
  0170's session A/B'd its `loca` repair against `issue7074_reduced.pdf` — ink 19.576 with the
  repair on and 19.576 with it off — and concluded the repair did not reach the page. The
  measurement was right and the inference was not: the page is three words of bold nine-point
  text and the defect was five narrow bars, under a tenth of a level. **Point the A/B at the
  quantity the hypothesis is about** — here, which glyph the space's code resolves to — which is
  one assertion rather than one render. ADR 0174.
- **A corpus can hold one document under a dozen names, and the bucket's shape lies until you
  check.** 154 of the ambiguous bucket's 678 were `tracemonkey.pdf` and eleven copies of it with
  annotations added — `pdftotext -f 9 -l 9 | md5sum` is identical across them. One measurement
  settled all 154, and the honest number to report is *one finding*.
- **When a metric accuses you, find one that measures the same thing differently.** Eight text
  pages failed on mean absolute difference and passed every other bound; the page's *total ink*
  put us within half a level of both voting references. One number from artefacts already written
  turned eight questions into one population.
- **A page that draws the same glyph twice is an instrument, and it needs no reference at all.**
  `issue7696.pdf` is 200×50 and draws four glyphs twice, 80 pixels apart. `poppler`, `mupdf` and
  `ghostscript` draw the two halves *byte-identically*; ours differ by 2893 and `hayro`'s by 3541.
  That is grid-fitting measured from the inside — the three C renderers share `FreeType` and its
  hinting, the two Rust ones place a glyph where §9.4.4's matrix puts it — and it settles a
  contradicted page without comparing anything to anybody. **Ask what a page repeats.**
- **An inconsistency inside a reference's own output outranks any distance from it.** Two
  renderers spacing one line at two different widths cannot both be reading the document's `/W`.
- **Agreement with one reference is not evidence**, and **"both readers fail the same way" is
  agreement about a symptom** — `poppler` reporting the same broken flate stream was taken as
  proof a file was damaged; both readers were deriving the same wrong key.
- **Two references against two is not a tie and not a vote — it is a question with an answer.**
  `Type3WordSpacing.pdf` splits them over a `d1` glyph's stroke colour and Table 111 settles it.
- **An unimplemented feature has a default, and the default is usually "draw it".** That is a more
  common failure of the oracle's premise than shared code.
- **Point your own instrument at their data**, and **ask the reference the same question you asked
  yourself**.
- **A test corpus has a bibliography, and it is the first step rather than an occasional one.**
  Every pdf.js file is named after the issue that introduced it —
  `issueNNNN…pdf` → `github.com/mozilla/pdf.js/issues/NNNN`, `bugNNNNNNN…pdf` →
  `bugzilla.mozilla.org/show_bug.cgi?id=NNNNNNN` — and the issue says what the file was added to
  prove. It corrected a written conclusion on the first afternoon, and §3a now turns on it.
  **A pair of fixtures with a common stem is an A/B the corpus built for you**: `issue7891_bc0`
  and `issue7891_bc1` differ in `/BC [0 0 0]` against `/BC [1 1 1]` and in nothing else.
  Two cautions. The issue describes **that reader's** defect, which may be one this tree does not
  have — pdf.js's 7891 is *ignoring* `/BC`, which `soft_mask::backdrop` reads and §11.6.5.1's
  outside-the-bounding-box rule is applied for. And an issue is evidence about a *file*, never
  about the clause: principle 5 is not suspended because a bug report is specific.
- **A corpus document can be a conformance test, and then it outranks every renderer**
  (`issue14256.pdf` draws one picture eight ways) — **or check a decoder against itself** (an LZW
  image must decode to exactly `width × height` bytes; 96 documents encode one image ninety-six
  ways). Ask **what does this file already say about itself?**
- **Look at what a corpus file is *for* before filing it under a group.**
- **"Ranked against the references" is not "the oracle has seen it", and this tree records both in
  the same words.** `doc/todo/03` says of four separate populations that each was "put in front of
  a reference", and §14 concludes "[e]very population on this disk is ranked" — all true, and all
  about the **ink ranking**: page one at 72 dpi against three renderers, sorted by our ink minus
  the lightest live reference's, by a script that lives for one round. It finds the head of a
  distribution and that is its whole job. The **oracle** is a different instrument on the same
  three programs: a bound derived from the references' own spread *on that page*, four verdicts,
  and ratchets held to equality in both directions. A population can have had the first many times
  and the second never, which is what the six-hundred-and-ninety-second session found of all 275
  documents under `doc/corpora/` (ADR 0541). **The tell is whether the round left a list of pages
  behind**, which is `doc/todo/03` §20's own rule — a chunk leaves a file rather than a memory —
  and a ranking cannot leave one because it reaches no verdict to record.
- **Before pointing the oracle at a new population, ask what clause its files are exercising.**
  ADR 0005's rule is that two implementations sharing no code agreeing about a page is evidence
  about the specification, and the inference needs a clause for them both to be reading. A corpus
  of *deliberately damaged* files has none — the standard "describes valid files and says nothing
  about the rest" — so three renderers agreeing there agree about three recovery heuristics, which
  is trap 9's shared gap with the gap put in on purpose. A corpus assembled *because*
  implementations disagree is worse still, and ADR 0393 has that one. Neither is a reason not to
  look; both are reasons not to **ratchet**, because a ratchet turns a contradiction into a thing
  to be made to go away.
