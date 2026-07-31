# Handover

Written 2026-07-26, updated 2026-07-31 at the end of the **fifty-seventh** working session. Read
`/CLAUDE.md` first — it holds the five non-negotiable principles, what *done* means, and the
closed list of exclusions. **Principle 5 is the one that changes how to work**: the specification
is the only source of truth, and agreement with poppler, mupdf or pdf.js is evidence that we read
it right, never the definition of right. `doc/PLAN.md` holds the phases and the conformance
ledger's design; `doc/adr/` holds every decision's argument. **This file is only the state of
play, the traps, and what to do next** — where something is also written there, this is a pointer.

Each session's own reasoning lives in its ADR. This file keeps a lesson exactly once: in a trap
if it changes how you write code, in "Habits" if it changes how you work, and in the numbers if
it is a fact about today.

## What the fifty-seventh session changed

**A click follows a link.** ADR 0062, and it is the first thing this program does *because
somebody pointed at something*.

With the ledger at zero unreviewed rows, the specification track is now its **195 `silent`
rows**, and almost all of clause 12's are one shape: pages render correctly and nothing happens
when a person clicks on one. §12.5.6.5's first sentence is where that starts — a link "represents
either a hypertext link to a destination elsewhere in the document … or an action to be
performed" — and everything underneath was already built: destinations resolve to a page index
(ADR 0054) and the viewer turns pages.

`pdf_model::link` is the region and the target, tested headlessly; `viewer-ui` is a cursor
position, a scale and a page number. Three things the clause decides, each in the model:

- **The activation region is `/QuadPoints` where the clause admits them**, and Table 176's third
  condition is the one a lenient reader gets wrong: "if any coordinates in the QuadPoints array
  lie outside the region specified by Rect then the activation region … shall be defined by its
  Rect entry". A stray quadrilateral is not a wider region — it is **no** region, and the
  rectangle stands.
- **`/Dest` and `/A` are exclusive**, and a URI, launch or ECMAScript action leads nowhere here
  by design: principle 3's sandbox is why §12.6.4.5 is absent.
- **Overlapping links resolve to the last one**, because the clause states no rule and the
  annotation drawn on top is the one under the cursor.

**Mapping a click back to the page is one function**, `user_space_at`, the inverse of the
transform every page is drawn under. §12.5.2 puts a `/Rect` "in default user space units" and
§7.7.3.3's `/Rotate` and `/CropBox` stand between that and a pixel — so a viewer that inverted
the *scale* alone would work on every unrotated page and fail on every rotated one.

| | |
|---|---|
| documents with a link on page one | **54**, holding **33 125** links |
| of those links, in one file | **32 768** — `bug1978317.pdf` is a stress test for exactly that; the other 53 documents share 357 |
| links leading to a page of their own document | **36**; the rest are URIs, which is what a web page printed to PDF produces |

| | before | now |
|---|---|---|
| §12.6.4.2's go-to action | `silent` | **`implemented`** — a link's `/A`, an outline item's and the catalog's `/OpenAction`, all three |
| ledger rows owing something in silence | 195 | **193** |
| pages agreeing with the reference consensus | 821 | **821** |
| **tests** | 643 | **646** |

What it taught:

- **The interesting half of a "viewer feature" is usually a clause.** Of this change, the mouse
  is four lines and the rest is Table 176's three conditions, §12.5.2's coordinate space and
  §7.7.3.3's rotation. Putting the region and the target in `pdf-model` is what let all of it be
  tested without a window — which matters here, because nothing in CI can open one.
- **A gate cannot ratchet what has no consumer.** §12.3.2's destinations were `partial` for three
  sessions with `/OpenAction` as their only user, and the corpus number that mattered — 36 links
  leading somewhere — could not be measured until something asked the question.

### The fifty-sixth session, in brief

**Every one of ISO 32000-2's 823 technical subclauses has been read against this code.**
`UNREVIEWED_CEILING` is **0**, and the assertion that guarded it is now an equality: a row that
arrives `unreviewed` — a future edition gaining a subclause, or `bin/ledger` finding one this
file lacks — fails the build until somebody reads it. ADR 0061.

| status | rows | |
|---|---|---|
| `implemented` | 256 | every normative requirement in the clause is executed |
| `partial` | 159 | some are, and the note says which are not |
| **`silent`** | **195** | not implemented, and nothing says so |
| `inapplicable` | 89 | a marking device, a layout engine or a production workflow |
| `out-of-scope` | 87 | on principle 5's closed exclusion list, which the row names |
| `reported` | 30 | not implemented, detected and named at runtime |
| `writer-side` | 7 | addresses a PDF writer; we do not create files |

**195 silences is the finding, and the number was 2 as recently as the forty-second session.**
Every one of the 193 that arrived since came from *reading* rather than from any change to the
code. `unreviewed` and `silent` are different admissions — *we have not asked* against *we
asked, and we owe it without saying so* — and the whole exercise has been converting the first
into the second, third or fourth. Where the silence is: clause 12's interactive half, where
almost every row is a *viewer* rather than a clause, and clause 14's structure, where none of it
changes a mark.

**And the fourth and last row waiting on the name-and-number-tree component closed on the way
in.** §14.7.5.4's structural parent tree is read: a page's `/StructParents` is the key into the
structure tree root's `/ParentTree`, and the marked-content identifier is "a zero-based index
into the array" that comes back. The clause states its own reason and it is worth keeping —
"[b]ecause a stream cannot contain object references, there is no way for content items that are
marked-content sequences to refer directly back to their parent structure elements".

The consumer is §14.9.4's replacement text on a *structure element*, which is the half the
previous session left `partial`. **8 corpus documents state one on page one**; 89 have a
`/StructTreeRoot`, 76 key the parent tree from page one, and 75 name an element. The one that
names none states an **empty array** for its key — a document saying its first page belongs to no
structure element — and the test asserts that rather than counting it as a failure.

`structure.rs` deliberately does *not* walk `/K` from the root. No consumer in this program needs
an ordering of elements: §12.3.2.3's structure destinations read one element's own entries, and
extraction asks about the element covering a sequence it is already inside. Building the tree
top-down would be a data structure with no user.

| | before | now |
|---|---|---|
| **ledger subclauses nobody has read** | 20 | **0**, out of 823 |
| ledger rows owing something in silence | 186 | **195** |
| pages agreeing with the reference consensus | 821 | **821** |
| corpus documents drawing with nothing reported | 858 | **858** |
| `§` citations the checker verified | 1542 | **1553** |
| **tests** | 640 | **643** |

What it taught:

- **The instrument produced work no other instrument could, and three kinds of it.** A missing
  *component* — four rows in two clauses naming one absent data structure, which no clause
  review and no corpus document would have surfaced. Three *false claims* — §8.7.3.1's `/BBox`,
  §8.7.2's pattern space, §14.6.2's "both forms" — each written from the clause during a review,
  each true of no code, and each found by the oracle rather than by the ledger. And a *ranking*
  a demand curve cannot give, because a demand curve cannot rank a requirement no file
  exercises.
- **A ledger note is a hypothesis the gates test, not a conclusion they inherit.** That is the
  right way round and it took three wrong rows to see it. `FILE_ONLY_EVIDENCE_CEILING` is the
  standing answer: 58 `implemented` rows still name a whole test *file*, which passes whatever
  it contains, and that number may only fall.
- **Reading every clause is not implementing every clause**, and the vocabulary says so: 195
  silences and 30 reports are the distance left. What changed is that the distance is itemised,
  and there is no longer anywhere for a requirement nobody has thought about to hide.

### The fifty-fifth session, in brief

**§14.9.4's `/ActualText`, and the reason it could not be read: the property list it lives in
was never a dictionary.** ADR 0060.

`/ActualText` is "a replacement, not a description, for the content" — a character substitution
a reader applies when *extracting* text, and the one row of §14.9 whose consequence this project
can already measure. `issue13226.pdf` shows `Mit`, a space glyph wrapped in
`/Span <</ActualText <FEFF00AD>>> BDC … EMC`, `arbei`, another such space, then `terinnen`: the
document is saying that the spaces are §14.8.2.3's hyphenation artefacts and the word is
`Mitarbeiterinnen`. We read back `Mit arbei terinnen`.

**The obvious implementation does nothing, and finding out why is the session.** The content
lexer yields tokens, not objects: `<<` and `>>` became `Object::Null`, so an inline property list
reached the operator dispatch as *five loose operands* and there was no dictionary to look in.
Optional content never noticed, because §8.11.3.2's list must be a **named** resource — a group
is an indirect object and §14.6.2 forbids indirect references inside a content stream — so the
one form that was implemented was the only form that one caller could use.

**And the ledger's row for §14.6.2 said "content.rs takes both forms". That is the third such
row in four sessions**, after §8.7.3.1's `/BBox` and §8.7.2's pattern space, and all three were
written from the clause during a review. This file carried the same claim in its own words —
"content.rs already parses that property list because §8.11.3.3's `/OC` arrives the same way" —
which is exactly the reasoning that produces the mistake: the two *do* arrive the same way, and
only one of the two ways was read.

`inline_dictionary` assembles the dictionary from the tokens, bounded in depth, leaving arrays
flattened because `TJ` and `d` have read their elements as separate operands since the
beginning. `true`, `false` and `null` are handled inside it — they lex as **keywords**, which is
how two corpus documents came to report `true` and `false` as *unknown operators*.
`/ActualText` is then applied at `EMC` by truncation, so nesting falls out for free.

| | before | now |
|---|---|---|
| `issue13226.pdf` reads back | `Mit arbei terinnen` | the soft hyphens the file states |
| **corpus documents drawing incompletely** | 97 | **96** — and nothing is drawn differently |
| pages agreeing with the reference consensus | 821 | **821**, over one *more* judged page |
| words found against `pdftotext` | 100% | **100%** |

**On the specification track, §14.8.4's twenty rows — the vocabulary of standard structure
types, which is most of what tagged PDF is.** All `silent`, all behind §14.7's tree, and **none
of it changes a mark**: a `H1` and a `P` are drawn by the same operators and differ only in what
they mean. Four things worth keeping:

- **The category is a property of the position, not of the name.** A type usable as either is
  inline "if the structure element is used inside a block level element" and block otherwise —
  the sort of rule an implementation gets wrong by building a table.
- **§14.8.4.8.3's header search is an algorithm, which is rare here**: where `/Headers` is
  absent, a cell's headers are found by searching towards the first cell in `/WritingMode`'s
  direction, stopping at the table's edge, at a data cell after a header cell, or at a header
  cell that states its own. Its NOTE says why no right-to-left special case is needed — "the
  structure always reflects the logical content order of the table" — which is §14.8.2.5.1
  doing the work it was defined for.
- **`Span` is the one type this tree already meets**, as the tag §14.9's four entries arrive
  under; one of the four is now read.
- **Ruby and warichu change nothing about drawing.** The glyphs are positioned by the content
  stream like any others; what the structure adds is knowing that the small text is a gloss.

| | before | now |
|---|---|---|
| **ledger subclauses nobody has read** | 40 | **20** — §14.8.5's attributes and §14.8.6's namespaces |
| `§` citations the checker verified | 1527 | **1542** |
| **tests** | 638 | **640** |

What it taught:

- **Two callers of one clause can use disjoint halves of it, and the half nobody uses is not
  implemented.** §14.6.2 has two forms; §8.11's optional content *cannot* use the inline one, so
  fifteen sessions of optional-content work proved nothing about it. When a row says "both
  forms", ask which caller exercises which.
- **A parser that recognises a delimiter without parsing it will be read as parsing it.** The
  comment saying only the brackets were recognised was accurate, present, and read by three
  people — including this file — as meaning the dictionary was available.

### The fifty-fourth session, in brief

**A page of twenty-four hard stripes was drawn with twenty-four seven-pixel gradients, and the
seven is arithmetic rather than coincidence.** ADR 0059.

`issue10572.pdf` states its steps exactly: an axial shading whose `/Function` is a type 3
stitching function over `[-6, 6]`, each of whose twelve sub-functions is *itself* a type 3 with
**`/Bounds [0.5 0.5]`** — a subdomain of zero width, which is how a producer writes a
discontinuity. A `Ramp` held 256 colours at even intervals, the shading's axis is 1800 units,
and 1800 ÷ 256 ≈ 7. Every step fell inside one sampling interval and both backends interpolated
across it, because that is what a gradient does between two stops.

**A ramp now carries a position per stop**, and `Function::breakpoints` reports where the
standard allows a jump — §7.10.4's `/Bounds`, recursively, each inner one mapped back through
`/Encode` and its subdomain, which is the exact inverse of what `eval_stitching` applies on the
way in. `Ramp::sample_across` puts **two stops at each break**, one sampled just below it and
one just above, which is how a gradient expresses a step in `tiny-skia` and in Vello alike.

Both backends already built their stops *with* positions, computing them from the index; they
now read the position. Three lines each.

**Sampling more finely would not have fixed it**, and saying why is the point: doubling the
resolution halves the smear and never reaches zero, while every shading in every document pays
sixteen times the stops. The clause states where the function jumps. A reader that has read it
does not have to guess.

**On the specification track, the first twenty of §14.8's sixty rows — tagged PDF's page-content
half.** Three things came out of it, and two of them are about *extraction* rather than drawing:

- **§14.8.2.4 is `implemented`, and it is the first row of clause 14 that is.** "[P]age content
  shall be considered to include all graphics objects in their entirety, regardless of whether
  they are visible when the document is displayed or printed" — text in rendering mode 3 is
  invisible and is extracted here, which is already a test.
- **§14.8.2.5.3's `/ReversedChars` would make `text_extraction.rs` return text backwards.** The
  tag says the show strings inside it hold their characters in reverse, for right-to-left text
  set in a left-to-right font; the glyphs are already positioned, so the *page* is correct
  either way. No corpus document writes one.
- **§14.8.2.2.1 makes the artifact question decidable without any tagging**: "Any content that is
  not included in the structure tree is an artifact even when not enclosed in a marked-content
  sequence using the tag Artifact." So in a tagged document the structure tree is the whole
  answer and the `/Artifact` tag is a convenience. 10 corpus documents write one; 78 carry a
  `/MarkInfo`.

§14.8.3's basic layout model is `inapplicable` for a reason worth stating once: reference areas
and progression directions exist so that a processor *reflowing* a document knows what §14.8.5's
attributes mean. This program lays nothing out — it draws the page the file describes, at the
positions the file states.

| | before | now |
|---|---|---|
| **pages agreeing with the reference consensus** | 820 | **821** |
| **pages contradicted** | 77 | **76** |
| **contradicted pages with no explanation** | 31 | **30** |
| corpus documents drawing with nothing reported | 858 | **858** |
| **ledger subclauses nobody has read** | 60 | **40** — all of them §14.8's second half |
| implemented rows naming a file rather than a test | 59 | **58** |
| `§` citations the checker verified | 1522 | **1527** |
| **tests** | 635 | **638** |

What it taught:

- **When a page's error has a suspiciously round size, do the arithmetic.** Seven pixels of
  gradient where there should be an edge is 1800 ÷ 256, and that division named the defect
  before any clause was opened. A measurement that matches a constant in your own code is not a
  coincidence.
- **A representation can forbid a correct answer.** No amount of care inside an evenly spaced
  array of colours can express a discontinuity; the fix had to change the *type*, and the same
  type is what a smaller, faster ramp would need. Ask what a data structure cannot say before
  asking what the code does wrong with it.

### The fifty-third session, in brief

**A page drew one of its two lines of text and left the other blank, and the clause is five
words long.** ADR 0058.

`pattern_text_embedded_font.pdf` sets `AbCdEf` twice — once filled with a shading, once with a
checkerboard **tiling** pattern. Three references draw both; this tree drew the shading line and
nothing where the other belonged. A tiling pattern is deliberately not a paint here — it is a
cell replayed across an area, which `end_path` does by calling `tile` — and **`show_text` did
not**. A glyph took `fill_paint()`, got the last solid colour set before the pattern was
selected, and painted with it.

§8.7.2: "All patterns shall be treated as colours". A glyph is filled with the fill colour; if
the fill colour is a pattern, the glyph is filled with the pattern. Nothing anywhere makes text
a special case.

**What it cost is the more interesting half, and it is trap 5's exchange running both ways on
one change.** Two documents started reporting:

- `scorecard_reduced.pdf` **strokes** with a tiling pattern, which needs the stroked outline the
  backends compute for themselves (ADR 0028) — there is no path here to replay a cell across. It
  had been stroked in the last solid colour, silently.
- `ContentStreamCycleType3insideType3.pdf` reports `MAX_FORM_DEPTH`, and the file is named for
  the reason: its pattern's `/Resources` name `/CyclicFont`, which **is** the Type 3 font whose
  glyph the pattern fills. Tiling a glyph means entering that cycle; stopping at a bounded depth
  and saying so is what the bound is for. Before this change the cycle was never entered,
  because the pattern was ignored, and the page was quietly wrong.

So `MAX_INCOMPLETE` rises 95 → 97 with both reasons written on it, and the oracle's judged set
falls by two pages.

**Three sessions in a row have found a defect in the pattern machinery** — the cell's `/BBox`
(ADR 0056), the space a pattern inside a form maps to (ADR 0057), and text as a pattern's target
(ADR 0058). All three were invisible to the corpus gate, which reports what cannot be *built*,
and visible only to the oracle, which asks whether what was built is right.

