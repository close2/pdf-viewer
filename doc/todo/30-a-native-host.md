# A native host, then the C ABI

Status: **all three built** — `crates/viewer-gtk` (ADR 0244), `crates/viewer-qt` with
`crates/viewer-host` beside it (ADR 0246), and the C ABI, `crates/viewer-ffi` (ADR 0247). How much
of `Command` and `Query` a C caller reaches is printed by `tools/state.sh hosts` (ADR 0509); every
`Query` variant reaches a symbol, and a test that matches exhaustively over the enum keeps it so
(ADR 0576). What each window reaches, and whether each gap is a debt, is printed by
`tools/state.sh windows` (ADRs 0577, 0603).
Priority: 30 — what is left is *surface* rather than architecture, and the file says which
Code: `crates/viewer-gtk`, `crates/viewer-qt`, `crates/viewer-host`, `crates/viewer-ffi`

## The goal, stated by the owner

The viewer is to be **embeddable in native frameworks** — Win32/WinUI, AppKit, KDE/Qt, GTK — not
built on a cross-platform toolkit. `viewer-core` is that interface: `Command` in, `Event` out,
`Query` → `Answer` beside them, with no type from a windowing or graphics library anywhere in its
API (ADRs 0116 to 0121, and `doc/ui-boundary.md` for the vocabulary and the three pixel tiers).

## The order, and why it was not negotiable

GTK4 through `gtk4-rs` first, Qt through `cxx-qt` second, `viewer-ffi` last: a C ABI is not frozen
until two Rust consumers have shaken the API out. The bridge to C++ is where the awkwardness showed
up, and none of it was the boundary's. The three amendments the two hosts named were taken before a
line of the ABI was written (ADR 0247): `pdf_render::RasterFormat` is exhaustive, `Answer::Outline`
is owned, and `Answer::Field` carries `pdf_model::view::ShownValue`, whose `obscured` keeps a
password field's bullets out of the value a host reads back and Table 231 bit 14's value out of a
saved file.

**A host has needed a variant widened more often than a message added.** A message added is a
channel that did not exist. A variant changed is a channel that was too narrow, and every consumer
then fails to compile until it says what it does — which is what `#[non_exhaustive]` would have
hidden. `Edit::SetField`'s value is `pdf_model::view::Entered` for §12.7.5.4's list box (ADR 0248),
and `viewer_core::Secret` holds a password so that a traced `Command` prints how many characters it
holds and not which (ADR 0545).

## Where the `unsafe` is

Three crates lift `deny(unsafe_code)` and no more: `viewer-qt`, for `#[cxx::bridge]`'s expansion,
with **one** hand-written token; `viewer-ffi`, whose `src/abi.rs` holds one lint lift, an
`#[unsafe(no_mangle)]` attribute per entry point (`tools/state.sh hosts` counts them) and **no
`unsafe` block at all**; and `pdf-vfs-ffi`, RFC 0003's C boundary for the file-system faces, whose
one `abi` module carries the lift and no PDF parsing. Each has a `tests/unsafe_position.rs` that
reads its own sources back, and `viewer-ffi`'s also asserts that `pdf-syntax`, `pdf-model`,
`pdf-font`, `pdf-render`, `render-cpu`, `viewer-core` and `viewer-host` still hold
`#![forbid(unsafe_code)]`. A fourth name appearing in either list is a change to a rule the
project owner stated and belongs in an ADR.

## What `viewer-host` is, and what it is not

`viewer-host` holds a host's non-toolkit half: the panel rows, §12.7.5's field as the control it is,
§12.7.6.4's file policy, the launch timeline, the key table and what a press comes to
(`viewer_host::decide`, ADR 2092), the clock, the drawing queue, the
popup's reading, the copy's order, the password policy and the restriction levels' sentences. None of
them names a toolkit type, and the C ABI takes `panel` too, because a native host on this boundary
is mostly not toolkit code. It is deliberately *not* in `viewer-core`: a mapping from three answers
into one row shape is a convenience for whoever draws a tree, not a statement about a document (ADR
0246). The third copy of a function is where two hosts stop agreeing, so a rule two hosts would
write goes here.

