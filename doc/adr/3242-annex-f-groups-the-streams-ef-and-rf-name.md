# 3242 — Annex F's embedded file streams are the streams `/EF` and `/RF` name

Session 2135. Status: accepted and built. Context: ISO 32000-2 §F.3.10, §F.4.7, Table 43, Table
44, §7.11.4.2; ADR 3234, whose habit this applies to the linearising writer.

## Decision

`linearize::graph_of` marked a stream as an embedded file stream (§F.3.10's own category, with
§F.4.7's hint table) when it stated `/Type /EmbeddedFile`, or when a direct `/EF` dictionary
named it. Table 44 makes `/Type` "(Optional)", so the first test covers streams no file
specification names and the second missed an indirect `/EF` and every `/RF`.

`collect_dict` now records each `/EF` and `/RF` value as written (`FileEntry`), and `graph_of`
resolves them one level against the assembly: every value of `/EF`, and every reference in each
of `/RF`'s related files arrays, whose second element of each pair "shall be an embedded file
stream" (§7.11.4.2). The `/Type` test is gone; the existing rule that only a stream is one stays.

## Why

The reader decides the same question the same way (ADR 3234); a writer that grouped by another
test would place in part 9's embedded file run a stream the reader decrypts as an ordinary one,
and leave an untyped attachment among the objects of whichever page reached it first.

A `/Type /EmbeddedFile` stream no file specification names is an object nothing reaches as an
embedded file, and it keeps the place the graph gives it.
