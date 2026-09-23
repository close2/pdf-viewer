# 1319 — An element is read under the `/AIS` it was painted under

Status: accepted and **built**. Session 1241.
Amends: ADR 1301 and ADR 1306 (the record of §11.6.4.3's readings, one per scope).
Depends on: ADR 0234, ADR 0415, ADR 1205, ADR 1256.
Context: `crates/pdf-model/src/content/transparency.rs` (`Readings`, `seal`, `unseal_list`,
`Interpreter::join_reading`), `content/{pattern,path,text,run}.rs`, `content.rs` (`finished`).
Clauses: ISO 32000-2 §11.4.6, §11.6.4.2, §11.6.4.3, §11.6.4.4, §11.7.4.4, §9.3.8.

`§N` is ISO 32000-2 and nothing else.

## Decision

§11.6.4.4: "the AIS ('alpha is shape') entry in a graphics state parameter dictionary shall
determine whether the alpha constants are interpreted as shape values ( true ) or opacity values (
false )". The entry is a graphics state parameter, so which reading applies is a fact of each
element. A scope's record (`Readings`) keeps one reading for what it holds by **sealing**: where
the scope turns from one reading to the other, every element painted so far that the flag
reinterprets becomes a `Command::Shaped` whose object is the element unchanged and whose shape is
`stated_shape` under the element's own reading. A sealed element is described by both readings,
so the record names the new one, and a knockout construction reads each stated shape as it is.

The statement decides nothing: `note_alpha_source` only puts a reading in force, and elements
reach the record when the next statement, scope close or stream end settles them
(`settle_readings_until`). An opaque, maskless element needs no reading, so a scope of such marks
under both readings seals nothing. The end of every nested stream is §8.10.1's restore and is
stated as one.

A nested group or tiling cell whose raster is not its own shape and whose content ran under the
other reading than its `Do` has its content sealed; its own constant and mask stay the `Do`'s. A
group's stated shape unions its elements by Over where any states a shape (`shapes_where_stated`):
a bare knockout draw weights by coverage, and §11.4.6's `(1 − f) × F + f` is Over's union.

## Why not the other two

- **A field on every mark**: 500-odd constructor and pattern sites across three backends, and
  every `raster_golden` list moves, for a quantity only `pdf-model` reads.
- **§11.6.7-style isolated wrapping**: exact only for an element under `/AIS true`, whose drawn
  alpha is its shape; an element under `false` in a `true` scope has no group whose raster is its
  shape. It costs a layer per element as well.

## The guarantee

`Command::Shaped` appears only as a direct element of a knockout group. A seal is made before
anybody knows whether it will be one, so `finished` (and a soft mask's run) takes every seal off
that did not become one (`unseal_list`), and only on a page that made one.

## Cost

Each element is sealed and moved once (`Readings::start` advances past what a seal covered). 4 000
fills alternating readings in one knockout group: 7.8 ms per interpretation against 5.5 ms for the
same page painted opaque. A page that never states `/AIS` walks nothing once its record names the
reading in force.

## What is left

An element whose shape cannot be stated at all keeps the record `Mixed` and is refused by name.
No route reaches one: `Paint` has two variants and both state a shape.
