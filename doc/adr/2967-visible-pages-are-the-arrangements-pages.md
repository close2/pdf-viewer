# 2967 — `VisiblePages` lists the groups of every page the arrangement shows

Session 2073. Status: **implemented.** Context: review 2038 (finding B51), ADR 0178 (superseded for this point), ADR
2814 section 3. Clause: ISO 32000-2 §8.11.4.3 (Table 99, `/ListMode`).

Table 99's `VisiblePages`: "Display only those groups in the Order array that are referenced by one or more visible
pages." ADR 0178 read the visible pages as the current page, which was the window's whole display when it showed
one page at a time. A two-page or continuous arrangement shows more, and Table 197's `/PV` already takes them as the
arrangement's pages (ADR 2814). `Query::Layers` now unions `groups_referenced_by` over every page placed on the
screen. Before anything is placed no page is known to be visible, and every group is listed, as before.
