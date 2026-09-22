# 1209 — The face is the operator's, and so are the bytes a locator names

Status: accepted and **built**.
Context: `crates/pdf-transform/src/archive/{fonts,remedies,config,census,decision,prepare,report,mod}.rs`,
`crates/pdf-transform/src/bin/quorra-transform.rs`, `crates/pdf-font/src/standard.rs`,
`crates/pdf-transform/tests/archive.rs`.
Answers: `doc/adr/1200` section 4's recommendation (`--font` is a flag this tree described in
seven places and did not accept), and `doc/adr/1199` section 5's remaining route
(`tool = "resolve-external"`), with the enumerability wrinkle that ADR recorded.
Builds on: `doc/questions/A54` (`apply` returns a request, the caller performs the act),
`doc/questions/A47` (embed a shipped face by default, refuse rather than guess),
`doc/rfc/0007` section 5b.1 (`supply` carries an obligation the other kinds do not),
RFC 0002 section 9 (a pass is a pure function of its inputs).
Clauses: ISO 32000-2 §9.9.1, §9.9.2, §9.6.5, §9.2.4, §7.11.5, §7.3.8.2 (Table 5); ISO 19005-2
sections 6.2.11.4.1 and 6.1.7.1, ISO 19005-4 section 6.1.6.1.

## 1. The two things have one shape, which is why they are one ADR

Both are a fact the *file* does not carry and this program may not invent: whether a font program
may lawfully be embedded, and what bytes sit behind a locator the document wrote. Both were
already designed — ADR 1200 section 4 and ADR 1199 section 5 — and both deliver into a seam that
exists. So what is built here is a flag, a config word and a request population, and no new
mechanism at all.

## 2. `--font <base-font>=<path>`: the clause makes it a licence, and a licence is a person's

ISO 19005-2 section 6.2.11.4.1 admits only a program that may lawfully be embedded for unlimited,
universal rendering. Nothing in a PDF states that, and §9.9.1 says why in as many words:

> Font programs are subject to copyright, and the copyright owner may impose conditions under
> which a font program may be used.

> One of the conditions may be that the font program cannot be embedded, in which case it should
> not be incorporated into a PDF file.

A converter that went looking for a face on whatever machine it happened to be running on would
be asserting that condition on somebody else's behalf, which is what
`crates/pdf-transform/src/archive/fonts.rs` has argued against since it was written. An operator
naming a file is the opposite act: they state it, and the build's whole obligation is to record
that it was theirs. So `Conversion::supplied` gains a row per supplied face — RFC 0007 section
5b.1's `supply` obligation — and the output's own `xmpMM:History` names the face and says on whose
authority it is there.

**Nothing else about the route changes, deliberately.** `pdf_font::standard::supplied_face` builds
the same 256-code table `shipped_face` builds, by §9.6.5's own route through the document's
`/Encoding`; §9.9's Table 124 decides which key carries the program exactly as before; and
`pdf_font::restate` makes the program's advances the numbers the dictionary already states. That
last is the invariant, and §9.2.4 is why it is the invariant rather than a nicety:

> Storing this information in the font dictionary, although redundant, enables a PDF processor to
> determine glyph positioning without having to look inside the font program.

The dictionary positions the glyphs, so a supplied face changes what the outlines *look* like and
may not change where they *are*. `embed_faces` now proves that before it writes, through the same
two readers a caller holding nothing but bytes has, and
`a_supplied_program_keeps_every_advance_and_every_glyph_where_the_page_put_it` asserts it twice
over: the `/Widths` array is the producer's byte for byte, and every glyph origin in the page's
display list is where it was.

**The subset tag is passed over**, on §9.9.2's own terms: a subset's `/BaseFont` begins with "a tag
followed by a plus sign (+) followed by the PostScript name of the font from which the subset was
created", the tag is "exactly six uppercase letters" and "the choice of letters is arbitrary". An
operator cannot know which six a producer picked, and two subsets of one face in one file have
different ones. The exact name is tried first, so an operator who does write the tag gets what they
asked for.

**A supply is never a default**, which the shipped profiles keep: no profile states a font, and
`--font` naming a `/BaseFont` the document does not state changes nothing.

## 3. What `--font` closes in the corpus is *nothing*, and that is the finding

