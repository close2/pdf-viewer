# The AccessKit bridge: what is left of it

Status: **built and verified on a real bus** in the three-hundred-and-seventy-sixth session
(ADR 0214). Four things are left, each small and each named.
Priority: 31 — capability
Clauses: §14.7, §14.8.4, §14.9
Code: `crates/viewer-accessibility/` (`role.rs`, `tree.rs`, `bridge.rs`),
`crates/viewer-ui/src/bin/pdf-viewer.rs` (`App::attend`, `App::speak`)

The item this file used to hold — "the answer exists and nothing asks" — is closed.
`viewer-accessibility` maps §14.8.4's forty-one standard structure types onto `accesskit::Role`,
builds the tree, and `accesskit_unix` puts it on AT-SPI, where `busctl` reads it back off the bus:
`Frame` → `DocumentFrame` → the page named by §12.4.2's label → the page's own elements, with
§14.9.3's `/Alt` where the document states one, and a `StatusBar` group carrying what the page
could not draw. The launch path is unmoved and the runtime is confined to one Linux-only crate.

## What is left

- **A `TH` cell's axis.** §14.8.4.8.3's table header cell may describe rows, columns or both, and
  which is the `Scope` attribute's answer (§14.8.5). AccessKit splits the two roles and has no
  neutral one, so `TH` becomes `ColumnHeader` today with the loss written down in `role.rs`.
  Closing it means carrying `Scope` across `viewer-core`'s boundary, which is a vocabulary change
  and wants a witness: no corpus document has been checked for one.

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

## And two things that are decided rather than owed

- **An untagged page is not given an invented structure.** 885 of the corpus's 974 state no
  structure tree, and what crosses is one node saying so. Reading order is what §14.7 exists to
  state; a guess presented where a person expects the author's answer is worse than the honest
  sentence. Revisit by argument, not by attrition.
- **macOS and Windows have no bridge**, and `Bridge::shortfall` says so in the program's first
  lines rather than exposing nothing quietly. AccessKit has adapters for both; nothing in this
  environment can test one. `doc/todo/35` is the same shape one interface over.
