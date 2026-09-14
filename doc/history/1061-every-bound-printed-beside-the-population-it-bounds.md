# 1061 — Every bound printed beside the population it bounds, and one ratchet with a pair of slack

The instruments slot. Two rounds found a ceiling clear of its population — `MAX_INCOMPLETE` at 91
against 61 (1036), `MAX_PAGELESS` at 6 against 5 (1054) — invisible because the run prints the
population and the constant does not. 1054 then checked that file's other four **by hand**, and a
check done by hand is done once. ADR 1075.

**`crates/gate-ratchet`**, four functions, no dependencies. Each prints `ratchet: <what>:
<population>, ceiling <bound>, slack <slack>`, then asserts the bound holds — the gate's own
assertion, unweakened — and then that the slack is within what the comment allows. `ceiling`/`floor`
allow none; the `_with_headroom` pair take an argued distance and print it. Seven of its own tests
plant each defect back and confirm it is named (trap 13).

**`tools/conformance/tests/ratchets.rs`**, tier 1, so it sees a bound added beside a walk nobody ran.
A `MAX_…`/`MIN_…`/`…_FLOOR`/`…_CEILING` integer const may appear in code only inside a
`gate_ratchet` call, and no gate file writes its own `floor`/`ceiling` (two did; one held twenty-eight
bounds). `// not a ratchet:` admits a parameter, checked both ways; what it cannot see — a bound named
nothing in particular — is in its module comment. **Its population is `doc/todo/02` §2's fenced
blocks, and that paid at once**: the brief named nine gate files, §2 names 28, and three carried
bounds nobody had listed — `dates` (`MIN_CONFORMING`, `MAX_NON_CONFORMING`) and `xmp` (`MIN_PACKETS`,
`MAX_REFUSED`, `MIN_PROPERTIES`). Trap 25, on this round's own brief.

**48 bounds now print; 47 sat exactly on their populations** — corpus 5, dates 2, xmp 3,
text_extraction 2, save_round_trip 6, accessibility_census 30, all but one at slack 0. The exception
is `CROSS_AXIS_FLOOR`, **8562 against 8563**. Read against its own comment, which says the count may rise and a fall is written
down there: `JUDGED_FLOOR` is unchanged at 503 on the same run, so no document entered the judged set
and a matched word gained the Table 120 pair its page states — a rise nobody recorded, so one matched
pair could have been lost in silence. Now 8563, on two runs.

**What should become a named population and did not, here**: `MAX_LOCKED` (10), `MAX_PAGELESS` (5) and
`MAX_UNREADABLE_ENCRYPTION` (1) already name their members in their own comments and `tally.locked` is
a `Vec` of those names — but `held(…)` changes what a gate *asserts* rather than tightening a bound,
which this round was told not to do. `launch-path.toml`'s bands are two-sided and stay outside.

**Shared worktree, three siblings mid-edit.** `cargo fmt --all --check` (1) and `clippy --workspace`
(101) fail only in `pdf-font`, `pdf-signature`, `pdf-model/examples`; `cargo test -p conformance`
(101) on `doc/conformance/ledger.toml:4332`, a neighbour's `\uXXXX`. None is this round's. Green:
`ratchets` 2/2, `gate-ratchet` 7/7, `nextest -p gate-ratchet -p conformance -p pdf-model -p
viewer-core` 1892/1894 (both a sibling's `submission.rs`), `--doc` 0, `fuzz` fmt and clippy 0, and
every gate this round touched — `corpus`, `dates`, `xmp`, `save_round_trip`, `text_extraction`,
`accessibility_census`, `selection_census` — all exit 0.
