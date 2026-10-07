# 1417 — A form page gets no turn row, and the second lane's re-ask fires

Measure slot of batch sixty-five. ADRs 1670 and 1671; no question written.

**The field table on the per-command path** (ADR 1670). Callgrind by function on an export of HEAD,
a warm repaint as three `Query::Fields` less one, over two: `opt_demo.pdf` 397.7 k instructions,
`form::fields` 371.5 k, `field_table` 77.1 k, its omitted roots 1.6 k; `xfa_filled_imm1344e.pdf`
57.1 k, 54.0 k, 17.1 k, 0.5 k. Every table was one page's width. **The `Listed` width was never built**:
`displayed` asks it only for a field that states a format, and neither document has one. Where one
does, it costs a second `/Fields` walk, 75.5 k and 16.6 k. The open builds no table. The best of
twenty queries took 0.044 ms and 0.010 ms. A turn adds `form::delegated_widgets`, which is
`form::fields` again, where a toolkit host delegates widgets. **No `turn_path` row**: that harness
draws widgets and has no `viewer-core`, so it cannot see the table. Its children turned both pages in
1.96 to 2.27 ms, under every row, beside p101 at 7.13 to 7.33 and `issue19802.pdf` at 6.09 to 6.11.
`turn-path.toml` says so in three comment lines.

**Batch sixty-four's lock** (ADR 1671). The five rounds queued 16 593.1 s and the merge 0 s. 1408
queued 5 742.9 s, 1411 5 098.1 s, 1412 2 626.6 s, 1409 2 091.0 s and 1410 1 034.5 s, mostly behind
the drive's two holds and the pixels round's arms. The `sccache` hold was 1 115.3 s, 19:18:39 to
19:37:14, with two rounds waiting on it. A FIFO replay reproduces all 24 observed waits to within 1 s.
On it, without the `sccache` hold: 14 363.3 s. Counts first and clock runs last: +313.5 to
+909.9 s, because the clock runs were the short holds. Without the export at open: −2 842.7 s.
Shortest hold first: −4 275.4 s. Two lanes: −6 384.7 to −8 919.6 s. ADR 1659's re-ask sum is
10 759 s against its 3 600 s, so the lane is worth building. It is owed to `tools/bounded.sh`.

**Premise.** Held: the three sites, and `turn-path.toml` naming neither file. Not held: "a page turn"
on these two. Both documents have one page, so the turn measured is `turn_path`'s, onto the page from
another document.

**Proposed habit.** Read what a gate's harness contains before proposing a row for a cost: this
harness interprets without `viewer-core`, and no row of it could see a field table.

**Gates.** `cargo test -p conformance`: exit 0, 403 passed. Behind the lock (queued 1 189.6, 750.5
and 303.9 s; held 407.7, 13.3 and 396.0 s), `pdf-sandbox --bins` first: exit 0; `launch_path
--release` counted: exit 0, 31 figures banded, 0 outside; `turn_path`: exit 0, 33 of 33 judged, 0
outside. No `.rs` file touched, so no rustfmt, clippy or nextest is owed.

**Unfinished.** The second lane (ADR 1671), for the next instruments round.
