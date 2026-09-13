# 1016 — The second rectangle, and the artifact element nobody writes
Four clause-14 rows, two pairs, and each pair turned out to be one join.

## §14.8.3.3 + §14.8.5.4.5 — the allocation rectangle
§14.8.3.3 gives every BLSE and ILSE **two** rectangles; only the first was derived. The second is
§14.8.5.4.5's: the content rectangle with its before and after edges moved by Table 379's
`/SpaceBefore` and `/SpaceAfter`. **Which two edges those are is §14.8.3.3's own answer** — that
clause exists because "before" and "after" are not "top" and "bottom" outside Western writing — so
the two rows are one piece of work. `pdf_model::structure` gains `WritingMode` (Table 378's eight
names), `BlockProgression`, `BlockSpacing`, `allocation_rectangle` and `Tree::{allocation,
block_spacing, writing_mode}`. The mode is inherited and the spacing is not (`inherited_attribute`
against `attribute`). The ILSE sentence needs no arm: §14.8.5.4.3 addresses Table 379 to block-
level elements, so an ILSE takes the table's default of 0 and the arithmetic returns the content
rectangle — which lets the function answer without §14.8.4.1's category, a fact only a tree walk
has. It reaches a person: `AccessibilityNode::allocation` crosses the sandbox and
`viewer_core::places` unions a child's allocation into the element above it, the clause's "plus
any additional spacing adjustments between these elements". §14.8.3.3 → `implemented`; its layout
`shall`s are the reflowing processor's, and §14.8.3.1 and §14.8.3.2 are `inapplicable` for exactly
that. §14.8.5.4.5 stays `partial` on a **smaller and different** gap, now named: the table cell's
row and column equalisation, whose operands `TableStack` already has.

## §14.8.5.8 + §14.8.2.2.2 — the Artifact element's `/Type` and `/Subtype`
The consumer already existed and it is `structure::Artifact`, the value the marked-content form
has produced all along. So `Tree::artifact` reads Table 385 into that type rather than a new one,
and `ArtifactKind` gains `Inline` — Table 385's fourth name where Table 363's is `Background`.
Each reader takes its own table's four and refuses the other's; that is the test's discrimination.
§14.8.5.8's first sentence is *applied*: `Tree::artifact_attribute` admits `Artifact` and `NSO`
where `Tree::attribute` admits all five PDF-native owners. `viewer_accessibility::tree` says the
kind on the description channel, because an artifact node has no name by design. §14.8.5.8 →
`implemented`; §14.8.2.2.2 stays `partial` on the sentence §14.8.2.2.1's row hands back to it —
content that is an artifact by *absence* from the structure tree.

## What the census says
Two new lines in `viewer-core --test accessibility_census`: one says the allocation rectangle is
not hypothetical, the other that **no document this project holds writes an `Artifact` structure
element** — coverage and robustness answering differently, and why that table went six hundred
sessions unread.
