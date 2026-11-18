# 2079 — The model's audit groups G051 to G078 are answered

Session 2079, an audit-fix round in `pdf-viewer-lane3`. Groups G051 to G078 of
`doc/reviews/2038-specification-audit/fixes.md`: 28 findings (0 B, 24 T, 4 D), and the carried B46 and B73.

- **Fixed 28, argued 0, owner 0, open 0** in the groups; B46 fixed, B73 left `open`.
- B46, ADR 3001: `file_spec.rs` divides a file specification at every SOLIDUS. The 256-component cap merged the
  tail into the last component, a name the producer did not write; the parser's `max_string_len` already
  bounds the cost.
- D29, ADR 3000: ADR 0420 §5's "a stroked path is one object" was an assumption about the tree, which B39
  refutes. The code change stays G092's.
- D20: `image/jpx.rs` and 7.4.9's note separate §7.4.9's producer sentence (19 excluded from image data) from
  the reader's (the JPX baseline, which holds 19); the departure is from the reader's, by A348.
- D45: `text_layer` and `viewer-core` `headless` named `issue1155.pdf`, which pdf.js does not hold, and passed
  by returning early. Both tests now build their page, and both corpus helpers panic on a file missing from
  a present corpus.
- D51: the PDFIUM-407-0 row of `doc/checks/fixed-documents.toml` carries ADR 2870's substitute-face report and
  the ink it draws; corpus batch5 is on this VM again, and the ignored gate passes.
- T06 (`ccitt_bound`) and T103 (`damaged_page_dictionaries`, the narrated blue rectangle now rendered) are
  closed by test edits; T33's comment was the audit's own.
- Every other T test was plant-checked in this worktree in four batches of disjoint plants: each fails on its
  plant and passes restored. T156's first two plants (in `samples_of_frame` and in
  the direct-output gate) did not reach the test's route; the third, in `decode_jpeg`, did.
- Owner audit file edited (A868): `named_page_geometry_spec_audit.rs`'s font `/BaseFont /InheritedOnlyetica`, a
  renaming accident, is `/Helvetica` again (T113 and the "failure with no finding").
- No ledger status changed; 7.4.9's note was reworded.

**Gates.** fmt exit 0; clippy `-D warnings --all-targets` (pdf-model, viewer-core) exit 0; nextest
`--lib --bins --tests` of pdf-model and viewer-core exit 100, 7 failed, equal to the manifest's 7 for those crates;
pdf-transform `archive_external_components_spec_audit` exit 0; rustdoc exit 0; conformance exit 101 on this record's
own Gates paragraph only, exit 0 with the paragraph written (`--test records`).

Trap: a warm-up `cargo build --workspace --all-targets` held this worktree's build lock for 40 minutes, and the
shared `sccache` queued every compile at 0 % CPU under a load of 190. Start the first test build at once and
let it queue; there is nothing to do about it but other work.
Habit: a corpus helper that returns `None` for a missing file turns a renamed corpus file into a passing test.
Panic when the corpus directory exists and the file does not.
