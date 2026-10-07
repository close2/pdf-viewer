# 1406 — An import carries `/RV`, and the runs reach combs, bidi, carets, widths and tabs

Rich text slot of batch sixty-three. ADRs 1648 and 1649; no question written.

**The import** (ADR 1648). `FdfField::rich_value` reads Table 249's `/RV` as the text string it is,
`Import::rich_value` and `FieldValue::Imported`'s `rich` carry it, and `rich_text::for_field` draws it
by ADR 1635's rule; an FDF field stating no `/RV` leaves the target's standing. XFDF's
`<value-richtext>` (XFDF 3.0, pages 31 and 36) is the same entry: markup becomes `/RV`, and with no
`<value>` beside it the value is the string's characters. `read_field` and `xfdf` no longer name it.
**The runs** (ADR 1649). A placed line carries display order, edges, extent and byte span; the stream
writes each group at its edge and a host's caret, point, range and glyphs are read off the same edges,
offsets mapped onto the value by `Alignment`. Comb cells run by run; UAX #9's levels and the joining
shape over the whole string; Table 120's `/FontStretch` chooses a width's face; `tab-interval`,
`tab-stops`, `xfa-tab-stops` and `xfa-tab-count` with left, centre, right and decimal stops, tested on
the shapes of chapter 27's Examples 27.26 to 27.28. Reported, not drawn: a tab leader, a tab in a line
read right to left, an unmet width. **Premises.** The brief's line numbers held. "`RichTextOneStyle`
goes" held for three of its four reasons: a character none of the runs' faces draws still takes the
one-style machine face (ADR 1414) — a U+202E beside an italic run reaches it. ADR 1634's "no clause
names a face by width" was true of §9.6.2.2 and §9.6.3 and not of Table 120.
**Rows.** §12.7.8.3.2 `partial` → `implemented` (Table 249's last entry), and its headings §12.7.8.3 and
§12.7.8 `partial` → `implemented` by ADR 1035's rule; §12.7.4.3, §12.7.5.3, §12.5.6.6 stay `partial`,
notes restated on what is left (`kerning-mode:pair`, leaders, right-to-left tabs, links, `xfa:embed`,
algorithmic lists, the machine-face fallback); §12.7's note drops §12.7.8.3.2. §12.5.6.2 is untouched:
its popup runs are slot 2's build. `doc/todo/65` bucket 4 and its aggregate list, `doc/todo/22`,
`doc/state-of-play.md`. **Looked at** (trap 1): a page of one 24-point run, a rich comb, three tab
stops and an overridden run, rendered at 3× and 8×; each drew as the tests assert.
**Shared-tree notes.** Slot 2 added `rich_text::parts` to `rich_text.rs` and matches `Piece::Tab`
exhaustively in `popup/rich.rs`; the variant must merge with that file. `tools/bounded.sh --tree 12 --
<script>` ran the gate script twice in one call (log lines 2–268 and 270–533), both exit 0.

**Gates.** `rustfmt --check --edition 2024` on my 10 `.rs` files: exit 0. `RUSTFLAGS="-D warnings"
cargo clippy -p pdf-model --all-targets`: exit 0. `cargo nextest run -p pdf-model`: exit 0, 1955
passed; `--test rich_text` 42 passed, the two import tests failing with the `/RV` planted out.
`cargo test -p conformance --no-fail-fast`: exit 0, 397 passed, `the_frontier_map` among them.
Behind the lock (261 s waited), `pdf-sandbox --bins` built first: `raster_golden` exit 0, held 974,
moved 0; `pdf-model --test corpus` exit 0, every ratchet at its ceiling; `save_round_trip` exit 0, every
floor held.
