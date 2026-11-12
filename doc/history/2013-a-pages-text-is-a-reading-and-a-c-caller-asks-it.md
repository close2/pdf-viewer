# 2013 — A page's text is a reading, and a C caller asks it

Session 2013, a single ledger round, the ledger lane. `doc/todo/76` rows 5 and 8; rows 9.10.1 and 7.6.4.2;
ADR 2844; CI's red `test` job.

**The slice, and why.** The brief's order: decide what `Query::Text` is under Table 22's bit 5, build the C ABI's
accessor CI's `test` job could not compile without, then row 8 if the budget remained. The first two are done; row 8
is left. Rows 6, 7 and 9 and ADR 2833's reporting are later rounds'.

**The premise.** Held: the accessor was unbuilt and effort only, and `every_query_reaches_the_abi.rs` failed to
compile on `&Query::Text`. The decision was not the owner's: Table 22's text and `CLAUDE.md`'s restrictions section
settle it, so no Q844. ADR 2836 is session 2009's, not this round's as the brief said.

**Decided (ADR 2844).** `Query::Text` is a reading, answered at every level. Bit 5 is asked at `Command::Copy`, the
gesture whose events an `ask` or `warn` needs; a query raises none, so a gate there could only be a refusal that
never becomes an ask. `Find` and §14.9's tree already read the characters, the second under bit 5's own carve-out
for assistive technology; bit 10 "PDF readers shall ignore".

**Built.** `quorra_text_pages`, `quorra_text_page`, `quorra_page_text` (two-call idiom) in `abi/page_reads.rs` over
`Session::text_pages`, `text_page`, `page_text`; `quorra.h`, `Native.OtherHalf.cs`; `c/open_a_page.c` prints the
searched page's bytes the search named; the exhaustive arm and sample; the two stated counts, 375 and 356.

**Tests.** `restriction_levels.rs::the_pages_text_is_a_reading_the_copy_level_does_not_withhold` (`bug1815476.pdf`,
`/P -1084` clears bit 5, copying `on`: copy refused, text answers); `session.rs::a_pages_text_is_a_reading_and_
answers_where_the_copy_is_refused` (bit 5 withheld: `Seiten`, U+00AD by §9.10.2's glyph-name method, the `'`
operator's line, `zahl.`); the C program's "Black Point" at bytes 57..68, the §9.4.4 reading the search line rests
on. **Plant**: `Query::Text` answering an empty list fails all three; reversed, they pass.

**Row 8 not taken.** ADR 0705's dots are a scene-reuse optimisation and other marks consume a scene too, so pricing
it needs each corpus scene's consumer counted, then a zoom clock: a measure round. 9.10.1 and 7.6.4.2 stay
`implemented`, each note naming the reading; row 5 is closed.

**Gates.** fmt `--check` and clippy `-D warnings --all-targets` on `viewer-core`, `viewer-ffi`: exit 0; nextest of
both, C programs included: 619 of 619 (the first run failed three: the two hand-stated counts and a C line printed
on page two; fixed); rustdoc `-D warnings --document-private-items` on both: exit 0; fuzz `cargo check --bins` not
owed (no public type of `viewer-core` changed); conformance after `git add -N`: exit 0; `state.sh docs`: exit 0,
re-run at the merge after it was stopped queued behind round 2011's and round 2012's walks. Tier 2:
`selection_census` exit 0 (974 documents), `accessibility_census` exit 0 (1 020, every ratchet at its floor). Not
owed: `corpus`, `raster_golden`, `awkward_classes`, `launch_path`, `script_corpus` (no model, pixel, confined,
launch or script code reached).
