# 1377 — A field's keystroke and format script run in an engine behind a feature

Scripts slot of batch fifty-nine, RFC 0008 section 11 item 3 (a). ADRs 1590 and 1591; no question.
The round was cut by the session's out-of-memory kill at 16:52 and resumed from the worktree.

**The engine (ADR 1590).** `crates/pdf-script`, Boa 0.22 behind `engine`, off by default; the crate
forbids `unsafe`, Boa's 604 (`grep`, seven crates) and `regress`'s 59 are the dependency's. Licence
first: all 57 new packages inside `deny.toml`'s list, recorded there; `cargo deny check` passes.
`Request` in, `Outcome` out (rc, value, change, ending, refusals, log), both with a byte codec for
the confined process that is the next robustness round's. Budgets: 100 ms wall and 40 M steps
between slices of Boa's budgeted evaluation, Boa's loop (100 000 per frame), recursion and stack
limits, and an element and string ceiling checked before the built-ins an argument makes large. A
runaway loop stops in 2.5 ms and `new Array(5e6).fill(0)` in 1.2 ms, each named, the process alive.

**The bridge and hook (ADR 1591).** `pdf_model::view::ScriptRunner`, supplied through
`ViewState::run_scripts_with`; none supplied is `off`, and no host supplies one. `/K` and `/F` reach
`event`, the field's value, `getField` of it, `console.println`, and the `AF*` library as natives over
`pdf_model::aform`; every other member of RFC sections 4.2 and 4.3 throws a `NotAllowedError` by name.
The drawn appearance does not yet ask the runner (named in the ADR).

**Tier 1 column.** `crates/pdf-script/tests/script_corpus.rs` (feature, ignored), on Tier 0's
population moved to `tests/support/script_population.rs`: 2 537 fields, 4 924 runs, 439 finished, 0
over budget, 4 485 threw — 4 393 `ReferenceError`s for functions document-level scripts define, 92
refusals (`util.printx` 55 first).

**The OOM audit.** My diff spawns no thread; Boa spawns none outside its tests; the walk's rayon pool
is 4. **The spawner was most likely `cargo-geiger` 0.13**, which I installed and ran unbounded in its
own target directory from about 16:42: re-run under `tools/bounded.sh --tree 12`, four jobs, it took
user AI's tasks from 188 to 7 845 and the tree to 13.23 GiB in 39 s and was killed by the bound. It
is uninstalled; the unsafe count is grep's. Threads of user AI around my first resumed test run:
82 before, 194 after (siblings resuming).

**Contract deviations.** `Cargo.lock` lists Boa's packages whatever the feature (Cargo locks
optional dependencies); nothing compiles them without it. The Tier 1 column is in `pdf-script`'s
tests, not `pdf-model`'s `script_corpus.rs`: naming the engine from `pdf-model` would make a cycle.

**Gates.** `rustfmt --check` 0; clippy `-D warnings` `pdf-script` with and without `engine`, and
`pdf-model`, 0; `cargo test -p pdf-script --features engine` 0 (28 tests), without 0 (6);
`cargo nextest run -p pdf-model` 0 (1 863); `cargo test -p conformance` 0; behind the lock: Tier 0
`script_corpus` 0 (347 held, 0 moved, 44 s), Tier 1 column 0 (36 s). All under `ulimit -u 8192`.
