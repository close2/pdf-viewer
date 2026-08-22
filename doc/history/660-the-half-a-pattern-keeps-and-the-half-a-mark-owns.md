# 660 — The half a pattern keeps, and the half a mark owns

The round before this one read §11.6.7 and found that a shading pattern's black point, intent and
smoothness are fixed at the beginning of the content stream holding the `scn` — earlier than either
of the two obvious answers. **This round is the sentence four lines further down the same
subclause**, which says the *painting* is the other way round: it "is subject to the values of the
graphics state parameters in effect at the time, just as in painting an object with a constant
colour". So a shading pattern's colours are built again at the mark, and the two clauses that put
something there — §10.5's transfer function and §11.7.2's compositing space — get their answer for
one change.

Date: 2026-08-22.
ADR: [0487](../adr/0487-the-half-a-pattern-keeps-and-the-half-a-mark-owns.md).

Touched: `crates/pdf-model/src/content.rs`, `src/content/pattern.rs`, `src/content/path.rs`,
`src/content/text.rs`, `src/content/image.rs`, `src/content/transparency.rs`,
`crates/pdf-model/tests/transfer_functions.rs`, `tests/rendering_intent.rs`,
`tests/transparency_groups.rs`, `crates/pdf-model/examples/pattern_state_census.rs`,
`doc/conformance/ledger.toml` (§10.5, §11.4.7, §11.6.7, §11.7.5.2), `doc/todo/13`,
`doc/todo/README.md`, the ADR and this file.

## What the price turned out to be, asked before writing anything

`doc/habits.md`'s rule is that a price is a claim that decays, and 655's was "a hundred lines and a
fixture". The cheapest way to re-derive one is to ask what the layers already hold, and five of the
six pieces were already here: `shading::Cache` keyed by object, resolution and conversion (ADR
0069); `content::PatternInitial` scoped per content stream (ADR 0483); `Interpreter::conversion_under`,
which 655 split out *for this caller*; `Interpreter::base`, which has carried §8.7.2's matrix through
the four ways of becoming a parent since the fifty-second session; and `shading::Colouring`, which
made the three colouring inputs one argument (ADR 0479). **The missing piece was the definition
itself** — `pattern()` read the `/Shading` object, the resources and the matrix and dropped all
three at the end of the function.

So the work is one struct that keeps what was being thrown away, one comparison, and moving two
methods off `GraphicsState` — which is where the extra lines went, because `fill_paint` and
`stroke_paint` had no document and no cache. **358 lines added, 173 removed, a net 185**, most of
the addition being the doc comments that carry the two clauses. The estimate was sound; what the
re-derivation bought was the knowledge that the cache makes a rebuild-per-mark a lookup, so the
design needs no memo of its own.

## What was built, and why it is a type

`PatternPaint::Shading` is one `Rc<ShadingPattern>` now — cheaper per `q` than the three fields it
replaces — holding the colours the `scn` built, Table 77's `/BBox`, a `ShadingDefinition` and the
`MarkColouring` those colours were built under. `Interpreter::shading_paint` compares and rebuilds.

**655's warning was that a rebuild reading the graphics state trades one departure for another**,
and this tree's precedent is that a type is worth more than a comment (`HeldContent` in 592,
`Ungrounded` in 628). So `mark_colouring` and `build_shading` take a `&ShadingDefinition` and
**no `&GraphicsState`**: §11.6.7's three parameters have no route in from the mark, and the wrong
version does not compile. §10.5's transfer arrives as the one explicit argument; §11.7.2's target
arrives through `self.compositing`, which is the raster rather than the state.

Two things then fell out rather than being built:

- **`group_press`'s fifth condition came off**, which ADR 0483 said would happen and wrote down as
  temporary. A pattern carried into a press is rebuilt in that press's half like any other colour.
  **The two conditions it had for the same reason were re-read and both stay**: §8.6.8's uncoloured
  cell takes a resolved `Color` from outside and carries no definition to rebuild from, so the
  argument still holds exactly; §11.7.5.3's black generation is a different reason entirely and this
  change does not touch it.
- **`Painted::Shading`'s `stale` flag went**, with the `Unsupported::TransferFunction` §10.5 raised
  through it. There is no stale state left to name.

## The population and what moved

`examples/pattern_state_census` gained the condition this change turns on — a Type 2 pattern plus a
Table 57 `/TR` or `/TR2` stating a real function — and a witness list, one name per line, so a
digest can be run over exactly what a count matched.

