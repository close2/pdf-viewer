# A native host, then the C ABI

Status: **one of three done.** GTK4 landed in the four-hundred-and-eighth session
(`crates/viewer-gtk`, ADR 0244). Qt is next; the C ABI is last and still waits on it.
Priority: 30 — the largest single thing the project owes, and §0 was built for it
Code: `crates/viewer-gtk` (exists), new crates beside it

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
2. **Qt/KDE via `cxx-qt`.** Second, because it costs a C++ bridge and should not be the
   experiment that shapes the API. **It now has a Rust native host to be compared against** rather
   than only `viewer-ui`, which is what the order was for.
3. **`viewer-ffi` last**, and it is the **only** crate in the tree permitted `unsafe`. Every
   crate touching PDF bytes keeps `#![forbid(unsafe_code)]`; the FFI crate touches messages, not
   documents, so the compiler-enforced rule survives.

**Do not freeze a C ABI until two Rust consumers have shaken the API out.** One of the two now
exists and **it needed no new message** — ten sessions of hosts added five messages between them and
a whole native host added none. That is evidence for the vocabulary and not yet permission: the
second consumer is the one that has a bridge, and a bridge is where a message that is awkward to
carry shows up.

## What a host has to do that the two existing consumers do not — and what happened when one did

Each of these was written before any native host existed. What the GTK host found is beside it.

- **a platform tree view over `Query::Outline`, `Query::Layers` and `Query::Attachments`.**
  **Done, and the three answered enough.** `viewer_gtk::panel` turns all three into one `PanelRow`
  shape with no re-derivation and no reaching into `viewer-ui`, and `viewer_gtk::tree` puts that in
  a `GtkListView` over a `GtkTreeListModel`. §12.3.3's `/Count` sign decides which rows open,
  Table 99's `/Locked` makes a switch insensitive, and an attachment row carries the
  `/EmbeddedFiles` key `Command::Extract` names *and* Table 43's `/UF` a person is shown, which is
  why both cross. One asymmetry worth knowing: `Answer::Outline` borrows the viewer where the other
  two are owned, so a toolkit host clones it — no change owed, ADR 0244 §4.
- **real controls over `Query::Fields`.** **Done**: `GtkEntry`, `GtkTextView`, `GtkPasswordEntry`,
  `GtkCheckButton`, `GtkButton`, `GtkDropDown` and a `GtkListView` for §12.7.5.4's list box, 76 of
  them placed over `160F-2019.pdf`'s 67 fields, and typing into one produces
  `Edit::SetField`. **One gap found**: `Answer::Field` answers a password field with Table 231 bit
  14's bullets, so ADR 0201's read-the-value-back rule has an exception nothing in the vocabulary
  points at (ADR 0244 §3).
- **interactive chrome drawn in the platform's own colours.** **Done as far as GTK allows, and
  what it allows is the finding**: GTK 4.22 exposes **no accent colour at all** to application code
  — no symbol containing `accent` exists in `gtk4-sys`, and `@accent_bg_color` is a libadwaita CSS
  name. What a widget can be asked for is `gtk_widget_get_color`, the theme's own foreground, which
  follows light and dark; that is what the selection fill and §12.5.1's focus ring are drawn in.
  The argument for crossing as geometry survives intact — handing over pixels would have made even
  that impossible.
- **the two decisions a host owns**: which files a document may name (§12.7.6.4) and what to do
  when one asks for a password (§7.6.4.1). **Both done**, and the password one verified end to end
  under `Xvfb`: `issue6010_1.pdf` prompts, `abc` opens it.

## What is left

- **Qt/KDE via `cxx-qt`**, which is now the next item in the order.
- **`viewer-ffi`**, after it.
- **The page drawn without its widget appearances**, which is not this file's — `doc/todo/37` owns
  it — and which the GTK host turned from a prediction into a photograph. It is now the thing
  standing between a native host and a *good* one, and ADR 0244 records a second consequence the
  audit had not named: a platform control has a **minimum size** the document's rectangle can be
  smaller than, so a `GtkEntry` over a 15-pixel widget covers the page's own text around it.
- **`pdf_render::RasterFormat` is `#[non_exhaustive]` and crosses the boundary**, which breaks
  `viewer-core`'s own rule that a new variant should fail to compile in every consumer. The GTK
  host refuses an unknown format by name; the next host will have to do the same. ADR 0244 §2.

Adding `egui` buys a widget set for a large dependency and no architectural proof: winit + a GPU
*is* the unnative UI. The thing worth adding was the headless consumer, and it is there.
