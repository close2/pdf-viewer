# 1056 — the file was in the document, and the media type was not the format's

Date: 2026-09-15. ADR 1070. Touched `pdf-model` (`submission.rs`, `appearance.rs`, `forms_data.rs`, `view.rs`),
`viewer-host` (`policy.rs`), `viewer-confined` (`protocol.rs`), two test files, four ledger rows. Continues 1047, 1052
and ADR 1066, which left §12.7.5.3's Table 231 bit 21 owing one bullet: the HTML-form submission's
`multipart/form-data`, "whose part is the file's *contents*, bytes on a filesystem this crate has none of".

**That reason answered one of the two questions the bullet asks.** The *media type* needs no file at all — "the
submission shall use the MIME content type multipart / form-data", a `shall` about the submission whose condition is the
flag rather than the file being readable — so the body is HTML 4.01 section 17.13.4.2's for any selection holding a
file-select control, and `Submission::media_type` carries RFC 2046 section 5.1.1's `boundary` beside it because that
parameter is computed from the body and no constant can stand for it. The *contents* need a file, and the clause says
what identifies it: "a file specification (7.11, "File specifications") identifying the selected file". §7.11.1 gives a
specification two forms and one of them carries the file — §7.11.4's embedded stream, through Table 43's `/EF` — which
is the one place a process principle 3 gives no filesystem can read a file from. The FDF body carries that same stream
as an indirect object rather than the name of a file the server has no copy of. A specification in the string form is
named on `Submission::owed` by field and by pathname; asking a host to open the path a person typed is rejected in the
ADR rather than deferred. Table 240 bit 4's GET loses to the same clause — a GET has no entity body for a media type to
describe — and is named, and bit 5's two pairs become two parts, which is what `name . x = xval & name . y = yval`
states once the other content type's punctuation comes off it.

**Reading the value as a specification found the appearance half silent.** §12.7.8.3.2's import replaces `/V` with what
the FDF states, which for this flag is that dictionary; `variable_text::value_text` matched a string, a stream and an
array, so a file-select control filled from an FDF drew an empty box and reported nothing. `appearance::text_field_text`
now reads the specification's name, Table 43's `/UF` before its `/F`, and only under bit 21, because for every other
field type a dictionary is no value at all.

**§12.7.8.3.3's refusal was also a claim the clause contradicts.** Table 252's `/Fields` are "the root fields that shall
be imported" — into the *target* document, by §12.7.4.2's name — and the row said they "name[] fields of the template's
own hierarchy rather than the target's `/AcroForm`". They go through `forms_data::match_fields` now, which Table 246's
go through as well. `/Rename` is what was never moot: a template is added by reference to a page this document already
holds, so every name conflicts, and the flag's `false` — "all fields with that name shall be updated" — is what an
import does, while its `true` asks for fields under names this document has not got and is refused by name.

Rows: §12.7.5.3 stays `partial` with one shape left instead of one flag; §12.7.8.3.3 with one branch of one flag;
§12.7.6.2 and §12.7.8.3.2 record what each gained. Calibrated (trap 13): nine planted defects, each named by its own
test. Two pages move and neither is a corpus page — no document on this disk carries an FDF.
