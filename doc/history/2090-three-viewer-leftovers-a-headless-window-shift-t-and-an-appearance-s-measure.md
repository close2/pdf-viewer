# 2090 — Three viewer leftovers: a headless window, Shift + T on a signed document, an appearance's `/Measure`

Session 2090, a host round in `pdf-viewer-gwg`. ADRs 3061 to 3063; no question.

- **A headless `viewer-ui` window** (ADR 3062; record 2086's gap). Premise held: `App` was one struct literal in
  `main`. It moved whole into `App::new(Launching)`, which `main` calls. `quorra/window/headless.rs` drives a
  real `App`: `open_document` on a `Loading` thread, `take_the_opening`, a `Resize`, then `click_page` and
  `pressed`. `a_typed_path_is_read_once_at_enter` counts no `chosen_reads()` per keystroke and one at Enter, and
  checks the field's text.
- **Shift + T at the window** (ADR 3063; record 2061's Left). The windows binding it are the C window, Qt, GTK
  and `viewer-ui`. `tools/conformance/test-support/signed_under_a_policy.rs` generates the signed document, the
  TS 119 172-3 policy with one `TAsInTrustedList` naming a `.invalid` list, and `bind` (`Command::PolicyCopy`).
  The C, Qt and `viewer-ui` windows each have `a_bound_policy_s_reference_goes_with_shift_t_to_the_refresh`: at
  `ask` the question holds and names the reference only once the copy is bound (unbound is the calibration),
  and the yes starts the refresh. The C window also says "the 1 list(s) a signature policy references". GTK is
  left: it has no constructor a test can call without a running `gtk4::Application`, and its Shift + T is the
  shared `requests::trusted_lists` the C and Qt tests drive.
- **An appearance stream's own `/Measure`** (ADR 3061; ADR 3030's Cost). §12.5.5: "Each appearance stream is a
  form XObject". So Table 95's `/Measure` and Table 269's bounds apply to it as they do to a `Do`'s form. The
  annotation pass calls ADR 3030's `note_measured` for a stored appearance, before drawing it as a `Do` does, with `/BBox` under `AA` and the
  page's transform. `viewer-core` `annotation_appearance_measurement`: a quarter-turned `/Matrix`, `A` doubling.
- **Plants**, applied together and reversed: `note_measured` dropped from the appearance arm; the references
  emptied in `requests::trusted_lists` and in `quorra`'s `fetch_trusted_lists`; `commit_of` asked with no control at
  Enter. All five fail planted. The C window's failed before its reference half: its own report's question at `ask`
  took Shift + T's place, so the test answers that one no first; the half is the shared code Qt's failed on.
- **Ledger.** 12.10.2 (`departed`, unchanged): its note, code and test lists name the appearance (ADR 3061).
  12.8.3.4.4 stays `partial`. Its note's remaining gap is per-signature policy syntax named at run time, which a
  window test does not close.

**Gates.** Over `-p pdf-model -p viewer-core -p viewer-host -p viewer-gtk -p viewer-qt -p viewer-ffi -p viewer-ui`:
fmt `--check` exit 0; clippy `-D warnings --all-targets` exit 101 three times (`draw_appearance` past 100 lines, so the note is the caller's; two bare `XObject`s; `main`'s `too_many_lines` expectation unfulfilled once the literal left, so it went), then 0; `cargo build -p pdf-sandbox --bins`, then nextest
`--lib --bins --tests` under `xvfb-run -a` exit 0, 4209 passed, none failed (the C window's test before the answer above; it and its five
neighbours pass after it), so nothing differs from
`expected-failures-current.json` and it is unchanged; rustdoc `-D warnings --document-private-items` (private
target) exit 0; conformance after `git add -N` exit 0, and 0 again after the last edits. `fuzz/` not checked: no public type changed.