**On the specification track, §14.10's web capture — eighteen rows, all `inapplicable`, and the
clause retires itself in its first sentence**: "The features described in this clause are
deprecated with PDF 2.0." Every structure in it is a registry a *capturing* application keeps so
that it need not download what it already has, and no corpus document writes a `/SpiderInfo`.
Two sentences are worth carrying out of it:

- **§14.10.2's `/V` is "a single real number, not a major and minor version number"**, so 1.2
  exceeds 1.15 — the opposite convention to §7.5.2's header version and §7.12.4's
  `/BaseVersion`, which is exactly the kind of detail a reader gets wrong by analogy.
- **§14.10.3.2's NOTE generalises past this clause**: because PERCENT SIGN is both unsafe and
  the escape character, "no number of encoding or decoding passes on a URL can ever cause it to
  reach a stable state".

| | before | now |
|---|---|---|
| pages agreeing with the reference consensus | 820 | **820** — one gained, one lost to a new report |
| **pages contradicted** | 78 | **77** |
| **contradicted pages with no explanation** | 32 | **31** |
| **corpus documents drawing incompletely** | 95 | **97**, both new reports and both written down |
| **ledger subclauses nobody has read** | 78 | **60** — all of them clause 14 |
| `§` citations the checker verified | 1514 | **1522** |
| **tests** | 634 | **635** |

What it taught:

- **A feature switched off in one place is switched off everywhere it is not switched on.** The
  comment saying a tiling pattern "is not a paint at all" was correct, load-bearing, and
  attached to the one call site that knew what to do instead. The other call site did not exist
  when it was written. Where a value means "handle me specially", the handling is a property of
  the *type*, not of the place — and grepping for the other places is the cheap check nobody ran.
- **A report that arrives with a fix is worth reading twice.** Both of this session's new reports
  came from drawing something we had been ignoring: one names a real gap, and one names a cycle
  the document was built to contain. Neither is a regression and both look like one in the
  count.

### The fifty-second session, in brief

**A page drew its axes and not the surface inside them, with `unsupported: []` beside it, and
the clause that fixes it is the sentence *after* the one this tree implemented.** ADR 0057.

`issue6231_1.pdf` was second on the ratio-ranked unexplained list. Three references draw a
blue-to-red plot; we drew the frame. **The display list held the surface all along** — 79
commands, one a `Fill` whose paint is a type 5 mesh with every triangle and colour in it —
positioned about 180 points below and 140 to the left of where it belonged, so every triangle
fell outside the clip and the rasteriser drew nothing. Nothing failed, so nothing reported.

The surface is painted inside a form XObject. §8.7.2 has two consecutive sentences about what a
pattern's matrix maps *to*, and this tree had the first:

> If a pattern is used on a page … the pattern matrix maps pattern space to the default
> (initial) coordinate space of the page.

> Similarly, if a pattern is used within a form XObject …, the pattern matrix maps pattern space
> to the form's default user space (that is, the form coordinate space at the time the form is
> painted with the Do operator).

`base` is now saved, replaced with the form's own space while the form runs, and restored.
**Three pages left the contradicted list** — `issue6231_1.pdf` and both pages of
`issue6961.pdf`.

**And the finding behind the finding, for the second session running: the ledger's row for
§8.7.2 already said this**, in as many words, with status `implemented`, since the twentieth
session. ADR 0056 found §8.7.3.1's "`/BBox` clips the cell" in the same condition a session ago.
Two is a pattern, and its shape is: **a row written while *reading* a clause describes what the
code should do, and nothing in the gate can tell that from what it does.**

Both wrong rows named a whole test *file* as their evidence, which passes whatever it contains.
The conformance gate now counts and ratchets that:

```
59 of the implemented rows name a test file rather than a test
```

`FILE_ONLY_EVIDENCE_CEILING` may only fall. Deliberately a count and not a rule — the gate
cannot tell whether a named test *covers* its clause, and making it a rule today would mean
writing fifty-nine tests against the clock, which is the rubber stamp the ledger exists to
prevent. What it does is bound the population in which a false claim can hide.

**On the specification track, §14.9's accessibility support — eleven rows, and this is the
family that says how much of §14.8 is in scope.** `CLAUDE.md` puts "tagged PDF as far as
accessibility needs it" in scope; §14.9 is the definition of *as far as*, because everything it
asks for hangs off §14.7's structure tree or a `Span` property list. Four findings:

- **One piece is already implemented and it is the smallest.** §7.9.2.2.2's language escape
  sequences are recognised inside every text string this tree decodes — and *removed*, with the
  language discarded. Three rows are `partial` for exactly that.
- **§14.9.4's `/ActualText` is a text-extraction requirement, not a rendering one**, and it is
  the one row here with a consequence this project can already measure: it is "a replacement,
  not a description, for the content", and `text_extraction.rs` does not read it. The clause's
  example is a hyphenated German `Druk-`/`ker` whose `/ActualText` is `c`. 5 corpus documents
  write one; 95 write a `/Lang` and 87 a `/StructTreeRoot`.
- **§14.9.6 states its own obligation and it is none**: "A PDF processor is not required to
  process pronunciation hints" — `inapplicable`, the same reading §10.7.2's flatness permission
  gets.
- **§14.9.2.4 is the first `out-of-scope` row outside clauses 12 and 13.** A multi-language text
  array is defined in exactly two places, Table 285's and Table 288's media clip dictionaries,
  and both are clause 13's.

| | before | now |
|---|---|---|
| **pages agreeing with the reference consensus** | 817 | **820** |
| **pages contradicted** | 81 | **78** |
| **contradicted pages with no explanation** | 35 | **32** |
| corpus documents drawing with nothing reported | 858 | **858** |
| **ledger subclauses nobody has read** | 89 | **78** — all of them clause 14 |
| `§` citations the checker verified | 1510 | **1514** |
| **tests** | 633 | **634** |

What it taught:

- **A display list that holds the right commands can still draw nothing, and no report will
  say so.** The mesh was complete, correct and 180 points away. Between "we could not build it"
  and "we drew it" there is a third state — *built and placed wrongly* — which every gate this
  project has is blind to except the oracle, and which the oracle only catches because another
  implementation drew it.
- **Two false `implemented` rows in two sessions is an instrument problem, not two mistakes.**
  Both were written from the clause during a review, both named a test file rather than a test,
  and the fix is to count that population rather than to promise to be careful.

### The fifty-first session, in brief

**A page in the unexplained list differed from three references by 128 levels of 255 in one
tile, and the rule that fixes it is one sentence this project's own ledger claimed to have
implemented.** ADR 0056.

Ranking the 36 unexplained contradicted pages by *our worst measurement over the bound it is
held to* — the handover's ratio, not a distance — put one page at **25.7×** and the next at 3.2×:

| | mean / bound | worst tile / bound | differing |
|---|---|---|---|
| `tiling-pattern-large-steps.pdf` page 1 | 1.60 / 1.00 | **128.49 / 5.00** | 0.79% |

A mean of 1.6 levels with a worst tile of 128 is not anti-aliasing and not a colour conversion:
it is a region one implementation draws and another does not, small next to a 4000-point page.

The file is 983 bytes and every one is legible. A tiling pattern's cell paints a rectangle to
x = 4000 inside a `/BBox` that ends at 3950. Sampling one row settles it with no tolerance
involved — **poppler, ghostscript and `hayro` stop at 3950; we and `mupdf` ran to the end of the
page.** Table 74 says which is right in one sentence: "These boundaries shall be used to clip the
pattern cell."

**This tree carried no `/BBox` on a tiling pattern at all.** The clip is now per *cell*, which is
what the clause is for: content past one cell's box would otherwise spill into its neighbour's,
and where `/XStep` exceeds the box — how a pattern tiles with gaps — into the gap between them. A
box with no extent is left unclipped rather than emptying the cell, because Table 74's NOTE 1
says "[a] BBox of zero height or width will still paint one pixel".

**And the finding behind the finding: the ledger's row for §8.7.3.1 has said "`/BBox` clips the
cell" since the twentieth session.** Written from the clause, never true of the code, and no test
asked. The row was `partial` for a different reason — `/TilingType` — so the status never looked
wrong either. **A row can be wrong by claiming as well as by disclaiming**, which is the mirror
of the forty-ninth session's §12.3.2.3, and the rule is now on the row: a note is a claim, and
only a test makes it a fact.

**On the specification track, §14.12's document parts — seven rows, and the family divides
exactly along who each sentence addresses.** §14.12.1 states its own purpose — job tickets, "a
production workflow, digital printing device, or other messaging channel" — so the family is
`inapplicable` for the same reason §14.2's procedure sets and §14.5's page-piece dictionaries
are. But **§14.12.2 and §14.12.3 are `writer-side`**: every sentence in them constrains how a
file is built ("shall not overlap", "shall have a DPart key", ranges "monotonically increasing").
Their NOTE 2 is what makes a reader owe nothing — "the pages will be enumerated the same using
either mechanism", so this tree's page order already *is* the DPart hierarchy's. **No corpus
document states a `/DPartRoot`.**

| | before | now |
|---|---|---|
| **pages agreeing with the reference consensus** | 816 | **817** |
| **pages contradicted** | 82 | **81** |
| **contradicted pages with no explanation** | 36 | **35** |
| corpus documents drawing with nothing reported | 858 | **858** |
| **ledger subclauses nobody has read** | 96 | **89** — all of them clause 14 |
| `§` citations the checker verified | 1509 | **1510** |
| **tests** | 632 | **633** |

What it taught:

- **Rank by the ratio, and the top of the list is a different kind of thing from the rest.** A
  25.7× against a 3.2× runner-up was not a page needing a careful eye; it was a rule nobody had
  implemented, visible in 983 bytes and four sampled pixels. Four sessions in five that the
  pairwise or ratio ranking has chosen the next item, it has been right before any artefact was
  opened.
- **Agreement with one reference is not evidence.** `mupdf` drew what we drew, which felt like
  support and was one implementation reading the clause as we had. The clause is what changed our
  mind, and the entry says so.

### The fiftieth session, in brief

**§12.3.3's document outline, and a `/Count` the clause states as an algorithm rather than as a
number — so a document states one fact twice and can be checked against itself.** ADR 0055.

The outline is the second thing downstream of the destinations built a session ago and the third
of the four rows the name trees unblocked. 176 of the 974 documents have one. It is also the
first item in this project whose whole purpose is a *panel*, in a viewer that has none, and the
answer to that is: **read the whole hierarchy, and use the part a viewer without a panel can
use.** `Outline::section_at` gives the innermost item covering the page being drawn, and the
title bar now says which section a reader is in beside the page number and the §12.4.2 label.
That mapping is a **documented choice** — §12.3.3 describes a panel a person clicks and says
nothing about going the other way.

**Table 151 defines `/Count` in three numbered steps**, over "those immediate children whose
Count is positive" — the open ones. Running the steps over what was read and comparing with what
the file states is a check on *this reader*: a walk that lost a level, took a closed item's
children as visible, or ran off a `/Next` chain would disagree with every producer that ran the
same steps.

| | states | the steps give |
|---|---|---|
| **144 of 146 documents** | agree | — |
| `nested_outline.pdf` | 3 | **9** — its three top-level items each carry `/Count 2`, which by step 3 makes them open and their six children visible |
| `outline_goto_action.pdf` | 1 | **2** — one parent, open, and one child |

Both exceptions contradict *themselves*, and both are hand-written pdf.js fixtures whose root
count was written rather than computed. **A number counted from the items is the clause's; a
number written beside them is a claim.**

Two more things the walk decides, each written where it is made. **Follow `/First` and `/Next`
and nothing else**: the clause threads six indirect references per item — `/Prev`, `/Last` and
`/Parent` are redundant with the forward walk and could only disagree with it. And **twenty-six
documents state an `/Outlines` that yields no items**, every one of them
`<< /Type /Outlines /Count 0 >>` with no `/First`, which Table 150 permits; the test asserts that
per document, so a root *with* a `/First` producing nothing would fail rather than be counted.

**On the specification track, §14.13's associated files — eleven rows, all of them read against
`/AF`, which nothing here has ever looked at.** The family is `silent` and §14.1's own opening
sentence is why it changes nothing on a page: "[t]he features described in this clause do not
affect the final appearance of a document." Three findings worth carrying:

- **Five of the eight objects that may carry `/AF` are objects this tree already parses** — the
  catalog, a page, a marked-content property list, an XObject, an annotation. The entry is one
  array lookup away wherever it matters; what does not exist is anything to do with the result.
- **§14.13.5 touches the rendering path without changing it.** Content is associated with a file
  by bracketing it in `BDC`/`EMC` with the tag `/AF` — the same mechanism §8.11.3.3's `/OC` and
  §14.7.5.2's `/MCID` arrive by, which `content.rs` already parses. An `/AF` bracket this tree
  ignores draws exactly what the clause says it should. Two prohibitions come with it: `DP` and
  `MP` "shall not be used with the AF tag", and `BMC` cannot be, because it takes no properties.
- **§14.13.6 genuinely needs the structure tree**, and saying so is worth something now that
  §12.3.2.3 turned out not to. The difference is exact: a structure *destination* reads one
  element's own `/K` and `/Pg`; an associated file on a structure element needs the element to be
  findable, which is the tree.

| | before | now |
|---|---|---|
| corpus documents drawing with nothing reported | 858 | **858** |
| pages agreeing with the reference consensus | 816 | **816** |
| **ledger subclauses nobody has read** | 107 | **96** — all of them clause 14 |
| ledger rows owing something in silence | 138 | **147** |
| `§` citations the checker verified | 1500 | **1509** |
| **tests** | 626 | **632** |

What it taught:

- **A clause that states an algorithm is a clause that can audit a corpus.** §12.4.2 gave nine
  labels beside a tree and those became the test; §12.3.3 gives three steps and a number, and
  those became a test over 146 documents that checks the reader against every producer at once.
  Look for the requirement stated as a *procedure* — it is worth more than the same requirement
  stated as a value.
- **"Two documents disagree with us" is a question, not a defect.** Both here disagree with
  themselves as well, which is visible only because the check was run on the file's own two
  statements rather than on ours against theirs.

### The forty-ninth session, in brief

**§12.3.2's destinations, in all three of the spellings the clause gives one object** — and the
finding is that the row saying it could not be done was this project's own. ADR 0054.

A destination is what an outline entry, a link, a go-to action and the catalog's `/OpenAction`
point at, and none of those four can be built before the thing they point at can be read.
`Destination::read` takes an explicit array (§12.3.2.2), a structure element (§12.3.2.3), a name
or a string (§12.3.2.4) or a dictionary with `/D`, because a caller holding `/Dest` does not know
in advance which it has and the clause never suggests it should.

**The ledger's row for §12.3.2.3 said it "needs §14.7's logical structure tree — unread, and the
reason this row cannot be closed before clause 14's is." That is wrong, and reading the clause is
all it took.** §12.3.2.3 states the whole algorithm in terms of one element's `/K` and `/Pg`:
kids in linear array order, a marked-content or object reference answering with its page, a child
element recursed into. Nothing in it needs the tree rooted, needs `/StructTreeRoot`, or needs the
parent tree. **Measure an entry before believing its label — including a label this project wrote
three weeks ago**, which is the forty-third session's lesson pointed at the ledger instead of at
the corpus.

Three more things the clause states and a first implementation loses:

- **A null parameter is not a zero.** "A null value for any of the parameters left, top, or zoom
  specifies that the current value of that parameter shall be retained unchanged" is an
  instruction, not a missing value — so every one is an `Option<f32>`, and "[a] zoom value of 0
  has the same meaning as a null value" arrives as `None`.
- **A page number is not a page index.** §12.3.2.2's NOTE gives the integer first entry to remote
  and embedded go-to actions, whose page is in another document, so it names nothing here.
- **The page tree decides what the first entry is**, before any `/Type` is read: a reference the
  page tree holds is a page whatever the object claims, and only one it does not hold can be a
  structure element.

**The consumer is `/OpenAction`, the one destination a viewer must resolve without anybody
clicking anything**, and the viewer now opens there. Table 29 states the other half — an absent
entry means "the top of the first page" — which is also what an unresolvable one gets, with
nothing reported, because the clause has already said what to do.

**What the corpus says, and the second number is the one to keep:**

| | |
|---|---|
| documents stating an `/OpenAction` | **55 of 974**, and 49 name a page this reader finds |
| named destinations reachable from links | **106, of which 22 resolve** |
| the other 84 | keys **their own document does not define** — five files carry named links and no table at all, and `pdfjs_wikipedia.pdf` links to 27 `cite_note-…` anchors while its table defines `cite_ref-…` |
| §12.3.2.4's name-to-dictionary, string-to-tree pairing | **exact**: the 22 are 2 names in a catalog `/Dests` and 20 strings in a name tree, not one crossing |

The test asserts the *two-sided* fact rather than the ratio — every key that **is** in a table
resolves — so a regression appears as a key we failed to find and not as a number moving.

**On the specification track, §7.11's file specifications and §7.12's extensions dictionary —
eighteen rows, and clause 7 is complete for real this time**, checked by the one-line grouping
the forty-eighth session added rather than by what this session touched.

