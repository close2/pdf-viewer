# 1463 — The tail of Tier 1 bridged, and the runners answer whether a page changed

Slot 2 of batch seventy-three, 2026-10-08, a script round. ADR 1762; no ADR 1763, no row moved
(§12.6.4.17 stays `out-of-scope` until Q286), no question.

**For slot 3 (the shape).** `run_annotation_scripts` and `run_page_scripts` answer
`view::ScriptsRan { handed, changed }`; `changed` is `ScriptEdit::redraws`, a value written over a
different one, or a `/CO` walk. `viewer-core`'s three comparisons now read `.handed` (named hunks in
`interact.rs` and `viewer.rs`, behaviour unchanged); re-interpreting on `.changed` is slot 3's. Two
more host calls exist for the windows: `ViewState::set_keys(Keys)` and `take_console_requests()`.

**Premise.** Held: the grep printed 21; the census (a patch applied and reversed, 72 s) ranks them
as ADR 1724 section 4 did. Did not hold twice: RFC 0008 section 4.2 has no `OCG` row, so it admits
`getIntent` nowhere — and no reason to refuse a read of Table 96's `/Intent` holds, so it is bridged
rather than refused by name; and section 4.2 admits `richValue`/`richChange` read-only, not "written
as a committed rich value". **Hypothesis** held: only the word pair rests on an open reading, and
the word rule (the readback cut at white space, `bStrip` trimming non-alphanumerics) is a choice.

**Built (ADR 1762).** The keys from the host; the typographic three from `/BS /W`, `/Border` and
the `/DA`'s `Tf`, written as one `/DA` and the widget's `/BS` (a `/DR` font by key or `/BaseFont`,
others reported) and the `font` constants; `console`'s three as `ScriptEdit::Console`; the word pair
from every page's words, read once when a handed script spells them (1 024 pages or 2 s); the rich
pair from `/RV` through a new public `pdf_model::span`; `util`'s seven in `engine/util.rs` (a forked
helper of this round wrote them and the span API); `getIntent`. `NOT_BRIDGED` is empty. Wire 13;
`script_wire` re-seeded (32 seeds).

**Unfinished.** The field's `/DS` is not applied to `richValue`'s spans (12 pt root); a `textFont`
naming a font the `/DR` lacks is not added to `/DR`; a word name built at run time is not read.

**Gates.** rustfmt `--check` on every file of mine: clean. `RUSTFLAGS="-D warnings" cargo clippy`:
`pdf-model` (`--lib --tests`), `pdf-script` (all features, all targets), `pdf-script-worker`
(`engine`), `viewer-core` exit 0. `cargo check --workspace --all-targets` exit 0. `cargo nextest
run`: `pdf-script` (`engine`) 165 passed; `pdf-model`, the worker and `viewer-core` 2 282 of 2 283,
the one a worker deadline test at load 40 that passes alone, and the worker's 26 three times at
load 12 (round 1457 saw the same). `cargo test -p conformance`: 326 passed. Behind the lock, `--tree
6`: the Tier 1 column exit 0, as HEAD (21 101 runs, 11 778 finished, 9 315 threw, 8 unparsed; 0 of 0
not-bridged members reached; 281 s, 1.48 GiB); the worker column exit 0 (43 137 runs, 150 workers,
4 190 at the raised sites, 0 lost, 0 `SIGSYS`; 600 s); `pdf-model --test script_corpus` exit 0 (356
held, 0 moved); `pdf-model --test corpus` exit 0; fuzz check exit 101 (`script.rs`'s `Layer`), fixed,
then 0. Duration 3 400 s.
