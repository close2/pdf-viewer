# The UI boundary — `viewer-core`, its vocabulary, and the three pixel tiers

Status: **built, shaken out, and frozen.** Read by: anybody writing a host, adding a `Command`,
`Event` or `Query`, or asking what the crate boundary permits. `doc/HANDOVER.md`'s reader table
points a round writing a host here, and ADRs 0116 to 0121 are the argument.

**Frozen means one test for a new message**: it carries a question a host cannot answer for
itself, or a value only a host has — never a second way to say something a host can. ADR 0247
states the condition the freeze waited on, *"Do not freeze a C ABI until two Rust consumers have
shaken the API out"*; it was met, and the three amendments `doc/todo/30` named were taken.

**Before a message, this boundary prefers three other mechanisms**, and the lists below show each
used many times:

- **a variant changes shape**, where a host needs two things a variant carried one of — every
  consumer then fails to compile, which is what nothing being `#[non_exhaustive]` is for;
- **a field is added** to a value that already crosses, where no host could derive it;
- **nothing is added**, because the channel existed and what was missing was hosts driving it. The
  decision that is the same in every window then goes into `viewer-host`, never onto this boundary:
  *which sentence a window is obeying* is shared, and `GtkWindow::fullscreen` against
  `QWidget::showFullScreen` against winit's `set_fullscreen` is what a toolkit is (ADR 0470).

**What a change costs is known before it is made.** Every consumer that matches the enumeration
exhaustively fails to compile. A C consumer cannot fail to compile, so the C ABI gains entry points
and a constant per event kind, and a caller checks `QUORRA_EVENT_KIND_COUNT` against its header
with `quorra_abi_check` (ADR 0250); a question added fails `viewer-ffi`'s
`tests/every_query_reaches_the_abi.rs` instead, which matches `Query` exhaustively (ADR 0576).
`QUORRA_ABI_VERSION` moves only when a shape an old caller passes changes — an entry point or a
struct *added* is one an old caller never uses. `grep -E 'define QUORRA_(ABI_VERSION|EVENT_KIND_COUNT)'
crates/viewer-ffi/include/quorra.h` prints both, and `tools/state.sh hosts` counts the entry points.

#### Messages added, and what each passed

The enumerations themselves are the list — `Command`, `Edit`, `Rendered` and `Purpose` in
`crates/viewer-core/src/command.rs`, `Event` in `event.rs`, `Query` and `Answer` in `query.rs`.
What follows is why each later addition passed the test.

- **`Command::Find` and `Event::Searched`** — Annex O's document-wide `search`, one page per step
  because rule 4 forbids blocking and rule 3 leaves no clock to budget with; `Query::Find` stays the
  page showing, out of a readback that exists (ADR 0250).
- **`Command::Present(PresentationMode)`** — §12.4.4.2's condition, which no state machine over a
  file can deduce (ADR 0316).
- **`Query::Highlight`** — Annex O's `highlight`, whose rectangle no host can place because the
  fragment arrives undecoded inside `Command::Open`, and whose look the annex leaves to the
  processor ("[t]he nature of the highlighting is implementation-dependent") (ADR 0357).
- **`Command::Layout(PageLayout)`** — Table 29's entry states the layout that "shall be used when
  the document is opened", and only a host knows the reader has since changed it; `Answer::Frame`
  became a `Vec<FrameView>` with it, and C gained `quorra_frame_count` to *ask* how many (ADR 0441).
- **`Query::View` and `Command::View(Viewing)`** — every command that makes a view is relative and
  the viewer clamps it, so no host can say where the reader is. **A host does not compose a
  `Viewing`; it echoes one** (ADR 0737).
- **`Edit::Attach`, `Edit::Detach`, `Command::Answer`, `Event::Asking`, `Event::Warned`,
  `Event::AttachmentsChanged`, and `RestrictionLevel`'s `Ask` and `Warn`** — the owner's *"I am
  missing adding embedded files"*, and each of `CLAUDE.md`'s two further levels arriving with the
  message that makes it a level rather than a variant nothing answers (ADR 0814).
- **`Command::Restrict` with a level per `pdf_model::restriction::Operation`, and `Command::Copy`
  with `Event::Copied`** — Table 22's positions have different subjects. A query could not carry
  the copy: `Query::Selection` is what a drag asks to draw a highlight, and a query raises no event
  for the *ask* level to wait on. `Event::Copied` carries both of §14.8.2.5's orders, chosen
  between once in `viewer_host::copied` (ADR 1144).
- **`Command::Trust(TrustPolicy)`** — RFC 5280 section 6.1.1's trust anchors, which that RFC makes
  the reader's policy; a file naming its own would vouch for itself (ADR 1076).
- **`Command::References(ReferenceFiles)`** — §8.10.4's target files as bytes, matched by §14.4's
  identifier and never by the path a document wrote; `Viewer::reference_refusals` is a method,
  because a refused file is a fact about the host's input and about no document (ADR 1101).
- **`Command::Audience`** — §8.11.4.4's user and language categories, which no file may assert
  (ADR 1106).
- **`Query::Measure` and `Answer::Measured`** — §12.9's traced path, mapped by ADR 0118's arithmetic
  and read through Table 267; no `Command`, because §12.9 states no state for a viewer to be in, so
  the mode is `viewer_host::Measuring` (ADR 1191).