- **§7.11 is decided by one sentence and by principle 3.** "The file is considered external to
  the PDF file in either case" — embedded or not — and this renderer has no filesystem and no
  network. So the family is refused by architecture rather than unimplemented by accident, and
  every path in the tree that meets a specification already refuses out loud: §7.3.8's external
  stream data, §8.10.4's reference XObject drawn as its proxy, §12.5.6.15's attachment icon.
  What is owed *in silence* is smaller and specific: embedded file streams need no filesystem at
  all, 10 of the 974 documents carry an `/EmbeddedFiles` tree, and §7.9.6's reader now walks one.
- **§7.11.2.2 hides a security rule inside a syntax rule.** A URL-based relative specification
  "shall be limited to paths", with "[t]he scheme, network location/login, fragment identifier,
  query information, and parameter sections" not allowed. Whoever implements it should enforce
  that rather than assume it.
- **§7.12 and §12.11 are the same idea twice.** An extensions dictionary is a document declaring
  what a processor needs to be able to do, and so is a requirements dictionary; this tree reads
  neither, while reporting per feature exactly what it cannot draw. **9 of the 974 documents
  state one and every one is Adobe's `ADBE`.** Putting the document's own claim beside our report
  is cheap and nobody has — the forty-sixth session said so of §12.11 and this is the second half
  of the same sentence.

| | was | is |
|---|---|---|
| §12.3.2.3, structure destinations | `silent`, "cannot be closed before clause 14's" | **`implemented`**, and clause 14 was never involved |
| §12.3.2.4, named destinations | `silent` | **`implemented`**, both tables and both key forms |
| rows the ledger found blocked on a name tree | 3 | **2** — §12.7.7's named pages and §14.7.5.4's `/ParentTree` |
| **clauses with no `unreviewed` row** | 8–13 | **7–13**, and this time the count was taken over the clause |

**The numbers:**

| | before | now |
|---|---|---|
| corpus documents drawing with nothing reported | 858 | **858** |
| pages agreeing with the reference consensus | 816 | **816** |
| **ledger subclauses nobody has read** | 125 | **107** — all of them clause 14 |
| ledger rows owing something in silence | 132 | **138** |
| `§` citations the checker verified | 1465 | **1500** |
| **tests** | 615 | **626** |

One correction to this file's own arithmetic: it said the oracle compares **1656** pages we call
complete against 138 incomplete. The gate prints **1655 and 139**, and printed them at the
previous commit too — checked by stashing this session's work and running it again, which is the
forty-seventh session's rule applied to a number rather than to a claim.

What it taught:

- **A ledger row is an entry, and an entry gets measured before it gets believed.** §12.3.2.3
  cost an afternoon and its row had priced it as a whole clause of clause 14. The failure mode is
  the same one `mesh_shading_empty.pdf` had for fifteen sessions, and it is worse here: a
  corpus entry is at least a note about somebody else's file, while this was a note this project
  wrote about its own reading.
- **A gap measured on both sides is a fact; measured on one side it is an accusation.** "22 of
  106 named destinations resolve" reads as a broken reader until the other half is checked —
  every key that exists in a table is found — and then it reads as five files with no destination
  table and one with the wrong anchors.

### The forty-eighth session, in brief

**The ledger found a missing *component* rather than a missing feature, and this session built
it.** Four `silent` rows in two clauses named the same absent thing: §12.3.2.4's named
destinations, §12.4.2's page labels, §12.7.7's named pages and §14.7.5.4's `/ParentTree` all
need a **name or number tree**, and nothing in this project read one. No single clause review
would have shown that and no corpus document would ever have asked for it — a demand curve ranks
features, and this is not a feature.

`pdf-syntax::tree` is §7.9.6 and §7.9.7 as one module, because §7.9.7 defines itself by
difference: "similar to a name tree … except that its keys shall be integers instead of
strings", with `/Nums` for `/Names`. `TreeKey` is that difference and nothing else. Two entry
points for two shapes of question — `lookup` descends, which is the clause's own reason for the
structure ("looked up efficiently without requiring the entire data structure to be read"), and
`number_pairs` walks the whole tree, which §12.4.2 needs because a labelling range runs to the
*next* key and no lookup produces a neighbour. **`/Limits` are a hint, not a gate**: real files
get them wrong, and a reader that trusted them would lose entries that are there.

**Then the first thing built on it: §12.4.2's page labels, in full.** Chosen because it is the
only row in clause 12's navigation half with no user-interface question in it — a label is a
string computed from the document — and because `CLAUDE.md` names it in scope. Four things the
clause states that a first implementation gets wrong, each now in the code with its sentence:

- **There is no default numbering style.** A range with a `/P` and no `/S` gives every one of
  its pages the *same* label, which the clause's own NOTE spells out with `Contents`.
- **`/A` is not base 26.** "A to Z for the first 26 pages, AA to ZZ for the next 26" — the
  twenty-eighth page is `BB`, where base 26 says `AB`.
- **The Roman form is subtractive**, which the clause fixes not by stating an algorithm but by
  its example running `i, ii, iii, iv` rather than `iiii`.
- **`/St` "shall be greater than or equal to 1"**, so a file writing zero gets the default.

**The clause's worked example is the test** — three ranges and the nine labels the standard
prints beside them, `i ii iii iv 1 2 3 A-8 A-9` — because no corpus document exercises all three
forms, which is trap 8. The corpus test is the other half: **22 of the 974 documents state page
labels** and every one labels its first page, as §12.4.2 requires. `viewer-ui` shows the label
beside the index rather than instead of it, since a title reading `iv` cannot also say `of 320`.
ADR 0053.

**And this file has been wrong for six sessions about which clauses are complete.** The
fortieth session's summary said "clauses with no `unreviewed` row: 7, 8, 10, 11, 13" after
reviewing §7.5, and every session since repeated it — up to "seven of the standard's eight
technical clauses" and "what remains is clause 14 alone". **Clause 7 was never complete.**
§7.10's functions, §7.11's file specifications and §7.12's extensions dictionary were all
unread, and nothing checked, because the count was taken over the families a session had
*touched* rather than over the clause.

The checking took one line — `unreviewed` rows grouped by their leading clause number — and it
is now the last thing this file's numbers are taken from. §7.10 went in with the correction and
was the cheap review the mistake had hidden: eleven rows, all `implemented`, for the four
function types that §8.7's shadings, §8.6.6's tint transforms and §11.5's `/TR` all run through.
§7.7.4's name dictionary is `silent` and is the **smallest missing piece in the navigation
family** — the three rows still waiting on a name tree reach their data through it.

| | was | is |
|---|---|---|
| **clauses actually complete** | claimed 7–13 | **8–13**, and clause 7 is 120 of 138 |
| name and number trees (§7.9.6, §7.9.7) | `reported` | **`implemented`** |
| page labels (§12.4.2) | `silent` | **`implemented`**, and shown in the title bar |
| rows whose missing piece was a tree | 4 | **3**, and each now needs only its own semantics |

**The numbers:**

| | before | now |
|---|---|---|
| corpus documents drawing with nothing reported | 858 | **858** |
| pages agreeing with the reference consensus | 816 | **816** |
| ledger subclauses nobody has read | 136 | **125** — 107 of clause 14 and 18 of clause 7 |
| `§` citations the checker verified | 1444 | **1465** |
| **tests** | 606 | **615** |

What it taught:

- **A ledger with a status per subclause can find a missing component, not only a missing
  feature** — but only if the rows are written well enough to be read *across* clauses. That is
  an argument for the prose the notes carry rather than against it, and it is the first time
  this instrument has produced a work item that no gate, no corpus and no single clause could
  have.
- **The standard sometimes states answers rather than rules, and those are the tests to write.**
  §12.4.2 gives no algorithm for Roman numerals and no formula for its letters; what it gives is
  nine labels beside a tree, and every one of the four mistakes above fails at least one of
  them.

### The forty-seventh session, in brief

**A `.max(0.0)` was answering a question nobody had asked, and one contradicted page is where
the three possible answers pull apart.** `issue19633.pdf` strokes one diagonal under
**`-0.1 w`**. §8.4.3.2 says the line width "shall be a non-negative number expressed in user
space units", so the value is outside the parameter's domain and the clause states no recovery.

Four readings exist and each renderer takes a different one:

| | draws |
|---|---|
| **ours** — clamp into the domain, then apply §8.4.3.2's rule for zero | one device pixel, solid |
| `poppler`, `mupdf` — the magnitude | about a tenth of a pixel's coverage, very faint |
| `ghostscript` | between the two |
| the clause's own definition of stroking, applied literally — "all points whose perpendicular distance … is less than or equal to half the line width" | **nothing**, and nobody takes it |

Ours is defensible and was never written down: the clamp sat in the `w` handler with no comment
and the rule for zero sat in `Stroke::device_width` with a comment about zero. The choice is now
stated in both places and on §8.4.3.2's ledger row, and the page moves to
`CONTRADICTED_NEGATIVE_LINE_WIDTH`. **One operator in one of 974 documents**, measured — so the
corpus cannot rank this and the clause does not decide it.

**On the specification track, §14.7's nineteen rows — logical structure, which is the half of
clause 14 `CLAUDE.md` puts in scope** ("tagged PDF as far as accessibility needs it"). None of
it is read, so all nineteen are `silent`, and two things came out of writing them:

- **Everything a structure tree hangs on already exists.** §14.7.5.2 attaches an element to
  content through a `BDC` property list carrying an `/MCID`, and `content.rs` already parses
  that property list because §8.11.3.3's `/OC` arrives the same way. §14.7.5.3 names annotations
  and XObjects as content items, and both are drawn. The tree is the missing part, not its
  attachments.
- **Four rows in two clauses are waiting on one data structure.** §14.7.5.4's `/ParentTree`,
  §12.3.2.4's named destinations, §12.4.2's page labels and §12.7.7's named pages all need a
  **number or name tree**, which nothing in this tree reads. That is the clearest single item
  the ledger has produced: one small piece of §7.9.6 and §7.9.7 unblocks four families.

| | before | now |
|---|---|---|
| a negative line width | `.max(0.0)`, undocumented | a documented choice, in three places |
| **contradicted pages with no explanation** | 37 | **36** |
| **ledger subclauses nobody has read** | 155 | **136** |
| corpus documents drawing with nothing reported | 858 | 858 |
| pages agreeing with the reference consensus | 816 | 816 |

What it taught:

- **A count taken over what you touched is not a count.** This file said clause 7 had no
`unreviewed` row left, because the session that reviewed §7.5 had listed clause 7's remaining
rows as "all of them §7.5's file structure" — which was true of the rows that session had
looked at and false of §7.10, §7.11 and §7.12, which nobody had. Six sessions repeated it and
grew it into "seven of eight technical clauses complete". The check is one line, grouping the
ledger's `unreviewed` rows by their leading clause number, and it now runs before any claim
about coverage is written. **Whatever this file asserts, run it once** — including the
arithmetic it did about itself.

**A clamp is a decision.** `width.max(0.0)` reads as defensive hygiene and is in fact this
  program's whole answer to a value the standard forbids — chosen once, by nobody, and visible
  on a page. Look at what a `max`, a `clamp` or an `unwrap_or` decides before calling it a
  guard.
- **The ledger can find a missing *component*, not only a missing feature.** Four rows across
  two clauses named the same absent data structure, which no single clause review would have
  shown and no corpus document would ever have asked for.

### The forty-sixth session, in brief

**Clause 12 is complete as a review — 166 rows, none `unreviewed`.** With 8, 9, 10, 11 and 13's
exclusions, that is six of the standard's eight technical clauses. *(This paragraph said seven
and named clause 7; see the forty-eighth session for why that was wrong.)*

The seventy rows this session added are form actions, FDF, digital signatures, measurement
properties, geospatial features and document requirements — and **the shape of the answer
matters more than the count**. Three things came out of reading them:

- **A signed document renders correctly and completely.** A signature's *appearance* is an
  ordinary widget annotation, drawn by §12.5 like any other; what is missing is the assertion
  that the file has not changed. And the one place signatures touch rendering is already right
  and was already measured — §7.6.2's exemption of a signature's `/Contents` from decryption,
  which eight corpus documents would need and none of them is encrypted.
- **§12.11's document requirements are the closest thing in the standard to principle 3's own
  rule.** `/Requirements` is a document stating what a processor must be able to do, with a
  penalty if it cannot. This tree reports what it cannot *draw*, per feature, and does not read
  the document's own claim — putting the two side by side would be cheap and nobody has.
- **§12.7.6.3's reset-form and §12.6.4.13's `/SetOCGState` are two actions whose machinery is
  entirely built.** Resetting a field to its `/DV` reaches §12.7.4.3's layout, which already
  rebuilds an appearance under `/NeedAppearances`; setting an optional-content group's state
  reaches §8.11, which is implemented in full. In both, what is missing is the value changing.

**And the median page was profiled, which the handover has asked for since the seventh
session.** `hayro-speed` over the corpus: **median 2.12× slower** across the 853 pages we draw
completely, our total 8.28 s. Callgrind on `examples/callgrind_interpret` says where
interpretation goes, and the answer is not what the "small and text-heavy" guess suggested:

| | share |
|---|---|
| `zlib_rs::inflate` | **28.9%** |
| `Interpreter::show_text` | 6.7% |
| `read_fonts::ps::agl::name_to_char` | 4.3% |
| `Lexer::next_token` | 4.2% |
| `inflate_table` | 4.2% |

**Nearly a third of interpreting a page is inflating it**, which is a dependency doing exactly
its job and is the answer to "where does the time go" for the typical page. The AGL entry is
ours and was a repeated search: §9.10.2's second method asks a four-thousand-entry list for
every character a page shows in a font with no `/ToUnicode`, and a font has at most 256 codes.
Resolving each code once is **2 013.8 M instructions to 1 989.1 M**, 1.2% of the whole, and the
AGL's share falls from 4.26% to 3.35% — what remains is §9.6.5.4's load-time route, which is
already once per font.

| | before | now |
|---|---|---|
| **clauses with no `unreviewed` row** | 8–11, 13 | **8–13** — and see below: this file said 7 for six sessions and was wrong |
| **ledger subclauses nobody has read** | 225 | **155** |
| interpreting the specification's page | 2 013.8 M | **1 989.1 M** |
| median page against `hayro` | 2.14× (818 pages) | **2.12× (853 pages)** |
| corpus documents drawing with nothing reported | 858 | 858 |
| pages agreeing with the reference consensus | 816 | 816 |

What it taught:

- **The guess about where the time goes was wrong, and it was in this file.** "The typical page
  is small and text-heavy, so the candidates are parsing, font loading and per-page setup rather
  than rasterisation, but that is a guess" — the largest single item is `flate2`'s inflate at
  28.9%, which is neither, and the largest item that is *ours* was a cache nobody had noticed
  was missing.
- **A count of `silent` rows is a map of a project's shape.** Clause 12 finished with 113 of
  them, and every one is a *viewer* rather than a clause: this program renders pages correctly
  and does nothing when a person clicks on one. That is a true summary of the project, and it
  did not exist as a number until the rows were written.

### The forty-fifth session, in brief

**A page's label was wrong and the measurement that replaced it says which renderer is right.**
`issue9915_reduced.pdf`'s entry had said, since the thirty-first session, that "our letters sit
about 1.39× closer together than `poppler`'s and `mupdf`'s — which is 1000/719 … [s]omebody is
not reading `/W`; the clause says which of us should be." Both halves needed work.

The ratio over the ink span is 1.20, not 1.39. And the fact that settles it is not a ratio at
all: **`poppler` and `mupdf` space the five letters 20 pt apart and the four digits about 15,
in one line, in one font** — 20 pt is the `/DW` default of 1000 that this document does not
state, and 15 is `/W`'s 719. *Their* spacing is consistent with no single reading of the array;
ours is 14.38 pt throughout, which is 719/1000 at 20 pt exactly, and `ghostscript`'s ink columns
are ours to the pixel. §9.7.4.3 makes `/W` the source of a CID's width, so this is not a tie —
and the page moves to `CONTRADICTED_REFERENCE_GLYPH_WIDTHS` with the numbers on it.

**Measure an entry before believing its label, including one this project wrote** — for the
eighth time, and the instrument was ten minutes with two rasters and the display list.

**On the specification track, §12.6's twenty-three rows — actions — and four of them are the
first `out-of-scope` rows outside clause 13.** `CLAUDE.md` splits this family down the middle:
field *appearance* is in scope and field *behaviour* is not, and "JavaScript and script-driven
form behaviour" is on the closed exclusion list by name. So §12.6.4.17's ECMAScript actions are
`out-of-scope` — the row that exclusion was written for — with rendition, Go-To-3D-View and
Rich-Media-Execute excluded by the multimedia entry, and the other nineteen are `silent`.

Three of the nineteen are worth more than the rest, because **they change what is drawn rather
than where the reader goes**, and in every case the mechanism underneath is already built:

- **`/SetOCGState`** turns optional content groups on and off, and §8.11 is implemented in full
  — configuration, membership, visibility expressions and all. What is missing is the ability to
  *change* the answer after the page is built.
- **`/Hide`** sets §12.5.3's Hidden flag, which `annotation.rs` already honours.
- **`/Trans`** runs §12.4.4's page transition, which is `silent` with it.

And one is worth naming for the opposite reason: **`/Launch`'s absence is a security property
rather than a gap.** Principle 3's sandbox exists so that a document cannot reach the machine,
and this action is a document asking to. It should stay absent until somebody writes the
argument.

