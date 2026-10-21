# 2171 — A launched document that is not a PDF is handed over at the reader's level, and a program never

Session 1667, the clause slot of batch one hundred and three. Status: **accepted**, the default and the
classification put to the owner in `doc/questions/Q562`. Clauses: ISO 32000-2 §12.6.4.6, Table 207,
§7.5.2. Context: ADRs 1155, 1227, 1358, 1368, 2154; `CLAUDE.md`'s clause-13 entry and
`doc/questions/A254`; principle 3's *A document's restrictions are the reader's to set*.

## 1. The reading

§12.6.4.6: "[a] launch action launches an application or opens or prints a document", and Table 207's
`/F` is "[t]he application that shall be launched or the document that shall be opened or printed".
`viewer_core` opens a PDF itself, by §7.5.2's header. Every other file was answered "an application
to launch, which the sandbox withholds" — two errors: a file that is not a PDF may be a document, and
the sandbox confines the renderer, not the host (ADR 2154's re-reading named both). **The standard
states no way to tell an application from a document**, so that is a choice this host makes, once, in
`viewer_host::policy::launched`, and writes down as one.

## 2. The decision

- **A program** — a file whose permissions let it be executed (a file whose permissions cannot be
  read counts as one), one beginning with an ELF, PE, Mach-O or `#!` signature, or one whose name ends
  in `RUN_BY_HANDLERS` (`desktop` above all, which `xdg-open` launches) — is started at no level and
  said by name: ADR 1358's departure, now for a program alone. Every doubt goes this way, because a
  document taken for a program is a sentence and a program taken for a document is a handler started.
- **A document** is every other file, handed to `URI_HANDLER` (the desktop's own handler, the person's
  setting) at `--launched-documents=`'s four levels — `refuse`, `ask`, `warn`, `open` — **`refuse`
  by default**, the brief's "`off` by default". It is a value of its own for ADR 1227's reason: a
  reader who let other PDFs be parsed has said nothing about another program being given a file.
- The core's sentence says only what is true of every such file: "is not a PDF document (§7.5.2's
  header), so this reader does not open it (Table 207)". The host adds what became of it.
  `viewer_core::states_a_pdf_header` is public so that both ask the one question of the same bytes.
- Carried out in `native_window/requests.rs` (GTK, Qt, the C window), `quorra`'s `dispatch.rs` and
  `quorra-confined`'s `asking.rs`, each with its *ask* answered. `open_uri` and `hand_over` share
  one `start_handler`.

## 3. What is not built

A menu entry for the level (`viewer_host::ActLevel`'s group) and the C ABI's `QUORRA_POLICY_*`
constant, which is `viewer-ffi`'s header, C# and count tests — `Policy::LaunchedDocuments` exists for
it. Printing, Table 207's third verb, is not chosen: as ADR 1358 reads it, only the deprecated `/Win`'s
`/O` could choose it, so a document is opened.
