# 1553 — A profile is a lower bound, a named code is drawn through its ToUnicode, and Pol.pdf converts

Slot 2 of batch eighty-eight, 2026-10-14/15, an archive round. ADRs 1942, 1943; Q448.

**The premise held in part.** The owner's "like printed" is `doc/profiles/as-if-printed.toml`,
reachable only by `--config <path>`; the window offers no archive path. The run exited 4, not 0.
**The hypothesis did not hold**: a cmap stating "the glyph the file already selects" would have
archived this reader's wrong choice. Rendered, page 8 drew "50²%": code 240 (octal), named `space`
in WinAnsi with no U+0020 glyph in the subset, was offered to the (1, 0) subtable as a Mac code.

**Built.** `pdf-font`: a non-symbolic named code no step of §9.6.5.4 reaches is first drawn
through its `/ToUnicode` character in the (3, 1) subtable (ADR 1943). `pdf-archive`: a shown Type
3 code its encoding names `.notdef` is reported (veraPDF reported it; this validator did not).
`pdf-transform`: the TrueType route is answered by `/Differences` naming the `/ToUnicode`
character's glyph, proved per shown code (`Stated`); a Type 3 `.notdef` is renamed in
`/Differences` and `/CharProcs` (`Mechanical`); `preserve` at `annotations/flags-entry-present`
keeps the annotation and writes `/F 4`; `--profile <name>` names a shipped profile;
`as-shown.toml` ships; `as-if-printed` and `keep-everything` answer the flags site (ADR 1942).

**Pol.pdf** converts to 2b under `--profile as-if-printed` and under `--profile as-shown`; veraPDF
1.30.3 reports both outputs compliant (0 failed rules). Every page draws as its source except
DeviceCMYK text, which the existing DefaultCMYK row moves by up to 34 of 255. 2u and 4 still
refuse (Type 3 ToUnicode; CMYK beside RGB in part 4) — `doc/todo/72`.

**Census** (`examples/archive_unbuilt.rs`, 1 450 of 1 477 pdf.js and corpora documents): at 2b,
39 sites unbuilt; most reached are `optional-content/configuration-names` 45 documents,
`graphics/only-operators-the-base-standard-defines` 43 (fence), `fonts/embedded-programs-define-
every-glyph-shown` 29 (fence), `graphics/no-transfer-function-in-a-graphics-state` 27,
`graphics/named-resources-are-defined` 18 (fence), `file-structure/stream-filters-are-standard` 13.

**Gates.** rustfmt clean on every file touched; `clippy -D warnings --all-targets` on pdf-font,
pdf-archive, pdf-transform: exit 0. `nextest` on the three: 1 062 passed, 0 failed.
`cargo test -p conformance`: 413 passed, 1 failed — `no_comment_names_a_round_by_number` on slot
1's `r1552_probe.rs`, not this round's. `t2-raster_golden`: exit 0. `t3-archive_corpus`: exit 0
(2b: 479 failing documents converted, 131 refused). `t3-archive-val`: exit 0.
`t3-foreign_corpus` (one `--clock` hold): exit 0, 0 panicked. The processor arm at 1x: exit 0,
0 of 968 page-one digests moved against HEAD's processor arm at 1× — ADR 1943's mark reaches
no pdf.js first page; Pol.pdf is its witness. The census over `doc/veraPDF-corpus` stalled on one
Isartor file (`doc/todo/72`) and was stopped.
