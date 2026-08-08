# ADR 0246 — The second host, and the one word it cost

Status: accepted, 2026-08-09 (session 410).

## Context

`doc/todo/30`'s order is the project owner's and is not negotiable: **GTK4 first**, because
`gtk4-rs` is a safe Rust binding with no C++ bridge; **Qt second, because it costs a C++ bridge and
should not be the experiment that shapes the API**; **`viewer-ffi` last**, under one sentence —

> **Do not freeze a C ABI until two Rust consumers have shaken the API out.**

The first of those two landed in the four-hundred-and-eighth session (ADR 0244) and its headline was
that a whole native host added **no new message**. That is evidence and not yet proof: GTK4 and
`viewer-core` are both Rust, both single-threaded, both owned by the same side. This round is the
second consumer, and its product is **what a toolkit unlike GTK4 asks of the boundary** — a
different widget model, a different ownership model, and a language boundary in the middle.

The short answer is the same answer, and it is now worth something: **the vocabulary needed nothing
again.** What did change is one word in `viewer-qt`'s crate root, and half of `viewer-gtk`.

## Decision 1 — `cxx` 1.0.199 and `cxx-qt-build` 0.9.1, confined to one crate

**Taken.** `crates/viewer-qt`, one library and one binary (`pdf-viewer-qt`), the only manifest in
the tree that names either, and the only C++ in the tree (`cpp/host.h`, `cpp/window.h`,
`cpp/window.cpp`, 1159 lines).

In ADR 0186's, 0214's and 0244's shape, which is what `doc/third-party-data.md` records:

| | |
|---|---|
| what it is | `cxx` generates the `extern "C"` shims that carry messages between Rust and C++; `cxx-qt-build` finds Qt 6 through `qmake6`, runs `moc` over `cpp/window.h`, compiles the C++ and links QtCore, QtGui and QtWidgets |
| licence | **23 packages**, every one MIT, Apache-2.0, or Unlicense-or-MIT. `cargo deny check` is clean on all four checks with them in the graph and **no new exception in `deny.toml`** |
| what reaches a shipped binary | **three of the 23** — `cxx`, `cxxbridge-macro`, `link-cplusplus`. The other twenty are build-time only (`cxx-qt-build`, `cxx-gen`, `qt-build-utils`, `clang-format`, `which`, `codespan-reporting`, …) and appear in no `cargo tree -e normal` |
| `unsafe` | **one hand-written token**, and decision 2 is about it |
| what it costs elsewhere | nothing on the three cross-target checks, which name their packages with `-p` — see "cross-compilation" below |

**`cxx-qt` itself is deliberately absent.** The crate that turns a Rust type into a `QObject` is
built for QML, and taking it costs two link-time initialisers (`cxx_qt_init_crate_cxx_qt`,
`cxx_qt_init_crate_cxx_qt_lib`) that a crate not using it fails to resolve. A Qt **Widgets** host
subclasses `QAbstractItemModel` and `QWidget` in C++, where `moc` can see them; only the *build*
half is wanted, and it is available on its own.

**One cost that is visible on every build and is not fixable from here**: `cxx`'s generated
`bridge.cxx.cpp` makes g++ 15 emit three `-Wmaybe-uninitialized` false positives, one per
`rust::Vec<T>` default constructor over a shared struct. `cc-rs` reports compiler warnings as
`cargo:warning=` lines. Silencing them needs `cc::Build::warnings(false)`, which `cxx-qt-build`
0.9 reaches only through `cc_builder` — an **`unsafe fn`**, so using it would put hand-written
`unsafe` in `build.rs` to hide a warning. That trade is refused and the noise is documented
instead; no gate fails on it.

## Decision 2 — `#![deny(unsafe_code)]`, one exemption, and a test on it

**This is the finding `doc/todo/30` was waiting for**, and it says the permission has to be argued
rather than waved through.

The file reserves `unsafe` for `viewer-ffi`: *"the only crate in the tree permitted `unsafe`. Every
crate touching PDF bytes keeps `#![forbid(unsafe_code)]`."* A C++ bridge forces the question a step
early, and the first thing to establish was whether it forces it at all. It does, and the evidence
is a count: putting `#![forbid(unsafe_code)]` on this crate produces **18 errors**, every one from
inside `#[cxx::bridge]`'s expansion — `unsafe extern` blocks, `unsafe` blocks, `unsafe` function
declarations, `#[export_name]` functions and unsafe trait implementations. `forbid` cannot be lifted
by an inner `allow`; that is the whole point of `forbid` and the reason the parsers carry it.

