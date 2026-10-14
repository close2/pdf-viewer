# 1548 — An XFA form is said at the report, and a print through an output intent before its grant

Slot 3 of batch eighty-seven, 2026-10-14, a clause round building RFC 0009 §12's item 1 under A254.
ADR 1932; no question. No status moves. The notes of §K.1, §K.2, §14.11.5 and §13.4 are re-read.

**Premise.** Held for two of the three items: nothing read `/XFA` and nothing told a printer about
the output intent. It failed for the poster. ADR 1561 had already drawn §13.4's stream `/Poster`, the
day after RFC 0009 listed it as owed, and A254 copied that list. The poster also belongs to a `Movie`
annotation's movie dictionary. Neither a rendition nor a screen annotation has one; Table 306 is the
only place the word appears. **Hypothesis.** The poster half did not hold, since there was no refusal
to lift. The two-sentences half held: both are `Event::Reported` with no page, and no host was edited.

**Built.** `pdf_model::xfa::stated` reads Table 224's `/XFA` in §K.2's two forms and Table 29's
`/NeedsRendering true`. `notes::about` says in three kinds that this program does not implement XFA,
as §K.1 permits, and what it shows instead. The report is asked once, after the first frame. §14.11.5's
selection moved into `pdf_model::output_intent::selected`, which `output_intent_space` now calls.
`begin_printing` says, before `Event::Printing`, which devices the colours were prepared for, and that
Table 401's output transformation is not performed. Each new test fails with its sentence taken out.

**Measured.** A temporary census (deleted) over the 970 documents of `doc/pdf.js` and `doc/corpora`
that open: 7 state an XFA resource, 3 asking to be regenerated; 16 print through an intent, 14 of
them `GTS_PDFA1`. `doc/corpora` holds one page-level array, which a comment had said none does.
**Nothing moved.** The move out of `content/colour.rs` is checked by the multiset of its lines: only
the corrected corpus claim, the new name and the `Selected` return are not found again.
**Unfinished.** A `/DestOutputProfileRef` with no readable profile is still silent (ADR 1932 §4).
The clause 13 entry in `CLAUDE.md` is RFC 0009's item 3.

**Files.** `crates/pdf-model/src/{xfa,output_intent}.rs` (new), `lib.rs`, `content/colour.rs`;
`crates/viewer-core/src/{notes,viewer,command,open}.rs`, `tests/printing.rs`; `viewer-host`'s
`report.rs`; `doc/conformance/ledger.toml` (four notes); `doc/state-of-play.md`; ADR 1932.

**Gates.** rustfmt clean on the ten files. `clippy -D warnings --all-targets` on `pdf-model`,
`viewer-core` and `viewer-host`: exit 0. nextest `pdf-model`: 2060 passed. nextest `viewer-core` and
`viewer-host`: 646 passed. `cargo test -p conformance`: exit 0. `t2-raster_golden`: exit 0.
`t2-corpus`: exit 0. `tools/batch.sh check`: exit 0. `drive-windows --step drive` on this round's
export (quorra, gtk, qt): exit 0, 116 works, 0 wrong, and `16-print` works in all three. A drive of
all four windows on two XFA documents, two intent documents and a control: each sentence came once
where owed and never on the control. The confined window prints nothing, so it says only the XFA
sentence. Duration 8 000 s.
