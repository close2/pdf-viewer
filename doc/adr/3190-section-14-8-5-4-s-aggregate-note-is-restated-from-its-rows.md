# 3190 — §14.8.5.4's aggregate note is restated from its rows

Session 2122. Status: accepted. Rows: §14.8.5.4 (`implemented`, note restated); §14.8.5.4.2
(`implemented`, one stale sentence restated). Context: ADRs 1599, 2384, 3177; `doc/questions/A634`.

## Decision

ADR 1599 makes an aggregate row's note a summary of its subrows. §14.8.5.4's note still said "nothing
in it exports" after §14.8.5.4.1 became `implemented` (ADR 3177) and §14.8.5.4.4 crossed to the
accessibility tree (ADR 2384), and §14.8.5.4.2's said "exports to no format". Both now say what §14.8.5.4.1's
NOTE 1 names — repurposing and export — and that this program does the second and not the first: the
accessibility tree is an export, and the reflowing view A634 keeps out is a layout engine. The
aggregate lists what crosses (writing mode, `/BBox`, `/TextAlign`, Table 380's decoration and
position, §14.8.5.4.5's rectangles) and points at each row for what does not.
