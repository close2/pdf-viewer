# ADR 1251 — The three layouts §12.3.6 describes a surface for, and what a preview picture is

Status: accepted, 2026-09-22. Session 1207, the batch's host-UI round.

Table 160's `FilmStrip`, `FreeForm` and `Linear` are drawn in all three windows, so §12.3.6's
selection rule now chooses from every name the table defines. What made them undrawable was not the
arrangement but the *picture*: each is built out of pictures of the attachments, and this program
had none. It has one now — the attachment's own first page's §12.3.4 `/Thumb`. Closes what ADR 1215
left in `doc/todo/65` bucket 1; supersedes nothing.

## 1. What the clause asks for, and what modal verb it asks in

Every sentence §12.3.6 addresses to a processor about a layout is a `should`, and Table 160's
selection rule is explicitly conditional on capability — "an interactive PDF processor should
present the first one it is capable of displaying in the order present in the array". So capability
is a legitimate input and was legitimately reported. What the table *states* is what each layout is:

> FilmStrip A layout which displays a strip of thumbnails, providing an index to the file
> attachments within the collection. The selected attachment should be previewed alongside the
> index.

> FreeForm A layout which places thumbnails of the file attachments within the collection randomly
> in the view.

> Linear A layout which provides a large size preview of one file attachment in the collection and
> displays alongside the preview the metadata for the file attachment, including the name,
> description and other collection schema entries.

and, in the prose under the table, that `FilmStrip`'s thumbnails "provide an index into the files
and folders present within the collection", that `FreeForm`'s are "displayed at a random location
on the view", and that for `Linear` "[a]n interactive PDF should display the first page of the file
and should use the file schema and file specification dictionary to provide information about the
attachment".

**`Linear` is one attachment and not a list**, which is worth writing down because the round was
briefed on the guess that it might be this panel's list in `/Sort` order. It is not: the entry says
*one file attachment*, large, with its metadata beside it.

## 2. The picture, which is the part that was missing

Three layouts made of thumbnails need a picture of an attachment, and the standard defines a
picture of a document's page in exactly one place — §12.3.4's `/Thumb`, "an image that shall be
used as a thumbnail image representing the page". So `viewer_core::Query::AttachmentPreview` opens
the named attachment as a document and answers its **first page's** `/Thumb`: the miniature that
file's own producer wrote, never one this program composed.

- **`None` is the common answer** — a file that is not a PDF, or states no `/Thumb`, or cannot be
  decoded. The row then carries [`Picture::kind`]'s icon and the panel's own sentence says which
  files those were, because an icon passed off as a thumbnail is trap 5's shape and the mirror
  image of the mistake ADR 1215 corrected.
- **One level, and no new bound.** The nested document is asked for a page's `/Thumb` and never for
  *its* attachments, so a document embedding itself is one open rather than a walk; the bytes come
  out under `pdf_syntax::Limits::max_stream_len` and the nested parse runs under the same limits,
  so nothing here picks a second number (trap 38).
- **One attachment at a time.** `viewer_host::panel::Previews` is `Miniatures`'s sibling, keyed by
  the `/EmbeddedFiles` key and bounded by the same `KEPT_MINIATURES`, and a file with no picture is
  held as one — otherwise the same embedded document is opened again on every frame.

## 3. What each window draws

`viewer_host::panel::Mode` has five values and `PanelRow::picture` says what each row carries.
Rows are **flat** in all three new layouts, the folder each file sits in named on the row rather
than drawn around it: §12.3.5.2's `shall` is about membership, and none of these three surfaces
nests. `FilmStrip` carries the folders as items of the index because its own prose says so;
`FreeForm` and `Linear` are stated over "the file attachments" and do not.

- **`viewer-ui`** draws a run of picture rows for `FilmStrip` and `Linear`, and a scatter of its
  own for `FreeForm` — a second drawing path with its own hit test, because a list with jitter is
  not a scatter. Its collection rows are now `viewer_host::panel::collection_rows`'s rather than
  its own: three layouts tripled what a panel decides about a collection, and a third copy of those
  decisions is where two windows begin to disagree about the clause. The *ink* stays here, which is
  what ADR 0711's division was protecting.
- **`viewer-gtk`** binds a `GtkImage` at the size the picture asks for, and a `GtkFixed` inside a
  `GtkScrolledWindow` is its scatter.
- **`viewer-qt`** puts a `QPixmap` in `Qt::DecorationRole` with a `Qt::SizeHintRole` to match, and
  §12.3.5's panel is a `QStackedWidget` of the `QTreeView` and a `QListView` in icon mode with free
  movement — the one Qt view whose item positions a program may set.

## 4. Three silences, chosen and written down

`CLAUDE.md` principle 5's rule for a genuine silence: say so, choose, and record the choice.

- **Where a scattered thumbnail goes.** "[A] random location on the view" and nothing else. The
  place is derived from the file's own `/EmbeddedFiles` key (`panel::scattered`), so it is stable
  under redraw — a scatter redrawn every frame is unusable — and the three windows share the
  derivation so that a person moving between them finds the same file in the same spot. That
  sharing is `doc/todo/30`'s level-hosts rule, not the clause's.
- **How large each picture is.** "[A] small icon", "thumbnails", "a large size preview": three
  sizes and no measurement. Each toolkit picks its own, which is what a size is.
- **How another attachment is reached under `Linear`.** The clause states one attachment and says
  nothing about choosing a different one. The others are listed under it as plain rows.

**Which attachment is previewed is not a silence**: §12.3.5.1's `/D` names it, and where `/D` names
the container or a file the tree does not hold, the clause's own fallback applies — "the first item
from the list of files to display in its user interface", which in this list is the first in
Table 153's `/Sort` order.

## 5. What is left reported

`panel::unsupported_presentation` keeps one case: a navigator naming *only* layouts Table 160 does
not define, which §12.3.6 permits — "[t]his mechanism is inherently extensible and allows inclusion
of custom named layouts" — while requiring a producer to name one of the seven as well. A
conforming file now always selects something this program draws.
