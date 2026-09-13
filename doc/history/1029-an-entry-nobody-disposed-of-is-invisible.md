# 1029 — the launch action the clause declines, and sixty-four entries nobody had disposed of

Date: 2026-09-13. No ADR: nothing here needs protecting from re-litigation. Touched:
`crates/pdf-model/src/action.rs`, `tools/conformance/src/unread.rs`, `doc/conformance/ledger.toml`.

`--bin entries` read 155 entries reported over 47 rows, **65 undisposed by the owning row's own
note**; it now reads 146 over 44 and **1** undisposed — §12.8.4.4's `/TU`, left to the rounds that
own §12.8. `--bin unread` read 195 keys over 76 rows, **114 quoted by the row's own code**; it now
reads 115 and **none of the 114 closed**, for a measured reason below.

**§12.6.4.6's launch action is declined for the reason the clause gives.** Table 207 requires `/F`
only "if none of the entries Win , Mac , or Unix is present", and states the rest itself: "If this
entry is absent and the interactive PDF processor does not understand any of the alternative
entries, it shall do nothing." Nothing here understands the three — `/Mac` and `/Unix` are typed
"(undefined)", `/Win` is deprecated in PDF 2.0 — so an action with an `/F` is withheld by
principle 3's sandbox and one without is declined by the standard, and those are different facts
about a file. `action::launch` reads all four and §7.3.9 decides what present means, so a null
`/F` is an absent one; Table 208's bytes do not reach the sentence, because `Action::Refused`
carries this program's vocabulary rather than the document's. Calibrated by making `launch` answer
the sandbox's sentence for all three shapes, under which the test's two `assert_ne!` lines fail.
Inert on what is measured: of 978 documents in `doc/pdf.js` and `doc/corpora`, one states a
`/S /Launch` and it states an `/F`. Five more entries closed by naming the file that already reads
them: §7.6.2 did not name `crypt.rs`, which reads Table 20's `/CF`, `/StmF` and `/StrF`; §9.7.4.1
did not name `loading.rs`, which hands the descendant CIDFont to `Request::derive` and so reads
Table 115's `/BaseFont`. Two of the 65 were not entries — `doc/md/` transposes Table 165's and
Table 200's Key and Value columns, so `/name` is `/Type` and `/dictionary` is `WC`/`WS`, checked
with `pdftotext -layout`.

**Why none of the 114 closed.** The sweep takes every `/Key` in a sentence holding a claim phrase,
and two ledger habits put read keys there: a **correction carries the claim it retires**, and a
**calibration describes a planted defect** ("`/Mask` never read fails seven tests"). Neither is a
statement about today's tree; both are now in `unread.rs`'s documentation with the way to see the
split. Five of this round's own disposal sentences named a read key beside an unread one and were
split, so no claim of this round's is false.

**Gates.** `rustfmt --check` on the two sources 0; `RUSTFLAGS="-D warnings" cargo clippy
-p pdf-model --lib` and `-p conformance --all-targets` 0; `cargo test -p conformance` 0; `cargo
nextest run -p pdf-model --lib` 0 (432); `-p pdf-model --doc` 0; `--profile gates -p pdf-model
--test actions -- --ignored` 0. `cargo fmt --all --check`, `clippy --workspace` and `nextest -p
viewer-core` fail only in siblings' sources and tests, none touched here.
