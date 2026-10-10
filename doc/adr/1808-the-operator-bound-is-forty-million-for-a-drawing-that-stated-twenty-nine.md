# 1808 — The operator bound is forty million, for a drawing that stated twenty-nine

Asked by the project owner on 2026-10-09, built at the merge. Status: **superseded by ADR 1809**, which
makes the count a figure that refuses nothing; ADR 1810 is the memory bound that took its place.
Amends ADRs 0271 and 0306 in one figure. Context: ADRs 0241, 0271, 0306; `tests/hostile_budgets.rs`.
Code: `crates/pdf-model/src/content.rs` (`MAX_OPERATIONS`). Tests:
`crates/pdf-model/tests/hostile_budgets.rs` (`a_content_stream_longer_than_the_bound_is_refused_by_name`,
`a_marking_cycle_through_a_tiling_cell_stays_inside_the_operator_budget`).

## 1. The file

The owner opened `/tmp/demo_heavy_engineering_A1.pdf`, a 39.5 MB single-page PDF 1.3 drawing written
by `gen_demo_a1.py 2.0.0`, and `quorra` said: "the page reached this program's MAX_OPERATIONS bound,
so part of what the document asked for is not on it", after 16.23 s of user time. Counted from the
file (its one `FlateDecode` content stream inflated with Python's `zlib`, tokens split on white space,
operators the alphabetic tokens of one to three characters): **278 579 415 bytes, 78 173 320 tokens,
28 992 250 operators**, 2.70 tokens per operator. The bound stood at four million operators, so the
page was cut at about a seventh of its artwork.

## 2. The decision

`MAX_OPERATIONS` is **forty million**. The figure admits this drawing with headroom and is ten times
the old one.

What does not change is the argument the old figure rested on, which is why the raise is cheap:
the bound was already understood as a bound on *slowness*, not on exhaustion, and as one that no
count could make into a bound on time — one `sh` paints the whole page, so **a count is not a
cost**. What bounds the time a page may take is the confined worker's cancel (ADR 0241), and that
stands at any count. The census behind the old figure also stands as a measurement: over 926 680
crawled pages, eight state more than four million operators and every document that reached the
bound terminated when it was lifted a hundredfold. Those eight are admitted now too, or cut later.

## 3. The cost

A page that really states forty million operators runs ten times longer before the bound speaks
than it did; on this machine the owner's drawing spent 16 s to reach four million, so its whole
artwork is of the order of two minutes of one processor, and a window shows it when the worker
answers. The alternative — a per-document or per-user setting for the bound — is the shape
`CLAUDE.md` principle 3 gives a *document's* restrictions and is not taken here: the bound is this
program's guard against a runaway stream, not a restriction a file asserts, and one figure that
admits every legitimate page seen so far is simpler than a knob. The bound's own test grows with
it: the fixture that must exceed it is now forty million operators long.

## 4. What stays open

The eight crawled pages above four million have not been counted individually; whether forty
million admits all eight is for the content budget census to say when it next runs
(`cargo run --release -p pdf-model --example content_budget_census`).
