# 2042 — XFA's three barcodes are built, and each reads back

Session 2042, a single ledger round, the second lane (`doc/questions/A857`; §K.2, row K). ADRs 2900 and 2901; no
question.

**The slice, and why.** A857's three items in the brief's order, each whole: Code 39's modulo-43 check character
and `code3Of9extended`, Data Matrix ECC 200, then PDF417 — the budget held all three, and they are the three
symbologies the census of 53 dynamic forms found (`doc/todo/78`).

**The premise.** Held: `xfa-barcode` built Code 39 from MIL-STD-1189A and refused `auto`, `code3Of9extended`,
`dataMatrix` and `pdf417` by name; A857 removed the only obstacle (no held text), so no Q869. The ISO/IEC 15438:2006
copy was fetched again and kept outside the tree; two of its pages (Table 5, clause 5.11.1) have no
text layer and were read from their images.

**Built.** Code 39: `VALUES`, `check_character`, `EXTENDED` from `rxing` (A857); page 440 ticks `auto` for
`code3of9`/`code3of9extended` and nothing for `logmars`, which ignores `checksum`. `data_matrix`: 24 square sizes
(page 442), six whole-message encodations and the shortest chosen, `0x12D` Reed-Solomon with interleaving, the utah
placement, the finder pattern, all derived from `rxing`'s encoder. `pdf417`: Annex A carried as data and held to
clause 5.3.1, Table 5, Text/Byte/Numeric compaction by Annex N's thresholds, clause 5.10's generator, Annex E's
default level, the template's matrix properties, row indicators, Compact PDF417. `encode_matrix` takes bytes (page
424). Each encoder bounds its input first and argues its arithmetic in one `expect`; `fuzz/`'s `xfa_barcode` holds it.

**Tests.** 32 in `xfa-barcode`: Annex R, Table 6, Annexes C, D, F, Q and clause 5.9.2's examples as fixed vectors;
every Data Matrix size and encodation end case, every PDF417 level, shape, row count 3 to 90 and Compact, and every
checked and extended Code 39 symbol read back by `rxing`. **Plants**: the field polynomial `0x11D` failed 5; an
Annex A entry's digits swapped failed 2; Code 39's `-` and `.` values exchanged failed 1; reversed, all pass.

**Gates.** Over `xfa-barcode`: `cargo fmt --check` 0; clippy `-D warnings --all-targets` 0; nextest 32 of 32;
doctests 0; rustdoc `-D warnings --document-private-items` 0. Fuzz: `fmt --check` 0, clippy `-D warnings --bins` 0;
`tools/fuzz.sh xfa_barcode` 0, a million runs, seeds 7 to 1015, 185 edges past the seeds. Conformance after
`git add -N`: 474 passed. `state.sh --round 2042 docs` exit 0, on 5432e3de. Fast-forwarded onto c77df9e2 (round 2041), then
nextest 32 of 32, fuzz `check --bins` 0 and conformance 474 passed again. Tier 2 not owed: no crate a walk builds
changed (`xfa-barcode` has no dependent), no script reached, `launch_path` not run.

**Habit, and what is left.** A table the standard prints is held to the clause that defines its cells (clause
5.3.1's widths, sum and cluster) as well as to a decoder: the parse of 2 787 entries is then checked by its own
definition. Row K stays `implemented`; what the drawing piece owes the barcodes — `charEncoding`, `dataPrep`, the
field's width into PDF417's columns, drawing — is `doc/todo/78` section 4's.
