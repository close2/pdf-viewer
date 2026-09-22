# 1239 — A second file a document names is asked for while the first is being applied

Status: **accepted**.
Context: `crates/viewer-core/src/command.rs` (`Purpose::NamedPage`, `Purpose::ThreadDocument`),
`crates/viewer-core/src/interact.rs` (`open_threaded`, `resume_threaded`, `decline_threaded`,
`request_named_page`, `supply_named_page`, `decline_named_page`),
`crates/viewer-core/src/open.rs` (`threading`, `page`),
`crates/pdf-model/src/view.rs` (`AwaitedPage`, `WantedPage`, `AppendedPage`, `file_awaited`,
`supply_named_pages`, `decline_named_pages`, `carried_page`, `MAX_NAMED_PAGE_FILES`),
`crates/pdf-model/src/forms_data.rs` (`carried`), `crates/pdf-model/src/action.rs` (`ThreadJump`,
`thread`), `crates/viewer-host/src/policy.rs` (`remote`, `asked_to_open_remote`,
`what_would_happen`, `asked_for`), the four windows, `crates/viewer-ffi/src/kinds.rs`,
`crates/viewer-confined/src/protocol.rs`, `doc/conformance/ledger.toml` §12.6.4.7, §12.7.8.3.2,
§12.7.8.3.3.
Builds: ADR 1227 (a file a document names has a level of its own), ADR 1235 (a named page becomes
a button's appearance, and the hop this crate cannot take), ADR 1223 (a value that crosses is
copied), ADR 1155 (a file a document named comes from a directory a person supplied), ADR 1185,
ADR 1070, ADR 0090.
Clauses: ISO 32000-2 §12.7.8.3.2 (Table 249's `/APRef`), §12.7.8.3.3 (Tables 252 and 253),
§12.6.4.7 (Table 209), §12.7.7, §12.7.6.4, §12.6.4.3, §12.6.4.4, §12.4.3, §7.8.2, §7.7.3.4,
§7.11.2.2, §14.11.2.1.

Three entries, one shape, one round. Table 253's `/F`, Table 249's `/APRef` through it, and Table
209's `/F` each name a **second PDF** and each was refused for a capability rather than for a
clause. ADR 1227 took that refusal off §12.6.4.3 and named what was left; this takes it off the
other two.

## 1. The shape ADR 1235 named, built

An import already suspends once: `viewer_core` raises `Purpose::ImportData`, a host answers from
`viewer_host::policy::read_import`'s person-supplied directory, and `interact::import` applies the
bytes. Table 253's `/F` is a **second** question raised *while the first is being applied*, and
ADR 1235 named it precisely — §12.6.4.4's suspended-walk shape, which `Purpose::TargetRoot` already
has. Every factual claim in that ADR held against the tree: `forms_data::carry` existed and copies a
crossing value whole, `named_page::page_as_form` existed and converts a page, and the missing piece
was the hop and nothing else.

What the hop needed that was not named: somewhere to *hold* the outstanding references. The model
kept them on `Import::appearance_reference` for `/APRef` and kept nothing at all for a template —
its refusal was a sentence. So `ViewState` now holds `AwaitedPage`s, one per reference, each saying
which file, which name, and what the page is wanted for; `file_awaited` names the next file,
`supply_named_pages` applies every reference into it, and `decline_named_pages` words what went
without. `Imported::awaiting` is a separate list from `Imported::refused` on purpose: a page in a
second file is a **question**, and a page this document does not name is a refusal, and printing
them as one sentence would tell a person something was declined while it was being asked for.

## 2. The level is `--remote-documents=`, and ADR 1227's own division decides it

ADR 1227 split the levels by *what the act is*: `--links=` decides whether another program on this
machine is started, and `--remote-documents=` decides "which PDFs beside their document may be
parsed by this program". Both new purposes are the second of those — a thread in another file is
§12.6.4.3's act reached through a different table, and a named page is one page of a second PDF
parsed and drawn. The import-data question is not a third level: `read_import` has the path rule and
no level at all, and putting these under it would make a person who said `--remote-documents=refuse`
find this program parsing a PDF the document named anyway.

**The path rule still comes before the level**, unchanged: `resolve_import` refuses anything that is
not a single path component beside the open document, at every level including `open`.

What differs per purpose is the *sentence*, and it has to: two of the three replace the document on
the screen and one draws a page into the one being read. `policy::what_would_happen` is that
sentence, so `asked_to_open_remote` puts the right question and `asked_for` prints the right clause
in a status line. A person asked "may this file be opened in place of the one you are reading?"
about a push button's artwork has been asked the wrong question.

## 3. A template page from a second file is carried, not referenced

`ViewState::appended` held `ObjectId`s of the document being read. A page of a second file has no
such identity, and this is the design question §12.7.8.3.4's row has held open — a second
`pdf_syntax::Document` reaching the interpreter. **It does not reach it.** `carried_page` builds the
page as a tree of direct objects: §7.8.2's concatenation of its own `/Contents` as the stream,
§7.7.3.4's inherited `/Resources` copied through `forms_data::carried`, §14.11.2.1's crop box, the
media box and Table 31's `/Rotate` as that page states them. `AppendedPage::Carried` holds it and
`viewer_core::open::page` builds it with `Pages::detached`, which is what §12.7.7's own templates
already go through.

**Every entry is a clause's or the other producer's, which is `CLAUDE.md`'s provenance test** —
ADR 1235 made the same argument entry by entry for a form, and a page is the same construction with
`/Type /Page` in place of `/Type /XObject`. `/Parent` is the one entry dropped, because the tree it
runs up is in the other file and its inheritance has already been applied here.

`/Annots` do **not** cross, and they are named. An annotation is identified by an object of the file
it is in, §12.5.5 draws one against the page it is on, and this program's whole annotation log is
keyed by `ObjectId` of one document; carrying them would be inventing identities. A person is told
how many did not come (trap 5).

## 4. Table 209's one remaining refusal is the table's own

§12.6.4.7's refusal read *"a thread in another file, which this reader has no filesystem to open"* —
ADR 1227's expired-capability shape exactly. `ThreadJump` now carries `/F` as the same `TargetRoot`
Table 203's `/F` is read into, so §7.11.2.2's restriction on a relative URL applies unchanged, and
`resume_threaded` reads the `/Threads` array, the thread's title and the bead's index in the
document that arrived — which is the only document any of the three is about.

One combination stays refused and the table refuses it: `/D`'s and `/B`'s reference forms each say
the thread "shall be in the current file", so a reference beside a `/F` is a file asserting that the
thread is here and elsewhere at once, and resolving it against either document would name an object
of the wrong one. That refusal is now built in `action::thread` rather than taken from `refused`'s
table of names, because its sentence depends on what the dictionary says — which is where `Launch`'s
already lived.

## Cost

A bound that is this program's and no clause's: `MAX_NAMED_PAGE_FILES`, sixteen second files per
import, past which the rest are refused by name. Table 253 states no number and could not — every
one of these costs a host question and a PDF parsed — and without a bound an FDF whose imported
template names its own file again would ask forever. Sixteen is far past any template library and far
short of what a file could write, since every imported field may name one (trap 38).

And a second: the copy is bounded by the allowance `forms_data`'s copy holds one crossing entry to,
sixteen mebibytes of stream data. A template page whose images pass it is refused whole rather than
carried half — half a page is a page drawn wrong, which is the failure that reports nothing.
