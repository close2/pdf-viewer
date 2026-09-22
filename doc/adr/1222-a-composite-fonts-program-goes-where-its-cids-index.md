# 1222 — A composite font's program goes where its CIDs index

Status: accepted and **built**.
Context: `crates/pdf-transform/src/archive/fonts/composite.rs` (new),
`crates/pdf-transform/src/archive/{fonts,rewrite}.rs`, `crates/pdf-font/src/embedding.rs`,
`crates/pdf-transform/tests/archive.rs`.
Answers: `doc/adr/1209` section 3's second remaining build — the composite route, "where the program
goes into the *descendant* CIDFont's descriptor and the advances to restate against are §9.7.4.3's
`/W` and `/DW` rather than a `/Widths` array".
Builds on: ADR 1221 (Table 124 as one function), ADR 1188 (what a `/CIDSystemInfo` may not be
rewritten into), `doc/questions/A47`, `doc/rfc/0007` section 5b.1.
Clauses: ISO 32000-2 §9.7.4.2, §9.7.4.3, §9.7.5.2, §9.7.6.2, §9.7.6.3, §9.9, §9.9.1, Tables 115,
124 and 125; ISO 19005-2 sections 6.2.11.4.1 and 6.2.11.8, ISO 19005-4 sections 6.2.10.4.1,
6.2.10.5 and 6.2.10.9.

## 1. Three things are different, and none of them is "wider codes"

- **Where the program goes.** §9.7.4.2 makes a CID an index into the glyphs of the font that
  defined it, and Table 115 makes the descendant's `/FontDescriptor` the one describing the
  CIDFont. A `/FontFile` written into the Type 0 dictionary's own descriptor would describe
  nothing, there being none.
- **What the advances are.** §9.7.4.3's `/W` and `/DW`, indexed by CID. What is restated is still
  the *program*, for §9.2.4's reason, so the invariant ADR 1209 proved is the same invariant here:
  the dictionary's numbers are the producer's byte for byte and the program is asked again through
  the same readers a caller holding nothing but bytes has.
- **What turns a code into a CID.** The CMap. That is why this build takes the two encodings the
  standard defines outright, `Identity-H` and `Identity-V`, where §9.7.5.2 makes the CID the code:
  for any other CMap the CIDs belong to a character collection, and whether a program an operator
  named defines *that* collection is nowhere in the file.

## 2. The programs it writes are the ones whose CIDs are glyph indices

Two clauses say the same thing about two formats, and together they are the population this embeds:

> The "CFF" font program has a Top DICT that does not use CIDFont operators: The CIDs shall be used
> directly as GID values, and the glyph procedure shall be retrieved using the CharStrings INDEX

and Table 115, of `/CIDToGIDMap`: "If the value of `CIDToGIDMap` is a name, it shall be `Identity`,
indicating that the mapping between CIDs and glyph indices is the identity mapping." So one rule —
the CID *is* the glyph index — covers a name-keyed bare CFF, a name-keyed `CFF ` inside an sfnt, and
a `glyf` sfnt under the identity map, and the coverage check is a single comparison against the
program's own glyph count.

**A CID-keyed program is refused by name rather than written**, and the reason is a clause rather
than effort. §9.7.4.2 makes such a program identify its character collection with a `CIDSystemInfo`
"which should be copied into the PDF `CIDFont` dictionary", and reaches its glyphs through the
program's charset. Embedding one under a dictionary whose `/CIDSystemInfo` names a different
collection would leave the file asserting a collection its program does not define — and correcting
a `/CIDSystemInfo` is what `doc/pdf-a-mitigations.md` keeps as *none* for
`fonts/cid-system-info-agrees-with-the-cmap` (ADR 1188). The refusal names what resolves it, which
is a program whose CIDs are its glyph indices.

## 3. Table 115's entry becomes owed the moment a program arrives

`/CIDToGIDMap` is "(Required for Type 2 CIDFonts with embedded font programs)". A descendant that
stated none while its program was elsewhere owes one as soon as this conversion writes a program
in, so the rewrite writes `Identity` — which is what the CIDs of an `Identity` CMap were already
being read as, and what the coverage check above was computed under. It is the only value written.

**A `/CIDToGIDMap` the producer already wrote is a refusal**, on §9.7.4.2's own sentence: where the
font program is not embedded the entry "shall be ignored, since it is not meaningful to refer to
glyph indices in an external font program". So such a map is a table no reader has used, written for
a program that was never in the file, and embedding one would turn it on over glyphs it was not
written for. This converter embeds under the identity mapping or not at all.

## 4. The one table a supplied program loses

§9.9.1, of a TrueType program: "If used with a `CIDFont` dictionary, the "cmap" table is not needed
and shall not be present, since the mapping from character codes to glyph descriptions is provided
separately." The mapping provided separately is the `/CIDToGIDMap` above, so nothing a conforming
reader consults goes with the table — but a `/FontFile2` written with one would be a program the
clause forbids.

`pdf_font::embedding::without_tables` is the removal, and it is a rebuild rather than an edit: the
directory's own length is what every table's offset is measured from, so dropping an entry moves
every table after it. Each kept table's bytes are the program's, copied; what is written afresh is
the directory, its search fields and — through the routine `restate` already uses — every checksum,
because a file whose layout this changed would otherwise state sums taken over a layout it no longer
has.

## 5. What the font dictionary is asked for, and what it is not

`LoadedFont::metrics_only` rather than `LoadedFont::load`. What embedding needs from the dictionary
is where one code ends and the next begins and what §9.7.4.3 says the displacement is, both of which
`metrics_only` reads with no glyph anywhere. Loading the font outright asks a second question —
whether this reader can *draw* the composite font as the file stands — and §9.7.5.2 makes the answer
to that *no* for every `Identity` encoding whose program is missing, which is the whole population
here. Asking the question that is not being answered would have refused every document this build
exists for.

## 6. The refusals, each with its clause

A CMap that is not `Identity-H` or `Identity-V` (§9.7.5, §9.7.4.2); a descendant this reader cannot
find or whose descriptor is written inline rather than as Table 115's indirect reference; a CID-keyed
program (section 2); a producer's own `/CIDToGIDMap` (section 3); a CID past the program's last glyph
or CID 0, which §9.7.6.3 makes the analogue of `.notdef` and both parts' `.notdef` clause forbids
showing; a vertically set font whose supplied program states `vmtx`, which ISO 19005-4 section
6.2.10.5 would then require to agree with `/DW2` and `/W2` and which this restates across the line
and not yet down it.

## 7. What it closes, measured

The corpus's composite refusals at `fonts/font-programs-embedded` are four documents — two at
PDF/A-2b and their two twins at PDF/A-4 — and **all four now convert when `--font` names a program**,
with the output held to its target again and every requirement met. `6-2-11-4-1-t01-fail-e.pdf` is a
`CIDFontType2` under `Identity-H` with `/CIDToGIDMap /Identity`, and takes a `glyf` face as
`/FontFile2` with its `/W` and `/DW` untouched; `6-2-11-4-1-t01-fail-d.pdf` is a `CIDFontType0`, and
takes a bare name-keyed CFF as `/FontFile3` with `/Subtype /CIDFontType0C`.

**That second one is worth being plain about**: the face named is not Kozuka Mincho, and the
conversion did not check that it was. `--font` is a statement an operator makes about a program, not
a guess this converter makes about a face, and what the clause requires of the *file* — that its CIDs
index the program it carries — holds either way. The report and the output's own `xmpMM:History` name
the program and say on whose authority it is there, which is the whole of what this program can
honestly assert about it.

The default sweep is unchanged, and has to be: no corpus run supplies a font.
