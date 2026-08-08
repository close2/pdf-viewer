# ADR 0244 — The first host whose widgets are not ours, and the one picture it draws twice

Status: accepted, 2026-08-08 (session 408).

## Context

`doc/todo/30` is the largest single thing this project owed and its order is the project owner's,
stated as non-negotiable: **GTK4 first**, because `gtk4-rs` is a safe Rust binding with no C++
bridge and GTK is the development platform; **Qt second**, because `cxx-qt` costs a C++ bridge and
should not be the experiment that shapes the interface; **`viewer-ffi` last**, and *"do not freeze a
C ABI until two Rust consumers have shaken the API out"*.

So this round is the first of those two consumers, and its product is **what the boundary turns out
to be missing** rather than a polished application. The two consumers that already existed cannot
answer that question between them: `viewer-ui` is winit and a graphics device drawing its own
sidebar, its own selection and its own form, and `viewer-core/tests/headless.rs` is not a program.
Neither has ever handed one of this crate's answers to a widget somebody else wrote.

## Decision 1 — `gtk4-rs` 0.11.4, confined to one crate

**Taken.** `crates/viewer-gtk`, one library and one binary (`pdf-viewer-gtk`), the only manifest in
the tree that names a toolkit.

In ADR 0186's and ADR 0214's shape, which is what `doc/third-party-data.md` records:

| | |
|---|---|
| what it is | the GNOME project's own binding: safe Rust over GTK 4, generated from GIR |
| licence | MIT for every one of the 41 crates it brings; `cargo deny check` is clean on all four checks with it in the graph |
| what reaches a shipped binary | `viewer-gtk` and its binary alone. `viewer-ui`, `viewer-confined` and every crate that touches PDF bytes are unchanged and name none of it |
| `unsafe` | **none in this tree**. `viewer-gtk` keeps `#![forbid(unsafe_code)]`, and so does its binary. The binding's `unsafe` is inside `gtk4-sys` and `glib`; a host written against the safe layer needs none of its own — which is precisely the property `doc/todo/30` chose GTK4 *for*, and precisely the property `cxx-qt` will not have |
| the feature floor | `v4_10`, deliberately: it is what makes `gtk::ColumnView`, `gtk::DropDown`, `gtk::TreeListModel` and `gtk_widget_get_color` available without reaching for the widgets GTK deprecated in 4.10 (`GtkTreeView`, `GtkComboBoxText`), which would compile with a deprecation warning and this workspace turns warnings into errors |
| what it costs elsewhere | nothing on the two cross-target checks, which name their packages with `-p` — see "cross-compilation" below |

**The alternative considered and rejected** was writing the first native host against Qt through
`cxx-qt`. It is the owner's stated order and the reason is the row above: a C++ bridge in the
experiment that shapes the interface would confuse "the boundary is missing something" with "the
bridge is missing something".

## Decision 2 — tier 1, and GTK4 offers no other

**Taken.** `doc/ui-boundary.md` names three pixel tiers. This host takes tier 1: `render-cpu`
rasterises on the host's own thread, the `Raster` comes back through `Rendered::Raster`, and
`Query::Frame`'s pixels become a `gdk::MemoryTexture` that GSK uploads.

The argument is not "tier 1 is the honest first step", though it is. It is that **GTK4's public API
admits no other tier**:

- **Tier 2** wants a raw window handle to drive `wgpu` against. GTK4 gives a widget no native
  surface of its own — there is one `GdkSurface` per toplevel and drawing into it directly bypasses
  GSK's compositing, so the page could not be inside a `GtkPaned` beside a sidebar.
- **Tier 3** wants the host's own device. GTK4 exposes neither its Vulkan device nor its GL context
  to application code; `GskRenderer` owns them and hands out no handle.

**And the copy is small.** `Raster` is row-major RGBA with straight alpha and no row padding, which
is `GDK_MEMORY_R8G8B8A8` exactly — so there is no conversion at all, only a `memcpy` into a
`glib::Bytes`. Measured by the host's own `--trace=frames`, on a 689×975 page:

> `2687100 bytes into a texture in 888.919µs` — five runs: 605.7, 692.3, 731.8, 888.9, 952.4 µs
> and one at 1.14 ms, so **2.69 MB in about 0.8 ms, ≈3.2 GB/s**.

`doc/ui-boundary.md`'s estimate was "1920×1080 RGBA is 8.3 MB, so full-window repaint at 60 fps is
~500 MB/s of memcpy — a few percent of a core". This measurement is the first taken through a real
toolkit and it agrees with the estimate.

