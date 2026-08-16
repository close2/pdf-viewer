# A draft that takes ten seconds to appear, and a third of a second per frame after that

Status: **open again, at a different phase** — both levers this file measured are built, and
session 533 measured the one it never named: a *zoom* frame is 59% quorra's coverage rasterisation
on one thread, and this file's remaining item is 2.4% of it (§5, ADR 0368). The owner asked whether displaying
this document can be improved and supplied a trace; session 497 closed the trace's hole,
attributed the interpretation with callgrind, and priced the encode cache (ADR 0332). Session 506
took §2's lexer candidate *and* its number-parsing second (ADR 0341): interpreting this document
lost 39.8% of its instructions, byte-identical readback on this document and on ISO 32000-2.
**Session 516 took §3** — upstream built the retained encode at `580fa4ac` (their ADR 0048, after
pricing it at `87898c69`), and this tree adopted it in ADR 0351: a frame whose page, placement,
window, medium and chrome are the last frame's builds no scene and encodes nothing. §3.2 is what
it did. **Session 535 retook §2's attribution, which three rounds of its own work had made stale,
and took the three levers the new one names** (§2a, ADR 0370) — a fixed-size operand marshalling,
§7.2.3's classification as a table, and §7.3.3's fixed format asked before the digit scan.
What is left of this file is §3.1's second half — the page-space construction that would
buy the `scene` phase back across *zoom* steps, which needs nothing from upstream and which the
trace this file is about does not exercise.
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

**This table is the attribution as it stood in session 497, kept because it is what the decisions
below were made from. It is no longer the shape of this document's interpretation — §2a is.**

**Both candidates this section named were taken in session 506 (ADR 0341).** The lexer borrows
its token bytes from the decoded stream (`Token<'a>`, `Keyword(&'a [u8])`), and §7.3.3's fixed
format is parsed from the bytes directly with an exactness argument that keeps it bit-identical
to `f64::from_str`. Measured on this document with the same instrument: 22 398 M instructions →
17 046 M after the borrow alone (−23.9%) → 13 487 M with the number parse (**−39.8%**), the
readback byte-identical here and over all 1023 pages of ISO 32000-2, and a corpus-normal cold
`find_cost` sweep improved 5.97% rather than regressed. The numbers, the caller survey and the
declined designs are ADR 0341's; the table above is kept as the measurement the decision was
made from.

## 2a. The attribution retaken, and the three levers under it (session 535, ADR 0370)

§2's table was three rounds old by the five-hundred-and-thirty-fifth: ADR 0341 had halved the
lexer under it and ADR 0365 had put the stream behind a window. **A profile that old cannot say
what to optimise**, and this is the rule the round is worth remembering for rather than any of its
numbers — the launch table still named interpretation as the largest single item on this
document's path, so the item was live and its evidence was not.

Retaken with the same instrument (`callgrind_interpret` on the witness, `RAYON_NUM_THREADS=1`,
`--profile gates`), the shape had changed in three ways worth stating as shapes:

- **The lexer had not moved at all.** `Lexer::next_token` was within 0.07% of where ADR 0341 left
  it; its *share* had fallen only because the denominator grew. ADR 0365's per-token bookkeeping
  is a separate item — the interpreter's own self cost — and reading the share as "the lexer got
  worse" would have optimised the wrong function.
- **What ADR 0332's table had ranked fourth was now second**: `points_from`, the marshalling of a
  path operator's operands, at 18.16% — because `numbers_from` collected a `filter_map` whose
  lower size hint is zero, so six floats cost a `malloc` and two `realloc`s, three million times.
  It was invisible in 2 while lexing was two thirds of the whole.
- **Three suspects were answered by the profile alone and cost nothing to check**: the graphics
  state cloned per `q` (1 856 Ir), the resource lookup (334 Ir) and the display list's own growth
  (237 613 Ir). A suspect list is worth writing down before it is worth acting on.

The three levers taken, the five declined and every number is ADR 0370. What the round did *not*
find is worth as much: nothing on `pdf-render`'s display list, and the reader's window bookkeeping
left alone on purpose because ADR 0365 bought 187 MB of peak memory with it.

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

### 3.1 What `87898c69` answered, and the question this tree owes back (session 512, ADR 0347)

Quorra's ADR 0045 priced the reuse and **built neither ask**, each for a stated reason:

- **The identical-frame replay is measured at 0.154 ms against 1.538 re-encoded** (their dense-text
  archetype, minima, RADV) — the 233.8 ms median `encode` of this document's trace going to
  approximately nothing, the fully-culled frames' 112–190 ms with it. It is unbuilt because a
  device-side cache keyed on scene identity would **miss every frame of this host**: the frame's
  scene carries the background and the overlays, rebuilt with fresh `Arc`s per frame
  (obstacle (a), confirmed independently from their side).
