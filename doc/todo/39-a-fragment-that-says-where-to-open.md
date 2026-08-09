# A fragment that says where to open

Status: **eight of the eleven are carried out since the four-hundred-and-fourteenth session; three
are reported by name.** Annex O is no longer `silent` — §O.1 and §O.2 are `implemented`, §O.2.1 and
§O.2.2 are `partial`, and the ledger's `silent` count is 0 for the first time since ADR 0206 found
these five rows. ADRs 0209, 0250.
Priority: 39 — capability, and what is left of it is three parameters with three different blockers
Clauses: Annex O (§O.2.1, §O.2.2), §12.7.8, §7.11.4
Code: `crates/pdf-model/src/fragment.rs`, `crates/viewer-core/src/open.rs`

## What is built

`pdf_model::fragment::Fragment::parse` reads all eleven parameters from the text after `#`, in the
order §O.2 makes normative, naming what it could not read rather than dropping it.
`viewer_core::Open::apply_fragment` carries out **`page`, `nameddest`, `structelem`, `comment`,
`zoom`, `view`, `viewrect` and `search`**, immediately after Table 29's `/OpenAction` as §O.2.2 asks.
`Command::Open` carries the fragment undecoded, and `pdf-viewer doc.pdf#page=5` is the first caller.

**`search` was the fourth reported parameter until the four-hundred-and-fourteenth session**, and
its reason — "no document-wide search" — expired when `viewer_core::Command::Find` became one. The
plan is made as the document opens and the *host* walks it, one page per `Find::Continue`, because
reading all 1023 pages of ISO 32000-2 is 5.84 s of interpretation and `CLAUDE.md`'s startup rules do
not permit that before page one is drawn. The word list is Annex O's own — any of the words matching
is a match — and the search does not wrap, because "the first matching word **in the document**"
would otherwise mean nothing. ADR 0250.

## The three that are left, and none of them is effort

Each is `reported` at runtime — [`Parameter::unhonoured`] names it and the host prints it — so
nothing here is silent. What each needs:

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
- **`highlight`** — a concept this vocabulary lacks, which is why it is last. **And a find bar has
  now been built without needing it**, which sharpens the case rather than weakening it: a *match*
  is a range of the readback and crossed as the shapes `Query::Find` already answered with, while
  this parameter's rectangle is measured from the corner of the page and is still a range of
  nothing. What this program
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
