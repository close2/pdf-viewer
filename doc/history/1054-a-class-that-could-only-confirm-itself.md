# 1054 — A class that could only confirm itself, and a ratchet with one document of slack

The corpus slot. The oracle's undiagnosed head was empty for the fourth round running and the ink
sweep's negative tail reproduced ADR 0940's, so the work came off the incomplete list: the six
§9.7.5.2 documents held *by class* rather than by name.

`issue19550.pdf` — the one member of that class named nowhere in this tree. Confirming a class by
opening one member is worth little when membership is decided by the refusal's own wording, which
is how `whose_defect` builds the composition line, so all eighteen were read out of their own bytes
with `qpdf --json`: Type 0, `/Encoding /Identity-H`, descendant `/FontDescriptor` holding none of
Table 120's three keys. All eighteen hold, and the two that *also* carry embedded `/Identity-H`
fonts (`issue6127.pdf` nineteen, `ThuluthFeatures.pdf` four) report only the unembedded ones.

`issue19550.pdf` itself: 200 × 50 points, one `Tj` of nineteen CIDs, `/BaseFont /Arial`,
`/CIDSystemInfo` `(Adobe) (Identity) 0`, no `/ToUnicode`, a descriptor with metrics and no program.
Ours blank; `poppler` `⊓Y∧A¢¢e¢ …`, `ghostscript` circled numerals, `hayro` `ÉA Á Á`, `mupdf` one
full stop — and `poppler` and `mupdf` each log *non-embedded font using identity encoding: Arial*.
Four readings, no two alike. **The file broke the rule** (§9.7.5.2, "The Identity-H and Identity-V
CMaps shall not be used with a non-embedded font"); the refusal is the clause and no pixel is owed.

**The refusal's premise was the call site's, not its own.** `collection_gap`'s first branch asserts
that the *file* embedded no program. What entitled it was the guard above it, which admitted
`FontError::UnsupportedProgram` beside `NotEmbedded` — and nothing in the workspace has constructed
that variant since §9.9's `/FontFile` was implemented (ADR 0040). No message was ever wrong; a
second error kind routed to substitution would have carried this clause's `shall not` to a file
that did embed a program. The variant is deleted, both guards name `NotEmbedded` alone, and the
entitlement is written on the variant that carries it.

**`MAX_PAGELESS` was 6 against a population of 5.** `poppler-742-0-fuzzed.pdf` left it in the
eight-hundred-and-sixtieth session (ADR 0784) and the bound stayed put — `MAX_INCOMPLETE`'s
91-against-61 in the same file, smaller and older. Now 5, with the reason above the constant.

`doc/todo/00` step 7 carries the reading; ADR 0433 gains the file-derived check it asked a future
round not to re-derive by hand; §9.7.5.2's ledger row (`implemented`) records both. No ADR: the
clause's decision is ADR 0433's and unchanged. **The worktree is shared and a sibling was
mid-edit**, so tier 1's two `clippy` lines fail in `pdf-signature/src/revocation.rs` and in the
`fuzz/x509.rs` that calls it — neither this round's. Everything else is green: `conformance` 7/7,
`nextest -p pdf-font -p pdf-syntax -p pdf-model` 1847/1847, `corpus`, `raster_golden` (held 974,
moved 0), `oracle` with verdict totals byte-identical either side, `on_disk`, `dates`, `xmp`,
`jpeg2000`, `pdf-transform --test gate` and `launch_path`.
