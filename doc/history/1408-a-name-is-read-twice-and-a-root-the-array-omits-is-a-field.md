# 1408 — A name is read twice, and a root the array omits is a field

Script slot of batch sixty-four. ADRs 1652, 1653; no ledger status moves (§12.7.4.2's note gains
the recovery and two tests), no question.
**Premises.** `AFExactMatch` absent, Table 224's `/Fields` at `doc/md` line 12421, round 1402's
column figures: held. "Adobe's reference, cited" did not: no page of `adobe/dc-acrobat-sdk-docs`
at `ab3b42a7` names `AFExactMatch`, so its rules are choices (ADR 1652 section 3).
**Names (ADR 1652).** `getField` matches as written, then reads a name that matched nothing again
without white space at either end and trailing PERIODs (§12.7.4.2 rules a PERIOD out of a partial
name), saying so in the run's notes. `name.N` of a terminal field is refused by name rather than
`null`; one census run (`5712688.pdf`'s template format) reaches it and now throws where it
finished on `null`. `Field.style` is refused by name instead of a silent own property.
**Omitted roots (ADR 1653).** A widget a page lists whose `/Parent` chain reaches a named root no
`/Fields` entry reaches has that root walked as an entry. `examples/orphan_field_census` (planted
file found first): 79 documents, 4 482 roots; `PDFBOX-3094-2.zip-2.pdf` (273 of 490) was a PDFBox
2.0.0 merge its reporter called a defect. **It broke the launch band, and the A/B says so**
(round 1411's finding; EC3 first-page `peak_anon_mib`, three children each, exports in their own
target dir): HEAD 28.4–30.1 MiB, HEAD + this round 77.4–78.8, + the fix 28.2–28.9. The cause was
`apply_resumed`, asked after every command, building the every-page table before looking for a
finished run; it now looks first, and the table has four widths (`view::Omitted`): one page for
`form::fields`, `field_at` and the open and page-turn scripts, `/CO`'s entries for the open's
recalculation, `/Fields` first for a repaint, every page only for what a person asked of the
document. The realm is told of a field the first time a wider table names it (`Told::names`).
**Per site** (threw, before → after): Calculate 9 488 → 9 263, Keystroke 18 → 14, Format 2 → 5,
Page(Open) 4 → 8, Validate 26, Library 1, WillPrint 1, DidPrint 1 unchanged; runs 20 860 → 21 100
(fields 3 845 → 3 903, the recovered roots' scripts), finished 11 311 → 11 773, threw 9 541 →
9 319, refused-and-finished 0. Ceiling and floor ratcheted to 9 319 and 21 100.
**Unfinished.** One widget of several (a `FieldState` per widget across the wire); `Field.style`'s
write (a check box glyph redrawn); `app.fs` (not in RFC 0008 section 4.2's `app`).
**Gates.** `rustfmt --check`: 0 on my files. Clippy 0 in my files; `-D warnings` exits 101 on slot
5's `rich_text/layout.rs` and `variable_text.rs` only. `cargo test -p conformance`: 101 on slot 5's
§12.7.4.3 test name only. Behind the lock: HEAD's Tier 1 column 0 (108 s); after, Tier 1 column 0
(146 s), worker column 0 (171 s, 149 workers, 0 lost, 0 `SIGSYS`, count for count), `pdf-model
--test corpus` 0 (974 documents), Tier 0 `script_corpus` 101 — 4 moved, 9 unheld, all 13
recovered-root documents — regenerated (`tests/script_corpus.tsv`), then 0 (356 held, 0 moved). The
second hold queued 3 736 s behind an `sccache` server that inherited the lock. After the fix, one
hold: `launch_path` 0 (0 outside, EC3 28.453 MiB), Tier 1 column 0 (21 100 runs, 9 319 threw, as
before), Tier 0 0, corpus 0; nextest `pdf-model` 1 965 / 1 965, `pdf-script` 98 / 98, worker 25 /
25.
