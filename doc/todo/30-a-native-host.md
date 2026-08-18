# A native host, then the C ABI

Status: **all three built.** GTK4 in the four-hundred-and-eighth session (`crates/viewer-gtk`,
ADR 0244); Qt in the four-hundred-and-tenth (`crates/viewer-qt`, ADR 0246), and with it
`crates/viewer-host`; **the C ABI in the four-hundred-and-eleventh** (`crates/viewer-ffi`,
ADR 0247), with its three amendments taken first. **This file absorbed `doc/todo/37` in the
four-hundred-and-ninth**, whose one open decision was taken (ADR 0245). **And its remaining surface
was taken in the five-hundred-and-eleventh** (ADR 0346): the ABI's entry points are the whole
vocabulary now, Table 229 bit 26 is obeyed, and ADR 0245's scale question is answered with the
messages that already existed.
Priority: 30 — what is left is *surface* rather than architecture, and the file says which
Code: `crates/viewer-gtk`, `crates/viewer-qt`, `crates/viewer-host`, `crates/viewer-ffi`

## The goal, stated by the owner

The viewer is to be **embeddable in native frameworks** — Win32/WinUI, AppKit, KDE/Qt, GTK — not
built on a cross-platform toolkit. `viewer-core` is that interface: `Command` in, `Event` out,
`Query` → `Answer` beside them, with no type from a windowing or graphics library anywhere in its
API (ADRs 0116 to 0121, and `doc/ui-boundary.md` for the
vocabulary and the three pixel tiers).

## The order, and it was not negotiable

1. ~~**GTK4 via `gtk4-rs`.**~~ **Done in the four-hundred-and-eighth** (ADR 0244).
2. ~~**Qt/KDE via `cxx-qt`.**~~ **Done in the four-hundred-and-tenth** (ADR 0246), and the order was
   right for the reason it was given: the bridge *is* where the awkwardness showed up, and none of
   it was the boundary's.
3. ~~**`viewer-ffi` last.**~~ **Done in the four-hundred-and-eleventh** (ADR 0247). *"Do not freeze
   a C ABI until two Rust consumers have shaken the API out"* was met, and the three amendments the
   two hosts named were taken before a line of the ABI was written.

**The vocabulary needed no new message for a third host running**, and it still needs none: eleven
messages in eleven rounds of hosts, and three whole hosts — one of them in another language — added
no *message* between them. **What they did find was a variant carrying too little**, which is a
different thing and has its own mechanism: `Edit::SetField`'s value changed shape in the
four-hundred-and-twelfth so that §12.7.5.4's list box could say which items are selected (ADR 0248),
and every consumer failed to compile until it said what it does. A message added is a channel that
did not exist; a variant changed is a channel that was too narrow, and only the second is something
`#[non_exhaustive]` would have hidden.

## What the three amendments cost, because it was not what this file predicted

All three are in ADR 0247. Two are worth knowing here because the file was **wrong about one of
them**:

- `pdf_render::RasterFormat` lost `#[non_exhaustive]`, and four consumers lost a runtime refusal for
  a condition a build now catches.
- `Answer::Outline` is owned. All five consumers cloned it anyway; measured at 481 ns for 14 rows
  and 80.7 µs for 988.
- **`Answer::Field`'s password value was not "one sentence in a doc comment", as this file said.** It
  was a live bug: `viewer-ui` read the value back after every keystroke (ADR 0201) and a password
  field answers with bullets, so typing into one produced `Edit::SetField` with the bullets and a
  character appended. The variant changed shape — `Option<pdf_model::view::ShownValue>`, characters
  and `obscured` from one reading — and every consumer failed to compile, which is what that is for.
  Reading the clause for it found a **second** sentence nobody had read: Table 231 bit 14's NOTE
  forbids storing the value in the file, and `ViewState::save` was storing it.

## What is left, and none of it is architecture

**Every item below is a function to add to the C ABI, a widget to place, or a clause to obey.** No
new message, and no new decision about the boundary. **Four of the five are closed in the
five-hundred-and-eleventh** (ADR 0346) and **the fifth in the five-hundred-and-seventy-first**
(ADR 0407), so what remains of this file is one tail — Qt's measurement being on the far side of a
`cxx` bridge — plus the standing note about where the `unsafe` is.

- ~~**The ABI is 43 entry points and not the whole vocabulary.**~~ **Closed in the
  five-hundred-and-eleventh** (ADR 0346): 43 → **111**, and the list this entry carried is all of
  it — `Command::Pointer` and `Command::Select` with the selection, the caret and §12.5.6.6's drag;
  `Query::Fields` and all four `Edit`s; `Command::Save` and `Command::Extract` with a **byte**
  accessor apiece, because a file is not text and the NUL idiom would cut one at its first zero
  byte; §8.11.4.3's layers and §7.11.4's files as a second flattened panel; §12.4.4's clock and its
  transitions; and the three policy values. **`PDFV_EVENT_KIND_COUNT` is 16 before and after**,
  which is the third demonstration in three rounds of what the shape was chosen for: a `Command` is
  a symbol, a `Query` is a symbol, and only an `Event` is a number. Two enumerations are now
  *answered with* — `ControlKind` and `RowKind` — and each has a count and a name of its own rather
  than a place in `pdfv_abi_check`, because an event arrives unasked and a control kind is the
  answer to a call the caller wrote. **And a `#define` had been missing since the count last moved**:
  `PDFV_EVENT_SEARCHED` was never added in the four-hundred-and-fourteenth, so a C caller wrote `15`
  by hand — `header_and_library_agree.rs` compares the header against a *hand-written* map, and a
  constant absent from both sides agrees with itself.
