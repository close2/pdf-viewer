# Q25 — Principle 2 states six things about this tree that are not true. Requirements still owed, or descriptions to correct?

Asked by round 921, sweeping the instruction files. **`Q24` is left for round 920**, which is
writing the amendment `doc/todo/59` §5 owes to principle 3. **Items 4 to 6 were added by round
925**, which built the instrument that measures the startup rules and then profiled what it
found; items 4 and 5 are round 922's findings (ADR 0885) filed here where they can be seen, and
item 6 is round 925's own (ADRs 0890 and 0891). They are the same question about the same
principle, which is why they are here rather than in a `Q26` of their own.

## The question

`CLAUDE.md` principle 2 makes six statements in the present indicative that the tree does not
bear out. A round cannot tell from the wording whether each is **a requirement the project has not
yet met** — in which case the sentence stays and the debt gets a todo file — or **a description
that has decayed**, in which case the sentence is what changes. Only the owner can say which,
because principle 2 is theirs and rewriting it is not a sweep's business.

The six, with what the tree actually holds. **Items 1 to 3 are about the perf gates and the
stack; items 4 to 6 are about the *Startup time is a first-class requirement* subsection, and
every one of those three is a flat prohibition that the subsection's own general rule two bullets
below already qualifies.**

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


4. **"A 500-page document must open no slower than a 5-page one."** False by about fifty times,
   measured (`tools/state.sh launch`, ADR 0885, and the profile in ADR 0890). It is false of the
   *open* and true of the *launch* — time to first page is within 10% across a 5-page and a
   1023-page document, because the graphics device takes longer than either — so the sentence
   names the wrong one of the two things it could mean.

   **And no amount of laziness rescues it**, which is the part a round could not decide alone.
   The open's three costs are linear in three different populations: the entries the
   cross-reference sections state (247 against 112 269), the pages the tree holds, and §12.3.3's
   outline items. Deferring everything that is not needed to draw the page still leaves the
   cross-reference table, and §7.5.6's precedence makes reading all of it the price of reading any
   of it — so the ratio falls from about 49× to about 19× and the sentence stays false. ADR 0891
   has the arithmetic.

5. **"No system font enumeration."** False without a condition: a page naming a font it does not
   embed sends `pdf_font::substitute` through the machine's font directories on the launch path —
   23 files opened under `share/fonts`, 47 more directory listings than a document whose fonts are
   its own, and about twice the time to first page (ADR 0885, and the gate's fourth row).

   **Session 920's resource port changed who enumerates, not whether.** A confined worker asks by
   description and the *broker* walks the directories, so the cost is still on the launch path and
   now copies the face's bytes across a pipe as well; what did change is that a host which offers
   nothing gets a worker with no machine fonts, so the sentence is true in that posture and false
   in the other. That makes it conditional on the host as well as on the document.

6. **"No full page-tree walk."** False for every document that has an outline, and this is the
   one item here that no earlier round had found. `Viewer::announce_page` puts the caption's
   section name on the launch path; §12.3.3's `Outline::section_at` resolves every item's
   destination to a page number; and doing that one at a time is a tree walk apiece, so it builds
   `Pages::indices()` — **every node of §7.7.3's tree, resolved.** On ISO 32000-2 that is 41% of
   what opening the document costs, and it is the largest single item in the open. ADR 0885 read
   this bullet against `Pages::new`, which takes `/Count` and does not walk; that is true and is
   about a step two functions earlier.

**What items 4 to 6 have in common, and it is the shape of the question.** Each is a flat
prohibition, and each names something that *is* needed to show page one for some document: a
substitute font, and the section name in the caption. Principle 2 states the general rule two
bullets below them — "[a]nything **not needed to show page one** is deferred until first use
(`OnceLock`, not startup)" — and under that rule none of the three is a violation at all. So the
question for items 4 to 6 is narrower than for 1 to 3: **are the flat prohibitions meant as
instances of the general rule, in which case they want its qualifier, or are they meant as
absolutes, in which case they are three pieces of owed work and the caption's section has to stop
being computed at open?**

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
- **All six sentences now have a number and a command that prints it**, which they did not when
  this question was written: `tools/state.sh launch` is the gate session 922 built, and
  `cargo run --release -p pdf-model --example open_cost -- <file>` is the step-by-step profile
  session 925 completed. Nothing waits on this answer — the tree is measured either way — and no
  round has amended `CLAUDE.md`.
- **Round 925 took the half of item 6 that costs nothing**: the page-tree walk was being paid
  again on *every page turn*, and the map is a function of an immutable file, so `Open` keeps it
  (0.77 ms off every arrow key on ISO 32000-2, ADR 0890). What it did **not** do is take it off
  the open, because that means the caption gains its section on a second `Event::PageChanged`
  that every host would have to expect — a host-visible change that does not make item 4's
  sentence true, and so a change to be asked for rather than made.

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

3. **For items 4 to 6, say whether the three prohibitions are absolutes or instances.** The
   cheapest answer that keeps every requirement is to leave the general rule as the requirement
   and let the three read as its examples — "no system font enumeration *at startup*", "no full
   page-tree walk *to show page one*", and an open sentence that says what it means: **the launch
   must not scale with the document**, which is true and measured, rather than the open, which
   cannot be made so. If instead they are absolutes, item 6 is owed work with a design already
   priced (ADR 0891) and item 4 is owed work nobody knows how to do.
