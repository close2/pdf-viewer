# 2151 — A cross-reference stream counted only where it could pass the budget

Session 2151, a measure round in the `lane3` worktree, on round 2146's three counted opens above their bands (EC3,
AN001, `xfa_filled_imm1344e`). ADR 3305; no ledger row moves (7.4, 7.5.4, 7.5.8.2 keep `implemented`; 7.5.4's row
gains the new test); no question.

**The cause.** Round 2140's saved profiles of `2c1e7568` (neither `3c3ed860` nor `4752821f`) diffed by function
against HEAD's: the growth is ADR 3257's `in_use_records`, a pass over every record's type byte before a stream's
entries are held, inlined into `read_section`'s closure — 2.36 M of EC3's open, 5.4 thousand of AN001's, 2.3 of the
XFA form's. The sorts did not grow (the `Filed` sort and the map's bulk build cost less than the ones they replaced),
EC3's newest section frees nothing so `fill`'s `retain` is skipped, and `4752821f`'s share was a parameter
dictionary cloned twice by `chain_of` and the cross-reference stream's `/Filter` and `/DecodeParms` cloned before
being borrowed.

**The change.** `read_xref_stream` counts only where the fewer of the `/Index` total and the records the data
carries is past what is left of `MAX_XREF_ENTRIES`; otherwise that number is the capacity. ADR 3257's property — a
stream past the budget costs its decode and no more — holds, and the cost (address space for free records, never
resident) is written down in ADR 3305. `chain_of` moves each dictionary to its stage; `decode_direct` borrows.

**Tests.** `a_stream_of_more_records_than_the_budget_whose_entries_are_free_is_read` (pdf-syntax
`xref_entry_budget.rs`): 2²³ + 1 records, three in use, read through the counted path.

**Figures** (thousands, this program's object, release under 1.100.0, `count-open` children): EC3 56 427.4 →
54 067.8, AN001 2 227.4 → 2 220.1, `xfa_filled_imm1344e` 287.2 → 284.8, WTPDF 10 177.0 → 10 107.6, `bug1815476`
1 768.2 → 1 764.2, `opt_demo` 406.8 → 407.1. EC3's first frame, which counts its open, 128 057.8 → 125 705.2. Both
EC3 bands move below their old lower edges by the file's rule (ADR 3305); no other band moves.

**Gates.** fmt 0; clippy `-D warnings --all-targets -p pdf-syntax` 0; `pdf-sandbox --bins` 0; nextest `--lib
--bins --tests -p pdf-syntax` under `xvfb-run -a` 0, 426 passed, against an empty manifest; rustdoc `-D warnings
--document-private-items` 0; conformance after `git add -N` 0 (478 passed); `launch_path` 101 before the bands
moved (EC3's open and first frame below), then 0, 36 banded; `turn_path` 0 through `state.sh` (33 judged, 0 outside),
then a direct run at load up to 205 put three clock figures 1 to 8 % past their tops (`images.pdf` step,
`issue14415` and `issue19802` seventh), on pages already open, which this change does not reach. No public type
changed, so no fuzz check was owed.

**Left.** Nothing of the three opens. A sorted `Vec` in place of the table's `BTreeMap` would save the bulk
build's second sort of already-sorted entries (0.8 M of EC3's open); not measured beyond that profile line.