- **`doc/pdf.js`**: 964 open, 38 hold a Type 2 pattern (601 of them), 0 state Table 75's
  `/ExtGState`, 0 can see the black point move, 2 hold a four-component group `/CS`, **0 state a real
  transfer function**.
- **The crawl**: 65 703 open, 1504 hold one (36 527 patterns), 42 state an `/ExtGState`, 0 can see
  the black point move, 211 hold a four-component group `/CS`, **11 state a real transfer function**.
  Every figure but the last is 655's, re-derived and identical to the digit.

`examples/raster_digest` on both arms — this tree and `HEAD` — over the 974 corpus first pages and
over the 221 crawled documents the census named: **byte-identical on both**. So nothing on this disk
moves, `doc/todo/00`'s step 7 is not owed, and the argument for the change is the clause. The
instrument's honest limit is that it draws first pages.

## The tests, each run against the defect it guards

- `a_pattern_is_painted_under_the_transfer_function_the_mark_states` reads the ramp both ways round;
  with `shading_paint` made never to rebuild, it fails.
- `a_rebuilt_patterns_black_point_is_still_its_definitions` is the warning's own test. It forces a
  rebuild without changing a colour by any other route — §7.10.3's identity *written as a function*,
  which Table 57 makes a stated function rather than the `/Identity` name that clears one — while
  the pattern's `/ExtGState` says `/UseBlackPtComp /OFF`. Mutating only the rebuild's colouring to
  read a compensating default fails it, and fails 655's two beside it.
- `a_shading_pattern_carried_into_a_press_is_rebuilt_in_the_groups_space` fails with 655's condition
  put back, and asserts one thing more than its predecessor: the same page with the `scn` outside the
  `Do` and inside it draws the same pixel.

## A finding about the instruments

`cargo nextest run -p pdf-model` on its own fails six CCITT tests **on `HEAD`, before this round's
first edit** — black and white exchanged. The same six pass under `cargo nextest run --workspace`,
which is what `doc/todo/02` §2 and CI run. It is Cargo feature unification: the package-scoped build
resolves `hayro-ccitt`'s features differently from the workspace-scoped one. Recorded because a round
narrowing the test command to save time meets six red tests that are not red, and this one lost
twenty minutes to it.

## The errata, confirmed rather than assumed

`cargo run --release -p spec-errata -- emit doc/*.pdf` over all fourteen documents, clauses 10 and 11
first: **no annotation falls in §10.5, §11.6.7, §11.7.2, §11.7.5.2, §11.7.5.3 or §8.7.4.1–§8.7.4.3.**
The nearest are §10.7.2's pair — which amend Table 57's *flatness*, not §10.7.3's smoothness — and
§11.6.6's three on pages 436 and 437, whose texts are Table 145's two deprecations and "of the
transparency group XObject". Since `emit` files an annotation by the page a heading opens, that last
one is the check that matters, and it does not stray into §11.6.7.

## Gates

A fifth round and a change that can move a pixel, so the whole of `doc/todo/02` §2 ran, and §5
rebuilt and installed the binaries. **Three other rounds were building beside this one, load average
between 39 and 58**, and every line came back green, so none needed the quiet re-run 626's rule
reserves for a red one.

`fmt` clean after one reflow. `RUSTFLAGS="-D warnings" cargo clippy --workspace --all-targets` exit
0. `RUSTFLAGS="-D warnings" cargo check --manifest-path fuzz/Cargo.toml --bins` exit 0.
`cargo nextest run --workspace` **2406 passed / 17 skipped**; doctests clean.

Corpus **974 documents, 68 incomplete**. Oracle **1794 pages: 908 agree, 65 contradicted, 786
ambiguous, 2 our geometry, 2 reference geometry, 13 not comparable, 18 no render**. Text extraction
**10969/11163 matched words in bounds (98.26%)** over 508 judged documents, 486 fully in bounds.
`selection_census` 0 panics; `accessibility_census` 57 116 caret elements, 876 of 876 untagged pages
answering the honest empty tree, 0 defects; `dates` **1514 of 1545 conforming (97.99%)**; `xmp` 318
of 319 read; `jpeg2000` green. `render-quorra` **957 pages: 933 agree, 22 differ, 2 refused, 17 not
comparable**. `fixed_documents` **40 checked, 0 absent**. `cargo test -p conformance` green.

Sweeps run because the ledger moved: `quotations` — 1 diverging ledger note, and it is §8.9.5's and
was there before; `counts`, `tables` and `pointers` printed their standing corrections and no new
hit.
