# 1670 — A form page gets no turn row, and the listed table is not built on its repaint

Session 1417. Status: **accepted**. Answers the question ADR 1653 section 4 left implicit: what
the field table's widths cost on the per-command path of a form document, and whether such a
document belongs in `doc/checks/turn-path.toml`. Supersedes nothing; nothing is built.
Context: ADRs 1577, 1604, 1653; `doc/habits/measuring.md` 41, 45 and 72.
Instruments: `viewer-core`'s `examples/fields_cost` and `examples/accessibility_cost` under
callgrind, and `turn_path`'s own child (`turn_probe`), all built `--release` from an export of
HEAD b9fa776a into a target directory of its own (trap 129).

## 1. The premise, and the part that did not hold

The three sites hold: `displayed` (`viewer.rs:2889`, under `Query::Fields`), `as_displayed` at
3245 and 4542 (under `Query::AccessibilityTree`). Both documents are **one page**, so neither has a
page turn inside it; the turn here is `turn_path`'s, a page turned to from another document, and
the per-turn cost of the field table is the repaint's plus what an interpretation adds.

## 2. Callgrind by function, the warm repaint as three queries less one, over two

| | `Query::Fields` | `form::fields` | `field_table` | of it, the omitted roots |
|---|---|---|---|---|
| `opt_demo.pdf`, 5 fields | 397.7 k | 371.5 k | 77.1 k | 1.6 k |
| `xfa_filled_imm1344e.pdf`, 1 field | 57.1 k | 54.0 k | 17.1 k | 0.5 k |

Instructions. Every `field_table` call came from `form::fields`, once a query: one page's width.
**The `Listed` width was never built.** `displayed` asks `ViewState::displayed_values` only for a
field that states a format, and neither document has one (`fields_cost`: "0 displayed through
/F"). Where a format is stated, `Listed` is a second walk of `/Fields`, the table less its omitted
roots: 75.5 k and 16.6 k. By the clock, the best of twenty queries took 0.044 ms and 0.010 ms, 0.5%
and 0.1% of a 120 Hz refresh. The open builds no field table at all. `Query::AccessibilityTree`
builds the same one-page table once a query (77 k, 17 k), 4.6% of its 1.69 M on `opt_demo.pdf`.

**A turn adds one more table where a toolkit host delegates widgets.** The interpretation asks
`form::delegated_widgets`, which is `form::fields` again: 371.5 k and 54.0 k. A host that draws
widget appearances itself pays nothing there.

## 3. The turn, in `turn_path`'s quantity

Three interleaved sets of `turn_probe` children, pinned to `0-3,12-15` with eight threads, load 5.4
to 6.1, two banded rows beside them as controls. Sets two and three, whose calibration figures sat
inside 0.62 to 0.78 ms (set one read 0.83 to 0.87 on three of its four, which the gate would not judge):

| page | turn | interp | encode | step |
|---|---|---|---|---|
| `opt_demo.pdf` p1, 463 commands | 1.96 to 2.08 ms | 0.51 to 0.52 | 0.54 to 0.58 | 3.13 to 3.17 |
| `xfa_filled_imm1344e.pdf` p1, 574 commands | 2.17 to 2.27 ms | 0.54 to 0.57 | 0.80 | 2.49 to 2.64 |
| ISO 32000-2 p101, banded 6.1 to 10.8 | 7.13 to 7.33 ms | 1.59 to 1.70 | 4.02 to 4.10 | 5.32 to 5.51 |
| `issue19802.pdf` p1, banded 5.2 to 8.1 | 6.09 to 6.11 ms | 0.57 to 0.59 | 4.22 to 4.23 | 4.71 to 4.75 |

## 4. Decision

**No `turn_path` row for a form page.** The gate's harness interprets with
`ViewState::of(document)`, whose widgets are drawn, and has no `viewer-core` in it. So a row could not
see the field table in either width, which is the cost the question was about. What it would see is
a turn cheaper than any row this file holds, about a quarter of a refresh. That page is text and appearance
streams, the same stages the p101 and `issue19802.pdf` rows already hold.

**The field table's cost is a repaint's, and is left ungated.** It is 77 k instructions at most on
these two documents. The open builds none of it, and ADR 1653's EC3 memory band already watches
the one width that could reach the launch path. The price ADR 1604 measured on 116 fields was 0.80 ms. A form document's turn
becomes a row when that changes: when a `Query::Fields` repaint on a gate document passes 0.5 ms,
or when a turn gate first runs through `viewer-core`.
