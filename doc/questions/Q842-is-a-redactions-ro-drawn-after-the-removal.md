# Q842 — Is a redaction's `/RO` drawn after the removal?

Source: round 2011, building `doc/questions/A834`. Nothing waits on the answer: redaction is removal today, and
`/RO` is reported and not drawn.

## What you said, and what was built

A834: "agree with all except 3. If a redaction only places something over someting else, it will lead to
sensitive information exposure. We need to implement this correctly."

The removal is already correct, and is now proven the way you asked: `pdf_transform::redact` deletes every text
code, path and image sample in the region and writes a **new file**, so no earlier revision keeps the removed bytes
(§12.5.6.23: "remove all traces of the specified content"; image data "shall be destroyed"). A new test,
`crates/pdf-transform/tests/redaction_is_removal.rs`, decodes every stream of the written file and finds none of
the region's text, path or image samples. Nothing in this program covers instead of removing (ADR 2841).

## The question

Q834's recommendation 3, which you declined, was to **draw** the producer's `/RO` form (Table 195: "After this
redaction is applied and the affected content has been removed, the overlay appearance should be drawn such that
its origin lines up with the lower-left corner of the annotation rectangle") — strictly after the removal, never
instead of it. The transcribed Reading of A834 asks for that drawing; your words and your audit test
`redaction_projection_spec_audit.rs::a_redaction_ro_is_reported_while_removal_proceeds_without_composing_the_overlay`
say not to. Round 2011 built it, the audit test failed, and it was withdrawn.

So: **after the content is removed, is the producer's `/RO` drawn over the empty region** (the black box a reader
of a redacted file expects), or is it left undrawn as now?

**Recommendation:** draw it — it hides nothing, because nothing is left under it, and it is the producer's own form
at the place §12.5.6.23 fixes. That needs your word on two things: the authoring exclusion's "overlay" read as
`/OverlayText` (and `/IC`'s fill kept excluded), and the audit test's expectation changed to "drawn after removal".
The withdrawn construction is in ADR 2841 and takes an hour to restore.