Adding `egui` buys a widget set for a large dependency and no architectural proof: winit + a GPU
*is* the unnative UI. The thing worth adding was the headless consumer, and it is there.

## The UI itself is work, and all three hosts stay level — the owner, 2026-08-20

> even though low priority, I think we should start investing time into the UI (and its API for the
> native versions).

**All three hosts stay level.** A feature lands on the boundary and `viewer-ui`, `viewer-gtk` and
`viewer-qt` all adopt it, rather than one being a flagship the others follow. That is roughly three
times the host-side work per feature and it is chosen deliberately: a feature living in one host is
a message nobody has tested. Being ahead or behind on a *binding* or a *panel* is not a thing a host
can be: `viewer_host::keys` and `viewer_host::Tab` are matched exhaustively in all three, so a key
or a panel added fails to compile in three places (ADRs 0526, 0564).

**The rule's subjects are the three hosts that hold the viewer in process, and `quorra-confined` is
not a fourth of them** (ADR 1190). It holds no `Viewer`, no document bytes and no parser, so a
feature that lands in `quorra` and not there measures nothing. What that window owes is decided by
the operation instead: **a host owes the control for an operation exactly when it can perform that
operation.** It performs none of the six a restriction decides, so it owes no `--restrictions=`,
and `Event::Asking` is refused out loud because nothing could set a level that raises it (ADR 0814).
It does hand a link to another program and read a file a document names, so it takes `--links=`
and `--remote-documents=` at the other windows' default, `ask`, and puts the question on the
flagship's `QuestionCard`, answered by Enter and Escape; a named file opens in place of the one
document it holds (ADR 2040). A click reaches the page as `Command::Pointer`'s two ends: a link inside
the document is followed, a field takes the keyboard, and a rendition's clip is handed over (ADR
1987). It sends a form and fetches an import named by a URI under `--submissions=`, `ask` by
default and on the same card: the worker composes the request, this side sends it through
`viewer_host::submit`, and the server's answer crosses back as `Command::Respond` for the worker to
import; a PDF answer is kept and said rather than opened (ADR 2076). For §12.7.6.4's import of a
file it answers `Command::Supply` with no bytes and tells the person by name (ADR 1227). It holds one document: a
second would first have to decide whether two documents share one worker's confinement (ADRs 0713,
1190). What arrives with each future feature, in the feature's own round:

- **selection or copy** → `--restrictions=` with that row, and the ask dialogue on the modal
  machinery ADR 0718 already loads lazily;
- **anything that writes a file** → the save question. The confinement is no excuse here: the
  *worker* has no filesystem, but the host holds one by design and opens the document itself;
- **§12.7.5.3's file-select control** → a file *chooser*, as the other windows have.
  `viewer-gtk` puts a `gtk4::FileDialog` behind an icon on the entry and `viewer-qt` a
  `QFileDialog` behind a trailing `QAction`, both inside the widget's own §12.5.2 rectangle;
  `quorra` takes the pathname the clause already makes the field's text, typed. Which controls may
  have one is `viewer_host::policy::may_choose_file`, asked where the affordance is offered and by
  `form::edit_of` when a path comes back, so the four levels attach in one place (ADRs 1216, 1240).

**The criterion for what is taken first** (ADR 0509) outlives any list: what a *reader* can do with
a document and cannot do here; then what costs no new message; then what makes the level-hosts
decision checkable. A toolkit floor does not rank an item last, but a round has to say what it
actually tried. A toolkit block is a claim to check: Table 234's `/TI` (ADR 0508) and Table 233 bit
19's editable combo box (ADR 0596) were each called a toolkit limit and were not. The ranking lives
in `tools/state.sh windows`, which prints each unreached variant with its reading beside the count,
so it cannot go stale between rounds the way a list in a file did (ADR 0603).

## What the three windows do that a toolkit made awkward

These are the places where a native host's shape differs from the tier-2 host's, and why.

