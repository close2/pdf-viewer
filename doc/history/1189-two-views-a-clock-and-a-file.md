# 1189 — Two views, a clock that is not a presentation's, and a file a person chose

The batch's host-UI round, on `doc/todo/65` bucket 1: three rows whose core was built and whose
surface was not.

**§12.3.5 and §12.3.5.1 — Table 153's `/View T`.** The entry states two drawable views and two
differences between them: all of the schema against a subset of it, and an icon. The panel
reported `T` as a surface it was not, beside §12.3.6's `FilmStrip`, `FreeForm` and `Linear` — a
reading of the word *tile* rather than of the entry. `PanelRow` gains `cells` (one per visible
schema field, in `/O` order, present whether or not the file states a value) and `icon`;
`panel::presentation` resolves the view through the navigator; all three windows draw both, with
a `GtkGrid`, a model whose `columnCount` is the schema's, and `viewer-ui`'s own ink. Three
silences chosen and written down: how many fields a tile shows, what the icon looks like, and that
§12.3.5.2's folders stand in both. ADR 1215. §12.3.5.1's residue is now one sentence — Table 158's
`Direction N`.

**§12.6.4.15 — the transition outside a presentation.** §12.4.4.1 conditions a page's `/Trans` on
presentation mode; this clause states no such condition and all three windows showed the end state
anyway. The only thing gated on the mode was the clock: `Clock::for_one_transition` is the same
machinery, and it never ticks, because `/Dur` is stated as presentation timing. `Clock::spent` is
how a host drops it. The row is `departed` rather than `partial` now, on §12.4.4's own reading of
the one thing left — the four styles Table 164 states no quantity for.

**§12.7.5.3 — the file-select control's contents.** The row recorded asking a host for the file as
*rejected rather than deferred*, because "the parameter would be a request to open an arbitrary
path on behalf of a document". True of a path the document wrote, false of a path a person typed —
the division `read_import` already draws. `ViewState::choose_file` does both halves of Table 231
bit 21, `Edit::ChooseFile` carries the file across the boundary and the confined protocol,
`policy::read_chosen` reads it under one function with a stated memory budget, and `form::edit_of`
is where the three windows tell the two verbs apart. `partial` now for a chooser *widget*, which
needs a round that can drive a dialogue. ADR 1216.

**Gates.** `rustfmt --check`; clippy `-D warnings`; `cargo nextest run` on the eight crates
touched; doctests; `cargo test -p conformance`. Tier 2 behind the lock: `launch_path`,
`selection_census`, `accessibility_census`, `save_round_trip`, all 0.

**Carried, not this round's.** `pdf_model::popup::PopupWindow` and `Comment` gained `subject` and
`created` elsewhere and nothing carried them into `viewer-host`, `viewer-confined` or `viewer-ui`,
which broke the workspace for about an hour. Carried here because those three crates are this
round's: the two entries now cross the confined protocol beside the rest of the window.
