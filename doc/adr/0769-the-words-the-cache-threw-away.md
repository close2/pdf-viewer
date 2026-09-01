# ADR 0769 — The words the cache threw away

Status: accepted, 2026-09-01. Session 842. Extends ADR 0513's abstention with a second route and
changes nothing it decided; closes the one route ADR 0768 left open. Amends ADR 0020's cache with a
`FORMAT` bump, and ADR 0574's renderer log with the storage that makes it survive a hit. Amends
§7.4.7's ledger row and one sentence of §7.4.9's, on a reading of the clause rather than on
anything this round built.

## The question

`rank_the_contradicted`'s head, `bitmap-symbol-context-reuse.pdf` page 1, has been contradicted
since the corpus gate existed and diagnosed for a hundred and sixty sessions. Three references
return a raster of one colour — `poppler` white, `ghostscript` white, `mupdf` entirely black — and
two of those three agree with each other at a spread of zero and outvote a render of ours that
draws the image. ADR 0499 had already shown that all four of the verdict's numbers are reproducible
by comparing our render with a **synthetic white sheet** of the same size, so no renderer that drew
the page could have met the bound the page was held to.

ADR 0513's abstention exists for exactly this shape and cannot reach this page. It refuses a vote
to a flat sheet **where a reference that drew marks disagrees with it**, and here nobody drew. ADR
0768 wrote the narrower rule that would have moved it — where every reference is flat and the flat
sheets disagree, none of them is a reading — tested it, and reverted it, because `pdfref`'s own
suite refutes it: `a_two_of_three_majority_forms_the_consensus` and
`references_disagreeing_among_themselves_is_not_our_failure` are two uniform white rasters against
a uniform black one, which is this page's shape exactly.

That refutation is the finding, and it generalises past the page:

> **A genuinely blank page with one broken renderer and a page nobody decoded with one broken
> renderer have the same three rasters.** No predicate over pixels separates them, and one that
> fires on both forgives a render of ours that painted marks on an empty sheet.

## What separates them, and why it was not available

The renderers say which of the two it is. On this page `mupdf` prints `library error: cannot decode
jbig2 image` and `ghostscript` prints `jbig2dec WARNING failed to decode; treating as end of file`.
A program that read the file and found it blank writes neither sentence.

`Reference::render_within` has captured both of a renderer's output streams into `<name>.log`
beside its image since ADR 0574. **`cache::render` returned on a hit without ever running it.** So
on a run at a 99.8% hit rate every verdict was reached from rasters while every diagnosis came from
log files an earlier run happened to leave behind — and a rule that read a file only a *miss*
writes would reach one verdict on the first run of a corpus and another on the second, which is the
one thing a cache may never do.

The cache stores the log beside the picture now and restores it with the picture, **empty
included**: a renderer that said nothing said nothing, which is a fact about the entry rather than
a gap in it. It is stored only beside a picture — a stored failure already carries the renderer's
sentence as its whole text, and a reference that produced no raster does not vote.

**The `FORMAT` bump is the mechanism and the alternative was rejected in writing.** Trap 10a
already refuses a bump to correct a few dozen stale failure *sentences*, and that stands: prose was
never in the key. This is different in kind — what an entry *means* changed — and the cheap
alternative, treating an entry with no stored log as a miss, would have left old entries readable
as new ones by a second ad-hoc route and would have made "no log stored" and "the renderer said
nothing" the same thing on disk, which is the distinction the whole rule rests on. It cost one
re-render of all 6707 entries, 1296 seconds of processor time in the three renderers, run once.

**That run is the control rather than a formality.** With the log stored and the rule not yet
wired, every verdict in every class was unchanged: 983 agrees, 61 contradicted, 836 ambiguous, 42
not comparable, 3 our geometry, 2 reference geometry, 18 no render. The bump moved the cache and
nothing else.

## The rule

> A reference whose raster is one colour also takes no part in the consensus where **its own log
> says it did not draw what the page asked for**.

Three things bound it, and each is deliberate:

- **our own render never enters it**, in either route, which is what keeps the whole abstention
  non-circular;
- **uniformity is still required.** A renderer that complained and drew marks anyway has produced a
  picture and there is no ground to discard one; the question only arises for a flat sheet;
- **silence concludes nothing.** A renderer that printed nothing has given no testimony, and a log
  that is missing is treated identically — so a caller that collected no logs gets the pixel rule
  unchanged, which is what `triangulate`'s empty slice is.

## The condition is a vocabulary, not a severity, and that is measured

Each of the three programs states its own severity, and reading it is the obvious rule and the
wrong one. Over the oracle's own population — the pdf.js corpus, `doc/corpora/` and the
specification PDFs in `doc/` — **28 901** of `poppler`'s `Syntax Error` lines are `Type mismatch in
PostScript function`, on pages it draws correctly, and `mupdf`'s `format error: incorrect number of
xref entries in trailer, repairing` says in its own text that it recovered. A predicate on severity
is trap 11's shape exactly.

What is read is what a program says it **produced**:

| program | sentence | what it is |
|---|---|---|
| `mupdf` | `library error:` | its severity for a library that failed to produce data — all five in the population are `cannot decode jbig2 image`, `cannot complete jbig2 image` and three `zlib error:` variants |
| `mupdf` | `cannot draw '` | it abandoned the page it was given |
| `mupdf`, `ghostscript` | `failed to decode` | **`jbig2dec`'s** words, reaching the log through both programs |
| `ghostscript` | `FATAL ERROR` | its severity for a decoder that stopped |
| `ghostscript` | `Page drawing error occurred` | its own statement that drawing the page failed |
| `ghostscript` | `Unrecoverable error` | its own statement that it stopped |