So the position taken is exact, and it is **narrower than "this crate has `unsafe`"**:

- the crate root **denies** `unsafe_code`;
- the exemption is **one attribute**, on `mod bridge`, and it is the only one in the tree;
- **the whole crate contains one hand-written `unsafe` token**: the `unsafe extern "C++"` block
  header. That token is not a licence but an *obligation* — `cxx`'s way of asking the author to
  assert that the C++ declared there is the C++ that exists and is safe to call with those types;
- it is discharged in the smallest way available. `cpp/host.h` declares **one function**,
  `run_qt_host(rust::Box<Host>, int32_t) -> int32_t`, and names no Qt type at all; `cpp/window.cpp`
  defines exactly that one. There is nothing else to audit.

**And it is enforced rather than promised.** `tests/unsafe_position.rs` reads the crate's own
sources back and asserts three things: that the only line in `src/` or `build.rs` using the token is
`bridge.rs`'s `unsafe extern "C++" {`; that `src/lib.rs` denies once and lifts once and the lift
attaches to `mod bridge;`; and that **no other crate under `crates/` lifts the denial**. A `deny`
with an exemption is a rule with a hole in it, and a hole nobody measures is a hole that grows.

`doc/todo/30`'s rule survives, restated: it was about *reviewable* `unsafe` — a promise a person
makes that a compiler cannot check — and there is one such promise, in one place, with a test on its
position. The file is corrected to say so rather than to keep a sentence the tree no longer matches.

## Decision 3 — `crates/viewer-host`, because half of `viewer-gtk` was never GTK's

**Taken, and it is the second host's other product.**

`viewer-gtk` shipped as eight modules. Writing the Qt host wanted four of them **unchanged**:

| module | what it is |
|---|---|
| `panel` | §12.3.3's outline, §8.11.4.3's `/Order` and §7.11.4's embedded files as one `PanelRow` tree with a `RowAction` per row |
| `form` | §12.7.5's field decided into the `ControlKind` a platform builds |
| `policy` | §12.7.6.4's import-data file, under the narrowest policy that performs the action |
| `trace` | `--trace=<topics>`, in the format `viewer-ui` prints |

Not one of them named a GTK type in its code; all four named one in a doc comment, which is what
changed when they moved. They were written
toolkit-free on purpose — it is the only part of a native host a workspace test suite can see
without a display — but nothing said whether that was a fact about *GTK* or a fact about **hosts**.
A second host is what could answer it, and the answer is the second.

So they are `crates/viewer-host`, depended on by both native hosts and by neither toolkit.
`viewer-gtk`'s public interface is now **`Host` and `HostError` and nothing else**, which is the
sharpest statement of the finding available: *a native host on this boundary is mostly not toolkit
code.*

**Deliberately not `viewer-core`.** That crate is a vocabulary — `Command` in, `Event` out,
`Query` → `Answer` beside them — and a mapping from three of its answers into one row shape is a
convenience for whoever draws a tree, not a statement about a document. Putting it there would make
the core answer a question no clause asks. Leaving it in each host would have it written twice, and
the second copy is where two hosts stop agreeing about what §12.3.3's `/Count` sign means.

**And nothing shared that the two hosts genuinely do differently**: no widget, no window, no event
loop, no pixel format. Each of those is where the toolkits diverge, and an abstraction over them
would be an invention rather than a finding.

## Decision 4 — tier 1, and Qt Widgets offers no other either

**Taken**, and for a different reason from GTK's, which is what makes the agreement worth something.

`doc/ui-boundary.md` names three pixel tiers. ADR 0244's argument was that GTK4 gives a widget no
native surface (tier 2 has nothing to bind) and hands out no device (tier 3). Qt's is not that: a
`QOpenGLWidget` and a `QVulkanWindow` **exist**, and either would reach tier 2. The argument is that
neither is the *comparable* host — they are different widgets with different rules about being
composited inside a `QSplitter` beside a sidebar — and that `QRhi`, which owns the device Qt itself
draws through, is a private module a Qt release may change without notice. So tier 1 is the tier a
plain `QWidget` admits, and the plain `QWidget` is the one this round is for.