| | was | is |
|---|---|---|
| `issue9915_reduced.pdf`'s entry | "somebody is not reading `/W`" | measured: the references' own line is internally inconsistent |
| **ledger rows owing something in silence** | 24 | **43** |
| **`out-of-scope` rows outside clause 13** | 0 | **4** |

**The numbers:**

| | before | now |
|---|---|---|
| corpus documents drawing with nothing reported | 858 | **858** |
| pages agreeing with the reference consensus | 816 | **816** |
| **contradicted pages with no explanation** | 38 | **37** |
| **ledger subclauses nobody has read** | 248 | **225** |
| tests | 606 | **606** |

What it taught:

- **An inconsistency *inside* a reference's own output is worth more than any distance from
  it.** Two renderers spacing one line of one font by two different widths cannot both be
  reading the document's `/W`, and no tolerance, ratio or vote was needed to see it — only
  the ink columns.
- **The exclusion list earns its keep at a row rather than at a clause.** §12.6 is one family
  in which four rows are excluded by principle 5 and nineteen are owed, and a status per row is
  what keeps "we chose not to" from spreading over "we have not got to it yet".

### The forty-fourth session, in brief

**A font stated which way round its own offsets were, twice, and we were the only renderer not
reading it.** `issue2537r.pdf` drew three `.notdef` boxes where three references draw
`LINE UP`, and it was the last page in the unexplained list where we differed from *every*
reference while they agreed with each other — 10.3 levels from each against their own closest
agreement of 1.03. The pairwise table ranked it by exactly that ratio.

Nothing about the PDF is wrong: `Identity-H` with no `/CIDToGIDMap` makes the CID the glyph
index, and CIDs 47, 44, 49, 40, 3, 56, 51 are `L I N E ␣ U P` in the standard Macintosh glyph
order. The **font** states `indexToLocFormat` as `0x0100` — 1 written in the wrong byte order,
and neither of the two values ISO/IEC 14496-22 defines — so `skrifa` read `loca` at the wrong
width and found no outlines, while still producing a few by coincidence, which is why the font
loaded and **nothing reported**.

`repaired_loca_format` rewrites those two bytes, and **the file decides the value**: `loca`'s
last entry is `glyf`'s length under exactly one of the two readings (2056 against 0, for a
`glyf` table of 2056 bytes), and `loca`'s own length is `4 × (n + 1)` rather than
`2 × (n + 1)` (244 against 122, for 60 glyphs). Two independent statements, one answer, no
other implementation involved. A font that satisfies neither is left alone, and that case has
its own test — a repair that always succeeds is a guess wearing a derivation's clothes.
ADR 0052.

**On the specification track, §12.1, §12.2 and §12.4 — nine rows, all `silent`, and the
ledger's count of that status reaches 24 from 2 three sessions ago.** Every one of the
twenty-two that arrived came from *reading*, not from any change to the code.

Two are worth carrying. **§12.4.2's page labels** are the only row in clause 12's navigation
half with no user-interface question in it — a label is a string computed from the document,
and `CLAUDE.md` names it in scope; it needs a **number tree**, which is also what §12.3.2.4's
named destinations need, so two rows share one missing piece. And **§12.2's viewer
preferences** are mostly about a window this program does not have, except `/PageLayout`,
`/PageMode` and `/Direction`, which change what a reader sees.

| | was | is |
|---|---|---|
| a byte-swapped `indexToLocFormat` | `.notdef` boxes, silently | repaired from the font's own two statements |
| **pages agreeing with the reference consensus** | 815 | **816** |
| **contradicted pages with no explanation** | 39 | **38** |
| **ledger rows owing something in silence** | 15 | **24** |

**The numbers:**

| | before | now |
|---|---|---|
| corpus documents drawing with nothing reported | 858 | **858** |
| **pages agreeing with the reference consensus** | 815 | **816** |
| **pages contradicted** | 83 | **82** |
| **ledger subclauses nobody has read** | 257 | **248** |
| tests | 603 | **606** |

What it taught:

- **Rank the unexplained list by a ratio, not by a distance.** Our nearest reference divided by
  the references' own nearest pair puts the pages where *we* are the outlier at the top, and it
  is the third time in five sessions that the pairwise table has picked the next thing to work
  on before an artefact was opened.
- **A font is a file too, and a file that states one fact twice can check itself.** The habit
  was learned from LZW stream lengths and from ninety-six JBIG2 encodings of one image; it
  applies just as well one level down, inside a font program, where the table directory states
  a length that the table itself states again.

### The forty-third session, in brief

**The mesh subdivision lattice is gone, three contradicted pages went with it, and the page
that showed the defect got 11.5× faster.**

§8.7.4.5.5 asks for Gouraud interpolation across a triangle. Neither rasteriser has one, so
both backends subdivided each triangle by quarters — up to 4096 pieces — and filled each piece
with the *mean* of its corners. That produced three defects and only the first was ever written
down: a visible lattice, a systematic *bias* (a mean is not the colour at any of the piece's
pixels), and seams that had to be closed by growing every piece by `SEAM_OVERLAP = 0.8` pixels,
a constant found by sweeping it until the backends agreed.

`pdf_render::MeshRaster` now rasterises the mesh once at device resolution: a pixel's colour is
the clause's interpolation at that pixel's *centre*, and a pixel belongs to the triangle whose
interior contains its centre. **Adjacent triangles then tile exactly**, so there are no seams
to repair — `SEAM_OVERLAP`, `FLAT_ENOUGH` and `MAX_SUBDIVISION` are all deleted. Each backend
draws the raster as a nearest-sampled image at 1:1, so the colour is `pdf-render`'s, identically
on both, and the edge is the backend's, antialiased as every other fill's is.

| | before | after |
|---|---|---|
| `issue2948.pdf`, callgrind | **35.47 G** | **3.08 G** |
| `tensor-allflags-withfunction.pdf` | did not finish in a 15-minute callgrind budget | **1.57 G** |
| `personwithdog.pdf` | did not finish in that budget | **6.77 G** |
| a page with no mesh | — | unchanged |

The old cost was per *piece* — 4096 fills of a few pixels, each compiling a `tiny-skia`
pipeline; the new one is per *pixel*, once. **The sixteenth session's lesson has an inverse: a
change made for correctness that is also an order of magnitude faster means the old code was
doing work that was worse than useless.** ADR 0051.

**On the specification track, §12.3's twelve rows — and the ledger's `silent` count goes from
2 to 15.** Document-level navigation: destinations, the outline, thumbnails, collections,
navigators. Not one is implemented and nothing says so.

**That number is the finding.** This project has been able to say "two `silent` rows are left"
only because clause 12's interactive half was `unreviewed` — and `unreviewed` and `silent` are
different admissions: one is *we have not asked*, the other is *we asked, and we owe it in
silence*. `CLAUDE.md` names "outlines, destinations, page labels" in scope explicitly, so this
is debt rather than a boundary, and the rows say what kind each is: a thumbnail panel, a name
tree and a SWF navigator are not one problem.

| | was | is |
|---|---|---|
| a Gouraud triangle | up to 4096 flat pieces, each grown 0.8 px | one raster, interpolated per pixel |
| **pages agreeing with the reference consensus** | 812 | **815** |
| **contradicted pages with no explanation** | 42 | **39** |
| **ledger rows owing something in silence** | 2 | **15** |

**The numbers:**

| | before | now |
|---|---|---|
| corpus documents drawing with nothing reported | 858 | **858** |
| **pages agreeing with the reference consensus** | 812 | **815** |
| **pages contradicted** | 86 | **83** |
| **ledger subclauses nobody has read** | 270 | **257** |
| tests | 603 | **603** |

What it taught:

- **A count of `silent` rows is only as honest as the clauses that have been read.** Two was
  never the number; it was the number of silences inside the parts of the standard this project
  had looked at.
- **"Needs a rasteriser in both backends" was right about the requirement and wrong about the
  difficulty.** The entry had stood for fifteen sessions as a reason not to try. One shared
  raster satisfies "both backends produce identical pixels" more completely than two
  implementations could, and it is *less* code than what it replaced.

### The forty-second session, in brief

**One page in the unexplained list differed from *every* reference while the three of them
agreed with each other, and that is the strongest signal the gate can produce.**
`issue215.pdf` is a masthead: four references draw **OPENMAGAZIN** in small capitals and we
drew **openmagazin** in lower case, 8.5 levels from each of them against their own spread of
0.7 to 1.9.

The file says what it means three times: its `/Differences` name `o.sc` through `n.sc`, its
`/ToUnicode` maps those codes to U+F76F and neighbours — the private-use block Adobe assigns
to small capitals — and its embedded program's `CFF ` charset names all eleven glyphs. **Two
readings of §9.6.5.4 were wrong and each hid the other:**

- A glyph name goes to Unicode "by consulting the Adobe Glyph List and Adobe Glyph List for
  New Fonts". Those are *lists*, and no entry in either contains a FULL STOP.
  `read_fonts::ps::agl::name_to_char` implements the wider Adobe Glyph List *Specification*,
  whose algorithm for an unlisted name strips everything after the first period — so `o.sc`
  answered `o`, a real letter with a real glyph, and the clause's next sentence never ran.
- That sentence would not have helped either. It sends an unmappable name to "the font
  program's `post` table", and this `OTTO`'s `post` is **version 3.0**, which by definition
  holds no names; a CFF-based OpenType keeps them in its charset. §9.6.2.1's NOTE 1 is what
  makes those the same structure — the **third** design question that sentence has settled.

**812 agreeing and 86 contradicted**, from 811 and 87, with nothing traded for a report.
ADR 0050.

**On the specification track, §14.1 to §14.6 — eleven rows, and most of them are
`inapplicable` for a reason the clause states itself.** §14.2's procedure sets "shall be used
only when the content stream is printed to a PostScript language compatible output device";
§14.5's page-piece dictionaries hold private data that "can be ignored by general-purpose PDF
processors"; §14.3's metadata is interchange. Two are not: §14.4's file identifier is
`implemented` because §7.6.4.3.2 takes `/ID[0]` into the encryption key, and §14.6's marked
content stays `partial` — read as a bracket by optional content and by §12.7.4.3's splice, and
not for any tag's meaning.

§14.1's opening sentence is worth carrying: "The features described in this clause do not
affect the final appearance of a document." True of most of clause 14 and **false of the two
parts this tree implements** — output intents change every pixel a CMYK fill covers, and page
boundaries decide what is drawn at all.

| | was | is |
|---|---|---|
| a suffixed glyph name (`o.sc`) | stripped to its base letter by the AGL *algorithm* | looked up in the program's own names, as §9.6.5.4 asks |
| a CFF-based OpenType's glyph names | looked for in a `post` table that has none | read from the `CFF ` charset |
| **pages agreeing with the reference consensus** | 811 | **812** |
| **contradicted pages with no explanation** | 43 | **42** |

**The numbers:**

| | before | now |
|---|---|---|
| corpus documents drawing with nothing reported | 858 | **858** |
| **pages agreeing with the reference consensus** | 811 | **812** |
| **pages contradicted** | 87 | **86** |
| **ledger subclauses nobody has read** | 280 | **270** |
| `§` citations the checker verified | 1419 | **1426** |
| tests | 602 | **603** |

What it taught:

- **The pairwise table is a standing instrument now.** Comparing every reference against every
  other — which the oracle renders for free and was computing for none of the ten pairs —
  found the shared ICC profile two sessions ago and found this page, both times *before* an
  artefact was opened. "We differ from all three while they agree" and "two clusters of two"
  are different diagnoses and one table separates them.
- **A clamp is a decision.** `width.max(0.0)` on the `w` operator reads as defensive hygiene and
is in fact this program's whole answer to a value §8.4.3.2 forbids — chosen once, by nobody, and
visible as a dark line on `issue19633.pdf` where two references draw a faint one. Ask what a
`max`, a `clamp` or an `unwrap_or` *decides* before calling it a guard.

**The ledger can find a missing component, not only a missing feature.** Four rows across two
clauses — §14.7.5.4's `/ParentTree`, §12.3.2.4's named destinations, §12.4.2's page labels and
§12.7.7's named pages — name the same absent data structure, a name or number tree. No single
clause review would have shown that and no corpus document would ever have asked for it.

**An inconsistency inside a reference's own output outranks any distance from it.** Two
renderers spacing one line of one font at two different widths — 20 pt for the letters and 15
for the digits — cannot both be reading the document's `/W`, and seeing that needed no
tolerance, no ratio and no vote, only the ink columns. When a page is contradicted, ask whether
the consensus is *self-consistent* before asking whether it is right.

**Rank a list of suspects by a ratio, not by a distance.** The unexplained contradicted pages
sort usefully by *our nearest reference divided by the references' own nearest pair*: that puts
the pages where we are the outlier at the top and the pages where everyone disagrees at the
bottom. It has chosen the next thing to work on three times in five sessions, each time before
an artefact was opened.

**A font is a file too.** "What does this file already say about itself?" was learned from LZW
stream lengths and from a corpus that encodes one image ninety-six ways. It applies one level
down: a font's table directory states `glyf`'s length, and `loca`'s last entry states it again,
so a font whose `indexToLocFormat` is byte-swapped can be repaired from its own bytes rather
than from another reader's leniency. Look for the second statement before reaching for a
heuristic.

**Price the work before believing a reason not to do it.** `mesh_shading_empty.pdf`'s entry
said, for fifteen sessions, that closing it "needs a Gouraud rasteriser in **both** backends,
since the cross-backend scenes hold them to identical pixels" — true, and read as a reason to
leave it. One shared raster satisfies that constraint *better* than two implementations could,
is less code than what it replaced, and made the page that showed the defect 11.5× faster. A
constraint is not a cost until somebody adds it up.

**A count of what you owe in silence is only as honest as the clauses you have read.** This
project said "two `silent` rows" for eight sessions. Reading §12.3 made it fifteen, and nothing
about the code changed — `unreviewed` had been carrying thirteen admissions that had never been
made. When a status count looks good, check what the `unreviewed` rows would say if they were
read.

**A dependency can implement more of a specification than the clause cites.** The AGL has a
  list and an algorithm, and `read_fonts` gives you both under one function name. Where a
  clause cites a *document*, check which part of it you are getting.

### The forty-first session, in brief

**A page drew mojibake for the project's whole life with `unsupported: []` beside it, and the
clause that fixes it is one analogy away from the one that forbids the file.**
`issue11740_reduced.pdf` shows `Оглавление`; we drew `Î ãëàâëåíèå`, which is the same word's
Windows-1251 bytes in a Latin-1 face. Trap 1's archetype.

Its `/F1` is a `CIDFontType0` whose descriptor embeds **`/FontFile`**, a bare Type 1 program —
which §9.9's Table 124 does not permit there, so the font was substituted for, and a
substituted composite font can only be addressed through `/ToUnicode`. **And this file's
`/ToUnicode` is a faithful record of the wrong thing**: CID 1 → U+00CE, CID 2 → U+00E3, which
are the *bytes* `CE E3` written as Latin-1 code points. Every step after the substitution was
correct.

The clause does not describe a `/FontFile` on a CIDFont and does not have to. §9.7.4.2 describes
the case one analogy away — a CFF whose Top DICT does not use CIDFont operators, where "[t]he
CIDs shall be used directly as GID values" — and §9.6.2.1's NOTE 1 says a CFF *is* "an
alternative, more compact but functionally equivalent representation of a Type 1 font program".
A bare Type 1 program's charstrings are in an order, so the sentence transfers. ADR 0049, and
this is the **second** design question NOTE 1 has settled.

**On the specification track, §9.10 — the family the demand item lands in, which is the best
shape the two-track rule has — and clause 9 is now complete.** §9.10.2 lists three methods for
mapping a code to Unicode "in the priority given", and this page is the witness that the
priority is right for one question and wrong for another: for *extraction* `/ToUnicode` first is
correct even here, because it is the producer's own statement; for *drawing* it was never the
right source and became one only because the program had been declared unusable. §9.10.1 draws
exactly that line.

**Two more of the unexplained contradicted pages turned out to be references that drew
nothing.** `issue11549_reduced.pdf` writes `/FontName /AASGAA+Arial,Unicode MS` — a SPACE is a
delimiter, so `MS` lands where §7.3.7 requires a key. `mupdf` and `ghostscript` discard the
object and render a page that is 255 in every channel; `poppler` and this reader recover and
draw. Two blank pages agree perfectly, and the gate reads that as consensus. The lesson is in
the logs: both said in words that they threw the object away.

**And one `ldd` corrected a load-bearing premise.** `Tolerance::widened_to` said the references'
spread is measured "by implementations that share no code with ours **or with each other**".
`pdftoppm`, `mutool` and `gs` on this machine all link the same `libfreetype.so.6`, while this
tree uses `skrifa` and `tiny-skia` — so on a page whose difference is a letter's edges, the
three are one rasteriser and we are the only second opinion. The comment is corrected and
**nothing else was changed**: the effect is one-sided (their spread understates the floor, so a
derived bound is too tight, and widening only ever loosens, so such a page falls back to the
fixed bounds), and loosening a gate to make contradictions disappear is the move this project
forbids itself.

| | was | is |
|---|---|---|
| a `CIDFont` embedding `/FontFile` | substituted, addressed by a `/ToUnicode` that lies | its charstrings indexed by CID (§9.7.4.2, §9.6.2.1) |
| **contradicted pages with no explanation** | 46 | **43** |
| **clauses with no `unreviewed` row** | 8, 10, 11, 13 | **8, 9, 10, 11, 13** (this file said 7 as well; it was wrong) |
| the three references' independence | asserted | measured with `ldd`, and they share `libfreetype` |

