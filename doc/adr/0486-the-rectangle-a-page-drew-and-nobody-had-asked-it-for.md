# ADR 0486 — The rectangle a page drew, and nobody had asked it for

Status: accepted, 2026-08-22. Session 658. Amends §14.8.3.3's, §14.8.5.4.5's, §14.8.5.4.3's and
§14.7.5.2's ledger rows. Extends ADR 0301's `/BBox` route and ADR 0338's annotation route with a
third, and **reverses one precedence ADR 0301 set**; changes nothing ADR 0214, 0325, 0394 or 0425
decided.

## The question

`doc/todo/31`'s largest remaining item, in its own words: an element that marks no text and states
no Table 379 `/BBox` "still has no place", and "[a] bound for those has to come from the *marks*
rather than from the document, which is a different kind of answer and wants an argument before it
wants code".

The argument is the whole of this ADR, and it turned out to be an argument about *whose* answer a
computed rectangle is. That was the wrong question, because the standard had already answered it.

## What the standard states

**Three of the four clauses this round was told to read are not where the answer is**, which is
worth saying first because the numbering moved: §14.7.4.2 in ISO 32000-2 is the *namespace
dictionary*, and marked-content sequences are §14.7.5.2. (Errata Collection 3's Issue #452 moves
that one up a level again; the ledger's §14.7.5.2 row carries the whole tangle and the citations in
this tree stay as the published standard prints them.)

**§14.7.5.2**, on what ties page content to an element, and it is a `shall`:

> The marked-content sequence shall contain a property list (see 14.6.2, "Property lists")
> containing an MCID entry, which shall be an integer marked-content identifier that uniquely
> identifies the marked-content sequence within its content stream

and **§14.7.5.1.1**, which is what makes one accumulator enough:

> A marked-content sequence corresponding to a structure content item shall not have another
> marked-content sequence for a structure content item nested within it though non-structural
> marked-content shall be allowed.

**§14.7.5.4** is how the identifier gets back to its element, and its own NOTE — "[b]ecause
marked-content identifiers serve as indices into an array in the structural parent tree, their
assigned values need to be as small as possible" — is the reason the answer is per page.
`spec-errata emit` on clause 14 says the rest of it out loud, as Issue #308's new NOTE 2
(`Review`/`Completed`), and the sentence 610 found is the one that binds here too:

> MCIDs are scoped by content stream and must start at zero, so the same MCID may reappear across
> pages or XObjects.

**§14.8.5.4.3, Table 379**, the producer's answer and the one this tree already reads:

> An array of four numbers in default user space units that shall give the coordinates of the left,
> bottom, right, and top edges, respectively, of the structure element's bounding box (the rectangle
> that completely encloses its visible content).

And then **§14.8.3.3**, four paragraphs below the progression directions the clause is named for,
where the answer actually is:

> Two enclosing rectangles shall be associated with each BLSE and ILSE (including direct content
> items that are treated implicitly as ILSEs):
>
> - The content rectangle shall be derived from the shape of the enclosed content and defines the
>   bounds used for the layout of any included child elements.

with **§14.8.5.4.5** stating that derivation, by structure type. Two of its five cases are marks
rather than layout:

> For a table cell (structure type TH or TD ), the content rectangle is determined from the bounding
> box of all graphics objects in the cell's content, taking into account any explicit bounding boxes
> (such as the BBox entry in a form XObject).

> For an ILSE that contains an illustration or table, the content rectangle shall be determined from
> the bounding box of all graphics objects in the content, and shall take into account any explicit
> bounding boxes (such as the BBox entry in a form XObject).

## The argument

**The union of a sequence's marks is not "ours" as against Table 379's "theirs". It is a second
thing the standard defines, with its own name, its own `shall` and its own derivation.**
`doc/todo/31` framed the choice as a stated rectangle against a computed one and asked whether a
computed one may stand in silently. The answer is that §14.8.3.3's content rectangle *is* the
computed one, the standard says it "shall be derived from the shape of the enclosed content", and
§14.8.5.4.5 spells the derivation out as the bounding box of all graphics objects in the content —
which is exactly the union, for exactly the elements that need it.

