# A native host, then the C ABI

Status: not started. The interface it needs exists and has two consumers.
Priority: 30 — the largest single thing the project owes, and §0 was built for it
Code: new crates beside `crates/viewer-core`

## The goal, stated by the owner

The viewer is to be **embeddable in native frameworks** — Win32/WinUI, AppKit, KDE/Qt, GTK — not
built on a cross-platform toolkit. `viewer-core` is that interface: `Command` in, `Event` out,
`Query` → `Answer` beside them, with no type from a windowing or graphics library anywhere in its
API (ADRs 0116 to 0121, and `doc/HANDOVER.md` §0 for the vocabulary and the three pixel tiers).

## The order, and it is not negotiable

1. **GTK4 via `gtk4-rs`.** Rust-safe, no C++ bridge, and it is the development platform.
2. **Qt/KDE via `cxx-qt`.** Second, because it costs a C++ bridge and should not be the
   experiment that shapes the API.
3. **`viewer-ffi` last**, and it is the **only** crate in the tree permitted `unsafe`. Every
   crate touching PDF bytes keeps `#![forbid(unsafe_code)]`; the FFI crate touches messages, not
   documents, so the compiler-enforced rule survives.

**Do not freeze a C ABI until two Rust consumers have shaken the API out.** The vocabulary
roughly doubles with selection and editing, and a frozen ABI is the one mistake that cannot be
taken back.

## What a host has to do that the two existing consumers do not

`viewer-ui` (winit + a GPU) and `viewer-core/tests/headless.rs` (no display at all) are tier 2
and tier 1. Neither can prove the interface alone — one is a toolkit, the other is not a program
— and together they are why the vocabulary is worth trusting. What a *native* host adds:

- **a platform tree view over `Query::Outline`, `Query::Layers` and `Query::Attachments`**,
  rather than `viewer_ui::chrome`'s own drawn sidebar. The chrome exists to prove the queries
  answer enough; a native host is the proof they answer enough for somebody else's widgets.
- **interactive chrome drawn in the platform's own colours.** Selection highlights, an
  in-progress rubber band, a caret, a focus ring — these cross as geometry rather than pixels
  precisely so a host can draw them in macOS's selection colour, KDE's accent, the Windows
  highlight brush. That is most of what makes an embedded view feel native.
- **the two decisions a host owns**: which files a document may name (§12.7.6.4) and what to do
  when one asks for a password (§7.6.4.1).

Adding `egui` buys a widget set for a large dependency and no architectural proof: winit + a GPU
*is* the unnative UI. The thing worth adding was the headless consumer, and it is there.
