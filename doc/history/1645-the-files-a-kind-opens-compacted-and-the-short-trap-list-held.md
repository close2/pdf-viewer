# 1645 — The files a kind opens compacted, and the short trap list held to the index

Session 1645, slot 7 of batch one hundred, a docs round; 2026-10-20. ADRs 2126 and 2127. The
owner's words: "go through the md files, which are read by every subagent. Can we compact them?
Is all information still valid?" — the second half, after ADR 2110's.

**Done.** `doc/state-of-play.md`, `doc/verify.md`, `doc/ui-boundary.md`, `doc/third-party-data.md`,
`doc/stack.md` and todo 01, 30, 56, 57 read whole by ten parallel readers and checked against the
tree under ADR 2110's rules; every deletion in ADR 2126 §3. ADR 1751, cited nowhere after the pass,
had its capability put back. `tests/every_round_rows.rs` holds `every-round.md`'s rows to the index's;
`tools/fuzz.sh` builds with `-s none` from scratch under a data limit (`tests/fuzz_wrapper.rs`); the
traps print, `Cargo.toml`'s comment and `drive-windows.sh`'s pointer fixed; trap 221 re-pointed.

**Measured.** Words, `wc -w`, over the set: 104 337 → 84 486 (−19.0 %), each file in ADR 2126 §1;
`state-of-play.md` 20 376 → 20 266, `ui-boundary.md` 16 485 → 6 670, todo 57 3 490 → 1 719.

**The premise that did not hold.** `state-of-play.md` carries almost no printed counts or restated
ADR titles: it is capabilities with their clauses and ADRs, which the rules keep. The words were in
files that had grown as logs — `ui-boundary.md`'s status block, todo 57's built work, todo 56's
answered questions.

**Found false, corrected.** 100 claims (ADR 2126 §2), among them: the owner's current target; the
toolkit windows "draw no page through a graphics device"; the command lists read as complete and
missing 39 variants; the codecs "from upstream"; `cargo fuzz` "no such subcommand"; `pdffs`; seven
ledger rows todo 56 called unbuilt; ISO 21757-1 and RFC 5280 quoted verbatim.

**Outside this slot.** `fuzz/seeds.sh`'s `x509` arm reads `crates/pdf-model/src/*.rs`, where the
certificates no longer are; A01 and A02 answered and neither built; `drive-windows.sh` lines
2035–2037, `doc/todo/15` lines 56–57, `pdffs` in `crates/pdf-fuse` and `doc/crate-map.md`, `NOTICE`'s
`pdf-viewer`, and six stale comments (ADR 2126's readers' reports, in the round's report).

**Gates.** `cargo test -p conformance --no-fail-fast`: 101, on `round_numbers` alone, naming slot 2's
`crates/pdf-model/examples/r1640_isolate.rs`; every other test 0, `every_round_rows` 2 passed,
`fuzz_wrapper` 3 passed, `traps` 0. Both calibrated by a planted defect: a drifted row 2 failed the
first, an emptied `-s none` the second. `--bin pointers` 0 and `--bin quotations` 0, neither naming a
file of this slot. `cargo fmt --check -p conformance` 0; `RUSTFLAGS="-D warnings" cargo clippy -p
conformance --all-targets` 0. `tools/state.sh --round 1645 docs` 0 after 40 s. `tools/state.sh traps`
prints "cited with no row in the index: 2 (trap 0, trap 632)". `bash -n` 0 on the three scripts.
Duration: about 2 000 s.
