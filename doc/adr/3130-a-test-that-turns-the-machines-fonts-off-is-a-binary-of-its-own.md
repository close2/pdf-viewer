# 3130 — A test that turns the machine's fonts off is a binary of its own

Session 2107. Status: **accepted and built.** Context: ADR 3086 (`pdf-model`'s tests grouped into suites), ADR 1425
and ADR 1438 (the two tests that save a machine face and re-open it with none), ADR 0870
(`pdf_font::substitute::no_machine_fonts`), ADR 1154 (a test needing a machine face says so and stops). Clauses:
ISO 32000-2 §9.9.2 (the subset tag), §9.5 (a substitute for an unavailable program).

## What failed

CI's `test` job (`cargo test --workspace`) failed two tests of `pdf-model`'s `text_suite` that pass here:
`subset_names_in_one_save::a_save_does_not_reuse_a_subset_name_the_file_already_holds` (the save left `["field"]`
unconstructed) and `unreadable_program_substitute::the_witnesses_draw_their_text` (no page of `issue11651.pdf`
drew a substitute).

## The cause, proved

Not the runner's fonts. ADR 3086 made `machine_face_written.rs` and `machine_cff_face_written.rs` modules of
`text_suite`, and both call `pdf_font::substitute::no_machine_fonts()` — a process-wide switch that cannot be undone,
which is why each header said "its own binary" — and `machine_cff_face_written.rs` also arms
`pdf_font::provider::faces_come_from`. Nextest runs each test in a process of its own, so every local gate passed;
`cargo test` runs a suite's tests on one harness's threads in one process, so whichever test asked the machine for a
face after the switch found none, and which ones did depended on scheduling.

Reproduced here: the runner's own faces (`fonts-dejavu-core`, `fonts-droid-fallback`, `fonts-noto-mono`,
`fonts-urw-base35`, `fonts-noto-color-emoji`, `fonts-liberation`) bind-mounted over `/usr/share/fonts` in a user
mount namespace pass both tests; the whole `text_suite` binary run once, as `cargo test` runs it, fails
`the_witnesses_draw_their_text` on this machine's full font set too.

## Decision

1. **The two are binaries of their own again**: a `[[test]]` each in `pdf-model`'s manifest, their `mod` lines out of
   `text_suite.rs`, whose header now says a module throws no process-wide switch. ADR 3086's own rule ("a file meant
   to be a walk of its own gets a `[[test]]`") covers it; the twelve walks' names are unchanged.
2. **`subset_names_in_one_save` asks its premise before the save.** Its skip read the save's result, after an
   assertion that the save constructed every appearance — so with no face the test failed instead of skipping, and
   with the skip moved after it, a defect on a machine that has the face would have read as a skip. It now asks
   `installed_covering_embeddable` for the value's displayed characters, as `variable_text::machine_set` does, skips
   by sentence where nothing answers, and otherwise expects the save to write the subset.
3. **CI changes nothing**: the faces the tests need are installed by the step that names them.
