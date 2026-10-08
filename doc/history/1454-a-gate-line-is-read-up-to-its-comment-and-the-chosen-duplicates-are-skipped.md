# 1454 — A gate line is read up to its comment, and the chosen duplicates are skipped

Slot 5 of batch seventy-one, 2026-10-08, an instruments round. ADRs 1744 and 1745; Q350 not used.
No ledger row moved. **Premises.** Records 1444–1449 are 36–40 lines, each with `**Gates.**`, so
none needed an edit. "38 duplicate warnings" held: the zune pair is 2 of them. "`batch.rs`'s twin"
did not hold. `batch.rs` already stops at a `#`. The third whole-line reader is `ratchets.rs`, which
is `state_sections.rs`'s twin by its own comment, and `bounded.rs`'s `state_walks` is a fourth.

**Read up to the comment (ADR 1744).** The population is the seven readers that
`grep '"-p"\|"--test"'` names in `tools/conformance/tests/`. In four of them, `command_words` now
stops at the first word that begins with `#`, which is the shell's own rule. Each of the four has a
planted line whose comment names another package and another target. Run before the fix, all four
failed by name (`pdf-sandbox, --test oracle`; `e --test clocked`, plus a `-p conformance` line taken
for a walk). After the fix, all four pass.

**The bare `flock`.** `doc/rfc/0008` line 247 is now the wrapper: `--lock --round <session> --data 8
--tree 12` with the census built by `--build`. `HELD_BARE_LOCK_INSTRUCTIONS` is `[&str; 0]`. The
sweep read 2 015 files and found 0 held and 0 owed.

**`deny.toml` (ADR 1745).** There is now a `[bans] skip` for the reference renderer's five
crates.io copies, each by exact version: `zune-jpeg` 0.5.15, `zune-core` 0.5.1, `hayro-jbig2` 0.3.0,
`hayro-jpeg2000` 0.3.5 and `hayro-ccitt` 0.3.0. The three codecs are ADR 1714's same decision. The
duplicate warnings go from 38 to 33. A planted skip for a version not in the lock was reported as
`unmatched-skip`. The same run reported `unmatched-source` for `allow-git`'s `close2/quorra`, which
has been in the workspace under `raster/` since `eacf4951`. That line is gone, so 0 unmatched.

**`doc/todo/65`**, re-derived after slot 3's record: 19 `partial` and 4 `reported` rows, as at HEAD,
each named in its bucket. Slot 3's §7.4.9 sentence names Q348, and no edit is owed.

**Unfinished.** The habit "a comment on a gate line in `doc/todo/02` section 2 names no flag" now
describes a defect that is fixed. Retiring it is a habit edit, so it is proposed in the report.

**Gates.** `rustfmt --check --edition 2024` on the four test files: exit 0. `RUSTFLAGS="-D warnings"
cargo clippy -p conformance --all-targets`: exit 0. `cargo nextest run -p conformance`: 421 passed.
`cargo test -p conformance`: exit 0, then 101 at the end only on slot 1's
in-flight record 1450 (its `**Gates.**` paragraph states no status yet). `cargo deny check`: exit 0, 33 duplicate warnings, 0 unmatched.
`bash -n` on the three scripts: exit 0. `tools/bounded.sh --self-test`: exit 0, 33 s.
`tools/batch.sh check`: exit 1 after 947 s, and only on siblings' in-flight files: the fuzz workspace
stopped on slot 1's unmatched `Property::Options`, and `cargo fmt --all --check` on `appearance.rs`.
