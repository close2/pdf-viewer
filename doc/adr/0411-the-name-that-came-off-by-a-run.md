# ADR 0411 — The name that came off by a run

Status: accepted, 2026-08-18. Session 576. Moves `Cargo.lock`'s two quorra pins from `eada81ec`
to `cad50156` — 121 commits, 92 of them non-merge, carrying fourteen new ADRs of theirs. Drops one name from
`REFUSED_AT_FOUR` in `crates/render-quorra/tests/corpus.rs`, amends §10.7.4's ledger row, and
answers the one question the release asks of this tree by name. No other source file in this
tree changed, and none had to.

## 1. What the release is, before what it costs

`doc/RENDER_LIBRARY.md` is the brief quorra was written against, and it is worth having in mind
when reading a range this size: what this viewer asked for is a *document* renderer that states
clause 11 natively and **refuses rather than silently drawing nothing** (§5 of that brief). Both
halves of this release are that brief being worked through — one refusal made narrower, one made
wider, and one rule of clause 10 that a lane was breaking without saying so.

The delta is 121 commits and 32 000 lines, most of it the library's own tests, notes and ADRs.
Four things in it reach this tree, and they are the four to know:

| | what moved | what it is |
|---|---|---|
| their ADR 0057 | a clipped mark's coverage tile is bounded by **its chain's own device box** rather than by the open clip rectangle, and a refused frame names the *sheet* it met | a refusal removed, and the one that stays made legible |
| their ADR 0070 | a mark whose thin axis is under the device coverage lane's sample-column spacing **keeps the processor lane** | §10.7.4's first requirement, met on the lane that was breaking it |
| their ADR 0066 | a soft mask is a knockout element's **opacity**, not its shape | a no-op for this translator, and §3 below says why that is a measurement rather than luck |
| their ADR 0069 | a *group* used as an element of a knockout group is refused by name | a refusal this tree already raises one crate earlier |
| `SceneError::InvalidImageAlpha` | an image's out-of-range alpha stops being reported as a *group*'s | additive; §4 is the question it comes with |

Everything else is splits (`error.rs` into seven modules, `raster.rs` into three, `pipeline.rs`,
`geom.rs`, `outline.rs`), additive counters and limits, a `quorra-pages` crate this tree does not
depend on, and the library's own display measurements.

**The bump itself cost nothing to compile.** `cargo build -p render-quorra --all-targets` passes
against `cad50156` with no source change: the release removes and renames nothing this tree names.
That is the seventh consecutive bump of which that is true, and it is a property of how the
library is versioned rather than of how little it did — 121 commits is the largest range this
tree has taken.

**One thing in the lock moved that is not quorra's**, and it is recorded rather than waved past:
re-resolution moved six packages' `windows-sys` from 0.61.2 to 0.52.0 — among them `rustix`,
`tempfile`, `gobject-sys` and `is-terminal`. Both versions were already in the lock and both
still are; nothing upstream asked for it, and quorra's only manifest change in the whole range is
a *dev*-dependency on their new `quorra-pages`, which this tree never resolves. It is
Windows-only code that no gate on this machine compiles, so it was checked the one way that can
see it: `doc/verify.md`'s cross-target runs under `-D warnings`, `x86_64-pc-windows-msvc` for
`pdf-sandbox`, `pdf-render`, `viewer-confined` and `viewer-ui`, with `aarch64-apple-darwin`
beside them. All clean, and `cargo deny check` is clean on the new graph.

## 2. The name came off by a run, which is the decision

`bug1703683_page2_reduced.pdf` is a page whose 141 clip chains ask for 1 008 561 911 coverage
texels where the chains themselves admit 2 297 897, and which this adapter's 16 384 × 16 384
scratch image therefore refused at 4×. Their ADR 0057 sizes a clipped mark's tile by the chain's
own box, and the page draws.

