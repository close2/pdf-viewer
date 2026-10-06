# The AccessKit bridge: what is left of it

Status: **built, and read back on a real bus in all three windows** (ADRs 0214, 0623). What a
screen reader is told is counted by `tools/state.sh accessibility`, a ratchet and a `doc/todo/02`
§2 line (ADRs 0342, 0425). What is left below is a platform's, a toolkit's, or a population this
tree holds no witness of.
Priority: 31 — capability
Clauses: §12.5.2, §12.7.5, §14.7, §14.7.5.2, §14.7.5.3, §14.7.5.4, §14.8.3.3, §14.8.4,
§14.8.4.7.2, §14.8.4.8.3, §14.8.5.4.3, §14.8.5.4.5, §14.8.5.7, §14.9
Code: `crates/viewer-accessibility/` (`role.rs`, `tree.rs`, `bridge.rs`, `reading.rs`),
`crates/viewer-core/src/accessibility.rs`, `crates/pdf-model/src/structure.rs`,
`crates/viewer-ui/src/bin/quorra/access.rs`, `crates/viewer-gtk/src/access.rs`,
`crates/viewer-qt/src/access.rs` (`attend`, `speak`, `act`, one shape in three windows)
Code (continued): `crates/viewer-core/src/accessibility.rs`'s `places`, which is the one place the
routes to an element's rectangle are composed and is what all three windows and the census ask.
Instruments: `tools/state.sh accessibility` — the corpus-scale census of what a screen reader is
told — `pdf-model --example element_bounds_census`, `pdf-model --example cell_header_census`,
`pdf-model --example mcid_stream_census`, `pdf-model --example table_header_census`,
`viewer-core --example accessibility_cost`, and `tools/drive-windows.sh`'s AT-SPI steps

## What the bridge publishes

`viewer-accessibility` maps §14.8.4's forty-one standard structure types onto `accesskit::Role`,
and `accesskit_unix` puts the tree on AT-SPI: `Frame` → one `DocumentFrame` per page the arrangement
shows, named by §12.4.2's label → the page's own elements, found through §14.7.5.4's parent tree in
§14.8.2.5's order (ADRs 0325, 0342, 0445). §14.9.3's `/Alt` substitutes where the document states
one, and a `StatusBar` group carries what the page could not draw. A page's elements are matched per
content stream, because §14.7.5.2 makes an identifier unique "within its content stream", and an
appearance stream named by `/Stm` or `/StmOwn` is one of those streams (ADRs 0488, 0719).

- **A `TH` says which axis it describes**: Table 384's `/Scope`, or §14.8.5.7's assumption from the
  cell's place in the grid, published as `RowHeader` or `ColumnHeader` (ADR 0300).
- **A cell says which header cells describe it**: Table 384's `/Headers` expanded by its own
  recursion, else §14.8.4.8.3's search, published as the cell's AT-SPI description (ADR 0312). The
  `labelled_by` relation reaches nobody on this platform, which is argued in `tree::headers`.
- **An element has a place**: measured text quadrilaterals, then §14.8.3.3's content rectangle from
  the element's own marks (`AccessibilityNode::drawn`), then Table 379's `/BBox`, then
  §14.8.5.4.5's container rectangle from the elements it encloses; a mixed element takes the union
  of its text and its other marks (ADRs 0301, 0486, 0768).
- **An element whose content is an annotation** is placed by §12.5.2's `/Rect`, and a `Form`
  element's widget is published as the control its §12.7.5 field type is — a check box, a radio
  button, an entry or a list — with §12.7.5.2's toggling state (ADR 0338). A table's `/Summary` and
  `/Short` are published too (ADR 0715).
- **A caret**: `AccessibilityNode::lines` gives each line of an element's text with each character's
  byte count and place, published as text runs on the page node (ADR 0394).