The measurement the build owed. Thirteen corpus documents refuse at
`fonts/font-programs-embedded`, and **not one of them is the case the flag was built for**: no
document refuses for want of repertoire, so a supplied DejaVu or Droid face — this machine has
both — closes none of them.

Twelve refuse on §9.9's Table 124 pairing the program's format with the dictionary's own
`/Subtype`: a `Type1` or `MMType1` dictionary takes a bare CFF program and the shipped sans face is
a `glyf`-based sfnt, and a `Type0` dictionary is a composite font this route does not reach at all
— §9.7.4.2 makes a CID an index into the glyphs of the font that defined it, so the program goes
into the *descendant* CIDFont's descriptor and the advances to restate against are §9.7.4.3's `/W`
and `/DW` rather than a `/Widths` array. The thirteenth states no `/FontDescriptor`, which §9.6.2.2
lets one of the standard 14 do and into which there is nothing to embed.

Two builds would close them and neither is this one: Table 124's `OpenType` row, which admits an
sfnt carrying a `CFF ` table under a `Type1` dictionary and needs that table read out of the
wrapper, and the composite route. **The messages were corrected rather than left**, which is the
same failure this ADR is closing one level down: `COMPOSITE_NOT_SUBSTITUTED` said `--font` resolves
a case it does not reach, and now says which build would.

## 4. `tool = "resolve-external"`: a request population and a config word

ADR 1199 section 5 predicted the shape and it held. A `preserve` row naming
`file-structure/no-external-stream-data` and a `[tool.…]` block is a `Resolution`; the conversion
returns one `ToolRequest` per stream whose data is outside the file, carrying **the file
specification the document wrote** as the request's input; the caller's executor runs the program;
and the bytes land in `ArchivePlan::external_data`, which is the entry `--resolve-external-data`
already fills. `external::embed` never learns who resolved them.

Three properties are the build's, and each is a clause or an answer rather than a taste:

- **The specification travels on stdin.** RFC 0007 section 4.1 keeps everything document-derived
  out of `args`, and §7.11.5's locator is the sharpest case of RFC 0007 section 4.5's warning —
  the operator's fetcher decides what a document's own string may reach.
- **`apply` still opens nothing.** `doc/questions/A54` and RFC 0002 section 9: the program travels
  as data, and a pass handed a recorded result is a pure function of its inputs.
- **The fetch is recorded in the file.** The rewrite writes the fetched bytes where the stream's
  own were, so nothing in the output says afterwards that they came from outside it. The report
  names the specification, the object, the resolved program and section 4.4's digest, and the
  packet's `xmpMM:History` carries [`FETCHED_BY_THE_OPERATORS_TOOL`].

`on-failure` takes only `stop`: a stream nobody fetched leaves its requirement refused, and the
alternatives — an empty stream, or a dropped one — put a document in an archive claiming to hold
bytes it does not.

## 5. The wrinkle: a `Mechanical` answer the table does not settle is still a site

ADR 1199 recorded it and this closes it. `config::sites` listed the requirements the census called
`Refused` or a `Loses` remedy, so once the external-data row became `Mechanical` the site dropped
off `--remedy-sites` although a URL-named stream still refused and `keep-everything.toml`'s answer
for it was still counted as not carried out.

The fix is a class rather than a special case, because there were two rows of this shape and not
one. `decision::CONDITIONAL` names the rows whose answer the decision table does not settle by
itself, and says what each waits on: `CallerSuppliesTheBytes` for this one, and `PerDocument(Loss)`
for ADR 1210's page boundaries, whose answer is mechanical for some files and an authorised loss
for others. `census::Kind::Conditional` is the standing, `config::sites` lists it, `--remedy-sites`
prints what the answer waits on, and `loss_sites` reads the per-document variant's loss so that a
`discard` at such a site authorises exactly that loss. Two rows made it a class; a third will not
need an argument.

## 6. What the sweep says

`archive_corpus` under the lock, before and after: **PDF/A-2b 471 converted and 139 refused
becomes 474 and 136**, all of it ADR 1210's. PDF/A-4 is 207 and 152 either way — part 4 states no
implementation-limits subclause at all, and the three streams that keep their data outside the file
there are URLs no corpus run configures a fetcher for. `tools/state.sh remedies` moves 2b from 58
of 77 sites not built to 57 of 78: the external-data site came back into the listing and the
page-boundary site gained a built `discard`.