- **Form controls are placed over the page** (`Command::Delegate`, §6.3.2.2): a `GtkEntry` or
  `QLineEdit`, a check box, a list, a combo box — the editable one as a `GtkEntry` beside a
  `GtkMenuButton` in GTK (ADR 0596). Table 234's `/TI` scrolls a list box from an idle in GTK, after
  layout, and with `scrollToItem` in Qt (ADR 0508). §12.7.5.2.3's at-most-one radio button is obeyed
  where `RadiosInUnison` is clear (ADR 0346). `viewer_host::ControlFit` says at which magnification
  every control fits its `/Rect`. `w` offers it and nothing takes it unasked, because a viewer that
  magnified a page because a form is on it would be answering a question nobody asked (ADRs 0346,
  0436).
- **Every click on a field reaches the field.** A person's click lands on the toolkit's control. An
  assistive technology's click arrives as a point on the page, so `viewer_host::form::clicked`
  decides it: a toggle goes to the field directly (ADR 0630), and a text field or a choice gives the
  keyboard to the control placed over it (ADR 1566). §12.5.1's Tab walks Table 31's `/Tabs`, and
  the keyboard follows the walk into a control or onto the page, where Space or Enter presses a
  push-button the page draws (ADR 1357).
- **Drawing is off the event thread** (`viewer_host::drawing`, ADR 0668): a queue, one job in
  flight, and a token, because `viewer_core` drops a `RenderReady` whose token is not the one
  outstanding. It is a *pull* — `Drawing::interval` answers how long to wait and each toolkit arms
  its own one-shot — because C++ owns the Qt `Host` and Rust calls no Qt object. A window with
  nothing on the screen yet waits for page one from a one-refresh budget rather than polling
  (`Drawing::settle`, ADR 0678). Two hosts on one arrangement are a differential instrument: one
  host cannot tell its toolkit's cost from its own, and two disagreeing localise it in one reading.
- **Opening is off the event thread too, past `WARN`** (`viewer_host::Loading`, ADR 1811): the
  document's thread is waited for no longer than `drawing::WARN`, and a page still being read then
  is a bar — GTK's and Qt's own widget in the status bar, the title in `quorra` and
  `quorra-confined` — until the look that finds it read. **Page one is drawn while it is read** in
  both toolkit windows as in `quorra` (`viewer_host::Ahead`, ADRs 1855, 1867): from half a second
  into the read the window shows the page as it stands, placed where its first frame will be, and
  the first frame is finished from that drawing. **A turn's read is off the toolkit's thread** in
  both toolkit windows and the C window (`Viewer::read_out`, ADR 2344): the key's handler returns
  with the retained picture on the screen and a second turn waits for the first's read. What is
  still owed: `quorra`'s own turn, an arrangement that brings on two unread pages and a
  re-interpretation are read on the window's thread with no bar, and an in-process interpretation
  has no stop but the window's close. `quorra-confined` owes neither:
  every command goes to its worker on a thread past `WARN` and the reader's keys are held meanwhile
  (ADR 1827); what it owes is a worker that takes a command while it draws, so that a held key
  reaches the viewer sooner.
- **A long draw is shown as it is built** (`viewer_host::drawing::over_frames`, ADR 1825): past
  `GLIMPSE_AFTER` the drawing thread is glanced at every `GLIMPSE_EVERY` and the page part-way is
  painted where it belongs, in painting order because the strips ADR 0139 grants are the only exact
  tiles. What is still owed: `quorra`'s processor fallback and the confined window's host draw do
  not ask.
