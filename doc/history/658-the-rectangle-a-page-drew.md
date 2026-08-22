# 658 — The rectangle a page drew

**The elements a `/BBox` cannot reach have a place, and the clause that gives it to them was four
paragraphs below a heading nobody had read past.** 2026-08-22. ADR 0486. Parallel round, branched
beside 655, 656 and 657.

## What the clauses say an element's extent is

Two things, with two names, and `doc/todo/31` had only found one of them.

Table 379's `/BBox` (§14.8.5.4.3) is what a **producer** writes: "the rectangle that completely
encloses its visible content", optional, stated by 39 of 1245 corpus documents. ADR 0301 reads it.

§14.8.3.3 — a clause titled *Progression direction*, whose ledger row was `inapplicable` on the
strength of that title — states the other, and it is a `shall`:

> Two enclosing rectangles shall be associated with each BLSE and ILSE (including direct content
> items that are treated implicitly as ILSEs):
>
> - The content rectangle shall be derived from the shape of the enclosed content

§14.8.5.4.5 states the derivation per structure type, and two of its five cases are marks rather
than layout: a table cell's rectangle "is determined from the bounding box of all graphics objects
in the cell's content", and an inline element holding "an illustration or table" the same.

So the union of what a §14.7.5.2 marked-content sequence painted is **the standard's own
construction under its own name**, not a convention this program would be inventing. That is the
argument this round was asked for, and it came out the other way round from the way `doc/todo/31`
posed it.

## The population

`pdf-model --example element_bounds_census`, extended this round. Of 166 724 structure elements in
1245 documents, **2124 mark no text**: 406 state a `/BBox`, 348 are placed by §12.5.2's annotation
rectangle (ADR 0338), **349 by their own marks and by nothing else**, and **1021 by no route at
all** — because their sequences marked nothing whatsoever. `tools/state.sh accessibility` now
counts the third route beside the first two, and 1336 elements that reach a client with no place.

## What was built

`Interpreter::draw` — the one route from the interpreter into the display list now, twenty
`self.list.push` sites — unions each command's own bound, narrowed by its clip chain, into the
sequence enclosing it. `Interpretation::marked` carries a rectangle per sequence;
`AccessibilityNode::drawn` carries the element's union in viewport pixels beside `::bounds`;
`viewer_accessibility::tree::place` asks text quadrilaterals, then the marks, then the producer's
rectangle. **That last ordering reverses half of ADR 0301**, on ADR 0301's own argument.

Not in the display list, and the near-miss is worth the sentence: a *range of command indices* per
sequence would cost nothing and is silently wrong, because `split_off_commands` moves a form
`XObject`'s commands into a `Command::Group` and §14.7.5.2 explicitly permits a sequence to live
inside one.

## What a real client sees

`doc/verify.md`'s AT-SPI recipe, the same binary twice with `tree::place` not consulting `drawn`.
On ISO 32000-2, which states **one** `/BBox` in 1023 pages, four `Figure` nodes that implemented no
`Component` interface at all — the call *errors* — now answer `(208, 75, 110, 50)`,
`(95, 396, 335, 72)`, `(180, 791, 110, 50)`, `(71, 1254, 330, 71)`. On `doc/PDF20_AN001-BPC.pdf` the
logo whose `/BBox` agrees with its marks does not move, and the Creative Commons badge whose
producer wrote `[-32768 -32768 32767 32767]` goes from the whole page to `(434, 658, 5, 5)`.

## What it costs

Callgrind, `Interpreter::draw`'s body short-circuited and rebuilt. **1.15 M instructions per dense
tagged page, 4.7% of interpreting it** (50 × ISO 32000-2 page 101: 1236.7 M against 1294.3 M).
**The launch page pays 0.14%** — page one of the same 1023-page document, 170.69 M against
170.93 M. An untagged page pays one `Vec::is_empty` per command. Nothing was added to the display
list. `Interpreter::clip_extent` is a memo the measurement asked for: the chain walked per command
was 1.46 M rather than 1.15 M.

## Gates

Full sequence, because `pdf-render` and `pdf-model` changed. `fmt`, `clippy --workspace
--all-targets` under `RUSTFLAGS="-D warnings"` and the fuzz `check` all silent · `nextest` **2402
passed, 17 skipped** · doctests · conformance **163 + 5 + 1** · corpus green · oracle **908 agrees,
65 contradicted, 786 ambiguous** — unchanged, so no pixel moved · `render-quorra` **957 pages, 933
agree, 22 differ, 2 refused** · `fixed_documents` **40 checked, 0 absent** · selection census, dates,
XMP, JPEG 2000 · accessibility census: **102 853 elements reached, 7538 placed by `/BBox` or
`/Rect`, 93 267 by their own marks, 1336 with no place at all, and 876 of 876 untagged pages still
answering the honest empty tree with 0 invented.**

That last pair is trap 5's half of this round, measured at corpus scale: a computed extent must not
turn a producer's silence into an answer, and no untagged page gained a node.

## Files

`crates/pdf-render/src/{geom.rs,display_list.rs}` (`Rect::intersection`, `DisplayList::clip_bounds`)
· `crates/pdf-model/src/content.rs` and `content/{marked,report,run,path,image,text,pattern,
transparency,annotations}.rs` · `crates/pdf-model/tests/content_rectangle.rs` (new, 7 tests) ·
`crates/pdf-model/examples/element_bounds_census.rs` · `crates/viewer-core/src/{accessibility,
viewer}.rs` and `tests/{headless,accessibility_census}.rs` · `crates/viewer-accessibility/src/tree.rs`
and `tests/tree.rs` · `crates/viewer-confined/src/protocol.rs` and `protocol/panels.rs` ·
`doc/conformance/ledger.toml` (§14.8.3.3 and §14.8.5.4.5 `inapplicable` → `partial`; §14.8.5.4.3 and
§14.7.5.2 notes) · `doc/todo/31-accessibility-host.md` · `doc/todo/README.md` · `doc/adr/0486-…`.
