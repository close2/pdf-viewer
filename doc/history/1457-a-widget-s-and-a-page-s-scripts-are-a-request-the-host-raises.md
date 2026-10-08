# 1457 — A widget's and a page's scripts are a request the host raises

Slot 2 of batch seventy-two, 2026-10-08, a script round. ADRs 1750, 1751; no row moved, no question.

**The shape, for slot 3 (written first; slot 3 had already built against it).** The requests are
the two methods that existed, unrenamed: `ViewState::run_annotation_scripts(&document, annotation,
action::Trigger) -> usize` for a widget's `/E` `/X` `/D` `/U` `/Fo` `/Bl`, and
`run_page_scripts(&document, page_index, PageTrigger) -> usize` for a page turn — `Close` runs each
annotation's `/PC` **and `/PI`**, then the page's `/C`; `Open` runs `/O`, then each annotation's `/PO`
**and `/PV`** — so a host raises `PageVisible`/`PageInvisible` scripts through `run_page_scripts`,
never per annotation. Each answers the scripts handed to the runner (0 at `off`). Before
`run_open_scripts` has run nothing runs: an open is the sequence's, any other request says it was
not run. The action path's `JavaScript` refusal stays.

**Premise.** Held: the grep found only definitions and tests (slot 3's callers arrived while this
was written), `action.rs` refuses `JavaScript`. Did not hold in two names: the page table is Table
198, not 196; the destination syntax is Table 149, not 151 (ADR 1736 had it so too).

**Built (ADR 1750).** The two requests as above; Table 197's visibility pair rides with the page's
event, in the open sequence too; nothing before the open sequence; a chain with no script costs no
field-tree walk. The worker column raises every annotation's six events and turns 64 pages.
**Built (ADR 1751).** The destination's page and Table 149 view cross together; wire version 12;
one named arm in `viewer-core`'s `carry_out_view_requests` (pending view, then the turn), its two
tests in `script_view.rs`. Unfinished: nothing; a continuous layout's neighbours' `/PV` is no window's.

**For slot 3 and slot 6.** Slot 3: `PageVisible | PageInvisible => runner` as `PageOpen | PageClose`
are, and a test host must send `Command::Presented` before a pointer script runs. Slot 6: wire 12,
`script_wire` re-seeded (32 seeds, two `Destination` edits new in `wire_seeds`).

**Gates.** rustfmt `--check` on my files and hunk: clean. `RUSTFLAGS="-D warnings" cargo clippy
--all-targets`: `pdf-model`, `pdf-script` (with and without `engine`), `pdf-script-worker` exit 0;
`viewer-core` fails only in slot 3's `tests/script_triggers.rs`. `cargo nextest run`: `pdf-model`
2 005, `pdf-script` 139 (`engine`) and 22, the worker 26 (a deadline failure at load 20 passes
alone and in a rerun at load 9), `viewer-core` 366, 0 failed. `cargo check --workspace
--all-targets` and `--manifest-path fuzz/Cargo.toml`: exit 0. `cargo test -p conformance`: 397
passed, 1 failed, slot 5's `bounded.rs` long-hold case. Behind the lock, `--tree 6`: the Tier 1
column exit 0, unchanged (21 101 runs, 11 778 finished, 9 315 threw, 8 unparsed; 286 s, 1.35 GiB);
the worker column exit 0 (42 792 runs through 150 workers, 4 190 at the raised sites, 0 lost, 0
`SIGSYS`; 349 s); `pdf-model --test script_corpus` exit 0 (356 held, 0 moved); `pdf-model --test
corpus` exit 0; the `script_wire` re-seed exit 0. Duration 2 400 s.
