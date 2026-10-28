# 1849 — A page gets the press it names, and pays for it from the memory its list is held to

Slot 10 of batch 125, 2026-10-28, a "clause" round (Sonnet 5.5, high); ADR 2532 used, ADR 2533 and Q715 unused. I worked
in `scratchpad/r1849/` and in two exports built there, their targets deleted. Ledger: §11.5.3, §11.6.6 and §11.7.2
`departed` to `implemented` (notes cite the clause and ADR 2532; `doc/todo/65` no longer lists them).
**Files.** `crates/pdf-colour/src/{colour.rs, colour/ink_search.rs, colour/press.rs, icc.rs}`;
`crates/pdf-model/src/{content.rs, content/list_budget.rs, content/report.rs, content/transparency/blending_space.rs,
content/transparency/press.rs, soft_mask.rs}`, `crates/pdf-model/examples/{press_census.rs, press_depth.rs}`,
`crates/pdf-model/tests/transparency_groups.rs`; comments in `tools/conformance/src/names.rs` and
`tools/safedocs/src/main.rs`; `doc/conformance/ledger.toml`, `doc/state-of-play.md`, `doc/verify.md`, `doc/todo/65`, ADR 2532.

**Premise.** "The bound is one constant and one fallback branch; the budget already charges a press's tables": the first
half held, the second did not. `MAX_PRESSES` (fourteen) and `NoPress::Spent` were the whole bound, but `MAX_LIST_BYTES`
charged the display list only, so fourteen presses held 15 MB it never saw.

**Built.** `PRESS_BYTES` and `Presses::within(bound)` replace `MAX_PRESSES`; a press is charged `PRESS_BYTES` plus
`Profile::owned_bytes` at admission, before any sampling; lookups are a `HashMap`. `ListBudget::passes` adds the presses
held to the list's charge, and `finished` asks `list_past(0)` once more so a page that ends past the bound says so.
`Profile::owned_bytes` charges everything the kept clone can hold: the colorimetric and "from CIE" tables and, for the
perceptual and saturation tags, their bytes and the largest table they parse into (the first charge counted the
colorimetric table only, 358 MiB against 1 033 MiB resident for 24 two-way profiles).

**Found, and what moved.** No page of the project's corpora names more than fourteen presses (pdf.js 1 283 pages: none
name any; SafeDocs every 11th, 13 188 pages: 83 name one), so the six arms differ on 0 of 968 and 965 pages. A hostile
page of 1 516 presses costs 1 472 MiB resident and 38 s of CPU under load 15 to 34, where HEAD held 36 MiB and refused
after fourteen; 24 two-way profiles at 33 points are charged 1 229 MB against 927 MB resident. The page one past the
bound is refused as `ListBytes`, not as a group in its parent's space (ADR 2532 sections 3 and 5).

**Gates.** `cargo test -p pdf-colour` exit 0 (155 passed, 1 ignored); `cargo test -p pdf-model --no-fail-fast` exit 0 (2 343
passed, 22 ignored), both with the sandbox and transform workers built; clippy `-D warnings`, `cargo fmt --check` and
`cargo doc` `-D warnings` for both crates exit 0; `raster_golden` and the pdf-model corpus gate exit 0; the six arms by
page against HEAD's: 0 differ; `press_depth` over pdf.js and the crawl exit 0; the memory probes under the lock, HEAD
beside this tree; `launch_path` (release, clock hold, load 11 to 16): exit 0, its six `peak_anon_mib` figures in band
(11.7 to 53.9 MiB), 17 clock figures outside under load and not judged as memory; `--bin cited` and `--bin quotations`
exit 0; the whole `cargo test -p conformance --no-fail-fast` exit 101 with one red, a sibling's (1841's record is 41 lines). **Validation.** An Opus validator failed the first reading on one behaviour defect — the press charge counted one table where the kept copy held four, so a hostile page would have killed the worker before being refused — and on the record's file list, history phrasing and an unrun memory gate; it passed the round after the charge was made to cover the kept copy, its probe re-run at 1 185 MiB charged against 1 032 resident.