**What tier 1 costs that is not the copy**, and it is the honest half: `CLAUDE.md`'s rule that page
one goes to the graphics device is about *this project's* window. A GTK host rasterises page one on
the processor and GSK composites the result on the device. On this machine, under `Xvfb` with no
hardware driver, that is the faster of the two:

| | first frame, five runs | median |
|---|---|---|
| `pdf-viewer-gtk` (GTK4, tier 1, `render-cpu`) | 92.6, 95.6, 96.5, 98.9, 104.3 ms | **96.5 ms** |
| `pdf-viewer` (winit, tier 2, vello on lavapipe) | 116.0, 117.5, 129.9 ms | **117.5 ms** |

That comparison is **not** evidence that tier 1 is faster, and saying so is the point: lavapipe
under `Xvfb` is the case where GPU bring-up is at its worst and CPU rasterisation is not
disadvantaged at all. On the real adapter the ranking may reverse. What the numbers do establish is
that a native host on tier 1 is not *slow*, and that its launch path is measured rather than assumed.

## What the host does

- A `GtkApplicationWindow` with a header bar, a `GtkPaned`, and a `GtkNotebook` of three tabs.
- **A platform tree over three answers.** `Query::Outline`, `Query::Layers` and
  `Query::Attachments` become one `PanelRow` shape and then a `GtkListView` over a
  `GtkTreeListModel` with a `GtkTreeExpander` per row — GTK4's modern list stack, not the
  `GtkTreeView` it deprecated. §12.3.3's `/Count` sign decides which rows start open; §8.11.4.3's
  groups get a `GtkCheckButton` and Table 99's `/Locked` makes it insensitive; §7.11.4's rows carry
  the `/EmbeddedFiles` key that `Command::Extract` names and show Table 43's `/UF` beside it.
- **Real controls over `Query::Fields`**, placed on a `GtkFixed` at the widgets' own rectangles: a
  `GtkEntry`, a `GtkTextView` for Table 231 bit 13, a `GtkPasswordEntry` for bit 14, a
  `GtkCheckButton` for §12.7.5.2.3 and §12.7.5.2.4, a `GtkButton` for §12.7.5.2.2, a `GtkDropDown`
  for Table 233 bit 18's combo box and a `GtkListView` for its list box.
- **Interactive chrome as geometry**, drawn by a `GtkDrawingArea` that cannot be targeted by input,
  over both. See "the accent colour" below.
- **The two decisions a host owns.** §12.7.6.4's file, under the narrowest policy that still
  performs the action — one path component, resolved against the document's own directory, every
  refusal typed and said; and §7.6.4.1's password, in a modal `GtkWindow` with a
  `GtkPasswordEntry`, three attempts.

## What the boundary turned out to be missing

Six findings, in descending order of what they cost.

### 1. The page cannot be drawn without its widget appearances — `doc/todo/37`'s open decision, and it is worse in a native host than the file predicted

`doc/todo/37` names it and says it wants a round of its own. **This round is the first evidence of
what it actually looks like**, and the evidence is a photograph: `160F-2019.pdf`, 67 fields on page
one, **76 controls placed**, every one of them sitting on top of the picture of itself. A person
sees each field twice.

Two things the file did not say, both found by placing the controls:

- **A native control has a minimum size the document's rectangle can be smaller than.** A
  `GtkEntry` under this theme is about 34 logical pixels tall; a widget's `/Rect` on a 596×842 page
  is commonly 15. `set_size_request` is a *floor*, so the control does not merely sit over its
  appearance — it **overflows** it and covers the page's own text around it. Deleting the appearance
  underneath would not fix that; it is a second, separate consequence of the same decision, and it
  is why a real native form host will also want to render the page at a scale it chooses.
- **A quadrilateral is not a rectangle.** `FormWidget::quad` arrives as four corners, correctly, so
  that §7.7.3.3's `/Rotate` and Table 189's `/R` survive the crossing — and every platform control
  is an axis-aligned rectangle. The host takes the axis-aligned bound and loses the rotation. That
  is the host's loss rather than the boundary's, and it is recorded because a rotated widget is the
  case where an appearance underneath a control is *most* visible.

