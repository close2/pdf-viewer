# 0441 — The column a window could not hold

Status: accepted.
Session: the six-hundred-and-sixth.
Subject: ISO 32000-2 Table 29's `/PageLayout`, obeyed; one command added to the boundary; one
answer changed shape; the pointer defect the reading found.

## What the clause says

§7.7.2, Table 29's `/PageLayout` row:

> A name object specifying the page layout shall be used when the document is opened

with six values and `SinglePage` as the table's own default. The clause states each value in a
phrase — "Display one page at a time", "Display the pages in one column", "Display the pages two
at a time" — and the four two-page values name a *side* for the odd-numbered pages.

**The six divide along two axes rather than one**, and reading them that way is what made the
implementation small:

| | one page across | two pages across |
|---|---|---|
| **one at a time** | `SinglePage` | `TwoPageLeft`, `TwoPageRight` |
| **a column to scroll** | `OneColumn` | `TwoColumnLeft`, `TwoColumnRight` |

Nothing else in the six differs. "At a time" is the phrase that separates the rows: it says what is
on the screen at one moment, where "in one column" describes a thing a reader moves through.

**"Odd-numbered" is the page's number and not an index.** Page one is odd, so `TwoColumnLeft` opens
with pages one and two side by side and `TwoColumnRight` leaves page one alone on the right — which
is the arrangement of a bound book, and the reason the standard states the two values separately at
all.

**Table 147's `/Direction` is declined, and that is a reading rather than an omission.** It says a
reading direction "may be used to determine the relative positioning of pages when displayed side by
side or printed n -up" — a *may* — while `/PageLayout` has already said *left* and *right* in so many
words. Where one entry states a side outright and another permits a rearrangement, the statement
wins; a document that wants its odd pages on the right has `TwoColumnRight` to say so. Recorded as a
choice, because the other reading is available and this one is not forced.

`spec-errata emit` was run over clause 7.7.2 and clause 12 before any of this was written. The
errata on Table 29's own pages concern `/Pages`, `/Dests` and `/Threads` being indirect references,
`/Lang`, and a `deprecated in PDF 2.0` on `/URI`; §12.2's concern `/ViewArea`'s table reference and
`UseAttachments`. **Not one touches `/PageLayout`**, so the row stands as printed.

## The boundary question, asked before any host code

`doc/ui-boundary.md`'s test is *what does a host not have that only the viewer has*. A layout looks
like a view concept, which is a host's by rule 5, so the question is worth asking carefully.

**A host cannot arrange the pages.** Deciding which pages a viewport shows needs their extents,
which means the page tree; placing them needs ADR 0118's arithmetic — the magnification, the
centring, the y flip — which is the one thing this boundary most deliberately keeps in one place;
and *drawing* them needs an interpretation per page, which only `viewer-core` produces. A host holds
none of the three. So the arrangement is the viewer's, and it is not a second way to say something a
host could already say.

**What only a host knows is which arrangement a person has chosen.** Table 29 states the layout that
"shall be used when the document is opened" — an *initial* state, which `viewer-core` reads out of
the catalog for itself and needs no host for. What no state machine over a file can see is that the
reader then pressed a key. That is exactly §12.4.4.2's shape (ADR 0316) and exactly `CLAUDE.md`'s
rule about a document's assertions over its reader: an arrangement a reader cannot leave is an
arrangement somebody else's file imposed on them.

So: **`Command::Layout(PageLayout)`**, the fourth policy value beside `Restrict`, `Present` and
`Delegate`. It differs from those three in one respect and the difference is the clause's: they are
facts about the *window* and apply to every open document, and this is a way of looking at one file,
because Table 29 makes it a *document's* entry.

**And one answer changed shape rather than a second question being added**: `Answer::Frame` carries a
`Vec<FrameView>`, one entry per page on the screen. `OneColumn` puts several pages in one window and
a host drawing only the first would draw a continuous view with a hole in it — which is the
mechanism `doc/ui-boundary.md` prefers whenever a host needs several of what a variant carried one
of (ADRs 0166, 0167, 0247, 0248, 0431). Four consumers failed to compile.
`PDFV_EVENT_KIND_COUNT` did not move: a `Command` is a symbol and only an `Event` is a number.

`Query::PageGeometry` needed **no** change at all, which is the pleasing half. Its own documentation
already said "a page that is not the one showing has no place on the screen", and what changed is
which pages those are.

