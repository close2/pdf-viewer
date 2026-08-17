# The AccessKit bridge: what is left of it

Status: **built and verified on a real bus** in the three-hundred-and-seventy-sixth session
(ADR 0214); a `TH`'s axis closed on one in the four-hundred-and-sixty-fifth (ADR 0300),
Table 379's `/BBox` in the four-hundred-and-sixty-sixth (ADR 0301), a cell's `/Headers` in
the four-hundred-and-seventy-seventh (ADR 0312), **the empty answer every page but the first
few of a large tagged document got** in the four-hundred-and-ninetieth (ADR 0325), and
**§14.7.5.3's object reference — a place and a control** in the five-hundred-and-third (ADR 0338),
and **page one of ten tagged documents, answered against a page-tree node instead of the page**,
in the five-hundred-and-seventh (ADR 0342) — found by the census that round built, which is what
`tools/state.sh accessibility` now prints; and **a caret, with a third taken off what a page turn
was paying for**, in the five-hundred-and-fifty-ninth (ADR 0394).
Priority: 31 — capability
Clauses: §12.5.2, §12.7.5, §14.7, §14.7.5.3, §14.7.5.4, §14.8.4, §14.8.4.7.2, §14.8.4.8.3,
§14.8.5.4.3, §14.8.5.7, §14.9
Code: `crates/viewer-accessibility/` (`role.rs`, `tree.rs`, `bridge.rs`),
`crates/viewer-core/src/accessibility.rs`, `crates/pdf-model/src/structure.rs`,
`crates/viewer-ui/src/bin/pdf-viewer.rs` (`App::attend`, `App::speak`)
Instruments: `tools/state.sh accessibility` — the corpus-scale census of what a screen reader is
told (ADR 0342) — `pdf-model --example element_bounds_census`,
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

**And an element whose content *is* an annotation has a place and, where it is a widget, a
control.** §14.7.5.3's object reference is what names it; §12.5.2's `/Rect` says where the
annotation is and §12.7.5 says which of the four field types the widget belongs to, so
§14.8.4.7.2's `Form` reaches AT-SPI as a check box, a radio button, an entry or a list rather than
as a group — with §12.7.5.2's toggling state beside it. `examples/element_bounds_census` is what
says it was worth doing: 333 of the 1675 corpus elements that mark no text and state no `/BBox` are
placed by the annotation, and **all 272 `Form` elements name a widget the field tree reaches**.

**And a cell says which header cells describe it** — Table 384's `/Headers` where the producer
wrote one, expanded by the entry's own recursion, and §14.8.4.8.3's search where it did not, in the
clause's order: the row's headers, then the column's, most specific first. It reaches a person as
the cell's AT-SPI **description** — `labelled_by` was the obvious relation and it reaches nobody,
which is recorded below. `examples/cell_header_census` is what says it was worth doing: 17 431 of
the corpus's 21 883 table cells end with at least one header and **17 152 of those get it from the
search rather than from the array**.

## What is left

- ~~The answer for any page but the first of a large tagged document is empty~~ — **closed in the
  four-hundred-and-ninetieth session** (ADR 0325), and checked on a real bus. The page's elements
  are found through
  §14.7.5.4's parent tree — `Tree::elements_on_page` for the three keyings the clause
  distinguishes, `Tree::ancestry` for Table 355's `/P` above them — and the walk descends from the
  root only into the subtree the page occupies, which keeps §14.8.2.5's order. Two things it
  leaves, both stated in the ADR and neither a corpus witness: a page of a *large* document that
  states no `/StructParents` still falls back to the whole-tree walk and so still answers empty,
  and a `/StructParents` array shorter than the page's sequences loses what it does not name.

  **Both had no number beside them until the five-hundred-and-seventh session, and the first now
  has one** (ADR 0342): `tools/state.sh accessibility` classifies every empty answer, and every
  page that takes the whole-tree fallback is a document whose *entire* tree is smaller than the
  walk's bound — so each is the file naming nothing on that page rather than the bound running
  out, and the residue has **no witness in this population**. The census names one the day a
  document exhibits it, which is what a count is for. The `/StructParents`-array half is not
  visible to that census and stays as recorded.

- **An answer cut at [`MAX_NODES`] says nothing about having been cut**, which is trap 5 inside a
  bound rather than inside a feature: `Answer::Accessibility` is a `Vec` and a host cannot tell a
  page of 8192 elements from a page truncated to them. **No page's answer reaches the bound**, over
  every page of every tagged document this project holds — counted by the census since ADR 0342,
  and the count is what will say when it stops being true. What it would cost is a flag on the
  answer and a wire field beside it, the shape `pdf_model::structure::Reading::truncated` already
  has one crate down.

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
  upstream question is one question for all **four** since ADR 0394 found the fourth: `Table`,
  `TableCell`, the relation set, and **which roles may carry a text interface** —
  `supports_text_ranges` admits `Label`, `Document`, `Terminal` and a text input, and not one of
  §14.8.4's forty-one types maps to any of them.

- ~~An element that marks no text and states no `/BBox` still has no place~~ — **the strongest
  route is taken** (ADR 0338), and it was the same missing link the `Form` entry needed: an object
  reference names an annotation and §12.5.2 states where an annotation is. What is left is what no
  clause answers. Of the 1675 corpus elements that mark no text and state no Table 379 `/BBox`, 333
  are now placed and **1342 are not** — `P`, `Div`, `Span`, `TD` and `Figure` elements that name no
  annotation. A bound for those has to come from the *marks* rather than from the document, which
  is a different kind of answer and wants an argument before it wants code: the display list
  records no `/MCID`, so nothing today can say which commands an element's content items made.
  **An `XObject` object reference is refused rather than pending**: its place is the matrix in
  force at the `Do` that painted it, and Table 358's NOTE 2 says one reference suffices however
  many times the object is drawn — so the reference is not naming a position.

