# A fragment that says where to open

Status: **nine of the eleven are carried out since the four-hundred-and-seventy-fifth session; two
are reported by name.** Annex O is no longer `silent` — §O.1 and §O.2 are `implemented`, §O.2.1 and
§O.2.2 are `partial`, and the ledger's `silent` count is 0 for the first time since ADR 0206 found
these five rows. ADRs 0209, 0250, 0310.
Priority: 39 — capability, and what is left of it is two parameters with two different blockers
Clauses: Annex O (§O.2.1, §O.2.2), §12.7.8, §7.11.4
Code: `crates/pdf-model/src/fragment.rs`, `crates/viewer-core/src/open.rs`,
`crates/viewer-host/src/policy.rs`

**Run `tools/state.sh annex-o` before reading any of this.** It reads
`Parameter::unhonoured` — the program's own answer — and this file is a reading beside it.

## What is built

`pdf_model::fragment::Fragment::parse` reads all eleven parameters from the text after `#`, in the
order §O.2 makes normative, naming what it could not read rather than dropping it.
`viewer_core::Open::apply_fragment` carries out **`page`, `nameddest`, `structelem`, `comment`,
`ef`, `zoom`, `view`, `viewrect` and `search`**, immediately after Table 29's `/OpenAction` as
§O.2.2 asks. `Command::Open` carries the fragment undecoded, and `pdf-viewer doc.pdf#page=5` is the
first caller.

**Two parameters have come off the refused list, and neither for the reason the list gave.**
`search` in the four-hundred-and-fourteenth, when `viewer_core::Command::Find` became a
document-wide search: the plan is made as the document opens and the *host* walks it, one page per
`Find::Continue`, because reading all 1023 pages of ISO 32000-2 is 5.84 s of interpretation and
`CLAUDE.md`'s startup rules do not permit that before page one is drawn. The word list is Annex O's
own — any of the words matching is a match — and the search does not wrap, because "the first
matching word **in the document**" would otherwise mean nothing (ADR 0250). `ef` in the
four-hundred-and-seventy-fifth, when nothing arrived at all: its reason was two claims joined by an
"and", and only the second was ever about `ef` (ADR 0310).

## The two that are left, and what each actually needs

Each is `reported` at runtime — [`Parameter::unhonoured`] names it and the host prints it — so
nothing here is silent.

- **`highlight`** — a shape this vocabulary has nowhere to put, and the annex is what says the
  obvious substitute is wrong. Table Annex O.4: "Open the document with the specified rectangle
  highlighted … The nature of the highlighting is implementation-dependent." The tempting reading is
  that this program's *selection* is the highlight — it already crosses as geometry, a
  `Selected::quads` is already a list of quadrilaterals and a rectangle is one — and the annex
  refuses it: its two neighbouring parameters that point a person at something both say **selected**
  (`comment`'s "with the specified comment selected", `search`'s "selecting the first matching
  word"), and this one says **highlighted**, with an implementation-dependent nature the other two
  are not given. So carrying it out is a new `Query` answering a rectangle in device pixels — the
  form `Query::Selection` and `Query::Focus` already take — and **it should not be added before a
  host asks**, which is ADR 0164's test. This entry read "a concept this vocabulary lacks" for a
  hundred and six sessions; the reason is now derived from the annex rather than from the program,
  which is what `doc/habits.md`'s first shape asks for.
- **`fdf`** — a fetch, and **`CLAUDE.md` does not close it**: the exclusion list excludes authoring,
  multimedia, XFA and script behaviour, and says nothing about a network. Principle 3 keeps the
  filesystem and the network out of the crate that reads PDFs, which is a statement about
  `pdf-model` rather than about a host. "Open the document and then import the data from the
  specified FDF or XFDF file", where the argument is a URI: the shape is `Event::NeedsFile` and
  `Command::Supply { purpose: ImportData }` — what §12.7.6.4's import action already uses — plus a
  host that has a URI to resolve it against. §12.7.8's reader is what receives the bytes and it is
  built. Checked in the four-hundred-and-seventy-fifth and the reason stands unchanged.

## What `ef` still owes, which is one sentence rather than the parameter

§O.2.1: "[a]ny remaining parameters after this parameter apply to the selected embedded file." That
would mean opening a *second document* from the first and applying the rest of the fragment to it,
which `DocumentId` can express and nothing composes — `Command::Open` is a host's. So
`apply_fragment` carries `ef` out and then stops, saying how many parameters it did not apply, and
§O.2.1's ledger row is `partial` for that and nothing else.

**The other half of `ef` is a host policy and it is built**: `viewer_host::may_write_extracted`,
beside §12.7.6.4's import policy. `Event::Extracted` says whether a person or a URI asked, and the
three hosts write the first and decline the second, in the annex's own words — "a PDF processor may
choose to prompt the user or even prevent opening of the file". `doc/todo/38`'s *ask* level is where
*prevent* becomes *prompt*, and the policy is already in the one place that would change.

## What not to do

- **Not a URI parser.** RFC 3986 splitting is the host's; what crosses is the fragment alone. The
  rule `pdf-viewer` uses is in ADR 0209: the filesystem decides, not the punctuation.
- **Not a second reading of Table 149.** `View::from_keyword` is the one place, with §12.3.2.2's
  array and Annex O's `view` parameter as its two callers.
- **Not a fourth copy of the extraction policy.** Three hosts, one `viewer_host::policy` function;
  a fourth host calls it rather than deciding again.
