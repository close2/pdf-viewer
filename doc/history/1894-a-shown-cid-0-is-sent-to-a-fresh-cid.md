# 1894 — A shown CID 0 is sent to a fresh CID by an embedded CMap

Session 1894, batch 130, slot 4 ("archive"), in two sessions. ADR 2622; ADR 2623 and Q770 unused.

**Contract.** ADR 2557 section 3: a `Type0` font whose shown code selects CID 0 under `Identity-H` kept the refusal,
because ISO 32000-2 §9.7.4.2 makes CID 0 the `CIDFont`'s `.notdef` character. Six corpus documents refused on it: three
`CIDFontType2` (`emi15514-sup-0002-SupinfoS2`, `4387ba48…`, `issue7544`) and three `CIDFontType0` over CID-keyed CFF
(`issue11718_reduced`, `issue4650`, `PDFBOX-3062-005717-p1`).

**Premise.** Held: each of the six states `/Encoding /Identity-H`, `Adobe-Identity-0`, a descendant of its own and no
`/CIDSet`; five have a `ToUnicode`. The CID `CMap` writer already stood in `font_files/glyph_indices.rs` (ADR 2342),
now `fonts/embedded_cmap.rs`, shared and writing the same bytes; the glyph copy is ADR 2556's and ADR 2598's.
`issue7544`'s map sends CID 0 to glyph 0x37, so there the fresh CID takes the program's own glyph.

**Built.** `fonts/fresh_cid.rs`: `Identity-H` restated as an embedded `CMap` (§9.7.5.3, Table 118) sending `<0000>` to
a CID past every CID shown, held and stated in `/W`, which gains it at CID 0's width; refused for another predefined
`CMap`, `Identity-V`, and where §9.10.2's third method would read the fresh CID as a character. Both routes plan it,
prove the reloaded font (every code selects its CID or the fresh one, draws what it drew, never CID 0) and write it;
`rewrite/values.rs` names the `CMap` and the `/W`; `pdf-font` gains `cff::highest_cid`. The corpus gate's width check
reads `/W` statement by statement and admits one added statement per font given an embedded `CMap`.

**Measured.** Against HEAD's census, no-pack and pack alike, every line gains three converted documents in each corpus
and both `shows CID 0` rows lose the six; `as-shown` part 4 over `corpora` gains two (`4387ba48…` stays on its
`DeviceCMYK` row). `issue11768_reduced` (not of the six: `Adobe-Japan1` under a `CMap` of its own) keeps the refusal.
veraPDF passes all six at 2b and at 4 (12/12), and veraPDF's own `6-2-11-8-t01-fail-c.pdf` at both; page counts kept.

**Plant.** The `<0000>` range one CID past the fresh CID: three unit tests fail, and in the built binary `issue7544`
and `PDFBOX-3062-005717-p1` are refused (exit 4) with no file written. A converted file's range written back to CID 0
(after `qpdf --qdf`): veraPDF adds exactly 6.2.11.8 and the in-tree checker (`drawn_cid`) refuses it; both controls pass.

**Gates.** In the export: `cargo clippy --no-deps -p pdf-transform -p pdf-font --all-targets -- -D warnings` exit 0;
`cargo fmt --check` exit 0; `--test archive --test profiles` 516 + 13 passed, exit 0; `--lib fresh_cid` 3 passed;
`cargo nextest run -p pdf-font` 336 passed, exit 0; census no-pack exit 0 and pack exit 0; `rank.py` exit 0;
`archive_corpus -- --ignored` 6 passed, 1 failed (the `/W` check above), after the fix that test 1 passed, exit 0.
`getrusage` under a `/W` of 131 073 widths: HEAD 42 848 KiB, this round 44 864 KiB, both refused, exit 4. In the
worktree: `cargo check --tests -p pdf-transform` exit 0; `--bin cited` exit 0; `--bin quotations` exit 0; `cargo
test -p conformance --no-fail-fast` exit 101, two reds, neither this round's: `records` (record 1892's Gates
paragraph) and `round_numbers` (`pdf-font/src/cff_stop_tests.rs:46` and `:90`, round 1897's).

**Found, not fixed.** `issue11768_reduced.pdf`: CID 0 under its own `Adobe-Japan1` `CMap`, no `ToUnicode` (ADR 2622 §2).
