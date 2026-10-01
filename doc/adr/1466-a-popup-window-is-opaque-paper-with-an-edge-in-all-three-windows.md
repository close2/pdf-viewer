# 1466 — A popup window is opaque paper with an edge, in all three windows

Session 1315. Status: **accepted**. Amends ADR 0613 section 1 ("What is *not* shared is the look")
in two respects and no others.
Context: `crates/viewer-host/src/popup.rs` (`PAPER`, `EDGE`), `crates/viewer-ui/src/chrome.rs`
(`draw_popup`), `crates/viewer-gtk/src/host.rs` (`popup_window`), `crates/viewer-qt/cpp/window.cpp`
(`PopupWindow`), `crates/viewer-host/src/wheel.rs` (`ZoomWheel`).
Clauses: ISO 32000-2 §12.5.6.14.

## 1. What the clause leaves to this program

§12.5.6.14 says of a popup annotation: "It shall have no appearance stream or associated actions of
its own". So nothing in the file says what its window looks like, and the window is furniture this
program draws over the page — a choice, not a reading (principle 5's last clause).

## 2. What the three windows drew

Driven under `Xvfb` with `tools/drive-windows.sh` and a copy of its `drive.pdf` whose popup is open
over the page's own text:

- `quorra` drew `viewer_ui::chrome`'s cream paper inside a one-pixel grey edge.
- `quorra-qt` filled the body with the platform's `Base` — white, the page's own colour — and its
  `QFrame::StyledPanel` drew no line under the style in use, so the window's extent was invisible.
- `quorra-gtk` drew the theme's `GtkFrame` line and **no ground at all**: the page's words showed
  through the note's.

## 3. The decision

The body is **opaque**, so the page does not show through a note, and **not the page's white**, so
a person can see where the window ends. Its ground is `viewer_host::popup::PAPER` and a one-pixel
line round it is `viewer_host::popup::EDGE` — the colours `quorra` already drew — in all three
windows: `quorra` reads the two constants, `quorra-qt` is handed them across the bridge (`paper`,
`edge` on `QtPopup`) and paints the edge in `PopupWindow::paintEvent`, and `quorra-gtk` paints both
in a drawing area under the window's content instead of a `GtkFrame`. The title bar keeps ADR
0613's split: Table 166's `/C` where the file states one, the toolkit's own where it does not.

ADR 0613 kept the look out of `viewer-host` because a shared abstraction over two toolkits' widgets
would be an invention. Two colours are not a widget; what three windows disagreed about here was
whether a person can read the note, which is not a fact about a toolkit.

## 4. Control and the wheel, which the same drive found missing

`quorra-gtk` and `quorra-qt` scrolled under Control where `quorra` zoomed (ADR 1453 section 3).
Both now send `Command::Zoom` anchored at the pointer in device pixels of the page area, and all
three count a gesture with `viewer_host::ZoomWheel` — ADR 1118's accumulator, moved out of
`quorra` so that a notch, a fraction of a line and fifty pixels of touchpad are one step in every
window. One limit seen and not chased: under `Xvfb`, `winit` reports one `xdotool` notch as two
lines, so `quorra` takes two steps where the other two take one; the binary before this change
does the same.

## 5. A frame the device refuses is said in the title, not only on the terminal

`CLAUDE.md` principle 2 keeps the processor drawing "a frame the graphics device refuses (a
coverage or budget refusal, not a swapchain state), reported out loud". `quorra` printed the
refusal on standard output and `quorra-confined` on standard error, and a person looking at the
window saw a correct page and nothing else. Both now put the sentence where each already reports
what a page could not draw — the title, which is `quorra`'s report channel (`retitle_incomplete`)
and `quorra-confined`'s only one — beside the terminal line, which stays: `App::on_the_processor`
in `quorra` (written by `surface.rs` when a landed frame's `fell_back` changes, so a page refused
on every frame is one title, not a storm of them) and `Host::on_the_processor` in
`quorra-confined` (set in `frame_landed` on a refusal, withdrawn by a device frame of a newly
taken screen that nothing refused). Not a dialogue: the page is right, and a card over it would
put a question in front of a reader who has nothing to answer. `quorra-gtk` and `quorra-qt` draw
no page through a graphics device, so there is no refusal for them to say. Driven:
`ContentStreamCycleType3insideType3.pdf` under `Xvfb` and lavapipe, whose frame needs 377 221 248
scene-derived bytes against a budget of 268 435 456 (`tools/drive-windows.sh`'s
`27-processor-fallback`).
