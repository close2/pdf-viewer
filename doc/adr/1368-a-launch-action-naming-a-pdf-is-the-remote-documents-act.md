# 1368 — A launch action naming a PDF is the remote-documents act, not the sandbox's

Session 1265. Status: accepted as a reading; **not built**.
Context: `crates/pdf-model/src/action.rs` (`launch`), `crates/viewer-host/src/policy.rs`
(`RemoteDocuments`, `under_remote_documents`), `crates/viewer-core/src/interact.rs`; ledger row
§12.6.4.6; `doc/todo/65` bucket 6.
Amends: ADR 0460's reading of §12.6.4.6. Its half about a launch action with no `/F` stands.

## 1. The reading that did not hold

§12.6.4.6's row sat in `doc/todo/65`'s not-owed bucket on two sentences. A launch action with no `/F`
is declined by Table 207's own "If this entry is absent and the interactive PDF processor does not
understand any of the alternative entries, it shall do nothing." That half holds. The other half said
one *with* an `/F` is withheld by principle 3's sandbox. That half is true of only one of the two
things the entry can name. Table 207 makes `/F` "[t]he application that shall be launched or the
document that shall be opened or printed", and `/NewWindow` "shall be ignored if the file designated
by the F entry is not a PDF document". So the table expects a launch action to name a PDF to open, in
a new window or this one.

Opening a PDF a document names is an act this tree already performs under the reader's four levels.
`viewer_host::policy::RemoteDocuments` asks it for Table 203's `/F` (§12.6.4.3), Table 209's (§12.6.4.7)
and Table 253's (§12.7.8.3.3), ADR 1227 and ADR 1239. Nothing about the fourth table's `/F` is more
dangerous than the first three: the file is parsed as a PDF inside the same confinement, and no
program is started.

## 2. The witness

The corpus's only launch action is this case. In `doc/pdf.js/test/pdfs/issue17846.pdf`, object 28 is
`<</F 29 0 R/NewWindow true/S/Launch>>`, and object 29 is a file specification naming `file1.pdf`.
`action::launch` tells a person "running an application, which the sandbox withholds", which is false
of that file.

## 3. What is owed, for the round that builds it

- `action::launch` keeps the three sentences it has for the no-`/F` and the application cases. It
  hands a launch action with an `/F` onward as a remote document (first page, `/NewWindow` read as
  Table 203's is).
- `under_remote_documents` gains a fourth purpose. Whether the supplied bytes are a PDF is decided
  after they arrive, by §7.5.2's header, and a file that is not one gets the sandbox's sentence.
  This is the only point where the file's kind is known, because a file specification's name is not
  evidence of its type.
- *Printed* is not in scope for this action. Table 207 names no way to choose between opening and
  printing except the deprecated `/Win` dictionary's `/O`, which nothing here understands.
- `viewer-core/tests/headless.rs`'s launch assertion changes with the sentence, and a fixture holds
  both kinds of `/F`.

The row stays `reported` until then. The crates are the viewer's, so the reading is recorded here and
the build is `doc/todo/65` bucket 6's.
