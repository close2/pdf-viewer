# 3001 — A file specification divides at every SOLIDUS, with no count cap

Session 2079. Status: **implemented.** Supersedes the 256-component cap `pdf_model::file_spec` held as
`MAX_COMPONENTS`. Context: review 2038 (finding B46). Clause: ISO 32000-2 §7.11.2.1.

§7.11.2.1: "The standard format for representing a simple file specification in string form divides the string
into component substrings separated by the SOLIDUS character (2Fh) (/)." The tree stopped dividing after 256
components and appended every later byte, separators included, to the 256th, so a 257-component path came back as
256 components whose last was `x/x`: a name the producer did not write, reported to the archive conversion
(`ExternalData::Named`) as though it were the file's, with nothing to say the split had stopped.

The cap's stated reason was cost, and the cost is bounded without it. The string is already held, and the
parser's `max_string_len` bounds its length; the split walks it once, and each component adds one `Vec` header,
which allocates nothing until a byte arrives. A cap therefore bought no bound the parser does not already give, and
paid for it with a wrong answer. `components` now divides at every SOLIDUS.

Tests: `pdf-transform`'s `archive_external_components_spec_audit` (257 components through the archive report,
unchanged) and `pdf-model`'s `file_spec::tests::every_separator_divides_however_many_the_string_holds`.
