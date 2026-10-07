# 1632 — A group's composed grid is sampled across the pool, a uniform run of stops is not asked again, and a group's second run is lexed again

Status: accepted. Session 1398. Answers the brief's question about `bug1721218_reduced.pdf`'s
largest stage, `interp` (ADR 1619's record). Supersedes nothing; amends nothing.
Context: `CLAUDE.md` principle 2 and its rule that an optimisation is justified by a benchmark and
explained by a comment; ISO 32000-2 §7.8.2, §8.5.4, §11.6.6, §11.7.2; ADRs 0068, 0272, 0343,
1507, 1521, 1529; `doc/habits/measuring.md` 45, 63, 65 and 68.
Code: `crates/pdf-model/src/content/transparency.rs` (new: `sampled_grid`; `composed_into_parent`,
`resampled_cube`), `crates/pdf-render/src/shading.rs` (new: `uniform_with`, `spans`; `simplify`,
`on_the_line`); instrument `crates/render-raster/examples/turn_interpret.rs` (new).
Tests: `pdf-render`'s `a_uniform_run_is_the_run_every_middle_would_have_made` (8 000 stop lists
against a copy of the old walk, bit for bit; watched failing with the finiteness test taken out
of `uniform_with`); the corpus gate's per-page digests on six arms.

## 1. Counted first: what `interp` is on this page

Callgrind by function, `--profile gates`, one thread, pinned, the page interpreted as
`frame_cost::read` interprets it: a turn is a run of two interpretations less a run of one
(`examples/turn_interpret`), because the first fills the process's press and ink tables that every
timed round finds full. **The turn is 1 388.4 million instructions**, about 69 ms.

- **The brief's premise does not hold: nothing is flattened during interpretation.** A `W n` adds
  §8.5.4's path and transform to the display list (`DisplayList::add_clip`, deduplicated by a
  digest and an equality): 7 229 clips a run at 5.0 thousand instructions each, SipHash included.
  The four Béziers become the 37-gon in `raster`, after the list exists. Every dot states its own
  `W n` (7 072 in the stream, 3 410 of them one dot) and its own `0 0 612 792 re W n`.
- **The content is interpreted twice.** The form states a `/DeviceCMYK` group, and
  `group_commands` runs its whole content for the chromatic half and again for the black half
  (ADR 0272), 597 413 tokens a run of a 2.8 MB stream; ADR 1529's reuse is raster's, after both
  lists exist. **The second run is 569.5 M, 41%**, and a probe skipping it (wrong bytes) took the
  interpretation from 73.8–81.4 to 47.0–48.6 ms in one sitting at a load of 11.
- **Per operator**: each `sh` (3 490 a run) is a `shading::Cache` hit but for 137 builds a turn at
  3.05 M each — `Function::parse` 1.07 M (a 4 096-sample stream decoded), 256 samples converted
  1.99 M, `simplify` 0.80 M; each path-painting or clip operator 4.6 thousand through `end_path`;
  the lexer 186.6 M; the sixteen soft masks 172 M.
- **What `interp` holds that is not interpretation**: the list's byte budget (ADR 1507) does not
  appear in the profile, and the scene is not in it. What is in it is colour management — the
  ramps above, and **the group's conversion out composed with its parent's cube: 83 521
  conversions, 136.4 M, about 5 ms on the interpreter's thread** (a probe skipping it).

## 2. Built, each exact by construction

- **The composed grid is sampled across rayon's pool** (`sampled_grid`, for the four-component
  composition and the three-component resampling). Each point is a function of its coordinates
  alone and `collect` keeps the index order, so the grid is the one the nested loops built; the
  index decomposition costs 12 M more instructions, all of them on the pool.
- **A uniform run of stops is not asked again.** `simplify` asks every middle of every candidate
  (ADR 0068), which is quadratic on a straight ramp. Where every stop from the anchor is the
  anchor's own finite colour at a finite position the answer is yes by the arithmetic itself — a
  zero times a fraction the clamp keeps finite — so it is not computed. 26 of the turn's 137 ramps
  are constant: `simplify` 109.5 → 75.9 M.

## 3. What it bought

Callgrind, the turn: 1 388.4 → 1 369.1 M, of which 79.8 M now on the pool. ISO 32000-2 page 101
fifty times, which states no such group: 1 323.45 → 1 318.46 M, all of it libc's `memcpy`
(habit 45: layout, no function's count moved). The clock, `frame_budget` on exports of the tree
before and after, md5-distinct, three runs of five rounds interleaved, pinned to the performance
cores, load 2.3 to 2.9, minima: the turn 152.43–157.97 → 146.32–147.45 ms, its `interp`
68.21–69.96 → 62.96–63.90; the seventh frame 152.17–155.10 → 147.72–148.58; the step, which
interprets nothing, 83.34–85.40 → 84.57–85.42. No launch row reaches either construction:
page one of each, under callgrind, reads within 0.08% of the tree before it.

## 4. Measured and not kept: a group's second run handed its tokens

Built and measured in this round, then taken out. While such a group ran, each content stream it
read cleanly was kept as the tokens its reader lent and replayed to the black half's run — exact
(the operand loop handed the same tokens through the same code), kept only for a whole read that
raised nothing (ADR 0343), asked for no inline image's bytes and stepped over no string (ADR 1521:
a fontless read's tape lends a later read under a font empty strings, which a test showed drawing
one fill of three), and bounded at 64 MiB a group. It took the lexer from 186.6 to 93.9 M and the
reader's look-ahead from 72.9 to 36.4 M, and the recording and the replay cost 58 M back, about
34 M net — near a millisecond of the turn. Every shape it was given put the cost of one more arm
on every other page's operand loop: a recorder field cost ISO 32000-2 page 101 0.38%, a shape of
the reader's own 0.19%, a single call site 0.26%. A millisecond on four-component groups is not
worth a tax on every page, so it is not built; the measurement is here so that it is not built
again on the same argument.

## 5. Priced and not built

- **The black run itself, 26 to 33 ms by the probe above.** Re-resolving the chromatic list's colours under the black
  half instead of running the content again needs every colour decision the interpreter makes —
  fills, strokes, shadings, images, patterns' cells, text, transfer, overprint — reproduced from
  the list, each a place a pair could diverge without a report. A design, not a lever.
- **The black run's `Function::parse`, 67 M, about 2 ms**: a probe memoising every parse across
  interpretations took 70.1–70.8 to 65.7–66.3 ms, and the black run's half of that is the part a
  memo inside one interpretation would take. `pdf_colour::shading::Cache` keys a build by its
  conversion, so the second half parses each function again. A memo of the parsed functions by
  object is exact and belongs to `pdf-colour`, which this round did not hold and a sibling is
  editing.
