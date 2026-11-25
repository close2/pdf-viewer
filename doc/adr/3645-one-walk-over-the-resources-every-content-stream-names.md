# 3645 — One walk over the resource dictionaries every content stream names

Session 2236. Status: accepted. Context: review 2234 (B01, B02), ISO 32000-2 §7.8.2 and §7.8.3 as
Errata Collection 3's Issue #128 restates it, ADRs 0793 and 1342.

## Decision

`pdf-model`'s `resource_walk::ResourceWalk` is the one enumeration of the resource dictionaries
beneath a root — a page's resources (optionally followed by each annotation's `/AP` `/N`), or a
soft mask's group — that a reader asks *before* the content streams are run. From each dictionary
it follows §7.8.2's list: a form `XObject`'s `/Resources`, a tiling pattern's, and a Type 3 font's
glyph descriptions' own `/Resources` and then the font's (Issue #128's search order). It yields each
dictionary once, depth-first in document order (`/Pattern`, `/XObject`, `/Font`).

Three readers stand on it and walk nothing of their own beneath a dictionary:
`optional_content::groups_referenced_by` and `groups_referenced_by_content` (B02: patterns' cells,
Type 3 glyphs and annotation appearances now reach Table 99's `VisiblePages` list),
`colourants::spot_colourants` (B01: a `Separation` in a glyph's own resources gets a plane), and
`content::resources_blend` (B01: a blend mode stated there is seen).

**Bounds.** Every indirect object (form, pattern, font, glyph, appearance, shared resource
dictionary) is entered once, which ends a cycle; and no dictionary deeper than
`content::MAX_FORM_DEPTH` (64) is read, because the interpreter draws nothing nested past it — so
the bound cuts nothing that is drawn. `truncated()` says when it cut. It replaces two depths of 8
(the layers walk and the blend walk) and the spot walk's absence of one. `resources_blend` keeps
its 256-dictionary cost guard and answers `true` at it or at a cut, the safe direction ADR 1342
gives it.

**Soft masks are not followed.** A mask's group is reached through `/ExtGState`, not drawn as a mark;
§11.7.3 withholds spot colours from it, and the blend question is asked per mask's group by
starting a walk at it.

## The other pre-drawing walks, and why each stays its own

A census of `crates/*/src` found eight more walks over `/Resources`. None moves onto this one:

- `pdf-archive`'s survey (`survey.rs` and its `pages`, `xobjects`, `patterns`, `fonts` modules) and
  `content/image.rs`'s walk-ahead are interpreters: they tokenise streams and need the resources in
  force at each operator, not a set of dictionaries.
- `pdf-archive/src/reach.rs` (names a stream states or lends) and `pdf-transform`'s
  `archive/fenced.rs` (every object an appearance reaches, including `/SMask /G` and halftones)
  enumerate *objects*, a wider and different population with soft masks inside it.
- `structure/owners.rs`, `archive/parent_tree.rs`, `pdf-transform/src/images.rs` and
  `archive/default_rgb/page_contexts.rs` read per-container data (structure keys, image
  descriptions, a form's `/Group` or `/OC` that refuses) along a subset of the routes their own
  clause names, with budgets their callers price.

A walk that only needs "every resource dictionary a page can draw from" uses `ResourceWalk`.

## Cost

The layers and spot answers can list more than before (a pattern's, a glyph's or an appearance's
groups and colourants); both were upper bounds already and over-listing is their safe direction.
Spot planes are now ordered by a pre-order over resource dictionaries, which differs from the old
interleaved order only on a page whose nested dictionaries name spot colourants alongside its own.
