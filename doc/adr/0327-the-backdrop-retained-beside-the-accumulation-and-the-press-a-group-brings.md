# ADR 0327 — The backdrop retained beside the accumulation, and the press a group brings

Date: 2026-08-14 (session 492)
Status: accepted

## Context

`doc/todo/23` had two open items left after ADR 0307, and both were priced there as
constructions rather than conditions:

- **§11.4.6's general knockout group** — non-isolated, elements that blend, and a knockout
  rule that can change a pixel. `issue18032.pdf` is the corpus witness: a `/I false /K true
  /CS /DeviceCMYK` group whose first element is composited under `/BM /Color` and whose
  second is invisible (`ca 0`) — the invisible element being the point, since under knockout
  its *shape* still replaces the accumulation with the initial backdrop.
- **§11.6.6's group blending colour space** — a group that *introduces* a space on a page
  that composites in another. `bug1721218_reduced.pdf` is the corpus witness: an isolated
  `/CS /DeviceCMYK` group covering the whole page, on a page that states no group at all.

This round built both, and a third thing neither item named but the first could not ship
without: `/AIS` was a page-wide monotone flag, and `issue18032.pdf` states it inside a form
whose group draws nothing — two forms before the knockout group the flag refused for it.

## §11.4.6 against a backdrop that is not transparent

The clause's two stages, per element:

> a) Composite the source object with the group's initial backdrop, disregarding the
> object's shape and using a source shape value of 1.0 everywhere.

> b) Compute a weighted average of this result with the object's immediate backdrop, using
> the source shape as the weighting factor.

For a non-isolated group the initial backdrop is the group's own — "[a] nonisolated
knockout group composites its topmost enclosing element with the group's backdrop" — so
stage a) is the element composited against the *page*, with its blend mode, opacity and
mask all applied there, and stage b) is `Pᵢ = (1 − fᵢ) × Pᵢ₋₁ + fᵢ × Eᵢ(B)` with `B`
retained beside the accumulation.

### The display list says it, the oracle draws it

`Command::Group` now admits `isolated: false` beside `knockout: true`, and `pdf-model`
emits the combination only where: the `Do`'s blend is Normal (the collapse of §11.4.4's
backdrop removal against §11.3.3's recompositing is the Normal blend function's — ADR
0237's cancellation, which knockout does not touch because both the removal and the
recompositing act on the group's *result*); no enclosing group is a knockout group (an
element of one is weighted by its own shape, which `Command::Group` does not carry); and
**every** element's shape is statable — `stated_elements` wraps each as a `Command::Shaped`,
including the ones whose shape is their coverage, because against a non-transparent
backdrop the weighted average's factor is needed per pixel for every element, not only
where shape and coverage diverge.

`render-cpu` keeps `B` (a copy of the page) beside the accumulation `P` (initialised to
`B`), and per element: draws the object ordinarily onto a scratch copy of `B`, draws the
shape half onto transparency, and applies stage b). Stage a) asks for a shape of 1.0
everywhere, which no drawing can produce for a group-valued element — but it does not have
to be produced, because it factors out. Expanding §11.3.6's premultiplied form at source
alpha `f × q` for the scratch and `q` for stage a)'s quantity `E`:

```text
f × E = S − (1 − f) × B        so        P' = (1 − f) × (P − B) + S
```

— exact for every blend mode and opacity, and `blend::knockout_average` states it with the
signed parenthesis clamped only at the end. The final composite onto the page is the
interpolation the non-isolated path already had.

`render-gpu` and `render-quorra` refuse the combination by name: neither a Vello layer nor
a `quorra_scene::GroupSpec` can retain a backdrop beside a layer's accumulation, and
quorra's staged `DestOut`/`Plus` pair is written on the transparent start §11.4.5 gives
(its ADRs 0025, 0032). The frame goes to the oracle, which is what `CLAUDE.md` keeps it
for.

### The closed form the tests hold

`test_scenes::knockout_group_on_its_own_backdrop`: opaque red page; an opaque blue element
under Multiply (`E₁ = red × blue = black` — black only because the blend saw the page); a
green element at opacity 0.3 (`E₂ = 0.7·red + 0.3·green = (178.5, 76.5, 0)`), whose shape
**replaces** element 1's black within it. Its edge sits at x = 30.5, so one device column
carries shape ½: `P' = ½·black + ½·E₂ = (89, 38, 0)` — where source-over gives `(0, 38, 0)`
and the transparent-backdrop pair draws element 1 blue. Both wrong constructions were
substituted in turn and both tests failed at those magnitudes before the fix went back.

## §11.6.6's group colour space: §11.4.7's pair, one scope down

§11.7.2:

> If the colour space of a graphics object within the group is not equivalent to the
> group's blending colour space, then it shall be converted to the group's colour space ,
> and all blending and compositing computations shall be done in that space (see 11.3.4,
> "Blending colour space"). The resulting colours shall then be interpreted in the group's
> colour space when the group is subsequently composited with its backdrop.

