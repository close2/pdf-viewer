# 2813 — A press defaults to invert, and a printed sheet shows the normal appearance at its own scale

Round 2000, the audit-fix round (group AN, annotation appearances).
Status: **accepted and built** in `pdf-model` (`annotation.rs`: `highlight`, `down_overridden`, `pressed_mark`;
`view/annotations.rs`: `ViewState::appearance_for`; `content/annotations.rs`: the `ViewGeometry` an annotation is
decided with).
It answers the owner's specification audit (`doc/reviews/1983-specification-audit.md`, lines 620-626, 946) and its
tests `link_highlighting_residual_audit`, `widget_annotation_spec_audit`, `annotation_appearance_audit`,
`annotation_print_appearance_audit` and `annotation_print_geometry_audit`.
Supersedes, for the points below only: ADR 0123's "default, which is a reading rather than the sentence"; ADR 0168,
ADR 1179 and ADR 1180 where they let a screen's magnification reach a print job.
Clauses: ISO 32000-2 §12.5.3 (Table 167), §12.5.5, §12.5.6.5 (Table 176), §12.5.6.19 (Table 191).

## 1. An absent `/H` is `I`, whatever `/AP` holds

Table 176 (a link) and Table 191 (a widget) each end their `/H` row "Default value: I". ADR 0123 applied that default
only where the annotation stated no down appearance, and read a stated `/D` with no `/H` as the writer having said `P`,
on the ground that otherwise 95 corpus annotations' down artwork could never be shown. Neither table conditions its
default on `/D`, and the owner's audit asks for the tables' reading. So:

- a **link** with no `/H` presses as `I`; Table 176 has no sentence taking the down appearance away, so §12.5.5's
  "down appearance shall be used when the mouse button is pressed" still selects `/D`, and `I` inverts it;
- a **widget** with no `/H` presses as `I`, and Table 191's "\[a] highlighting mode other than P shall override any
  down appearance defined for the annotation" applies to the default as to a stated `I`: the normal appearance is
  shown, inverted.

**Cost.** A widget whose producer drew a `/D` and wrote no `/H` no longer shows that artwork under a press. That is
what Table 191 says; a producer that wants the artwork states `/H /P`.

## 2. Printing uses the normal appearance, wherever the pointer is

§12.5.5: "The normal appearance shall be used when the annotation is not interacting with the user. This appearance
is also used for printing the annotation." `ViewState::appearance_for` answered the pointer's state under every
purpose, so a print job started while a button was held printed the down appearance. Under
`Purpose::Print` it now answers `Normal`; the pressed mark and `ToggleNoView`'s pointer reading follow from it, since
both read that answer.

## 3. `NoZoom` reads no screen's magnification during a print

Table 167's `NoZoom` keeps an annotation "the same fixed size on the screen" against "the magnification level at which
the page itself is displayed". A print job is drawn at the job's own scale, and the view's zoom is no property of the
sheet; carrying it in made two identical print jobs differ by the last zoom on screen. Under `Purpose::Print` the
annotation is decided with no magnification (`None`, which is "nobody has said" and under which `NoZoom` moves
nothing). `NoRotate` under print is untouched by this ADR: the audit records it as unresolved (line 946) and it is
listed as an open question in the round's results rather than decided here.
