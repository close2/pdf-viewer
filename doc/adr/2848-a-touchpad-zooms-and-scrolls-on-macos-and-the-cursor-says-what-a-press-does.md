# 2848 — A touchpad zooms and scrolls on macOS, and the cursor says what a press does

Session 2015, a host round on the owner's request for macOS trackpad gestures. Status:
implemented. Context: ADRs 0166 (the pointer over a link), 1192 (a Control this program does not
bind means nothing), 2048 §2.5 (the platform's command modifier, not Control), 2072 (the modifier
named by its role), 2274 and 2275 (a zoom in proportion, held at the point it was pointed at).

## Context

The owner, on the reference window under macOS: text can be selected, but the pointer does not
change over it, and the touchpad does not zoom. Read against the code, four separate gaps:

1. **No text cursor.** Every window changed the pointer over §12.5.6.5's activation region and
   nowhere else, and the boundary could not have said more: `Query::LinkAt` was its only question
   about a point at pointer speed. Where the glyphs are is this crate's — §9.4.4's text rendering
   matrix, ADR 0118's text layer — so no host could answer it.
2. **⌘ meant nothing.** `quorra` folded Control into the key table's `ctrl` and nothing else, so on
   macOS ⌘ + wheel scrolled, ⌘ + C copied only because bare `c` does, and Control + wheel — the
   system's own screen zoom — was this window's zoom. ADR 2048 §2.5 had already said "a modifier as
   the platform's command modifier and not as the Control key"; ADR 1192's sentence that the
   platform key is never folded predates it and was written about Linux, where that key is the
   window manager's.
3. **Two-finger scrolling went down only.** `quorra`'s wheel sent `dx: 0.0` whatever the touchpad
   reported across.
4. **Pinch and Smart zoom were never read.** winit 0.30 reports both on macOS (`PinchGesture`,
   `DoubleTapGesture`); no window had an arm for either.

## Decision

1. **`Query::TextAt(at)` answers `Answer::OverText(bool)`**: a point inside the bounding box of a
   glyph of the interpreted page under it. Inside rather than nearest, which is the difference from
   the selection's `position_at` — a drag off the text still means the nearest position, a pointer
   is *over* text only where a glyph is. A page not yet interpreted answers false, so the question
   never interprets. `viewer_host::Under::at` decides **link before text** for every window,
   because a press over a link follows it. `quorra`, `quorra-gtk` and `quorra-qt` show their
   toolkit's text cursor (`CursorIcon::Text`, `"text"`, `Qt::IBeamCursor`); C asks
   `quorra_text_at`; the confined pipe carries it as query and answer kind 39.
2. **The key table's `ctrl` is the platform's command modifier**: ⌘ on macOS, Control elsewhere
   (`quorra`'s `COMMAND`). The other of the two is still folded into nothing, which is ADR 1192's
   rule kept on both platforms.
3. **A touchpad scrolls on both axes**, at the pixels it reports.
4. **A pinch is a factor**: `viewer_host::pinch(delta)` is `ZoomTravel::Factor(1 + delta)`, spent by
   ADR 2274's `ZoomTravel::commands` about the pointer, so ADR 2275's gesture holds its point. A
   delta of nothing, nonsense, or −1 and below is no zoom.
5. **Smart zoom toggles the page's width and back** (`viewer_host::SmartZoom`). A first tap fits
   the width about the pointer; a second, with the magnification where the first left it, puts back
   the magnification the first began from. A page already at its width is doubled instead, so a
   first tap always goes in. Anything that moved the magnification in between makes the next tap a
   first tap. **A choice**: ISO 32000-2 says nothing about a touchpad, and macOS's own Smart zoom
   fits a block of content the viewer can find, which a page's text layer does not state as one.

**The system's settings are obeyed by construction.** macOS sends `smartMagnifyWithEvent:` only
while *Smart zoom* is on in the trackpad settings and `magnifyWithEvent:` only while *Zoom in or
out* is, so a window that answers the events does what the person set.

## Consequences

`viewer-core` states thirty-nine questions; `viewer-confined` carries all of them; `viewer-ffi`
gains one entry point. The .NET window declares `quorra_text_at` and does not yet call it; its
cursor is still the link's alone (`quorra_window_pointer`'s answer), which is the one window this
round left unlevel. GTK and Qt were edited and **not compiled on the round's machine** (macOS,
neither toolkit installed); CI builds them. Three-finger swiping between pages and overscroll's
elastic edge are not reported by winit 0.30 and are a native macOS window's (ADR 2048 §2.5).
