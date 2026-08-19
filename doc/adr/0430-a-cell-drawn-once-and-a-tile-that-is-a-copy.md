# ADR 0430 — A cell drawn once, and a tile that is a copy

Status: accepted, 2026-08-19. Session 595. Takes road D's last owed item, which ADR 0427 named as
the one exception to its own rule and ADR 0429 left standing: **§8.7.3.1's tiling cell**, whose
decode could not be windowed while the cell was re-interpreted once per site. Amends §8.7.2,
§8.7.3, §8.7.3.1 and §12.3.2.4's ledger rows.

## What changed, in one line

**A tiling pattern interprets its cell once and every other site is that cell's commands
displaced.** A decompression bomb hidden in a cell falls from **1055 MB of peak resident memory,
1.27 s and no report at all** to **9.4 MB, 0.12 s and `MAX_OPERATIONS`**; an ordinary hatched page
costs **a tenth of the instructions** it did; and the exclusion that kept the cell out of ADR
0427's window comes off, so all four of §7.8.2's nested streams are now routed by one rule.

## Why the cell may be copied at all, which is the whole of the argument

§8.7.3.1 does not merely permit it, it is the clause's own picture:

> The effect is as if the figure were painted on the surface of a clear glass tile, identical
> copies of which were then laid down in an array covering the area and trimmed to its boundaries.

and the cell is "a content stream containing the painting operators needed to paint **one**
instance of the cell". So the pattern is one figure and a site is that figure moved. Three
properties of *this* interpreter make the copy an equality rather than an approximation, and each
is checked rather than hoped for:

- **The only input that differs between two sites is the transform**, and it differs by a
  translation in pattern space. The content stream, the resources, the optional-content state and
  §8.7.3.3's tint belong to the pattern.
- **The cell's graphics state is initial at every site**, which §11.6.7 requires outright: "[t]he
  definition shall not inherit the current values of the graphics state parameters at the time it
  is evaluated". No site can be reached carrying what another left.
- **Every display-list command carries its own absolute geometry**, so moving one is composing one
  transform — and a clip or a soft mask the cell built moves with it, because it is part of the
  same figure. `pdf_render::Cell` copies both, and refers to a clip that was already in force
  rather than duplicating it: that one bounds the tiling, not the cell.

What a copy is **not** is bit-identical to a re-interpretation. `t.then(by)` adds the site's
displacement to a translation the cell's matrices had already accumulated, and the same sum in a
different order is the same number to within one `f32` rounding. That is the documented cost, it
is smaller than the tolerance `repeated_subpaths` already works to, and the evidence for it is
below.

## The finding this uncovered, which is a clause and not a rounding

18 of the 974 corpus display lists moved. Every one kept its command count exactly, so they were
the rounding above — except that rendering them found **`issue8565.pdf` losing a radial glow to a
flat orange**, and the reason was not the copying:

> A pattern can be used within another pattern (§8.7.2)

— and that sentence finishes by giving the inner pattern's matrix its relationship to the pattern
space of the *outer* pattern.

A pattern named inside a cell was being anchored to the **page**, not to the cell. `Interpreter::form`
has swapped `base` for the form's own space since the fifty-second session, for the sentence
immediately before that one; the tiling cell never did. Page-anchoring makes the tiles differ from
one another, which is exactly what §8.7.3.1's glass tile forbids — and re-interpreting per site hid
it, because each site then re-derived the same page-anchored gradient and one of the tiles happened
to be right. **Drawing the cell once made a five-hundred-and-ninety-five-session-old misreading
visible in one page.** `run_cell` now swaps `base` for the cell's space, and
`a_shading_pattern_inside_a_cell_is_anchored_to_the_cell` fails in both directions.

The other silence found beside it: a cell whose content stream could not be decoded at all was
dropped without a word — the page came back `complete` with the pattern unpainted. A form has said
`undecodable form /Fx` in that circumstance since it was written; a tiling now says the same.

## What it costs and what it buys, measured

`/usr/bin/time` is not on this machine; peak resident is `VmHWM` sampled from `/proc` while the
process runs, and the A/B is this round's own patch applied and un-applied in one sitting, release
profile, `RAYON_NUM_THREADS=1` for the instruction counts.

**A bomb in a tiling cell** — `doc/todo/10` §2's Bomb B moved from a form into a pattern cell: a
1.07 MB file whose cell inflates to 1.10 GB of `n` operators, 1028.7:1, over a 3 × 3 lattice.