**The numbers:**

| | before | now |
|---|---|---|
| corpus documents drawing with nothing reported | 859 | **858** |
| pages agreeing with the reference consensus | 811 | **811** |
| **pages contradicted** | 88 | **87** |
| **ledger subclauses nobody has read** | 286 | **280** |
| `§` citations the checker verified | 1410 | **1419** |
| tests | 601 | **602** |

The fall of one is `issue5751.pdf`, whose `/FontFile` this tree's Type 1 reader refuses: a
malformed program is now *reported* where it used to be quietly substituted, which is what a
simple font with the same defect already got. It left the contradicted list on the same change.
Trap 5's exchange, in both directions at once.

What it taught:

- **A test that counts glyphs cannot tell a substitute from the real program**, because the
  substitute drew ten glyphs too. What separates them is one number about the *shape*: the
  first glyph's width over its height is 0.94 for the capital О the program contains and 0.34
  for the capital Î the substitute drew. Both readings also have two contours, which is the
  first discriminator this session tried and it passed under both.
- **A metric can measure the background.** A first attempt to sort the unexplained list asked
  whether the difference sits where our own render is *flat* — and on a text page 80% of the
  pixels are white paper, which every renderer agrees about, so it reported "edges only" for a
  page where two references had drawn nothing at all. Caught by opening the artefact, which is
  the thing the metric was meant to save.

### The fortieth session, in brief

**Four of the fifty unexplained contradicted pages were not defects, and the way that was
settled is worth more than the pages.** Comparing every render against every other before
opening any of them showed two clusters: `ours` and `poppler` within 0.6 of a level,
`mupdf`, `ghostscript` and `hayro` within 0.6 of each other, and the groups 3.6 to 10 levels
apart. Two clusters of two is not one page's problem.

All three documents reach `DeviceCMYK`, and `postscript_type4_many_outputs.pdf` is a
controlled experiment somebody else wrote: one axial shading over a 200-pixel page whose
colour is exactly `(t, 0, 0, 0)` for `t` linear across it. Every renderer agrees at both ends
and differs only in between — the signature of an *interpolation*, not of a formula.

**Trap 9 has a third shape, and this tree's own instrument proved it: they share data.**
`/usr/share/ghostscript/iccprofiles/default_cmyk.icc`, evaluated by `pdf_model::icc` — the A2B
evaluator written for `ICCBased` streams, pointed at another program's file — reproduces both
`mupdf`'s and `ghostscript`'s renders to within five levels across the whole ramp. The two
references that outvoted us are **one CMYK profile run twice**, so their agreement is not
evidence about the clause, and its tightness is also what shrank the relative bound until ours
fell outside it (trap 12).

The pages are listed, not fixed: §8.6.4.4 states no destination for `DeviceCMYK`, §10.3.2
licenses a processor to supply a profile, and adopting somebody else's press because it moves
four pages is curve-fitting with a licence attached. ADR 0048.

**On the specification track, §7.5 read as a family — thirteen rows, and clause 7 is now
complete.**

- **§7.5.2's second half was not implemented.** "[B]yte offsets shall be calculated from the
  PERCENT SIGN (25h)", and NOTE 1 permits arbitrary bytes before the header — so a file with
  junk in front of `%PDF-` has a *correct* table whose every offset is short by the junk's
  length, and this reader scanned the whole file for object headers instead. The rule is now
  four `saturating_add`s. One corpus document has junk before its header and says so in its
  own page text.
- **Table 15's `/Size` rule is a departure and now it has a number.** "Any object … whose
  number is greater than this value shall be ignored and defined to be missing by a PDF
  reader": enforced temporarily, the corpus gate's *no page one* count goes from **11 to 77**.
  68 documents write an entry beyond `/Size`. The rule protects nothing here and costs 66
  documents.
- Three rows record a requirement satisfied *by construction*, which is worth as much as one
  implemented: §7.5.7's rule that a string inside an object stream is not separately encrypted
  is true because there is no path to a second decryption; §7.5.6's "most recent copy of each
  object" is one line in `XrefTable::add`; §7.5.8.4's hybrid `/XRefStm` precedence is the order
  of two loops.

| | was | is |
|---|---|---|
| **contradicted pages with no explanation** | 50 | **46** |
| a file with junk before its header | every offset wrong, recovered by scanning | read from its own table (§7.5.2) |
| Table 15's `/Size` | unread, unmentioned | a departure with 11 → 77 measured against it |
| **clauses with no `unreviewed` row** | 8, 10, 11, 13 | 8, 10, 11, 13 — *this row claimed 7 as well and was wrong; §7.10, §7.11 and §7.12 were still unread* |

**The numbers:**

| | before | now |
|---|---|---|
| corpus documents drawing with nothing reported | 859 | **859** |
| pages agreeing with the reference consensus | 811 | **811** |
| **ledger subclauses nobody has read** | 299 | **286** |
| `§` citations the checker verified | 1399 | **1410** |
| tests | 600 | **601** |

What it taught:

- **A dependency can implement more of a specification than the clause cites.** §9.6.5.4 maps a
glyph name to Unicode "by consulting the Adobe Glyph List and Adobe Glyph List for New Fonts" —
two *lists*, neither of which holds a name containing a period. `read_fonts::ps::agl` gives you
the list *and* the Adobe Glyph List Specification's algorithm for unlisted names, which strips
everything after the first period, under one function. So `o.sc` answered `o`, a real letter,
and the clause's own next sentence never ran. Where a clause cites a document, check which part
of it your library is giving you.

**A clause one analogy away is still the clause.** §9.9's Table 124 forbids a `/FontFile` on a
CIDFont, so the tree refused to read one and substituted — and §9.7.4.2 states, of a CFF whose
Top DICT does not use CIDFont operators, that "[t]he CIDs shall be used directly as GID values",
while §9.6.2.1's NOTE 1 says a CFF *is* a Type 1 program in another spelling. Two sentences,
neither about the case in hand, deciding it exactly. When a clause says a construction is not
permitted, that answers whether a *writer* may produce it and not what a reader does with one.

**A discriminating test has to discriminate; check by breaking the thing.** The first assertion
written for that fix counted the contours of the first glyph — two for the capital О the program
contains, and two for the capital Î the substitute drew. It passed under both readings, and
running it against the reverted code is what said so. The number that works is the glyph's width
over its height: 0.94 against 0.34.

**A metric can measure the background.** Sorting the unexplained contradicted pages by whether
the difference sits where our own render is *flat* looked principled and reported "edges only"
for a page where two references had drawn nothing at all — because on a text page 80% of the
pixels are white paper and every renderer agrees about paper. The artefact caught it, which is
what the metric was meant to save.

**Compare the references with each other before opening a page.** Four of the fifty sorted
  themselves into a group from a table of pairwise means, with no artefact opened and no clause
  read. The oracle already renders all five; nothing was computing the other ten distances.
- **Point your own instrument at their data.** "Do mupdf and ghostscript agree because they
  share a profile" reads like a question about two other projects and is a question about one
  file on this machine.

### The thirty-ninth session, in brief

**The correctness oracle was wrong about three blend modes, and now it is not.** ADR 0046 found
`Hue`, `Color` and `Luminosity` 113 of 255 from Vello with §11.3.5.3's closed form saying the CPU
backend was the wrong one, and left the fix as separate work because taking the four non-separable
modes back from `tiny-skia` means `render-cpu` reading the destination. `render-cpu/src/blend.rs`
is that: `Lum`, `ClipColor`, `SetLum`, `SetSat`, Table 135's four functions and §11.3.6's
compositing formula, about 150 lines because the clause states the arithmetic exactly. **All
sixteen modes now agree between the backends to the channel**, and `DISAGREE` is empty.

Three things it cost, and each is worth more than the fix:

- **The corpus cannot see it.** One `eprintln!` over 974 documents: **five uses of a non-separable
  mode, in two documents**, and neither page's raster moves by a byte — verified by rendering
  `issue21570.pdf` before and after and comparing SHA-256s. A clause every other renderer
  implements, wrong here for the project's whole life, and the demand curve's answer is zero.
- **ADR 0046's second piece of evidence was worthless and has been retracted.** It called
  `tiny-skia`'s debug-build "attempt to multiply with overflow" the sharpest evidence the defect
  was an intermediate wrapping. With the three modes no longer reaching the library, the same
  panic still fires — from `lowp::overlay`, and `Overlay` is correct to the channel in release.
  The lanes are *meant* to wrap. What settled the three was the closed form and only the closed
  form.
- **`ClipColor` is undefined where `Lum(C)` is out of range**, which nothing can reach because the
  clause only ever calls it from `SetLum`. A property of the clause's structure, so it is written
  on the function and the test uses only colours `SetLum` can produce.

**On the specification track, §7.2 and §7.3 read as a family — fifteen rows, and two of them
found something.** Both are the shape trap 8 describes: required of every valid PDF, invisible to
974 documents.

- **§7.3.8.1's external stream was a silent gap.** "[T]he bytes may be contained in an external
  file, in which case … any bytes between stream and endstream **shall be ignored** by a PDF
  processor" — and those bytes were being decoded and drawn. `Document::is_external` refuses such
  a stream now, which is the answer principle 3 leaves available: the renderer has no filesystem
  to fetch the file from. No corpus document writes one, measured.
- **§7.3.7's null value was asserted through the one accessor that could not see it.** "A
  dictionary entry whose value is null … shall be treated the same as if the entry does not
  exist" was tested through `Document::get_key`, which answers `Object::Null` for an absent key —
  so the assertion was the implementation restated and passed while `Dictionary::get` returned
  `Some(Null)`, `len` counted the entry and §8.9.7's presence test for an abbreviated key could
  still see it. Three such entries exist in the corpus, in two documents.
- **And the duplicate-key rule was justified by poppler.** "First wins here, matching poppler" is
  principle 5's forbidden sentence in the code. The clause does state something: the entries
  "shall be unordered" and "[t]hat ordering shall be ignored", so *no* rule preferring the first
  or the last can be derived — which makes keeping the first a documented choice, and poppler's
  agreement evidence rather than reason.
- Two smaller ones: `object.rs` called the object model "the eight basic types" for the project's
  whole life, and §7.3.1 says nine — integer and real are two, which is exactly the division
  `as_integer` and `as_number` keep in code. And the tree cited **Table 136** for the
  non-separable blend modes in three places; they are Table 135, and Table 136 is *Variables used
  in the source shape and opacity formulas*.

| | was | is |
|---|---|---|
| **blend modes the two backends disagree about** | `Hue`, `Color`, `Luminosity` | **none, to the channel** |
| §11.3.5.3 | `partial`, with the closed form on the row | `implemented`, in `render-cpu/src/blend.rs` |
| a stream whose data is in an external file | decoded and drawn | refused, §7.3.8.1 |
| a `/Key null` entry | `Some(Null)` from `Dictionary::get` | absent, §7.3.7 |
| **clause 7's unread rows** | 57 | **42** |

**The numbers:**

| | before | now |
|---|---|---|
| corpus documents drawing with nothing reported | 859 | **859** |
| pages agreeing with the reference consensus | 811 | **811** |
| **ledger subclauses nobody has read** | 314 | **299** |
| `§` citations the checker verified | 1365 | **1399** |
| tests | 590 | **600** |

What it taught:

- **Two independent implementations of a clause's arithmetic agreeing to the channel is the
  strongest evidence this project can produce**, and it is why `blend.rs` stays in `render-cpu`
  rather than moving to `pdf-render`. Trap 2 sends a *decision* into the shared crate because a
  decision either backend can make alone is a decision neither has made; §11.3.5.3 is not a
  decision, and hoisting it would make the cross-backend scene compare one implementation against
  itself.
- **A panic in a dependency is not a diagnosis.** ADR 0046's overflow evidence was true, reported
  by the right tool, and about nothing — the same panic fires on a mode that is right.

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
| 31 | Bare Type 1 fonts (`/FontFile`); the oracle's tolerance class asks whether glyphs were drawn; §9.9 reviewed | ADR 0040 |
| 32 | All five bit depths; an inline image's abbreviated keys win; `BX`/`EX`; §7.8 and §7.3.7 reviewed | ADR 0041 |
| 33 | §10.4.2.5 exists; Table 57's `/Font`; the whole of clause 10 reviewed | ADR 0042 |
| 34 | §12.5.6.10's text markup appearances; §7.9 and §14.11 reviewed; `REVIEW_OWED` emptied | ADR 0043 |
| 35 | §8.11.4.4's usage application dictionaries — the ledger's last original `silent` row | ADR 0044 |
| 36 | Vertical writing: §9.2.4's second set of metrics, §9.7.4.3's `/W2` and `/DW2` | ADR 0045 |
| 37 | The blend-mode scene nobody had written; clause 11 completed as a review | ADR 0046 |
| 38 | Clause 8 completed as a review — the graphics clause, 20 rows | ADR 0046 |
| 39 | §11.3.5.3's four modes taken back from `tiny-skia`; §7.2 and §7.3 reviewed | ADR 0047 |
| 40 | Four unexplained pages are one shared ICC profile; §7.5 reviewed, and clause 7 is complete | ADR 0048 |
| 41 | A CID into a bare Type 1 program; §9.10 reviewed, and clause 9 is complete | ADR 0049 |
| 42 | A suffixed glyph name is the program's, not the AGL's; §14.1–§14.6 reviewed | ADR 0050 |
| 43 | One mesh raster instead of 4096 flat triangles; §12.3 reviewed, and `silent` is 15 | ADR 0051 |
| 44 | A font's own tables say which way round its offsets are; §12.1, §12.2 and §12.4 reviewed | ADR 0052 |
| 45 | A contradicted page's label measured and replaced; §12.6's actions reviewed | — |
| 46 | Clause 12 completed as a review; the median page profiled at last | — |
| 47 | A negative line width is a choice, written down; §14.7 reviewed | — |
| 48 | Name and number trees, and §12.4.2's page labels on top of them | ADR 0053 |

The contradicted count has gone 174 → 120 → 108 → 106 → 104 → 108 → 103 → 103 → 104 → 103 → 100
→ 93 → 96 → 96 → 98 → 102 → 102 → 102 → 102 → 102 → 102 → 102 → 101 → 101 → 99 → 88 across
sessions 6 to 31, and the corpus's incomplete count 291 → 368 → 250 → 290 → 283 → 263 → 251 →
235 → 232 → 231 → 231 → 237 → 220 → 220 → 189 → 147 → 137 → 129 → 130 → 130 → 130 → 130 → 130 →
130 → 130 → 110 → 106 → 105 → 97 → 94.
Both move in both directions on purpose: a rise in the first can mean pages *joined* the
comparison, and a rise in the second is honesty when a silence ends. The sections below say
which.

## Where we are

A PDF **renderer** that opens real files and draws pages: geometry, colour, images, shadings,
patterns, embedded text, transparency groups, soft masks, and annotations both from their stored
appearance streams and constructed where the standard states one — on a CPU and a GPU backend,
with JBIG2 and JPEG 2000 decoded in a confined worker, encrypted files decrypted at every
revision and method §7.6 states, **a form field's value laid out from its `/DA` string**, a page's
own label from §12.4.2 shown in the title bar, **§12.3.2's destinations**, which decide the page a
document opens at, **§12.3.3's outline**, which names the section the page being shown is in, and
— since the fifty-seventh session — **§12.5.6.5's links, which a click follows**. It is not yet a
PDF *viewer* in the full sense — nothing edits a field or asks a person for a password — and the gap
is now measured *by clause* as well as by corpus: 193 of the ledger's rows are `silent`, and
almost every one of them is a viewer rather than a renderer.

- **646 tests**, `clippy` clean under `pedantic` + `unwrap_used`/`panic`/`arithmetic_side_effects`,
  `cargo fmt --check` clean, `cargo deny` clean on all four checks — verified by running them, not
  assumed. (The thirteenth session found this line had been *wrong*: eleven warnings had
  accumulated because `allow-panic-in-tests` does not reach an integration test's helper
  functions.)
- **The 14 specification PDFs in `doc/`** — including ISO 32000-2 itself, 1023 pages and 101 318
  objects — all parse, all render page one with **nothing reported at all**, and all extract
  **100% of the words `pdftotext` finds**.
- **The 974-document pdf.js corpus is a gate, not a survey.** All 974 open except ten that are
  encrypted — 8 waiting for a password, 2 by something §7.6 does not specify or we do not
  implement — 953 reach page one, **858 draw with nothing reported**, and everything the other 95
  cannot draw is named. 1501 of 1501 PDF functions parse; all 1793 shadings build, mesh types
  included. The whole gate runs in **~2 s** with no named slow document left. Counts are
  ratcheted.
- **A second gate asks whether what we drew is *right*.** `oracle.rs` compares us against poppler,
  mupdf and ghostscript over **1794 pages** — every corpus page plus page one of each
  specification PDF — in **~26–33 s**, because the references' renders are remembered between runs
  (ADR 0020). Of the 1654 pages we claim to draw completely, **821 agree with the reference
  consensus, 76 are contradicted and 746 are pages the references cannot agree about among
  themselves**. The 76 are named, grouped and ratcheted in both directions. Twenty-five pages
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
  standard has — 1553 of them — every rustdoc blockquote to the standard's own words, and the
  ledger's 823 rows to the standard's subclauses. It prints the title of every table the tree
  cites, which is how the twentieth session found six comments calling Table 57 "Table 58". ADR
  0016, `doc/PLAN.md` §5a.
