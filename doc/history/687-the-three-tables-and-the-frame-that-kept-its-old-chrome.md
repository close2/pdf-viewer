# 687 — The three tables that disagreed, and the frame that kept its old chrome

Date: 2026-08-23. ADR [0526](../adr/0526-what-a-key-means-stated-once.md).

Touched: `crates/viewer-host/src/keys.rs` (new) and `lib.rs`; `crates/viewer-gtk/src/host.rs`;
`crates/viewer-qt/src/{keys.rs,host.rs,bridge.rs}` and `cpp/{window.cpp,window.h}`;
`crates/viewer-ui/src/bin/pdf-viewer.rs` and `src/bin/pdf-viewer/{window.rs,arguments.rs,
renderer.rs,surface.rs}`; `NOTICE`; `doc/conformance/ledger.toml` (§12.4.4.2, §12.5.1),
`doc/todo/30-a-native-host.md`, `doc/state-of-play.md`.

The third round on the project owner's *"even though low priority, I think we should start
investing time into the UI (and its API for the native versions)"*, taking **item 2** of the
ordering ADR 0509 wrote — and item 6 came with it, exactly as that ADR predicted it would.

## The item

Three windowed hosts, three key tables, and they disagreed: `f` was the find bar in `viewer-gtk`
and §12.5.6.6's free-text drag in `viewer-ui`; Up and Down scrolled the view in one host and turned
the page in two; Escape cleared the selection in the two native hosts and **quit the process** in
the third. `viewer_host::keys` is the one table now, and each host supplies only the translation
from its own toolkit's key.

**It needed no message** — the eighth time since the six-hundred-and-seventh that a feature landing
in every host has needed no channel, and it was checked rather than assumed.

## What the briefing was wrong about, and it was worth checking

**"Four key tables."** There are three. `viewer-ffi` has no keyboard and never had one, and its own
header says so where it mentions the only key the standard names: *"§12.5.1's tab key. The order is
the document's (Table 31's `/Tabs`); the key is yours."* A C caller places its own toolkit and owns
its own keyboard, so the fourth consumer is a different kind of thing here rather than a host that
is behind. What it *could* have — the table as data, `pdfv_key_meaning` — is an addition to the
ABI's surface, and that is `doc/todo/30`'s item 5; it is written down there rather than taken here.

## What the standard actually decides, which is two rows

§12.5.1's *"in particular, the tab key"* and §12.4.4.2's *"[p]ressing an arrow key"*. The second
settled the sharpest disagreement, because both of its sentences are inside the **presentation**
subclause: all four arrows navigate while one is running and Up and Down move the view while one is
not. It fixed a gap in the other direction too — `viewer-ui`'s arrows scrolled *during* a
presentation, so two of the four keys the clause names made no navigation request at all there.

§7.7.2's Table 29 takes a binding away rather than giving one: while a presentation is running,
`f`, `/`, `o` and `?` mean nothing, which is "no … other window visible" applied to the keyboard.

Everything else is a documented choice, and they are written down as choices in the module.

## The instrument, which is what ADR 0509's third criterion asked for

Each host carries `every_key_the_table_states_has_one_in_this_toolkit`: it walks `Key::ALL` through
a match that is **exhaustive over the enumeration**, so a binding added to `viewer-host` fails to
compile in all three hosts, and then asserts that the host's runtime translation agrees. A key named
in the test and forgotten in the translation fails rather than drifting.

## The defect that was not a key's

`?` had to mean something in all three hosts. In `viewer-ui` it did nothing — and the branch that
toggles the card is what the old handler had, character for character. **Every overlay that host
draws was unreachable on the graphics device's path**: the find bar, the panel, §12.5.1's ring, the
selection and the notices card. `surface.rs`'s *"a rendering of exactly these lists at exactly these
targets needs no successor"* compared the pages and their targets and not the chrome drawn over
them, so a window whose pages had not moved put up a frame drawn with the *previous* chrome; and the
one branch that needed the clock armed afterwards did not arm it, so a frame asked for on that tick
would have sat in the channel unread.

Found the way trap 1 says: `about.shown` was `true`, the display list existed for the frame, and the
screen was the only thing that could say otherwise.

## What was measured

**Three release binaries driven under `Xvfb` on `doc/PDF20_AN001-BPC.pdf`**, keys through plain
`xdotool key` (XTEST — 683's instrument lesson, because `--window` is ignored by Qt):

- `f` opens the find bar in all three and Escape closes it; **no host exits on Escape**, checked by
  the process still running afterwards.
- `h` with nothing selected is refused **in the same sentence** by all three; `a` then `h` marks the
  page up in all three, which the title bar's `•` says, and `z` takes it back.
- `t` arms §12.5.6.6's drag in `viewer-ui` and is **refused by name** in both native hosts.
- `w` answers in all three — the native hosts with *"every control on this page already fits its
  /Rect"* and the tier-2 host with why it never has one to magnify for.
- `l` cycles to `TwoColumnLeft` in all three.
- `?` puts the notices on the screen in all three: a top-level *Third-party notices* window in the
  two native hosts (window count 1 → 2 in each), and the card over the page in `viewer-ui`.
- `o` takes the panel away and gives the page the whole window, in all three.

**The frame change, A/B in one sitting with `--trace=frames`**: an idle window draws its last frame
at about 0.9 s and **zero frames after ten seconds on both builds** — `doc/todo/36`'s fourth rule
holding — while six chrome-changing key presses cost **14 frames before and 28 after**, which is one
render and one present per change and is the feature rather than an overhead.

## What was run

`fmt`, `clippy --workspace --all-targets` under `-D warnings`, `nextest --workspace`, the workspace
doctest, the fuzz targets' `check`, and `cargo test -p conformance` — the core plus the conformance
gate, which is what `doc/todo/02` §2's map puts a change to the five host crates and the ledger
under. §5's binaries were rebuilt and installed twice: once before any measurement, and again after
the frame fix, because the second measurement is of the second tree.

One gate caught a real mistake: `every_quotation_is_the_standards_own_words` refused Table 29's
`FullScreen` wording because the nearest citation above the blockquote was §12.5.6.6 from the bullet
before it. The fix is the citation, not the quote.

**`tools/round.sh` printed session 685 and "a fifth round"**, because the two rounds between it and
this one had not written their history files yet; this is 687, which is not a fifth. Nothing here
can change a pixel a corpus gate rasterises — no `pdf-*` crate, no rasteriser — and the change map's
own row for the host crates is the core.

**Nothing is queued for the owner's measurement loop.** Everything here is a window, a key and a
screenshot, and `Xvfb` answers all three. The one thing a real display would add is worth naming
rather than queuing: the frame fix was measured on `lavapipe` under `Xvfb`, so the *number* of
frames a chrome change costs is this adapter's; that it is one render and one present is the
program's and is the same everywhere.
