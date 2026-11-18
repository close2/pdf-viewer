# 3013 — An element's own lines and its children stand in `/K` order

Session 2081. Status: **implemented.** Context: review 2038 (finding B43). Clauses: ISO 32000-2 §14.8.2.5.1
(logical order), §14.7.2 Table 355 (`/K`).

The accessibility tree put an element's own text first and its child elements after, because
`viewer_core::AccessibilityNode` did not record where a child stood among the element's own content items.
§14.8.2.5's logical order is the depth-first traversal of the structure tree, and a `/K` of a marked-content
reference, a child element and another reference puts the child between the two: `P(K=[MCR A, Span B, MCR C])`
read `ACB`. The walk now records, for each element, how many of its parent's own sequences the parent's `/K` named
before it; `finish` lays the own sequences into lines one stretch at a time between the children, and the node
carries `interleaving`, the number of lines before each child. `viewer-accessibility` lays the runs and the
children out in that order, and the confined wire carries the field.

The cost is stated: a line of the page holding content from both sides of a child is two lines in the tree, which
is what a caret reading in the file's order needs. An element with no child inside its own sequences has one
stretch and the lines it had before. A control's held content follows the text its `/K` names before any child, as
ADR 1489 put it after the text. This supersedes the "lines first, children after" construction that `tree.rs`
recorded without an ADR.

Tests: `viewer-accessibility` `interleaved_struct_content_spec_audit`.
