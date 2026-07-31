# ADR 0076 — The files inside the file

Status: accepted, 2026-07-31.

## Context

§7.11's ten rows are `reported` for one sentence: a file specification "refers to a file external
to the PDF file", and §7.11.1 is explicit that this is true "in either case", embedded or not.
This program has no filesystem (principle 3, ADR 0014), so it refuses them by architecture.

§7.11.4 is the exception the family's own row already named: **an embedded file needs no
filesystem at all**, because the bytes are inside the document. Ten of the 974 corpus documents
carry a `/Names /EmbeddedFiles` tree, and the §12.11 requirement `Attachment` — written two
sessions ago — had "no attachment list: §7.11.4's embedded file streams are unread" as its
reason.

## Decision

**List them; decode nothing; write nothing.**

`attachment::attachments` walks §7.7.4's `/EmbeddedFiles` name tree and reads, per file, the
tree's key, Table 43's `/UF` or `/F` and `/Desc`, Table 44's `/Subtype` media type and Table 45's
`/Size`, dates and `/CheckSum`. It carries the `Stream` and stops there. `viewer-ui` prints one
line per attachment when a document opens.

Three things are deliberately *not* done, and only one of them is about the sandbox:

- **Writing a file** is what principle 3's confinement exists to prevent, and an attachment is
  arbitrary bytes a document controls. Extracting one is a person's decision.
- **Decoding eagerly** would inflate every attachment of every document that has one, on the path
  that opens a document. A 200 MB spreadsheet is a legal attachment.
- **Verifying the `/CheckSum`** would mean doing exactly that, for a value the clause says is
  "strictly a checksum, and is not used for security purposes".

`tree::name_pairs` is the piece §7.9.6 was missing: `number_pairs` had existed since §12.4.2's
page labels needed every key of a number tree, and a caller listing attachments has no key to
look up either. The two now share one descent, because §7.9.7 defines a number tree as a name
tree with integer keys and a second copy of the cycle guard is a second place to get it wrong.

Checked against the corpus rather than only against a fixture: 10 documents, 23 files, mostly
`application/mathml+xml` fragments from a LaTeX producer. **Two of the 23 refuse to decode**, and
that is right — both are in documents whose `/Encrypt` this reader has no password for, and
§7.6.6 puts the refusal on the stream whose key is missing rather than on the file.

## The other half of this session: everything re-measured

Six sessions of change since the last verification, so all of it was run:

| | |
|---|---|
| tests | 747, all passing |
| `clippy` under `pedantic` + three deny lints | clean |
| `cargo fmt --check` | clean |
| `cargo deny` | clean on all four checks |
| four fuzz targets at 50 000 runs | clean |
| corpus gate | 89 documents draw incompletely |
| oracle gate | 837 agree, 65 contradicted |
| text gate | 97.8% of `pdftotext`'s words |

**Interpretation costs 2 105.9 M instructions against a baseline of 2 103.8 M** — the same
`callgrind_interpret` page, measured on the commit these six sessions started from, rebuilt and
run in the same sitting. That is **+0.10%**, below the 0.23% floor this project records for
comparing the number across sessions, so six sessions of clause work cost nothing measurable on
the page-drawing path.

One number worth writing down: the handover recorded 2 094.9 M for that same baseline commit, and
it measures 2 103.8 M today. **0.42% of drift with no code between them**, which widens the floor
the sixtieth session established at 0.23% and is the second time this project has caught its own
performance number moving under it. Quote a measurement against one taken the same afternoon.

## Consequences

- `silent` falls 132 → 129: §7.7.4, §7.11.4 and §7.11.4.1 become `partial`, and clause 7 has two
  silences left — §7.11.4.2's related files and §7.11.6's collection items, both behind features
  nobody has asked for.
- The `Attachment` requirement's reason in `requirements.rs` changes with the code, which is the
  decay that table was written to expect.