- **`Command::Separations(bool)`** — §10.8.3's simulation, which §10.8.1 leaves "up to the
  processing software" (ADRs 1227, 1228).
- **`Command::Beside(Option<DocumentId>)`** — a name held in reserve for Tables 203 and 204's
  `/NewWindow`, deliberately not a field on `Command::Supply`, which would answer the weaker of the
  two entries (ADRs 1263, 1264).
- **`Command::Report`** — the one message whose reason is principle 2 rather than a clause: what a
  document says about itself is asked for once the frame is on the screen, as
  `viewer_host::report::Due` holds for every host (ADR 1044).
- **`Command::Archive` and `Event::Archiving`** — print's shape for a verb this crate cannot run:
  `pdf-transform` depends on this crate, so `viewer_host::archiving` runs the conversion for every
  host, and the confined window's wire refuses both as `Uncarried` (ADRs 1954, 1966, 1967).
- **`Query::TimerDue` and `Event::Beep`** — a script's timers, counted down in the milliseconds
  `Command::Tick` carries (rule 3), with one clock per window, and its sound; a tick advances
  §12.4.4.1's `/Dur` only under `Command::Present(On)` (ADR 1702).
- **`Command::Keys` and `Event::Console`** — a script's modifier keys, which only a host has, and
  its console requests (ADRs 1771, 1786).
- **`Command::Oversize(Oversize::Listed)`** — a host draws a page past `MAX_PIXELS` from its list
  and answers `Rendered::Listed`; the confined worker sends such a page's marks where they fit and
  refuses it by name where they do not (ADRs 1892, 1944).
- **`Command::Respond`** — §12.7.8's form data a server answered with, named by document because it
  arrives when the server answers; the network is a host's (ADRs 1291, 1527, 2076).
- **`Event::SignaturePoliciesPublished`** — §12.8.3.4.4's published policy copies, answered under
  the submissions level (ADR 1738).

#### Shapes changed and fields added

- `Command::Zoom` gained the viewport point to hold still (ADR 0166). `Answer::Field` gained
  §14.9.3's second name (ADR 0167) and then `pdf_model::view::ShownValue`, Table 231 bit 14's
  `obscured` beside the characters, which `Answer::Fields` shares (ADR 0247).
- `Edit::SetField`'s value is `pdf_model::view::Entered`, because Table 233 bit 22 lets a list box
  hold several items (ADR 0248).
- `Event::Extracted` carries the rest of the URI's fragment, for §O.2.1's "[a]ny remaining
  parameters after this parameter apply to the selected embedded file" (ADR 0431).
- `Answer::Selected`'s `text` is a `Cow`: borrowed inside a page, assembled across a page boundary,
  because §9.4.1's text position "shall not persist from one text object to the next" and so there
  is no document-wide offset (ADRs 0257, 0444).
- `Answer::Reports`, `Answer::Readback` and `Answer::Accessibility` carry one entry per page Table
  29's arrangement shows (`PageReports`, `PageReadback`, `PageStructure`). Two pages' structure
  trees share no numbering (§14.7.5.2, §14.7.5.4) and §14.8.2.5 states no order between them, so
  `viewer-accessibility` gives each page one `Role::Document` node (ADR 0445).
- `Command::Restrict` carries a `RestrictionScope` — the window's `RestrictionPolicy`, or a
  `RestrictionOverride` the focused document departs from it in, kept in `open::Open` and met in
  `RestrictionPolicy::under` (ADR 1145).
- `Purpose` gained `RemoteDocument`, `NamedPage` and `ThreadDocument` on the `Event::NeedsFile` and
  `Command::Supply` that existed (ADRs 1228, 1239), and `Event::NeedsFile` gained `beside` (below).
- `Located::At`'s `display` is `pdf_model::geospatial::Displayed`, a projected `/DCS` as an easting
  and a northing (ADR 1678).
- `Rendered::Listed` — *the host took this request's own list* — is said about a page, where
  `Presented` is said about the viewer (ADR 0640, and the bullet below).
- `RenderRequest::ink` tells a host keeping pixels a re-interpretation of unchanged ink from changed
  ink, which the `Arc<DisplayList>`'s address cannot; `Open::stale` decides it (ADR 0457).
- `PageStructure` carries `widgets`, the widget annotations no element reaches (ADRs 1369, 1381),
  and `tagging`, which of `viewer_core::Tagging`'s four answers an empty or full list is (ADR 1393).
  An `AccessibilityNode` for a text field or a combo box carries its §12.7.4.3 value (ADR 1489),
  `value_lines` (ADR 1501) and `standard`, its type's meaning in the namespace §14.8.6.2's walk ended
  in (ADR 2028).
- `PopupWindow::replies` carries §12.5.6.2's threaded comments, which Table 172 makes a `shall`; what
  makes an annotation a reply is `/IRT` and `/RT`, which no host reads (ADRs 1090, 1727).
  `PopupWindow::rich` and `Comment::rich` are Table 172's `/RC` read into runs, so no host parses
  the markup (ADRs 1642, 1654, 1655); `PopupWindow::note` names the note `Edit::SetNoteText`
  retypes (ADR 1726).

Each of these crosses the confined window's wire, whose greeting moves whenever the wire's shape
does: it is `PDFVCF18`, the `MAGIC` in `crates/viewer-confined/src/protocol.rs`, and
`tools/conformance/tests/greetings.rs` holds this sentence to it.

#### Capabilities that added nothing

Each needed a channel that already existed; the shared decision is in `viewer-host`.

