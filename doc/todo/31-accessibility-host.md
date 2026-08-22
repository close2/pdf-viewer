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
was paying for**, in the five-hundred-and-fifty-ninth (ADR 0394), and **the actions a client may
request**, in the five-hundred-and-ninetieth (ADR 0425) — which also made the census a ratchet and
so closed `doc/todo/05`'s third instrument — and **§14.8.3.3's content rectangle, the place an
element's own marks give it**, in the six-hundred-and-fifty-eighth (ADR 0486), and **the content
stream that identifier is only unique within**, in the six-hundred-and-sixty-first (ADR 0488).
Priority: 31 — capability
Clauses: §12.5.2, §12.7.5, §14.7, §14.7.5.2, §14.7.5.3, §14.7.5.4, §14.8.3.3, §14.8.4,
§14.8.4.7.2, §14.8.4.8.3, §14.8.5.4.3, §14.8.5.4.5, §14.8.5.7, §14.9
Code: `crates/viewer-accessibility/` (`role.rs`, `tree.rs`, `bridge.rs`),
`crates/viewer-core/src/accessibility.rs`, `crates/pdf-model/src/structure.rs`,
`crates/viewer-ui/src/bin/pdf-viewer/access.rs` (`App::attend`, `App::speak`, `App::act`)
Instruments: `tools/state.sh accessibility` — the corpus-scale census of what a screen reader is
told, a **ratchet** and a `doc/todo/02` §2 line since ADR 0425 (built by ADR 0342) — `pdf-model --example element_bounds_census`,
`pdf-model --example cell_header_census`, `pdf-model --example mcid_stream_census`,
`viewer-core --example accessibility_cost`

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

- ~~An element that marks no text and states no `/BBox` still has no place~~ — **the third route
  is taken** (ADR 0486), and the argument this entry asked for came out the other way round from
  the way it was posed. It said a bound from the marks "is a different kind of answer" and one this
  program would be inventing; it is not. §14.8.3.3 gives every block- and inline-level element a
  *content rectangle* and makes it a `shall` — "derived from the shape of the enclosed content" —
  and §14.8.5.4.5 states the derivation for the two cases that are marks rather than layout, a
  table cell's and an illustration's, as "the bounding box of all graphics objects" in the content.
  So the union is the standard's construction under its own name, carried beside Table 379's
  rectangle rather than in place of it (`AccessibilityNode::drawn` against `::bounds`), and on the
  bus the order is measured quadrilaterals, then the marks, then what the producer wrote.

  It is accumulated in the *interpreter* rather than in the display list, and the entry's guess
  about why — "the display list records no `/MCID`" — is right about the fact and wrong about the
  fix: a range of command indices per sequence would cost nothing and would silently break the
  moment `split_off_commands` collects a form `XObject`'s commands into a `Command::Group`, which
  §14.7.5.2 explicitly permits. `Interpreter::draw` is the one moment both are in hand.

  **What is left is the residue and it is the largest of the four routes.** Of the 2124 corpus
  elements that mark no text: 406 state a `/BBox`, 348 are placed by an annotation, **349 by their
  own marks**, and **1021 by nothing** — because their sequences marked nothing at all. `P`, `TD`,
  `Div`, `Span` and `TR` elements around an empty `BDC` … `EMC`, or around content a clip excludes.
  No clause derives a rectangle from no marks, so that is an answer rather than a debt; the count
  is `pdf-model --example element_bounds_census` and it is what will say if it stops being true.

  **An `XObject` object reference is refused rather than pending**: its place is the matrix in
  force at the `Do` that painted it, and Table 358's NOTE 2 says one reference suffices however
  many times the object is drawn — so the reference is not naming a position.

- ~~A sequence inside a form `XObject` shares one numbering with the page's~~ — **measured and
  closed in the six-hundred-and-sixty-first session** (ADR 0488), and the clause answered it
  cleanly rather than leaving it ambiguous: §14.7.5.2 makes the identifier unique "within its
  content stream", Table 357's `/Stm` names which stream and its *absence* is a `shall` that the
  sequence is the page's, and §14.7.5.4 gives each content stream its own parent tree entry — so
  the route back was per stream all along and this tree was flattening it.
  `content::ContentStream` is the other half of the key, `content::named_sequences` the one place
  the match is made, and `Tree::logical_text`, `Tree::logical_range` and both of
  `viewer_core::accessibility`'s readers go through it.

  **The population is `pdf-model --example mcid_stream_census`**, and it is what this entry asked
  for: over the crawl's 65 944 documents — 65 703 opened, 23 447 tagged — **701 have a page marked
  by two or more content streams and 42 have a page where two of them share an identifier**, 545
  state a `/Stm` at all, and 635 have a form with its own `/StructParents`. Over pdf.js and
  `doc/corpora`, one document of 153 tagged ones. **A negative here would have decayed and the
  positive still might**: those are figures about one crawl on one day, and the example is what
  re-derives them.

  Two things it leaves.

  **An appearance stream is `ContentStream::Unnameable`**, because `annotation::Appearance` keeps
  the stream rather than the reference to it and a §12.7.4.3 construction has no object at all.
  That is sound and fixes the half that misleads — the page's `/MCID 0` can no longer be answered
  with a widget's — but an `/MCR` whose `/Stm` names an appearance stream now finds nothing where
  it used to find the wrong thing. No document in either population closes a sequence in an
  unnameable stream. What would close it is the `/AP` reference carried through `Appearance`, and
  **Table 357's `/StmOwn` is the same item from the structure side** — "[t]he indirect reference to
  the PDF object referencing the stream identified by the Stm key", whose NOTE names the annotation
  owning an appearance stream as the common use. Take the two together or not at all.

  **The one recovery is an inference and nothing says so.** Two of the corpus's 153 tagged
  documents put every sequence in one form, state no `/StructParents` anywhere and name each
  content item with a bare integer, which §14.7.5.2 says means the page's own stream; read strictly
  they say nothing to a screen reader, and 61 elements lost their place when they were. So where
  the page's own stream holds no such identifier and exactly one other stream does, that one is
  answered — and a caller cannot tell that attribution from a stated one, for
  `Interpretation::codes_without_a_character`'s reason (ADR 0152): there is no channel here for a
  readback shortfall, and a *report* would cost the oracle a judged page (trap 11).

