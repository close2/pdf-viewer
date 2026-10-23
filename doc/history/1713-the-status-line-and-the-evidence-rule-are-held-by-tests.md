# 1713 — The Status line and the evidence rule are held by tests

Session 1713, slot 2 of batch one hundred and ten, a docs round; 2026-10-23. ADR 2260; ADR 2261
unused; no `Q`. No clause cited, no ledger row moved, no crate but `tools/conformance` touched.

**Premise and hypothesis.** ADR 2242 decisions 3–4 and review 1704 findings 3, 5, 7 and 8 held in
the text. The premise that `no_record_is_in_the_population` should be widened did not: its module
keeps `doc/adr/` out because its `§` rule reaches every file it reads, so the Status check is a
population of its own, bounded by number (ADR 2260 section 1). The hypothesis of two tests under a
hundred lines each held at two tests, each about 250 lines with its calibration; nineteen Status
lines was twenty-one, because 2188 and 2189 wrote "Accepted:" and "Deferred:" with no `Status:`.

**What it built.** `tools/conformance/tests/adr_status.rs`: from ADR 2242 on, the opening carries
`Status:` with one of seven states, and "pending", "under validation" and a `proposed` naming no `Q`
are refused; 2178–2241 print beside it (2215, 2220, 2221, 2237 `assessment`, 2241 "diagnostic").
`tools/conformance/tests/evidence.rs`: records and reviews from session 1712 and ADRs from 2258
name a path under `/home/AI/` or `scratchpad/` only in a sentence saying the session "worked in" a
directory. Each has a planted calibration of every verdict (trap 13), and each refused a file
planted in the tree and removed. The session worked in `scratchpad/r1713/`, which holds the two
editing scripts and nothing a sentence here rests on.

**What it wrote.** Twenty-one Status lines in ADRs 2188–2237, each the state at the merge that took
it, by commit, with nothing else in the record changed. The rule for an ADR's Status line, its
numbers and a gap, with review 1704's four gaps (2205, 2238, 2239, 2243) and whose each was, in
`doc/todo/02` section 6's ADR bullet. `doc/todo/75` item 3's figures, gate verdicts and
`/home/AI/` path replaced by the two commands that print them.

**Left.** ADRs 2209 and 2233 keep their "accepted" lines, though ADR 2242 decision 2 overrules the
red-gate acceptance in each; ADR 0265 says the line is where that goes, and neither was this slot's.
`doc/todo/75`'s figures from before review 1704's span (the witness's command counts, 9 474
commands, 28 684 dots) are counted facts too and stay for the round that owns the item. Any record
of this batch that names HEAD's arms directory as its comparison is red under `--test evidence`.

**Gates.** `cargo test -p conformance --no-fail-fast`: in the worktree 460 passed, 3 failed, all
three a sibling's mid-edit (`conformance.rs` 2 on `pdf-transform` and `pdf-model` comments,
`names.rs` 1 on `viewer-ffi`); in an export of this round's 27 files over HEAD (habit 138) those
pass and 8 fail that read `git ls-files`, which an export under `scratchpad/` cannot answer, and
pass in the worktree. `--test adr_status` 2/2 and `--test evidence` 2/2 in both. `cargo fmt
--check -p conformance` exit 0; `RUSTFLAGS="-D warnings" cargo clippy -p conformance --all-targets`
exit 0; `rustfmt --check --edition 2024` on both new files exit 0. Duration 1 600 s.
