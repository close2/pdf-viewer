# 2997 — A partial XFA resource is refused, and output protection goes only where it is written

Session 2078. Status: **implemented.** Amends ADR 1270. Context: review 2038 (findings
B58, T112, D56, T109). Clauses: ISO 32000-2 §12.7.3 Table 224 (`/XFA`), Annex K; §7.6.4; ISO 19005's
`file-structure/no-encryption`.

**XFA.** Annex K: "A packet is a pair of a string and stream." `attach::xfa_resource` skipped any array element
that was not a stream, so an array whose second packet was a string attached the opening tag alone while the report
said the original was kept. The array is now read as pairs, and one that is not wholly string-and-stream pairs is no
resource, which the caller refuses by name (`XFA_NOT_READABLE`) as its documentation already promised.

**Protection.** `apply_protected` accepted a `Protect` for every verb and the archive dispatch ignored it, so a
caller asking for an encrypted output received a plain PDF/A file and no word. The command line already refused the
combination; the library now refuses it too, for every verb but split, merge, pages, optimize and redact, with a
`Refusal::Assembly` naming §7.6.4. An archive cannot take it: its output may state no `/Encrypt`.

Tests: `pdf-transform` `archive` `xfa_attachment_refuses_a_packet_array_with_a_non_stream_packet`,
`an_archive_given_output_protection_is_refused_rather_than_written_plain`; `protect_dispatch_spec_audit`.
