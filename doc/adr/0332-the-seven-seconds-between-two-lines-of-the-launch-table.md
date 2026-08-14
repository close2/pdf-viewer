# ADR 0332 — The seven seconds between two lines of the launch table

Status: accepted, 2026-08-14. Session 497. Executes steps 1 and 2 of
[`doc/todo/44`](../todo/44-a-draft-that-takes-ten-seconds.md) and prices its step 3 without
building it; the todo file carries the numbers forward.

## Context

The project owner asked whether displaying `tmp/Entwurf.pdf` — their own document, 49.7 MB, one
page, 58 009 display commands — can be improved, and supplied a trace
(`tmp/trace.entwurf.txt`). The trace's launch table ended:

```text
document joined         505.704 ms  (+15.120)
first present         10220.077 ms  (+9714.373)
```

Nine and a half seconds with no line naming them, on the instrument whose whole purpose is that a
slow launch is legible step by step. The first frame's own line accounted for 2 698 ms of it
(`scene 982.1`, `device 1706.6`); roughly seven seconds sat between joining the document and
having a display list, with nothing to say what they were. `doc/todo/44`'s first instruction was
the owner's own: if the trace does not say enough, make `--trace` say it.

## Decision 1: two milestones between `document joined` and `first present`

The hole turned out to be exactly two things, and each now has a line:

- **`interpreted, N cmd`** — page one's display list exists. Marked at the first
  `Event::NeedsRender`, which is where the host first holds the list; the command count is in the
  step name because a slow interpretation is only legible beside how much it interpreted.
  `Launch::interpreted` keeps only the first request — every later one is the steady state and
  the frame log's business — and nothing after the timeline has closed.
- **`first scene built`** — the first frame's display lists have been translated into a GPU
  scene. This boundary is *relayed rather than fabricated*: scene building and device submission
  happen inside one `QuorraPresenter::present` call, so the host cannot take a clock reading
  between them — but `FrameCost::scene` is quorra's own measurement of the translation from the
  moment the call began, so `Launch::scene_built` records the instant the host handed the frame
  over plus that duration. The same discipline ADR 0227 used for the frame stages, one call
  earlier.

`Launch::marks` becomes `Vec<(String, Duration)>` so a milestone can carry its number; everything
else about the table — one `Instant` at `main`'s first statement, two columns, printed once at
the first present — is unchanged.

Verified by running the release binary under `Xvfb` on the owner's document with `--trace`. The
machine was carrying nine parallel rounds, so the wall clock is not evidence and is not quoted as
any; what the run establishes is *structure*: the new lines print, they partition the former gap
completely (`document joined` → `interpreted, 58009 cmd` → `first scene built` →
`first present`, each delta named), and `first scene built` minus `interpreted` agreed with the
first frame line's own `scene` figure to within half a millisecond — the relayed boundary and the
frame's accounting are one measurement, not two.

On the owner's trace, read back through the new table's shape: the ten seconds are ~7.0 s
interpretation, ~1.0 s scene translation, ~1.7 s device (of which `encode` 978 ms) — every
second named.

## Decision 2: the interpretation is attributed, and it is the lexer

`valgrind --tool=callgrind` over `examples/callgrind_interpret tmp/Entwurf.pdf 1` — one
`Document::open` plus one `interpret` of page one, 22 411 M instructions total, of which the open
is ~26 M. Inclusive shares of the total:

