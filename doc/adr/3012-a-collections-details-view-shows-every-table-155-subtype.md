# 3012 — A collection's details view shows every Table 155 subtype

Session 2081. Status: **implemented.** Context: review 2038 (findings B05, B06). Clauses: ISO 32000-2 §12.3.5.1
Table 153 (`/View D`), Table 154 (other keys), Table 155 (`/Subtype`), §7.11.6 Tables 46 and 47.

`viewer_host::panel::collection_rows` answered only the file-related subtypes, because a host held the attachments
and no collection item: an `S`, `D` or `N` column and Table 155's PDF 2.0 `CompressedSize` were empty cells, while
`/View D` presents "all information in the Schema dictionary". `pdf_model::collection::Collection` now carries
`items`, §7.11.6's collection item of every `/EmbeddedFiles` file by the tree's key, read by the function
`sorted_keys` orders by, and the confined wire carries them beside the schema.

A column reads its value under its own schema key, as Table 154 says the key "shall be used to identify a
corresponding collection item dictionary". The schema's type decides how the object is read: Table 46's "the type
of each entry shall match the type of data identified by the collection field dictionary" makes a value of another
type a broken `shall`, and its cell is empty rather than a reading the schema did not ask for. Table 47's `/P` goes
in front of every kind. A date is shown as every other date in the panels is (`stamp`), since the clause leaves its
format to the processor. `CompressedSize` is the stream dictionary's direct `/Length`, or else the length of the
bytes the stream holds, which is what an indirect `/Length` delimited.

Tests: `viewer-host` `collection_schema_cells_audit` (all four); `viewer-confined`
`every_panel_answer_round_trips` carries items with and without values.