- **Whether a stated `/BBox` should win over the shapes that were drawn.** `tree::place` prefers
  the quads where an element has both, on the conservative reading — the marks are what is on the
  screen. A `Figure` holding a caption *and* a picture has text quads covering only the caption
  while the attribute covers both, so the two disagree by exactly the picture. Nothing has measured
  how often that happens or by how much; `element_bounds_census` has the walk and would need the
  text layer beside it.

- ~~A `Form` element's control role~~ — **closed in the five-hundred-and-third session**
  (ADR 0338), and checked on a real bus against a document that labels its own answers.
  `AccessibilityNode::control` carries `pdf_model::form::Control` for the widget behind
  §14.7.5.3's `/OBJR`, and `viewer_accessibility::role` reads it for `Form` and nothing else, on
  Table 368's own division of the annotations between `Link`, `Annot` and `Form`. Two things it
  leaves: **§12.7.5.5's signature field has no role in either vocabulary** and keeps a group with
  the loss named in its description; and `Tree::print_field`'s §14.8.5.6 `PrintField` — the
  *printed* form of a field, for a form that was flattened — is still read by nobody, which is a
  separate entry rather than this one, and 0 corpus elements state it.

- ~~AT-SPI's `Text` interface~~ — **taken in the five-hundred-and-fifty-ninth session**
  (ADR 0394), and checked on a real bus with `GetText`, `GetStringAtOffset` by word and by line,
  and `GetCharacterExtents`. `AccessibilityNode::lines` carries the element's own text one line at
  a time with each character's byte count and place, and `viewer-accessibility` turns each line
  into a `Role::TextRun` — invisible on the bus, because `common_filter` excludes that role, so
  what the change adds is an interface on the nodes that were already there rather than nodes.
  `tools/state.sh accessibility` counts the elements a caret can move through.

  **Three things it leaves, and the first is the platform's.**
  `accesskit_consumer::supports_text_ranges` admits only a text input or `Label`, `Document`,
  `Terminal`, so **no role §14.8.4 maps to can carry the interface**: a client asking a paragraph
  for `org.a11y.atspi.Text` gets nothing, and the interface sits on the *page* node instead. That
  is the same upstream question as `Table`, `TableCell` and the relation set above, and it should
  be asked as one. Second: an element with text of its own *and* structure elements below it
  publishes its lines before its children, because the flat answer does not record where its own
  content items sat among them — rare, and unmeasured. Third: a run's text is the readback rather
  than §14.9's speech, deliberately, because a substitution has no glyphs — so a document stating
  `/ActualText` on a sequence inside a paragraph says one thing to a caret and another to a voice.

- **Actions.** The tree declares none, so a conforming client requests none; one that arrives
  anyway reaches `Bridge::requested` and `pdf-viewer` prints it by name. The first worth carrying
  out is `ScrollIntoView` on an element, which is `Command::Scroll` and a rectangle this crate
  already has. **This entry got sharper in the five-hundred-and-third session and nothing else
  changed about it**: a check box now announces itself as a check box and says whether it is
  ticked, which invites exactly the request this tree declines. `Command::Activate` on the widget
  is the answer and `AccessibilityNode::control` is already the evidence that the node is one.
  **And the five-hundred-and-fifty-ninth added a second invitation of the same kind**: a page that
  says a caret may move through it invites `SetCaretOffset` as surely as a check box invites a
  click. This is the sharpest entry left on the file, and what it needs is not a reading of a
  clause — it is two halves of one change in two crates, `viewer-accessibility` declaring the
  action on the node and `pdf-viewer`'s `App` carrying it out. ADR 0394 says why the round that
  took the other two entries did not take this one.

- **The question costs tens of milliseconds on a thousand-page document**, and a screen reader asks
  it on every page turn — against the 0.13–0.25 ms ADR 0228 recorded on a five-page one.
  **The first of the two levers this entry named is taken** (ADR 0325): the walk no longer descends
  the whole document's tree, so what is left to price is what remains — the ancestors' `/K` arrays,
  every child of which is resolved to find out whether it is one of the page's, and §14.7.3's role
  map resolved per element. **Nothing here has been measured since**, deliberately: the round that
  made the change ran beside nine others and a stopwatch would have measured the machine. Two
  candidates for whoever takes it, both recorded rather than taken — memoise the role map, and skip
  a child whose reference is outside the page's set *before* resolving it, which ADR 0325 rejects
  as written because §14.7.5.1.1's content items may themselves be indirect. This belongs to
  whoever takes `doc/todo/45` next as much as it belongs here. **`viewer-core --example
  accessibility_cost` is the stopwatch** since ADR 0301, which is what the entry needed: the
  four-hundred-and-sixty-fifth measured this by hand and left nothing anybody could rerun.

  **Measured in the five-hundred-and-fifty-ninth session and taken down by a third** (ADR 0394),
  and the answer was neither of the two candidates above. 70.8% of the query was
  `Tree::identified_children`, and inside it `Tree::child`, which read one `/K` entry with **three
  deep copies of it**: one resolution to test for §14.7.5.1.1's bare integer, a second for the
  dictionary, and a clone into `Child::Element`. It resolves once and moves. The role map was not
  the cost, and the reason is worth keeping: it is asked once per element the walk *enters*, which
  is tens, while the resolution is asked of every child of every ancestor, which is thousands. The
  skip stays rejected and now has a second reason — the narrowest sound variant of it, skipping
  unresolved only under an element §14.7.5.4 does not name as an owner, would lose exactly the
  short-`/StructParents` case the residue above is about.

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
