# A native host, then the C ABI

Status: **two of three done, and the third may start.** GTK4 landed in the four-hundred-and-eighth
session (`crates/viewer-gtk`, ADR 0244); **Qt landed in the four-hundred-and-tenth**
(`crates/viewer-qt`, ADR 0246), and with it `crates/viewer-host` — the half of a native host that
turned out not to be a toolkit's. The C ABI is what is left, and **the condition it waited on is
met**. **This file absorbed `doc/todo/37` in the four-hundred-and-ninth**, whose one open decision
was taken (ADR 0245).
Priority: 30 — the largest single thing the project owes, and §0 was built for it
Code: `crates/viewer-gtk`, `crates/viewer-qt`, `crates/viewer-host` (all exist), `viewer-ffi` next

## The goal, stated by the owner

The viewer is to be **embeddable in native frameworks** — Win32/WinUI, AppKit, KDE/Qt, GTK — not
built on a cross-platform toolkit. `viewer-core` is that interface: `Command` in, `Event` out,
`Query` → `Answer` beside them, with no type from a windowing or graphics library anywhere in its
API (ADRs 0116 to 0121, and `doc/ui-boundary.md` — `doc/HANDOVER.md` §0's pointer — for the
vocabulary and the three pixel tiers).

## The order, and it is not negotiable

1. ~~**GTK4 via `gtk4-rs`.**~~ **Done in the four-hundred-and-eighth** (ADR 0244). Rust-safe with
   no C++ bridge, `#![forbid(unsafe_code)]` held in the host crate itself, `gtk4` named by that
   crate's manifest and no other, tier 1 because GTK4's public API admits no other tier.
2. ~~**Qt/KDE via `cxx-qt`.**~~ **Done in the four-hundred-and-tenth** (ADR 0246), and the order was
   right for the reason it was given: the bridge *is* where the awkwardness showed up, and none of
   it was the boundary's. `cxx` + `cxx-qt-build` in one manifest, `cxx-qt` itself declined,
   `#![deny(unsafe_code)]` with one exemption and a test on its position, tier 1 again.
3. **`viewer-ffi` last**, and it is the only crate in the tree that will *hand-write* `unsafe` at
   any scale. Every crate touching PDF bytes keeps `#![forbid(unsafe_code)]`; the FFI crate touches
   messages, not documents, so the compiler-enforced rule survives.

**"Do not freeze a C ABI until two Rust consumers have shaken the API out."** Both now exist, and
**neither added a message** — eleven messages in eleven rounds of hosts, and two whole native hosts
between them added none. That was the condition, and it is met.

## The ABI may be frozen, with three amendments to take first

Stated as a decision rather than as an opinion, because the file existed to reach it. **Three
things should change before the freeze, and the reason is the same for all three: a C ABI cannot
fail to compile in anybody.** Every one of them is a defect a Rust consumer survives by writing a
line it should not have to write.

1. **`pdf_render::RasterFormat` is `#[non_exhaustive]` and it crosses the boundary** — inside
   `Rendered::Raster` and `Answer::Frame`. `viewer-core`'s own rule is that nothing in its
   vocabulary is `#[non_exhaustive]`, because "a new `Event` should fail to compile in every
   consumer". Both native hosts refuse an unknown format by name (`PixelError::UnknownFormat`),
   which is trap 5 and is the best a Rust host can do; a C consumer has no such option. **This is
   the one that must be settled first**, and settling it is a decision about `pdf-render`'s
   stability guarantee rather than about hosts. ADRs 0244 §2, 0246.
2. **`Answer::Outline` borrows the viewer** where `Answer::Layers` and `Answer::Attachments` are
   owned. A Rust host clones it; a C ABI has to own it anyway, so the asymmetry becomes a shape the
   generated header has to explain. Worth knowing that it costs the two hosts differently: GTK
   clones lazily per expansion, Qt clones the whole tree because `QAbstractItemModel` admits no
   laziness. ADRs 0244 §4, 0246.
3. **`Answer::Field`'s value for a password field cannot be read back**, and nothing in the
   vocabulary says so. Table 231 bit 14's field answers with bullets, so ADR 0201's
   read-the-value-back rule has an exception discoverable only by reading two doc comments and
   noticing they interact. Both hosts ship the exception; a C consumer would ship the bug. The fix
   is one sentence in a doc comment. ADR 0244 §3.

## What a host has to do that the two original consumers do not — and what both hosts found

Each of these was written before any native host existed. What the two found is beside it.

