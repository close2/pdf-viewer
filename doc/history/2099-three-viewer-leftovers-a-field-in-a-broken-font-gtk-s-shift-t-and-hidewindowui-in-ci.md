# 2099 — Three viewer leftovers: a field in a broken font, GTK's Shift + T, `/HideWindowUI` in CI

Session 2099, a host round in `pdf-viewer-lane4`. ADRs 3097 to 3099; no question.

- **The accessibility census** (ADR 3097). The brief's premise was half right: the census had exited 101 on every
  train since `0ebbf41b`, but on its ratchets, not on the witness, which no assertion held. A per-document A/B
  against `7471eeff` (one print line in both builds, removed) put it on two decided changes: ADR 3013's `/K` order
  splits a line around a child (138 more lines, no characters; 19 of them in the tracked `bug1997343.pdf`,
  `bug2009627.pdf` and the two `bug1937438` files), and ADR 2976's `/ActualText` placed once (`Well-Tagged-PDF`:
  56 fewer lines, 92 fewer characters). The floors moved to 2502, 223 573 and 5 700 393, each with that reason,
  and the header floors to 24 095 and 34 874 with ADR 2977's (Issue #200: 107 more cells and associations).
  The witness, `PDFBOX-3148-2-fuzzed.pdf`'s `multiline`, was a model defect: its `/DA` font's `/FontFile2` does
  not decode, the page draws such a font from its substitute (ADR 2619), and `variable_text` alone refused. The
  layout now takes the same substitute and reports `Owed::FontSubstituted`; the field has four placed digits, the
  floor moved 1588 → 1592, and the witness is held at zero by an assertion. Test: `viewer-core`
  `untagged_widgets::a_field_set_in_an_unreadable_font_program_has_places_from_its_substitute`, failing with the
  change reverted.
- **GTK's Shift + T** (ADR 3098). No new constructor: `Host::open` on an application `register`ed and never run,
  under `#[gtk4::test]` and Xvfb. `Host::key` became `pub(super)`. `host/trusted_lists_shift_t.rs` reads the
  question off the window's own dialogue and answers with its buttons. Two plants fail it (references emptied in
  `requests::trusted_lists`; GTK's `TrustedLists` arm dropped).
- **`/HideWindowUI` in CI** (ADR 3099). The `window` job's Table 147 step writes `bare.pdf` and holds
  `WindowProbe.Controls`' collapsed line, calibrated by the placed page's visible one; ceilings 8 → 10 and
  36 → 38 minutes. Not run here: no PowerShell on the machine, and fetching one was refused.
- **Ledger.** 12.7.4.3 and 9.5 name the test and the substitute; 12.8.3.4.4 names the GTK test; 12.2's note names
  the step. No status moved.

**Gates.** Over `-p pdf-model -p viewer-core -p viewer-gtk`: fmt `--check` exit 0; clippy `-D warnings --all-targets`
exit 101 once (`useless_vec` in the new test), then 0, and 0 again after the census's last floors; `cargo build -p
pdf-sandbox --bins`, then nextest `--lib --bins --tests` under `xvfb-run -a` exit 0, 3566 passed, none failed, so
nothing differs from `expected-failures-current.json` and it is unchanged; rustdoc `-D warnings
--document-private-items` (private target) exit 0; the accessibility census exit 0 (the witness 0); conformance after
`git add -N` exit 101 once (this record's gates line, unwritten), then 0. `fuzz/` not checked: no public type changed.
