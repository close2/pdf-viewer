# ADR 0300 — A header cell's axis, and the grid that decides it

Status: accepted, 2026-08-13. Session 465. Amends §14.8.5.7's, §14.8.5.4.3's, §14.8.5's, §14.8.4.8.3's
and §14.8's ledger rows. Extends ADR 0214's bridge; changes nothing it decided.

## The question

`doc/todo/31` named four things the AccessKit bridge did not carry, and the first of them was a
sentence in `role.rs`:

> | `TH` | `ColumnHeader` | AccessKit splits header cells by axis and §14.8.4.8.3 leaves the axis to
> the `Scope` attribute, which this program does not read here.

So every table header cell in every document reached a screen reader as a *column's* header. The
question is what the standard says the axis is, and whether the difference is one a person would
hear.

## What the standard settles, and it settles all of it

§14.8.4.8.3 says what a `TH` is:

> A table header cell containing content describing one or more rows, columns or rows and columns
> of the table.

Three possibilities, and Table 384's `/Scope` is which — "A name whose value shall be one of the
following: Row, Column" and `Both`. **The entry is optional and the clause still answers for a cell
that omits it**, which is the part worth reading twice:

> If a Scope is not specified for a TH structure element, then the assumed value for the Scope
> shall be determined as follows, taking into account the current value for WritingMode

> if it is in the first row and column, the Scope is assumed to be Both

> otherwise, if it is in the first row, the Scope is assumed to be Column

> otherwise, if it is in the first column, the Scope is assumed to be Row

> otherwise, the Scope is assumed to be Both

That is a `shall` on the reader, not a hint, and it turns the question into a different one: **where
is the cell?**

### The row and the column are the logical ones, and the standard says so

"[T]aking into account the current value for `WritingMode`" invites the reading that §14.8.5.4.2's
layout attribute has to be read first. It does not. §14.8.4.8.3's own NOTE, on the header search
these assumptions feed, says

> This algorithm works for languages with different intrinsic directionality of the script (such as
> right-to-left) because the structure always reflects the logical content order of the table.

So `WritingMode` decides where the first row and column are *drawn*; the structure decides which
they are. A reader with the structure tree can answer without reading a layout attribute at all,
and this one has it.

### And a cell's column is not its position among its row's children

Table 384's `/RowSpan` is

> The number of rows in the enclosing table that shall be spanned by the cell.

so a corner cell two rows tall occupies column 0 of the row below it, and that row's first *child*
is in column 1. A reader that counted children would call it the first column's and assume `Row`
where the clause's last bullet assumes `Both`. **Placing a cell needs a grid, which needs the whole
table, which is why this belongs on the reading side of the boundary and not in a host.**

## What was built

- **`pdf_model::structure`**: `HeaderScope` with Table 384's three names and its four assumptions;
  `CellPlacement`; `Tree::cell_span` and `Tree::header_scope`; and `TableStack`, which is the grid
  discipline as a thing a walk *drives* rather than a second walk. It is fed every element, because
  what closes a table is leaving it, and it is keyed by the walk's own depth — which both consumers
  have already. A `TableGrid` inside it fills a row at a time and tracks how far each column is
  spilled into by a `/RowSpan` above it.
- **`viewer_core::AccessibilityNode::header_scope`**, the answer for a `TH` and `None` for
  everything else. It crosses because a host cannot work it out: the assumption is about the grid,
  and a host has the elements without the spans that placed them. `viewer-confined`'s pipe carries
  it as a discriminant rather than a name, so the confined side cannot invent a fourth value.
- **`viewer_accessibility::role`**: `Row` → `accesskit::Role::RowHeader`, `Column` →
  `ColumnHeader`.

## The three losses, each recorded as a choice

1. **`Both` has no role, in either vocabulary.** AccessKit splits header cells by axis and AT-SPI
   has `ColumnHeader` and `RowHeader` and nothing between them. The node keeps `ColumnHeader` and
   its *description* says the document scopes it to both — chosen over a plain `Cell`, which would
   lose that it is a header at all. That is the more expensive of the two losses; inventing an axis
   is the one this mapping exists to refuse.
2. **A `TH` outside a `TR` has no axis and is not folded into the first case.** §14.8.5.7's
   assumption has nothing to work from, so `TableGrid::place` answers `None`, the scope is `None`,
   and the description says the axis is not known. The same rule the untagged page follows: say
   what is not known rather than the plausible thing.
3. **The cell's coordinates do not cross at all, and the reason is the platform's.**
   `accesskit_atspi_common` implements `Accessible`, `Action`, `Component`, `Hyperlink`,
   `Selection`, `Text` and `Value` — and **not** `org.a11y.atspi.Table` or `TableCell`. A row index,
   a column index and a span set on an AccessKit node would therefore reach AccessKit and stop
   there, which is `doc/habits.md`'s "a capability that reached the crate and never reached the
   program" wearing a different hat. The grid stays inside the reader, where it decides the axis
   that *does* cross.