- **A zoom shows the last frame scaled until its own lands** (`viewer_host::stand_in`, ADR 1843):
  each page's pixels are painted into the extent `Query::PageGeometry` gives it now — Qt's
  `drawImage` into a rectangle, GTK's `GtkFixed` child transform — where the shape is the same, and
  the part-way picture replaces it when it comes. **A page with no frame at all** — turned back to,
  scrolled back into a column — is shown from `viewer_host::stand_in::Retained`, a 512-pixel
  reduction of its last finished frame at the document's ink, eight pages by default and
  `--proxy-pages N` in both windows, until its frame lands (ADR 1866). No window has a view rotation,
  so a turn has nothing to stand in for. **A page's own pixels are shown at a whole device pixel**
  — in GTK the surface's own pixel, at the fractional scale the compositor states (ADR 2250) —
  and a transition's faces are that picture (`viewer_host::clock::face_as_shown`), so the effect's
  last frame meets the page without a seam (ADR 1865). **A page zoomed until its largest window — the
  viewport and a quarter round it — keeps at most seven eighths of its commands or half its pixels
  is drawn as the window shows it**, and so is one past `viewer_core::MAX_PIXELS`
  (`viewer_host::oversize`, `Command::Oversize`, answered `Rendered::Listed`; ADRs 1892, 1930,
  1994). **A page drawn whole of which the window shows less than half the rows has those rows
  drawn first**, as a look of their own over the stand-in, and the page whole after as the frame
  (`Drawing::ask_showing`, ADR 1868). **The confined window draws a page past the bound too**, by
  the marks its worker sends or by the window its worker draws, as the window prices the two, the
  toolkits' rule applied to either (ADRs 1944, 1986, 2016, 2017, 2026, `doc/questions/A477`).
  What is still owed: a page
  never drawn before that a notch out brings on screen has no picture of any kind until it is
  drawn.
- **Table 29's arrangements, §12.4.4's presentation and its clock** are shared policy
  (`viewer_core::layout`, `viewer_host::Presenting`, `viewer_host::Clock`, and the turn and its
  parking in `viewer_host::presentation_pump`), and each toolkit supplies the wall clock (ADRs
  0441, 0442, 0470, 0473, 2300); `quorra`'s winit loop keeps its own turn, parks it on winit's
  occlusion and minimised state and keeps a presentation per tab, and GTK under Wayland parks on
  the compositor's `SUSPENDED` read by its value (ADR 2317). The gap between pages is
  `pdf_render::SURROUND` in all three: GTK applies it through a `CssProvider` rule and Qt through
  `PageArea`'s palette, because a toolkit has no notion of the surface a document is laid on
  (ADR 0446).
- **A copy is `Command::Copy`**, and `Event::Copied` carries both of §14.8.2.5's orders to
  `viewer_host::copied` (ADRs 0519, 1144). Qt's copy goes out as a `QtUpdate` flag, because Rust
  never calls `QClipboard`; a C caller's `quorra_selection_copy_text` reads back what the last
  granted `quorra_copy` released and nothing before it (ADR 2087).
- **The six panels are `viewer_host::Tab`**, down the side in both toolkits because six labels do
  not fit across a sidebar. §12.3.4's miniatures decode on demand in a virtual list
  (`viewer_host::Miniatures`), never at the launch (ADR 0564). §12.5.6.14's popup windows are an
  unmeasured `GtkOverlay` child and a layout-free Qt widget, because a popup's `/Rect` is the
  document's and must not size the window (ADR 0613, trap 19). A note's `/RC` is drawn formatted in
  all three from `viewer_host::popup`'s shared readings of each run — faces, right-to-left order,
  spacing and scales, tab stops with their leaders, a list tag — and the C ABI hands the runs, the
  stops and the leaders over (ADRs 1642, 1654, 1655, 1667, 1679, 1690, 1722, 1726); Pango states no
  per-run glyph scale, so `quorra-gtk` says one (ADR 1666). A text note is retyped in its window in
  all three, the editor starting at the character the press was on (ADRs 1739, 1770, 1782); a
  reply stating no popup is a comment in its note's window (ADRs 1726, 1727); and a carriage return
  followed by a line feed is one paragraph break in every note window, split once by
  `viewer_host::popup::paragraphs` (ADR 1798). §12.3.5's collection is shown in all three. A
  signature's published policy is fetched in all three under the submissions level, declined in
  `quorra-confined`, and `quorra_event_policy_bind` binds a C caller's copy (ADRs 1738, 1753). A
  widget's and a page's `/AA` scripts are raised in all three, a press on a toolkit's own control
  included, every window's console being its log (ADRs 1752, 1771); a choice field's selection runs
  its Table 199 `/K` once, with `event.keyDown` true where an arrow made it (ADRs 1786, 1787, 1799).
