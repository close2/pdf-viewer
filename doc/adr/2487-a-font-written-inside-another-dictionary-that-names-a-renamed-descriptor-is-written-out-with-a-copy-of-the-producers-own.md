# 2487 — A font written inside another dictionary that names a renamed descriptor is written out with a copy of the producer's own

Status: implemented. Session 1826, the archive slot of batch one hundred and twenty-three. Context:
ADR 2476 (a font that is an object keeps a copy; this was its named limit), ADR 2162 (a font
written directly inside its resources becomes an object, and the rewriter puts a reference where
the dictionary stood), `doc/history/1821` ("found, not fixed").

## The defect

ADR 2476 points every font **object** that names a renamed descriptor, and that the conversion left
its own `/BaseFont`, at a copy of the producer's descriptor. A bystander written directly inside
another dictionary — a resource dictionary's `/Font` entry, a Type 0 font's `/DescendantFonts` array
— has no object number, so the walk skipped it and it kept the renamed descriptor. That is worse
than the object case: a `CIDFontType2` whose descriptor now holds a program and whose dictionary
states no `/CIDToGIDMap` fails `fonts/cid-to-gid-map-present`, and the conversion **declines to write
the file** (the planted document below: "the output would still fail 1 requirement(s):
fonts/cid-to-gid-map-present").

## The reading

Tables 109, 110 and 115 say of the `/FontDescriptor` entry that it "shall be an indirect
reference", so the entry that must change is the direct dictionary's own: there is no way to point
a direct font at a copy except by changing the dictionary that holds the reference. ISO 32000-2
§7.3.10 makes a direct and an indirect spelling of a value one value:

> Except where documented to the contrary, any object value may be a direct or an indirect
> reference; the semantics are equivalent.

Table 119 asks of `/DescendantFonts` an array "specifying the CIDFont dictionary", and nothing in
the resource dictionary's `/Font` subdictionary documents the contrary. The dictionary may therefore
become an object of its own, which is ADR 2162's construction for a shown font.

## The decision

`bystanders::keep_their_own_descriptors` also walks every object's values (a stream's dictionary
included, to a depth of 32) for dictionaries whose `/FontDescriptor` is a renamed descriptor. Each
one the conversion does not write itself (not a key of `Substitutes::direct`) is written out as an
object of its own with the copy of the descriptor in its `/FontDescriptor` (one copy for all the
bystanders of an object, shared with the object-valued ones), and recorded in `Substitutes::direct`
so that `Rewriter::reference_direct_fonts` puts a reference to it where the dictionary stood. Equal
dictionaries are one font by §7.3.10 and share the object, as ADR 2162 has it.

## Proof

- `tests/archive/rekeyed.rs::an_inline_font_that_shares_a_rekeyed_descriptor_keeps_the_name_it_states`:
  a Type 0 font nothing shows whose `CIDFont` is written inside `/DescendantFonts`. With the walk
  disabled the conversion is declined (`fonts/cid-to-gid-map-present`) and the test fails; with it
  the output holds the font with a descriptor stating its own name, unembedded, beside the shown
  font's embedded one, and Table 120 holds for every font dictionary.
- veraPDF 1.30.3 passes the planted document converted at PDF/A-2b and PDF/A-4.
- The census (`archive_unbuilt` over pdf.js and `doc/corpora`, `--tree 6`) is line for line the
  pre-change one: no corpus document has such a bystander.

## Consequences

- The limit ADR 2476 named is gone, and `doc/pdf-a-mitigations.md` says so.
- The walk costs a recursion over every object, once, and only where a route renamed a descriptor.
