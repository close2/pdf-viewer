# The AccessKit bridge: what is left of it

Status: **built and verified on a real bus** in the three-hundred-and-seventy-sixth session
(ADR 0214); a `TH`'s axis closed on one in the four-hundred-and-sixty-fifth (ADR 0300),
Table 379's `/BBox` in the four-hundred-and-sixty-sixth (ADR 0301) and a cell's `/Headers` in
the four-hundred-and-seventy-seventh (ADR 0312).
Priority: 31 — capability
Clauses: §14.7, §14.8.4, §14.8.4.8.3, §14.8.5.4.3, §14.8.5.7, §14.9
Code: `crates/viewer-accessibility/` (`role.rs`, `tree.rs`, `bridge.rs`),
`crates/viewer-core/src/accessibility.rs`, `crates/pdf-model/src/structure.rs`,
`crates/viewer-ui/src/bin/pdf-viewer.rs` (`App::attend`, `App::speak`)
Instruments: `pdf-model --example element_bounds_census`,
`pdf-model --example cell_header_census`, `viewer-core --example accessibility_cost`

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

**And a cell says which header cells describe it** — Table 384's `/Headers` where the producer
wrote one, expanded by the entry's own recursion, and §14.8.4.8.3's search where it did not, in the
clause's order: the row's headers, then the column's, most specific first. It reaches a person as
the cell's AT-SPI **description** — `labelled_by` was the obvious relation and it reaches nobody,
which is recorded below. `examples/cell_header_census` is what says it was worth doing: 17 431 of
the corpus's 21 883 table cells end with at least one header and **17 152 of those get it from the
search rather than from the array**.

## What is left

- **The answer for any page but the first of a large tagged document is empty**, found while
  pricing the round above and not looked into. `viewer_core::accessibility::walk` stops at
  `MAX_NODES` = 8192 elements of the **whole document's** tree and prunes to the page afterwards,
  so ISO 32000-2's page 1 answers with 17 nodes and its page 400 answers with **none at all** —
  `viewer-core --example accessibility_cost <file> 400` prints both. A screen reader on a
  thousand-page tagged document therefore hears nothing past the first few pages, and nothing says
  so. The bound is not the defect; walking the whole tree to answer for one page is, and the fix is
  the same one the cost item below wants: reach the page's elements through §14.7.5.4's parent tree
  instead of walking down from the root.

- **Table 384's `/Short`, which nothing states.** "Contains a short form of the content of a TH
  structure element's content", and its EXAMPLE is precisely this feature: "for each table cell the
  applicable header cells are read to the user … It can become cumbersome for a user to repeatedly
  have to listen to the full contents of a TH structure element." **0 of the corpus's 6197 `TH`
  state one** (`examples/cell_header_census`), which is why it was not taken with `/Headers`: it is
  five lines and a wire field for a population of nothing. Take it when a witness appears, or as
  spec-driven work with that count written beside it.

- **`/Summary`**, Table 384's sentence about a whole table, is unread for the same reason and has
  not been counted.

- **The cell's coordinates cannot cross on this platform, and that is the platform's.**
  `accesskit_atspi_common` implements `Accessible`, `Action`, `Component`, `Hyperlink`, `Selection`,
  `Text` and `Value`, and **not** `org.a11y.atspi.Table` or `TableCell`. So a row index, a column
  index and a `/RowSpan` set on a node would reach AccessKit and stop there. The grid that would
  fill them exists (`pdf_model::structure::TableStack`); what is missing is somewhere for it to
  arrive. Worth an upstream question rather than code here.

  **And the same is true of `labelled_by`, which this file said the adapter exposes.** It does not:
  `accesskit_atspi_common::Node::relation_set` builds exactly one relation, `ControllerFor`, out of
  `Node::controls`, and no other. Worse than inert — `accesskit_consumer::Node::label` *falls back*
  to the labelled-by nodes' text where a node has no label of its own, so an empty table cell would
  be announced as its own headers. That is why §14.8.4.8.3's answer goes into the node's
  description instead, which is a choice about a platform and is argued in `tree::headers`. The
  upstream question is one question for all three: `Table`, `TableCell` and the relation set.

- **An element that marks no text and states no `/BBox` still has no place.** Table 379's
  rectangle is read since ADR 0301 and it answers for 61 of the 1700 corpus elements whose content
  produced no text; the other 1639 have nothing the standard states about where they are. 576 of
  them reach the page only through §14.7.5.3's `/OBJR`, and **that is the strongest remaining
  route**: an object reference names an annotation, §12.5.2's `/Rect` says where an annotation is,
  and `AccessibilityNode` does not carry the object — the same missing link the `Form` item below
  needs. The rest would need a bound taken from the *marks* rather than from the document, which is
  a different kind of answer and wants an argument before it wants code: the display list records
  no `/MCID`, so nothing today can say which commands an element's content items made.

- **Whether a stated `/BBox` should win over the shapes that were drawn.** `tree::place` prefers
  the quads where an element has both, on the conservative reading — the marks are what is on the
  screen. A `Figure` holding a caption *and* a picture has text quads covering only the caption
  while the attribute covers both, so the two disagree by exactly the picture. Nothing has measured
  how often that happens or by how much; `element_bounds_census` has the walk and would need the
  text layer beside it.

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

- **The question costs tens of milliseconds on a thousand-page document**, and a screen reader asks
  it on every page turn — against the 0.13–0.25 ms ADR 0228 recorded on a five-page one.
  `viewer_core::accessibility::nodes` walks the **whole document's** structure tree and prunes
  afterwards, resolving §14.7.3's role map per element as it goes. Two obvious levers — reach the
  page's elements through §14.7.5.4's parent tree instead of walking down, or memoise the role map
  — and neither has been priced. This belongs to whoever takes `doc/todo/45` next as much as it
  belongs here. **`viewer-core --example accessibility_cost` is the stopwatch** since ADR 0301,
  which is what the entry needed: the four-hundred-and-sixty-fifth measured this by hand and left
  nothing anybody could rerun.

  **And a stopwatch is the wrong instrument for a small change on a busy machine**, which ADR 0312
  found the hard way with five other rounds building beside it: the same binary read 56 ms and
  151 ms for the same work. `valgrind --tool=callgrind` over the example is load-independent and
  exact, and the query's own cost separates from the open by running it 1 and 11 times and taking
  the difference over ten. The wall clock stays worth printing; it is not worth an A/B.

## And two things that are decided rather than owed

- **An untagged page is not given an invented structure.** 885 of the corpus's 974 state no
  structure tree, and what crosses is one node saying so. Reading order is what §14.7 exists to
  state; a guess presented where a person expects the author's answer is worse than the honest
  sentence. Revisit by argument, not by attrition.
- **macOS and Windows have no bridge**, and `Bridge::shortfall` says so in the program's first
  lines rather than exposing nothing quietly. AccessKit has adapters for both; nothing in this
  environment can test one. `doc/todo/35` is the same shape one interface over.