- **Obstacle (b) is corrected rather than confirmed: zoom reuse is not available at any price.**
  The linear part of the device transform is inside every atlas key, the flattening and the lane
  choice, and the quantised sub-pixel phase is the fractional translation — a zoom step is a
  genuinely different rasterisation of every glyph. What building the page scene in page space
  under `Viewport`'s existing affine buys is the **`scene` phase only** (median 50.2 ms here),
  and that needs nothing from upstream — §2.3 of the brief was already that contract. This
  file's earlier sentence "a zoom step … under reuse that survives a transform change it is the
  same ~60 ms" is therefore **withdrawn**; what reuse takes is the case the trace is actually
  full of, 28 frames of one document at one view.

**Their question back**: *can the host draw the page and the overlays as two `render` calls into
the same target?* If yes, replay needs no new scene vocabulary; if no, the reason why is the
specification for fragment composition, designed from that reason rather than from the general
shape.

**This tree's answer is no, for a reason `present.rs` can name** (`render_quorra::present::build`):
the frame is deliberately **one scene** — background rect, page commands, overlays — because the
selection overlay is `Multiply` fills (ADR 0176) that must composite against the page beneath
them. Two `render` calls would need the second's root pass to begin over the target's existing
pixels rather than over a cleared backdrop (their `PassLoad::Clear` is the first pass onto every
plan today), and §11.3.5's implicit blend group would need the target's content as its backdrop.
That — a root over stated content, or a composable retained fragment — is the specification
upstream asked for, and carrying it across is the next step of this item. Not designed here: it
is a change to the contract between the two trees, so it is theirs to shape from this reason
(the same order `Device::warm_for` followed, in reverse).

### 3.2 Taken at `580fa4ac` (session 516, ADR 0351) — and the residue is what §3 computed

Upstream built the retained encode: `RetainedScene` is a handle the caller holds, owning the
`Scene` and the encode of its last frame, and `Device::render_retained` replays that encode when
nothing an encode reads has moved. `doc/QUORRA_RETAINED_FRAME.md` is the migration they wrote for
this tree; ADR 0351 is what it cost and the four judgements inside it. The shape here:
`render-quorra`'s `FrameSlot` keys the frame's scene on the page display list's `Arc` identity and
placement, the window, the medium, and the chrome by value — so the *page* is reused by identity
and the chrome, which this host rebuilds every frame, by content.

Re-run on this document, under `Xvfb` on `llvmpipe`, 25 frames that change nothing (`Up` with the
page already at the top), three runs an arm alternating, both arms at the same quorra revision so
the adoption is the only variable. Medians of the two quiet runs an arm, in ms:

| | frame | `scene` | `encode` | `transfer` | `execute` | `elsewhere` | `settle` |
|---|---:|---:|---:|---:|---:|---:|---:|
| before | 128.8 / 126.9 | 14.2 / 17.3 | 81.6 / 81.9 | 0.3 | 28.0 / 26.2 | 2.4 / 2.3 | 0.8 / 0.7 |
| after | **29.3 / 31.3** | **0.0** | **0.0** | 0.3 | 28.0 / 29.9 | 0.8 / 0.9 | **0.0** |

**What is left is `execute` and a fraction**, which is the sentence §3 wrote before anything was
built. Its ≈56–60 ms was arithmetic on the owner's RADV trace and this is the software adapter, so
the number is a different machine's and only the structure carries across — and `execute` being
unmoved is quorra's own llvmpipe finding, which is what makes the residue believable rather than a
measurement that lost something. Without a clock: **24 of 25 frames replayed**, uploads went
58 989 → 58 029 (40 a frame to none), and the handle held **3 830 032 bytes** for this page.
The launch table is unmoved, as a first frame that reuses nothing requires.

### 3.3 Their question back is answered — and it is a *target* question rather than a scene one (session 547, ADR 0382)

*Can the host draw the page and the overlays as two `render` calls into the same target?*
**Yes, and there is no case here that needs fragment composition.** `render_quorra::present::build`
assembles one scene in one order — a window-sized background rectangle, the page's display list,
then the chrome's — with no transparency group spanning page and overlay, no overlay clipped by page
geometry, and no blend mode on an overlay that has to see the page beneath it.

So the specification for scene-fragment composition that upstream offered to design from does not
exist, and neither side should build it. What the answer turns into instead is narrower and is
**ours**:

- Two `render` calls into one target work today **against `Target::Texture` and only against it**: a
  non-empty `Viewport::damage` is honoured exactly there (`LoadOp::Load` over the retained
  contents), so the page survives the overlay pass.
- Against **`Target::Surface`**, which is what this tree's window uses, a surface has no retained
  contents to patch — the second call clears and redraws the whole thing, and quorra reports it
  rather than doing it quietly.

**So the encode cache's obstacle (a) is closed by moving the presenter's target, not by new scene
vocabulary**, and that same move is what session 548's reprojecting presenter needs.
`QUORRA_FEEDBACK.md` §28.4 and §28.6 have the argument and the one question that went back with it;
ADR 0382 §6 is why the three items are one item.

## 4. What is left

