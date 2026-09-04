# Q25 — Principle 2 states three things about this tree that are not true. Requirements still owed, or descriptions to correct?

Asked by round 921, sweeping the instruction files. **`Q24` is left for round 920**, which is
writing the amendment `doc/todo/59` §5 owes to principle 3.

## The question

`CLAUDE.md` principle 2 makes three statements in the present indicative that the tree does not
bear out. A round cannot tell from the wording whether each is **a requirement the project has not
yet met** — in which case the sentence stays and the debt gets a todo file — or **a description
that has decayed**, in which case the sentence is what changes. Only the owner can say which,
because principle 2 is theirs and rewriting it is not a sweep's business.

The three, with what the tree actually holds:

1. **"Perf gates run in CI: cold open, time-to-first-page, page-turn latency, memory high-water. A
   regression fails the build."** `.github/workflows/ci.yml` has seven jobs — `check`, `test`,
   `deny`, `nightly`, `platforms`, `snapshot`, `publish-snapshot` — and not one of them is a perf
   job. None of the four named measurements runs there and no threshold exists, so nothing can
   fail a build on one. The identical sentence is in `doc/PLAN.md`'s "Phase 4 — Test layers", where
   it is plainly a *plan*; principle 2 carries it as a fact.

2. **"Cold-start and time-to-first-page are CI gates with numbers attached, measured with a cold
   page cache. Targets are set once Spike A gives a real baseline, rather than invented now."**
   The gates do not exist (as above), and **Spike A is finished and was about something else**:
   `crates/render-cpu/tests/headless_render.rs`'s own header is *"Spike A: prove that a page can be
   rendered headlessly and reproducibly"*, three properties about determinism and the y flip, no
   performance baseline in it. So the sentence defers a target to an event that happened long ago
   and never produced one. The launch-path numbers this project does have are measured by hand and
   live in `doc/todo/42`, read back by nothing.

3. **"Parallelism (rayon) and GPU offload (vello/wgpu) are used wherever they genuinely help."**
   The rayon half is true. The parenthetical names the library the product no longer renders with:
   `vello` is `render-gpu`'s only, and `render-gpu` is reached by nothing that ships —
   `render-quorra`'s manifest takes it as a **dev-dependency** and says so. `viewer-ui` presents
   with `render-quorra` over `quorra-gpu`. `doc/stack.md` already reads "GPU first … `tiny-skia` as
   the correctness oracle", so the principles file is the one place still naming vello as the
   offload.

## Why this cannot be settled without the owner

Principle 2 is one of the five stated principles, and `CLAUDE.md`'s own instruction to a sweeping
round is that a principle reading false is a question rather than an edit. Items 1 and 2 in
particular are ambiguous in a way that matters: **deleting a perf gate the owner intended is
exactly the "revisit by attrition" the file forbids**, and leaving it as a description makes the
file lie to every round that reads it. Item 3 is smaller but is the same shape — correcting
"vello/wgpu" to "quorra" would be a round deciding, on its own, what the principle names.

## What the tree does meanwhile

- **Principle 2 is unedited.** This round changed nothing in `CLAUDE.md`.
- Performance is measured, but by hand and off the gates: `crates/pdf-model/examples/open_cost.rs`,
  `crates/render-quorra/examples/bring_up.rs` and `first_frame.rs`, with `doc/todo/02` §5's rule
  that **a stale binary is a measurement of the past** and §5's rebuild owed before any
  measurement. `doc/todo/42` holds the launch path's own numbers and its open items.
- The GPU path that ships is `render-quorra`; `render-gpu` is the cross-backend comparison, which
  `doc/state-of-play.md` and `doc/stack.md` both already say.

## Recommendation

1. **Keep the perf gate as a requirement and make it visible as owed**: leave principle 2's
   sentence, strike the dangling "once Spike A gives a real baseline" (that spike is closed, and
   the deferral is now unconditional by accident), and give the CI perf gate a todo file in the
   `40`–`49` band, priced against `doc/todo/42`'s existing measurements. That keeps the
   requirement, removes the false *description*, and puts the debt where a round can take it.
2. **Correct the parenthetical to name the shipped backend**: "(quorra/wgpu, with vello as the
   cross-backend comparison)" — one word of principle 2, and it is the only sentence in the
   instruction files still pointing a round at vello as the thing the product draws with.

Both are one-line changes to `CLAUDE.md`; neither is one a round should make on its own word.