- **A password is asked in all three**, and a document with no pages or one that failed to open is
  said rather than shown blank (`viewer_host::cannot_open`, `no_pages`, ADRs 0545, 0564).
- **`?` shows `viewer_host::NOTICE`** in all three, because the compiled-in standard 14 font
  programs' licences require a binary distribution to reproduce their notices (ADR 0526).
- **A window holds several documents** (ADRs 1263, 1264, 1275): a `gtk4::Notebook`, a `QTabWidget`
  and a strip `viewer-ui` draws, with `viewer_host::Documents` as the bookkeeping. `Command::Beside`
  names a document §12.6.4.3's `/NewWindow true` would open beside the one showing. Ctrl + O is
  `WindowAct::OpenDocument`, and every path a person names becomes bytes in
  `viewer_host::open_chosen`, where the levels attach.
- **A screen reader is published to by AccessKit in all three** (ADR 0623), and `doc/todo/31` is
  what it still owes.
- **The reader's three policy words are every window's** (ADRs 1580, 1581): `--trust-anchors` and
  `--accept-unknown-revocation` (`Command::Trust`), `--reference-files` (`Command::References`), and
  `--reader-name`, `--reader-title`, `--reader-organisation` and `--interface-language`
  (`Command::Audience`). `viewer_host::ReaderWords` reads them for `quorra`, `quorra-gtk`,
  `quorra-qt` and `quorra-confined` alike, rather than each toolkit's own parser, and its
  `commands` are sent on the document's thread before the first `Command::Open`; the confined window
  carries them across its wire and sends them again to a worker that replaces a dead one.
  `tools/drive-windows.sh`'s steps 34 to 36 drive each word with and without it.

## What is left, and none of it is architecture

Ranked by ADR 0509's criterion; `tools/state.sh windows` prints the window rows with their readings.

- **A note typed into writes a line feed where §12.5.6.2 asks for a carriage return**: "When
  separating text into paragraphs, a CARRIAGE RETURN (0Dh) shall be used". `quorra`'s Enter inserts
  `\n`, and the toolkits' editors hand back their own line feeds with `Edit::SetNoteText`. The fix
  touches all three windows' editors and their offset arithmetic (ADR 1798 section 3).

- **ISO 19005's conversion is chosen once per window** (`--archive <target>:<profile>`, or the
  card or dialogue Shift+A puts up where nothing was chosen, ADRs 1954, 1966): a window that has
  chosen converts to that choice for its life, and choosing again needs a new window.
- **`quorra` shows what would print and has no printer of its own** (ADR 1180 section 6), where
  `quorra-gtk` prints through `GtkPrintOperation` and `quorra-qt` through a `QPrinter`.
- **§12.5.6.6's free-text drag is refused by name in the two native windows**, because authoring
  that annotation is a drag mode plus an editor; it is `doc/todo/33`'s.
