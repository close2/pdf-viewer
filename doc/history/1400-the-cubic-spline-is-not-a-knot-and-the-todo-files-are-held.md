# 1400 — The cubic spline is not-a-knot, and the todo files are held

Instruments slot of batch sixty-two. ADRs 1636 and 1637; no question written.

**§7.10.2 `partial` → `implemented`, §7.10 with it.** "Valid values shall be 1 and 3, specifying
linear and cubic spline interpolation" names the kind and not the spline. The brief offered natural
against Catmull-Rom; the tree takes not-a-knot (ADR 1636). It is C2, and it needs exactly the four
samples of "If Size is less than 4, cubic spline interpolation is not possible". It reproduces a
cubic exactly, which a natural spline and Catmull-Rom both fail. The spline is a tensor product
whose B-spline coefficients are solved at parse time (`function/cubic_spline.rs`). A `/Size` under
four ignores the order for the whole function; an unlisted `/Order` is refused. **The brief's
premise was off once more**: the encoding mapped from [0 1] and not from the function's
`/Domain`, which EXAMPLE 2 pins, and that is fixed. Five fixtures: EXAMPLE 2, a cubic, a bicubic,
the end pieces as one cubic, and the ignore/refuse case.

**Census** (byte census, 126 393 type 0 functions): 170 state `/Order 3`, in 60 crawl documents;
none state another value; none are tracked. Nine page ones move, A/B on one binary, `/Order 3`
rewritten as `/Order 1` in place: `GHOSTSCRIPT-687352-0.pdf` (29%, by up to 27 levels),
`poppler-235-0.pdf` and `poppler-68861-0.pdf` (by up to 7), and six by one level. All are under
`corpus-cache/tika-issue-tracker/`. Callgrind on `poppler-235-0.pdf`: 531 M against 320 M, of
which evaluation is 245 M against 102 M. 585 functions state a non-unit domain, none tracked.

**Todo ordinals** (ADR 1637). All 65 spelled lines are rewritten, except `56`'s 2, which belong to
round 1395. The digit form went to 0 in the same sixteen files and in `02`, whose incidents keep
the incident without the number. `the_todo_files_name_no_round` holds both counts, at 2 and 119;
`<=` stays while six rounds share the tree, and becomes `==` at zero. `22` carries 0.

**Documents.** The nav docs were read against ADRs 1614–1625. ADRs 1616 and 1625 were already
stated in all three places. `crate-map` was stale on 1614–1616, 1618, 1624 and 1625; `PLAN` gains
1620, 1621 and 1624; `HANDOVER` and `state-of-play` gain ADR 1622's `shall`, and `state-of-play`
the spline. `doc/todo/65` and `01` drop §7.10.2 and §7.10. `65` was re-read at the end, the
ledger holding no other row of round 1399's yet, and the frontier gate passes. Records 1389–1394
are 34–40 lines, each with **Gates.** `gates-cost` names 35 of 35, and the last log begins with
`build-sandbox`'s line, as ADR 1625 intends; one gate there, `t3-accessibility`, exited 101.

**Gates.** `rustfmt --check --edition 2024` on the three `.rs` files: exit 0. Clippy `-D warnings
--all-targets`, `pdf-colour` and `conformance`: exit 0. `cargo nextest run -p pdf-colour`: exit
0, 121 passed. `cargo test -p conformance`: exit 0. Behind the lock: `raster_golden` exit 0, 974
held, 0 moved; `pdf-model --test corpus` exit 0; `shadings` and `sampled_function_extent` exit 0,
41 passed. `ps -u AI -o nlwp=` summed: 125 threads at the first walk.
