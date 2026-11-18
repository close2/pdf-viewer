# 2088 — The golden's moved pages are read one by one, and the installed CJK face is never opened

Session 2088, an oracle round in `pdf-viewer-audit`: A879's follow-up after `fonts-noto-cjk` was installed.
ADR 3051 is the page-by-page table; ADRs 3052 to 3055 and Q910 unused.

- **Premise, half held.** Both walks failed as before: `corpus` 101 (incomplete 67 against a ceiling of 65,
  `issue15053.pdf` and `mixedfonts.pdf` joined), `raster_golden` the train's 23. Seven of the 23 were never font
  pages, and an eighth had been misread as one since 2026-11-05.
- **Method.** The golden's own `interpret` dumped per page and digested as the golden digests, under HEAD, HEAD with
  face 0 of `NotoSansCJK-Regular.ttc` extracted to a standalone file in the user's own font directory, HEAD with
  `no_machine_fonts()`, and two builds on this machine: `82b0b11d` (before the audit merge) and `44e42f63` (the
  last golden written on the notebook). At `44e42f63` eight pages reproduce the notebook's digests exactly.
- **Restated, (b), eight:** the two `annotation-*-without-appearance`, `bug1538111`, `issue14565`, `issue19389`
  (ADR 2818 §4 and §5), `issue19634` (ADR 2812 §3), `bug1703683_page2_reduced` (§7.10.4's degenerate first
  interval, 79d0c6c4; its `/AntiAlias` are all false, so round 2083's ADR 3024 does not reach it) and
  `issue11150_reduced` (ADR 2687, 593a8e2d). Spliced from one update run; every other line is the file's own.
- **Held, fifteen font pages.** Their lists are identical from `44e42f63` to HEAD here: the faces are the whole
  move. They are not restated, because the face this machine would hold them to is Droid Sans Fallback (Q879's
  rejected option c).
- **Defect 1, left to round 2087 or a later `pdf-font` round:** `fonts-noto-cjk` ships only `.ttc` collections,
  and `substitute.rs` opens every catalogue file with `skrifa::FontRef::new`, which refuses a collection. The
  catalogue lists them by extension and never qualifies one. `FontRef::from_index` and a face index carried
  through `machine_face`, the provider's request and the loader are what it needs.
- **Defect 2, the corpus failure, same owner:** `loading.rs::composite_substitute` chooses a substituted CID font's
  face by the collection's script sample, not by the characters shown. Both joined documents show Latin through
  an Adobe-Japan1 font; Droid covers the sample, has no Basic Latin, and draws none of 29 and 74 characters while
  DejaVu and Nimbus would draw all. With the extracted Noto face both draw whole and their reports equal the
  golden's. The ratchet list is not edited: the two are on it only through these defects.
- **Left, in order:** defect 1, then restating the fifteen from Noto CJK by ADR 1016's update (and `corpus`'s two
  leave with it); defect 2 for a machine whose only CJK face lacks Latin; after round 2087's merge, re-check `issue19389`
  (a descriptor-less Helvetica, ADR 3051 §5).

**Gates.** `corpus` 101 (the two joined, cause above). `raster_golden` in update mode 0, held 951 and moved the
train's 23; after the splice the next check holds 959 and moves the fifteen, not re-run. Over `-p pdf-model`: fmt 0,
clippy `-D warnings --all-targets` 0, `pdf-sandbox --bins` 0, nextest `--lib --bins --tests` 0 (2871 passed; the
manifest expects none), rustdoc `-D warnings --document-private-items` 0. `cargo test -p conformance` 101 once (`evidence`: both files named a home font directory by path), 0 for `--test evidence` after the rewording; every other binary passed.
