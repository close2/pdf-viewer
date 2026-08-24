# ADR 0564 — Three panels the native hosts did not have, and the list of panels itself

Status: accepted, 2026-08-24. Session 704, the **fifth** round on the project owner's *"even though
low priority, I think we should start investing time into the UI (and its API for the native
versions)"*, taking **item 4** of the ordering ADR 0509 wrote — §12.3.4's thumbnails, §12.4.3's
articles and §14.3.3's properties in `viewer-gtk` and `viewer-qt` — and the residue ADR 0545 named
and deliberately left.

## 1. The item, and what it turned out to contain

`viewer-ui` drew six sidebar tabs and the two native hosts drew three. ADR 0509 ranked closing that
gap fourth on the ground that a panel is a thing a reader can do without; what it did not predict is
that the *list of panels* was the part worth building, and that the tier-2 host — the one that was
ahead — held the round's only clause violation.

Four things landed:

- **`viewer_host::Tab`**, a closed enumeration of the six panels this program shows, with `ALL`, one
  label apiece and Table 29's `/PageMode` mapping. `viewer-ui` lost `chrome::Tab` adopting it.
- **`viewer_host::panel::article_rows` and `property_rows`**, so that §12.4.3's threads and
  §14.3.3's information are one mapping and not three; `viewer-ui` builds its rows from them now.
- **`viewer_host::page_entry` and `Miniatures`** for §12.3.4, which is the one answer that is not a
  row — see §3.
- **`viewer_host::status::cannot_open` and `no_pages`**, which is ADR 0545's residue: `viewer-ui`
  called `std::process::exit(1)` on `Event::OpenFailed` and on a document with no pages.

**No new message**, which is the tenth time since the six-hundred-and-seventh that a feature landing
in every host has needed no channel. `Query::Thumbnail`, `Query::Articles` and `Query::Properties`
have all existed and all answered since long before this round; what was missing was three hosts
asking them.

## 2. Why the list of panels is a value, and why it is in `viewer-host`

ADR 0526 made "all three hosts stay level" checkable for a *key*: `viewer_host::Key` is closed, each
host walks `Key::ALL` through a match exhaustive over it, and a binding added to the table fails to
compile in three hosts until each supplies its toolkit's key. That mechanism had no counterpart for
the other thing a window shows, which is why six tabs against three could sit there for sixty
sessions without any instrument saying so.

`Tab` is that mechanism applied to a panel, and it is checked in two places rather than one:

- **The compiler.** `viewer_gtk::Host::panel_of`, `viewer_qt::Host::panel_of` and
  `viewer_ui::chrome::Sidebar::rows` each match exhaustively over `Tab`, so a seventh panel fails to
  compile in all three hosts until each says what it draws for it.
- **A test per host**, in the shape ADR 0526 established: GTK asserts that its notebook page numbers
  are `Tab::ALL`'s own order (the two ways of addressing a panel cannot disagree), and Qt asserts
  that the *place* its C++ builds a `QListView` at is the one place the match calls not-a-tree.

It is deliberately not in `viewer_core`, for that crate's rule 5: a panel is chrome. What the core
answers is five queries; which of them a window offers, in what order and under what words is a
host's — and it is one decision rather than three, which is the argument `Presenting`, `Clock`,
`keys` and `password` are all in this crate under.

**The words come with it.** `Tab::label` is six strings and `viewer-qt` asks for them across the
bridge rather than writing a second set into `window.cpp`, which is the same reason `notices` and
`password_prompt` are functions there.

## 3. §12.3.4 is the one answer that is not a row, and the split that follows

A thumbnail is a picture, and a picture is a `gdk::Texture`, a `QPixmap` or a `pdf_render::Image`
depending on who draws it. `viewer-host`'s own documentation says it holds "no widget … and no pixel
format", and that line is where this item divides:

- **shared**: `page_entry(viewer, index)` — the two queries a row needs and the fallback label a
  page with no §12.4.2 label gets — and `Miniatures<T>`, which is *the policy*: decode on demand,
  keep what is near, drop what is far, bounded by `KEPT_MINIATURES`.
- **each toolkit's**: the widget, and the picture in its own type.

`Miniatures` evicts by distance from the row last asked for rather than by age, because a panel is
scrolled: what a view wants next is next to what it just wanted, and a least-recently-used order
would throw away the row above the viewport in favour of one the reader left a thousand pages ago.
`viewer-qt` keeps its `QPixmap` cache in C++, because a `QPixmap` is C++'s — but it asks the bridge
for the *bound*, so the number is one number.

## 4. The measurement, and the clause violation it found in the host that was ahead

`CLAUDE.md` section 2 forbids thumbnail generation on the launch path **by name**, and
`Query::Thumbnail` answers one page at a time so that a host can obey it: *"the panel knows which
eight it is showing; this crate does not."* `viewer_ui::chrome::Content::pages` carried a comment
saying exactly that. The code did the opposite: `App::ensure_pages` looped over the page count the
first time the tab was shown.

