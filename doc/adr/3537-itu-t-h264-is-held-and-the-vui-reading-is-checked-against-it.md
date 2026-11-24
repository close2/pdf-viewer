# 3537 — ITU-T H.264 is held, and the VUI reading is checked against it

Session 2209. Status: accepted. Context: ADR 3529 (which applied Annex E "as recalled"), CLAUDE.md
principle 5; RFC 0015 section 6.

## Decision

1. **The text is held.** Rec. ITU-T H.264 (V16) (06/2026), the edition in force, free of charge from
   itu.int (`T-REC-H.264-202606-I!!PDF-E`), is `doc/ITU-T-H.264-202606.pdf` (gitignored like the
   other reference PDFs), SHA-256
   `028cf2cfefd6083ced98d3acb0b586d9328e1631c13d6f21057ad54c569b1b79`, 854 pages. Comments that
   cited "H.264, not held" now cite this edition's clauses; they still paraphrase.
2. **What the text confirms.** `pdf-media-worker`'s walk matches 7.3.2.1.1 field for field
   (including the thirteen chroma profiles and the 8/12 scaling lists), 7.3.2.1.1.1's
   `scaling_list`, E.1.1 up to `chroma_sample_loc_type_bottom_field`, and E.2.1's inferences
   (`video_full_range_flag` 0, `colour_primaries`, `transfer_characteristics`,
   `matrix_coefficients` 2, both chroma locations 0). Table E-5's pairs for codes 1, 4, 5, 6, 7 and 9,
   Equations E-17 to E-19 and E-23 to E-25 (the range) and E-35 to E-37 (the matrix) are as
   `pdf_render::video` states them.
3. **What the text contradicts, fixed with a test each.**
   - Table E-5 defines codes 15 (`IPT-C2`), 16 (`YCgCo-Re`) and 17 (`YCgCo-Ro`); only 18 to 255 are
     reserved. They were read as reserved and drawn by the unspecified matrix's choice — plausible
     and wrong (trap 5). They are now refused by name (`render-cpu`'s
     `the_choice_and_the_refusals_are_said`).
   - 7.4.2.1.1 bounds `num_ref_frames_in_pic_order_cnt_cycle` at 255; a larger count was read as
     255, so the fields after it were read from the wrong bits. It is now refused out loud
     (`vui::tests::a_picture_order_cycle_past_255_is_refused_not_read_short`).

## Consequences

ADR 3529 decision 4's refused list grows by three codes. Primaries and transfer stay carried and
not applied.