- ~~**The scale a native form host draws the page at.**~~ **Answered in the
  five-hundred-and-eleventh, and it needed no message** (ADR 0346). `viewer_host::ControlFit` is the
  one piece that did not exist: a control's minimum does not change with the page's magnification
  and its `/Rect` does, in proportion, so the magnification at which everything fits is the current
  one times the worst ratio of minimum to asked. `Query::Fields` gives the rectangles,
  the toolkit gives the minimums, `Zoom::Scale` applies the answer. Driven under `Xvfb` on
  `160F-2019.pdf`: `11 of 76 control(s) wider than their /Rect (worst +85 on 120 px), 76 taller
  (worst +22 on 12 px); every control fits at 3.278`, and after `w` sends it, `0 of 76 … 0 taller`.
  **Two things are left of it, and both are small.** `viewer-qt` still measures in `cpp/window.cpp`,
  so feeding the shared arithmetic means carrying the `(asked, minimum)` pairs across the `cxx`
  bridge — a bridge change rather than a decision. And *when* to apply it is deliberately not
  decided: a viewer that magnified a page by itself because a form is on it would be answering a
  question nobody asked, so `w` offers it and nothing takes it.
- ~~**Table 229 bit 26's `RadiosInUnison` crosses and is not obeyed.**~~ **Obeyed since the
  five-hundred-and-eleventh, and this entry was wrong about it in both directions** (ADR 0346). The
  half it describes — turning on every button of a set that shares an on state — was already
  happening, by construction and without the flag ever being read: `/V` is a name and a widget is on
  when its `/AP /N` holds it. What was *not* obeyed is §12.7.5.2.3's requirement for the flag being
  **clear**, "at most one radio button in a field shall be set at a time", and this tree turned them
  all on. The sentence is in the **check box** subclause, attached to `/Opt`, which is why a round
  reading §12.7.5.2.4 for a radio button's flag would not have found it. Which button stays on is a
  documented choice — the first in `/Kids` — because the clause states none and a file that gave two
  buttons one name cannot say. No corpus witness in either direction over 1293 documents
  (`field_flag_census`, which counts bit 26 by field type since that round).
- ~~**§12.7.5.4's list box is the one place the boundary genuinely limits a host.**~~ **Closed in
  the four-hundred-and-twelfth** (ADR 0248), and it was the only thing on this list that changed
  `viewer-core`. `Edit::SetField`'s value is `pdf_model::view::Entered` now — characters,
  §12.7.5.4's chosen options by index, or a clear — so `viewer-gtk` builds a `GtkMultiSelection` and
  `viewer-qt` an `ExtendedSelection` where Table 233 bit 22 is set, and each writes `/V` in both the
  shapes the clause states with Table 234's `/I` beside it. Driven under `Xvfb` on `issue17492.pdf`
  in both hosts, which wrote **byte-identical** files. The variant changed and every consumer failed
  to compile, which is the shape ADRs 0166, 0167 and 0247 established and the fourth time it has
  been used.
- ~~**§12.7.5.4's list box still draws nothing on the page.**~~ **Closed in the
  five-hundred-and-seventy-first** (ADR 0407), and the entry was wrong in the way a refusal is
  usually wrong: the clause states no highlight for the *selection* and states the options
  outright — "each of which shall be represented by a text string that shall be displayed on the
  screen", in the array's own order by Table 233 bit 20, from Table 234's `/TI`. A mark added over
  an item that is drawn either way may not take the item down with it (ADR 0106's test), so the
  options are drawn and the unmarked selection is reported. What is left on the page is the mark,
  which no clause states and which this tree will not invent; a host draws it from
  `ChoiceControl::selected`, as it draws a text selection, in its own colour.

## Three hosts, and what turned out not to be a toolkit's

`crates/viewer-host` exists since the four-hundred-and-tenth because the second host wanted four of
`viewer-gtk`'s eight modules **unchanged**: the three panel answers as one row shape, §12.7.5's
field as the control it is, §12.7.6.4's file policy, and the launch timeline. None of them named a
GTK type. **The third host takes `panel` too**, which is the finding tested rather than repeated: a
C ABI is a native host, and a native host on this boundary is mostly not toolkit code. It is
deliberately *not* in `viewer-core` — a mapping from three answers into one row shape is a
convenience for whoever draws a tree, not a statement about a document (ADR 0246).

Adding `egui` buys a widget set for a large dependency and no architectural proof: winit + a GPU
*is* the unnative UI. The thing worth adding was the headless consumer, and it is there.

## Where the `unsafe` is, now that both crates that have any exist

`doc/todo/30` used to say `viewer-ffi` would be "the **only** crate in the tree permitted `unsafe`".
Two crates lift `deny(unsafe_code)` and no more: `viewer-qt`, for `#[cxx::bridge]`'s expansion, with
**one** hand-written token; and `viewer-ffi`, whose `src/abi.rs` holds one lint lift, 111
`#[unsafe(no_mangle)]` attributes and 103 signatures, and **no `unsafe` block at all**. Each has a
test that reads its own sources back, and `viewer-ffi`'s additionally asserts that `pdf-syntax`,
`pdf-model`, `pdf-font`, `pdf-render`, `render-cpu`, `viewer-core` and `viewer-host` still hold
`#![forbid(unsafe_code)]` — the compiler-enforced rule this file promised would survive, checked
rather than promised. A third name appearing in either list is a change to a rule the project owner
stated and belongs in an ADR.