**Not taken this round**, as instructed. What it costs, stated from having hit it: the change is a
flag on the render request rather than a query, because it is a departure a host *asks for*; and it
changes `interpret`, which is what the oracle's 1794-page comparison rests on. The cheapest shape
that does not touch the oracle is a flag that is `false` in every existing caller, so that the
gates' display lists are byte-identical by construction and only a host that sets it sees a
different page. That is a claim to test, not a design to adopt.

### 2. `pdf_render::RasterFormat` is `#[non_exhaustive]`, and it crosses the boundary

`viewer-core`'s own rule is that nothing in its vocabulary is `#[non_exhaustive]`, because "a
catch-all arm is where a message added later goes to be ignored in silence" and "a new `Event`
should fail to compile in every consumer". `Raster` crosses that boundary — inside
`Rendered::Raster` and `Answer::Frame` — and its `format` field is an enum from *another* crate
which **is** `#[non_exhaustive]`. A host mapping the format to its platform's must therefore write
a catch-all, and a second pixel format added to `pdf-render` would compile in every host and produce
a wrong picture or a runtime refusal rather than a build failure.

`viewer-gtk` refuses it by name (`PixelError::UnknownFormat`) rather than defaulting, which is trap
5 — but a refusal at runtime is exactly what the rule exists to avoid. **Not changed this round**,
because removing `#[non_exhaustive]` from a `pdf-render` type is a decision about that crate's
stability guarantee and not about hosts.

### 3. `Answer::Field`'s value for a password field cannot be read back, and nothing in the vocabulary says so