- **A Windows-native .NET host joins the level hosts through the C ABI** (the owner's `A287`, ADR
  2048), and RFC 0010 proposes it. Its C# declarations are `hosts/windows/Quorra.Interop/`, held
  to `include/quorra.h` by `crates/viewer-ffi/tests/header_and_interop_agree.rs` (ADR 2054), so an
  entry point added to the header is written there too. Every `Command` reaches the ABI (ADRs
  2060, 2061), a page
  past `MAX_PIXELS` drawn a window at a time on the processor among them; **the key table is handed over**: `QUORRA_KEY_*` and `QUORRA_ACT_*` with their counts, and
  `quorra_key_press`, which acts on a press where every window acts and returns what only a window can
  do, its modifier named by role (Ctrl here, ⌘ on macOS) (ADR 2072); `quorra_window`, RFC 0010's
  N2, runs the loop the toolkit windows share and hands over the panels' rows, the controls and the
  marks over the page, with their commits (ADRs 2088, 2104), opens under the levels a host states
  (`quorra_settings_*`, read as the four binaries read their words, ADR 2120), and puts the archive
  chooser, a script's dialogue and a print grant to its caller as records it answers (ADR 2121).
  Control and the wheel cross as `quorra_window_zoom_wheel`, travel in a unit whose fraction of a
  step the window carries (ADR 2152). The reader's other words — anchors, target documents, the
  audience, the conversion — reach it on the same handle, read as the binaries read them through
  `viewer_host::policy::Words`, and a file a directory holds that does not take is a `SAID` record
  (ADR 2156); the print key's decision is `viewer_host::print_key`, every window's (ADR 2157). The
  launch — the open on its thread, the reader's commands and refusals before the document, the first
  size's wait and join — is `viewer_host::native_window::Launch` and `NativeWindow::sized` for the
  three windows, each toolkit's part a typed parameter (ADR 2172); a layer drawn for a named reader
  reaches the C window on the panels record and its frames, which `drive_a_window.c` reads (ADR 2173). N3a, the WinUI 3 window the owner chose
  (`doc/questions/A501`), is `hosts/windows/Quorra.App/`: the page on the processor lane, the marks
  over it in Windows' colours, a control over each widget whose appearance it draws itself, the
  title, the status line, the keys, the pointer, the wheel and Control with it, built by CI's
  `window` job and declining by name each record it has no dialogue for yet (ADRs 2136, 2137,
  2153). A repaint says what changed and the window copies only the pictures it calls fresh; Table
  231 bit 25's combs, the measured fit `w` sends, §12.5.6.14's popups and §12.9's traced path are
  drawn (ADRs 2168, 2169). Its controls still lack bit 21's chooser (N4) and a popup's retyping; the
  panels and the cards are written (ADR 2185), compiled by the portable C# check; Windows layout
  and interaction validation, and N4's printer selection and spooling, remain owed.
- **A dmabuf texture in GTK and a `QRhiWidget` in Qt** — ADR 2049's step after the device path,
  which is built: the renderer is the library `crates/viewer-device/` (ADR 2052), and both toolkit
  windows draw on the device, page one by default, and read the frame back into the
  `MemoryTexture` and `QImage` they show (ADRs 2070, 2071). They wait for a profile showing the
  read-back and the copy hold the time: on the drive's small page they do not — the read back is 0.5
  to 1.3 ms of a 4.4 to 10.2 ms page and the copy into the toolkit's image about 1 ms (ADR 2124),
  and the medium the processor used to put under the read-back page is the device's own pass now
  (ADR 2150).
- **A C caller cannot stop a draw it was handed.** `quorra_window`'s loop abandons one on
  `QUORRA_ACT_ABORT_DRAWING` as every window does, but a request a caller rasterises itself is
  moved to a thread of its own, as `quorra.h` says, and has no flag to raise: that entry point is a
  header change and a levelness question of its own.

## Decided rather than owed

- **Ctrl + C inside a field is the toolkit's binding** in the two native windows, because they place
  a real `GtkEntry` and `QLineEdit`. `viewer-ui` draws its own field and makes the call the page's
  copy makes.
- **`quorra`'s drawn choice list takes Up and Down and not Enter**: the arrows are the toolkits'
  convention and what Adobe's `event.keyDown` names, and a pick is whole when it is made, so there
  is nothing for Enter to commit (ADR 1786).
- **`quorra-gtk` places no window**: GTK 4 gives a program no way to set its own position, so
  §12.2's `/CenterWindow` is said when the document opens rather than obeyed (ADR 1429).
- **`quorra` plays no sound for a script's `app.beep`**: winit has no system sound, so the window
  prints which of the reference's five was asked for and that none was played; GTK and Qt play
  their one sound for all five (ADR 1702).

## Two places the API forces a host into an awkward shape

Neither is an argument for changing the vocabulary today. The per-page answers are two shapes:
`Reports`, `Readback`, `Accessibility` and `Frame` name a page apiece, while `Fields`, `Popups` and
a selection's quadrilaterals are flat lists, and the enum does not say which rule a variant follows.
And `Command::Delegate` is a policy about a *document* while `Query::Fields` answers about a
*screen*: the pair is safe only while both follow the arrangement, and if they ever disagree the
result is a form with holes and no report anywhere.