**`ghostscript` labels one of these a `WARNING` and it is still a refusal.** `jbig2dec WARNING
failed to decode; treating as end of file` is two statements: the severity is `ghostscript`'s
judgement about how to carry on, and *failed to decode* is `jbig2dec`'s statement about what it
produced. The same sentence reaches `mupdf` as `warning: jbig2dec warning: failed to decode…` under
a `library error:` of `mupdf`'s own — two programs labelling one library's words differently, which
is the clearest evidence there is that the severity is not the thing to read.

**`poppler` is read not at all, and that is a measurement.** Its refusals are worded as ordinary
syntax errors — `Syntax Error (681): Too many symbols in JBIG2 symbol dictionary` on the page that
occasioned this work, `Could not find start of jpeg data` on four others — and nothing in the
wording separates them from the tens of thousands it writes about defects it recovers from. Taking
one of its sentences and not the others would be a list fitted to the pages this round wanted to
move. Its flat sheets are judged by pixels alone, exactly as before.

## The audit is printed, both ways, every run

Trap 11's real demand is not "derive the condition" — this one was derived — but *"list everything
in this population that satisfies the condition and does not satisfy the question"*, and its
converse. The oracle prints both: over its 1945 pages there are **334 reference rasters of one
colour**, of which **55 name a refusal** in 19 distinct sentences, **157 said nothing at all**, and
the remainder said something in 666 distinct sentences that the condition did not match, the
commonest 25 of each printed with counts and the renderer that wrote them.

It is a report and not a ratchet, for the reason the abstention census beside it is not one: how
often three other programs fail is not this tree's to hold. What it is for is that **the condition's
right-hand side belongs to three other projects**, who can reword it at any release — a claim that
decays exactly the way a ledger row does, and the only sweep that can see it is the gate's own
output.

## What it cost

Four pages, all now in `NOT_COMPARABLE_THE_RENDERERS_SAID_THEY_DREW_NOTHING`, and they divide by
what *we* drew.

One is the gain. `bitmap-symbol-context-reuse.pdf` page 1 leaves `contradicted`: we draw the image
— 10 950 black pixels of 159 600, byte-identical to our render of every other encoding of the same
drawing, which is `tests/jbig2.rs`'s invariant and not any renderer's agreement — and the two
programs that said they could not decode it no longer outvote us.

Three were `agrees` and are the price: `jbig2_file_header.pdf` page 1 and `poppler-90-0-fuzzed.pdf`
pages 12 and 16, where we drew nothing either, so the agreement was four programs failing at one
file and matching each other exactly — the empty picture ADR 0005's inference does not reach.
**All three are pages this tree already reports as incomplete**, and the count of agreements on
pages the gate calls complete did not move: 943 before and 943 after. That is the number to check
before widening the vocabulary, and it is why widening it is a measurement rather than a taste.

Nothing moved toward a verdict that flatters us: nothing became `agrees`, and the one page that
stopped being contradicted did so by ceasing to be comparable at all.

## The second track: §7.4.7 was `partial` on a requirement addressed to somebody else

Read against the clause rather than against the row's own sentence. §7.4.7's `partial` rested on
one thing — *"the JBIG2Decode filter shall not be used with inline images"* is not enforced — and
that sentence is one of six in the clause that constrain how a file is **built**: the 2-byte marker
"shall not be present", the file header and end-of-page segments "shall not be present", a
segment's page association "shall be set to 1", global segments "shall be placed in a separate PDF
stream". None of them is addressed to whoever decodes the data, and the row's own opening sentence
already calls them "what a reader may assume".

§7.4.10's row in the same file makes exactly that distinction for exactly that shape — *"The Crypt
filter shall be the first filter in the Filter array entry' is a requirement on the writer that
this reader does not need"* — and the reading is stronger here, because refusing an inline image
that names the filter would draw **less than the producer wrote**, on a restriction the producer
broke, which is the opposite of what `CLAUDE.md` calls done.

The three sentences that *are* addressed to the reader are the filter decoding "bitonal (1 bit per
pixel) image data … excluding colour palette coding", "[t]he filter shall use the embedded file
organisation of JBIG2 as defined in ISO/IEC 14492:2019, Annex D.3", and Table 12's `/JBIG2Globals`.
All three are implemented, so the row is `implemented` and names the test that would fail if it
stopped being true. §7.4.9 carries the identical sentence for `JPXDecode` and its row now says so;
it stays `partial` on its own two debts.

**What the tree does with such an inline image is a consequence rather than a decision**, and the
row records it: `pdf_syntax`'s `delimiting` has no rule for this filter, so §8.9.7's `EI` search
decides where the data ends and `image::decode` then decodes it through the sandbox exactly as it
would in an image XObject.

## Where it lives

- `pdfref::Testimony` and `Reference::refusals` in `tools/pdfref/src/reference.rs`, beside the
  renderer they are about — the vocabulary is a per-program claim, and `poppler`'s empty list is
  part of the statement.
- `pdfref::consensus_abstentions` gains the testimony slice and the second route;
  `triangulate_with` passes it through and `triangulate` passes none.
- `cache::LOG`, and the `FORMAT` bump, in `tools/pdfref/src/cache.rs`.
- The audit is the oracle's `what_the_flat_sheets_said`, over `Examined::flat_sheets`, which takes
  the **same** testimony the rule was given rather than re-reading the logs — an audit that
  described a different population from the one that was judged would be worth nothing.

Five tests in `pdfref` pin it, and two of them carry the three renderers' logs **verbatim**, because
a condition over another project's prose is one a paraphrased fixture would pass while the rule
stopped working — trap 13, run the sweep against the defect. `end_to_end`'s
`a_hit_reproduces_what_the_renderer_produced` now demands the log a hit leaves be byte-identical to
the one the renderer wrote.