| | before | after |
|---|---|---|
| peak resident | 1055 MB | **9.3–9.5 MB** |
| wall clock | 1.27 s | **0.10–0.14 s** |
| what it says | nothing — `"complete": true`, `"unsupported": []` | **`LimitReached { limit: "MAX_OPERATIONS" }`** |

The refusal is a named budget refusal, which is what the road asked for: not ADR 0343's
damaged-prefix sentence, and not the silence the whole-decode route produced.

**Ordinary pages**, interpreted under callgrind (instructions, lower is better):

| | before | after | |
|---|---|---|---|
| a hatched page: 24 marks a cell, 961 sites | 141 375 247 | **13 483 385** | −90.5% |
| `issue2177.pdf` page 1 (sheared pattern) | 31 739 968 | **1 887 674** | −94.1% |
| `issue16038.pdf` page 1 | 3 912 912 | **1 268 937** | −67.6% |
| ISO 32000-2 page 101 × 50 — no pattern, the control | 1 211 699 935 | 1 211 779 472 | +0.0066% |

A tiling is now bounded by the *copies* rather than by the operators it re-read: each copy is
charged to `MAX_OPERATIONS`, the same budget the cell's operators were charged to. The trade is
exact in the direction that matters — a command costs at least one operator to state, so no page
that finished its tiling before reaches the bound now.

## Why the window could then be taken back

ADR 0427's rule is that a decode the memo declines is re-run on every read *anyway*, so windowing
it costs nothing that was not already being paid. The rule had one exception and the `page` fuzz
target found it: `Tiling` held the cell's decode for the whole tiling, so a window would inflate it
once per site — 0.24 s against 9.0 s on a mutated pattern, with `MAX_TILES` allowing four thousand
sites. **The exception was a consequence of the loop, and the loop is gone**: the cell is read once,
so a window is inflated once. `Tiling::content` is an ordinary `NestedContent`, `HeldContent` and
`NestedContent::held` are deleted, and the four nested streams are again decided by one function.

## What was considered and rejected

- **Falling back to per-site interpretation for cells the copy cannot reproduce.** There is no such
  cell — the match in `Displaced::command` is exhaustive on purpose, so a display-list variant added
  later is a compile error rather than a silently un-copied mark — and a fallback would have brought
  the re-inflation back for exactly the documents a bomb would hide behind.
- **Keeping the cell's decode held and copying anyway.** That leaves the gibibyte in place for no
  reason: the reason it was held was the loop.
- **Making the copy bit-identical** by recomposing each command's transform from a cell-local one.
  It cannot be done without interpreting the cell at a scale it is not drawn at, which would move
  every scale-dependent decision inside it (a shading's ramp resolution, a deferred image's grid).
  The rounding is smaller than what the page can show, and the ink sweep below is the evidence.

## Correctness

`examples/display_list_digest` over every pdf.js corpus document's page one, both arms in one
sitting with the same `pdf-sandbox-worker` on disk: **18 of 958 pages moved and every one kept its
command count**. All 18 were then rendered at 2× in both arms and compared byte by byte:

- **16 are pixel-identical**, including every document with `tiling` or `pattern` in its name;
- `22060_A1_01_Plans.pdf` differs in **65 channel bytes of 16 045 152**, by one level;
- `bug1795263.pdf` differs in **18 of 8 015 840**, by one level.

That is the anti-aliasing of a mark whose coordinate moved in its last `f32` digit, and it is what
the rounding above predicts. `issue8565.pdf` is pixel-identical *after* the §8.7.2 fix and was
38.9% different before it, which is how the clause was found.

## Files

- `crates/pdf-render/src/repeat.rs` — `Mark`, `Cell`, `Displaced`, four tests.
- `crates/pdf-render/src/display_list.rs` — `clip_count`.
- `crates/pdf-model/src/content/pattern.rs` — `repeat_cell`, `displacement`, `run_cell`'s `base`
  swap and its report, `settle_cell_box` simplified, `Tiling::content` windowed.
- `crates/pdf-model/src/content/reader.rs`, `run.rs` — `HeldContent` and `held_content_stream`
  deleted.
- `crates/pdf-model/tests/tiling.rs` — three tests; `tests/nested_content_window.rs` — the
  exception's test inverted.
