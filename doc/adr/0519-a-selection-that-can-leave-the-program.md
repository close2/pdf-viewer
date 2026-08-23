# ADR 0519 — A selection that can leave the program

Status: accepted, 2026-08-23. Session 683, the second round on the project owner's *"we should
start investing time into the UI (and its API for the native versions)"* and the first item of the
ordering ADR 0509 wrote. **All four consumers can now put a selection where another application can
take it**, and none of them needed a message to do it. Adds `viewer_host::copying`, one dependency
priced in the workspace manifest, one C entry point, and the key `c` in the two native hosts.

## 1. The item, and the claim it was taken on

`doc/todo/30`'s ranked list, first place: *"All three windowed hosts draw a selection; none can give
it to another application."* `viewer-ui` kept an in-process `String` whose own comment said the
platform's clipboard "belongs to the platform" and that a native host "owns that end by
construction" — and neither native host had taken it. The claim attached to the item was that it
needs **no new message**, on the evidence that `Query::Selection` answers with the text and
`Query::LogicalSelection` with §14.8.2.5's order.

**The claim is true, and it was checked rather than assumed.** Both queries exist, both answer, and
nothing in this round added a `Command`, an `Event`, a `Query` or an `Answer`. That is the seventh
time since the six-hundred-and-seventh session that a feature landing in every host has needed no
channel, and — unlike the six before it — this one is a capability that reaches *outside* the
program, which is the strongest test that vocabulary has had.

**What the item was wrong about is the C ABI.** ADR 0509 wrote *"The C ABI has `pdfv_selection_text`
already"*, and it does — answering in **page content order**, which is what a selection is measured
in and what the quadrilaterals are drawn from. A C caller could not ask for the other order at all:
`Query::LogicalSelection` was one of the twelve `tools/state.sh hosts` names, so the fourth consumer
could copy text and could not copy it *right*. That is a fifth of item 5 taken here, because item 1
is not finished without it.

## 2. What §14.8.2.5 makes a copy choose

The clause states both orders and requires neither to be the other:

> Page content order shall be defined by the sequencing of graphics objects within a page's content
> stream.

> Logical content order -the ordering for semantic purposes -shall be defined by a depth-first
> traversal of the document's logical structure

and then only recommends that they coincide — "[t]he page content order in a tagged PDF *should*
coincide with the logical content order". Measured over the corpus, 5 of 77 tagged first pages
disagree, among them a tax form whose logical order starts with its title and whose stream starts
with a margin note.

So a copy is a *decision* and not a read: page content order is the right answer for anything drawn,
because the shapes are in it, and logical content order is the right answer for anything pasted.
`Query::LogicalSelection` answers `Answer::None` where the document states no structure tree **and**
where the tree does not reach every byte of the selection — deliberately, because a copy that
silently dropped the part a tree missed would be worse than one that hands back the order it had.

**That decision is one function, in `viewer-host`**, and the argument is `Presenting`'s and
`Clock`'s verbatim (ADRs 0470, 0473): four consumers making the same choice in four places is where
two of them stop agreeing. `viewer_host::copying::copied` takes the two answers and produces the
text with a `ContentOrder` beside it; the three windowed hosts print which order they got, because a
person who copied a two-column page and got the columns interleaved has been told nothing at all.
`viewer-ui` lost a private function doing this, which is what distinguishes it from a fourth copy.

## 3. Four platforms, and the layering that keeps `viewer-core` out of it

A clipboard is a *platform* surface, and the four ends are four different kinds of thing:

- **`viewer-gtk`** asks the drawing area for its `gdk::Clipboard` and calls `set_text`. One line,
  nothing to keep, nothing to bring up: a GTK clipboard is a property of the display the widget is
  on.
- **`viewer-qt`** cannot do that, and the reason is this crate's own shape rather than Qt's. Its
  bridge documentation states that **C++ owns the `Host` for the life of `QApplication::exec` and
  Rust never calls a Qt object** — which is what keeps the crate to one hand-written `unsafe` token
  with a test on its position. So a copy is not Rust reaching for `QClipboard`: `QtUpdate` gains a
  `clipboard` flag and `take_clipboard` beside it, and `window.cpp` calls
  `QGuiApplication::clipboard()->setText`. That is exactly the shape the `window` flag already has
  (ADR 0470), and it was chosen rather than adding a second declaration to the `unsafe extern "C++"`
  block.
