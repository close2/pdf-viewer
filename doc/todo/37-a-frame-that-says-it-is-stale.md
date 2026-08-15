# A frame that says it is stale — feedback while the real one is still being built

Status: **asked for by the project owner, reluctantly, and that reluctance is part of the
specification.** Their words: *"even though I was hoping we could avoid it completely … When we
are fast enough, we can print every frame correctly. But when not, we currently don't give any
feedback."* Nothing is built.
Priority: 37 — capability: the program cannot do this at all
Witness: `tmp/Entwurf.pdf` — one page, 58 009 commands, **not in the repository**, so no test may
name that path. Its zoom step is the case: ~640 ms in which the window shows the old view
unchanged and nothing says why (ADR 0368's attribution; 74% of it is quorra's encode).
Instrument: the window's own `--trace` frame line, which already names every phase; a reprojected
frame must be legible there rather than inferred.
Clauses: none — this is presentation, not a reading. §10.7.4 does not reach it, because nothing
reprojected is a *rendering* of the page.
Code: `crates/viewer-ui/src/bin/pdf-viewer/surface.rs` (the presenter), `crates/render-quorra`
(where ADR 0297 already keeps a raster in the window's backend), and **not** `viewer-core`

## What it is

When a view changes — a zoom step, a scroll — the frame for the new view takes as long as it
takes. Today the window shows the *previous* view's pixels, unmoved, until the new frame lands: a
stall with no feedback, and on a heavy page that is most of a second. A stale-frame reprojection
takes the raster already on the screen and transforms it to where the new view puts it — the same
pixels, moved and scaled — so the window answers the input immediately, and replaces it with the
real frame the moment that frame exists.

It is an approximation by construction: a raster scaled up is blurred, a scroll reveals an edge
the old raster has no pixels for, and anything the new view would draw that the old one did not is
simply absent.

## The hazard, stated first because it is the reason the owner hoped to avoid this

**This project's first principle forbids drawing something plausible instead of something true**
(`CLAUDE.md` principle 1; trap 5 — "unsupported input must stay loud", and its whole point is that
a viewer must not quietly show a wrong picture). A reprojected frame *is* a wrong picture, shown
deliberately. That is defensible only if every one of these holds, and a round taking this owes
all of them:

1. **It is visibly transient, and it is never the last word.** The real frame must always follow,
   and the reprojection must never be the state the window settles in. A reprojection that is
   still on the screen when the machine goes idle is a defect, not a degradation.
2. **Nothing that judges a picture ever sees one.** The oracle, the corpus gates, `Query::Frame`,
   the confined worker's raster, `render_at`, the headless harness and every artefact a person
   diagnoses from must be the real render. This is the sharpest rule: an instrument that
   accidentally photographs a reprojection is an instrument that lies, and it is exactly the
   shape of trap 1's archetype. The reprojection therefore belongs to the *presenter*, on the
   path that has a window, and to nothing else.
3. **It says so.** The trace's frame line names it, and the count is available to a host. A
   reader of the trace must never have to infer that a frame was approximated.
4. **It costs the real frame nothing.** No work is taken from the frame being built to produce
   the approximation; if the reprojection cannot be produced within a small fraction of the frame
   it replaces, it is not produced at all.
5. **It does not fire when it is not needed.** A view whose frame is ready in a few milliseconds
   must show that frame and never an approximation — which means a threshold, and the threshold
   is a *measurement* rather than a taste (`CLAUDE.md`: nothing arbitrary replaced by something
   equally arbitrary).

## What it is not

- **Not a substitute for the frame being fast.** ADR 0368 measured the zoom step and found 74% of
  it inside quorra's encode; `doc/QUORRA_ENCODE_THREADS_ANSWER.md` divides its geometry phase
  6.6× on this page's shape. Reprojection is what covers the residue that is left after the frame
  is as fast as it is going to get, and the owner's sentence — *"when we are fast enough, we can
  print every frame correctly"* — is the standing intent: **the better this gets, the less this
  feature should ever be seen.**
- **Not progressive rendering.** Drawing a partial page as it is interpreted is a different
  feature with a different argument (`doc/todo/16`'s road C names it), and it draws only marks the
  file states.
- **Not a message on `viewer-core`'s boundary**, unless the design genuinely requires one, in
  which case `doc/ui-boundary.md` holds the test it must pass. The presenter already knows when a
  frame was requested and when it arrived; that is where the knowledge is.

## What a round taking this owes

- The five rules above, each with the thing that enforces it — a test, an assertion, or a type,
  not a comment.
- A measured threshold, and the measurement.
- The witness looked at: a zoom step on the witness with the reprojection on, photographed
  mid-flight (`doc/environment.md`'s Xvfb recipe), showing that the window answered the input.
- Every gate identical, because nothing on any judged path may change.
