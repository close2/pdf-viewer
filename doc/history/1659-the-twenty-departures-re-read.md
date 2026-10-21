# 1659 — The twenty departures re-read, and two silent cuts beside them said

Session 1659, slot 5 of batch one hundred and two, a clause round; 2026-10-21. ADR 2154 (2155 unused, no
Q). Every `departed` row, in `git blame` order of its note, read against `doc/md/` and the code it names.
The hypothesis (fourteen hold, four rewritten, two defects) did not hold: five held and fifteen were rewritten.

| row | verdict | one line |
|---|---|---|
| §11.5.3, §11.6.6 | rewritten | the press bound holds; `Plane::*`, `soft_mask.rs`, `group_own_space` are the names now |
| §11.7.5.3 | rewritten | holds; "nine" presses is seven, four paths moved; open: the report may fire where an ICCBased `/CS` carries it out |
| §12.2 | rewritten, defect named | holds; four windows: WinUI 3 logs `QUORRA_OWED_CHROME` while the shared sentence says `/HideToolbar` is obeyed |
| §12.5.6.23 | rewritten | the two refusals hold; seven sentences told their own history (ADR 1023) |
| §12.6.4.6 | rewritten | holds as ADR 1358's decision; the reason named the sandbox, which confines the renderer and not the host |
| §12.7.8.3.3 | rewritten, defect fixed | holds; both stated reasons had expired; a save dropped template pages silently, now `Written::unwritten_pages` |
| §7.4.2 | holds | the stray byte is skipped and pinned; `filter.rs`'s comment called the row `partial` (fixed) |
| §7.5.5 | holds | Table 15's `/Size` sentence still unapplied; no test pins the departure (named) |
| §8.5.3.3.1 | rewritten | holds; not "decided in one place": `raster-gpu`'s `collapsed_marks` mirrors it, and `code` names it |
| §8.7.4.5.7 | rewritten | holds; `shading::Cache` is `pdf-colour`'s; open: the note names no `shall` under it (ADR 1622 §1) |
| §8.7.4.5.8 | holds | stands on §8.7.4.5.7's reasons; Table 85's interior order matches `control_grid` |
| §7.4.8 | holds | Table 13's default yields only to the Adobe marker; `issue11931.pdf` is the one page |
| §10.7.5 | rewritten | holds; quorra applies the width rule a second time, and that copy's `adjust` branch has no test (named) |
| §12.10.2 | rewritten | holds (A271); a retired refusal's name and a repeated sentence went |
| §12.5.2 | holds | Table 166's writer `shall` is declined only where `Written::unappeared` names it |
| §11.7.2 | rewritten | holds; one profile is at most seven presses, not nine; `Press::table` |
| §12.7.4.1 | rewritten, defect fixed | holds; `merge::walk_field` stopped at 64 in silence, now 256 with a `Warning` |
| §12.5.5 | rewritten | holds; the note said both that the standard orders annotations and that it does not; two comments too |
| §10.7.4 | rewritten | five departures hold; nine stale sentences outside the window part (count, A538, budgets, strips, lane) |

**Fixed.** The merge's field walk names every hierarchy past 256 links (a field there had no §12.7.4.2
test and kept a signature's `/V`); its test fails with the warning removed. A save counts the template
pages it does not write, and `viewer-core` reports them (ADR 2154). Comments: `pages.rs`, `form.rs`, `writing.rs`.

**Named, with witnesses.** §10.7.4's window part (slot 2's): "2 levels at most" against ADR 2133's 0,
`page_past_the_tile.rs:188`'s "to a level", "58 … where it moved 181". `interact.rs:1009`, `policy.rs:580`
call every non-PDF launch an application the sandbox withholds. `press.rs:533,552` read a poisoned lock as a spent budget.

**Gates.** `cargo test -p conformance --no-fail-fast`: 101 on `round_numbers` alone, two siblings' scratch
examples, as before my edits; every ledger gate passes. `--bin quotations`: 0, ledger diverging 3 → 3.
fmt 0; clippy `-D warnings` 0 on pdf-syntax, pdf-model, viewer-core, pdf-transform `--lib --tests`; nextest pdf-syntax 294/294,
pdf-model 2 122/2 122, viewer-core 403/403, pdf-transform 609/611 (slot 1's two); `state.sh docs` 0; no pixel moves, no arms; 2361 s.
