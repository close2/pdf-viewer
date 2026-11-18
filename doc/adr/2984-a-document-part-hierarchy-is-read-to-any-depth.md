# 2984 — A document part hierarchy is read to any depth, and `/DParts` is Issue #612's flat array

Session 2076. Status: **implemented.** Context: review 2038 (findings B36, T48, D21, D25). Clauses: ISO 32000-2
§14.12.2, §14.12.4.1 Table 409 with Errata Collection 3 Issue #612, §14.13.8.

`document_part` bounded its walk at 64 levels and called a longer chain "a cycle a malformed file wrote". §14.12.2's
tree rules bound no chain's length, so a deep hierarchy is valid, and a `DPart`'s `/AF` past level 64 vanished from
the attachments list without a word. Cycles never needed the bound: §14.12.2's "[a] child DPart dictionary shall
not be referenced by more than one parent DPart dictionary" is already applied as a set of references walked, which
makes any cycle finite. So `walk` keeps its own stack — no recursion, no depth bound — and `MAX_PARTS` (65 536 nodes)
is the walk's only budget. `first_page` is the same walk stopping at the first `/Start`, which is the first leaf in
§14.12.3's depth-first order.

D21: Issue #612 (a `Caret` on printed page 851, `Review/Accepted`) rewrites `/DParts` as "an array of indirect
references to immediate descendant DPart dictionaries. Empty arrays shall not be present." The printed array of
arrays is the retired shape; it is still read, element by element, and the comments now say which is which.

Tests: `viewer-core` `dpart_depth_spec_audit`; `document_part::tests::a_deep_chain_is_read_whole` (200 levels, flat
references).