That changes what trap 5's additive-or-substitutive test is being applied to. A derived rectangle is
**additive**: it is a place where the answer was previously an error on `org.a11y.atspi.Component`,
and it takes nothing away from anybody's statement, because Table 379's rectangle still crosses the
boundary as its own field. The two are carried side by side for the same reason
`AccessibilityNode::quads` and `AccessibilityNode::bounds` already are, and the same reason
§14.8.3.3 and §14.8.5.4.3 are two clauses: one is what the page turned out to draw and the other is
what a producer wrote down.

**Where the two disagree, the marks win, and that reverses half of ADR 0301.** That ADR put the
measured text quadrilaterals above the stated rectangle on the argument that "the marks are what is
on the screen" and "the attribute is a claim about a layout this program has already carried out",
and then had the stated rectangle win over *nothing*, because there was nothing else. There is now,
and the argument does not care whether the marks it is talking about are glyphs or a picture. The
order on the bus is therefore: text quadrilaterals, then the content rectangle, then Table 379's
`/BBox`, then §12.5.2's annotation rectangle. `doc/PDF20_AN001-BPC.pdf` is what makes this concrete
rather than tidy — it states `[-32768 -32768 32767 32767]` for a Creative Commons badge, and the
page draws that badge five pixels square.

**An untagged page keeps saying it is one.** Nothing here invents an element: the rectangle is
attached to a §14.7.5.2 sequence, a page with no structure tree has no sequences and no elements,
and `viewer_core::accessibility::nodes` still answers with the one node that says the document
states no structure. And within a tagged page, a sequence that marked nothing gets `None` rather
than a degenerate rectangle — "this element's content marked nothing" stays distinguishable from
"this element has a place".

## Where it lives, and why not in the display list

**In `pdf-model`, beside the text spans, and not in `pdf-render`'s neutral form.** Three reasons,
and the third is the one that decided it:

- A `pdf_render::Command` is a drawing instruction with its graphics state resolved. A
  marked-content identifier is not an instruction and no backend would read one; a field on every
  command would be memory every page pays for so that a screen reader can ask about one in nine.
- `Interpretation::marked` already exists, one entry per sequence, carrying the range of the
  readback it produced. A rectangle beside that range is the same fact about the same thing.
- **A range of display-list indices would have been wrong and would have looked right.** The obvious
  cheap design — record `command_count()` at `BDC` and at `EMC`, and bound the commands in between
  when somebody asks — costs nothing at interpretation time and is defensible until a marked-content
  sequence lives inside a form `XObject` that turns out to be a transparency group, which §14.7.5.2
  explicitly permits. `DisplayList::split_off_commands` then takes those commands out of the list
  and puts them inside a `Command::Group`, and the recorded indices name whatever moved up behind
  them. So the union is taken *as each command is emitted*, which is the only moment the interpreter
  knows both the command and the sequence enclosing it.

`Interpreter::draw` is that moment, and making it the one route from the interpreter into the
display list is most of the change: twenty `self.list.push` sites became `self.draw`.

## Two narrowings, and one refused

**The clip chain narrows it.** §14.8.5.4.3's rectangle encloses "visible" content and §8.5.4 makes
the clipping path "the boundary of the area to be painted", so a fill reaching past its clip marks
nothing out there. `DisplayList::clip_bounds` intersects the chain's path hulls;
`Interpreter::clip_extent` answers it once per clip rather than once per command, because ISO
32000-2's page 6 wraps `q`/`W n`/`Q` around 303 text runs and states one region between them.

