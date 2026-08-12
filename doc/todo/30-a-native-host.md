# A native host, then the C ABI

Status: **all three built.** GTK4 in the four-hundred-and-eighth session (`crates/viewer-gtk`,
ADR 0244); Qt in the four-hundred-and-tenth (`crates/viewer-qt`, ADR 0246), and with it
`crates/viewer-host`; **the C ABI in the four-hundred-and-eleventh** (`crates/viewer-ffi`,
ADR 0247), with its three amendments taken first. **This file absorbed `doc/todo/37` in the
four-hundred-and-ninth**, whose one open decision was taken (ADR 0245).
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
new message, and no new decision about the boundary.

- **The ABI is 43 entry points and not the whole vocabulary**, and two rounds have now shown what
  that costs rather than predicting it. The four-hundred-and-twelfth: `Edit::SetField`'s value
  changed shape, five Rust consumers failed to compile, and this crate did not notice because
  `Command::Edit` is on the list below — `PDFV_EVENT_KIND_COUNT` **15** before and after. The
  four-hundred-and-fourteenth is the other half: a document-wide search added a *kind*, so the count
  moved **15 → 16** and an old caller's `pdfv_abi_check` refuses at startup naming the number rather
  than meeting a message it has no arm for. Four entry points came with it — `pdfv_find_start`,
  `pdfv_find_continue`, `pdfv_find_stop`, `pdfv_event_searched` (ADR 0250). What a C caller cannot
  do yet:
  `Command::Pointer` and `Command::Select` (and therefore selection, the caret and §12.5.6.6's
  drag), `Query::Fields` and `Command::Edit` (the form), `Command::Save` and `Command::Extract`
  (and the `Saved`/`Extracted` events' bytes, which want a byte-buffer accessor rather than a
  string one), `Query::Layers` and `Query::Attachments` (the other two panels, which are already
  `viewer_host::panel` rows and would flatten exactly as the outline does), `Command::Tick` and
  §12.4.4's transitions, `Command::Restrict` and `Command::Delegate`. Each is a *symbol*, and a
  symbol added later costs a compiled caller nothing — which is the property the shape was chosen
  for and the reason stopping at 39 is honest rather than half-built.
- **The scale a native form host draws the page at.** ADR 0245 left this as the third decision and
  the second host settled the half that was in doubt: it is **not** a GTK theme's accident. Qt
  places 13 of 76 controls wider than their `/Rect` (worst +66 on 18 px) and all 76 taller (worst
  +20 on 14 px) where GTK places 11 wider and all 76 taller. *Every* control is taller than its
  rectangle on both toolkits, so a platform control's minimum size is a property of platform
  controls. **Nothing new may be needed at all**: a host that zooms until the worst `/Rect` fits has
  answered it with the messages that exist, and `Query::Fields` already gives it every widget's
  rectangle in device pixels. Establishing that is a round's work and no host has done it.
- **Table 229 bit 26's `RadiosInUnison` crosses and is not obeyed** (from `doc/todo/37`). Turning on
  every button of a set that shares an on state is a decision for whatever handles the press, and
  all the hosts have the flag rather than the behaviour.
- ~~**§12.7.5.4's list box is the one place the boundary genuinely limits a host.**~~ **Closed in
  the four-hundred-and-twelfth** (ADR 0248), and it was the only thing on this list that changed
  `viewer-core`. `Edit::SetField`'s value is `pdf_model::view::Entered` now — characters,
  §12.7.5.4's chosen options by index, or a clear — so `viewer-gtk` builds a `GtkMultiSelection` and
  `viewer-qt` an `ExtendedSelection` where Table 233 bit 22 is set, and each writes `/V` in both the
  shapes the clause states with Table 234's `/I` beside it. Driven under `Xvfb` on `issue17492.pdf`
  in both hosts, which wrote **byte-identical** files. The variant changed and every consumer failed
  to compile, which is the shape ADRs 0166, 0167 and 0247 established and the fourth time it has
  been used.
- **§12.7.5.4's list box still draws nothing on the page**, and says so: the clause states which
  items are selected and states no highlight, so `variable_text` refuses it. A host with the items
  and the selection draws a real list — which is the point — but a page with a list box on it is
  still light, and the report is what says so.

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
**one** hand-written token; and `viewer-ffi`, whose `src/abi.rs` holds one lint lift, 43
`#[unsafe(no_mangle)]` attributes and 39 signatures, and **no `unsafe` block at all**. Each has a
test that reads its own sources back, and `viewer-ffi`'s additionally asserts that `pdf-syntax`,
`pdf-model`, `pdf-font`, `pdf-render`, `render-cpu`, `viewer-core` and `viewer-host` still hold
`#![forbid(unsafe_code)]` — the compiler-enforced rule this file promised would survive, checked
rather than promised. A third name appearing in either list is a change to a rule the project owner
stated and belongs in an ADR.
