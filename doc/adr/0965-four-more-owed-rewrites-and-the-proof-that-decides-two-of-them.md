# 0965 — Four more owed rewrites, the proof that decides two of them, and four catalogue entries that were wrong

Session 962. Status: **accepted**. It **continues ADR 0957** over the eleven of
`doc/pdf-a-mitigations.md` §13.3's twenty-two that were left, moves **four** more rows out of
`decision.rs`'s `REFUSED_BY_NAME`, and — the part worth keeping longer than the code —
**corrects four catalogue entries that were wrong about the standard or about the work**, one of
which would have produced a file whose vertical text had moved.

Context: `crates/pdf-transform/src/archive/{decision,prepare,rewrite,sites}.rs`,
`crates/pdf-transform/tests/archive.rs`, `doc/pdf-a-mitigations.md` §13.3 and §13.3.1,
ISO 19005-2 6.2.11.6, 6.4.2 and 6.6.2.1, ISO 19005-4 6.2.10.5, 6.2.10.6, 6.4.2 and 6.7.2.1.

## The four built

Two of them are one line of the standard each, and two rest on a proof this converter had never
made.

### `metadata/xmp-packet-header-attributes` — and a rewrite that was declared and never written

The deprecated `bytes` and `encoding` attributes come out of the packet's `<?xpacket … ?>`
processing instruction, every other byte of the producer's packet crossing unchanged. Both
describe the packet's *own framing*, so nothing a reader of the metadata uses is in them.

Two things the building found that the catalogue did not say:

- **`Rewrite::PacketHeaderAttributes` already existed**, with a full doc comment, a `describe`
  arm and a `word` arm, added in session 957 — and nothing referenced it, no `REMEDIES` row
  answered with it, and the function its doc comment named,
  `pdf_model::xmp::without_deprecated_header_attributes`, did not exist. A variant that names a
  function nobody wrote is the shape `CLAUDE.md`'s "no placeholder implementations" is about, and
  it survived a round because an unused enum variant is not a compile error. It is now wired, and
  the cut lives in `archive::prepare` rather than in `pdf-model` — the change→gate map makes a
  `pdf-model` edit *everything*, and this cut is the converter's business rather than the
  reader's.
- **A packet takes up to three edits and they had no order.** The header cut, section 6.6.2.3.1's
  property removal and section 6.6.4's identification schema all rewrite the same bytes; two of
  them already composed by hand in `prepare_metadata`. `Edited` is now that order as a type — the
  header first, the removal into what it left, the schema into what *that* left — because a writer
  reading the producer's original instead silently undoes the writer before it.

The cut declines three shapes rather than guessing at them: a packet with no `<?xpacket` in the
window the validator searches, a header padded with the NUL bytes one of ISO 16684-1's wide
encodings writes, and an attribute whose value has no quotation marks to cut to. And it reads the
header back afterwards, which is `prepare_properties`'s own rule: the writer says what it cut and
the packet says what it holds, and only the second is what a validator sees.

### `forms/no-needs-rendering` — the half of a two-sentence subclause whose answer the base standard prints

ISO 19005-2 6.4.2 forbids two entries: the interactive form dictionary's `/XFA` and the catalog's
`/NeedsRendering`. They were refused together, under one sentence, and they are not one question.
§7.7.2's Table 29 says what the second is:

> ( Optional; deprecated in PDF 2.0 ) A flag used to expedite the display of PDF documents
> containing XFA forms. It specifies whether the document shall be regenerated when the document
> is first opened. See Annex K, ' XFA forms ' . Default value: false .

Deprecated, about XFA, and `false` by default — so removing it states what an absent entry states.
**What makes it lossless rather than nearly so is the requirement beside it**: a document whose
form dictionary still states an `/XFA` fails `forms/no-xfa-key`, which stays refused, so no file
this rewrite reaches is one where a reader had a form to regenerate. The dependency is written
into the rewrite's doc comment, because a later round that builds the `/XFA` removal has to know
that this row was resting on its refusal.

### The two TrueType encoding rows, and the proof

