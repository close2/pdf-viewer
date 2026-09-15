# 1069 — One cause on every row, and two figures that never moved

The measurement slot, answering ADR 1081's third Consequence: 1067 found three `open_kinstructions`
figures 10–15% from the **high** edge of bands whose spread is 0.011% and moved nothing, correctly
— a band moves on a measurement. This is it (ADR 1083). **Two arms, each its own checkout and build
directory**: `d9fd1906`, the batch carrying session 1027, and `e168f02c`, where 1067 printed —
counted under callgrind on the gate's own `count-open` child, environment cleared. Thousands of
instructions, last column fixed:

| document | at 1027 | at 1067 | Δ | of which `xref::read` | fixed |
|---|---|---|---|---|---|
| `PDF20_AN001-BPC.pdf` | 4942.7 | 4955.1 | +12.4 | +12.243 | 4943.6 |
| `Well-Tagged-PDF-WTPDF-1.0.pdf` | 26665.3 | 26677.8 | +12.5 | +12.243 | 26665.8 |
| `ISO_32000-2_sponsored_EC3.pdf` | 185009.6 | 185019.6 | +10.0 | +12.242 | 185014.9 |
| `bug1815476.pdf` | 3394.7 | 3408.5 | +13.8 | +12.245 | 3396.6 |
| `xfa_filled_imm1344e.pdf` | 1801.4 | 1814.6 | +13.2 | +12.243 | 1802.0 |

**One cause, the same number on every row, and it is (b).** Session 1052's ADR 1066 made §7.5.2's
header the *earlier* of `%PDF-` and `%FDF-` — right, and implemented as two searches of the
kilobyte and a `min`, the second running to the end for a marker that is not there. "The earlier of
the two markers" **is** the first window that is either of them, so `header_position` asks each
window once and stops at the first hit; same answer in every case, and
`cross_references.rs::the_header_is_whichever_marker_stands_first_in_the_file` holds both orders
untouched. `position_of` had these two callers and is gone. Nothing is *deferred* — nothing here
can be, every offset depends on the header; what came off answered nothing.

**Every other candidate was exonerated by the profile, not by reading**: the signed row's open has
no `pdf_signature`, `notes::about`, `submission` or trust in it at all — ADR 1044's deferral holds,
`notes::losses` costs 108, and `Command::Trust` never reaches this path.

**And 1067's alarm had two causes wearing one shape.** `Well-Tagged` and `ISO_32000-2` read 26665.3
and 185009.6 at the **1027** arm — 17.9% and 11.3% from their high edges before a commit of this
window landed; 1027 measured them at 1.6% and 1.8% above their floors and left both ends alone.
Their bands are session 938's, and the 0.64% and 0.78% that put them off-centre is growth no round
attributed and this one cannot. So **no band moves**: one re-centres with the change that needs it
— ADR 1044 read forwards. The signed row returns from 14.6% of its high edge to 48.6%; the gate
reads 26 figures banded, 0 outside.

**Shared worktree**: the gate would not build on a neighbour's mid-edit `viewer-core`, so both arms
went in scratch checkouts with build directories of their own, never the shared one (`doc/verify.md`).