That is not merely deferred work, because **Table 29's `/PageMode /UseThumbs` opens that tab as the
document opens**. Measured on a thousand-page document stating it, under `Xvfb`, release binaries
rebuilt before each run:

| | first present | §12.3.4's list |
|---|---|---|
| `viewer-ui`, before | 156 ms | 1000 rows, **121 ms** |
| `viewer-ui`, after | **48 ms** | 8 rows, 0.30 ms |
| `viewer-gtk` | 108 ms | 1000 rows registered, nothing decoded |
| `viewer-qt` | 74 ms | 1000 rows registered, nothing decoded |

The fix is `Sidebar::visible_pages`, and it comes with a decision worth stating: **every row of that
tab is the same height whether or not it has a picture.** A row whose height depended on whether its
`/Thumb` had been decoded makes the layout a function of the fetch and the fetch a function of the
layout — which is precisely why this host decoded all of them. Scrolling the list fetches one row
per step (`rows 27..35 of 1000 … 1 fetched in 82.6 µs, 35 held`).

**The two native hosts never had it**, and that is a fact about their toolkits rather than about
this project's care: a `GtkListView` binds the rows it lays out and a `QAbstractListModel` is asked
`data` for the rows a view lays out. Writing the same panel twice in toolkits that are virtual by
construction is what showed that the host drawing its own rows was not.

## 5. What the screen said that no compiler could

Three things, and each cost a run:

- **GTK binds a row synchronously when the list is put into a realised window**, which happens
  inside `build_panels` — with the host's `RefCell` borrowed. The first run printed *"note: the host
  was busy, so page 1's row was drawn without its /Thumb"* and drew a page list with no pictures in
  it. The list is appended from an idle now, which is the move `find.connect_search_mode_enabled_notify`
  already makes and which `viewer-qt` makes with `Busy`. **The note is the finding**: a silent
  fallback would have shipped an empty panel, and trap 5 is why there was a sentence to read.
- **Six tab labels do not fit across a sidebar.** Both toolkits answer a notebook that cannot fit
  its tabs by putting the rest behind scroll arrows, so four of six panels in GTK and three of six
  in Qt were reachable only by pressing an arrow nobody would look for. Both hosts put their tabs
  down the side (`PositionType::Left`, `QTabWidget::West`) in this round, which is two toolkits
  agreeing about a problem rather than a taste.
- **An empty list is not a sentence.** `outline_rows` answers an empty vector for a document with no
  outline, and a panel that drew nothing looks exactly like a panel this program failed to fill.
  Every panel now has a row saying which of the two it is, `PanelRow::note` marks it, and each
  toolkit says so its own way — a `dim-label`, `Qt::ItemIsEnabled` cleared, a paler italic.

## 6. The residue ADR 0545 named, and why it is the same argument

`viewer-ui` called `std::process::exit(1)` on `Event::OpenFailed` and on a document whose page tree
has no leaves. ADR 0545 removed the same call from §7.6.4.1's prompt one round earlier and left
these two explicitly for a later round rather than widening its own item.

The argument is that ADR's, unchanged: a window that leaves the process has told a person who
launched it from a desktop nothing at all. Both native hosts have put the sentence in a status bar
and stayed up since their first session — **except that neither said anything about a page tree with
no leaves**, which is not an error at all: §7.7.3.2 makes `/Count` "the number of leaf nodes (page
objects) that are descendants of this node" and states no floor, so this program has read the file
correctly and there is nothing to show. A blank window and a broken file look identical, which is
trap 5's own subject.

`viewer_host::cannot_open` and `no_pages` are the two sentences, `viewer_ui::chrome::Refusal` is the
card, and all three hosts now say the same words and stay up — checked under `Xvfb` on a file with
no `%PDF-` header and on a `/Count 0` document, six runs, every process still running afterwards.

**The card has a border and §7.6.4.1's does not**, which is a difference worth a line: the password
card is drawn over a page dimmed to 45% black and stands out against it, while this one is drawn on
a window with no page at all, where a near-white card on a near-white ground is a sentence nobody
can see.

## 7. What was not done, and is named rather than left silent

- **`Query::Collection` and `Query::Popups` still reach no native host.** §12.3.5's collection is
  the files panel presenting itself differently and §12.5.6.14's popups are drawn over the page
  rather than in a panel; neither is a *tab*, so neither is `Tab`'s business, and both are
  `viewer-ui`-only for now. `tools/state.sh hosts` counts what a C caller cannot reach; nothing
  counts what a *window* cannot, and that is the instrument this round did not build.
- **`viewer-ffi` gained nothing**, and that is right rather than an omission: a C caller draws no
  panel and `Query::Thumbnail`, `Query::Articles` and `Query::Properties` reaching the ABI is
  `doc/todo/30`'s item 5, which is where ABI surface is decided.
- **GTK draws an outline item's Table 152 style nowhere**, which this round did not change and which
  `PanelRow` has never carried. It is a real asymmetry with `viewer-ui` and it is a *row's* property
  rather than a panel's, so it belongs to whichever round makes `PanelRow` carry style at all.