## What the arrangement is, and why it walks no page tree

`crates/viewer-core/src/layout.rs`, and the whole of it is *rows*.

- `Open::page_index` is the current page **and** the row the scroll is measured from. Under
  `SinglePage` that is what it always was.
- A scroll that leaves the current row behind moves the current page forward and subtracts the same
  distance from the scroll, so **nothing on the screen moves** when it happens. That is what makes a
  continuous view one surface rather than a sequence of pages, and it is what raises §12.6.3's page
  events — a page that has scrolled off the top is a page turned.
- Placement measures a row only when it is about to place it, stops as soon as a row starts below the
  window, and is bounded by `layout::MOST`. **A 500-page document costs exactly what a 5-page one
  costs**, which is `CLAUDE.md`'s startup rule stated as an algorithm rather than promised.

`Open::on_screen` replaced four fields that used to describe *the* page — its object, its
interpretation, the render outstanding for it and the pixels a host handed back — with one entry per
page of the arrangement. **The bound the old comment asked for arrived with the clause**: that
comment said a cache of display lists "would also need a bound and an eviction rule, and both should
be written after somebody measures what a display list costs to hold". `/PageLayout` supplies both
without anybody choosing a number — what is kept is what is on the screen, and what is evicted is
what has scrolled off it.

Two smaller consequences, both improvements that the clause forced rather than suggested:

- **A selection carries its page.** A range is into one page's readback; until an arrangement could
  show several pages the page was always the current one, so "a page turn ends a selection" was the
  whole rule. A scroll across a row changes the current page without a person having touched what
  they selected, so the rule is now the honest one: a selection lives until its own page leaves the
  screen.
- **`Frame` and `Interpreted` each lost a field** that the entry holding them already stated.

Two choices the standard does not state, written down as choices: the gap between neighbouring pages
is 8 logical pixels, and pages in a spread share their top edge.

## The defect the reading found

`Viewer::pointer` mapped its point into default user space with `Self::user_space` and then put the
result through `content::user_space_at` **again** for the annotation under the pointer. Two
applications of one inverse. So on every page whose crop box does not start at the origin, and on
every page §7.7.3.3 turns, §12.5.5's appearance state, §12.6.3's four pointer triggers, the focus a
press gives a widget and §12.5.1's popup activation all hit-tested somewhere the pointer was not.

It is trap 12a's shape for the third time, and it survived for the same reason as the first: no gate
clicks, and `Query::LinkAt` next door — which applies the inverse once — was right, so the two
answers disagreed in a way nothing compared. Found by reading the function in order to make it
page-aware, not by a test.

## What each host does with it

The owner's decision of 2026-08-20 is that all three hosts stay level. **Two of the three took it and
the third did not**, and the reason is a property of its tier rather than an omission of effort:

- **`viewer-gtk`** draws one `gdk::MemoryTexture` per page into a `GtkFixed`, growing the widgets it
  needs and hiding the spares. `l` cycles the six.
- **`viewer-qt`** builds one `QImage` per page and `PageArea` paints the list. `l` cycles the six.
- **`viewer-ui`** is tier 2: its surface draws exactly one `Arc<DisplayList>` per frame and
  `crate::stale`'s reprojection is keyed on that list's identity, so a column would need either a
  merge of display lists in `pdf-render` or a second rendering path. It therefore **asks the viewer
  for `SinglePage`** when a document opens — a statement of what it can draw, made through the
  boundary in one message — and goes on saying out loud what the document asked for. That is not a
  half-wired host: it is a host declaring a capability, and the next round has one task rather than a
  silent wrong picture to find.

`viewer-ffi` gained two entry points, 112 → 114: `pdfv_layout`, and `pdfv_frame_count`, which is the
one thing a C caller could not have deduced — a C consumer cannot fail to compile, so an arrangement
putting a second page on the screen has to be something a caller can *ask* about.
`pdfv_frame_info` and `pdfv_frame_copy` gained an index.

`viewer-confined` carries the command as wire byte 24 and the frames as a counted list.

## What it costs, and what it does not

`SinglePage` is Table 29's default and what every document that states nothing opens in, and under it
every structure above holds exactly one entry: one placement, one interpretation, one render request,
one frame. The 2 224 workspace tests and the whole gate sequence run on that path and are what say
the cost is nothing where the arrangement is one page.