Both halves of that already existed — the conversion in per stated colour
(`Compositing::Subtractive`, `rgb_to_ink`), the conversion out as a sampled grid
(`Press::blending_space`) — and what was missing was the display list's vocabulary, exactly
as `doc/todo/23` priced it. `Command::Group` now carries `Option<Box<GroupBlending>>`: the
group's elements resolved in the black half of the space beside the command's own chromatic
list, plus the grid. `render-cpu` composites each list onto transparency, resolves the pair
per pixel (`blending::resolve`, the page-level function unchanged), and paints the result
once — which is where §11.7.2's second sentence puts the interpretation. The other two
backends refuse by name: the pair resolves *after* the group composites and *before* its
`Do`, and a scene under composition cannot be read back; the page-level trick — two whole
renders — has no group-scoped analogue.

### Which groups get the pair

`Interpreter::group_press` answers, and every condition is a clause's: the group is
isolated (§11.6.6 inherits every other case); the parent composites on the device (inside
a press page the space is restated, not introduced, and a *different* press inside one
would need a per-pixel conversion between two presses — kept as a report); the `/CS` names
four components this tree can sample, with `/DeviceCMYK` ranked through `/DefaultCMYK` in
the group's own resources (Table 145 subjects a device space here to the *current* resource
dictionary's remapping) and then §14.11.5's output intent, as the page does; no §8.6.8
figure supplies the colour from outside; no §11.7.5.3 black generation is stated; not a
knockout group (the staged rewrites edit the element list after the runs, and editing one
half of a pair would leave the other describing a different construction).

### Two runs, one readback

The group's content is interpreted twice, chromatic then black, and the readback — text,
placements, glyph counts, marked content — is rewound around every run after the first,
because a colour changes no glyph's place and a reader must not receive the page twice.
`ReadbackMark` records the accumulators; what is deliberately not rewound is `operations`
(the second run is real work the budget counts), the deduplicated reports, and the display
list's clip and soft-mask tables (the second run's commands reference their own entries).
The two element lists are then checked to have paired structurally — the group-scoped
analogue of the page pair's geometry digest, by lockstep comparison, with soft masks
compared by presence because the second run registers its own copies. A group in which
nothing composites is re-run once on the device instead: §11.3.4 cannot change a pixel of
an opaque Normal mark, which is the same condition the report fired on, so the pair's cost
is paid only where the clause can show.

## `/AIS` is a graphics state parameter, and now it is scoped like one

The flag that refuses knockout groups under §11.6.4.3's alpha source was set once and
never cleared, deliberately, on a ledger claim that no corpus document states the entry.
Nine do, and `issue18032.pdf` shows why the scope matters: `/AIS true` inside `q … Q` in a
form whose group draws nothing refused a knockout group two forms later. The entry now
lives in `GraphicsState` (both values read, `q`/`Q` bounding it as Table 57 says), and the
question a closing group asks — "was this in force while my content ran" — is seeded from
the state at the `Do`, monotone within the group's own run, and restored OR-ed into the
enclosing scope so a nested statement still reaches every enclosing group. A soft mask's
run restores it exactly, since a mask's content is an element of nothing. Within one scope
it remains an over-approximation, erring toward the report.

## Consequences

- `issue18032.pdf` and `bug1721218_reduced.pdf` — `doc/todo/23`'s last two corpus
  witnesses — leave the corpus's incomplete list; the todo file's own amendment carries
  the gate figures. `bug1721218_reduced.pdf` keeps a *smaller* report: its inner groups
  introduce a one-component `ICCBased` space inside the now-ink `/DeviceCMYK` group, which
  is a conversion between two presses this tree still does not have.
- Two new display-list shapes reach the backends, each refused by name on the two
  non-oracle backends and each with a refusal test beside the drawing tests.
- `Command::Group` gains a field, which is a workspace-wide construction-site change; the
  quorra translation's exhaustive destructure caught it at compile time, the `..` patterns
  in the other two backends were guarded by hand and tested.

## Alternatives considered

**A per-pixel conversion between two presses**, which would draw the remaining §11.6.6
shapes (a three-component group inside a four-component page, and `bug1721218_reduced`'s
gray-in-CMYK). Not taken: the conversion out of the inner space lands in the *parent's*
components per pixel, which is `rgb_to_ink` run per pixel through an ICC curve — a cost and
a construction (per-pixel search, or a second sampled inverse per press pair) that eight
web documents and no corpus page's dominant group justify today. The reports stay, named.

**Emitting the pair for every isolated press group**, compositing or not. Rejected on trap
11's arithmetic sideways: the pair doubles interpretation and buys nothing where nothing
composites — an opaque Normal mark carries its colour through whatever space carries it —
and the rounding of resolve's divide-by-alpha would move pixels on pages the clause says
cannot differ.

**Keeping `/AIS` monotone and reporting `issue18032` for it.** Rejected because the
refusal's own condition was not the clause's: the entry is a graphics state parameter, and
a statement `Q` has restored reaches nothing painted afterwards. Narrowing it is trap 11's
direction — the condition comes from the clause — and the report it feeds still fires
wherever the flag was actually in force.

**Substituting §11.4.4's construction for the non-isolated knockout group** — right for
the first element, wrong for every overlap — was already rejected in ADR 0307 and stays
rejected: this project does not draw "closer".
