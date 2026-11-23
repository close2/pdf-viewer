# 3486 — An appearance stream's own /OC is asked as a Do would ask it

Session 2196. Status: accepted. Context: review 2195 (finding 2195-B03); ISO 32000-2 §12.5.5,
§8.11.3.3, Table 93.

§12.5.5: "Each appearance stream is a form XObject", and Table 93's `/OC` of a form is asked before
the form is processed: "If it is determined to be invisible, the entire form shall be skipped, as
if there were no Do operator to invoke it." The interpreter draws an appearance
stream itself rather than through a `Do`, so it asks the stream's `/OC` itself, in
`Interpreter::draw_annotation`, after the annotation dictionary's `/OC` (§8.11.3.3) and §12.5.3's
flags: all three must show for the appearance to be drawn.

What a skipped appearance takes with it is what a skipped form takes: its drawing, its measured
note, its text and structure span, and §12.5.6.19's `/H` highlight, which highlights the
appearance. A sound playing in the annotation's rectangle (Table 294's `/W` 3) is still painted:
it belongs to the screen annotation and the player, not to the appearance form. The skip is
silent, as the annotation dictionary's is — content the document hides is not content this
program failed to draw. A constructed appearance (§12.7.4.3, the synthesised subtypes) is no
stream the file wrote and has no form dictionary to state one.

Row 12.5.5's `departed` note is about Table 166's `/ca` and `/CA` and is untouched by this.
