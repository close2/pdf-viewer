# ADR 1066 — A submission carries the file, and narrows whole what it cannot judge

## Status

Accepted, 2026-09-14. Session 1052. Continues ADR 1062, which divided §12.7.6.2 into a
composition this crate does and a transmission a host does, and settles the three questions the
remaining Table 240 flags raise.

`§N` is ISO 32000-2 and nothing else.

## Context

ADR 1062 left §12.7.6.2 `partial` with five of Table 240's flags reported rather than applied —
bits 7, 8, 10, 11 and 14 — on the general ground that each wanted something the composition did
not have. Read one at a time against the table, that ground holds for two of them and not for
three.

Bit 7 wants Table 246's `/Differences`, "[a] stream containing all the bytes in all incremental
updates made to the underlying PDF document since it was opened", and the same row requires the
save that produces them: "An incremental update shall be automatically performed just before the
submission takes place, in order to capture all changes made to the document." That is
`ViewState::save`, which this tree already writes and which bit 9 already submits whole. Bit 14
wants "the F entry of the submitted FDF" to be "a file specification containing an embedded file
stream representing the PDF file from which the FDF is being submitted" — the same bytes, in
§7.11.3's dictionary around §7.11.4's stream. Bit 8 wants "all markup annotations in the
underlying PDF document", which Table 171's third column enumerates and which §12.7.8.3.4 asks be
written with Table 254's ordinal `/Page`. None of the three needs a network; each needed a
writer.

## Decision

**Bits 7, 8, 12 and 14 are composed.** Bit 12's `ExclFKey` comes with bit 14 because the two
speak about one entry, and it was previously "met" only by an absence.

**Bit 11 narrows the whole set, and that is the ruling later rounds are not to re-litigate.** It
asks for "only those markup annotations whose T entry … matches the name of the current user, as
determined by the remote server to which the form is being submitted". The determining party is
the server, which principle 3 keeps out of this process; and this program has no name of its own
to fall back on either, because `ViewState::add_markup` deliberately writes no `/T` — "a person's
name is not something this program knows". The predicate therefore cannot be evaluated here, and
the two ways to be wrong are not symmetric: including an annotation that does not match breaks
bit 11's own *only*, and including none breaks nothing the table states, since bit 11 is itself a
narrowing of bit 8. So the narrowing is applied to all of them and the sentence goes to the host
on `Submission::owed`. The privacy of the annotations the document asked to withhold and the
letter of the flag point the same way; where they did not, the letter would still decide.

**Bit 10 stays owed, on the standard's own words rather than on effort.** Its NOTE 1 says which
fields hold dates "is not specified explicitly in the field itself but only in the ECMAScript code
that processes it", and `CLAUDE.md` excludes ECMAScript.

**The composed FDF states §7.5.4's cross-reference table**, although §12.7.8.2.1 makes one
optional. Bit 14's body holds a whole PDF file, and that file's bytes contain `obj`, `endobj`,
`trailer` and `%%EOF` of its own; a consumer that finds objects by scanning finds the embedded
file's. The optional table is what makes the body unambiguous, and `startxref` goes with it
because a table nothing points at is a table nobody finds.

**§7.5.2's header is the earlier of the two markers, not the PDF one first.** `header_position`
preferred `%PDF-` and fell back to `%FDF-`, which is right about a PDF whose first kilobyte
happens to hold the second marker and wrong about the file this ADR makes: an FDF whose header is
at byte zero and which carries `%PDF-` a few hundred bytes in. Measured from the wrong marker
every offset in a correct table is short by that distance, the document is recovered by scanning,
and the scan finds the embedded file's objects. Earliest wins is right in both directions: a
header is the first line of its file, and anything in front of it is the junk §7.5.2's NOTE 1
licenses.

## Alternatives

**Leave the three flags reported.** Honest while nobody had read them one at a time, and that is
exactly what made them look alike: the word *network* was doing work for flags that never asked
for one.

**Ask a host for the current user's name so bit 11 can be evaluated.** Rejected, and not on
effort: the table says the name is the *server's* determination, so a host's guess would be a
different predicate wearing the flag's name. Nothing in `viewer-core` or in the three windows
holds such a name today either, so the parameter would be plumbing with no supplier — which
principle 1 calls a placeholder.

**Carry `/AP` and `/Popup` into the FDF by inlining what they resolve to.** Rejected: an FDF has
no object space of this document's, `/AP` resolves to streams, and `/Popup`'s `/Parent` points
back at the annotation being written. The entries are left out and named, which is the same answer
§12.7.6.2's push-button sentence already gets.

## Consequences

`pdf_model::submission` gains `Carried` and an FDF writer that emits indirect stream objects and a
cross-reference table; `forms_data::FormsData::read` reads Table 246's `/F` through
`crate::file_spec::FileSpec`, so the dictionary form this program now writes is not read back as
naming no source. `pdf_syntax::xref::header_position` ranks the two markers by position.

§12.7.6.2 stays `partial` and its remainder is two flags rather than five; §12.7.8, §12.7.8.3.4
and §7.5.2's rows record what changed. Nothing here moves a pixel: a submission is composed and
handed over, and no appearance is drawn from any of it.
