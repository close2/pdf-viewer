# 1099 — The annotation cannot stay, and its marks need not go

Status: accepted. Session 1085.
Context: `crates/pdf-archive/src/table/interaction.rs`
(`annotation_subtype_permitted`, `annotations_of_a_forbidden_subtype`, `ForbiddenSubtype`),
`crates/pdf-model/src/appearance.rs` (`placement`),
`crates/pdf-transform/src/archive/{decision,prepare,preserve,rewrite,report,config,mod}.rs`,
`crates/pdf-transform/tests/archive.rs`, `doc/profiles/keep-everything.toml`.
Builds: `doc/rfc/0007` section 2's `discard` and `preserve`, the second in section 4.6.1's
appending mechanism, under the permission `doc/adr/1014` records and the owner's `A58` gave.
`doc/adr/1025` is the mechanism this extends and `doc/adr/0954` the bill it pays.
Clauses: ISO 19005-2 sections 6.2.1, 6.2.2 and 6.3.1; ISO 19005-4 section 6.3.1; ISO 32000-2
§7.7.3.3 (Table 31), §8.10.1, §8.10.2, §12.5.2 (Tables 166, 170), §12.5.5, §12.5.6.14.

## 1. What was measured, and what was taken

`crates/pdf-transform/tests/archive_corpus.rs` ranks the requirements that stop a conversion by how
many corpus documents each stops. After session 1073 the largest row at PDF/A-2b that is neither a
profile the operator has not supplied (`WRONG_FAMILY`, the limits document's 10.1) nor already an
argued refusal was `annotations/subtype-defined-in-iso-32000-1` at fourteen; its part 4 twin stops
seven more. That is what this round took.

Reading the fourteen witnesses is what shaped the answer. Each is one annotation — `Sound`,
`Screen`, `Movie`, `3D` or `RichMedia` — and they split cleanly in two: **ten carry a normal
appearance stream and four carry no `/AP` at all.** So the site is not one question but two, and
they have different answers.

## 2. The clause states a prohibition and no alternative, so the remedy is a loss

ISO 19005-2 section 6.3.1 forbids any subtype ISO 32000-1 does not define and strikes `3D`,
`Sound`, `Screen` and `Movie` by name; ISO 19005-4 section 6.3.1 strikes the last three. Neither
offers anything to put in the annotation's place, so the only rewrite that meets either removes it
— `doc/pdf-a-conversion-limits.md` section 3.2's *Ask*, and `doc/adr/0816`'s fence is why the
tempting alternative is not available: re-badging the annotation as a subtype the part admits
would be inventing an annotation the producer did not write.

So `Loss::ForbiddenAnnotation`, authorised by `--authorise forbidden-annotation`. What goes with
the annotation is everything it carried — the sound, the movie, the rendition, the 3D artwork, its
own `/Contents` — and none of that is content any of the six targets holds, which is why the
clause strikes the subtype at all. The rewrite takes the *reference* out of the page's `/Annots`
and lets the walk do the rest: it copies what the converted document reaches, so an annotation no
array names is an object the output does not hold, and its media stream goes with it without this
rewrite naming any of them. §12.5.6.14's `/Popup` goes too, being a window onto nothing once its
parent has gone.

## 3. Why the marks need not go with it, and what clause says so

An annotation's normal appearance is a form `XObject` **the producer wrote**, and §12.5.5 makes it
what a reader draws for the annotation. Nothing in that is media: it is a drawing the page already
carries. So the question is whether a target will hold that drawing somewhere, and ISO 19005-2
answers it directly — section 6.2.2's NOTE 2 lists what a content stream may be, naming page
descriptions, form `XObject`s, patterns, Type 3 fonts *and the appearances of annotations*, all
under sections 6.2.2 to 6.2.11 alike. Section 6.2.1 says the same from the other side: the
restrictions are on "the graphical elements", which a conforming reader renders onto their
respective pages. **A page may therefore carry, directly, exactly what an appearance stream
carried**, and moving it asks nothing of the file the file was not already asked. `pdf_archive`'s
own survey had reached the same reading years earlier: it walks every `/AP` entry into the same
clause-6.2 population as a page's content.

That makes this site a `preserve` in `doc/rfc/0007` section 2's sense — the information neither
stays where it was nor is lost, it moves somewhere the target admits — and `doc/adr/1025`'s
appended page is the mechanism already permitted. The page states the `/MediaBox`, `/CropBox` and
`/Rotate` of the page the annotation was on, and invokes the producer's stream under §12.5.5's own
matrix `AA`. **No placement choice is made.** Where the metadata page of ADR 1025 needed ten
documented choices because a packet is not anywhere on any page, this one needs none: the marks
already had a place and the algorithm that puts them there is the standard's. `/Rotate` is the
source page's rather than ADR 1025 section 4's zero, and that departure is the same reason read
forward — there an inherited turn would have laid composed text on its side, here the turn is part
of where the producer's marks are.

## 4. Why the marks are *not* flattened back onto the page they came off

The smaller-looking answer is to append `q AA cm /X Do Q` to the annotation's own page: the page
count would not change, and the rendered page would be identical rather than merely equivalent.
**It was not taken, and the reason is a boundary rather than a difficulty.** `CLAUDE.md`'s third
amendment to the authoring exclusion permits *appending a page* composed solely of content the
document already holds, and in the same paragraph keeps the far side closed: "the watermark stays
on the far side: it composes new content *over* pages, and nothing here reaches it". Writing
operators into a page a producer wrote is that operation, whoever the marks belong to. A round may
not widen a ratified amendment by argument alone, so the question goes to the owner as
`doc/questions/Q65` and the appended page — which is inside the permission as it stands — is what
this round built.

Two further things would have to be settled if the answer is yes, and they are in Q65 because they
are the honest cost of asking: §8.4.4 requires `q` and `Q` to balance within a content stream and
real files do not always, so the appended operators would inherit whatever state the producer's
stream left; and an annotation drawn *over* other annotations would move under them.

## 5. Where an annotation drew nothing, the remedy refuses by name

An annotation with no normal appearance stream has no marks of the producer's to keep. This
program can *construct* an appearance from a subtype's own entries — `pdf_model::appearance`, for
the requirement that asks for an appearance dictionary — and preserving one of those would be
keeping a picture this program drew and calling it the producer's, which `doc/questions/A48`
forbids and ADR 1014 bounds. So a configured `preserve` refuses the document, naming the
annotation and saying that the authorised loss removes it without a page and loses nothing that
was ever drawn. An `/AP` `/N` that is a subdictionary of appearance states is refused the same
way: §12.5.5 makes which of its streams is drawn a question the annotation's own `/AS` answers, so
no one stream is *the* normal appearance.

## 6. The two flavour rows are not these two, and stay refused

ISO 19005-4 section 6.3.1's later paragraphs confine `3D` and `RichMedia` to PDF/A-4e and
`FileAttachment` to PDF/A-4f. Those are narrower statements — the subtype is admitted by the part
and refused by the flavour — so their first answer is the flavour that admits it, and for a
`FileAttachment` `doc/pdf-a-mitigations.md` says the second: drop the marker, keep the file it
names, which is a rewrite this one is not. Both keep their refusal, with a sentence that now says
which. `annotation_subtype_permitted` takes a `Part` rather than a `Target` so that the converter
*cannot* be handed the flavour rows' population by accident.

## 7. What it did, measured

Over the veraPDF corpus, with every loss authorised: at PDF/A-2b 437 failing documents converted
and 173 were refused, and now 447 convert and 163 are refused; at PDF/A-4, 171 and 188 become 176
and 183. The subtype rows leave both targets' refusal rankings entirely. The default run — nothing
authorised — is unchanged at every target, which is what an *Ask* means and is the difference
between this row and session 1073's. No other target moves.