**The page boundary does not, here.** ADR 0301 intersects Table 379's rectangle with the crop box
because that rectangle is a *claim* and §14.11.2.1 says how much of a page can be looked at. These
are commands that were actually put in the display list and that a backend already clips to the same
boundary; applying it twice would be this crate second-guessing what it drew.

**And it is a bound, never an underestimate.** `Command::device_bounds` counts a curve by its
control polygon and a stroke by its mitre's reach. For a focus ring that is the right direction to
err in, and it is written down rather than left to be discovered.

## The population, measured before the viewer half was built

`pdf-model --example element_bounds_census`, extended this round with the two counts that decide it,
over the pdf.js corpus, `doc/corpora/` and `doc/`:

```sh
cargo run --release -p pdf-model --example element_bounds_census -- \
  $(find doc/pdf.js/test/pdfs -maxdepth 1 -name '*.pdf') $(find -L doc/corpora -name '*.pdf') doc/*.pdf
```

| | |
|---|---|
| documents read / with a structure tree | 1245 / 153 |
| structure elements | 166 724 |
| elements whose content items produced **no text** | **2124** |
| of those, stating a Table 379 `/BBox` | 406 |
| of the rest, placed by §12.5.2's annotation rectangle (ADR 0338) | 348 |
| of those left, **placed by their own marks** | **349** |
| placed by no route at all | **1021** |
| elements with a content rectangle at all, text or not | 757 |
| by role, of the 349's wider set | `Figure` 600, `P` 74, `Span` 37, `Sect` 14, `Part` 7, `TD` 5, `Link` 5, `TR` 4 |

So the third route is worth about as much as the second, and the residue is still the largest of the
four. **What the residue is matters and is not what `doc/todo/31` assumed**: 1021 elements whose
sequences marked *nothing at all* — 384 `P`, 212 `TD`, 199 `Div`, 86 `Span`, 61 `TR` — a producer
opening and closing a sequence around no operator, or around content a clip excludes. No rectangle
can be derived for those because nothing was drawn, and that is an answer rather than a gap.

**Two things a later round should re-derive rather than trust** (`doc/habits.md`: a price is a claim
that decays). The denominators moved between ADR 0301's run and this one — 1080 documents and 117
tagged then, 1245 and 153 now — because the corpus grew, so *every* ratio in this table is about
this corpus on this day. And the split between the three routes is a property of which producers
this corpus holds: 600 of the 757 are `Figure`s, and a corpus of forms rather than of standards
documents would rank them differently.

## What it costs

The A/B is `Interpreter::draw`'s body short-circuited and the binary rebuilt, under callgrind,
because a stopwatch is the wrong instrument for a change this size (ADR 0312, the hard way).

| | instructions |
|---|---|
| 50 × ISO 32000-2 page 101, without | 1 236 728 467 |
| 50 × the same, with | 1 294 349 882 |
| 50 × the same, with the clip chain walked per command | 1 309 588 269 |
| ISO 32000-2 page 1 once, without / with | 170 693 644 / 170 926 226 |

**1.15 M instructions per dense tagged page, 4.7% of what interpreting that page costs.** Two thirds
of it is the command's own bound and one third was the clip chain before `Interpreter::clip_extent`
memoised it. The walk under both is `Path::hull`, which is computed once per distinct path and kept
on the `Path` itself — so a page repeating one glyph outline three hundred times pays for it once,
and that is why the number is 4.7% rather than several times it.

**The launch path pays 0.14%**, measured on the page `CLAUDE.md`'s startup rules are about: page one
of a 1023-page document, 170.69 M against 170.93 M. **An untagged page pays one `Vec::is_empty` per
command** — `Interpreter::marking` is empty unless a sequence with an `/MCID` is open — and that is
885 of the corpus's 974 documents. Nothing was added to the display list, so nothing changed about
what a page costs to *hold*.

## How it was verified, and it is the bus rather than a test

