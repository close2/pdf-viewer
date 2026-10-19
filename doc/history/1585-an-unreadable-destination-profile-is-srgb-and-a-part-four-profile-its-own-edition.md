# 1585 — An unreadable destination profile is sRGB, and a part 4 profile is held to its own edition

Slot 1 of batch ninety-three, 2026-10-19, an archive round. ADRs 2006, 2007; no question.

**Premises and hypotheses.** The census premise held (32 of 74 not built at 2b) and its ranking
did not: over pdf.js and `doc/corpora` no `not-built-yet` site at 2b is reached by any document —
every counted row is the fence or a part-4-only limit — so the veraPDF corpus ranked them:
destination profile class and colour space 9, halftone `TransferFunction` 3. Neither is
mechanical: the first is a statement where this reader does not draw through the profile and a
refusal where it does, the second ADR 1964's statement-or-loss. The part 4 ICC hypothesis held:
the same three answers under the edition the header names.

**Built.** ADR 2006: `pdf_archive::icc_version_tag_shortfall`; `archive/icc.rs`'s `Reading`
chooses part 2's or part 4's population, every answer shared; the part 4 row is a `REMEDIES` row,
suppliable, and `as-shown`, `as-if-printed`, `keep-everything` state its two texts. ADR 2007:
`prepare::ReplacedDestination` writes shipped sRGB (or a supplied RGB profile) at the one object
every catalog entry names where `Profile::parse` refuses it and `/N` is 3
(`Rewrite::DestinationProfileReplaced`); drawn-through, non-RGB, two-object, inline and page-level
cases refused by name. `archive/transfer.rs` takes `halftone-transfer-function-only-where-
required` as `/TR`'s third row: `Identity` free, a function the loss `transfer-function` kept in
`xmpMM:History`, a missing required function refused. Catalogue and `doc/todo/72` updated.

**Pixels.** Isartor `6-2-2-t02-fail-a` (a DeviceRGB square, looked at), veraPDF `6-2-2-t03-fail-b`,
`6-2-3-t01-fail-b`, `6-2-3-t03-fail-b`: refused at HEAD, now 2b, each page byte-identical at 72 dpi
before and after, veraPDF 2b compliant. Fixtures: display lists equal, or within 1/255 where part
2's added sRGB intent draws DeviceRGB; the inverting halftone function moves the page (the loss).
`fonts/embedded-programs-define-every-glyph-shown`: 29 (pdf.js and corpora), 22 (veraPDF); its
predicate asks `pdf_font`'s `glyph_index`, which carries ADR 1943's step, so that route is spent.

**Gates.** rustfmt clean on every file touched. `RUSTFLAGS=-D warnings clippy -p pdf-archive -p
pdf-transform --all-targets`: exit 0. `nextest -p pdf-archive -p pdf-transform`: 822 passed, 0
failed. `cargo test -p conformance --no-fail-fast`: exit 0, 0 failed. `tools/state.sh docs`: exit
0. One `--tree 6` hold: `t3-archive_corpus` exit 0 (2b 488 converted, 123 refused — 486/125 at
1577; 4: 228/131 from 226/133), `t3-archive-val` exit 0 (2b `over` 1 at 6.6.2.3.1, metadata code
untouched here), veraPDF census exit 0 (2b: 30 unbuilt), pdf.js and corpora census exit 0 (1 450
of 1 477; 30 unbuilt at 2b). One `--clock` hold, 129 s: `t3-foreign_corpus` exit 0, 0 panicked.
`--remedy-sites`: 2b 30 of 73 not built (32 of 74), 4: 28 of 63 (30 of 65).

**Unfinished.** `…-carries-the-tags-its-class-requires`, `…-states-a-correct-profile-id` (next; one
version 5 witness fails both). Siblings' `pdf-colour`/`pdf-model` edits were in every walk. 7 750 s.
