# 451 — The expert set, and the punctuation a fallback would have got right

**Finding.** Annex D.4's table is transcribable and was not transcribed, so a font naming
`MacExpertEncoding` drew no text. With both tables in hand the old refusal's reason turns out to
have understated itself: six of the expert set's codes mean exactly what they mean in
`WinAnsiEncoding`, and all six are punctuation.

**Date.** 2026-08-12.
**ADR.** [0286](../adr/0286-the-expert-set-and-the-punctuation-that-would-have-been-right.md).
**Touched.** `crates/pdf-font/src/encoding.rs`, `crates/pdf-font/src/lib.rs`,
`crates/pdf-model/tests/corpus.rs`, `doc/conformance/ledger.toml` (D.4, D.1),
`doc/adr/0286-*`, this file.

## How it was chosen

The ledger's `reported` rows, read as a list: eighteen of them, of which twelve are on
`CLAUDE.md`'s exclusion list or need a network, three are documented refusals that were
re-reasoned and stand (§12.5.6.11's caret, §12.5.6.12's stamp, §10.8.3's separation simulation),
and one is a **missing table** in a normative annex. That one.

## Gates

Everything unmoved but the ledger — implemented 412 → 413, reported 18 → 17 — and two more tests,
1623 passing. Not one corpus document names the encoding, which is the point of the spec-driven
track rather than an argument against it.
