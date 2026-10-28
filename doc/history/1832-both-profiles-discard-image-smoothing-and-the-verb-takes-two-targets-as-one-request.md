# 1832 — Both profiles discard `image-smoothing`, and `--or-else-to` is one request for two parts

Slot 1 of batch one hundred and twenty-four, 2026-10-27, an archive slot (Sonnet, high), ADR 2490's first
two remedies, R02 and R06b; ADRs 2498 and 2499 used, Q698 unused. I worked in `scratchpad/r1832/`; the round ran as two sessions, the second continuing from a handover. No
ledger status moves (§8.9.5.3 is `implemented` and stays; the clauses are ISO 19005's, which hold no row).
**Premise that did not hold:** ADR 2490 says every loss is in `xmpMM:History`, and `image-smoothing` was
in the report only; `archive/smoothing.rs` writes the entry and it is a condition of the rewrite (ADR
1964's pattern). The first build dropped it silently when the packet already identified the file, found by
the test and fixed in `Rewriter::wants_metadata`.
**R02.** `doc/profiles/as-if-printed.toml` and `as-shown.toml` answer `graphics/image-interpolation-is-off`
with `discard`; §8.9.5.3's sentence is quoted verbatim in ADR 2498 (re-read in `doc/md/`). The inline `/I`
(`TAMReview.pdf`, `bug1065245.pdf`) keeps its refusal; a test asserts it. **R06b.** `archive --to 2b
--or-else-to 4`: `convert`, `fall_back`, `Fallback` (JSON `refused_first`), exit 4 when both refuse, a usage
error with a tool-declaring configuration; `ArchivePlan` is not widened. **Census** (ADR 2499, four runs,
with and without the pack): freed 39, 36, 36, 34 documents against the bounds 41, 39, 38, 37; the request
writes 26 documents at 4 after 2b refused under `as-if-printed`, 21 under `as-shown`.
**Files.** New `archive/smoothing.rs`, `archive/fallback.rs`, `tests/archive/image_smoothing.rs`,
`tests/archive/or_else_to.rs`, ADRs 2498 and 2499; edited the two profiles, `archive/{mod,report,prepare,
rewrite}.rs`, `bin/quorra-transform.rs`, `tests/archive.rs`, `tests/archive_corpus.rs` (the either-part
census line), `doc/pdf-a-mitigations.md`, `doc/todo/72`, `doc/state-of-play.md`.
**Found, not fixed.** The windows' archive command (`viewer_host::archiving`) does not offer a second
target. `475121.pdf` at 2b: 40 content-stream integers past 2147483647 that this tree's validator misses
and veraPDF fails (§6.1.13), alike at HEAD under `--authorise image-smoothing`. `poppler-67295-0.pdf` at 4:
veraPDF throws on the file HEAD writes too. `Montenegro2021.pdf` at 4 is refused for its signature, no
decision of the table's.
**Gates.** `cargo fmt --check -p pdf-transform` exit 0; `RUSTFLAGS="-D warnings" cargo clippy -p
pdf-transform --all-targets` exit 0; rustdoc `-D warnings --document-private-items` exit 0; `cargo nextest
run -p pdf-transform` in the export, the sandbox worker built first (habit 185), 760 passed, exit 0 (mine: 8 of 8); the planted R02 and R06b each fail 2 of those 8; census runs exit 0 (4 logs, `rank.py` exit 0);
veraPDF 1.30.3 145 R02 files, 143 PASS, 2 FAIL (`475121.pdf`), and 47 R06b files, 45 PASS, 2 ERROR
(`poppler-67295-0.pdf`); `--bin cited` exit 0; `--bin quotations` exit 0; `cargo test --profile gates -p pdf-transform --test archive_corpus -- --ignored --nocapture` in the export exit 0, 7 passed; `-p pdf-archive --test cross_check -- --ignored` exit 0, 1 passed; `tools/state.sh --round 1832 archive` exit 0; `cargo test -p conformance --no-fail-fast` in the worktree exit 0 (the export, which is no git tree, fails the tests that read git, none of them mine). The Opus validator's six findings (comment, record, two ADR sentences, a test citation, a doc comment) were fixed by me. **Validation** (the orchestrator): the Opus validator's verdict was FAIL on six wording and paperwork findings — a stale comment, a refuted test figure, a non-conforming fallback stated, an incomplete list, a wrong reason, a target name — and PASS on the behaviour, the plants, the census and the gates; the round fixed them.
