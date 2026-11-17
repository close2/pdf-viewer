# 2071 — Every slot of a name-keyed encoding names a glyph, and `issue18059`'s nulls advance

Session 2071, a fonts round in a third lane (`pdf-viewer-lane3`). Row 9.6.5.2; ADR 2956. What round 2067 left: ADR 0932
against ADR 2948, and the one `raster_golden` page the train found moved beyond the CJK set.

**The slice.** The brief's two linked items in order: which ADR holds for which case, then `issue18059.pdf` p1.

**The premise.** It held on HEAD. `substitutes_notdef` drew nothing for a code no encoding names. `simple_code_table`
let a named encoding's empty slot fall to the program's own name. ADR 2948 reported the clash and changed neither.

**The reading (ADR 2956).** The Type 1 Font Format's section 10.3 has a special encoding built "as shown in the example
in section 2.3". That example fills all 256 slots with `/.notdef` first, and `BuildChar` takes a name for every code.
TN 5176's Appendix B lists `StandardEncoding`'s codes 0 to 31 as `.notdef`. So in a Type 1 or CFF encoding, a
never-assigned code names `.notdef`, and §9.6.5.2 leaves that glyph to the designer. ADR 0932's blank is superseded.
Its census and `assigned_codes` stand: the code table still reports *no glyph of its own*, and the code is drawn at the
outline step. A code meaning whitespace keeps the departure. A named encoding "shall override" the whole mapping, so its
empty slot names `.notdef` too and no longer borrows the program's own name. A name-alone standard font is ADR 2948,
unchanged. A TrueType `cmap` is partial: §9.6.5.4's "a mapping of its choosing" and ADR 0015 decide it, unchanged.

**`issue18059.pdf` p1.** `/Helvetica` is stated by name alone with `/WinAnsiEncoding`. One `Tj` shows `<0044 0045 …
005d>`, and each letter follows a code 0, which `WinAnsiEncoding` leaves empty. `metrics::notdef_advanced` (ADR 2948,
merged at `ce6bc1d8`) now advances each code 0 by the shipped face's `.notdef`, 0.365 em. Before: the letters touch
(`DEFG…Z[\]`, ink 326 px wide at scale 2, from x 101). After: a 3.65 pt gap before each letter (ink 509 px, from x 108),
and nothing painted in the gap. Rendered with `render_at` at HEAD and with 2067's empty-name filter planted back. The
move is right by item 1's reading, so the golden takes that one line by ADR 1016's update, spliced alone.

**Fixtures.** `loading/empty_slot_tests.rs` draws an embedded program's never-assigned code as its boxed `.notdef`,
the outline `/Differences [66 /.notdef]` draws, and nothing for one whose `/ToUnicode` reads a space;
`an_empty_slot_of_a_named_encoding_is_not_the_programs_own` holds the named-encoding case. The `pdf-transform` archive
fixtures `type1_notdef_glyph` and `cff_notdef_glyph` reached glyph 0 for code 10 through a `WinAnsiEncoding` empty
slot and now name it `/.notdef` outright, the route their tests are about. Plants: the `encoded` condition restored
and the named base falling through fail all three new tests; reversed, they pass.

**Gates.** Over `pdf-font`, `pdf-model`, `pdf-transform`: fmt exit 0; clippy `-D warnings --all-targets` exit 0;
nextest `--lib --bins --tests` under `xvfb-run -a` 4291 of 4291 passed, exit 0; rustdoc of `pdf-font` exit 0.
`raster_golden` once, in update mode: held 950, moved 24 (the train's 24 exactly), exit 0; only `issue18059.pdf` p1's line
is kept. Conformance after `git add -N`: exit 101 on this record; then, under `wait.sh --start`, exit 101 on `batch` and
`wait` alone, each exit 0 rerun in the foreground. Fuzz not owed: no public type changed.

**Left.** The CJK pages (Q879); the train's oracle and corpus walks, for a page now painting a `.notdef`.
