# 650 — The curve a ramp had already thrown away

§10.5's transfer function reaches a shading's colours now. **Where** it is applied turned out to be
the whole of the work: the price the round before this one wrote down would have drawn the
commonest shading in PDF wrong, and the reason is a simplifier that had been exactly right for
seven hundred rounds.

Date: 2026-08-22.
ADR: [0479](../adr/0479-the-curve-a-ramp-had-already-thrown-away.md).

Touched: `crates/pdf-model/src/shading.rs`, `src/mesh.rs`, `src/content.rs`,
`src/content/ext_gstate.rs`, `src/content/pattern.rs`, `src/content/transparency.rs`,
`src/content/report.rs`, `src/content/path.rs`, `src/content/text.rs`, `src/content/image.rs`,
`crates/pdf-model/tests/transfer_functions.rs`,
`crates/pdf-model/examples/transfer_function_census.rs`,
`doc/conformance/ledger.toml` (§10.5, §8.7.4.4, §11.7.5.2), `doc/todo/13`, `doc/todo/README.md`,
the ADR and this file.

## What §10.5 says a transfer function maps

Read whole from `doc/md/`, and it is a rule about a *component* that never names an object:

> In the sequence of steps for processing colours, the PDF processor shall apply the transfer
> function after performing any needed conversions between colour spaces.

> The input shall be the value of a colour component in the device's native colour space, either
> specified directly or produced by conversion from some other colour space. The output shall be
> the transformed component value to be transmitted to the device (after halftoning, if necessary).

> Each colour component shall have its own separate transfer function; there shall not be
> interaction between components.

That places it twice over. *After the colour-space conversion* puts it downstream of
`Conversion::paint`, which is where every shading colour in this tree is born; *the value
transmitted to the device* means the colour at a point, so there is no reading on which a ramp's
stops are exempt for being a ramp. §10.1's list of rendering steps orders it against the rest of
clause 10 — convert, transfer, **then** scan convert — and a shading is scan converted from the
colours the display list hands over, so the function has to be inside them before they get there.

`spec-errata emit` over all fourteen documents, for clause 10 first as the round was told: **no
erratum touches §10.5, §11.7.5 or §8.7.4.1–8.7.4.5.7.** The nearest in clause 10 are §10.4.1,
§10.4.2.4, §10.6.5.4, §10.6.5.6 and §10.7.2, and §8.7.4.5.8 has one asking for a stray green line
to be removed from a figure. Confirmed rather than assumed, which is what 637 asked the next round
to do.

## Every route a shading's colour takes, and which lacked it

Enumerated by 637's own method — list the call sites, then read what is not on the list — over
`pdf-render`, `render-cpu`, `render-gpu` and `render-quorra`. There are seven, and **all seven
lacked the function**:

| the colour | who reads it |
|---|---|
| `Stop.colour` of an axial or radial `Ramp` | all three backends |
| `Ramp::colour_at` at the ends, for `/Extend false`'s transparent stop | cpu, gpu |
| `Ramp::colour_at` inside `RadialRaster::build`, §8.7.4.5.4's cone | all three |
| `Ramp::colour_at` inside `Triangle::paint`, for `Corners::Parameters` | all three |
| `Corners::Colours`, a mesh's stated vertex colours | all three |
| `ColourGrid.pixels`, a type 1 shading's grid | cpu, quorra (gpu refuses the kind) |
| `ShadingProgram`, §7.10.5 lowered to GPU instructions | quorra only |

Six of the seven are `Color` values `pdf-model` put in the list, so mapping them at build time
reaches every backend at once — trap 2's rule satisfied by construction rather than by three
matching edits. The seventh is not a colour at all: quorra evaluates the instruction list on the
device, so §10.5 has nowhere to go on that path and the program is **withdrawn** when a transfer is
in force, exactly as `Shading::with_alpha` already withdraws it for §11.6.4.4's constant alpha.

A shading inside a soft mask's group and a shading inside a tiling pattern's cell add no new route:
both replay the same three construction sites.

## The decision, and the number that made it

ADR 0469 priced this as `Shading::with_alpha`'s walk done again with a closure, in `pdf-render`.
**That is right for alpha and wrong for a transfer.**

A `Ramp` is not the shading's colour function; it is a sampling of it, and ADR 0068's `simplify`
then drops every stop within half an eight-bit level of the line its neighbours draw — lossless,
because both rasterisers interpolate linearly between the survivors. So a `/FunctionType 2`
interpolation with `/N 1`, which is most of the shadings that exist, reaches the display list as
**two stops**. Map those two and the rasteriser draws the chord between them; the clause asks for
the transfer of the colour at each point. Under a transfer that squares its input the ramp's
midpoint is 0.5² = 0.25 and the chord gives 0.5 — **64 levels of 255**, on the commonest shading in
PDF, from an implementation that would have passed every other test in this tree.

So the function is applied *inside the sampling*, in `shading::kind_of` and `mesh::read`, where the
ramp becomes a sampling of the composition and the simplifier then measures the colours that will
actually be drawn. **`pdf-render` needed no line of it.** The general shape, which is worth more
than the clause: *a lossless simplification is lossless with respect to the operations that were
already going to be applied* — insert a new one downstream and it stops being lossless, silently,
in the direction of "smoother than the file says".

Two consequences are carried with their reasons: a shading built under a transfer is **not cached**
(a `Transfer` is a group of parsed functions with no identity a key could use, which is the answer
`shading::Cache` already gives a named `/ColorSpace`), and the device program goes.