ADR 0201's rule is that a host keeps the *point* it clicked and never the text, because §12.7.5.3's
truncation means the field can take less than was typed — so a host writes the value back into its
control after every keystroke. **Table 231 bit 14's field is the exception**: `Answer::Field`
answers it with bullets rather than with characters ("a host is allowed to draw them and not to know
them"), so a host that followed the rule would replace what a person typed with a row of dots and
send *those* as the next value.

The exception is discoverable — the flag is in `Control::Text`, which the same query carries — but
it is discoverable only by reading two doc comments and noticing they interact. A host that read the
read-back rule alone would ship the bug. Recorded rather than changed: the fix is one sentence in
`Answer::Field`'s doc comment, and it belongs in a round that is looking at that clause.

### 4. `Answer::Outline` borrows the viewer, and a toolkit's callbacks are `'static`

`Query::Outline` answers `Answer::Outline(&'a pdf_model::outline::Outline)` while every other panel
answer is owned. That is right for `viewer-ui`, which draws from the borrow inside one call; a
toolkit host has to clone the whole tree into its own model anyway, because a `GListModel` outlives
the query. **No change is owed** — the borrow costs the host one clone it was going to make — but it
is the one asymmetry in the panel answers and it is worth knowing that it is a `viewer-ui`-shaped
choice rather than a general one.

### 5. There is no event that says "the page's geometry moved"

A host placing controls over the page must re-place them whenever the origin or the scale changes.
`Event::Damage` covers it in practice — a scroll and a zoom both emit one — but `Damage` is
documented as "a bound on what changed", not as "the mapping moved", so a host relies on a
coincidence. `viewer-gtk` sidesteps it by re-querying `Query::Frame`, `Query::Fields`,
`Query::Selection` and `Query::Focus` after **every** pump, which is idempotent and cheap and makes
the window a function of the viewer's state rather than of the order events arrived in. Recorded
because the next host will make the same choice and should know it is a choice.

### 6. GTK 4.22 has no accent colour, so "the desktop's accent" is not available to ask for

`doc/ui-boundary.md`'s argument for chrome crossing as geometry is that it lets a host draw
selection "in macOS's selection colour, KDE's accent, the Windows highlight brush". On GTK4 that
colour **cannot be obtained**: there is no symbol containing `accent` anywhere in `gtk4-sys` 0.11.4,
and `@accent_bg_color` is a CSS name libadwaita defines rather than something a widget can be asked
for. What GTK does give is `gtk_widget_get_color` (since 4.10), the theme's own foreground at that
widget, which follows light and dark without the program knowing which is on — so that is what the
selection fill and the focus ring are drawn in.

**This does not weaken the geometry argument; it sharpens it.** Handing over pixels would have made
even *this* impossible, and the colour a host can get is a fact about each platform rather than
about the boundary.

## Evidence, under `Xvfb`

ADR 0126's recipe, `Xvfb :78` at 1100×1200, `GSK_RENDERER=cairo`, `xdotool` for input and `xwd` for
the pixels. Numbers read off the run rather than described.

| what | what the run said |
|---|---|
| page drawn | `PDF20_AN001-BPC.pdf`, first frame at **96.5 ms** median of five; the window shows the cover page and the title bar reads `PDF20_AN001-BPC.pdf — Cover — page 1 of 5 — PDF 2.0 Application Note 001: …` — §12.4.2's label and §12.3.3's section, from `Event::PageChanged` |
| page turned | two `Right` presses: `GoTo(Next)` → page 2 rasterised in **2.65 ms**, `GoTo(Next)` → page 3 in **6.11 ms** |
| a tree from a real document | `outline 5 row(s)` at the top level and **14 rows in all**, every one of which §12.3.3's `/Count` sign asks to be open — and 14 rows is what the photograph shows, each an expander and a label in a `GtkListView`. ISO 32000-2's own outline is **38 top-level, 988 rows, 38 shown**, because that document closes all of them, so the tree builds its children lazily and `EXPANSION_LIMIT` is never approached (`cargo run -p viewer-gtk --example outline_census`) |
| a **layer** switched from a native check button | `visibility_expressions.pdf`: three rows `A`, `B`, `C` with `A` and `B` checked. Clicking `C` sent `SetGroup { group: ObjectId { number: 10, generation: 0 }, on: true }`, the page re-rendered, and **6656 of 474 721 pixels changed (1.40%)** — the line reading *A and (not C)* going from drawn to invisible, which is §8.11.2.2's visibility expression evaluated |
| a **form** in native controls | `160F-2019.pdf`: `67 field(s) on the page, 76 control(s) placed` |
| a field **filled in** | a click into an entry and `xdotool type QRS` produced `Edit(SetField { field: "X.minus1", value: Some("-1Q") })`, `…Some("-1QR")`, `…Some("-1QRS")`, and the entry on the screen reads `-1QRS` with the platform's own caret in it |
| an **attachment** extracted from a tree row | `attachment.pdf`: `Extract { name: "foo.txt" }` → `wrote 9 bytes to …/foo.txt`, whose content is `bar baz` |
| §7.6.4.1's **password** | `issue6010_1.pdf` opened with no password produced one `PasswordRequired`; `abc` typed into the `GtkPasswordEntry` and `Return` produced a second `Open` with 3 events and a page reading *Issue 6010* |
| a 1023-page document | `ISO_32000-2_sponsored_EC3.pdf`: first frame at **152.1 ms** median of five against 96.5 ms for a five-page one — the whole difference is `Open`, 3 ms against 40 ms, which is `CLAUDE.md`'s incremental-parsing rule holding through a second host |

**One instrument limit, stated rather than hidden.** The password dialogue's own window is not
visible in an `xwd` of the root: `Xvfb` runs no window manager, so a modal transient is created and
takes keystrokes — `xdotool search --name '^Password$'` finds it and typing into it opens the
document — but nothing maps it above its parent. That is a fact about the instrument. The
functional evidence is the second `Open` and the page.

## Cross-compilation, and the exclusion is deliberate

The two checks in `doc/verify.md` name their packages with `-p` and are **unmoved**: all three
commands pass with this crate in the workspace, because none of them builds it.

`viewer-gtk` is excluded on purpose, and the reason is checkable — asking for it says so:

```
$ cargo check --target x86_64-pc-windows-msvc -p viewer-gtk
error: failed to run custom build command for `glib-sys v0.22.8`
  cargo:warning=pkg-config has not been configured to support cross-compilation.
```

A GTK host needs GTK 4's development files *for the target*, which is a platform's package manager's
job and not a Rust target's. This is the same shape as `viewer-accessibility` being Linux-only in
its own manifest (ADR 0214): a host binds a platform, and the platform is where it is checked.

## Consequences

- The vocabulary needed **no new message**. Ten sessions of hosts added five messages; a whole
  native host added none, which is the strongest thing this round has to say about ADRs 0116–0121.
- `doc/todo/30`'s first of three is done. Qt is next, and it now has a Rust host to be compared
  against rather than only `viewer-ui`.
- **`doc/todo/37`'s one open decision is now the thing blocking a *good* native host**, with a
  photograph behind it instead of a prediction, and with a second consequence (a control's minimum
  size) that the audit did not name.
- The fifth sweep of `doc/todo/01` was run with a fifth host crate: **246 `pub fn`s in `pdf-model`,
  85 named by no host, and the GTK host names not one that the other four do not.** A native host
  reaches `pdf-model` for *types* the answers carry and for no function of its own, which is the
  boundary working as designed.
