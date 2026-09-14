# ADR 1070 — The file a form selects is one the document carries, and the media type is the submission's

## Status

Accepted, 2026-09-15. Session 1056. Continues ADR 1062 and ADR 1066, which divided §12.7.6.2 into
a composition this crate does and a transmission a host does. This settles §12.7.5.3's Table 231
bit 21, whose last bullet those two left owed.

`§N` is ISO 32000-2 and nothing else.

## Context

Table 231 bit 21 makes a text field "function as a file-select control", whose "text represents the
pathname of a file whose contents shall be submitted as the field's value", and §12.7.5.3 then
splits by format. The FDF bullet was written in session 1047; the XML bullet is inside the XFDF
refusal; the HTML one — "the submission shall use the MIME content type multipart / form-data, as
described in Internet RFC 2045" — was named on `Submission::owed` because the file's contents were
taken to be "bytes on a filesystem this crate has none of".

Read again, that reason answers one of the two questions the bullet asks and not the other. The
*media type* is a property of the submission and needs no file at all. The *contents* need a file,
and the clause says what identifies it: "a file specification (7.11, "File specifications")
identifying the selected file". §7.11.1 gives a specification two forms, and one of them carries
the file — §7.11.4's embedded file stream, through Table 43's `/EF`.

## Decision

**The media type belongs to the submission, not to the format.** `Submission::media_type` is what a
host sends; `Format::content_type` stays the type of a body written in that format alone. They
differ for one reason and it is RFC 2046 section 5.1.1's: a `multipart/form-data` carries a
`boundary` parameter computed from the body, which no constant can stand for.

**Its condition is the flag, not the file.** The sentence is about the *submission*, so a selection
holding a file-select control is written as multipart whether or not the file behind it can be
read. `Chosen::file_select` carries that separately from the entries, because a field contributing
no part still decides how the body is written.

**The file comes from the document or from nowhere.** Where the value is §7.11.1's dictionary form
with `/EF`, the part carries the stream's decoded bytes, Table 44's `/Subtype` as its content type
and the specification's own name as HTML 4.01 section 17.13.4.2's `filename`; the FDF body carries
the same stream as an indirect object, since an FDF has a body to put one in and the server has no
copy of the sender's disk. Where the value is the string form, the field is named on
`Submission::owed` by field and by pathname. **Asking a host for the file is rejected**, and not on
effort: `viewer_core`'s rule 2 gives the layers above no filesystem either, §12.7.6.4's import
channel resolves one name against the open document's directory under a host policy, and a
file-select control's pathname is whatever a person typed — so the parameter would be a request to
open an arbitrary path on behalf of a document, which is the act principle 3's sandbox exists to
refuse. The honest answer is that this program cannot read that file, said by name.

**Table 240 bit 4 loses to §12.7.5.3 where both are asked for.** An HTTP GET carries its data in the
URL and has no entity body for a media type to describe. The specific `shall` wins — bit 4 speaks
about "field names and values" in general and §12.7.5.3 about the control whose file is the reason
there is a body at all — the submission is composed as a POST, and bit 4 goes to the host as a
sentence.

**A file-select control's value is read as a specification on the way in as well as out.**
§12.7.8.3.2's import replaces `/V` with whatever the FDF states, which for this flag is a
specification, so `appearance::text_field_text` lays out the name it gives (Table 43's `/UF` before
its `/F`). Without that the field drew nothing and said nothing, which is trap 5's silence inside a
feature otherwise built.

**Table 252's `/Fields` are imported, and `/Rename` is what decides whether they may be.** The
entry is "the root fields that shall be imported (those with no ancestors in the field hierarchy)"
— imported into the *target* document, by §12.7.4.2's name, which is the act §12.7.8.3.2 defines —
so they go through the same matching as Table 246's. A template page is added by *reference* to a
page this document already holds, so every one of its names conflicts with an existing field, and
the flag says what to do about that: `false` — "all fields with that name shall be updated" — is
what an import already does and is applied; `true` asks for new fields under names this document
has not got, which `CLAUDE.md` rule 1's immutable document has nowhere to put, and is refused by
name. **Applying a `true` as though it were a `false` is the thing a later round must not do**: it
would write the template's values onto the fields the flag exists to leave alone, which is a wrong
answer rather than a partial one. This replaces the reason the ledger gave — that Table 252's
`/Fields` "names fields of the *template*'s own hierarchy rather than the target's `/AcroForm`" —
which the entry's own sentence contradicts.

## Alternatives

**Keep writing `application/x-www-form-urlencoded` and go on owing the sentence.** Rejected: the
clause's `shall` is met by a body this crate can write today, and what was owed was only ever the
part inside it.

**Take the file from a host.** Rejected above, on the sandbox rather than on effort.

**Apply a template's fields whatever `/Rename` says.** Rejected above: the two branches are
different operations and only one of them is about the fields this document has.

**A random boundary.** Rejected: a body that differs run to run is a body no test can hold to
anything. The search is deterministic and bounded by `MAX_BOUNDARY_TRIES`; a body that contains
every delimiter it would try is refused by name rather than written with a delimiter that would
split it in the wrong place. HTML 4.01 section 17.13.4.2 puts the choice outside its own scope —
"how this is done lies outside the scope of this specification" — so the bound is this program's
and is written down as one.

## Consequences

`Submission` gains `media_type`, which crosses `viewer-confined`'s protocol as a string and is what
`viewer_host::policy::submission_note` prints. `Refusal` gains `Undelimitable`. §12.7.5.3 stays
`partial` and its remainder is one shape rather than one flag: a specification in §7.11.1's string
form, whose file is outside the document. §12.7.8.3.3 stays `partial` on `/Rename`'s `true` branch
alone. Nothing here moves a pixel except a file-select control filled from an FDF, which drew
nothing before, and a template field imported under `/Rename false`, which drew the document's own
value before.
