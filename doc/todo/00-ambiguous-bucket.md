# Empty the oracle's ambiguous bucket

Status: **standing task**, since the hundred-and-seventy-sixth session. **The queue is empty
again**, since the six-hundred-and-ninety-fourth (ADR 0543); `tools/state.sh` counts it, and the
paragraphs below say where it came from and what emptiness is and is not a fact about.
Priority: 00 — the last large population where a defect can live without a name
Corpus: **two populations since the six-hundred-and-ninety-second session** — `doc/pdf.js`'s and
`doc/corpora/pdfbox`'s 143 pages, which the oracle judged for the first time (ADR 0541) and whose
63 ambiguous ones are diagnosed in `AMBIGUOUS_TEXT_AT_DOCUMENT_SIZE` and
`AMBIGUOUS_PAGE_PLACED_A_ROW_APART`.

The figures below are the pdf.js population's and were the whole of this item until that round: 786
ambiguous pages (**754** on documents we call complete); **all 786 diagnosed, 0 held by
name** — and the 72 this line used to say was `wc -l` of a file with a twelve-line header, corrected
in the three-hundred-and-seventeenth session by counting what the gate counts. **This parenthesis
said 750 for 140 commits and the gate prints 754**, read off it in the four-hundred-and-forty-fifth.
Where the four went was not chased and does not need to be: the *ambiguous* total has been 786
throughout, and only the complete/incomplete split moves — which is what a report arriving on or
leaving a page already in this bucket does, and several rounds in this block did that. The lesson is
the one the test count taught: a number a round does not print is a number from the last round that
did.

**And "all 786 diagnosed" means two different things for the two halves of that split, which the
five-hundred-and-ninety-eighth session found by opening the head of step 7's sweep.** The pages on
*complete* documents are diagnosed by an `AMBIGUOUS_*` group with an argument beside it, held in
both directions by `check_the_ratchets`. The pages on incomplete ones are outside that ratchet by
construction — its `named` closure filters on `e.complete` — and what holds them is the corpus
gate's own report. That is a ratchet rather than a diagnosis: it says *that* something was skipped
and never *why the skip is right*. Eleven of them turned out to share one clause and nothing in the
tree had said which (ADR 0433).

Code: `crates/pdf-model/tests/oracle.rs`, `crates/pdf-model/tests/ambiguous_undiagnosed.txt`

**The list emptied in the three-hundred-and-seventy-ninth session and the task does not end with it.**
The gate holds `ambiguous_undiagnosed.txt` to equality in *both* directions, so a page that stops
agreeing arrives in an empty file and fails the build on the arrival — which is the regression this
instrument was built to see and is now the only thing it has left to do. Step 7 is likewise standing:
it is the half of the work no ranking can perform, and it is re-run after any round that changes what
gets drawn.

**And a page can now arrive here without anything about it changing at all, which is a third way in
and is new in the seven-hundred-and-twenty-ninth session** (ADR 0617). A verdict is one *every*
maximal consensus reaches; agreement between references is not transitive, so a page can carry two
maximal agreeing sets that reach different conclusions about our render, and such a page has no
reading to hold us to and is therefore `ambiguous`. Four pages entered this bucket that way, off the
contradicted list, with no pixel moved. **They are diagnosed on arrival** — `AMBIGUOUS_DIVIDED_CONSENSUS`
carries the reading of each and is chained into `diagnosed_ambiguous()` — so the queue this item is
about did not grow, and that is deliberate: a verdict rule that emptied a watched list into an
unwatched one would be this item's own failure mode. The bucket is 4 larger and the *undiagnosed*
population is unchanged.

**And every one of the gate's seven verdicts is held by name now.** This item carried the other
unwatched buckets until the five-hundred-and-seventy-ninth session: `no render` was diagnosed and
ratcheted in the five-hundred-and-seventy-fifth (ADR 0410), and `not comparable` and
`reference geometry` in the five-hundred-and-seventy-ninth (ADR 0414) — the reading of all fifteen
pages is `doc/oracle-and-corpus.md` §3e and the groups are `oracle.rs`'s `NOT_COMPARABLE_*` and
`REFERENCE_GEOMETRY_*`. ~~So what is left of this item is the two standing halves above and nothing
else: the equality ratchet, and step 7's ink sweep after a round that moves pixels.~~

**That last sentence was true of one population, and the six-hundred-and-ninety-second session gave
the gate a second** (ADR 0541). `doc/corpora/pdfbox`'s 143 pages had never been through any raster
gate, and 63 of them were `ambiguous` with no diagnosis, so this item had a queue again — the
first since the three-hundred-and-seventy-ninth session and not one page of it a regression. The
distinction the emptiness was hiding is worth keeping: an empty bucket was a fact about
`doc/pdf.js`, never about this reader. **The six-hundred-and-ninety-fourth took the whole 63**
(ADR 0543), so it is now a fact about two populations rather than one, and a corpus added tomorrow
is expected to fill it again. `tools/state.sh` counts; the census in `doc/oracle-and-corpus.md` §2e
prints the list with each page's metrics beside it.

**The shape it was filed under was a hypothesis and two thirds of it was wrong, which is this
item's method arriving as a lesson about itself.** The queue's own description said *62 of the 63
fail the differing fraction and the structural similarity while sitting well inside the mean and
the worst tile*. Counted off the gate's lines: the differing fraction fails on all 63 and is the
worst ratio on 59 of them, the similarity on 47, **the mean on 47 and the worst tile on 4** — the
last two where the sentence said none. Nothing acted on it and the correction cost nothing; what it
is worth is that a shape read off a listing is a hypothesis in exactly the way the sentence below
says, and the round that writes one owes the count.

So what is left of this item is two things: the equality ratchet, and step 7's ink sweep after a
round that moves pixels.

## Why this is work rather than a caveat

`ambiguous` means "no two references agree closely enough for anybody to be called wrong". It is
the right verdict for the *ratchet* to reach and it is not the same as "right". `issue7406.pdf`
drew a JPEG cyan-on-black inside an `ambiguous` verdict for as long as anybody looked, and it is
correct now, and **nothing announced either event**. That is the bucket in one sentence:
unwatched in both directions. The project owner's judgement in the hundred-and-seventy-fifth
session is that the tree is far enough along for this to be the work.

## The instrument, in three parts

- **`AMBIGUOUS_*` groups** — a page with a written diagnosis, held by name, exactly the shape the
  contradicted list has had since the sixth session. A name that stops being ambiguous fails the
  build, because a diagnosis that outlives what it diagnosed is this project's oldest failure.
- **`tests/ambiguous_undiagnosed.txt`** — the rest by name, `include_str!`d and held to equality.
  A page arriving in it *used to agree*, which is the regression nobody could see before; a page
  leaving it has been fixed or diagnosed. Data rather than a `const` because the argument for
  each name is that there is not one yet.
- **A ranking the gate prints itself**, of the ten undiagnosed pages we sit furthest from the
  **nearest** reference on. Not the furthest: the printed per-page number is our distance from
  the *worst* reference, and on nineteen JBIG2 pages that is a `mupdf` which drew a black
  rectangle. `Distance::nearest` is the number that accuses us; `Distance::furthest` beside it
  says whether the references are the ones disagreeing.
- **A second ranking, of the verdict rather than of us**, since the five-hundred-and-eighteenth
  session: `rank_the_manufactured_ambiguity`, the ten ambiguous pages on which the **closest two
  voting references** sit furthest outside the bound. Every other instrument in this file
  measures our page; this one measures how hard the consensus failed, and until it existed
  nothing did — which is trap 9's fifth shape stated as an instrument rather than as a caution.
  Below 1 is impossible here by construction; a little above 1 is trap 12's arithmetic; twenty is
  a renderer that failed.

  **Its two columns were two instruments and the line invited you to divide one by the other**,
  which the seven-hundred-and-forty-first session found by measuring (ADR 0643). The pair's number
  is `outside_by` — all four bounds — and the `ours` beside it is `Distance::nearest`, which is
  three of them. On `jp2k-resetprob.pdf` that reads *35.12 between them, 5.03 ours*, as though the
  references disagreed seven times more than we differ from the nearest of them; in one unit the two
  numbers are **35.12 and 32.42**, eight percent apart. Ours is printed in both units now and the
  count under the list is taken in the pair's. **A number is only comparable with one the same
  instrument produced**, which is this file's oldest rule about ink arriving as a rule about ratios.

- **A third ranking, of step 1's own shape**, since the seven-hundred-and-forty-fourth session:
  `rank_the_pages_we_are_alone_on`, the pages we sit further from every reference on than the
  closest two references sit from each other, counted in both units and **ordered by the
  three-measure ratio** — because that is the unit session 518's hand-taken reading was in, and
  the four-measure reading names seven pages in ten. `consensus_missed_in_three_measures` is the
  number that made it possible: until it existed the pair had no figure in `Distance`'s unit and
  the queue could be counted but not opened. ADR 0647, and the paragraph below is how to read it.

  **Its numerator has to be outside a bound**, since the seven-hundred-and-fifty-first (ADR 0663):
  a page whose nearest reference sits inside all three of them is one that reference would have
  *accepted*, and a ratio taken there ranks how closely the references agree. The pages that
  requirement drops stay printed as a count, and the count under the list is now the sublist rather
  than the population — how many of the listed pages have a closest pair inside all three bounds
  while we are outside one.

  **And the rows carry a mark saying where the list's head ends**, since the
  seven-hundred-and-sixty-first (ADR 0684). 751's requirement is a threshold against the class
  *floor*, which is what `pdfref::decide` returns because no consensus formed — the weakest bound
  in the gate, and most of this list is outside it while the references are further outside it than
  we are. `[widened: outside]` marks the pages where our nearest is outside the bound
  `Judgement::CORPUS` would have set **from that pair's own spread**, twice it and floored, which is
  the bound the gate applies on every page where a consensus does form. Below the mark a consensus
  at that spread would have accepted us, so the page is alone against a constant. The list prints
  ten rows or the whole marked head, whichever is longer, because a count naming a head a reader
  cannot open is the defect ADR 0643 found in two columns.

  A ratio of 2 or more is the readable sufficient condition for the mark and not the same test: both
  sides of the printed ratio are a maximum over three measures, so a page whose *worst* measure and
  the pair's *worst* measure are different measures can be marked at a ratio well under 2 — two of
  the marked pages are.

  **And every row now says which measure each of its two halves is, and against which renderers**,
  since the seven-hundred-and-sixty-fourth (ADR 0688). `outside_by_in_three_measures` returned a
  maximum and threw away the name where `worst_ratio` keeps it, so a note could price a mechanism
  on a row of this list without ever saying which of the three the number was — and **a mechanism
  that accounts for a mean does not thereby account for a structural similarity**. The count under
  the list says how many rows divide one measure by another; on the rest the printed ratio is a
  ratio of like for like and may be read as one. This is ADR 0497's sixth criterion — the question
  `--bin unpriced` asks of a *contradicted* page's failing bound — asked of a ranking instead, and
  the gate has to print it because no per-page line carries it: that line is our render against the
  consensus's **worst** member and this ratio is taken against its **nearest**.

## What the two rankings say when read together

**Taken in the five-hundred-and-eighteenth session over all 786.** The gate's own ranking is in
bounds (`outside_by` over all four measures); the by-hand sweep beside it is mean absolute
difference over all ten renderer pairs plus each panel's ink, from artefacts already on disk with
nothing rendered again. Two orderings, and the first is the new one:

| ordering | what its head is |
|---|---|
| the closest **voting pair**, in bounds — what the gate prints | `jp2k-resetprob.pdf` **35.12** (ours 5.03 in three measures, 32.42 in four), `issue5475.pdf` **31.63** (ours **0.00** in either), `bitmap-refine-tpgron.pdf` 28.91, then seven more `bitmap-*-refine` pages at 28.58 |
| our nearest **over** the closest voting pair, in levels of 255 | `issue4260_reduced.pdf` **8.27**, `bug1743245.pdf` **5.34**, `bug1671312_ArialNarrow.pdf` 3.69, `bug766086.pdf` 2.98 |

**Two JPEG 2000 pages at the very head, and that is the `objdump` finding arriving as a
measurement.** All three voting references link the same `libopenjp2.so.7`, so on a `JPXDecode`
page they are one decoder with three callers — and on `issue5475.pdf` the three of them span 9 to
19 of 255 among themselves while **ours and `mupdf` are 0.0002 apart over 262 144 pixels**. Shared
code manufacturing the *absence* of a consensus without the shared code having failed, which is
trap 9's fifth shape one step further than the JBIG2 pages that named it. What settles that page
is `tests/jpeg2000.rs`, which asks ISO/IEC 15444-5's own software and no renderer at all.

Below them the whole of `AMBIGUOUS_SHARED_JBIG2_DECODER`, which is the instrument reproducing by
itself the finding it was built from: on `bitmap-refine-tpgron.pdf` `mupdf` paints the sheet black
at ink **255.000** and `ghostscript` paints it white at **0.000**, the full range apart.

**The second ordering's head is not an accusation, and that is the calibration to keep.** A ratio
above 1 means we sit further from every voting reference than the closest two sit from each other
— the shape step 1 reads as *we are alone* — and it is true of **56 of the 786**. The top two are
pages where a clause says we are right and the closest pair agrees by **sharing a departure**:

