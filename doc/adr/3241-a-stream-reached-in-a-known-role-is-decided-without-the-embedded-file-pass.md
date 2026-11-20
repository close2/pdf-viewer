# 3241 — A stream reached in a known role is decided without the embedded file pass

Session 2135. Status: accepted and built. Context: ADR 3234's cost; ISO 32000-2 §7.6.6 Table 20
(`/EFF`), Table 43 (`/EF`, `/RF`), Table 44; CLAUDE.md principle 2 ("a 500-page document must
reach its first page no slower than a 5-page one").

## Decision

ADR 3234 decides whether an untyped stream is an embedded file stream by a pass over every
indirect object, run the first time such a stream is loaded in a document whose `/EFF` method
differs from `/StmF`'s. Page one's content stream was that first stream, so those files paid the
whole file before their first page.

Table 43 names the one route to an embedded file stream: a file specification's `/EF`, and
`/RF`'s related files arrays. A caller that reaches a stream by another route says so:
`Document::resolve_as(object, StreamRole)` and `get_key_as` record the referenced number in
`EmbeddedFileStreams::reached_in_role` before loading it, and `stream_filter` gives a recorded
untyped stream `/StmF` without the pass. `reached_as` records without resolving, for a caller that
resolves later. The roles, each by the clause that defines the route:

- `PageContents` — Table 31's `/Contents` (`ContentReader::for_page`);
- `Resource` — Table 34's `/XObject`, `/Pattern` and `/Shading` entries (`resource_entry`);
- `FontProgram` — Table 120's `/FontFile`, `/FontFile2`, `/FontFile3` (`pdf_font::program`);
- `Appearance` — Table 170's `/N`, `/R`, `/D` and their states (`annotation.rs`).

A stream reached by number alone, or through any route not listed (an image's `/SMask`, an
`/ExtGState`'s soft mask group), is decided by the pass as before: missing a role costs the pass,
never a wrong filter. Nothing is recorded in a document that is not encrypted or whose `/EFF`
names `/StmF`'s method.

## Why this construction

ADR 3234 rejected deciding at the referrer because the object cache keeps whichever answer is
loaded first, and a caller reaching the stream by number would get the other one. That stays
true of the *embedded file* side, which still goes through the pass. The other side is different:
a stream reached as a page's contents is not one Table 43 names, unless the file names the same
object from `/EF` too. Such a file wrote one ciphertext for two filters, so one of the two
readings is noise whichever is taken; here the first load decides. That is the one place the
order of reads can change an answer, it needs a self-contradicting file, and it is written down
here rather than paid for with the pass on every launch.

The role parameter decides nothing beyond "not an embedded file"; it is carried so that each
call site names the clause its claim rests on.

## Cost and measurement

One `HashSet` insert under a lock per stream reached in a role, in the files that need it.
`crates/pdf-model/tests/embedded_file_scan_launch.rs` builds the files (revision 5, `/StmF
/StdCF`, `/EFF /Identity`, one attachment, an annotated page one) and its ignored test times open
plus `pdf_model::interpret` of page one in release, median of 31; the record of session 2135 has
the numbers. Its other test asserts `Document::embedded_file_streams_collected` is false after
page one and true after the attachment is read by number.