- **Actions**: `ScrollIntoView` on an element with a place, `Click` on one whose content is an
  annotation, `SetTextSelection` on the page node. Each resolves to a place in the viewport's device
  pixels (`viewer_accessibility::Act`), so a host sends the `Command::Scroll` or `Command::Pointer`
  it already has (ADR 0425).
- **A click on a form field is what a person's click is, in every window.** §12.7.5.2's toggling is
  `viewer_host::form::toggling`, and the widget under a point is found by `viewer_host::form::clicked`
  (ADR 0630). On a text field or a choice, the two native windows give the keyboard to the control
  they placed over the widget, and `quorra` puts its own caret or list there (ADR 1566).
- **The window says whether it has the keyboard.** Each window tells the adapter when the bridge
  comes up and on every change, so the frame is `ACTIVE` with the keyboard and the tree's focus is
  `FOCUSED` (ADR 1565).
- **A form field's value is published in all three windows.** `AccessibilityNode::value` carries
  §12.7.4.3's text across the confined pipe too. A text field publishes it as a text run below the
  control, with each character's box from §12.7.4.3's layout (`AccessibilityNode::value_lines`), and
  a choice field publishes its options as items, the chosen ones selected (ADRs 1489, 1501). In
  `quorra-gtk` and `quorra-qt` the document's node is placed in the window at the page area's origin.
  So its characters lie inside the toolkit's own field, and in `quorra-gtk` it is the node that
  answers `GetCharacterExtents` (ADRs 1516, 1528).

`tools/drive-windows.sh` brings up one private AT-SPI bus for the whole drive (ADR 1453). It asks
each window where the widgets it clicks are, counts the `DocumentFrame` nodes, reads the reopened
form's values and a field's character boxes, reads whether the window is active, and clicks and
types into a field through AT-SPI alone.

**Three windows publish one tree** (ADR 0623). `viewer_accessibility::Reading` is the six queries
and their assembly. The native windows publish through AccessKit rather than `GtkAccessible` or
`QAccessible`, on the standard's argument: §14.7.3's role map is a `shall` on this reader, and
mapping §14.8.4's types onto a platform vocabulary once is what makes the three say the same thing.
The cost is that `accesskit_unix` embeds an application root of its own. A native window's process
therefore publishes **two** applications on the accessibility desktop, both named for the binary:
one holds the toolkit's widgets and one holds §14.7's tree.

## What is left

Ranked by what a reader meets; none of it is a clause this tree has not read.

- **`quorra-gtk` reports a node's place in its own window, not on the screen.** AT-SPI wants screen
  coordinates, and GTK 4 exposes a toplevel's position nowhere: not on `GtkWindow`, `GdkSurface` or
  `GdkToplevel`, and `gtk4-sys` has no symbol for one. The window says so on the `access` topic when
  the bridge comes up. A `gdk4-x11` dependency could answer it, at the price of a platform-specific
  dependency for a coordinate system Wayland does not have, which is a `doc/stack.md` question.
- **The two native windows poll for a client's requests.** `Bridge::wait_millis` is slow until a
  client attaches and fast after. `viewer-ui` is woken through winit's `EventLoopProxy`, which is
  `Send`. A safe UNIX-descriptor source would close it, and neither `glib` nor `gio` offers one in
  the versions this tree binds; a `QSocketNotifier` is a Qt object, which Rust does not call.
- **The platform has no home for a table's grid or a cell's relations.** `accesskit_atspi_common`
  implements `Accessible`, `Action`, `Component`, `Hyperlink`, `Selection`, `Text` and `Value`, and
  not `Table` or `TableCell`. Its relation set holds `ControllerFor` alone. Its
  `supports_text_ranges` admits a text input, `Label`, `Document` and `Terminal`, none of which
  §14.8.4's types map to, so the text interface sits on the page node rather than on the paragraph.
  `pdf_model::structure::TableStack` holds the grid. All four are one upstream question.
- **A `Click` lands on the node's middle rather than on a hit test.** An element whose place is the
  union of two far-apart quadrilaterals could be clicked between them; no corpus document has been
  checked for one.
