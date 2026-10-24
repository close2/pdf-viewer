# 1736 — A conversion is the same from every build, and a font program is filed by its format

Slot 1 of batch one hundred and thirteen, 2026-10-24, an archive round; ADRs 2306, 2307; no question.
I worked in `scratchpad/r1736/`. No ledger row moves; the ISO 19005 table gains one requirement.
**Determinism (ADR 2306).** `PDF-HUL-35/143424…189.pdf` at 2b, `as-if-printed`: ten runs from one
release build of HEAD and ten from a second (own target directory, binary digests differ) are
byte-identical but for `stEvt:when` and the `/ID` digesting it — 89 properties cut, converted. The
same source with ADR 2295's `schema_packets` hunk taken back cuts 35 and refuses at
`metadata/properties-use-known-schemas`: round 1734's "build-dependent" cut was round 1730's hunk,
absent from its first builds. No `HashMap`, pointer key or thread order is on the archive path. The
replay's `/ID` was the clock (§14.4's digest covers the instant). Built: `ArchivePlan::clock`
(`Clock::System` / `Clock::At`), the replay test stating it and comparing whole files, and
`a_conversion_is_the_same_bytes_under_two_hasher_seeds` (two fresh threads, forty packets past the
findings cap, a recorded removal), which a planted `HashMap` order fails (trap 13).
**FontFile3 (ADR 2307).** Read 19005-4 6.2.10.4.1 ("as defined in" 9.9) and ISO 32000-2 Tables 124
and 125 (the brief's Table 126 is ISO 32000-1's number). New row
`fonts/font-programs-filed-by-their-format` (both parts), `pdf_archive::table_124_rows`, and the
`Mechanical` `Rewrite::FontProgramRefiled`: the name restated, or the stream moved to `/FontFile3`;
refused by name where no row admits. 6.2.11.3.1's veraPDF reading is written into its entry.
**Drawn** (page 1, 100 dpi, looked at two): `issue4668`, `issue5751`, `issue6782`, `issue7598`,
`bug921409` — source, HEAD's output and this one byte-identical. veraPDF 1.30.3: those five PASS at
4 and 2b (four failed 6.2.10.4.1 / 6.2.11.4.1 at HEAD); `issue9949` refused; `issue13193`,
`issue9262_reduced` (a `ttcf` under `/FontFile2`) still fail, left unjudged.
**Census** against HEAD's export (converted / conforming / refused / unreadable), both profiles:
pdf.js 2b 757/2/197/11 → 753/2/201/11 and 737/2/217/11 → 733/2/221/11; part 4 751/0/205/11 →
747/0/209/11 and 732/0/224/11 → 728/0/228/11; `doc/corpora` unchanged (344/6/124/16, 303/6/166/16,
339/0/136/16, 292/0/183/16), as is the veraPDF corpus' converter census. Newly refused: `issue9949`,
`issue11740_reduced`, `bug1186827`, `issue5084` (ADR 2307 section 3).
**Hypotheses.** "A `HashMap` in the cut": did not hold. "The `/ID` is time-derived": held. "The rule
is ours": held. **Premises that did not hold**: the build dependence; the five's missing `/Subtype`.
**Gates.** Export of HEAD and my hunks (habit 138): `cargo fmt --check` and `-D warnings` clippy
`--all-targets` on pdf-archive, pdf-transform, viewer-host exit 0; nextest 1272 passed, 0 failed,
18 skipped; rustdoc `-D warnings --document-private-items` on pdf-archive and pdf-transform exit 0.
`tools/state.sh --round 1736 archive` in both exports exit 0: validator over the veraPDF corpus
identical (missed 0, over 0), cross-check 13 560 compared, 0 disagreements; every walk exit 0.
`t3-foreign_corpus` not run: it reads back five writers, not `archive` (its header). Arms not
compared: `cargo tree -i` on both crates reaches no arm binary's crate (habit 119). `cargo test -p
conformance` 463 passed, 1 failed (sibling 1741's record); `evidence`, `adr_status` exit 0, `records` mine clean (1737, 1741, 1742 not)
**Found, not fixed.** `CIDFontType2` + `Identity` over a name-keyed CFF refiles to `CIDFontType0` glyph
for glyph (§9.7.4.2), owed; is a `ttcf` a TrueType program (the Manual's to say); `key_for` reads Table 124 twice. 9124 s.