**And the copy is the same copy.** `Raster` is row-major RGBA with straight alpha and no padding,
which is `QImage::Format_RGBA8888` exactly — not `Format_RGBA8888_Premultiplied` — so there is no
conversion at all, only a `memcpy`, exactly as into GTK's `gdk::MemoryTexture`. The bridge carries
the pixels as a **borrowed slice** (`fn frame_pixels(&self) -> &[u8]`, `rust::Slice<const uint8_t>`
in C++) rather than a `Vec`, so tier 1 costs the one copy `doc/ui-boundary.md` prices it at and not
two.

**Measured on both hosts, in one sitting, on the same document** — which is the first time the two
have been measured against each other rather than against an estimate:

| | first frame's copy, five runs | median | rate |
|---|---|---|---|
| `pdf-viewer-gtk`, 2 687 100 B into a `gdk::MemoryTexture` | 895, 657, 748, 627, 884 µs | **748 µs** | 3.6 GB/s |
| `pdf-viewer-qt`, 2 765 244 B into a `QImage` | 1275, 1037, 790, 1310, 1078 µs | **1078 µs** | 2.6 GB/s |

| | steady-state copies, after the first page turn | median | rate |
|---|---|---|---|
| `pdf-viewer-gtk`, twelve samples | 101–623 µs | **234 µs** | 11.5 GB/s |
| `pdf-viewer-qt`, four samples | 200–289 µs | **231 µs** | 12.0 GB/s |

**ADR 0244's "≈3.2 GB/s" was a first-frame number**, and the steady state is three to four times
faster on *both* toolkits. That is the honest correction, and the second host is what produced it:
one measurement of one host cannot tell "this is what the copy costs" from "this is what the first
copy costs". The two toolkits' steady states agree to within 4%, which is what "no conversion at
all, only a `memcpy`" predicts and is now checked rather than argued.

## What Qt asked of the boundary that GTK did not

Six things, in descending order of what they cost. **None of them is a message.**

### 1. The tree has to arrive whole, and every node needs a stable identity

`GtkTreeListModel` takes a closure and calls it when a person opens a row, so `viewer-gtk` never
builds a subtree nobody looked at. `QAbstractItemModel` is the opposite contract: it must answer
`index`, `parent`, `rowCount` and `columnCount` for **any** node at **any** moment, and every
`QModelIndex` carries an identity (`internalId`) that must stay valid while the view holds it.

So `viewer-qt` flattens: `Query::Outline`, `Query::Layers` and `Query::Attachments` become
`viewer_host::panel`'s one row shape, then a depth-first `Vec<QtRow>` with a depth on each row, and
`PanelModel::setRows` rebuilds the parentage from a stack of open parents in one pass. Every node
keeps a fixed slot in a `std::vector`.

**This is where `Answer::Outline`'s borrow gets slightly worse**, which is ADR 0244 finding 4 tested
rather than repeated. That answer is `&'a Outline` where `Query::Layers` and `Query::Attachments`
are owned. ADR 0244 said "no change is owed — the borrow costs the host one clone it was going to
make". For GTK that is exactly right. For Qt the clone is not merely made, it is **made in full and
in advance**, because laziness is not available. Still no change owed — a thousand rows is a
thousand small strings — but the asymmetry is now known to be a `viewer-ui`-shaped choice that costs
the two hosts differently.

**And the eager build is free at the sizes that exist, which is the point of measuring it.** It is
on the launch path: `applyUpdates` builds the panels before it shows the frame. ISO 32000-2's
outline is 38 top-level items and 988 rows; the five-page application note's is 5 and 14. Both were
timed over five runs:

> 14 rows into three models: 2017, 3106, 3337, 3418, 4708 µs — median **3.34 ms**
> 990 rows into three models: 2885, 3034, 3655, 4216, 4238 µs — median **3.66 ms**

976 extra rows cost about 0.3 ms, which is inside the run-to-run spread. What the three milliseconds
buy is three `QTreeView`s and three models existing at all; the rows are free. GTK's laziness is a
real property and it is worth nothing at outline sizes documents have.

### 2. §8.11.4.3's switch is a *role*, not a widget

`viewer-gtk` appends a `GtkCheckButton` to each layer row's box. Qt's answer is `Qt::CheckStateRole`
on the model, with `setData` receiving the click and `flags()` deciding whether a person may make
it — so the switch is **data** in one host and a **widget** in the other, over the same
`RowAction::Toggle { group, on, locked }`.

Table 99's `/Locked` lands better on Qt for the same reason. GTK makes the button insensitive, which
greys it; Qt omits `Qt::ItemIsUserCheckable` from that row's flags, so the box still shows the
document's own answer and simply cannot be moved. The clause — *"[t]he state of a locked group
cannot be changed through the user interface of an interactive PDF processor"* — is about the
change, not about the display, and Qt's spelling says exactly that. `Host::toggle_row` refuses it a
second time on the Rust side, because a host that obeyed a clause only in its view code would obey
it only as long as nobody wrote a second view.

