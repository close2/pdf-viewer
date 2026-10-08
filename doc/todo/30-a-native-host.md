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

Two crates lift `deny(unsafe_code)` and no more: `viewer-qt`, for `#[cxx::bridge]`'s expansion,
with **one** hand-written token; and `viewer-ffi`, whose `src/abi.rs` holds one lint lift, an
`#[unsafe(no_mangle)]` attribute per entry point (`tools/state.sh hosts` counts them) and **no
`unsafe` block at all**. Each has a test that reads its own sources back. `viewer-ffi`'s also
asserts that `pdf-syntax`, `pdf-model`, `pdf-font`, `pdf-render`, `render-cpu`, `viewer-core` and
`viewer-host` still hold `#![forbid(unsafe_code)]`. A third name appearing in either list is a
change to a rule the project owner stated and belongs in an ADR.

## What `viewer-host` is, and what it is not

`viewer-host` holds a host's non-toolkit half: the panel rows, §12.7.5's field as the control it is,
§12.7.6.4's file policy, the launch timeline, the key table, the clock, the drawing queue, the
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
operation.** It performs none of the six a restriction decides, supplies no files and follows no
links, so it owes nothing. Each gap a reader notices is answered: there is no `--restrictions=`
because there is nothing to decide, `Event::Asking` is refused out loud because nobody could answer
(ADR 0814), and `--links=` and `--remote-documents=` are both pinned to `refuse` because there is no
dialogue and no click handling (ADRs 1155, 1227). For a file a document names it answers
`Command::Supply` with no bytes and tells the person by name (ADR 1227). It holds one document: a
second would first have to decide whether two documents share one worker's confinement (ADRs 0713,
1190). What arrives with each future feature, in the feature's own round:

- **selection or copy** → `--restrictions=` with that row, and the ask dialogue on the modal
  machinery ADR 0718 already loads lazily;
- **click handling** → `--links=` and `--remote-documents=` with real `Ask` arms, replacing the
  pins;
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
- **Table 29's arrangements, §12.4.4's presentation and its clock** are shared policy
  (`viewer_core::layout`, `viewer_host::Presenting`, `viewer_host::Clock`), and each toolkit
  supplies the wall clock (ADRs 0441, 0442, 0470, 0473). The gap between pages is
  `pdf_render::SURROUND` in all three: GTK applies it through a `CssProvider` rule and Qt through
  `PageArea`'s palette, because a toolkit has no notion of the surface a document is laid on
  (ADR 0446).
- **A copy is `Command::Copy`**, and `Event::Copied` carries both of §14.8.2.5's orders to
  `viewer_host::copied` (ADRs 0519, 1144). Qt's copy goes out as a `QtUpdate` flag, because Rust
  never calls `QClipboard`.
- **The six panels are `viewer_host::Tab`**, down the side in both toolkits because six labels do
  not fit across a sidebar. §12.3.4's miniatures decode on demand in a virtual list
  (`viewer_host::Miniatures`), never at the launch (ADR 0564). §12.5.6.14's popup windows are an
  unmeasured `GtkOverlay` child and a layout-free Qt widget, because a popup's `/Rect` is the
  document's and must not size the window (ADR 0613, trap 19). A note's `/RC` is drawn formatted in
  all three from `viewer_host::popup`'s shared readings of each run (ADR 1642) — `quorra` setting each
  run in its own face, ordering a right-to-left paragraph across its runs and drawing its spacing and
  scales (ADR 1654), all three setting a paragraph's tab stops and a list tag at its start edge, and
  `quorra-qt` the scales through a document built format by format; Pango states no per-run glyph
  scale, so `quorra-gtk` says one (ADR 1666); all three draw a stop's leader, the toolkits
  painting it over their own line from the tab's extent there (ADRs 1679, 1722); all three lay a right-to-left paragraph's tabs leftward, the toolkits from stops
  handed per width as distances from the line's right edge, and both toolkits say a decimal stop
  there, which each places as though the number read right to left (ADR 1690) — and the C ABI hands
  the runs, the stops and each stop's leader over, with the grid the windows break a leader on
  (ADRs 1655, 1667, 1726). A text note is retyped in its window in all three — a text view placed
  over it in the toolkits, a caret a press places and the arrows move in `quorra`, at the end of a
  note drawn from `/RC` (ADR 1739) — and a reply stating no popup is a comment in its note's window
  (ADRs 1726, 1727). §12.3.5's collection is shown in all three. A signature's published policy is
  fetched in all three under the submissions level and a bound copy opened beside, declined in
  `quorra-confined`; a C caller fetches the copy and `quorra_event_policy_bind` binds it (ADRs
  1738, 1753). A widget's and a page's `/AA` scripts are raised in all three, a press on a
  toolkit's own control included (ADR 1752).
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

- **`quorra` shows what would print and has no printer of its own** (ADR 1180 section 6), where
  `quorra-gtk` prints through `GtkPrintOperation` and `quorra-qt` through a `QPrinter`.
- **§12.5.6.6's free-text drag is refused by name in the two native windows**, because authoring
  that annotation is a drag mode plus an editor; it is `doc/todo/33`'s.
- **`viewer-ffi` cannot stop a draw it has handed a caller.** A C caller is told to move the request
  to a thread of its own, so the structure is right; what is missing is an entry point to raise a
  flag with, which is a header change and a levelness question of its own.

## Decided rather than owed

- **`viewer-ffi` has no keyboard table.** A C caller places its own toolkit and owns its keyboard,
  and `include/quorra.h` says so where it mentions §12.5.1's tab key. `quorra_key_meaning(key,
  shift, presenting)` with a `QUORRA_KEY_*` enumeration would hand `viewer_host::keys` over, with a
  count beside the enumeration because a C caller cannot fail to compile; it is a decision about the
  ABI's surface, and nothing is blocked on it.
- **Ctrl + C inside a field is the toolkit's binding** in the two native windows, because they place
  a real `GtkEntry` and `QLineEdit`. `viewer-ui` draws its own field and makes the call the page's
  copy makes.
- **`quorra`'s drawn choice list takes no keyboard**: Up, Down and Enter would be this host's
  convention, and no clause states one.
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
