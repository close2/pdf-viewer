# 1579 — A window is drawn where it leaves out an eighth of its page's commands or half its pixels

Slot 3 of batch ninety-two, 2026-10-19, a pixels round. ADR 1994; ADR 1995 not used; no question;
no ledger row moves (the change cites no clause).

**Premise, held**: `oversized` drew a window only at half the page's area, and ADR 1980's crossing
sat at 70–83 %. **Hypothesis, held in its quantity and not its figure**: commands kept decide, not
area. On `issue12810.pdf` a window of 61 % keeps 97.9 % of the commands and ties, while the
witness's window of 63 % keeps 83 % and leads by up to 100 ms. The bound is seven eighths, not
four fifths. Pixels still decide a page of few commands (`22060_A1_01_Plans.pdf`), so the half stays
as a second clause. **The two rules did not collapse.** The confined worker draws windows only past
the bound, where the wire admits them, and its window shape is unchanged, so ADR 1986's two
statements still agree (ADR 1994 §3).

**Built.** `DisplayList::share_reaching` estimates a window's share from a stride of at most 4 096
commands, within 1.5 points in 80–580 µs on the toolkit thread. `oversize::oversized` prices the
centred largest window by it (`KEPT` = 7/8), and `largest_centred` moved there from the example.
`window_against_whole` prints commands kept, the estimate and its cost, and what the page's target
leaves out; `WINDOW_AGAINST_WHOLE_COPY` puts the pre-1980 copy back. **The finding**: no command
falls outside the witness's own target at any magnification. ADR 1980's 20–150 ms on the whole page
followed the window before it in the instrument's alternation. Putting that copy back cost the next
whole draw 0–135 ms, not monotone in its size; the mechanism is not shown.

**Proof that the pixels did not move.** The six arms of the export (HEAD + `reach.rs`) against
HEAD's arms: 968, 968, 968, 965, 965, 965 pages, 0 digests and 0 mean errors differ. What
moves is which toolkit pages are drawn as windows, and that is ADR 1994.

**Files.** `pdf-render/src/reach.rs`, `viewer-host/src/oversize.rs`,
`viewer-host/examples/window_against_whole.rs`, `viewer-host/tests/oversized_pages.rs`,
`tools/drive-windows.sh` (step `82-zoomed-window`), ADR 1994, `doc/{performance,state-of-play}.md`,
`doc/todo/30`, this record. **Unfinished**: the confined window's bound (a wire change);
`issue1905.pdf`'s windows tie on 24 threads; the copy's mechanism.

**Gates.** `rustfmt --check` 0; `clippy -D warnings --all-targets -p pdf-render -p viewer-host` 0;
`nextest` 565 passed, 1 skipped; `cargo test -p conformance` 0; `state.sh docs` 0. Plants: an
estimate without the margin failed the short-list test, `KEPT` of 0 the new `oversized_pages` one.
Arms cpu/gpu/compute at 1× and 4× 0 each. Drive (82's six steps and 84, four windows): 21 works,
1 wrong, 84 in `quorra-confined`, whose bin lacked `pdf-view-worker`; with it 84 and 82's three
confined steps work. The clock hold (load 6.9–9.1) is ADR 1994 §1, headless: a toolkit's window is
drawn on the processor, so the owner's loop has no figure to add. Duration 8 800 s.