Three separate messages from upstream said so before this round — `doc/QUORRA_FEEDBACK.md`,
`doc/notes-release-matrix.md`'s three matrices in their tree, and the doc comment on
`REFUSED_AT_FOUR` itself, which a previous round wrote so that the name's departure would be
*expected* rather than rediscovered. **None of them took the name off**, and ADR 0402 decision 3
is why: a ratchet held by name exists so that a name comes off by a run. The
five-hundred-and-sixty-seventh session was told this name could go, ran the gate against
`eada81ec`, and left it on — which was right, because at that revision the refusal was the
correct behaviour of the revision this tree depended on.

This round is the run. `PDFVIEWER_QUORRA_SCALE=4` on the default lane failed first, with the
assertion naming the one element of the difference; the page then rendered on its own and
**agreed with the CPU oracle**, which is the check that matters — a page moving from *refused* to
*drawn wrong* would look identical in the refusal list. `REFUSED_AT_FOUR` is three names now.

**The general form is worth stating because this round is the third occasion of it.** An
upstream report is evidence about upstream. It can be accurate — this one was, to the page name —
and still not be a reason to move a ratchet, because the ratchet is not a record of what is true
somewhere; it is this tree's own last measurement. The message was accurate and it was not
sufficient, and those are two different sentences.

## 3. §10.7.4, on the lane the gate does not run by default

This is the release's other half and it is the one `doc/todo/02` §2's extra lane exists for.

quorra has two coverage lanes and this tree switches to the device one past
`GPU_COVERAGE_MAGNIFICATION`, which is ten times magnification. That lane counts samples on an
ordered grid, so the question it answers is *is this sample point inside the shape*. §10.7.4 asks
a different one — whether the shape meets the pixel's half-open square at all, no matter how small
the intersection — and states in the same paragraph what the difference costs: it is the rule that
keeps a shape from disappearing under unfavourable placement against the pixel grid. A mark
thinner than the grid's column spacing therefore vanished at some sub-pixel placements on that
lane, and was drawn heavy at others.

Their ADR 0070 sends such a mark down the processor lane instead, on a threshold that is the
grid's own column spacing rather than a chosen number. **This tree's corpus gate is where that
becomes a measurement**, and both revisions were run here side by side, in one sitting, on the
real Radeon 890M under RADV:

| lane, scale | `eada81ec` | `cad50156` |
|---|---|---|
| `cpu`, scale 1 (the default gate) | 932 / 23 / 2 / 17 | **932 / 23 / 2 / 17** |
| `gpu`, scale 1 | 930 / 25 / 2 / 17 | **932 / 23 / 2 / 17** |
| `cpu`, scale 4 | 937 / 11 / 4 / 22 | **938 / 11 / 3 / 22** |
| `gpu`, scale 4 | 938 / 10 / 4 / 22 | **939 / 10 / 3 / 22** |

*agree / differ / refused / not comparable.* The `gpu` scale-1 rows were `diff`ed line by line
rather than compared as totals: **exactly two lines leave and every other line is
character-identical**, `bug1883609.pdf` and `vertical.pdf`, both moving from differing with the
processor oracle to agreeing with it. At 4× on the same lane `issue12295.pdf` moves toward the
oracle without reaching it — mean 0.9517 → 0.9201, differing 0.0490 → 0.0473, similarity
0.95585 → 0.95881 — with its worst tile unmoved at 16.31, so a matrix of totals alone would have
called that row null.

**The counts converge and the sets do not, and that is the part worth keeping.** After the change
both lanes differ from the CPU oracle on 23 pages at page scale, and they are not the same 23: the
device lane alone differs on `bug1863910.pdf` and `issue16500.pdf`, the processor lane alone on
`bug1743245.pdf` and `issue21068.pdf`. Neither of the first two carries a moved line in either
column, so they are a residue the thin-mark condition does not reach rather than one it caused.
That is where a round wanting the two lanes to converge starts, and §10.7.4's ledger row now says
so.

