# A fragment that says where to open

Status: **seven of the eleven are carried out since the three-hundred-and-sixty-ninth session; four
are reported by name.** Annex O is no longer `silent` — §O.1 and §O.2 are `implemented`, §O.2.1 and
§O.2.2 are `partial`, and the ledger's `silent` count is 0 for the first time since ADR 0206 found
these five rows. ADR 0209.
Priority: 39 — capability, and what is left of it is four parameters with four different blockers
Clauses: Annex O (§O.2.1, §O.2.2), §12.7.8, §7.11.4
Code: `crates/pdf-model/src/fragment.rs`, `crates/viewer-core/src/open.rs`

## What is built

`pdf_model::fragment::Fragment::parse` reads all eleven parameters from the text after `#`, in the
order §O.2 makes normative, naming what it could not read rather than dropping it.
`viewer_core::Open::apply_fragment` carries out **`page`, `nameddest`, `structelem`, `comment`,
`zoom`, `view` and `viewrect`**, immediately after Table 29's `/OpenAction` as §O.2.2 asks.
`Command::Open` carries the fragment undecoded, and `pdf-viewer doc.pdf#page=5` is the first caller.

## The four that are left, and none of them is effort

Each is `reported` at runtime — [`Parameter::unhonoured`] names it and the host prints it — so
nothing here is silent. What each needs:

- **`search`** — a document-wide search. `Query::Find` answers for the page showing;
  "selecting the first matching word in the document" is a search across pages, which
  `viewer-core`'s crate map has owed since the boundary was built. The wordList is already parsed
  into its words.
- **`fdf`** — a fetch. "Open the document and then import the data from the specified FDF or XFDF
  file", where the argument is a URI: principle 3 keeps the filesystem and the network out of the
  crate that reads PDFs, so this is `Event::NeedsFile` and `Command::Supply { purpose:
  ImportData }` — the shape §12.7.6.4's import action already uses — plus a host that has a URI to
  resolve it against. §12.7.8's reader is what receives the bytes and it is built.
- **`ef`** — a policy, not a mechanism. §7.11.4's extraction exists (`Command::Extract`), and the
  annex asks for the thing `doc/todo/38` is about: "[s]ecurity should be strongly considered when
  opening an embedded file … a PDF processor may choose to prompt the user or even prevent opening
  of the file". That is `off` / `on` / *ask* / *warn*, arriving from a second direction. It also
  needs the sentence after it — "[a]ny remaining parameters after this parameter apply to the
  selected embedded file" — to mean opening a *second document* from the first and applying the
  rest of the fragment to it, which `DocumentId` can express and nothing composes yet. Until then
  `apply_fragment` stops at `ef` and says how many parameters it did not apply.
- **`highlight`** — a concept this vocabulary lacks, which is why it is last. What this program
  highlights is a *range of the readback*, with the geometry the text layer gives it; a rectangle
  measured from the corner of the page is not a range of anything. The honest shapes are either a
  new `Query` answering "this rectangle, in device pixels" — the form `Query::Selection` and
  `Query::Focus` already take — or a `Command` that sets one. **Neither should be added before a
  host asks**, which is ADR 0164's test and the reason this was left rather than guessed at: the
  annex itself says "[t]he nature of the highlighting is implementation-dependent", so the shape is
  settled by whoever draws it.

## What not to do

- **Not a URI parser.** RFC 3986 splitting is the host's; what crosses is the fragment alone. The
  rule `pdf-viewer` uses is in ADR 0209: the filesystem decides, not the punctuation.
- **Not a second reading of Table 149.** `View::from_keyword` is the one place, with §12.3.2.2's
  array and Annex O's `view` parameter as its two callers.
