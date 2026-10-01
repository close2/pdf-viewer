# 1473 — A PDF 2.0 file's deprecated metadata is written into its packet alone

Session 1319. Status: **accepted**.
Context: ISO 32000-2 §3.15, §14.3.1, §14.3.2, §14.3.3 (Table 349 and its NOTEs), §14.3.4;
`crates/pdf-transform/src/update.rs` (`TABLE_349`, `deprecated_in`, `restated_packet`,
`metadata_stream`, `set_information`, `restate_metadata`), `crates/pdf-transform/src/merge.rs`
(`write_information`, `output_version`), `crates/pdf-transform/src/archive/prepare.rs`. Amends ADR
0855's §14.3.4 paragraph and ADR 1212 section 3's packet; carries out what ADR 1461 priced for §14.3.1.

## 1. The reading

§14.3.1: "Except for the CreationDate and ModDate entries, the use of the document information
dictionary for document metadata is deprecated in PDF 2.0", and §3.15 defines the word as "a part of
ISO 32000 that should not be written into a PDF 2.0 document". The same clause names the other home:
"Metadata streams are the preferred method in PDF 2.0." The condition is the *file's* version, so the
writer asks the version it writes: `Document::version` for an update (the header raised by the
catalog's `/Version`), the highest source's for a merge, which is what the merged header states.

## 2. The decision

- **Version 2.0 or later**: Table 349's entries other than the two dates go into §14.3.2's packet
  alone, each under the property the table's own NOTE names. An update removes a stated deprecated
  key from the dictionary — the operator's value replaces it, in its preferred home — and leaves every
  key the operator did not state as the producer wrote it. A 2.0 file with no packet is given one, and
  the catalog is rewritten in the same update to name it. A merge writes no `/Info` at all when only
  deprecated entries were stated.
- **Earlier versions**: the dictionary keeps every entry, because nothing deprecates it there. A
  packet the document already holds is restated beside it; a document with none is given none.
- **Both versions**: where both sources are written they say the same thing. `restated_packet`
  removes the stated property (`xmp::remove`) and then adds the new value (`xmp::supplement`).
  `supplement` alone is additive by design — §14.3.4's rule for a processor *not* asked to change a
  value — so it would leave the old title beside the new one. The removal takes every language
  alternative of a `dc:title` with it, since each renders the title being replaced.
- **A packet that cannot be edited in place** (not UTF-8, malformed, no `rdf:RDF` to write into) is
  a warning naming why, and the entry stays in the dictionary. A deprecated home is a "should not";
  losing the operator's value would be worse, and the warning says which happened.

- **Only a change is an edit.** An entry whose value equals what the document already states,
  as section 5 reads it, touches neither source. A deprecated entry written into a 2.0 file's packet
  is named in a warning. `/Trapped` absent and `/Trapped /Unknown` count as the same value, because
  Table 349 gives the entry "Default value: Unknown".

`TABLE_349` moved from the archive converter into `update.rs`, so the three writers of these entries
read one table.

## 3. What this amends

ADR 0855 said the in-place update writes one source and warns about §14.3.4, because RFC 0003 makes
`meta/xmp.xml` a derived file. That file is still derived and still refuses writes. What changed is
that setting an entry now restates the packet as part of the same edit, so the warning has no case
left except the unwritable packet. ADR 1212 section 3's merge packet carried three XMP basic
properties; it now carries every stated entry's counterpart.

## 4. Tests

`tests/update.rs`: `in_a_pdf_2_0_file_the_deprecated_entries_go_into_the_packet_alone`,
`in_an_earlier_file_both_sources_state_each_entry_and_agree` (a date included, §14.3.4's fourth
rule), `an_earlier_file_with_no_packet_is_given_none`,
`restating_what_the_document_says_leaves_the_packet_alone`. `tests/merge.rs`:
`a_merged_files_information_dictionary_and_packet_agree`,
`a_merged_pdf_2_0_file_states_the_deprecated_entries_in_the_packet_alone`, both `qpdf --check` clean.

## 5. The view: `meta/info.json` presents Table 349 as the document states it

The tier-3 gate `pdf-vfs --test write_corpus` caught what section 2 left open. `meta/info.json` read
only the dictionary. After a title was written into a 2.0 file through it, the title was in the
packet and the view reported `null`, so the file no longer stated what the document stated. §14.3.3
itself has no consistency sentence. It says: "In PDF 2.0 such use is deprecated except for two
entries, CreationDate and ModDate . For any other document level metadata, a metadata stream (see
14.3.2 "Metadata streams") should be used instead." So in a 2.0 file the stream is where an entry's
value is. The sentence that binds agreement is §14.3.4's: "a PDF processor shall ensure that the
data in the document information dictionary and the document level metadata stream -if both are
written -are fully equivalent". Its reader's half reads: "it is at the discretion of the PDF
processor how to use this data".

`update::stated_information` is that reading, and both directions use it:
- **Where each value comes from.** Each of the nine entries comes from the dictionary where the
  file keeps it there. Otherwise, for every key except the two dates, it comes from the packet
  property `TABLE_349` names. The dates' home is the dictionary in every version, and the packet
  spells them in ISO 8601, not §7.9.4, so a date absent from the dictionary is reported absent.
- **`/Trapped` must be a name.** A `/Trapped` written as a string states nothing, because Table 349
  says "This shall be the name True , not the boolean value true ." This matches
  `pdf_model::metadata`, and `calrgb.pdf`'s `/Trapped (False)` in the read gate is the witness.
- **Reading.** `pdf-vfs`'s `information_json` renders it. `tests/read_corpus.rs` derives its expected
  value separately, from `pdf_model::metadata` and the NOTE properties, so the gate is not the code
  it checks.
- **Writing.** `set_information` compares each written entry against it. A file read and written
  back whole therefore changes nothing, and a changed entry goes where section 2 sends it.

Test: `pdf-vfs`'s `a_write.rs::info_json_reads_a_pdf_2_0_files_title_from_its_packet_and_writing_it_back_leaves_the_packet`.
It reads a 2.0 file's packet-only title back, writes the view back unchanged with the packet's
bytes untouched, and checks that a changed title lands in the packet.