ISO 19005-2 6.2.11.6 asks a non-symbolic TrueType font to name `MacRomanEncoding` or
`WinAnsiEncoding`, and a symbolic one to name no encoding at all. The failing entry is in the font
dictionary rather than in the program, so ADR 0816's fence does not stand in the way — and the
catalogue said so, and then said the rewrite "owes a **proof**, per font and per code used, that
the program's own tables already agree", because §9.6.5.4 makes the encoding decide which `cmap`
subtable a code is looked up through.

The proof is now made, and it is the cheapest possible one: **load the font twice**. `LoadedFont`
takes a `&Dictionary` and a `&Document`, so the candidate is a clone of the font dictionary with
the entry written or removed, loaded against the same document; `glyph_index` is then asked of
every code, and the rewrite happens only where the two agree everywhere. Nothing re-reads
§9.6.5.4: the procedure is the font reader's, asked twice.

Two decisions inside it are worth the words.

**The denominator is the codes the content streams showed, not all 256.** That is this tree's own
precedent read twice over — `archive::fonts` restates the advances of the glyphs a font *showed*
and `archive::to_unicode` derives a `CMap` over the codes a content stream *showed* — and it is
what the catalogue asked for. It is also the difference between a rewrite that works and one that
never fires: measured on the shipped Liberation Sans, `StandardEncoding` and `WinAnsiEncoding`
reach different glyphs at **80** of the 256 codes, because a full Latin face resolves the upper
half of both. A proof over all 256 would refuse every real non-symbolic font in existence and
would have been indistinguishable, from the outside, from not having built the rewrite at all.

`pdf_archive`'s `SelectedFont::shown_complete` is honoured: its own documentation says a rule
asking whether *every* shown code is sound may not read a truncated list, and this is exactly such
a rule, so a font whose strings outran the survey's budget is refused.

**A font with no program of its own is refused rather than proved.** Where `pdf-font` substituted
a face, what the codes draw is not a fact about the document, so the comparison would be this
reader's substitution against itself.

The survey those shown strings come from is now made **once** for the four preparations that need
it, rather than inside `prepare_fonts` for two of them: the walk reads every content stream, and a
second one would double the cost of converting a large document to answer the same question twice.

## Four catalogue entries corrected

`doc/pdf-a-mitigations.md` §13.3.1 carries these too, because that is where a reader of the
catalogue looks. Two of them are the standard, and two are the work.

### 1. The vertical metrics entry had the writing direction backwards, and would have moved marks

The entry proposed restating `/DW2` and `/W2` from the embedded program's own vertical metrics,
"on the same argument that already justifies the horizontal case". The horizontal case restates
the **program**, and for the reason that makes the other direction unsafe: §9.2.4 makes the font
dictionary's numbers what a processor positions glyphs by without looking inside the program, and
§9.7.4.3 gives `/DW2` and `/W2` that role going down the page. Restating them moves every glyph on
a vertical line. Restating the program's `vmtx` moves nothing.

The entry was also wrong about what the requirement waits on: it said "this tree's font reader
handing back vertical metrics", and `pdf_font::LoadedFont::program_vertical_advance` already does
— `pdf_archive`'s own predicate for this row is built on it. What is missing is the **writer**:
`pdf_font::restate` rewrites an sfnt's `hmtx` and a charstring's leading width operand and nothing
vertical. The row stays refused, with a sentence that now names the right half of the tree.

### 2. The two duplicate-profile rows are not mechanical, and their site is not named

The entry called naming `DeviceCMYK` in place of an `ICCBased` space duplicating the output
intent's profile "mechanical and lossless … the only work is establishing that each use of the
space can take the substitution". Two things stop it, and the first is decisive.

- **The failure is reported where the content stream selected the space.** ISO 19005-4 6.2.4.2's
  last requirement binds a space that is *used*, so `pdf_archive`'s population is its survey's
  selections — and the `cs`/`CS` operator path records `Where::page(index)` with no object at all.
  The array to rewrite sits in a resource dictionary no finding names. Siting the rewrite means
  walking the content streams again to decide which space was used, which is the validator's
  reading made a second time in this crate — the drift `CLAUDE.md` principle 5 and
  `archive::sites`'s own module comment exist to prevent.
