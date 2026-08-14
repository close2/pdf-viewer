# A draft that takes ten seconds to appear, and a third of a second per frame after that

Status: **measured** — the owner asked whether displaying this document can be improved and
supplied a trace; session 497 closed the trace's hole, attributed the interpretation with
callgrind, and priced the encode cache (ADR 0332). What is left is *choosing what to build*, and
each candidate below has its number beside it.
Priority: 44
Corpus: none — `tmp/Entwurf.pdf` is the owner's own document (49.7 MB, one page, 58 009 display
commands), outside the tree like `doc/todo/28`'s, with its trace beside it as
`tmp/trace.entwurf.txt` (also untracked; the numbers below are copied from it so this file
survives the trace's deletion, taken 2026-08-14 on the owner's machine, AMD 890M/RADV).
Clauses: none — this is a performance item; §2's launch rules in `CLAUDE.md` are the standard it
is judged against
Code: `crates/viewer-ui/src/bin/pdf-viewer/timing.rs` (the launch table, now with the two stages),
`crates/pdf-syntax/src/lexer.rs` (where the interpretation cost lives),
`crates/render-quorra` (`encode`; where the retained scene would sit, beside ADR 0297's cache)

## 1. The trace's hole is closed (session 497, ADR 0332)

The launch table jumped from `document joined 505.704 ms` to `first present 10220.077 ms
(+9714.373)` with nothing between. It now carries two more milestones:

- **`interpreted, N cmd`** — page one's display list exists, marked at the first
  `Event::NeedsRender` with the command count in the step name;
- **`first scene built`** — the first frame's lists translated into a GPU scene, relayed from
  quorra's own `FrameCost::scene` measurement because the boundary is inside one
  `QuorraPresenter::present` call.

Verified under `Xvfb` on this document (structure only — the machine carried nine parallel
rounds, so no wall clock from that run is quoted): the new lines print, they partition the former
gap completely, and `first scene built` − `interpreted` agrees with the frame line's own `scene`
figure to half a millisecond. Read back through the owner's trace, the ten seconds are **~7.0 s
interpretation, ~1.0 s scene translation, ~1.7 s device** (of which `encode` 978 ms) — every
second named.

## 2. What the seven seconds are (callgrind, session 497)

`valgrind --tool=callgrind` over `examples/callgrind_interpret tmp/Entwurf.pdf 1`: one open plus
one interpretation of page one is **22 411 M instructions**, of which the open is ~26 M. The page
is **one content stream inflating to 141.12 MiB** — `examples/content_budget_census`'s largest
ever — carrying **20 834 587 lexer tokens** for **3 185 295 operators**, collapsed to 58 009
display commands. Inclusive shares of the total:

| function, inclusive | Ir | share |
|---|---|---|
| `pdf_model::content::interpret` | 22 385 M | 99.9% |
| `Interpreter::run` | 19 071 M | 85.1% |
| **`pdf_syntax::lexer::Lexer::next_token`** | **14 257 M** | **63.6%** |
| — of which `<f64 as FromStr>::from_str` | 3 379 M | 15.1% |
| — of which `Lexer::read_regular_run` | 3 229 M | 14.4% |
| `Document::decoded_stream_data_reported` (§7.4's flate, once) | 2 850 M | 12.7% |
| `content::run::points_from` (path operands) | 2 408 M | 10.7% |
| `content::run::numbers_from` | 1 665 M | 7.4% |
| allocator, self (`malloc` + `free` + `realloc` + `RawVec` growth) | ~4 650 M | ~20.8% |

**Lexing is two thirds of the whole; resource lookups are under 1%.** The shape under the lexer:
`read_regular_run` ends in `.to_vec()` (`lexer.rs:241`), so ~21 M tokens are ~21 M short-lived
`Vec<u8>`s — the allocator's fifth of the total is that — and every numeric operand takes
`str::parse::<f64>` (`lexer.rs:426`), 15.1% for operands that are almost all short decimals.
`doc/todo/41`'s population argument held: the 141 MiB inflates once, so the decoded-stream memo
is not the lever here.

**The candidate this names** (not built, not decided): a lexer that borrows token bytes from the
decoded stream rather than copying each token — worth up to ~a fifth of interpretation by the
table above, and a `pdf-syntax` API change with every parser caller in its blast radius. Number
parsing is the second candidate at up to 15%. Measure on this document either way; the
instruments are one command each.

## 3. The encode cache, priced (todo/45's quorra `encode` row — pricing only, ADR 0332)

The trace's 28 frames, sums in ms: frame 17 063.8, of which `scene` 2 396.8, `device` 14 596.8
(`encode` **9 869.0**, `transfer` 2 133.5, `execute` **13.1**, `elsewhere` 2 581.2), `settle`
69.1. Medians: frame 393.1, `scene` 50.2, `encode` 233.8, `transfer` 31.0, `execute` 0.5. The
display list never changed after the first frame.

- **What full reuse buys.** A frame whose display list and view are unchanged re-pays
  `scene` + `encode` + `transfer` — median ~315 ms of a 393 ms frame — for a byte-identical
  answer. What would remain is `execute` + `elsewhere` + `settle` ≈ **56–60 ms**. Even the
  fully-culled frames (58 029 of 58 029 commands culled) pay 112–190 ms in `device` for `encode`
  to walk the commands and drop them; reuse takes those too. A zoom step is currently
  160–310 ms of `device`; under reuse that survives a transform change it is the same ~60 ms.
- **Where it lives, and the split matters.** The retained *page scene* is this tree's, in
  `render-quorra`'s presenter beside ADR 0297's reduced-raster cache and keyed the same way
  (page display list `Arc` identity + the transform + viewport). But retaining the `Scene`
  alone saves only the `scene` phase — median 50.2 ms, 2.4 s of 17.1 here — because `encode`
  runs inside `Device::render` on every call. **The phase that pays is quorra's to reuse**, and
  `doc/QUORRA_FEEDBACK.md` §13's fit (3.86 µs/cmd + 3.84 ms) is confirmed by this document on a
  second adapter: 58 009 × 3.86 µs ≈ 224 ms against the trace's 233.8 median.
- **Two design obstacles, both upstream API questions.** (a) The frame's scene also carries the
  background and the overlays, which this host rebuilds every frame with fresh `Arc`s
  (`Overlays::of`), so the retained unit must be the page's own *sub-scene* — and
  `quorra_scene` has no way to compose a retained fragment into a frame today. (b) The target
  transform is baked into every command by `render-quorra`'s `Encoder`, so reuse across a zoom
  step needs the page scene built in page space under a root affine (`Viewport` already takes
  one) rather than re-encoded per scale.
- **So the item is an upstream ask first** — a retained/reusable encoded scene, or scene-fragment
  composition — with the host-side retained page scene beside ADR 0297's precedent once the
  reuse exists to feed. quorra's `Options::instrument_encode` (its ADR 0023, unused here) can
  subdivide `encode` first if the ask wants finer numbers.

## 4. What is left

Choose what to build, from the numbers above — `CLAUDE.md` forbids optimising what nobody
measured, and everything here now is. The two levers are independent and address the two
different costs: the lexer (the ten seconds, once per open) and the encode reuse (the third of a
second, every frame). Neither is small; both have their measurement in this file.

## Cross-references

`doc/todo/45` (where a frame goes — quorra's `encode` was already its open row; §3 above is that
row priced on a second document), `doc/todo/42` (the launch path; its items are the program's own
startup, where this document's cost is one page's interpretation — different lever, same gate),
ADR 0297 (a per-frame recomputation kept out of the loop once before, and where the key would
live), ADR 0332 (this round's argument).
