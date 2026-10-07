# 1412 — Every list type, a leader, a tab read right to left, and a save writes an import

Rich text's third round, slot 5 of batch sixty-four. ADRs 1660 and 1661; `doc/questions/Q308`.

**Built** (ADR 1660). Every `list-style-type` *List Support* requires, by its [CSS3-Lists]
algorithm (the 2011 and 2002 W3C drafts, fetched, not held); chapter 2's tab leaders, dots, rules
and content on the margin's grid; a tab in a paragraph read right to left moving to the stop on the
left, each tab's group in its own display order, `after` and `before` read by direction; a list tag
in its own display order and written a face at a time; a width no `/DR` face states set in the
nearest one; a character no chosen face draws set in a machine face beside the runs, one face per
script set (379 ms against 130 ms on the four-field fixture, the catalogue read per set).
**Choices** under ADR 1622's rule, because §12.7.4.3 delegates "formatting information": following
an `<a href>` (report withdrawn) and resolving `xfa:embed` (still said). **The save** (ADR 1661):
an import's `/V`, `/RV`, `/Ff` and `/F` and the appearance drawn from them reach §7.5.6's update.
**Premises.** The slot's list held but for its pointer: `doc/history/1406` names no unwritten
import — ADR 1648 section 4 does. "Link following is left to the processor" held only through
§12.7.4.3's delegation: chapter 27 itself describes the click. A `/DR` face that falls short is now
said beside the runs instead of sending the string to one style.
**Rows.** §12.7.4.3, §12.7.5.3, §12.5.6.6 stay `partial`, on `kerning-mode:pair` alone: no pair
data is held (Q308 asks for Adobe's Core 14 AFM pairs). Notes restated; `doc/todo/65` bucket 4,
`doc/todo/22`, `doc/state-of-play.md`. **Looked at** (trap 1): leaders, Roman, Hebrew and Japanese
tags, a right-to-left tab line and a mixed-face run at 3×; a tag first drew its full stop as a box
in the machine face, which the per-face writing fixed.
**Shared-tree notes.** `rustfmt` ran on `view.rs`, whose ADR 1653 hunks are another slot's.

**Gates.** `rustfmt --check --edition 2024` on my 7 `.rs` files: exit 0. `RUSTFLAGS="-D warnings" cargo
clippy -p pdf-model --lib --tests --benches`: exit 0 (`--all-targets` stops at round 1408's example
`r1408_field_table_cost`, not mine). `cargo nextest run -p pdf-model`: exit 0, 1965 passed; each new
test failed with its build planted out. `cargo test -p conformance --no-fail-fast`: exit 0, 400 passed, `the_frontier_map` among them.
Behind the lock (2628 s waited), `pdf-sandbox --bins` built first: `raster_golden` exit 0, held 974,
moved 0; `pdf-model --test corpus` exit 0, every ratchet at its ceiling; `save_round_trip` exit 0,
every floor held.
