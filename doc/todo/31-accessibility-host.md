# The AccessKit bridge: what is left of it

Status: **built and verified on a real bus** in the three-hundred-and-seventy-sixth session
(ADR 0214); a `TH`'s axis closed on one in the four-hundred-and-sixty-fifth (ADR 0300).
Priority: 31 — capability
Clauses: §14.7, §14.8.4, §14.8.5.4.3, §14.8.5.7, §14.9
Code: `crates/viewer-accessibility/` (`role.rs`, `tree.rs`, `bridge.rs`),
`crates/viewer-core/src/accessibility.rs`, `crates/pdf-model/src/structure.rs`,
`crates/viewer-ui/src/bin/pdf-viewer.rs` (`App::attend`, `App::speak`)

The item this file used to hold — "the answer exists and nothing asks" — is closed.
`viewer-accessibility` maps §14.8.4's forty-one standard structure types onto `accesskit::Role`,
builds the tree, and `accesskit_unix` puts it on AT-SPI, where `busctl` reads it back off the bus:
`Frame` → `DocumentFrame` → the page named by §12.4.2's label → the page's own elements, with
§14.9.3's `/Alt` where the document states one, and a `StatusBar` group carrying what the page
could not draw. The launch path is unmoved and the runtime is confined to one Linux-only crate.

**And a `TH` says which axis it describes** — Table 384's `/Scope` where the document states one,
§14.8.5.7's assumption from the cell's place in the grid where it does not, `RowHeader` or
`ColumnHeader` on the bus. `examples/table_header_census` is what says it was worth doing: of the
corpus's 5965 header cells, 3114 are a **row**'s and were all being announced as a column's.

## What is left

- **A cell's `Headers`, and the search behind it.** Table 384's `/Headers` is an array of the `ID`s
  of the `TH` cells a cell's own headers are, and 281 corpus cells state one. It is half a clause:
  §14.8.4.8.3 states an *algorithm* for the cells that state nothing, walking outward from the cell
  until it meets the table's edge, a data cell after a header cell, or a header cell that states
  `/Headers` itself. **Implementing only the array would be trap 5's shape** — a clause with two
  routes to one answer, one of them silent. `Tree::element_by_id` already resolves an `ID` and
  `TableStack` already holds the grid the search walks, so what is missing is the search and a way
  to say "this cell's headers are those nodes": AccessKit has `labelled_by`, which
  `accesskit_atspi_common` does expose, so it can reach a person.

- **The cell's coordinates cannot cross on this platform, and that is the platform's.**
  `accesskit_atspi_common` implements `Accessible`, `Action`, `Component`, `Hyperlink`, `Selection`,
  `Text` and `Value`, and **not** `org.a11y.atspi.Table` or `TableCell`. So a row index, a column
  index and a `/RowSpan` set on a node would reach AccessKit and stop there. The grid that would
  fill them exists (`pdf_model::structure::TableStack`); what is missing is somewhere for it to
  arrive. Worth an upstream question rather than code here.

- **An element that marks no text has no place at all.** `AccessibilityNode::quads` is built from
  the text layer, so a `Figure`, or a table cell holding an image, crosses with an empty rectangle
  and a magnifier has nothing to point at. Table 379's `/BBox` is a description of where the element
  was laid out, in default user space, and §14.8.5.4.3's ledger row is `silent` for it since ADR
  0300. **Measure first**: no census has counted a `/BBox` layout attribute in the corpus, and the
  first question is whether any document states one.

- **A `Form` element's control role.** §14.8.4.7.2 makes the `Form` structure type *one widget
  annotation*, and a screen reader wants `CheckBox`, `TextInput`, `RadioButton` — which needs the
  annotation behind §14.7.5.3's `/OBJR` and §12.7's field type. `Tree::print_field` reads
  §14.8.5.6's `PrintField` attribute, which is the *printed* form of a field and not this; the
  route is the object reference, and `AccessibilityNode` does not carry one. `Role::Group` today.

- **AT-SPI's `Text` interface.** A paragraph crosses as a node with a name, so an assistive
  technology reads it whole. Moving through it by word, character or line — and reporting a
  caret inside it — is `org.a11y.atspi.Text`, which AccessKit exposes for a node with text runs
  (`Role::TextRun` children carrying offsets). This tree has the offsets: `Interpretation::text_layer`
  is one `Placed` per character code. What is missing is the shape, not the data.

- **Actions.** The tree declares none, so a conforming client requests none; one that arrives
  anyway reaches `Bridge::requested` and `pdf-viewer` prints it by name. The first worth carrying
  out is `ScrollIntoView` on an element, which is `Command::Scroll` and a rectangle this crate
  already has.

- **The question costs eighty milliseconds on a thousand-page document**, and a screen reader asks
  it on every page turn. Measured in the four-hundred-and-sixty-fifth session, in release, on ISO
  32000-2: `Query::AccessibilityTree` is 67–91 ms, against the 0.13–0.25 ms ADR 0228 recorded on a
  five-page one. `viewer_core::accessibility::nodes` walks the **whole document's** structure tree
  and prunes afterwards, resolving §14.7.3's role map per element as it goes. Two obvious levers —
  reach the page's elements through §14.7.5.4's parent tree instead of walking down, or memoise the
  role map — and neither has been priced. This belongs to whoever takes `doc/todo/45` next as much
  as it belongs here.

## And two things that are decided rather than owed

- **An untagged page is not given an invented structure.** 885 of the corpus's 974 state no
  structure tree, and what crosses is one node saying so. Reading order is what §14.7 exists to
  state; a guess presented where a person expects the author's answer is worse than the honest
  sentence. Revisit by argument, not by attrition.
- **macOS and Windows have no bridge**, and `Bridge::shortfall` says so in the program's first
  lines rather than exposing nothing quietly. AccessKit has adapters for both; nothing in this
  environment can test one. `doc/todo/35` is the same shape one interface over.
