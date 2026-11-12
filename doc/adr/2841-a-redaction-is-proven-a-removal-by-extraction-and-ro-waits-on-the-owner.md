# 2841 — A redaction is proven a removal by extraction, and `/RO` waits on the owner

Session 2011. Status: **accepted**; tests built (`crates/pdf-transform/tests/redaction_is_removal.rs`), no change
to `pdf_transform::redact`. It answers `doc/questions/A834` item 3 as far as the owner's own words reach, and
asks the rest as `doc/questions/Q842`. Clauses: ISO 32000-2 §12.5.6.23 (Table 195).

## The premise, checked

A834: "agree with all except 3. If a redaction only places something over someting else, it will lead to
sensitive information exposure. We need to implement this correctly." `pdf_transform::redact` already removes:
text codes, painted paths, image and mask samples, inline images, shading data and form content in the region,
written as a new file by RFC 0002's serializer — never §7.5.6's incremental update, so no earlier revision holds
the removed bytes — and a page whose removal cannot be proven is refused by name. No other path applies a
redaction: `appearance.rs` refuses to draw a `Redact` annotation before application and no window offers an
apply. Nothing in the tree covers instead of removing, and nothing draws `/RO`, `/IC` or `/OverlayText`.

## What was built

`redaction_is_removal.rs` makes the owner's requirement a test of the written file rather than of a picture: every
stream of every object is decoded, and none holds the region's text, its path's coordinates or its image samples
(all zero); the annotation is gone; the file has one `%%EOF` and no `/Prev`. One case states `/RO`, `/IC` and
`/OverlayText` and proves nothing is drawn over the removed region and the departure is reported; the control
states none. A plant that kept the producer's content and its image fails both.

## What was not built, and why

A834's transcribed Reading goes further than its quoted words: it has `/RO` drawn after the removal under the
clause-driven relocation amendment, with the authoring exclusion's "overlay" read as `/OverlayText`. Recommendation
3 of `Q834` *was* that construction, and the owner's words decline it ("except 3"); the owner's audit test
`redaction_projection_spec_audit.rs::a_redaction_ro_is_reported_while_removal_proceeds_without_composing_the_overlay`
holds the current reading, and round 2000's rule makes a change that contradicts an owner's audit test a question.
The construction was built, failed that test, and was withdrawn; `Q842` sets it out with a recommendation.
`doc/scope-ratifications.md` is unchanged.
