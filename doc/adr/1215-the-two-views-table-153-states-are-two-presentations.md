# ADR 1215 — The two views Table 153 states are two presentations, and both are drawn

Status: accepted, 2026-09-22. Session 1189, the batch's host-UI round.

Table 153's `/View T` is drawn in all three windows beside `/View D`, and §12.3.6's `Tree`
selection rule gains a fourth layout to select from. What the panel used to say about tile mode —
`viewer_host::panel::unsupported_presentation`'s sentence — is gone, because a report is not
obedience and the surface it reported on now exists. Supersedes nothing; it closes what ADR 0711
and ADR 1168 left in `doc/todo/65` bucket 1.

## 1. What the clause actually asks for

Table 153 states each `/View` value as its own `shall`, and the two that describe a list of files
differ in two stated ways and no others:

> D The collection view shall be presented in details mode, with all information in the Schema
> dictionary presented in a multi- column format. This mode provides the most information to the
> user.

> T The collection view shall be presented in tile mode, with each file in the collection denoted
> by a small icon and a subset of information from the Schema dictionary. This mode provides
> top-level information about the file attachments to the user.

So: **all of the schema against a subset of it**, and **an icon**. Everything else a reader might
imagine about a tile — a grid, a card, a size — is not in the clause. The earlier reading called
`T` "a tile grid, a strip of miniatures, a free canvas" and put it beside `FilmStrip`, `FreeForm`
and `Linear` as a *surface this panel is not*; that was a reading of the word *tile* rather than of
the entry, and it is what kept a `shall` reported for eleven sessions. The three named layouts stay
out, because §12.3.6 does describe surfaces for them — "a strip of thumbnails", "a large size
preview", thumbnails "at a random location on the view".

## 2. What was built

`viewer_host::panel::presentation` answers Table 153's `/View` through §12.3.6's navigator, exactly
as `unsupported_presentation` already asked it, and the two modes differ in the two stated ways:

- **The cells.** `PanelRow::cells` is one `Cell` per visible schema field, in Table 155's `/O`
  order, heading and value — **one cell per column whether or not the file has a value for it**,
  because a column format wants the same headings in the same places on every row. The details
  view takes all of them; the tile view takes the head of the same order.
- **The icon.** `PanelRow::icon` is `Some` in tile mode only.

`PanelRow::detail` is derived from the cells rather than built beside them, so a toolkit with
columns and one without cannot disagree about what a file's fields say.

Each window draws them with what it has: GTK a `GtkGrid` of heading-over-value pairs and a
`GtkImage` from the icon theme; Qt a model whose `columnCount` is the schema's, with `headerData`
carrying Table 155's `/N` and `Qt::DecorationRole` the icon; `viewer-ui` its own ink, because it
draws its own page and has no theme.

## 3. Three things the standard does not state, chosen and written down

`CLAUDE.md` principle 5's rule for a genuine silence: say so plainly, choose, and document the
choice as one.

- **How many fields a tile shows.** Table 153 states "a subset" and "top-level information" and no
  number. `panel::TILE_FIELDS` is **two**, taken from the head of the producer's own `/O` order so
  that the fields a document put first are the fields it keeps. What the standard does fix is that
  it is *fewer* than the details view's, and the test holds that rather than the constant.
- **What the icon looks like.** "[D]enoted by a small icon" and nothing else — the `Text`
  annotation's situation one clause over. What this crate decides is the *kind*, from Table 44's
  `/Subtype` where the document states one, because that is the only thing about the file the
  standard puts in this program's hands; a file stating none is `Icon::File`, which says only that
  it is a file. Two of the three windows resolve the freedesktop icon-theme name
  (`Icon::theme_name`) and the third draws a page or a folder with nought to four bars in it.
- **Whether a tile keeps §12.3.5.2's folders.** Table 153 says "each file in the collection", and
  §12.3.5.2 is a separate `shall` about where a file sits. So the folder tree stands in both views
  and the mode changes only what a row shows — which also keeps this panel's standing rule that a
  file cannot fall out of it by being filed oddly.

## 4. What is still reported

`unsupported_presentation` keeps `FilmStrip`, `FreeForm` and `Linear`, and `/View C` naming a
navigator the file does not state. `DRAWN_LAYOUTS` is four names rather than three, so §12.3.6's
selection rule now selects tile mode for a producer that asked for it and this program can draw.
