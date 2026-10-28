# 1852 — An operator's configuration is read at all six targets and may name a shipped base, and a derived PDF is converted before it is attached or appended

Slot 2 of batch 126, 2026-10-28, an "archive" round (Opus 5.5), four sessions (4176 s, about 2350 s, about 2250 s and
about 7300 s, most of the last queued for the walk lock); ADRs 2538 and 2539, Q718 unused. No ledger row moved.

**Premise.** A706 adopts RFC 0014 section 9 whole. `base` held as a merge of two files' tables; `derive` is in
`remedies.rs`, not `external.rs`; "stage 2 is the verb on the derived bytes under a child budget" held (`run_pass`).

**Built.** S1: `--config`/`--profile` read at every target before any document; `--remedy-sites` with no `--to` prints
six listings; `[profile] base`, one level, each site `(this file)` or `(from <base>)` (ADR 2538). S2: `derived.rs`
converts a `derive` tool's PDF at the part sections 6.8 and 6.9 admit (2b, plain 4), one level by the plan's shape, each
file charged against one stream's ceiling before it is opened; part 4 states `/AFRelationship /Alternative` (ADR 2539).
S3: `derive:<tool>:append` — each derived page a form in a wrapper invoking its shown annotations' appearances (§12.5.3,
§12.5.5), pages, object numbers and `/Page` references bounded before the work, the inner intent's families licensed by
the archive's, a tagged document refused by name; `--archive-profiles <dir>` in viewer-host (ADR 2538 item 6);
`no-data-loss` and `state-every-loss` on slot 1's `try` and `append-statement`, eight shipped. S4: a substituted font's
`xmpMM:History` entry names its resource (`Helvetica (resource F1)`), with a test; ADR 2539's Table 66 is Table 65.

**Plant.** A derived PDF holding a CSV attachment: no second program runs, the row is `not-converted`. veraPDF: attach,
append and drawn-page outputs pass at 2b and 4, as do `state-every-loss` 2b and `no-data-loss` 4 outputs (0 failed
rules); each planted with `/OutputIntents` renamed fails section 6.2.4.3 tests 2 and 4. **Probe** (max RSS by
`getrusage`, gates build, HEAD's binary beside): a tool's page decoding to 1 GiB, 1 076 436 KiB attached, 2 125 972 KiB
appended; HEAD costs 1 069 048 KiB on that PDF as a source and 2 108 020 KiB on the appended
output, so stage two adds nothing of its own. 64 pages of 1 024 forms: 260 216 KiB attached, appended refused at
`MOST_IMPORTED_OBJECTS` (260 556 KiB), HEAD 240 044 KiB as a source. **Census by hand** (the harness runs two
profiles only), 24 R15/R16 documents, without and with a CJK pack copy, identical: `no-data-loss` converts 5 at 2b
and 4 at 4, `state-every-loss` 7 and 4; every other run exits 4 naming its refusal.

**Gates.** `cargo test -p pdf-transform --test archive --lib --test profiles`: 0 (474, 170, 12 passed); `-p viewer-host
--lib`: 0 (229); `cargo check -p viewer-gtk -p viewer-qt`: 0; clippy `--no-deps -D warnings`, pdf-transform and
viewer-host: 0.
`tools/state.sh --round 1852 archive`: 0 in 5312 s — sites unbuilt 19 of 65 (2b), 13 of 54 (4); validator 0 missed,
0 over at every target; `archive_corpus` (exit 0, 219 s) 2b 375 conforming kept 375, 540 converted, 71 refused, 0
unreadable; `cross_check` (exit 0, 35 s) 13 560 selections, 0 disagreements. `--bin cited`: 0. `--bin quotations`: 0.
`cargo test -p conformance --no-fail-fast`: 101 (`records`, the 1853 and 1858 records), then 0 after this. My duplicate
`archive_corpus` was killed by its group just after its grant; state.sh's ran whole.

**Found, not fixed.** `crates/viewer-ffi/src/abi/conversion.rs:26` offers only the shipped profiles, so the WinUI and
macOS windows have no operator directory. The checker holds a form a form invokes twice (2 108 020 against
1 069 048 KiB for the content on the page). `derived.rs:247`'s §12.5.3 is not in that row's `code`.
