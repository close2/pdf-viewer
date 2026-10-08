# 1698 — A round's number in a comment is counted, and a state section walks behind the lock

Session 1431. Status: **accepted** and **built**. Builds the instrument ADR 1680 section 4 owed and
the rewrite it mapped; decides the question ADR 1684 section 3 left as a note. Supersedes nothing.
Context: ADRs 1023, 1637, 1680, 1684, 0798; traps 13, 25, 109, 130.
Code: `tools/conformance/tests/round_numbers.rs` (new), `tools/state.sh` (`walk`, `--round`, every
corpus section), `tools/conformance/tests/bounded.rs` (`merge_gates`, `joined_lines`, `state_walks`,
`every_walk_a_state_section_runs_is_locked_in_its_declared_lane`, the held list emptied),
`tools/conformance/tests/read_only.rs` (`first_command`), `doc/environment.md`'s rule line,
`crates/pdf-model/examples/substitution_census.rs`, and comment hunks in eighteen files.

## 1. The sweep

**The rule**: a comment line under `crates/`, `tools/`, `raster/` or `fuzz/` may not name a round as
the word `round` or `rounds`, either case, at a word's start, then whitespace, then three or four
digits that end a word. The whitespace may be a line break inside one comment, because two sites
split the word from the number across a wrapped line (`cache.rs`, `launch_path.rs`). The source is
**lexed, not grepped**: line and nested block comments are read, and every string literal (plain,
byte, C, raw with any number of `#`) and character literal is skipped, so a fixture's data is code —
`write_corpus.rs`'s title string, `bounded.rs`'s planted lock log (`round 103`) and
`spelled_ordinals.rs`'s own assertions are not hits by the rule rather than by a list. A lifetime is
told from a character literal by the closing quote only the literal has.

**Calibrated** (trap 13) twice. A planted source names a trailing comment, a capitalised doc
comment, a split pair, a block comment inside a nested one and a comment after a raw identifier, and
passes a plain, raw and byte string, a quote inside a character literal, `around 300`, two and five
digits, `rounding 400`, and a word and a number with code between them. On the tree, before any
rewrite, it named 59 lines in 21 files of 1 461 read — every site the brief's grep gave, the two
split ones the grep could not see, and the capitalised and plural ones (`Round 908`, `rounds 910 and
911`) it did not match. The held-count check fired too, on a guessed count of 22 against 17.

**The held list** holds a file another round owns while the sweep lands, by its count with `==`, as
ADR 1637 holds the todo files: `viewer-ui/tests/launch_path.rs` (17) and
`viewer-confined/tests/confined.rs` (1) are slot 2's crates this batch, `render-raster/examples/
image_phase.rs` (1) slot 3's. Each leaves when its owner rewrites it to zero.

## 2. The map the rewrite used

ADR 1680 section 4 mapped each round to an ADR; one entry was short. **Round 911 wrote two ADRs**:
0864 for what the kernel found in the FUSE face (`EPERM`, the owner, `ESTALE`, `touch`, the epoch,
the probe noise) and 0865 for what it found below it (the confinement's second image, the cache's
lengths and eviction, a worker kill) — so `pdf-vfs/tests/confined.rs`, which cited ADR 0864 for the
arena, now cites 0865. The others: 896 → 0836 (the font program it refused), 902 → 0847, 908 → 0858,
910 → 0862, 923 → 0886, 1371 → 1579 (its three-field fixture), 1405 → 1675. Rounds 1121 and 1145
wrote no ADR, so `named_appearances.rs` names the two readers instead. A sentence whose subject was
the round ("round 923 measured…") was rewritten to the present reason; a parenthesis became the ADR.

## 3. A state section may not walk unlocked

`doc/environment.md`'s first rule is one heavy walk on each lane, and `tools/state.sh` is run by
rounds as often as by a person. Nine of its walks ran under `tools/bounded.sh` without `--lock` and
twenty-six under nothing at all, so each took no lane and wrote no line. **Every walk now goes through
`walk <kind> -- <command>`**, which takes the lock (`--round` names the session, `-` without it) at
four threads, the merge's own figure, because a peak is the documents in flight (ADR 0798). A walk
inside a hold runs under it; a `clock` walk there is refused, and the cure is to run the section bare.

**The kinds, from the merge's lock lines of batch sixty-six** (peaks at four threads): `clock` for
every gate `clock_gates` names, `launch_path`, `frame_budget`, and the ratchets loop, whose derived
population holds clock gates; `small` for a walk that peaked at 3 GiB or less, half the second lane's
kill — `corpus` 1.58, the transform writers 1.20–2.02, the vfs walks 1.76–2.29, the census pair
2.13 and 1.57, `awkward_classes` 2.03, the archive three 0.78–1.20, the rest under 1.5; `large` for `raster_golden` 3.33,
`jpeg2000` 3.57, and the workspace's tests and doctests, which no line measured. The test holds every
`cargo test`, `nextest` or `run` under `--profile gates`, `--release`, a profile variable or
`--workspace` to a `walk`, and a gate the merge also runs to `clock` exactly where the merge's is.

## 4. What stays

- `gate_binaries` and each section's `--bins` build run outside the lock, so trap 109's rebuild
  inside the walk's hold is not what a state section does; the merge builds first and rounds rarely
  run corpus sections. Re-ask when a state section is the first to fail on a stale worker.
- A small walk is killed at 6 GiB with any build inside it. A kind is re-taken from the merge's next
  lines when a walk's peak passes 3 GiB (`tools/state.sh gates-cost`).
- `substitution_census` declares `--tree 12`, a large walk, for want of a measured peak.
