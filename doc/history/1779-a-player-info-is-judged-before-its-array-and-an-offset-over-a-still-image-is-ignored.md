# 1779 — a player info is judged before its array, and an offset over a still image is ignored

Session 1779, slot 1 of batch 118, the ledger slot (ADR 2370); ADR 2392; ADR 2393 unused; no
question. I worked in the shared worktree and in my scratch directory, `scratchpad/r1779/`.

**Rows.** From `partial` to `implemented`: §13.2.7.2.1 ("If a non-viable media player info
dictionary is referenced by MU , NU , or A , it shall be treated as if it were not present in its
original array"), §13.2.7.3 (an `MH` entry where "Currently, there are no defined entries for this
dictionary"), §13.2.7.4.1 (a partially recognised software identifier, set aside by §13.2.7.2.1's
"same rule"), §13.2.7.4.4 ("If a version array contains negative numbers, it is non-viable, as is
the enclosing software identifier"), and the headings §13.2.7.4, §13.2.7.2 and §13.2.7. §13.2.4.3
stays `partial`, its note rewritten: "For media types where an offset makes no sense (such as JPEG
images), B and E shall be ignored, with no effect on viability" is now met; owed are the two timeline
rules, which need the clip's container, and a must-honour `/B` at the start, which needs §13.2.6.2's
value. `doc/todo/65` edited to match: the bucket line and the aggregate list.

**Premise and hypothesis.** ADR 2376's two verdicts held as stated, both probed by fixture before the
fix. The hypothesis held: one viability function and a reordered section; four leaves and three
headings. The drive's rendition fixture holds neither construction (no section, no `/PL`), and
`viewer-core`'s renditions test passes unchanged. Not built: a corpus census of the two shapes.

**Evidence.** Five tests in `media_viability.rs`, one per subclause; with `timeless` and
`player_info_set_aside` planted back to the old verdict, all five fail and the other eight pass.

**Gates.** In my export of HEAD plus my two files: `cargo fmt --check -p pdf-model` exit 0;
`RUSTFLAGS="-D warnings" cargo clippy -p pdf-model --all-targets` exit 0; `cargo nextest run -p
pdf-model --no-fail-fast` 2220 passed; `cargo test -p viewer-core --test renditions` 5 passed. In the
worktree: `rustfmt --check` on my two files exit 0; `cargo test -p pdf-model --test media_viability`
13 passed; `cargo test --profile gates -p pdf-model --test actions -- --ignored --nocapture` 2 passed;
`--bin inapplicable`, `--bin cited`, `--bin quotations` exit 0, no divergence in §13.2;
`tools/state.sh --round 1779 ledger` exit 0; `cargo test -p conformance --no-fail-fast` exit
101, 68 binaries passing and one red, none mine: four test names under §12.5.6.16, §12.6.4.9, §13.3
and §13.7.2.3.2, slot 4's rows mid-edit. The six arms are owed nothing: no page-rendering path calls `media::judge`.
