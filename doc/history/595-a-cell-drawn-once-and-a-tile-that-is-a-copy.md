# 595 — A cell drawn once, and a tile that is a copy

Road D's last item. ADR 0427 windowed three of §7.8.2's nested content streams and had to exclude
the fourth — §8.7.3.1's tiling cell — because `Tiling` held its decode for a loop that ran once per
site; ADR 0429 left it standing. This round removes the loop instead of the exclusion: **the cell
is interpreted once and every other site is its commands displaced**, which is §8.7.3.1's own
construction rather than an optimisation of it. The window then costs nothing, and road D closes.

Date: 2026-08-19.
ADR: [0430](../adr/0430-a-cell-drawn-once-and-a-tile-that-is-a-copy.md).

## What the numbers said

A bomb built for a pattern cell — `doc/todo/10` §2's Bomb B, 1.07 MB inflating to 1.10 GB of `n`
operators at 1028.7:1, over a 3 × 3 lattice. `VmHWM` from `/proc`, `pdf-retrieve page … 0`,
release, this round's own patch applied and un-applied in one sitting:

| the bomb in a tiling cell | before | after |
|---|---|---|
| peak resident | 1055 MB | **9.3–9.5 MB** |
| wall clock | 1.27 s | **0.10–0.14 s** |
| what it reported | **nothing** — `"complete": true`, `"unsupported": []` | `LimitReached { limit: "MAX_OPERATIONS" }` |

The silence is the second finding: a cell whose stream could not be decoded was dropped without a
word, where a form has said `undecodable form /Fx` since it was written. It now says the same.

Ordinary pages, callgrind, `RAYON_NUM_THREADS=1`, instructions:

| | before | after | |
|---|---|---|---|
| a hatched page, 24 marks a cell over 961 sites | 141 375 247 | 13 483 385 | **−90.5%** |
| `issue2177.pdf` page 1 | 31 739 968 | 1 887 674 | **−94.1%** |
| `issue16038.pdf` page 1 | 3 912 912 | 1 268 937 | **−67.6%** |
| ISO 32000-2 page 101 × 50, the control | 1 211 699 935 | 1 211 779 472 | +0.0066% |

Both directions improve, which is what the road predicted: interpreting a figure once is less work
than interpreting it four thousand times, and holding no decode is less memory than holding one.

## The digest moved, and looking at the pages it moved found a clause

**18 of 958 pages** moved in `examples/display_list_digest`, every one with its command count
unchanged — the signature of `t.then(by)` adding a displacement to a translation the cell's
matrices had already accumulated. All 18 were rendered at 2× in both arms and compared byte by
byte: **16 pixel-identical**, `22060_A1_01_Plans.pdf` differing in 65 channel bytes of 16 045 152
by one level, `bug1795263.pdf` in 18 of 8 015 840.

The seventeenth was `issue8565.pdf`, **38.9% different**, and it was not the rounding. §8.7.2:
"[a] pattern can be used within another pattern", the sentence going on to give the inner pattern's
matrix its relationship to the pattern space of the *outer* pattern — and a pattern named inside a
cell was anchored to the **page**. `Interpreter::form` has swapped `base` for the form's own space
since the fifty-second session, for the sentence immediately before that one; the cell never did.
Re-interpreting per site hid it, because every site then re-derived the same page-anchored gradient
and whichever site sat at the pattern origin looked right. Drawing the cell once made a
five-hundred-session-old misreading visible in one page, which is trap 1 paying for itself:
`run_cell` swaps `base`, and the page is pixel-identical again.

## The spec-driven half — §12.3.2.4's `/SD`

Session 594 priced it and left it: an erratum repointed the clause's reference from Table 201 to
Tables 202–204, which made "may optionally contain an SD entry" readable, and
`Destination::read_within` never looked for one. It does now, through **one** function —
`preferring_structure`, which §12.6.4.2's action and §12.3.2.4's destination dictionary both ask,
because the clause makes them one entry. An `/SD` that resolves to nothing falls back to `/D`,
which is what a `should` beside a required entry permits.

**The corpus cannot rank it, said out loud rather than around** (trap 8):
`examples/structure_destination_census`, extended to walk both of the clause's tables, over
**66 901 documents** — the 974 pdf.js corpus and 65 937 crawled — finds **374 101 named
destinations and not one states an `/SD`**. So the witness is built: a pair differing in the single
entry, reached once as a name through the catalog's `/Dests` and once as a string through the name
tree.

## Fuzzing

`cargo +nightly fuzz run page`, seeded with the round's own tiling documents — the bomb, a hatched
page, `issue8565.pdf`, `issue16038.pdf`, `issue2177.pdf`, `tiling-pattern-box.pdf` — for the ten
minutes `doc/todo/02` allows: **345 363 executions, 8569 covered edges, 0 crashes, 0 OOMs, 0
timeouts**, and no new artifact. The exception this round removed was found by this target in 592,
which is why it is the one to run.

## Gates

`doc/todo/02` §2's full sequence, green. One failure during the round, fixed and re-run:
`every_quotation_is_the_standards_own_words` caught a blockquote of §8.7.2 that was not verbatim —
the standard sets "relationship" broken across a line — so the quote is now the clause it can be
and the rest is prose. Binaries rebuilt and installed before every measurement, per §5.

## Files

- `crates/pdf-render/src/repeat.rs` — `Mark`, `Cell`, `Displaced`; four tests.
- `crates/pdf-render/src/display_list.rs` — `clip_count`.
- `crates/pdf-model/src/content/pattern.rs` — `repeat_cell`, `displacement`, `run_cell`'s `base`
  swap and its report, `settle_cell_box` simplified, `Tiling::content` windowed.
- `crates/pdf-model/src/content/reader.rs`, `run.rs` — `HeldContent`, `held_content_stream` and
  `NestedContent::held` deleted; the exception ends with the loop that caused it.
- `crates/pdf-model/src/destination.rs` — `preferring_structure`, and its test.
- `crates/pdf-model/examples/structure_destination_census.rs` — §12.3.2.4's two tables.
- `crates/pdf-model/tests/tiling.rs` — three tests; `tests/nested_content_window.rs` — the
  exception's test inverted.
- `doc/conformance/ledger.toml` — §8.7.2, §8.7.3.1, §12.3.2.4.
- `doc/adr/0430`, `doc/todo/14` (closed), `doc/todo/10`, `doc/todo/README.md`.