- **a platform tree view over `Query::Outline`, `Query::Layers` and `Query::Attachments`.**
  **Done twice, and the three answered enough both times.** `viewer_host::panel` turns all three
  into one `PanelRow` shape with no re-derivation and no reaching into `viewer-ui`; `viewer-gtk`
  puts that in a `GtkListView` over a `GtkTreeListModel` and `viewer-qt` in a `QTreeView` over a
  `QAbstractItemModel`. The two models are the sharpest difference the round found — GTK pulls a
  subtree when a person opens it, Qt must answer for every node at any moment — and it cost
  nothing: 990 rows build in 3.66 ms against 3.34 ms for fourteen, so the laziness is worth nothing
  at outline sizes documents have (ADR 0246).
- **real controls over `Query::Fields`.** **Done twice**, 76 controls over `160F-2019.pdf`'s 67
  fields in both, from the same `viewer_host::form` decision. One clause Qt obeys that GTK cannot:
  Table 233 bit 19's editable combo box, because `QComboBox` is editable and `GtkDropDown` is not.
- **interactive chrome drawn in the platform's own colours.** **Done twice, and the two platforms
  gave different answers, which is the finding.** GTK 4.22 exposes no accent colour to application
  code at all, so `viewer-gtk` draws in `gtk_widget_get_color`; Qt has `QPalette::Highlight` and
  `QPalette::Accent` since 6.6, and `viewer-qt` draws the selection in `#3daee9` — KDE's own — with
  the pixel sampled back out of the window to prove it. The argument for crossing as geometry pays
  rather than merely surviving.
- **the two decisions a host owns**: which files a document may name (§12.7.6.4) and what to do when
  one asks for a password (§7.6.4.1). **Both done twice**, and the file policy is now *one*
  implementation in `viewer-host` rather than two that could drift. Both passwords verified end to
  end under `Xvfb`: `issue6010_1.pdf` prompts, `abc` opens it.

## What is left

- **`viewer-ffi`**, with the three amendments above taken first.
- **The scale a native form host draws the page at.** ADR 0245 left this as the third decision, and
  the second host settled the half of it that was in doubt: it is **not** a GTK theme's accident.
  Qt places **13 of 76 controls wider than their `/Rect` (worst +66 on 18 px) and all 76 taller
  (worst +20 on 14 px)** where GTK places 11 wider and all 76 taller. *Every* control is taller
  than its rectangle on both toolkits, so a platform control's minimum size is a property of
  platform controls. **Nothing new may be needed at all**: a host that zooms until the worst `/Rect`
  fits has answered it with the messages that exist, and `Query::Fields` already gives it every
  widget's rectangle in device pixels. Establishing that is a round's work and neither host has
  done it.
- **Table 229 bit 26's `RadiosInUnison` crosses and is not obeyed** (from `doc/todo/37`). Turning on
  every button of a set that shares an on state is a decision for whatever handles the press, and
  both hosts have the flag rather than the behaviour.
- **§12.7.5.4's list box is the one place the boundary genuinely limits a host**, and the second
  host is what established it. `viewer-gtk` offers one selection because `GtkListView` was set up
  that way; `viewer-qt` asks `QListWidget` for `SingleSelection` *deliberately*, because
  `QListWidget` does multiple selection natively and **`Edit::SetField` carries one value**. Table
  233 bit 22 permits several. Offering the control and sending one of what a person chose would be
  worse than not offering it. This is a message-shaped gap — the only one two native hosts found —
  and it belongs in the same round as the ABI amendments above.
- **§12.7.5.4's list box still draws nothing on the page**, and says so: the clause states which
  items are selected and states no highlight, so `variable_text` refuses it. A host with the items
  and the selection draws a real list — which is the point — but a page with a list box on it is
  still light, and the report is what says so.

## Two hosts, and what turned out not to be a toolkit's

`crates/viewer-host` exists since the four-hundred-and-tenth because the second host wanted four of
`viewer-gtk`'s eight modules **unchanged**: the three panel answers as one row shape, §12.7.5's field
as the control it is, §12.7.6.4's file policy, and the launch timeline. None of them named a GTK
type. `viewer-gtk`'s public interface is now `Host` and `HostError` and nothing else, which is the
sharpest way to say what a second host proved: **a native host on this boundary is mostly not
toolkit code.** It is deliberately *not* in `viewer-core` — a mapping from three answers into one
row shape is a convenience for whoever draws a tree, not a statement about a document (ADR 0246).

Adding `egui` buys a widget set for a large dependency and no architectural proof: winit + a GPU
*is* the unnative UI. The thing worth adding was the headless consumer, and it is there.
