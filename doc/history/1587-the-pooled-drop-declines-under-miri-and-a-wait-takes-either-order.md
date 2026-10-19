# 1587 — The pooled drop declines under Miri, a wait takes its options on either side, and ten ADRs reach the state of play

Slot 3 of batch ninety-three, 2026-10-19, a docs round. ADRs 2010, 2011; no question.

**Premise.** It held: CI's `nightly` log stops at `display_list.rs`'s pooled drop, on crossbeam-epoch
0.9.20's retag at `internal.rs:562`. **Hypothesis.** It did not hold: one `cfg` was not enough. With
the drop guarded, Miri stopped again at `reach::tests::the_share_a_window_keeps_of_a_short_list_is_its_selection_counted`.
ADR 1994 added that test after CI's run, and it was the one test of its module without the guard.
**Built.** (1) `#[cfg(not(miri))]` on `DisplayList`'s `Drop`, its constant and its rayon import, with
the dependency and the Stacked Borrows rule named beside it; ADR 0450's `cfg_attr(miri, ignore)` on
the new test (ADR 2010). (2) `tools/wait.sh` takes options before or after the log. It refuses two
logs, a log beside `--start` or `--stop`, and an option with no value, each on one `wait:` line with
exit 64. Three self-test cases; the first, planted on the old script, exits 64 (ADR 2011).
(3) Comments rewritten to what is: `render-raster`'s header (`raster/`, not `../render-lib`);
the "# State" of `raster` and `raster-gpu`, and `raster`'s M2 parenthesis; `render-cpu`'s jobs in
principle 2's words; `quorra-confined`'s `Asking` and `Warned` arms (a click fills a field and hands
a rendition over, and the window sets no level); `archive/overprint.rs`'s `cmyk_tints` parenthesis,
now `ProcessTints::on` (ADR 1996). (4) `doc/state-of-play.md`: ADRs 1928, 1929, 1936, 1941 §2.3,
1943, 1956, 1957 §2, 1962, 1970 §3 and 1974, one clause each.

**Proof that nothing moved.** `cargo +nightly rustc -p pdf-render --lib -- -Zunpretty=expanded`,
run on an export of HEAD and on HEAD plus this round's two `pdf-render` files: 20 475 lines each,
byte-identical. The test build differs only in the line numbers of the tests after the new
attribute. The other edits are comments, ADRs and a shell script. The six arms of that export against
HEAD's arms, by page: 968, 968, 968, 965, 965 and 965 pages, 0 frame digests moved
(the compare, planted with one changed digest, names it); the worker's SHA-256 is HEAD's.

**Files.** `crates/pdf-render/src/{display_list,reach}.rs`, `crates/{render-raster,render-cpu}/src/lib.rs`,
`raster/crates/{raster,raster-gpu}/src/lib.rs`, `crates/viewer-ui/src/bin/quorra-confined.rs`,
`crates/pdf-transform/src/archive/overprint.rs`, `tools/wait.sh`, `doc/state-of-play.md`, ADRs 2010, 2011.

**Gates.** `cargo +nightly miri test -p pdf-render -p pdf-syntax --lib` (CI's line): exit 0, with
271 passed and 16 ignored in `pdf-render` (2 160 s) and 121 and 17 in `pdf-syntax`; exit 1 before
the second guard. `tools/wait.sh --self-test`: 15 of 15. `cargo nextest run -p pdf-render -p raster
-p raster-gpu`: 1014 passed. Clippy `-D warnings` on those three, `render-cpu`, `render-raster`,
`pdf-transform` and `quorra-confined`: exit 0. `rustdoc -D warnings` on `render-cpu`, `raster-gpu`
and `raster`: exit 0. `cargo test -p conformance --no-fail-fast`: 57 binaries, 440 passed, exit 0.
`--bin quotations` and `--bin pointers`: exit 0, no line naming this round's files. `tools/state.sh
--round 1587 docs`: exit 101, on slot 6's new `quorra-confined/pricing.rs:12` (`[c]`). Six arms:
exit 0 each, 1 599 s. About 7 300 s.
