# 1724 — The `Doc` members a view state answers, and the window's own view refused by name

Status: accepted and **built**. Session 1444. Builds on ADR 1689's tail ranking and ADR 1700's rule
that a script writes what a reader's edit reaches; RFC 0008 section 4.2's `Doc` and `app` rows.
Code: `crates/pdf-script/src/engine/pages.rs` (the members), `crates/pdf-script/src/surface.rs`
(`REFUSED`; `NOT_BRIDGED` from 43 names to 21, eight of them ADR 1725's), `crates/pdf-script/src/engine/bridge.rs` (`Refused`),
`crates/pdf-model/src/view/script_model.rs` (`PageState`, `MAX_PAGES`, `page_states`,
`ScriptEdit::Destination` and `Calculation`), `crates/pdf-model/src/view/scripts.rs`
(`go_to_named`, the `/CO` walk's gate), `crates/pdf-script/src/wire.rs` (version 10, with ADR 1725).
Tests: `crates/pdf-script/tests/pages_and_choices.rs` (one test per member family),
`crates/pdf-script/tests/fixtures.rs` (`every_refused_member_throws_not_allowed_error_by_name`, now
over `REFUSED` too), `pages::tests`.

## 1. What the census says first

`examples/javascript_census.rs` over the 90 763 files (929 scripted), with a measurement patch
applied and reversed that counts every `.name` a script spells and the host-rooted chains ending in
it: `this.calculate` 5 documents, `this.zoomType` 3, `this.gotoNamedDest` 3, `app.activeDocs` 3,
`this.zoom` 2, `this.layout` 1, `this.documentFileName` 1; `getPageLabel`, `getPageBox`,
`getPageRotation`, `scroll`, `title`, `goBack`, `goForward` none. `this.pageNum` is 34, the control.

## 2. Bridged, each from the entry the standard states

Adobe's *JavaScript for Acrobat API Reference* at `adobe/dc-acrobat-sdk-docs` commit `ab3b42a7`,
"Doc methods", "Doc properties", "app properties", is cited and never quoted; each answer below is a
documented choice under principle 5.

- **The pages are told once with the document**: every page's §12.4.2 label, Table 31's five
  boundaries after §14.11.2.1's defaults and intersection, and the inherited `/Rotate`, at most
  32 768 (`MAX_PAGES`, half the wire's count). A page past it is refused naming the bound.
- **`getPageLabel(nPage)`** answers the label, or the page's number from one where `/PageLabels`
  labels it not — the number a viewer shows; `page_label.rs` leaves that answer to a viewer.
- **`getPageRotation(nPage)`** answers `/Rotate`. **`getPageBox(cBox, nPage)`** answers `Art`,
  `Bleed`, `Crop` (the default), `Trim` and `Media` (the reference's own example passes it) in the
  reference's upper-left, lower-right order, in "rotated user space", which the reference names and
  never defines: default user space turned clockwise by `/Rotate`, Table 31's direction, and moved so
  the turned media box's lower-left corner is the media box's own. `BBox`, the bounds of the page's
  marks, is no Table 31 boundary and the realm is not told the marks: refused by name. A page number
  that names no page is a `RangeError`.
- **`gotoNamedDest(cName)`** is `ScriptEdit::Destination`: the view state reads the name as
  §12.3.2.4 does (`Destination::read` over the catalog's `/Dests` and the name tree) and holds its
  page as the turn a host carries out for `this.pageNum` (ADR 1640). The destination's own view —
  Table 151's position and magnification — is not carried, and the run's report says so, because the
  host's request for a script's turn is a page; a name the document does not define turns no page
  and is reported.
- **`title`** reads Table 349's `/Title` as `this.info.Title` does, which the reference says
  supersedes it; `undefined` where none; a write is refused as `info`'s is (ADR 1626).
- **`calculate`** is read back for the realm's lifetime and carried as `ScriptEdit::Calculation`;
  while it is false the view state walks no part of Table 224's `/CO`, `calculateNow` included,
  since the reference says no calculation is performed for the document.
- **`app.activeDocs`** is an array of this document alone: RFC 0008 section 4.2 admits it "this
  document only" and section 4.3 excludes the rest. The brief named it Tier 2; the RFC's two tables
  say otherwise, and the RFC is what A193 accepted.

## 3. Refused by name, with the reason (`surface::REFUSED`)

A third list beside Tier 2 and the not-yet-bridged: Tier 1 members this program keeps out, each
throwing `NotAllowedError` with `RefusalKind::Unreachable` and the row's reason.

- `zoom`, `zoomType`, `scroll`, `layout`: the window's magnification, scroll position and page
  layout. The brief's premise that "the edit log already carries" them did not hold: a script's
  edits reach a host as a page index, a focus, a timer and a sound, and `view::Request` — whose
  `Display` carries a destination's magnification — is the action path, which no script edit
  reaches. A read would need the host's view told to the realm; a write, a host request no host
  takes. Both are a host round's, and moving a row out of `REFUSED` is that round's argument.
- `documentFileName`: a file name on the reader's machine, which the confined worker has no business
  knowing — a departure from RFC 0008 section 4.2, which admits it. One document uses it.
- `app.goBack`, `app.goForward`: the window's history of views, which a host keeps.

## 4. The tail now

`NOT_BRIDGED` holds 21 names: `event`'s five, `Field.lineWidth`, `textSize`, `textFont`,
`getPageNumWords`/`getPageNthWord` (5 documents each, this document's own text — the next `Doc`
work), `util`'s seven and `console`'s three, `OCG.getIntent`. In documents: the word pair 5, `event.shift`
3, `lineWidth` 3, `console.show` 2, `event.modifier` 2, `textSize` 2, `util.scand` 1,
`console.clear` 1; the rest none.