**And the lint found the design smell before a reader did.** Adding the third parameter took four
functions to eight arguments; rather than a third `#[expect(too_many_arguments)]`, the three became
`shading::Colouring` — §10.7.3's resolution, §8.6.5.9's conversion, §10.5's transfer — and every one
of those functions is now shorter than it was before the round started.

## The population, measured before the code and over both corpora

`examples/transfer_function_census` gained a third figure, the witness page's index, and rayon; it
now takes the SafeDocs crawl in **46 seconds**.

- `doc/pdf.js`: **964 open, 13 state a Table 57 `/TR` or `/TR2`, 1 states a real one.**
- The crawl: **65 703 open, 1352 state one, 32 state a real one.** A hundred times the pdf.js
  population, and nobody had asked it before.
- **Documents that paint a shading while a real one is in force: zero, in both.** Five crawled
  documents state a real transfer function and paint a shading *on the same page* — that is the
  page-level over-approximation the census can compute — and all five render byte-identically with
  the mapping in and out (`6100014.pdf#2`, `1899545.pdf#0`, `4482198.pdf#0`, `4605986.pdf#0`,
  `6942037.pdf#13`). The exact condition, run as a probe over all 32, matches none of them.

So **no row was added to `doc/checks/fixed-documents.toml`**: nothing was fixed, because nothing was
broken that any file on this disk exercises. The five fixtures are the whole of the evidence
(trap 8), and saying that plainly is better than hedging — the argument for the change is the
clause, and the argument for this *placement* of it is a number no corpus here can produce.

## What was built

`tests/transfer_functions.rs` gained five tests and lost one. Each was run against the defect it
guards (trap 13) and **the calibration was done twice**, which is what separates this design from
the other one:

- with the mapping removed altogether, all five fail;
- with the mapping done ADR 0469's way — over the finished, simplified ramp — four pass and
  `a_ramp_is_the_composition_and_not_its_endpoints` fails, printing `0.5 is not 0.25`.

The fifth test is the residual: a shading **pattern** selected under one `/TR` and painted under
another. §8.7.2 makes a pattern a colour and `scn` is where a colour is set, so a shading pattern's
colours are resolved at the selection and the mark may be several graphics states later. It is
**reported** (`Painted::of`'s `stale`, by `Arc::ptr_eq`, over-approximating in the safe direction)
rather than closed, because the `scn` is also where this tree resolves §8.6.5.9's black point and
§11.4.7's compositing target for those same colours and neither of *those* says a word — so it is
three clauses' question, and `doc/todo/13` now prices it as one.

## The ledger

§10.5 stays `partial` and the content of the `partial` inverts: from "a shading's colours never
reach the function, and that half was never named" to "they all do, and what is left is the pattern
resolved a state too early, which is reported". §8.7.4.4 gains the sentence that bounds a mesh's
accuracy — a rasteriser interpolates between three transferred corners where the clause would have
the transfer of the mixed colour, which is that clause's own "some subset of the points" and not a
departure invented here. §11.7.5.2 gains one: its over-approximation **widened**, because a shading
painted under a transfer now counts as a mark carrying one, where before it did not.

## Gates

Fifth round, and a change that can move a pixel, so the whole of `doc/todo/02` §2 ran — **three
other rounds were building beside it, load average between 16 and 42**, and every line came back
green, so no line needed the quiet re-run 626's rule reserves for a red one.

`fmt` clean. `RUSTFLAGS="-D warnings" cargo clippy --workspace --all-targets` exit 0 — it caught
two things and both were real: the argument count above, and a `panic!` in a new test that owed an
`#[expect]`. `RUSTFLAGS="-D warnings" cargo check --manifest-path fuzz/Cargo.toml --bins` exit 0.
`cargo nextest run --workspace` **2385 passed / 17 skipped**; doctests clean.

Corpus **974 documents, 68 incomplete**. Oracle **1794 pages: 908 agree, 65 contradicted, 786
ambiguous, 2 our geometry, 2 reference geometry, 13 not comparable, 18 no render**. Text extraction
**10969/11163 matched words in bounds (98.26%)** over 508 documents, 486 fully in bounds.
`selection_census` 0 panics; `accessibility_census` 57 116 caret elements, 876 of 876 untagged pages
answering the honest empty tree, 0 defects; `dates` **1545 strings, 1514 conforming (97.99%)**;
`xmp` green; `jpeg2000` green. `render-quorra` **957 pages: 933 agree, 22 differ, 2 refused, 17 not
comparable**. `fixed_documents` **35 checked, 0 absent**. `cargo test -p conformance` green — **875
rows, 0 unreviewed, 972 verbatim quotations**, breakdown 436 implemented, 222 partial, 18 reported,
78 inapplicable, 8 writer-side, 113 out-of-scope, **no `silent` row**.

**And the claim trap 1 asks for, made with the instrument built for it**: `examples/raster_digest`
over the whole corpus on both arms — the tree as it ships, and the tree with the mapping neutered —
is **byte-identical over all 957 first pages**. Nothing this round drew moved, which is what the
population said would happen and is why `doc/todo/00`'s step 7 is not owed.

Sweeps run because the ledger moved: `quotations` — 1722 ledger quotations, **1 diverging, and it is
§8.9.5's and was there before**; `counts`, `tables` and `pointers` printed their standing false
positives and no new hit. §5's binaries rebuilt and installed.
