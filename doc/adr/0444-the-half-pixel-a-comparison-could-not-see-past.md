# 0444 — The half pixel a comparison could not see past, and the boundary a drag stopped at

Status: accepted.
Date: 2026-08-20.
Session: 609.
Supersedes nothing; amends ADR 0442 (`Refusal::Rearranged`'s exact comparison) and the shape of
`Answer::Selected` last settled in ADR 0119.

## Context

Two defects of the continuous column, both found by the two rounds before this one and both left
where they were on purpose.

**The first is a comparison.** ADR 0442 made a reprojection defensible in a `OneColumn`
arrangement by asking whether one affine carries every page of the picture held onto the view being
asked for: `stale::one_placement` composes each page's own `settled⁻¹ ∘ asked` and answers only
where they agree. It compared them with `==`, and said why — "a threshold here would be a number
nobody measured a purpose for, which is the mistake this file already records twice at two scales".
Session 608 drove it on a real column and found the argument's cost: the composition goes through
`Transform::invert`, whose division by a determinant is not exact in `f32`, so an **ordinary
scroll** produces placements that print identically to three decimals and are not equal. Its trace
has the pair — `(1.000 0.000 0.000 1.000 0.000 -371.000)` refused against
`(1.000 0.000 0.000 1.000 0.000 -371.000)`. The sharp base was refused for a share of the view
changes of a continuous layout and the window fell back to the blurry retained page with the real
pixels in hand. 608 named the fix and declined to guess it: *a placement wrong by less than half a
device pixel moves no pixel*, and said it needed a measurement of its own.

**The second is a shape.** `doc/todo/30` has carried it since 607: `Answer::Selected` is one page's
range, `viewer.rs`'s `Dragged` arm refused to leave the page the press landed on, and a continuous
column made a drag across a row boundary an ordinary gesture rather than an exotic one. A sweep
from one page onto the next selected the first page's half of a paragraph and stopped, silently.

## Decision 1 — the bound is half a device pixel, derived from the raster and measured after

`stale::AGREEMENT = 0.5`, in device pixels, applied to `stale::disagreement`'s answer.

**The geometry does the work, and it is exact rather than bounded.** Both placements map the same
texels — the base is one texture of the window — so their difference is affine: a texel at `p` is
put at `first(p)` by one and `then(p)` by the other, and `|then(p) − first(p)|` is a convex
function of `p`. A convex function on a convex polygon attains its maximum at a vertex, so the four
corners of the picture answer the question outright. There is no norm to estimate and nothing to be
conservative about.

**Half a pixel is the raster's own quantisation.** A device pixel is a sample of unit spacing and
half of one is the largest distance a point can move without leaving the pixel it is a sample of, so
two placements disagreeing by less than that put every feature of the picture in the same pixel.
It is the argument `TargetSpec::for_page` already rounds a page's extent by (ADR 0064) and it is in
the display's own unit, which is the discipline ADR 0384 re-grounded rules 4 and 5 on.

**What it claims is that and no more**, and the doc comment says so: not that the two resamples are
bit-identical — a filtered sample is a continuous function of the placement — but that the choice
between them moves nothing into a different pixel, on a picture that is a deliberate approximation
already.

**Then the measurement, in that order.** `doc/traps/pixels-and-rasterisers.md` trap 12 is about a
bound tightened until a comparison passes; the defence against it is that the number came from the
grid and the measurement was taken afterwards to see where the populations actually lie.

Over `doc/PDF20_AN001-BPC.pdf` in `OneColumn` in an 800×1000 window, through fifteen scrolls of
137 px and five zoom steps, with the arrangement taken from `viewer_core::layout` rather than
composed by the test:

| view change | of a picture of | disagreement | against the bound |
|---|---|---|---|
| 15 scrolls | 2–3 pages | 0 – 0.000183 px | 2 700× under it |
| 5 zoom steps | 2–3 pages | 1.25 – 2.75 px | 2.5× to 5.5× over it |

Twelve of the fifteen scrolls disagree by exactly zero and three by 0.000183 px, which is the
defect: under `==` those three were refused. The two populations are four orders of magnitude
apart and the bound sits between them touching neither. **The separation is the finding rather than
the threshold** — the residual is `f32` round-off in an inverse, and a zoom's disagreement is
`GAP × (1 − k)` per gap crossed, a real rearrangement in units of a gap `viewer_core::layout`
states in logical pixels and deliberately does not scale. A later measurement finding a residual
near this bound would be a defect in the composition to fix, not a reason to raise it.

**On the screen**, `doc/ISO_32000-2_sponsored_EC3.pdf` under `Xvfb` with `--trace=frames`, in a
four-page column: scrolls reproject, `one placement for 4 pages: they disagree by 0.0000 px`
through `0.0001 px`; Ctrl + wheel zooms are refused, `1.2501` to `5.3749 px apart at the worst
corner`; and switching the layout — which genuinely rearranges — is refused at `877.9067 px`.

**Three things deliberately did not change.**

- **`depicts` still compares exactly.** It asks whether the picture held *is* of the view being
  asked for, and both sides of that comparison are the same computation of the same arrangement
  from the same inputs — not a composition through an inverse. There is no residual to absorb and a
  tolerance there would make a view change look like a still window.
- **`Proxies::placements` needs no bound.** The retained pages are one texture *per page*, so
  nothing is compared with anything.
- **The refusal keeps its name and its kind.** A zoom in a column is still *impossible* rather than
  *unwise*: it is a statement about the arrangement, not a judgement between two measurements.

**And the number is now visible from outside.** `one_placement` returns `Carried { placement,
within }`, `Refusal::Rearranged` carries `by`, and the frame line prints both — so a run says what
the residual is instead of leaving the bound a claim, and the next round to touch this can
re-measure without instrumenting anything. Silent for a single page, which has nothing to disagree
with.

## Decision 2 — a selection is two `(page, offset)` ends, and the variant changes shape

`viewer_core::open::Chosen` is `{ from: Spot, to: Spot }` with `Spot { page, offset }`, and
`Query::Selection` answers `Selected { text: Cow<'a, str>, quads }`.

**This is a variant changing shape, not a message being added**, and the precedents decide it:
§12.4.4.2's `Command::Present` is a message a clause asked for, while 596's `Event::Extracted` and
606's `Answer::Frame` changed a variant's shape because the host already had half the answer. Here
the host has *exactly* half: it is handed text and quadrilaterals and it draws and copies them
without knowing which pages they came from. What it was handed was one page's worth. Nothing a host
needs is missing, so nothing a host asks is new.

**The standard is why both ends carry a page.** ISO 32000-2 §9.4.1 says of the text matrix, the
text line matrix and the text rendering matrix that they "may be specified only within a text object
and shall not persist from one text object to the next" — so text has no position that persists
even between two text objects of one page, and §12.4.2's page indices are the only sequence there
is. `doc/todo/30`'s framing was right that there is no document-wide offset and wrong about what
followed: a *pair* composes across a boundary where a single number could not exist.

**`text` is a `Cow` and the variant is the fact rather than an optimisation.** A selection inside
one page *is* a slice of that page's readback — the identity `selection_census` asserts of
`Selection::All` against `pdf_model::Interpretation::text`, and the one `pdf-retrieve`'s default
answer is held to (ADR 0257) — and a slice cannot be anything else. A selection crossing a boundary
is bytes from two readbacks and is one string in neither, so it is assembled, and only that case
allocates. The hot path a drag runs sixty times a second is untouched.

**Where the pages join is a documented choice**: a newline. No clause states anything about it, the
break is a character in neither page's readback, and a paragraph ending at the foot of a page reads
back as a line ending there.

**`Selection::All` stays one page**, deliberately. It is what two instruments rest on, and a command
that quietly selected several pages would put that newline — which no page states — into the string
those instruments compare byte for byte.

**§12.5.6.10's mark-up over such a selection is several annotations, and the standard requires it
rather than permitting it.** §12.5.2: "A given annotation dictionary shall be referenced from the
Annots array of only one page", and Table 182 states the quadrilaterals "in default user space",
which is a page's. So `Done::Markup` carries a list of `(page, quadrilaterals)` — one log entry, so
that one drag undoes in one, and one annotation per page, because two pages cannot share one.

**§14.8.2.5's logical order joins per page**, by the same argument: the clause is about "the
sequencing of graphics objects within a page's content stream" and states no order between two
pages, so each page's range goes through that page's own filtered traversal and the answers are
joined in page order. Its refusal keeps its shape and reaches further — a page whose tree does not
cover its part of the range takes the whole answer with it, because a copy missing one page of a
selection is the same silent loss as one missing a run.

**A selection lives while *every* page it covers is on the screen.** The old rule was "until its
page leaves"; the honest extension is the conjunction, not the disjunction. A selection whose middle
this crate no longer holds a readback for could answer with text that has a hole in it, and a hole
nothing announces is what principle 1 is about.

## Consequences

**Five consumers had to say what they do, and one had a choice to make.** `viewer-core`'s census,
`viewer-confined`'s wire, `viewer-ffi`'s `pdfv_selection_text`, `viewer-ui`'s copy and
`viewer-confined`'s round-trip fixture all failed to compile. The FFI and the copy path both
answered with `into_owned()`, which copies only in the case that has something to copy; the wire
takes `&selected.text`, which is the same bytes it always took. **No host needed a line about
pages**, which is the claim `doc/ui-boundary.md` makes about this vocabulary tested rather than
repeated — and `PDFV_EVENT_KIND_COUNT` did not move, because no event was added.

**All three hosts have decision 2 and one host has decision 1**, and the asymmetry is not a
deferral: the reprojection exists only in the tier-2 host, because `viewer-gtk` and `viewer-qt` are
tier 1 and are handed a raster per page. That is `doc/todo/37`'s standing item about the processor's
window and the native hosts, not something this round left half-done. Decision 2 reaches all three
by construction — each asks `Query::Selection` per repaint and draws the quadrilaterals it is
handed, in the viewport's own device pixels, so a selection over two pages draws over both without a
host changing a line.

**What was measured after both.** The drag census reads 1000/1011 words (98.91%) over 453
documents, unmoved; `Selection::All` differs from the interpreter's readback on 0 of 966 documents;
the caret inverts on all 2094 offsets; the word-box geometry gate reads 10969/11163 (98.26%) with
486 of 508 documents fully in bounds; the accessibility census's ratchets hold.

## What is left

`doc/todo/30`'s second column item — `Query::Reports`, `Query::Readback` and
`Query::AccessibilityTree` answering for the current page alone — is untouched and is a different
shape of question. `doc/todo/37`'s two standing items are unchanged: the processor's window has no
stand-in of any kind, and a `SinglePage` page turn cannot use a retained page because the identity
is the `Arc`'s.