- §12.4.4's full-screen window: `Command::Present`, `Query::Opening` and `Query::Preferences`
  (Table 147 whole, `/NonFullScreenPageMode` among it), decided in `viewer_host::Presenting` (ADR
  0470); Table 147's window entries by `Presenting::place`, `fitted` and `centred` (ADR 1429).
- §12.4.4.1's clock in every window: `Command::Tick` (ADR 0135) and `Event::Transition` (ADR 0230),
  driven by `viewer_host::Clock` (ADR 0473).
- A click on §12.7.5.2's check box or radio button: `Query::FieldAt`, `Query::Fields` and
  `Edit::SetField`, decided in `viewer_host::form::Clicked` (ADR 0630).
- Drawing off the toolkit's thread, in `viewer_host::drawing`: `Viewer::rendered` drops a
  `Command::RenderReady` whose token is not the one outstanding, so a draw whose token the viewer no
  longer holds cannot change a pixel — which is when a host may raise `pdf_render::Interrupt` (ADR
  0668).
- The loading bar: `pdf_model::progress::Progress`, read by the window's thread beside the core, and
  `viewer_host::Loading`, which waits for the page no longer than `drawing::WARN` (ADR 1811).
- Table 29's arrangement in the tier-2 host: `Query::PageGeometry` answers `Answer::None` for a page
  the arrangement does not show (ADR 0442).
- A file chooser: `Edit::ChooseFile`, asked of `viewer_host::policy::may_choose_file` (ADR 1240).
- Control in the key table: `viewer_host::keys::Modifiers` and `ctrl_meaning` (ADR 1192).
- §12.9's rubber band, drawn by each window over `Query::Measure`'s points (ADR 1264).
- AT-SPI's actions, which resolve to places `Command::Scroll` and `Command::Pointer` already take
  (ADR 0425).
- Where the viewport sits in the window: `Reading::at` in `quorra-gtk`, a fact about the toolkit's
  layout (ADR 1516).
- `viewer_host::Documents`, the strip of tabs every window has (ADRs 1263, 1264).

**A password may be asked about a file a host supplied, and the answer goes to that file.** Table
203's, Table 209's and Table 253's `/F` each name a second PDF, and one that wants §7.6.4.1's
password raises `Event::PasswordRequired` under the name the core would use for it — the name
`Command::Beside` held out, or the document the link was in. The answer is `Command::Open` under
that name with that file's bytes; a named page, which opens under no name, is drawn into the
document that asked and answered with `Event::Closed`. `Event::NeedsFile`'s `beside` tells a host
asking a person where the file would open. A prompt cancelled is the file declined: `Command::Supply`
with no bytes ends the core's hold and gives back a held name with `Event::Closed`.
`viewer_host::Arrivals` is that bookkeeping for the windows (ADRs 1332, 1335, 1345).
#### The goal, stated by the owner

The viewer is to be **embeddable in native frameworks** — Win32/WinUI, AppKit, KDE/Qt, GTK — not
built on a cross-platform toolkit. Later it must support **text selection** and **annotation
editing**, possibly form-field text editing. `CLAUDE.md`'s exclusion list permits the writing that
implies.

#### What exists

Seven consumers: `viewer-ui`'s `quorra` (winit, tier 2), `viewer-core/tests/headless.rs` (no
display at all, tier 1), `viewer-confined`'s `pdf-view-worker` (a process with no filesystem, tier
1), **`viewer-gtk`'s `quorra-gtk`**, a GTK4 application, tier 1 (ADR 0244), **`viewer-qt`'s
`quorra-qt`**, Qt 6 Widgets through a C++ bridge, tier 1 (ADR 0246), **`viewer-ffi`'s C ABI** with
a hand-written header and two C programs that drive it, tier 1 (ADR 0247), and **`viewer-ui`'s
`quorra-confined`**, the window on the far side of `pdf-view-worker`'s pipe, which sends `Command`s,
reads `Event`s and takes its pixels from `viewer-confined`'s `Reply` without ever holding a `Viewer`
(ADR 0713). Over the C ABI sits a .NET layer, `hosts/windows/Quorra.Interop`, held to the header by
a test (ADR 2054). The toolkit hosts are what `doc/todo/30` calls the proof that the answers are
enough for *somebody else's widgets*: a `GtkListView` and a `QTreeView` over §12.3.3's outline, a
`GtkEntry` and a `QLineEdit` over §12.7.5.3's text field, a `GtkCheckButton` and a
`Qt::CheckStateRole` over §12.7.5.2.3's check box. The windows are held to one list of what a
reader does by `tools/drive-windows.sh`, which drives each under `Xvfb` and photographs every step
(ADRs 1453, 1478).

**What the bridges cost.** `#[cxx::bridge]` expands to `unsafe` and a `forbid` cannot be lifted, so
`viewer-qt` holds `#![deny(unsafe_code)]` with one exemption on `mod bridge` and **one hand-written
`unsafe` token** — the `unsafe extern "C++"` header, `cxx` asking the author to assert that the C++
declared there exists and is safe to call. `cpp/host.h` declares one function and names no Qt type.
`viewer-ffi` denies `unsafe_code` and lifts it once, on `mod abi`, with **no `unsafe` block
anywhere in the crate**. Each
crate's `tests/unsafe_position.rs` asserts its own shape, and `viewer-qt`'s also names every crate
in the tree that lifts the denial — the two C ABIs, this bridge and the three workers' close (ADRs
0246, 0247, 1945) — while `viewer-ffi`'s asserts that every crate touching PDF bytes still
*forbids* it.

