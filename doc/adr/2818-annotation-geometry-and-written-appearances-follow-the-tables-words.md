# 2818 — Annotation geometry and written appearances follow the tables' words

Round 2000, the audit-fix round (group AN, annotation appearances).
Status: **accepted and built** in `pdf-model` (`appearance.rs`: `line`, `polygon`, `Quad`, `for_saving`,
`toggling_states`, `in_form_box`, `regenerate`, `field_text`; `variable_text.rs`: `for_a_file`, `retag`;
`view/writing.rs`: `contents_string`; `view/markup.rs`: `set_free_text`).
It answers the owner's specification audit (`doc/reviews/1983-specification-audit.md`, lines 1141-1167, 1226, 1238)
and its tests `line_ending_endpoint_spec_audit`, `polygon_polyline_spec_audit`, `text_markup_orientation_audit`,
`variable_text_resource_merge_audit`, `variable_text_resource_metric_audit`, `markup_annotation_audit` and
`variable_text::subset_collision_audit`.
Supersedes, for the points below only: ADR 0075 (the side of `/L` the line proper is on), ADR 0192 (where `/LE`'s
endings sit when `/LL` is present), ADR 1784 (a polyline's `/IC`), ADR 0043 (which quadrilateral edge is the
bottom, and the quadrilateral read as a parallelogram), ADR 1425 (that different subsets cannot share a tag), and
ADR 0304's page-space box where it reached a widget.
Clauses: ISO 32000-2 §9.9.2, §12.5.6.2, §12.5.6.7 (Table 178), §12.5.6.9 (Table 181), §12.5.6.10 (Table 182),
§12.7.4.3.

## 1. Leader lines run from the line proper to `/L`

Table 178's `/LL` is "the length of leader lines ... that extend from each endpoint of the line perpendicular to the
line itself", positive meaning "clockwise when traversing the line from its starting point to its ending point (as
specified by L)", and with `/LL` present `/L` holds "the endpoints of the leader lines rather than the endpoints of the
line itself". A leader therefore starts on the line proper and ends at `/L`, so the line proper is `/L` minus `/LL`
times the clockwise unit vector. ADR 0075 added it. `/LLO` and `/LLE` are measured along the same reversed direction.

## 2. `/LE`'s endings sit at `/L`'s coordinates

Table 178 gives `/LE`'s styles "for the endpoints defined, respectively, by the first and second pairs of coordinates,
( x 1 , y 1 ) and ( x 2 , y 2 ), in the L array". ADR 0192 moved them onto the line proper on the strength of Figure 80
drawing an arrowhead there; the audit's reading of the original figure found only separate measurement arrows, so the
table's words stand alone and name `/L`'s points. Each ending still points along the line, which is parallel to `/L`.
**This reading is the one the owner's audit asks for; the other reading (that "defined by" includes the `/LL`
offset) is recorded in the round's results as a question for the owner, not argued here.**

## 3. A polyline fills its endings with `/IC`; a polygon closes a `/Path`; the first ending is at the first vertex

Table 181: "For Polyline annotations, the value of the IC key is used to fill only the line ending." ADR 1784's reading
left a polyline's `/IC` unread. It is now read for both subtypes, filling the shape of a polygon and the closed
endings of a polyline. A polygon's `/Path` is closed before it is painted, because §12.5.6.9 describes a polyline as a
polygon "except that the first and last vertex are not implicitly connected". `/LE`'s first style sits on the first
`/Vertices` pair, pointing away from the second; it had been drawn on the second.

## 4. A text markup quadrilateral is four corners, bottom edge (x1, y1)-(x2, y2)

Table 182 gives the vertices "in counterclockwise order" and orients the text "with respect to the edge connecting points
( x 1 , y 1 ) and ( x 2 , y 2 )"; Figure 84 draws that edge as the baseline (ADR 0043 said the figure drew it on top,
which the audit found false). The bottom is now decided in the text's own frame: if the opposite edge lies
counterclockwise of x1 to x2 the stated edge is the bottom, otherwise it is the top (a file that did not give the
counterclockwise order). The page's y no longer decides it, so a turned quadrilateral keeps its underline on its
stated edge. The region is the four stated corners blended bilinearly, so a trapezoid's far corner bounds a highlight.

**Where the underline sits about that edge is a choice, not a reading.** Neither §12.5.6.10 nor Table 182 states an
underline's thickness or its place relative to the edge. This reader centres the bar (one sixteenth of the
quadrilateral's height) on the stated bottom edge, so a turned quadrilateral's underline is drawn on the edge itself,
where the audit's rotated test looks for it. The cost: half the mark falls outside the quadrilateral that "shall
encompass" the words, and outside `/Rect` where a file's quadrilateral already reaches its edge (corpus `issue14565`).
Keeping the bar inside the quadrilateral was the alternative; it moves two corpus first pages less (`bug1538111`,
`issue14565`) and would need the audit's sample moved inside the edge, which is the owner's record.

## 5. Written widget appearances: the form box, the font the stream names, and distinct subset tags

- §12.7.4.3: a constructed appearance's "lower-left corner of the bounding box ( BBox ) is set to coordinates (0, 0)
  in the form coordinate system". A widget with no stored stream is now constructed posed with its `/Rect` at the
  origin and written with that `/BBox`; with no `/Matrix`, §12.5.5 maps it back onto `/Rect` by a translation.
- Regeneration keeps the stored stream's `/Resources` over `/DR`'s same-named entries, and the layout now measures
  in that merged dictionary, so quadding uses the width of the font the stream shows the text in.
- §9.9.2: "different subsets of the same font in the same PDF file shall have different tags". The tag is a digest,
  and digests collide. Within one resource dictionary a later subset meeting an earlier one's tag with different
  contents is given the next tag (base-26 increment). **Cost**: uniqueness across separate appearance streams of the
  same file is not yet enforced; that needs a file-wide registry in the writers and is left open.

## 6. A markup annotation's written `/Contents` separates paragraphs with CR

§12.5.6.2: "When separating text into paragraphs, a CARRIAGE RETURN (0Dh) shall be used and not, for example, a LINE
FEED character (0Ah)." What a person types is written with each LF and CR LF turned into CR.

## 7. A `Screen` with no appearance stream draws nothing by choice, not by quotation

ADR 0901 quoted §12.5.6.18's "If AP is not present, the screen annotation shall not have a default visual
appearance and shall not be printed." as the clause's answer. Errata Collection 3, issue 42, strikes that
sentence (the audit matched the strikeout on physical page 513; `doc/md`'s copy still prints it, so the
Markdown is not the effective text here). The behaviour stays — Table 190 states no artwork, and its `/MK`
`/I` cell describes how a stored stream was made — but the comments in `annotation.rs` and `appearance.rs`
now call it this reader's choice and cite the erratum, and ADR 0901's rendition-exclusion premise is read
as superseded by A254 and ADR 1960, as the audit says.
