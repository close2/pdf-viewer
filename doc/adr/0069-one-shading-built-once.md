# ADR 0069 — One shading object, built once

Status: accepted, 2026-07-31.

## Context

ADR 0068 took `bug1721218_reduced.pdf` from 144.05 G instructions to 54.05 G by handing the
rasteriser the stops it cannot compute for itself. What was left of that page, in order, was
`tiny_skia::pipeline::lowp::gradient` at 29%, **`Function::parse` at 6.7%**, `Mask::intersect_path`
at 6.5%, `build_soft_mask` at 6.4%, and — a little further down — `Function::eval` at 4.1% and
`pdf_model::shading::ramp` at 3.2%.

That trio is one thing: building a shading's colours. And the handover has carried a question
about it since the forty-sixth session — whether the page's shadings "are 3576 distinct functions
or one re-parsed". The sixty-fifth session answered it by instrumenting the *pattern* path,
counted eight shadings, and concluded that "parsing was never the cost".

**That measurement was taken on the wrong path.** Instrumenting `Function::parse` itself counts
**7000+ calls**, and instrumenting both call sites shows where they come from: the pattern path
runs **once** and the `sh` operator runs **3576 times**. The page paints one gradient object
thousands of times, and every painting parsed its functions again — for a type 0 function, that
means inflating a stream and decoding 4096 samples — and then evaluated them 256 times to build a
`Ramp`.

Nothing in any of that depends on *where* the shading is painted.

## Decision

**A shading's colours are a property of the object that states them, and are built once per
object; only its placement is built per painting.**

`pdf_model::shading::Cache` holds `Arc<ShadingKind>` and the shading's own matrix, keyed by the
`ObjectId` the resource dictionary names. `Cache::build` returns a `Shading` sharing that kind and
composing the caller's transform onto it. `pdf_render::Shading::kind` became an `Arc` for the same
reason: a mesh's triangles must not be copied per painting, so the sharing has to reach the
display list rather than stopping at the model.

The `Interpreter` holds one cache per page, beside the font cache and for the same reason.

### The key is an identity, and the one thing that is not

A colour space stated as a **name** is resolved through the `/ColorSpace` subdictionary of the
resource dictionary in force (§8.6.5.1), and even the device names go through §8.6.5.6's
`/DefaultGray`, `/DefaultRGB` and `/DefaultCMYK` in that same dictionary. So one shading object
under two resource dictionaries can legitimately be two different sets of colours, and an
identity is not a sufficient key for it.

**Those are not cached at all.** The refusal is exact rather than approximately right, it costs
nothing measurable — the file this ADR is about states its spaces as arrays, which is what a
`Separation` or an `ICCBased` space has to be — and it is the case
`a_named_colour_space_is_resolved_against_the_resources_that_paint_it` builds: one shading object
painted from a page that inks `/Space` red and a form that inks the same name blue. That test was
confirmed to fail when the refusal is removed.

Reaching the identity took one other change: `Interpreter::resource` resolved a reference before
returning it, which throws away the only thing that says two paintings are of one object.
`resource_entry` is the unresolved spelling, and `shading::Cache` is its only caller.

## Consequences

Measured with `callgrind_rasterise` on `bug1721218_reduced.pdf`, page 1:

| | before | after |
|---|---|---|
| whole page | 53.96 G | **43.13 G** |
| `Function::parse` | 3.61 G (6.7%) | gone |
| `Function::eval` | 2.23 G (4.1%) | gone |
| `pdf_model::shading::ramp` | 1.72 G (3.2%) | gone |
| `calloc` | 2.20 G (4.1%) | 1.93 G |

**A fifth of the worst page in the corpus, and nothing else.** Over the whole corpus the change is
invisible where it should be: `hayro-speed` measured 6.92 s before and 6.91 s after over the 858
pages we draw completely, in one sitting — because this page reports a soft-mask departure and is
not one of them. Over all 946 pages it is 8.19 s to 7.55 s, and essentially all of that difference
is this page.

Both pixel gates are unchanged to the verdict: 836 agreements, 65 contradictions, 749 ambiguous.
That is the property a cache has to have, and it is why the test that proves the cache *works*
asserts pointer identity rather than a number — a cache that changed a pixel would be a defect,
so no measurement of the output can show that it is being used.

## What this does not do

- **It does not cache across pages.** The cache lives in the `Interpreter`, which is per page, so
  a document painting one gradient on every page still builds it per page. Nothing measures that
  today; a document cache is a lifetime question about `Document` rather than a lookup.
- **It does not cache a failure.** A shading that cannot be built is rare, is deduplicated by the
  report, and remembering the error would mean deciding whether it is a property of the object or
  of the moment.
- **It does not touch `Function::parse` itself**, which is still linear in a sampled function's
  table every time something outside a shading asks for one — a `Separation` tint transform in
  `colour.rs`, a soft mask's `/TR`. Those are per-colour-space and per-`gs`, not per-painting, and
  neither shows in any profile this project has taken.