| function, inclusive | Ir | share |
|---|---|---|
| `pdf_model::content::interpret` | 22 385 M | 99.9% |
| `Interpreter::run` | 19 071 M | 85.1% |
| **`pdf_syntax::lexer::Lexer::next_token`** | **14 257 M** | **63.6%** |
| — of which `<f64 as FromStr>::from_str` | 3 379 M | 15.1% |
| — of which `Lexer::read_regular_run` | 3 229 M | 14.4% |
| `Document::decoded_stream_data_reported` (§7.4's flate) | 2 850 M | 12.7% |
| — of which `zlib_rs::inflate_fast_help_avx2` | 2 448 M | 10.9% |
| `content::run::points_from` (path operands) | 2 408 M | 10.7% |
| `content::run::numbers_from` | 1 665 M | 7.4% |
| allocator, self (`malloc` + `free` + `realloc` + `RawVec` growth) | ~4 650 M | ~20.8% |

What the page is, counted by `examples/content_budget_census`: **one content stream that inflates
to 141.12 MiB** — the largest decoded stream any instrument of this project has printed — carrying
**20 834 587 lexer tokens** for **3 185 295 operators**, collapsing to 58 009 display commands
(§7.8.2's path operators fold many operators into one command; ~55 to 1 here).

So the answer to the todo's question — lexing, resource lookups, or path arithmetic? — is
**lexing, at about two thirds of the whole**, and the shape under it is worth naming:

- **A heap allocation per token.** `Lexer::read_regular_run` ends in `.to_vec()`
  (`lexer.rs:241`), so ~21 M tokens are ~21 M short-lived `Vec<u8>`s — which is what puts the
  allocator's self cost at a fifth of the interpretation. ~500 instructions per token of pure
  lexing on a page whose tokens are almost all two-to-eight-byte numbers.
- **Float parsing by the standard library's full path.** Every numeric operand goes through
  `str::parse::<f64>` (`lexer.rs:426`), 15.1% of the whole for operands that are almost all
  short decimals.
- **Resource lookups are nowhere** — under 1%, invisible in the table. One giant stream with few
  resources is the opposite population from a form-heavy page, and the todo's suspect list was
  right to rank them last.
- The decoded-stream memo is not the lever, exactly as `doc/todo/41`'s population argument
  predicted for a one-page document opened once: the 141 MiB inflates once, 12.7%.

**No optimisation was built this round.** The attribution is the deliverable; the todo file now
carries it beside the trace so that whoever takes the item optimises what was measured. (An
obvious candidate — a lexer that borrows its token bytes from the decoded stream instead of
copying each token — would be a `pdf-syntax` API change with every parser caller in its blast
radius, and it is priced by this table at up to a fifth of interpretation, not decided here.)

## Decision 3: the encode cache is priced, not implemented

`doc/todo/45`'s quorra `encode` row, made concrete by this document — the full pricing, from the
owner's trace's own numbers, is written into `doc/todo/44` §3, and the shape of it:

- A frame whose display list and view are unchanged re-pays `scene` + `encode` + `transfer` —
  median 315 ms of a 393 ms frame — for a byte-identical answer; what a full reuse leaves is
  ~56 ms. Even a fully-culled frame pays 112–190 ms for `encode` to walk 58 009 commands and
  drop them.
- The half this tree owns is a **retained page scene** in `render-quorra`'s presenter, beside
  ADR 0297's reduced-raster cache and keyed the same way (`Arc` identity + the transform) — but
  retaining the `Scene` alone saves only the median 50 ms `scene` phase, because `encode` runs
  inside `Device::render` on every call.
- The half that pays — reusing the *encoding* — is quorra's, and the two obstacles a design must
  answer (the overlays are rebuilt each frame, so the retained unit must be the page's sub-scene,
  which `quorra_scene` cannot compose today; and reuse across a zoom step needs the page scene
  under a root affine rather than the transform baked per command) are both upstream API
  questions. `doc/QUORRA_FEEDBACK.md` §13's fit — 3.86 µs a command — is the number a retained
  encode has to beat, and 58 009 × 3.86 µs ≈ 224 ms is the trace's own 233.8 ms median, which is
  the fit confirmed on a second document and a second adapter.

Implementation is a later round's, upstream ask first.

## Consequences

- The launch table can no longer hide an interpretation: the next document that is slow to first
  present says which of the three costs it is paying in its own table.
- `doc/todo/44` is rewritten from *evaluation owed* to *measured*: the trace's hole is closed,
  the interpretation cost has names, and the encode cache has a price and a design constraint.
  What remains open there is choosing what to build, which `CLAUDE.md` requires be done from
  these numbers.
- The cost of the new lines is two comparisons per render request and, once, one string format —
  nothing on a steady-state frame.