```
host toolkit  ──Command──▶  viewer-core (no threads, no I/O, no clock)  ──Event──▶  host
 Win32/AppKit/Qt/GTK/winit  ◀──Answer──   query(&self, Query)   ◀──Query──
                                    │                                  ▲
                                    └──NeedsRender──▶ worker ──RenderReady──┘
```

- `Viewer::handle(&mut self, Command) -> impl Iterator<Item = Event>`, and
  `Viewer::query(&self, Query) -> Answer` beside it. **Selection cannot wait for a render round
  trip**, which is why the second channel is not a command.
- **And one method that is neither**: `Viewer::readback_cache(DocumentId) -> Option<ReadbackCache>`,
  how much of a document's readback the search cache holds, against what budget, with hits, misses
  and evictions; `quorra --trace=search` prints it. It is an *instrument*: **if a host would draw
  it, it is a `Query`; if a person would read it, it is a method** (ADR 0256).
- **The variants whose shape needs a reason beyond the lists above.** `Command::Open { id, bytes,
  password, fragment }` takes the password as a `viewer_core::Secret`, because the enum derives
  `Debug` and hosts trace a command by printing one (ADR 0545); the bytes as a
  `pdf_syntax::FileBytes`, whole in memory or open on disk and read where the file's offsets point
  (ADR 0809), a file on disk crossing the confinement as its open descriptor (ADR 0812); and the
  fragment as Annex O's text after `#`, undecoded, because splitting a URI is the host's and
  percent-decoding belongs to whoever knows which component it decodes (ADR 0209).
  **`Activate(ObjectId)` is what a panel row sends** — the object, not a payload, so the
  *document* decides what activating it means (ADR 0144). `Focused(FocusMove)` is §12.5.1's tab key.
  `Edit` is the eight edits `command.rs` lists. **`Answer { document, proceed }`** is the only
  command whose whole content is one bit only a person can supply (ADR 0814). The policy values a
  host supplies — `Restrict`, `Present`, `Delegate`, `Trust`, `References`, `Audience`, `Clock`
  (Table 166's entry, ADR 1160) and `Separations` — are facts about the window or the reader's
  machine and reach every open document and every one opened after; `Layout` is a way of looking at
  one file (ADR 0441), a `Restrict` override is one document's (ADR 1145), and `Beside`'s name is
  consumed by the one document opened under it (ADR 1263).
- **`Event::Refused { document, operation, notes }`** is an operation this reader declined *on the
  document's instructions*, deliberately not `Reported`: that one says what the **document** could
  not do, this what the reader's own policy did, and it carries the operation so that it can become
  a question (ADR 0212) — `Asking` and `Warned` are the same list under `Ask` and `Warn`, so
  `Refused` is the answer of exactly one level. `Reported { document, page: Option<usize>, notes }`
  with page `None` is what the *document* says about itself (§12.11, §12.8, §7.11.4).
  `Searched` is the only event a host has to *pump*: `remaining` above zero means send
  `Find::Continue` again, the division `NeedsRender` makes (ADR 0250).
- **`Query::Selection` answers in device pixels and produces no events**: a drag emits `Damage` and
  never `NeedsRender`, which keeps chrome off the rendering path. A selection crossing pages is
  joined by a newline no clause states, a documented choice. `Query::Frame`, `Reports`, `Readback`
  and `AccessibilityTree` answer one entry per page the arrangement shows, and `PageGeometry`
  answers for each of them, because its documentation already said that a page the arrangement
  does not show has no place on the screen.
- **Nothing is `#[non_exhaustive]`**, deliberately: it forces a catch-all arm on every host, and a
  catch-all arm is where a message added later is ignored in silence. The rule binds a
  *vocabulary*, so `viewer_confined::ConfinedError` is `#[non_exhaustive]` on purpose: its
  population is the kernel's, and what a host decides about one is whether another worker is worth
  starting — `viewer_confined::Resume`, two arms, closed (ADRs 0734, 0737). **The rule reaches types
  this crate does not declare**: `pdf_render::RasterFormat` crosses inside `Rendered::Raster` and
  `Answer::Frame`, so it is not `#[non_exhaustive]` either (ADR 0247).
- **This crate interprets; the host rasterises.** `NeedsRender` carries an `Arc<DisplayList>` and a
  `TargetSpec`, so a zoom or a scroll re-rasterises *without re-interpreting* — asserted by pointer
  equality of the list in `zooming_rasterises_again_without_interpreting_again`.
- **A stale token is dropped**, so a page turned mid-render cannot be overwritten by the frame the
  previous page produced.
- `Rendered::{Raster, Presented, Listed, Failed}` is where tier 1 and tier 2 differ and the only
  place they do. Two say the viewer holds no pixels, and not the same thing: `Presented` is
  remembered for the life of the viewer and takes `MAX_PIXELS` with it, `Listed` is forgotten with
  the page and leaves the budget alone (ADR 0640).

#### Editing, the form, and the chrome

**Edits are entries in a log beside the immutable document** (rule 1, and the edit log below), and
§7.5.6's incremental update writes them.

- **`Edit::Markup`** marks up what is selected in one of §12.5.6.10's four ways; `ViewState::add_markup`
  builds the annotation and the update appends its reference to the page's `/Annots`. `Page`
  carries its own `ObjectId`, because an edit is filed against a page and the interpreter may not
  walk the page tree, and the log records what was *done* rather than what was asked for, because
  undo is a replay (ADR 0196).
- **A caret**: `Query::Caret { at, offset }` answers with the segment the next character is drawn
  against, computed inside §12.7.4.3's own layout rather than from the text layer, because an empty
  field has no glyphs (ADR 0211). `Query::Offset` is its inverse, and `Query::FieldSelection` answers
  the shapes over a range — a third question rather than two carets, because §12.7.5.3's Multiline
  flag lets the layout break a value into lines a host cannot see (ADR 0225). Copy, cut and paste
  inside a field need no message: the characters are a slice of the value a host already read, and
  the edit is the `Edit::SetField` a keystroke sends.
- **§12.5.6.6's free text**: `Edit::FreeText { from, to, colour }` takes a drag's two corners, since
  that subtype "displays text directly on the page" and has nothing to be over; `Edit::SetFreeText`
  says what it says; `Query::FreeTextAt { at }` names the annotation, by object, because an
  annotation has no name to address it by. The caret questions answer for it unchanged, because
  §12.5.6.6 sends its subtype to §12.7.4.3 (ADR 0238). An annotation the *file* states is answered
  and retyped too, its `/AP` replaced by the update (ADR 0304).
- **§12.5.6.4's note, retyped in its own window**: `Edit::SetNoteText`, the note named by
  `PopupWindow::note`, because §12.5.6.14's popup "shall be used for editing the parent's text"
  (ADRs 1721, 1726).
- **§7.11.4's file**: `Edit::Attach { bytes, name, description, mime, home }` takes the file whole —
  rule 2 gives the host the filesystem, so what crosses is bytes and never a path — and `AttachHome`
  chooses §7.11.4.1's `/EmbeddedFiles` tree or §12.5.6.15's annotation under a viewport point.
  `Edit::Detach { name }` removes one by the key `Query::Attachments` answers with, and that answer
  is the **log's** view, which is why `Event::AttachmentsChanged` exists. The name is one namespace
  across both homes, under §7.9.6's "[t]he keys contained within the various nodes' Names entries
  shall not overlap"; `operation_of` answers
  `Operation::Annotate` for a page and `Operation::Modify` for the tree (ADR 0814).
- **A form**: `Query::Fields` answers every field with a widget on the page shown — §12.7.5's type,
  the flags of Tables 227, 229, 231 and 233, Table 234's items and selection, Table 232's `/MaxLen`,
  and the appearance-state name §12.7.5.2.3 makes a check box's value — so a host builds a real
  `QLineEdit`, combo box and check box rather than taking a form as pixels (ADR 0235). §12.7.5.2.3's
  "[t]he value of the V key shall also be the value of the AS key" binds the processor that changes
  `/V`, and is held.
- **The page drawn without those widgets' appearances**: `Command::Delegate(WidgetAppearances)` is
  §6.3.2.2's "unless otherwise instructed" as a value only a host can supply. It reaches `interpret`
  through `ViewState`, and removes exactly the widgets of the fields a host places a control over —
  `pdf_model::form::Control::is_delegable` and `viewer_host::ControlKind::is_placed` are one set,
  held together by a test. A push-button, a signature and a field stating no `/FT` keep their
  appearance, because no control replaced it; Space or Enter at §12.5.1's focus is
  `viewer_host::pressed`'s (ADRs 0245, 1357). `ViewState::of` is the other value, so every existing
  caller's display list is unchanged.

**Ctrl + wheel zooms in every window**, anchored at the pointer: a zoom has to hold a page point
`Open::origin` knows about and the scroll does not, because a page smaller than the viewport is
centred (ADR 0166), and each window counts a gesture's travel with `viewer_host::ZoomWheel` (ADR
1118).

