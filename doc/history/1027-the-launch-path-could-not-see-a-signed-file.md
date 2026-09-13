# 1027 — The launch path could not see a signed file

Instruments slot of batch 1026–1031, on `batch-1026-1031`. A gate that passed because its population
excluded the case it was for.

**1. The hole.** `crates/viewer-ui/tests/launch_path.rs` banded principle 2's figures over four
documents — 5, 57, 1023 and 1 pages — and **none was signed**. `viewer_core::notes::about` ran
inside `Command::Open`, and its §12.8 answer makes four whole-range passes: opening a signed
document digested most of the file before page one existed, with nothing measuring it.

**2. Widened first, then watched to fail** (trap 13). `doc/pdf.js/test/pdfs/xfa_filled_imm1344e.pdf`
is the largest signed document in the tracked corpora — 3.0 MB, one page, named in `notes.rs`
already. As a fifth row with the defect still in place the gate said `read_kib` 1303.982 outside
97 .. 102, `read_calls` 118 outside 24 .. 28, `open_kinstructions` 47639.835 outside 1783 .. 1820.

**3. The fix.** `notes::about` left `Command::Open` for a new `Command::Report`, behind a
`OnceCell` on `Open` so a second asking costs nothing. After: **99 KiB, 26 calls, 1801.6 k
instructions** — bytes ÷13, calls ÷4.5, instructions ÷26, warm open 2.34 → 0.34 ms. The four other
rows kept every band; the saving reached them too (5089.8 → 4942.3 and 3446.5 → 3394.4 thousand), so
**two floors moved down by the measured saving, no ceiling moved, nothing widened** — session 925's
`turn_ms` step in another unit. ADR 1044 argues why a command rather than a render acknowledgement.

**4. Still reached, proved twice.** `viewer_host::report::Due` is armed by `Event::Opened`, spent by
the frame a host has just presented — one rule, four hosts, with `Command::Report` crossing the
confined protocol and `quorra_document_report` reaching the C ABI. `headless.rs::what_a_signature_
covers_is_said_when_the_document_is_asked_and_not_when_it_opens` asserts **both** halves: the open
says nothing about a signature, asking says all of it. `viewer-host`'s new sweep derives the host
population from the tree and fails if a crate arms without asking — calibrated by deleting one call
and watching it name `viewer-qt`.

**5. Gates.** Tier 1 green, exit 0 on every line: `fmt --all --check`; `RUSTFLAGS="-D warnings"
clippy --workspace --all-targets`; `nextest run --workspace` (4569 passed, 0 failed); `--workspace
--doc`; both `fuzz/` lines; `-p conformance`. Tier 2's one applicable line, `--release -p viewer-ui
--test launch_path --ignored`: **5 documents, 42 figures banded, 0 not judged, 0 outside**,
calibration 0.686 ms in band. §12.8.1's ledger note now says the reports are off the open path.

**6. Refused.** The new row's three clock figures state `none`: a clock band needs a quiet machine
and five rounds were running (`doc/todo/05`'s rule). A bigger witness,
`corpus-cache/safedocs/cc-main-2021-31/7557/7557575.pdf` — 45 KiB read without the report, 7374 with
it — stayed out: `corpus-cache` is a download and an absent row is a skipped one.
