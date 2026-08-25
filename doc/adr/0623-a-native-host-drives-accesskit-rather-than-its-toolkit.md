# ADR 0623 — A native host drives AccessKit rather than its own toolkit's accessibility layer

Status: accepted, seven-hundred-and-thirty-first session.
Supersedes nothing. Amends `doc/todo/31`'s "that is `viewer-ui`'s", `doc/todo/30`'s largest
remaining debt, and one false sentence in `viewer-gtk`'s own module documentation.

## The question

`tools/state.sh windows` named it: `Query::AccessibilityTree` and `Query::Readback` reached one
window of three, so a screen reader on `pdf-viewer-gtk` or `pdf-viewer-qt` was handed a picture.
The debt has two possible shapes and they are not variations on each other:

1. **Publish through the toolkit.** GTK4 has `GtkAccessible` and an AT-SPI backend of its own; Qt
   has `QAccessible` and `QAccessibleInterface`. §14.8.4's forty-one structure types would be
   mapped onto `GtkAccessibleRole` and onto `QAccessible::Role`, and each toolkit would map *that*
   onto AT-SPI.
2. **Drive AccessKit directly**, as `viewer-ui` has since ADR 0214: §14.8.4 onto `accesskit::Role`,
   and `accesskit_unix` onto AT-SPI.

## What was checked rather than assumed

**A screen reader on this platform talks to AT-SPI, and both routes end there.** So the choice is
not about reaching the person; it is about how many times this project maps §14.7 onto somebody
else's vocabulary, and where the second implementation would live.

**Route 1 is open on GTK, and the reason this file gives for not taking it is *not* the one the
tree already had written down.** `viewer-gtk/src/host.rs` said `#![forbid(unsafe_code)]` "is what
makes subclassing the wrong answer here". That is false, and it was disproved by writing the
subclass: `#[glib::object_subclass]` expands to `unsafe impl` and `unsafe` blocks, the `unsafe_code`
lint does not fire on a proc macro's expansion, and a `GObject` implementing `gtk4::Accessible`
with `gtk4::ATContext::create` compiles in that crate with the attribute untouched. Trap 17's own
subject — a floor read off a catalogue — one round after ADR 0508 paid for it on Table 233 bit 19.
The comment is corrected.

**Route 1 is nearly closed on Qt, and that half is structural.** `crates/viewer-qt`'s stated
position is that C++ owns the `Host` for the life of `QApplication::exec` and **Rust never calls a
Qt object**; the crate holds one hand-written `unsafe` token and `tests/unsafe_position.rs` asserts
it. `QAccessible` is a Qt object and a `QAccessibleInterface` subclass is C++, so publishing
through it means a second implementation of §14.8.4 in the one language in this tree that has no
test harness — or a table of new `unsafe extern "C++"` declarations.

## The decision

**All three windows drive `viewer-accessibility`.** The argument is the standard's rather than the
toolkits':

- §14.7.3's role map is a `shall` on this reader, and §14.8.4's types are then mapped onto a
  platform vocabulary **once**. Route 1 inserts a third vocabulary between ours and AT-SPI's — the
  toolkit's — which can only lose, and it does so differently in each toolkit, so two of this
  project's windows would say different things about one document. The owner's rule that all three
  hosts stay level is a rule about *what a reader gets*, and a second mapping breaks it in the one
  place a person cannot check for themselves.
- `tools/state.sh accessibility` is a **ratchet** over the corpus (ADR 0425). It measures one tree
  builder. A second one would be unmeasured by construction.
- `accesskit_unix::Adapter::new` takes three handlers, `set_root_window_bounds` two rectangles and
  `update_if_active` a closure. **It names no toolkit at all**, so both native hosts reach it from
  safe Rust and `viewer-qt`'s one-token invariant is untouched — checked, not asserted.

**What it costs, measured rather than predicted.** `accesskit_unix` opens its own connection to the
accessibility bus and calls `Socket.Embed`, so the process publishes a **second application root**.
Walked with `busctl` from the registry root under `Xvfb`: `applications on the accessibility
desktop: 2`, both named `pdf-viewer-gtk`, one carrying the toolkit's widgets and one carrying
§14.7's tree. That is the price of this decision and it is written here rather than discovered
later; the alternative price was a second mapping of the standard.

## What was built

- **`viewer_accessibility::Reading`** — the six queries a host asks and the assembly of them.
  `viewer-ui` held a hundred and fifty lines of this in its own `access.rs`; three hosts would have
  been three copies. It is in this crate rather than `viewer-host` because nothing may depend on
  `accesskit_unix` by accident, and `viewer-ffi` depends on `viewer-host`.
- **`viewer_accessibility::Showing`** — the page and the viewport, which is what decides whether the
  expensive half is asked for at all.
