# 2868 — A redaction's `/RO` is drawn strictly after the removal

Session 2026. Status: **implemented**; it supersedes ADR 2841's withdrawal of the drawing, on the owner's answer
`doc/questions/A842` (2026-11-13, "agreed"). Clauses: ISO 32000-2 §12.5.6.23 (Table 195), §8.10.1, §8.4.2.

## The premise, checked

ADR 2841 proved `pdf_transform::redact` a removal by extraction and withdrew drawing `/RO`, because the owner's
words in A834 declined Q834's recommendation 3 and the owner's audit test expected `/RO` reported. A842 adopts
Q842's recommendation: draw the producer's `/RO` form over the emptied region, strictly after the removal, read the
authoring line's "overlay" as `/OverlayText` and `/IC` (ADR 2869), and change that one audit test to expect the
drawing. At HEAD nothing drew `/RO`; effort was the only obstacle.

## The construction

Table 195: "After this redaction is applied and the affected content has been removed, the overlay appearance
should be drawn such that its origin lines up with the lower-left corner of the annotation rectangle."

1. **After, by the order of the code.** `run` plans the page's removal first; only the edited content it returns is
   handed to `redact::overlay::drawn_after`, which appends the forms behind it. The removal never walks the form, and
   a page whose removal is refused is written as the file had it, with no overlay.
2. **The page's first state.** The edited content is opened with `q` and closed with one `Q` more than the `q`s it
   leaves open, so each `q 1 0 0 1 x y cm /OverlayRO<n> Do Q` runs in default user space. A content stream whose
   `Q` pops a state it never pushed breaks §8.4.2's balance and leaves no state to place the form in, so the form is
   not drawn and the page's departure says so (ADR 1123's refusal, for the same reason).
3. **"Its origin" is form space's.** §8.10.1's `/Matrix` carries form space's origin to `(e, f)`, so the translation
   is `/Rect`'s lower-left corner (normalised, §7.9.5) less `(e, f)`; an identity matrix gives the corner itself.
   This is a reading, chosen because it is the only one under which the form's origin and the corner coincide.
4. **The form is the producer's, carried as written.** It is named in a `/XObject` entry the page's restated
   resources gain and carried by the writer's closure like any other resource.
5. **Not drawn, by name**: a `/RO` that is not a reference to a form with a `/BBox` (Table 93); one whose references
   reach the page's original content streams or a `/Redact` annotation, because carrying it would carry the removed
   bytes back into the file (§12.5.6.23's "remove all traces" outranks the table's "should"); a redaction with no
   `/Rect` to line up with. Each is a per-page `Departure`.
6. **`/IC` and `/OverlayText`**: where `/RO` is present the table says both are "ignored", so they depart from
   nothing; where it is absent they are not composed and that is the departure, as before (ADR 2869).

## Tests, and the plant

`redaction_is_removal.rs` proves the order by extraction: every stream of the written file holds none of the
region's text, path coordinates or image samples; the page's content invokes `OverlayRO1` once, after every
operator of the edited content, at the stated translation; and the form the page names is the producer's bytes
unaltered. Two plants failed them: the forms written before the edited content, and the producer's original content
kept with the form drawn over it. The audit test now expects the form's green at (15, 45) and no departure.

## Cost

One more resource name and three operators per drawn form; the page's resources are restated, as they already are
where the removal copied an object.