Both levers this file measured are built: the lexer (the ten seconds, once per open — §2, ADR
0341) and the encode reuse (the third of a second, every frame — §3.2, ADR 0351). What is left is
one item and it is *not* on this document's own critical path, which is why the file stays open
rather than closing:

- **The page scene built in page space under `Viewport`'s root affine.** It buys the `scene` phase
  across *zoom* steps and nothing else — §3.1's second bullet is upstream's correction, and it is
  final: a zoom step is a genuinely different rasterisation of every glyph, so no design reuses an
  encode across one. It needs nothing from quorra. **Session 533 priced it: 2.4% of a zoom frame**
  (§5). It stays open because it is cheap and correct — the brief's §2.3 already asks a scene to be
  viewport-independent — and not because it is a lever.

## 5. The zoom step, measured at last, and it is not the `scene` phase (session 533, ADR 0368)

§4 said a person zooming is a different population and that a witness would have to come from
`doc/todo/45`. The project owner asked the question directly instead — *could this document be
rendered every frame?* — and ADR 0368 is the answer. What it changes here is the ranking: §3's
whole conversation was about `encode` being **replayed**, and what a zoom frame actually spends is
`encode` being **computed**, in a phase this file never named.

The frame at the fit view, 58 009 commands, nothing culled, the magnification new (llvmpipe,
three sessions, 639.8 / 660.0 / 661.9 ms):

| | ms | share |
|---|---:|---:|
| `scene` — what §4's remaining item would remove | 15.8 | **2.5%** |
| `encode` | 475.9 | 74.4% |
| — `encode: geometry` | 406.3 | **59% of the frame** |
| — `encode: recording` | 82.9 | 12.9% |
| — `encode: staging` | 23.6 | 3.7% |
| `transfer` | 65.4 | 10.2% |
| `execute` (the adapter's own timestamps) | 29.1 | 4.5% |

### 5b. The same frame after the thread pool, on the owner's own adapter (session 552, ADR 0387)

ADR 0377 turned `encode_threads` on and the frame changed shape. The owner's `tmp/trace2.entwurf.txt`,
medians of 15 frames of a zoom session:

| | ms | share |
|---|---:|---:|
| `scene` | 12.6 | 4.6 % |
| `encode` | 128.9 | 47.4 % |
| `transfer` | 64.3 | 23.6 % |
| **`execute`** | **0.2** | **0.07 %** |
| `elsewhere` | 62.6 | 23.0 % |
| **whole frame** | **272.0** | |

**The row to read is `execute`.** The graphics device does about a thousandth of this frame; every
other row is one host thread, and two of them — `transfer` and `elsewhere` — are quorra's with no
lever on this side at all. What session 552 took is the third: `scene`, −20.5 % by removing a
device-pixel window computed for every fill and read by none of them (ADR 0387 §3). What it
established about the other two is that neither is what it looked like: the 40 uploads move none of
`transfer`'s bytes (§3a of `doc/todo/45`), and `elsewhere` is host time inside `Device::render` that
quorra measures and discards. `crates/render-quorra/examples/zoom_frame.rs` is the instrument, and it
runs on the real adapter without a window.

The subdivision is `Options::instrument_encode`, which §3 named as available and nobody had
switched on. **The geometry phase is quorra's scanline rasteriser flattening this page's
3 011 879 path segments into 58 003 coverage tiles on one thread**, and it is identified rather
than merely named: the same view drawn twice in one session costs `encode` 483.8 ms and then
**90.6**, and the second draw's subdivision is geometry **1.7** against recording **91.8** — so the
406 ms is coverage and the 92 ms that survives every cache is instance building.

So the answer to the owner's question is three answers. **A still window: already yes** (§3.2). **A
repeat magnification: 140 ms rather than 640**, on quorra's cache — though this session's script
hit that reuse once and missed it once, which ADR 0368 leaves open. **A new magnification: no**, and
ADR 0368 enumerates why no change to the boundary or to the IR buys it — a page-space scene 2.4%,
batching by paint state 1.0% of the commands and a loss beyond that, damage nothing, and dropping
sub-pixel marks forbidden by §10.7.4 outright. `doc/QUORRA_ENCODE_THREADS.md` is what went
upstream: divide `encode` across more than one thread, with its own ceiling stated — geometry at
zero still leaves a 235 ms frame.

## Cross-references

`doc/todo/45` (where a frame goes — quorra's `encode` was already its open row; §3 above is that
row priced on a second document), `doc/todo/42` (the launch path; its items are the program's own
startup, where this document's cost is one page's interpretation — different lever, same gate),
ADR 0297 (a per-frame recomputation kept out of the loop once before, and whose key shape §3.2's
took), ADR 0332 (the round that priced this), ADR 0351 (the round that took §3), ADR 0368 and
`doc/QUORRA_ENCODE_THREADS.md` (§5 above: where a *zoom* frame goes, and the ask it produced),
`doc/QUORRA_RETAINED_FRAME.md` (the migration upstream wrote for it) and `doc/QUORRA_FEEDBACK.md`
§23 (what went back: one correction to that document, one declined item, and the answer to the
question it asked).
