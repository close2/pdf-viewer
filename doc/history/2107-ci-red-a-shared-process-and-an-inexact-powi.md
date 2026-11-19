# 2107 — CI's red: two tests sharing a process with a font switch, and a `powi` Miri perturbs

Session 2107, an instruments round in the gwg worktree on CI run 44519312127 (main at 37cd82d9). ADRs 3129-3130.

**`test` job (ADR 3130).** The brief suspected the runner's fonts; the runner's faces were rebuilt here (the four
packages CI's log installs, plus `fonts-noto-color-emoji` from the runner image's list and `fonts-liberation` which
its browsers pull), bind-mounted over `/usr/share/fonts` in a user mount namespace: both tests passed. The cause is
process sharing. ADR 3086 grouped `machine_face_written.rs` and `machine_cff_face_written.rs` into `text_suite`; each
calls `pdf_font::substitute::no_machine_fonts()`, which cannot be undone. Nextest, which every local gate runs, gives
each test a process; `cargo test`, which CI runs, does not, so a test scheduled after the switch found no face. Running
the `text_suite` binary whole, as `cargo test` does, failed `the_witnesses_draw_their_text` here as well. The two are
`[[test]]` binaries again; three whole runs natively and two under the runner's faces pass 200 of 200.
`conformance`'s `test_targets.rs` gains `no_module_of_a_suite_throws_a_process_wide_switch`.

`subset_names_in_one_save` read its skip after asserting the save constructed every appearance, so with no face it
failed instead of skipping. It now asks `installed_covering_embeddable` first (as `variable_text::machine_set` does)
and expects the subset where a face answers; with `/usr/share/fonts` empty it skips by sentence.

**`miri (pdf-render)` (ADR 3129).** `box_down`'s weight was `1 / 4f32.powi(2)`. `powi` is not an IEEE operation and
Miri perturbs it per seed: `-Zmiri-many-seeds=0..32` failed at seed 19 with CI's `(127, 255, 0)`, the default seed
passes. A weight a unit under 1/16 makes the half-covered pixel 127.4999 and rounds to 127. Now `1 / (n × n)`, exact;
seeds 0..64 pass both anti-aliased tests. The test's 128 is exact arithmetic and stays.

**CI** needs no change. **Left**: `the_witnesses_draw_their_text` asserts a substitute on `issue11651.pdf`, a
composite font reached only through a machine face, with no premise asked; on a machine without one it fails rather
than skipping.

**Gates.** `cargo fmt --check` of pdf-model, pdf-render, conformance: exit 0. Clippy `-D warnings --all-targets` of
the three: exit 0. `cargo build -p pdf-sandbox --bins`: exit 0. Nextest `--lib --bins --tests` of pdf-model and
pdf-render under `xvfb-run -a`: exit 0, 3189 passed, 22 skipped; against `expected-failures-current.json` failed 0,
expected 0. Rustdoc `-D warnings --document-private-items` of pdf-model and pdf-render: exit 0. `cargo test -p
conformance --no-fail-fast`: exit 101 on this record's missing gates only; with them, `records`, `test_targets`, `spelled_ordinals`, `round_numbers`: exit 0. The new guard fails
with `machine_face_written` put back in `text_suite` and passes without it. Miri, seeds 0..64: pass.

**Trap.** A gate that runs under nextest only cannot see what one process shared by a suite does; CI's `cargo test`
can. A process-wide switch in a grouped test is now a conformance failure.
