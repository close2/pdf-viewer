# 1185 — An embedded FDF is a file, and only its encryption is deprecated

Status: **accepted**.
Context: `crates/pdf-model/src/forms_data.rs` (`FormsData::embedded`, `FormsData::files`,
`FormsData::statuses`, `read_embedded`, `Budget`, `MAX_EMBEDDED_FILES`, `MAX_EMBEDDED_DEPTH`,
`MAX_EMBEDDED_BYTES`, `match_to_document`), `crates/pdf-model/src/view.rs`
(`ViewState::append_templates`), `crates/viewer-core/src/interact.rs`,
`crates/pdf-model/src/xfdf.rs`.
Builds: ADR 0907 (a requirement written as a statement of fact), ADR 0919 (the version ranking),
`doc/errata-read.md` Issue #173, `doc/todo/65` bucket 4.
Clauses: ISO 32000-2 §12.7.8.3.1 (Tables 246 and 247), §12.7.8.3.2, §7.11, §7.11.4.

## 1. The entry is not deprecated, and the row's whole residue rested on the belief that it was

Table 246's cell reads, in full: "( Optional; PDF 1.4 ) An array of file specifications (see 7.11,
"File specifications") representing other FDF files embedded within this one (7.11.4, "Embedded
file streams")." It carries no deprecation marker of any kind.

What ISO 32000-2 deprecates is the *encrypted* form. Table 247 states `/EncryptionRevision` as
"(Required if the FDF file is encrypted; deprecated in PDF 2.0)", and the paragraph introducing
that table opens with a participle whose subject is ambiguous between the array and the
encryption. Errata Collection 3 Issue #173 (`/State` `Review` `Completed`) rewrites the opening so
that it governs FDF file encryption. `doc/errata-read.md` records the strike against p. 576.

So the array is an ordinary PDF 1.4 entry and a reader owes it. That much this tree had already
written down; what it had not done is read one.

## 2. An embedded FDF is read as a file, not merged as a fragment

Three readings stack in one element and each is the cell's own: §7.11's file specification,
§7.11.4's embedded file stream under `/EF`, and an FDF file in the bytes that come out. The last
of those is the decision worth recording: the bytes go to `FormsData::read`, the same function
that read the file naming them, and the result is kept whole on `FormsData::embedded`.

**Merging the fields into the outer file's list would have been wrong, and not only untidy.**
Table 246's `/Encoding` is stated per file and decides how *that* file's field names and values
become characters — an embedded FDF decoded under the outer file's entry would be read in the
wrong character set. The same holds for `/Status`, which the table makes a string that "shall be
displayed": it belongs to the file that states it, and `FormsData::statuses` is what hands every
one of them over.

## 3. What importing a nesting means, and why the order is the answer

§12.7.8.3.2 states the operation about a *field*: "importing a field causes the values of the
entries in the FDF field dictionary to replace those of the corresponding entries in the field
with the same fully qualified name in the target document". Nothing in it is about a file, so a
nesting needs no second rule — it needs an order.

`FormsData::files` is that order: this file, then each of `/EmbeddedFDFs` in the array's own order
and everything under it, depth first. `match_to_document` walks it and returns the pairs in that
order, so a caller applying them in turn leaves the later file's statement about a widget
standing — which is exactly what two separate imports of two separate files would leave. `/Pages`
follows the same order in `ViewState::append_templates`, because a template page added by an
embedded file is a page the import adds.

**What this does not do is invent a precedence rule.** The standard states none, and one file
overriding another is the consequence of applying them in turn rather than a policy bolted on.

## 4. The bounds, and what they are bounds on

An embedded FDF is a stream *inside* the file that names it, so each level is strictly contained
in the one above and a cycle cannot be written the way Table 249's `/Kids` cycle can. What a file
can still write is a nesting whose decoded size grows at every level, and a decompression bomb at
any one of them.

Three numbers, and the budget is shared across the whole nesting rather than applied per stream,
because what a bomb costs is the total: `MAX_EMBEDDED_FILES` 64, `MAX_EMBEDDED_DEPTH` 8,
`MAX_EMBEDDED_BYTES` 16 MiB. Trap 38 was asked and answered: neither ISO 32000-2 nor anything it
requires a reader to carry states a number here, so these are bounds on *work* and the constants'
comments say so. Reaching one is reported on `FormsData::owed` and never fatal.

## 5. The encrypted form is refused by name, and that is the departure

Table 247's revision 1 — "the only one defined" — is a 40-bit RC4 key derived by MD5 from a padded
user-supplied password. This program has no password to derive it from, no place to ask for one
that belongs to an FDF rather than to a document, and no reason to carry a key derivation the
standard deprecates. An embedded file stream stating `/EncryptionRevision` is therefore skipped
with a sentence on `owed`, so a person is told which file went unread rather than shown a form
quietly missing its values (trap 5).

That refusal is the whole of what §12.7.8.3.1 does not execute, which is why its row is `departed`
rather than `partial`.

## 6. What this does not decide

Whether an FDF file may be *written* with embedded FDFs inside it. `submission.rs` composes
§12.7.6.2's export and states no `/EmbeddedFDFs`; nothing in Table 240's flags asks for one.