### 3. The ownership inverts, and one compiler check becomes a promise

`gtk4-rs` callbacks are `'static` Rust closures the Rust side installs, so `viewer-gtk` holds itself
as `Rc<RefCell<Host>>` and every callback goes through a `try_borrow_mut` that prints a note when a
call arrives mid-write. Qt's callbacks are **C++ lambdas**, so C++ owns the host — `rust::Box<Host>`
inside `MainWindow` — and the Rust half is a plain struct with no interior mutability anywhere.

That is simpler, and it moves one guarantee across the language boundary. Two overlapping
`&mut Host` is undefined behaviour, and on this side nothing would say so; Qt delivers events from
**nested event loops** (`QDialog::exec` is one, and this host opens one for §7.6.4.1's password), so
the case is real rather than theoretical. `MainWindow::busy_` and the `Busy` guard in `window.cpp`
are what make the promise mechanical, and §7.6.4.1's prompt is posted through
`QTimer::singleShot(0, …)` so that the handler holding the host unwinds before the nested loop
starts.

**Recorded as a cost of the bridge rather than of the boundary**, because the boundary did not
notice: `Command` is a value either way.

### 4. Qt has the accent colour, and GTK4 does not

`doc/ui-boundary.md` argues that chrome crosses as geometry so that a native host can draw selection
in *"macOS's selection colour, KDE's accent, the Windows highlight brush"*. ADR 0244 had to record
that on GTK 4.22 that colour **cannot be obtained** — no symbol containing `accent` exists in
`gtk4-sys`, and `@accent_bg_color` is a libadwaita CSS name — and settled for
`gtk_widget_get_color`, the theme's foreground.

Qt has both. `QPalette::Highlight` is the selection brush and `QPalette::Accent` has existed since
Qt 6.6, and KDE writes both from the colour scheme. The host says which it used, out loud, so the
claim is checkable:

> `chrome in the platform's colours: selection #3daee9 (QPalette::Highlight), focus ring #3daee9
> (QPalette::Accent)`

and the pixel it drew, sampled out of the `xwd`, is `srgb(187,227,248)` — which is `#3daee9` at the
0.35 alpha the overlay uses, composited over white: `0.65 × 255 + 0.35 × 61 = 187`, `0.65 × 255 +
0.35 × 174 = 227`, `0.65 × 255 + 0.35 × 233 = 247`. The selection on the screen **is** the desktop's
own colour, arithmetic included.

**This is the geometry argument paying rather than being defended.** Which colour a platform will
part with is a fact about each platform; handing over finished pixels would have made even GTK's
answer impossible.

### 5. One clause Qt can obey that GTK could not, and one neither can

Table 233 bit 19: *"the combo box shall include an editable text box as well as a drop-down list"*.
`GtkDropDown` is not editable, so `viewer-gtk` carries the flag and reports it; `QComboBox` is, so
`viewer-qt` calls `setEditable(control.editable)` and the clause is obeyed. That is the one place the
two hosts differ in **what they can do** rather than in how they spell it — and it is a fact about
toolkits, because `Answer::Fields` carried the flag to both.

Table 233 bit 22 is the other way: *"more than one of the field's option items may be selected
simultaneously"*. Qt's `QListWidget` does multiple selection natively and `viewer-qt` still asks for
`SingleSelection`, because `Edit::SetField` carries **one** value. Offering the control and sending
one of what a person chose would be worse than not offering it. **This one is the boundary's**, and
it is the only place in two hosts where that is true — it is now in `doc/todo/30` as such rather
than as a note about a list box.

### 6. `pdf_render::RasterFormat` is `#[non_exhaustive]`, and the second host makes it worse

ADR 0244 finding 2, unchanged in substance and changed in weight. `viewer-core`'s own rule is that
nothing in its vocabulary is `#[non_exhaustive]`, because *"a new `Event` should fail to compile in
every consumer"*. `Raster` crosses inside `Rendered::Raster` and `Answer::Frame`, and its `format`
is an enum from another crate which **is** `#[non_exhaustive]`.

`viewer-qt` refuses an unknown format by name in `page::describe`, as `viewer-gtk` does. **The
catch-all arm is now written twice**, and a third consumer writes it a third time — and the third
consumer is a **C ABI**, which cannot fail to compile in anybody at all. This is the one thing on
`doc/todo/30`'s list that should be settled *before* the freeze rather than after it, and this
round's contribution is to say why the deadline is real.

## What the host does

Everything ADR 0244's checklist has, so that the two are comparable:

- A `QMainWindow` with a toolbar, a `QSplitter`, and a `QTabWidget` of three `QTreeView`s.
- **A platform tree over three answers** — `PanelModel : QAbstractItemModel` over
  `viewer_host::panel`'s rows, two columns (Qt's own way of showing a second line), §12.3.3's
  `/Count` sign deciding which rows open, §8.11.4.3's switch as `Qt::CheckStateRole` and Table 99's
  `/Locked` as the absence of `Qt::ItemIsUserCheckable`, §7.11.4's rows carrying the
  `/EmbeddedFiles` key `Command::Extract` names.