**Why the two 4× rows below the default one are run at all**, given that the ratchets check one
lane: `doc/todo/02` §2's note says a release may land entirely inside a lane the default gate does
not exercise, and this one is the second demonstration of it. The default lane is
character-identical across the whole bump; the release's clause-level improvement is invisible
there and is two lines and a moved page on the other one.

## 4. The question the release asks, and this tree's answer

Upstream's `doc/api-change-image-alpha.md` puts one decision to this tree rather than taking it:
`SceneError` is not `#[non_exhaustive]`, so the next variant added to it would break a downstream
`match` that has no wildcard arm, and whether to mark the enum is "a decision neither side can
take alone".

**The answer is yes for `SceneError` and `RenderError`, and no for `SurfaceProblem`**, and the
asymmetry is the whole of it rather than a hedge.

- `SceneError` is reached here through one `#[from]` into `QuorraRasterError::Scene`
  (`crates/render-quorra/src/lib.rs`) and is matched nowhere. `RenderError` is matched in one
  place, `viewer-ui`'s `surface.rs`, with a catch-all arm that turns anything unrecognised into a
  `Refusal::DeviceRefused` carrying what quorra said. Both are open-ended vocabularies of things a
  backend could not draw, and this tree's handling of an unknown one is already right: report it
  by name. `#[non_exhaustive]` on either costs this caller nothing today and costs it nothing
  later, which is exactly when a compatibility marker is worth having.
- `SurfaceProblem` is the opposite and must stay exhaustive. That same `match` covers all five of
  its variants **with no wildcard**, deliberately: a swapchain state is not a refusal to report but
  a decision to take — `Outdated` and `Lost` ask for a redraw, `Timeout` and `Occluded` for
  nothing, `Validation` for a failure the person is told about. Its own module comment says its
  completeness "is not ours to argue", being one for one with `wgpu`'s non-success arms; so if
  `wgpu` grows a sixth, **the thing that should happen is that this tree fails to compile**, and
  `#[non_exhaustive]` is precisely what would take that away and leave a new swapchain state
  quietly falling into a catch-all.

The rule underneath: mark the enum whose variants a caller *reports*; leave exhaustive the enum
whose variants a caller *decides on*. This tree has one of each from the same library.

## 5. One stale claim, found by reading the brief the round was told to read

`doc/RENDER_LIBRARY.md` §4.1 ended: "`render-quorra` has not yet expanded `Command::Shaped` into
the pair, and until it does it refuses the command by name". `Scene::shaped` has emitted the
`DestOut` and `Plus` halves since the four-hundred-and-fifty-sixth session (ADR 0291) — **a
hundred and twenty rounds of that sentence being false**, in the one document this project hands
to the team writing its renderer. It is corrected, and the same paragraph now carries their ADR
0066's amendment to what `DestOut` is weighted by, because that is a change to a *specification*
this tree wrote and not only to a library it uses.

The shape of the miss is `doc/todo/01`'s fourth sweep exactly: a claim retired in the code and
left standing in a document. What made it survive is that `RENDER_LIBRARY.md` is read at the start
of a quorra round and written by nobody — the round that made the sentence false was not a quorra
round.

## 6. What this round did not do

- **No timing is published.** Two of the four lanes were run twice, the desktop was doing other
  work, and `doc/HANDOVER.md`'s rule holds — which pages refuse is arithmetic and
  machine-independent, which lane is faster is not.
- **The window is not exercised.** This release touches `present.wgsl` and the presenting layer's
  own rectangle (their ADR 0058), which no headless device reaches and which `doc/environment.md`
  puts in the owner's session. Nothing in this tree's gates can see it and this round did not
  pretend otherwise.
- **`quorra-pages` is not depended on.** A new member crate in their workspace; this tree names
  `quorra-gpu` and `quorra-scene` and nothing else, and there is no reason yet to name a third.