**The panels are drawn** in every window, the list of them being `viewer_host::Tab` (ADR 0564).
`quorra`'s are `viewer_ui::chrome`, a `pdf-render` display list at an identity transform so that
both backends draw it (ADRs 0142 to 0145); `viewer-ui/tests/panel.rs` rasterises it with
`render-cpu` and **counts ink**, calibrated by deleting the glyph fill.

**Text this program sets for itself has a font.** `LoadedFont::standard` loads one of §9.6.2.2's
fourteen through `LoadedFont::load` against `Document::empty`, so the encoding is §9.6.5.2's and the
widths the clause's own on a machine with no fonts installed. A character none of the fourteen
states is asked of the machine by file (ADR 1382), on `viewer_host::machine_faces`' thread (ADR
1406), and `quorra-qt` registers the same file with Qt (ADR 1418). **What crosses the boundary is
the stored text, in logical order**: a toolkit shapes and orders a label itself, and `quorra`'s
chrome does the same through `pdf_font::shaping::Label` (ADR 1417).

**What the hosts still owe is in their files**: [30](todo/30-a-native-host.md), whose remainder is
surface rather than architecture; [31](todo/31-accessibility-host.md), whose four named edges are
closed (ADRs 0300, 0338, 0394, 0425) and whose remainder is upstream and geometric; and
[32](todo/32-presentation-player.md), a presentation player.

#### Crates

Each crate's `[dependencies]` in its `Cargo.toml` is the list; what is said here is the shape.

