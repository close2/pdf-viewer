# 1599 — A threshold halftone is a press's loss, and a halftone phase and a `Custom` intent are stated

Slot 1 of batch ninety-five, 2026-10-19, an archive round. ADRs 2034, 2035; no question.

**Premises and hypotheses.** The census premise held — `halftone-type-is-one-or-five` 2 (the 2b
and 4 copies of `6-2-5-t03-fail-a`), the phase and the intent row 1 each — but its "types 6, 10
and 16" was the instrument's: the witness's `ExtGState` table names two halftone-shaped
dictionaries as graphics states, and only `/GS0`'s `HT` names a halftone (veraPDF: one check).
The halftone hypothesis held for the type and for part 4's `HTO` (a loss, `discard` under
`as-shown`). It did not hold for `HTP`: neither ISO 32000-1 nor ISO 32000-2 defines it, and Annex
I.3 makes such an entry change nothing a processor does: a statement under every profile. The
intent hypothesis held: the `Custom` entry is given the profile this verb's own entry would name.

**Built.** ADR 2034: `pdf_archive`'s `is_halftone_site` — an `ExtGState` table's entry is a
graphics state, a halftone only where an `HT` or a type 5 names it too (`named_as_halftones`), for
all three halftone rows; `Loss::Halftone` (`--authorise halftone`), `Rewrite::HalftoneWrittenDefault`
(`HT` written Table 57's `Default`, the halftone leaving with ADR 1099's walk) and
`HalftoneOriginRemoved`, `halftones::{defaults, origins}`; a type 5 naming one is refused by name.
ADR 2035: `Rewrite::HalftonePhaseRemoved` (`halftones::phases`) and `DestinationProfileSupplied`
(`prepare::SuppliedDestination`, both an entry of its own and one in the array; no second entry
appended; a non-`Custom` identifier, a non-stream profile and a page-level array refused by name).
`as-shown` answers the two losses `discard`. Three methods moved out of functions past clippy's
bound (`Rewriter::of`, `write_output_intents`, `halftone_and_intent_entries`); the multiset of lines
differs only by signatures and the `changed` scaffolding.
**Pixels.** Six outputs (`t03`, `t01-fail-b`, `6-2-2-t02-fail-a`, each at 2b and 4): every page a
byte-identical PNG at 72 and 720 dpi before and after (looked at: the invader, "Hello World");
veraPDF 1.30.3 compliant for five, the part 4 intent output failing only 6.7.3-1, the `https`
identification namespace `IDENTIFICATION_URIS` admits, which this change does not reach.

**Gates.** rustfmt clean on every file touched. `RUSTFLAGS=-D warnings clippy -p pdf-archive -p
pdf-transform --all-targets` exit 0; their nextest 835 passed, 0 failed (828); `cargo test -p
conformance --no-fail-fast` exit 0, 442 passed (`state.sh docs` not owed). `tools/state.sh --round
1599 archive` exit 0, 905 s: `--remedy-sites` 2b 25 of 69 not built (28 of 71), 4 23 of 60 (26 of
61); `t3-archive-val` 2b `missed` 0, `over` 0, `settled` 8 (8); `archive_corpus` 2b 491 converted,
120 refused (489/122), 4 231/128 (229/130); `cross_check` 0 disagreements; pdf.js and corpora census
1 450 of 1 477, no `not-built-yet` site at 2b. veraPDF census `--tree 6` exit 0: 2b 25 unbuilt
(28), its one `not-built-yet` site with a document `metadata/extension-schema-structure-fields-are-
described` (1). One `--clock` hold, 123 s: `t3-foreign_corpus` exit 0, 0 panicked.

**Unfinished.** That site next (`6-6-2-3-3-t03-fail-b`); no halftone preserve (as ADR 1965). 4 500 s.