- **An element with text of its own and structure elements below it publishes its lines before its
  children**, because the flat answer does not record where its own content items sat among them.
  Unmeasured.
- **An answer cut at [`MAX_NODES`] does not say it was cut.** No page of any tagged document this
  project holds reaches the bound, and the census counts it. A flag on the answer and a wire field
  beside it would carry it, as `pdf_model::structure::Reading::truncated` already does one crate
  down.
- **A `/StructParents` array shorter than the page's sequences loses what it does not name.** The
  census cannot see this case, and no witness is known.
- **One recovery is an inference and nothing says so.** Where a page's own stream holds no
  identifier and exactly one form does, that form is answered (a bare integer means the page's own
  stream by §14.7.5.2). A caller cannot tell that attribution from a stated one: there is no channel
  for a readback shortfall, and a report would cost the oracle a judged page (ADR 0152, trap 11).
- **§12.7.5.5's signature field has no role in either vocabulary.** It keeps a group, with the loss
  named in its description. §14.8.5.6's `PrintField` (`Tree::print_field`) is read by nobody, and no
  corpus element states one.
- **macOS and Windows have no bridge**, and `Bridge::shortfall` says so in the program's first
  lines in all three windows. AccessKit has adapters for both, and nothing in this environment can
  test one. `doc/todo/35` is the same shape one interface over.

## Decided rather than owed

- **An untagged page is not given an invented structure.** What crosses is one node saying the
  document states none. Reading order is what §14.7 exists to state, and a guess presented where a
  person expects the author's answer is worse than the honest sentence. Revisit by argument, not by
  attrition.
- **An untagged page's fields are published all the same** (ADR 1369): `PageStructure::widgets`
  carries each widget annotation §12.5.3 lets a person interact with, as the control its field type
  is, named by Table 226's `/TU` or its §12.7.4.2 name, in §12.5.1's tab order, with the click a
  `Form` element declares. **A tagged page's widget that no element's `/OBJR` names is published the
  same way**, after the structure's own nodes (ADR 1381). The census holds a widget published twice
  at zero.
- **An empty page is one of three silences, and each says its own sentence** (ADR 1393):
  `PageStructure::tagging` answers `viewer_core::Tagging` — no `/StructTreeRoot` (§14.7.2), the page
  not read yet, or a structure that reaches none of this page's content, where a catalog claiming
  §14.8.1's `/Marked true` adds that the producer left the page out.
- **A `Form` element whose content is its widget alone is named** by Table 355's `/T`, else the
  field's §14.9.3 name; `/Alt` stays a substitution ahead of both (ADR 1394). **Any element with no
  text of its own is named by its `/T`**, and a titled `Sect` is published as a region, AT-SPI's
  landmark (ADR 1405).
- **A text run carries the readback, not §14.9's substitutions**, because a substituted phrase has
  no glyphs for a caret to move over. A document stating `/ActualText` inside a paragraph therefore
  says one thing to a caret and another to a voice.
- **An `XObject` object reference is refused rather than placed**: its place is the matrix in force
  at the `Do` that painted it, and Table 358's NOTE 2 lets one reference stand for every drawing.
- **`Action::ScrollToPoint` is not carried out and is printed by name**: AT-SPI's
  `Component.ScrollToPoint` asks for the node to be moved to a stated point, which is a different
  request from making it visible.
- **`Action::SetValue` is not taken for a text field because it cannot arrive.** The adapter has no
  `EditableText` interface and raises `SetValue` only with a number, from `Value`; a client types
  after a click instead (ADR 1566). Other actions a client might raise reach `Bridge::requested`
  with `means: None` and are printed.
- **The question's cost is measured with callgrind over `viewer-core --example
  accessibility_cost`**, not a stopwatch (ADR 0312), and per *screen*: neighbouring pages share the
  ancestry that is §14.7.5.4's expensive part (ADRs 0394, 0445). `viewer_core::layout::MOST` bounds
  how many pages a screen holds.