- `viewer-core` — the state machine. Depends on PDF crates only, never a toolkit, a graphics
  library or an I/O crate (rule 5). Owns the open-document set, page/zoom/scroll, links and §12.6's
  actions, the selection, search, the edit log and the render scheduler's *bookkeeping* (not its
  threads). A navigation history is still owed.
- `viewer-host` — toolkit-free, and what a second host discovered: the panel rows and the list of
  panels (`Tab`, ADR 0564), §12.3.4's miniatures a row at a time, §12.7.5's field as the control it
  is, the file, extraction and restriction policies, the launch timeline, the magnification at which
  a platform control fits the `/Rect` the document states (`fit::ControlFit`, ADR 0346), and every
  decision named under *Capabilities that added nothing*. Every window and the C ABI depend on it,
  and it is deliberately *not* in `viewer-core`: a mapping from answers into one row shape is a
  convenience for whoever draws, not a statement about a document (ADRs 0246, 0310).
- `viewer-device` — the device renderer: the chooser, the kept scene, the presenter thread, the
  stand-ins and the processor's composition, in a library that names no window (ADRs 2049 §2, 2052).
  Only a window may depend on it, which `stale::tests::nothing_but_a_window_depends_on_the_library_that_holds_the_stand_ins`
  holds for ADR 0378's rule 2, *nothing that judges a picture ever sees one* (ADR 2053).