`doc/verify.md`'s AT-SPI recipe: `dbus-run-session`, `at-spi-bus-launcher`, `at-spi2-registryd` with
a `DISPLAY` of its own, `Xvfb`, `org.a11y.Status IsEnabled` set on the session bus, and a client
walking `org.a11y.atspi.Accessible` from the registry root asking `Component.GetExtents` at every
node. The A/B is the same binary twice, one expression of difference — `tree::place` not consulting
`drawn` — and rebuilt.

`doc/ISO_32000-2_sponsored_EC3.pdf`, which states **one** `/BBox` in 1023 pages:

```text
      [document] 'page Cover-A (1 of 1023)' (0, 0, 500, 708)
          [image] 'PDF Association logo.'          before: —   after: (208, 75, 110, 50)
          [image] 'Adobe, Apryse and Foxit logos.' before: —   after: (95, 396, 335, 72)
      [document] 'page Cover-B (2 of 1023)' (0, 716, 500, 708)
          [image] 'PDF Association logo.'          before: —   after: (180, 791, 110, 50)
          [image] 'Adobe, Apryse and Foxit logos.' before: —   after: (71, 1254, 330, 71)
```

`—` is not a zero rectangle: **the call errors**, because a node with no bounds implements no
`Component` interface at all, which is what "this element has no place" looks like from a client
(ADR 0301 recorded the same shape from the other side).

And `doc/PDF20_AN001-BPC.pdf`, which is where the precedence reversal shows:

```text
          [image] 'PDF Association logo'  before: (35, 35, 105, 48)   after: (35, 35, 105, 48)
            [image] 'Creative Commons'    before: (0, 0, 500, 707)    after: (434, 658, 5, 5)
```

The logo's producer wrote a `/BBox` that agrees with its marks, and nothing moved — which is the
reassuring half. The badge's producer wrote the whole representable plane, and what a magnifier is
now pointed at is the badge.

## What this does not do

- **The residue is 1021 elements and no clause answers for them.** They marked nothing; see above.
- **A sequence in a form `XObject`'s own content stream shares one numbering with the page's.**
  §14.7.5.2 identifies such a sequence by Table 357's `/Stm` and Issue #308 says the identifiers may
  collide, and `Interpretation::marked` is keyed by the identifier alone — so a document using both
  routes on one page could attribute the form's rectangle to the page's element. That is
  pre-existing: the text `range` beside it has had the same key since ADR 0134, and no corpus
  document has been checked for it. `doc/todo/31` carries it now.
- **The allocation rectangle is not derived.** §14.8.5.4.5 makes it the content rectangle adjusted
  by `/SpaceBefore` and `/SpaceAfter`, which describe how an element sits among its neighbours
  rather than where its content is; the three layout-derived content rectangles in the same clause
  are a reflowing processor's for the same reason. §14.8.5.4.5's row is `partial` and says so.
- **An element with text of its own *and* a picture in one sequence still answers with the text
  quadrilaterals.** That is `doc/todo/31`'s standing unmeasured question, unchanged: the sequence's
  content rectangle covers both, but `tree::place` asks the quadrilaterals first and they exist.
  What is new is that the rectangle now exists to compare against, so the question is measurable.

## What it corrected on the way

**Two ledger rows said this clause family cannot reach this program, and both were wrong in the same
way.** §14.8.3.3 was `inapplicable` on "[p]rogression direction — inline and block — for the four
writing modes … nothing here decides where content goes", which describes the clause's title and not
its one `shall`. §14.8.5.4.5 was `inapplicable` on "[t]his is the arithmetic of a layout engine,
stated so that two processors reflowing the same file agree" — true of three of its five cases and
false of the two that are derived from marks a viewer has already made.

That is `CLAUDE.md`'s own rule arriving on schedule: a row saying a clause has no meaning for a
screen is a claim about the specification, and it decays. Both were written when the only question
being asked of clause 14 was what a page *says*; the moment the program had to answer where an
element *is*, a clause about deriving rectangles from marks stopped being a layout engine's.
