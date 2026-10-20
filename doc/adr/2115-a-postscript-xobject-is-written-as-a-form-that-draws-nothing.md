# 2115 — A PostScript XObject is written as a form that draws nothing

Session 1639, the archive slot of batch one hundred. Status: **accepted**, **built** in
`pdf-transform`. It **replaces** `Rewrite::PostScriptXObject`'s removal. Context: ADRs 0816, 0947,
1006; `doc/questions/A62`; `doc/pdf-a-conversion-limits.md` sections 5.2 and the table at its
PostScript row. Code: `crates/pdf-transform/src/archive/rewrite/streams.rs` (`drawing_nothing`).

## 1. The defect

ISO 19005-2 section 6.2.9.3 forbids a PostScript XObject. The rewrite dropped the object, so the
serializer wrote a null in its place and the resource entry naming it disappeared (§7.3.7). Where
a content stream invokes it — veraPDF's `6-2-9-3-t01-fail-a`, `/X0 Do` — the name the producer
wrote was then undefined, which ISO 19005-2 reads as section 6.2.2 requiring (TechNote 0010 A002),
and `doc/adr/0947`'s second verdict refused the file for breaking a requirement the source met.
Under both shipped profiles this was the one refusal of its shape at PDF/A-2b. The refusal was
loud, so nothing was silently wrong; but a remedy that cannot be applied to the one case that
needs it is a defect in the remedy, not a fact about the file.

## 2. The construction

The object keeps its number and becomes §8.10's form `XObject` with an empty content stream and a
`/BBox` of `[0 0 0 0]`. What a page draws is unchanged: ISO 32000-1 8.8.2, the edition part 2
delegates to and the only one with PostScript XObjects, has their fragments take no effect on a
screen or a non-PostScript printer, and ISO 32000-2 §8.8.1 names image and form XObjects only; a
form with no operators paints nothing. The producer's `Do` is not edited (the fence, ADR 0816), the
name it uses stays defined, and no mark is composed — the form contains none. `Decision::Mechanical`
as before.

## 3. The net's own test

`tests/archive.rs`'s `a_conversion_that_would_break_what_the_source_met_writes_nothing` used this
case as the witness of the second verdict's regression list. That witness now converts, so the net
is held where it acts instead: `archive::tests::a_requirement_the_source_met_and_the_output_fails_is_named_a_regression`
hands `hold_the_output_to_the_target` (which now takes the target alone) an output missing a
resource the source defined.
