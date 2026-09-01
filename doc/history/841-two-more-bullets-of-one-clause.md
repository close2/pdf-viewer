# 841 — Two more bullets of one clause

The round was pointed at `doc/todo/31`'s two named remainders about where an element is: the
elements whose sequences marked nothing, and whether a stated `/BBox` should beat the shapes that
were drawn. Both turned out to be the same defect of reading. §14.8.5.4.5 has five bullets and
ADR 0486 read two of them; the mixed element is bullet five ("finding the extreme top and bottom
for all elements") and the container is bullet two ("the sum of the heights of all BLSEs it
contains"). ADR 0768 is the reading, what it determined, and the one thing it left as a choice.

## What was done

- `pdf-model --example element_bounds_census` extended first, before either half was built, with
  the two counts that decide them — a per-element text extent beside the marks' extent, and the
  question of whether an element no route places encloses one that some route does. Both are
  printed per document as well as in the totals, which is how the witnesses were found.
- `viewer_core::places` is new and is the one place the routes to an element's rectangle are
  composed: what the element's own content drew (its text quadrilaterals **unioned** with its
  marks), then what the document says (`bounds`), then the union of the places of the elements it
  encloses. One reverse pass over the page's answer. It is a function rather than a field on
  `AccessibilityNode`, because a host can compute it and `doc/ui-boundary.md`'s test is what a host
  cannot; and it is one function rather than one per consumer because the precedence had been
  written twice — in `viewer_accessibility::tree::place` and again in the census that prices it.
  Both now ask it, which is most of the diff.
- Five unit tests in `viewer-core` for the three routes and the two orderings, one new test in
  `viewer-accessibility` for a container placed by the widget in the cell below it, and one
  existing test there **rewritten**: it pinned the text quadrilaterals winning over the union,
  which is exactly what bullet five overturns.
- Verified on a real AT-SPI bus, `doc/verify.md`'s recipe, on `annotation-text-widget.pdf`: ten
  container nodes — a panel and nine sections — now answer `Component.GetExtents` with the union of
  the widget rectangles below them, and ten is the number the census counts for that document
  independently. The nodes with a place of their own are unchanged.
- The ledger's §14.8.3.3, §14.8.5.4.5 and §14.8.5.4.3 rows carry the reading, the populations and
  the choice; `doc/todo/31`'s two entries are closed; `doc/state-of-play.md`'s sentence about where
  an element is now names the three routes rather than Table 379 alone.

## Second track

The head of `rank_the_contradicted` is `bitmap-symbol-context-reuse.pdf` page 1, 28.91 bounds from
its nearest reference and 28.91× outside the worst tile. It is diagnosed — every one of the 61
contradicted pages is, and `--bin unpriced`'s one hit (`issue6069.pdf`) is accounted for in its own
note — so the round re-derived it rather than taking it on trust, and tried to move it.

The rule that would move it is the narrowest reading of ADR 0513's abstention: where no reference
drew marks and the flat sheets disagree with each other, none of them is a reading of the page. It
was written and tested, and `pdfref`'s own suite refuted it —
`a_two_of_three_majority_forms_the_consensus` and
`references_disagreeing_among_themselves_is_not_our_failure` are two uniform white rasters against a
uniform black one, which is this page's shape exactly, because a genuinely blank page with one
broken renderer and a page nobody decoded with one broken renderer have the same rasters. The change
was reverted. What would separate them is the renderers' own words — all three logs say *failed to
decode* — and `cache::render` returns on a hit without ever calling the code that captures them, so
the price of having them is a `FORMAT` bump and one re-render of every cached entry (6707 on this
corpus). The group note carries the whole of it, and one fact it did not have: `hayro`'s raster is
byte-identical to ours here because `pdf-sandbox` decodes §7.4.7 through `hayro-jbig2`, so it is our
own decoder rather than a fourth reading.

## Gates

The full §2 sequence, all green: fmt, clippy under `-D warnings`, nextest, doctests, both `fuzz/`
lines, the sandbox and hayro builds, and every gates-profile line — corpus, oracle, the three text
gates, both censuses, dates, xmp, jpeg2000, the quorra gate, `fixed_documents` and
`cargo test -p conformance`. §5's binaries were rebuilt and installed before the bus check, as they
were older than `HEAD`. The oracle's verdicts are unchanged in every class, which is what a change
that moves no pixel should do. The accessibility census's floors all held and its placeless count
fell by the 245 elements the new route places — the two are one run's own arithmetic, since the
route is asked only of an element the old condition counted as placeless. The `quotations`,
`pointers`, `undenominated`, `overtaken` and `unpriced` sweeps ran for the moved documents;
`undenominated` named one sentence of this round's that quantified over a corpus without saying
which, and it was fixed.