## What the corpus says, because a mapping nobody exercises proves nothing

`crates/pdf-model/examples/table_header_census.rs`, over 978 documents (the pdf.js corpus and
`doc/`'s specifications):

| | |
|---|---|
| documents with a structure tree | 103 |
| documents stating at least one `Table` | 25, 829 tables |
| `TH` / `TD` | 5965 / 12291 |
| `TH` stating Table 384's `/Scope` | 227 |
| axes, stated or assumed | **Row 3114, Column 1670, Both 1181** |
| `TH` this reader could place in no grid | 0 |
| cells spanning more than one row or column | 154 |
| cells stating `/Headers` | 281 |

**More than half of every header cell in the corpus was being announced as the wrong kind**, and
1181 of the rest were being announced as one kind of a cell the document says is two. ISO 32000-2's
own PDF is the largest witness — 745 tables, 5432 `TH`, not one stating a `/Scope`, so every one of
them is the assumption working or not working.

## How it was verified, and it is the bus rather than a `TreeUpdate`

`doc/verify.md`'s recipe: `dbus-run-session`, `at-spi-bus-launcher`, `at-spi2-registryd`, `Xvfb`,
and a client walking `org.a11y.atspi.Accessible` from the registry root.

- `pdfjs_wikipedia.pdf`, whose ten `TH` each state `/Scope /Row`. **Before: ten `[ColumnHeader]`
  nodes. After: ten `[RowHeader]`.** The same binary, the same page, one field of difference.
- `bug2014080.pdf`, whose eight `TH` state no `/Scope` at all, so every axis on the bus is
  §14.8.5.7's assumption: the `THead` row comes back as one `[ColumnHeader]` carrying the
  description *"this header cell describes both its row and its column (ISO 32000-2 Table 384,
  Scope Both)…"* followed by two plain `[ColumnHeader]`s, and each of the five `TBody` rows begins
  with a `[RowHeader]`. That is the corner cell, the header row and the header column, read off a
  real bus by a real client.

**Two facts about the instrument, because the next round will want them.** The AT-SPI adapter does
not implement `GetRoleName`, so a client asks `GetRole` and gets AT-SPI's integer; the walker used
here reads the names out of `atspi-common`'s own enum in declaration order rather than numbering
them by hand. And the registry needs a `DISPLAY`: without one it prints *AT-SPI: Cannot open default
display*, exits, and every later call fails with `ServiceUnknown`, which looks nothing like the
cause.

## What it costs

Nothing measurable. A/B in one sitting on ISO 32000-2's 129 389-element tree, `Query::
AccessibilityTree` in release, best of five, three runs each: **67–91 ms with the change, 77–89 ms
without**. The addition is inside the instrument's own spread.

**What the measurement found instead is worth more than what it was taken for.** ADR 0228 recorded
this query at 0.13–0.25 ms — on a five-page document. On a thousand-page one it is **eighty
milliseconds**, because `accessibility::nodes` walks the whole document's structure tree and prunes
afterwards, and the walk resolves §14.7.3's role map per element. A screen reader asks this question
on every page turn. Not taken here, and written down in `doc/todo/31`.

## The ledger, and the shape the sweep found

§14.8.5.7 was **`inapplicable`**, on the reason "[n]othing here is drawn; a spanned cell was drawn as
whatever marks the content stream made". That is a *rendering* argument refusing a requirement
addressed to a reader — and §14.8.5.6 next door had already stopped being inapplicable for exactly
that reason, because a `PrintField` attribute is not drawn either and it is what a screen reader
says. Two rows about one family, disagreeing, with the older one's reason naming a capability. It is
`doc/todo/01`'s seventh sweep, and its own generalisation: **when a row's reason is about what this
program is rather than about what the clause says, find the other row.**

Run once more over the same family, it paid a second time. **§14.8.5.4.3 was `inapplicable` on the
same shape and is now `silent`**: ten of Table 379's thirteen attributes describe the layout process
that produced an appearance this reader already has, and `/BBox`, `/Width` and `/Height` do not.
`AccessibilityNode::quads` is built from the text layer, so an element that marks no text — a
figure, a cell holding an image — crosses to an assistive technology with **no place at all**, which
`tree::bounding_box` says in its own words. A description of where the element was laid out is
exactly what a magnifier wants to be pointed at. The population is unmeasured and `doc/todo/31`
carries both halves.

**The test the `inapplicable` rows in §14.8.5 should be put to is not "does it change a pixel".** It
is "does anything read it aloud" — and this program has had something that does since ADR 0214.