- `viewer-ffi` — the C ABI, the last host `doc/todo/30` names. `tools/state.sh hosts` counts its
  entry points and how much of the vocabulary they cover (ADR 0509), and
  `tests/every_query_reaches_the_abi.rs` keeps every `Query` reaching a symbol (ADR 0576). ADR 0346
  added the pointer and the selection, §12.7's form and its edits, save and extract, the panels,
  §12.4.4's clock and the policy values; ADRs 2060 and 2061 the eight commands a toolkit's controls,
  clock, network and scripts need; ADR 2072 the key table — a caller translates its toolkit's key
  into a `QUORRA_KEY_*`, `quorra_key_press` acts on it as the windows act, and what only a window can
  do comes back as a `QUORRA_ACT_*`. **Above the vocabulary sits `quorra_window`** (ADR 2088, RFC 0010
  section 3.2a): `NativeWindow` run by the library, so a C host runs no event match of its own, and
  what a toolkit would do arrives as a batch of `QUORRA_OWED_*` records after each call, never as a
  callback. Beside them, `include/quorra.h` and `c/open_a_page.c` and `c/drive_a_window.c`, which a
  test compiles with `-Werror` and runs. Commands are functions rather than a tagged union (a
  union's size is part of an ABI; a symbol is not); events and answers arrive owned, so no borrow of
  the viewer crosses; a render request is an opaque handle a caller may move to a thread; a frame is
  copied into a buffer the caller owns. It links `render-cpu` so that a caller has something to draw
  a display list with, and `doc/verify.md` cross-checks it for the Windows and macOS targets, which
  neither toolkit host builds for. ADR 0247.
- `viewer-accessibility` — §14.7's tree onto AccessKit, and the only crate that names
  `accesskit_unix` and therefore an async runtime. The three windows depend on it.
- `viewer-ui` — `quorra` and `quorra-confined`, the tier-2 reference (ADR 2048).
- `viewer-gtk` — GTK4 through `gtk4-rs`, tier 1, `#![forbid(unsafe_code)]` held; nothing depends on
  it. ADR 0244.
- `viewer-qt` — the only C++ in the tree: Qt 6 Widgets through `cxx` and `cxx-qt-build`, tier 1, `#![deny(unsafe_code)]`
  with one exemption and one hand-written token. **C++ owns the host** for the life of
  `QApplication::exec`, the ownership inverted from `viewer-gtk`'s, which is why this one needs no
  `RefCell`. ADR 0246.
- **A frame the device refuses is drawn on the processor and said in the title** as well as on the
  terminal, in every window that draws on the device — `quorra` and `quorra-confined` (ADR 1466),
  and `quorra-gtk` and `quorra-qt` on the lane they read back from, in the same words (ADR 2070).
- `pdf-model` — has the text layer (ADR 0118). The edit log lives in `viewer-core` and reaches
  interpretation through `ViewState`, which was already the log §12.6.4's actions write to, so
  `interpret` needs no third input and rule 1 holds.

#### Five rules, and each has a reason that already exists in the tree

1. **`pdf_syntax::Document` is immutable, forever.** An edit is a log beside it, not a change to
   it — the pattern `view.rs` already uses for §12.6.4's actions. `interpret` stays a pure function
   of the document and the view state, which is what keeps the oracle's comparison meaning
   anything. Stated in `CLAUDE.md`, and held.
2. **No filesystem in the core.** The host supplies bytes; the core produces bytes. Not new
   policy — `Request::Import` and `Request::Resolve` already do exactly this, argued in ADRs 0090
   and 0104: a document naming a file is a document asking this machine for something, and
   whether to give it is not a rendering decision. **A file the host opened and handed over as a
   `pdf_syntax::FileBytes` on disk keeps the rule** (ADR 0809): the host chose the path and opened
   it, and the core reads through the handle it was given and never a path of its own. **The
   confined worker keeps it the same way** (ADR 0812): the descriptor it receives is the host's open
   file, which the filter lets it read and nothing else — not name, not `stat`, not open another.
3. **No clock.** §12.4.4's transitions and `/Dur` auto-advance arrive as `Command::Tick { millis }`.
4. **No threads the core was not handed**, and no blocking.
5. **No toolkit or graphics type in `viewer-core`'s public API.** Tier 2 (below) lives in a
   separate crate.

#### Pixels: three tiers, and interactive chrome is not pixels

| tier | what crosses | hosts | cost |
|---|---|---|---|
| **1** | a CPU `Raster` | everything, no unsafe | one copy per frame |
| **2** | a raw window handle; we drive wgpu/vello | anything producing `raw-window-handle` | a graphics dependency in the *binding* |
| **3** | the host's own GPU device/texture | one toolkit at a time | interop per platform |

**Define the interface at tier 1**, because that is what makes the core toolkit-free by
construction rather than by discipline. Page one goes to the graphics device (`CLAUDE.md`,
principle 2), so tier 1 is a portability choice and not a startup one. **Neither native host had a
choice about it**, for two different reasons. GTK4 gives a widget no native surface for tier 2 and
hands out no device for tier 3 (ADR 0244). Qt *has* `QOpenGLWidget` and `QVulkanWindow`, but they
are different widgets with different rules about being composited inside a `QSplitter` beside a
sidebar, while `QRhi`, which owns the device Qt draws through, is a private module a release may
change without notice (ADR 0246).

**The copy is measured on both, and it is only a `memcpy`.** `Raster` is `GDK_MEMORY_R8G8B8A8` and
`QImage::Format_RGBA8888` exactly, so there is no conversion in either direction:

| | first frame's copy, median of five | steady state, after a page turn |
|---|---|---|
| `gdk::MemoryTexture`, 2 687 100 B | 748 µs — 3.6 GB/s | **234 µs — 11.5 GB/s** |
| `QImage`, 2 765 244 B | 1078 µs — 2.6 GB/s | **231 µs — 12.0 GB/s** |

The two toolkits agree to within 4%, which is what "only a `memcpy`" predicts. Full-window repaint
of 1920×1080 RGBA at 60 fps is about 500 MB/s of copy — a few percent of a core even at the cold
rate, and only during smooth scroll. `TargetSpec::transform` already carries any tile offset, so
tiled repaint is the first lever if it matters.

**Interactive chrome crosses as geometry, not pixels.** Selection highlights, an in-progress
annotation rubber band, resize handles, a caret change at pointer speed and must not force a page
re-render. Emitting them as quads and points lets a native host draw selection in **macOS's
selection colour, KDE's accent, the Windows highlight brush**, with its own caret blink and focus
ring — most of what makes an embedded view feel native, and unreachable from finished pixels. A
slow render never blocks feedback.

**Which colour a platform parts with is a fact about the platform.** GTK 4.22 exposes no accent
colour to application code — `@accent_bg_color` is a CSS name libadwaita defines — so `viewer-gtk`
draws the selection fill and §12.5.1's ring in `gtk_widget_get_color`, the theme's own foreground.
Qt has `QPalette::Highlight` and, from Qt 6.6, `QPalette::Accent`, which KDE writes from the colour
scheme; `viewer-qt` says which it used (ADRs 0244, 0246).

| | crosses as | changes at | drawn by |
|---|---|---|---|
| page content | `Raster` | page, zoom, edit | us |
| interactive chrome | geometry | pointer speed | the host |

#### What each platform admits for the device

The owner's `doc/questions/A287`: "If possible gtk, qt, and dotnet should provide a way to still
render on the GPU." The tiers above are about what crosses; this is about who draws, and **the
device can draw behind tier 1**: a frame the device drew and read back is a `Rendered::Raster` like
any other, so the first device path in a native host changes no message (ADR 2049, G1 and Q1).
**That path is built in both toolkit windows**: `quorra-gtk` and `quorra-qt` draw a page on the
device and read it back in a lane beside the drawing thread (ADR 2070), page one included by
default (ADR 2071), with `--cpu` as the way out. ADR 2049 measured the read-back and the passes
after it, which are what a path without a copy would save, beside the copy above.

| host | without a copy | what it costs | status |
|---|---|---|---|
| `quorra-gtk` | a dmabuf as a `gdk::DmabufTexture` (GTK 4.14), which `GtkGraphicsOffload` may put on a subsurface. GTK 4 hands out no other surface | `viewer-gtk`'s `forbid(unsafe_code)`, `ash`, Linux and Vulkan only | priced, waits for a profile (ADR 2049) |
| `quorra-qt` | a `QRhiWidget` on Vulkan, whose texture wgpu draws into on Qt's imported device | `ash`, a second `unsafe` site, QRhi's "limited compatibility guarantees" | priced, waits for a profile (ADR 2049) |
| a .NET host | a WinUI 3 `SwapChainPanel` (wgpu 30's DX12 target), XAML controls composed over it | one surface entry point in `viewer-ffi`'s `abi.rs` | proposed (RFC 0010 §4) |
| a macOS host | a `CAMetalLayer` behind an `NSView`, made on the main thread | the same entry point, kind `CA_METAL_LAYER` | in mind, not built (ADR 2048) |

**All four share one first step, and it is taken**: `viewer-device` (above), from which a host
supplies a `Wake`, its frame trace and its clock, and makes its surface from whatever its window
converts to a `wgpu::SurfaceTarget` (ADRs 2049 §2, 2052). **And all four rule out a native child
window over which nothing composes**: the controls `Command::Delegate` places over the page have to
be drawn above the device's picture, which a Qt window container and a WPF `HwndHost` both prevent.

#### Two artefacts: a text layer and an edit log

**The text layer** (ADR 0118), with selection on it (ADR 0119). `Interpretation::text_layer` is one
`Placed` per character code: the range of the readback it produced and the quadrilateral its glyph
occupies, in the display list's coordinates. The box is the glyph's advance by Table 120's `/Ascent`
and `/Descent`, built for rendering modes 3 and 7 too, because an OCR layer under a scanned page is
exactly the text a person selects. It is kept unconditional, with its cost measured and written
down in ADR 0118. Its other consumers: **search** — `Query::Find` answers with the shapes
`Query::Selection` does, case-insensitively, and a match highlight is the same kind of thing as a
selection highlight, drawn by each window's find bar in its own colours (ADR 0250); and
**§14.8.2.5's logical order** — a selection is taken in content order, so `Tree::logical_range`
maps a range of the readback through the structure tree's order and `Query::LogicalSelection` is
what a host asks when a person presses copy. It answers nothing where the tree does not reach every
byte of the range, because a copy that silently dropped what the tree missed would be worse than one
handing back content order. **Not built on it**: word and paragraph selection. The caret is
deliberately not on it — §12.7.4.3's layout knows where the next character in a field goes, and the
text layer has no glyphs for an empty field (ADR 0211).

**The edit log** (ADR 0120). `Open::log` is what a person did, with a cursor; undo moves the cursor
and the surviving prefix is *replayed* rather than inverted, because an inverse would have to
remember what each edit replaced and would drift the moment two edits touched one field.
`ViewState::set_field` is the fourth statement about a field's value beside Table 226's `/V`,
§12.7.6.3's `/DV` and §12.7.8's imported one, and the last one made stands.

**It is saved** (ADR 0121): `ViewState::save` produces the file with §7.5.6's incremental update
appended, the host writes the bytes, and the producer's bytes are still there underneath, which is
the clause's whole point. §12.7.4.3's appearance stream is *written* rather than owed to the next
reader behind Table 224's `/NeedAppearances` — the bytes are the ones this program draws — and the
flag is set only for a widget whose stream this program could not produce whole (ADR 0130). A widget
with no `/AP` gets an object *added*, the half of §7.5.6's "changed, replaced, or deleted" that adds,
numbered from the larger of `/Size` and the highest number the cross-reference table holds, because
corpus documents understate the first. **An encrypted document is written** (ADR 0129): §7.6.2's
ciphers run on the way out through `decrypt_object`'s mirror, so the clause's exceptions are stated
once. §7.6.3.2's fresh random initialisation vector per AES string makes an encrypted save differ
from one run to the next, so its tests read the file back rather than compare it.

#### One boundary, not two: the confined process speaks this vocabulary

Principle 3 wants the interpreter and rasteriser confined, and **the confined process speaks this
vocabulary rather than a second protocol**: `viewer-confined` is `Command`/`Event` with `Raster`
payloads, the confined process owns document, interpretation and rasterisation, and the host
receives pixels and events (ADR 0218, `doc/todo/34`). **`viewer-core` needed no change for it**: the
five rules above describe a confined process — no filesystem, no clock, no threads it was not handed
— so a crate written to be free of a *toolkit* is free of a *kernel* too. What had to be decided was
on the transport's side: the questions that answer with `pdf-model` types are refused by name, and a
page draws on one thread because `glibc`'s allocator asks the kernel how many processors there are by
reading a file.

**For a tier-2 window, display lists cross** (ADR 0607). A device confined at any point in the
ordering dies on its first `ioctl`, and a process holding one cannot install Landlock under this
crate's descriptor ceiling, so the confined process ships marks rather than holding a device; the
raster payload stays, chosen per page by comparing two sizes the confined process can both compute.
The choice is `viewer_confined::Framed::payload`, on the *transport's* side of the boundary, because
which of two encodings a page crosses in is a fact about a pipe and not about what a host may ask
(ADR 0633). What this vocabulary owed it was **`Rendered::Listed`, per page** (ADR 0640): a worker
answering `Presented` for a page shipped as marks would set `holds_rasters` false for every page,
silence `Query::Frame` about the pages that must still cross as pixels, and stop `MAX_PIXELS`
bounding what a confined process is asked to draw — and inside a confinement an unbounded raster is
an abort rather than a refusal. `viewer_confined::protocol::Marks` holds each page's origin, from
`Query::PageGeometry`, and `encode_answer` joins the pixels the viewer holds to the marks the store
holds. `Answer::Frame` did **not** change shape: that would have shipped an unreachable arm to the
hosts that never answer `Listed`, and a C frame with a size and no bytes. A cancel stops the work the
*worker* does, so on the marks arm it covers the interpretation and not the drawing, which is the
host's.

**A page drawn as §10.8.3's simulated press crosses as marks like any other**, because what it adds
to a display list is data: the spot planes, each a companion list as the black one is, each
colourant's name and its step b) samples, and the two sampled conversions around the multiply
(`pdf_render::SpotSeparation`). The codec states it as the page's fourth blending shape, beside the
pair, the curve and the cube, and a companion list stating one is refused on the way in, so the
recursion stops one level down (ADR 1317).

#### Near, and far

What is left of *using* a document is [todo 33](todo/33-annotation-editing.md): a flag that waits on
a verb this program does not have. Editing the page's own text is far and deliberately out of scope.