- **`viewer_accessibility::republishes`** — which commands change what §14.7's tree says.
  `viewer-ui` had this as a private `matches!` since ADR 0425; **a released pointer joined the list
  here**, because the native hosts delegate §12.7's widgets and an assistive technology's click
  arrives as a press and a release rather than as an `Edit`.
- **`Bridge::speak`, `Bridge::attended`, `Bridge::wait_millis`** — and `viewer_host::trace::Topic::Access`.
- Both native hosts: the bridge up after the first frame, the tree published on a page turn or a
  resize, `Act` matched **exhaustively** in all three windows, and §9.10.2's readback shortfall
  crossing with it.

## Two platform differences, named rather than levelled away

**Only Qt can say where its window is.** AT-SPI reports a node's extents in screen coordinates and
this program's are the viewport's, so the adapter needs the window's origin. `QWidget::frameGeometry`
and `QWidget::geometry` are both in screen coordinates and cross as `QtPlace` on a `moveEvent`.
GTK4 exposes a toplevel's position **nowhere** — not on `GtkWindow`, not on `GdkSurface`, not on
`GdkToplevel`, and `gtk4-sys` has no symbol for one, which is a deliberate consequence of Wayland
having no such concept. So `viewer-gtk` does not call `Bridge::placed` and says so on the `access`
topic the moment the bridge comes up. This is the owner's parity rule working as intended: a real
platform difference is recorded, not hidden.

**Neither native host can be woken from another thread, and `viewer-ui` can.** `accesskit_unix`
calls back from its own D-Bus thread; winit's `EventLoopProxy` is `Send`, a `Weak<RefCell<Host>>`
is not, and reaching `QApplication` from a foreign thread would cost `viewer-qt` a second
hand-written `unsafe` token. Neither `glib` nor `gio` offers a safe UNIX-descriptor source in the
versions this tree binds, so a self-pipe is not open either. Both native hosts therefore **poll**,
at an interval `Bridge::wait_millis` decides for both of them: `LISTEN_MILLIS` until a client
attaches and `POLL_MILLIS` after. The slow interval is what closes a hole a fast-only poll would
have hidden — a client's *first* act is an AT-SPI call, so a host that armed nothing until it saw
one would never see one.

## What the bus said

Under `Xvfb`, with a session bus, `at-spi-bus-launcher`, `at-spi2-registryd` and `busctl` as the
client (`doc/verify.md`'s recipe):

- `doc/PDF20_AN001-BPC.pdf` in both native hosts: two page nodes, 32 elements, the same tree
  `viewer-ui` publishes.
- `annotation-button-widget.pdf`: 9 nodes declaring `click` in all three hosts, the same 9.
- **`DoAction` on all seven `Link` elements of ISO 32000-2's cover opens the same URIs in all
  three**, read back off each host's own `link:` line.
- `Component.ScrollTo` answers true and moves nothing where the element is already on the screen,
  which is the designed answer.
- The launch order is the rule and it was caught by the instrument: the first version of the GTK
  host printed `accessibility bridge up` at 1.656 s and `first frame on the screen` at 1.671,
  because `refresh` runs once on the first allocation before the document is even open. It is
  gated on `presented` now.

## The one thing this round found and did not build

**A click on a §12.7 widget does nothing in a host that delegates it.** `viewer-ui` toggled three
of the nine actionable nodes on `annotation-button-widget.pdf`; both native hosts toggled none,
while `DoAction` answered `true` in every case. The cause is not the bridge: `viewer-ui`'s click is
four steps and one of them is `App::toggle_button`, seventy lines of Table 227, Table 229 bit 15
and §12.7.5.2.3 built on `Query::FieldAt` and `Query::Fields`. A host that sent `Command::Delegate`
has a real `GtkCheckButton` over the widget and a synthetic press at a page coordinate goes past it.

So the two native hosts **refuse it by name** — `viewer_host::delegated_click`, 9 of 9 on that
document, with the field's own label in the sentence — rather than appearing to work. Building the
other half is `doc/todo/31`'s next item and needs **no message**: both queries exist, which is why
`viewer-ui` needed nothing added to do it. It is also what refutes `tools/state.sh windows`'
standing reading of `Query::FieldAt` as "not a debt, a delegation": that argument rested on a press
landing on a control, and an assistive technology's press does not.

## And one instrument corrected, by its own second direction

Moving the six queries into `viewer-accessibility` made `tools/state.sh windows` report
`viewer-ui` reaching **fewer** queries than before, with `AccessibilityTree` and `Readback`
credited to no window at all on the day all three began asking them — because the section's
population was the host crates plus `viewer-host`, and the crate a host's non-toolkit half had just
moved into was not in it. The population is `viewer-accessibility` too now. Trap 11 caught by the
`SPENT` half of ADR 0603's own check, which is what that half is for.
