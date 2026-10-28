# 1858 — A font selected and never shown reports nothing, a null resource entry is undefined, and a page read in part is written as what was read

Slot 8 of batch 126, 2026-10-28, a "parsers" round (Opus 5.5, two sessions: the first built the fixes and handed over
after part (2), the second proved them on HEAD, planted, gated and wrote the paperwork); ADRs 2550 and 2551 used, Q724
unused.

**Premise.** Four documents, four causes, (4)'s in the writer. Three held; (4)'s did not. `veraPDFHiResWrongObjectID`'s
object headers agree with both cross-reference sections, and HEAD wrote no output at all: its page 9 names `/Im0 99 0 R`,
object 99 does not exist, and the survey counted the entry present without resolving it (trap 258), so the output's own
check refused the conversion. The writer already restates every `/Length` (ADR 2550 section 4).

**Built.** `pdf-model`: a `Tf` whose name reaches no font dictionary notes nothing; the text state carries the sentence
and the first show that draws nothing for it says it (Table 103; `2145704`'s `BT/ -1 Tf ET` is no longer incomplete), and
the empty name prints as "/ (the empty name, §7.3.5)". `pdf-archive`: `peek` resolves an entry before it counts as found
(§7.3.9). `pdf-transform`: `insert_name` defines a name whose entry resolves to null in its place; a page whose
dictionary stops being readable is written as the entries the reader drew it from and named in `read_in_part`, the
sentence, the JSON and `xmpMM:History` (ADR 2551, amending ADR 2502 decisions 6 and 8). `issue9105_other`'s header
`%PDF-1.` has no minor digit, so §7.5.2's refusal is right and its remedy is R06a's, named to Slot 4.

**Measured.** `veraPDFHiResWrongObjectID` converts at 2b and 4 under both shipped profiles, veraPDF PASS on all four.
`poppler-742-0-fuzzed` is rebuilt (1 page stated, drawn and written, from 7 entries) and refused by ordinary rows. Each
of the five new tests fails on HEAD, with HEAD's own behaviour and not a missing name: the survey lists no undefined
name, a `Tf` alone reports, the dangling name is refused, and both partly-read pages are refused by the rebuild.

**Plant.** A page whose content stream states `/Length 8700` over 62 bytes and whose form states it through an
indirect `8700`: written at 2b and 4 with each `/Length` the bytes', one packet, an `/ID`; read back by `pdf-syntax`
(the page drawn as the source's; the form's green is 1 level apart in red, through the output intent's profile), by the converter's own check (every requirement met) and by
veraPDF (PASS at both). The output with one `/Length` raised by 8: veraPDF FAIL 6.1.7.1-1, our check names
`file-structure/stream-length-matches-the-data`.

**Memory.** 50 000 undefined `Tf` with no show: 15 MiB high-water and no report, HEAD 26 MiB and 50 000 reports; with a
show under each, 27 MiB and HEAD 26 MiB, the same 50 001 lines. 50 000 dangling `XObject` names: 75 MiB, converted, HEAD 52 MiB,
refused.

**Gates.** In the export: `cargo clippy -D warnings` on `pdf-model`, `pdf-archive`, `pdf-transform` exit 0; `cargo nextest run` on the three exit 0 (3504 passed); `cargo test -p pdf-syntax` exit 0 (0 failed); the corpus gate exit 0 (974 documents, 59 incomplete, its list and the 2 `no /Font resource named` documents unchanged); `raster_golden` exit 0 (held 974, moved 0); `text_extraction` exit 0; the oracle under `--clock` exit 0; the six render-raster arms exit 0 each and 0 pages of 5 799 differing by digest against HEAD's six arms; `fuzz` `display_list` and `page` ten minutes each exit 0, 0 artefacts. In the worktree: `cargo check --tests` on the three exit 0; `--bin cited` exit 0; `--bin quotations` exit 0; `cargo test -p conformance --no-fail-fast` exit 101, four reds none of this round's (a quotation in `transparency_groups.rs`, record 1854's path, two `OrCmapRestated` names), and `--test records` exit 0 once this paragraph was written.

**Found, not fixed.** `doc/pdfa-blocking-issues.md` section 9's census lists still carry the two documents until the
census re-counts. `crates/pdf-transform/src/split.rs:1105` and other sentences write "entr(ies)".
