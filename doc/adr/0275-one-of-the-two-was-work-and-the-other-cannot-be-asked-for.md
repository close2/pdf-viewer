# ADR 0275 — One of the two was work, and the other cannot be asked for

Date: 2026-08-11 (session 439)
Status: accepted

## Context

Session 438 moved the quorra pin to `89d7dd77` and found that two of this tree's three open asks
had been answered by it (ADR 0274). Both answers left `render-quorra` refusing a display list, and
both refusals stopped being requests to somebody else:

- **§14** — `quorra_scene::Compose::DestOut` and `Plus` exist, weighted by shape rather than by the
  paint's alpha, which is exactly what `pdf_render::Command::Shaped`'s second member is. The
  backend still refused `Shaped`, and its message was corrected to say the translation was
  unwritten *here*.
- **§17** — two `Target::Readback` renders against one device work, are cheap, and
  `quorra-gpu/tests/two_rasters.rs` now holds it. That is what sessions 426–427's four-component
  page needs: §11.3.4 composites per component, so the page is drawn twice with a different three
  loaded.

This round wrote both translations. **One of them is in the tree and the other one is not
writable**, and the second half is the finding.

## Decision 1 — §11.4.7's four components go through quorra, as two renders of one device

`QuorraRasterizer::rasterize` renders the display list and, where `DisplayList::blending` is
`Some`, renders `DisplayList::black` against the same device before `pdf_render::blending::resolve`
puts the two together. Both passes go through one private `QuorraRasterizer::render`, which is the
whole of the decision worth stating:

> Separate from `Rasterizer::rasterize` because §11.4.7's four-component page is two lists over one
> page and both take exactly this path — the same device, the same caches, the same per-frame
> releases — and nothing about drawing one of them may depend on which of the two it is.

The recombination sits **before** `impose_on_medium`, because that is where the clause puts it:
§11.4.7 converts "the entire result" to the device's space and *then* composites it with the
backdrop the medium supplies. Both rasters are premultiplied for the pair of passes that follow and
demultiplied once at the end, because `blending::resolve` divides each channel by the pixel's own
alpha to recover the ink and `impose_on_medium` composites where the composite is exact. The round
trip is skipped entirely where neither is needed, since it is lossy at a partly transparent pixel.

**Nothing else changed**, which is the point: no new scene vocabulary, no second raster format, no
special handling on the second pass. Everything §17.1 promised held on the first run.

### What it draws, measured

`test_scenes::four_component_page` is the fixture, and its expected pixels come from the clause
rather than from a golden file. Four ink marks on one page, and two of them decide something:

| mark | inks | what it is worth |
|---|---|---|
| half registration black over paper | all four at ½ | **(76, 66, 64)** — per component the pixel is ½ of each ink, and the multilinear interpolation of the cube at (½,½,½,½) is the mean of its sixteen corners |
| process black alone | k = 1 | **(35, 31, 32)** — this mark is *white* in the chromatic raster, so it is the one that says the second render happened |

The control is the same two marks converted first and composited on the device's three components,
which is **127.5** — 51 of 255 away, and the whole reason the clause is implemented rather than
approximated. `render-cpu`'s `four_component_page.rs` establishes both as the oracle and
`headless_quorra.rs` asserts the same two on the graphics device; `render-gpu` refuses the fixture
by name and now has a test that says so, which it did not before.

### The corpus gate

| | agree | differ | refused | not comparable | median page |
|---|---|---|---|---|---|
| session 438 | 914 | 35 | 8 | 17 | 2.41× |
| now | **917** | 35 | **5** | 17 | 2.37× |

The `differ` list is held to equality by the gate in both directions and did not move. Three names
leave `REFUSED`: `personwithdog.pdf`, `bug1365930.pdf` and `issue12798_page1_reduced.pdf`.

**The pages were looked at, not only counted** (trap 1). `personwithdog.pdf` is a shaded figure and
dog with soft drop shadows, `bug1365930.pdf` is a page whose only mark is the numeral 201, and
`issue12798_page1_reduced.pdf` is a magenta banner with white text on it. All three are
indistinguishable from the CPU oracle's render by eye, at 0.0288, 0.0093 and 0.0760 mean, with page
inks 20.3953/20.3659, 0.8637/0.8634 and 16.9057/16.9117. The third is the page ADR 0274 found
carrying two refusals stacked; its second one has now gone too.

The whole-run rasterisation cost through quorra went 8.09 s → 8.51 s over 957 pages, which is three
pages drawn twice out of 952 drawn once. The median ratio moved 2.41× → 2.37×, inside the
run-to-run band this gate has always had, and is not evidence of anything.