- Both backends draw everything the display list can express and agree on **every one of the
  sixteen blend modes, to the channel**: **fifteen** headless GPU scenes hold `tiny-skia` and
  Vello to the same pixels at more than one scale and along both axes (see trap 2), plus one
  single-pixel test, `vello_hands_back_straight_alpha`. The fifteenth is the one that found
  something — `cpu_and_gpu_agree_on_every_blend_mode` caught `Hue`, `Color` and `Luminosity`
  113 of 255 apart with §11.3.5.3's closed form saying the CPU backend was wrong (ADR 0046), and
  the thirty-ninth session wrote those four modes in `render-cpu/src/blend.rs` (ADR 0047). The
  list of disagreeing modes is empty and is still a *list*, ratcheted both ways.

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
cargo test -p conformance -- --nocapture   # 1553 citations, 159 quotations, 85 tables, 823 rows
cargo run -p conformance --bin ledger      # regenerates the rows, keeps every status
# Both gates decode images in a separate program, and -p pdf-model does not rebuild another
# package's binaries. Build it first or the numbers below are somebody else's.
cargo build --release -p pdf-sandbox --bins
cargo test --release -p pdf-model --test corpus -- --ignored --nocapture   # 974 docs, ~2 s
cargo test --release -p pdf-model --test oracle -- --ignored --nocapture   # 1794 pages, ~35 s
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
| `pdf-syntax` | Lexer, objects, xref, filters, `Document`, decryption | Touches untrusted bytes first. `crypt.rs` is §7.6's standard security handler — every algorithm the clause numbers, written against its own subclause; `document.rs` is where §7.6.2 decides *what* is decrypted, because that is where an object's identity is known (ADR 0031). `tree.rs` is §7.9.6's name trees and §7.9.7's number trees, one module because the second clause defines itself as the first with integer keys — the component the conformance ledger found by four `silent` rows in two clauses naming it (ADR 0053). `text_string.rs` is §7.9.2.2 and Annex D's Table D.3, which is a code-to-Unicode table and so belongs here rather than beside `pdf-font`'s glyph-name encodings. `object.rs` is §7.3.1's nine basic types plus the reference that labels any of them, and `parser.rs` is where §7.3.7 drops a null-valued entry and keeps the first of a duplicated key. `filter.rs` is §7.4's ten standard filters — four decoded here, one a pass-through for §7.6.6, four image codecs deliberately answered `None` so a *content* stream naming one is visibly unsupported |
| `pdf-model` | Page tree, content interpreter, annotations, optional content, Type 3 fonts, image decode | Where PDF semantics live. `annotation.rs` is selection and placement (§12.5.5) and knows no subtype; `appearance.rs` is where a missing appearance is *constructed* from what its subtype's clause states, where a stored one is *spliced* under `/NeedAppearances`, and where the refusals are argued (ADRs 0030, 0032). `variable_text.rs` is §12.7.4.3 and the one place in the tree that writes a content stream rather than reading one — it knows nothing about annotations or field types, only about a string, a box and a `/DA`. `soft_mask.rs` reads Table 142 and nothing else. `optional_content.rs` answers "is this layer on". `type3.rs` reads a font whose glyphs are content streams. `inline_image.rs` turns `BI` … `EI` into the stream an image `XObject` would have been. `image.rs` owns §8.9.6's and §11.6.5.2's masking, with `combine_on_the_finer_grid` the one place two rasters of different sizes are combined rather than refused; its `Decode` is §8.9.5.2's map held as one table per component and its `Conversion` is an *exact* per-image memo, which is what makes converting every image through its real colour space affordable (ADRs 0034, 0035). `page.rs` is §7.7.3: the tree walk, the four inheritable entries and the twelve that are not, and `/UserUnit` (ADR 0038). `page_label.rs` is §12.4.2 in full over `pdf-syntax`'s number tree — the clause's four traps are no default numbering style, letters that repeat rather than carry, subtractive Roman numerals and a `/St` floor of 1, and its own worked example is the test |
| `pdf-font` | Glyph outlines via `skrifa` | Owns both simple-font encoding algorithms (§9.6.5.2 for name-keyed programs, §9.6.5.4 for `TrueType`, ADR 0015). `name_keyed.rs` is what a name-keyed program offers a code — glyph by name, glyph by built-in code, and that code's name — and `cff.rs` and `type1.rs` each produce one, because §9.6.2.1's NOTE 1 makes them one format's two spellings (ADR 0040). `type1.rs` is §9.9's `/FontFile` and is the one program kept *parsed*, measured: re-parsing per distinct glyph put 11 ms on `tracemonkey.pdf`. `simple_code_table` takes no font descriptor, which is the shape of ADR 0039's finding: Table 112 makes an *embedded* program's own built-in encoding the base, and the Symbolic flag decides only among the cases where nothing is embedded. `DEFAULT_WIDTH` is Table 120's 0 rather than a preference. `code_for` is the one *backwards* route — a character to the code that draws it — and it is built by running the forward mapping over every code the font defines, so the two cannot disagree. `cff.rs` adapts `read-fonts`; `encoding.rs` is Annex D data; `substitute.rs` is the only machine-dependent code in the tree. `cmap.rs` is §9.7's composite encoding, where `Code` carries a value *and* a length because the clause looks a code up "in the character code mappings for codes of that length" (ADR 0029). Deliberately not `tounicode.rs`: same file format, different destination. A Type 3 font is refused here |
| `pdf-render` | Display list + `Rasterizer` trait | No PDF semantics, no rasteriser. Three device decisions live here so the two backends cannot make them differently: `Image::is_smoothed`, `Image::area_averaged` (a departure from §10.7.4, ADR 0025) and `Stroke::device_width` (§8.4.3.2 with §10.7.5, ADR 0028). `soft_mask.rs` turns rendered pixels into §11.5's mask values. `Command::Group` is the one nested command (ADR 0026) and `impose_on_medium` is §11.4.7. `Path::extend_transformed` is the one place geometry moves rather than travelling with a transform (§9.3.6, ADR 0022). `MeshRaster` is §8.7.4.5.5's Gouraud interpolation, evaluated per device pixel and shared by both backends because neither rasteriser has the primitive and a second copy would only drift (ADR 0051). `Transform::max_stretch` is *not* `determinant().abs().sqrt()`: a shear separates the singular values without changing the determinant |
| `render-cpu` | `tiny-skia` backend | Correctness oracle **and** startup path. `blend.rs` is §11.3.5.3's four non-separable modes and §11.3.6's compositing formula, written here rather than in `pdf-render` on purpose: the clause states the arithmetic, Vello states it again in its own shader, and the cross-backend scene compares the two — sharing them would make it compare one implementation against itself (ADR 0047) |
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

**And a scene set is worth what its scenes can *express*.** Fourteen cross-backend scenes
existed and every `Command` in every one of them carried `BlendMode::Normal`, so the two
backends' sixteen blend functions had never been compared at all — and three of them disagree by
113 of 255. The question to ask of a scene set is not "does it pass" but **"what parameter does
every scene in it leave at its default"**. ADR 0046.

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

There are now four places where a report accompanies drawing rather than replacing it, each
deliberate. An `/AcroForm` setting `/NeedAppearances` says its stored appearances may be stale and
we draw them anyway, because they are all the file offers (§12.7.4.3). §11.6.5.2's `/Matte` in a
colour space whose pre-blending cannot be undone after conversion is applied, because refusing it
would draw a rectangle of pure matte colour. A constructed appearance draws what its clause
states while reporting what it does not — a widget's background with its field's value named
(ADR 0030). And §8.11.4.4's `/User` and `/Language` categories leave a layer's state as the
configuration set it and say so, because switching it off would answer a question about this
machine that nobody asked (ADR 0044). Two different true statements; suppressing either loses
information. Do not generalise
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

The other half was written here for thirty-two sessions and was **wrong**. This file said "ISO
32000-2 defines no `DeviceCMYK` conversion at all", on the evidence of §8.6.4.4, which says
"concentrations of process colourants" and stops. **§10.4.2.5 defines one** — and it is the
formula the code's own comment called naive. What the standard does is *rank two answers*:
§10.3's ICC route for an ICC-enabled processor, which this tree is, and §10.4.2's "crude
approximations" for a less-capable one, with §10.3.2 licensing the fallback table itself. The
three sources that outrank the table are the same as before — `/DefaultCMYK` (§8.6.5.6), an
output intent's `/DestOutputProfile` (§14.11.5), an `ICCBased` profile. When you touch that
table, read ADRs 0009 and 0042 and change it as a documented choice. The same shape recurs
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
agreeing about a page is evidence. There are four ways for that to fail, and the second and
third are the common ones.

