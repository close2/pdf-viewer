# 1826 — Two shown descendants over one re-keyed descriptor, and an inline bystander's descriptor

Slot 4 of batch one hundred and twenty-three, 2026-10-27, an archive slot (Sonnet, high); ADRs 2486
and 2487 used, Q692 unused. I worked in `scratchpad/r1826/`. No ledger status moves (ISO 19005 has
no ledger rows).
**Premise (a)** held: in `rekeyed::embed` a second shown descendant over a shared descriptor widens
the program, the subset tag (letters `pdf_font::embed` derives from the glyphs; §9.9.2 leaves them
arbitrary) changes, so the name changes, and the first
descendant kept the old `/BaseFont` (descriptor `DDUAZV+ChromSerifOTF` against the first
descendant's `JZBHUQ+ChromSerifOTF` on a planted pair). **Premise (b)** held and was worse than
stated: the inline bystander keeps a `CIDFontType2` over a descriptor that now holds a program, so the
conversion is declined on `fonts/cid-to-gid-map-present`, not merely a mismatched name.
**Built.** (a) `Rekeyed` records the `CIDFont` objects showing the program; every embed writes the
gains to all of them, `/Subtype` stated only where a descendant needs it (ADR 2486). (b)
`bystanders::keep_their_own_descriptors` also walks the values of every object for a dictionary whose
`/FontDescriptor` is a renamed descriptor, writes it out as an object holding a copy of the
producer's descriptor and records it in `Substitutes::direct` for ADR 2162's rewriter (ADR 2487;
Tables 109, 110, 115 make the entry a reference, §7.3.10 the direct and indirect spelling one value).
**Proof.** Two new tests and one shared helper in `tests/archive/rekeyed.rs`; (a)'s fails on Table
120 without the change; (b)'s fails (declined) with the walk disabled. veraPDF 1.30.3 PASS on the
shown, inline and unshown planted outputs at 2b and 4 (six files). The census (`archive_unbuilt`,
pdf.js and `doc/corpora`) is line for line the pre-change one (150 lines), so no corpus document
carries either shape.
**Found, not fixed**: the Type 0 parent's `/BaseFont` is not restated when its `CIDFont`'s changes
(Table 120 speaks of the dictionary that refers to the descriptor, which the parent does not).
**Gates.** In an export of HEAD and my three files (a sibling's mid-edit broke the shared build):
`cargo fmt --check -p pdf-transform` exit 0; `RUSTFLAGS="-D warnings" cargo clippy -p pdf-transform
--all-targets` exit 0; rustdoc `-D warnings --document-private-items` exit 0; `cargo test -p
pdf-transform --test archive` exit 0, 355 passed, 6 ignored; `archive_corpus --ignored` (gates profile)
exit 0, 7 passed; census `--tree 6` exit 0 before and after, identical; `tools/state.sh --round 1826
archive` exit 0 (2619 s; every walk of it passed, the converter over the veraPDF corpus included);
`cargo test -p conformance --no-fail-fast` exit 0, no red. `cargo tree -p render-cpu`, `-p raster-compare` and `-p render-raster` each list no `pdf-transform`
(count 0), so against HEAD's six arms nothing moves. `cargo nextest run -p pdf-transform` (the Opus
validator, in the worktree): 752 passed, 21 skipped. The validator's one finding, a comment that made
the tag's derivation §9.9.2's, was fixed by me. About 6300 s. **Validation** (the orchestrator): the Opus validator's verdict was FAIL on one attribution (the subset tag's letters put on §9.9.2) and PASS on both fixes, three plants, provenance and the gates; the round fixed the finding.