- **Even sited, it would not be a restatement.** §8.6.7 applies non-zero overprint mode only to
  operations "when the current colour space is `DeviceCMYK` (or is implicitly converted to
  `DeviceCMYK`)", so the substitution can decide a composite §8.6.5.7 leaves open — which is
  precisely the ambiguity 6.2.4.2's own NOTE 2 gives as the reason for the prohibition. A rewrite
  that resolves an ambiguity the standard names is at best ADR 0948's `Stated`, never `Mechanical`.

### 3. The two JPEG 2000 box rows: the cost was right and the value was never settled

The entry's finding — these two fields live in the JP2 wrapper rather than the codestream, so
meeting the clause is "a rewrite of a hundred-odd bytes that touches no sample" — is true and is
about the *cost*. It says nothing about the **value**, and every available value is a choice:

- A `METH` outside the three the part admits describes this image's colour in a way the part does
  not read. Writing one of the three in its place states a colour space the box did not.
- "Exactly one shall be marked as the best available" cannot be met by restatement where the file
  marks none: `APPROX` is the ranking, and choosing which of the producer's own specifications is
  the more faithful is evidence the file does not carry. Dropping the others throws one away.
- And the row is not only a box rewrite. Its second sentence requires the ICC profile of the
  *selected* specification to conform to the base standard, which is the profile-replacement case
  and not byte surgery at all — the entry did not mention it.

Both rows stay refused, with sentences that say the answer is a choice rather than an afternoon's
work. **That is the correction: "not built yet" and "not decided" are different facts**, and the
catalogue had filed the second under the first.

### 4. The colourants entry: a PDF function cannot call another one

The entry said the open question — how a single colourant's tint transform is derived from an
*N*-input one — "is arithmetic rather than policy". It is not arithmetic, because §7.10 gives a
PDF function no way to call another: the derived §8.6.6.4 `Separation` needs a one-input function
whose values are the producer's *N*-input function restricted to one axis, and the general way to
get one is to **sample** the producer's function — an approximation of their definition written
into an archive as though it were their definition.

One shape could be exact and is the thing to build first: a §7.10.2 sampled transform already
states its values on a grid, so the samples along one axis are the producer's own numbers rather
than a re-approximation of them. The refusal now says that, which turns an unbuilt rewrite into a
specified one.

## What was not built, and why that is the honest answer

Of the eleven, **four are built**, **five turn out not to be losslessly buildable** and are now
refused with the argument above rather than with a debt, and **two are still lossless rewrites
nobody has written**:

- `fonts/vertical-metrics-agree-with-the-program`, whose entry is corrected but whose rewrite is
  owed — by `pdf_font::restate`, one crate over, where a `vmtx` writer would go beside the `hmtx`
  one.
- `graphics/one-destination-profile-per-output-intents-array`, untouched, and the one to take
  next: unlike the duplicate-profile pair its findings *do* name the offending output intent's
  object, so the rewrite is sited, and the only question it has to answer is whether two entries'
  destination profiles are the same bytes — where they are, pointing both at one object loses
  nothing, and where they are not, the entry itself already calls that the group's only `discard`.

**A promise nothing keeps is worse than a refusal with a sentence**, which is why the five
corrections count as much as the four rewrites. §13.3's claim was that twenty-two refusals were
"waiting on code, not on a decision". After two rounds of building it: fifteen were, five were
waiting on a decision after all, and two are still waiting on code.

## Consequences

- `tests/archive_unconsidered.txt` is still empty and still held to equality in both directions;
  the census reports **0** requirements with no considered answer at any of the six targets.
- Fourteen target-document pairs in `doc/veraPDF-corpus` fail the four requirements now answered
  (six the non-symbolic encoding row, four the packet header, two each the symbolic encoding row
  and `/NeedsRendering`), so none of the four is dead code by construction.
- `archive::prepare`'s `Prepared::of` grew an ordering it did not have; `Already` and `Edited` are
  the two types that carry it, and both exist because the order is load-bearing rather than for
  tidiness.
