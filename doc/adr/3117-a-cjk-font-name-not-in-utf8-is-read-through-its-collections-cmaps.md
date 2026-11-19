# 3117 — A `/BaseFont` that is not ASCII is read as characters: UTF-8 first, then its collection's encodings

Session 2104. Status: **accepted and built.** Context: ADR 3101 (the collection names a language), ADR 3102 (the
serifed CJK styles named), `crates/pdf-font/src/substitute/native_name.rs`, `substitute::folded_base_font` and
`family_of`. Clauses: ISO 32000-2 §7.3.5, §9.5 NOTE 5, §9.7.3 (Table 114), §9.7.5.2 (Table 116), §9.10.2.

## Context

`issue3521` names its font 黑体 (Hei, a sans style) in GBK bytes, `BA DA CC E5`, under a `CIDFont` of Adobe-GB1 whose
descriptor sets Table 121's Serif flag. The name was folded to its ASCII letters, which are none, so the flag decided
and the text was drawn by a serif face: a page that says "我们都是黑体字" ("we are all Hei characters") set in Song.

## The reading

§7.3.5 makes a name bytes ("a sequence of any characters (8-bit values) except null"), names `/BaseFont` as the case
where a processor reads one as text anyway, and says how:

> In such situations, the sequence of bytes making up the name object should be interpreted according to UTF-8, a
> variable-length byte-encoded representation of Unicode in which the printable ASCII characters have the same
> representations as in ASCII.

So a UTF-8 name is its characters, for any font: that is the standard's reading. A name that is not UTF-8 has departed
from a "should", and from there **the standard is silent** on what its bytes mean.

## Decision

1. **UTF-8 first**, for every font, as §7.3.5 asks.
2. **Otherwise, and only for a font naming a collection, a documented choice**: Table 116 ties each collection to a
   language and lists that language's byte encodings as predefined `CMap`s of the collection (GBK, Big Five and HKSCS,
   Shift-JIS and EUC-JP, Unified Hangul Code). The name is read through each — bytes to CIDs by the `CMap`, CIDs to
   characters by the collection's `-UCS2` table, §9.10.2's third method — and a reading counts only where every code
   maps and it names a style: Mincho, Song, Ming, Myeongjo or Batang as serif; Hei, Gothic, Dotum or Gulim as sans,
   in their own scripts. Two readings naming different styles say nothing. An Adobe-KR font is read through
   Adobe-Korea1's encoding, the only Korean byte encoding Table 116 lists.
3. **The name decides before the flags**, as a Latin name already does (`family_of`, ADR 1441): what the producer
   called the font outranks a bit producers set carelessly.

The composite font's own `/Encoding` is not read: `Request::derive` is handed the descendant, which states the
collection and not the `CMap`, and the archive's routes call it the same way, so the window and a conversion read one
name alike. Each `CMap` read is one Table 116 lists for the collection, and the encoding a producer names is among them.

## Consequences

`issue3521` draws in Noto Sans CJK SC, as its text says. Pages that move: those whose CJK font name is not ASCII and
names a style the flags contradicted. A romanised sans name (`SimHei`, `MS Gothic`) is not yet a sans request; the
Latin list holds the serif names only.