- **Real controls over `Query::Fields`**, placed by `setGeometry` on a layout-free `QWidget` — Qt's
  `GtkFixed` — a `QLineEdit`, a `QPlainTextEdit` for Table 231 bit 13, `QLineEdit::Password` for
  bit 14, a `QCheckBox` and a `QRadioButton` for §12.7.5.2.3 and §12.7.5.2.4, a `QPushButton` for
  §12.7.5.2.2, a `QComboBox` for Table 233 bit 18's combo box and a `QListWidget` for its list box.
  §12.5.6.19's appearances are delegated away by default (`Command::Delegate`, ADR 0245).
- **Interactive chrome as geometry**, on a `ChromeOverlay` carrying
  `Qt::WA_TransparentForMouseEvents` — the same job `gtk_widget_set_can_target(FALSE)` does, and
  both toolkits have an answer because both need one.
- **The two decisions a host owns.** §12.7.6.4's file through `viewer_host::policy`, shared with the
  other host and therefore identical by construction; §7.6.4.1's password in a modal `QDialog` with
  a `QLineEdit` in `Password` echo mode, three attempts.

**One thing `viewer-qt` has that `viewer-gtk` does not**: `--quit-after=<ms>`. A window under `Xvfb`
has nobody to close it, and a test that killed the process could not tell a clean exit from a crash.
Every number below was taken from a run that ended by itself with status 0.

## Evidence, under `Xvfb`

ADR 0126's recipe: `Xvfb :78` at 1100×1200, `QT_QPA_PLATFORM=xcb`, `xdotool` for input and `xwd` for
the pixels. Release binaries. Numbers read off the run rather than described.

| what | what the run said |
|---|---|
| page drawn | `PDF20_AN001-BPC.pdf`, first frame at **105.9 ms** median of five (128.9, 105.9, 98.6, 153.3, 104.9) — and `pdf-viewer-gtk` on the same machine in the same session was **139.7 ms** median of five. Both draw the cover page; the title bar reads `PDF20_AN001-BPC.pdf — Cover — page 1 of 5 — PDF 2.0 Application Note 001: Black Point Compensation`, which is §12.4.2's label and §12.3.3's section from `Event::PageChanged` |
| page turned | six `Right` presses: pages 2, 3, 4, 5 rasterised in **6.31, 12.19, 9.62, 8.52 ms**, and the title bar reached `page 3 of 5 — References` |
| a tree from a real document | `outline 5 row(s)` at the top level, **14 rows in all**, in a two-column `QTreeView` with four of them expanded — every item that *has* children. (ADR 0244 wrote this as "every one of which asks to be open"; a leaf carries no `/Count` and is neither open nor closed, which `the_outline_crosses_depth_first_with_a_depth_on_every_row` now asserts precisely.) ISO 32000-2's is **38 top-level, 990 rows across the three trees**, built in **3.66 ms** median of five against **3.34 ms** for fourteen |
| a **layer** switched from a native check box | `visibility_expressions.pdf`: three rows `A`, `B`, `C` with `A` and `B` checked, drawn by `Qt::CheckStateRole`. Clicking `C` sent `SetGroup { group: ObjectId { number: 10, generation: 0 }, on: true }` — the same object ADR 0244 recorded — the page re-rendered, and **6836 of 832 000 pixels of the page area changed (0.82%)**, which is §8.11.2.2's *A and (not C)* going from drawn to invisible |
| a **form** in native controls | `160F-2019.pdf`: `67 field(s) on the page, 76 control(s) placed` — the same 67 and the same 76 `viewer-gtk` places, which is `viewer-host` shared rather than reimplemented |
| how badly the controls fit | Qt: **13 of 76 wider than their `/Rect` (worst +66 on 18 px), 76 taller (worst +20 on 14 px)**. GTK, from ADR 0245: 11 of 76 wider (worst +85 on 120 px), 76 taller (worst +22 on 12 px). **Every control is taller than its rectangle on both toolkits**, which is what turns ADR 0245's scale question from a Breeze-versus-Adwaita accident into a fact about platform controls |
| a field **filled in** | a click into the employer-name entry and `xdotool type QRS` produced `Edit(SetField { field: "A.EMP", value: Some("Q") })`, `…Some("QR")`, `…Some("QRS")`, and the entry on the screen reads `QRS` |
| an **attachment** extracted from a tree row | `attachment.pdf`: `Extract { name: "foo.txt" }` → `wrote 9 bytes to …/foo.txt`, whose content is `bar baz` |
| §7.6.4.1's **password** | `issue6010_1.pdf` opened with no password produced one `PasswordRequired`; `abc` typed into the `QLineEdit` and `Return` produced a second `Open` with 3 events and a window titled `issue6010_1.pdf — page 1 of 1` |
| **chrome in the platform's colours** | `Select(All)` drew the page's text in `#3daee9`, which the host reports as `QPalette::Highlight`, and the sampled pixel `srgb(187,227,248)` is that colour at 0.35 over white |
| a 1023-page document | `ISO_32000-2_sponsored_EC3.pdf`: first frame at **193.8 ms** median of five against 105.9 ms for a five-page one, with `opened, 1023 page(s)` — `CLAUDE.md`'s incremental-parsing rule holding through a third host |