- **Whether a stated `/BBox` should win over the shapes that were drawn.** `tree::place` prefers
  the quads where an element has both, on the conservative reading — the marks are what is on the
  screen. A `Figure` holding a caption *and* a picture has text quads covering only the caption
  while the attribute covers both, so the two disagree by exactly the picture. Nothing has measured
  how often that happens or by how much.

  **Since ADR 0486 the question is measurable and one third of it is answered.** The sequence's own
  content rectangle covers the caption *and* the picture, so `AccessibilityNode::drawn` is the
  quantity the attribute should be compared against — and where an element has no text at all the
  comparison has been made and the marks win, on `doc/PDF20_AN001-BPC.pdf`'s `[-32768 -32768 32767
  32767]`. What is still unmeasured is the mixed element: `tree::place` asks the quadrilaterals
  first and they exist, so the picture inside a `Figure` that also has a caption is still outside
  the ring. `element_bounds_census` now has both rectangles and could count the disagreement.

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

- ~~Actions~~ — **taken in the five-hundred-and-ninetieth session** (ADR 0425), and checked on a
  real bus by asking for each of the three and reading back what changed. `ScrollIntoView` is
  declared on an element that has a place, `Click` on one whose content *is* §12.5's annotation —
  which needed `AccessibilityNode::annotation` to cross, because neither a rectangle nor a control
  says that an element is an annotation — and `SetTextSelection` on the page node that carries the
  text interface. Each resolves to a **place** in the viewport's own device pixels
  (`viewer_accessibility::Act`), so the host sends the `Command::Scroll` or `Command::Pointer` it
  already had and the boundary gained no message. `App::click_page` is one definition of a click
  for the mouse and for a client, so the two cannot drift apart.

  **Three things it leaves.** A `Click` is refused politely where the point lands on nothing —
  which is right, but it is the *node's* middle rather than a hit test, so an element whose place
  is a union of two far-apart quadrilaterals could be clicked between them; no corpus document has
  been checked for this. `Action::ScrollToPoint` is deliberately not carried out and is printed by
  name: AT-SPI's `Component.ScrollToPoint` asks for the node to be moved *to a stated point*, which
  is a different request. And the other five actions AccessKit defines that a client might raise —
  `Focus`, `SetValue` and the rest — reach `Bridge::requested` with `means: None` and are printed;
  `SetValue` on a text field is the one worth taking next, and it is `Edit::SetField`.

- ~~The question costs tens of milliseconds on a thousand-page document~~ — **measured and taken
  down by a third in the five-hundred-and-fifty-ninth session** (ADR 0394), and neither of the two
  candidates this entry had named was the cost. 70.8% of the query was `Tree::identified_children`,
  and inside it `Tree::child`, which read one `/K` entry with three deep copies of it; it resolves
  once now, and a warm page turn on ISO 32000-2's page 700 went from 65.9 M instructions to 43.8 M
  while gaining the caret. The skip ADR 0325 rejected stays rejected and has a second reason: the
  narrowest sound variant of it would lose exactly the short-`/StructParents` case the residue
  above is about.

  **What is left is the instrument rather than the number.** `viewer-core --example
  accessibility_cost` is the stopwatch, and `valgrind --tool=callgrind` over it is what to use — a
  stopwatch is the wrong instrument for a small change on a busy machine, which ADR 0312 found the
  hard way when the same binary read 56 ms and 151 ms for the same work. The actions added in the
  five-hundred-and-ninetieth cost the *query* nothing: they are a declaration per node and a lookup
  per request.

  **And since the six-hundred-and-tenth the question is asked of a *screen* rather than of a page**
  (ADR 0445), so the number to watch is the marginal page. The example takes `column` as a fourth
  argument, which puts `OneColumn` at half magnification; on ISO 32000-2's page 700 three pages
  cost 1.44× what one costs and carry 4.35× the nodes, because §14.7.5.4's expensive part is the
  ancestry between a page and the root and neighbouring pages share it — and because
  `Viewer::structure` takes each page out of `OnScreen::object` rather than walking the page tree
  for it. An arrangement putting many more pages on the screen than three is what would make this
  a question again; `viewer_core::layout::MOST` is the bound on how many that can be.

## And two things that are decided rather than owed

- **An untagged page is not given an invented structure.** 885 of the corpus's 974 state no
  structure tree, and what crosses is one node saying so. Reading order is what §14.7 exists to
  state; a guess presented where a person expects the author's answer is worse than the honest
  sentence. Revisit by argument, not by attrition.
- **macOS and Windows have no bridge**, and `Bridge::shortfall` says so in the program's first
  lines rather than exposing nothing quietly. AccessKit has adapters for both; nothing in this
  environment can test one. `doc/todo/35` is the same shape one interface over.