- **`viewer-ffi`** has no platform at all and must not guess one. What a C caller needs is the
  *answer* — `pdfv_selection_copy_text` hands back the characters in the order those three hosts
  would use, with `PDFV_ORDER_LOGICAL` or `PDFV_ORDER_PAGE_CONTENT` in an out-parameter so a caller
  that disagrees can still tell. Handing over both strings and letting the caller pick was the
  alternative and is the worse one: it would make a fifth consumer re-derive a reading of the
  standard this tree already states. `PDFV_NO_ANSWER` where nothing is selected, because a copy with
  nothing to copy must not empty somebody's clipboard.
- **`viewer-ui`** is the one that costs something, and §4 is about that.

`viewer-core` learned nothing. It has no clipboard command, it has no notion of a platform, and the
two questions it already answered are the whole of what crossed.

## 4. The one dependency, and what was measured before taking it

`winit` offers no clipboard, and that is deliberate rather than an omission: on X11 a clipboard is
not a property of a window but a **selection owner** — a service that has to answer
`SelectionRequest` for as long as the program lives — so it is not a windowing library's job. The
tier-2 host is therefore the only consumer here that cannot reach the platform through something it
already links.

`arboard` 3.6.1, `MIT OR Apache-2.0`, and **on this platform it is one compiled package**. `cargo
add` locks four — `arboard`, `clipboard-win`, `error-code`, `objc2-app-kit` — and three of those are
`cfg`-gated to Windows and macOS; its Linux dependencies are `x11rb`, `parking_lot`,
`percent-encoding` and `log`, every one of which was already in this lockfile through `winit` and
`softbuffer`. That is the measurement that decided it, and it is the number `doc/stack.md`'s own
lesson asks for: a sentence about somebody else's crate is a prediction, and a prediction is
re-measured at the moment of spending.

**Two of its features are off and both refusals have a price written down.**

- `image-data` is the *default* feature and brings the whole `image` crate so that a raster can go on
  the clipboard. What leaves this program is §14.8.2.5's text; a page copied as a picture is a
  feature nobody asked for.
- `wayland-data-control` adds six packages — `wl-clipboard-rs`, `tree_magic_mini`, `nom`,
  `petgraph`, `fixedbitset`, `os_pipe`, a MIME sniffer and a graph library for the business of
  handing over a string — and what it speaks is `zwlr_data_control`, a wlroots protocol rather than a
  Wayland one. Without it `arboard` uses its X11 backend, which reaches a Wayland session through
  XWayland. **Where there is no XWayland the copy is refused by name** (`ClipboardError::Unavailable`,
  printed), which is trap 5 rather than a silent nothing. Six packages for the compositors that offer
  the protocol is the wrong price, and the entry is worth revisiting if this program is ever asked to
  run without XWayland.

**Nothing about it is on the launch path.** `viewer_ui::clipboard::Clipboard::new` is a `const fn`
that connects to nothing; the connection is made inside the first copy. `Clipboard::connected` is
public so that this is a property a test asserts rather than a promise a comment makes, and
`a_clipboard_connects_to_nothing_until_somebody_copies` is that test.

The in-process `String` stays, beside the platform's rather than instead of it: it is what Ctrl + V
pastes into a §12.7.4.3 field, and `arboard`'s X11 backend *reads* a clipboard by asking whoever owns
the selection and waiting for them — so a paste that went to the platform would put another program's
response time inside a keystroke.

## 5. What the C ABI's new enumeration deliberately lacks

`OrderKind` has no `pdfv_order_kind_name` and no `PDFV_ORDER_KIND_COUNT`, unlike `ControlKind` and
`RowKind`. Those exist for an enumeration that may **grow** under a caller compiled against an older
header. §14.8.2.5.1 defines exactly two content orders, and a third would be a change to the standard
rather than to this build — so a pair of functions guarding that case is machinery pretending to be
caution. `LayoutKind` set the same precedent for Table 29's six values, which are also the standard's
own closed list.

## 6. Where the asymmetry that remains is, and it is named rather than left silent

Two of `doc/todo/30`'s consumers can copy text **out of a form field** and two cannot, and this is a
platform difference rather than a gap in this tree: `viewer-gtk` and `viewer-qt` place a real
`GtkEntry` and a real `QLineEdit`, so Ctrl + C inside one is the toolkit's own binding and reaches
the session's clipboard without this program being involved. `viewer-ui` draws its own field and its
Ctrl + C now goes to the platform through the same call the page's copy uses — Table 231 bit 14's
password field never reaches it, because `aim_at_field` refuses the keyboard to one, so nothing this
puts on a clipboard is a value the document said to obscure.

The C ABI has no field-level copy and needs none: a caller that placed its own controls owns their
keyboard entirely, and `pdfv_field_text` already answers with the value.