**A shared gap.** An unimplemented feature almost always falls through to a *default*, so two
unrelated programs that skipped the same clause produce the same picture and the gate reads it as
agreement. `visibility_expressions.pdf` is the case: `mupdf`'s `pdf-layer.c` carries `/* FIXME:
Calculate visibility from array */ return 0;` and `ghostscript`'s `pdf_optcontent.c` prints
`WARNING: OCMD contains VE, which is not supported (ignoring)`, while `poppler` and pdf.js
implement `/VE` and §8.11.2.2 is unambiguous. So the page stays contradicted, listed with the
source citations beside it.

**Shared data.** `mupdf` and `ghostscript` disagree with us about four pages whose colour
reaches `DeviceCMYK`, and agree with each other to under a level — because they are running the
same CMYK ICC profile. What settled it was *this tree's own* A2B evaluator, pointed at
`/usr/share/ghostscript/iccprofiles/default_cmyk.icc`: it reproduces both of their ramps to
within five levels while ours interpolates the sixteen corners. The general move is worth more
than the finding — **when two references agree suspiciously closely, ask what data they are
both reading, and evaluate it yourself.** ADR 0048.

**Shared code, and it is wider than `jbig2dec`.** One `ldd` in the forty-first session:
`pdftoppm`, `mutool` and `gs` on this machine all link the same `libfreetype.so.6`, while this
tree rasterises glyphs with `skrifa` and `tiny-skia`. So on a page whose difference is a
letter's edges the three references are one rasteriser and we are the only second opinion —
recorded on `Reference::independence` and in `Tolerance::widened_to`, whose comment had asserted
they share no code "with each other", and **acted on nowhere**: marking three references
`Shared` for text would leave nothing to vote, when what they share is one component of a page
and everything else about it is still three readings.

`mupdf` and `ghostscript` also both link `jbig2dec`, and on seven corpus pages it
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
| Encryption: a password prompt | 8 | Small | §7.6 is implemented (ADR 0031); what is missing is the *interaction* §7.6.4.1 describes — "the interactive PDF processor should prompt for a password". `Document::open_with_password` takes one and nothing asks for it, so 8 corpus documents are refused at the gate that a viewer with a window would open. This is `viewer-ui` work, not clause work. |
| Encryption: public-key handlers (§7.6.5) | 0 | Medium | Refused by name. Needs CMS enveloped data (RFC 5652), X.509 certificates and access to the user's private keys — a public-key infrastructure and a threat model rather than a cipher. No corpus document uses one. |
| Encryption: `/R` 5, and a non-ASCII revision-4 password | 1 | Small | Two refusals, one of them now cheap to close. Table 21 says `/R` 5 "shall not be used" and states no algorithm for it, so implementing it would mean copying another reader; `issue21579.pdf` writes it anyway. §7.6.4.3.2 step (a) wants a password in `PDFDocEncoding`, which `crypt.rs` refuses outside the range where it and Unicode provably agree — **and `pdf-syntax` now holds Table D.3**, since §12.7.4.3 needed it, so inverting that table would close this refusal outright. Nobody has done it and no corpus document needs it. |
| Annotation icons (§12.5.6.4, .12, .15, .16) | 2 | Small | A `Text`, `Stamp`, `FileAttachment` or `Sound` annotation with no `/AP` displays an icon whose artwork no clause states. Refused and named. Every stamp in the corpus carries an `/AP`, which is what a producer who cares has to do. |
| Predefined `CMap`s (§9.7.5.2) | 12 | Medium | 15 fonts name one of Table 116's registered `CMap` files (`90ms-RKSJ-H`, `UniJIS-UTF16-H`, …), which are not in the tree. Vendoring them is a licensing decision; guessing draws plausible text that says something else. The machinery they would plug into exists. |
| Text: a substitute that cannot be addressed | 42 | Medium | Counting *fonts*: 27 composite fonts with no `/ToUnicode`, so a CID cannot be taken to a character a substitute could draw, and 23 whose substitute draws none of the declared codes. Honest refusals rather than clause gaps; closing them means better substitution. |
| Optional content: the interactive half | — | Medium | §8.11 is honoured wherever it decides what is *drawn*, and since the thirty-fifth session that includes §8.11.4.4's `/AS` usage application dictionaries for the `View` event (ADRs 0017, 0044). Missing: a layer panel and what feeds it — `/Order`, `/ListMode`, `/RBGroups`, `/Locked` and alternate `/Configs` — and the two usage categories that are questions about this processor rather than about the document, `/User` and `/Language`, which are reported. |
| Text knockout (`Tk`, §9.3.8) | 1 | Medium | Table 102's ninth text state parameter, and the only one absent. Its initial value is `true`, which makes a text object a non-isolated knockout group; we composite each glyph separately, which is indistinguishable while glyphs are opaque under Normal. Reported where both of the clause's conditions hold. Implementing it is §11.4.6's knockout groups seen from clause 9. |
| Compositing an object in parts (§11.6.2) | 3 | Medium | "Portions of an object shall not be composited with one another", and `B` paints one object as a `Fill` and a `Stroke`, so the band they share composites twice. Reported where the paint composites and both parts mark the page. The fix is the same as `Tk`'s. |
| Transparency group and mask departures (§11.4, §11.5.3) | 24 | Medium | Three answers a `/Group` may give that are drawn as the isolated, non-knockout group instead, each reported where it can change a pixel (ADR 0026): **knockout** (§11.4.6, 6 documents; for an isolated knockout group the implementation is a Porter-Duff Source composite modulated by coverage and nothing more), **non-isolated with a blend mode inside it** (§11.4.4, 9 documents; without one the two computations are provably identical), and **a blending colour space that is not the device's three components** (§11.6.6, 4 documents, all `/DeviceCMYK`, which means a second raster format). Plus **a soft mask's group with such a space** (§11.5.3, 7 documents). |
| Grid-fitting a stroke's coordinates (`/SA`, §10.7.5) | — | Small | The clause's single-pixel rule is implemented; adjusting "the line width and the coordinates of a stroke … to produce lines of uniform thickness" is a **documented departure**, because the non-uniformity it removes is an artefact of the binary scan conversion §10.7.4 requires and this tree already departs from by anti-aliasing. Nothing reports it: there is no page on which this device could do better. |
| Smoothness tolerance (`/SM`, §10.7.3) | 23 | Small | Read nowhere. This renderer has one fixed internal bound — a 256-sample `Ramp`, and `Triangle::is_subpixel` — where the clause asks for a per-document one, and "each output device may have internal limits" contemplates that. A document asking for a *coarser* shading gets a finer one; one asking for finer than 1/256 of a component is not honoured and nothing says so. That silence hides inside a `partial` row. |
| Image `/Mask` on a filtered image, `/Matte` outside the device spaces | 0 | Small | What is left of §8.9.6 and §11.6.5.2 after ADRs 0023 and 0024, and no corpus document writes any of it. A colour key is a test on the samples a filter delivers, and a `DCTDecode` or `JPXDecode` image has become RGBA before the unpacker sees it — the clause's own NOTE 2 names that pair as the one lossy coding makes unreliable. A `/Mask` stream that is not an image mask is here too, which Table 87 excludes and 1 document writes. So is a `/Matte` on an image whose space is not `DeviceGray` or `DeviceRGB`: §11.6.5.2 requires the pre-blending to be undone *before* colour conversion, and this crate holds one RGBA raster per image, so the inversion is exact only where that conversion was the identity on components. |
| A font selected by `/ExtGState` `/Font` (§8.4.5) | 1 | Small | Table 57's `/Font` is `[font size]` with the font an **indirect reference**, where `Tf` and this crate's font cache are both keyed by a resource *name*. `extgstate.pdf` writes one, and what it decides is which glyphs the page draws, so it is reported rather than passed over. Closing it means a font cache keyed by object identity as well as by name. |
| A degenerate subpath's single device pixel (§8.5.3.3.1) | — | Small | "[A] degenerate subpath … shall be considered to enclose the single device pixel lying under that point" when *filled* — distinct from §8.5.3.2's stroking rule, which is implemented. Neither backend paints it, and the clause calls the result "device-dependent and not generally useful" in the same breath. Recorded in the ledger rather than reported, because a report would name pages on which no reader could tell. |
| Annotation `NoZoom`, `NoRotate`, `/FixedPrint` | — | Small | Table 167 bits 4 and 5, and a watermark's `/FixedPrint`, make an appearance's size or orientation depend on the *view*, which a resolution-independent display list cannot express. Rare. |
| Soft masks and `/Mask` at a grid the bound refuses | 1 | Small | `issue16263.pdf` gives a 2x2 image a 34862x4332 mask — 151 million samples, 604 MB — and that pair is refused and named. The answer the clause describes is compositing at *device* resolution, which means the display list carrying an image and its mask separately. |
| JPEG 2000 at reduced resolution | 1 | Small | `issue19517.pdf` is a 12608x16806 scan whose full decode wants gigabytes for a page drawn at four megapixels. The format's answer is to decode a lower resolution level, which needs the intended scale to reach the decoder. |
| A stream whose data is in an external file (§7.3.8.1) | 0 | Small | Table 5's `/F`, `/FFilter` and `/FDecodeParms`. The clause says the bytes between `stream` and `endstream` "shall be ignored" and the data is in a named file, which the renderer has no filesystem to open (principle 3, ADR 0014). So such a stream is refused by name rather than drawn from the bytes the clause discards — which is what it used to do, silently, for the project's whole life. No corpus document writes one, measured. |
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

**The fourth is the conformance ledger**, and its headline is a count of unasked questions: **125
of 823 subclauses are `unreviewed`** — 107 of clause 14 and 18 of clause 7 — and 698 have been
read against this code. 86 of those carry principle 5's exclusions, all but four of them clause
13, and **131 are `silent`**, which is a fact about this project's shape: it renders pages
correctly and does nothing when a person clicks on one. So the honest summary is that the project
has measured 85% of its clause coverage — up from 37% eighteen sessions ago, and **six clauses
have no `unreviewed` row at all**: 8, 9, 10, 11, 12 and 13's exclusions. **Clause 7 is 120 of
138** and this file claimed it complete for six sessions; see the forty-eighth session. What
remains unread is clause 14's tagged PDF, metadata and web capture, and §7.11 and §7.12.

**The ledger has been wrong twice and this file's arithmetic about it once**, which is worth
knowing before trusting a row or a summary: §8.9.5.3's note
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
| **draws with nothing reported** | **858** | **88%** |
| draws, with something reported | 95 | 10% |

That 87% is the number to quote for *reporting*. It **rose by twenty in the thirty-first
session**, all of them documents that embed a bare Type 1 font program: they had been drawing in
a substitute face and saying nothing, which is the failure mode a fallback creates and no count
can see (ADR 0040). It **fell by one** in the twenty-fourth session,
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

This is the number to worry about. Over all 1794 pages compared, of the 1655 we claim to draw
completely:

| | count | share of the 1620 |
|---|---|---|
| agree with the reference consensus | 821 | 50% |
| **contradicted by it** | **76** | **5%** |
| the references cannot agree among themselves | 747 | 45% |
| not comparable (geometry, or fewer than two renderers) | 10 | 1% |

**One page in twenty that we say we drew completely, two independent implementations say we did
not.** The 82 are named in `oracle.rs` and grouped by what the page carries: 15 use a font nobody
embeds so every renderer substitutes differently, **11 are pages where the references that agree
are not reading the clause differently** — 7 sharing a JBIG2 decoder, 1 sharing a `/VE` gap, 3
link borders where one reference has no such feature and the other is rendering for paper (trap 9
has all three shapes) — 8 are a one-pixel page-rounding difference, 1 an image half a device pixel
tall, 1 a `CalRGB` alternate two references do not convert, 1 a level of mask quantisation on a
flat page (trap 12), 1 a symbolic font whose (3, 0) subtable reaches an empty glyph, **4 a `DeviceCMYK`
conversion where the two agreeing references share one ICC profile** (ADR 0048), 2 where the
agreeing references drew nothing at all, 1 where two references space one line by two
different widths, 1 a line width the clause forbids, and **36 have nothing on them to explain
it**.
That last group is the most valuable list in the repository, and 21 of them are pages beyond the
first, which a page-one comparison would never have seen. **One page left the `substituted fonts`
group in the twenty-eighth session by being fixed** — `issue8092.pdf`, whose difference was a
shading's `/BBox` and had nothing to do with its fonts (ADR 0037).

**The pattern to read this table by**: a feature that makes pages drawable adds them to the set
being judged, so the numerator and the denominator move together and only one of those is news.
The denominator moved in the thirty-first session for the first time in eight, by **20**, and
every one of those pages is a document that embeds a bare Type 1 program and had been drawing in
a substitute face without saying so. Before that: the twenty-eighth fixed one contradicted page
(a shading's `/BBox`), the twenty-ninth moved three out of the geometry bucket into agreement
(`/UserUnit`), and the thirtieth moved two — one a fix (`/MissingWidth`) and one a *bound*,
`issue3566.pdf`, whose raster is byte-identical and which changed tolerance class when its glyph
names became readable. **The thirty-first also moved 25 pages out of `ambiguous`** by making the
tolerance class ask whether glyphs were drawn rather than whether they could be named, and 6 of
those are newly contradicted — pages that were already wrong and could not be said to be.
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
| 7 Syntax | 138 | **120 of 138 rows reviewed**, and this row said *complete* for six sessions while §7.10, §7.11 and §7.12 sat unread — the count was taken over the families a session had touched rather than over the clause. §7.10's functions went in the forty-eighth session and were the cheap review the mistake had hidden: all four types, 1501 of 1501 corpus functions parsing. What is left is §7.11's file specifications and §7.12's extensions dictionary, neither of which this tree reads. — §7.2 and §7.3 as families from the thirty-ninth session, which found §7.3.8.1's external stream being decoded where the clause says its bytes "shall be ignored", and §7.3.7's null-valued entry surviving `Dictionary::get` — the whole of §7.4, §7.6, §7.7 and §7.8 as families. Objects, **every standard filter**, classic and stream xrefs, object streams, incremental updates, recovery by scanning, and **encryption at every revision and method §7.6 states**. What is left as *work* is a public-key handler and a password prompt. §7.5 went in the fortieth session and found two things: §7.5.2's rule that byte offsets are measured from the `%PDF-`, which was not implemented, and Table 15's `/Size`, whose enforcement is a departure costing 66 documents their page tree (measured, ADR 0048). §7.8's content streams and resource dictionaries are read in full, including Table 33's `BX`/`EX` compatibility section, in which an unrecognised operator is ignored without error (ADR 0041). §7.9.2's string object types are read, including Annex D Table D.3's `PDFDocEncoding`. |
| 8 Graphics | 128 | **Complete as a review**, all 128 rows, and the clause with the most ledger coverage. The whole of the graphics state and of path construction and painting, including §8.5.3.2's strokes with no length and §8.5.4's empty clipping path. Paths, clipping, all eleven colour space families, all seven shading types, both pattern types, form and image XObjects, inline images, `/Interpolate`, an image's `/Mask` in both forms, ICC colour management, optional content (§8.11) wherever it decides what is drawn, a form clipped by its `/BBox` (§8.10.1), and §8.6.6.4's `/All` and `/None` colourants. §8.9.5.2's `/Decode` array in full, Table 88's per-space defaults included, and an image's colour space is the one a fill gets — `ICCBased` profiles and §8.6.5.6's default spaces both (ADRs 0034, 0035). **All five of Table 87's bit depths** are unpacked, and §8.9.7's abbreviated keys beat their full names when a file writes both (ADR 0041). |
| 9 Text | 65 | **Complete as a review**, all 65 rows — §9.2, §9.3, §9.4, §9.6, §9.8, §9.9 and the whole of §9.7 as families. Simple and composite fonts through **every font program Table 124 defines** — TrueType, CFF, OpenType and, from the thirty-first session, the bare Type 1 of `/FontFile`; the standard 14 by substitution; `/ToUnicode`; Type 3 fonts; all eight text rendering modes; both simple-font encoding algorithms in full; §9.7's two mappings in full. An embedded program's own built-in encoding is the base encoding Table 112 says it is, and `/MissingWidth` defaults to Table 120's 0 (ADR 0039). Both writing modes, from §9.2.4's two sets of metrics (ADR 0045). A `CIDFont` embedding a bare Type 1 program indexes its charstrings by CID, which §9.7.4.2 states of a non-CID-keyed CFF and §9.6.2.1's NOTE 1 makes the same format (ADR 0049). §9.10's extraction is read: `/ToUnicode` then the Adobe Glyph List, in the priority the clause gives. Missing: Table 116's predefined `CMap`s and the `registry-ordering-UCS2` files §9.10.2's third method needs, text knockout (§9.3.8, reported), and §9.8.3's `/Style` and `/FD`, which are the ledger's two `silent` rows and reach nothing but a substitute's choice. |
| 10 Rendering | 36 | **Complete as a review**, all 36 rows. 19 of them are `inapplicable`, because halftoning and transfer functions describe a marking device and `/TR` is deprecated in PDF 2.0 besides; 1 is `reported`, §10.8.3's separation simulation, which a *document* cannot ask for. **§10.4.2.5 defines the `DeviceCMYK` → RGB conversion this project spent thirty-two sessions saying the standard does not** — and §10.4.2.1 ranks it below §10.3's ICC route, which is the one this tree is on (ADR 0042). Colour management and rendering intents are done. **Flatness is not "inapplicable"**: §10.7.2 makes ignoring it an explicit permission, which is a better answer. §10.7.4 is `partial` with three deliberate departures named — anti-aliasing twice over and area averaging — and §10.7.5 with a fourth. |
| 11 Transparency | 58 | **Complete as a review**, all 58 rows. All sixteen blend modes reach both backends, including §11.6.3's rule for choosing among an array of names — including Table 135's four non-separable ones, which are `render-cpu`'s own arithmetic since the thirty-ninth session — three of them were 113 of 255 wrong while `tiny-skia` computed them, found by writing the cross-backend scene clause 11 had never had (ADRs 0046, 0047). `ca` and `CA` reach a shading as well as a colour; an image's `/SMask` supplies alpha at any resolution with `/Matte` undone; a `/Group` is composited as one object with the page itself an isolated group; a graphics-state `/SMask` is a group evaluated for alpha or luminosity with `/BC` and `/TR`. Left: knockout, a non-isolated group whose elements blend, and a blending space that is not the device's — all reported. **Overprinting (§11.7.4) was six `silent` rows and is not a gap.** `/AIS` is argued in ADR 0027: with one alpha per pixel, shape and opacity multiply to the same number. |
| 12 Interactive features | 166 | **Complete as a review**, all 166 rows — and 113 of them `silent`, which is the honest shape of this project: it renders a page correctly and does nothing when a person clicks on it. **Appearances, constructed ones, and a field's own text** are what is implemented — clause 12's whole navigation half (§12.1 to §12.4) was read in the forty-third and forty-fourth sessions and not one subclause of it is implemented — the whole of §12.5, and the whole of §12.7.4 and §12.7.5 with §12.7 to §12.7.3 above them. An annotation is placed and drawn from `/AP` (§12.5.5) with §12.5.3's flags and §8.11.3.3's `/OC` honoured; one with no `/AP` is constructed from its subtype's clause or refused with the reason named (ADR 0030); and a field's value, caption or free text is laid out from its `/DA` by §12.7.4.3 (ADR 0032). What does not exist is *behaviour*: no actions (§12.7.6), no FDF (§12.7.8), no navigation, no signature validation (§12.8). |
| 13 Multimedia | 81 | **Excluded** by name on principle 5's closed list. Its rows carry that exclusion rather than being omitted, because an invisible exclusion is indistinguishable from an oversight. |
| 14 Document interchange | 152 | **Output intents, and marked content as a bracket**, 15 rows reviewed — §14.1 to §14.6 in the forty-second session, most of them `inapplicable` on the clause's own words: procedure sets are for a PostScript device, page-piece dictionaries hold data the clause says a general processor may ignore, metadata is interchange. §14.4's file identifier is `implemented` because §7.6.4.3.2 takes `/ID[0]` into the encryption key. No tagged PDF, no metadata, no marked-content *semantics* — but §14.6.1's nesting rule is now read twice over: `BDC`/`EMC` maintain the optional-content stack, and §12.7.4.3's splice has to find the `EMC` matching a `/Tx BMC`, which is the same sentence as an algorithm. §14.3.2 is read only as far as Table 21's `/EncryptMetadata` needs. |

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
| Font programs | **All five of Table 124's**: bare Type 1 (`/FontFile`), TrueType, CFF, CFF-in-OpenType and CID-keyed CFF — plus Type 3, whose glyphs are content streams and are run by `pdf-model`. Which reader applies is decided by the program's leading bytes, not by the key or Table 125's `/Subtype`. A CIDFont writing `/FontFile` — which Table 124 does not permit — has its charstrings indexed by the CID, because §9.7.4.2 says that of a non-CID-keyed CFF and §9.6.2.1's NOTE 1 makes the two one format (ADR 0049). |
| Vertical writing (§9.2.4) | Both sets of a glyph's metrics: mode 0's from `/Widths` or `/W`, mode 1's from `/W2` and `/DW2` with Table 122's `[880 -1000]` default and `v`'s horizontal component at half the glyph's width. §9.4.4's three writing-mode-dependent terms follow — the displacement is `ty`, `Th` multiplies `tx` alone, and a `TJ` adjustment moves along the writing direction (ADR 0045). |
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
| Optional content | §8.11 wherever it decides what is drawn: configuration, membership, `/VE`, intent, all three places `/OC` can appear, and §8.11.4.4's `/AS` usage application dictionaries for the `View` event, at a magnification of 1.0 (ADR 0044). Not read: `/Order`, `/ListMode`, `/RBGroups`, and the `/User` and `/Language` categories, which are reported. |

## What to do next

**Two tracks, and the discipline is to take from both in every session.** *Demand-driven* is
everything the corpus and the oracle name — 76 contradicted pages, 30 of them unexplained, and a
feature list sized by how many documents want each item. **The list is nearly empty of clause
work**: nine sessions took `/FontFile`, all five bit depths, text markup appearances, `/AS`
usage dictionaries, vertical writing and Table 57's `/Font` off it, and what is left that any
corpus document names is a licensing decision (predefined `CMap`s, 12), `viewer-ui` work (a
password prompt, 8), substitution quality (24 fonts), and three transparency-group departures
that need a second raster format or a backdrop. *Spec-driven* is what the ledger and
§6.3.2.2's ranking name — and as of the fifty-sixth session **that track has no unread clause left: 0 of 823 subclauses are `unreviewed`, and what remains is 193 `silent` rows and 30 `reported` ones, each naming what it owes**. A project running only the
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

**The ledger's own first work item.** Four `silent` rows named one absent data structure and the
forty-eighth session built it (ADR 0053). Two of the four have closed on top of it — §12.4.2's
page labels and, in the forty-ninth session, §12.3.2.4's named destinations — and **two are
left**: §12.7.7's named pages and §14.7.5.4's `/ParentTree`, each now needing only its own
semantics on top of `pdf_syntax::tree`. That is what a conformance ledger can produce and a
demand curve cannot: a component nobody would have asked for.

**And the next thing downstream of it.** §12.3.2's destinations are read (ADR 0054) and §12.3.3's
outline with them (ADR 0055), so what is left of clause 12's navigation half is the two that need
a *gesture*: §12.5.6.5's link annotations and §12.6.4.2's go-to actions, which are `viewer-ui`
work — a mouse, a hit test against `/Rect`, and a page change this program already performs.

**And a third thing, on neither track: the instrument.** 95% of the oracle's cost was three other
programs answering a question they had already answered, and nobody had looked because 85 seconds
is not obviously wrong. The thirteenth session found the citation checker blind to table numbers,
and one wrong. The tree was also not `clippy` clean while this file said it was. **Whatever this
file asserts about the tooling, run it once before believing it.**

The one-line version of the demand track: **88 pages we claim to draw are contradicted, 46 of
them for no reason visible on the page**, and the largest thing left that any corpus document
names is §9.7.5.2's predefined `CMap`s at 12 — a licensing decision rather than code — followed
by a password prompt at 8, which is `viewer-ui` work. **§12.5.6.10's text markup left this list
in the thirty-fourth session**, by the clause turning out to state the mark after all. **Variable
text has left this list, as encryption did before it**, and what replaced both is not clause work:
eight documents need a password prompt and five write a `/DA` naming a font their own `/DR` does
not define. **A shading's `/BBox`, `/UserUnit` and `/MissingWidth` are the three rendering
items that came back onto it and off it again** in the twenty-eighth, twenty-ninth and
thirtieth sessions, and none was announced by a document: all three were found by reading a
clause family, and each fixed a page the gate had been carrying (ADRs 0037, 0038, 0039). The one-line version of the spec track: **`REVIEW_OWED` is empty**, and **0 of 823 subclauses remain unread**; the debt is now 193 `silent` rows and 30 `reported` ones, each of which names what it owes.

**The corpus has gone quiet, and the nine sessions from the thirtieth to the thirty-eighth are
what that looks like when the two-track rule is followed.** Everything that moved a gate number
in them came from the *specification* track and every one was invisible to the demand curve
until a clause family was read: Table 112's base encoding and Table 120's `/MissingWidth`;
§9.9's `/FontFile`, whose corpus count was recorded as zero while 57 documents embedded one;
all five of Table 87's bit depths; §8.9.7's abbreviated keys; §7.8.2's compatibility section;
§10.4.2.5, which defines a conversion this project had asserted for thirty-two sessions did not
exist; §8.4.5's Table 57 `/Font`; §12.5.6.10's text markup, refused for thirteen sessions on a
reading of the clause that was wrong; §8.11.4.4's usage dictionaries; §9.2.4's second set of
glyph metrics; and — found by reading §11.3.5 rather than by any gate — the fact that no
cross-backend scene had ever selected a blend mode, and that three of them differ.

**A demand curve cannot rank a requirement no file exercises**, and it cannot notice a
requirement a *fallback* hides: `/FontFile` sat at a corpus count of zero because an unreadable
program fell through to substitution, and substitution says nothing.

### 0. The ledger, and the cheapest reviews available

- **`REVIEW_OWED` is empty**, for the first time since it was written. Keep it that way: a
  clause the code cites and nobody has read is the cheapest debt this project can accrue, and
  the list now fails the build the moment one appears. Every clearing of it has produced
  findings the demand item could not have reached — most recently §10.4.2.5.
- **The ledger is complete: no `unreviewed` row anywhere in the standard** (ADR 0061). What replaces "read the next family" is **the 193 `silent` rows**, which say what is owed and where — the fifty-seventh session took
  §12.5.6.5's and §12.6.4.2's off that list by making a click follow a link, and the same shape
  is left for §12.6.4.13's `/SetOCGState` over §8.11, §12.6.4.9's `/Hide` over §12.5.3's flag and
  §12.4.4's page transitions, each of whose machinery is already built; and `FILE_ONLY_EVIDENCE_CEILING`, 58 `implemented` rows whose evidence is a whole test file rather than a test, which is where a false claim can still hide. Every other technical clause — 7, 8, 9, 10, 11,
  12 and 13 — has no `unreviewed` row, and clause 7 became true rather than claimed in the
  forty-ninth session (§7.11 and §7.12; the count is taken by grouping the ledger's `unreviewed`
  rows by leading clause number, never by what a session touched). What remains is **§14.8's
  tagged PDF at 60 rows**, §14.9's accessibility support at 11, §14.10's web capture at 18 and
  §14.13's associated files went in the fiftieth session, §14.12's document parts in the
  fifty-first, §14.9's accessibility support in the fifty-second and §14.10's web capture in the
  fifty-third. **§14.9 is the one to read before starting §14.8**: `CLAUDE.md`'s "as far as
  accessibility needs it" is defined by §14.9 and by nothing else, so it says which of §14.8's
  sixty rows are in scope. `CLAUDE.md` puts
  "tagged PDF as far as accessibility needs it" in scope and nothing in clause 14 on the
  exclusion list, so these are reviews owed rather than a boundary. Record every row, including
  the `inapplicable` ones — a clause read and dismissed is worth as much as one implemented, and
  costs a minute.
- **147 `silent` rows, and almost all of them arrived by *reading* rather than by any change to
  the code** — clause 12's interactive half in three sessions (§12.3, then §12.1/§12.2/§12.4,
  then §12.6's actions), §14.7's nineteen, §7.11 and §7.12's in the forty-ninth, §14.13's in the
  fiftieth. Three
  of §12.6's would change what is *drawn* and each has its mechanism already built:
  `/SetOCGState` over §8.11, `/Hide` over §12.5.3's Hidden flag, and `/Trans` over §12.4.4. The count was two only because clause 12's interactive half was
  `unreviewed`; `unreviewed` and `silent` are different admissions, and the second is the one
  that says we owe something without saying so. §12.3's twelve — destinations, the outline,
  thumbnails, collections, navigators — are all a *viewer* rather than a clause, and
  `CLAUDE.md` names three of them in scope by name. **Two `silent` rows predate that and
  neither is a decision not to build something.** §8.11.4.4's
  usage dictionaries — the row that had been called the last silence since the ninth session —
  closed in the thirty-fifth (ADR 0044), and what is left is §9.8.3's `/Style /Panose` and
  `/FD`: read by nobody, unable to change an *embedded* CIDFont's glyph, and able to change
  which installed face stands in for one that is not. Their debt is substitution quality rather
  than a clause gap, and it is written on the rows. Its method is trap 11's, unchanged: an `eprintln!` naming the
  documents that carry an `/AS` array *and* a group whose `/Usage` would turn it off at the
  resolution we draw, before any condition and long before any code.
- **§11.3.5.3 closed in the thirty-ninth session** (ADR 0047), and the `partial` rows left with
  something hiding in them are fewer than the count suggests. **One silence still hides inside a
  `partial` row** — §10.7.3's `/SM`. Three others were
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

- **Bound a group's buffer to the band its clip admits.** The CPU backend gives every transparency
  group a page-sized pixmap, because a group's elements resolve their clips against the *target*.
  No corpus page pays for it, but a page with hundreds of groups would. Measure before building:
  `callgrind_rasterise` over a group-heavy page, and the sixteenth session's lesson about a
  benchmark that measures nothing applies.
- **Sandbox the interpreter and rasteriser too.** Spike D exists and is exercised; the rest of the
  renderer still runs in the main process, which is the half of principle 3 not yet built. The
  protocol would have to carry a display list rather than an image, which is a real design
  question.
- **The median page has been profiled** (forty-sixth session) and what is left of it is not
  obviously ours: 28.9% of interpretation is `zlib_rs` inflating the page. The next-largest
  items are `show_text` at 6.7% and the lexer at 4.2%, and the one avoidable item found — a
  repeated Adobe Glyph List search — was worth 1.2%. Anyone returning to this should start by
  asking whether the *decompression* can be avoided rather than made faster: a content stream is
  inflated once per interpretation and nothing caches it between the corpus gate's two passes.
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

`CONTRADICTED_UNEXPLAINED` in `oracle.rs`: 30 pages carrying no undrawn annotation, no hidden
optional content and no substituted font, so the difference is in something we believe we
implement. **Read trap 9 before starting**, because an entry may be any of its three shapes, and
checking costs a web search of the other project's source.

**Rank the list before opening anything, by our worst measurement over the bound it is held to
— the largest of mean, worst tile and SSIM.** The fifty-first session did that and the top of
the list was a different *kind* of thing from the rest: `tiling-pattern-large-steps.pdf` at
**25.7×** against a 3.2× runner-up, and it was a rule nobody had implemented rather than a page
needing a careful eye (ADR 0056). The ranking as it stands after that page left:

| | ratio | |
|---|---|---|
| `issue3694_reduced.pdf` page 1 | 1.81 | 12.47% of pixels differing, the largest share on the list |
| `issue7891_bc1.pdf` page 1 | 1.78 | tile 10.76 against 6.04, and `mupdf` and `ghostscript` are the pair that agree |
| the remaining 27 | 1.6 down to 0.22 | mostly text pages against two references that share `FreeType` |

`issue6231_1.pdf` was the 3.17 at the top of this list until the fifty-second session; it was a
whole surface drawn 180 points from where it belonged (ADR 0057). **A worst tile far above its
bound with a small mean is the signature worth chasing**: it is a
region drawn by one implementation and not by another, and on a large page the mean hides it.

The one cause that was identified, measured and live is **closed**: the subdivision lattice
took `mesh_shading_empty.pdf`, `issue2948.pdf` and `issue18816.pdf` with it in the forty-third
session (ADR 0051). Its entry had said, for fifteen sessions, that closing it "needs a Gouraud
rasteriser in **both** backends, since the cross-backend scenes hold them to identical
pixels" — right about the requirement, wrong about the difficulty. One shared raster satisfies
that constraint better than two implementations could and is *less* code than what it replaced.
**Measure an entry before believing its label, including a label written here** — and price the
work before believing a reason not to do it.

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
- **§12.5.6.10's text markup appearances landed in the thirty-fourth session** and are worth
  reading about before the next refusal is written: the clause states the mark, the region, the
  orientation and the colour, and leaves a thickness, where the refusal that stood for thirteen
  sessions said it states nothing (ADR 0043).
- **Colour-managing an image in parallel** is what the twenty-sixth session left behind rather
  than a clause gap. An `ICCBased` image is now converted through its profile (ADR 0035), which
  is work that was not being done, and interpreting `issue19971.pdf`'s 3.4-megapixel photograph
  went from 30 ms to 120 ms. The loop is embarrassingly parallel apart from its memo, one cache
  per row band would keep it exact, and this tree already has rayon. Nobody has tried it, and
  the sixteenth session's lesson about benchmarks that measure nothing applies.
- **Predefined `CMap`s** (12 documents) are a decision about vendoring third-party data and its
  licence, not an algorithm. **Vertical writing** (4) is §9.2.4's `/W2` metrics rather than §9.7.
  **Type 1 fonts landed in the thirty-first session** and were the opposite of small: the entry
  above them said "no corpus page one reaches one", and 57 do.

### 3. Where the time went, and where it still goes

**There is one fair thing to measure against.** Every other renderer here is C, so a timing
difference against `poppler` confounds the language, the allocator and thirty years of tuning.
`hayro` is Rust, forbids unsafe as we do, and rasterises on the CPU single-threaded as we do.
`cargo run --release -p hayro-compare --bin hayro-speed -- <files>` renders page one of each file
with both, alternating, best of N.

| | |
|---|---|
| total, ours | **8.28 s** against `hayro`'s 112.7 s, over 853 complete pages |
| **median page** | **2.12× slower** |
| worst page | 56×, and it is `issue19176.pdf` at 532 µs against 9.5 µs — a 9x11-point page where the absolute numbers are too small to mean anything |

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

**Where interpretation goes on the median page, measured in the forty-sixth session** — the
first time anybody looked, and the guess this file carried was wrong. `callgrind_interpret`
over the specification's own page:

| | share |
|---|---|
| `zlib_rs::inflate` | **28.9%** |
| `Interpreter::show_text` | 6.7% |
| `read_fonts::ps::agl::name_to_char` | 4.3%, now 3.35% |
| `Lexer::next_token` | 4.2% |
| `inflate_table` | 4.2% |

**Nearly a third of interpreting a page is inflating it.** That is `flate2` doing its job and is
the answer for the typical page; the guess in this file had been "parsing, font loading and
per-page setup". The one item that was *ours* and avoidable was the AGL: §9.10.2's second method
searched a four-thousand-entry list per character shown, in a font with at most 256 codes, and a
cache took the whole of interpretation from 2 013.8 M instructions to 1 989.1 M.

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
| draws incompletely | 96 | Counted by each document's *first* report; 20 left it in the thirty-first session when `/FontFile` began to be read, and 4 in the thirty-second when the last three bit depths did |
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
- **The font row was 100 before ADR 0029, then 67, and is 45 documents.** Nothing on it is a
  `CMap` question and nothing on it is a Type 1 program: what is left is fonts with no
  `/ToUnicode` so a substitute cannot be addressed, substitutes that draw none of their declared
  codes, the 15 naming a predefined `CMap`, the 4 asking for vertical writing, and malformed
  programs.
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

| of the 1655 pages we call complete | count | |
|---|---|---|
| agree with the reference consensus | 821 | |
| **contradicted** | **76** | 8 page rounding, 7 a shared JBIG2 decoder, 1 a shared *gap*, 3 a link border two references do not draw for two unrelated reasons, 1 a sub-pixel image, 1 a `CalRGB` alternate, 1 an eight-bit mask value, 1 a symbolic font reaching an empty glyph, 4 a `DeviceCMYK` conversion (ADR 0048), 2 a reference that drew nothing (ADR 0049), 1 a CID width two references space inconsistently, 1 a negative line width, 15 substituted fonts, **30 unexplained** |
| ambiguous | 747 | the references disagree with each other; 372 are two long books set in fonts nobody embedded |
| our page geometry differs | 0 | all three were `/UserUnit`, applied in the twenty-ninth session (ADR 0038) |
| not comparable | 8 | fewer than two references produced an image, or they disagree on the page size |

The 139 incomplete pages are compared and printed too, but cannot fail the gate: a page we already
say we cannot draw is expected to differ. **The gated set was the same 1620 pages for seven sessions and is now
1655**, which is why the six before the thirty-first moved `agrees` and `contradicted` without
moving either denominator: every one of them fixed or clarified a page already in the comparison rather than
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

**Compare the references with each other before opening a page.** Four of the fifty unexplained
contradicted pages sorted themselves into one group from a table of pairwise means — no artefact
opened, no clause read: `ours` and `poppler` within 0.6 of a level, `mupdf`, `ghostscript` and
`hayro` within 0.6 of each other, the groups ten apart. The oracle already renders all five and
nothing was computing the other ten distances. **Two clusters of two is a fact about the
question, not about the page.**

**Point your own instrument at their data.** "Do `mupdf` and `ghostscript` agree because they
share a colour profile" reads like a question about two other projects and is a question about
one file on this machine — and `pdf_model::icc`, written for `ICCBased` streams, answers it in
one run. Before reading somebody's source to explain their output, ask whether you can *evaluate*
what they are reading.

**A panic in a dependency is a symptom, not a diagnosis — especially where its arithmetic is
modular.** ADR 0046 called `tiny-skia`'s debug-build "attempt to multiply with overflow" the
sharpest evidence available that three blend modes were wrong. With those three no longer
reaching the library, the same panic still fires, from a mode that is correct to the channel in
release: the `u16x16` lanes are *meant* to wrap, because that is what the SIMD instruction they
stand in for does, and Rust's checked arithmetic firing inside them says nothing at all. The
closed form settled it, alone. **Being right for the wrong reason is worse than being wrong**,
one ADR after that sentence was written down.

**Where a clause states arithmetic exactly, two independent implementations are worth more than
one shared one.** Trap 2's rule — a device *decision* goes in the crate both backends share —
does not reach §11.3.5.3, whose formulas the standard writes out. `render-cpu/src/blend.rs` and
Vello's shader are two readings of the same sentences, and `cpu_and_gpu_agree_on_every_blend_mode`
compares them to the channel. Hoisting the functions into `pdf-render` would have looked tidier
and would have made that scene compare one implementation against itself. Ask which of the two a
rule is before deciding where it lives.

**A test asserted through the accessor that normalises the thing being tested is not a test.**
§7.3.7's "a dictionary entry whose value is null shall be treated the same as if the entry does
not exist" was checked through `Document::get_key`, which answers `Object::Null` for an absent
key — so both sides of the assertion were the same function, it passed, and `Dictionary::get`
went on returning `Some(Null)` for the project's whole life. The clause's sentence is about the
*dictionary*; assert it about the dictionary.

**A claim that the standard is silent is a claim about the whole standard, and it is
checkable.** This project asserted for thirty-two sessions, in this file, in `CLAUDE.md` and on
the code, that ISO 32000-2 defines no `DeviceCMYK` → RGB conversion. §10.4.2.5 is called
"Conversion from DeviceCMYK to DeviceRGB" and states one. Twice now a recorded silence has been
a clause four subclauses from one the tree cites constantly, and both times in clause 10 — the
first was §10.7.4 and image reduction. Before writing "the specification says nothing about X",
`grep -n '^## '` the conversion in `doc/md/` and read the *titles*; it takes a minute.

**Being right for the wrong reason is worse than being wrong.** The sixteen-corner table is the
better answer — measured: §10.4.2.5's formula moves the oracle from 802 agreeing and 88
contradicted to 800 and 90 — and it was recorded as an invention where no clause existed. A
departure nobody knows is a departure cannot be revisited, and a justification nobody can check
is not one.

**A cheap family review is where the expensive findings are.** Clause 10 was picked because
most of it was expected to be `inapplicable` at a minute a row. Nineteen rows were; one was the
above.

**A corpus document can be a conformance test, and then it outranks every renderer.**
`issue14256.pdf` draws one picture eight ways, says so in its own `/Title`, and comments each
case with what a correct reader should show. Eight images that must agree with *each other*
need no reference, so principle 5 is not in tension — and where the file leaves two readings
open, its own bytes settle which: a stream that only decodes under one of them has voted. Look
at what a corpus file is *for* before filing it under a group.

**An operator that is matched and ignored may still be a rule.** `BX` and `EX` sat in the same
match arm as `MP` and `DP` for thirty-one sessions, and §7.8.2 makes them the one place an
unrecognised operator is *not* an error. "We handle this" and "we skip this" look identical in
a match arm; only reading the clause family separates them.

**A "not implemented" count of zero can mean "nothing reports it", and those are different
facts.** Every count on the not-implemented table is a count of *reports*, and a gap with a
fallback in front of it reports nothing: `/FontFile` was recorded at zero corpus documents while
57 of them embedded one and drew in a substitute face in silence. Before writing a zero, ask what
the code does when the feature is absent. If the answer is "something plausible", the count is
measuring the fallback.

**Fixing an instrument can be worth a feature, and the two can be the same afternoon's work.**
One line — what `has_text` asks — moved 25 pages into the oracle's judged set and showed that one
of them was drawing nothing at all. A gate that cannot judge a page is not neutral about it.

**The clause can tell you two readers are one algorithm.** §9.6.2.1's NOTE 1 calls a CFF "an
alternative, more compact but functionally equivalent representation of a Type 1 font program",
and §9.6.5.2 states one encoding algorithm for both — which is why `cff.rs` and `type1.rs` share
a *type* rather than a copy of the rules. Look for that sentence before writing the second
implementation.

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
  tolerance from our own render's content, so a change that adds glyphs to a page also loosens its
  bound — and can move it from "ambiguous" to "judged". When a page appears in the
  newly-contradicted list, check whether its bound changed before concluding the render got worse.
  Since the thirty-first session the question it asks is `Interpretation::glyphs`, "did glyphs mark
  the page", rather than "did we read any text back" — which had made a page of unnameable CJK a
  vector page and a page of invisible OCR text a text page, both backwards.
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
