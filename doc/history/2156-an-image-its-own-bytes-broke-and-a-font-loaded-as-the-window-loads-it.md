# 2156 — An image its own bytes broke, and a font loaded as the window loads it

Session 2156, an archive round in the `lane3` worktree, on the census's next document, `507676`.
ADRs 3325 to 3327; question Q978; ledger rows 7.4.4.4, 7.3.8.2 and 9.6.5.4 (`implemented`, before
and after; each gains a test).

**`507676`, the image stop (ADR 3325).** Printed at the refusal: page 4's `R73` (`FlateDecode`,
`/Predictor 15`) inflates to 7 999 of its 8 000 bytes under an Adler-32 that disagrees, and row 46
carries tag 252, which Table 10 does not define, so the predictor refuses, as for one inline image.
The reader rightly draws neither, but said "stream did not decode", as of a filter it does not run.
`FilterRefusal::Corrupt` is now named apart (`Document::image_stream_reported`) and the wider gate
bakes it in, with an image short of its grid. Not a reader defect: nothing draws differently.

**The three font stops (ADR 3326).** Printed at the refusal: `codes_drawn` on font 19, `Malformed {
"/FontFile2 decoded whole and its check value disagrees (13419 bytes)…" }`. Two of the twelve
symbolic `TTE…` fonts carry programs their own checksums disown; the window draws them from
`substitute_for_unreadable`'s face, which the substitution embeds; the restatement used `load` alone,
so one font refused three rows for all twelve. It now asks as the window asks.

**Result.** `as-if-printed` part 4 converts; veraPDF 1.30.3 passes. At 2b the one stop left is 6.1.13
(R06b, the two-target request). Renders at 60 dpi, source against output: every page within 2.2 %
of pixels at a 12 % fuzz, the most on pages 8 and 9, which at 200 dpi is glyph placement in the
substituted bold face; three pages are photographed. `as-shown` keeps 6.2.2. A symbolic
`LiberationSans` fixture with a disowned program converts at 2b and 4, veraPDF passing both; its
`cut_program` test fails with the old `load` planted.

**`as-shown` (ADR 3327).** Its R03 and R08 stops are the content-stream fence or a mapping of the
reader's choosing; the one that differs from a built remedy only by where its key sits — an inline
image's `/I true`, two pdf.js documents' only stop — is Q978.

**Census.** With the pack, corpora `as-if-printed` part 4 converts 482 (was 481) and refuses 3 (was
4); the other seven lines are unchanged, and without the pack the same as before (`issue15053`).
Sections 1, 2 and 9, the R03 paragraphs, `rank.py`'s R03 and R11 statuses, two mitigation entries.

**Gates.** Over `-p pdf-syntax -p pdf-model -p pdf-transform`: fmt 0; clippy `-D warnings
--all-targets` 0 (after a helper moved above a test module); `pdf-sandbox` build 0; nextest `--lib
--bins --tests` 100, 4616 passed and one failed, B04, as `expected-failures-current.json` expects
(after `a_page_the_window_could_not_draw_whole_is_not_photographed` took a filter this reader does not
run for its short image, which ADR 3325 makes the file's fault); rustdoc 0; conformance 0; both
censuses 0. No public type changed, so no `fuzz` check is owed.