- `issue4260_reduced.pdf` (`AMBIGUOUS_ZERO_AREA_FILL`) — ink ours **19.79**, `hayro` 19.83,
  `ghostscript` 6.30, `poppler` 3.52, `mupdf` 2.17, reproducing that group's own note to a
  hundredth. §10.7.4 asks for the pixel to be painted "no matter how small the intersection is";
  the closest voting pair is `poppler` and `mupdf` at 1.92, and what they agree about is painting
  a fifth of it.
- `bug1743245.pdf` (`AMBIGUOUS_STROKE_ADJUSTMENT`) — the closest voting pair is `mupdf` and
  `ghostscript` at **4.12** where every other pair is 22 to 28, and the thing those two share is
  ignoring §10.7.5's "the stroke shall be rendered as a single-pixel line".
- `bug766086.pdf` (`AMBIGUOUS_LINK_BORDER`) — the same two at **3.03**, agreeing about drawing no
  link border for two unrelated reasons, which is trap 9's fourth shape. Ours and `poppler` draw
  it and their inks agree to **0.09 of 255**; the other three are 7.5 lighter, which is the
  border, and the four-panel strip shows exactly that.

**So a high ratio means "the closest two references agree through a gap" at least as often as it
means anything about us.** Read it with the picture, never alone.

**The 56 was reproduced in the gate's own arithmetic in the seven-hundred-and-forty-first session,
and the reproduction is what settled a different question** (ADR 0643). That figure was taken by
hand, in levels of 255, over the 786. Computed in *bounds* over all 836 of this run's ambiguous
pages — our nearest against the closest voting pair, both in `Distance`'s three measures — it is
**58**, which is 6.9% of the population against that session's 7.1% of a smaller one. Asked the same
way over **four** measures it is **583**, which is seven pages in ten. That is the whole argument for
leaving the differing fraction out of `Distance`: the bound `doc/todo/12` is about is one the
references miss by as much as we do, so a reading that includes it says *we are alone* about most of
the bucket and therefore says nothing. **A shape read off one instrument is a hypothesis until a
second instrument produces it**, and here two did — the gate now prints the four-measure count under
the ranking, ~~and the three-measure list of 58 has never been read as a list.~~

**It has now, and what it turned out to be is a lesson about the ratio rather than about a page**
(ADR 0647). The three-measure list is **48 of the 804 complete pages**, printed and ordered by the
gate itself since the seven-hundred-and-forty-fourth session. Three things came out of reading it:

- **Every one of the head's ten is a documented departure** — `issue11403_reduced.pdf`,
  `bug766086.pdf`, `bug1743245.pdf`, five pages of `freeculture.pdf`, `issue4260_reduced.pdf` and
  `issue16224.pdf` — each held by an `AMBIGUOUS_*` group whose argument is the reason it is there.
  That is the corrected instrument agreeing with this tree's own record, which is what a corrected
  instrument's first reading should mostly be.
- **The ratio has no floor, and on most of the list neither number is outside anything.** On **31 of
  the 48** the closest pair sits inside all three bounds — the page is ambiguous on the differing
  fraction alone — and on **22** our own nearest is inside them too. There the ratio ranks a page
  higher the more closely the references agree, not the further away we sit. The gate prints both
  counts under the list for that reason. **The head is the sharpest instance**:
  `issue11403_reduced.pdf` is 9.06×, ours 0.51 over 0.06, and its verdict line says
  `differing alone, 6.24%/5.00%` — a page whose disagreement is *invisible to the three measures the
  list is computed in*. Both units have a blind spot and they are different ones, which is this
  file's "read it with the picture, never alone" arriving as arithmetic.
- **The sublist to open is the nine where we are outside a bound and the closest pair is inside**:
  `bug766086.pdf`, `freeculture.pdf` 315, 322, 323, 329 and 333, `issue16224.pdf`, `endchar.pdf`
  and `issue12337.pdf`.

**The second of those three was a question and it is answered: the list requires our own number to
be outside a bound now** (ADR 0663). The reasoning is in `rank_the_pages_we_are_alone_on`'s doc
comment and the short form is that the threshold is not arbitrary and not the references': on an
ambiguous page `pdfref::decide` returns the **class floor** unwidened, because widening is a
consensus's and there is no consensus — so *outside 1* means outside the fixed tolerance for this
page's class, the same constant for every text page in the pool. Below it, the nearest reference
would have accepted our page had it been in a consensus, and a page somebody accepts is not one we
are alone on.

