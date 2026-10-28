# 1862 — A header naming no version converts under the target's, its own kept in the history

Slot 2 of batch 127, 2026-10-28, Opus 5.5, one session; ADR 2558 used, ADR 2559 and Q728 unused. R06a built; R12
(part 2) not started, the slot's sixtieth call having passed before R06a closed. No ledger row moved: the
contract is a remedy of `doc/pdfa-blocking-issues.md`, and §7.5.2 is not a row this change reopens.

**First step (habit).** HEAD's converter on `T01_004_header_invalid-syntax-no-dash` and `bug1606566`, to part 4:
both refused at `file-structure/file-header` with `version_for`'s one sentence ("choosing one for it would be
inventing what its producer wrote"); the table's mechanical row was not the refusal. The hypothesis held, and a
second copy of the sentence stood in `recovered::build`, which is what declined `issue9105_other` (`%PDF-1.`).

**Built** (ADR 2558). `archive::header`: the target's version where neither header nor catalog states one (1.7 at
part 2, 2.0 at part 4), and the source's first line (≤ 64 bytes, a non-printable byte `\xHH`) as one `converted`
entry in `xmpMM:History` and the report's `recorded`, the entry a condition of the header as ADRs 1964 and 2498 have
it. `recovered::rebuild` takes the target, and a rebuilt page that states no `/Type` is stated `/Page` (Table 31):
without it veraPDF could not parse `issue9105_other`'s output ("unknown type of page tree node").

**Outcome.** The five `T01_00[3-7]` files, `bug1606566` and `issue9105_other` (under `recovered-structure`) convert
and pass veraPDF at 2b and at 4. `PDFBOX-6040-nodeloop` first converted to a tree veraPDF could not parse (root
naming itself `/Parent`, a null kid); on the orchestrator's word the conversion's page tree walk now declines both
shapes (Table 30) to `recovered-structure`, and its rebuilt output passes veraPDF at both. Census (before that
fix) against HEAD's: pdf.js converted +2 at all four lines, corpora +6.

**Plant.** A header `%P\xB5F-1.4` over a well-formed file, at both targets: the history entry read back from the
output's packet and from the report byte for byte. With the packet's entry withheld (`header: None`) the test
fails; restored by a copy of my own. veraPDF's planted failure: the T01_004 source, at part 4, is not parsed.

**Gates.** `cargo test -p pdf-transform --lib archive::` exit 0 (126 passed); `--test archive` on the new test
exit 0, and the plant exit 101; clippy `-D warnings -p pdf-transform --all-targets` exit 0; `rustfmt --check` on
my files exit 0; `cargo check -p pdf-transform --tests` in the shared tree exit 0 after the three-way merge; census
no-pack exit 0 (480 s, lane 2); census with the pack exit 0, the same +2 and +6; `--bin cited` exit 0; `--bin quotations` exit 0;
`cargo test -p conformance --no-fail-fast` exit 101, two reds and neither mine (`adr_status` names ADR 2573,
`fuzz_workspace` the fuzz lock, both slot 9's). After the tree fix: `--test archive` (filtered, 20) exit 0; clippy
`--no-deps -D warnings -p pdf-transform --all-targets` exit 0; PDFBOX-6040 exit 4 unauthorised, exit 0 and veraPDF
PASS at 2b and 4 under `as-shown`; the census not re-run.

**Found, not fixed.** `pdf-archive` passes a page tree root with `/Parent` and a null kid, both against §7.7.3.2's
Table 30 — the checker's own gap, batch 128. `rustfmt` on `archive/mod.rs` formats its child modules too,
and reformatted a sibling's new `fonts/cid_notdef_glyph.rs` (format only).
