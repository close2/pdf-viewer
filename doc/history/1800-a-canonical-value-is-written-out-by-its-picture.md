# 1800 — XFA's fourth round: a canonical value written out by its picture

Session 1800, a clause round in batch 120; 2026-10-27. ADR 2434; ADR 2435 and Q666 unused.

**Contract.** Slot 3, at A633's pace: the output formatter and the `validate` picture's format test,
so that `bind`- and `validate`-picture fields write their `datasets` value (RFC 0011 section 13).

**Premises.** *The formatter is a writer per category beside the parser* held. It is
`xfa::picture::format`, a child module that reads the parser's own tokens. *Both kinds leave
`xfa_unkept`* held, apart from a value the format test fails, which stays named with the level and
the reason. *The four hybrids are unchanged* held, since none carries a picture. Three readings went
past the brief. Pages 159 and 165 format with the first alternative only, while page 1159 and
EXAMPLE 26.12 try each in turn; chapter 26 is followed. Page 880 makes `warning` the default, and
page 162 still writes the data only once the test succeeds, so no override means no write. The
round trip found three parser defects: `8` read one fraction digit, `zero{9}` was refused, and a
zero clause matched any digit. All three are fixed (ADR 2434 §6).

**Built.** A writer per category for date, time, numeric (`z`, `Z`, `8`, `%`, `v`), text, null and
zero, with alternatives tried in order. Nothing is rounded: a value whose digit, sign or part the
picture cannot show is not written by it. `canonical` gained readers for dates, times and decimals.
`typed` canonicalises (by the `bind` picture where there is no edit picture, Rule 2), runs the
format test, and writes through `bind`, or canonical where it cannot; `formatTest` has three levels.

**Files.** New: `crates/pdf-model/src/xfa/picture/format.rs`, ADR 2434, this record. Edited:
`crates/pdf-model/src/xfa.rs` (its stale module comment), `xfa/{picture,canonical,datasets,template}.rs`,
`crates/pdf-model/tests/xfa_datasets.rs`, RFC 0011 section 13's step 3, ledger K and K.2's code
lists and K.2's note (`-U0`, no status move), `doc/state-of-play.md`. I worked in `scratchpad/r1800/`.

**Gates.** Tier 1: `rustfmt --check` on my seven files, exit 0. `cargo fmt --check -p pdf-model`,
exit 0. The worktree's clippy failed in a sibling's `pdf-syntax`, so it ran in my export (habit
138): clippy `-D warnings -p pdf-model --all-targets`, exit 0; `cargo nextest run -p pdf-model`,
exit 0, 2 265 passed; rustdoc `-D warnings -p pdf-model`, exit 0; `tools/state.sh --round 1800
docs`, exit 101, its reds in siblings' files only. `--test xfa_datasets`, 8 passed,
and both new tests fail with the bind picture and the format test planted out (trap 13), the plant
reversed by its own patch. Unit tests `xfa::`, 24 passed: chapter 26's output tables as printed,
and parse ∘ format = identity per category. In the export, against HEAD plus my paths: `save_round_trip`, exit 0, ratchet held;
`writer_corpus`, exit 0. `cargo test -p conformance --no-fail-fast`, exit 101, 467 passed, 2 red and not mine (records 1801, 1802, 1805).

**Owed.** Chapter 15's extended mapping rules are unread, including a `config` `transform`'s
default bind picture (page 527). Step 4's null test and its report to the person come next.
