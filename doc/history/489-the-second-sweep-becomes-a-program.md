# 489 — The second sweep becomes a program, and the splits test the instruments

**Finding.** A sweep round under `doc/todo/01`'s new binding rule. The second sweep — an entry a
note claims is unread, grepped against the tree — is `conformance --bin unread` now, and its
first run as a program found §7.5.5 disposing of the trailer's `/Info` as "unread because
§14.3.3 deprecates it" while the properties panel's reader takes exactly that entry. Running the
committed entries sweep at this head then caught the previous rounds' file splits in the
instrument's own mirror: 34 entries "moved" to *named only elsewhere* because a `code` path that
is a module root was read as one file — fixed in the sweep (`entries::covered_by`, Rust's own
module-root rule), not in the ledger, because the splits kept `content.rs` and `pdf-viewer.rs`
precisely so that citations of those paths stay valid. The table-number sweep found a ten-defect
block, all one shape (an entry attributed to the table its value points at); the retired-claim
sweep run over the noun `silent` found three rows using the ledger's own status word for things
that are not that; and the blame list's next band — seventeen rows, commits 138 to 165 — held six
wrong notes and one piece of work: §14.13.3 was `implemented` on a function nothing outside its
own tests called, so `attachments` now lists the catalog's associated files, deduplicated by
stream, and a payload a PDF/A-3 producer filed under no name reaches every host's panel.

**Date.** 2026-08-14.
**ADR.** [0324](../adr/0324-the-second-sweep-becomes-a-program-and-a-split-tests-the-instruments.md).

**Sweep results, verbatim from the runs.**

- `unread` (new): 62 rows claim, 171 keys; 55 confirmed, 116 quoted over 49 rows, 53 by the
  row's own code. One defect (§7.5.5's `/Info`).
- `entries`: 217 rows in the population; 140 entries over 43 rows before the module-root
  correction, 106 over 42 after; 29 named nowhere, 77 only elsewhere, 37 not named by the row's
  note. One row worked (§14.13.3, via its family).
- `quotations`: 2885 quotations in 425 documents, 1366 verbatim, 21 diverging, 0 defects — the
  divergences are corrections quoting retired wording plus two `doc/md/` conversion losses
  (Table 29's `/OpenAction` default, Table 176's wrapped row), both checked against the PDF.
- Ledger quotation marks (11): 1205 spans under a session-local normaliser, 633 verbatim, 45
  diverging, 0 defects. Expired blockers: 7 ledger / 14 source, none live. Capability reasons: 32 / 78, none live.
- Retired claim over `silent`, `Present`, `/SD`, the split paths: three `silent` defects
  (§14.12.4.1, §14.13.8, §14.8.6); two ADRs amended for retired table numbers (0284, 0295).
- Caller sweep: 280 distinct `pub fn` names in `pdf-model`, 92 unnamed by hosts, 76 by nothing —
  the known three populations. Arithmetic: §7.9.2 and §O, clean. Inapplicable: 70 of 80 name
  source vocabulary, none wrong. Citations: 3 hits, all the known correction-quoting-its-pointer
  shape. Table numbers: 409 headings, ~1000 citations, 105 suspects, ten defects (the block).
  Parent counts: 10 hits, 0 defects. Sweep 14: 24 hits, 0 defects. Errata: "151 struck passage(s) of 4 words or more that doc/md/ still
  carries as current text", unchanged, with every quoting landing a known in-place annotation.

**Rows corrected in this commit.** §7.5.5 (`/Info`), §8.11.1 and §8.11.4 (Table 98's
`/Configs`), §12.5.6.15 (Table 166's `/Contents`), §12.7.5.5 (Table 235's `/SV`), §14.7.2
(Table 29's `/StructTreeRoot`), §14.12.4.1 (Table 29's `/DPartRoot`; `partial`, not "silent"),
§14.13.8 (likewise), §14.8.6 (the status word retired from prose), §14.6.2 (the "and nothing
else" list), §14.9.2.2 (§9.8.3's `/Style` and `/FD` have readers), §14.8.2 (the reading-order
debt was paid), §7.7.4 (four trees read, not two), §14.13.3 (the new consumer and its tests).
Kept with evidence recorded: §14.9.2, §14.7.4.2, §12.11.5, §7.7.4's remaining six trees. Read
and kept without an edit: §14.8.6.2, §14.8.2.2.1, §14.8.2.2.2, §14.8.2.3, §12.11.3, §12.11.6,
§14.13, §14.13.2.

**Source corrected.** `view.rs` and `free_text_census.rs` (Table 170's `/N`), `requirements.rs`
(Table 257's `/P` level), `image.rs` (Table 5's `/DecodeParms`), `attachment.rs` twice and
`file_attachment_census.rs` twice (Table 166's `/Contents`), ADRs 0284 and 0295 (amended in
place, per ADR 0265's rule).

**Code.** `tools/conformance/src/unread.rs` and `bin/unread.rs` (new, with five unit tests);
`entries::covered_by` and its test; `pdf-model/src/attachment.rs::attachments` lists the
catalog's `/AF` files (two new tests).

**Touched.** `doc/conformance/ledger.toml`, `doc/todo/01-ledger-partial-rows.md` (the run's
record; three commands now), `doc/todo/02-every-round.md` §4 (the new command's line),
`doc/adr/0324-*` (new), the two amended ADRs, this file.

**Gates.** `cargo fmt --all --check` clean; `cargo clippy --workspace --all-targets` silent;
`cargo nextest run --workspace` all green; `cargo test --workspace --doc` green;
`cargo test -p conformance -- --nocapture` green (5 checks). No change reaches a raster, so the
corpus and oracle gates were not owed by this round's edits.
