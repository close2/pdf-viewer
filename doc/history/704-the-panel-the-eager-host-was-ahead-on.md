# 704 — Three panels the native hosts did not have, and the launch path the host that was ahead put them on

Date: 2026-08-24. ADR [0564](../adr/0564-three-panels-a-native-host-did-not-have.md).

Touched: `crates/viewer-host/src/{panel.rs,status.rs,lib.rs}`;
`crates/viewer-gtk/src/{host.rs,page.rs,pages.rs (new),tree.rs,lib.rs}`;
`crates/viewer-qt/src/{host.rs,bridge.rs}` and `cpp/{window.h,window.cpp}`;
`crates/viewer-ui/src/chrome.rs` and
`src/bin/pdf-viewer{.rs,/app.rs,/dispatch.rs,/sidebar.rs,/overlays.rs,/surface.rs,/trace.rs}`,
`tests/{panel.rs,chrome_over_a_magnified_page.rs}`, `examples/chrome_ladder.rs`;
`doc/conformance/ledger.toml` (§7.7.2, §12.3.4, §12.4.3, §14.3.2, §14.3.3),
`doc/todo/30-a-native-host.md`, `doc/state-of-play.md`, `doc/ui-boundary.md`.

The **fifth** round on the project owner's *"even though low priority, I think we should start
investing time into the UI (and its API for the native versions)"*, taking **item 4** of the
ordering ADR 0509 wrote, and the residue ADR 0545 named.

## The item, and the thing it turned out to be

`viewer-ui` drew six sidebar tabs and the two native hosts drew three. What was built is not three
panels but **`viewer_host::Tab`** — a closed list of the six, with Table 29's `/PageMode` mapping on
it, matched exhaustively in all three hosts, so a seventh fails to compile in three places. That is
ADR 0526's key mechanism applied to the other thing a window shows, and it is the instrument
"all three hosts stay level" had for a *binding* and not for a *panel*.

`article_rows` and `property_rows` joined `outline_rows`, `layer_rows` and `attachment_rows`; the
tier-2 host lost `chrome::Tab` and its own copies of both adopting them. **It needed no message**,
the tenth time since the six-hundred-and-seventh.

**§12.3.4 is the one answer that is not a row**, and it is where this crate's "no widget and no
pixel format" line actually falls: `page_entry` and `Miniatures<T>` are shared — the two queries a
row needs, and the policy of decoding on demand, keeping what is near and dropping what is far under
one bound — while the picture stays a `gdk::Texture`, a `QPixmap` or a `pdf_render::Image`.

## The host that was ahead held the clause violation

`CLAUDE.md` section 2 forbids thumbnail generation on the launch path by name, and
`Query::Thumbnail` answers one page at a time so that a host can obey it. `viewer-ui` looped over
the page count the first time the tab was shown — and Table 29's `/PageMode /UseThumbs` opens that
tab *as the document opens*, so the whole list was on the launch path.

Measured under `Xvfb` on a thousand-page document stating `UseThumbs`, release binaries rebuilt
before each run:

| | first present | §12.3.4's list |
|---|---|---|
| `viewer-ui`, before | 156 ms | 1000 rows, **121 ms** |
| `viewer-ui`, after | **48 ms** | 8 rows, 0.30 ms |
| `viewer-gtk` | 108 ms | 1000 rows registered, nothing decoded |
| `viewer-qt` | 74 ms | 1000 rows registered, nothing decoded |

Scrolling fetches one row per step (`rows 27..35 of 1000 … 1 fetched in 82.6 µs, 35 held`). The fix
came with a decision: **every row of that tab is the same height whether or not it has a picture**,
because a height that depended on the fetch made the layout a function of the fetch and the fetch a
function of the layout — which is why the host decoded all of them.

**The two native hosts never had it, and that is their toolkits' doing rather than this project's
care.** Writing the same panel twice in two toolkits that are virtual by construction is what showed
that the host drawing its own rows was not.

## Three things only the screen could say

- **GTK binds a row synchronously when the list goes into a realised window** — inside
  `build_panels`, which runs with the host's `RefCell` borrowed. The first run printed *"the host was
  busy, so page 1's row was drawn without its /Thumb"* and drew a page list with no pictures. The
  list is appended from an idle now. **The note is the finding**: a silent fallback would have
  shipped an empty panel.
- **Six tab labels do not fit across a sidebar**, in either toolkit, and both answer by hiding the
  rest behind scroll arrows — four of six in GTK, three of six in Qt. Both hosts put their tabs down
  the side. Two toolkits agreeing about a problem is not a taste.
- **An empty list is not a sentence.** Every panel now carries a row saying which of "the document
  states none" and "this program failed to fill it" it is; `PanelRow::note` marks it and each
  toolkit dims it its own way.

## The residue ADR 0545 named

`Event::OpenFailed` and a page tree with no leaves both called `std::process::exit(1)` in
`viewer-ui`. `viewer_host::cannot_open` and `no_pages` are the two sentences,
`viewer_ui::chrome::Refusal` is the card, and **neither native host said anything about a document
with no pages either** — §7.7.3.2 states no floor on `/Count`, so such a file is correctly read and
has nothing to show, and a blank window looks exactly like a broken one. Six runs under `Xvfb`, two
files, three hosts: the same words every time and every process still running afterwards.

## What was driven, and on what

`Xvfb :74`, plain `xdotool` (XTEST), release binaries installed before every measurement:
`doc/pdf.js/test/pdfs/personwithdog.pdf` (a real `/Thumb`, 76×99 — the miniature is drawn in all
three hosts), a thread fixture (*Man Bites Dog*, 3 beads — the Read panel in all three), a
thousand-page `/PageMode /UseThumbs` fixture (the table above, and §7.7.2 obeyed in a host that used
to report it), a file with no `%PDF-` header and a `/Count 0` document.

**One instrument fact is worth keeping**: `pkill -f pdf-viewer` in a script under this worktree
kills the round's own shell, because the *path* contains that string. `pkill -x` matches the process
name and does not.

## What was run

`fmt`, `clippy --workspace --all-targets` under `-D warnings`, `nextest --workspace`, the workspace
doctests, `check` over the fuzz targets, and `cargo test -p conformance` — the core plus the
conformance gate, which is `doc/todo/02` §2's row for a change to the host crates and the ledger,
plus the two censuses `viewer-core` is under. §5's binaries were rebuilt and installed before every
measurement.

Nothing here can change a pixel a corpus gate rasterises: no `pdf-*` crate and no rasteriser. The
one `pdf-render` name added is `Image`, read and not changed.

## What is left, named rather than left silent

§12.3.5's collection and §12.5.6.14's popup windows are still `viewer-ui`'s alone, and neither is a
*tab*, so `Tab` does not reach them. **Nothing counts what a window cannot reach** the way
`tools/state.sh hosts` counts what a C caller cannot, and that is the instrument this round did not
build.

**Nothing is queued for the owner's measurement loop.** Every number here is a window, a key and a
screenshot, and `Xvfb` answers all three.