**The same instrument limit ADR 0244 recorded, confirmed on a second toolkit.** The password
dialogue is not visible in an `xwd` of the root: `Xvfb` runs no window manager, so a modal transient
is created and takes keystrokes — `xdotool search --name '^Password$'` finds it, typing into it opens
the document — but nothing maps it above its parent. That is a fact about the instrument rather than
about either toolkit, and the functional evidence is the second `Open` and the page.

## Cross-compilation, and the exclusion is deliberate

The three checks in `doc/verify.md` name their packages with `-p` and are **unmoved**: all three
pass with this crate in the workspace, because none of them builds it.

`viewer-qt` is excluded on purpose, and the reason is checkable — asking for it says so:

```
$ cargo check --target x86_64-pc-windows-msvc -p viewer-qt
error occurred in cc-rs: failed to find tool "lib.exe": No such file or directory
```

A Qt host needs a C++ toolchain **and** Qt 6's development files *for the target*, which is a
platform's package manager's job and not a Rust target's. That is the same shape as `viewer-gtk`'s
exclusion (ADR 0244) and `viewer-accessibility`'s Linux-only manifest (ADR 0214): a host binds a
platform, and the platform is where it is checked.

**And the workspace gates now name what they need.** `cargo clippy --workspace --all-targets` and
`cargo test --workspace` build both native hosts, so both toolkits' development files have to exist
on the machine running them. CI installed neither; it now installs `libgtk-4-dev`, `qt6-base-dev`
and `qt6-base-dev-tools` in the two jobs that run those commands. That gap arrived with the first
host and is closed for both.

## Consequences

- **The vocabulary needed no new message, for the second host running.** Eleven messages in eleven
  rounds of hosts, each because a clause needed a channel; two whole native hosts have added none
  between them. That is the strongest thing this round has to say about ADRs 0116–0121, and it is
  what `doc/todo/30` was waiting to hear.
- **The C ABI may be frozen**, with three amendments named, and `doc/todo/30` carries them: the
  `#[non_exhaustive]` `RasterFormat` (a C ABI cannot fail to compile in anybody, so it has to be
  settled first), `Answer::Outline`'s borrow (which a C ABI has to own anyway), and
  `Answer::Field`'s unreadable password value (ADR 0244 finding 3, still one sentence in a doc
  comment).
- **`viewer-gtk` lost four modules and its public interface is two items.** A native host on this
  boundary is mostly not toolkit code, which nobody could have said with one host.
- **The tier-1 copy is now known rather than estimated**, on two toolkits, cold and warm — and ADR
  0244's number is corrected as a first-frame measurement rather than a steady-state one.
- **`unsafe` reaches the tree one round earlier than `doc/todo/30` planned**, as one token in one
  place with a test on its position, and the file is corrected rather than left saying something
  the tree no longer matches.
