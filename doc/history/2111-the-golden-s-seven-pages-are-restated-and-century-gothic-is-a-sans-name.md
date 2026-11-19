# 2111 — The golden's seven pages are restated, and Century Gothic is a sans-serif name

Session 2111, an oracle round in `pdf-viewer-gwg` at `4a66dc26`. ADR 3145 is the golden's table, ADR 3146 the font
words, ADR 3147 e-sRGB's scales; ADR 3148 and Q933 unused.

- **Method (ADR 3051 §1).** The check walk at `4a66dc26`, with no wall-clock refusal, held 967 and moved round 2108's
  7. Round 2103's scratch dump was built at `e2ef18ed`, `3a742aaf`, `c862f264`, `25c93430` and `4a66dc26`: at
  `e2ef18ed` its digests equal the golden's on all 7, at `3a742aaf` they take the check's new values, and the three
  later commits change none. `e2ef18ed` and `d24c3aea` change only `pdf-transform`.
- **`issue3521`, raster and list:** with path geometry set aside the lists and reports are equal; 7 glyph fills
  change outline, its GBK 黑体 now read as Hei (ADR 3117) and drawn by a sans face (crops in `r2111/side-issue3521.png`).
- **Six pages, reports only** (`issue17929`, `issue18099_reduced`, `issue2391-2`, `issue6069`, `issue6106`,
  `issue6108`): each gains ADR 3119's one "states no /FontDescriptor" item; the files agree (`/Arial`, `/Monaco` with
  none, `issue17929`'s `/GontDescriptor`).
- **Restated:** the update walk in the worktree, the leftovers built in, held 967; its diff is exactly the 7 lines, each
  the dump's value. The check after it held 974. No page left moved.
- **Leftovers.** (a) `offset_and_scale` and `top_and_middle` form their integers by shifts and saturating integer
  arithmetic, converted once (ADR 3147); natively unchanged, tested exactly at every depth. (b) `century` is a serif word
  unless the name says `gothic` (ADR 3146). (c) `hiramin` joins the CJK serif words. `issue4722`'s `CenturyGothic,Bold`
  moved nothing: an `Identity-H` CIDFont with no program and no `/ToUnicode` is refused before a family is asked.
- **Trap.** None new. A clippy 1.100 `arithmetic_side_effects` finding met the first exact form (`+`, `*`, `-` on `u32`);
  the saturating forms pass it.

**Gates.** `raster_golden` check 101 (held 967, moved 7, no wall-clock refusal), update 0 (diff the 7), check 0 (held
974). Over `-p pdf-colour -p pdf-font -p pdf-model -p pdf-transform`: fmt 0; clippy `-D warnings --all-targets` 0
(101 first, the finding above); `pdf-sandbox --bins` 0; nextest `--lib --bins --tests` 0 (4777 passed;
`expected-failures-current.json` lists none, and none failed); rustdoc `-D warnings --document-private-items` 0 for
each; `cargo test -p conformance` 101 (`records`, this paragraph unwritten), `--test records` 0. No public type
changed, so no `fuzz` check is owed.
