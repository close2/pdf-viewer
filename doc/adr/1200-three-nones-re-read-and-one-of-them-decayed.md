# 1200 — Three *none*s re-read against their clauses, and one of them decayed

Status: accepted, **read rather than built**. Session 1181.
Context: `doc/pdf-a-mitigations.md` sections 3 and 5, `doc/todo/66`,
`crates/pdf-archive/src/table/{file_structure.rs,fonts.rs}`,
`crates/pdf-transform/src/archive/fonts.rs`.
Answers: `CLAUDE.md`'s rule that a claim about the specification decays, applied to the three
catalogue entries behind the largest refusal counts the archive sweep prints after the encryption
and separation families were built.
Clauses: ISO 19005-2 sections 6.1.13, 6.2.11.4.1 and 6.2.11.8; ISO 32000-2 §7.7.3.3 (Table 31),
§14.11.2.1, §9.3.6.

## 1. `implementation-limits/page-boundary-sizes` — the *none* is too wide, and §14.11.2.1 says so

The catalogue's entry is one line: *nothing. Rescaling a page moves every mark on it, and tiling one
into several pages composes pages nobody produced.* That is true of the **media box** and of nothing
else, and the difference is two sentences of the base standard.

ISO 19005-2 section 6.1.13's last requirement holds *any* of the page boundaries §14.11.2 describes
to between 3 and 14 400 units in either direction, and §14.11.2 describes five. Of those, Table 31
makes **`/MediaBox` required and the other four optional**, and §14.11.2.1 gives each optional one a
default that is another box in the same file: the crop box's "default value is the page's media
box", and the bleed, trim and art boxes' "default value is the page's crop box". **Removing an
optional entry is therefore not an edit to what the page says — it is the page saying it the way
Table 31 admits**, and no mark moves.

Whether the removal is *lossless* is a second question, and §14.11.2.1 answers it outright for the
case that matters:

> If the bounds of the crop, trim, bleed or art box extends outside of the bounds of the media box,
> a processor shall treat the box as its intersection with the media box.

So an over-sized box is **already** its intersection with the media box to every conforming
processor. Where that intersection is what the default would give — a crop box larger than the
media box; a bleed, trim or art box larger than it with no crop box narrowing them — removing the
entry changes nothing any reader computes. That is `doc/adr/1176`'s shape exactly: the standard has
decided what the bytes mean and the rewrite transcribes the decision.

The predicate, stated so a later round does not re-derive it: **removing box `B` is mechanical where
the effective box after removal equals the effective box before it**, both effective boxes being
§14.11.2.1's intersection with the media box, and the default `B` falls back to being the crop box's
own effective value. Where the two differ — a *too small* box, or an over-sized box a narrower crop
box stands behind — the removal is a genuine loss, since a reader would show, clip or trim a
different region, and it belongs in the catalogue as a `discard` with that cost stated. Where the
failing box is the **media box**, the entry's *none* stands unchanged and for its original reason.

**The corpus exhibits every one of these shapes**, which is why this is a decay rather than a
technicality. Among `PDF_A-2b/6.1 File structure/6.1.13 Implementation limits`,
`6-1-13-t09-fail-f` states a crop box of 14 402 units over a media box of 4 — already clipped to 4
by the sentence above, so the entry carries no information a reader uses — while `t09-fail-a` and
`t09-fail-e` fail at the media box itself, where nothing helps. How many of each the whole corpus
holds is the sweep's to count and not this file's.

**Not built this session**, deliberately: the predicate is a geometry rule with three cases and a
wrong one would change what a page shows, which is the one class of mistake this verb may not make
quietly. What is recorded here is the reading and the predicate; the entry in
`doc/pdf-a-mitigations.md` section 3 now says the same.

## 2. `fonts/embedded-programs-define-every-glyph-shown` — the *none* holds, and the clause's own exemption is already read

ISO 19005-2 section 6.2.11.4.1's third requirement is that embedded fonts "shall define all glyphs
referenced for rendering within the conforming file". The catalogue's *none* rests on the program
being the producer's and fixed, so that the only routes are taking a code off the page — a mark
removed — or drawing a glyph for it — a mark invented. Both are on the far side of ADR 0816's fence.
That stands.

The place a decay could have hidden is the clause's own narrowing. Its second sentence makes a font
used "if at least one of its glyphs is referenced from a content stream", and its NOTE 2 takes
§9.3.6's text rendering mode 3 out: a font referenced solely in that mode "is therefore not rendered
and is thus exempt from the embedding requirement". A validator that counted mode-3 text would refuse
documents the clause does not bind. **It does not**: `crates/pdf-archive/src/table/fonts.rs` records
the exemption against the font in its survey and checks the narrower population — the codes actually
shown outside mode 3. So the refusals this site prints are the clause's own, and the *none* is about
all of them.

## 3. `fonts/no-notdef-glyph-shown` — the *none* holds, and its clause is stronger than its neighbour's

The catalogue treats this as one entry with the requirement above. For the remedy that is right —
the same two routes, the same fence — but the two clauses are **not** the same shape, and the
difference is worth the sentence. ISO 19005-2 section 6.2.11.8 forbids a reference to the `.notdef`
glyph from any text-showing operator **regardless of text rendering mode**, in as many words. So the
exemption section 2 above turns on is denied here by the clause itself, and a reading that carried
it across from the neighbouring entry would under-report. `crates/pdf-archive/src/table/fonts.rs`
already states this, and the catalogue entry now does too.

## 4. `fonts/font-programs-embedded` — the supply the tree describes is not a flag the tree accepts

Not a *none*: the site is answered by `Rewrite::SubstituteFontProgram`, which embeds one of the
faces this program ships, and `doc/questions/A47` made that the default. What is refused is the case
A47 attaches a condition to — no shipped face covers the document's characters — and five messages
in this tree tell the user what to do about it: *supply the font with `--font`*.

**`--font` is not a flag `quorra-transform` accepts.** It is in neither `VALUED` nor `KNOWN`, so the
argument reader answers a usage error, and `doc/pdf-a-conversion-limits.md` section 4.9's
`--font <name>=<file>` is a design nobody built. That is a promise the program cannot keep, which is
`CLAUDE.md` principle 1's own failure mode, and it is recorded here rather than removed because the
design is right: an operator naming a face is `doc/rfc/0007`'s `supply` in its oldest form, and
ISO 19005-2 section 6.2.11.4.1's *only font programs that are legally embeddable in a file for
unlimited, universal rendering shall be used* is a fact about a licence that **only the operator can
state**. What `crates/pdf-transform/src/archive/fonts.rs` argues against is this program picking a
face off the machine by itself, which is a different act.

So the recommendation, for whichever round takes it: build `--font <base-font>=<path>` as a `supply`
— the program in the plan, the operator's authority in the report and in `xmpMM:History` beside the
substitution already recorded there, and `pdf_font::restate` applied exactly as the shipped-face
route applies it, so no glyph moves. Until then the messages name a flag that does not exist, and
`doc/todo/66` says so.
