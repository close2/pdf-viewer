# 1331 — An embedded document a fragment names is asked about by default

Session 1247. Status: **accepted** and built.
Context: `crates/viewer-host/src/policy.rs` (`EmbeddedDocuments`, `Unpacking`,
`may_open_extracted`, `asked_to_open_embedded`, `embedded_note`, `embedded_declined`,
`EMBEDDED_DOCUMENTS`, `embedded_documents`, `Settings::embedded_documents`),
`crates/viewer-host/src/restriction.rs` (`Act`, `ActLevel`, `ActEntry`, `Row::ActLevel`,
`Restrictions::set`, `Restrictions::with`, `Restrictions::act_entries`, `act_chosen`,
`Subject::Embedded`), the three windows' `extracted` and menus, `quorra-confined`'s `Extracted` arm.
Builds: ADR 1155 (the Links levels and their direction), ADR 1227 (remote documents), ADR 1291 (the
third menu group), ADR 1316 (`ef` opens a tab). Clause: ISO 32000-2 §O.2.1, Table Annex O.3.

## 1. The decision a later round must not re-litigate: the default is `ask`

Table Annex O.3's `ef` row says "the PDF processor shall open the embedded file contained within
the EmbeddedFiles name tree identified by name", and in the same cell "a PDF processor may choose to
prompt the user or even prevent opening of the file". The four levels are that choice given to the
reader: `refuse` is the prevention, `ask` the prompt, `warn` and `open` the `shall` with and without
a sentence afterwards.

The default is `ask`. The argument is the one the owner has ratified twice for acts a document asks
of this machine. `doc/questions/A67` kept `ask` for links: the narrowest-answer rule was written for
policies a person cannot see, and `refuse` by default would impose on the reader what `CLAUDE.md`
forbids a document to impose. `doc/questions/A98` chose `ask` for forms so that a person is the one
who decides. Both hold here with one addition: §O.2.1 says its identifiers are "useful primarily when
referring to them from external to the PDF such as a web page or web API". So the sentence that
names the file is frequently not the reader's, and it has to be shown to them. `open` by default
would let a web page unpack a file with nobody consulted. `refuse` would make the annex's `shall`
something a reader has to configure before it works. Only `ask` avoids both.

`quorra-confined` answers at `refuse`, as it does for links and forms. It has no dialogue and holds
one document, and it passes no fragment today, so the arm is the policy said out loud.

## 2. A level of its own, and on the menu

`RemoteDocuments` decides which PDFs beside the reader's document are parsed. This level decides
whether a file carried inside the document is. A reader who let their documents cross-reference
each other has said nothing about what a URI may unpack, so one value for both would make each
answer the other's question. The level is global, for `Submissions`' reason (`doc/questions/Q98`).
The menu's third group now holds two acts, so `Row::Sending` became `Row::ActLevel` over `Act` and
`ActLevel`. That keeps two levels from being mistaken for each other.

It also has a command-line word, `--embedded-documents=`, as links and remote documents do. The
annex's act happens while a document opens, before any menu can be reached, so a level set only from
the menu could never govern the launch's own fragment.

A file a person asked for from the files panel (`Extraction::Asked`) is opened at every level. The
caution is about a sentence that was not the reader's.
