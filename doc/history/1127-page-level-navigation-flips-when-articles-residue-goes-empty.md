# 1127 — Page-level navigation flips when the articles residue turns out empty

Contract: §12.4.3 and §12.4 (aggregate under ADR 1035 §4). Of §12.4's children only §12.4.3
(articles) still owed — the rest `implemented` or `departed` — so §12.4 flips iff §12.4.3 settles.

## §12.4.3 → `implemented`, so §12.4 → `implemented`

Read against §12.4.3 and Table 162/163. The reader reads what the clause states — `/Threads`,
thread dicts, `/I`, beads walked `/F`→`/N`, the ring, page `/B` checked against it — and the
navigation §12.4.3 asks for is a permission ("may provide navigation facilities") taken by a panel
in all three hosts and the ABI. The two things the row was `partial` for are **not requirements
owed** (ADR 1035 §1):
1. `beads_on_page` is a capability no host calls; a permission granted is not a debt.
2. Table 162's `/Metadata` reading is **§14.3.2's** (`implemented`) — `Xmp::read` is general over
   every carrier and reads back every object `/Metadata` the corpus states — and no in-scope clause
   owes a *consumer* of an object-level packet (§14.3.1 interchange; §6.3.2.2 owes none of it).
   Empty residue carried elsewhere: the §8.6.4.4 → §10.4.2.5 shape. `/V` unread is the redundant
   back-link of a doubly-linked list, as §12.4.4.2's row leaves `/Prev`; nothing decided-against,
   so `implemented`, not `departed` (no ADR).

## A stale comment corrected (clause read beside code)

`article.rs`'s "What is not here" said the bead `/R` "is read, and nothing zooms" — false since
§12.3.2.1 went `implemented`: `interact.rs` composes §12.6.4.7's `ThreadJump` carrying `/R` as
`/FitR`, which `Open::apply_view` zooms and scrolls to. Text rewritten; no behaviour change.

## Measured (trap 8) / calibrated (trap 13)

Census over 1477 curated files (1450 opened): **113** state `/PageLabels`; `/S` seen are `D` 111,
`r` 17, `R` 1, `a` 1, and **150 no-`/S`** prefix-only ranges (158 with `/P`, 26 with `/St`≠1) — all
five styles plus no-style handled, `A` absent from the corpus but read. Added `page_labels.rs`
fixture: pages 0–3 `/S /r`, page 4+ `/S /D /P (A-) /St 1` ⇒ `i ii iii iv A-1 A-2 A-3`, the
range-boundary restart at the default `/St`.

## Gates

fmt (my files clean; `fmt --all` differs only on siblings' examples); clippy `-p pdf-model --lib
--test page_labels` 0; nextest `--test page_labels --test articles` 5 passed; `-p conformance` ok;
pdf-model doctests ok; `--test corpus` ratchets at ceiling, slack 0. `raster_golden` not run — no
rendering code changed (doc comment, test, ledger prose), so no pixel moves.