## Decision 2 — §11.4.6's two marks stay refused, and the refusal names what actually blocks

**The translation was written out and it does not compile into a scene.** Two independent reasons,
both established by running rather than by reading:

1. **`SceneBuilder::fill` refuses a staged operator inside a knockout group** —
   `StagedComposeUnsupported { compose: DestOut, reason: InsideKnockoutGroup }` — and a knockout
   group is the *only* position `Command::Shaped` occurs in. That is not an accident of this
   tree's interpreter; the command's own documentation states it as a guarantee, because outside a
   knockout group §11.4.4's formulas reach the shape only through shape × opacity and a backend may
   draw the object alone. So the one position the pair is needed in is one of the two the builder
   declines.
2. **`SceneBuilder::group`, `stroke` and `image` carry no `Compose` at all.** Only `fill` does. And
   three of the four corpus pages behind this refusal — and this tree's own fixture — state a
   `Shaped` whose two halves are **groups**, because §11.6.4.2 makes a nested group's shape the
   union of its elements'. Only `knockout_smask.pdf` is a fill-and-fill pair, so even lifting the
   first would take one page of four.

`doc/QUORRA_FEEDBACK.md` §14.1 recorded both refused positions as "nothing this side emits". That
was a claim about *this tree's display lists*, made on this side of the boundary, without building
one — and the round that built one found it false. The correction is ours, not quorra's: the
operators do exactly what their documentation says.

### What stays, and how it is held

The refusal message no longer says the translation is unwritten here. It names both obstacles:

> a knockout element whose shape is not its coverage: quorra's Destination-Out is refused inside a
> knockout group, which is the only place this element occurs, and a group mark carries no
> compositing operator at all (ISO 32000-2 §11.4.6)

and `headless_quorra.rs::quorra_will_not_take_the_pair_where_this_tree_would_hand_it_over` asks the
builder for that exact call and asserts the refusal. It is a test rather than a paragraph for one
reason: **it fails the day quorra lifts the restriction**, which is the notification the next round
wants. The second obstacle cannot be a test at all — a missing parameter does not compile — and is
recorded in that test's doc comment instead.

### Why not route around it

Two routes were considered and both are worse than the refusal.

**Set `knockout: false` on the group and state §11.4.6 elementwise**, giving every element an
explicit `Compose::Src` and the `Shaped` ones the pair. `inside_knockout()` would then be false and
the first obstacle would lift. It fails on the second — the group-halves are still unsayable — and
it costs something even where it works: quorra's encoder routes a mark with a non-Normal blend
through an implicit blend group **when the enclosing style is Over**, and skips that entirely
inside a knockout group, which is right because §11.3.6 gives a blend against a transparent initial
backdrop no effect. Telling the library a knockout group is not one would silently change what a
*blended* element in that group draws. That is a plausible wrong picture for other pages bought
with one page of four.

**Draw one of the two marks.** `Compose::Plus` alone drives a premultiplied channel past its alpha
and the library states the pairing as the caller's obligation. Half of the pair is worse than
neither.

So the answer is the one trap 5 asks for: refused by name, with the name saying what it is waiting
for.

## Consequences

- `render-quorra` draws §11.4.7's four-component page. **`render-gpu` is now the only backend that
  differs about which colour space a page composites in**, and its refusal has a test.
- `doc/QUORRA_FEEDBACK.md` §17 is **closed** — it asked one question, the answer was yes, and the
  work it named is done. §14 gains **§14.2**, which is an ask with a correction of this side's own
  reading in front of it, and re-offers §14's original alternative (a per-element shape channel)
  as the design that would answer both obstacles at once.
- `doc/todo/23`'s last backend row keeps its four pages and states why they cannot move here.
- The ledger rows for §11.3.4, §11.4.6 and §11.4.7 were read against the code, which is `doc/todo/02`
  §1's spec-track helping, and **two of the three said something false**: §11.3.4 still opened with
  "[t]he blending colour space is the device's three components, always" thirteen rounds after that
  stopped being true, contradicting its own last two sentences; §11.4.6 said `render-quorra` "draws
  neither form of it", where it draws the coverage form through `GroupSpec::knockout` and refuses
  only the stated-shape one. Both are corrected. The clause-11 quotations are clean under
  `tools/spec-errata`.
- `cargo clippy --workspace --all-targets` was **not** silent at `1c3e00a` — one `doc_markdown`
  warning in `render-quorra/tests/corpus.rs`, on a word session 438 added. Fixed here; it is worth
  noting because §2's sequence lists that gate second and it had been left red for a round.
