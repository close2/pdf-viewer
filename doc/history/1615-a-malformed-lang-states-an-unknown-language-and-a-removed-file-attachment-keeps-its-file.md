# 1615 — A malformed `/Lang` states an unknown language or the operator's tag, and a removed `FileAttachment` keeps its file

Slot 1 of batch ninety-seven, 2026-10-20, an archive round. ADRs 2066, 2067; no question.

**Premises and hypotheses.** The census premise held: the two `Lang` rows lead 2a's `not-built-yet`
sites (22 and 41 documents once the veraPDF corpus is counted; the brief's 25 was the smaller set).
"Section 6.7.2 wants a language for the document and every differing element" did not hold: the
row's clause is 6.7.4, whose only `shall` is that a stated `/Lang` be a §14.9.2.2 identifier; the
rest are recommendations, and `pdf_archive` was already right. The hypothesis held in shape — the
catalog and the elements are object edits, the inline property list is a `BDC` operand respelled
in place — and every witness placing its `/Lang` in a property list places it inline. Removal is
not the loss answer: §14.9.2.3's inheritance would assert the parent's language, so `discard`
writes the empty identifier, "the language is unknown" (ADR 2066).

**Built.** ADR 2066: `languages.rs`, `Loss::LanguageIdentifier`, `CatalogLanguageRestated` and
`MarkedLanguagesRestated`, `Supplied::Language` (`language = "<tag>"`, held to BCP 47 when read;
empty stops), the report's `restated_language_identifiers`; a direct element or a property list
direct in the resources refused by name. ADR 2067: the inline respelling, sharing `hexadecimal.rs`'s
stream walk and composing with its repair; section 4, `FileAttachment` outside 4f in the subtype
rows' removal, its `/FS` filed in `/EmbeddedFiles` by `attach.rs` (no appended page: the target
holds the file where it is), `keep-attachment = false` refused. Every shipped profile answers the
new rows (ADR 2066 section 2); `document_level.rs`'s misplaced doc comment moved, lines unchanged.

**Pixels.** 63 `Lang` witnesses at 2a with `--authorise language-identifier`: 32 convert, every one
veraPDF 1.30.3 compliant (failing 6.7.4 before), every page byte-identical at 72 and 720 dpi; 31
stop at other sites, none at these two. `FileAttachment`, 16 witnesses: 10 convert at 4e and 9 at 4,
veraPDF compliant but five PDF/UA files at 4 (6.9 test 3, an embedded PDF declaring no part, input
and output alike); the paperclip kept under `as-shown` differs in 47 edge pixels at 72 dpi
(at most 17/255), looked at.

**Gates.** rustfmt per file clean; `cargo fmt --check`, `RUSTFLAGS=-D warnings clippy --all-targets`
for `pdf-transform`, `pdf-archive` exit 0; their nextest 851 passed, 0 failed; `cargo test -p
conformance` 443 passed, 0 failed. `tools/state.sh --round 1615 archive` exit 0: `--remedy-sites`
2a 26 of 73, 4 21 of 60, 4e 22 of 58; the census's unbuilt sites 2a 28 → 26, 4 22 → 21, 4e 23 → 22;
validator `missed` 0, `over` 0 every target; `archive_corpus` 4 234/125 (233/126), 2a 8/8 (4/12
inferred from the four 6.7.4 witnesses); `cross_check` 0 disagreements. `docs` exit 0.
`t3-foreign_corpus` under `--clock` exit 0, 150 s, 0 panicked.

**Unfinished.** The relocation's edge pixels at a fractional `/Rect` origin (reported; not this
round's code); 1607's `supply` of an operator's field description. 5 430 s.