What the cut costs is measured rather than assumed. **The list loses exactly the pages where our own
nearest was inside, the head loses exactly one of its ten, and the one it loses is the page 744
named as the defect** — `issue11403_reduced.pdf`, which led at 9.06× on ours 0.51 over 0.06 with a
verdict line reading `differing alone`. What rises into the tenth place is `endchar.pdf`, which is
in the sublist. Nothing else in the printed ten moves, no page that was invisible becomes the head
(ADR 0349's warning), and the pages dropped stay printed as a count. **The count underneath now
names the sublist directly**: how many of the list have a closest pair inside all three bounds while
we are outside one, which is this queue.

**In four measures the same requirement changes nothing, and the reason is the asymmetry itself**:
the closest pair is above 1 in four measures on every ambiguous page by construction, so *ours >
theirs* already implied *ours > 1* over there. The three-measure denominator has no such floor, and
that is the whole of what went wrong.

**And the five book pages were measured rather than handed to their population's argument.** Three
ladders on page 315 — ours 11.8908 → 11.9540 → 11.9855, `poppler` 11.8704 → 11.9478 → 11.9592,
`mupdf` 11.9611 → 11.9979 → 11.9914 at 1×, 4× and 8× — converge with ours **between** the other two
at every rung and all three within 0.032 of 255 at the limit; on all five our 72-dpi ink is inside
the references' own spread to 0.09 of 255. **What lifts them is the denominator, and it is trap 9 in
a place nothing had priced it**: over the book's 321 compared pages, `poppler` and `mupdf` — the two
voting references that share `libfreetype`, where `ghostscript` links its own copy — are the closest
pair on **9 of the 11 book pages that reach this list** and on **7 of the other 310**, and their
own median MAE is **724** over those 11 against **1760** over the rest. Shared code manufacturing
an agreement in a ratio's *denominator* is the same trap seen from the other end, and the page it
lifts is one every instrument agrees is fine.

**And the other four of the nine were opened, and they are four different shapes rather than one**
(ADR 0663). The reading of each is beside its own group in `oracle.rs`; what belongs here is the
answer to the question 744's finding poses, which is *whether the sublist is one mechanism*:

| page | our number is | the divisor is | and that is |
|---|---|---|---|
| `bug766086.pdf` | 2.58, the **similarity**, against `poppler` | `mupdf` + `ghostscript` at 0.45 | trap 9's *shared gap*: neither draws a link border, for two unrelated reasons |
| `issue16224.pdf` | 1.13 against `mupdf` | `poppler` + `mupdf` at 0.41 | trap 9's **tenth** mechanism: the `libfreetype.so.6` pair, 7.5× closer to each other than either is to `ghostscript` |
| `endchar.pdf` | 1.97, the **mean**, against `mupdf` | `poppler` + `ghostscript` at 0.83 | neither — four ladders now put the coverage inside 0.153 of 255 and what is left is §10.7.4's glyph scan conversion on a 15 × 34 raster |
| `issue12337.pdf` | 1.12, the **mean**, against `ghostscript` | `mupdf` + `ghostscript` at 0.88 | neither — and the *numerator* is the finding |

So one of the four is the mechanism 744 measured on the book, one is a different bullet of the same
trap, and two are not that trap at all. **A sublist is not a diagnosis**, which is the general form
of it: the shape *we are outside and they are inside* is worth opening precisely because what is
behind it differs page by page.

**Two of the four were priced by taking the mechanism out of the document**, which is trap 9's own
instrument (`doc/traps/oracle-and-references.md`, the `visibility_expressions.pdf` bullet) pointed at
our own accusation instead of somebody's excuse. On both, `/Annots` was replaced in place by an empty
array of the same byte length, so the cross-reference table still resolves, and all four renderers
were re-run:

- On `bug766086.pdf` our nearest falls **2.58 → 0.43** — inside every bound — while the pair the
  ratio divides by is **byte-identical to the digit**, because neither of those two draws the
  annotation at all. Numerator and denominator are the same clause, counted twice, in opposite
  directions.
- On `issue12337.pdf` our nearest falls **1.12 → 0.61** while the divisor moves 0.88 → 0.89, so
  without the annotation the page is not on the list at all. And the annotation is the one thing on
  the page a clause reaches: a `/Highlight` with no `/AP` whose `/QuadPoints` and `/Rect` are the
  same rectangle, where ours is the only one of five renderers whose yellow stays inside it —
  `poppler`, `mupdf` and `ghostscript` bulge 17 to 27 device columns past each end and `hayro` draws
  none.

**The instrument to copy is the removal.** A ratio's two halves can be the same mechanism, and the
only way to see that is to make the mechanism unable to act and re-measure *both*.

**And `bug766086.pdf`'s row was taken one step further in the seven-hundred-and-fifty-sixth session,
which is what the row was for** (ADR 0675). Whether the border ours and `poppler` disagree about by
one pixel goes inside `/Rect` or across its edge is a question for §12.5.4 and not for a ratio, and
the clause answers it: the border "shall be drawn completely inside the annotation rectangle", so
its path is inset by half its width, and there is no width-1 case anywhere in the subclause. Ours is
that. `poppler` puts the path *on* the boundary — five units outside on all four sides at a width of
10, measured, and one pixel on two of four at a width of 1 because it snaps a thin line to the grid.
`crates/pdf-model/examples/border_overhang_census.rs` says it over a population rather than a
witness, and the exemption on this page is therefore **a documented departure of the reference's**.

Two things that round found by reading a *neighbouring* page of the same clause are worth the
pointer. Our own oversized border was drawing the wrong shape and three documents said otherwise
(ADR 0674, and `doc/traps/pixels-and-rasterisers.md` trap 1's new paragraph). And fixing it moved
`bug1552113.pdf` from no printed list to **second on this one**, because the two references nearest
us there are nearest by drawing no border at all — the numerator moving *away* from a shared gap
looks exactly like a regression on this ranking and is the opposite of one.

## The bucket is two camps, and the camp that votes is the one that cannot agree with itself

**Measured in the five-hundred-and-eighteenth over all 786 pages**, every one of the ten renderer
pairs:

| population | closest pair is `ours + hayro` | median ours-to-`hayro` | median closest voting pair | median widest voting pair |
|---|---|---|---|---|
| all 786 | **651** | 1.92 | 5.34 | 9.24 |
| the 670 judged as text | **612** | 1.94 | 5.39 | 9.28 |
| the 116 judged as vector | 39 | 0.30 | 2.09 | 8.03 |

`hayro` is a separate interpreter written by other people; what it shares with this tree is
`skrifa` and not an interpretation. **On nine text pages in ten it is closer to us than any two of
the three voting references are to each other**, and it is the one reference that may not vote.
That is trap 9's third shape — three C renderers, one FreeType — measured over a population
instead of asserted from one `ldd`, and it says what the verdict `ambiguous` is *made of* on a
text page: not five renderers scattered, but two camps, with the voting camp internally 2.8×
wider than the gap between us and the renderer that abstains.

**It is not evidence that we are right, and the direction of inference matters here more than
anywhere.** Agreement with `hayro` is worth nothing and `Reference::independence` says so; what
the number establishes is that the *absence of consensus* on these pages is a property of the
references, so an `ambiguous` verdict carries no information about our page unless something else
supplies it. The closed forms in steps 5 and 6 are that something else, and this table is an
argument for taking them rather than a substitute for one.

**The instrument's own assumption was checked rather than assumed.** Our panels and `hayro`'s
carry an alpha channel and the three C references' do not, which is step 5's `-alpha off` trap and
would manufacture exactly this result if any panel were transparent. All **4535** panels on disk
were tested and **not one pixel is less than fully opaque**, so dropping the channel is the same
operation as `-alpha off` here. A population result that the instrument's own known defect would
also produce has to have that defect ruled out before it is written down.

## A group's diagnosis can migrate to the group above it, and three had

**Found in the five-hundred-and-eighteenth, and it is this file's oldest rule failing in a
direction nobody had watched.** A group is an array of page names with its argument in the doc
comment above it, and Rust attaches a doc comment to whatever item follows. So an edit that
inserts a new `const` between an existing comment and the const it documented welds two notes
together and leaves an array with **none** — invisible to `rustc`, to `clippy` and to every gate,
because nothing is malformed.

It had happened three times, to **`AMBIGUOUS_GLYPH_COVERAGE` (3 pages), `AMBIGUOUS_MASKED_BLUR`
(1) and `AMBIGUOUS_OURS_ON_THE_LIMIT` (3)**. Seven pages whose argument was written down, in this
tree, filed above a group it does not describe — so a reader of `AMBIGUOUS_OUTLINED_TEXT` got four
pages' worth of diagnosis for a group of one, and three groups said nothing at all while
"0 undiagnosed" was true by the ratchet's definition. All three are moved back, and
`every_group_of_pages_carries_a_diagnosis_naming_one_of_them` fails the build on the next one: for
every non-empty group, the comment above it must name at least one document in it. Deliberately
weak — a group of 370 pages cannot name them all and several notes cite a neighbour's page on
purpose — and it catches the whole of this failure, because a welded comment names none of the
array under it.

**The lesson is the sharper half.** "0 undiagnosed" counts names against a list; it cannot see a
diagnosis that is present, correct and pointing at the wrong pages. A count is not a picture, one
directory over.

## How to take one

1. **Read the ranking**, and prefer a page whose two numbers are close — that is the shape that
   says *we* are alone. `issue7229.pdf` sat at 77 from the nearest with the two nearly equal,
   and it was drawing the wrong page.

   **The gate ranks the other *alone* shape by itself now** — our nearest over the closest pair of
   references, which is the comparison *What the two rankings say when read together* is about —
   and every page on it has our own nearest outside a bound, so the ratio's numerator is an
   accusation rather than an artefact. `rank_the_pages_we_are_alone_on`, ADR 0647 and ADR 0663.

   **Read the line under it before reading the ratios**, and read it as two different things. The
   first count is the sublist — the closest pair inside all three bounds while we are outside one,
   which is this step's own shape at its sharpest. The second is the pages the list no longer
   prints, where we are further from everybody than the pair is from itself *and* inside every
   bound; they are not a queue, and a round that wants them has the count to ask for.

   **And the ratio's two halves can be the same mechanism**, which is the newest thing this step
   knows and no number on the line can say. `bug766086.pdf` is the standing witness: with its link
   annotation removed our number falls from 2.58 bounds to 0.43 while the divisor is byte-identical,
   because the two references it is taken between do not draw the annotation at all. **Take the
   mechanism out of the document and re-measure both sides**, before reading a high ratio as ours.

   **Where to stop, which is a criterion rather than a budget** (ADR 0684). Work down the marked
   rows — `[widened: outside]`, our nearest outside the bound a consensus at the closest pair's own
   spread would have set — and stop at the first unmarked one. Below the mark the gate's own
   widening would have accepted us, so what a high ratio there measures is how closely two
   references happen to agree, and the answer is in the divisor rather than in the page. The
   seven-hundred-and-sixty-first session read the mark's head to the end and every page of it is
   now priced against the *measure* its own number is taken on; the two the mark added to the
   readable cut, `freeculture.pdf` page 1 and `copy_paste_ligatures.pdf`, are both pages where our
   worst measure is the structural similarity and the pair's is the mean.

   **And a mechanism is only priced when it is priced in the measure the row is ranked on**, which
   is the newest thing this step knows and is what the row's own `[measure v reference]` brackets
   are for (ADR 0688). The seven-hundred-and-sixty-fourth session read the marked head that way and
   the sharpest result is `bug1743245.pdf`: its note argues two camps over §10.7.5's single-pixel
   rule in **whole-page mean grey**, and the row's 31.43 is a **structural similarity against
   `poppler`** — a renderer in our own camp. Removing the mechanism from the document (`/SA true`
   renamed to `/S1 true`, eight bytes for eight) moves our nearest to **2.62 against `mupdf`** and
   takes the page off the list, while all four references render byte-identically, so no reference
   on that page reads the entry at all. The mechanism is real, the note is right about it, and it
   explains the *divisor*; what explains the numerator is the other half of the same clause, which
   the note records as a departure and never joined to a number.

   **A page below the mark is still worth opening when its own note disclaims it**, and that is the
   one exception. `freeculture.pdf` page 255 sat at 1.35× under a group note saying in as many
   words that whatever the page is, it is not that group's diagnosis — while a table four hundred
   sessions older in *the same comment* had measured it and cleared it on ink. Both were true: the
   ink was right and answers *how much*, and the page's whole disagreement is *where*. It is
   `AMBIGUOUS_IMAGE_REDUCTION`'s (ADR 0685). **A disclaimer inside a group note is a page nobody is
   holding**, and `grep` finds those in a second.
2. **Read the file's bibliography before opening anything.** Every pdf.js fixture is named after
   the issue that introduced it — `issueNNNN…pdf` → `github.com/mozilla/pdf.js/issues/NNNN`,
   `bugNNNNNNN…pdf` → `bugzilla.mozilla.org/show_bug.cgi?id=NNNNNNN` — and the issue says what
   the file was added to prove. A pair with a common stem is an A/B the corpus built for you
   (`issue7891_bc0` and `issue7891_bc1` differ in `/BC [0 0 0]` against `/BC [1 1 1]` and in
   nothing else). Two cautions: the issue describes **that reader's** defect, and an issue is
   evidence about a *file*, never about the clause.
3. **Open the side-by-side.** `<target>/tmp/oracle/<stem>/p<n>/` already holds our render, each
   reference's, a four-panel strip and a heatmap per reference. The picture has explained every
   page it was pointed at, and twice it named a defect the numbers could not (a shading painted
   as a square; a photograph rendered black).

   **`<stem>-p<n>-ours.png` is not our page size, and two diagnoses were built on believing it
   was.** It is our raster *after* `normalise::to_common_size` cropped it to the smallest size any
   voting reference produced; the reference PNGs beside it come from the render cache and are not
   cropped. So on a page whose box is fractional the listing shows ours at 595 next to a `poppler`
   at 596 and reads like this tree rounding down — and `TargetSpec::for_page` rounds *up*, so our
   own render is 596 and `ghostscript` is the one that truncates. `CONTRADICTED_PAGE_ROUNDING` held
   two pages on that misreading for four hundred sessions (ADR 0279). **The only place our page
   size can be read is a render of our own**, `examples/render_at`, which is the same rule as
   trap 1 one file over: the instrument that reports a thing is not the thing.
4. **Ask what the page is made of before measuring anything.** `cargo run --release -p pdf-model
   --example open_one -- <file> 1` prints the command count. **One command has meant one image
   three times running** — `freeculture.pdf`, `issue5747.pdf` and `issue13372.pdf` — and
   `pdfimages -list -f 1 -l 1 <file>` then names it in a second. **Zero commands means a blank
   page reported complete**, which is the worst thing this bucket hides and has been a real
   defect every time (`issue13372.pdf`, `issue8372.pdf`, `issue13316_reduced.pdf`).
5b. **Ask the renderer under test the same question at rising resolution too.** Step 6 uses a
   *reference* at 8× to find the geometry; running `cargo run --release -p pdf-model --example
   render_at -- <file> <page> <scale> <out.png>` and measuring the same way says whether the
   difference is our scan conversion or our *shapes*. `bug1538111.pdf` is where it paid: our ink
   is 1.48 at 1×, 4× and 16× while `poppler`'s limit is 2.24, so the two draw different marks and
   no amount of anti-aliasing argument was going to explain it. (They are markup annotations
   whose artwork §12.5.6.10 does not state at all.)

5. **Measure with a closed form where the clause states one**, and with pairwise distances only
   as corroboration. The ink is

   ```sh
   magick <png> -alpha off -channel R -colorspace Gray -format "%[fx:(1-mean)*255]" info:
   ```

   and `magick compare -metric MAE a.png b.png null:` is the pairwise number.

   **`-alpha off` is not optional and this file said so too late.** Our renders and `hayro`'s
   carry an alpha channel; `poppler`'s, `mupdf`'s and `ghostscript`'s do not. Without it
   `-colorspace Gray` averages alpha in as a second channel and returns **exactly half** the ink
   — so a comparison between our panel and a reference's compares half of one number with all of
   another, and the two renderers that "agree" with us are the two whose *file format* matches
   ours. Session 161 found this and wrote it in `CONTRADICTED_GLYPH_EDGES` and in the handover's
   Habits; the recipe here was not corrected, and the two-hundred-and-second session followed the
   recipe and drew two wrong conclusions from it (ADR 0163). **A lesson recorded in the place it
   was learned and not in the place it is used has not been recorded.**

6. **Where the difference is scan conversion, the closed form is the same page at eight times the
   resolution.** Ink is a geometric quantity and a renderer's departure from it shrinks as the
   pixels do, so `pdftoppm -cropbox -r 576` measures what the page's marks actually cover — no
   reference is being trusted, because the same renderer is being asked at two resolutions and
   only the *limit* is used. `bug1799927.pdf` is where this paid: at 72 dpi the five renderers
   span 5.94 to 13.40 and the limit is 10.8, which says which of them is measuring area.

   **`-cropbox` is not optional either, and it is `-alpha off`'s twin.** `pdftoppm` renders the
   **`/MediaBox`** by default; the oracle, `mutool draw` and this tree all render the
   **`/CropBox`**. On a document where the two differ the comparison is between two different
   pages, and the ink is wrong by the ratio of their areas — on `freeculture.pdf` that is 1.378,
   so a ladder taken without the flag put `poppler` at 9.10 against our 12.18 and would have
   manufactured a 34% defect on four pages that agree to **0.03 of 255** (the
   two-hundred-and-thirty-third session, `AMBIGUOUS_DENSE_TEXT_AT_BOOK_SIZE`). The tell is the
   raster's size: `magick identify` every panel before believing any number, and if the
   dimensions differ the measurement has not started yet. `mutool draw -r N` needs no flag.

   **And a band of rows is a hypothesis about what is in it**, which the three-hundred-and-forty-first
   session learned by paying for it. A strip picked by eye off the 1× page as "the empty cells,
   rules only" turned out to be six rows of caption, one of rule and six of white, and the ink it
   produced said this tree over-paints a table rule by 14%. It does not. **The instrument that
   checks a band is a per-row ink profile**, which costs one `magick -resize 1xH!` per panel and
   is the same shape as step 7's per-row heatmap:

   ```sh
   magick <png> -alpha off -colorspace Gray -resize 1x<height>! -depth 8 txt:-
   ```

   Read row by row it also *localises* a difference, which no ink table can: on
   `issue9972-1.pdf` every one of the twelve worst rows was within a fifth of a pixel of one of
   the page's four horizontal rules, and that is what turned "a dense form somewhere in the band"
   into `AMBIGUOUS_TABLE_RULE_EDGES`. Take it before taking a ladder, not after.

   **And the assumption inside it is checkable, because it has failed once.** The step assumes
   the reference *converges on the geometry* as the pixels shrink. On `issue2177.pdf`, a page of
   §8.7.3 tiling patterns, `poppler` goes 34.15 → 18.03 → 16.32 from 72 to 2304 dpi — its
   strokes get thinner rather than its edges getting sharper — and taking its limit would have
   said all five renderers paint two to three times the geometry. Ours is flat at 36.75, 37.37,
   37.25, 37.20 across four scales and `mupdf` at 8× is 37.20, which is the answer.
   **Take a second renderer's ladder, or ours beside one**: a limit is only a limit if the thing
   taking it is converging, and one ladder cannot tell convergence from drift.

## 7. Sweep the whole bucket for the one defect a distance cannot name

Steps 1 to 6 take a page at a time off a ranking. **The ranking cannot see missing content**,
because a page that draws less than everybody is not necessarily far from anybody: `issue19634.pdf`
sat at 0.85 with a quarter of its marks absent. What sees it is one number over the artefacts
already on disk — **our ink minus the lightest reference's** — and the sweep costs three minutes
because nothing has to be rendered again:

```python
# for every ambiguous page, over <target>/tmp/oracle/<stem>/p<n>/
live = [ink(r) for r in (poppler, mupdf, ghostscript, hayro) if ink(r) > 0]
gap  = ink(ours) - min(live)
```

sorted ascending. A large negative gap is content we are not drawing; a large positive one is
content nobody else is.

**Three corrections to that loop, all from the two-hundred-and-sixty-fifth and -sixth sessions,
and each of them changed what the sweep found:**

1. **Drop a reference that drew nothing before taking the minimum.** A blank is not a lower bound
   on the geometry, and leaving it in turns another program's failure into our surplus: four pages
   came back at +21 to +29 of 255 because `mupdf` draws nothing on `colorspace_sin.pdf`, `_cos`
   and `_atan`, and `hayro` nothing on `issue2840.pdf`. What the *positive* side is good for is
   exactly that — finding a reference that failed. Thirty-five such pairs across the bucket, most
   of them `ghostscript` on the JBIG2 fixtures.
2. **Run it over every ambiguous page and not only the undiagnosed ones.** Diagnosing a population
   takes its pages off `ambiguous_undiagnosed.txt`, and if the sweep reads that file then
   diagnosing 364 pages in one session removes 364 pages from the only instrument that sees
   content this tree is not drawing. The list of names to sweep is the gate's own output — every
   line it prints as `ambiguous` — which is 787 rather than 100.
3. **Read the result beside the corpus's incomplete list.** A page this tree *reports* is expected
   to be light: drawing less ink is what the report says, made visible.

**The full run, over all 787: twenty names at or past −1, seventeen of them documents this tree
already calls incomplete**, and the other three diagnosed and consistent with their diagnoses —
`issue16038.pdf` at −6.70 (`AMBIGUOUS_TILING_CELL_CLIP`, whose own note measures the interior
coverage 13% short), `issue12295.pdf` at −1.71 (`AMBIGUOUS_EVERYONE_OVER_THE_GEOMETRY`, where
every renderer paints more than the geometry and ours least, so a negative gap is the finding
rather than a defect) and `issue7821.pdf` at −1.00 (`AMBIGUOUS_GRADIENT_QUANTISATION`). **Nothing
unexplained anywhere in the bucket**, which is also the check the two long books' population
argument needed.

**The negative head has a name now, and it had gone unread for hundreds of sessions.** Every run
below prints `issue12418_reduced.pdf` −19.4 at the top and every one of them passes over it,
because the label `[incomplete]` reads as *explained*. It is not the same as *diagnosed*, and the
five-hundred-and-ninety-eighth session opened the top of the list instead: **eleven of the negative
tail's incomplete names are one cause**, including the top three. Each is a Type 0 font with
`/Encoding /Identity-H` over a `CIDFontType2` with no font program, no `/ToUnicode` and
`/Ordering (Identity)` — the combination §9.7.5.2 forbids in as many words ("The Identity-H and
Identity-V CMaps shall not be used with a non-embedded font"), so this tree draws nothing and says
so while the four references produce four different strings from the same bytes. **ADR 0433 has the
clause, the eleven names, what each reference guesses, and the measurement that says the pages
would be `ambiguous` whatever we drew.** Read it before re-deriving the head.

**Re-run whole in the eight-hundred-and-sixth**, over every page the gate prints as `ambiguous`,
after the round that gave a patterned stroke its own region (ADR 0735) had gone in and while
nothing in this round could move a pixel. On this file's own recipe (`-alpha off -colorspace
Gray`): **19 at or past −1, 16 of them documents this tree calls incomplete**, and on the complete
documents `issue16038.pdf` −5.642, `issue12295.pdf` −2.362, `issue14297.pdf` −1.135, then
`issue7821.pdf` −0.957, `jpx_smaskindata.pdf` −0.840 and nothing past −0.535 — **three names past
−1 and all three diagnosed**, the alarm holding again. The head is the eight-hundred-and-second
session's to the thousandth on both of its entries.

**And the annotation four of the runs below carry on that head is spent**: they gloss
`issue16038.pdf` as "13% short by its own note", which was that note's interior-coverage figure
before ADR 0155 and ADR 0213. Re-measured in this round the two squares are within 3% of the
geometry at the page's own scale and within 0.2% at 24×, and the page's whole ink is **313.02
against a corrected closed form of 313.12** — the 316.29 that note carried counts the twenty rules'
ends twice, once as rule and once as the border they run under. So the head of this ranking is the
references' excess rather than our shortfall, which the paragraph below already argued and had
never held a limit of ours to. ADR 0738.

**And the head's number is a measurement of the *recipe* as much as of the page, which cost this
round twenty minutes.** The same run taken with a greyscale of one's own — Rec601 luma over the
three channels, which is what a reader reaches for — puts `issue16038.pdf` at **−5.394** and
`issue12295.pdf` at −2.364. The second is unmoved and the first is a quarter of a level out,
because that page's rules are pure **blue** and every greyscale weights blue differently, while
`issue12295.pdf` is near-black and weights the same in all of them. This file already says an
absolute value differs between recipes on a coloured page; what it did not say is that the
*difference between two renderers* does too, and a quarter of a level is the size of the movement
this sweep is watched for. **Take the head with `magick` and the recipe as written**, or the
comparison against the last round's number is a comparison of two instruments.

**Re-run in the two-hundred-and-sixty-fifth over the tail, and it produced a defect** —
`rc_annotation.pdf` page 1 at **−1.783 of 255**, past the −1 this file names as the alarm. The
page is one text annotation with `/Rect [50 50 50 50]`, this tree drew **nothing** for it, and
§12.5.6.4 says a text annotation is "attached to a point" and "shall appear as an icon". It sat at
0.73 from the nearest reference — a nearly blank page resembles a nearly blank page — so no
ranking would ever have produced it. **This is the instrument's first positive result and the
reason it exists.**

**Re-run whole in the three-hundred-and-forty-eighth, over all 786, and the alarm held again**:
twenty names at or past −1, **seventeen of them documents this tree already calls incomplete**, and
the other three are the same three session 265 named — `issue16038.pdf` at −6.70
(`AMBIGUOUS_TILING_CELL_CLIP`, whose own note measures the interior 13% short), `issue12295.pdf` at
−1.71 (`AMBIGUOUS_EVERYONE_OVER_THE_GEOMETRY`) and `issue7821.pdf` at −1.00
(`AMBIGUOUS_GRADIENT_QUANTISATION`). Fourteen rounds of change and the negative tail has not moved,
which is what a standing alarm is for.

**Re-run before and after in the three-hundred-and-eighty-third**, the round that carried an image's
samples and a shading's ramp into the quantity §11.5.3 composites (ADR 0220), which moves pixels.
**Every negative entry is identical to a thousandth** — twenty at or past −1, sixteen of them
documents this tree calls incomplete, head `issue16038.pdf` −5.642, then `issue12295.pdf` −1.712,
`checkbox_no_appearance.pdf` −1.200 and `issue14297.pdf` −1.146. That last one is the only line
whose *label* changed: it lost its `[incomplete]`, the same number on a page that stopped reporting
and now carries a diagnosis instead, and the diagnosis is two ladders — `poppler` 10.121 → 8.754 and
`mupdf` 9.840 → 8.875 from 72 to 576 dpi against ours 8.694 → 8.821, so the references' extra ink at
the page's own scale is their scan conversion of five-point type and ours is already at the limit.
That is the alarm doing what it is for: a page arriving in the negative tail with an explanation
rather than a shrug.

**Re-run once in the four-hundred-and-sixth, and it was not owed**, which is worth a line because
the round that ran it changed no rendering code at all: its whole diff under `crates/` is
`tests/oracle.rs`, so our rasters are byte-identical by construction and a before/after pair would
have compared a file with itself. Run anyway, over all 786, on this file's own recipe (`-alpha off
-channel R -colorspace Gray`): **twenty at or past −1 and sixteen of them documents this tree calls
incomplete**, head `issue16038.pdf` −5.758, then `issue12295.pdf` −1.712, `checkbox_no_appearance.pdf`
−1.200 [incomplete], `issue14297.pdf` −1.146 and `issue7821.pdf` −1.000 — **the same five names, in
the same order, to the thousandth, as the three-hundred-and-ninety-seventh's run**, which is the
ninth consecutive time the alarm has held. On the complete documents the four at or past −1 are all
diagnosed and nothing else passes −0.536.

**And one entry on the positive side is a lesson rather than a finding.** `issue13520.pdf` reads
+0.695 where the three-hundred-and-ninety-seventh recorded +2.554. This round did not move it —
nothing this round could move a pixel — so it moved somewhere in the eight rounds between, none of
which re-ran this sweep. The positive side is where a round that changes what gets drawn shows up,
and it only shows up if somebody runs it: **a round that changes drawing and skips step 7 does not
leave the number unchanged, it leaves it unwatched.**

**Re-run before and after in the four-hundred-and-forty-fourth**, the round that changed how the correctness oracle composes a clip chain (ADR 0280) — a change that moves pixels on any page where two clip boundaries fall in the same pixel. **The negative tail is byte-identical**: twenty at or past −1, sixteen of them documents this tree calls incomplete, and on the complete documents `issue16038.pdf` −5.734, `issue12295.pdf` −2.956, `issue14297.pdf` −1.150, `issue7821.pdf` −1.000, `jpx_smaskindata.pdf` −0.839, `issue16473.pdf` −0.717 and nothing past −0.536. **Twenty-one rows moved and twenty of them up**, by 0.001 to 0.025; the one that moves *down* is `22060_A1_01_Plans.pdf`, −0.265 → −0.280, and it is the right direction rather than a surprise — a clip that admits more of a *pale* mark subtracts ink rather than adding it, and that page is 72 sampled images. The *before* half was taken by stashing the round and re-running the gate, for the reason the three-hundred-and-ninety-seventh recorded.

**Re-run before and after in the five-hundred-and-eighty-third**, the round that stopped `tiny-skia` compiling its low-precision raster pipeline for this backend's paints (ADR 0418) — a change that moves a pixel wherever a coverage or an alpha under 1 meets a destination, which is most pages that have any transparency at all. All 786 measured both ways: **our own ink is unchanged to a thousandth on 342 of them and the median move is 0.0035 of 255**, the head is the same names in the same order and all diagnosed, and the count at or past −1 on complete documents went **4 → 3** — `issue7821.pdf` crossed up, −1.000 → −0.957. **Exactly one page moves by more than 0.36 and it is `issue12295.pdf`, −2.827 → −3.773**, which needs no new hypothesis: `examples/sub_pixel_width_census` says that page states **65 859 sub-pixel strokes, every one 0.1366 of a device pixel wide and near-black**, so it is ADR 0268's alpha-carried construction over two thirds of a sheet and the low-precision pipeline's upward bias was a larger share of a thinner mark's whole ink. **That is the round's own lesson at page scale**: the same bias had been flattering `render-quorra`'s turned ladder, where the thinnest rung read −0.2% for a construction that is 16.8% short. And the side-by-side is why the movement is not a regression — our ECG traces are a ghost either way while all four references draw them dark, which is `doc/todo/11`'s standing item rather than this change's. Most moves are *upward*: `issue4402_reduced.pdf` +0.36 and `issue840.pdf` +0.13 are the largest, so this is a re-rounding rather than a loss.

**Re-run whole in the five-hundred-and-ninety-eighth**, over all 786, on this file's own recipe (`-alpha off -channel R -colorspace Gray`): **19 at or past −1, 16 of them documents this tree calls incomplete**, head `issue12418_reduced.pdf` −19.447, `issue4722.pdf` −13.810, `issue15977_reduced.pdf` −12.927, `bug1050040.pdf` −11.272, `issue5801.pdf` −8.991. On the complete documents `issue16038.pdf` −5.737, `issue12295.pdf` −2.363, `issue14297.pdf` −1.130, then `issue7821.pdf` −0.957, `jpx_smaskindata.pdf` −0.840 and nothing past −0.536 — **three names past −1 and all three diagnosed**. The count on complete documents is 3 rather than 4 because `issue7821.pdf` crossed up in the five-hundred-and-eighty-third and has stayed there. The positive tail is unchanged in shape: `recursiveCompositGlyf.pdf` +198.653, `bug1743245.pdf` +23.277, `bug920426.pdf` +21.073, `issue4260_reduced.pdf` +17.607. **What this run added is not a number but a reading of the head** — see the paragraph above and ADR 0433.

**Re-run whole in the five-hundred-and-fourteenth**, the round that let a `/FontFile` whose bytes are a bare CFF be read as one (ADR 0349) — a change that moves pixels on one page of the corpus, and on that page from nothing to a line of text. All 786 measured: **twenty at or past −1, sixteen of them documents this tree calls incomplete**, and on the complete documents `issue16038.pdf` −5.734, `issue12295.pdf` −2.823, `issue14297.pdf` −1.145, `issue7821.pdf` −1.000, `jpx_smaskindata.pdf` −0.840, `issue16473.pdf` −0.683 and nothing past −0.536. **The same four names past −1, in the same order, all four diagnosed** — the alarm's twelfth consecutive hold. The page this round moved is invisible to it for the reason the four-hundred-and-fifth's entry gives: `issue5751.pdf` was *contradicted* before and *agrees* after, and a page crossing those two states is not in the ambiguous bucket at either end. What the sweep does say about this round is the claim worth having — no ambiguous page's ink moved — and the oracle's own per-page lines say it a second way, byte-identical on all 786.

**And two of this file's own names had drifted, which is the lesson above arriving a second time.** Sessions 405 and 406 recorded `issue16038.pdf` at −5.507 and −5.758 and `issue12295.pdf` at −1.709 and −1.712; the other four names past −0.5 reproduce here to the thousandth. Both drifters are the pages ADR 0213's and ADR 0268's work is about, and the sweep was last run whole in the four-hundred-and-fifteenth. A round that changes drawing and skips step 7 leaves the number unwatched rather than unchanged — and the corollary is that a *stale* number in this file is not evidence a page moved recently, only that nobody looked in between.

**Re-run before and after in the four-hundred-and-fifth**, the round that made a substituted
standard-14 font's third width source reachable (§9.6.2.1), over all 786 and with the corpus's
incomplete list labelled inside the loop. **Every one of the 786 lines is byte-identical**, numbers
and labels both. On the complete documents the negative head is `issue16038.pdf` −5.507,
`issue12295.pdf` −1.709, `issue14297.pdf` −1.120, `issue7821.pdf` −1.032, then `jpx_smaskindata.pdf`
−0.839 and `issue16473.pdf` −0.717 and nothing past −0.537 — **four names at or past −1 and all four
diagnosed**, the eighth consecutive run of the alarm holding. The positive tail is `bug1743245.pdf`
+23.129, `colorspace_atan.pdf` +28.004, `colorspace_cos.pdf` and `_sin.pdf` +29.138 and
`issue6006.pdf` +113.420, every one of them a reference that drew nearly nothing.

**And the identity is a statement rather than a shrug, which is the same property the
three-hundred-and-ninety-seventh's run had**: the page this round moved was *contradicted* before it
and *agrees* after it, and a page crossing those two states is invisible to a sweep whose population
is the ambiguous bucket. What the byte-identity does say is that no ambiguous page's ink changed —
which is a real claim about this round, because `standard_fonts.pdf`'s fourteen pages are ambiguous
and set specimen text in all fourteen substituted faces with no `/Widths`. Adobe's published metrics
answered every code they use, so the new third source was asked nothing there. This run's ink is
`(1 − mean) × 255` over a luma greyscale, which is why its absolute values sit a little below the
`-colorspace Gray` runs above; what is compared across a round is the same instrument before and
after, and it did not move.

**Re-run before and after in the three-hundred-and-ninety-seventh**, the round that stated a
knockout element's shape apart from its alpha (ADR 0234), and **every line of all 786 is identical
— the numbers and the labels both**. Head `issue16038.pdf` −5.758, then `issue12295.pdf` −1.712,
`checkbox_no_appearance.pdf` −1.200, `issue14297.pdf` −1.146 and `issue7821.pdf` −1.000; twenty at
or past −1 and sixteen of them incomplete, as in the two runs before it. **That is the expected
result and the reason is a property of this instrument rather than of the round**: the sweep's
population is the ambiguous bucket, and every page that round moved was *contradicted* before it
and *agrees* after it. A page moving between those two states cannot be seen here at all — so
"nothing moved" is the sweep saying nothing stopped being drawn, not the sweep failing to notice.
The *before* half was taken by stashing the round and re-running the gate, because each oracle run
overwrites the artefacts the sweep reads.

**Three lines on the *positive* side did move, and they are the round**: `issue13520.pdf` +3.804 →
+2.554, `bug1703683_page2_reduced.pdf` +0.142 → +0.141 and `issue12798_page1_reduced.pdf` in the
fourth decimal at +0.068. Every one of the three is a mask group whose raster this round redirected,
and a positive gap shrinking is ours coming *down* toward the lightest reference. Worth saying
plainly because the two halves of this sweep answer different questions: the negative tail is the
alarm, and it did not move; the positive side is where a round that changes what gets drawn is
expected to show up, and a round that moved nothing there would be a round that had not run.

**And the head's number was already stale, which only a *before* sweep can say.** The
three-hundred-and-eighty-second recorded `issue16038.pdf` at **−5.398** and called the sweep
byte-identical; run on that same commit before a line of this round existed, it is **−5.642**. This
round did not move it — both of its sweeps agree — so the change happened between the two sessions
with nothing in `crates/` to account for it, and the candidate is the reference side: the oracle's
run reported 16 renders *produced* against 6173 from the cache, and a reference re-rendered by a
newer `poppler` or `mupdf` moves `min(live)` under a page whose own ink never changed. **The lesson
is the sweep's, not the page's**: this number is a difference between two programs, so a "head
unchanged" claim is only worth what the before-run behind it is worth, and a round that reports one
without re-running the before has reported the last session's arithmetic.

**Re-run in the three-hundred-and-sixty-eighth**, after the round that snapped §10.7.4's marks to
the pixel grid (ADR 0208), over all 786 and filtered to the 743 on documents this tree calls
complete. **The head did not move**: `issue16038.pdf` −6.40, `issue12295.pdf` −1.71,
`issue7821.pdf` −1.07, then `jpx_smaskindata.pdf` −0.84 and `issue16473.pdf` −0.72 and nothing past
−0.54 — three names at or past −1 and all three diagnosed, which is the fourth consecutive run of
the alarm holding. The only entry that moved anywhere was the round's own page,
`issue4260_reduced.pdf`, on the *positive* side at +17.635 → +17.577: two rules landing within one
device pixel of each other now paint the same pixel instead of two overlapping bands, which is the
clause. The absolute values here are a hair off the numbers above because this run's ink is
`255 − mean` over a straight `L` conversion; what is compared across runs is the same instrument
before and after, and it is unchanged.

**Re-run whole in the three-hundred-and-seventy-ninth**, the round that emptied the ranking and moved
no pixel, over all 786 and filtered to the 743 on documents this tree calls complete. **The head is the
same five names in the same order** — `issue16038.pdf` −5.642, `issue12295.pdf` −1.712,
`issue7821.pdf` −1.000, `jpx_smaskindata.pdf` −0.840, `issue16473.pdf` −0.717, then nothing past
−0.535 — **three names at or past −1 and all three diagnosed**, which is the sixth consecutive run of
the alarm holding. The positive side is unchanged too: `bug1743245.pdf` +23.129, `bug920426.pdf`
+21.073, `issue4260_reduced.pdf` +17.577. This run's ink is `(1 − mean) × 255` after `-alpha off
-colorspace Gray`, which is why `issue16038.pdf`'s absolute value differs from the
three-hundred-and-seventy-fourth's `L`-conversion figure on a *coloured* page while
`issue4260_reduced.pdf`'s agrees to the thousandth; the gate's own numbers for that page — worst mean
40.55, similarity 0.3935 — are identical to that round's, so nothing moved.

**Re-run whole in the three-hundred-and-seventy-fourth**, the round that folded a tiling's repeated
mark (ADR 0213), over all 786 and filtered to the same 743. **The head moved and nothing else did**:
`issue16038.pdf` **−6.404 → −5.398**, `issue12295.pdf` −1.708, `issue7821.pdf` −1.069,
`jpx_smaskindata.pdf` −0.839, `issue16473.pdf` −0.717 and nothing past −0.536 — every entry but the
round's own page unchanged to a thousandth, and the positive side identical.

**And the head is expected to stay there, which is worth writing down rather than leaving as a
puzzle.** The gap is our ink minus the *lightest live* reference's, and on this page every reference
paints more than the geometry — `hayro` 139% of it, `mupdf` 115%, `poppler` 157%, `ghostscript` 299%
— because a rule 0.4 of a device pixel wide is a whole painted pixel under §10.7.4 read literally.
Ours is at 95% of the geometry now against 91% before. **A page can sit at the head of this ranking
because the references are heavy rather than because we are light**, and the way to tell the two
apart is the one this page carries: a closed form for the ink the document asks for.

**And the run's *positive* side produced a name off the undiagnosed list**, which it had not done
before. `bug920426.pdf` page 1 at **+21.07** — ours 25.49 against a lightest live reference of 4.42
— is `poppler` drawing fourteen `.notdef` boxes where the other four draw *Checkliste Service*;
ours and `hayro` agree to 0.006 of 255. The ranking had it at 0.35 from the nearest and 2.62 from
the furthest, which accuses nobody, so no amount of reading the ranking would have produced it.
`AMBIGUOUS_REFERENCE_DREW_NOTHING`'s second.

**What the positive side is and is not.** It takes the **minimum** over live references, so one
outlier is the whole comparison — which is exactly what makes it good at finding a reference that
failed and useless as a measure of how much we over-paint. The largest entry in the run,
`recursiveCompositGlyf.pdf` at +198.65, is that property at full stretch: ours, `poppler` and
`hayro` all paint the page's red box, `mupdf` paints nothing and is dropped, and `ghostscript`
paints only the words — so the minimum is 2.12 and the gap is a fact about `ghostscript`.

**And the sweep itself had a defect the same run exposed**: `min` over the references includes a
reference that drew *nothing*, and a blank is not a lower bound on the geometry. Four pages came
back at +21 to +29 of 255 — `mupdf` draws nothing on `colorspace_sin.pdf`, `_cos` and `_atan`, and
`hayro` nothing on `issue2840.pdf`. Drop a zero-ink reference before taking the minimum; what the
positive side is *good* for is finding a reference that failed.

**Run in the two-hundred-and-fortieth session over all 493 names it produced a negative result,
and the negative result is the finding**: the whole bucket lies between **−0.84 and +0.42 of
255** of every reference. After ADR 0173 and 0174 there is no ambiguous page left where this tree
draws materially less than the lightest of four other renderers. That is the class of defect the
bucket was most likely to be hiding — `issue19634.pdf` was −4.76 before ADR 0173 — and it has
been swept for.

What the sweep's own head is worth reading anyway, because a small gap can still be a clause:
`jpx_smaskindata.pdf` at −0.84 (`AMBIGUOUS_MATTE_WITHOUT_A_SOFT_MASK_IMAGE`), `issue16473.pdf` at
−0.72, `issue7454.pdf` at −0.15 but with the *references* spread over 9.3, and `bug1308536.pdf`
at +0.42.

**Re-run it after any round that changes what gets drawn**, and expect it to stay empty; a name
appearing at −1 or beyond is a regression no other gate would report as one.

**And it works on the *contradicted* list, which nobody had tried until the
four-hundred-and-thirty-first.** The loop is the same loop — our ink minus the lightest live
reference's, over artefacts already on disk — and the population is the gate's 68 `CONTRADICTED`
lines instead of its 787 `ambiguous` ones. It costs seconds and it reads differently, because a
contradicted page already has somebody pointing at it: what the sweep adds is *how much* and *which
direction*, over a whole list at once.

```text
−5.115  issue5751.pdf p1   [incomplete]  we draw nothing; a Type 1 program this reader refuses
−2.203  issue4436r.pdf p1                CONTRADICTED_SUBPIXEL_IMAGE, and its own note's 0.502
−1.549  issue9243.pdf p1                 a substituted sans, 0.6875 em of cap against 0.729167
−0.779  smask_luminosity_oob_transfer.pdf p1   CONTRADICTED_MASK_QUANTISATION
−0.482  issue7580.pdf p1                 the same cap height, at 18 pt
                                         then nothing past −0.4
+9.982  issue14802.pdf p1                CONTRADICTED_LINK_BORDER: two references drew no border
+13.704 issue11740_reduced.pdf p1        CONTRADICTED_REFERENCES_DREW_NOTHING, by name
```

**Nothing unexplained anywhere on the list**, which is the statement this file makes about the
ambiguous bucket and had never made about the contradicted one. The head being a page this tree
*reports* is correction 3 working one list over: a page we say we could not draw is expected to be
light.

**Run in the three-hundred-and-thirty-fourth over all 786**, after twenty rounds that changed the
readback, the chrome, the annotations a person can add and nine pages' worth of diagnoses. Filtered
against the corpus's own incomplete list first — correction 3 inside the loop rather than beside it
— which leaves **743 pages**:

```text
−6.700  issue16038.pdf p1       AMBIGUOUS_TILING_CELL_CLIP, 13% short by its own note
−1.712  issue12295.pdf p1       AMBIGUOUS_EVERYONE_OVER_THE_GEOMETRY
−1.000  issue7821.pdf p1        AMBIGUOUS_GRADIENT_QUANTISATION
−0.840  jpx_smaskindata.pdf p1  AMBIGUOUS_MATTE_WITHOUT_A_SOFT_MASK_IMAGE
−0.717  issue16473.pdf p1       then nothing past −0.54
```

**Three names at or past −1 and all three diagnosed**, which is the same head the
two-hundred-and-ninety-first found and the alarm holding. The positive side did its job too:
`bug1743245.pdf` at +23.1, `bug920426.pdf` at +21.1, `issue4260_reduced.pdf` at +17.6 and
`issue6931_reduced.pdf` at +17.3 are four references that drew nearly nothing where four
renderers draw a page.

The sweep is `doc/todo/00`'s own recipe and now has a script beside it in the round's scratch
notes; what makes it cheap is that nothing is rendered again — every panel is already on disk
under `<target>/tmp/oracle/`.

**Run in the two-hundred-and-ninety-first**, after three rounds that changed pixels — a `Tf`
naming `/Helvetica` (ADR 0183), a written `/Differences` (0184), §9.6.5.2's `.notdef` (none, as it
turned out). All 786 ambiguous pages, and **correction 3 is worth doing inside the loop rather
than beside it**: filtering the corpus's incomplete list out first turns two lists into one, and
what is left is the only list that can hold a surprise.

```text
on documents we report (10 of the 12 largest gaps)   −19.4 to −6.0, every one of them
                                                     "a substitute cannot be addressed (§9.10.2)"
on documents we call complete, 742 pages:
  −6.700  issue16038.pdf p1        AMBIGUOUS_TILING_CELL_CLIP, 13% short by its own note
  −1.712  issue12295.pdf p1        AMBIGUOUS_EVERYONE_OVER_THE_GEOMETRY
  −1.000  issue7821.pdf p1         AMBIGUOUS_GRADIENT_QUANTISATION
  −0.840  jpx_smaskindata.pdf p1   AMBIGUOUS_MATTE_WITHOUT_A_SOFT_MASK_IMAGE
  −0.717  issue16473.pdf p1
  −0.535  blendmode.pdf p1   −0.470  issue7339_reduced.pdf p1   then nothing past −0.29
```

**Four names past −0.7 and all four already diagnosed**; the alarm at −1 holds. The negative head
of the *unfiltered* run is entirely `doc/todo/21` item 2's population — composite fonts naming an
`Identity` ordering, which report and draw nothing — and that is the sweep working rather than a
finding: a page this tree reports is expected to be light.

The positive side did its job too: `bug920426.pdf` at **+21.07** is one reference drawing a row of
empty boxes where we and the other three draw *Checkliste Service*.

## What a group must say

**`ambiguous` is the gate's verdict and never the answer.** The owner put it in one sentence:
*even if the oracle cannot agree, we should be able to determine what is actually true, based on
the spec.* A group whose whole argument is "we sit inside their spread" has answered the easy
question. Every group must say **what the specification determines**, and there are three
shapes, all of them findings:

- **The clause determines it and we can be checked against it.**
  `AMBIGUOUS_SHARED_JBIG2_DECODER`: ISO/IEC 14492 defines the decoding exactly, and
  `tests/jbig2.rs` checks us against the corpus's own invariant — ninety-six encodings of one
  image, byte-identical — with no reference involved at all.
- **The clause determines that everyone here is departing from it.**
  `AMBIGUOUS_IMAGE_REDUCTION`: §10.7.4 says "there shall not be averaging over the pixel area",
  all five renderers average, ours is ADR 0025's documented departure. The finding is the
  departure; the spread is corroboration.
- **The clause puts the answer beyond itself, and says so.**
  `AMBIGUOUS_DEVICE_CMYK_CONVERSION`: §10.4.2.1 ranks two answers, §10.3.1 makes the destination
  profile "beyond the scope of this document", and its NOTE names "assumptions made by the PDF
  processor software". Say which clause leaves it open, and name the assumption this tree makes.

A fourth shape is not acceptable: a group that names no clause. And a group may say **"we are
wrong"** — `AMBIGUOUS_ZERO_AREA_FILL` did, for two sessions, before the fix.

## What has come out of it, so far

Ten sessions from the hundred-and-seventy-sixth, then twenty more: **ten defects found, eight
of them fixed** — a page one that was page two (ADR 0148), a photograph rendered black (0149), a
shading painted as a square (0150), a stencil that drew nothing (0151), a whole grid that
disappeared (0154), a sentence drawn as one Greek letter (0158), a stamp's gradient painted flat
(0160), and two coverage losses that moved the oracle's own headline (0165: a `/BBox` clip on a
widget border's own edge, and a miter bound on a comb field's separators). The ninth was found and not fixed for
twenty-six sessions and is fixed now: §8.7.4.5.4's greatest admissible root, which every backend
got wrong from the same place because every gradient library gets it wrong the same way
(ADR 0171).

Beside them: the ten documents whose substituted font drew none of its characters in silence
(0152), the coverage rule that made eight of them draw (0153), a pattern cell's clip worth 15% of
a page's ink (0155), a font program that draws nothing now saying so (0157), **thirteen JPEG 2000
codestreams that decode to the wrong samples** (0161, `doc/JPEG2000_FEEDBACK.md`), and a
measuring command that had been halving our own ink for two sessions (0163).

The tenth is found and not fixed either: a stroke under a pixel wide loses the half of
`tiny-skia`'s hairline smear that falls outside the raster's top edge, which is `doc/todo/11`
item 3 and was found by a synthetic ladder rather than by a reference.

The bucket itself went 754 → 704 → **0** undiagnosed, and that is the least interesting number in this
file. *Nine defects nobody could see* is the one to watch — **and one gate that found thirteen
more.** `jp2k-resetprob.pdf` sat at the top of the ranking with a name that named its own
hypothesis, and checking the hypothesis meant building `tests/jpeg2000.rs`: every corpus
`JPXDecode` stream against ISO/IEC 15444-5's reference software. It ruled the codec out for that
file — the decode is byte-identical — and found thirteen of the other twenty-nine codestreams
wrong (ADR 0161). **A page on this list is sometimes a question about an instrument that does not
exist yet.**

## Its shape, measured

| | |
|---|---|
| distinct documents the pages come from | ~181 |
| `freeculture.pdf` (309) and `pdkids.pdf` (52) | **361 — two long books** |
| **one paper under twelve names** (`tracemonkey.pdf` and eleven copies) | **154, diagnosed in the two-hundred-and-thirty-third session** |
| documents contributing exactly **one** page | ~154 |

**A quarter of the bucket was one document wearing different names, and nothing said so.**
`tracemonkey.pdf` is pdf.js's canonical fixture and eleven other corpus documents are the same
fourteen pages with an annotation added; `pdftotext` on page 9 gives the same md5 for all of
them. One measurement settled 154 names — and the number to report is *one finding*, not 154,
which is why `AMBIGUOUS_DENSE_TEXT_AT_PAPER_SIZE` says so in its first line. **Before taking a
name off this list, check what else in it is the same file**: `pdftotext -f N -l N | md5sum`
across the documents sharing a page count costs a second and can be worth a hundred names. It was
worth three more in the three-hundred-and-first — `issue15012.pdf` and `bug1885505.pdf` are the
paper's first page under two more names and `issue7014.pdf` is it a third time with §12.5.6.10's
markup over the abstract — which was **half the ranking's top six**.

**And the method has one false positive, found in the same run.** `multiline.pdf` and
`bug852992_reduced.pdf` matched each other on the md5 of *no text at all*: a page whose readback
is empty matches every other page whose readback is empty. So the check is evidence only where the
readback is non-empty, and a match on an empty one says the two documents have no text rather than
that they have the same text.

**Both books were taken as populations in the two-hundred-and-sixty-second session**, which is
what took the undiagnosed list from 489 to 136 — and the method is the part worth keeping. Six
pages had been measured one at a time over three sessions, all the same way, so the question
stopped being "what is wrong with page 329" and became "is this book one finding or three
hundred". Twelve more pages spread through both books, with two ladders each, put ours within
0.012 of `poppler`'s own limit every time; then the *whole* population's printed metrics were read
as a band, and it is one band with no gaps.

**The two-hundred-and-sixty-third took the next two populations the same way**:
`TAMReview.pdf`'s 22 pages, which are one band (mean 4.05 to 9.96, similarity 0.7722 to 0.9214)
and four ladders inside `AMBIGUOUS_DENSE_TEXT_AT_PAPER_SIZE`'s own finding; and `calrgb.pdf`'s
eight, which are the bucket's sharpest instance of shape 3 — §8.6.5.3 defines the components-to-XYZ
arithmetic exactly, the sheet's first page states an identity so the file is naming XYZ values
directly, several of them are outside any gamut, and §10.3.1 says in one sentence that how a
processor gets from there to a pixel "is beyond the scope of this document".

**The band caught the page the sample would have buried.** `freeculture.pdf` page 171 has a worst
tile of 81.57 where nothing else in the book exceeds 29.09: its cartoon is a one-bit stencil that
`ghostscript` thresholds to a black blob, and it belongs to `AMBIGUOUS_IMAGE_REDUCTION`. **A
population argument needs the population's own numbers and not only a sample's** — read the band,
then look at whatever sits outside it.

Two books and a long tail of single pages. **The books are not what this file said they were.**
It read "set in fonts nobody embedded, so each renderer substitutes differently", and `pdffonts`
says `freeculture.pdf` embeds all four of its fonts — nothing substitutes on any of its pages
(the two-hundred-and-twenty-ninth session, `AMBIGUOUS_DENSE_TEXT_AT_BOOK_SIZE`). What they are is
dense text at book size, which earns the page the *text* tolerance: 0.90 similarity, measured
over 153 reference-against-reference pairs because five rasterisers cannot agree more closely
than that about small glyphs. The bound is loose for a reason that was measured, not for a reason
about the file. **Take the tail first**: each of those is a file somebody added to a corpus for a
reason, and the reason is written down.

**And the book is what a looser gate would convict, which is the answer to the standing question
about it** (ADR 0776). `doc/todo/12` item 1 asked what raising the bound that *forms* a consensus
would do; the gate counts it every run now, and of the 276 pages it would newly contradict,
**272 are `freeculture.pdf`**. So the population this section already holds by name is almost the
whole of what that change was worth — and the reason those pages would be convicted is not the
bound being raised: 274 of the 276 convict on **structural similarity**, and on
`freeculture.pdf` page 100 our own render sits nearer to `mupdf` (ssim 0.9558) than the convicting
pair sits to itself (0.9315, 11.19% of channels apart). Three renderers scatter over 8-point body
text at 72 dpi by more than any of them sits from us, which is this section's own diagnosis
arriving from the other side.

## The next names on the ranking

**There are none, as of the three-hundred-and-seventy-ninth session**, and the shape of that last
five is the result worth keeping. Every one of them was a *font* page — five different files, five
different mechanisms, and not one of them a defect in this tree:

- **`issue4665.pdf`** (0.17 / 0.93) and **`bug911034.pdf`** (0.29 / 2.44) are `AMBIGUOUS_GLYPH_SCAN_CONVERSION`,
  and the first is the group's cleanest instance yet: **all four ladders end within 0.044 of 255 and
  the three without `ghostscript` within 0.009**, so the reference that is 38% heavy at 72 dpi agrees
  with everybody at 1 152 dpi. That is a stronger statement than a spread — it is the *same*
  renderer's later rungs saying that its own 1× excess was scan conversion. `bug911034.pdf` is the
  same page at a quarter the glyph size: three ladders end within 0.031, ours between them, and
  `ghostscript` is 9.07 of 255 over its own limit at the page's own scale.
- **`issue9084.pdf`** (0.16 / 1.09) is `AMBIGUOUS_SUBSTITUTED_FACE`, and it is shape 3 stated by the
  clause outright: a non-embedded `ArialMT` under `Identity-H`, where §9.7.4.2 ends "[t]he means by
  which this is accomplished are implementation-dependent". What that clause does *not* leave open is
  checkable and was checked — "they shall always be used to determine the glyph metrics" — and at 8×
  ours and `mupdf` put the line's ink in the same 1022 × 123 box **to the pixel** while all four
  references span the same width to 0.7%. Four flat ladders in two camps 1.46 apart.
- **`issue12705.pdf`** (0.18 / 1.00) is `AMBIGUOUS_UNCOLOURED_GLYPH_PROCEDURE`, the sharpest instance
  of shape 1 the bucket has had: **111 of the file's 114 Type 3 glyph descriptions state `1 1 1 rg`
  before filling**, and §9.6.4 Table 111 says a `d1` description's colour operators "shall be
  ignored". A processor honouring them would paint white on white and the page would be *blank*. All
  five draw the line, so all five apply the rule, and the two readings are as far apart as two
  readings get.
- **`bug1308536.pdf`** (0.35 / 3.79) is `AMBIGUOUS_REFUSED_EMBEDDED_FONT`, a group of one and a
  mechanism this bucket had not named: `ghostscript` prints *An embedded font is invalid* and loads
  `NimbusSans-Regular` instead, where four renderers draw the producer's own ultra-condensed face and
  end within 0.015 of 255 at the limit. The program's Private DICT really is corrupt — six real
  operands the format cannot mean — and **the corrupt part carries no outline**: the hinting
  parameters are broken, the CharStrings INDEX is not, and §9.2.4's advances come from `/Widths`.
  Table 124 requires the program to conform and states no consequence for one that does not.

**The step this adds, and it is about a ladder that does *not* converge.** On `issue12705.pdf` four
of the five renderers bracket the geometry and `ghostscript` sits 6.0% to 7.1% above the ink at 4×,
8× *and* 16× alike. Dividing its excess by the ink a one-pixel erosion of our own raster removes
turns that into an outward offset — 0.161, 0.355 and 0.587 device pixels — which **triples in device
pixels and holds at 0.040 ± 0.004 points**. A constant offset in *user* space is a different shape
being filled; a constant offset in *device* space is scan conversion. One ratio tells them apart, and
until this round the bucket had only "it does not converge, so it is not scan conversion".

**Three in the three-hundred-and-seventy-second, and two of them turned a group's argument into
arithmetic.** The ranking's whole head is now pages whose *nearest* is under 0.4, so step 1's
"prefer a page whose two numbers are close" has nothing left to prefer; what these three had
instead is a page small enough, or a placement regular enough, for a **closed form** to exist
without a ladder.

- **`issue14953.pdf`** (0.28 / 3.64) is 200 × 50 with fifteen codes from ConTeXt's embedded Type 3
  face, and **every glyph description begins `wx 0 0 0 0 0 d1`** with `/FontBBox [0 0 0 0]` beside
  it. §9.6.4 Table 111 names that exact situation — "[i]f any marks fall outside this bounding box,
  the result is implementation-dependent" — so it is the bucket's sharpest instance of shape 3. The
  measurement that makes it a diagnosis rather than a citation is **a synthetic A/B this project
  wrote**: two 100 × 40 pages, one Type 3 glyph, one stroke, identical in every byte but the four
  `d1` operands. Ours and `mupdf` are byte-identical across the pair at 72, 288, 576 and 1152 dpi;
  `ghostscript` draws *exactly nothing* above 72 dpi; `poppler` is byte-identical at 72 and then
  diverges as the pixels shrink. That is also the corpus page's ladder — `poppler` 13.68 → 3.95 →
  1.48 → 0.26, drift rather than a limit — explained by a file the corpus does not contain.
  `AMBIGUOUS_DEGENERATE_GLYPH_BOX`.
- **`issue4379.pdf`** (0.19 / 3.67) is one command placing a 1000 × 800 stencil-masked image onto
  device x `[36, 536)`, an exact **two-to-one reduction onto integer coordinates** — which makes it
  the first page in `AMBIGUOUS_IMAGE_REDUCTION` whose clause can be *evaluated*. §10.7.4 names one
  raster sample by sample; the samples themselves are settled by rendering at 2×, where ours,
  `mupdf` and `ghostscript` are byte-identical over 1190 × 1684. Against the raster the clause
  states: `ghostscript` differs on **0** of 500 990 pixels and this tree on **3 927**, 0.78% of the
  page. **The five renderers' ink agrees to 0.023 of 255**, so no ink measurement could ever have
  seen it — and this is ADR 0025's stated cost, measured on a real page for the first time.
- **`bug1889122.pdf`** (0.13 / 2.94) is one text field on a 231 × 85 crop box whose whole appearance
  is `q 0 G 0.5 0.5 149 21 re s Q` under an identity §12.5.5 map, so its ink is **arithmetic**:
  `150 × 22 − 148 × 20 = 340` square points over 19 635 pixels is 4.4156 of 255. Ours is 0.05% over
  it, `mupdf` 0.39% over, `poppler` 2.3% under, `ghostscript` **26.7% over** and `hayro` 17% under —
  the same two outliers in the same two directions `AMBIGUOUS_WIDGET_BORDER` has now seen six times,
  and the first time against a number the file states rather than a limit two references agreed on.

**The step this adds**: before taking a ladder, ask whether the page's marks have an area you can
*write down*. A single stroked rectangle does; an image whose placement is an integer reduction
does; and where they do, the closed form is exact where a limit is only an agreement. **None of the
three was a defect**, and the round's ledger work came out of them instead — two rows citing a
sentence Table 111 does not contain (below).

**Step 7 was re-run whole after them**, over all 786, filtered to the 743 on documents this tree
calls complete: `issue16038.pdf` −6.404, `issue12295.pdf` −1.708, `issue7821.pdf` −1.069, then
`jpx_smaskindata.pdf` −0.839 and `issue16473.pdf` −0.717 and nothing past −0.536. **Three names at
or past −1 and all three diagnosed**, unchanged from the three-hundred-and-sixty-eighth's run to
within a thousandth — the fifth consecutive time the alarm has held. The positive side corroborated
this round's own work without being asked: `issue14953.pdf` sits at **+11.37**, because the lightest
live reference on it is a `ghostscript` that clipped the page away.

**Three in the three-hundred-and-thirtieth, and one of them is why step 3 exists.**
`issue13343.pdf`'s two pages are **eight commands** each — a line reading `( 57)【要約】` in a
non-embedded `Ryumin-Light-90ms-RKSJ-H` — and the ink table says ours is 30% darker than either
reference. The four-panel strip says what the table cannot: `poppler` draws `【要約】` and not
`( 57)`, `hayro` draws `( 57)` and not the ideographs, and three of us draw the line in three
faces of different weight. Five renderers, three different *sets of characters*
(`AMBIGUOUS_SUBSTITUTED_FACE`). `tiling-pattern-box.pdf` is a cube on a §8.7.3 hairline grid with
0.67 of 255 of ink on the whole page: two ladders converge to 0.0024 of each other, ours lands
between them, and `poppler` at the page's own scale is **34% over its own limit**
(`AMBIGUOUS_SUB_PIXEL_LINE_WORK`).


**And one in the three-hundred-and-twenty-fifth, where the page was small enough to read row by
row.** `issue19083.pdf` is 149 × 68 device pixels: one choice field with an auto-sized `/DA`
reading *Hello World* inside a one-unit border, twelve commands. `poppler` sits at its own limit
from 72 dpi and ours climbs 1.44 of 255 to reach the same place — and the difference is not the
auto-size, because at 8× the ink's bounding box is 126 × 21 at (15, 26) in **both**, to the pixel.

**The row means say which mark it is.** The field's two horizontal borders are one raster row each
for `poppler`, carrying 213.93; ours spreads 176.08 and 174.52 across two rows apiece, which is a
one-unit line at a fractional device position drawn as its own area. Those two marks are **77.3 of
the 99.2** of row-mean separating the whole page. `AMBIGUOUS_WIDGET_BORDER`.

**The step this adds**: when a page is small, the per-row mean is a free heatmap. Twenty rows of
two numbers said in one look what the ink table could only say the size of.


**And two in the three-hundred-and-twenty-third, off one document, with two different answers.**
`issue840.pdf` is a festival timetable and both its pages were on the list. Page 1 is 4 328
commands of flat coloured blocks: three ladders climb **in parallel** — `mupdf` 0.60 of 255 above
`poppler` at every rung, ours 0.22 below it at every rung — and a difference that does not shrink
with the pixels is a colour. The file states it outright: `0.125 0.125 0.125 rg` is 31.875 of 255,
ours and `poppler` round it to 32 and `mupdf` truncates to 31, on nearly every pixel of a page that
is nearly all flat fill (`AMBIGUOUS_EIGHT_BIT_COMPOSITING`). Page 2 is a light page of text whose
two ladders converge from opposite sides to **0.0002 of 255** — the tightest limit this bucket has
measured — with ours 0.005 under it (`AMBIGUOUS_GLYPH_SCAN_CONVERSION`).

**So the instruction is "check what else on the list is the same file", not "assume it is the same
answer".** Five times that check has paid four names for one measurement; this is the first time
one document has needed two.


**And one in the three-hundred-and-twentieth, where the ink table was measuring a colour.**
`issue269_1.pdf` page 1 is 100 × 100 points of Illustrator art in **three** commands, each a `k`
operator inside its own `/OC` section. Ours is flat at 28.288 from 4× to 32× — an area-exact
rasteriser has nothing left to converge — `poppler` descends onto it (28.7324 → 28.3097 and still
falling), and `mupdf` is flat 0.135 *above*. A flat offset is not scan conversion, and the
difference image says so: it is the *interiors* of both glyphs at 2 to 6 levels rather than their
outlines. At 2304 dpi the whole raster is three colours, so the histogram is the measurement —
ours and `poppler` byte-identical at (35, 31, 32) and (38, 40, 108), `mupdf` two to three levels
away on every channel. `AMBIGUOUS_DEVICE_CMYK_CONVERSION`, which had one member since the
hundred-and-seventy-eighth.

**The step this adds**: when a ladder is *flat* rather than converging, the difference is not in
the marks. Take the histogram of a page whose colours are few, and the answer is exact rather than
statistical.


**And one in the three-hundred-and-nineteenth, where the references are the ones short.**
`issue13242.pdf` page 1 — 0.51 from the nearest and 1.21 from the furthest — is 2 449 commands of
Lorem ipsum in one embedded Calibri subset with §12.5.6.10's yellow wash over eight lines. Two
ladders converge to 0.0116 of 255 of each other and ours ends between them; what is unusual is
72 dpi, where **ours is already 0.003 from its own 8× value** and both references are 0.18 below
theirs. `ghostscript` is 1.1 over and `hayro` 2.4 under, which is why nobody can be called wrong.


**And one in the three-hundred-and-eighteenth, where ours lands *between* the two limits.**
`issue6132.pdf` page 1 — 0.50 from the nearest reference and 0.93 from the furthest — is a US
Letter page of 2 328 commands set in nine embedded Computer Modern `Type1C` subsets, with no image
anywhere, so its mean *is* its glyph coverage.

```text
                 72 dpi   288 dpi   576 dpi
poppler         10.3520   10.4153   10.4255
mupdf           10.4040   10.4455   10.4430
ours (1x/4x/8x) 10.4147   10.4315   10.4361
```

Both references climb and end 0.0175 of 255 apart; ours climbs too and ends between them. And it
is the clearest instance of the other half of `AMBIGUOUS_GLYPH_SCAN_CONVERSION`'s argument: at the
page's own scale ours is 0.02 from its own limit where `poppler` is 0.07 and `mupdf` 0.04 below
theirs, so of the five renderers at 72 dpi ours is nearest a limit no reference is trusted for.


**And one in the three-hundred-and-seventeenth, where the page's own name was the hypothesis.**
`blendmode.pdf` page 1 sat at **0.46 from the nearest reference and 0.59 from the furthest** — the
tightest ratio the tail had left, which step 1 reads as *we are alone*. It is sixteen labelled
swatches, each a 100 × 100 JPEG with an 8-bit soft mask at 90 ppi, so every one of the thirty-two
images is reduced by 0.8. Two ladders converge to 30.1531 and 30.1638 and ours is flat at 30.07
across three scales; at the page's own scale ours is 0.11 from its own limit where `poppler` is
0.55 and `mupdf` 0.61 *above* theirs, which is `AMBIGUOUS_IMAGE_REDUCTION`'s sentence.

**What is new is the second measurement, and it is the one the file's name asks for.** At 8×,
`|ours − mupdf|` is 0.53 of 255 per pixel against `|mupdf − poppler|` 0.67 — we are inside the
references' own spread — while the *signed* ink difference over the same page is 0.09, an eighth
of that. A difference that cancels is where an edge is, not what was drawn. And a four-by-eight
grid of tile means puts the ratio of difference to the tile's own ink between 0.009 and 0.055 over
every tile that has ink, largest on the one tile that is a heading rather than a photograph. **No
blend mode is an outlier**, which is the hypothesis a page called `blendmode.pdf` exists to invite.

**And the count in this file's own header was wrong**: it said 72 undiagnosed names, which is
`wc -l` of `ambiguous_undiagnosed.txt` — a file with a twelve-line header. The gate counts the
lines that are not comments and holds *that* list to equality, so the number was 60 before this
round and is 59 now. Trap 1 one directory over: the instrument that reports a count is not the
count.


**The head went in the two-hundred-and-ninety-fifth, and it produced a mechanism this bucket had
not named.** `issue19971.pdf` pages 5 and 6 are one document — a specimen of lists, headings,
paragraphs and four scripts — and they came apart into two findings:

- **Page 6** is 456 commands of text in four scripts and no image at all. Two ladders agree at 8×
  to 0.0055 of 255, ours climbs onto the limit ending 0.025 short, and a four-by-four grid of tile
  means says the residual is spread over every tile in proportion to its ink.
  `AMBIGUOUS_GLYPH_SCAN_CONVERSION`, with no new argument needed.
- **Page 5** is the same text plus one 2500 × 1750 `DCTDecode` photograph in an `ICCBased` space,
  and it is a new group. The two ladders agree to **0.0008 of 255** — the tightest limit this
  bucket has produced — and ours stops 0.155 short, of which the photograph is 57% on 12% of the
  page.

**And step 6's own assumption failed for the second time, in the direction that is a finding.**
The step works because a renderer's departure from the geometry shrinks with the pixels. Rendering
the same page at **16×**, where the image is enlarged rather than reduced, gave a per-channel
difference identical to 8× to three decimal places — so it is neither scan conversion nor the
reduction, and the only two places left are the decoder and the colour space. Decoding the
extracted codestream twice ruled out the first (under 0.2 of 255, mixed in sign, against a uniform
lift six times larger), which leaves `pdf_model::icc` and `lcms` evaluating one 296-byte
matrix-shaper profile — §10.3.1's "beyond the scope of this document", one colour space over from
`AMBIGUOUS_DEVICE_CMYK_CONVERSION`. `AMBIGUOUS_ICC_MATRIX_PROFILE`.

**And the next name down, in the two-hundred-and-ninety-ninth, is the shape a wide ratio is for
and the reason step 3 exists.** `issue19326.pdf` page 1 sat at 0.65 from the nearest reference and
**11.06 from the furthest**. The ink says almost nothing — ours 46.25 against `ghostscript`'s
47.64, which on a page of black letterforms reads as an edge difference — and the picture says
everything: ours, `poppler`, `mupdf` and `hayro` draw the letters *JPX*, and `ghostscript` draws a
band of scrambled blocks with about the same coverage. **A reference that decoded an image wrongly
can have the right amount of ink**, so no metric on that page would have produced it and the
side-by-side did in one look. `AMBIGUOUS_A_REFERENCE_DECODED_THE_IMAGE_WRONG`, with the honest
caveat written into it: `tests/jpeg2000.rs` declines this codestream because it is sixteen-bit, so
the evidence is four decoders agreeing rather than ISO/IEC 15444-5's reference software, and it is
recorded as the weaker kind.

**And four more in the three-hundred-and-thirteenth, off one document, the same way.**
`file_pdfjs_test.pdf` had four of the seventy-six names — Mozilla's own test-suite documentation,
four US Letter pages of headings and bulleted lists in six embedded subsets and no image at all, so
each page's mean *is* its glyph coverage. Two ladders converge on each page independently and agree
to **0.0004 to 0.0034 of 255**; ours climbs onto every one of the four from below and ends 0.006 to
0.015 short. `AMBIGUOUS_GLYPH_SCAN_CONVERSION`, with no new argument needed — which is the fifth
time "check what else on the list is the same file" has paid, and the second time it has paid four
names for one measurement.

**And four in the three-hundredth, off one document, by this file's own instruction.**
`issue12963.pdf` had four pages on the undiagnosed list and two more already inside
`AMBIGUOUS_EVERYONE_OVER_THE_GEOMETRY` — so step 1's ranking was pointing at page 5 while the
answer was already written down two pages over. **Check what else on the list is the same file**
paid four names for one measurement: the two ladders agree to **0.0004 of 255**, the tightest
limit this bucket has produced, on each of the four independently.

**The lesson is about the ladder rather than the page**: a limit that a renderer does not approach
*at all* is not a loose limit, it is a difference in a different quantity, and one more rung is
what tells them apart. The two-hundred-and-sixteenth found the same step failing the other way, on
a reference that drifted instead of converging.


**Two off the head in the two-hundred-and-seventy-ninth, and both were one shape apiece.**
`issue7769.pdf` page 1 — 0.67 from the nearest and 0.97 from the furthest, the tightest ratio the
tail had left, which step 1 reads as *we are alone* — is 24 commands setting one sentence on a
153 × 63 page, so its mean is its glyph coverage: two ladders agree to **0.003 of 255** and ours
climbs onto the limit from 0.5 below it (`AMBIGUOUS_GLYPH_SCAN_CONVERSION`). `issue11473.pdf`
page 1 is four hatch swatches whose tiling cell is a **0.3985-unit stroke** — 0.4 of a device
pixel — where `ghostscript` paints 60% more than the geometry, `poppler` 46% more and ours 10%
less (`AMBIGUOUS_SUB_PIXEL_LINE_WORK`). **Neither was a defect and both were a width**: the tail's
head is now populations of *scan conversion*, which is the same result the ranking reached one
level up in the two-hundred-and-fifteenth.

**Two more in the two-hundred-and-eighty-sixth, and both joined an existing group with no new
argument needed.** `two_pages.pdf` page 1 is **one command** — step 4's "one command has meant one
image" for the fourth time — a 512 × 543 JPEG with a JPEG soft mask reduced by a third, where ours
is flat to four decimal places and two ladders land 0.011 of 255 around it
(`AMBIGUOUS_IMAGE_REDUCTION`). `textfields.pdf` page 1 is six empty fields on a letter page whose
whole ink is one-point borders and comb cells: the two ladders agree to **0.0025 of 255**, and at
the page's own scale `ghostscript` is 27% over the geometry and `hayro` 19% under — the same two
outliers in the same two directions as `bug1863910.pdf`'s 28% and 22%, which is why it is that
page's group rather than a new one (`AMBIGUOUS_WIDGET_BORDER`).

**The ranking is a different list now.** With the three populations gone the head is 0.76 and
below, and the two-hundred-and-sixty-fourth session took four of it: `issue11913.pdf` page 1,
where the two ladders and ours agree to **0.024 of 255** — the tightest three-way agreement the
bucket has produced — `issue1350.pdf` pages 1 and 3, and `ZapfDingbats.pdf` page 1, whose eight
fonts are all standard 14 with nothing embedded and whose 0.60 of 255 is Foxit's outlines against
URW's. What is left below them: `issue12963.pdf` page 7 (0.76 / 1.92), `issue17065.pdf` page 1
(0.73 / **14.86** — a ratio of twenty, which step 1 says is a page about the references),
`issue16473.pdf` page 1 (0.72 / 2.77), `issue19971.pdf` page 6, `textfields.pdf` page 1 and
`issue11473.pdf` page 1.


**`chrome-text-selection-markedContent.pdf` left it in the two-hundred-and-fifty-ninth**, and it
is the cleanest instance of shape 1 so far: the whole difference is **one level of green over a
third of the page**, the file states every number in the fill that produces it, and §11.3.6's
arithmetic on those numbers gives 235.569 — which is 236, which is ours. Both references give 235.
`AMBIGUOUS_EIGHT_BIT_COMPOSITING`, and the way in was step 6's two ladders saying *not scan
conversion* (ours flat at 26.95 while both references climbed onto 27.21) followed by a
three-by-six grid of per-tile differences, which put the whole of it in two columns, followed by a
per-channel mean, which named the channel. **Localise before explaining**: a page-level number
said "0.25 low everywhere" and the truth was "one level low on one third".

**`bug1703683_page2_reduced.pdf` and `issue2884_reduced.pdf` went in the two-hundred-and-sixtieth**,
both to existing groups and both by the same instrument: two reference ladders, and ours beside
them. The first is one indexed image with a JPEG soft mask reduced by four, where `poppler`
descends onto 5.3695 and ours is flat at 5.364 — **0.006 of 255 apart, the tightest agreement
`AMBIGUOUS_IMAGE_REDUCTION` has produced** — while `mupdf` is flat 0.14 below both and is the
reference the page is about. The second is a 169 × 19 crop box holding one line of Japanese, whose
mean *is* its glyph coverage: the ladders agree to 0.018, ours climbs onto the limit from below,
and at eight times the two panels are indistinguishable. `AMBIGUOUS_GLYPH_SCAN_CONVERSION`.

Then **`freeculture.pdf` for seven of the next eight** — pages 163, 165, 184, 172, 156, 160 and 325,
from 0.86 down to 0.77.

**Every name above 0.75 has a furthest at least twice its nearest.** Step 1 says to prefer a page
whose two numbers are *close*, because that is the shape that says we are alone, and there is no
longer one on the ranking. That is a result about the list: its head is pages where the
*references* disagree, and its tail is the long book, which
`AMBIGUOUS_DENSE_TEXT_AT_BOOK_SIZE` has already measured twice.

**So step 7 is where the next defect is more likely to be than step 1.** The ranking has been
worked down to a population; the sweep is what looks at all 492 at once.

**And the two-hundred-and-thirty-fourth session took the two with the widest ratio, 8.01 and
9.63, and both were exactly that.** `issue21436.pdf` is 450 bytes whose catalogue's `/Pages`
names a `/Type /Page`: `mupdf` refuses the document, `ghostscript` paints a one-unit stroke 27%
over its geometry, and ours is 4.5836 at 1× against 4.5900 at 8× — the geometry itself.
`issue11931.pdf` is a `DCTDecode` image whose `SOF0` identifiers are the letters R, G and B:
`ghostscript` obeys Table 13's default and paints the band magenta at six and a half times the
page's ink, and the other four read the codestream. **Both are a clause read correctly by the
renderer that is alone**, which is the shape a wide ratio is *for* — and both produced a
correction to a ledger row rather than a change to any pixel.

**And the two-hundred-and-thirty-seventh took the next one down and it was ours.**
`issue19634.pdf` sat at 0.85 / 5.96 — Skia's own `blurSmallRadii`, five renderers giving five
answers between 2.87 and 47.98 — and the picture said what no number could: **we drew none of
the red text**. §8.6.8's uncoloured restriction was still in force inside the soft mask's own
group, so a `d1` glyph procedure that set a `/Luminosity` mask had its mask evaluated to zero.
Ink 2.87 → 8.03 against `mupdf`'s 7.63 and `hayro`'s 8.11. ADR 0173. **A five-way spread is
never scan conversion**, which is the reading to take from the ratio rather than "the references
disagree, so it is not ours".

**Four `freeculture.pdf` pages and one paper under twelve names left it in the
two-hundred-and-thirty-third**, which is 158 of them, and the shape of that result is in the
section above: a quarter of this bucket was one document.

**`issue4402_reduced.pdf` left the list in the two-hundred-and-thirty-first session**, and it is
the clearest instance so far of shape 3 — the clause puts the answer beyond itself and says so.
The page is a 215 × 28 crop box holding one line of eight-point text and a rule, so its mean *is*
its glyph coverage, and §10.7.4's last sentence is "[s]can conversion of character glyphs may be
performed by a different algorithm from the preceding one". The two ladders agree to 0.012 of 255
and ours climbs onto the limit (55.41 → 56.71 → 56.78 → 56.91 against 56.98), while at the page's
own resolution the five renderers spread by 3.0 of 255. `AMBIGUOUS_GLYPH_SCAN_CONVERSION`.

**And the pairwise matrix is worth taking on a page like this**, because it answers a question
step 5 cannot: ours against `hayro` is 0.0219 MAE, the *smallest* pair among all ten, and every
pair involving `ghostscript` is larger than our worst. A page where the references disagree with
each other more than they disagree with us is not a page about us.

**`issue18529.pdf` left the list with a difference, not with an explanation**, and it is the one
name here worth coming back to: ours and `hayro` are both 5.8% under the high-resolution limit on
a 65×50 page that is one §8.7.4.5.3 gradient, and the three C renderers are on it. 1.3 of 255,
and the two renderers on one side of it are the two that share no library with the other three.

**The whole of the list above 1.6 went in the two-hundred-and-fifteenth session**, six pages in
one sitting, and the shape of that result is worth as much as the pages: **the ranking's top is
now populations rather than defects.** Two were a face nobody ships, two were one word on a page
the size of a postage stamp, one was two hairlines, one was an eight-bit ramp — and the only new
*defect* among them is a rasteriser property that a synthetic page found in ten minutes
(`doc/todo/11` item 3).

**`issue4706.pdf` is already known to be about *where* rather than *how much*.** It and
`stamps.pdf` come out within 0.12 and 0.2 of every renderer on ink *and* against the
high-resolution limit, so whatever separates them is placement. That is worth knowing before
opening one: **step 5's closed form answers "how much" and is silent on "where"**, and a page
where everybody's ink agrees needs the heatmap instead.

**A page can be fixed and stay on this list, and this file said otherwise.** The paragraph below
recorded that `issue7821.pdf` "left it in the hundred-and-ninety-ninth from the top of the list".
What left was its *position*: ADR 0160 took it from 5.44 to 1.79 and it sat at the top of the
undiagnosed ranking for fifteen more sessions, because a fix is not a diagnosis and only the
second takes a name off `ambiguous_undiagnosed.txt`. It has one now
(`AMBIGUOUS_GRADIENT_QUANTISATION`). **When a session fixes a page on this list, write its group
in the same session** — the same lesson the text gate's ratchet taught in the hundred-and-sixty-
sixth, one list over.

**Fifteen names left the list in the two-hundred-and-fifth to -eleventh sessions**, and the shape
of the result is the argument for the tail: two were defects in this tree (`bug1863910.pdf`'s
`/BBox` clip and `issue21068.pdf`'s miter bound, both ADR 0165), one was a defect with its own
file (`radial_gradients.pdf`, fixed in the two-hundred-and-thirty-second session, ADR 0171),
one is a clause `poppler` does not honour
(`bug1552113.pdf`'s 112-unit border), and the rest are scan conversion or artwork the standard
does not state.

**And the sixth defect the bucket has produced came out of the next name down.** `bug1863910.pdf`
was two empty text fields, and its one-point borders carried 22% less ink than their geometry —
an anti-aliased `/BBox` clip lying exactly on the stroke's outer edge, which is ADR 0155's finding
one path over. Fixing it moved the oracle's own headline: **agrees 849 → 851, contradicted 70 →
68** (ADR 0165). Two of the three pages the ranking has produced since the instrument was repaired
were defects.

**Step 6 emptied the top of the list in one session.** Four of the five names above 3.5 were
image reductions whose whole difference is scan conversion, and the high-resolution limit settled
each in minutes: `bug1799927.pdf`, `issue1985.pdf`, `issue7200.pdf` and `jp2k-resetprob.pdf`.
The fifth, `issue18894.pdf`, was a file that had broken Table 73's operand count. None was a
defect; all five now say *what the clause determines* rather than sitting inside a spread.

**`issue8697.pdf` left this list in the hundred-and-ninety-seventh session and is the ranking's
own argument**: 3.52 from the nearest against 3.55 from the furthest, which step 1 says to
prefer, and it was drawing one Greek letter where the file states a sentence. ADR 0158. And
`issue7821.pdf` was **fixed** in the hundred-and-ninety-ninth from the *top* of the list, where
it had been for four sessions: 5.44, and the picture was a stamp anybody would have accepted
(ADR 0160). It left the list itself only in the two-hundred-and-fifteenth, which is the
distinction the section above draws.
`jp2k-resetprob.pdf`, `S2.pdf` and `issue5475.pdf` left it in the two-hundredth, all three
through `tests/jpeg2000.rs`. ADR 0161.

## The contradicted list has no next name either — read whole in the eight-hundred-and-seventy-third

**The question was *take the worst-ranked contradicted page whose cause is not already diagnosed
and held by name*, and the answer is that there is none** (ADR 0805). The oracle in a fresh
worktree, every reference re-rendered by today's binaries — the cache hit rate the gate printed was
0.1%, which is the control that none of these verdicts is a stale panel's — reports the pool at
sixty, all sixty held by a `CONTRADICTED_*` group and the ratchet green. The instrument outside the
gate whose question this is says the same thing: `unpriced` finds every failing bound on every one
of the sixty named by the note that holds its page, and no contradicted page outside every page-list
note. `quoted` and `overtaken`, run over the same log, name ten and twelve `CONTRADICTED_*` notes
respectively — and both say on their last line that a hit is a reading list and not a verdict, so
neither answers whether a page is held. The gate itself now prints, beside each row of the
by-the-bound ranking, which group holds the page, and under the ten rows how many of the pool no
group holds — the hour this round spent reconstructing that by hand is why.

**The head, opened rather than trusted, and the notes held.** The by-the-bound ranking's first seven
are three groups: `xobject-image.pdf` (a file that contradicts itself, our choice documented and
reported), the three `bitmap-*` pages of `CONTRADICTED_SHARED_JBIG2_DECODER`, and three of
`CONTRADICTED_DEVICE_CMYK_CONVERSION`'s five. The side-by-sides say what the notes say: on the
halftone composite ours and `hayro` draw the family's one drawing while both Artifex programs draw
`jbig2dec`'s garbled halftone region and `poppler` a stray bar of its own; on the CMYK shadings ours
and `poppler` are §10.4.2.5's arithmetic and the three that share a SWOP characterisation are
desaturated together, which ADR 0773 priced by taking the profile away. Below them are the three
link borders (a reference gap and a printer's Print flag), one substituted symbolic face with a
closed form, one page where the references space glyphs by a width the file does not state, and
then the differing-fraction population `doc/todo/12` is about.

**So the next page is not on this list, and the rule for choosing one from a fully held pool is
the line the gate prints: the highest row whose note names a departure of *ours* rather than a
reference's.** That row is `issue4436r.pdf` at 1.16× on the differing fraction, and its departure
is §10.7.4's own departure (1) — an image's edge drawn at its coverage where the clause's image
paragraph paints only the pixels whose centres are inside. It was read against the clause and
against `doc/todo/11` §5 and *declined* in this round, with the reasoning on §10.7.4's ledger row:
an aliased image edge beside anti-aliased everything else is a change to a priced decision, not a
fix, and it moves the page's verdict nowhere. Below it the pool is `CONTRADICTED_GLYPH_EDGES`'s
twenty-seven and the bound they fail is a bound a voting reference cannot meet either. **A round
sent to the contradicted list for a defect should now be sent somewhere else** — the crawl's fixed-
document ranking in `doc/todo/03`, or this file's own three rankings, whose heads are held too.

