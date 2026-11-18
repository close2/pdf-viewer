# 2977 — A table header scoped to the other axis is stepped over by the header search

Session 2075. Status: **implemented.** Context: review 2038 (finding B53), ADR 0312 (which adopted the algorithm).
Clause: ISO 32000-2 §14.8.4.8.3, Errata Collection 3 Issue #200.

ADR 0312 adopted §14.8.4.8.3's implicit header search. Issue #200 adds a rule to it: a header cell whose `/Scope`
conflicts with the direction of the search — a `Row` header met while searching up a column — neither terminates
the search nor is added to the list, irrespective of its `/Headers`. `TableStack::search` returned at any header
stating `/Headers` before it looked at the scope, so a conflicting cell ended the column search and the eligible
headers beyond it were lost. It now steps over a conflicting cell exactly as it steps over a gap, before the cell
counts as "a header cell" for the data-cell-after-header end. A cell of the search's own scope, or `Both`, is read
as before.
