# 1903 — A photographed page keeps the packet it names, a shared program is restated in a copy, and the second pass's statement is the report's

Batch 131, slot 3, an "archive" round: three defects of the converter's own. ADR 2640; ADR 2641 unused; no question.

**What changed.** (a) `064034`: the final revision reuses object 6 as the catalog's `/Metadata` while page 3, which
the font row photographs, still names it as `/Contents`. The hexadecimal-digit repair and the `/Lang` respelling
treated the packet as a content stream, wrote `0` before 29 `>` and lost the identification edit. A stream only
photographed pages name, other than one the survey opened, is now no content stream of the output
(`hexadecimal::taken_off_by`, `Completed::leaving_out`, `languages::Restated::leaving_out`), and `decision.rs` answers
the digit row by the photograph when nothing is left to repair. (b) `507618`: fonts F8 and F1 share descriptor 96
and CFF program 115; F8's widths disagree with the program, and its restatement overwrote the shared program, which
broke F1. Where another font's widths would break in place, `fonts::restate_metrics` writes the restated program and
a copied descriptor naming it, and `rewrite.rs` redirects that font's `/FontDescriptor` and counts the restatement;
`SHARED_PROGRAM_NOT_COPIED` refuses by name where no copy can be made. (c) `settle::fold` now joins the second
conversion's `statement` and `unstated` to the report (`fold_statement`).

**The premise that did not hold.** (a) was a reused object, not `prepare_properties`; (b) was a shared program, not a
glyph writer; (c): the second pass runs the first's plan and states its own losses on a page it appends after the
first's statement pages, so nothing went unstated on a page. Only the report lost that page. No document in the
corpora reaches a second-pass photograph: the construction refuses an Identity-H face, and a symbolic face's appearance
left the second pass refused (two probes), so (c)'s fixture is the fold's unit test.

**Measured.** `064034` and `507618` exit 0 at 2b and 4 under `as-if-printed`, and veraPDF passes all four outputs;
a planted `pdfaid:part=3` copy of `064034` and a planted `507618` fail. Against HEAD's census, without the pack the
corpora's `as-if-printed` move from 422 to 424 converted at 2b, from 430 to 432 at 4, and from 44 to 42 refused at
both parts. The six lines that changed are those two documents' `output-fails` lines. With the pack the same six lines
and counts move and nothing else; `as-shown` refuses both documents as before, for faults of their own.

**Plants.** Each fix removed in turn goes red: `leaving_out` (`064034`'s three metadata rows), the decision routing
(Mechanical), the shared-program check (`fonts/widths-agree-with-the-program`), and `fold_statement`'s join (its test).
**Unfinished.** The first statement cannot carry the second pass's entries (ADR 2640 section 3).

**Gates.** In the export: `cargo test -p pdf-transform --test archive --test profiles` exit 0
(526 and 13 passed), `--lib` exit 0 (193), `cargo test -p pdf-archive` exit 0 (253 in the library). Both censuses exit 0:
without the pack 335 s with a peak of 2.48 GiB, with it 325 s with a peak of 1.67 GiB. `rank.py` exit 0, with D-defect
at 8/8 for `as-if-printed` at both parts. In the worktree: `cargo check -p pdf-transform --tests` exit 0, clippy
`--no-deps -D warnings -p pdf-transform --all-targets` exit 0, and `rustfmt --edition 2024 --check` exit 0 on the
touched files. `--bin cited` exit 0, `--bin quotations` exit 0 (2851 verbatim, 3 diverging). `cargo test -p conformance
--no-fail-fast` exit 101 on one target, `records`, which fails on 1905's record ("a Gates paragraph with no exit status
or count"), no file of this round; re-run with this record, `records` exit 0 (3 passed).
